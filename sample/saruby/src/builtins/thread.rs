// Green threads: each Thread is a fiber chain switched by the main thread's scheduler at
// blocking points (join, Queue, Mutex, ConditionVariable, sleep, pipe reads, Thread.pass).
// There is no preemption: a thread runs until it blocks or ends.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{is_frozen, Global};
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::hash::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};
use crate::builtins::blop::{self, Id, Op, Res};
use crate::builtins::sys::{self, PollFd};

pub struct RThread {
    fiber: Value, inner: Value, started: bool, dead: bool, value: Value, exc: Value, locals: Value, tvars: Value, name: Value,
    blop: Option<Id>,   // the op it is parked on (None: runnable when switched out)
    waiting: bool, wake: bool, pending: Value, killed: bool,
    report: Value, abort: bool, loc: String,
    start: Value,   // [proc, args] until the first run makes the fiber (stacks only for threads that run)
}
impl RData for RThread {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { for v in [&mut self.fiber, &mut self.inner, &mut self.value, &mut self.exc, &mut self.locals, &mut self.tvars, &mut self.name, &mut self.pending, &mut self.report, &mut self.start] { m.visit(v); } }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
impl RThread {
    fn blank(locals: Value, tvars: Value) -> RThread {
        RThread { fiber: NIL, inner: NIL, started: false, dead: false, value: NIL, exc: NIL, locals, tvars, name: NIL,
            blop: None, waiting: false, wake: false, pending: NIL, killed: false, report: UNDEF, abort: false, loc: String::new(), start: NIL }
    }
    fn interrupted(&self) -> bool { self.pending != NIL || self.killed }
    /// The scheduler should switch it in: not started, yielded, its op completed, or interrupted.
    fn runnable(&self, c: &Ctx) -> bool { !self.dead && (!self.started || self.blop.is_none_or(|id| blop::is_done(c, id)) || self.interrupted()) }
}
static LIST: Global = Global::new();      // Array of live non-main threads
static MAIN: Global = Global::new();
static CURRENT: Global = Global::new();
static C_KILL: Global = Global::new();    // internal exception unwinding a killed thread
static C_FATAL: Global = Global::new();   // the unnamed-constant `fatal` exception class
static ABORT_ALL: AtomicBool = AtomicBool::new(false);
static REPORT_ALL: AtomicBool = AtomicBool::new(true);
/// Thread.ignore_deadlock: a deadlock waits (for a signal) instead of raising fatal.
pub static IGNORE_DEADLOCK: AtomicBool = AtomicBool::new(false);
static FORKED: AtomicBool = AtomicBool::new(false);   // a forked child whose main thread was a green thread

fn th(c: &Ctx, v: Value) -> &RThread { c.data::<RThread>(v).expect("a Thread") }
fn th_mut(c: &mut Ctx, v: Value) -> &mut RThread { c.data_mut::<RThread>(v).expect("a Thread") }
fn thr(c: &mut Ctx, v: Value) -> Result<(), Esc> { if c.data::<RThread>(v).is_some() { Ok(()) } else { type_err(c, "wrong argument type (expected Thread)") } }
fn cur(c: &Ctx) -> Value { CURRENT.get(c) }
fn is_main(c: &Ctx) -> bool { CURRENT.get(c) == MAIN.get(c) }
/// Other threads exist (or we are inside one): blocking calls must go through the scheduler.
pub fn active(c: &Ctx) -> bool { !is_main(c) || c.ary_len(LIST.get(c)) > 0 }
fn new_thread(c: &mut Ctx, klass: Value) -> Value {
    let l = c.hash_new();
    let t = c.hash_new();
    c.alloc_with(klass, move || Body::Data(Box::new(RThread::blank(l, t))))
}
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let k = c.vm().c_thread;
    if k != NIL && crate::builtins::inherits(klass, k) { return Ok(Some(new_thread(c, klass))); }
    Ok(None)
}
fn list_remove(c: &mut Ctx, t: Value) { let l = LIST.get(c); c.ary_mut(l).retain(|&x| x != t); }

fn check_interrupt(c: &mut Ctx, t: Value) -> Result<(), Esc> {
    let r = th_mut(c, t);
    if r.pending != NIL { let e = r.pending; r.pending = NIL; return Err(raise_exc(c, e)); }
    if r.killed { r.killed = false; let e = make_exception(C_KILL.get(c), "killed"); return Err(raise_exc(c, e)); }
    Ok(())
}
/// Make a parked thread runnable (an interrupt or a wakeup was posted for it).
fn kick(c: &mut Ctx, t: Value) { if let Some(id) = th(c, t).blop { blop::interrupt(c, id, blop::ECANCELED); } }
/// A thread's root fiber returned: record value / exception.
fn finish_thread(c: &mut Ctx, t: Value) -> Result<(), Esc> {
    let f = th(c, t).fiber;
    let (is_err, v) = crate::fiber::take_result(c, f);
    let r = th_mut(c, t);
    r.dead = true;
    list_remove(c, t);
    release_locks(c, t);
    blop::notify(c, t.raw());
    if FORKED.load(Relaxed) && t == MAIN.get(c) { forked_main_exit(c, is_err, v); }
    if !is_err { th_mut(c, t).value = v; return Ok(()); }
    if is_kind_of(v, C_KILL.get(c)) { return Ok(()); }
    if is_kind_of(v, c.vm().c_system_exit) { return Err(raise_exc(c, v)); }
    let r = th_mut(c, t);
    r.exc = v;
    let report = if r.report == UNDEF { REPORT_ALL.load(Relaxed) } else { r.report.truthy() };
    if report {
        let head = format!("{} terminated with exception (report_on_exception is true):\n", inspect_thread(c, t, "run"));
        let msg = crate::builtins::exception::full_message(c, v, false).unwrap_or_default();
        err_write(format!("{}{}", head, msg).as_bytes());
    }
    if th(c, t).abort || ABORT_ALL.load(Relaxed) { return Err(raise_exc(c, v)); }
    Ok(())
}
/// A child forked from a green thread ends when that thread does (it is the child's main thread).
fn forked_main_exit(c: &mut Ctx, is_err: bool, v: Value) -> ! {
    let r = if !is_err || is_kind_of(v, C_KILL.get(c)) { Ok(NIL) } else { Err(raise_exc(c, v)) };
    crate::builtins::process::child_exit(c, r)
}
/// Give every runnable thread one turn; true when any ran.
fn run_pass(c: &mut Ctx) -> Result<bool, Esc> {
    let v: Vec<Value> = c.ary(LIST.get(c)).iter().copied().filter(|&t| th(c, t).runnable(c)).collect();
    if v.is_empty() { return Ok(false); }
    let snap = c.ary_from_vec(v);   // a thread killed during the pass leaves LIST: this keeps it alive
    let main = MAIN.get(c);
    let mut i = 0;
    while let Some(&t) = c.ary(snap).get(i) {
        i += 1;
        if th(c, t).dead { continue; }   // runnable at the snapshot, and an op does not un-complete
        let r = th_mut(c, t);
        r.started = true;
        if r.fiber == NIL {
            let st = std::mem::replace(&mut r.start, NIL);
            let (p, args, storage) = (c.ary_get(st, 0), c.ary_get(st, 1), c.ary_get(st, 2));
            let f = crate::fiber::new_thread_fiber(c, p, args, storage);
            let r = th_mut(c, t);
            r.fiber = f; r.inner = f;
        }
        let r = th(c, t);
        let (f, inner) = (r.fiber, r.inner);
        CURRENT.set(c, t);
        let r = crate::fiber::thread_switch_in(c, f, inner);
        CURRENT.set(c, main);
        r?;
        let f = th(c, t).fiber;
        if crate::fiber::is_done(c, f) { finish_thread(c, t)?; } else { let o = crate::fiber::last_switched_out(c); th_mut(c, t).inner = o; }
    }
    Ok(true)
}
/// Duration of a Ruby time interval (huge / NaN values mean "practically forever").
fn secs(f: f64) -> Duration { Duration::from_secs_f64(if f.is_finite() { f.clamp(0.0, 1e9) } else { 1e9 }) }

/// The one suspension point (koruby's korb_blop_wait): park the current green thread until `op`
/// completes, its deadline passes (Err(ETIMEDOUT)) or the thread is interrupted (Err(ECANCELED)).
/// The op comes back with its result; a pending Thread#raise / #kill is delivered on return
/// (a completion that raced it is dropped, as in CRuby).
pub fn wait(c: &mut Ctx, op: Op, deadline: Option<Instant>, deadlockable: bool) -> Result<(Option<Op>, Res), Esc> {
    let me = cur(c);
    check_interrupt(c, me)?;
    let id = blop::submit(c, op, deadline, deadlockable);
    let r = park(c, me, id);
    let out = blop::take(c, id);
    // a notify_one this thread got but will not act on goes to the next waiter
    if r.is_err() || th(c, me).interrupted() { if let (Some(Op::Wake(k)), Ok(_)) = &out { blop::notify_one(c, *k); } }
    r?;
    check_interrupt(c, me)?;
    Ok(out)
}
fn park(c: &mut Ctx, me: Value, id: Id) -> Result<(), Esc> {
    if !is_main(c) {
        while !blop::is_done(c, id) && !th(c, me).interrupted() {
            let r = th_mut(c, me);
            r.blop = Some(id); r.waiting = true;
            crate::fiber::thread_switch_out(c);
            let r = th_mut(c, me);
            r.blop = None; r.waiting = false;
        }
        return Ok(());
    }
    th_mut(c, me).waiting = true;
    let r = schedule(c, me, id);
    th_mut(c, me).waiting = false;
    r
}
/// The main thread's loop while it waits on `id`: trap handlers, the runnable threads, and the
/// engine's pump (the only place the interpreter sleeps) when nobody can run.
fn schedule(c: &mut Ctx, me: Value, id: Id) -> Result<(), Esc> {
    loop {
        if blop::is_done(c, id) || th(c, me).interrupted() { return Ok(()); }
        if crate::builtins::process::signals_pending() {
            crate::builtins::process::run_pending_signals(c)?;
            if blop::is_done(c, id) || th(c, me).interrupted() { return Ok(()); }
        }
        if run_pass(c)? { blop::pump(c, false); continue; }
        if !blop::can_finish_alone(c) && !IGNORE_DEADLOCK.load(Relaxed) { return err(c, C_FATAL.get(c), "No live threads left. Deadlock?"); }
        blop::pump(c, true);
    }
}
/// Block the current thread until cond holds (Ok(true)) or the deadline passes (Ok(false)).
/// `key`: what blop::notify ends the wait (an object's address; None: only time or an interrupt does).
pub fn block_on(c: &mut Ctx, mut cond: impl FnMut(&mut Ctx) -> Result<bool, Esc>, key: Option<u64>, deadline: Option<Instant>, deadlockable: bool) -> Result<bool, Esc> {
    let me = cur(c);
    loop {
        check_interrupt(c, me)?;
        if cond(c)? { return Ok(true); }
        if deadline.is_some_and(|d| Instant::now() >= d) { return Ok(false); }
        let op = match key { Some(k) => Op::Wake(k), None => Op::Timer };
        let _ = wait(c, op, deadline, deadlockable)?;   // any wake re-checks cond
    }
}
/// Kernel#sleep when threads are in play.
pub fn sleep(c: &mut Ctx, dur: Option<f64>) -> Result<(), Esc> { sleep_as(c, dur, false) }
/// `deadlockable`: a sleep forever counts for deadlock detection (Mutex#sleep does, Kernel#sleep does not, as CRuby).
fn sleep_as(c: &mut Ctx, dur: Option<f64>, deadlockable: bool) -> Result<(), Esc> {
    let me = cur(c);
    th_mut(c, me).wake = false;
    let deadline = dur.map(|d| Instant::now() + secs(d));
    block_on(c, |c| Ok(std::mem::take(&mut th_mut(c, me).wake)), None, deadline, deadlockable)?;
    Ok(())
}
/// Sleep `d` through the scheduler (back-off of a retried call that has no fd to wait on).
pub fn sleep_for(c: &mut Ctx, d: Duration) -> Result<(), Esc> {
    let deadline = Instant::now() + d;
    block_on(c, |_| Ok(false), None, Some(deadline), false)?;
    Ok(())
}
fn closed_in_another_thread(c: &mut Ctx) -> Esc {
    let k = c.vm().c_io_error;
    let e = make_exception(k, "stream closed in another thread");
    raise_exc(c, e)
}
/// Wait for readiness of `fds` (poll(2) events): Some(fds with revents) or None at the deadline.
/// IOError if another thread closes one of them.
pub fn poll_wait(c: &mut Ctx, mut fds: Vec<PollFd>, deadline: Option<Instant>) -> Result<Option<Vec<PollFd>>, Esc> {
    if sys::poll(&mut fds, 0) > 0 { return Ok(Some(fds)); }
    loop {
        if deadline.is_some_and(|d| Instant::now() >= d) { return Ok(None); }
        let (op, r) = wait(c, Op::Poll(fds), deadline, false)?;
        let Some(Op::Poll(f)) = op else { unreachable!() };
        match r {
            Ok(_) => return Ok(Some(f)),
            Err(blop::ETIMEDOUT) => return Ok(None),
            Err(blop::EBADF) => return Err(closed_in_another_thread(c)),
            Err(_) => fds = f,   // Thread#wakeup: wait again
        }
    }
}
/// Wait until fd is readable, letting other threads run meanwhile.
pub fn wait_readable(c: &mut Ctx, fd: i32) -> Result<(), Esc> { poll_wait(c, vec![PollFd { fd, events: sys::POLLIN, revents: 0 }], None)?; Ok(()) }
/// Wait until fd is writable, letting other threads run meanwhile.
pub fn wait_writable(c: &mut Ctx, fd: i32) -> Result<(), Esc> { poll_wait(c, vec![PollFd { fd, events: sys::POLLOUT, revents: 0 }], None)?; Ok(()) }
/// One read(2) of fd into `buf` once it is readable (a READ blop): (buf, bytes valid in it or errno).
pub fn read_op(c: &mut Ctx, fd: i32, mut buf: Vec<u8>) -> Result<(Vec<u8>, Result<usize, i64>), Esc> {
    loop {
        let (op, r) = wait(c, Op::Read { fd, buf }, None, false)?;
        let Some(Op::Read { buf: b, .. }) = op else { unreachable!() };
        match r {
            Err(blop::EBADF) => return Err(closed_in_another_thread(c)),
            Err(blop::ECANCELED) => buf = b,   // Thread#wakeup
            r => return Ok((b, r)),
        }
    }
}
/// One write(2) of (a prefix of) `buf` once fd is writable (a WRITE blop): bytes written or errno.
pub fn write_op(c: &mut Ctx, fd: i32, mut buf: Vec<u8>) -> Result<Result<usize, i64>, Esc> {
    loop {
        let (op, r) = wait(c, Op::Write { fd, buf }, None, false)?;
        let Some(Op::Write { buf: b, .. }) = op else { unreachable!() };
        match r {
            Err(blop::EBADF) => return Err(closed_in_another_thread(c)),
            Err(blop::ECANCELED) => buf = b,
            r => return Ok(r),
        }
    }
}
/// Forked child: only the forking thread survives, and it is the child's main thread (as CRuby).
pub fn after_fork_child(c: &mut Ctx) {
    blop::after_fork(c);
    let me = cur(c);
    let l = LIST.get(c);
    let others: Vec<Value> = c.ary(l).iter().copied().filter(|&t| t != me).collect();
    for t in others { let r = th_mut(c, t); r.dead = true; r.blop = None; r.waiting = false; }
    c.ary_mut(l).clear();
    let main = MAIN.get(c);
    if me != main {
        let r = th_mut(c, main); r.dead = true; r.waiting = false;
        th_mut(c, me).blop = None;
        MAIN.set(c, me);
        FORKED.store(true, Relaxed);
    }
}

// ---- Thread ---------------------------------------------------------------------------------

fn status_str(c: &Ctx, t: Value) -> &'static str { let r = th(c, t); if r.dead { "dead" } else if t == cur(c) { "run" } else if r.waiting || !r.started { if r.started { "sleep" } else { "run" } } else { "run" } }
fn inspect_thread(c: &Ctx, t: Value, status: &str) -> String {
    let r = th(c, t);
    let name = if r.name != NIL { format!("@{}", String::from_utf8_lossy(c.str_bytes(r.name))) } else { String::new() };
    let loc = if r.loc.is_empty() { String::new() } else { format!(" {}", r.loc) };
    format!("#<{}:0x{:016x}{}{} {}>", class_path(real_class_of(t)), t.raw(), name, loc, status)
}
fn th_s_new(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return err(c, c.vm().c_thread_error, "must be called with a block"); }
    let p = block_to_proc(c, b)?;
    let t = new_thread(c, slf);
    let args = c.ary_new_from(a);
    // the new thread gets a copy of the creating fiber's storage (rb_fiber_inherit_storage)
    let storage = crate::builtins::fiber_methods::inherited_storage(c)?;
    let start = c.ary_new_from(&[p, args, storage]);
    let loc = c.caller_frame().map(|f| format!("{}:{}", frame_file(f), f.line())).unwrap_or_default();
    let r = th_mut(c, t);
    r.start = start; r.loc = loc;
    let l = LIST.get(c);
    c.ary_push(l, t);
    Ok(t)
}
fn th_join(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    thr(c, slf)?;
    if slf == cur(c) { return err(c, c.vm().c_thread_error, "Target thread must not be current thread"); }
    if slf == MAIN.get(c) { return err(c, c.vm().c_thread_error, "Target thread must not be main thread"); }
    let deadline = match a.first() { Some(&t) if t != NIL => { let f = crate::builtins::numeric_methods::to_f(c, t)?; Some(Instant::now() + secs(f)) } _ => None };
    if !block_on(c, |c| Ok(th(c, slf).dead), Some(slf.raw()), deadline, true)? { return Ok(NIL); }
    let e = th(c, slf).exc;
    if e != NIL { return Err(raise_exc(c, e)); }
    Ok(slf)
}
fn th_value(c: &mut Ctx, slf: Value, _a: &[Value], b: *const Block) -> RV { th_join(c, slf, &[], b)?; Ok(th(c, slf).value) }
fn th_current(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(cur(c)) }
fn th_main(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(MAIN.get(c)) }
fn th_list(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let mut v = vec![MAIN.get(c)];
    v.extend(c.ary(LIST.get(c)).iter().copied().filter(|&t| !th(c, t).dead));
    Ok(c.ary_from_vec(v))
}
fn th_pass(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    if is_main(c) { run_pass(c)?; blop::pump(c, false); return Ok(NIL); }
    let me = cur(c);
    check_interrupt(c, me)?;
    crate::fiber::thread_switch_out(c);
    check_interrupt(c, me)?;
    Ok(NIL)
}
fn th_stop(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let me = cur(c);
    th_mut(c, me).wake = false;
    block_on(c, |c| Ok(std::mem::take(&mut th_mut(c, me).wake)), None, None, true)?;
    Ok(NIL)
}
fn exit_exception(c: &mut Ctx) -> Value {
    let e = make_exception(c.vm().c_system_exit, "exit");
    ivar_set_raw(e, crate::symbol::s::status, Value::fix(0));
    e
}
fn kill_thread(c: &mut Ctx, t: Value) -> RV {
    thr(c, t)?;
    if th(c, t).dead { return Ok(t); }
    if t == cur(c) {
        if is_main(c) { let e = exit_exception(c); return Err(raise_exc(c, e)); }
        let e = make_exception(C_KILL.get(c), "killed");
        return Err(raise_exc(c, e));
    }
    if t == MAIN.get(c) { let e = exit_exception(c); th_mut(c, t).pending = e; return Ok(t); }
    if !th(c, t).started {
        let r = th_mut(c, t);
        r.dead = true;
        let f = r.fiber;
        r.start = NIL;
        list_remove(c, t);
        blop::notify(c, t.raw());
        if f != NIL { crate::fiber::discard(c, f); }
        return Ok(t);
    }
    let r = th_mut(c, t);
    r.killed = true; r.wake = true;
    kick(c, t);
    Ok(t)
}
fn th_kill(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { kill_thread(c, slf) }
fn th_s_kill(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { kill_thread(c, a[0]) }
fn th_s_exit(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { let t = cur(c); kill_thread(c, t) }
fn th_raise(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let e = if a.is_empty() { make_exception(c.vm().c_runtime_error, "unhandled exception") } else { make_exception_from_args(c, a)? };
    thr(c, slf)?;
    if th(c, slf).dead { return Ok(NIL); }
    if slf == cur(c) { return Err(raise_exc(c, e)); }
    let r = th_mut(c, slf);
    r.pending = e; r.wake = true;
    kick(c, slf);
    Ok(NIL)
}
fn th_wakeup(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    thr(c, slf)?;
    if th(c, slf).dead { return err(c, c.vm().c_thread_error, "killed thread"); }
    th_mut(c, slf).wake = true;
    kick(c, slf);
    Ok(slf)
}
fn th_run(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { th_wakeup(c, slf, a, b)?; th_pass(c, slf, a, b)?; Ok(slf) }
fn th_alive(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { thr(c, s)?; Ok(Value::bool(!th(c, s).dead)) }
fn th_stop_p(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { thr(c, s)?; let r = th(c, s); Ok(Value::bool(r.dead || (r.waiting && s != cur(c)))) }
fn th_status(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    thr(c, s)?;
    let r = th(c, s);
    if r.dead { return Ok(if r.exc != NIL { NIL } else { FALSE }); }
    let st = status_str(c, s);
    Ok(c.str_new(st.as_bytes()))
}
fn th_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { thr(c, s)?; let t = inspect_thread(c, s, status_str(c, s)); Ok(c.str_new(t.as_bytes())) }
/// rb_frozen_error_raise(thread, "can't modify frozen thread locals")
fn frozen_locals(c: &mut Ctx, t: Value) -> Esc {
    let e = make_exception(c.vm().c_frozen_error, "can't modify frozen thread locals");
    ivar_set_raw(e, crate::symbol::s::receiver, t);
    raise_exc(c, e)
}
/// rb_to_symbol: a Symbol, a String, or what #to_str makes a String.
pub fn key_sym(c: &mut Ctx, k: Value) -> RV {
    if k.is_sym() { return Ok(k); }
    if is_string(k) { return Ok(c.str_to_sym(k)); }
    if respond_to(k, intern(b"to_str"), true) {
        let s = funcall(c, k, intern(b"to_str"), &[], std::ptr::null())?;
        if is_string(s) { return Ok(c.str_to_sym(s)); }
    }
    let d = inspect_str(c, k)?;
    type_err(c, format!("{} is not a symbol nor a string", d))
}
/// Thread#[] is fiber-local: the fiber the thread is running (a root fiber's are the Thread's own).
fn locals(c: &mut Ctx, t: Value) -> RV {
    thr(c, t)?;
    let f = if t == cur(c) { crate::fiber::current(c) }
        else if t == MAIN.get(c) { crate::fiber::main_thread_fiber(c) }
        else { let r = th(c, t); if r.dead { NIL } else if r.inner != NIL { r.inner } else { r.fiber } };
    if f == NIL || crate::fiber::is_root(c, f) { return Ok(th(c, t).locals); }
    Ok(crate::fiber::locals(c, f))
}
fn tvars(c: &mut Ctx, t: Value) -> RV { thr(c, t)?; Ok(th(c, t).tvars) }
fn hash_keys(c: &mut Ctx, h: Value) -> Value { let v: Vec<Value> = c.hash_pairs(h).into_iter().map(|p| p.0).collect(); c.ary_from_vec(v) }
fn th_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let k = key_sym(c, a[0])?; let h = locals(c, slf)?; Ok(hash_get_opt(c, h, k)?.unwrap_or(NIL)) }
fn th_aset(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let k = key_sym(c, a[0])?;
    if is_frozen(slf) { return Err(frozen_locals(c, slf)); }
    let h = locals(c, slf)?;
    if a[1] == NIL { hash_delete(c, h, k)?; } else { hash_set(c, h, k, a[1])?; }
    Ok(a[1])
}
fn th_key_p(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let k = key_sym(c, a[0])?; let h = locals(c, slf)?; Ok(Value::bool(hash_lookup(c, h, k)?.is_some())) }
fn th_keys(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let h = locals(c, slf)?; Ok(hash_keys(c, h)) }
fn th_tv_get(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let k = key_sym(c, a[0])?; let h = tvars(c, slf)?; Ok(hash_get_opt(c, h, k)?.unwrap_or(NIL)) }
fn th_tv_set(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let k = key_sym(c, a[0])?;
    if is_frozen(slf) { return Err(frozen_locals(c, slf)); }
    let h = tvars(c, slf)?;
    if a[1] == NIL { hash_delete(c, h, k)?; } else { hash_set(c, h, k, a[1])?; }
    Ok(a[1])
}
fn th_tv_p(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let k = key_sym(c, a[0])?; let h = tvars(c, slf)?; Ok(Value::bool(hash_lookup(c, h, k)?.is_some())) }
fn th_tvs(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let h = tvars(c, slf)?; Ok(hash_keys(c, h)) }
fn th_name(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { thr(c, s)?; Ok(th(c, s).name) }
fn th_set_name(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = if a[0] == NIL { NIL } else { let v = expect_str(c, a[0])?; c.str_dup(v) };
    thr(c, s)?;
    th_mut(c, s).name = n;
    Ok(a[0])
}
fn th_report(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { thr(c, s)?; let r = th(c, s).report; Ok(if r == UNDEF { Value::bool(REPORT_ALL.load(Relaxed)) } else { r }) }
fn th_set_report(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { thr(c, s)?; th_mut(c, s).report = Value::bool(a[0].truthy()); Ok(a[0]) }
fn th_abort(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { thr(c, s)?; Ok(Value::bool(th(c, s).abort)) }
fn th_set_abort(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { thr(c, s)?; th_mut(c, s).abort = a[0].truthy(); Ok(a[0]) }
fn th_group(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    thr(c, s)?;
    if th(c, s).dead { return Ok(NIL); }
    init_sync(c);
    Ok(const_get_at(const_get_at(c.vm().c_object, intern(b"ThreadGroup")).unwrap(), intern(b"Default")).unwrap())
}
fn tg_list(c: &mut Ctx, _s: Value, a: &[Value], b: *const Block) -> RV { th_list(c, NIL, a, b) }

// ---- Mutex / Monitor --------------------------------------------------------------------------

static HELD: AtomicUsize = AtomicUsize::new(0); // locks currently owned (skips the heap walk at thread exit)
fn held_dec() { let _ = HELD.fetch_update(Relaxed, Relaxed, |n| Some(n.saturating_sub(1))); }
#[derive(Clone, Copy)]
pub struct RLock { owner: Value, count: usize, monitor: bool }
impl RData for RLock {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.owner); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn lk(c: &Ctx, v: Value) -> RLock { *c.data::<RLock>(v).expect("a Mutex") }
fn lk_mut(c: &mut Ctx, v: Value) -> &mut RLock { c.data_mut::<RLock>(v).expect("a Mutex") }
pub fn alloc_lock(c: &mut Ctx, klass: Value) -> Option<Value> {
    let (km, kx) = (c.vm().c_monitor, c.vm().c_mutex);
    let mon = km != NIL && crate::builtins::inherits(klass, km);
    if mon || (kx != NIL && crate::builtins::inherits(klass, kx)) { return Some(c.alloc(klass, Body::Data(Box::new(RLock { owner: NIL, count: 0, monitor: mon })))); }
    None
}
/// A dead thread gives its locks back.
fn release_locks(c: &mut Ctx, t: Value) {
    if HELD.load(Relaxed) == 0 { return; }
    // each_object does not allocate: the closure's short borrows of c are the only ones
    let mut freed = vec![];
    crate::gc::each_object(|v| { if let Some(l) = c.data_mut::<RLock>(v) { if l.owner == t { l.owner = NIL; l.count = 0; held_dec(); freed.push(v.raw()); } } });
    for k in freed { blop::notify_one(c, k); }
}
fn acquire(c: &mut Ctx, m: Value) -> Result<(), Esc> {
    let me = cur(c);
    let l = lk(c, m);
    if l.owner == me {
        if l.monitor { lk_mut(c, m).count += 1; return Ok(()); }
        return err(c, c.vm().c_thread_error, "deadlock; recursive locking");
    }
    block_on(c, |c| { let o = lk(c, m).owner; Ok(o == NIL || th(c, o).dead) }, Some(m.raw()), None, true)?;
    let l = lk_mut(c, m);
    l.owner = me; l.count = 1;
    HELD.fetch_add(1, Relaxed);
    Ok(())
}
fn release(c: &mut Ctx, m: Value) -> Result<(), Esc> {
    let l = lk(c, m);
    if l.owner == NIL { return err(c, c.vm().c_thread_error, if l.monitor { "current thread not owner" } else { "Attempt to unlock a mutex which is not locked" }); }
    if l.owner != cur(c) { return err(c, c.vm().c_thread_error, if l.monitor { "current thread not owner" } else { "Attempt to unlock a mutex which is locked by another thread/fiber" }); }
    let l = lk_mut(c, m);
    l.count -= 1;
    if l.count == 0 { l.owner = NIL; held_dec(); blop::notify_one(c, m.raw()); }
    Ok(())
}
fn mx_lock(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { acquire(c, s)?; Ok(s) }
fn mx_unlock(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { release(c, s)?; Ok(s) }
fn mx_try_lock(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let me = cur(c);
    let l = lk_mut(c, s);
    if l.owner == NIL { l.owner = me; l.count = 1; HELD.fetch_add(1, Relaxed); return Ok(TRUE); }
    if l.owner == me && l.monitor { l.count += 1; return Ok(TRUE); }
    Ok(FALSE)
}
fn mx_locked(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(lk(c, s).owner != NIL)) }
fn mx_owned(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(lk(c, s).owner == cur(c))) }
fn mx_sync(c: &mut Ctx, s: Value, _a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return err(c, c.vm().c_thread_error, "must be called with a block"); }
    acquire(c, s)?;
    let r = yield_block(c, b, &[]);
    let (ek, ev) = (c.esc, c.esc_val);
    if lk(c, s).owner == cur(c) { release(c, s)?; }
    if r.is_err() { c.esc = ek; c.esc_val = ev; }
    r
}
fn mx_sleep(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let dur = match a.first() { Some(&t) if t != NIL => Some(crate::builtins::numeric_methods::to_f(c, t)?), _ => None };
    let t0 = Instant::now();
    release(c, s)?;
    let r = sleep_as(c, dur, true);
    acquire(c, s)?;
    r?;
    Ok(Value::fix(t0.elapsed().as_secs_f64().round() as i64))
}
fn mon_check_owner(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    if lk(c, s).owner != cur(c) { return err(c, c.vm().c_thread_error, "current thread not owner"); }
    Ok(NIL)
}
/// Monitor#wait_for_cond(cond, timeout): fully release, wait on cond, re-enter with the old count.
fn mon_wait_for_cond(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let count = lk(c, s).count;
    let l = lk_mut(c, s);
    l.owner = NIL; l.count = 0;
    held_dec();
    blop::notify_one(c, s.raw());
    let r = cv_wait_raw(c, a[0], a[1]);
    let me = cur(c);
    block_on(c, |c| Ok(lk(c, s).owner == NIL), Some(s.raw()), None, true)?;
    let l = lk_mut(c, s);
    l.owner = me; l.count = count;
    HELD.fetch_add(1, Relaxed);
    r?;
    Ok(TRUE)
}

// ---- ConditionVariable ------------------------------------------------------------------------

fn cv_waiters(c: &mut Ctx, cv: Value) -> Value {
    let w = ivar_get(cv, intern(b"__waiters"));
    if w != NIL { return w; }
    let w = c.ary_new();
    ivar_set_raw(cv, intern(b"__waiters"), w);
    w
}
fn cv_wait_raw(c: &mut Ctx, cv: Value, timeout: Value) -> Result<bool, Esc> {
    let deadline = if timeout == NIL { None } else { let f = crate::builtins::numeric_methods::to_f(c, timeout)?; Some(Instant::now() + secs(f)) };
    let token = c.ary_new_from(&[FALSE]);
    let w = cv_waiters(c, cv);
    c.ary_push(w, token);
    let r = block_on(c, |c| Ok(c.ary_get(token, 0).truthy()), Some(token.raw()), deadline, true);
    c.ary_mut(w).retain(|&x| x != token);
    r
}
fn cv_wait(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let m = a[0];
    let native = c.data::<RLock>(m).is_some();
    if native { release(c, m)?; } else { funcall(c, m, intern(b"unlock"), &[], std::ptr::null())?; }
    let r = cv_wait_raw(c, s, a.get(1).copied().unwrap_or(NIL));
    if native { acquire(c, m)?; } else { funcall(c, m, intern(b"lock"), &[], std::ptr::null())?; }
    Ok(if r? { s } else { NIL })
}
fn cv_signal(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let w = cv_waiters(c, s);
    if c.ary_len(w) > 0 { let t = c.ary_mut(w).remove(0); c.ary_mut(t)[0] = TRUE; blop::notify(c, t.raw()); }
    Ok(s)
}
fn cv_broadcast(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let w = cv_waiters(c, s);
    let ts = c.ary(w).to_vec();
    for t in ts { c.ary_mut(t)[0] = TRUE; blop::notify(c, t.raw()); }
    c.ary_mut(w).clear();
    Ok(s)
}

// ---- Queue / SizedQueue -------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct RQueue { items: Value, head: usize, closed: bool, max: usize, waiting: usize }   // items[head..] are queued
impl RData for RQueue {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.items); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn qu(c: &Ctx, v: Value) -> RQueue { *c.data::<RQueue>(v).expect("a Queue") }
fn qu_mut(c: &mut Ctx, v: Value) -> &mut RQueue { c.data_mut::<RQueue>(v).expect("a Queue") }
fn q_len(c: &Ctx, q: Value) -> usize { let q = qu(c, q); c.ary_len(q.items) - q.head }
pub fn alloc_queue(c: &mut Ctx, klass: Value) -> Option<Value> {
    let k = c.vm().c_queue;
    if k != NIL && crate::builtins::inherits(klass, k) {
        let items = c.ary_new();
        return Some(c.alloc_with(klass, move || Body::Data(Box::new(RQueue { items, head: 0, closed: false, max: 0, waiting: 0 }))));
    }
    None
}
fn q_init(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    if let Some(&init) = a.first() {
        if !respond_to(init, intern(b"to_a"), true) { return type_err(c, format!("can't convert {} into Array", class_name_for_err(init))); }
        let l = funcall(c, init, intern(b"to_a"), &[], std::ptr::null())?;
        if !is_array(l) { return type_err(c, format!("can't convert {} to Array ({}#to_a gives {})", class_name_for_err(init), class_name_for_err(init), class_name_for_err(l))); }
        let v = c.ary(l).to_vec();
        let items = c.ary_from_vec(v);
        let q = qu_mut(c, s);
        q.items = items; q.head = 0;
    }
    Ok(NIL)
}
fn sq_init(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let m = num_to_long(c, a[0])?;
    if m <= 0 { return arg_err(c, "queue size must be positive"); }
    qu_mut(c, s).max = m as usize;
    Ok(NIL)
}
fn q_opts(c: &mut Ctx, a: &[Value]) -> Result<(bool, Option<Instant>), Esc> {
    let (pos, opts) = match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    let nb = pos.first().map(|v| v.truthy()).unwrap_or(false);
    let mut deadline = None;
    if let Some(h) = opts { if let Some(t) = hash_get_opt(c, h, sym_value("timeout"))? { if t != NIL {
        if nb { return arg_err(c, "can't set a timeout if non_block is enabled"); }
        let f = crate::builtins::numeric_methods::to_f(c, t)?; deadline = Some(Instant::now() + secs(f));
    } } }
    Ok((nb, deadline))
}
/// SizedQueue pushers wait on this key, poppers on the queue's address (heap addresses are 8-aligned).
fn space_key(q: Value) -> u64 { q.raw() | 1 }
fn q_closed_err<T>(c: &mut Ctx) -> Result<T, Esc> { err(c, c.vm().c_closed_queue_error, "queue closed") }
fn q_push(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, 2); }
    let q = qu(c, s);
    if q.closed { return q_closed_err(c); }
    if q.max > 0 && q_len(c, s) >= q.max {
        let (nb, deadline) = q_opts(c, &a[1..])?;
        if nb { return err(c, c.vm().c_thread_error, "queue full"); }
        qu_mut(c, s).waiting += 1;
        let r = block_on(c, |c| { let q = qu(c, s); Ok(q.closed || q_len(c, s) < q.max) }, Some(space_key(s)), deadline, true);
        qu_mut(c, s).waiting -= 1;
        if !r? { return Ok(NIL); }
        if qu(c, s).closed { return q_closed_err(c); }
    }
    let items = qu(c, s).items;
    c.ary_push(items, a[0]);
    blop::notify_one(c, s.raw());
    Ok(s)
}
fn q_pop(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let (nb, deadline) = q_opts(c, a)?;
    if q_len(c, s) == 0 {
        if nb { return err(c, c.vm().c_thread_error, "queue empty"); }
        if qu(c, s).closed { return Ok(NIL); }
        qu_mut(c, s).waiting += 1;
        let r = block_on(c, |c| Ok(qu(c, s).closed || q_len(c, s) > 0), Some(s.raw()), deadline, true);
        qu_mut(c, s).waiting -= 1;
        if !r? || q_len(c, s) == 0 { return Ok(NIL); }
    }
    // O(1): advance head; the dead prefix is dropped once it is the larger half
    let RQueue { items, head, .. } = qu(c, s);
    let v = c.ary_mut(items);
    let x = std::mem::replace(&mut v[head], NIL);
    let head = head + 1;
    let head = if head == v.len() { v.clear(); 0 } else if head >= 64 && head * 2 >= v.len() { v.drain(..head); 0 } else { head };
    let q = qu_mut(c, s);
    q.head = head;
    if q.max > 0 { blop::notify_one(c, space_key(s)); }
    Ok(x)
}
fn q_size(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(q_len(c, s) as i64)) }
fn q_empty(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(q_len(c, s) == 0)) }
fn q_clear(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let items = qu(c, s).items; c.ary_mut(items).clear(); qu_mut(c, s).head = 0; blop::notify(c, space_key(s)); Ok(s) }
fn q_close(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { qu_mut(c, s).closed = true; blop::notify(c, s.raw()); blop::notify(c, space_key(s)); Ok(s) }
fn q_closed(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(qu(c, s).closed)) }
fn q_num_waiting(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(qu(c, s).waiting as i64)) }
fn q_freeze(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { type_err(c, format!("cannot freeze #<{}:0x{:016x}>", class_path(real_class_of(s)), s.raw())) }
fn sq_max(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(qu(c, s).max as i64)) }
fn sq_set_max(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let m = num_to_long(c, a[0])?;
    if m <= 0 { return arg_err(c, "queue size must be positive"); }
    qu_mut(c, s).max = m as usize;
    blop::notify(c, space_key(s));
    Ok(a[0])
}

/// Boot: Thread and the main thread (no context yet: allocates with gc::alloc directly).
pub fn init(vmr: &mut Vm) {
    let ob = vmr.c_object;
    let th = define_class("Thread", ob);
    class_set_alloc(th, AllocKind::None);
    vmr.c_thread = th;
    let list = crate::gc::alloc(vmr.c_array, Body::Array(crate::avec::AVec::new()));
    LIST.set_vm(vmr, list);
    let ch = vmr.c_hash;
    let l = crate::gc::alloc(ch, Body::Hash(Box::new(RHash::with_capacity(0))));
    let t = crate::gc::alloc(ch, Body::Hash(Box::new(RHash::with_capacity(0))));
    let main = crate::gc::alloc_with(th, move || { let mut r = RThread::blank(l, t); r.started = true; Body::Data(Box::new(r)) });
    MAIN.set_vm(vmr, main);
    CURRENT.set_vm(vmr, main);
    let kill = new_class(vmr.c_exception);
    C_KILL.set_vm(vmr, kill);
    let fatal = new_class(vmr.c_exception);
    name_class(fatal, ob, intern(b"fatal"));
    C_FATAL.set_vm(vmr, fatal);
    let ds = |n: &str, f: CFunc, a: i32| define_singleton_cfunc(th, n, f, a);
    ds("new", th_s_new, -1); ds("start", th_s_new, -1); ds("fork", th_s_new, -1); ds("current", th_current, 0); ds("main", th_main, 0); ds("list", th_list, 0);
    ds("pass", th_pass, 0); ds("stop", th_stop, 0); ds("kill", th_s_kill, 1); ds("exit", th_s_exit, 0);
    ds("abort_on_exception", |_c, _s, _a, _b| Ok(Value::bool(ABORT_ALL.load(Relaxed))), 0);
    ds("abort_on_exception=", |_c, _s, a, _b| { ABORT_ALL.store(a[0].truthy(), Relaxed); Ok(a[0]) }, 1);
    ds("report_on_exception", |_c, _s, _a, _b| Ok(Value::bool(REPORT_ALL.load(Relaxed))), 0);
    ds("report_on_exception=", |_c, _s, a, _b| { REPORT_ALL.store(a[0].truthy(), Relaxed); Ok(a[0]) }, 1);
    ds("handle_interrupt", |c, _s, _a, b| yield_block(c, b, &[]), -1);
    ds("pending_interrupt?", |_c, _s, _a, _b| Ok(FALSE), -1);
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(th, n, f, a);
    d("join", th_join, -1); d("value", th_value, 0); d("[]", th_aref, 1); d("[]=", th_aset, 2); d("key?", th_key_p, 1); d("keys", th_keys, 0);
    d("thread_variable_get", th_tv_get, 1); d("thread_variable_set", th_tv_set, 2); d("thread_variable?", th_tv_p, 1); d("thread_variables", th_tvs, 0);
    d("alive?", th_alive, 0); d("stop?", th_stop_p, 0); d("status", th_status, 0); d("inspect", th_inspect, 0); d("to_s", th_inspect, 0);
    d("name", th_name, 0); d("name=", th_set_name, 1); d("kill", th_kill, 0); d("terminate", th_kill, 0); d("exit", th_kill, 0); d("raise", th_raise, -1);
    d("wakeup", th_wakeup, 0); d("run", th_run, 0); d("report_on_exception", th_report, 0); d("report_on_exception=", th_set_report, 1);
    d("abort_on_exception", th_abort, 0); d("abort_on_exception=", th_set_abort, 1); d("group", th_group, 0);
    d("priority", |_c, _s, _a, _b| Ok(Value::fix(0)), 0); d("priority=", |_c, _s, a, _b| Ok(a[0]), 1);
    d("backtrace", |c, _s, _a, _b| Ok(c.ary_new()), -1); d("backtrace_locations", |c, _s, _a, _b| Ok(c.ary_new()), -1);
    d("native_thread_id", |_c, _s, _a, _b| Ok(Value::fix(crate::builtins::sys::getpid() as i64)), 0);
    d("pending_interrupt?", |_c, _s, _a, _b| Ok(FALSE), -1);
    d("fetch", |c, s, a, b| { check_argc(c, a, 1, 2)?; let k = key_sym(c, a[0])?; let h = locals(c, s)?; match hash_get_opt(c, h, k)? { Some(v) => Ok(v), None if !b.is_null() => yield1(c, b, a[0]), None if a.len() > 1 => Ok(a[1]), None => { let d = inspect_str(c, k)?; err(c, c.vm().c_key_error, format!("key not found: {}", d)) } } }, -1);
    for n in ["Mutex", "Queue", "SizedQueue", "ConditionVariable"] { crate::builtins::prelude::lazy_const(th, n, "saruby/sync"); crate::builtins::prelude::lazy_const(ob, n, "saruby/sync"); }
    crate::builtins::prelude::lazy_const(ob, "Monitor", "saruby/sync");
    crate::builtins::prelude::lazy_const(ob, "ThreadGroup", "saruby/sync");
}
/// ThreadGroup, Mutex, Monitor, Queue, SizedQueue, ConditionVariable (the "saruby/sync" feature).
pub fn init_sync(c: &mut Ctx) {
    if c.vm().c_mutex != NIL { return; }
    let (ob, th) = (c.vm().c_object, c.vm().c_thread);
    let tg = define_class("ThreadGroup", ob);
    let dg = new_object(tg);
    const_set(tg, intern(b"Default"), dg);
    define_cfunc(tg, "list", tg_list, 0);
    define_cfunc(tg, "add", |_c, s, _a, _b| Ok(s), 1);
    define_cfunc(tg, "enclosed?", |_c, _s, _a, _b| Ok(FALSE), 0);
    // Mutex / Monitor
    let mx = define_class_under(th, "Mutex", ob);
    class_set_alloc(mx, AllocKind::None);
    const_set(ob, intern(b"Mutex"), mx);
    c.vm_mut().c_mutex = mx;
    let mon = define_class("Monitor", ob);
    class_set_alloc(mon, AllocKind::None);
    c.vm_mut().c_monitor = mon;
    for (k, pre) in [(mx, ""), (mon, "mon_")] {
        define_cfunc(k, "synchronize", mx_sync, 0);
        define_cfunc(k, &format!("{}locked?", pre), mx_locked, 0);
        define_cfunc(k, &format!("{}owned?", pre), mx_owned, 0);
    }
    define_cfunc(mx, "lock", mx_lock, 0); define_cfunc(mx, "unlock", mx_unlock, 0); define_cfunc(mx, "try_lock", mx_try_lock, 0); define_cfunc(mx, "sleep", mx_sleep, -1);
    define_cfunc(mon, "enter", mx_lock, 0); define_cfunc(mon, "exit", mx_unlock, 0); define_cfunc(mon, "try_enter", mx_try_lock, 0);
    define_cfunc(mon, "new_cond", |c, s, a, b| { let f = c.str_new(b"monitor"); crate::builtins::kernel::require_feature(c, f, false)?; funcall(c, s, intern(b"new_cond"), a, b) }, 0);
    define_cfunc(mon, "mon_synchronize", mx_sync, 0); define_cfunc(mon, "mon_check_owner", mon_check_owner, 0); define_cfunc(mon, "wait_for_cond", mon_wait_for_cond, 2);
    // Queue / SizedQueue / ConditionVariable
    let q = define_class_under(th, "Queue", ob);
    class_set_alloc(q, AllocKind::None);
    const_set(ob, intern(b"Queue"), q);
    c.vm_mut().c_queue = q;
    define_private_cfunc(q, "initialize", q_init, -1);
    for n in ["push", "<<", "enq"] { define_cfunc(q, n, q_push, -1); }
    for n in ["pop", "shift", "deq"] { define_cfunc(q, n, q_pop, -1); }
    for n in ["size", "length"] { define_cfunc(q, n, q_size, 0); }
    define_cfunc(q, "empty?", q_empty, 0); define_cfunc(q, "clear", q_clear, 0); define_cfunc(q, "close", q_close, 0); define_cfunc(q, "closed?", q_closed, 0);
    define_cfunc(q, "num_waiting", q_num_waiting, 0); define_cfunc(q, "freeze", q_freeze, 0);
    let sq = define_class_under(th, "SizedQueue", q);
    const_set(ob, intern(b"SizedQueue"), sq);
    define_private_cfunc(sq, "initialize", sq_init, 1);
    define_cfunc(sq, "max", sq_max, 0); define_cfunc(sq, "max=", sq_set_max, 1);
    let cv = define_class_under(th, "ConditionVariable", ob);
    const_set(ob, intern(b"ConditionVariable"), cv);
    define_cfunc(cv, "wait", cv_wait, -1); define_cfunc(cv, "signal", cv_signal, 0); define_cfunc(cv, "broadcast", cv_broadcast, 0);
}
