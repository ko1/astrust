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

pub struct RBinding { pub self_: Value, pub env: Value, pub names: Vec<Vec<(Sym, u32, u32)>>, pub cref: *const Cref, pub me: *const MethodEntry, pub file: &'static str, pub line: u32, pub dc: Value,
    pub implicit: Vec<Sym>,   // its own block's numbered parameters / `it` (compile.rs names: _1.. / __it)
    pub fkind: (u8, *const Iseq), // its frame's kind and iseq: the label of code eval'd in it
    pub svar: u64,    // its frame's home id for $~ / $_ (svar.rs); 0: none
    pub ret: u64,     // its frame's id (return target of code eval'd in it); 0: the caller's
    pub blk: Value,   // its frame's block as a Proc (rb_vm_make_binding escapes it), or nil
    pub visi: u8 }    // its scope's default visibility (CRuby keeps it in the cref)
impl RData for RBinding {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.self_); m.visit(&mut self.env); m.visit(&mut self.dc); m.visit(&mut self.blk); }
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

/// rb_iseq_local_variables: every local of `is` and of the scopes it is a block of, declared or not yet assigned.
pub fn iseq_var_names(c: &Ctx, is: *const Iseq) -> Vec<Sym> {
    let mut out: Vec<Sym> = vec![];
    let mut cur = c.iseq_ref(is);
    while let Some(i) = cur {
        for &(n, _, _) in &i.local_names { if !internal_name(n) && !out.contains(&n) { out.push(n); } }
        if i.kind == IseqKind::Eval { for &(n, _, _) in &i.outer_names { if !internal_name(n) && !out.contains(&n) { out.push(n); } } }
        if !matches!(i.kind, IseqKind::Block | IseqKind::Lambda) { break; }
        cur = c.iseq_ref(i.parent.get());
    }
    out
}
// compile.rs's hidden locals (`it`, anonymous forwarded params)
fn internal_name(n: Sym) -> bool { matches!(crate::symbol::sym_bytes(n), b"__it" | b"__fwd_rest" | b"__fwd_kw" | b"__fwd_blk") }

fn new_binding(c: &mut Ctx, b: RBinding) -> Value {
    let k = c.vm().c_binding.get();
    // the Values stay on the stack (in the closure) until the slot exists
    c.alloc_with(k, move || Body::Data(Box::new(b)))
}

/// Binding#dup / #clone: the same scope; locals defined by eval so far are carried over, later ones are not shared.
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    let Some(b) = c.data::<RBinding>(v) else { return Ok(None) };
    let (self_, env, names, cref, me, file, line, dc, svar, blk, visi, implicit, fkind) = (b.self_, b.env, b.names.clone(), b.cref, b.me, b.file, b.line, b.dc, (b.svar, b.ret), b.blk, b.visi, b.implicit.clone(), b.fkind);
    Ok(Some(new_binding(c, RBinding { self_, env, names, cref, me, file, line, dc, svar: svar.0, ret: svar.1, blk, visi, implicit, fkind })))
}

/// The main script's own TOPLEVEL_BINDING (it names the constant): replaces one a required file used,
/// keeping what local_variable_set put there, without a redefinition warning.
pub fn set_main_toplevel(c: &mut Ctx, b: Value) -> Result<(), Esc> {
    let ob = c.vm().c_object.get();
    let name = crate::symbol::intern(b"TOPLEVEL_BINDING");
    if let Some(old) = const_get_at(c, ob, name) {
        if c.data::<RBinding>(old).is_some() {
            let (olds, oenv) = { let o = rbind(c, old); (env_names(&o.names), o.env) };
            for (n, h, i) in olds {
                if find_var(rbind(c, b), n).is_some() { continue; }
                let e = c.env_up(oenv, h);
                let v = c.env_get(e, i);
                add_var(c, b, n, v);
            }
        }
    }
    const_set(c, ob, name, b);
    Ok(())
}

/// TOPLEVEL_BINDING when the main script does not name it (so its locals were not captured):
/// self is main, no locals of its own.
pub fn init_toplevel(c: &mut Ctx) {
    let ek = c.vm().c_env.get();
    let env = c.alloc(ek, Body::Env(crate::envvals::EnvVals::new(0), NIL));
    let (slf, cref) = (c.vm().main_obj.get(), c.vm().top_cref.get());
    let file: &'static str = "<main>";
    let b = new_binding(c, RBinding { self_: slf, env, names: vec![vec![]], cref, me: std::ptr::null(), file, line: 0, dc: NIL, svar: 0, ret: 0, blk: NIL, visi: VISI_PUBLIC, implicit: vec![], fkind: (FK_TOP, std::ptr::null()) });
    let ob = c.vm().c_object.get();
    const_set(c, ob, crate::symbol::intern(b"TOPLEVEL_BINDING"), b);
}

/// A frame's Binding and its block, which the caller promotes (`with_block`) once the frame borrow ends.
fn frame_binding(c: &Ctx, f: &Frame) -> (RBinding, *const Block) {   // blk-audit: the frame's block (H5)
    (RBinding { self_: f.fp[0].get(), env: f.fp[1].get(), names: vec![iseq_names(c, f.iseq(), true)], cref: f.cref(), me: f.me, file: frame_file(f), line: f.line(), dc: f.defined_class(),
               svar: crate::builtins::svar::frame_home_id(c, f), ret: f.id, blk: NIL,
               visi: if matches!(f.kind, FK_CLASS | FK_TOP | FK_EVAL | FK_BLOCK) { f.visi } else { VISI_PUBLIC },
               implicit: iseq_implicit(c, f.iseq()), fkind: if f.kind == FK_CFUNC { (FK_TOP, std::ptr::null()) } else { (f.kind, f.iseq()) } }, f.blk)
}
/// The numbered parameters / `it` of the block `is` itself (not of enclosing or nested blocks), in order.
fn iseq_implicit(c: &Ctx, is: *const Iseq) -> Vec<Sym> {
    let Some(i) = c.iseq_ref(is) else { return vec![] };
    if !matches!(i.kind, IseqKind::Block | IseqKind::Lambda) { return vec![]; }
    let mut v: Vec<Sym> = i.local_names.iter().map(|x| x.0).filter(|&n| numparam(n) || crate::symbol::sym_bytes(n) == b"__it").collect();
    v.sort_by(|a, b| crate::symbol::sym_bytes(*a).cmp(crate::symbol::sym_bytes(*b)));
    v.dedup();
    v
}
/// The Binding with its frame's block as a Proc: eval'd `yield` after the frame returned runs the copy.
fn with_block(c: &mut Ctx, (mut b, blk): (RBinding, *const Block)) -> Result<RBinding, Esc> {   // blk-audit: promoted before the frame can return
    b.blk = block_to_proc(c, blk)?;
    Ok(b)
}
pub fn set_receiver(c: &mut Ctx, b: Value, slf: Value) { rbind_mut(c, b).self_ = slf; }
/// C methods CRuby runs without a frame of their own (send, Proc#call, Method#call): binding / eval look past them.
fn frameless_cfunc(c: &Ctx, f: &Frame) -> bool {
    if f.kind != FK_CFUNC { return false; }
    let Some(m) = c.me_ref(f.me) else { return false };
    let n = crate::symbol::sym_bytes(m.name);
    match n {
        b"send" | b"__send__" | b"public_send" => true,
        b"call" | b"()" | b"yield" | b"===" | b"[]" => m.owner == c.vm().c_proc.get() || m.owner == c.vm().c_method.get(),
        _ => false,
    }
}
pub fn binding_of_caller(c: &mut Ctx) -> RV {
    let mut f = c.caller_frame().expect("binding: no caller frame");
    while frameless_cfunc(c, f) { match c.prev_frame(f) { Some(p) => f = p, None => break } }
    let b = frame_binding(c, f);
    let b = with_block(c, b)?;
    Ok(new_binding(c, b))
}
/// Binding of the frame at `addr` on the current frame chain (TracePoint#binding); nil if it is gone.
pub fn binding_of_frame_at(c: &mut Ctx, addr: usize) -> RV {
    let mut f = c.cur_frame();
    while let Some(x) = f {
        if x as *const Frame as usize == addr { let b = frame_binding(c, x); let b = with_block(c, b)?; return Ok(new_binding(c, b)); }
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
    // a Proc's outer block was promoted with it
    let blk = block_to_proc(c, b.outer_blk)?;
    Ok(new_binding(c, RBinding { self_: b.self_, env: b.env, names: vec![names], cref: b.cref, me: b.me, file, line, dc: b.defined_class, svar: b.home_id, ret: b.home_id, blk, visi: VISI_PUBLIC, implicit: vec![], fkind: (FK_TOP, std::ptr::null()) }))
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
    NAMES.with(|m| *m.borrow_mut().entry(name.clone()).or_insert_with(|| { crate::compile::set_file_enc(&name, crate::object::ENC_BINARY); Box::leak(name.into_boxed_str()) }))
}

pub fn eval_in_binding(c: &mut Ctx, bind: Value, src: &[u8], enc: u8, file: &'static str, line: i32) -> RV {
    let b = rbind(c, bind);
    let outer = vec![env_names(&b.names)];
    let (slf, env, cref, me, dc, svar, blk, visi) = (b.self_, b.env, b.cref, b.me, b.dc, (b.svar, b.ret), b.blk, b.visi);
    crate::compile::EVAL_NO_YIELD.set(me.is_null());
    let zs = match c.me_ref(me).map(|m| m.kind) { Some(MKind::Iseq(is)) => crate::compile::METHOD_ZSUPER.with(|m| m.borrow().get(&(is as usize)).cloned()).or(Some(vec![])), _ => None };
    crate::compile::EVAL_ZSUPER.with(|z| *z.borrow_mut() = zs);
    let fk = rbind(c, bind).fkind;
    let label = if fk.1.is_null() { None } else { Some(frame_label_of(c, fk.0, me, fk.1)) };
    crate::compile::EVAL_LABEL.with(|l| *l.borrow_mut() = label);
    let is = crate::compile::compile_source_enc(src, enc, file, line, IseqKind::Eval, outer);
    crate::compile::EVAL_LABEL.with(|l| *l.borrow_mut() = None);
    crate::compile::EVAL_ZSUPER.with(|z| *z.borrow_mut() = None);
    crate::compile::EVAL_NO_YIELD.set(false);
    crate::compile::flush_parse_warnings(c)?;
    let is = match is {
        Ok(is) => is,
        Err(m) => return syntax_err(c, m, file),
    };
    crate::builtins::tracepoint::trace_script_compiled(c, src)?;
    let mut new_env = NIL;
    let r = run_eval_iseq_env(c, is, slf, env, cref, me, visi, &mut new_env, dc, svar, blk);
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


#[allow(clippy::too_many_arguments)]
/// `blk`: the block `yield` runs (a Proc, kept alive by the Binding or the caller), or nil.
fn run_eval_iseq_env(c: &mut Ctx, is: &'static Iseq, slf: Value, env: Value, cref: *const Cref, me: *const MethodEntry, visi: u8, out_env: &mut Value, dc: Value, (svar, ret): (u64, u64), blk: Value) -> RV {
    let base = c.top();
    stack_check(c, is.nlocals as usize + 8)?;
    let id = c.cur_frame().map_or(0, |f| f.id);
    let blk = if blk == NIL { std::ptr::null() } else { proc_block(c, blk) };
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
/// `dt`: the cref's def_target, `dc`: the eval frame's definee (rt::def_class).
#[allow(clippy::too_many_arguments)]
fn eval_under(c: &mut Ctx, slf: Value, k: Value, dt: Value, dc: Value, src: &[u8], enc: u8, file: &'static str, line: i32) -> RV {
    let bind = binding_of_caller(c)?;
    let b = rbind(c, bind);
    let outer = vec![env_names(&b.names)];
    let (env, fk, me) = (b.env, b.fkind, b.me);
    let label = if fk.1.is_null() { None } else { Some(frame_label_of(c, fk.0, me, fk.1)) };
    crate::compile::EVAL_LABEL.with(|l| *l.borrow_mut() = label);
    let is = crate::compile::compile_source_enc(src, enc, file, line, IseqKind::Eval, outer);
    crate::compile::EVAL_LABEL.with(|l| *l.borrow_mut() = None);
    crate::compile::flush_parse_warnings(c)?;
    let is = match is { Ok(is) => is, Err(m) => return syntax_err(c, m, file) };
    crate::builtins::tracepoint::trace_script_compiled(c, src)?;
    let cur = c.cur_frame().map_or(std::ptr::null(), |f| f.cref());
    let cref = new_cref_target(k, cur, dt);
    let blk = rbind(c, bind).blk;
    let mut e = NIL;
    run_eval_iseq_env(c, is, slf, env, cref, std::ptr::null(), VISI_PUBLIC, &mut e, dc, (0, 0), blk)
}
/// instance_eval(String). As CRuby 4.0, the cref is the object's class while it has no singleton class
/// (constants resolve there first); `def` still makes the singleton class.
pub fn eval_with_self(c: &mut Ctx, slf: Value, src: &[u8], enc: u8, file: &'static str, line: i32) -> RV {
    if crate::rt::lacks_singleton(c, slf) { return eval_under(c, slf, class_of(slf), NIL, crate::rt::NO_DEFINEE, src, enc, file, line); }
    if !slf.is_heap() { return eval_under(c, slf, class_of(slf), NIL, class_of(slf), src, enc, file, line); }
    let k = class_of(slf);
    if c.class(k).is_singleton && c.class(k).attached == slf || is_class_or_module(slf) {
        let s = singleton_class(c, slf);
        return eval_under(c, slf, s, NIL, NIL, src, enc, file, line);
    }
    eval_under(c, slf, class_of(slf), slf, NIL, src, enc, file, line)
}
pub fn eval_in_class(c: &mut Ctx, k: Value, src: &[u8], enc: u8, file: &'static str, line: i32) -> RV {
    eval_under(c, k, k, NIL, NIL, src, enc, file, line)
}

pub fn local_variables(c: &mut Ctx, b: Value) -> RV {
    let names = env_names(&rbind(c, b).names);
    let out = c.ary_new();
    for (n, _, _) in names {
        let s = crate::symbol::sym_bytes(n);
        if (!s.starts_with(b"_") || s.len() > 1 && !s.starts_with(b"__")) && !numparam(n) { c.ary_push(out, Value::sym(n)); }
    }
    Ok(out)
}
/// _1 .. _9 (rb_numparam_id_p)
fn numparam(n: Sym) -> bool { matches!(crate::symbol::sym_bytes(n), [b'_', b'1'..=b'9']) }
/// check_local_id: a numbered parameter is not a local variable of a binding.
fn check_local_id(c: &mut Ctx, s: Value, n: Sym) -> Result<(), Esc> {
    let msg = if numparam(n) { format!("numbered parameter '{}' is not a local variable", sym_str(n)) }
        else if !local_name(crate::symbol::sym_bytes(n)) { let ins = crate::builtins::kernel::inspect_str(c, s)?; format!("wrong local variable name '{}' for {}", sym_str(n), ins) }
        else { return Ok(()) };
    let e = make_name_error(c, c.vm().c_name_error.get(), &msg, Value::sym(n), s)?;
    Err(raise_exc(c, e))
}
/// rb_is_local_name: a lowercase / `_` / non-ASCII start, then identifier characters only.
fn local_name(b: &[u8]) -> bool {
    match b.first() { Some(&x) if x == b'_' || x.is_ascii_lowercase() || x >= 0x80 => {} _ => return false }
    b.iter().all(|&x| x == b'_' || x.is_ascii_alphanumeric() || x >= 0x80)
}
/// A new variable of this Binding only (bind_local_variable_set): an env of its own over the binding's.
fn add_var(c: &mut Ctx, bind: Value, n: Sym, v: Value) {
    let ek = c.vm().c_env.get();
    let parent = rbind(c, bind).env;
    let e = c.alloc(ek, Body::Env(crate::envvals::EnvVals::new(1), parent));
    c.env_set(e, 0, v);
    let b = rbind_mut(c, bind);
    let mut names = vec![(n, 0, 0)];
    for &(m, h, i) in b.names.iter().flatten() { names.push((m, h + 1, i)); }
    b.names = vec![names];
    b.env = e;
}
fn find_var(b: &RBinding, n: Sym) -> Option<(u32, u32)> {
    for (nm, h, i) in b.names.iter().flatten() { if *nm == n { return Some((*h, *i)); } }
    None
}
fn b_lvar_get(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let n = crate::builtins::object::to_sym_arg(c, a[0])?;
    check_local_id(c, s, n)?;
    let b = rbind(c, s);
    let (var, env) = (find_var(b, n), b.env);
    match var {
        Some((h, i)) => { let e = c.env_up(env, h); Ok(c.env_get(e, i)) }
        None => {
            let ins = crate::builtins::kernel::inspect_str(c, s)?;
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("local variable '{}' is not defined for {}", sym_str(n), ins), Value::sym(n), s)?;
            Err(raise_exc(c, e))
        }
    }
}
fn b_lvar_set(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let n = crate::builtins::object::to_sym_arg(c, a[0])?;
    check_local_id(c, s, n)?;
    let b = rbind(c, s);
    let (var, env) = (find_var(b, n), b.env);
    match var {
        Some((h, i)) => { let e = c.env_up(env, h); c.env_set(e, i, a[1]); }
        None => add_var(c, s, n, a[1]),
    }
    Ok(a[1])
}
fn b_lvar_defined(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let n = crate::builtins::object::to_sym_arg(c, a[0])?;
    check_local_id(c, s, n)?;
    Ok(Value::bool(find_var(rbind(c, s), n).is_some()))
}
fn b_receiver(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(rbind(c, s).self_) }
fn b_eval(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 3)?;
    let src = expect_str(c, a[0])?;
    let (f, line) = eval_file_line(c, a.get(1).copied(), a.get(2).copied())?;
    let bytes = c.str_bytes(src).to_vec();
    let enc = c.str_enc(src);
    eval_in_binding(c, s, &bytes, enc, f, line)
}
/// eval's file and line arguments (StringValue / NUM2INT); no file or nil: "(eval at FILE:LINE)".
pub fn eval_file_line(c: &mut Ctx, file: Option<Value>, line: Option<Value>) -> Result<(&'static str, i32), Esc> {
    let f = match file {
        Some(v) if v != NIL => {
            let s = expect_str(c, v)?;
            let n = String::from_utf8_lossy(c.str_bytes(s)).into_owned();
            let e = c.str_enc(s);
            if e != crate::object::ENC_UTF8 || crate::compile::file_enc(&n) != e { crate::compile::set_file_enc(&n, e); }
            intern_file(n)
        }
        _ => eval_file(c),
    };
    let l = match line { Some(v) => crate::builtins::numeric_methods::num_to_long(c, v)? as i32, None => 1 };
    Ok((f, l))
}
/// File names given to eval, kept once each (frames and iseqs hold them as &'static str).
fn intern_file(name: String) -> &'static str {
    thread_local! { static NAMES: std::cell::RefCell<std::collections::HashMap<String, &'static str>> = Default::default(); }
    NAMES.with(|m| { let mut m = m.borrow_mut(); if let Some(s) = m.get(&name) { return *s; } let s: &'static str = Box::leak(name.clone().into_boxed_str()); m.insert(name, s); s })
}
/// implicit_parameter_get / _defined?'s name: (as given, compile.rs's name) for _1.._9 / it.
fn implicit_name(c: &mut Ctx, s: Value, v: Value) -> Result<(Sym, Sym), Esc> {
    let n = crate::builtins::object::to_sym_arg(c, v)?;
    if numparam(n) { return Ok((n, n)); }
    if crate::symbol::sym_bytes(n) == b"it" { return Ok((n, crate::symbol::intern(b"__it"))); }
    let e = make_name_error(c, c.vm().c_name_error.get(), &format!("'{}' is not an implicit parameter", sym_str(n)), Value::sym(n), s)?;
    Err(raise_exc(c, e))
}
fn b_implicit_get(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (n, real) = implicit_name(c, s, a[0])?;
    let b = rbind(c, s);
    if b.implicit.contains(&real) {
        if let Some((h, i)) = find_var(b, real) { let env = b.env; let e = c.env_up(env, h); return Ok(c.env_get(e, i)); }
    }
    let ins = crate::builtins::kernel::inspect_str(c, s)?;
    let e = make_name_error(c, c.vm().c_name_error.get(), &format!("implicit parameter '{}' is not defined for {}", sym_str(n), ins), Value::sym(n), s)?;
    Err(raise_exc(c, e))
}
fn b_implicit_defined(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (_, real) = implicit_name(c, s, a[0])?;
    Ok(Value::bool(rbind(c, s).implicit.contains(&real)))
}
fn b_implicit_parameters(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (it, it_name) = (crate::symbol::intern(b"__it"), crate::symbol::intern(b"it"));
    let v: Vec<Value> = rbind(c, s).implicit.iter().map(|&n| Value::sym(if n == it { it_name } else { n })).collect();
    Ok(c.ary_from_vec(v))
}
fn b_local_variables(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { local_variables(c, s) }
fn b_source_location(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (file, line) = { let b = rbind(c, s); (b.file, b.line) };
    let f = c.str_new(file.as_bytes());
    Ok(c.ary_new_from(&[f, Value::fix(line as i32 as i64)]))
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
    define_cfunc(c, k, "implicit_parameter_get", b_implicit_get, 1);
    define_cfunc(c, k, "implicit_parameter_defined?", b_implicit_defined, 1);
    define_cfunc(c, k, "implicit_parameters", b_implicit_parameters, 0);
}
