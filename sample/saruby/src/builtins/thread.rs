// Green threads: each Thread is a fiber chain switched by the main thread's scheduler at
// blocking points (join, Queue, Mutex, ConditionVariable, sleep, pipe reads, Thread.pass), and
// preempted at check points after its time slice (the timer thread below, docs/io_design.md).
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
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering::{Relaxed, SeqCst}};
use std::time::{Duration, Instant};
use crate::builtins::blop::{self, Id, Op, Res};
use crate::builtins::sys::{self, PollFd};

pub struct RThread {
    fiber: Value, inner: Value, started: bool, dead: bool, value: Value, exc: Value, locals: Value, tvars: Value, name: Value,
    blop: Option<Id>,   // the op it is parked on (None: runnable when switched out)
    waiting: bool, wake: bool,
    pending: Value,   // queued Thread#raise exceptions and KILL (an Array, or nil when empty)
    masks: Value,     // Thread.handle_interrupt stack: Symbol (every exception) or [mod, sym, ...] (nil: empty)
    priority: i32,
    report: Value, abort: bool, loc: String,
    start: Value,   // [proc, args] until the first run makes the fiber (stacks only for threads that run)
    last_status: u64,   // $? (process.rs)
    ractor: Value,   // the Ractor it belongs to (nil: the main one; ractor.rs)
    scheduler: Value,   // Fiber.set_scheduler (nil: none)
}
impl RData for RThread {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { for v in [&mut self.fiber, &mut self.inner, &mut self.value, &mut self.exc, &mut self.locals, &mut self.tvars, &mut self.name, &mut self.pending, &mut self.masks, &mut self.report, &mut self.start, &mut self.ractor, &mut self.scheduler] { m.visit(v); } }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
impl RThread {
    fn blank(locals: Value, tvars: Value) -> RThread {
        RThread { fiber: NIL, inner: NIL, started: false, dead: false, value: NIL, exc: NIL, locals, tvars, name: NIL,
            blop: None, waiting: false, wake: false, pending: NIL, masks: NIL, priority: 0, report: UNDEF, abort: false, loc: String::new(), start: NIL,
            last_status: crate::builtins::process::NO_STATUS, ractor: NIL, scheduler: NIL }
    }
    /// An interrupt a blocking call would deliver is queued.
    fn interrupted(&self, c: &Ctx) -> bool { self.pending != NIL && self.deliverable(c, true).is_some() }
    /// The scheduler should switch it in: not started, yielded, its op completed, or interrupted.
    fn runnable(&self, c: &Ctx) -> bool { !self.dead && (!self.started || self.blop.is_none_or(|id| blop::is_done(c, id)) || self.interrupted(c)) }
    /// Index of the first queued interrupt the masks let through now (rb_threadptr_pending_interrupt_deque).
    fn deliverable(&self, c: &Ctx, blocking: bool) -> Option<usize> {
        if self.pending == NIL { return None; }
        c.ary(self.pending).iter().position(|&e| match self.timing(c, e) { Timing::Never => false, Timing::OnBlocking => blocking, Timing::Immediate => true })
    }
    /// rb_threadptr_pending_interrupt_check_mask: the innermost mask naming an ancestor of e's class.
    fn timing(&self, c: &Ctx, e: Value) -> Timing {
        if self.masks == NIL { return Timing::Immediate; }
        let anc = crate::vm::ancestors(c, class_of(e));
        for &m in c.ary(self.masks).iter().rev() {
            // a bare symbol does not cover Thread#kill (queued as an Integer, as CRuby's)
            if m.is_sym() { if e != kill_mark() { return Timing::of(m); } continue; }
            let pairs = c.ary(m);
            for &k in anc.iter() { if let Some(p) = pairs.chunks(2).find(|p| p[0] == k) { return Timing::of(p[1]); } }
        }
        Timing::Immediate
    }
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
#[derive(Clone, Copy, PartialEq)]
enum Timing { Immediate, OnBlocking, Never }
impl Timing {
    fn of(sym: Value) -> Timing { if sym == sym_value("never") { Timing::Never } else if sym == sym_value("on_blocking") { Timing::OnBlocking } else { Timing::Immediate } }
}
/// Thread#kill in the interrupt queue (CRuby's RUBY_FATAL_THREAD_KILLED, INT2FIX(0)).
fn kill_mark() -> Value { Value::fix(0) }

fn th(c: &Ctx, v: Value) -> &RThread { c.data::<RThread>(v).expect("a Thread") }
fn th_mut(c: &mut Ctx, v: Value) -> &mut RThread { c.data_mut::<RThread>(v).expect("a Thread") }
fn thr(c: &mut Ctx, v: Value) -> Result<(), Esc> { if c.data::<RThread>(v).is_some() { Ok(()) } else { type_err(c, "wrong argument type (expected Thread)") } }
fn cur(c: &Ctx) -> Value { CURRENT.get(c) }
/// Thread.current
pub fn current_thread(c: &Ctx) -> Value { cur(c) }
/// The Ractor of the current thread (nil: the main Ractor).
pub fn current_ractor(c: &Ctx) -> Value { th(c, cur(c)).ractor }
/// (started, parked, dead) of a thread, for Ractor#inspect.
pub fn thread_state(c: &Ctx, t: Value) -> (bool, bool, bool) { let r = th(c, t); (r.started, r.waiting, r.dead) }
pub fn last_status(c: &Ctx) -> u64 { c.data::<RThread>(cur(c)).map_or(crate::builtins::process::NO_STATUS, |r| r.last_status) }
pub fn set_last_status(c: &mut Ctx, w: u64) { let t = cur(c); if let Some(r) = c.data_mut::<RThread>(t) { r.last_status = w; } }
#[inline]
fn is_main(c: &Ctx) -> bool { CURRENT.get(c) == MAIN.get(c) }
/// Other threads exist (or we are inside one), or some thread has a Fiber scheduler: blocking calls
/// must go through the scheduler.  One load: COOP counts the live non-main threads and the threads
/// with a Fiber scheduler (not #[inline]: the static then stays local, no GOT load).
pub fn active(_c: &Ctx) -> bool { COOP.load(Relaxed) != 0 }
static COOP: AtomicUsize = AtomicUsize::new(0);
fn new_thread(c: &mut Ctx, klass: Value) -> Value {
    let l = c.hash_new();
    let t = c.hash_new();
    c.alloc_with(klass, move || Body::Data(Box::new(RThread::blank(l, t))))
}
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let k = c.vm().c_thread.get();
    if k != NIL && crate::builtins::inherits(c, klass, k) { return Ok(Some(new_thread(c, klass))); }
    Ok(None)
}
fn list_remove(c: &mut Ctx, t: Value) {
    let q = std::mem::replace(&mut th_mut(c, t).pending, NIL);
    if q != NIL { let n = c.ary_len(q); QUEUED.fetch_sub(n, Relaxed); }
    let l = LIST.get(c);
    let n = c.ary_len(l);
    c.ary_mut(l).retain(|&x| x != t);
    COOP.fetch_sub(n - c.ary_len(l), Relaxed);
    if c.ary_len(l) == 0 { timer_run(false); }
}

/// Deliver t's first interrupt the masks let through (`blocking`: at a blocking call, where
/// :on_blocking ones go too).
#[inline]
fn check_interrupt_at(c: &mut Ctx, t: Value, blocking: bool) -> Result<(), Esc> {
    if th(c, t).pending == NIL { return Ok(()); }
    deliver_interrupt(c, t, blocking)
}
#[inline(never)]
fn deliver_interrupt(c: &mut Ctx, t: Value, blocking: bool) -> Result<(), Esc> {
    let r = th(c, t);
    let Some(i) = r.deliverable(c, blocking) else { return Ok(()) };
    let q = r.pending;
    let e = c.ary_mut(q).remove(i);
    QUEUED.fetch_sub(1, Relaxed);
    if c.ary_len(q) == 0 { th_mut(c, t).pending = NIL; }
    if e == kill_mark() { let k = make_exception(c, C_KILL.get(c), "killed"); return Err(raise_exc(c, k)); }
    Err(raise_exc(c, e))
}
fn check_interrupt(c: &mut Ctx, t: Value) -> Result<(), Esc> { check_interrupt_at(c, t, true) }
/// Interrupts queued in all threads: lets a blocking IO call skip the queue test.
static QUEUED: AtomicUsize = AtomicUsize::new(0);
/// puts / print / IO#write are blocking calls: :on_blocking interrupts are delivered before the write.
#[inline(always)]
pub fn blocking_point(c: &mut Ctx) -> Result<(), Esc> {
    if QUEUED.load(Relaxed) != 0 { return blocking_point_slow(c); }
    Ok(())
}
#[cold]
#[inline(never)]
fn blocking_point_slow(c: &mut Ctx) -> Result<(), Esc> { let me = cur(c); check_interrupt_at(c, me, true) }
/// Queue an interrupt for t (rb_threadptr_pending_interrupt_enque) and wake it if it may take it.
fn enqueue_interrupt(c: &mut Ctx, t: Value, e: Value) {
    let q = th(c, t).pending;
    if q == NIL { let q = c.ary_new_from(&[e]); th_mut(c, t).pending = q; } else { c.ary_push(q, e); }
    QUEUED.fetch_add(1, Relaxed);
    if th(c, t).interrupted(c) { kick(c, t); }
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
    release_locks(c, t)?;
    wake_all(c, t.raw())?;
    if FORKED.load(Relaxed) && t == MAIN.get(c) { forked_main_exit(c, is_err, v); }
    if !is_err { th_mut(c, t).value = v; return Ok(()); }
    if is_kind_of(c, v, C_KILL.get(c)) { return Ok(()); }
    if is_kind_of(c, v, c.vm().c_system_exit.get()) { return Err(raise_exc(c, v)); }
    let r = th_mut(c, t);
    r.exc = v;
    let report = if r.report == UNDEF { REPORT_ALL.load(Relaxed) } else { r.report.truthy() };
    if report {
        let head = format!("{} terminated with exception (report_on_exception is true):\n", inspect_thread(c, t, "run"));
        let msg = crate::builtins::exception::full_message(c, v).unwrap_or_default();
        err_write(format!("{}{}", head, msg).as_bytes());
    }
    if th(c, t).abort || ABORT_ALL.load(Relaxed) { return Err(raise_exc(c, v)); }
    Ok(())
}
/// A child forked from a green thread ends when that thread does (it is the child's main thread).
fn forked_main_exit(c: &mut Ctx, is_err: bool, v: Value) -> ! {
    let r = if !is_err || is_kind_of(c, v, C_KILL.get(c)) { Ok(NIL) } else { Err(raise_exc(c, v)) };
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
        slice_start();
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
    run_unblocks(c)?;
    let me = cur(c);
    check_interrupt(c, me)?;
    let id = blop::submit(c, op, deadline, deadlockable);
    let r = park(c, me, id);
    let out = blop::take(c, id);
    // a notify_one this thread got but will not act on goes to the next waiter
    if r.is_err() || th(c, me).interrupted(c) { if let (Some(Op::Wake(k)), Ok(_)) = &out { pass_on(c, *k); } }
    r?;
    check_interrupt(c, me)?;
    Ok(out)
}
fn park(c: &mut Ctx, me: Value, id: Id) -> Result<(), Esc> {
    if !is_main(c) {
        while !blop::is_done(c, id) && !th(c, me).interrupted(c) {
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
        run_unblocks(c)?;
        if blop::is_done(c, id) || th(c, me).interrupted(c) { return Ok(()); }
        if crate::builtins::process::signals_pending() {
            crate::builtins::process::run_pending_signals(c)?;
            if blop::is_done(c, id) || th(c, me).interrupted(c) { return Ok(()); }
        }
        if run_pass(c)? { blop::pump(c, false); continue; }
        if !blop::can_finish_alone(c) && !IGNORE_DEADLOCK.load(Relaxed) { return err(c, C_FATAL.get(c), "No live threads left. Deadlock?"); }
        // idle: no time slice to measure
        timer_run(false);
        blop::pump(c, true);
        timer_run(c.ary_len(LIST.get(c)) > 0);
    }
}
/// Block the current thread until cond holds (Ok(true)) or the deadline passes (Ok(false)).
/// `key`: what blop::notify ends the wait (an object's address; None: only time or an interrupt does).
pub fn block_on(c: &mut Ctx, cond: impl FnMut(&mut Ctx) -> Result<bool, Esc>, key: Option<u64>, deadline: Option<Instant>, deadlockable: bool) -> Result<bool, Esc> {
    block_on_by(c, Hook::None, NIL, cond, key, deadline, deadlockable)
}
/// block_on; a non-blocking fiber with a scheduler calls `hook` instead of waiting, and a notify of
/// `key` calls the scheduler's unblock(`ublocker`, fiber).
fn block_on_by(c: &mut Ctx, hook: Hook, ublocker: Value, mut cond: impl FnMut(&mut Ctx) -> Result<bool, Esc>, key: Option<u64>, deadline: Option<Instant>, deadlockable: bool) -> Result<bool, Esc> {
    let me = cur(c);
    loop {
        check_interrupt(c, me)?;
        if cond(c)? { return Ok(true); }
        if deadline.is_some_and(|d| Instant::now() >= d) { return Ok(false); }
        if let (Some(k), false) = (key, matches!(hook, Hook::None)) {
            let s = scheduler_current(c);
            if s != NIL { sched_block(c, s, k, hook, ublocker, deadline)?; continue; }
        }
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
    let k = c.vm().c_io_error.get();
    let e = make_exception(c, k, "stream closed in another thread");
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
    for t in others {
        let r = th_mut(c, t); r.dead = true; r.blop = None; r.waiting = false; r.pending = NIL;
        // their locks are abandoned (rb_mutex_abandon_all)
        let _ = release_locks(c, t);
        c.esc = EscKind::None;
    }
    let q = th(c, me).pending;
    QUEUED.store(if q == NIL { 0 } else { c.ary_len(q) }, Relaxed);
    COOP.fetch_sub(c.ary_len(l), Relaxed);
    c.ary_mut(l).clear();
    // the child has no scheduler, and the forking fiber is blocking (rb_thread_atfork)
    if SCHEDULERS.load(Relaxed) != 0 {
        let all: Vec<Value> = std::iter::once(MAIN.get(c)).chain(std::iter::once(me)).collect();
        for t in all { th_mut(c, t).scheduler = NIL; }
        COOP.fetch_sub(SCHEDULERS.swap(0, Relaxed), Relaxed);
        let f = crate::fiber::current(c);
        crate::fiber::set_blocking(c, f, true);
    }
    // the timer thread did not survive the fork: timer_run spawns another
    TIMER_RUN.store(false, SeqCst);
    TIMER_UP.store(false, SeqCst);
    let main = MAIN.get(c);
    if me != main {
        let r = th_mut(c, main); r.dead = true; r.waiting = false;
        th_mut(c, me).blop = None;
        MAIN.set(c, me);
        FORKED.store(true, Relaxed);
    }
}

// ---- time slice -------------------------------------------------------------------------------
// CRuby's timer thread: while other threads exist and the interpreter is not idle in the pump, an
// OS thread ticks every TICK and sets the interrupt bit (vm::set_ints).  The check points (a call
// site's cache miss, a loop's back edge, Kernel#loop) then poll: after the current thread's quantum
// it gives the others a turn.  The timer touches nothing but atomics.

const TICK: Duration = Duration::from_millis(10);
static TIMER_RUN: AtomicBool = AtomicBool::new(false);
static TIMER_PARKED: AtomicBool = AtomicBool::new(false);
static TIMER_UP: AtomicBool = AtomicBool::new(false);   // spawned in this process
static TIMER: std::sync::Mutex<Option<std::thread::Thread>> = std::sync::Mutex::new(None);
static TICKS: AtomicU32 = AtomicU32::new(0);
static SLICE_START: AtomicU32 = AtomicU32::new(0);   // TICKS when the running thread was switched in

fn timer_main() {
    sys::block_all_signals();   // signals stay with the interpreter thread (EINTR, trap)
    let mut idle = 0;
    loop {
        std::thread::sleep(TICK);
        if TIMER_RUN.load(Relaxed) { idle = 0; TICKS.fetch_add(1, Relaxed); crate::vm::set_ints(); continue; }
        // park after a while idle: no wake-ups while nothing runs (the store / load pairs with timer_run's)
        idle += 1;
        if idle < 10 { continue; }
        TIMER_PARKED.store(true, SeqCst);
        if !TIMER_RUN.load(SeqCst) { std::thread::park(); }
        TIMER_PARKED.store(false, SeqCst);
        idle = 0;
    }
}
/// Ticks wanted (other threads exist, the interpreter is not idle) or not.
fn timer_run(on: bool) {
    if TIMER_RUN.load(Relaxed) == on { return; }
    TIMER_RUN.store(on, SeqCst);
    if !on { return; }
    let mut t = TIMER.lock().unwrap_or_else(|e| e.into_inner());
    if !TIMER_UP.load(Relaxed) {
        // first use, or a forked child (the thread is not copied)
        TIMER_PARKED.store(false, SeqCst);
        if let Ok(h) = std::thread::Builder::new().name("saruby-timer".into()).spawn(timer_main) { *t = Some(h.thread().clone()); TIMER_UP.store(true, Relaxed); }
    } else if TIMER_PARKED.load(SeqCst) { if let Some(h) = &*t { h.unpark(); } }
}
fn slice_start() { SLICE_START.store(TICKS.load(Relaxed), Relaxed); }
/// The running thread used its quantum: CRuby's 100 ms, << priority (>> for a negative one).
fn slice_over(pri: i32) -> bool {
    let us: u64 = if pri >= 0 { 100_000 << pri } else { 100_000 >> -pri };
    let ticks = us.div_ceil(TICK.as_micros() as u64) as u32;
    TICKS.load(Relaxed).wrapping_sub(SLICE_START.load(Relaxed)) >= ticks
}
/// A check point saw the interrupt bit (vm::ints_pending): the time slice, trap handlers, then the
/// current thread's immediate interrupts (rb_threadptr_execute_interrupts).
#[cold]
#[inline(never)]
pub fn poll(c: &mut Ctx) -> Result<(), Esc> {
    crate::vm::clear_ints();
    run_unblocks(c)?;
    if crate::builtins::ractor::multi() { crate::builtins::ractor::check_deferred(c)?; }
    let me = cur(c);
    let main = is_main(c);
    let sig = crate::builtins::process::signals_pending();
    if active(c) && (slice_over(th(c, me).priority) || (sig && !main)) {
        // the others' turn; trap handlers run in the main thread
        if main { blop::pump(c, false); run_pass(c)?; } else { crate::fiber::thread_switch_out(c); }
        slice_start();
    }
    if main && crate::builtins::process::signals_pending() { crate::builtins::process::run_pending_signals(c)?; }
    crate::builtins::misc::run_finalizer_queue(c, crate::builtins::misc::FinAt::Here);
    check_interrupt_at(c, me, false)
}

/// Thread.handle_interrupt(hash) { }: push a mask; queued interrupts it lets through are delivered
/// before the block, the ones it held back when it is popped (rb_thread_s_handle_interrupt).
fn th_s_handle_interrupt(c: &mut Ctx, _s: Value, a: &[Value], b: Blk) -> RV {
    check_argc(c, a, 1, 1)?;
    if b.is_null() { return arg_err(c, "block is needed."); }
    if !is_hash(a[0]) { return type_err(c, format!("no implicit conversion of {} into Hash", class_name_for_err(c, a[0]))); }
    let (imm, onb, nev) = (sym_value("immediate"), sym_value("on_blocking"), sym_value("never"));
    let pairs = c.hash_pairs(a[0]);
    if pairs.iter().any(|p| p.1 != imm && p.1 != onb && p.1 != nev) { return arg_err(c, "unknown mask signature"); }
    if pairs.is_empty() { return yield_block(c, b, &[NIL]); }
    let exc = c.vm().c_exception.get();
    let mask = if pairs.len() == 1 && pairs[0].0 == exc { pairs[0].1 } else {
        let v: Vec<Value> = pairs.iter().flat_map(|p| [p.0, p.1]).collect();
        c.ary_from_vec(v)
    };
    let me = cur(c);
    let ms = th(c, me).masks;
    if ms == NIL { let ms = c.ary_new_from(&[mask]); th_mut(c, me).masks = ms; } else { c.ary_push(ms, mask); }
    let r = check_interrupt_at(c, me, false).and_then(|_| yield_block(c, b, &[NIL]));
    let (ek, ev) = (c.esc, c.esc_val);
    let ms = th(c, me).masks;
    c.ary_mut(ms).pop();
    if c.ary_len(ms) == 0 { th_mut(c, me).masks = NIL; }
    // a held-back interrupt replaces the block's result or exception
    check_interrupt_at(c, me, false)?;
    if r.is_err() { c.esc = ek; c.esc_val = ev; }
    r
}
/// Thread.pending_interrupt?(err = nil) / Thread#pending_interrupt?
fn pending_interrupt_p(c: &mut Ctx, t: Value, a: &[Value]) -> RV {
    check_argc(c, a, 0, 1)?;
    let q = th(c, t).pending;
    if q == NIL { return Ok(FALSE); }
    let Some(&k) = a.first() else { return Ok(TRUE) };
    if !is_class_or_module(k) { return type_err(c, "class or module required for rescue clause"); }
    let es = c.ary(q).to_vec();
    Ok(Value::bool(es.into_iter().any(|e| is_kind_of(c, e, k))))
}
fn th_priority(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { thr(c, s)?; Ok(Value::fix(th(c, s).priority as i64)) }
fn th_set_priority(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    thr(c, s)?;
    let p = num_to_long(c, a[0])?.clamp(-3, 3) as i32;
    th_mut(c, s).priority = p;
    Ok(Value::fix(p as i64))
}

// ---- Fiber scheduler (scheduler.c) -------------------------------------------------------------
// A thread's scheduler (Fiber.set_scheduler) is called instead of blocking while a non-blocking fiber
// runs: kernel_sleep, block / unblock (Mutex, Queue, ConditionVariable, Thread#join), io_wait (io.rs,
// socket.rs), io_select, process_wait, address_resolve.  A fiber blocked through `block` keeps a Wake op
// in the blop wait list, so the notify that would have woken a thread calls the scheduler's unblock.

static SCHEDULERS: AtomicUsize = AtomicUsize::new(0);   // threads with a scheduler set
static FWAITS: Global = Global::new();   // [idx, gen, ublocker, fiber, thread] per fiber blocked through `block`
/// What a blocking call asks the scheduler: nothing (not routed), block(blocker, timeout), kernel_sleep(timeout).
#[derive(Clone, Copy)]
enum Hook { None, Block(Value), Sleep(Value), Join(Value) }   // Join: block(thread), unblock(the waiting thread)

/// Some thread has a Fiber scheduler.  Not #[inline]: SCHEDULERS then stays local to the crate (a
/// RIP-relative compare; reached from an exported inline function it would go through the GOT).
pub fn schedulers_set() -> bool { SCHEDULERS.load(Relaxed) != 0 }
/// The running thread's scheduler (Fiber.scheduler).
pub fn thread_scheduler(c: &Ctx) -> Value { th(c, cur(c)).scheduler }
/// rb_fiber_scheduler_current: the thread's scheduler while a non-blocking fiber runs, else nil.
pub fn scheduler_current(c: &Ctx) -> Value { if SCHEDULERS.load(Relaxed) == 0 { NIL } else { scheduler_current_slow(c) } }
#[inline(never)]
fn scheduler_current_slow(c: &Ctx) -> Value {
    let s = th(c, cur(c)).scheduler;
    if s == NIL || crate::fiber::is_blocking(c, crate::fiber::current(c)) { NIL } else { s }
}
fn set_thread_scheduler(c: &mut Ctx, t: Value, s: Value) {
    let r = th_mut(c, t);
    let (was, now) = (r.scheduler != NIL, s != NIL);
    r.scheduler = s;
    if was && !now { SCHEDULERS.fetch_sub(1, Relaxed); COOP.fetch_sub(1, Relaxed); }
    if now && !was { SCHEDULERS.fetch_add(1, Relaxed); COOP.fetch_add(1, Relaxed); }
}
/// rb_fiber_scheduler_set: the old one is closed first (and dropped even if close raises).
pub fn set_scheduler(c: &mut Ctx, s: Value) -> RV {
    if s != NIL {
        for m in ["block", "unblock", "kernel_sleep", "io_wait"] {
            if !respond_to(c, s, intern(m.as_bytes()), false) { return arg_err(c, format!("Scheduler must implement #{}", m)); }
        }
        if !respond_to(c, s, intern(b"fiber_interrupt"), false) { gvar_warn(c, "Scheduler should implement #fiber_interrupt", false)?; }
    }
    let me = cur(c);
    let old = th(c, me).scheduler;
    if old != NIL {
        let r = scheduler_close(c, old);
        set_thread_scheduler(c, me, NIL);
        r?;
    }
    set_thread_scheduler(c, me, s);
    Ok(s)
}
/// rb_fiber_scheduler_close: scheduler_close, else close.
fn scheduler_close(c: &mut Ctx, s: Value) -> RV {
    for m in [&b"scheduler_close"[..], b"close"] {
        if respond_to(c, s, intern(m), true) { return funcall(c, s, intern(m), &[], std::ptr::null()); }
    }
    Ok(NIL)
}
/// The thread ends (or the process exits): its scheduler runs to completion (set_scheduler(nil)).
pub fn scheduler_finalize(c: &mut Ctx) -> Result<(), Esc> {
    if thread_scheduler(c) == NIL { return Ok(()); }
    set_scheduler(c, NIL).map(|_| ())
}
/// A scheduler hook the scheduler may lack (rb_check_funcall): None when it does not respond.
pub fn sched_hook(c: &mut Ctx, s: Value, name: &str, args: &[Value]) -> Result<Option<Value>, Esc> {
    let m = intern(name.as_bytes());
    if !respond_to(c, s, m, true) { return Ok(None); }
    funcall(c, s, m, args, std::ptr::null()).map(Some)
}
/// The scheduler has hook `name` (rb_respond_to).
pub fn sched_has(c: &Ctx, s: Value, name: &str) -> bool { respond_to(c, s, intern(name.as_bytes()), true) }
/// The timeout argument of a hook: the time left to `deadline`, or nil.
fn timeout_value(c: &mut Ctx, deadline: Option<Instant>) -> Value {
    match deadline { Some(d) => c.float_new(d.saturating_duration_since(Instant::now()).as_secs_f64()), None => NIL }
}
static FIOS: Global = Global::new();   // [io, fiber, thread] per fiber in a scheduler's io_wait (IO#close interrupts them)
/// rb_fiber_scheduler_io_wait(io, events, timeout): Some(ready events) or None (timed out).
pub fn sched_io_wait(c: &mut Ctx, s: Value, io: Value, events: i16, timeout: Value) -> Result<Option<i16>, Esc> {
    let l = match FIOS.get(c) { NIL => { let l = c.ary_new(); FIOS.set(c, l); l } l => l };
    let (f, me) = (crate::fiber::current(c), cur(c));
    let ent = c.ary_new_from(&[io, f, me]);
    c.ary_push(l, ent);
    let r = funcall(c, s, intern(b"io_wait"), &[io, Value::fix(events as i64), timeout], std::ptr::null());
    c.ary_mut(l).retain(|&e| e != ent);
    // an IO#close waiting for this fiber to leave io_wait goes on
    if CLOSING.load(Relaxed) != 0 { pass_on_all(c, io.raw() | 2); }
    let r = r?;
    Ok(if r.is_fixnum() { Some(r.as_fix() as i16) } else if r.truthy() { Some(events) } else { None })
}
static CLOSING: AtomicUsize = AtomicUsize::new(0);   // IO#close calls waiting in io_close_interrupt
fn fio_waiting(c: &Ctx, io: Value) -> bool { let l = FIOS.get(c); l != NIL && c.ary(l).iter().any(|&e| c.ary_get(e, 0) == io) }
/// IO#close: a fiber waiting on `io` in its scheduler's io_wait gets IOError through fiber_interrupt,
/// and the close waits until it has left (rb_thread_io_close_interrupt / _wait).
pub fn io_close_interrupt(c: &mut Ctx, io: Value) -> Result<(), Esc> {
    let l = FIOS.get(c);
    if l == NIL || c.ary_len(l) == 0 { return Ok(()); }
    let ents: Vec<Value> = c.ary(l).iter().copied().filter(|&e| c.ary_get(e, 0) == io).collect();
    // the hooks may end those waits (their entries leave FIOS): the snapshot keeps them alive
    let snap = c.ary_from_vec(ents);
    let mut any = false;
    let mut i = 0;
    while let Some(&e) = c.ary(snap).get(i) {
        i += 1;
        let (f, t) = (c.ary_get(e, 1), c.ary_get(e, 2));
        let s = th(c, t).scheduler;
        if s == NIL || !respond_to(c, s, intern(b"fiber_interrupt"), true) { continue; }
        let k = c.vm().c_io_error.get();
        let x = make_exception(c, k, "stream closed in another thread");
        funcall(c, s, intern(b"fiber_interrupt"), &[f, x], std::ptr::null())?;
        any = true;
    }
    if !any { return Ok(()); }
    CLOSING.fetch_add(1, Relaxed);
    let hook = if scheduler_current(c) != NIL { Hook::Sleep(NIL) } else { Hook::None };
    let r = block_on_by(c, hook, NIL, |c| Ok(!fio_waiting(c, io)), Some(io.raw() | 2), None, false);
    CLOSING.fetch_sub(1, Relaxed);
    r.map(|_| ())
}
/// wake_all while an exception may be on its way out.
fn pass_on_all(c: &mut Ctx, key: u64) {
    let (ek, ev) = (c.esc, c.esc_val);
    if wake_all(c, key).is_err() { c.esc = EscKind::None; }
    c.esc = ek; c.esc_val = ev;
}
fn fwaits(c: &mut Ctx) -> Value {
    let l = FWAITS.get(c);
    if l != NIL { return l; }
    let l = c.ary_new();
    FWAITS.set(c, l);
    l
}
/// Block the running non-blocking fiber through scheduler `s` until a notify of `key` (or the hook returns).
fn sched_block(c: &mut Ctx, s: Value, key: u64, hook: Hook, ublocker: Value, deadline: Option<Instant>) -> Result<(), Esc> {
    let id = blop::submit_fiber(c, key);
    let (i, g) = id.parts();
    let (f, me) = (crate::fiber::current(c), cur(c));
    let ublocker = if matches!(hook, Hook::Join(_)) { me } else { ublocker };
    let ent = c.ary_new_from(&[Value::fix(i), Value::fix(g), ublocker, f, me]);
    let l = fwaits(c);
    c.ary_push(l, ent);
    let t = timeout_value(c, deadline);
    let r = match hook {
        Hook::Block(b) | Hook::Join(b) => funcall(c, s, intern(b"block"), &[b, t], std::ptr::null()),
        Hook::Sleep(d) => funcall(c, s, intern(b"kernel_sleep"), &[d], std::ptr::null()),
        Hook::None => Ok(NIL),
    };
    c.ary_mut(l).retain(|&e| e != ent);
    let (_, res) = blop::take(c, id);
    // a wake-up this fiber got but leaves without acting on goes to the next waiter
    if r.is_err() && res.is_ok() { pass_on(c, key); }
    r.map(|_| ())
}
/// The scheduler's unblock for the fibers whose ops a notify completed (rb_fiber_scheduler_unblock;
/// called at the waker's next check point or wait instead of inside the notify: the notify paths then
/// carry no test of their own).
#[inline]
fn run_unblocks(c: &mut Ctx) -> Result<(), Esc> {
    if !blop::unblocks_pending() { return Ok(()); }
    run_unblocks_now(c)
}
#[cold]
#[inline(never)]
fn run_unblocks_now(c: &mut Ctx) -> Result<(), Esc> {
    for id in blop::take_unblocks(c) { sched_unblock(c, id)?; }
    Ok(())
}
/// The op `id` was completed by a notify: if a fiber blocked through a scheduler owns it, unblock it.
#[cold]
fn sched_unblock(c: &mut Ctx, id: blop::Id) -> Result<(), Esc> {
    let (i, g) = id.parts();
    let l = fwaits(c);
    let Some(ent) = c.ary(l).iter().copied().find(|&e| c.ary_get(e, 0) == Value::fix(i) && c.ary_get(e, 1) == Value::fix(g)) else { return Ok(()) };
    let (ub, f, t) = (c.ary_get(ent, 2), c.ary_get(ent, 3), c.ary_get(ent, 4));
    let s = th(c, t).scheduler;
    if s == NIL { return Ok(()); }
    funcall(c, s, intern(b"unblock"), &[ub, f], std::ptr::null())?;
    Ok(())
}
/// Wake the oldest waiter on `key` (a fiber blocked through its scheduler: run_unblocks).
#[inline(always)]
fn wake_one(c: &mut Ctx, key: u64) -> Result<(), Esc> { blop::notify_one(c, key); Ok(()) }
/// Wake every waiter on `key`.
#[inline(always)]
fn wake_all(c: &mut Ctx, key: u64) -> Result<(), Esc> { blop::notify(c, key); Ok(()) }
/// Hand a wake-up on `key` to the next waiter while an exception is on its way out.
fn pass_on(c: &mut Ctx, key: u64) {
    let (ek, ev) = (c.esc, c.esc_val);
    if wake_one(c, key).is_err() { c.esc = EscKind::None; }
    c.esc = ek; c.esc_val = ev;
}
/// Kernel#sleep with a scheduler: kernel_sleep(*args).
pub fn sched_kernel_sleep(c: &mut Ctx, s: Value, args: &[Value]) -> Result<(), Esc> {
    funcall(c, s, intern(b"kernel_sleep"), args, std::ptr::null())?;
    Ok(())
}

// ---- Thread ---------------------------------------------------------------------------------

fn status_str(c: &Ctx, t: Value) -> &'static str { let r = th(c, t); if r.dead { "dead" } else if t == cur(c) { "run" } else if r.waiting || !r.started { if r.started { "sleep" } else { "run" } } else { "run" } }
fn inspect_thread(c: &Ctx, t: Value, status: &str) -> String {
    let r = th(c, t);
    let name = if r.name != NIL { format!("@{}", String::from_utf8_lossy(c.str_bytes(r.name))) } else { String::new() };
    let loc = if r.loc.is_empty() { String::new() } else { format!(" {}", r.loc) };
    format!("#<{}:0x{:016x}{}{} {}>", class_path(c, real_class_of(c, t)), t.raw(), name, loc, status)
}
fn th_s_new(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if b.is_null() { return err(c, c.vm().c_thread_error.get(), "must be called with a block"); }
    let p = block_to_proc(c, b)?;
    let t = new_thread(c, slf);
    let args = c.ary_new_from(a);
    // the new thread gets a copy of the creating fiber's storage (rb_fiber_inherit_storage)
    let storage = crate::builtins::fiber_methods::inherited_storage(c)?;
    let start = c.ary_new_from(&[p, args, storage]);
    let loc = c.caller_frame().map(|f| format!("{}:{}", frame_file(f), f.line())).unwrap_or_default();
    let (pri, ractor) = { let m = th(c, cur(c)); (m.priority, m.ractor) };
    let r = th_mut(c, t);
    r.start = start; r.loc = loc; r.priority = pri; r.ractor = ractor;
    let l = LIST.get(c);
    c.ary_push(l, t);
    COOP.fetch_add(1, Relaxed);
    timer_run(true);
    Ok(t)
}
/// The main thread of a new Ractor: runs `p` (ractor.rs), no location in its inspect (as CRuby).
pub fn spawn_ractor_thread(c: &mut Ctx, p: Value, ractor: Value) -> RV {
    let t = new_thread(c, c.vm().c_thread.get());
    let args = c.ary_new();
    let storage = crate::builtins::fiber_methods::inherited_storage(c)?;
    let start = c.ary_new_from(&[p, args, storage]);
    let r = th_mut(c, t);
    r.start = start; r.ractor = ractor;
    let l = LIST.get(c);
    c.ary_push(l, t);
    COOP.fetch_add(1, Relaxed);
    timer_run(true);
    Ok(t)
}
fn th_join(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    thr(c, slf)?;
    if slf == cur(c) { return err(c, c.vm().c_thread_error.get(), "Target thread must not be current thread"); }
    if slf == MAIN.get(c) { return err(c, c.vm().c_thread_error.get(), "Target thread must not be main thread"); }
    let deadline = match a.first() { Some(&t) if t != NIL => { let f = crate::builtins::numeric_methods::to_f(c, t)?; Some(Instant::now() + secs(f)) } _ => None };
    if !block_on_by(c, Hook::Join(slf), NIL, |c| Ok(th(c, slf).dead), Some(slf.raw()), deadline, true)? { return Ok(NIL); }
    let e = th(c, slf).exc;
    if e != NIL { return Err(raise_exc(c, e)); }
    Ok(slf)
}
fn th_value(c: &mut Ctx, slf: Value, _a: &[Value], b: Blk) -> RV { th_join(c, slf, &[], b)?; Ok(th(c, slf).value) }
fn th_current(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(cur(c)) }
/// Thread.main / Thread.list see the current Ractor's threads only.
fn th_main(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let r = current_ractor(c);
    Ok(if r == NIL { MAIN.get(c) } else { crate::builtins::ractor::main_thread(c, r) })
}
fn th_list(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let r = current_ractor(c);
    let mut v = if r == NIL { vec![MAIN.get(c)] } else { vec![] };
    v.extend(c.ary(LIST.get(c)).iter().copied().filter(|&t| !th(c, t).dead && th(c, t).ractor == r));
    Ok(c.ary_from_vec(v))
}
fn th_pass(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let me = cur(c);
    if is_main(c) { run_pass(c)?; blop::pump(c, false); } else {
        check_interrupt_at(c, me, false)?;
        crate::fiber::thread_switch_out(c);
    }
    // not a blocking call: :on_blocking interrupts wait (rb_thread_schedule's RUBY_VM_CHECK_INTS)
    check_interrupt_at(c, me, false)?;
    Ok(NIL)
}
/// Thread.pass
pub fn pass(c: &mut Ctx) -> Result<(), Esc> { th_pass(c, NIL, &[], Blk::NONE).map(|_| ()) }
fn th_stop(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let me = cur(c);
    th_mut(c, me).wake = false;
    block_on(c, |c| Ok(std::mem::take(&mut th_mut(c, me).wake)), None, None, true)?;
    Ok(NIL)
}
fn exit_exception(c: &mut Ctx) -> Value {
    let e = make_exception(c, c.vm().c_system_exit.get(), "exit");
    ivar_set_raw(c, e, crate::symbol::s::status, Value::fix(0));
    e
}
fn kill_thread(c: &mut Ctx, t: Value) -> RV {
    thr(c, t)?;
    if th(c, t).dead { return Ok(t); }
    if t == cur(c) {
        if is_main(c) { let e = exit_exception(c); return Err(raise_exc(c, e)); }
        let e = make_exception(c, C_KILL.get(c), "killed");
        return Err(raise_exc(c, e));
    }
    if t == MAIN.get(c) { let e = exit_exception(c); enqueue_interrupt(c, t, e); return Ok(t); }
    if !th(c, t).started {
        let r = th_mut(c, t);
        r.dead = true;
        let f = r.fiber;
        r.start = NIL;
        list_remove(c, t);
        wake_all(c, t.raw())?;
        if f != NIL { crate::fiber::discard(c, f); }
        return Ok(t);
    }
    enqueue_interrupt(c, t, kill_mark());
    Ok(t)
}
fn th_kill(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { kill_thread(c, slf) }
fn th_s_kill(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { kill_thread(c, a[0]) }
fn th_s_exit(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { let t = cur(c); kill_thread(c, t) }
fn th_raise(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    thr(c, slf)?;
    if th(c, slf).dead { return Ok(NIL); }
    let e = exception_setup(c, a)?;
    if slf == cur(c) { return Err(raise_exc(c, e)); }
    enqueue_interrupt(c, slf, e);
    Ok(NIL)
}
fn th_wakeup(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    thr(c, slf)?;
    if th(c, slf).dead { return err(c, c.vm().c_thread_error.get(), "killed thread"); }
    th_mut(c, slf).wake = true;
    kick(c, slf);
    Ok(slf)
}
fn th_run(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { th_wakeup(c, slf, a, b)?; th_pass(c, slf, a, b)?; Ok(slf) }
fn th_alive(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { thr(c, s)?; Ok(Value::bool(!th(c, s).dead)) }
fn th_stop_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { thr(c, s)?; let r = th(c, s); Ok(Value::bool(r.dead || (r.waiting && s != cur(c)))) }
fn th_status(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    thr(c, s)?;
    let r = th(c, s);
    if r.dead { return Ok(if r.exc != NIL { NIL } else { FALSE }); }
    let st = status_str(c, s);
    Ok(c.str_new(st.as_bytes()))
}
fn th_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { thr(c, s)?; let t = inspect_thread(c, s, status_str(c, s)); Ok(c.str_new(t.as_bytes())) }
/// rb_frozen_error_raise(thread, "can't modify frozen thread locals")
fn frozen_locals(c: &mut Ctx, t: Value) -> Esc {
    let e = make_exception(c, c.vm().c_frozen_error.get(), "can't modify frozen thread locals");
    ivar_set_raw(c, e, crate::symbol::s::receiver, t);
    raise_exc(c, e)
}
/// rb_to_symbol: a Symbol, a String, or what #to_str makes a String.
pub fn key_sym(c: &mut Ctx, k: Value) -> RV {
    if k.is_sym() { return Ok(k); }
    if is_string(k) { return Ok(c.str_to_sym(k)); }
    if respond_to(c, k, intern(b"to_str"), true) {
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
pub fn current_locals(c: &mut Ctx) -> RV { let t = cur(c); locals(c, t) }
fn tvars(c: &mut Ctx, t: Value) -> RV { thr(c, t)?; Ok(th(c, t).tvars) }
fn hash_keys(c: &mut Ctx, h: Value) -> Value { let v: Vec<Value> = c.hash_pairs(h).into_iter().map(|p| p.0).collect(); c.ary_from_vec(v) }
fn th_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let k = key_sym(c, a[0])?;
    if k == recursive_key() && slf == cur(c) { return crate::builtins::kernel::rec_outer(c); }
    let h = locals(c, slf)?; Ok(hash_get_opt(c, h, k)?.unwrap_or(NIL))
}
/// :__recursive_key__ (the recursion guard's Hash) is hidden from key? and keys, as in CRuby
fn recursive_key() -> Value { Value::sym(crate::symbol::intern(b"__recursive_key__")) }
fn th_aset(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let k = key_sym(c, a[0])?;
    if is_frozen(slf) { return Err(frozen_locals(c, slf)); }
    let h = locals(c, slf)?;
    if a[1] == NIL { hash_delete(c, h, k)?; } else { hash_set(c, h, k, a[1])?; }
    Ok(a[1])
}
fn th_key_p(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let k = key_sym(c, a[0])?; if k == recursive_key() { return Ok(FALSE); } let h = locals(c, slf)?; Ok(Value::bool(hash_lookup(c, h, k)?.is_some())) }
fn th_keys(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let h = locals(c, slf)?;
    let v: Vec<Value> = c.hash_pairs(h).into_iter().map(|p| p.0).filter(|&k| k != recursive_key()).collect();
    Ok(c.ary_from_vec(v))
}
fn th_tv_get(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let k = key_sym(c, a[0])?; let h = tvars(c, slf)?; Ok(hash_get_opt(c, h, k)?.unwrap_or(NIL)) }
fn th_tv_set(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let k = key_sym(c, a[0])?;
    if is_frozen(slf) { return Err(frozen_locals(c, slf)); }
    let h = tvars(c, slf)?;
    if a[1] == NIL { hash_delete(c, h, k)?; } else { hash_set(c, h, k, a[1])?; }
    Ok(a[1])
}
fn th_tv_p(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let k = key_sym(c, a[0])?; let h = tvars(c, slf)?; Ok(Value::bool(hash_lookup(c, h, k)?.is_some())) }
fn th_tvs(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let h = tvars(c, slf)?; Ok(hash_keys(c, h)) }
fn th_name(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { thr(c, s)?; Ok(th(c, s).name) }
fn th_set_name(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let n = if a[0] == NIL { NIL } else { let v = expect_str(c, a[0])?; c.str_dup(v) };
    thr(c, s)?;
    th_mut(c, s).name = n;
    Ok(a[0])
}
fn th_report(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { thr(c, s)?; let r = th(c, s).report; Ok(if r == UNDEF { Value::bool(REPORT_ALL.load(Relaxed)) } else { r }) }
fn th_set_report(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { thr(c, s)?; th_mut(c, s).report = Value::bool(a[0].truthy()); Ok(a[0]) }
fn th_abort(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { thr(c, s)?; Ok(Value::bool(th(c, s).abort)) }
fn th_set_abort(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { thr(c, s)?; th_mut(c, s).abort = a[0].truthy(); Ok(a[0]) }
fn th_group(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    thr(c, s)?;
    if th(c, s).dead { return Ok(NIL); }
    init_sync(c);
    Ok(const_get_at(c, const_get_at(c, c.vm().c_object.get(), intern(b"ThreadGroup")).unwrap(), intern(b"Default")).unwrap())
}
fn tg_list(c: &mut Ctx, _s: Value, a: &[Value], b: Blk) -> RV { th_list(c, NIL, a, b) }

// ---- Mutex / Monitor --------------------------------------------------------------------------
// Owned by a fiber, as CRuby's (the ec): another fiber of the same thread does not own it.

static HELD: AtomicUsize = AtomicUsize::new(0); // locks currently owned (skips the heap walk at thread exit)
fn held_dec() { let _ = HELD.fetch_update(Relaxed, Relaxed, |n| Some(n.saturating_sub(1))); }
#[derive(Clone, Copy)]
pub struct RLock { owner: Value, count: usize, monitor: bool }   // owner: a fiber
impl RData for RLock {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.owner); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn lk(c: &Ctx, v: Value) -> RLock { *c.data::<RLock>(v).expect("a Mutex") }
fn lk_mut(c: &mut Ctx, v: Value) -> &mut RLock { c.data_mut::<RLock>(v).expect("a Mutex") }
#[inline]
fn cur_fiber(c: &Ctx) -> Value { crate::fiber::current(c) }
pub fn alloc_lock(c: &mut Ctx, klass: Value) -> Option<Value> {
    let (km, kx) = (c.vm().c_monitor.get(), c.vm().c_mutex.get());
    let mon = km != NIL && crate::builtins::inherits(c, klass, km);
    if mon || (kx != NIL && crate::builtins::inherits(c, klass, kx)) { return Some(c.alloc(klass, Body::Data(Box::new(RLock { owner: NIL, count: 0, monitor: mon })))); }
    None
}
/// A dead thread gives its locks back (those of every fiber of it).
#[inline]
fn release_locks(c: &mut Ctx, t: Value) -> Result<(), Esc> {
    if HELD.load(Relaxed) == 0 { return Ok(()); }
    release_locks_of(c, t)
}
#[cold]
#[inline(never)]
fn release_locks_of(c: &mut Ctx, t: Value) -> Result<(), Esc> {
    let root = if t == MAIN.get(c) { crate::fiber::thread_root_of(c, crate::fiber::main_thread_fiber(c)) } else { th(c, t).fiber };
    if root == NIL { return Ok(()); }
    // each_object does not allocate: the closure's short borrows of c are the only ones
    let mut owned = vec![];
    crate::gc::each_object(|v| { if let Some(l) = c.data::<RLock>(v) { if l.owner != NIL { owned.push(v); } } });
    let mut freed = vec![];
    for v in owned { if crate::fiber::thread_root_of(c, lk(c, v).owner) == root { let l = lk_mut(c, v); l.owner = NIL; l.count = 0; held_dec(); freed.push(v.raw()); } }
    for k in freed { wake_one(c, k)?; }
    Ok(())
}
fn acquire(c: &mut Ctx, m: Value) -> Result<(), Esc> {
    let me = cur_fiber(c);
    let l = lk(c, m);
    if l.owner == me {
        if l.monitor { lk_mut(c, m).count += 1; return Ok(()); }
        return err(c, c.vm().c_thread_error.get(), "deadlock; recursive locking");
    }
    if l.owner != NIL { lock_wait(c, m)?; }
    let l = lk_mut(c, m);
    l.owner = me; l.count = 1;
    HELD.fetch_add(1, Relaxed);
    Ok(())
}
#[inline(never)]
fn lock_wait(c: &mut Ctx, m: Value) -> Result<(), Esc> {
    let o = lk(c, m).owner;
    if scheduler_current(c) == NIL && !IGNORE_DEADLOCK.load(Relaxed) && crate::fiber::thread_root_of(c, o) == crate::fiber::current_thread_root(c) {
        return err(c, c.vm().c_thread_error.get(), "deadlock; lock already owned by another fiber belonging to the same thread");
    }
    block_on_by(c, Hook::Block(m), m, |c| Ok(lk(c, m).owner == NIL), Some(m.raw()), None, true)?;
    Ok(())
}
fn release(c: &mut Ctx, m: Value) -> Result<(), Esc> {
    let l = lk(c, m);
    if l.owner == NIL { return err(c, c.vm().c_thread_error.get(), if l.monitor { "current fiber not owner" } else { "Attempt to unlock a mutex which is not locked" }); }
    if l.owner != cur_fiber(c) { return err(c, c.vm().c_thread_error.get(), if l.monitor { "current fiber not owner" } else { "Attempt to unlock a mutex which is locked by another thread/fiber" }); }
    let l = lk_mut(c, m);
    l.count -= 1;
    if l.count == 0 { l.owner = NIL; held_dec(); wake_one(c, m.raw())?; }
    Ok(())
}
fn mx_lock(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { acquire(c, s)?; Ok(s) }
fn mx_unlock(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { release(c, s)?; Ok(s) }
fn mx_try_lock(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let me = cur_fiber(c);
    let l = lk_mut(c, s);
    if l.owner == NIL { l.owner = me; l.count = 1; HELD.fetch_add(1, Relaxed); return Ok(TRUE); }
    if l.owner == me && l.monitor { l.count += 1; return Ok(TRUE); }
    Ok(FALSE)
}
fn mx_locked(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(lk(c, s).owner != NIL)) }
fn mx_owned(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(lk(c, s).owner == cur_fiber(c))) }
fn mx_sync(c: &mut Ctx, s: Value, _a: &[Value], b: Blk) -> RV {
    if b.is_null() { return err(c, c.vm().c_thread_error.get(), "must be called with a block"); }
    acquire(c, s)?;
    let r = yield_block(c, b, &[]);
    let (ek, ev) = (c.esc, c.esc_val);
    if lk(c, s).owner == cur_fiber(c) { release(c, s)?; }
    if r.is_err() { c.esc = ek; c.esc_val = ev; }
    r
}
/// monitor_synchronize: rb_ensure(yield, monitor_exit), so leaving unowned raises.
fn mon_sync(c: &mut Ctx, s: Value, _a: &[Value], b: Blk) -> RV {
    acquire(c, s)?;
    if b.is_null() { release(c, s)?; return err(c, c.vm().c_local_jump_error.get(), "no block given"); }
    let r = yield_block(c, b, &[]);
    let (ek, ev) = (c.esc, c.esc_val);
    release(c, s)?;
    if r.is_err() { c.esc = ek; c.esc_val = ev; }
    r
}
fn mx_sleep(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let t = a.first().copied().unwrap_or(NIL);
    let dur = if t != NIL { Some(crate::builtins::process::time_interval(c, t)?) } else { None };
    let t0 = Instant::now();
    release(c, s)?;
    let sch = scheduler_current(c);
    let r = if sch != NIL { funcall(c, sch, intern(b"kernel_sleep"), &[t], std::ptr::null()).map(|_| ()) } else { sleep_as(c, dur, true) };
    let (ek, ev) = (c.esc, c.esc_val);
    acquire(c, s)?;
    if r.is_err() { c.esc = ek; c.esc_val = ev; }
    r?;
    Ok(Value::fix(t0.elapsed().as_secs_f64().round() as i64))
}
fn mon_check_owner(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    if lk(c, s).owner != cur_fiber(c) { return err(c, c.vm().c_thread_error.get(), "current fiber not owner"); }
    Ok(NIL)
}
/// Monitor#wait_for_cond(cond, timeout): fully release, wait on cond, re-enter with the old count.
fn mon_wait_for_cond(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let count = lk(c, s).count;
    let l = lk_mut(c, s);
    l.owner = NIL; l.count = 0;
    held_dec();
    wake_one(c, s.raw())?;
    let r = cv_wait_raw(c, a[0], a[1], s);
    let me = cur_fiber(c);
    block_on_by(c, Hook::Block(s), s, |c| Ok(lk(c, s).owner == NIL), Some(s.raw()), None, true)?;
    let l = lk_mut(c, s);
    l.owner = me; l.count = count;
    HELD.fetch_add(1, Relaxed);
    r?;
    Ok(TRUE)
}

// ---- ConditionVariable ------------------------------------------------------------------------

fn cv_waiters(c: &mut Ctx, cv: Value) -> Value {
    let w = ivar_get(c, cv, intern(b"__waiters"));
    if w != NIL { return w; }
    let w = c.ary_new();
    ivar_set_raw(c, cv, intern(b"__waiters"), w);
    w
}
/// With a scheduler the wait is its kernel_sleep(timeout) and a signal its unblock(mutex, fiber), as CRuby's.
fn cv_wait_raw(c: &mut Ctx, cv: Value, timeout: Value, mutex: Value) -> Result<bool, Esc> {
    let deadline = if timeout == NIL { None } else { let f = crate::builtins::numeric_methods::to_f(c, timeout)?; Some(Instant::now() + secs(f)) };
    let token = c.ary_new_from(&[FALSE]);
    let w = cv_waiters(c, cv);
    c.ary_push(w, token);
    let hook = if scheduler_current(c) != NIL { Hook::Sleep(timeout) } else { Hook::None };
    let r = block_on_by(c, hook, mutex, |c| Ok(c.ary_get(token, 0).truthy()), Some(token.raw()), deadline, true);
    c.ary_mut(w).retain(|&x| x != token);
    r
}
fn cv_wait(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let m = a[0];
    let native = c.data::<RLock>(m).is_some();
    if native { release(c, m)?; } else { funcall(c, m, intern(b"unlock"), &[], std::ptr::null())?; }
    let r = cv_wait_raw(c, s, a.get(1).copied().unwrap_or(NIL), m);
    let (ek, ev) = (c.esc, c.esc_val);
    if native { acquire(c, m)?; } else { funcall(c, m, intern(b"lock"), &[], std::ptr::null())?; }
    if r.is_err() { c.esc = ek; c.esc_val = ev; }
    Ok(if r? { s } else { NIL })
}
fn cv_signal(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let w = cv_waiters(c, s);
    if c.ary_len(w) > 0 { let t = c.ary_mut(w).remove(0); c.ary_mut(t)[0] = TRUE; wake_all(c, t.raw())?; }
    Ok(s)
}
fn cv_broadcast(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let w = cv_waiters(c, s);
    let ts = c.ary(w).to_vec();
    for &t in &ts { c.ary_mut(t)[0] = TRUE; }
    c.ary_mut(w).clear();
    // the waits are woken by key: an unblock hook may run Ruby code meanwhile
    let keys: Vec<u64> = ts.iter().map(|t| t.raw()).collect();
    for k in keys { wake_all(c, k)?; }
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
    let k = c.vm().c_queue.get();
    if k != NIL && crate::builtins::inherits(c, klass, k) {
        let items = c.ary_new();
        return Some(c.alloc_with(klass, move || Body::Data(Box::new(RQueue { items, head: 0, closed: false, max: 0, waiting: 0 }))));
    }
    None
}
fn q_init(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    if let Some(&init) = a.first() {
        if !respond_to(c, init, intern(b"to_a"), true) { return type_err(c, format!("can't convert {} into Array", class_name_for_err(c, init))); }
        let l = funcall(c, init, intern(b"to_a"), &[], std::ptr::null())?;
        if !is_array(l) { return type_err(c, format!("can't convert {} to Array ({}#to_a gives {})", class_name_for_err(c, init), class_name_for_err(c, init), class_name_for_err(c, l))); }
        let v = c.ary(l).to_vec();
        let items = c.ary_from_vec(v);
        let q = qu_mut(c, s);
        q.items = items; q.head = 0;
    }
    Ok(NIL)
}
fn sq_init(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
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
fn q_closed_err<T>(c: &mut Ctx) -> Result<T, Esc> { err(c, c.vm().c_closed_queue_error.get(), "queue closed") }
fn q_push(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, 2); }
    let q = qu(c, s);
    if q.closed { return q_closed_err(c); }
    if q.max > 0 && q_len(c, s) >= q.max {
        let (nb, deadline) = q_opts(c, &a[1..])?;
        if nb { return err(c, c.vm().c_thread_error.get(), "queue full"); }
        qu_mut(c, s).waiting += 1;
        let r = block_on_by(c, Hook::Block(s), s, |c| { let q = qu(c, s); Ok(q.closed || q_len(c, s) < q.max) }, Some(space_key(s)), deadline, true);
        qu_mut(c, s).waiting -= 1;
        if !r? { return Ok(NIL); }
        if qu(c, s).closed { return q_closed_err(c); }
    }
    let items = qu(c, s).items;
    c.ary_push(items, a[0]);
    wake_one(c, s.raw())?;
    Ok(s)
}
fn q_pop(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (nb, deadline) = q_opts(c, a)?;
    if q_len(c, s) == 0 {
        if nb { return err(c, c.vm().c_thread_error.get(), "queue empty"); }
        if qu(c, s).closed { return Ok(NIL); }
        qu_mut(c, s).waiting += 1;
        let r = block_on_by(c, Hook::Block(s), s, |c| Ok(qu(c, s).closed || q_len(c, s) > 0), Some(s.raw()), deadline, true);
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
    if q.max > 0 { wake_one(c, space_key(s))?; }
    Ok(x)
}
fn q_size(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(q_len(c, s) as i64)) }
fn q_empty(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(q_len(c, s) == 0)) }
fn q_clear(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let items = qu(c, s).items; c.ary_mut(items).clear(); qu_mut(c, s).head = 0; wake_all(c, space_key(s))?; Ok(s) }
fn q_close(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { qu_mut(c, s).closed = true; wake_all(c, s.raw())?; wake_all(c, space_key(s))?; Ok(s) }
fn q_closed(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(qu(c, s).closed)) }
fn q_num_waiting(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(qu(c, s).waiting as i64)) }
fn q_freeze(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { type_err(c, format!("cannot freeze #<{}:0x{:016x}>", class_path(c, real_class_of(c, s)), s.raw())) }
fn sq_max(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(qu(c, s).max as i64)) }
fn sq_set_max(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let m = num_to_long(c, a[0])?;
    if m <= 0 { return arg_err(c, "queue size must be positive"); }
    qu_mut(c, s).max = m as usize;
    wake_all(c, space_key(s))?;
    Ok(a[0])
}

/// Boot: Thread and the main thread.
pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let ob = vmr.c_object.get();
    let th = define_class(c, "Thread", ob);
    class_set_alloc(c, th, AllocKind::None);
    vmr.c_thread.set(th);
    let list = c.alloc(vmr.c_array.get(), Body::Array(crate::avec::AVec::new()));
    LIST.set_vm(vmr, list);
    let l = c.hash_new();
    let t = c.hash_new();
    let main = c.alloc_with(th, move || { let mut r = RThread::blank(l, t); r.started = true; Body::Data(Box::new(r)) });
    MAIN.set_vm(vmr, main);
    CURRENT.set_vm(vmr, main);
    let kill = new_class(c, vmr.c_exception.get());
    C_KILL.set_vm(vmr, kill);
    let fatal = new_class(c, vmr.c_exception.get());
    name_class(c, fatal, ob, intern(b"fatal"));
    C_FATAL.set_vm(vmr, fatal);
    let ds = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_singleton_cfunc(c, th, n, f, a);
    ds(c, "new", th_s_new, -1); ds(c, "start", th_s_new, -1); ds(c, "fork", th_s_new, -1); ds(c, "current", th_current, 0); ds(c, "main", th_main, 0); ds(c, "list", th_list, 0);
    ds(c, "pass", th_pass, 0); ds(c, "stop", th_stop, 0); ds(c, "kill", th_s_kill, 1); ds(c, "exit", th_s_exit, 0);
    ds(c, "abort_on_exception", |_c, _s, _a, _b| Ok(Value::bool(ABORT_ALL.load(Relaxed))), 0);
    ds(c, "abort_on_exception=", |_c, _s, a, _b| { ABORT_ALL.store(a[0].truthy(), Relaxed); Ok(a[0]) }, 1);
    ds(c, "report_on_exception", |_c, _s, _a, _b| Ok(Value::bool(REPORT_ALL.load(Relaxed))), 0);
    ds(c, "report_on_exception=", |_c, _s, a, _b| { REPORT_ALL.store(a[0].truthy(), Relaxed); Ok(a[0]) }, 1);
    ds(c, "handle_interrupt", th_s_handle_interrupt, -1);
    ds(c, "pending_interrupt?", |c, _s, a, _b| { let t = cur(c); pending_interrupt_p(c, t, a) }, -1);
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, th, n, f, a);
    d(c, "join", th_join, -1); d(c, "value", th_value, 0); d(c, "[]", th_aref, 1); d(c, "[]=", th_aset, 2); d(c, "key?", th_key_p, 1); d(c, "keys", th_keys, 0);
    d(c, "thread_variable_get", th_tv_get, 1); d(c, "thread_variable_set", th_tv_set, 2); d(c, "thread_variable?", th_tv_p, 1); d(c, "thread_variables", th_tvs, 0);
    d(c, "alive?", th_alive, 0); d(c, "stop?", th_stop_p, 0); d(c, "status", th_status, 0); d(c, "inspect", th_inspect, 0); d(c, "to_s", th_inspect, 0);
    d(c, "name", th_name, 0); d(c, "name=", th_set_name, 1); d(c, "kill", th_kill, 0); d(c, "terminate", th_kill, 0); d(c, "exit", th_kill, 0); d(c, "raise", th_raise, -1);
    d(c, "wakeup", th_wakeup, 0); d(c, "run", th_run, 0); d(c, "report_on_exception", th_report, 0); d(c, "report_on_exception=", th_set_report, 1);
    d(c, "abort_on_exception", th_abort, 0); d(c, "abort_on_exception=", th_set_abort, 1); d(c, "group", th_group, 0);
    d(c, "priority", th_priority, 0); d(c, "priority=", th_set_priority, 1);
    d(c, "backtrace", |c, _s, _a, _b| Ok(c.ary_new()), -1); d(c, "backtrace_locations", |c, _s, _a, _b| Ok(c.ary_new()), -1);
    d(c, "native_thread_id", |_c, _s, _a, _b| Ok(Value::fix(crate::builtins::sys::getpid() as i64)), 0);
    d(c, "pending_interrupt?", |c, s, a, _b| { thr(c, s)?; pending_interrupt_p(c, s, a) }, -1);
    d(c, "fetch", |c, s, a, b| { check_argc(c, a, 1, 2)?; let k = key_sym(c, a[0])?; let h = locals(c, s)?; match hash_get_opt(c, h, k)? { Some(v) => Ok(v), None if !b.is_null() => yield1(c, b, a[0]), None if a.len() > 1 => Ok(a[1]), None => { let d = inspect_str(c, k)?; err(c, c.vm().c_key_error.get(), format!("key not found: {}", d)) } } }, -1);
    for n in ["Mutex", "Queue", "SizedQueue", "ConditionVariable"] { crate::builtins::prelude::lazy_const(c, th, n, "saruby/sync"); crate::builtins::prelude::lazy_const(c, ob, n, "saruby/sync"); }
    crate::builtins::prelude::lazy_const(c, ob, "Monitor", "saruby/sync");
    crate::builtins::prelude::lazy_const(c, ob, "ThreadGroup", "saruby/sync");
}
/// ThreadGroup, Mutex, Monitor, Queue, SizedQueue, ConditionVariable (the "saruby/sync" feature).
pub fn init_sync(c: &mut Ctx) {
    if c.vm().c_mutex.get() != NIL { return; }
    let (ob, th) = (c.vm().c_object.get(), c.vm().c_thread.get());
    let tg = define_class(c, "ThreadGroup", ob);
    let dg = new_object(c, tg);
    const_set(c, tg, intern(b"Default"), dg);
    define_cfunc(c, tg, "list", tg_list, 0);
    define_cfunc(c, tg, "add", |_c, s, _a, _b| Ok(s), 1);
    define_cfunc(c, tg, "enclosed?", |_c, _s, _a, _b| Ok(FALSE), 0);
    // Mutex / Monitor
    let mx = define_class_under(c, th, "Mutex", ob);
    class_set_alloc(c, mx, AllocKind::None);
    const_set(c, ob, intern(b"Mutex"), mx);
    c.vm().c_mutex.set(mx);
    let mon = define_class(c, "Monitor", ob);
    class_set_alloc(c, mon, AllocKind::None);
    c.vm().c_monitor.set(mon);
    for (k, pre) in [(mx, ""), (mon, "mon_")] {
        define_cfunc(c, k, "synchronize", if k == mon { mon_sync } else { mx_sync }, 0);
        define_cfunc(c, k, &format!("{}locked?", pre), mx_locked, 0);
        define_cfunc(c, k, &format!("{}owned?", pre), mx_owned, 0);
    }
    define_cfunc(c, mx, "lock", mx_lock, 0); define_cfunc(c, mx, "unlock", mx_unlock, 0); define_cfunc(c, mx, "try_lock", mx_try_lock, 0); define_cfunc(c, mx, "sleep", mx_sleep, -1);
    define_cfunc(c, mon, "enter", mx_lock, 0); define_cfunc(c, mon, "exit", mx_unlock, 0); define_cfunc(c, mon, "try_enter", mx_try_lock, 0);
    define_cfunc(c, mon, "new_cond", |c, s, a, b| { let f = c.str_new(b"monitor"); crate::builtins::kernel::require_feature(c, f, false)?; funcall(c, s, intern(b"new_cond"), a, b) }, 0);
    define_cfunc(c, mon, "mon_synchronize", mon_sync, 0); define_cfunc(c, mon, "mon_check_owner", mon_check_owner, 0); define_cfunc(c, mon, "wait_for_cond", mon_wait_for_cond, 2);
    // Queue / SizedQueue / ConditionVariable
    let q = define_class_under(c, th, "Queue", ob);
    class_set_alloc(c, q, AllocKind::None);
    const_set(c, ob, intern(b"Queue"), q);
    c.vm().c_queue.set(q);
    define_private_cfunc(c, q, "initialize", q_init, -1);
    for n in ["push", "<<", "enq"] { define_cfunc(c, q, n, q_push, -1); }
    for n in ["pop", "shift", "deq"] { define_cfunc(c, q, n, q_pop, -1); }
    for n in ["size", "length"] { define_cfunc(c, q, n, q_size, 0); }
    define_cfunc(c, q, "empty?", q_empty, 0); define_cfunc(c, q, "clear", q_clear, 0); define_cfunc(c, q, "close", q_close, 0); define_cfunc(c, q, "closed?", q_closed, 0);
    define_cfunc(c, q, "num_waiting", q_num_waiting, 0); define_cfunc(c, q, "freeze", q_freeze, 0);
    let sq = define_class_under(c, th, "SizedQueue", q);
    const_set(c, ob, intern(b"SizedQueue"), sq);
    define_private_cfunc(c, sq, "initialize", sq_init, 1);
    define_cfunc(c, sq, "max", sq_max, 0); define_cfunc(c, sq, "max=", sq_set_max, 1);
    let cv = define_class_under(c, th, "ConditionVariable", ob);
    const_set(c, ob, intern(b"ConditionVariable"), cv);
    define_cfunc(c, cv, "wait", cv_wait, -1); define_cfunc(c, cv, "signal", cv_signal, 0); define_cfunc(c, cv, "broadcast", cv_broadcast, 0);
}
