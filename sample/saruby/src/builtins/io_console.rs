// io/console: console.c of io-console 0.9.5 (the gem CRuby 4.0.10 activates), its termios part.
// Terminal modes are the kernel's struct termios through ioctl(TCGETS / TCSETS) (sys::ioctl_buf),
// IO::Console::Mode keeps one as a hidden 36-byte String.
#![forbid(unsafe_code)]
use crate::array::*;
use crate::builtins::io::{self, RIo};
use crate::builtins::numeric_methods::{num_to_int32, num_to_long};
use crate::builtins::sys;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

const TCGETS: u64 = 0x5401;
const TCSETS: u64 = 0x5402;
const TCFLSH: u64 = 0x540b;
const TIOCGWINSZ: u64 = 0x5413;
const TIOCSWINSZ: u64 = 0x5414;
/// sizeof(struct termios) of the kernel: 4 flag words, c_line, c_cc[19]
const TLEN: usize = 36;
type Term = [u8; TLEN];
const IFLAG: usize = 0; const OFLAG: usize = 1; const CFLAG: usize = 2; const LFLAG: usize = 3;
const CC: usize = 17;
const VTIME: usize = 5; const VMIN: usize = 6;
const IGNBRK: u32 = 0o1; const BRKINT: u32 = 0o2; const PARMRK: u32 = 0o10; const ISTRIP: u32 = 0o40; const INLCR: u32 = 0o100; const IGNCR: u32 = 0o200; const ICRNL: u32 = 0o400; const IXON: u32 = 0o2000;
const OPOST: u32 = 0o1;
const ISIG: u32 = 0o1; const ICANON: u32 = 0o2; const ECHO: u32 = 0o10; const ECHOE: u32 = 0o20; const ECHOK: u32 = 0o40; const ECHONL: u32 = 0o100; const IEXTEN: u32 = 0o100000;
const CSIZE: u32 = 0o60; const CS8: u32 = 0o60; const PARENB: u32 = 0o400;
const CSI: &str = "\x1b[";

fn fl(t: &Term, i: usize) -> u32 { u32::from_ne_bytes([t[i * 4], t[i * 4 + 1], t[i * 4 + 2], t[i * 4 + 3]]) }
fn set_fl(t: &mut Term, i: usize, f: impl Fn(u32) -> u32) { let v = f(fl(t, i)); t[i * 4..i * 4 + 4].copy_from_slice(&v.to_ne_bytes()); }

// ioctl_buf: TCGETS / TCSETS move 36 bytes, TIOC[GS]WINSZ 8: every buffer here is 64
fn getattr(fd: i32) -> Result<Term, i64> {
    let mut b = [0u8; 64];
    if sys::ioctl_buf(fd, TCGETS, &mut b) < 0 { return Err(sys::errno()); }
    let mut t = [0u8; TLEN];
    t.copy_from_slice(&b[..TLEN]);
    Ok(t)
}
/// tcsetattr(TCSANOW), retried on EINTR.
fn setattr(fd: i32, t: &Term) -> Result<(), i64> {
    loop {
        let mut b = [0u8; 64];
        b[..TLEN].copy_from_slice(t);
        if sys::ioctl_buf(fd, TCSETS, &mut b) == 0 { return Ok(()); }
        let e = sys::errno();
        if e != sys::EINTR { return Err(e); }
    }
}

#[derive(Clone, Copy)]
struct RawOpts { vmin: u8, vtime: u8, intr: bool }
#[derive(Clone, Copy)]
enum Setter { Raw(Option<RawOpts>), Cooked, NoEcho, Echo }
fn apply(t: &mut Term, s: Setter) {
    match s {
        Setter::Raw(o) => {
            // cfmakeraw, then ECHOE / ECHOK off
            set_fl(t, IFLAG, |x| x & !(IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON));
            set_fl(t, OFLAG, |x| x & !OPOST);
            set_fl(t, LFLAG, |x| x & !(ECHO | ECHONL | ICANON | ISIG | IEXTEN | ECHOE | ECHOK));
            set_fl(t, CFLAG, |x| (x & !(CSIZE | PARENB)) | CS8);
            t[CC + VMIN] = 1; t[CC + VTIME] = 0;
            if let Some(r) = o {
                t[CC + VMIN] = r.vmin; t[CC + VTIME] = r.vtime;
                if r.intr { set_fl(t, IFLAG, |x| x | BRKINT); set_fl(t, LFLAG, |x| x | ISIG); set_fl(t, OFLAG, |x| x | OPOST); }
            }
        }
        Setter::Cooked => {
            set_fl(t, IFLAG, |x| x | BRKINT | ISTRIP | ICRNL | IXON);
            set_fl(t, OFLAG, |x| x | OPOST);
            set_fl(t, LFLAG, |x| x | ECHO | ECHOE | ECHOK | ECHONL | ICANON | ISIG | IEXTEN);
        }
        Setter::NoEcho => set_fl(t, LFLAG, |x| x & !(ECHO | ECHOE | ECHOK | ECHONL)),
        Setter::Echo => set_fl(t, LFLAG, |x| x | ECHO | ECHOE | ECHOK | ECHONL),
    }
}
fn echo_p(t: &Term) -> bool { fl(t, LFLAG) & (ECHO | ECHONL) != 0 }

fn clamp_uchar(n: i64) -> u8 { n.clamp(0, 255) as u8 }
/// (vtime * 10) as NUM2INT does
fn to_vtime(c: &mut Ctx, v: Value) -> Result<i64, Esc> { let x = funcall(c, v, intern(b"*"), &[Value::fix(10)], Blk::NONE)?; num_to_int32(c, x) }
/// rawmode_opt: min: / time: / intr: (no positional argument).
fn rawmode_opt(c: &mut Ctx, a: &[Value], max: usize) -> Result<(Option<RawOpts>, usize), Esc> {
    let (pos, kw) = kw_split(c, a);
    check_argc(c, pos, 0, max)?;
    let n = pos.len();
    let Some(h) = kw else { return Ok((None, n)) };
    kw_unknown(c, h, &["min", "time", "intr"])?;
    let get = |c: &mut Ctx, k: &str| -> Result<Value, Esc> { Ok(crate::hash::hash_get_opt(c, h, sym_value(k))?.unwrap_or(NIL)) };
    let (vmin, vtime, intr) = (get(c, "min")?, get(c, "time")?, get(c, "intr")?);
    let mut o = RawOpts { vmin: 1, vtime: 0, intr: false };
    let mut set = false;
    if vmin != NIL { o.vmin = clamp_uchar(num_to_int32(c, vmin)?); set = true; }
    if vtime != NIL { let t = to_vtime(c, vtime)?; o.vtime = clamp_uchar(t); set = true; }
    if intr == TRUE { o.intr = true; set = true; } else if intr == FALSE { set = true; } else if intr != NIL {
        let s = crate::rt::srb_tostr(c, intr)?;
        let m = format!("true or false expected as intr: {}", String::from_utf8_lossy(c.str_bytes(s)));
        return arg_err(c, m);
    }
    Ok((if set { Some(o) } else { None }, n))
}

/// (read fd, write fd): rb_io_descriptor of the IO and of its write IO.
fn fds(c: &mut Ctx, v: Value) -> Result<(i32, i32), Esc> {
    let io: &mut RIo = io::get_io(c, v)?;
    Ok((io.fd, if io.wfd >= 0 { io.wfd } else { io.fd }))
}
fn rfd(c: &mut Ctx, v: Value) -> Result<i32, Esc> { Ok(fds(c, v)?.0) }
fn wfd(c: &mut Ctx, v: Value) -> Result<i32, Esc> { Ok(fds(c, v)?.1) }
/// sys_fail(io): rb_sys_fail_str(rb_io_path(io)).
fn sys_fail(c: &mut Ctx, v: Value, e: i64) -> Esc {
    let p = io::rio(c, v).filter(|i| i.has_path).map(|i| String::from_utf8_lossy(&i.path).into_owned());
    match p { Some(p) => crate::builtins::exception::raise_errno_plain(c, e, &p), None => crate::builtins::exception::raise_errno(c, e, "") }
}

/// ttymode: both descriptors in the mode while `f` runs, then back (a failure to restore wins).
fn ttymode(c: &mut Ctx, v: Value, s: Setter, f: impl FnOnce(&mut Ctx) -> RV) -> RV {
    let (r, w) = fds(c, v)?;
    let (mut saved, mut ok, mut error): ([Option<(i32, Term)>; 2], bool, i64) = ([None, None], false, 0);
    for (i, fd) in [r, w].into_iter().enumerate() {
        if fd == -1 || (i == 1 && fd == r) { continue; }
        match getattr(fd) {
            Ok(t) => { let mut n = t; apply(&mut n, s); match setattr(fd, &n) { Ok(()) => { saved[i] = Some((fd, t)); ok = true; } Err(e) => error = e } }
            Err(e) => error = e,
        }
    }
    let res = if ok { Some(f(c)) } else { None };
    let mut failed = !ok;
    for (i, sv) in saved.iter().enumerate() {
        let Some((fd, t)) = sv else { continue };
        let now = fds(c, v)?;
        if *fd != if i == 0 { now.0 } else { now.1 } { continue; }
        if let Err(e) = setattr(*fd, t) { error = e; failed = true; }
    }
    if failed { return Err(crate::builtins::exception::raise_errno(c, error, "")); }
    res.unwrap_or(Ok(NIL))
}

fn m_raw(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let (o, _) = rawmode_opt(c, a, 0)?;
    ttymode(c, slf, Setter::Raw(o), |c| yield1(c, b, slf))
}
fn set_mode_now(c: &mut Ctx, slf: Value, s: Setter) -> RV {
    let fd = rfd(c, slf)?;
    let mut t = match getattr(fd) { Ok(t) => t, Err(e) => return Err(sys_fail(c, slf, e)) };
    apply(&mut t, s);
    if let Err(e) = setattr(fd, &t) { return Err(sys_fail(c, slf, e)); }
    Ok(slf)
}
fn m_set_raw(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let (o, _) = rawmode_opt(c, a, 0)?; set_mode_now(c, slf, Setter::Raw(o)) }
fn m_cooked(c: &mut Ctx, slf: Value, _a: &[Value], b: Blk) -> RV { ttymode(c, slf, Setter::Cooked, |c| yield1(c, b, slf)) }
fn m_set_cooked(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { set_mode_now(c, slf, Setter::Cooked) }
fn m_getch(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (o, _) = rawmode_opt(c, a, 0)?;
    ttymode(c, slf, Setter::Raw(o), |c| funcall(c, slf, intern(b"getc"), &[], Blk::NONE))
}
fn m_input_pending(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let io = io::get_io(c, slf)?;
    if io.rpos < io.rbuf.len() { return Ok(TRUE); }
    let fd = io.fd;
    let r = io::wait_one(c, slf, fd, sys::POLLIN, Some(std::time::Instant::now()), Value::fix(0))?;
    Ok(Value::bool(r.is_some()))
}
fn m_noecho(c: &mut Ctx, slf: Value, _a: &[Value], b: Blk) -> RV { ttymode(c, slf, Setter::NoEcho, |c| yield1(c, b, slf)) }
fn m_set_echo(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let fd = rfd(c, slf)?;
    let mut t = match getattr(fd) { Ok(t) => t, Err(e) => return Err(sys_fail(c, slf, e)) };
    apply(&mut t, if a[0].truthy() { Setter::Echo } else { Setter::NoEcho });
    if let Err(e) = setattr(fd, &t) { return Err(sys_fail(c, slf, e)); }
    Ok(slf)
}
fn m_echo_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let fd = rfd(c, slf)?;
    match getattr(fd) { Ok(t) => Ok(Value::bool(echo_p(&t))), Err(e) => Err(sys_fail(c, slf, e)) }
}

// ---- IO::Console::Mode ----------------------------------------------------------------------

fn term_sym() -> Sym { intern(b"__termios") }
fn mode_class(c: &Ctx) -> Option<Value> {
    let con = const_get_at(c, c.vm().c_io.get(), intern(b"Console"))?;
    const_get_at(c, con, intern(b"Mode"))
}
fn is_mode(c: &Ctx, v: Value) -> bool { mode_class(c).is_some_and(|k| v.is_heap() && crate::builtins::inherits(c, real_class_of(c, v), k)) }
/// rb_check_typeddata of a Mode (an allocated, never-set one is zeroed).
fn mode_get(c: &mut Ctx, v: Value) -> Result<Term, Esc> {
    if !is_mode(c, v) { return type_err(c, format!("wrong argument type {} (expected console-mode)", crate::builtins::kernel::class_name_for_err(c, v))); }
    let s = ivar_get(c, v, term_sym());
    let mut t = [0u8; TLEN];
    if is_string(s) && c.str_bytes(s).len() == TLEN { t.copy_from_slice(c.str_bytes(s)); }
    Ok(t)
}
fn mode_put(c: &mut Ctx, v: Value, t: &Term) { let s = c.str_new_enc(t, crate::object::ENC_BINARY); ivar_set_raw(c, v, term_sym(), s); }
fn mode_new(c: &mut Ctx, k: Value, t: &Term) -> RV { let v = crate::builtins::object::alloc_instance(c, k)?; mode_put(c, v, t); Ok(v) }
fn mode_init_copy(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { mode_get(c, slf)?; let t = mode_get(c, a[0])?; mode_put(c, slf, &t); Ok(slf) }
fn mode_echo(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let t = mode_get(c, slf)?; Ok(Value::bool(echo_p(&t))) }
fn mode_set_echo(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut t = mode_get(c, slf)?;
    apply(&mut t, if a[0].truthy() { Setter::Echo } else { Setter::NoEcho });
    mode_put(c, slf, &t);
    Ok(slf)
}
fn mode_set_raw(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut t = mode_get(c, slf)?;
    let (o, _) = rawmode_opt(c, a, 0)?;
    apply(&mut t, Setter::Raw(o));
    mode_put(c, slf, &t);
    Ok(slf)
}
fn mode_raw(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut t = mode_get(c, slf)?;
    let (o, _) = rawmode_opt(c, a, 0)?;
    apply(&mut t, Setter::Raw(o));
    let k = real_class_of(c, slf);
    mode_new(c, k, &t)
}
fn mode_min(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let t = mode_get(c, slf)?; Ok(Value::fix(t[CC + VMIN] as i64)) }
fn mode_set_min(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut t = mode_get(c, slf)?;
    t[CC + VMIN] = if a[0] == NIL { 1 } else { clamp_uchar(num_to_int32(c, a[0])?) };
    mode_put(c, slf, &t);
    Ok(a[0])
}
fn mode_time(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let t = mode_get(c, slf)?;
    Ok(crate::builtins::rational::rat_new(c, Value::fix(t[CC + VTIME] as i64), Value::fix(10)))
}
fn mode_set_time(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut t = mode_get(c, slf)?;
    t[CC + VTIME] = if a[0] == NIL { 0 } else { let x = to_vtime(c, a[0])?; clamp_uchar(x) };
    mode_put(c, slf, &t);
    Ok(a[0])
}
fn m_console_mode(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let fd = rfd(c, slf)?;
    let t = match getattr(fd) { Ok(t) => t, Err(e) => return Err(sys_fail(c, slf, e)) };
    let Some(k) = mode_class(c) else { return Ok(NIL) };
    mode_new(c, k, &t)
}
fn m_set_console_mode(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let fd = rfd(c, slf)?;
    let t = mode_get(c, a[0])?;
    if let Err(e) = setattr(fd, &t) { return Err(sys_fail(c, slf, e)); }
    Ok(a[0])
}

// ---- window size, flushing ------------------------------------------------------------------

fn m_winsize(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let fd = wfd(c, slf)?;
    let mut b = [0u8; 64];
    if sys::ioctl_buf(fd, TIOCGWINSZ, &mut b) < 0 { let e = sys::errno(); return Err(sys_fail(c, slf, e)); }
    let (r, col) = (u16::from_ne_bytes([b[0], b[1]]), u16::from_ne_bytes([b[2], b[3]]));
    Ok(c.ary_new_from(&[Value::fix(r as i64), Value::fix(col as i64)]))
}
/// NUM2UINT: an unsigned int, negative values wrapping
fn num2uint(c: &mut Ctx, v: Value) -> Result<u32, Esc> {
    let n = num_to_long(c, v)?;
    if n > u32::MAX as i64 || n < i32::MIN as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too {} to convert to 'unsigned int'", n, if n < 0 { "small" } else { "big" })); }
    Ok(n as u32)
}
fn m_set_winsize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let size = crate::builtins::kernel::k_array(c, NIL, &[a[0]], Blk::NONE)?;
    let sz = c.ary(size).to_vec();
    if sz.len() != 2 && sz.len() != 4 { return arg_err(c, format!("wrong number of arguments (given {}, expected 2 or 4)", sz.len())); }
    let fd = wfd(c, slf)?;
    let mut b = [0u8; 64];
    for (i, &x) in sz.iter().enumerate() { let n = if x == NIL { 0 } else { num2uint(c, x)? as u16 }; b[i * 2..i * 2 + 2].copy_from_slice(&n.to_ne_bytes()); }
    if sys::ioctl_buf(fd, TIOCSWINSZ, &mut b) < 0 { let e = sys::errno(); return Err(sys_fail(c, slf, e)); }
    Ok(slf)
}
fn tcflush(c: &mut Ctx, slf: Value, fd: i32, q: i64) -> Result<(), Esc> {
    if sys::ioctl_int(fd, TCFLSH, q) < 0 { let e = sys::errno(); return Err(sys_fail(c, slf, e)); }
    Ok(())
}
fn m_iflush(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let fd = rfd(c, slf)?; tcflush(c, slf, fd, 0)?; Ok(slf) }
fn m_oflush(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let fd = wfd(c, slf)?; tcflush(c, slf, fd, 1)?; Ok(slf) }
fn m_ioflush(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let (r, w) = fds(c, slf)?;
    if w != -1 && r != w { tcflush(c, slf, r, 0)?; tcflush(c, slf, w, 1)?; } else { tcflush(c, slf, r, 2)?; }
    Ok(slf)
}
fn m_beep(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let fd = wfd(c, slf)?;
    if sys::write(fd, b"\x07") < 0 { let e = sys::errno(); return Err(sys_fail(c, slf, e)); }
    Ok(slf)
}

// ---- VT escape sequences --------------------------------------------------------------------

fn write_str(c: &mut Ctx, io: Value, s: &str) -> Result<(), Esc> { let v = c.str_new(s.as_bytes()); funcall(c, io, intern(b"write"), &[v], Blk::NONE)?; Ok(()) }
fn m_goto(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (y, x) = (num2uint(c, a[0])?, num2uint(c, a[1])?);
    write_str(c, slf, &format!("{}{};{}H", CSI, y.wrapping_add(1) as i32, x.wrapping_add(1) as i32))?;
    Ok(slf)
}
fn console_move(c: &mut Ctx, io: Value, y: i32, x: i32) -> RV {
    if x != 0 || y != 0 {
        let mut s = String::new();
        if y != 0 { s += &format!("{}{}{}", CSI, y.unsigned_abs() as i32, if y < 0 { 'A' } else { 'B' }); }
        if x != 0 { s += &format!("{}{}{}", CSI, x.unsigned_abs() as i32, if x < 0 { 'D' } else { 'C' }); }
        write_str(c, io, &s)?;
        funcall(c, io, intern(b"flush"), &[], Blk::NONE)?;
    }
    Ok(io)
}
fn int_arg(c: &mut Ctx, v: Value) -> Result<i32, Esc> { Ok(num_to_int32(c, v)? as i32) }
fn m_cursor_up(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let n = int_arg(c, a[0])?; console_move(c, slf, n.wrapping_neg(), 0) }
fn m_cursor_down(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let n = int_arg(c, a[0])?; console_move(c, slf, n, 0) }
fn m_cursor_left(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let n = int_arg(c, a[0])?; console_move(c, slf, 0, n.wrapping_neg()) }
fn m_cursor_right(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let n = int_arg(c, a[0])?; console_move(c, slf, 0, n) }
fn m_goto_column(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let x = num2uint(c, a[0])?;
    write_str(c, slf, &format!("{}{}G", CSI, x.wrapping_add(1) as i32))?;
    Ok(slf)
}
fn mode_in_range(c: &mut Ctx, v: Value, high: i64, name: &str) -> Result<i64, Esc> {
    if v == NIL { return Ok(0); }
    let bad = |c: &mut Ctx| -> Result<i64, Esc> { let s = crate::rt::srb_tostr(c, v)?; let m = format!("wrong {} mode: {}", name, String::from_utf8_lossy(c.str_bytes(s))); arg_err(c, m) };
    if !is_integer(v) { return bad(c); }
    let m = num_to_int32(c, v)?;
    if m < 0 || m > high { return bad(c); }
    Ok(m)
}
fn m_erase_line(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let m = mode_in_range(c, a[0], 2, "line erase")?; write_str(c, slf, &format!("{}{}K", CSI, m))?; Ok(slf) }
fn m_erase_screen(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let m = mode_in_range(c, a[0], 3, "screen erase")?; write_str(c, slf, &format!("{}{}J", CSI, m))?; Ok(slf) }
fn console_scroll(c: &mut Ctx, io: Value, line: i32) -> RV {
    if line != 0 { write_str(c, io, &format!("{}{}{}", CSI, line.unsigned_abs() as i32, if line < 0 { 'T' } else { 'S' }))?; }
    Ok(io)
}
fn m_scroll_forward(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let n = int_arg(c, a[0])?; console_scroll(c, slf, n) }
fn m_scroll_backward(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let n = int_arg(c, a[0])?; console_scroll(c, slf, n.wrapping_neg()) }
fn m_clear_screen(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    m_erase_screen(c, slf, &[Value::fix(2)], Blk::NONE)?;
    m_goto(c, slf, &[Value::fix(0), Value::fix(0)], Blk::NONE)
}
fn m_hide_cursor(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { write_str(c, slf, "\x1b[?25l")?; Ok(slf) }
fn m_show_cursor(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { write_str(c, slf, "\x1b[?25h")?; Ok(slf) }
/// read_vt_response: "\e[" numbers separated by ';' and a final byte.
fn read_vt_response(c: &mut Ctx, io: Value, query: &str) -> RV {
    if io::is_io(c, io) { write_str(c, io, query)?; funcall(c, io, intern(b"flush"), &[], Blk::NONE)?; } else { return Ok(NIL); }
    let gb = |c: &mut Ctx| funcall(c, io, intern(b"getbyte"), &[], Blk::NONE);
    if gb(c)? != Value::fix(0x1b) { return Ok(NIL); }
    if gb(c)? != Value::fix(b'[' as i64) { return Ok(NIL); }
    let res = c.ary_new();
    let mut num: i64 = 0;
    let mut last = NIL;
    loop {
        let b = gb(c)?;
        if b == NIL { break; }
        let ch = b.as_fix() as u8;
        if ch == b';' { c.ary_push(res, Value::fix(num)); num = 0; }
        else if ch.is_ascii_digit() { num = num * 10 + (ch - b'0') as i64; }
        else { c.ary_push(res, Value::fix(num)); last = c.str_new(&[ch]); break; }
    }
    c.ary_push(res, last);
    Ok(res)
}
fn m_cursor(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let resp = ttymode(c, slf, Setter::Raw(None), |c| read_vt_response(c, slf, "\x1b[6n"))?;
    if !is_array(resp) || c.ary_len(resp) != 3 { return Ok(NIL); }
    let term = c.ary_get(resp, 2);
    if !is_string(term) || c.str_bytes(term) != b"R" { return Ok(NIL); }
    let (r, col) = (c.ary_get(resp, 0), c.ary_get(resp, 1));
    let (r, col) = (num2uint(c, r)?.wrapping_sub(1), num2uint(c, col)?.wrapping_sub(1));
    let (rv, cv) = (c.int_from_i64(r as i64), c.int_from_i64(col as i64));
    Ok(c.ary_new_from(&[rv, cv]))
}
fn m_set_cursor(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let Some(p) = crate::array::check_array(c, a[0])? else {
        return type_err(c, format!("no implicit conversion of {} into Array", crate::builtins::kernel::class_name_for_err(c, a[0])));
    };
    if c.ary_len(p) != 2 { return arg_err(c, "expected 2D coordinate"); }
    let (y, x) = (c.ary_get(p, 0), c.ary_get(p, 1));
    m_goto(c, slf, &[y, x], Blk::NONE)
}
fn not_impl(c: &mut Ctx, name: &str) -> RV { err(c, c.vm().c_not_impl_error.get(), format!("{}() function is unimplemented on this machine", name)) }
fn m_pressed(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { not_impl(c, "pressed?") }
fn m_input_events(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { not_impl(c, "console_input_events") }
fn m_winsize_changed(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { not_impl(c, "check_winsize_changed") }

// ---- getpass, ttyname, IO.console -----------------------------------------------------------

fn default_rs(c: &mut Ctx) -> Value { c.str_new_enc(b"\n", crate::object::ENC_ASCII) }
fn m_getpass(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    // the write IO; STDIN prompts on STDERR
    let wio = if slf == c.vm().stdin.get() { c.vm().stderr.get() } else { slf };
    if let Some(&p) = a.first().filter(|&&p| p != NIL) {
        let s = expect_str(c, p)?;
        if c.str_bytes(s).contains(&0) { return arg_err(c, "string contains null byte"); }
        funcall(c, wio, intern(b"write"), &[s], Blk::NONE)?;
    }
    funcall(c, wio, intern(b"flush"), &[], Blk::NONE)?;
    let r = ttymode(c, slf, Setter::NoEcho, |c| funcall(c, slf, intern(b"gets"), &[], Blk::NONE));
    let rs = default_rs(c);
    funcall(c, wio, intern(b"write"), &[rs], Blk::NONE)?;
    let r = r?;
    if r != NIL { funcall(c, r, intern(b"chomp!"), &[rs], Blk::NONE)?; }
    Ok(r)
}
fn m_ttyname(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let fd = rfd(c, slf)?;
    if sys::isatty(fd) != 1 { return Ok(NIL); }
    // ttyname_r reads the same link
    match std::fs::read_link(format!("/proc/self/fd/{}", fd)) {
        Ok(p) => { use std::os::unix::ffi::OsStrExt; let s = c.str_new_enc(p.as_os_str().as_bytes(), crate::object::ENC_ASCII); obj_set_flags(s, FL_FROZEN); Ok(s) }
        Err(e) => Err(crate::builtins::exception::raise_errno_plain(c, crate::builtins::exception::io_err_to_errno(&e), &format!("ttyname_r({})", fd))),
    }
}
fn console_sym() -> Sym { intern(b"__console_dev") }
fn s_console(c: &mut Ctx, klass: Value, a: &[Value], _b: Blk) -> RV {
    let sym = match a.first() {
        Some(&s) if s.is_sym() => Some(s),
        Some(&s) => return type_err(c, format!("wrong argument type {} (expected Symbol)", crate::builtins::kernel::class_name_for_err(c, s))),
        None => None,
    };
    let klass = if klass == c.vm().c_io.get() { c.vm().c_file.get() } else { klass };
    // one console per process (ractor-local storage in CRuby), kept on IO
    let holder = c.vm().c_io.get();
    let mut con = ivar_get(c, holder, console_sym());
    if con != NIL && io::rio(c, con).map(|i| i.closed || !i.init).unwrap_or(true) { ivar_set_raw(c, holder, console_sym(), NIL); con = NIL; }
    if let Some(s) = sym {
        if s == sym_value("close") && a.len() == 1 {
            if con != NIL { crate::builtins::io::close_io(c, con)?; ivar_set_raw(c, holder, console_sym(), NIL); }
            return Ok(NIL);
        }
    }
    if con == NIL {
        let fd = sys::open(c"/dev/tty", 2 | sys::O_CLOEXEC, 0);
        if fd < 0 { return Ok(NIL); }
        con = io::new_io(c, klass, fd, b"/dev/tty", true, true);
        if let Some(i) = c.data_mut::<RIo>(con) { i.sync = true; i.has_path = true; }
        ivar_set_raw(c, holder, console_sym(), con);
    }
    if let Some(s) = sym { return funcall(c, con, intern(b"send"), &[&[s][..], &a[1..]].concat(), Blk::NONE); }
    Ok(con)
}
/// IO::platform_tty#tty?: nil or :any (the only kinds on this platform), then IO#tty?.
fn platform_tty_p(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    for &m in a {
        if m == NIL { continue; }
        if !m.is_sym() { return type_err(c, format!("wrong argument type {} (expected Symbol)", crate::builtins::kernel::class_name_for_err(c, m))); }
        if m != sym_value("any") { let s = crate::builtins::kernel::inspect_str(c, m)?; return arg_err(c, format!("unknown tty type: {}", s)); }
    }
    let io = io::get_io(c, slf)?;
    Ok(Value::bool(sys::isatty(io.fd) == 1))
}

pub fn init(c: &mut Ctx) {
    let io = c.vm().c_io.get();
    let con = define_module_under(c, io, "Console");
    let ms: &[(&str, CFunc, i32)] = &[("raw", m_raw, -1), ("raw!", m_set_raw, -1), ("cooked", m_cooked, 0), ("cooked!", m_set_cooked, 0),
        ("getch", m_getch, -1), ("input_pending?", m_input_pending, 0), ("echo=", m_set_echo, 1), ("echo?", m_echo_p, 0),
        ("console_mode", m_console_mode, 0), ("console_mode=", m_set_console_mode, 1), ("noecho", m_noecho, 0), ("winsize", m_winsize, 0),
        ("winsize=", m_set_winsize, 1), ("iflush", m_iflush, 0), ("oflush", m_oflush, 0), ("ioflush", m_ioflush, 0), ("beep", m_beep, 0),
        ("goto", m_goto, 2), ("cursor", m_cursor, 0), ("cursor=", m_set_cursor, 1), ("hide_cursor", m_hide_cursor, 0), ("show_cursor", m_show_cursor, 0),
        ("cursor_up", m_cursor_up, 1), ("cursor_down", m_cursor_down, 1), ("cursor_left", m_cursor_left, 1), ("cursor_right", m_cursor_right, 1),
        ("goto_column", m_goto_column, 1), ("erase_line", m_erase_line, 1), ("erase_screen", m_erase_screen, 1), ("scroll_forward", m_scroll_forward, 1),
        ("scroll_backward", m_scroll_backward, 1), ("clear_screen", m_clear_screen, 0), ("pressed?", m_pressed, 1), ("console_input_events", m_input_events, -1),
        ("check_winsize_changed", m_winsize_changed, 0), ("getpass", m_getpass, -1), ("ttyname", m_ttyname, 0)];
    for &(n, f, ar) in ms { define_cfunc(c, io, n, f, ar); }
    let pt = define_module_under(c, io, "platform_tty");
    define_cfunc(c, pt, "tty?", platform_tty_p, -1);
    define_cfunc(c, pt, "isatty", platform_tty_p, -1);
    prepend_module(c, io, pt);
    define_singleton_cfunc(c, io, "console", s_console, -1);
    crate::builtins::stringio::init_console(c);
    let ver = c.str_new(b"0.9.5");
    obj_set_flags(ver, FL_FROZEN);
    const_set(c, con, intern(b"VERSION"), ver);
    let ob = c.vm().c_object.get();
    let mode = define_class_under(c, con, "Mode", ob);
    const_set(c, io, intern(b"ConsoleMode"), mode);
    const_set(c, mode, intern(b"VERSION"), ver);
    let _ = funcall(c, io, intern(b"deprecate_constant"), &[sym_value("ConsoleMode")], Blk::NONE);
    let _ = funcall(c, mode, intern(b"deprecate_constant"), &[sym_value("VERSION")], Blk::NONE);
    let _ = funcall(c, mode, intern(b"undef_method"), &[sym_value("initialize")], Blk::NONE);
    let mm: &[(&str, CFunc, i32)] = &[("initialize_copy", mode_init_copy, 1), ("echo?", mode_echo, 0), ("echo", mode_echo, 0), ("echo=", mode_set_echo, 1),
        ("raw!", mode_set_raw, -1), ("raw", mode_raw, -1), ("min", mode_min, 0), ("min=", mode_set_min, 1), ("time", mode_time, 0), ("time=", mode_set_time, 1)];
    for &(n, f, ar) in mm { define_cfunc(c, mode, n, f, ar); }
}
