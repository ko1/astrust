// Binding / eval.  Locals of a scope that calls binding/eval live in its env
// (compile.rs marks them captured), so a Binding is (self, env, names, cref).
use crate::array::*;
use crate::string::*;
use crate::symbol::{intern, sym_str};
use crate::value::*;
use crate::vm::*;

pub struct RBinding { pub self_: Value, pub env: Value, pub names: Vec<Vec<(Sym, u32, u32)>>, pub cref: *const Cref, pub me: *const MethodEntry, pub file: &'static str, pub line: u32, pub extra: Value }
impl RData for RBinding {
    fn mark(&self, m: &mut crate::gc::Marker) { m.mark(self.self_); m.mark(self.env); m.mark(self.extra); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn rbind(v: Value) -> &'static mut RBinding { rdata::<RBinding>(v).unwrap() }

/// Names visible from frame f: (name, hops, env idx), innermost scope first.
fn scope_names(f: &Frame) -> Vec<Vec<(Sym, u32, u32)>> {
    let mut out = vec![];
    if f.iseq.is_null() { return out; }
    let is = unsafe { &*f.iseq };
    let mut lv = vec![];
    for &(n, _sl, ei) in &is.local_names { if ei >= 0 { lv.push((n, 0, ei as u32)); } }
    out.push(lv);
    // outer scopes of a block
    if !f.self_blk.is_null() {
        let mut b = f.self_blk;
        let mut hops = if is.env_size > 0 { 1 } else { 0 };
        let _ = &mut b;
        let mut env = unsafe { *f.fp.add(1) };
        if is.env_size > 0 { env = match &obj(env).body { Body::Env(_, p) => *p, _ => NIL }; }
        let _ = env;
        let _ = &mut hops;
    }
    out
}

pub fn binding_of_frame(_c: &mut Ctx, f: &Frame) -> RV {
    let env = unsafe { *f.fp.add(1) };
    let names = scope_names(f);
    let file = frame_file(f);
    Ok(crate::gc::alloc(vm().c_binding, Body::Data(Box::new(RBinding { self_: unsafe { *f.fp }, env, names, cref: f.cref, me: f.me, file, line: f.line, extra: NIL }))))
}
pub fn binding_of_caller(c: &mut Ctx) -> RV {
    let f = c.frame();
    let p = unsafe { &*f.prev };
    binding_of_frame(c, p)
}
pub fn binding_of_proc(_c: &mut Ctx, p: Value) -> RV {
    let b = rproc(p).blk;
    Ok(crate::gc::alloc(vm().c_binding, Body::Data(Box::new(RBinding { self_: b.self_, env: b.env, names: vec![], cref: b.cref, me: b.me, file: "(binding)", line: 0, extra: NIL }))))
}

fn env_names(env: Value, names: &[Vec<(Sym, u32, u32)>]) -> Vec<(Sym, u32, u32)> {
    let _ = env;
    names.iter().flatten().cloned().collect()
}

pub fn eval_in_binding(c: &mut Ctx, bind: Value, src: &[u8], file: &'static str, line: i32) -> RV {
    let b = rbind(bind);
    let outer = vec![env_names(b.env, &b.names)];
    let is = match crate::compile::compile_source(src, file, line, IseqKind::Eval, outer) {
        Ok(is) => is,
        Err(m) => return err(c, vm().c_syntax_error, m),
    };
    let (slf, env, cref, me) = (b.self_, b.env, b.cref, b.me);
    run_eval_iseq(c, is, slf, env, cref, me, VISI_PUBLIC)
}

pub fn run_eval_iseq(c: &mut Ctx, is: &'static Iseq, slf: Value, env: Value, cref: *const Cref, me: *const MethodEntry, visi: u8) -> RV {
    let base = c.sp;
    stack_check(c, is.nlocals as usize + 8)?;
    let mut fr = Frame::empty();
    fr.prev = c.frame; fr.fp = base; fr.iseq = is; fr.kind = FK_EVAL; fr.visi = visi;
    fr.id = unsafe { (*c.frame).id }; fr.cref = if cref.is_null() { vm().top_cref } else { cref }; fr.me = me;
    fr.blk = unsafe { (*c.frame).blk };
    unsafe {
        *base = slf;
        for i in 1..is.nlocals as usize { *base.add(i) = NIL; }
        c.sp = base.add(is.nlocals as usize);
        let e = if is.env_size > 0 { crate::gc::alloc(vm().c_env, Body::Env(vec![NIL; is.env_size as usize], env)) } else { env };
        *base.add(1) = if e == NIL { crate::gc::alloc(vm().c_env, Body::Env(vec![], NIL)) } else { e };
    }
    c.frame = &mut fr;
    let r = run_body(c, is.body, base);
    c.frame = fr.prev;
    c.sp = base;
    r
}

pub fn eval_with_self(c: &mut Ctx, slf: Value, src: &[u8], file: &'static str, line: i32) -> RV {
    let is = match crate::compile::compile_source(src, file, line, IseqKind::Eval, vec![]) { Ok(is) => is, Err(m) => return err(c, vm().c_syntax_error, m) };
    let k = if slf.is_heap() { singleton_class(slf) } else { class_of(slf) };
    let cref = new_cref(k, unsafe { (*c.frame).cref });
    run_eval_iseq(c, is, slf, NIL, cref, std::ptr::null(), VISI_PUBLIC)
}
pub fn eval_in_class(c: &mut Ctx, k: Value, src: &[u8], file: &'static str, line: i32) -> RV {
    let is = match crate::compile::compile_source(src, file, line, IseqKind::Eval, vec![]) { Ok(is) => is, Err(m) => return err(c, vm().c_syntax_error, m) };
    let cref = new_cref(k, unsafe { (*c.frame).cref });
    let base = c.sp;
    let r = run_eval_iseq(c, is, k, NIL, cref, std::ptr::null(), VISI_PUBLIC);
    let _ = base;
    r
}

pub fn local_variables(_c: &mut Ctx, b: Value) -> RV {
    let out = ary_new();
    for (n, _, _) in env_names(rbind(b).env, &rbind(b).names) { if !crate::symbol::sym_bytes(n).starts_with(b"_") || crate::symbol::sym_bytes(n).len() > 1 && !crate::symbol::sym_bytes(n).starts_with(b"__") { ary_push(out, Value::sym(n)); } }
    Ok(out)
}
fn find_var(b: &RBinding, n: Sym) -> Option<(u32, u32)> {
    for (nm, h, i) in b.names.iter().flatten() { if *nm == n { return Some((*h, *i)); } }
    None
}
fn b_lvar_get(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = crate::builtins::object::to_sym_arg(c, a[0])?;
    let b = rbind(s);
    match find_var(b, n) {
        Some((h, i)) => Ok(crate::rt::env_get(crate::rt::env_at(b.env, h), i)),
        None => {
            if b.extra != NIL { if let Some(v) = crate::hash::hash_get_opt(c, b.extra, Value::sym(n))? { return Ok(v); } }
            let ins = crate::builtins::kernel::inspect_str(c, s)?;
            let e = make_name_error(c, vm().c_name_error, &format!("local variable '{}' is not defined for {}", sym_str(n), ins), Value::sym(n), s)?;
            Err(raise_exc(c, e))
        }
    }
}
fn b_lvar_set(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = crate::builtins::object::to_sym_arg(c, a[0])?;
    let b = rbind(s);
    match find_var(b, n) {
        Some((h, i)) => { crate::rt::env_set(crate::rt::env_at(b.env, h), i, a[1]); }
        None => { if b.extra == NIL { b.extra = crate::hash::hash_new(); } let ex = b.extra; crate::hash::hash_set(c, ex, Value::sym(n), a[1])?; }
    }
    Ok(a[1])
}
fn b_lvar_defined(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = crate::builtins::object::to_sym_arg(c, a[0])?;
    Ok(Value::bool(find_var(rbind(s), n).is_some()))
}
fn b_receiver(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(rbind(s).self_) }
fn b_eval(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let src = expect_str(c, a[0])?;
    let bytes = rstr(src).b.clone();
    eval_in_binding(c, s, &bytes, "(eval)", 1)
}
fn b_local_variables(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { local_variables(c, s) }
fn b_source_location(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let b = rbind(s); Ok(ary_new_from(&[str_new(b.file.as_bytes()), Value::fix(b.line as i64)])) }

pub fn init() {
    let k = vm().c_binding;
    define_cfunc(k, "local_variable_get", b_lvar_get, 1);
    define_cfunc(k, "local_variable_set", b_lvar_set, 2);
    define_cfunc(k, "local_variable_defined?", b_lvar_defined, 1);
    define_cfunc(k, "receiver", b_receiver, 0);
    define_cfunc(k, "eval", b_eval, -1);
    define_cfunc(k, "local_variables", b_local_variables, 0);
    define_cfunc(k, "source_location", b_source_location, 0);
    let _ = intern;
}
