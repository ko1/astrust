// Green threads: each Thread is a fiber chain switched by the main thread's scheduler at
// blocking points (join, Queue, Mutex, ConditionVariable, sleep, pipe reads, Thread.pass).
// There is no preemption: a thread runs until it blocks or ends.
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::hash::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::time::{Duration, Instant};

pub struct RThread {
    fiber: Value, inner: Value, started: bool, dead: bool, value: Value, exc: Value, locals: Value, tvars: Value, name: Value,
    deadline: Option<Instant>, waiting: bool, io_wait: bool, wake: bool, pending: Value, killed: bool, progress: bool,
    report: Value, abort: bool, loc: String,
}
impl RData for RThread {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { for v in [&mut self.fiber, &mut self.inner, &mut self.value, &mut self.exc, &mut self.locals, &mut self.tvars, &mut self.name, &mut self.pending, &mut self.report] { m.visit(v); } }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
static mut LIST: Value = NIL;      // Array of live non-main threads
static mut MAIN: Value = NIL;
static mut CURRENT: Value = NIL;
static mut C_KILL: Value = NIL;    // internal exception unwinding a killed thread
static mut C_FATAL: Value = NIL;   // the unnamed-constant `fatal` exception class
static mut ABORT_ALL: bool = false;
static mut REPORT_ALL: bool = true;

fn th(v: Value) -> &'static mut RThread { rdata::<RThread>(v).unwrap() }
fn thr(c: &mut Ctx, v: Value) -> Result<&'static mut RThread, Esc> { match rdata::<RThread>(v) { Some(t) => Ok(t), None => type_err(c, "wrong argument type (expected Thread)") } }
fn cur() -> Value { unsafe { CURRENT } }
fn is_main() -> bool { unsafe { CURRENT == MAIN } }
/// Other threads exist (or we are inside one): blocking calls must go through the scheduler.
pub fn active() -> bool { unsafe { CURRENT != MAIN || ary_len(LIST) > 0 } }
fn new_thread(klass: Value) -> Value {
    crate::gc::alloc(klass, Body::Data(Box::new(RThread { fiber: NIL, inner: NIL, started: false, dead: false, value: NIL, exc: NIL, locals: hash_new(), tvars: hash_new(), name: NIL,
        deadline: None, waiting: false, io_wait: false, wake: false, pending: NIL, killed: false, progress: false, report: UNDEF, abort: false, loc: String::new() })))
}
pub fn alloc_data(_c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if vm().c_thread != NIL && crate::builtins::inherits(klass, vm().c_thread) { return Ok(Some(new_thread(klass))); }
    Ok(None)
}

fn check_interrupt(c: &mut Ctx, t: Value) -> Result<(), Esc> {
    let r = th(t);
    if r.pending != NIL { let e = r.pending; r.pending = NIL; return Err(raise_exc(c, e)); }
    if r.killed { r.killed = false; let e = make_exception(unsafe { C_KILL }, "killed"); return Err(raise_exc(c, e)); }
    Ok(())
}
/// A thread's root fiber returned: record value / exception.
fn finish_thread(c: &mut Ctx, t: Value) -> Result<(), Esc> {
    let r = th(t);
    let (is_err, v) = crate::fiber::take_result(r.fiber);
    r.dead = true; r.progress = true;
    rary(unsafe { LIST }).retain(|&x| x != t);
    release_locks(t);
    if !is_err { r.value = v; return Ok(()); }
    if is_kind_of(v, unsafe { C_KILL }) { return Ok(()); }
    if is_kind_of(v, vm().c_system_exit) { return Err(raise_exc(c, v)); }
    r.exc = v;
    let report = if r.report == UNDEF { unsafe { REPORT_ALL } } else { r.report.truthy() };
    if report {
        let head = format!("{} terminated with exception (report_on_exception is true):\n", inspect_thread(t, "run"));
        let msg = crate::builtins::exception::full_message(c, v, false).unwrap_or_default();
        err_write(format!("{}{}", head, msg).as_bytes());
    }
    if r.abort || unsafe { ABORT_ALL } { return Err(raise_exc(c, v)); }
    Ok(())
}
/// Give every live thread one turn; true when any of them got somewhere.
fn run_pass(c: &mut Ctx) -> Result<bool, Esc> {
    let list: Vec<Value> = ary_to_vec(unsafe { LIST });
    let mut progressed = false;
    for t in list {
        let r = th(t);
        if r.dead { continue; }
        r.progress = !r.started;
        r.started = true;
        unsafe { CURRENT = t; }
        crate::fiber::thread_switch_in(r.fiber, r.inner);
        unsafe { CURRENT = MAIN; }
        let r = th(t);
        if crate::fiber::is_done(r.fiber) { finish_thread(c, t)?; } else { r.inner = crate::fiber::last_switched_out(); }
        progressed |= th(t).progress;
    }
    Ok(progressed)
}
/// Duration of a Ruby time interval (huge / NaN values mean "practically forever").
fn secs(f: f64) -> Duration { Duration::from_secs_f64(if f.is_finite() { f.clamp(0.0, 1e9) } else { 1e9 }) }
fn sleep_real(d: Duration) {
    let ts = crate::builtins::sys::Timespec { sec: d.as_secs() as i64, nsec: d.subsec_nanos() as i64 };
    unsafe { crate::builtins::sys::nanosleep(&ts, std::ptr::null_mut()); }
}
/// Block the current thread until cond holds (Ok(true)) or the deadline passes (Ok(false)).
pub fn block_on(c: &mut Ctx, mut cond: impl FnMut(&mut Ctx) -> Result<bool, Esc>, deadline: Option<Instant>, deadlockable: bool, io: bool) -> Result<bool, Esc> {
    let me = cur();
    if !is_main() {
        loop {
            check_interrupt(c, me)?;
            if cond(c)? { th(me).progress = true; return Ok(true); }
            if deadline.map(|d| Instant::now() >= d).unwrap_or(false) { th(me).progress = true; return Ok(false); }
            let r = th(me);
            r.waiting = true; r.deadline = deadline; r.io_wait = io;
            crate::fiber::thread_switch_out();
            let r = th(me);
            r.waiting = false; r.deadline = None; r.io_wait = false;
        }
    }
    loop {
        check_interrupt(c, me)?;
        crate::builtins::process::run_pending_signals(c)?;
        if cond(c)? { return Ok(true); }
        if deadline.map(|d| Instant::now() >= d).unwrap_or(false) { return Ok(false); }
        th(me).waiting = true;
        let p = run_pass(c);
        th(me).waiting = false;
        if p? { continue; }
        if cond(c)? { return Ok(true); }
        // nobody can run: wait for the nearest timer / fd, or report the deadlock
        let mut next = deadline; let mut any_io = io;
        for t in ary_to_vec(unsafe { LIST }) { let r = th(t); if let Some(d) = r.deadline { next = Some(next.map(|n| n.min(d)).unwrap_or(d)); } any_io |= r.io_wait; }
        let now = Instant::now();
        match next {
            Some(n) => { let d = n.saturating_duration_since(now); sleep_real(if any_io { d.min(Duration::from_millis(1)) } else { d }); }
            None if any_io => sleep_real(Duration::from_millis(1)),
            None if deadlockable => {
                return err(c, unsafe { C_FATAL }, "No live threads left. Deadlock?");
            }
            None => sleep_real(Duration::from_secs(3600)),
        }
    }
}
/// Kernel#sleep when threads are in play.
pub fn sleep(c: &mut Ctx, dur: Option<f64>) -> Result<(), Esc> {
    let me = cur();
    th(me).wake = false;
    let deadline = dur.map(|d| Instant::now() + secs(d));
    block_on(c, |_| Ok(std::mem::take(&mut th(me).wake)), deadline, false, false)?;
    Ok(())
}
/// Wait until fd is readable, letting other threads run meanwhile.
pub fn wait_readable(c: &mut Ctx, fd: i32) -> Result<(), Esc> {
    let ready = |_: &mut Ctx| { let mut p = crate::builtins::sys::PollFd { fd, events: 1, revents: 0 }; Ok(unsafe { crate::builtins::sys::poll(&mut p, 1, 0) } != 0) };
    block_on(c, ready, None, false, true)?;
    Ok(())
}

// ---- Thread ---------------------------------------------------------------------------------

fn status_str(t: Value) -> &'static str { let r = th(t); if r.dead { "dead" } else if t == cur() { "run" } else if r.waiting || !r.started { if r.started { "sleep" } else { "run" } } else { "run" } }
fn inspect_thread(t: Value, status: &str) -> String {
    let r = th(t);
    let name = if r.name != NIL { format!("@{}", String::from_utf8_lossy(str_bytes(r.name))) } else { String::new() };
    let loc = if r.loc.is_empty() { String::new() } else { format!(" {}", r.loc) };
    format!("#<{}:0x{:016x}{}{} {}>", class_path(real_class_of(t)), t.raw(), name, loc, status)
}
fn th_s_new(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return err(c, vm().c_thread_error, "must be called with a block"); }
    let p = block_to_proc(c, b)?;
    let t = new_thread(slf);
    let f = crate::fiber::new_thread_fiber(p, ary_new_from(a));
    let r = th(t);
    r.fiber = f; r.inner = f;
    let caller = unsafe { &*(*c.frame).prev };
    r.loc = format!("{}:{}", frame_file(caller), caller.line);
    ary_push(unsafe { LIST }, t);
    Ok(t)
}
fn th_join(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    thr(c, slf)?;
    if slf == cur() { return err(c, vm().c_thread_error, "Target thread must not be current thread"); }
    if slf == unsafe { MAIN } { return err(c, vm().c_thread_error, "Target thread must not be main thread"); }
    let deadline = match a.first() { Some(&t) if t != NIL => { let f = crate::builtins::numeric_methods::to_f(c, t)?; Some(Instant::now() + secs(f)) } _ => None };
    if !block_on(c, |_| Ok(th(slf).dead), deadline, true, false)? { return Ok(NIL); }
    let e = th(slf).exc;
    if e != NIL { return Err(raise_exc(c, e)); }
    Ok(slf)
}
fn th_value(c: &mut Ctx, slf: Value, _a: &[Value], b: *const Block) -> RV { th_join(c, slf, &[], b)?; Ok(th(slf).value) }
fn th_current(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(cur()) }
fn th_main(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(unsafe { MAIN }) }
fn th_list(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let out = ary_new_from(&[unsafe { MAIN }]);
    for t in ary_to_vec(unsafe { LIST }) { if !th(t).dead { ary_push(out, t); } }
    Ok(out)
}
fn th_pass(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    if is_main() { run_pass(c)?; return Ok(NIL); }
    let mut n = 0;
    block_on(c, |_| { n += 1; Ok(n > 1) }, None, false, false)?;
    Ok(NIL)
}
fn th_stop(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let me = cur();
    th(me).wake = false;
    block_on(c, |_| Ok(std::mem::take(&mut th(me).wake)), None, true, false)?;
    Ok(NIL)
}
fn kill_thread(c: &mut Ctx, t: Value) -> RV {
    let r = thr(c, t)?;
    if r.dead { return Ok(t); }
    if t == cur() {
        if is_main() { let e = make_exception(vm().c_system_exit, "exit"); ivar_set_raw(e, crate::symbol::s::status, Value::fix(0)); return Err(raise_exc(c, e)); }
        let e = make_exception(unsafe { C_KILL }, "killed");
        return Err(raise_exc(c, e));
    }
    if t == unsafe { MAIN } { let e = make_exception(vm().c_system_exit, "exit"); ivar_set_raw(e, crate::symbol::s::status, Value::fix(0)); r.pending = e; return Ok(t); }
    if !r.started { r.dead = true; rary(unsafe { LIST }).retain(|&x| x != t); crate::fiber::discard(r.fiber); return Ok(t); }
    r.killed = true; r.wake = true;
    Ok(t)
}
fn th_kill(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { kill_thread(c, slf) }
fn th_s_kill(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { kill_thread(c, a[0]) }
fn th_s_exit(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { kill_thread(c, cur()) }
fn th_raise(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let e = if a.is_empty() { make_exception(vm().c_runtime_error, "unhandled exception") } else { make_exception_from_args(c, a)? };
    let r = thr(c, slf)?;
    if r.dead { return Ok(NIL); }
    if slf == cur() { return Err(raise_exc(c, e)); }
    r.pending = e; r.wake = true;
    Ok(NIL)
}
fn th_wakeup(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let r = thr(c, slf)?;
    if r.dead { return err(c, vm().c_thread_error, "killed thread"); }
    r.wake = true;
    Ok(slf)
}
fn th_run(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { th_wakeup(c, slf, a, b)?; th_pass(c, slf, a, b)?; Ok(slf) }
fn th_alive(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(!thr(c, s)?.dead)) }
fn th_stop_p(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let r = thr(c, s)?; Ok(Value::bool(r.dead || (r.waiting && s != cur()))) }
fn th_status(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let r = thr(c, s)?;
    if r.dead { return Ok(if r.exc != NIL { NIL } else { FALSE }); }
    Ok(str_new(status_str(s).as_bytes()))
}
fn th_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { thr(c, s)?; Ok(str_new(inspect_thread(s, status_str(s)).as_bytes())) }
fn key_sym(c: &mut Ctx, k: Value) -> RV {
    if k.is_sym() { return Ok(k); }
    if is_string(k) { return Ok(str_to_sym(k)); }
    let d = inspect_str(c, k)?;
    type_err(c, format!("{} is not a symbol nor a string", d))
}
fn th_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let k = key_sym(c, a[0])?; let h = thr(c, slf)?.locals; Ok(hash_get_opt(c, h, k)?.unwrap_or(NIL)) }
fn th_aset(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let k = key_sym(c, a[0])?;
    if is_frozen(slf) { return Err(raise_frozen(c, slf)); }
    let h = thr(c, slf)?.locals;
    if a[1] == NIL { hash_delete(c, h, k)?; } else { hash_set(c, h, k, a[1])?; }
    Ok(a[1])
}
fn th_key_p(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let k = key_sym(c, a[0])?; let h = thr(c, slf)?.locals; Ok(Value::bool(hash_lookup(c, h, k)?.is_some())) }
fn th_keys(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let h = thr(c, slf)?.locals; Ok(ary_from_vec(hash_pairs(h).into_iter().map(|p| p.0).collect())) }
fn th_tv_get(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let k = key_sym(c, a[0])?; let h = thr(c, slf)?.tvars; Ok(hash_get_opt(c, h, k)?.unwrap_or(NIL)) }
fn th_tv_set(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let k = key_sym(c, a[0])?;
    if is_frozen(slf) { return Err(raise_frozen(c, slf)); }
    let h = thr(c, slf)?.tvars;
    if a[1] == NIL { hash_delete(c, h, k)?; } else { hash_set(c, h, k, a[1])?; }
    Ok(a[1])
}
fn th_tv_p(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let k = key_sym(c, a[0])?; let h = thr(c, slf)?.tvars; Ok(Value::bool(hash_lookup(c, h, k)?.is_some())) }
fn th_tvs(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let h = thr(c, slf)?.tvars; Ok(ary_from_vec(hash_pairs(h).into_iter().map(|p| p.0).collect())) }
fn th_name(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(thr(c, s)?.name) }
fn th_set_name(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { let n = if a[0] == NIL { NIL } else { let v = expect_str(c, a[0])?; str_dup(v) }; thr(c, s)?.name = n; Ok(a[0]) }
fn th_report(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let r = thr(c, s)?; Ok(if r.report == UNDEF { Value::bool(unsafe { REPORT_ALL }) } else { r.report }) }
fn th_set_report(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { thr(c, s)?.report = Value::bool(a[0].truthy()); Ok(a[0]) }
fn th_abort(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(thr(c, s)?.abort)) }
fn th_set_abort(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { thr(c, s)?.abort = a[0].truthy(); Ok(a[0]) }
fn th_group(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    if thr(c, s)?.dead { return Ok(NIL); }
    init_sync();
    Ok(const_get_at(const_get_at(vm().c_object, intern(b"ThreadGroup")).unwrap(), intern(b"Default")).unwrap())
}
fn tg_list(c: &mut Ctx, _s: Value, a: &[Value], b: *const Block) -> RV { th_list(c, NIL, a, b) }

// ---- Mutex / Monitor --------------------------------------------------------------------------

static mut HELD: usize = 0; // locks currently owned (skips the heap walk at thread exit)
pub struct RLock { owner: Value, count: usize, monitor: bool }
impl RData for RLock {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.owner); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn lk(v: Value) -> &'static mut RLock { rdata::<RLock>(v).unwrap() }
pub fn alloc_lock(klass: Value) -> Option<Value> {
    let vm = vm();
    let mon = vm.c_monitor != NIL && crate::builtins::inherits(klass, vm.c_monitor);
    if mon || (vm.c_mutex != NIL && crate::builtins::inherits(klass, vm.c_mutex)) { return Some(crate::gc::alloc(klass, Body::Data(Box::new(RLock { owner: NIL, count: 0, monitor: mon })))); }
    None
}
/// A dead thread gives its locks back.
fn release_locks(t: Value) {
    if unsafe { HELD } == 0 { return; }
    crate::gc::each_object(|v| { if let Some(l) = rdata::<RLock>(v) { if l.owner == t { l.owner = NIL; l.count = 0; unsafe { HELD = HELD.saturating_sub(1); } } } });
}
fn acquire(c: &mut Ctx, m: Value) -> Result<(), Esc> {
    let me = cur();
    let l = lk(m);
    if l.owner == me {
        if l.monitor { l.count += 1; return Ok(()); }
        return err(c, vm().c_thread_error, "deadlock; recursive locking");
    }
    block_on(c, |_| Ok(lk(m).owner == NIL || th(lk(m).owner).dead), None, true, false)?;
    let l = lk(m);
    l.owner = me; l.count = 1;
    unsafe { HELD += 1; }
    Ok(())
}
fn release(c: &mut Ctx, m: Value) -> Result<(), Esc> {
    let l = lk(m);
    if l.owner == NIL { return err(c, vm().c_thread_error, if l.monitor { "current thread not owner" } else { "Attempt to unlock a mutex which is not locked" }); }
    if l.owner != cur() { return err(c, vm().c_thread_error, if l.monitor { "current thread not owner" } else { "Attempt to unlock a mutex which is locked by another thread/fiber" }); }
    l.count -= 1;
    if l.count == 0 { l.owner = NIL; unsafe { HELD = HELD.saturating_sub(1); } }
    Ok(())
}
fn mx_lock(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { acquire(c, s)?; Ok(s) }
fn mx_unlock(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { release(c, s)?; Ok(s) }
fn mx_try_lock(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let l = lk(s);
    let me = cur();
    if l.owner == NIL { l.owner = me; l.count = 1; unsafe { HELD += 1; } return Ok(TRUE); }
    if l.owner == me && l.monitor { l.count += 1; return Ok(TRUE); }
    Ok(FALSE)
}
fn mx_locked(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(lk(s).owner != NIL)) }
fn mx_owned(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(lk(s).owner == cur())) }
fn mx_sync(c: &mut Ctx, s: Value, _a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return err(c, vm().c_thread_error, "must be called with a block"); }
    acquire(c, s)?;
    let r = yield_block(c, b, &[]);
    let (ek, ev) = (c.esc, c.esc_val);
    if lk(s).owner == cur() { release(c, s)?; }
    if r.is_err() { c.esc = ek; c.esc_val = ev; }
    r
}
fn mx_sleep(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let dur = match a.first() { Some(&t) if t != NIL => Some(crate::builtins::numeric_methods::to_f(c, t)?), _ => None };
    let t0 = Instant::now();
    release(c, s)?;
    let r = sleep(c, dur);
    acquire(c, s)?;
    r?;
    Ok(Value::fix(t0.elapsed().as_secs_f64().round() as i64))
}
fn mon_check_owner(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    if lk(s).owner != cur() { return err(c, vm().c_thread_error, "current thread not owner"); }
    Ok(NIL)
}
/// Monitor#wait_for_cond(cond, timeout): fully release, wait on cond, re-enter with the old count.
fn mon_wait_for_cond(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let count = lk(s).count;
    lk(s).owner = NIL; lk(s).count = 0;
    unsafe { HELD = HELD.saturating_sub(1); }
    let r = cv_wait_raw(c, a[0], a[1]);
    let me = cur();
    block_on(c, |_| Ok(lk(s).owner == NIL), None, true, false)?;
    lk(s).owner = me; lk(s).count = count;
    unsafe { HELD += 1; }
    r?;
    Ok(TRUE)
}

// ---- ConditionVariable ------------------------------------------------------------------------

fn cv_waiters(cv: Value) -> Value {
    let w = ivar_get(cv, intern(b"__waiters"));
    if w != NIL { return w; }
    let w = ary_new();
    ivar_set_raw(cv, intern(b"__waiters"), w);
    w
}
fn cv_wait_raw(c: &mut Ctx, cv: Value, timeout: Value) -> Result<bool, Esc> {
    let deadline = if timeout == NIL { None } else { let f = crate::builtins::numeric_methods::to_f(c, timeout)?; Some(Instant::now() + secs(f)) };
    let token = ary_new_from(&[FALSE]);
    let w = cv_waiters(cv);
    ary_push(w, token);
    let r = block_on(c, |_| Ok(ary_get(token, 0).truthy()), deadline, true, false);
    rary(w).retain(|&x| x != token);
    r
}
fn cv_wait(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let m = a[0];
    let native = rdata::<RLock>(m).is_some();
    if native { release(c, m)?; } else { funcall(c, m, intern(b"unlock"), &[], std::ptr::null())?; }
    let r = cv_wait_raw(c, s, a.get(1).copied().unwrap_or(NIL));
    if native { acquire(c, m)?; } else { funcall(c, m, intern(b"lock"), &[], std::ptr::null())?; }
    Ok(if r? { s } else { NIL })
}
fn cv_signal(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let w = cv_waiters(s);
    if ary_len(w) > 0 { let t = rary(w).remove(0); rary(t)[0] = TRUE; }
    Ok(s)
}
fn cv_broadcast(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let w = cv_waiters(s);
    for t in ary_to_vec(w) { rary(t)[0] = TRUE; }
    rary(w).clear();
    Ok(s)
}

// ---- Queue / SizedQueue -------------------------------------------------------------------------

pub struct RQueue { items: Value, closed: bool, max: usize, waiting: usize }
impl RData for RQueue {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.items); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn qu(v: Value) -> &'static mut RQueue { rdata::<RQueue>(v).unwrap() }
pub fn alloc_queue(klass: Value) -> Option<Value> {
    if vm().c_queue != NIL && crate::builtins::inherits(klass, vm().c_queue) { return Some(crate::gc::alloc(klass, Body::Data(Box::new(RQueue { items: ary_new(), closed: false, max: 0, waiting: 0 })))); }
    None
}
fn q_init(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    if let Some(&init) = a.first() {
        if !respond_to(init, intern(b"to_a"), true) { return type_err(c, format!("can't convert {} into Array", class_name_for_err(init))); }
        let l = funcall(c, init, intern(b"to_a"), &[], std::ptr::null())?;
        if !is_array(l) { return type_err(c, format!("can't convert {} to Array ({}#to_a gives {})", class_name_for_err(init), class_name_for_err(init), class_name_for_err(l))); }
        qu(s).items = ary_new_from(rary(l));
    }
    Ok(NIL)
}
fn sq_init(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let m = num_to_long(c, a[0])?;
    if m <= 0 { return arg_err(c, "queue size must be positive"); }
    qu(s).max = m as usize;
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
fn q_closed_err<T>(c: &mut Ctx) -> Result<T, Esc> { err(c, vm().c_closed_queue_error, "queue closed") }
fn q_push(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, 2); }
    let q = qu(s);
    if q.closed { return q_closed_err(c); }
    if q.max > 0 && ary_len(q.items) >= q.max {
        let (nb, deadline) = q_opts(c, &a[1..])?;
        if nb { return err(c, vm().c_thread_error, "queue full"); }
        qu(s).waiting += 1;
        let r = block_on(c, |_| Ok(qu(s).closed || ary_len(qu(s).items) < qu(s).max), deadline, true, false);
        qu(s).waiting -= 1;
        if !r? { return Ok(NIL); }
        if qu(s).closed { return q_closed_err(c); }
    }
    ary_push(qu(s).items, a[0]);
    Ok(s)
}
fn q_pop(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let (nb, deadline) = q_opts(c, a)?;
    if ary_len(qu(s).items) == 0 {
        if nb { return err(c, vm().c_thread_error, "queue empty"); }
        if qu(s).closed { return Ok(NIL); }
        qu(s).waiting += 1;
        let r = block_on(c, |_| Ok(qu(s).closed || ary_len(qu(s).items) > 0), deadline, true, false);
        qu(s).waiting -= 1;
        if !r? || ary_len(qu(s).items) == 0 { return Ok(NIL); }
    }
    Ok(rary(qu(s).items).remove(0))
}
fn q_size(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(ary_len(qu(s).items) as i64)) }
fn q_empty(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(ary_len(qu(s).items) == 0)) }
fn q_clear(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { rary(qu(s).items).clear(); Ok(s) }
fn q_close(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { qu(s).closed = true; Ok(s) }
fn q_closed(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(qu(s).closed)) }
fn q_num_waiting(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(qu(s).waiting as i64)) }
fn q_freeze(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { type_err(c, format!("cannot freeze #<{}:0x{:016x}>", class_path(real_class_of(s)), s.raw())) }
fn sq_max(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(qu(s).max as i64)) }
fn sq_set_max(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let m = num_to_long(c, a[0])?;
    if m <= 0 { return arg_err(c, "queue size must be positive"); }
    qu(s).max = m as usize;
    Ok(a[0])
}

pub fn init() {
    let vm = vm();
    let ob = vm.c_object;
    let th = define_class("Thread", ob);
    rclass(th).alloc = AllocKind::None;
    vm.c_thread = th;
    let list = ary_new();
    let main = new_thread(th);
    self::th(main).started = true;
    unsafe { LIST = list; MAIN = main; CURRENT = main; C_KILL = new_class(vm.c_exception); }
    let fatal = new_class(vm.c_exception);
    name_class(fatal, ob, intern(b"fatal"));
    unsafe { C_FATAL = fatal; }
    vm.pinned.push(list); vm.pinned.push(main); vm.pinned.push(unsafe { C_KILL }); vm.pinned.push(fatal);
    let ds = |n: &str, f: CFunc, a: i32| define_singleton_cfunc(th, n, f, a);
    ds("new", th_s_new, -1); ds("start", th_s_new, -1); ds("fork", th_s_new, -1); ds("current", th_current, 0); ds("main", th_main, 0); ds("list", th_list, 0);
    ds("pass", th_pass, 0); ds("stop", th_stop, 0); ds("kill", th_s_kill, 1); ds("exit", th_s_exit, 0);
    ds("abort_on_exception", |_c, _s, _a, _b| Ok(Value::bool(unsafe { ABORT_ALL })), 0);
    ds("abort_on_exception=", |_c, _s, a, _b| { unsafe { ABORT_ALL = a[0].truthy(); } Ok(a[0]) }, 1);
    ds("report_on_exception", |_c, _s, _a, _b| Ok(Value::bool(unsafe { REPORT_ALL })), 0);
    ds("report_on_exception=", |_c, _s, a, _b| { unsafe { REPORT_ALL = a[0].truthy(); } Ok(a[0]) }, 1);
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
    d("backtrace", |_c, _s, _a, _b| Ok(ary_new()), -1); d("backtrace_locations", |_c, _s, _a, _b| Ok(ary_new()), -1);
    d("native_thread_id", |_c, _s, _a, _b| Ok(Value::fix(unsafe { crate::builtins::sys::getpid() } as i64)), 0);
    d("pending_interrupt?", |_c, _s, _a, _b| Ok(FALSE), -1);
    d("fetch", |c, s, a, b| { let k = key_sym(c, a[0])?; let h = thr(c, s)?.locals; match hash_get_opt(c, h, k)? { Some(v) => Ok(v), None if !b.is_null() => yield1(c, b, a[0]), None if a.len() > 1 => Ok(a[1]), None => { let d = inspect_str(c, k)?; err(c, crate::vm::vm().c_key_error, format!("key not found: {}", d)) } } }, -1);
    for n in ["Mutex", "Queue", "SizedQueue", "ConditionVariable"] { crate::builtins::prelude::lazy_const(th, n, "saruby/sync"); crate::builtins::prelude::lazy_const(ob, n, "saruby/sync"); }
    crate::builtins::prelude::lazy_const(ob, "Monitor", "saruby/sync");
    crate::builtins::prelude::lazy_const(ob, "ThreadGroup", "saruby/sync");
}
/// ThreadGroup, Mutex, Monitor, Queue, SizedQueue, ConditionVariable (the "saruby/sync" feature).
pub fn init_sync() {
    let vm = vm();
    if vm.c_mutex != NIL { return; }
    let (ob, th) = (vm.c_object, vm.c_thread);
    let tg = define_class("ThreadGroup", ob);
    let dg = new_object(tg);
    const_set(tg, intern(b"Default"), dg);
    define_cfunc(tg, "list", tg_list, 0);
    define_cfunc(tg, "add", |_c, s, _a, _b| Ok(s), 1);
    define_cfunc(tg, "enclosed?", |_c, _s, _a, _b| Ok(FALSE), 0);
    // Mutex / Monitor
    let mx = define_class_under(th, "Mutex", ob);
    rclass(mx).alloc = AllocKind::None;
    const_set(ob, intern(b"Mutex"), mx);
    vm.c_mutex = mx;
    let mon = define_class("Monitor", ob);
    rclass(mon).alloc = AllocKind::None;
    vm.c_monitor = mon;
    for (k, pre) in [(mx, ""), (mon, "mon_")] {
        define_cfunc(k, "synchronize", mx_sync, 0);
        define_cfunc(k, &format!("{}locked?", pre), mx_locked, 0);
        define_cfunc(k, &format!("{}owned?", pre), mx_owned, 0);
    }
    define_cfunc(mx, "lock", mx_lock, 0); define_cfunc(mx, "unlock", mx_unlock, 0); define_cfunc(mx, "try_lock", mx_try_lock, 0); define_cfunc(mx, "sleep", mx_sleep, -1);
    define_cfunc(mon, "enter", mx_lock, 0); define_cfunc(mon, "exit", mx_unlock, 0); define_cfunc(mon, "try_enter", mx_try_lock, 0);
    define_cfunc(mon, "new_cond", |c, s, a, b| { let f = str_new(b"monitor"); crate::builtins::kernel::require_feature(c, f, false)?; funcall(c, s, intern(b"new_cond"), a, b) }, 0);
    define_cfunc(mon, "mon_synchronize", mx_sync, 0); define_cfunc(mon, "mon_check_owner", mon_check_owner, 0); define_cfunc(mon, "wait_for_cond", mon_wait_for_cond, 2);
    // Queue / SizedQueue / ConditionVariable
    let q = define_class_under(th, "Queue", ob);
    rclass(q).alloc = AllocKind::None;
    const_set(ob, intern(b"Queue"), q);
    vm.c_queue = q;
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
