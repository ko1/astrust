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

// (pid << 32) | raw status; pid -1: none yet
static LAST_STATUS: AtomicU64 = AtomicU64::new(0xffff_ffff_0000_0000);
pub fn set_last_status(pid: i32, raw: i32) { LAST_STATUS.store(((pid as u32 as u64) << 32) | raw as u32 as u64, Ordering::Relaxed); }
fn new_status(c: &mut Ctx, pid: i32, raw: i32) -> Value {
    let k = const_get_at(c, c.vm().m_process.get(), intern(b"Status")).unwrap();
    let o = new_object(c, k);
    ivar_set_raw(c, o, intern(b"@pid"), Value::fix(pid as i64));
    ivar_set_raw(c, o, intern(b"@st"), Value::fix(raw as i64));
    o
}
pub fn last_status(c: &mut Ctx) -> Value {
    let w = LAST_STATUS.load(Ordering::Relaxed);
    let (pid, raw) = ((w >> 32) as u32 as i32, w as u32 as i32);
    if pid < 0 { return NIL; }
    new_status(c, pid, raw)
}
fn exited(st: i32) -> bool { st & 0x7f == 0 }
fn exitstatus(st: i32) -> i32 { (st >> 8) & 0xff }
fn signaled(st: i32) -> bool { ((st & 0x7f) + 1) as i8 >> 1 > 0 }
fn stopped(st: i32) -> bool { st & 0xff == 0x7f }
fn st_raw(c: &Ctx, s: Value) -> i32 { let v = ivar_get(c, s, intern(b"@st")); if v.is_fixnum() { v.as_fix() as i32 } else { 0 } }
fn st_pid_of(c: &Ctx, s: Value) -> i64 { let v = ivar_get(c, s, intern(b"@pid")); if v.is_fixnum() { v.as_fix() } else { 0 } }
fn st_exitstatus(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let st = st_raw(c, s); Ok(if exited(st) { Value::fix(exitstatus(st) as i64) } else { NIL }) }
fn st_to_i(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(st_raw(c, s) as i64)) }
fn st_pid(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(st_pid_of(c, s))) }
fn st_success(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let st = st_raw(c, s); Ok(if exited(st) { Value::bool(exitstatus(st) == 0) } else { NIL }) }
fn st_exited(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(exited(st_raw(c, s)))) }
fn st_signaled(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(signaled(st_raw(c, s)))) }
fn st_stopped(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(stopped(st_raw(c, s)))) }
fn st_termsig(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let st = st_raw(c, s); Ok(if signaled(st) { Value::fix((st & 0x7f) as i64) } else { NIL }) }
fn st_stopsig(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let st = st_raw(c, s); Ok(if stopped(st) { Value::fix(exitstatus(st) as i64) } else { NIL }) }
fn st_coredump(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(st_raw(c, s) & 0x80 != 0 && signaled(st_raw(c, s)))) }
fn st_desc(c: &Ctx, s: Value) -> String {
    let st = st_raw(c, s);
    let mut d = format!("pid {}", st_pid_of(c, s));
    if stopped(st) { let sg = exitstatus(st); d.push_str(&format!(" stopped SIG{} (signal {})", signame(sg).unwrap_or("?"), sg)); }
    else if signaled(st) { let sg = st & 0x7f; d.push_str(&format!(" SIG{} (signal {})", signame(sg).unwrap_or("?"), sg)); if st & 0x80 != 0 { d.push_str(" (core dumped)"); } }
    else { d.push_str(&format!(" exit {}", exitstatus(st))); }
    d
}
fn st_to_s(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(c.str_new(st_desc(c, s).as_bytes())) }
fn st_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(c.str_new(format!("#<Process::Status: {}>", st_desc(c, s)).as_bytes())) }
fn st_eq(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0].is_fixnum() { return Ok(Value::bool(st_raw(c, s) as i64 == a[0].as_fix())); }
    Ok(Value::bool(a[0].is_heap() && real_class_of(c, a[0]) == real_class_of(c, s) && st_raw(c, a[0]) == st_raw(c, s)))
}

// ---- signals ----------------------------------------------------------------------------------

const SIGNALS: &[(&str, i32)] = &[("EXIT", 0), ("HUP", 1), ("INT", 2), ("QUIT", 3), ("ILL", 4), ("TRAP", 5), ("ABRT", 6), ("IOT", 6), ("BUS", 7), ("FPE", 8), ("KILL", 9),
    ("USR1", 10), ("SEGV", 11), ("USR2", 12), ("PIPE", 13), ("ALRM", 14), ("TERM", 15), ("STKFLT", 16), ("CHLD", 17), ("CLD", 17), ("CONT", 18), ("STOP", 19), ("TSTP", 20),
    ("TTIN", 21), ("TTOU", 22), ("URG", 23), ("XCPU", 24), ("XFSZ", 25), ("VTALRM", 26), ("PROF", 27), ("WINCH", 28), ("IO", 29), ("POLL", 29), ("PWR", 30), ("SYS", 31)];
pub fn signame(n: i32) -> Option<&'static str> { SIGNALS.iter().find(|s| s.1 == n).map(|s| s.0) }
static PENDING: AtomicU64 = AtomicU64::new(0);
static HANDLERS: Global = Global::new(); // Array indexed by signo: nil (default) / String / Proc
extern "C" fn on_signal(sig: i32) { PENDING.fetch_or(1 << sig, Ordering::SeqCst); crate::builtins::blop::signal_wake(); }

pub fn signo_arg(c: &mut Ctx, v: Value) -> Result<i32, Esc> {
    if v.is_fixnum() { let n = v.as_fix(); if !(0..65).contains(&n) { return arg_err(c, format!("invalid signal number ({})", n)); } return Ok(n as i32); }
    let name = if v.is_sym() { crate::symbol::sym_str(v.as_sym()) } else if is_string(v) { String::from_utf8_lossy(c.str_bytes(v)).into_owned() } else {
        return arg_err(c, format!("bad signal type {}", class_name_for_err(c, v)));
    };
    let (neg, n) = match name.strip_prefix('-') { Some(r) => (true, r), None => (false, name.as_str()) };
    let n = n.strip_prefix("SIG").unwrap_or(n);
    match SIGNALS.iter().find(|s| s.0 == n) {
        Some(s) => Ok(if neg { -s.1 } else { s.1 }),
        None => arg_err(c, format!("unsupported signal 'SIG{}'", n)),
    }
}
fn signal_trap(c: &mut Ctx, _s: Value, a: &[Value], blk: *const Block) -> RV {
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
    let (store, action): (Value, sys::SigAction) = if cmd == NIL { (NIL, sys::SigAction::Ignore) } else if is_string(cmd) || cmd.is_sym() {
        let sname = if cmd.is_sym() { crate::symbol::sym_str(cmd.as_sym()) } else { String::from_utf8_lossy(c.str_bytes(cmd)).into_owned() };
        match sname.as_str() {
            "" | "IGNORE" | "SIG_IGN" => (c.str_new(b"IGNORE"), sys::SigAction::Ignore),
            "DEFAULT" | "SIG_DFL" => (UNDEF, sys::SigAction::Default),
            "SYSTEM_DEFAULT" => (c.str_new(b"SYSTEM_DEFAULT"), sys::SigAction::Default),
            "EXIT" => (c.str_new(b"EXIT"), sys::SigAction::Catch(on_signal)),
            _ => return arg_err(c, format!("wrong trap - {}", sname)),
        }
    } else { (cmd, sys::SigAction::Catch(on_signal)) };
    let h = HANDLERS.get(c);
    while c.ary_len(h) <= sig as usize { c.ary_push(h, UNDEF); }
    let old = c.ary_get(h, sig as usize);
    c.ary_mut(h)[sig as usize] = store;
    if sig != 0 { sys::signal(sig, action); }
    Ok(if old == UNDEF { if sig == 0 { NIL } else { c.str_new(b"DEFAULT") } } else { old })
}
fn run_handler(c: &mut Ctx, sig: i32) -> Result<(), Esc> {
    let h = c.ary_get(HANDLERS.get(c), sig as usize);
    if is_string(h) {
        if c.str_bytes(h) == b"EXIT" { let e = make_exception(c, c.vm().c_system_exit.get(), "exit"); ivar_set_raw(c, e, crate::symbol::s::status, Value::fix(0)); return Err(raise_exc(c, e)); }
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
/// trap("EXIT") handler, run after the at_exit procs.
pub fn run_exit_trap(c: &mut Ctx) -> Result<(), Esc> {
    let hs = HANDLERS.get(c);
    let h = c.ary_get(hs, 0);
    let r = if h == NIL || h == UNDEF || is_string(h) { Ok(()) } else {
        c.ary_mut(hs)[0] = NIL;
        funcall(c, h, intern(b"call"), &[Value::fix(0)], std::ptr::null()).map(|_| ())
    };
    if r.is_ok() { crate::builtins::misc::run_finalizers(c); }
    r
}
fn signal_list(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let h = c.hash_new();
    for (n, v) in SIGNALS { let k = c.str_new(n.as_bytes()); crate::hash::hash_set(c, h, k, Value::fix(*v as i64))?; }
    Ok(h)
}
fn signal_signame(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = num_to_long(c, a[0])? as i32;
    Ok(match signame(n) { Some(s) => c.str_new(s.as_bytes()), None => NIL })
}
fn p_kill(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
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

fn k_sleep(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    check_argc(c, args, 0, 1)?;
    let t0 = std::time::Instant::now();
    let dur = match args.first() {
        Some(&v) if v != NIL => {
            if !crate::numeric::is_numeric(v) && !crate::builtins::rational::is_rational(c, v) { return type_err(c, format!("can't convert {} into time interval", class_name_for_err(c, v))); }
            let secs = crate::builtins::numeric_methods::to_f(c, v)?;
            if secs < 0.0 { return arg_err(c, "time interval must not be negative"); }
            Some(secs)
        }
        _ => None,
    };
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
    let mut sp = Spec { argv: vec![], prog: CString::new("").unwrap(), env: vec![], clear_env: false, chdir: None, redirect: vec![], close: vec![], opened: vec![], pgroup: None, umask: None, close_others: vec![] };
    if args.is_empty() { return argc_err(c, 0, 1, -1); }
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
                "pgroup" => { sp.pgroup = if v == TRUE { Some(0) } else if v.is_fixnum() { Some(v.as_fix() as i32) } else { None }; continue; }
                "new_pgroup" => { continue; }
                "umask" => { sp.umask = Some(num_to_long(c, v)? as u32); continue; }
                "unsetenv_others" => { sp.clear_env = v.truthy(); continue; }
                "close_others" => {
                    if v.truthy() {
                        // the fds open now (listed before fork: the child must not allocate)
                        if let Ok(d) = std::fs::read_dir("/proc/self/fd") {
                            sp.close_others = d.filter_map(|e| e.ok()?.file_name().to_str()?.parse::<i32>().ok()).filter(|&fd| fd >= 3).collect();
                        }
                    } else { sp.close_others.clear(); }
                    continue;
                }
                "exception" => { continue; }
                "in" | "out" | "err" => {}
                o => { if o.starts_with("rlimit_") { continue; } return arg_err(c, format!("wrong exec option symbol: {}", o)); }
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
            let mut om = crate::builtins::io::parse_mode(c, &rest)?;
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
fn child_setup(sp: &Spec) -> bool {
    if let Some(pg) = sp.pgroup { sys::setpgid(0, pg); }
    if let Some(d) = &sp.chdir { if sys::chdir(d) < 0 { return false; } }
    if let Some(m) = sp.umask { sys::umask(m); }
    if sp.clear_env { sys::clearenv(); }
    for (k, v) in &sp.env { match v { Some(v) => { sys::setenv(k, v); } None => { sys::unsetenv(k); } } }
    for &(dst, src) in &sp.redirect {
        if dst == src { let f = sys::fcntl(src, sys::F_GETFD, 0); sys::fcntl(src, sys::F_SETFD, f & !sys::FD_CLOEXEC); }
        else if sys::dup2(src, dst) < 0 { return false; }
    }
    for &fd in &sp.close { sys::close(fd); }
    for &fd in &sp.close_others {
        if sp.redirect.iter().any(|&(dst, _)| dst == fd) { continue; }
        let f = sys::fcntl(fd, sys::F_GETFD, 0);
        if f >= 0 { sys::fcntl(fd, sys::F_SETFD, f | sys::FD_CLOEXEC); }
    }
    for sig in [13, 2, 3, 15, 1, 10, 12, 14, 17] { sys::signal(sig, sys::SigAction::Default); }
    true
}
fn do_exec(sp: &Spec) { sys::execvp(&sp.prog, &sp.argv); }
/// fork + exec; an exec failure in the child is reported as the parent's Errno.
pub fn spawn_exec(c: &mut Ctx, sp: &Spec) -> Result<i32, Esc> {
    let mut ep = [0i32; 2];
    if sys::pipe2(&mut ep, sys::O_CLOEXEC) < 0 { return Err(raise_errno(c, sys::errno(), "pipe")); }
    crate::builtins::io::flush_all(c);
    let pid = sys::fork();
    if pid < 0 { let e = sys::errno(); sys::close(ep[0]); sys::close(ep[1]); return Err(raise_errno(c, e, "fork(2)")); }
    if pid == 0 {
        if child_setup(sp) { do_exec(sp); }
        let e = (sys::errno() as i32).to_ne_bytes();
        sys::write(ep[1], &e);
        sys::_exit(127);
    }
    sys::close(ep[1]);
    let mut eb = [0u8; 4];
    let n = sys::read_fd(ep[0], &mut eb).unwrap_or(0);
    sys::close(ep[0]);
    if n == 4 {
        let mut st = 0;
        sys::waitpid(pid, &mut st, 0);
        set_last_status(pid, 127 << 8);
        return Err(raise_errno_plain(c, i32::from_ne_bytes(eb) as i64, &sp.prog.to_string_lossy()));
    }
    Ok(pid)
}
/// Split spawn-style arguments into (env, command words, options).
fn split_args(a: &[Value]) -> (Option<Value>, &[Value], Option<Value>) {
    let (env, rest) = match a.first() { Some(&e) if is_hash(e) && a.len() > 1 => (Some(e), &a[1..]), _ => (None, a) };
    match rest.last() { Some(&l) if is_hash(l) && rest.len() > 1 => (env, &rest[..rest.len() - 1], Some(l)), _ => (env, rest, None) }
}
fn waitpid_retry(c: &mut Ctx, pid: i32, flags: i32) -> Result<(i32, i32), Esc> {
    if flags & WNOHANG == 0 && crate::builtins::thread::active(c) { return waitpid_coop(c, pid, flags)?.map_err(|e| raise_errno(c, e, "")); }
    loop {
        let mut st = 0;
        let r = sys::waitpid(pid, &mut st, flags);
        if r < 0 && sys::errno() == sys::EINTR { run_pending_signals(c)?; continue; }
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
    if !crate::builtins::thread::active(c) { wait_child(pid); return Ok(()); }
    if let Ok((_, st)) = waitpid_coop(c, pid, 0)? { set_last_status(pid, st); }
    Ok(())
}
/// Reap a popen child (called from IO#close).
pub fn wait_child(pid: i32) {
    let mut st = 0;
    loop { let r = sys::waitpid(pid, &mut st, 0); if r >= 0 { set_last_status(pid, st); break; } if sys::errno() != sys::EINTR { break; } }
}
fn k_spawn(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (env, cmd, opts) = split_args(a);
    let sp = spawn_spec(c, env, cmd, opts)?;
    Ok(Value::fix(spawn_exec(c, &sp)? as i64))
}
fn k_system(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (env, cmd, opts) = split_args(a);
    let exc = match opts { Some(h) => crate::hash::hash_get_opt(c, h, sym_value("exception"))?.map(|v| v.truthy()).unwrap_or(false), None => false };
    let sp = spawn_spec(c, env, cmd, opts)?;
    let pid = match spawn_exec(c, &sp) {
        Ok(p) => p,
        Err(e) => { if exc { return Err(e); } c.esc = EscKind::None; return Ok(NIL); }
    };
    let (_, st) = waitpid_retry(c, pid, 0)?;
    set_last_status(pid, st);
    if exc && !(exited(st) && exitstatus(st) == 0) {
        let ns = new_status(c, pid, st); let d = st_desc(c, ns);
        return err(c, c.vm().c_runtime_error.get(), format!("Command failed with {}: {}", d.splitn(3, ' ').nth(2).unwrap_or(""), sp.argv.last().map(|a| a.to_string_lossy().into_owned()).unwrap_or_default()));
    }
    Ok(Value::bool(exited(st) && exitstatus(st) == 0))
}
fn k_backtick(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let mut sp = spawn_spec(c, None, &a[..1], None)?;
    let mut p = [0i32; 2];
    if sys::pipe2(&mut p, sys::O_CLOEXEC) < 0 { return Err(raise_errno(c, sys::errno(), "pipe")); }
    sp.redirect.push((1, p[1]));
    let pid = spawn_exec(c, &sp);
    sys::close(p[1]);
    let pid = match pid { Ok(p) => p, Err(e) => { sys::close(p[0]); return Err(e); } };
    let mut out = vec![]; let mut buf = [0u8; 65536];
    let coop = crate::builtins::thread::active(c);
    loop {
        if coop { if let Err(e) = crate::builtins::thread::wait_readable(c, p[0]) { sys::close(p[0]); return Err(e); } }
        match sys::read_fd(p[0], &mut buf) { Ok(0) | Err(_) => break, Ok(n) => out.extend_from_slice(&buf[..n]) }
    }
    sys::close(p[0]);
    let (_, st) = waitpid_retry(c, pid, 0)?;
    set_last_status(pid, st);
    Ok(c.str_from_vec_enc(out, crate::builtins::encoding::default_external()))
}
fn k_exec(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (env, cmd, opts) = split_args(a);
    let sp = spawn_spec(c, env, cmd, opts)?;
    crate::builtins::io::flush_all(c);
    if child_setup(&sp) { do_exec(&sp); }
    Err(raise_errno_plain(c, sys::errno(), &sp.prog.to_string_lossy()))
}
/// Block every signal in the calling (non-interpreter) thread.
pub fn block_signals_in_this_thread() {
    sys::block_all_signals();
}
fn p_itimer(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
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
fn k_fork(c: &mut Ctx, _s: Value, _a: &[Value], blk: *const Block) -> RV {
    crate::builtins::io::flush_all(c);
    let pid = sys::fork();
    if pid < 0 { return Err(raise_errno(c, sys::errno(), "fork(2)")); }
    if pid == 0 {
        after_fork_child(c);
        if blk.is_null() { return Ok(NIL); }
        let r = yield_block(c, blk, &[]);
        child_exit(c, r);
    }
    Ok(Value::fix(pid as i64))
}
fn wait_impl(c: &mut Ctx, a: &[Value]) -> Result<Option<(i32, i32)>, Esc> {
    check_argc(c, a, 0, 2)?;
    let pid = match a.first() { Some(&p) if p != NIL => num_to_long(c, p)? as i32, _ => -1 };
    let flags = match a.get(1) { Some(&f) if f != NIL => num_to_long(c, f)? as i32, _ => 0 };
    out_flush();
    let (r, st) = waitpid_retry(c, pid, flags)?;
    if r == 0 { return Ok(None); }
    set_last_status(r, st);
    Ok(Some((r, st)))
}
fn p_wait(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(match wait_impl(c, a)? { Some((p, _)) => Value::fix(p as i64), None => NIL }) }
fn p_wait2(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    Ok(match wait_impl(c, a)? { Some((p, st)) => { let s = new_status(c, p, st); c.ary_new_from(&[Value::fix(p as i64), s]) } None => NIL })
}
fn p_waitall(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let out = c.ary_new();
    loop {
        let mut st = 0;
        let r = sys::waitpid(-1, &mut st, 0);
        if r < 0 { if sys::errno() == sys::EINTR { run_pending_signals(c)?; continue; } break; }
        set_last_status(r, st);
        let s = new_status(c, r, st);
        let pair = c.ary_new_from(&[Value::fix(r as i64), s]);
        c.ary_push(out, pair);
    }
    Ok(out)
}
fn p_detach(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    // no native threads: a Thread-like object whose value/join waits for the child
    let pid = a[0];
    let k = const_get_at(c, c.vm().m_process.get(), intern(b"Waiter")).unwrap();
    let o = new_object(c, k);
    ivar_set_raw(c, o, intern(b"@pid"), pid);
    Ok(o)
}
fn waiter_value(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let v = ivar_get(c, s, intern(b"@value"));
    if v != NIL { return Ok(v); }
    let pid = ivar_get(c, s, intern(b"@pid"));
    let r = match wait_impl(c, &[pid]) { Ok(Some((p, st))) => new_status(c, p, st), Ok(None) => NIL, Err(_) => { c.esc = EscKind::None; NIL } };
    ivar_set_raw(c, s, intern(b"@value"), r);
    Ok(r)
}
fn waiter_join(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { waiter_value(c, s, a, b)?; Ok(s) }
fn waiter_pid(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(c, s, intern(b"@pid"))) }
fn waiter_alive(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    if ivar_get(c, s, intern(b"@value")) != NIL { return Ok(FALSE); }
    let pid = ivar_get(c, s, intern(b"@pid"));
    match wait_impl(c, &[pid, Value::fix(1)]) { Ok(Some((p, st))) => { let v = new_status(c, p, st); ivar_set_raw(c, s, intern(b"@value"), v); Ok(FALSE) } Ok(None) => Ok(TRUE), Err(_) => { c.esc = EscKind::None; Ok(FALSE) } }
}

// ---- clocks / ids -----------------------------------------------------------------------------

fn clock_id(c: &mut Ctx, v: Value) -> Result<i32, Esc> {
    if v.is_fixnum() { return Ok(v.as_fix() as i32); }
    if v.is_sym() { return Ok(match crate::symbol::sym_str(v.as_sym()).as_str() {
        "GETTIMEOFDAY_BASED_CLOCK_REALTIME" | "TIME_BASED_CLOCK_REALTIME" => 0, "TIMES_BASED_CLOCK_MONOTONIC" => 1,
        "GETRUSAGE_BASED_CLOCK_PROCESS_CPUTIME_ID" | "TIMES_BASED_CLOCK_PROCESS_CPUTIME_ID" | "CLOCK_BASED_CLOCK_PROCESS_CPUTIME_ID" => 2,
        _ => return Err(raise_errno(c, 22, "clock_gettime")),
    }); }
    type_err(c, format!("no implicit conversion of {} into Integer", class_name_for_err(c, v)))
}
fn timespec_value(c: &mut Ctx, t: &sys::Timespec, unit: Value) -> RV {
    let ns = t.sec as i128 * 1_000_000_000 + t.nsec as i128;
    let secs = t.sec as f64 + t.nsec as f64 * 1e-9;
    if unit == NIL { return Ok(c.float_new(secs)); }
    if !unit.is_sym() { return arg_err(c, "unexpected unit"); }
    Ok(match crate::symbol::sym_str(unit.as_sym()).as_str() {
        "nanosecond" => c.int_from_i128(ns),
        "microsecond" => c.int_from_i128(ns / 1000),
        "millisecond" => c.int_from_i128(ns / 1_000_000),
        "second" => Value::fix(t.sec),
        "float_millisecond" => c.float_new(ns as f64 / 1e6),
        "float_microsecond" => c.float_new(ns as f64 / 1e3),
        "float_second" => c.float_new(secs),
        o => return arg_err(c, format!("unexpected unit: {}", o)),
    })
}
fn clock_gettime(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let id = clock_id(c, a[0])?;
    let mut t = sys::Timespec { sec: 0, nsec: 0 };
    if sys::clock_gettime(id, &mut t) < 0 { return Err(raise_errno(c, sys::errno(), "clock_gettime")); }
    timespec_value(c, &t, a.get(1).copied().unwrap_or(NIL))
}
fn clock_getres(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let id = clock_id(c, a[0])?;
    let mut t = sys::Timespec { sec: 0, nsec: 0 };
    if sys::clock_getres(id, &mut t) < 0 { return Err(raise_errno(c, sys::errno(), "clock_getres")); }
    timespec_value(c, &t, a.get(1).copied().unwrap_or(NIL))
}
fn p_times(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let mut t = sys::Tms { utime: 0, stime: 0, cutime: 0, cstime: 0 };
    sys::times(&mut t);
    let k = const_get_at(c, c.vm().m_process.get(), intern(b"Tms")).unwrap();
    let o = new_object(c, k);
    for (n, v) in [("@utime", t.utime), ("@stime", t.stime), ("@cutime", t.cutime), ("@cstime", t.cstime)] { let f = c.float_new(v as f64 / 100.0); ivar_set_raw(c, o, intern(n.as_bytes()), f); }
    Ok(o)
}
fn p_getrlimit(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let r = num_to_long(c, a[0])? as i32;
    let mut l = sys::Rlimit { cur: 0, max: 0 };
    if sys::getrlimit(r, &mut l) < 0 { return Err(raise_errno(c, sys::errno(), "getrlimit")); }
    let cv = c.int_from_i128(l.cur as i128); let mv = c.int_from_i128(l.max as i128);
    Ok(c.ary_new_from(&[cv, mv]))
}
fn p_exit_bang(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let st = match a.first() { None => 0, Some(&v) if v == TRUE => 0, Some(&v) if v == FALSE => 1, Some(&v) => num_to_long(c, v)? };
    out_flush();
    sys::_exit(st as i32)
}

/// Boot.
pub fn init(c: &mut Ctx) {
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
    mf(c, "getrlimit", p_getrlimit, 1); mf(c, "__itimer", p_itimer, 1); mf(c, "setproctitle", |_c, _s, a, _b| Ok(a[0]), 1); mf(c, "last_status", |c, _s, _a, _b| Ok(last_status(c)), 0);
    mf(c, "argv0", |c, _s, _a, _b| Ok(c.vm().dollar_zero.get()), 0); mf(c, "warmup", |_c, _s, _a, _b| Ok(TRUE), 0);
    mf(c, "groups", |c, _s, _a, _b| Ok(c.ary_new_from(&[Value::fix(sys::getgid() as i64)])), 0);
    mf(c, "daemon", |c, _s, _a, _b| err(c, c.vm().c_not_impl_error.get(), "daemon() function is unimplemented on this machine"), -1);
    for (n, v) in [("CLOCK_REALTIME", 0), ("CLOCK_MONOTONIC", 1), ("CLOCK_PROCESS_CPUTIME_ID", 2), ("CLOCK_THREAD_CPUTIME_ID", 3), ("CLOCK_MONOTONIC_RAW", 4),
                   ("CLOCK_REALTIME_COARSE", 5), ("CLOCK_MONOTONIC_COARSE", 6), ("CLOCK_BOOTTIME", 7), ("WNOHANG", 1), ("WUNTRACED", 2), ("PRIO_PROCESS", 0),
                   ("RLIMIT_CPU", 0), ("RLIMIT_FSIZE", 1), ("RLIMIT_DATA", 2), ("RLIMIT_STACK", 3), ("RLIMIT_CORE", 4), ("RLIMIT_RSS", 5), ("RLIMIT_NPROC", 6),
                   ("RLIMIT_NOFILE", 7), ("RLIMIT_MEMLOCK", 8), ("RLIMIT_AS", 9)] {
        const_set(c, p, intern(n.as_bytes()), Value::fix(v));
    }
    let inf = c.alloc(vmr.c_integer.get(), Body::Bignum(Box::new(crate::bignum::BigInt::from_i128(u64::MAX as i128))));
    crate::heap::obj_set_flags(inf, FL_FROZEN);
    const_set(c, p, intern(b"RLIM_INFINITY"), inf);
    let st = define_class_under(c, p, "Status", vmr.c_object.get());
    let sd = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, st, n, f, a);
    sd(c, "exitstatus", st_exitstatus, 0); sd(c, "to_i", st_to_i, 0); sd(c, "pid", st_pid, 0); sd(c, "success?", st_success, 0); sd(c, "exited?", st_exited, 0);
    sd(c, "signaled?", st_signaled, 0); sd(c, "stopped?", st_stopped, 0); sd(c, "termsig", st_termsig, 0); sd(c, "stopsig", st_stopsig, 0); sd(c, "coredump?", st_coredump, 0);
    sd(c, "to_s", st_to_s, 0); sd(c, "inspect", st_inspect, 0); sd(c, "==", st_eq, 1);
    let tms = define_class_under(c, p, "Tms", vmr.c_object.get());
    for n in ["utime", "stime", "cutime", "cstime"] { define_method_me(c, tms, intern(n.as_bytes()), new_me(intern(n.as_bytes()), tms, VISI_PUBLIC, MKind::AttrReader(intern(format!("@{}", n).as_bytes())))); }
    let w = define_class_under(c, p, "Waiter", vmr.c_object.get());
    define_cfunc(c, w, "value", waiter_value, 0); define_cfunc(c, w, "join", waiter_join, -1); define_cfunc(c, w, "pid", waiter_pid, 0); define_cfunc(c, w, "alive?", waiter_alive, 0);
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
