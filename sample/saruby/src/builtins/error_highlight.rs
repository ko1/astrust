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
fn eh_find(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
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
fn eh_node_id(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if !a[0].is_heap() { return Ok(NIL); }
    let n = ivar_get(c, a[0], intern(b"__nid"));
    Ok(if n.is_fixnum() && n.as_fix() > 0 { n } else { NIL })
}

// 0: not loaded, 1: loaded, 2: disabled (--disable-error_highlight: options are read on another thread)
static STATE: AtomicU8 = AtomicU8::new(0);
pub fn disable() { STATE.store(2, Relaxed); }

/// Exception#detailed_message of a NameError / TypeError / ArgumentError before error_highlight is loaded:
/// load it (CRuby loads it at boot; here on first use) and answer as its CoreExt would.  None: not such an
/// exception, or already loaded (then CoreExt is in front of the caller already).
pub fn detailed_message(c: &mut Ctx, e: Value, a: &[Value]) -> Result<Option<Value>, Esc> {
    if STATE.load(Relaxed) != 0 { return Ok(None); }
    let v = c.vm();
    if ![v.c_name_error.get(), v.c_type_error.get(), v.c_argument_error.get()].iter().any(|&k| is_kind_of(c, e, k)) { return Ok(None); }
    STATE.store(1, Relaxed);   // once, even if it fails
    let f = c.str_new(b"error_highlight");
    crate::builtins::kernel::require_feature(c, f, false)?;
    let Some(m) = const_get_inherit(c, c.vm().c_object.get(), intern(b"ErrorHighlight"))? else { return Ok(None) };
    let kw = match kw_arg(a) { Some(h) => h, None => c.hash_new() };
    Ok(Some(funcall(c, m, intern(b"__detailed_message"), &[e, kw], std::ptr::null())?))
}

pub fn init(c: &mut Ctx) {
    if STATE.load(Relaxed) == 0 { STATE.store(1, Relaxed); }
    let m = define_module(c, "ErrorHighlight");
    define_singleton_cfunc(c, m, "__find", eh_find, 2);
    define_singleton_cfunc(c, m, "__node_id", eh_node_id, 1);
}
