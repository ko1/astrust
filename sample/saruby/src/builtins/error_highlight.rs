// The native part of error_highlight (prelude/lib/error_highlight/): a backtrace location's prism node id and
// the node of that id in a re-parse of the file, in place of RubyVM::AbstractSyntaxTree and the prism gem.
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
use crate::prism_generic::{Field, GNode};
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

/// [type, node_id, start_offset, length, {field => node | [node] | [start, length] | Symbol | String | Integer | nil}]
fn node_value(c: &mut Ctx, n: &GNode, consts: &[Vec<u8>]) -> RV {
    let h = c.hash_new();
    for (name, f) in &n.fields {
        let v = match f {
            Field::Node(Some(x)) => node_value(c, x, consts)?,
            Field::Nodes(xs) => { let a = c.ary_new_capa(xs.len()); for x in xs { let v = node_value(c, x, consts)?; c.ary_push(a, v); } a }
            Field::Str(s) => c.str_new(s),
            Field::Cst(Some(k)) => Value::sym(intern(&consts[*k as usize])),
            Field::Csts(ks) => { let a = c.ary_new_capa(ks.len()); for k in ks { c.ary_push(a, Value::sym(intern(&consts[*k as usize]))); } a }
            Field::Loc(Some(l)) => c.ary_new_from(&[Value::fix(l.start as i64), Value::fix(l.len as i64)]),
            Field::Int(i) => Value::fix(*i as i64),
            Field::Node(None) | Field::Cst(None) | Field::Loc(None) | Field::Other => NIL,
        };
        crate::hash::hash_set(c, h, Value::sym(intern(name.as_bytes())), v)?;
    }
    Ok(c.ary_new_from(&[Value::sym(intern(n.ty.as_bytes())), Value::fix(n.id as i64), Value::fix(n.loc.start as i64), Value::fix(n.loc.len as i64), h]))
}

/// ErrorHighlight.__find(path, node_id): [source, line offsets, node] or nil (no such file or node)
fn eh_find(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let p = crate::string::expect_str(c, a[0])?;
    let path = c.str_bytes(p).to_vec();
    if !a[1].is_fixnum() { return Ok(NIL); }
    let id = a[1].as_fix() as u32;
    let Ok(src) = std::fs::read(String::from_utf8_lossy(&path).as_ref()) else { return Ok(NIL) };
    let (root, offsets, consts) = crate::prism_generic::parse(&src, &path);
    let Some(n) = root.find(id) else { return Ok(NIL) };
    let node = node_value(c, n, &consts)?;
    let s = c.str_new(&src);
    let offs: Vec<Value> = offsets.iter().map(|&o| Value::fix(o as i64)).collect();
    let o = c.ary_from_vec(offs);
    Ok(c.ary_new_from(&[s, o, node]))
}

/// ErrorHighlight.__node_id(location): its prism node id (RubyVM::AbstractSyntaxTree.node_id_for_backtrace_location), or nil
fn eh_node_id(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    if !a[0].is_heap() { return Ok(NIL); }
    let n = ivar_get(c, a[0], intern(b"__nid"));
    Ok(if n.is_fixnum() && n.as_fix() > 0 { n } else { NIL })
}

// 0: not loaded, 1: loaded, 2: disabled (--disable-error_highlight: options are read on another thread)
static STATE: AtomicU8 = AtomicU8::new(0);
pub fn disable() { STATE.store(2, Relaxed); }

/// Boot: what gem_prelude's `require "error_highlight"` makes visible, without compiling the library: CoreExt
/// prepended to NameError / TypeError / ArgumentError (so a detailed_message defined on them is behind it, as in
/// CRuby) and ErrorHighlight's entry points, each a stub that loads the library (which redefines them) and calls
/// the real one.
pub fn boot(c: &mut Ctx) {
    if STATE.load(Relaxed) == 2 { return; }
    let m = define_module(c, "ErrorHighlight");
    let ce = define_module_under(c, m, "CoreExt");
    define_cfunc(c, ce, "detailed_message", stub_detailed_message, -1);
    let v = c.vm();
    for k in [v.c_name_error.get(), v.c_type_error.get(), v.c_argument_error.get()] { prepend_module(c, k, ce); }
    define_singleton_cfunc(c, m, "spot", |c, s, a, b| stub_call(c, s, a, b, b"spot"), -1);
    define_singleton_cfunc(c, m, "formatter", |c, s, a, b| stub_call(c, s, a, b, b"formatter"), -1);
    define_singleton_cfunc(c, m, "formatter=", |c, s, a, b| stub_call(c, s, a, b, b"formatter="), -1);
    crate::builtins::lazy_consts(c, m, &["DefaultFormatter", "VERSION"], "error_highlight");
}

/// Load the library; false: it did not define the real methods (then the stubs answer as Exception would).
fn load(c: &mut Ctx) -> Result<bool, Esc> {
    if STATE.load(Relaxed) == 0 {
        let f = c.str_new(b"error_highlight");
        crate::builtins::kernel::require_feature(c, f, false)?;
    }
    Ok(STATE.load(Relaxed) == 1)
}

/// CoreExt#detailed_message before the library is loaded: its CoreExt#detailed_message, bound to this exception
/// (its super is then what is behind CoreExt, as for this call).
fn stub_detailed_message(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if !load(c)? { return crate::builtins::exception::exc_detailed_message(c, slf, a, b); }
    let m = c.vm().c_object.get();
    let Some(m) = const_get_inherit(c, m, intern(b"ErrorHighlight"))? else { return crate::builtins::exception::exc_detailed_message(c, slf, a, b) };
    let kw = match kw_arg(c, a) { Some(h) => h, None => c.hash_new() };
    funcall(c, m, intern(b"__detailed_message"), &[slf, kw], std::ptr::null())
}

/// ErrorHighlight.spot / .formatter / .formatter= before the library is loaded: the library's, which replaced this.
fn stub_call(c: &mut Ctx, slf: Value, a: &[Value], b: Blk, name: &[u8]) -> RV {
    if !load(c)? { return Ok(NIL); }
    match kw_arg(c, a) {
        Some(_) => funcall_kw(c, slf, intern(name), a, b, true),
        None => funcall(c, slf, intern(name), a, b),
    }
}

/// The library's native part (required by error_highlight/saruby.rb).
pub fn init(c: &mut Ctx) {
    STATE.store(1, Relaxed);
    let o = c.vm().c_object.get();
    let m = match const_get_at(c, o, intern(b"ErrorHighlight")) { Some(m) if crate::array::is_class_or_module(m) => m, _ => define_module(c, "ErrorHighlight") };
    define_singleton_cfunc(c, m, "__find", eh_find, 2);
    define_singleton_cfunc(c, m, "__node_id", eh_node_id, 1);
}
