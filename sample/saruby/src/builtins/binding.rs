// Binding / eval.  Locals of a scope that calls binding/eval live in its env
// (compile.rs marks them captured), so a Binding is (self, env, names, cref).
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::object::*;
use crate::string::expect_str;
use crate::symbol::sym_str;
use crate::value::*;
use crate::vm::*;

pub struct RBinding { pub self_: Value, pub env: Value, pub names: Vec<Vec<(Sym, u32, u32)>>, pub cref: *const Cref, pub me: *const MethodEntry, pub file: &'static str, pub line: u32, pub extra: Value, pub dc: Value,
    pub svar: u64,    // its frame's home id for $~ / $_ (svar.rs); 0: none
    pub ret: u64 }    // its frame's id (return target of code eval'd in it); 0: the caller's
impl RData for RBinding {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.self_); m.visit(&mut self.env); m.visit(&mut self.extra); m.visit(&mut self.dc); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn rbind(c: &Ctx, v: Value) -> &RBinding { c.data::<RBinding>(v).expect("not a Binding") }
pub fn is_binding(c: &Ctx, v: Value) -> bool { c.data::<RBinding>(v).is_some() }
fn rbind_mut(c: &mut Ctx, v: Value) -> &mut RBinding { c.data_mut::<RBinding>(v).expect("not a Binding") }

/// Names visible from code whose env (fp[1]) belongs to `is` (or to its parent when
/// `is` has no env of its own), innermost first: (name, hops, env idx).
fn iseq_names(c: &Ctx, is: *const Iseq, own: bool) -> Vec<(Sym, u32, u32)> {
    let mut out = vec![];
    let mut hops = 0;
    let mut cur = c.iseq_ref(is);
    if !own { cur = cur.and_then(|i| c.iseq_ref(i.parent.get())); }
    while let Some(i) = cur {
        for &(n, _sl, ei) in &i.local_names { if ei >= 0 && !out.iter().any(|x: &(Sym, u32, u32)| x.0 == n) { out.push((n, hops, ei as u32)); } }
        let h = if i.env_size > 0 { 1 } else { 0 };
        if i.kind == IseqKind::Eval {
            for &(n, oh, ei) in &i.outer_names { if !out.iter().any(|x| x.0 == n) { out.push((n, oh + hops + h, ei)); } }
        }
        hops += h;
        if !matches!(i.kind, IseqKind::Block | IseqKind::Lambda) { break; }
        cur = c.iseq_ref(i.parent.get());
    }
    out
}

fn new_binding(c: &mut Ctx, b: RBinding) -> Value {
    let k = c.vm().c_binding.get();
    // the Values stay on the stack (in the closure) until the slot exists
    c.alloc_with(k, move || Body::Data(Box::new(b)))
}

/// Binding#dup / #clone: the same scope; locals defined by eval so far are carried over, later ones are not shared.
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    let Some(b) = c.data::<RBinding>(v) else { return Ok(None) };
    let (self_, env, names, cref, me, file, line, extra, dc, svar) = (b.self_, b.env, b.names.clone(), b.cref, b.me, b.file, b.line, b.extra, b.dc, (b.svar, b.ret));
    let extra = if extra == NIL { NIL } else { c.hash_dup(extra) };
    Ok(Some(new_binding(c, RBinding { self_, env, names, cref, me, file, line, extra, dc, svar: svar.0, ret: svar.1 })))
}

/// TOPLEVEL_BINDING when the main script does not name it (so its locals were not captured):
/// self is main, no locals of its own.
pub fn init_toplevel(c: &mut Ctx) {
    let ek = c.vm().c_env.get();
    let env = c.alloc(ek, Body::Env(crate::envvals::EnvVals::new(0), NIL));
    let (slf, cref) = (c.vm().main_obj.get(), c.vm().top_cref.get());
    let file: &'static str = "<main>";
    let b = new_binding(c, RBinding { self_: slf, env, names: vec![vec![]], cref, me: std::ptr::null(), file, line: 0, extra: NIL, dc: NIL, svar: 0, ret: 0 });
    let ob = c.vm().c_object.get();
    const_set(c, ob, crate::symbol::intern(b"TOPLEVEL_BINDING"), b);
}

fn frame_binding(c: &Ctx, f: &Frame) -> RBinding {
    RBinding { self_: f.fp[0].get(), env: f.fp[1].get(), names: vec![iseq_names(c, f.iseq(), true)], cref: f.cref(), me: f.me, file: frame_file(f), line: f.line(), extra: NIL, dc: f.defined_class(),
               svar: crate::builtins::svar::frame_home_id(c, f), ret: f.id }
}
pub fn binding_of_caller(c: &mut Ctx) -> RV {
    let b = frame_binding(c, c.caller_frame().expect("binding: no caller frame"));
    Ok(new_binding(c, b))
}
/// Binding of the frame at `addr` on the current frame chain (TracePoint#binding); nil if it is gone.
pub fn binding_of_frame_at(c: &mut Ctx, addr: usize) -> RV {
    let mut f = c.cur_frame();
    while let Some(x) = f {
        if x as *const Frame as usize == addr { let b = frame_binding(c, x); return Ok(new_binding(c, b)); }
        f = c.prev_frame(x);
    }
    Ok(NIL)
}
pub fn binding_of_proc(c: &mut Ctx, p: Value) -> RV {
    let b = c.proc_data(p).blk;
    let (names, file, line) = match b.kind {
        BlockKind::Iseq(is) => match c.iseq_ref(is) { Some(i) => (iseq_names(c, is, false), i.info.file, i.info.line), None => (vec![], "(binding)", 0) },
        _ => (vec![], "(binding)", 0),
    };
    Ok(new_binding(c, RBinding { self_: b.self_, env: b.env, names: vec![names], cref: b.cref, me: b.me, file, line, extra: NIL, dc: b.defined_class, svar: b.home_id, ret: b.home_id }))
}

fn env_names(names: &[Vec<(Sym, u32, u32)>]) -> Vec<(Sym, u32, u32)> {
    names.iter().flatten().cloned().collect()
}

/// The file name of an eval given none: "(eval at FILE:LINE)" of the calling Ruby code.
pub fn eval_file(c: &Ctx) -> &'static str {
    thread_local! { static NAMES: std::cell::RefCell<std::collections::HashMap<String, &'static str>> = Default::default(); }
    let mut f = c.cur_frame();
    while let Some(x) = f { if !x.iseq().is_null() { break; } f = c.prev_frame(x); }
    let Some(f) = f else { return "(eval)" };
    let name = format!("(eval at {}:{})", frame_file(f), f.line());
    NAMES.with(|m| *m.borrow_mut().entry(name.clone()).or_insert_with(|| Box::leak(name.into_boxed_str())))
}

pub fn eval_in_binding(c: &mut Ctx, bind: Value, src: &[u8], file: &'static str, line: i32) -> RV {
    let b = rbind(c, bind);
    let outer = vec![env_names(&b.names)];
    let (slf, env, cref, me, dc, svar) = (b.self_, b.env, b.cref, b.me, b.dc, (b.svar, b.ret));
    let is = match crate::compile::compile_source(src, file, line, IseqKind::Eval, outer) {
        Ok(is) => is,
        Err(m) => return err(c, c.vm().c_syntax_error.get(), m),
    };
    crate::builtins::tracepoint::trace_script_compiled(c, src)?;
    let mut new_env = NIL;
    let r = run_eval_iseq_env(c, is, slf, env, cref, me, VISI_PUBLIC, &mut new_env, dc, svar);
    // locals the eval introduced stay visible through this Binding
    if is.env_size > 0 && new_env != NIL {
        let b = rbind_mut(c, bind);
        let mut names: Vec<(Sym, u32, u32)> = is.local_names.iter().filter(|x| x.2 >= 0).map(|&(n, _, ei)| (n, 0, ei as u32)).collect();
        for &(n, h, i) in b.names.iter().flatten() { if !names.iter().any(|x| x.0 == n) { names.push((n, h + 1, i)); } }
        b.names = vec![names];
        b.env = new_env;
    }
    r
}

pub fn run_eval_iseq(c: &mut Ctx, is: &'static Iseq, slf: Value, env: Value, cref: *const Cref, me: *const MethodEntry, visi: u8) -> RV {
    let mut e = NIL;
    run_eval_iseq_env(c, is, slf, env, cref, me, visi, &mut e, NIL, (0, 0))
}

#[allow(clippy::too_many_arguments)]
fn run_eval_iseq_env(c: &mut Ctx, is: &'static Iseq, slf: Value, env: Value, cref: *const Cref, me: *const MethodEntry, visi: u8, out_env: &mut Value, dc: Value, (svar, ret): (u64, u64)) -> RV {
    let base = c.top();
    stack_check(c, is.nlocals as usize + 8)?;
    let (id, blk) = c.cur_frame().map_or((0, std::ptr::null()), |f| (f.id, f.blk));
    // blocks made (and a return) here belong to the binding's frame while it runs
    let id = if ret != 0 && crate::builtins::svar::home_live(c, ret) { ret } else { id };
    let mut fr = Frame::empty(base);
    fr.prev = c.frame; fr.iseq = is; fr.kind = FK_EVAL; fr.visi = visi;
    fr.id = id; fr.cref = if cref.is_null() { c.vm().top_cref.get() } else { cref }; fr.me = me;
    fr.blk = blk;
    // a binding taken in a module_eval / instance_eval block keeps that block's definee
    fr.defined_class = dc;
    base[0].set(slf);
    nil_fill(base, 1, is.nlocals as usize);
    c.set_sp(base, is.nlocals as usize);
    let ek = c.vm().c_env.get();
    let e = if is.env_size > 0 { c.alloc(ek, Body::Env(crate::envvals::EnvVals::new(is.env_size as usize), env)) } else { env };
    base[1].set(if e == NIL { c.alloc(ek, Body::Env(crate::envvals::EnvVals::new(0), NIL)) } else { e });
    *out_env = base[1].get();
    c.frame = &mut fr.h;
    crate::builtins::svar::redirect(c, svar);
    let r = run_body(c, is.body, base);
    crate::builtins::svar::end_redirect(c, svar);
    c.frame = fr.prev;
    c.set_sp(base, 0);
    r
}

/// instance_eval / module_eval with a String: self and the definee are given, locals are the caller's.
fn eval_under(c: &mut Ctx, slf: Value, k: Value, src: &[u8], file: &'static str, line: i32) -> RV {
    let bind = binding_of_caller(c)?;
    let b = rbind(c, bind);
    let outer = vec![env_names(&b.names)];
    let env = b.env;
    let is = match crate::compile::compile_source(src, file, line, IseqKind::Eval, outer) { Ok(is) => is, Err(m) => return err(c, c.vm().c_syntax_error.get(), m) };
    crate::builtins::tracepoint::trace_script_compiled(c, src)?;
    let cur = c.cur_frame().map_or(std::ptr::null(), |f| f.cref());
    let cref = new_cref(k, cur);
    run_eval_iseq(c, is, slf, env, cref, std::ptr::null(), VISI_PUBLIC)
}
pub fn eval_with_self(c: &mut Ctx, slf: Value, src: &[u8], file: &'static str, line: i32) -> RV {
    let k = if slf.is_heap() { singleton_class(c, slf) } else { class_of(slf) };
    eval_under(c, slf, k, src, file, line)
}
pub fn eval_in_class(c: &mut Ctx, k: Value, src: &[u8], file: &'static str, line: i32) -> RV {
    eval_under(c, k, k, src, file, line)
}

pub fn local_variables(c: &mut Ctx, b: Value) -> RV {
    let names = env_names(&rbind(c, b).names);
    let out = c.ary_new();
    for (n, _, _) in names {
        let s = crate::symbol::sym_bytes(n);
        if !s.starts_with(b"_") || s.len() > 1 && !s.starts_with(b"__") { c.ary_push(out, Value::sym(n)); }
    }
    Ok(out)
}
fn find_var(b: &RBinding, n: Sym) -> Option<(u32, u32)> {
    for (nm, h, i) in b.names.iter().flatten() { if *nm == n { return Some((*h, *i)); } }
    None
}
fn b_lvar_get(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = crate::builtins::object::to_sym_arg(c, a[0])?;
    let b = rbind(c, s);
    let (var, env, extra) = (find_var(b, n), b.env, b.extra);
    match var {
        Some((h, i)) => { let e = c.env_up(env, h); Ok(c.env_get(e, i)) }
        None => {
            if extra != NIL { if let Some(v) = crate::hash::hash_get_opt(c, extra, Value::sym(n))? { return Ok(v); } }
            let ins = crate::builtins::kernel::inspect_str(c, s)?;
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("local variable '{}' is not defined for {}", sym_str(n), ins), Value::sym(n), s)?;
            Err(raise_exc(c, e))
        }
    }
}
fn b_lvar_set(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = crate::builtins::object::to_sym_arg(c, a[0])?;
    let b = rbind(c, s);
    let (var, env, extra) = (find_var(b, n), b.env, b.extra);
    match var {
        Some((h, i)) => { let e = c.env_up(env, h); c.env_set(e, i, a[1]); }
        None => {
            let ex = if extra == NIL { let h = c.hash_new(); rbind_mut(c, s).extra = h; h } else { extra };
            crate::hash::hash_set(c, ex, Value::sym(n), a[1])?;
        }
    }
    Ok(a[1])
}
fn b_lvar_defined(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = crate::builtins::object::to_sym_arg(c, a[0])?;
    Ok(Value::bool(find_var(rbind(c, s), n).is_some()))
}
fn b_receiver(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(rbind(c, s).self_) }
fn b_eval(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let src = expect_str(c, a[0])?;
    let bytes = c.str_bytes(src).to_vec();
    let f = eval_file(c);
    eval_in_binding(c, s, &bytes, f, 1)
}
fn b_local_variables(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { local_variables(c, s) }
fn b_source_location(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (file, line) = { let b = rbind(c, s); (b.file, b.line) };
    let f = c.str_new(file.as_bytes());
    Ok(c.ary_new_from(&[f, Value::fix(line as i64)]))
}

pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let k = vmr.c_binding.get();
    define_cfunc(c, k, "local_variable_get", b_lvar_get, 1);
    define_cfunc(c, k, "local_variable_set", b_lvar_set, 2);
    define_cfunc(c, k, "local_variable_defined?", b_lvar_defined, 1);
    define_cfunc(c, k, "receiver", b_receiver, 0);
    define_cfunc(c, k, "eval", b_eval, -1);
    define_cfunc(c, k, "local_variables", b_local_variables, 0);
    define_cfunc(c, k, "source_location", b_source_location, 0);
}
