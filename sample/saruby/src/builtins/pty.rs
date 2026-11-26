// PTY: ext/pty/pty.c of CRuby 4.0.10 (posix_openpt path). The pty pair comes from /dev/ptmx with
// ioctl(TIOCSPTLCK / TIOCGPTN) (what glibc's unlockpt / ptsname do); the child of spawn starts a session,
// takes the slave as its controlling terminal and its standard streams (process::Spec::pty).
#![forbid(unsafe_code)]
use crate::builtins::io;
use crate::builtins::process;
use crate::builtins::sys;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::ffi::CString;

const TIOCGPTN: u64 = 0x8004_5430;
const TIOCSPTLCK: u64 = 0x4004_5431;
const WUNTRACED: i32 = 2;
const F_DUPFD_CLOEXEC: i32 = 1030;

/// get_device_once: (master, slave, slave name).
fn device_once(nomesg: bool) -> Option<(i32, i32, String)> {
    let m = sys::open(c"/dev/ptmx", sys::O_RDWR | sys::O_NOCTTY | sys::O_CLOEXEC, 0);
    if m < 0 { return None; }
    let fail = |fds: &[i32]| { for &f in fds { sys::close(f); } None };
    // ioctl_buf: both requests move one int; the buffer is larger
    let mut b = [0u8; 16];
    if sys::ioctl_buf(m, TIOCSPTLCK, &mut b) < 0 { return fail(&[m]); }
    let mut n = [0u8; 16];
    if sys::ioctl_buf(m, TIOCGPTN, &mut n) < 0 { return fail(&[m]); }
    let name = format!("/dev/pts/{}", u32::from_ne_bytes([n[0], n[1], n[2], n[3]]));
    let cn = CString::new(name.clone()).ok()?;
    if nomesg && sys::chmod(&cn, 0o600) < 0 { return fail(&[m]); }
    let s = sys::open(&cn, sys::O_RDWR | sys::O_NOCTTY | sys::O_CLOEXEC, 0);
    if s < 0 { return fail(&[m]); }
    Some((m, s, name))
}
/// getDevice: once more after a GC (descriptors may be waiting for finalization), then RuntimeError.
fn device(c: &mut Ctx, nomesg: bool) -> Result<(i32, i32, String), Esc> {
    if let Some(d) = device_once(nomesg) { return Ok(d); }
    let gc = c.vm().m_gc.get();
    funcall(c, gc, intern(b"start"), &[], Blk::NONE)?;
    match device_once(nomesg) { Some(d) => Ok(d), None => err(c, c.vm().c_runtime_error.get(), "can't get Master/Slave device") }
}
fn open_io(c: &mut Ctx, klass: Value, fd: i32, path: &str, r: bool, w: bool, sync: bool) -> Value {
    let v = io::new_io(c, klass, fd, path.as_bytes(), r, w);
    if let Some(i) = c.data_mut::<io::RIo>(v) { i.has_path = true; i.sync = sync; }
    v
}
fn close_all(c: &mut Ctx, pair: Value) -> Result<(), Esc> {
    for i in 0..2 { let v = c.ary_get(pair, i); if io::is_io(c, v) { io::close_io(c, v)?; } }
    Ok(())
}
fn pty_open(c: &mut Ctx, _s: Value, _a: &[Value], b: Blk) -> RV {
    let (m, s, name) = device(c, true)?;
    let (ki, kf) = (c.vm().c_io.get(), c.vm().c_file.get());
    let mio = open_io(c, ki, m, &format!("masterpty:{}", name), true, true, true);
    let sio = open_io(c, kf, s, &name, true, true, true);
    let pair = c.ary_new_from(&[mio, sio]);
    if b.is_null() { return Ok(pair); }
    let r = yield1(c, b, pair);
    close_all(c, pair)?;
    r
}
/// The shell for a spawn without a command: $SHELL, the user's login shell, /bin/sh.
fn default_shell() -> String {
    if let Ok(s) = std::env::var("SHELL") { return s; }
    if let Some(pw) = std::env::var("USER").ok().and_then(|u| std::ffi::CString::new(u).ok()).and_then(|u| sys::getpwnam(&u)) {
        return String::from_utf8_lossy(&pw.shell).into_owned();
    }
    "/bin/sh".into()
}
fn pty_spawn(c: &mut Ctx, _s: Value, a: &[Value], b: Blk) -> RV {
    let shell;
    let a = if a.is_empty() { shell = [c.str_new(default_shell().as_bytes())]; &shell[..] } else { a };
    let (env, cmd, opts) = process::split_args(c, a)?;
    let mut sp = process::spawn_spec(c, env, &a[cmd], opts)?;
    let (m, s, name) = device(c, false)?;
    sp.pty = Some((m, s));
    let pid = process::spawn_exec(c, &mut sp);
    sys::close(s);
    let pid = match pid { Ok(p) => p, Err(e) => { sys::close(m); return Err(e); } };
    let kf = c.vm().c_file.get();
    let r = open_io(c, kf, m, &name, true, false, false);
    let wfd = sys::fcntl(m, F_DUPFD_CLOEXEC, 3);
    if wfd < 0 { let e = sys::errno(); return Err(crate::builtins::exception::raise_errno_plain(c, e, "dup()")); }
    let w = open_io(c, kf, wfd, &name, false, true, true);
    let res = c.ary_new_from(&[r, w, Value::fix(pid as i64)]);
    if b.is_null() { return Ok(res); }
    let y = yield1(c, b, res);
    // pty_detach_process: a child still running is left to Process.detach
    let mut st = 0;
    if sys::waitpid(pid, &mut st, 1) > 0 {
        process::set_last_status(c, pid, st);
        let pm = c.vm().m_process.get();
        funcall(c, pm, intern(b"detach"), &[Value::fix(pid as i64)], Blk::NONE)?;
    }
    y?;
    Ok(NIL)
}
fn pty_check(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let pid = crate::builtins::numeric_methods::num_to_long(c, a[0])? as i32;
    let mut st = 0;
    let r = loop { let r = sys::waitpid(pid, &mut st, 1 | WUNTRACED); if r < 0 && sys::errno() == sys::EINTR { continue; } break r; };
    if r <= 0 { return Ok(NIL); }
    process::set_last_status(c, r, st);
    let ls = process::last_status(c);
    if !a.get(1).is_some_and(|x| x.truthy()) { return Ok(ls); }
    let state = if st & 0xff == 0x7f { "stopped" } else if sys::kill(r, 0) == 0 { "changed" } else { "exited" };
    let pty = const_get_at(c, c.vm().c_object.get(), intern(b"PTY")).unwrap_or(NIL);
    let k = const_get_at(c, pty, intern(b"ChildExited")).unwrap_or(NIL);
    let e = make_exception(c, k, &format!("pty - {}: {}", state, r));
    ivar_set_raw(c, e, intern(b"status"), ls);
    Err(raise_exc(c, e))
}
fn child_exited_status(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, slf, intern(b"status"))) }

pub fn init(c: &mut Ctx) {
    let m = define_module(c, "PTY");
    define_module_function(c, m, "getpty", pty_spawn, -1);
    define_module_function(c, m, "spawn", pty_spawn, -1);
    define_singleton_cfunc(c, m, "check", pty_check, -1);
    define_singleton_cfunc(c, m, "open", pty_open, 0);
    let re = c.vm().c_runtime_error.get();
    let ce = define_class_under(c, m, "ChildExited", re);
    define_cfunc(c, ce, "status", child_exited_status, 0);
}
