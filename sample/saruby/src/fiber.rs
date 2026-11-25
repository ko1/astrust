// Fibers with CRuby's semantics (resume / yield / transfer / raise / kill), the switching of green
// threads (builtins/thread.rs), and Enumerator#next / #peek.  A fiber gets its machine stack, Ctx
// and value stack at its first run and gives them back when it ends or is collected; the machine
// and OS side is fiber_os.rs, this file is safe code but for the one call of the switch.
//
// GC: a Fiber is an ordinary object.  The current fiber is a root (CUR); a switched-out fiber's
// machine stack and contexts are traced from its Fiber object (RData::visit), so a suspended fiber
// nothing refers to is collected without running its ensure clauses, as in CRuby.
#![deny(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
use crate::fiber_os::{self as os, FiberCtx, NativeStack};
use crate::array::*;
use crate::heap::Global;
use crate::stack::*;
use crate::value::*;
use crate::vm::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

const VSTACK: usize = (1 << 18) + FRAME;   // the last FRAME slots cannot start a frame
/// Fiber starts between collections: at least this many, and as many as fibers held stacks after the last.
const STARTS_MIN: usize = 1024;
/// Ruby calls raise SystemStackError this close to the bottom of a fiber's machine stack.
const NATIVE_MARGIN: usize = 512 << 10;

pub struct FiberData {
    ctx: Option<FiberCtx>,          // None: not started, finished, or the main thread's root
    stack: Option<NativeStack>,
    vstack: Option<Stack>,
    sp: usize,                      // saved stack pointer while switched out (0: none)
    running: bool,                  // the CPU is on this fiber's stack
    main: bool,                     // the main thread's root fiber (its stack is the main stack)
    started: bool,
    done: bool,
    yielding: bool,
    prev: Value,                    // the fiber that resumed this one
    resuming: Value,                // the fiber this one resumed
    proc_: Value,
    xfer: Value,                    // a green thread's arguments, then its result
    exc: bool,                      // the result is an exception
    storage: Value,                 // Fiber[] / Fiber#storage: nil or a Hash
    locals: Value,                  // fiber-local Thread#[] (a non-root fiber's; roots use RThread's)
    owner: Value,                   // root fiber of the thread the fiber belongs to
    blocking: bool,
    killed: bool,                   // Fiber#kill was called (a second call returns false)
}
impl FiberData {
    fn new(proc_: Value) -> FiberData {
        FiberData { ctx: None, stack: None, vstack: None, sp: 0, running: false, main: false, started: false, done: false, yielding: false,
                    prev: NIL, resuming: NIL, proc_, xfer: NIL, exc: false,
                    storage: NIL, locals: NIL, owner: NIL, blocking: false, killed: false }
    }
    /// Becoming the running fiber: (saved stack pointer, stack top).
    fn enter(&mut self) -> (usize, usize) {
        self.running = true;
        (std::mem::take(&mut self.sp), self.stack.as_ref().map_or(0, |s| s.top()))
    }
    /// Being switched out: where the switch saves the stack pointer.
    fn suspend(&mut self) -> *mut usize { self.running = false; &raw mut self.sp }
}
impl RData for FiberData {
    fn visit(&mut self, m: &mut crate::gc::Visitor) {
        for v in [&mut self.proc_, &mut self.xfer, &mut self.prev, &mut self.resuming, &mut self.storage, &mut self.locals, &mut self.owner] { m.visit(v); }
        if let Some(c) = &mut self.ctx { if c.ptr() != crate::vm::scan_ctx() { visit_ctx(m, c.get()); } }
        if self.running || self.sp == 0 { return; }
        match &self.stack {
            Some(s) => m.visit_ambiguous(s.words(self.sp)),
            None if self.main => m.visit_ambiguous(os::main_stack_words(self.sp, crate::gc::stack_base(), crate::STACK_BYTES)),
            None => {}
        }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
impl Drop for FiberData {
    fn drop(&mut self) {
        if let Some(s) = self.vstack.take() { stack_release(s); STACKS.fetch_sub(1, Relaxed); }
    }
}

static CUR: Global = Global::new();          // the running fiber
static MAIN_ROOT: Global = Global::new();
static THREAD_ROOT: Global = Global::new();  // root fiber of the running green thread
static SCHED: Global = Global::new();        // fiber the scheduler runs in, while a green thread runs
static LAST_OUT: Global = Global::new();     // innermost fiber of the thread that just suspended
static ZOMBIE: Global = Global::new();       // a finished fiber whose stacks are released after the switch
static HAS_ZOMBIE: AtomicBool = AtomicBool::new(false);
static KILL: Global = Global::new();         // throw tag of Fiber#kill
static STACKS: AtomicUsize = AtomicUsize::new(0);        // fibers holding stacks
static STARTS: AtomicUsize = AtomicUsize::new(0);        // fiber starts since the GC numbered GC_SEEN
static GC_SEEN: AtomicUsize = AtomicUsize::new(0);
static STACKS_AT_GC: AtomicUsize = AtomicUsize::new(0);
static CUR_TOP: AtomicUsize = AtomicUsize::new(0);   // top of the running fiber's stack; 0: the main stack

fn fd(c: &Ctx, f: Value) -> &FiberData { c.data::<FiberData>(f).expect("a Fiber") }
fn fdm(c: &mut Ctx, f: Value) -> &mut FiberData { c.data_mut::<FiberData>(f).expect("a Fiber") }

/// Top (highest address) of the stack the CPU is running on.
pub fn current_stack_top() -> usize {
    match CUR_TOP.load(Relaxed) { 0 => crate::gc::stack_base(), t => t }
}

fn new_fiber(c: &mut Ctx, klass: Value, proc_: Value) -> Value {
    let mut d = FiberData::new(proc_);
    d.owner = THREAD_ROOT.get(c);
    c.alloc(klass, Body::Data(Box::new(d)))
}

/// Give a fiber its stacks and a first frame that runs fiber_main.
fn start(c: &mut Ctx, f: Value) -> Result<(), Esc> {
    collect_dead_fibers(c);
    let Some((mut stack, _)) = NativeStack::take() else {
        let k = c.vm().c_fiber_error.get();
        return err(c, k, "can't alloc machine stack to fiber");
    };
    let mut ctx = FiberCtx::new(stack.bottom() + NATIVE_MARGIN);
    let vs = stack_new(VSTACK);
    ctx.get().attach_stack(vs);
    let sp = stack.prepare(&mut ctx).expect("an unused stack");
    let d = fdm(c, f);
    d.started = true; d.sp = sp;
    d.stack = Some(stack); d.ctx = Some(ctx); d.vstack = Some(vs);
    STACKS.fetch_add(1, Relaxed);
    Ok(())
}

/// Stacks come back only when their fibers are collected: collect once fibers have started as
/// often as there were fibers with stacks after the last collection (amortized, like heap growth).
fn collect_dead_fibers(c: &mut Ctx) {
    let n = crate::gc::collections();
    if n != GC_SEEN.load(Relaxed) {
        GC_SEEN.store(n, Relaxed);
        STARTS.store(0, Relaxed);
        STACKS_AT_GC.store(STACKS.load(Relaxed), Relaxed);
    }
    if STARTS.fetch_add(1, Relaxed) < STACKS_AT_GC.load(Relaxed).max(STARTS_MIN) { return; }
    crate::gc::collect_auto(c);
}

/// Release what a finished fiber no longer needs (it cannot be done on its own stack).
#[inline(always)]
fn reap(c: &mut Ctx) { if HAS_ZOMBIE.load(Relaxed) { reap_zombie(c); } }
#[inline(never)]
fn reap_zombie(c: &mut Ctx) {
    HAS_ZOMBIE.store(false, Relaxed);
    let z = ZOMBIE.get(c);
    ZOMBIE.set(c, NIL);
    let d = fdm(c, z);
    d.stack = None; d.ctx = None; d.sp = 0;
    if let Some(s) = d.vstack.take() { stack_release(s); STACKS.fetch_sub(1, Relaxed); }
}

/// Switch to `to`, entered at (sp, top) by FiberData::enter, passing `v` (an exception if `exc`);
/// the running fiber's stack pointer goes to `save` (FiberData::suspend).  Returns what the
/// switch that comes back passes.
#[inline(always)]
fn leave(c: &mut Ctx, to: Value, (to_sp, top): (usize, usize), save: *mut usize, v: Value, exc: bool) -> (Value, bool) {
    debug_assert!(to_sp != 0);
    CUR.set(c, to);
    CUR_TOP.store(top, Relaxed);
    // SAFETY: to_sp is the stack pointer saved by the last switch out of `to` (or laid out by start),
    // taken from its FiberData, which owns that stack; `to` is reachable (CUR), so the stack stays
    // mapped; `save` points into the running fiber's FiberData and is written before the switch;
    // no borrow is held across it
    #[allow(unsafe_code)]
    let r = unsafe { os::switch(save, to_sp, v, exc) };
    reap(c);
    c.redef = crate::rt::redef_word();   // the others may have redefined an operator meanwhile
    r
}

/// Leave the running fiber `cur` for `to`, passing `v` (an exception to raise there if `exc`).
fn switch_to(c: &mut Ctx, cur: Value, to: Value, v: Value, exc: bool) -> Result<(Value, bool), Esc> {
    if !fd(c, to).started { start(c, to)?; }
    let e = fdm(c, to).enter();
    let save = fdm(c, cur).suspend();
    Ok(leave(c, to, e, save, v, exc))
}

/// What arrived at a switch back: a value, an exception, or Fiber#kill.
fn received(c: &mut Ctx, (v, exc): (Value, bool)) -> RV {
    if !exc { return Ok(v); }
    if v == KILL.get(c) { c.esc = EscKind::Throw; c.esc_tag_v = v; c.esc_val = NIL; return Err(Esc); }
    Err(raise_exc(c, v))
}

pub(crate) fn fiber_main(c: &mut Ctx, args: Value, exc: bool) -> ! {
    reap(c);
    c.redef = crate::rt::redef_word();
    let f = CUR.get(c);
    let mut top = Frame::empty(c.bottom());
    c.frame = &mut top.h;
    let p = fd(c, f).proc_;
    let thread = f == THREAD_ROOT.get(c);
    let r = if exc { received(c, (args, true)) } else {
        let argv: Vec<Value> = if is_array(args) { c.ary(args).to_vec() } else { vec![args] };
        let tb = if thread && trace_mask() & EV_THREAD_BEGIN != 0 { crate::builtins::tracepoint::trace_thread(c, EV_THREAD_BEGIN) } else { Ok(()) };
        let kw = is_array(args) && crate::heap::obj_flags(args) & crate::object::FL_KWHASH != 0;
        tb.and_then(|_| call_proc_kw(c, p, &argv, std::ptr::null(), kw))
    };
    std::hint::black_box(args);   // keeps argv's values on this stack
    // a thread's scheduler is closed at its end, before thread_end (thread_start_func_2); its error wins
    let r = if thread && crate::builtins::thread::schedulers_set() { thread_scheduler_end(c, r) } else { r };
    let r = if thread && trace_mask() & EV_THREAD_END != 0 { crate::builtins::tracepoint::trace_thread(c, EV_THREAD_END).and(r) } else { r };
    let kill = KILL.get(c);
    let (v, failed) = match r {
        Ok(v) => (v, false),
        Err(_) if c.esc == EscKind::Throw && c.esc_tag_v == kill => (NIL, false),
        Err(_) if c.esc == EscKind::Raise => (c.esc_val, true),
        Err(_) => { let k = c.vm().c_local_jump_error.get(); (make_exception_rb(c, k, "break from proc-closure"), true) }
    };
    c.esc = EscKind::None;
    c.frame = std::ptr::null_mut();
    let root = THREAD_ROOT.get(c);
    let d = fdm(c, f);
    d.done = true;
    if f == root { d.xfer = v; d.exc = failed; }   // a green thread's result (take_result)
    let to = return_fiber(c, f);
    ZOMBIE.set(c, f);
    HAS_ZOMBIE.store(true, Relaxed);
    let _ = switch_to(c, f, to, v, failed);
    unreachable!("a finished fiber was switched to");
}

#[inline(never)]
fn thread_scheduler_end(c: &mut Ctx, r: RV) -> RV {
    if crate::builtins::thread::thread_scheduler(c) == NIL { return r; }
    let (ek, ev) = (c.esc, c.esc_val);
    c.esc = EscKind::None;
    if let Err(e) = crate::builtins::thread::scheduler_finalize(c) { return Err(e); }
    c.esc = ek; c.esc_val = ev;
    r
}

/// Where control goes when `cur` ends (CRuby return_fiber(true); fiber_yield does the other case).
fn return_fiber(c: &mut Ctx, cur: Value) -> Value {
    let prev = fd(c, cur).prev;
    if prev != NIL {
        fdm(c, cur).prev = NIL;
        fdm(c, prev).resuming = NIL;
        return prev;
    }
    let root = THREAD_ROOT.get(c);
    if cur == root { return SCHED.get(c); }   // a green thread's root fiber ended
    let mut f = root;
    while fd(c, f).resuming != NIL { f = fd(c, f).resuming; }
    f
}

fn owner_of(c: &Ctx, f: Value) -> Value { let d = fd(c, f); if d.main { MAIN_ROOT.get(c) } else { d.owner } }
/// The root fiber of the thread `f` belongs to.
pub fn thread_root_of(c: &Ctx, f: Value) -> Value { owner_of(c, f) }
/// The root fiber of the running thread.
pub fn current_thread_root(c: &Ctx) -> Value { THREAD_ROOT.get(c) }

fn fiber_err<T>(c: &mut Ctx, msg: &str) -> Result<T, Esc> { let k = c.vm().c_fiber_error.get(); err(c, k, msg) }

/// Resume f passing `v`, or the arguments `a` when v is UNDEF.
fn resume_kw(c: &mut Ctx, f: Value, v: Value, a: &[Value], exc: bool) -> RV {
    let cur = CUR.get(c);
    let across = owner_of(c, f) != THREAD_ROOT.get(c);
    let d = fdm(c, f);
    let started = d.started;
    let e = if exc && !d.started { "cannot raise exception on unborn fiber" }
        else if d.done { "attempt to resume a terminated fiber" }
        else if f == cur { "attempt to resume the current fiber" }
        else if d.prev != NIL { "attempt to resume a resumed fiber (double resume)" }
        else if d.resuming != NIL { "attempt to resume a resuming fiber" }
        else if d.started && !d.yielding { "attempt to resume a transferring fiber" }
        else if across { "fiber called across threads" }
        else { "" };
    if !e.is_empty() { return fiber_err(c, e); }
    let d = if started && (v != UNDEF || a.len() <= 1) { d } else {
        let v2 = if v != UNDEF { v } else if started { args_value(c, a) } else { start_args(c, a) };
        if !started { start(c, f)?; }
        return resume_started(c, cur, f, v2, exc);
    };
    let v = if v != UNDEF { v } else { a.first().copied().unwrap_or(NIL) };
    d.prev = cur; d.yielding = false;
    let e = d.enter();
    let d = fdm(c, cur);
    d.resuming = f;
    let save = d.suspend();
    let r = leave(c, f, e, save, v, exc);
    received(c, r)
}

fn resume_started(c: &mut Ctx, cur: Value, f: Value, v: Value, exc: bool) -> RV {
    let d = fdm(c, f);
    d.prev = cur; d.yielding = false;
    let e = d.enter();
    let d = fdm(c, cur);
    d.resuming = f;
    let save = d.suspend();
    let r = leave(c, f, e, save, v, exc);
    received(c, r)
}

/// Switch into fiber f passing v; returns what it yields/returns.
pub fn resume(c: &mut Ctx, f: Value, v: Value) -> RV { resume_kw(c, f, v, &[], false) }

fn transfer_kw(c: &mut Ctx, f: Value, v: Value, exc: bool) -> RV {
    let d = fd(c, f);
    if d.resuming != NIL { return fiber_err(c, "attempt to transfer to a resuming fiber"); }
    if d.yielding { return fiber_err(c, "attempt to transfer to a yielding fiber"); }
    let cur = CUR.get(c);
    if f == cur { return received(c, (v, exc)); }
    if owner_of(c, f) != THREAD_ROOT.get(c) { return fiber_err(c, "fiber called across threads"); }
    if fd(c, f).done { return fiber_err(c, "dead fiber called"); }
    let r = switch_to(c, cur, f, v, exc)?;
    received(c, r)
}

/// Fiber.yield from the current fiber.
pub fn fiber_yield(c: &mut Ctx, v: Value) -> RV {
    let cur = CUR.get(c);
    let d = fdm(c, cur);
    let prev = d.prev;
    if prev == NIL { return fiber_err(c, "attempt to yield on a not resumed fiber"); }
    d.prev = NIL; d.yielding = true;
    let save = d.suspend();
    let d = fdm(c, prev);
    d.resuming = NIL;
    let e = d.enter();
    let r = leave(c, prev, e, save, v, false);
    received(c, r)
}

// ---- green threads (builtins/thread.rs): a thread is a fiber chain entered from the scheduler ----

pub fn new_thread_fiber(c: &mut Ctx, proc_: Value, args: Value, storage: Value) -> Value {
    let k = c.vm().c_fiber.get();
    let f = new_fiber(c, k, proc_);
    let d = fdm(c, f);
    d.xfer = args; d.owner = f; d.storage = storage; d.blocking = true;
    f
}
pub fn is_done(c: &Ctx, f: Value) -> bool { fd(c, f).done }
pub fn take_result(c: &mut Ctx, f: Value) -> (bool, Value) {
    let d = fdm(c, f);
    (std::mem::take(&mut d.exc), std::mem::replace(&mut d.xfer, NIL))
}
pub fn last_switched_out(c: &Ctx) -> Value { LAST_OUT.get(c) }
/// A thread that never ran: its fiber will not either.
pub fn discard(c: &mut Ctx, f: Value) { let d = fdm(c, f); if !d.started { d.done = true; } }
/// Run a thread (root fiber `root`, resuming at `inner`) until it suspends or ends.
pub fn thread_switch_in(c: &mut Ctx, root: Value, inner: Value) -> Result<(), Esc> {
    let cur = CUR.get(c);
    let (sched, troot) = (SCHED.get(c), THREAD_ROOT.get(c));
    SCHED.set(c, cur);
    THREAD_ROOT.set(c, root);
    let args = if fd(c, inner).started { NIL } else { std::mem::replace(&mut fdm(c, inner).xfer, NIL) };
    let r = switch_to(c, cur, inner, args, false);
    SCHED.set(c, sched);
    THREAD_ROOT.set(c, troot);
    r.map(|_| ())
}
/// Suspend the running thread (whatever fiber it is in) back to the scheduler.
pub fn thread_switch_out(c: &mut Ctx) {
    let cur = CUR.get(c);
    LAST_OUT.set(c, cur);
    let to = SCHED.get(c);
    let _ = switch_to(c, cur, to, NIL, false);
}

// ---- Fiber methods --------------------------------------------------------------------------

fn args_value(c: &mut Ctx, a: &[Value]) -> Value { match a.len() { 0 => NIL, 1 => a[0], _ => c.ary_new_from(a) } }
/// A fiber's first run gets its block's arguments as an Array.
fn pass_args(c: &mut Ctx, f: Value, a: &[Value]) -> Value { if fd(c, f).started { args_value(c, a) } else { start_args(c, a) } }
/// The block arguments of a fiber's (or thread's) start; FL_KWHASH on the Array: the last is keywords.
pub fn start_args(c: &mut Ctx, a: &[Value]) -> Value {
    let r = c.ary_new_from(a);
    if kw_arg(c, a).is_some() { crate::heap::obj_set_flags(r, crate::object::FL_KWHASH); }
    r
}

/// Fiber.new (builtins/fiber_methods.rs): storage is nil or a Hash already copied.
pub fn new_user_fiber(c: &mut Ctx, klass: Value, proc_: Value, blocking: bool, storage: Value) -> Value {
    let f = new_fiber(c, klass, proc_);
    let d = fdm(c, f);
    d.blocking = blocking; d.storage = storage;
    f
}
pub fn current(c: &Ctx) -> Value { CUR.get(c) }
pub fn storage(c: &Ctx, f: Value) -> Value { fd(c, f).storage }
pub fn set_storage(c: &mut Ctx, f: Value, h: Value) { fdm(c, f).storage = h; }
pub fn is_blocking(c: &Ctx, f: Value) -> bool { fd(c, f).blocking }
pub fn set_blocking(c: &mut Ctx, f: Value, b: bool) { fdm(c, f).blocking = b; }
/// The fiber's proc (NIL for a thread's root fiber, whose inspect shows no location).
pub fn first_proc(c: &Ctx, f: Value) -> Value { let d = fd(c, f); if d.main || d.owner == f { NIL } else { d.proc_ } }
/// CRuby's fiber_status_name, plus whether it resumed another fiber.
pub fn status(c: &Ctx, f: Value) -> (&'static str, bool) {
    let d = fd(c, f);
    let s = if d.done { "terminated" } else if !d.started { "created" } else if d.running { "resumed" } else { "suspended" };
    (s, d.resuming != NIL)
}
/// A thread's root fiber (Thread#[] of those lives in the Thread).
pub fn is_root(c: &Ctx, f: Value) -> bool { let d = fd(c, f); d.main || d.owner == f }
/// Thread#[] storage of a non-root fiber, made on first use.
pub fn locals(c: &mut Ctx, f: Value) -> Value {
    let l = fd(c, f).locals;
    if l != NIL { return l; }
    let h = c.hash_new();
    fdm(c, f).locals = h;
    h
}
/// The main thread's running fiber while a green thread runs (the scheduler's), else the current one.
pub fn main_thread_fiber(c: &Ctx) -> Value { if THREAD_ROOT.get(c) == MAIN_ROOT.get(c) { CUR.get(c) } else { SCHED.get(c) } }
fn f_resume(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { resume_kw(c, slf, UNDEF, a, false) }
fn f_transfer(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let v = pass_args(c, slf, a); transfer_kw(c, slf, v, false) }
fn f_s_yield(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { let v = args_value(c, a); fiber_yield(c, v) }
fn f_alive(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(!fd(c, slf).done)) }
fn f_s_current(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(CUR.get(c)) }
/// Fiber#raise: resume a yielding fiber (transfer to a transferring one) raising there.
fn f_raise(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let e = crate::builtins::kernel::exception_setup(c, a)?;
    raise_in(c, slf, e)
}
/// CRuby fiber_raise: a fiber that resumed another passes it on to that one.
fn raise_in(c: &mut Ctx, f: Value, e: Value) -> RV {
    if f == CUR.get(c) { return received(c, (e, true)); }
    let d = fd(c, f);
    if d.resuming != NIL { let r = d.resuming; return raise_in(c, r, e); }
    if d.started && !d.done && !d.running && !d.yielding { transfer_kw(c, f, e, true) } else { resume_kw(c, f, e, &[], true) }
}
/// Fiber#backtrace(start = 0, length = nil) / #backtrace_locations: the running fiber's frames ([] for
/// a terminated one; a suspended fiber's frames are not walked: [] too).
fn f_backtrace_any(c: &mut Ctx, slf: Value, a: &[Value], locs: bool) -> RV {
    if slf != CUR.get(c) { return Ok(c.ary_new()); }
    let (start, len) = if a.is_empty() { (0, None) } else {
        let Some(r) = crate::builtins::exception::caller_range(c, a, |c| backtrace_lines_n(c, 0, None).map_or(0, |v| v.len()))? else { return Ok(NIL) };
        r
    };
    let Some(lines) = backtrace_lines_n(c, start, len) else { return Ok(NIL) };
    if locs { return Ok(crate::builtins::exception::locations_from_lines(c, lines)); }
    let v: Vec<Value> = lines.into_iter().map(|l| c.str_new(l.as_bytes())).collect();
    Ok(c.ary_from_vec(v))
}
/// The frames of suspended fiber `f`, read through its own context (None: it has none, e.g. the main root).
pub fn suspended_backtrace(c: &mut Ctx, f: Value, skip: usize, n: Option<usize>) -> Option<Option<Vec<BtLine>>> {
    if f == CUR.get(c) || fd(c, f).running { return None; }
    let mut ctx = fdm(c, f).ctx.take()?;
    let r = backtrace_locs_n(ctx.get(), skip, n);
    fdm(c, f).ctx = Some(ctx);
    Some(r)
}
fn f_backtrace(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { f_backtrace_any(c, slf, a, false) }
fn f_backtrace_locations(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { f_backtrace_any(c, slf, a, true) }
/// Fiber#kill: unwind the fiber (its ensure clauses run) and let it end.
fn f_kill(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let d = fdm(c, slf);
    if d.killed { return Ok(FALSE); }
    d.killed = true;
    if d.done { return Ok(slf); }
    if !d.started { fdm(c, slf).done = true; return Ok(slf); }
    let kill = KILL.get(c);
    if slf == CUR.get(c) { return received(c, (kill, true)); }
    raise_in(c, slf, kill)?;
    Ok(slf)
}

// ---- Enumerator#next / #peek ----------------------------------------------------------------

/// next_ii: hand the packed values to #next, then give yield the value #feed left.
fn enum_feed(c: &mut Ctx, e: Value, args: &[Value], _b: Blk, _o: Blk) -> RV {
    let v = c.ary_new_from(args);
    fiber_yield(c, v)?;
    let en = c.data_mut::<REnum>(e).expect("an Enumerator");
    let f = std::mem::replace(&mut en.feed, UNDEF);
    Ok(if f == UNDEF { NIL } else { f })
}
/// next_i: the iteration's result goes into the StopIteration kept for #next.
fn enum_body(c: &mut Ctx, e: Value, _args: &[Value], _b: Blk, _o: Blk) -> RV {
    let blk = Block::cfunc(c, enum_feed, e);
    let r = crate::builtins::enumerator::enum_each_with(c, e, Blk::of(&blk))?;
    let k = c.vm().c_stop_iteration.get();
    let ex = make_exception_rb(c, k, "iteration reached an end");
    ivar_set_raw(c, ex, crate::symbol::intern(b"result"), r);
    c.data_mut::<REnum>(e).expect("an Enumerator").stop = ex;
    Ok(UNDEF)
}

use crate::builtins::enumerator::REnum;
fn renum(c: &Ctx, e: Value) -> &REnum { c.data::<REnum>(e).expect("an Enumerator") }

fn enum_fiber(c: &mut Ctx, e: Value) -> Value {
    let blk = Block::cfunc(c, enum_body, e);
    let pk = c.vm().c_proc.get();
    let p = c.alloc(pk, Body::Proc(Box::new(ProcData { blk, is_lambda: false, from_method: NIL })));
    c.proc_data_mut(p).blk.proc_val = p;
    let fk = c.vm().c_fiber.get();
    let f = new_fiber(c, fk, p);
    // a copy of the storage of the fiber that calls next (as Fiber.new)
    let st = storage(c, CUR.get(c));
    if st != NIL { let st = crate::builtins::object::obj_dup(c, st, false, None).unwrap_or(NIL); fdm(c, f).storage = st; }
    let en = c.data_mut::<REnum>(e).expect("an Enumerator");
    en.fiber = f; en.look = UNDEF;
    f
}

/// get_next_values: the next yielded values (an Array); a fiber ended by an exception starts over.
pub fn enum_next_values(c: &mut Ctx, e: Value) -> RV {
    let stop = renum(c, e).stop;
    if stop != NIL {
        let res = ivar_get(c, stop, crate::symbol::intern(b"result"));
        let m = ivar_get(c, stop, crate::symbol::s::mesg);
        let m = if is_string(m) { c.str_copy(m) } else { m };
        let k = c.vm().c_stop_iteration.get();
        let ex = crate::vm::new_object(c, k);
        ivar_set_raw(c, ex, crate::symbol::s::mesg, m);
        ivar_set_raw(c, ex, crate::symbol::s::cause, stop);
        ivar_set_raw(c, ex, crate::symbol::intern(b"result"), res);
        return Err(raise_exc(c, ex));
    }
    let f = renum(c, e).fiber;
    let f = if f == NIL || fd(c, f).done { enum_fiber(c, e) } else { f };
    let v = resume(c, f, NIL)?;
    let stop = renum(c, e).stop;
    if stop != NIL {
        let en = c.data_mut::<REnum>(e).expect("an Enumerator");
        en.fiber = NIL; en.look = UNDEF; en.feed = UNDEF;
        return Err(raise_exc(c, stop));
    }
    Ok(v)
}
/// Enumerator `e` is between #next calls (CRuby's e->fib, cleared when the iteration ended).
pub fn enum_fiber_live(c: &Ctx, e: Value) -> bool { let f = renum(c, e).fiber; f != NIL && !fd(c, f).done }

pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let k = define_class(c, "Fiber", vmr.c_object.get());
    class_set_alloc(c, k, AllocKind::None);
    vmr.c_fiber.set(k);
    define_singleton_cfunc(c, k, "yield", f_s_yield, -1);
    define_singleton_cfunc(c, k, "current", f_s_current, 0);
    define_cfunc(c, k, "resume", f_resume, -1);
    define_cfunc(c, k, "transfer", f_transfer, -1);
    define_cfunc(c, k, "raise", f_raise, -1);
    define_cfunc(c, k, "kill", f_kill, 0);
    define_cfunc(c, k, "alive?", f_alive, 0);
    define_cfunc(c, k, "backtrace", f_backtrace, -1);
    define_cfunc(c, k, "backtrace_locations", f_backtrace_locations, -1);
    let mut d = FiberData::new(NIL);
    d.main = true; d.started = true; d.running = true; d.blocking = true;
    let root = c.alloc(k, Body::Data(Box::new(d)));
    MAIN_ROOT.set_vm(vmr, root);
    THREAD_ROOT.set_vm(vmr, root);
    CUR.set_vm(vmr, root);
    let tag = c.alloc(vmr.c_array.get(), Body::Array(crate::avec::AVec::new()));
    KILL.set_vm(vmr, tag);
}
