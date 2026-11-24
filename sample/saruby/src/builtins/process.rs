// Process / Process::Status / Signal, fork-exec-wait, Kernel#system/spawn/`/exec/fork/sleep.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::Global;
use crate::array::*;
use crate::builtins::exception::{raise_errno, raise_errno_plain};
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::builtins::io::RIo;
use crate::builtins::sys;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::ffi::CString;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};

// $? is thread-local: (pid << 32) | raw status in the current Thread (thread.rs); pid -1: none yet
pub const NO_STATUS: u64 = 0x8000_0000_0000_0000;
pub fn set_last_status(c: &mut Ctx, pid: i32, raw: i32) { crate::builtins::thread::set_last_status(c, ((pid as u32 as u64) << 32) | raw as u32 as u64); }
fn new_status(c: &mut Ctx, pid: i32, raw: i32) -> Value {
    let k = const_get_at(c, c.vm().m_process.get(), intern(b"Status")).unwrap();
    let o = new_object(c, k);
    ivar_set_raw(c, o, intern(b"@pid"), Value::fix(pid as i64));
    ivar_set_raw(c, o, intern(b"@st"), Value::fix(raw as i64));
    o
}
/// Process.argv0: $0 before the first `$0 =`.
static ARGV0: Global = Global::new();
/// `$0 =` / Process.setproctitle: CRuby rewrites the argv area (ps's command line) and leaves comm alone; not done here (todo.md).
pub fn set_proctitle(_c: &mut Ctx, _title: Value) {}
/// `$0 = v`: a frozen String, and the process title.
pub fn set_dollar_zero(c: &mut Ctx, v: Value) -> RV {
    let s = expect_str(c, v)?;
    if ARGV0.get(c) == NIL { let o = c.vm().dollar_zero.get(); ARGV0.set(c, o); }
    let s = c.str_copy(s);
    crate::heap::obj_set_flags(s, crate::object::FL_FROZEN);
    c.vm().dollar_zero.set(s);
    set_proctitle(c, s);
    Ok(s)
}
/// [packed status, Process::Status] of the last `$?` read: the same object until the status changes.
static LAST_ST: Global = Global::new();
pub fn last_status(c: &mut Ctx) -> Value {
    let w = crate::builtins::thread::last_status(c);
    let (pid, raw) = ((w >> 32) as u32 as i32, w as u32 as i32);
    if w == NO_STATUS { return NIL; }
    let key = Value::fix(w as i64);
    let cache = LAST_ST.get(c);
    if cache != NIL && c.ary_get(cache, 0) == key { return c.ary_get(cache, 1); }
    let st = new_status(c, pid, raw);
    let pair = c.ary_new_from(&[key, st]);
    LAST_ST.set(c, pair);
    st
}
fn exited(st: i32) -> bool { st & 0x7f == 0 }
fn exitstatus(st: i32) -> i32 { (st >> 8) & 0xff }
fn signaled(st: i32) -> bool { ((st & 0x7f) + 1) as i8 >> 1 > 0 }
fn stopped(st: i32) -> bool { st & 0xff == 0x7f }
fn st_raw(c: &Ctx, s: Value) -> i32 { let v = ivar_get(c, s, intern(b"@st")); if v.is_fixnum() { v.as_fix() as i32 } else { 0 } }
fn st_pid_of(c: &Ctx, s: Value) -> i64 { let v = ivar_get(c, s, intern(b"@pid")); if v.is_fixnum() { v.as_fix() } else { 0 } }
fn st_exitstatus(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let st = st_raw(c, s); Ok(if exited(st) { Value::fix(exitstatus(st) as i64) } else { NIL }) }
fn st_to_i(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(st_raw(c, s) as i64)) }
fn st_pid(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(st_pid_of(c, s))) }
fn st_success(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let st = st_raw(c, s); Ok(if exited(st) { Value::bool(exitstatus(st) == 0) } else { NIL }) }
fn st_exited(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(exited(st_raw(c, s)))) }
fn st_signaled(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(signaled(st_raw(c, s)))) }
fn st_stopped(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(stopped(st_raw(c, s)))) }
fn st_termsig(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let st = st_raw(c, s); Ok(if signaled(st) { Value::fix((st & 0x7f) as i64) } else { NIL }) }
fn st_stopsig(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let st = st_raw(c, s); Ok(if stopped(st) { Value::fix(exitstatus(st) as i64) } else { NIL }) }
fn st_coredump(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(st_raw(c, s) & 0x80 != 0 && signaled(st_raw(c, s)))) }
fn st_desc(c: &Ctx, s: Value) -> String {
    let st = st_raw(c, s);
    let mut d = format!("pid {}", st_pid_of(c, s));
    if stopped(st) { let sg = exitstatus(st); d.push_str(&format!(" stopped SIG{} (signal {})", signame(sg).unwrap_or("?"), sg)); }
    else if signaled(st) { let sg = st & 0x7f; d.push_str(&format!(" SIG{} (signal {})", signame(sg).unwrap_or("?"), sg)); if st & 0x80 != 0 { d.push_str(" (core dumped)"); } }
    else { d.push_str(&format!(" exit {}", exitstatus(st))); }
    d
}
fn st_to_s(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.str_new(st_desc(c, s).as_bytes())) }
fn st_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.str_new(format!("#<Process::Status: {}>", st_desc(c, s)).as_bytes())) }
fn st_eq(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    if a[0].is_fixnum() { return Ok(Value::bool(st_raw(c, s) as i64 == a[0].as_fix())); }
    Ok(Value::bool(a[0].is_heap() && real_class_of(c, a[0]) == real_class_of(c, s) && st_raw(c, a[0]) == st_raw(c, s)))
}

// ---- signals ----------------------------------------------------------------------------------

// signal.c siglist order (Linux numbers)
const SIGNALS: &[(&str, i32)] = &[("EXIT", 0), ("HUP", 1), ("INT", 2), ("QUIT", 3), ("ILL", 4), ("TRAP", 5), ("ABRT", 6), ("IOT", 6), ("FPE", 8), ("KILL", 9), ("BUS", 7),
    ("SEGV", 11), ("SYS", 31), ("PIPE", 13), ("ALRM", 14), ("TERM", 15), ("URG", 23), ("STOP", 19), ("TSTP", 20), ("CONT", 18), ("CHLD", 17), ("CLD", 17),
    ("TTIN", 21), ("TTOU", 22), ("IO", 29), ("XCPU", 24), ("XFSZ", 25), ("VTALRM", 26), ("PROF", 27), ("WINCH", 28), ("USR1", 10), ("USR2", 12), ("PWR", 30), ("POLL", 29)];
pub fn signame(n: i32) -> Option<&'static str> { SIGNALS.iter().find(|s| s.1 == n).map(|s| s.0) }
static PENDING: AtomicU64 = AtomicU64::new(0);
static HANDLERS: Global = Global::new(); // Array indexed by signo: nil (default) / String / Proc
extern "C" fn on_signal(sig: i32) { PENDING.fetch_or(1 << sig, Ordering::SeqCst); crate::vm::set_ints(); crate::builtins::blop::signal_wake(); }

pub fn signo_arg(c: &mut Ctx, v: Value) -> Result<i32, Esc> {
    if v.is_fixnum() { let n = v.as_fix(); if !(0..65).contains(&n) { return arg_err(c, format!("invalid signal number ({})", n)); } return Ok(n as i32); }
    let name = if v.is_sym() { crate::symbol::sym_str(v.as_sym()) } else if is_string(v) { String::from_utf8_lossy(c.str_bytes(v)).into_owned() } else {
        // rb_check_string_type
        let s = if v.is_heap() && !is_class_or_module(v) && respond_to(c, v, intern(b"to_str"), true) { funcall(c, v, intern(b"to_str"), &[], std::ptr::null())? } else { NIL };
        if !is_string(s) { let k = class_path(c, real_class_of(c, v)); return arg_err(c, format!("bad signal type {}", k)); }
        String::from_utf8_lossy(c.str_bytes(s)).into_owned()
    };
    let (neg, n) = match name.strip_prefix('-') { Some(r) => (true, r), None => (false, name.as_str()) };
    let n = n.strip_prefix("SIG").unwrap_or(n);
    match SIGNALS.iter().find(|s| s.0 == n) {
        Some(s) => Ok(if neg { -s.1 } else { s.1 }),
        None => arg_err(c, format!("unsupported signal 'SIG{}'", n)),
    }
}
// What a signal's OS handler is, as CRuby's trap() tells them apart for the old value it returns:
// SIG_DFL ("SYSTEM_DEFAULT"), SIG_IGN ("IGNORE"), Ruby's sighandler ("DEFAULT"), another (nil).
const F_DFL: u8 = 0; const F_IGN: u8 = 1; const F_RUBY: u8 = 2; const F_OTHER: u8 = 3;
/// The handlers CRuby installs at boot (and "DEFAULT" reinstalls): sighandler, sig_do_nothing for PIPE / SYS, SIGABRT's own.
fn boot_func(sig: i32) -> u8 { match sig { 1 | 2 | 3 | 10 | 12 | 14 | 15 | 17 => F_RUBY, 6 | 13 | 31 => F_OTHER, _ => F_DFL } }
static FUNCS: std::sync::Mutex<[u8; 65]> = std::sync::Mutex::new([u8::MAX; 65]);
/// `os_old`: the disposition signal(2) replaced; before the first trap it is the inherited one, and CRuby keeps an
/// inherited SIG_IGN (install_sighandler) but for PIPE (forced; Rust's runtime ignores it before main anyway).
fn swap_func(sig: i32, f: u8, os_old: usize) -> u8 {
    let mut t = FUNCS.lock().unwrap_or_else(|e| e.into_inner());
    let old = if t[sig as usize] != u8::MAX { t[sig as usize] } else if os_old == 1 && sig != 13 { F_IGN } else { boot_func(sig) };
    t[sig as usize] = f;
    old
}
fn signal_trap(c: &mut Ctx, _s: Value, a: &[Value], blk: Blk) -> RV {
    if a.is_empty() || a.len() > 2 || (a.len() == 1 && blk.is_null()) { if a.len() == 1 { return arg_err(c, "tried to create Proc object without a block"); } return argc_err(c, a.len(), 1, 2); }
    let sig = signo_arg(c, a[0])?;
    if sig < 0 {
        let nm = if a[0].is_sym() { crate::symbol::sym_str(a[0].as_sym()) } else { String::from_utf8_lossy(c.str_bytes(a[0])).into_owned() };
        return arg_err(c, format!("negative signal name: {}", nm));
    }
    if matches!(sig, 4 | 7 | 8 | 11 | 26) { return arg_err(c, format!("can't trap reserved signal: SIG{}", signame(sig).unwrap())); }
    if matches!(sig, 9 | 19) { return Err(raise_errno_plain(c, 22, &format!("SIG{}", signame(sig).unwrap()))); }
    let cmd = if a.len() == 2 { a[1] } else { block_to_proc(c, blk)? };
    // normalize: nil / "IGNORE" / "DEFAULT" strings or a callable
    let (store, action, func): (Value, sys::SigAction, u8) = if cmd == NIL { (NIL, sys::SigAction::Ignore, F_IGN) } else if is_string(cmd) || cmd.is_sym() {
        let sname = if cmd.is_sym() { crate::symbol::sym_str(cmd.as_sym()) } else { String::from_utf8_lossy(c.str_bytes(cmd)).into_owned() };
        match sname.as_str() {
            "" | "IGNORE" | "SIG_IGN" => (c.str_new(b"IGNORE"), sys::SigAction::Ignore, F_IGN),
            // SIGPIPE / SIGSYS get sig_do_nothing: a write still sees EPIPE
            "DEFAULT" | "SIG_DFL" => match boot_func(sig) {
                F_OTHER if sig != 6 => (UNDEF, sys::SigAction::Catch(on_signal), F_OTHER),
                F_RUBY => (UNDEF, sys::SigAction::Default, F_RUBY),
                _ => (UNDEF, sys::SigAction::Default, F_DFL),
            },
            "SYSTEM_DEFAULT" => (c.str_new(b"SYSTEM_DEFAULT"), sys::SigAction::Default, F_DFL),
            "EXIT" => (c.str_new(b"EXIT"), sys::SigAction::Catch(on_signal), F_RUBY),
            // another String is Ruby code the handler evals (rb_eval_cmd_kw)
            _ if is_string(cmd) => { let d = c.str_dup(cmd); (d, sys::SigAction::Catch(on_signal), F_RUBY) }
            _ => return arg_err(c, format!("wrong trap - {}", sname)),
        }
    } else { (cmd, sys::SigAction::Catch(on_signal), F_RUBY) };
    let h = HANDLERS.get(c);
    while c.ary_len(h) <= sig as usize { c.ary_push(h, UNDEF); }
    let old = c.ary_get(h, sig as usize);
    c.ary_mut(h)[sig as usize] = store;
    if sig == 0 { return Ok(if old == UNDEF { NIL } else { old }); }
    let os_old = sys::signal(sig, action);
    let oldf = swap_func(sig, func, os_old);
    // a command other than these strings is returned as it is (nil too)
    if old != UNDEF && !(is_string(old) && matches!(c.str_bytes(old), b"IGNORE" | b"SYSTEM_DEFAULT")) { return Ok(old); }
    Ok(match oldf { F_DFL => c.str_new(b"SYSTEM_DEFAULT"), F_IGN => c.str_new(b"IGNORE"), F_RUBY => c.str_new(b"DEFAULT"), _ => NIL })
}
fn run_handler(c: &mut Ctx, sig: i32) -> Result<(), Esc> {
    let h = c.ary_get(HANDLERS.get(c), sig as usize);
    if is_string(h) {
        if c.str_bytes(h) == b"EXIT" { let e = make_exception(c, c.vm().c_system_exit.get(), "exit"); ivar_set_raw(c, e, crate::symbol::s::status, Value::fix(0)); return Err(raise_exc(c, e)); }
        if !matches!(c.str_bytes(h), b"IGNORE" | b"SYSTEM_DEFAULT") { let m = c.vm().main_obj.get(); funcall(c, m, intern(b"eval"), &[h], std::ptr::null())?; }
        return Ok(());
    }
    if h == NIL || h == UNDEF { return Ok(()); }
    funcall(c, h, intern(b"call"), &[Value::fix(sig as i64)], std::ptr::null())?;
    Ok(())
}
/// Some trapped signal arrived and its handler has not run yet.
pub fn signals_pending() -> bool { PENDING.load(Ordering::Relaxed) != 0 }
/// Run trap handlers of signals delivered since the last check.
pub fn run_pending_signals(c: &mut Ctx) -> Result<(), Esc> {
    loop {
        let p = PENDING.swap(0, Ordering::SeqCst);
        if p == 0 { return Ok(()); }
        for sig in 1..64 { if p & (1 << sig) != 0 { if let Err(e) = run_handler(c, sig) { PENDING.fetch_or(p & !((2u64 << sig) - 1), Ordering::SeqCst); return Err(e); } } }
    }
}
/// For syscall-retry loops without a Ctx: handlers stay pending until the next check.
pub fn run_pending_signals_nothrow() {}
/// trap("EXIT") handler, run before the at_exit procs.
pub fn run_exit_trap(c: &mut Ctx) -> Result<(), Esc> {
    let hs = HANDLERS.get(c);
    let h = c.ary_get(hs, 0);
    let r = if h == NIL || h == UNDEF || is_string(h) { Ok(()) } else {
        c.ary_mut(hs)[0] = NIL;
        funcall(c, h, intern(b"call"), &[Value::fix(0)], std::ptr::null()).map(|_| ())
    };
    r
}
fn signal_list(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let h = c.hash_new();
    // the names are fstrings (the same objects every call)
    for (n, v) in SIGNALS { let s = c.str_new(n.as_bytes()); let k = funcall(c, s, intern(b"-@"), &[], std::ptr::null())?; crate::hash::hash_set(c, h, k, Value::fix(*v as i64))?; }
    Ok(h)
}
fn signal_signame(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let n = num_to_long(c, a[0])? as i32;
    Ok(match signame(n) { Some(s) => c.str_new(s.as_bytes()), None => NIL })
}
fn p_kill(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    if a.len() < 2 { return argc_err(c, a.len(), 2, -1); }
    let sig = signo_arg(c, a[0])?;
    let me = sys::getpid();
    let mut to_self = false;
    for &p in &a[1..] {
        let pid = num_to_long(c, p)? as i32;
        if pid == me && matches!(sig, 1 | 2 | 3 | 10 | 12 | 14 | 15) {
            // untrapped: CRuby's own handler raises SignalException / Interrupt in the main thread
            let hs = HANDLERS.get(c);
            let h = c.ary_get(hs, sig as usize);
            if h == UNDEF || (h == NIL && c.ary_len(hs) <= sig as usize) {
                let name = format!("SIG{}", signame(sig).unwrap());
                let e = make_exception(c, if sig == 2 { c.vm().c_interrupt.get() } else { c.vm().c_signal_exception.get() }, if sig == 2 { "" } else { &name });
                ivar_set_raw(c, e, intern(b"signo"), Value::fix(sig as i64));
                return Err(raise_exc(c, e));
            }
        }
        if pid == me { to_self = true; out_flush(); }
        let r = if sig < 0 { sys::kill(-pid, -sig) } else { sys::kill(pid, sig) };
        if r < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    }
    if to_self {
        // delivery is asynchronous: give a trapped signal a moment to arrive
        let h = c.ary_get(HANDLERS.get(c), sig as usize);
        if h != NIL && h != UNDEF && sig > 0 { let t0 = std::time::Instant::now(); while PENDING.load(Ordering::SeqCst) & (1 << sig) == 0 && t0.elapsed().as_millis() < 100 { std::thread::yield_now(); } }
        run_pending_signals(c)?;
    }
    Ok(Value::fix(a.len() as i64 - 1))
}

// ---- sleep ------------------------------------------------------------------------------------

/// rb_time_interval: seconds of a Numeric, or of what #divmod(1) gives; never negative.
pub fn time_interval(c: &mut Ctx, v: Value) -> Result<f64, Esc> {
    let secs = if crate::numeric::is_numeric(v) || crate::builtins::rational::is_rational(c, v) {
        crate::builtins::numeric_methods::to_f(c, v)?
    } else if !is_string(v) && v != NIL && respond_to(c, v, intern(b"divmod"), true) {
        let r = funcall(c, v, intern(b"divmod"), &[Value::fix(1)], std::ptr::null())?;
        if !is_array(r) || c.ary_len(r) != 2 { let k = class_path(c, real_class_of(c, v)); return type_err(c, format!("can't convert {} into time interval", k)); }
        let (i, f) = (c.ary_get(r, 0), c.ary_get(r, 1));
        crate::builtins::numeric_methods::to_f(c, i)? + crate::builtins::numeric_methods::to_f(c, f)?
    } else { let k = class_path(c, real_class_of(c, v)); return type_err(c, format!("can't convert {} into time interval", k)); };
    if secs < 0.0 { return arg_err(c, "time interval must not be negative"); }
    Ok(secs)
}

fn k_sleep(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    let t0 = std::time::Instant::now();
    // rb_f_sleep: a non-blocking fiber's scheduler gets the arguments as they are
    let sch = crate::builtins::thread::scheduler_current(c);
    if sch != NIL {
        crate::builtins::thread::sched_kernel_sleep(c, sch, args)?;
        return Ok(Value::fix(t0.elapsed().as_secs() as i64));
    }
    check_argc(c, args, 0, 1)?;
    let dur = match args.first() { Some(&v) if v != NIL => Some(time_interval(c, v)?), _ => None };
    out_flush();
    run_pending_signals(c)?;
    // a TIMER blop with or without other threads: the pump wakes for trap handlers (eventfd), which run
    // and the sleep goes on (CRuby 4.0: a handler does not end `sleep`, forever or timed)
    crate::builtins::thread::sleep(c, dur)?;
    Ok(Value::fix(t0.elapsed().as_secs_f64().round() as i64))
}

// ---- spawn / exec -----------------------------------------------------------------------------

pub struct Spec {
    pub argv: Vec<CString>, pub prog: CString, pub env: Vec<(CString, Option<CString>)>, pub clear_env: bool, pub chdir: Option<CString>,
    pub redirect: Vec<(i32, i32)>, pub close: Vec<i32>, pub opened: Vec<i32>, pub pgroup: Option<i32>, pub umask: Option<u32>,
    pub close_others: Vec<i32>,   // close_others: true: fds >= 3 the child gets with close-on-exec
    pub rlimits: Vec<(i32, sys::Rlimit)>, pub path: Option<CString>,   // path: the PATH search result (None: not found)
    pub pty: Option<(i32, i32)>,   // PTY.spawn: (master, slave) the child takes as its terminal
}
impl Drop for Spec { fn drop(&mut self) { for &fd in &self.opened { sys::close(fd); } } }

fn needs_shell(s: &str) -> bool {
    let t = s.trim_start();
    let first = t.split(|ch: char| ch.is_ascii_whitespace()).next().unwrap_or("");
    s.bytes().any(|b| b"*?{}[]<>()~&|\\$;'`\"\n#".contains(&b)) || first.contains('=')
        || matches!(first, "exit" | "cd" | "export" | "exec" | "set" | "unset" | "." | ":" | "eval" | "source" | "trap" | "ulimit" | "umask" | "wait" | "alias" | "command" | "type" | "read" | "if" | "for" | "while" | "case" | "!")
}
fn cstring(c: &mut Ctx, v: Value) -> Result<CString, Esc> {
    let sv = if is_string(v) { v } else if respond_to(c, v, intern(b"to_path"), true) && !v.is_sym() { funcall(c, v, intern(b"to_path"), &[], std::ptr::null())? } else { expect_str(c, v)? };
    if c.str_bytes(sv).contains(&0) { return arg_err(c, "string contains null byte"); }
    Ok(CString::new(c.str_bytes(sv)).unwrap())
}
fn fd_of(c: &mut Ctx, v: Value) -> Result<Option<i32>, Esc> {
    if v.is_fixnum() { return Ok(Some(v.as_fix() as i32)); }
    if v.is_sym() { return Ok(match crate::symbol::sym_str(v.as_sym()).as_str() { "in" => Some(0), "out" => Some(1), "err" => Some(2), _ => None }); }
    if let Some(io) = c.data::<RIo>(v) {
        if io.fd == 1 { out_flush(); }
        let _ = funcall(c, v, intern(b"flush"), &[], std::ptr::null());
        let Some(io) = c.data::<RIo>(v) else { return Ok(None) };
        return Ok(Some(if io.wfd >= 0 && !io.readable { io.wfd } else { io.fd }));
    }
    if respond_to(c, v, intern(b"to_io"), true) { let io = funcall(c, v, intern(b"to_io"), &[], std::ptr::null())?; return fd_of(c, io); }
    Ok(None)
}
/// Pairs of Hash `h`, each also pushed to `keep` (a Vec is not scanned by the collector).
fn pairs_kept(c: &mut Ctx, h: Value, keep: Value) -> Vec<(Value, Value)> {
    let ps = c.hash_pairs(h);
    for &(k, v) in &ps { c.ary_push(keep, k); c.ary_push(keep, v); }
    ps
}
/// Build the child description from (env, cmd..., opts) as Kernel#spawn takes them.
pub fn spawn_spec(c: &mut Ctx, env: Option<Value>, args: &[Value], opts: Option<Value>) -> Result<Spec, Esc> {
    let mut sp = Spec { argv: vec![], prog: CString::new("").unwrap(), env: vec![], clear_env: false, chdir: None, redirect: vec![], close: vec![], opened: vec![], pgroup: None, umask: None, close_others: vec![], rlimits: vec![], path: None, pty: None };
    if args.is_empty() { return argc_err(c, 0, 1, -1); }
    // rb_check_argv: a first argument with to_ary is [prog, argv0]
    let mut args = args.to_vec();
    if !is_array(args[0]) && !is_string(args[0]) && args[0].is_heap() && respond_to(c, args[0], intern(b"to_ary"), true) {
        let a0 = funcall(c, args[0], intern(b"to_ary"), &[], std::ptr::null())?;
        if a0 != NIL { if !is_array(a0) { return type_err(c, format!("can't convert {} to Array", class_name_for_err(c, args[0]))); } args[0] = a0; }
    }
    let args = &args[..];
    if args.len() == 1 && !is_array(args[0]) {
        let cmd = cstring(c, args[0])?;
        let s = cmd.to_string_lossy().into_owned();
        if needs_shell(&s) { sp.prog = CString::new("/bin/sh").unwrap(); sp.argv = vec![CString::new("sh").unwrap(), CString::new("-c").unwrap(), cmd]; }
        else {
            sp.argv = s.split_ascii_whitespace().map(|w| CString::new(w).unwrap()).collect();
            if sp.argv.is_empty() { return Err(raise_errno_plain(c, 2, &s)); }
            sp.prog = sp.argv[0].clone();
        }
    } else {
        if is_array(args[0]) {
            if c.ary_len(args[0]) != 2 { return arg_err(c, "wrong first argument"); }
            let (p0, a0) = (c.ary_get(args[0], 0), c.ary_get(args[0], 1));
            sp.prog = cstring(c, p0)?; sp.argv.push(cstring(c, a0)?);
        } else { sp.prog = cstring(c, args[0])?; sp.argv.push(sp.prog.clone()); }
        for &x in &args[1..] { sp.argv.push(cstring(c, x)?); }
    }
    // callbacks (to_str, to_path, to_io) may empty the Hashes: keep what was read from them
    let keep = c.ary_new();
    if let Some(e) = env {
        for (k, v) in pairs_kept(c, e, keep) {
            let k = cstring(c, k)?;
            if k.as_bytes().contains(&b'=') { return arg_err(c, format!("environment name contains a equal : {}", k.to_string_lossy())); }
            let v = if v == NIL { None } else { Some(cstring(c, v)?) };
            sp.env.push((k, v));
        }
    }
    let Some(h) = opts else { return Ok(sp) };
    let mut child_refs = vec![];
    for (k, v) in pairs_kept(c, h, keep) {
        if k.is_sym() {
            match crate::symbol::sym_str(k.as_sym()).as_str() {
                "chdir" => { sp.chdir = Some(cstring(c, v)?); continue; }
                "pgroup" => {
                    sp.pgroup = if v == TRUE { Some(0) } else if v == NIL || v == FALSE { None } else {
                        let g = num_to_long(c, v)?;
                        if g < 0 { return arg_err(c, format!("negative process group ID : {}", g)); }
                        Some(g as i32)
                    };
                    continue;
                }
                "new_pgroup" => { continue; }
                "umask" => { sp.umask = Some(num_to_long(c, v)? as u32); continue; }
                "unsetenv_others" => { sp.clear_env = bool_opt(c, v, "unsetenv_others")?; continue; }
                "close_others" => {
                    if bool_opt(c, v, "close_others")? {
                        // the fds open now (listed before fork: the child must not allocate)
                        if let Ok(d) = std::fs::read_dir("/proc/self/fd") {
                            sp.close_others = d.filter_map(|e| e.ok()?.file_name().to_str()?.parse::<i32>().ok()).filter(|&fd| fd >= 3).collect();
                        }
                    } else { sp.close_others.clear(); }
                    continue;
                }
                "exception" => { continue; }
                "in" | "out" | "err" => {}
                o => {
                    let r = o.strip_prefix("rlimit_").filter(|n| !n.bytes().any(|b| b.is_ascii_uppercase())).and_then(|n| RLIMITS.iter().find(|r| r.0.eq_ignore_ascii_case(n)));
                    let Some(&(_, res)) = r else { return arg_err(c, format!("wrong exec option symbol: {}", o)); };
                    let (cur, max) = if is_array(v) && (1..=2).contains(&c.ary_len(v)) {
                        let (a, b) = (c.ary_get(v, 0), c.ary_get(v, c.ary_len(v) - 1));
                        (rlim_num(c, a)?, rlim_num(c, b)?)
                    } else if is_array(v) { return arg_err(c, "wrong exec rlimit option"); } else { let n = rlim_num(c, v)?; (n, n) };
                    sp.rlimits.push((res, sys::Rlimit { cur, max }));
                    continue;
                }
            }
        }
        let keys = if is_array(k) { c.ary(k).to_vec() } else { vec![k] };
        for &x in &keys { c.ary_push(keep, x); }
        let mut dsts = vec![];
        for kk in keys { match fd_of(c, kk)? { Some(fd) => dsts.push(fd), None => return arg_err(c, "wrong exec redirect action") } }
        let reading = dsts.iter().all(|&d| d == 0);
        if v.is_sym() && v == sym_value("close") { sp.close.extend(dsts); continue; }
        let src = if is_string(v) || (is_array(v) && c.ary_len(v) >= 1 && is_string(c.ary_get(v, 0))) {
            let (path, rest): (Value, Vec<Value>) = if is_string(v) { (v, vec![]) } else { (c.ary_get(v, 0), c.ary(v)[1..].to_vec()) };
            for &x in &rest { c.ary_push(keep, x); }
            let p = crate::builtins::io::path_arg(c, path)?;
            let mut om = crate::builtins::io::parse_mode(c, &rest, None)?;
            if rest.is_empty() { om.flags = if reading { 0 } else { sys::O_WRONLY | sys::O_CREAT | sys::O_TRUNC }; om.perm = 0o644; }
            let fd = crate::builtins::io::sysopen(c, &p, &om)?;
            sp.opened.push(fd);
            fd
        } else if is_array(v) && c.ary_len(v) == 2 && c.ary_get(v, 0) == sym_value("child") {
            let v1 = c.ary_get(v, 1);
            let Some(s) = fd_of(c, v1)? else { return arg_err(c, "wrong exec redirect action") };
            for d in dsts { child_refs.push((d, s)); }
            continue;
        } else {
            match fd_of(c, v)? { Some(fd) => fd, None => return arg_err(c, "wrong exec redirect action") }
        };
        for d in dsts { sp.redirect.push((d, src)); }
    }
    sp.redirect.extend(child_refs);
    std::hint::black_box(keep);
    Ok(sp)
}
/// Where a child's setup failed (the Errno's message).
#[derive(Clone, Copy)]
enum Stage { Exec = 0, Chdir = 1, Setpgid = 2, Setrlimit = 3, Dup2 = 4 }
fn stage_msg(sp: &Spec, st: u32) -> String {
    match st { 1 => sp.chdir.as_ref().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default(), 2 => "setpgid".into(), 3 => "setrlimit".into(), 4 => "dup2".into(), _ if sp.pty.is_some() => "fork failed".into(), _ => sp.prog.to_string_lossy().into_owned() }
}
fn child_setup(sp: &Spec) -> Result<(), Stage> {
    // pty.c's chfunc: a new session whose controlling terminal is the slave
    if let Some((m, s)) = sp.pty {
        sys::setsid();
        sys::close(m);
        sys::ioctl_int(s, 0x540e, 0);
        for fd in 0..3 { sys::dup2(s, fd); }
        if s > 2 { sys::close(s); }
    }
    if let Some(pg) = sp.pgroup { if sys::setpgid(0, pg) < 0 { return Err(Stage::Setpgid); } }
    for (res, l) in &sp.rlimits { if sys::setrlimit(*res, l) < 0 { return Err(Stage::Setrlimit); } }
    if let Some(m) = sp.umask { sys::umask(m); }
    if sp.clear_env { sys::clearenv(); }
    for (k, v) in &sp.env { match v { Some(v) => { sys::setenv(k, v); } None => { sys::unsetenv(k); } } }
    for &(dst, src) in &sp.redirect {
        if dst == src { let f = sys::fcntl(src, sys::F_GETFD, 0); sys::fcntl(src, sys::F_SETFD, f & !sys::FD_CLOEXEC); }
        else if sys::dup2(src, dst) < 0 { return Err(Stage::Dup2); }
    }
    for &fd in &sp.close { sys::close(fd); }
    for &fd in &sp.close_others {
        if sp.redirect.iter().any(|&(dst, _)| dst == fd) { continue; }
        let f = sys::fcntl(fd, sys::F_GETFD, 0);
        if f >= 0 { sys::fcntl(fd, sys::F_SETFD, f | sys::FD_CLOEXEC); }
    }
    if let Some(d) = &sp.chdir { if sys::chdir(d) < 0 { return Err(Stage::Chdir); } }
    // disable_child_handler_fork_child: handlers back to SIG_DFL, ignored signals stay ignored but PIPE
    for sig in 1..65 { if sys::signal(sig, sys::SigAction::Default) == 1 && sig != 13 { sys::signal(sig, sys::SigAction::Ignore); } }
    Ok(())
}
/// exec the PATH search result; a name not found there is ENOENT (a non-executable match is skipped, as dln_find_exe_r does).
fn do_exec(sp: &Spec) {
    // stdfd_clear_nonblock: many programs cannot deal with non-blocking standard streams (IO.pipe's are)
    for fd in 0..3 { let fl = sys::fcntl(fd, sys::F_GETFL, 0); if fl >= 0 && fl & sys::O_NONBLOCK != 0 { sys::fcntl(fd, sys::F_SETFL, fl & !sys::O_NONBLOCK); } }
    if sp.prog.as_bytes().contains(&b'/') { sys::execvp(&sp.prog, &sp.argv); return; }
    match &sp.path { Some(p) => { sys::execvp(p, &sp.argv); } None => sys::set_errno(2) }
}
/// Search PATH (the spawn env's when it sets one) for an executable regular file named `prog`.
fn find_exe(sp: &Spec) -> Option<CString> {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::os::unix::fs::PermissionsExt;
    let path = match sp.env.iter().find(|(k, _)| k.as_bytes() == b"PATH") {
        Some((_, v)) => v.as_ref().map(|v| v.as_bytes().to_vec()).unwrap_or_default(),
        None => std::env::var_os("PATH").map(|p| p.into_vec()).unwrap_or_else(|| b"/usr/local/bin:/usr/bin:/bin".to_vec()),
    };
    for d in path.split(|&b| b == b':') {
        let d: &[u8] = if d.is_empty() { b"." } else { d };
        let mut f = d.to_vec(); f.push(b'/'); f.extend_from_slice(sp.prog.as_bytes());
        if let Ok(m) = std::fs::metadata(std::ffi::OsStr::from_bytes(&f)) { if m.is_file() && m.permissions().mode() & 0o111 != 0 { return CString::new(f).ok(); } }
    }
    None
}
fn bool_opt(c: &mut Ctx, v: Value, name: &str) -> Result<bool, Esc> {
    if v == NIL || v == FALSE { return Ok(false); }
    if v == TRUE { return Ok(true); }
    let s = inspect_str(c, v)?;
    arg_err(c, format!("expected true or false as {}: {}", name, s))
}
/// fork + exec; an exec failure in the child is reported as the parent's Errno.
pub fn spawn_exec(c: &mut Ctx, sp: &mut Spec) -> Result<i32, Esc> {
    if !sp.prog.as_bytes().contains(&b'/') { sp.path = find_exe(sp); }
    let sp = &*sp;
    let mut ep = [0i32; 2];
    if sys::pipe2(&mut ep, sys::O_CLOEXEC) < 0 { return Err(raise_errno(c, sys::errno(), "pipe")); }
    crate::builtins::io::flush_all(c);
    let pid = sys::fork();
    if pid < 0 { let e = sys::errno(); sys::close(ep[0]); sys::close(ep[1]); return Err(raise_errno(c, e, "fork(2)")); }
    if pid == 0 {
        let stage = match child_setup(sp) { Ok(()) => { do_exec(sp); Stage::Exec } Err(s) => s };
        let mut e = [0u8; 8];
        e[..4].copy_from_slice(&(sys::errno() as i32).to_ne_bytes());
        e[4..].copy_from_slice(&(stage as u32).to_ne_bytes());
        sys::write(ep[1], &e);
        sys::_exit(127);
    }
    sys::close(ep[1]);
    let mut eb = [0u8; 8];
    let n = sys::read_fd(ep[0], &mut eb).unwrap_or(0);
    sys::close(ep[0]);
    if n == 8 {
        let mut st = 0;
        sys::waitpid(pid, &mut st, 0);
        set_last_status(c, pid, 127 << 8);
        let (e, stage) = (i32::from_ne_bytes([eb[0], eb[1], eb[2], eb[3]]), u32::from_ne_bytes([eb[4], eb[5], eb[6], eb[7]]));
        return Err(raise_errno_plain(c, e as i64, &stage_msg(sp, stage)));
    }
    Ok(pid)
}
/// rb_check_hash_type: a Hash, what to_hash gives, or None.
fn check_hash(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if is_hash(v) { return Ok(Some(v)); }
    if !v.is_heap() || is_string(v) || !respond_to(c, v, intern(b"to_hash"), true) { return Ok(None); }
    let h = funcall(c, v, intern(b"to_hash"), &[], std::ptr::null())?;
    if h == NIL { return Ok(None); }
    if !is_hash(h) { let k = class_name_for_err(c, v); return type_err(c, format!("can't convert {} to Hash ({}#to_hash gives {})", k, k, class_path(c, real_class_of(c, h)))); }
    Ok(Some(h))
}
/// Split spawn-style arguments into (env, command words, options) as rb_exec_getargs: the options first.
pub fn split_args(c: &mut Ctx, a: &[Value]) -> Result<(Option<Value>, Vec<Value>, Option<Value>), Esc> {
    let mut rest = a.to_vec();
    let opts = match rest.last() { Some(&l) => check_hash(c, l)?, None => None };
    if opts.is_some() { rest.pop(); }
    let env = match rest.first() { Some(&e) => check_hash(c, e)?, None => None };
    if env.is_some() { rest.remove(0); }
    Ok((env, rest, opts))
}
const ECHILD: i64 = 10;
fn waitpid_retry(c: &mut Ctx, pid: i32, flags: i32) -> Result<(i32, i32), Esc> { waitpid_q(c, pid, flags, false) }
/// `quiet`: no child to wait for (SIGCHLD ignored) is pid -1, status 0 (rb_process_status_wait for system / ``).
fn waitpid_q(c: &mut Ctx, pid: i32, flags: i32, quiet: bool) -> Result<(i32, i32), Esc> {
    // rb_process_status_wait: a blocking wait in a non-blocking fiber asks its scheduler's process_wait
    if flags & WNOHANG == 0 {
        let s = crate::builtins::thread::scheduler_current(c);
        if s != NIL {
            if let Some(st) = crate::builtins::thread::sched_hook(c, s, "process_wait", &[Value::fix(pid as i64), Value::fix(flags as i64)])? {
                if st == NIL { return Ok((0, 0)); }
                let k = const_get_at(c, c.vm().m_process.get(), intern(b"Status")).unwrap_or(NIL);
                if !is_kind_of(c, st, k) { return type_err(c, format!("wrong argument type {} (expected Process::Status)", class_name_for_err(c, st))); }
                return Ok((st_pid_of(c, st) as i32, st_raw(c, st)));
            }
        }
    }
    if flags & WNOHANG == 0 && crate::builtins::thread::active(c) { return waitpid_coop(c, pid, flags)?.or_else(|e| if e == ECHILD && quiet { Ok((-1, 0)) } else { Err(raise_errno(c, e, "")) }); }
    loop {
        let mut st = 0;
        let r = sys::waitpid(pid, &mut st, flags);
        if r < 0 && sys::errno() == sys::EINTR { run_pending_signals(c)?; continue; }
        if r < 0 && quiet && sys::errno() == ECHILD { return Ok((-1, 0)); }
        if r < 0 { return Err(raise_errno(c, sys::errno(), "")); }
        return Ok((r, st));
    }
}
const WNOHANG: i32 = 1;
/// waitpid with other threads running: a pidfd's readiness (a POLL blop) for one pid, a
/// back-off sleep for "any child" / a process group (no fd to wait on).
fn waitpid_coop(c: &mut Ctx, pid: i32, flags: i32) -> Result<Result<(i32, i32), i64>, Esc> {
    let pfd = if pid > 0 { sys::pidfd_open(pid) } else { -1 };
    let mut nap = std::time::Duration::from_millis(1);
    let r = loop {
        let mut st = 0;
        let r = sys::waitpid(pid, &mut st, flags | WNOHANG);
        if r < 0 && sys::errno() == sys::EINTR { continue; }
        if r < 0 { break Err(sys::errno()); }
        if r > 0 { break Ok((r, st)); }
        let w = if pfd >= 0 { crate::builtins::thread::wait_readable(c, pfd) } else {
            let w = crate::builtins::thread::sleep_for(c, nap);
            nap = (nap * 2).min(std::time::Duration::from_millis(20));
            w
        };
        if let Err(e) = w { if pfd >= 0 { sys::close(pfd); } return Err(e); }
    };
    if pfd >= 0 { sys::close(pfd); }
    Ok(r)
}
/// Reap a popen child from IO#close (other threads run meanwhile).
pub fn reap_child(c: &mut Ctx, pid: i32) -> Result<(), Esc> {
    if !crate::builtins::thread::active(c) { wait_child(c, pid); return Ok(()); }
    if let Ok((_, st)) = waitpid_coop(c, pid, 0)? { set_last_status(c, pid, st); }
    Ok(())
}
/// Reap a popen child (called from IO#close).
pub fn wait_child(c: &mut Ctx, pid: i32) {
    let mut st = 0;
    loop { let r = sys::waitpid(pid, &mut st, 0); if r >= 0 { set_last_status(c, pid, st); break; } if sys::errno() != sys::EINTR { break; } }
}
fn k_spawn(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (env, cmd, opts) = split_args(c, a)?;
    let mut sp = spawn_spec(c, env, &cmd, opts)?;
    Ok(Value::fix(spawn_exec(c, &mut sp)? as i64))
}
fn k_system(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (env, cmd, opts) = split_args(c, a)?;
    let exc = match opts { Some(h) => match crate::hash::hash_get_opt(c, h, sym_value("exception"))? { Some(v) => bool_opt(c, v, "exception")?, None => false }, None => false };
    let mut sp = spawn_spec(c, env, &cmd, opts)?;
    let pid = match spawn_exec(c, &mut sp) {
        Ok(p) => p,
        Err(e) => { if exc { return Err(e); } c.esc = EscKind::None; return Ok(NIL); }
    };
    let (rp, st) = waitpid_q(c, pid, 0, true)?;
    let pid = if rp < 0 { rp } else { pid };
    set_last_status(c, pid, st);
    if exc && !(exited(st) && exitstatus(st) == 0) {
        let ns = new_status(c, pid, st); let d = st_desc(c, ns);
        return err(c, c.vm().c_runtime_error.get(), format!("Command failed with {}: {}", d.splitn(3, ' ').nth(2).unwrap_or(""), sp.argv.last().map(|a| a.to_string_lossy().into_owned()).unwrap_or_default()));
    }
    Ok(Value::bool(exited(st) && exitstatus(st) == 0))
}
fn k_backtick(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let mut sp = spawn_spec(c, None, &a[..1], None)?;
    let mut p = [0i32; 2];
    if sys::pipe2(&mut p, sys::O_CLOEXEC) < 0 { return Err(raise_errno(c, sys::errno(), "pipe")); }
    sp.redirect.push((1, p[1]));
    let pid = spawn_exec(c, &mut sp);
    sys::close(p[1]);
    let pid = match pid { Ok(p) => p, Err(e) => { sys::close(p[0]); return Err(e); } };
    let mut out = vec![]; let mut buf = [0u8; 65536];
    let coop = crate::builtins::thread::active(c);
    // a non-blocking fiber waits for the pipe in its scheduler's io_wait (CRuby reads a popen IO)
    let sch = crate::builtins::thread::scheduler_current(c);
    let pio = if sch != NIL { crate::builtins::io::io_for_fd_noclose(c, p[0], true, false) } else { NIL };
    loop {
        if sch != NIL && !crate::builtins::io::fd_ready(p[0], sys::POLLIN) {
            if let Err(e) = crate::builtins::thread::sched_io_wait(c, sch, pio, sys::POLLIN, NIL) { sys::close(p[0]); return Err(e); }
        } else if coop { if let Err(e) = crate::builtins::thread::wait_readable(c, p[0]) { sys::close(p[0]); return Err(e); } }
        match sys::read_fd(p[0], &mut buf) { Ok(0) | Err(_) => break, Ok(n) => out.extend_from_slice(&buf[..n]) }
    }
    // the scheduler may close the descriptor itself (io_close), as for the popen IO CRuby reads
    let closed = sch != NIL && crate::builtins::thread::sched_hook(c, sch, "io_close", &[Value::fix(p[0] as i64)])?.is_some_and(|r| r.truthy());
    if !closed { sys::close(p[0]); }
    let (rp, st) = waitpid_q(c, pid, 0, true)?;
    // `` leaves no status when there was no child to wait for
    if rp < 0 { crate::builtins::thread::set_last_status(c, NO_STATUS); } else { set_last_status(c, pid, st); }
    Ok(c.str_from_vec_enc(out, crate::builtins::encoding::default_external()))
}
fn k_exec(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (env, cmd, opts) = split_args(c, a)?;
    let mut sp = spawn_spec(c, env, &cmd, opts)?;
    if !sp.prog.as_bytes().contains(&b'/') { sp.path = find_exe(&sp); }
    crate::builtins::io::flush_all(c);
    let stage = match child_setup(&sp) { Ok(()) => { do_exec(&sp); Stage::Exec } Err(s) => s };
    let e = sys::errno();
    Err(raise_errno_plain(c, e, &stage_msg(&sp, stage as u32)))
}
/// Block every signal in the calling (non-interpreter) thread.
pub fn block_signals_in_this_thread() {
    sys::block_all_signals();
}
fn p_itimer(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let f = crate::builtins::numeric_methods::to_f(c, a[0])?;
    sys::set_itimer_real(f.floor() as i64, ((f - f.floor()) * 1e6) as i64);
    Ok(NIL)
}
static DIE_SIGNAL: AtomicI32 = AtomicI32::new(0);
/// An uncaught SignalException ends the process by that signal once the exit handlers ran.
pub fn die_by_signal_at_exit(sig: i32) { DIE_SIGNAL.store(sig, Ordering::Relaxed); }
pub fn die_if_signaled() {
    let sig = DIE_SIGNAL.load(Ordering::Relaxed);
    if sig > 0 { sys::signal(sig, sys::SigAction::Default); sys::kill(sys::getpid(), sig); }
}
pub fn after_fork_child(c: &mut Ctx) { PENDING.store(0, Ordering::SeqCst); crate::builtins::thread::after_fork_child(c); }
/// Finish a forked child that ran a block.
pub fn child_exit(c: &mut Ctx, r: RV) -> ! {
    let st = crate::finish(c, r.is_err());
    std::process::exit(st);
}
/// Process._fork: the fork(2) that Kernel#fork goes through (a hook point since 3.1).
fn p_underscore_fork(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    crate::builtins::io::flush_all(c);
    let pid = sys::fork();
    if pid < 0 { return Err(raise_errno(c, sys::errno(), "fork(2)")); }
    if pid == 0 { after_fork_child(c); }
    Ok(Value::fix(pid as i64))
}
fn k_fork(c: &mut Ctx, _s: Value, _a: &[Value], blk: Blk) -> RV {
    if crate::builtins::ractor::isolated(c) { return err(c, crate::builtins::ractor::isolation_error_class(c), "can not fork from non-main Ractors"); }
    let parent = sys::getpid();
    let pv = funcall(c, c.vm().m_process.get(), intern(b"_fork"), &[], std::ptr::null());
    // rb_call_proc__fork: a _fork hook raising in the child stops the child there (ruby_stop)
    let pv = match pv { Err(e) if sys::getpid() != parent => child_exit(c, Err(e)), r => r? };
    let pid = num_to_long(c, pv)?;
    if pid == 0 {
        if blk.is_null() { return Ok(NIL); }
        let r = yield_block(c, blk, &[]);
        child_exit(c, r);
    }
    Ok(pv)
}
fn wait_impl(c: &mut Ctx, a: &[Value]) -> Result<Option<(i32, i32)>, Esc> {
    check_argc(c, a, 0, 2)?;
    let pid = match a.first() { Some(&p) if p != NIL => num_to_long(c, p)? as i32, _ => -1 };
    let flags = match a.get(1) { Some(&f) if f != NIL => num_to_long(c, f)? as i32, _ => 0 };
    out_flush();
    let (r, st) = waitpid_retry(c, pid, flags)?;
    if r == 0 { return Ok(None); }
    set_last_status(c, r, st);
    Ok(Some((r, st)))
}
/// Process::Status.wait(pid = -1, flags = 0): the status ($? is left alone); pid -1 when there is no such child.
fn st_s_wait(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 2)?;
    let pid = match a.first() { Some(&p) if p != NIL => num_to_long(c, p)? as i32, _ => -1 };
    let flags = match a.get(1) { Some(&f) if f != NIL => num_to_long(c, f)? as i32, _ => 0 };
    let (r, st) = match waitpid_retry(c, pid, flags) { Ok(x) => x, Err(e) => { if !is_kind_of(c, c.esc_val, c.vm().c_system_call_error.get()) { return Err(e); } c.esc = EscKind::None; (-1, 0) } };
    if r == 0 { return Ok(NIL); }
    Ok(new_status(c, r, st))
}
fn p_wait(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { Ok(match wait_impl(c, a)? { Some((p, _)) => Value::fix(p as i64), None => NIL }) }
fn p_wait2(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    Ok(match wait_impl(c, a)? { Some((p, st)) => { let s = new_status(c, p, st); c.ary_new_from(&[Value::fix(p as i64), s]) } None => NIL })
}
fn p_waitall(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let out = c.ary_new();
    loop {
        let mut st = 0;
        let r = sys::waitpid(-1, &mut st, 0);
        if r < 0 { if sys::errno() == sys::EINTR { run_pending_signals(c)?; continue; } break; }
        set_last_status(c, r, st);
        let s = new_status(c, r, st);
        let pair = c.ary_new_from(&[Value::fix(r as i64), s]);
        c.ary_push(out, pair);
    }
    Ok(out)
}
/// A Process constant defined by lib saruby/process (loaded on first reference).
fn lazy_class(c: &mut Ctx, name: &[u8]) -> RV {
    match const_get_inherit(c, c.vm().m_process.get(), intern(name))? { Some(k) => Ok(k), None => err(c, c.vm().c_name_error.get(), format!("uninitialized constant Process::{}", String::from_utf8_lossy(name))) }
}
/// Process.detach: a Process::Waiter thread (lib saruby/process) that reaps `pid`.
fn p_detach(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let pid = num_to_long(c, a[0])? as i32;
    let k = lazy_class(c, b"Waiter")?;
    funcall(c, k, intern(b"__start"), &[Value::fix(pid as i64)], std::ptr::null())
}
/// The detach thread's body: wait for `pid`; its status, or nil when it is not a child.
fn p_detach_wait(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let pid = num_to_long(c, a[0])? as i32;
    match waitpid_retry(c, pid, 0) {
        Ok((r, st)) => { set_last_status(c, r, st); Ok(new_status(c, r, st)) }
        Err(e) => { if !is_kind_of(c, c.esc_val, c.vm().c_system_call_error.get()) { return Err(e); } c.esc = EscKind::None; Ok(NIL) }
    }
}

// ---- clocks / ids -----------------------------------------------------------------------------

fn clock_id(c: &mut Ctx, v: Value) -> Result<i32, Esc> {
    if v.is_fixnum() { return Ok(v.as_fix() as i32); }
    if v.is_sym() { return Ok(match crate::symbol::sym_str(v.as_sym()).as_str() {
        "GETTIMEOFDAY_BASED_CLOCK_REALTIME" | "TIME_BASED_CLOCK_REALTIME" => 0, "TIMES_BASED_CLOCK_MONOTONIC" => 1,
        "GETRUSAGE_BASED_CLOCK_PROCESS_CPUTIME_ID" | "TIMES_BASED_CLOCK_PROCESS_CPUTIME_ID" | "CLOCK_BASED_CLOCK_PROCESS_CPUTIME_ID" => 2,
        _ => { let d = inspect_str(c, v)?; return Err(raise_errno_plain(c, 22, &format!("clock_gettime({})", d))); }
    }); }
    type_err(c, format!("no implicit conversion of {} into Integer", class_name_for_err(c, v)))
}
fn timespec_value(c: &mut Ctx, t: &sys::Timespec, unit: Value) -> RV {
    let ns = t.sec as i128 * 1_000_000_000 + t.nsec as i128;
    // timetick2dblnum: sec * 1e9 + nsec, then divided (CRuby's rounding)
    let d = t.sec as f64 * 1e9 + t.nsec as f64;
    let secs = d / 1e9;
    if unit == NIL { return Ok(c.float_new(secs)); }
    if !unit.is_sym() { return arg_err(c, "unexpected unit"); }
    Ok(match crate::symbol::sym_str(unit.as_sym()).as_str() {
        "nanosecond" => c.int_from_i128(ns),
        "microsecond" => c.int_from_i128(ns / 1000),
        "millisecond" => c.int_from_i128(ns / 1_000_000),
        "second" => Value::fix(t.sec),
        "float_millisecond" => c.float_new(d / 1e6),
        "float_microsecond" => c.float_new(d / 1e3),
        "float_second" => c.float_new(secs),
        o => return arg_err(c, format!("unexpected unit: {}", o)),
    })
}
fn clock_gettime(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let id = clock_id(c, a[0])?;
    let mut t = sys::Timespec { sec: 0, nsec: 0 };
    if sys::clock_gettime(id, &mut t) < 0 { let d = inspect_str(c, a[0])?; return Err(raise_errno_plain(c, sys::errno(), &format!("clock_gettime({})", d))); }
    timespec_value(c, &t, a.get(1).copied().unwrap_or(NIL))
}
fn clock_getres(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    // the emulated clocks' resolutions (gettimeofday / getrusage / clock(): 1 us, time(): 1 s, times(): 1 tick)
    if a[0].is_sym() {
        let ns = match crate::symbol::sym_str(a[0].as_sym()).as_str() {
            "GETTIMEOFDAY_BASED_CLOCK_REALTIME" | "GETRUSAGE_BASED_CLOCK_PROCESS_CPUTIME_ID" | "CLOCK_BASED_CLOCK_PROCESS_CPUTIME_ID" => 1000,
            "TIME_BASED_CLOCK_REALTIME" => 1_000_000_000,
            "TIMES_BASED_CLOCK_MONOTONIC" | "TIMES_BASED_CLOCK_PROCESS_CPUTIME_ID" => 10_000_000,
            _ => { let d = inspect_str(c, a[0])?; return Err(raise_errno_plain(c, 22, &format!("clock_getres({})", d))); }
        };
        return timespec_value(c, &sys::Timespec { sec: ns / 1_000_000_000, nsec: ns % 1_000_000_000 }, a.get(1).copied().unwrap_or(NIL));
    }
    let id = clock_id(c, a[0])?;
    let mut t = sys::Timespec { sec: 0, nsec: 0 };
    if sys::clock_getres(id, &mut t) < 0 { let d = inspect_str(c, a[0])?; return Err(raise_errno_plain(c, sys::errno(), &format!("clock_getres({})", d))); }
    timespec_value(c, &t, a.get(1).copied().unwrap_or(NIL))
}
fn p_times(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let mut t = sys::Tms { utime: 0, stime: 0, cutime: 0, cstime: 0 };
    sys::times(&mut t);
    let k = lazy_class(c, b"Tms")?;
    let v: Vec<Value> = [t.utime, t.stime, t.cutime, t.cstime].iter().map(|&x| c.float_new(x as f64 / 100.0)).collect();
    funcall(c, k, intern(b"new"), &v, std::ptr::null())
}
const RLIMITS: &[(&str, i32)] = &[("AS", 9), ("CORE", 4), ("CPU", 0), ("DATA", 2), ("FSIZE", 1), ("MEMLOCK", 8), ("MSGQUEUE", 12), ("NICE", 13), ("NOFILE", 7),
    ("NPROC", 6), ("RSS", 5), ("RTPRIO", 14), ("RTTIME", 15), ("SIGPENDING", 11), ("STACK", 3)];
/// rb_check_string_type for a Symbol too: the name's bytes, or None.
fn name_arg(c: &mut Ctx, v: Value) -> Result<Option<Vec<u8>>, Esc> {
    if v.is_sym() { return Ok(Some(crate::symbol::sym_str(v.as_sym()).into_bytes())); }
    if is_string(v) { return Ok(Some(c.str_bytes(v).to_vec())); }
    if !v.is_heap() || !respond_to(c, v, intern(b"to_str"), true) { return Ok(None); }
    let s = funcall(c, v, intern(b"to_str"), &[], std::ptr::null())?;
    if s == NIL { return Ok(None); }
    if !is_string(s) { let k = class_name_for_err(c, v); return type_err(c, format!("can't convert {} to String ({}#to_str gives {})", k, k, class_path(c, real_class_of(c, s)))); }
    Ok(Some(c.str_bytes(s).to_vec()))
}
fn rlimit_resource(c: &mut Ctx, v: Value) -> Result<i32, Esc> {
    match name_arg(c, v)? {
        Some(n) => match RLIMITS.iter().find(|r| r.0.as_bytes() == n.as_slice()) { Some(r) => Ok(r.1), None => arg_err(c, format!("invalid resource name: {}", String::from_utf8_lossy(&n))) },
        None => Ok(num_to_long(c, v)? as i32),
    }
}
/// NUM2RLIM: an Integer (to_int) as an unsigned 64-bit limit; negatives wrap as in C.
fn rlim_num(c: &mut Ctx, v: Value) -> Result<u64, Esc> {
    let v = if is_integer(v) { v } else if v.is_heap() && !is_float(v) && respond_to(c, v, intern(b"to_int"), true) {
        let r = funcall(c, v, intern(b"to_int"), &[], std::ptr::null())?;
        if !is_integer(r) { let k = class_name_for_err(c, v); return type_err(c, format!("can't convert {} to Integer ({}#to_int gives {})", k, k, class_path(c, real_class_of(c, r)))); }
        r
    } else { return Ok(num_to_long(c, v)? as u64) };
    if v.is_fixnum() { return Ok(v.as_fix() as u64); }
    let b = c.big(v);
    if let Some(i) = b.to_i64() { return Ok(i as u64); }
    if !b.neg && b.mag.len() <= 2 { return Ok(b.mag[0] as u64 | (*b.mag.get(1).unwrap_or(&0) as u64) << 32); }
    err(c, c.vm().c_range_error.get(), "bignum too big to convert into 'unsigned long long'")
}
fn rlimit_value(c: &mut Ctx, v: Value) -> Result<u64, Esc> {
    match name_arg(c, v)? {
        Some(n) => match n.as_slice() { b"INFINITY" | b"SAVED_MAX" | b"SAVED_CUR" => Ok(u64::MAX), _ => arg_err(c, format!("invalid resource value: {}", String::from_utf8_lossy(&n))) },
        None => rlim_num(c, v),
    }
}
fn p_getrlimit(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let r = rlimit_resource(c, a[0])?;
    let mut l = sys::Rlimit { cur: 0, max: 0 };
    if sys::getrlimit(r, &mut l) < 0 { return Err(raise_errno_plain(c, sys::errno(), "getrlimit")); }
    let cv = c.int_from_i128(l.cur as i128); let mv = c.int_from_i128(l.max as i128);
    Ok(c.ary_new_from(&[cv, mv]))
}
fn p_setrlimit(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 2, 3)?;
    let res = rlimit_resource(c, a[0])?;
    let cur = rlimit_value(c, a[1])?;
    let max = match a.get(2) { Some(&m) => rlimit_value(c, m)?, None => cur };
    if sys::setrlimit(res, &sys::Rlimit { cur, max }) < 0 { return Err(raise_errno_plain(c, sys::errno(), "setrlimit")); }
    Ok(NIL)
}

// ---- user / group ids, priority -------------------------------------------------------------

/// A line of /etc/passwd or /etc/group whose first field is `name`: its third field (the id).
fn id_by_name(file: &str, name: &[u8]) -> Option<u32> {
    let t = std::fs::read(file).ok()?;
    t.split(|&b| b == b'\n').find_map(|l| {
        let f: Vec<&[u8]> = l.split(|&b| b == b':').collect();
        if f.len() > 2 && f[0] == name { std::str::from_utf8(f[2]).ok()?.parse().ok() } else { None }
    })
}
/// OBJ2UID / OBJ2GID: an Integer, or a user (group) name looked up in /etc/passwd (/etc/group).
fn id_arg(c: &mut Ctx, v: Value, group: bool) -> Result<u32, Esc> {
    if !is_string(v) && !(v.is_heap() && !is_integer(v) && respond_to(c, v, intern(b"to_str"), true)) { return Ok(num_to_long(c, v)? as u32); }
    let s = expect_str(c, v)?;
    let n = c.str_bytes(s).to_vec();
    match id_by_name(if group { "/etc/group" } else { "/etc/passwd" }, &n) {
        Some(i) => Ok(i),
        None => arg_err(c, format!("can't find {} for {}", if group { "group" } else { "user" }, String::from_utf8_lossy(&n))),
    }
}
const KEEP: u32 = u32::MAX;
fn cred_ok(c: &mut Ctx, r: i32) -> Result<(), Esc> { if r < 0 { Err(raise_errno(c, sys::errno(), "")) } else { Ok(()) } }
fn p_set_uid(c: &mut Ctx, a: Value, which: u8) -> RV {
    let group = which >= 2;
    let id = id_arg(c, a, group)?;
    let (r, e) = if which % 2 == 0 { (id, KEEP) } else { (KEEP, id) };
    let rc = if group { sys::setresgid(r, e, KEEP) } else { sys::setresuid(r, e, KEEP) };
    cred_ok(c, rc)?;
    Ok(Value::fix(id as i64))
}
fn ids_ary(c: &mut Ctx, ids: &[u32]) -> Value { let v: Vec<Value> = ids.iter().map(|&i| Value::fix(i as i64)).collect(); c.ary_new_from(&v) }
fn p_groups(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { groups_value(c) }
fn groups_value(c: &mut Ctx) -> RV {
    let n = sys::getgroups(&mut []);
    if n < 0 { return Err(raise_errno(c, sys::errno(), "getgroups")); }
    let mut g = vec![0u32; n as usize];
    let n = sys::getgroups(&mut g);
    if n < 0 { return Err(raise_errno(c, sys::errno(), "getgroups")); }
    g.truncate(n as usize);
    Ok(ids_ary(c, &g))
}
static MAXGROUPS: AtomicI32 = AtomicI32::new(65536);
fn p_set_groups(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let l = expect_array(c, a[0])?;
    let n = c.ary_len(l);
    if n > MAXGROUPS.load(Ordering::Relaxed) as usize { return arg_err(c, format!("too many groups, {} max", MAXGROUPS.load(Ordering::Relaxed))); }
    let mut g = vec![];
    for i in 0..n { let x = c.ary_get(l, i); g.push(id_arg(c, x, true)?); }
    if sys::setgroups(&g) < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    groups_value(c)
}
fn p_initgroups(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let u = cstring(c, a[0])?;
    let g = id_arg(c, a[1], true)?;
    if sys::initgroups(&u, g) < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    groups_value(c)
}
fn p_set_maxgroups(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let n = num_to_long(c, a[0])?;
    if n <= 0 { return arg_err(c, format!("maxgroups {} should be positive", n)); }
    MAXGROUPS.store(n.min(65536) as i32, Ordering::Relaxed);
    Ok(a[0])
}
fn p_getpriority(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (w, who) = (num_to_long(c, a[0])? as i32, num_to_long(c, a[1])? as i32);
    let r = sys::getpriority_raw(w, who);
    if r < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    Ok(Value::fix(20 - r as i64))
}
fn p_setpriority(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (w, who, pr) = (num_to_long(c, a[0])? as i32, num_to_long(c, a[1])? as i32, num_to_long(c, a[2])? as i32);
    if sys::setpriority(w, who, pr) < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    Ok(Value::fix(0))
}
/// Process::Sys.setresuid / setresgid (-1 keeps an id).
fn sys_setres(c: &mut Ctx, a: &[Value], group: bool) -> RV {
    let mut ids = [KEEP; 3];
    for i in 0..3 { if !(a[i].is_fixnum() && a[i].as_fix() == -1) { ids[i] = id_arg(c, a[i], group)?; } }
    let rc = if group { sys::setresgid(ids[0], ids[1], ids[2]) } else { sys::setresuid(ids[0], ids[1], ids[2]) };
    cred_ok(c, rc)?;
    Ok(NIL)
}
fn p_daemon(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 2)?;
    let nochdir = match a.first() { Some(&v) => bool_opt(c, v, "nochdir")?, None => false };
    let noclose = match a.get(1) { Some(&v) => bool_opt(c, v, "noclose")?, None => false };
    crate::builtins::io::flush_all(c);
    // daemon(3): the parent leaves at once (no exit handlers); the child gets a new session
    let pid = sys::fork();
    if pid < 0 { return Err(raise_errno_plain(c, sys::errno(), "daemon")); }
    if pid > 0 { sys::_exit(0); }
    after_fork_child(c);
    sys::setsid();
    if !nochdir { sys::chdir(c"/"); }
    if !noclose {
        let fd = sys::open(c"/dev/null", sys::O_RDWR, 0);
        if fd >= 0 { for t in 0..3 { sys::dup2(fd, t); } if fd > 2 { sys::close(fd); } }
    }
    Ok(Value::fix(0))
}
fn p_exit_bang(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let st = match a.first() { None => 0, Some(&v) if v == TRUE => 0, Some(&v) if v == FALSE => 1, Some(&v) => num_to_long(c, v)? };
    out_flush();
    sys::_exit(st as i32)
}

/// Boot.
extern "C" fn sig_do_nothing(_sig: i32) {}
pub fn init(c: &mut Ctx) {
    // CRuby's SIGPIPE handler (a write sees EPIPE) in place of the SIG_IGN Rust's runtime set before main
    sys::signal(13, sys::SigAction::Catch(sig_do_nothing));
    let vmr = c.vm();
    let p = define_module(c, "Process");
    vmr.m_process.set(p);
    let h = c.alloc(vmr.c_array.get(), Body::Array(crate::avec::AVec::new()));
    HANDLERS.set_vm(vmr, h);
    let mf = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_module_function(c, p, n, f, a);
    mf(c, "clock_gettime", clock_gettime, -1); mf(c, "clock_getres", clock_getres, -1); mf(c, "times", p_times, 0);
    mf(c, "pid", |_c, _s, _a, _b| Ok(Value::fix(sys::getpid() as i64)), 0);
    mf(c, "ppid", |_c, _s, _a, _b| Ok(Value::fix(sys::getppid() as i64)), 0);
    mf(c, "uid", |_c, _s, _a, _b| Ok(Value::fix(sys::getuid() as i64)), 0);
    mf(c, "euid", |_c, _s, _a, _b| Ok(Value::fix(sys::geteuid() as i64)), 0);
    mf(c, "gid", |_c, _s, _a, _b| Ok(Value::fix(sys::getgid() as i64)), 0);
    mf(c, "egid", |_c, _s, _a, _b| Ok(Value::fix(sys::getegid() as i64)), 0);
    mf(c, "getpgrp", |_c, _s, _a, _b| Ok(Value::fix(sys::getpgid(0) as i64)), 0);
    mf(c, "getpgid", |c, _s, a, _b| { let p = num_to_long(c, a[0])? as i32; let r = sys::getpgid(p); if r < 0 { return Err(raise_errno(c, sys::errno(), "")); } Ok(Value::fix(r as i64)) }, 1);
    mf(c, "setpgid", |c, _s, a, _b| { let (p, g) = (num_to_long(c, a[0])? as i32, num_to_long(c, a[1])? as i32); if sys::setpgid(p, g) < 0 { return Err(raise_errno(c, sys::errno(), "")); } Ok(Value::fix(0)) }, 2);
    mf(c, "setsid", |c, _s, _a, _b| { let r = sys::setsid(); if r < 0 { return Err(raise_errno(c, sys::errno(), "")); } Ok(Value::fix(r as i64)) }, 0);
    mf(c, "kill", p_kill, -1); mf(c, "wait", p_wait, -1); mf(c, "waitpid", p_wait, -1); mf(c, "wait2", p_wait2, -1); mf(c, "waitpid2", p_wait2, -1); mf(c, "waitall", p_waitall, 0);
    mf(c, "detach", p_detach, 1); mf(c, "fork", k_fork, 0); mf(c, "spawn", k_spawn, -1); mf(c, "exec", k_exec, -1); mf(c, "exit!", p_exit_bang, -1); mf(c, "exit", k_exit, -1);
    mf(c, "abort", k_abort, -1);
    mf(c, "getrlimit", p_getrlimit, 1); mf(c, "setrlimit", p_setrlimit, -1); mf(c, "_fork", p_underscore_fork, 0);
    mf(c, "uid=", |c, _s, a, _b| p_set_uid(c, a[0], 0), 1); mf(c, "euid=", |c, _s, a, _b| p_set_uid(c, a[0], 1), 1);
    mf(c, "gid=", |c, _s, a, _b| p_set_uid(c, a[0], 2), 1); mf(c, "egid=", |c, _s, a, _b| p_set_uid(c, a[0], 3), 1);
    mf(c, "groups", p_groups, 0); mf(c, "groups=", p_set_groups, 1); mf(c, "initgroups", p_initgroups, 2);
    mf(c, "maxgroups", |_c, _s, _a, _b| Ok(Value::fix(MAXGROUPS.load(Ordering::Relaxed) as i64)), 0); mf(c, "maxgroups=", p_set_maxgroups, 1);
    mf(c, "getpriority", p_getpriority, 2); mf(c, "setpriority", p_setpriority, 3); mf(c, "daemon", p_daemon, -1);
    mf(c, "__detach_wait", p_detach_wait, 1);
    mf(c, "__setresid", |c, _s, a, _b| sys_setres(c, &a[1..4], a[0].truthy()), 4);
    mf(c, "__resid", |c, _s, a, _b| { let mut ids = [0u32; 3]; if if a[0].truthy() { sys::getresgid(&mut ids) } else { sys::getresuid(&mut ids) } < 0 { return Err(raise_errno(c, sys::errno(), "")); } Ok(ids_ary(c, &ids)) }, 1);
    mf(c, "__id_of", |c, _s, a, _b| Ok(Value::fix(id_arg(c, a[0], a[1].truthy())? as i64)), 2); mf(c, "__itimer", p_itimer, 1); mf(c, "setproctitle", |c, _s, a, _b| { let s = expect_str(c, a[0])?; set_proctitle(c, s); Ok(a[0]) }, 1); mf(c, "last_status", |c, _s, _a, _b| Ok(last_status(c)), 0);
    mf(c, "argv0", |c, _s, _a, _b| Ok(match ARGV0.get(c) { NIL => c.vm().dollar_zero.get(), v => v }), 0); mf(c, "warmup", |_c, _s, _a, _b| Ok(TRUE), 0);
    for (n, v) in [("CLOCK_REALTIME", 0), ("CLOCK_MONOTONIC", 1), ("CLOCK_PROCESS_CPUTIME_ID", 2), ("CLOCK_THREAD_CPUTIME_ID", 3), ("CLOCK_MONOTONIC_RAW", 4),
                   ("CLOCK_REALTIME_COARSE", 5), ("CLOCK_MONOTONIC_COARSE", 6), ("CLOCK_BOOTTIME", 7), ("CLOCK_REALTIME_ALARM", 8), ("CLOCK_BOOTTIME_ALARM", 9), ("CLOCK_TAI", 11),
                   ("WNOHANG", 1), ("WUNTRACED", 2), ("PRIO_PROCESS", 0), ("PRIO_PGRP", 1), ("PRIO_USER", 2)] {
        const_set(c, p, intern(n.as_bytes()), Value::fix(v));
    }
    for (n, v) in RLIMITS { const_set(c, p, intern(format!("RLIMIT_{}", n).as_bytes()), Value::fix(*v as i64)); }
    let inf = c.alloc(vmr.c_integer.get(), Body::Bignum(Box::new(crate::bignum::BigInt::from_i128(u64::MAX as i128))));
    crate::heap::obj_set_flags(inf, FL_FROZEN);
    for n in ["RLIM_INFINITY", "RLIM_SAVED_MAX", "RLIM_SAVED_CUR"] { const_set(c, p, intern(n.as_bytes()), inf); }
    let st = define_class_under(c, p, "Status", vmr.c_object.get());
    let sd = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, st, n, f, a);
    sd(c, "exitstatus", st_exitstatus, 0); sd(c, "to_i", st_to_i, 0); sd(c, "pid", st_pid, 0); sd(c, "success?", st_success, 0); sd(c, "exited?", st_exited, 0);
    sd(c, "signaled?", st_signaled, 0); sd(c, "stopped?", st_stopped, 0); sd(c, "termsig", st_termsig, 0); sd(c, "stopsig", st_stopsig, 0); sd(c, "coredump?", st_coredump, 0);
    sd(c, "to_s", st_to_s, 0); sd(c, "inspect", st_inspect, 0); sd(c, "==", st_eq, 1);
    define_singleton_cfunc(c, st, "wait", st_s_wait, -1);
    crate::builtins::lazy_consts(c, p, &["Tms", "Waiter", "UID", "GID", "Sys"], "saruby/process");
    let sg = define_module(c, "Signal");
    vmr.m_signal.set(sg);
    define_module_function(c, sg, "trap", signal_trap, -1);
    define_module_function(c, sg, "list", signal_list, 0);
    define_module_function(c, sg, "signame", signal_signame, 1);
    let k = vmr.m_kernel.get();
    define_module_function(c, k, "trap", signal_trap, -1);
    define_module_function(c, k, "sleep", k_sleep, -1);
    define_module_function(c, k, "system", k_system, -1);
    define_module_function(c, k, "`", k_backtick, 1);
    define_module_function(c, k, "spawn", k_spawn, -1);
    define_module_function(c, k, "exec", k_exec, -1);
    define_module_function(c, k, "fork", k_fork, 0);
}
