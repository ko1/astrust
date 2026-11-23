// TracePoint and Kernel#set_trace_func.
//
// Nothing here runs unless a hook is enabled (vm::trace_mask() != 0).  Where events come from:
// * call / return / c_call / c_return: vm::invoke_impl hands every call to invoke_traced while call
//   events are traced; fill_cc then caches no fast path and enabling bumps the method serial, so
//   interpreted and baked call sites (whose inline paths are all guarded by the serial) reach invoke.
// * line / rescue: node_trace, which compile.rs puts before each statement (and rescue clause) of
//   code compiled for tracing (vm::TRACE_COMPILE); call / b_call / class events of that code are
//   reported in its own frame (run_traced_body), so binding / self / lineno are the callee's.
// * raise: vm::raise_exc.  class / end: rt::run_class_body.  script_compiled: eval.
// * thread_begin / thread_end: fiber::fiber_main of a thread's root fiber.
#![forbid(unsafe_code)]
use crate::array::*;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::symbol::{intern, sym_str};
use crate::value::*;
use crate::vm::*;
use std::cell::Cell;

const PRELUDE_FILE: &str = "<internal:prelude>";
const EV_ALL: u32 = EV_LINE | EV_CLASS | EV_END | EV_CALL | EV_RETURN | EV_C_CALL | EV_C_RETURN | EV_RAISE | EV_B_CALL | EV_B_RETURN
    | EV_THREAD_BEGIN | EV_THREAD_END | EV_SCRIPT_COMPILED | EV_RESCUE;
const EVENTS: [(u32, &str); 14] = [
    (EV_LINE, "line"), (EV_CLASS, "class"), (EV_END, "end"), (EV_CALL, "call"), (EV_RETURN, "return"), (EV_C_CALL, "c_call"),
    (EV_C_RETURN, "c_return"), (EV_RAISE, "raise"), (EV_B_CALL, "b_call"), (EV_B_RETURN, "b_return"), (EV_THREAD_BEGIN, "thread_begin"),
    (EV_THREAD_END, "thread_end"), (EV_SCRIPT_COMPILED, "script_compiled"), (EV_RESCUE, "rescue"),
];
fn ev_name(ev: u32) -> &'static str { EVENTS.iter().find(|e| e.0 == ev).map(|e| e.1).unwrap_or("unknown") }

/// What a hook sees of the event it is called for.
#[derive(Clone, Copy)]
pub struct Arg {
    ev: u32,
    path: &'static str,
    line: u32,
    mid: Option<Sym>,
    callee: Option<Sym>,
    klass: Value,
    self_: Value,
    ret: Value,
    exc: Value,
    frame: usize,      // the event's frame (for #binding), 0: none
    iseq: usize,       // its code (targets, #parameters), 0: none
    lambda: bool,
    script: Value,
}

pub struct RTrace { events: u32, proc_: Value, enabled: bool, target: usize, target_line: u32, target_thread: Value, stf: bool, cur: Option<Arg> }
impl RData for RTrace {
    fn visit(&mut self, m: &mut crate::gc::Visitor) {
        m.visit(&mut self.proc_); m.visit(&mut self.target_thread);
        if let Some(a) = &mut self.cur { m.visit(&mut a.klass); m.visit(&mut a.self_); m.visit(&mut a.ret); m.visit(&mut a.exc); m.visit(&mut a.script); }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

thread_local! {
    static IN_HOOK: Cell<bool> = const { Cell::new(false) };
    static REENTRY: Cell<bool> = const { Cell::new(false) };
    // a raise in a builtin is reported after the builtin's c_return, as CRuby does
    static RAISE_PENDING: Cell<bool> = const { Cell::new(false) };
}

/// Run `f` with no events reported (as code CRuby runs in C).
pub fn quiet<R>(f: impl FnOnce() -> R) -> R {
    let was = IN_HOOK.with(|h| h.replace(true));
    let re = REENTRY.with(|r| r.replace(false));
    let r = f();
    REENTRY.with(|r| r.set(re));
    IN_HOOK.with(|h| h.set(was));
    r
}

fn rt(c: &Ctx, v: Value) -> Option<&RTrace> { c.data::<RTrace>(v) }
fn rt_mut(c: &mut Ctx, v: Value) -> &mut RTrace { c.data_mut::<RTrace>(v).expect("a TracePoint") }

pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let k = c.vm().c_tracepoint.get();
    if k != NIL && crate::builtins::inherits(c, klass, k) {
        return Ok(Some(c.alloc(klass, Body::Data(Box::new(RTrace { events: 0, proc_: NIL, enabled: false, target: 0, target_line: 0, target_thread: NIL, stf: false, cur: None })))));
    }
    Ok(None)
}

fn list(c: &mut Ctx) -> Value {
    let l = c.vm().trace_list.get();
    if l != NIL { return l; }
    let a = c.ary_new();
    c.vm().trace_list.set(a);
    a
}
/// The mask is the union of the enabled hooks' events.
fn update_mask(c: &mut Ctx) {
    let l = list(c);
    let m = c.ary(l).iter().filter_map(|&t| rt(c, t)).filter(|d| d.enabled).fold(0, |m, d| m | d.events);
    set_trace_mask(m);
}

// ---- event data from frames ---------------------------------------------------------------------

/// The nearest frame running Ruby code (cfunc frames have no iseq).
fn ruby_frame(c: &Ctx) -> Option<&Frame> {
    let mut f = c.cur_frame();
    while let Some(x) = f { if x.kind != FK_CFUNC && !x.iseq().is_null() { return Some(x); } f = c.prev_frame(x); }
    None
}
fn method_of(c: &Ctx, me: *const MethodEntry) -> (Option<Sym>, Option<Sym>, Value) {
    match c.me_ref(me) {
        Some(m) => {
            let orig = match m.kind { MKind::Iseq(is) => c.iseq_ref(is).map(|i| i.name).filter(|n| n.0 != 0).unwrap_or(m.name), _ => m.name };
            // an alias' entry is a copy of the original under its own name (object::alias_method)
            let called = c.vm().alias_names.borrow().get(&(me as usize)).copied().unwrap_or(m.name);
            (Some(orig), Some(called), m.owner)
        }
        None => (None, None, NIL),
    }
}
/// An event at the current Ruby frame.
fn frame_arg(c: &Ctx, ev: u32) -> Arg {
    let mut a = Arg { ev, path: "", line: 0, mid: None, callee: None, klass: NIL, self_: NIL, ret: NIL, exc: NIL, frame: 0, iseq: 0, lambda: false, script: NIL };
    if let Some(f) = ruby_frame(c) {
        a.path = frame_file(f);
        a.line = f.line();
        a.self_ = f.fp[0].get();
        a.frame = f as *const Frame as usize;
        a.iseq = f.iseq() as usize;
        a.lambda = f.kind == FK_LAMBDA;
        let (m, k, o) = method_of(c, f.me);
        a.mid = m; a.callee = k; a.klass = o;
    }
    a
}
fn is_internal(c: &Ctx) -> bool { ruby_frame(c).is_some_and(|f| frame_file(f) == PRELUDE_FILE) }

/// Is code `is` the target or a block inside it?
fn in_target(c: &Ctx, is: usize, target: usize) -> bool {
    let mut cur = c.iseq_ref(is as *const Iseq);
    while let Some(i) = cur {
        if i as *const Iseq as usize == target { return true; }
        if !matches!(i.kind, IseqKind::Block | IseqKind::Lambda) { return false; }
        cur = c.iseq_ref(i.parent.get());
    }
    false
}

fn current_thread(c: &mut Ctx) -> Value {
    let k = c.vm().c_thread.get();
    // reports nothing itself
    let was = IN_HOOK.with(|h| h.replace(true));
    let saved = (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v);
    let t = funcall(c, k, intern(b"current"), &[], std::ptr::null()).unwrap_or(NIL);
    (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v) = saved;
    IN_HOOK.with(|h| h.set(was));
    t
}

/// Run the hooks enabled for event `a`.  An exception from a hook propagates.
pub fn fire(c: &mut Ctx, a: Arg) -> Result<(), Esc> {
    if IN_HOOK.with(|h| h.get()) && !REENTRY.with(|r| r.get()) { return Ok(()); }
    let l = c.vm().trace_list.get();
    if l == NIL { return Ok(()); }
    // by index: enable / disable replace the list, so `l` (kept on the stack) holds the hooks alive
    let mut thread = None;
    for i in 0..c.ary(l).len() {
        let Some(&tp) = c.ary(l).get(i) else { break };
        let Some(d) = rt(c, tp) else { continue };
        if !d.enabled || d.events & a.ev == 0 { continue; }
        if d.target != 0 && (a.iseq == 0 || !in_target(c, a.iseq, d.target)) { continue; }
        if d.target_line != 0 && a.line != d.target_line { continue; }
        let tt = d.target_thread;
        if tt != NIL {
            let cur = match thread { Some(t) => t, None => { let t = current_thread(c); thread = Some(t); t } };
            if cur != tt { continue; }
        }
        call_hook(c, tp, a)?;
    }
    Ok(())
}

fn call_hook(c: &mut Ctx, tp: Value, a: Arg) -> Result<(), Esc> {
    let saved = (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v, c.errinfo);
    c.errinfo = NIL;
    let (p, stf) = { let d = rt_mut(c, tp); d.cur = Some(a); (d.proc_, d.stf) };
    // allow_reentry lets events reach hooks again, but not the hooks those run
    let was = IN_HOOK.with(|h| h.replace(true));
    let re = REENTRY.with(|r| r.replace(false));
    let r = if stf { stf_call(c, p, &a) } else { call_proc(c, p, &[tp], std::ptr::null()) };
    REENTRY.with(|r| r.set(re));
    IN_HOOK.with(|h| h.set(was));
    if let Some(d) = c.data_mut::<RTrace>(tp) { d.cur = None; }
    r?;
    (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v, c.errinfo) = saved;
    Ok(())
}

/// set_trace_func's proc: (event, file, line, id, binding, classname).
fn stf_call(c: &mut Ctx, p: Value, a: &Arg) -> RV {
    let name = match a.ev { EV_C_CALL => "c-call", EV_C_RETURN => "c-return", e => ev_name(e) };
    let ev = c.str_new(name.as_bytes());
    let file = if a.path.is_empty() { NIL } else { c.str_new(a.path.as_bytes()) };
    let id = a.mid.map(Value::sym).unwrap_or(NIL);
    let b = if a.frame != 0 && !matches!(a.ev, EV_C_CALL | EV_C_RETURN) { crate::builtins::binding::binding_of_frame_at(c, a.frame)? } else { NIL };
    call_proc(c, p, &[ev, file, Value::fix(a.line as i64), id, b, a.klass], std::ptr::null())
}

// ---- event sources (called from the VM when trace_mask() has the event) --------------------------

/// node_trace: a line or rescue event at the current frame (its line was just stored).
#[cold]
#[inline(never)]
pub fn trace_point(c: &mut Ctx, ev: u32) -> Result<(), Esc> {
    if trace_mask() & ev == 0 { return Ok(()); }
    let mut a = frame_arg(c, ev);
    if ev == EV_RESCUE { a.exc = c.errinfo; }
    fire(c, a)
}

/// The body of code compiled for tracing, between its call and return events (method, block,
/// class), in its own frame.
#[cold]
#[inline(never)]
pub fn run_traced_body(c: &mut Ctx, is: &'static Iseq, base: Stk, call_ev: u32, ret_ev: u32) -> RV {
    if trace_mask() & call_ev != 0 {
        let mut a = frame_arg(c, call_ev);
        if call_ev == EV_CLASS { a.mid = None; a.callee = None; a.klass = NIL; }
        fire(c, a)?;
    }
    let r = run_body(c, is.body, base);
    if trace_mask() & ret_ev == 0 { return r; }
    let mut a = frame_arg(c, ret_ev);
    if ret_ev == EV_END { a.mid = None; a.callee = None; a.klass = NIL; }
    let fid = c.cur_frame().map_or(0, |f| f.id);
    match &r {
        Ok(v) => { a.ret = *v; a.line = is.info.end_line; }
        Err(_) if (c.esc == EscKind::Return && c.esc_tag == fid) || (ret_ev == EV_B_RETURN && c.esc == EscKind::Next) => a.ret = c.esc_val,
        Err(_) => {}
    }
    fire(c, a)?;
    r
}

/// invoke while call events are traced: the events of a method that is not compiled for tracing are
/// reported around it (a builtin's, or the prelude's as a builtin's).
#[cold]
#[inline(never)]
pub fn invoke_traced(c: &mut Ctx, me: *const MethodEntry, recv: Value, base: Stk, argc: u32, blk: Blk, flags: u32) -> RV {
    let Some(m) = c.me_ref(me) else { return invoke_untraced(c, me, recv, base, argc, blk, flags) };
    let (kind, name, owner) = (m.kind, m.name, m.owner);
    let (call_ev, ret_ev) = match kind {
        MKind::Iseq(is) => match c.iseq_ref(is) {
            Some(i) if i.traced => return invoke_untraced(c, me, recv, base, argc, blk, flags),
            Some(i) if i.info.file == PRELUDE_FILE => (EV_C_CALL, EV_C_RETURN),
            _ => (EV_CALL, EV_RETURN),
        },
        MKind::Bmethod(_) => (EV_CALL, EV_RETURN),
        MKind::Cfunc(..) | MKind::AttrReader(_) | MKind::AttrWriter(_) => {
            // Proc#call and friends are not reported (CRuby's opt_call)
            if owner == c.vm().c_proc.get() && matches!(sym_str(name).as_str(), "call" | "()" | "yield" | "===" | "[]") { return invoke_untraced(c, me, recv, base, argc, blk, flags); }
            (EV_C_CALL, EV_C_RETURN)
        }
        _ => return invoke_untraced(c, me, recv, base, argc, blk, flags),
    };
    if is_internal(c) { return invoke_untraced(c, me, recv, base, argc, blk, flags); }
    let k = c.vm().c_tracepoint.get();
    if k != NIL && (recv == k || real_class_of(c, recv) == k) {
        if let Some(&(_, d, b, e)) = TP_RB.iter().find(|x| x.0 == sym_str(name)) { return invoke_tp_rb(c, me, recv, base, argc, blk, flags, (d, b, e)); }
    }
    if trace_mask() & (call_ev | ret_ev) == 0 { return invoke_untraced(c, me, recv, base, argc, blk, flags); }
    // Class#new is not reported (CRuby 4.0's opt_new), its BasicObject#initialize is (here the default is not called)
    if me == c.vm().ids.c_new.get() {
        let r = invoke_untraced(c, me, recv, base, argc, blk, flags)?;
        let im = lookup_method(c, real_class_of(c, r), intern(b"initialize"));
        if im == c.vm().ids.bo_init.get() {
            let mut a = frame_arg(c, EV_C_CALL);
            let (mid, callee, klass) = method_of(c, im);
            a.mid = mid; a.callee = callee; a.klass = klass; a.self_ = r;
            if trace_mask() & EV_C_CALL != 0 { fire(c, a)?; }
            if trace_mask() & EV_C_RETURN != 0 { fire(c, Arg { ev: EV_C_RETURN, ..a })?; }
        }
        return Ok(r);
    }
    let mut a = frame_arg(c, call_ev);
    let (mid, callee, klass) = method_of(c, me);
    a.mid = mid; a.callee = callee; a.klass = klass; a.self_ = recv;
    let mut end_line = 0;
    if call_ev == EV_CALL {
        // a method's lines are its code's (define_method: its block's)
        let is = match kind { MKind::Iseq(is) => is, MKind::Bmethod(p) => match c.proc_data(p).blk.kind { BlockKind::Iseq(is) => is, _ => std::ptr::null() }, _ => std::ptr::null() };
        if let Some(i) = c.iseq_ref(is) { a.path = i.info.file; a.line = i.info.line; end_line = i.info.end_line; }
        a.frame = 0; a.iseq = is as usize;
    }
    if trace_mask() & call_ev != 0 { fire(c, a)?; }
    let r = invoke_untraced(c, me, recv, base, argc, blk, flags);
    let pending = call_ev == EV_C_CALL && RAISE_PENDING.with(|p| p.replace(false)) && r.is_err() && c.esc == EscKind::Raise;
    if trace_mask() & ret_ev != 0 {
        let mut b = a;
        b.ev = ret_ev;
        b.ret = match &r { Ok(v) => *v, Err(_) => NIL };
        if call_ev == EV_C_CALL { if let Some(f) = ruby_frame(c) { b.line = f.line(); } } else if end_line != 0 { b.line = end_line; }
        fire(c, b)?;
    }
    if pending { let e = c.esc_val; trace_raise(c, e); }
    r
}

// CRuby defines TracePoint's methods in Ruby (trace_point.rb of 4.0: def, body and end lines), so
// a hook sees their call / line / return events there
const TP_RB_FILE: &str = "<internal:trace_point>";
const TP_RB: [(&str, u32, u32, u32); 21] = [
    ("new", 96, 98, 99), ("inspect", 106, 107, 108), ("stat", 119, 120, 121), ("trace", 134, 136, 137), ("allow_reentry", 200, 202, 203),
    ("enable", 261, 263, 264), ("disable", 297, 299, 300), ("enabled?", 306, 307, 308), ("event", 313, 314, 315), ("lineno", 318, 319, 320),
    ("path", 323, 324, 325), ("parameters", 329, 330, 331), ("method_id", 334, 335, 336), ("callee_id", 339, 340, 341),
    ("defined_class", 375, 376, 377), ("binding", 383, 384, 385), ("self", 393, 394, 395), ("return_value", 398, 399, 400),
    ("raised_exception", 403, 404, 405), ("eval_script", 409, 410, 411), ("instruction_sequence", 417, 418, 419),
];
#[allow(clippy::too_many_arguments)]
fn invoke_tp_rb(c: &mut Ctx, me: *const MethodEntry, recv: Value, base: Stk, argc: u32, blk: Blk, flags: u32, (d, b, e): (u32, u32, u32)) -> RV {
    let (mid, callee, klass) = method_of(c, me);
    let a = Arg { ev: EV_CALL, path: TP_RB_FILE, line: d, mid, callee, klass, self_: recv, ret: NIL, exc: NIL, frame: 0, iseq: 0, lambda: false, script: NIL };
    let before = trace_mask();
    if before & EV_CALL != 0 { fire(c, a)?; }
    if trace_mask() & EV_LINE != 0 { fire(c, Arg { ev: EV_LINE, line: b, ..a })?; }
    let r = invoke_untraced(c, me, recv, base, argc, blk, flags);
    // a return event needs the trace points the method started with
    if before & trace_mask() & EV_RETURN != 0 { fire(c, Arg { ev: EV_RETURN, line: e, ret: match &r { Ok(v) => *v, Err(_) => NIL }, ..a })?; }
    r
}

/// vm::raise_exc: the raise event (an exception from its hook is dropped).
#[cold]
#[inline(never)]
pub fn trace_raise(c: &mut Ctx, e: Value) {
    if IN_HOOK.with(|h| h.get()) { return; }
    if trace_mask() & EV_C_RETURN != 0 && c.cur_frame().is_some_and(|f| f.kind == FK_CFUNC) { RAISE_PENDING.with(|p| p.set(true)); return; }
    let mut a = frame_arg(c, EV_RAISE);
    a.exc = e;
    let saved = (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v, c.errinfo);
    let _ = fire(c, a);
    (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v, c.errinfo) = saved;
}

/// fiber_main of a thread's root fiber: thread_begin / thread_end (No path, line 0).  An exception
/// from the hook ends the thread with it.
#[cold]
#[inline(never)]
pub fn trace_thread(c: &mut Ctx, ev: u32) -> Result<(), Esc> {
    let th = crate::builtins::thread::current_thread(c);
    let a = Arg { ev, path: "", line: 0, mid: None, callee: None, klass: NIL, self_: th, ret: NIL, exc: NIL, frame: 0, iseq: 0, lambda: false, script: NIL };
    let saved = (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v, c.errinfo);
    fire(c, a)?;
    (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v, c.errinfo) = saved;
    Ok(())
}

/// eval: the script_compiled event.
#[cold]
#[inline(never)]
pub fn trace_script_compiled(c: &mut Ctx, src: &[u8]) -> Result<(), Esc> {
    if trace_mask() & EV_SCRIPT_COMPILED == 0 { return Ok(()); }
    let mut a = frame_arg(c, EV_SCRIPT_COMPILED);
    a.script = c.str_new(src);
    fire(c, a)
}

// ---- TracePoint ---------------------------------------------------------------------------------

fn event_of(c: &mut Ctx, v: Value) -> Result<u32, Esc> {
    let s = if v.is_sym() { v } else if is_string(v) { Value::sym(intern(c.str_bytes(v))) } else {
        let cn = crate::builtins::kernel::class_name_for_err(c, v);
        if !respond_to(c, v, intern(b"to_sym"), true) { return type_err(c, format!("no implicit conversion of {} into Symbol", cn)); }
        let r = funcall(c, v, intern(b"to_sym"), &[], std::ptr::null())?;
        if !r.is_sym() { return type_err(c, format!("can't convert {} to Symbol ({}#to_sym gives {})", cn, cn, crate::builtins::kernel::class_name_for_err(c, r))); }
        r
    };
    let n = sym_str(s.as_sym());
    match EVENTS.iter().find(|e| e.1 == n) {
        Some(e) => Ok(e.0),
        None => match n.as_str() {
            "a_call" => Ok(EV_CALL | EV_B_CALL | EV_C_CALL),
            "a_return" => Ok(EV_RETURN | EV_B_RETURN | EV_C_RETURN),
            "fiber_switch" => Ok(0),
            _ => arg_err(c, format!("unknown event: {}", n)),
        },
    }
}

fn tp_new(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let mut ev = 0;
    for &v in a { ev |= event_of(c, v)?; }
    if a.is_empty() { ev = EV_ALL; }
    if blk.is_null() { return arg_err(c, "must be called with a block"); }
    let p = block_to_proc(c, blk)?;
    TRACE_COMPILE.store(true, std::sync::atomic::Ordering::Relaxed);
    let k = if is_class_or_module(slf) { slf } else { c.vm().c_tracepoint.get() };
    let t = alloc_data(c, k)?.unwrap();
    let d = rt_mut(c, t);
    d.events = ev; d.proc_ = p;
    Ok(t)
}
fn tp_s_trace(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let t = tp_new(c, slf, a, blk)?;
    enable(c, t, 0, 0, NIL)?;
    Ok(t)
}

fn enable(c: &mut Ctx, t: Value, target: usize, line: u32, thread: Value) -> Result<(), Esc> {
    let l = list(c);
    let d = rt_mut(c, t);
    d.enabled = true; d.target = target; d.target_line = line; d.target_thread = thread;
    // global hooks run first, the latest enabled first; then targeted ones, the latest first
    let pos = if target == 0 { 0 } else { c.ary(l).iter().position(|&x| rt(c, x).is_some_and(|d| d.target != 0)).unwrap_or(c.ary(l).len()) };
    let mut v: Vec<Value> = c.ary(l).iter().copied().filter(|&x| x != t).collect();
    v.insert(pos.min(v.len()), t);
    let na = c.ary_from_vec(v);
    c.vm().trace_list.set(na);
    update_mask(c);
    Ok(())
}
fn disable(c: &mut Ctx, t: Value) {
    let l = list(c);
    let v: Vec<Value> = c.ary(l).iter().copied().filter(|&x| x != t).collect();
    let na = c.ary_from_vec(v);
    c.vm().trace_list.set(na);
    let d = rt_mut(c, t);
    d.enabled = false; d.target = 0; d.target_line = 0; d.target_thread = NIL;
    update_mask(c);
}

/// The code a target: Method / UnboundMethod / Proc stands for.
fn target_iseq(c: &mut Ctx, v: Value) -> Result<usize, Esc> {
    let is = if is_method_obj(v) {
        let me = c.method_obj(v).me;
        match c.me_ref(me).map(|m| m.kind) {
            Some(MKind::Iseq(is)) => Some(is),
            Some(MKind::Bmethod(p)) => match c.proc_data(p).blk.kind { BlockKind::Iseq(is) => Some(is), _ => None },
            _ => None,
        }
    } else if is_proc(v) {
        match c.proc_data(v).blk.kind { BlockKind::Iseq(is) => Some(is), _ => None }
    } else { None };
    match is { Some(is) => Ok(is as usize), None => arg_err(c, "specified target is not supported") }
}

fn tp_enable(c: &mut Ctx, t: Value, a: &[Value], blk: Blk) -> RV {
    let kw = kw_arg(a);
    let pos = if kw.is_some() { &a[..a.len() - 1] } else { a };
    if !pos.is_empty() { return argc_err(c, pos.len(), 0, 0); }
    let get = |c: &mut Ctx, k: &str| -> Result<Option<Value>, Esc> { match kw { Some(h) => crate::hash::hash_get_opt(c, h, sym_value(k)), None => Ok(None) } };
    let target_v = get(c, "target")?.filter(|&v| v != NIL);
    let line_v = get(c, "target_line")?.filter(|&v| v != NIL);
    let thread_v = get(c, "target_thread")?;
    let was = rt(c, t).is_some_and(|d| d.enabled);
    let had_target = rt(c, t).is_some_and(|d| d.target != 0);
    if was && (target_v.is_some() || had_target) { return arg_err(c, "can't nest-enable a targeting TracePoint"); }
    let target = match target_v { Some(v) => target_iseq(c, v)?, None => 0 };
    let line = match line_v {
        Some(v) => {
            if target == 0 { return arg_err(c, "only target_line is specified"); }
            let n = crate::builtins::numeric_methods::num_to_long(c, v)?;
            if rt(c, t).map_or(0, |d| d.events) & EV_LINE == 0 { return arg_err(c, "target_line is specified, but line event is not specified"); }
            let (s, e) = c.iseq_ref(target as *const Iseq).map_or((0, 0), |i| (i.info.line, i.info.end_line));
            if n < s as i64 || n > e as i64 { return arg_err(c, "can not enable any hooks"); }
            n as u32
        }
        None => 0,
    };
    if target != 0 {
        // a Proc reports no method events, a method no block-only ones
        let ev = rt(c, t).map_or(0, |d| d.events);
        let kind = c.iseq_ref(target as *const Iseq).map(|i| i.kind);
        let ok = match kind { Some(IseqKind::Block) | Some(IseqKind::Lambda) => ev & (EV_LINE | EV_B_CALL | EV_B_RETURN | EV_RAISE | EV_RESCUE | EV_C_CALL | EV_C_RETURN) != 0, _ => ev & !(EV_THREAD_BEGIN | EV_THREAD_END | EV_SCRIPT_COMPILED) != 0 };
        if !ok { return arg_err(c, "can not enable any hooks"); }
    }
    let thread = match thread_v { Some(v) => v, None => if !blk.is_null() && target == 0 { current_thread(c) } else { NIL } };
    if was && blk.is_null() { return Ok(TRUE); }
    let saved = rt(c, t).map(|d| (d.target, d.target_line, d.target_thread));
    enable(c, t, target, line, thread)?;
    if blk.is_null() { return Ok(Value::bool(was)); }
    let r = yield_block(c, blk, &[]);
    if was { if let Some((tg, ln, th)) = saved { let d = rt_mut(c, t); d.target = tg; d.target_line = ln; d.target_thread = th; } }
    else { disable(c, t); }
    r
}
fn tp_disable(c: &mut Ctx, t: Value, _a: &[Value], blk: Blk) -> RV {
    let (was, targeted) = rt(c, t).map_or((false, false), |d| (d.enabled, d.target != 0));
    if blk.is_null() { if was { disable(c, t); } return Ok(Value::bool(was)); }
    if targeted { return arg_err(c, "can't disable a targeting TracePoint in a block"); }
    let th = rt(c, t).map_or(NIL, |d| d.target_thread);
    if was { disable(c, t); }
    let r = yield_block(c, blk, &[]);
    if was { enable(c, t, 0, 0, th)?; }
    r
}
fn tp_enabled_p(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(rt(c, t).is_some_and(|d| d.enabled))) }

fn cur(c: &mut Ctx, t: Value) -> Result<Arg, Esc> {
    match rt(c, t).and_then(|d| d.cur) { Some(a) => Ok(a), None => err(c, c.vm().c_runtime_error.get(), "access from outside") }
}
fn tp_event(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV { let a = cur(c, t)?; Ok(sym_value(ev_name(a.ev))) }
fn tp_lineno(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV { let a = cur(c, t)?; Ok(Value::fix(a.line as i64)) }
fn tp_path(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV { let a = cur(c, t)?; Ok(if a.path.is_empty() { NIL } else { c.str_new(a.path.as_bytes()) }) }
fn tp_method_id(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV { let a = cur(c, t)?; Ok(a.mid.map(Value::sym).unwrap_or(NIL)) }
fn tp_callee_id(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV { let a = cur(c, t)?; Ok(a.callee.map(Value::sym).unwrap_or(NIL)) }
fn tp_defined_class(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV { let a = cur(c, t)?; Ok(a.klass) }
fn tp_self(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV {
    let a = cur(c, t)?;
    Ok(a.self_)
}
fn tp_binding(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV {
    let a = cur(c, t)?;
    if a.frame == 0 || matches!(a.ev, EV_C_CALL | EV_C_RETURN) { return Ok(NIL); }
    crate::builtins::binding::binding_of_frame_at(c, a.frame)
}
fn tp_return_value(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV {
    let a = cur(c, t)?;
    if !matches!(a.ev, EV_RETURN | EV_C_RETURN | EV_B_RETURN) { return err(c, c.vm().c_runtime_error.get(), "not supported by this event"); }
    Ok(a.ret)
}
fn tp_raised_exception(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV {
    let a = cur(c, t)?;
    if !matches!(a.ev, EV_RAISE | EV_RESCUE) { return err(c, c.vm().c_runtime_error.get(), "not supported by this event"); }
    Ok(a.exc)
}
fn tp_eval_script(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV {
    let a = cur(c, t)?;
    if a.ev != EV_SCRIPT_COMPILED { return err(c, c.vm().c_runtime_error.get(), "not supported by this event"); }
    Ok(a.script)
}
fn tp_parameters(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV {
    let a = cur(c, t)?;
    match a.ev {
        EV_CALL | EV_RETURN | EV_B_CALL | EV_B_RETURN if a.iseq != 0 => {
            let lambda = !matches!(a.ev, EV_B_CALL | EV_B_RETURN) || a.lambda;
            Ok(crate::builtins::proc::params_list(c, a.iseq as *const Iseq, lambda))
        }
        EV_CALL | EV_RETURN | EV_B_CALL | EV_B_RETURN | EV_C_CALL | EV_C_RETURN => Ok(c.ary_new()),
        _ => err(c, c.vm().c_runtime_error.get(), "not supported by this event"),
    }
}
fn tp_inspect(c: &mut Ctx, t: Value, _a: &[Value], _b: Blk) -> RV {
    let s = match rt(c, t).and_then(|d| d.cur) {
        None => if rt(c, t).is_some_and(|d| d.enabled) { "#<TracePoint:enabled>".to_string() } else { "#<TracePoint:disabled>".to_string() },
        Some(a) => match a.ev {
            EV_LINE => format!("#<TracePoint:line {}:{}>", a.path, a.line),
            EV_CALL | EV_C_CALL | EV_RETURN | EV_C_RETURN => format!("#<TracePoint:{} '{}' {}:{}>", ev_name(a.ev), a.mid.map(sym_str).unwrap_or_default(), a.path, a.line),
            EV_THREAD_BEGIN | EV_THREAD_END => { let i = crate::builtins::kernel::inspect_str(c, a.self_)?; format!("#<TracePoint:{} {}>", ev_name(a.ev), i) }
            e => format!("#<TracePoint:{} {}:{}>", ev_name(e), a.path, a.line),
        },
    };
    Ok(c.str_new(s.as_bytes()))
}
fn tp_s_allow_reentry(c: &mut Ctx, _s: Value, _a: &[Value], blk: Blk) -> RV {
    if !IN_HOOK.with(|h| h.get()) { return err(c, c.vm().c_runtime_error.get(), "No need to allow reentrance."); }
    let was = REENTRY.with(|r| r.replace(true));
    let in_hook = IN_HOOK.with(|h| h.replace(false));
    let r = yield_block(c, blk, &[]);
    IN_HOOK.with(|h| h.set(in_hook));
    REENTRY.with(|r| r.set(was));
    r
}
fn tp_s_stat(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.hash_new()) }

/// Kernel#set_trace_func(proc or nil)
fn k_set_trace_func(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let l = list(c);
    let old: Vec<Value> = c.ary(l).iter().copied().filter(|&x| rt(c, x).is_some_and(|d| d.stf)).collect();
    for t in old { disable(c, t); }
    if a[0] == NIL { return Ok(NIL); }
    if !is_proc(a[0]) { return type_err(c, "trace_func needs to be Proc"); }
    let before = trace_mask();
    TRACE_COMPILE.store(true, std::sync::atomic::Ordering::Relaxed);
    init_class(c);
    let k = c.vm().c_tracepoint.get();
    let t = alloc_data(c, k)?.unwrap();
    let d = rt_mut(c, t);
    d.events = EV_LINE | EV_CLASS | EV_END | EV_CALL | EV_RETURN | EV_C_CALL | EV_C_RETURN | EV_RAISE;
    d.proc_ = a[0]; d.stf = true;
    enable(c, t, 0, 0, NIL)?;
    // its own c-return, which invoke_traced did not see coming
    if before & EV_CALLS == 0 {
        let mut ar = frame_arg(c, EV_C_RETURN);
        ar.mid = Some(intern(b"set_trace_func")); ar.callee = ar.mid; ar.klass = c.vm().m_kernel.get(); ar.ret = a[0];
        fire(c, ar)?;
    }
    Ok(a[0])
}

/// set_trace_func at boot; TracePoint itself is defined on its first reference ("saruby/tracepoint").
pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    define_module_function(c, vmr.m_kernel.get(), "set_trace_func", k_set_trace_func, 1);
    crate::builtins::lazy_const(c, vmr.c_object.get(), "TracePoint", "saruby/tracepoint");
}
pub fn init_class(c: &mut Ctx) {
    if c.vm().c_tracepoint.get() != NIL { return; }
    let k = define_class(c, "TracePoint", c.vm().c_object.get());
    class_set_alloc(c, k, AllocKind::None);
    c.vm().c_tracepoint.set(k);
    define_singleton_cfunc(c, k, "new", tp_new, -1);
    define_singleton_cfunc(c, k, "trace", tp_s_trace, -1);
    define_singleton_cfunc(c, k, "allow_reentry", tp_s_allow_reentry, 0);
    define_singleton_cfunc(c, k, "stat", tp_s_stat, 0);
    for (n, f, ar) in [("enable", tp_enable as CFunc, -1), ("disable", tp_disable, 0), ("enabled?", tp_enabled_p, 0), ("event", tp_event, 0),
                       ("lineno", tp_lineno, 0), ("path", tp_path, 0), ("method_id", tp_method_id, 0), ("callee_id", tp_callee_id, 0),
                       ("defined_class", tp_defined_class, 0), ("self", tp_self, 0), ("binding", tp_binding, 0), ("return_value", tp_return_value, 0),
                       ("raised_exception", tp_raised_exception, 0), ("eval_script", tp_eval_script, 0), ("parameters", tp_parameters, 0),
                       ("inspect", tp_inspect, 0)] {
        define_cfunc(c, k, n, f, ar);
    }
}
