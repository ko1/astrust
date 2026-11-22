// IO: fd-based streams with read/write buffers (File/Dir/Stat live in file.rs).
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::obj_set_flags;
use crate::array::*;
use crate::builtins::exception::raise_errno;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::builtins::sys;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;
use std::os::unix::ffi::OsStrExt;

pub static STDOUT_SYNC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub struct RIo {
    pub fd: i32, pub wfd: i32, pub path: Vec<u8>, pub rbuf: Vec<u8>, pub rpos: usize, pub eof: bool, pub wbuf: Vec<u8>,
    pub closed: bool, pub init: bool, pub lineno: i64, pub writable: bool, pub readable: bool, pub binmode: bool, pub sync: bool,
    pub pid: i32, pub enc: Option<u8>, pub enc2: Option<u8>, pub ecflags: u8, pub autoclose: bool, pub has_path: bool, pub data: bool, pub rerr: i64,
    pub coop: bool, pub wouldblock: bool,   // coop: fill does not block, it sets wouldblock (other threads must run)
}
impl RData for RIo {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
impl Drop for RIo {
    fn drop(&mut self) { if self.init && !self.closed && self.fd > 2 { io_flush_one(self); if self.autoclose { close_fds(self); } } }
}
/// The IO payload of `v`, if it is an IO.
pub fn rio(c: &Ctx, v: Value) -> Option<&RIo> { c.data::<RIo>(v) }
pub fn is_io(c: &Ctx, v: Value) -> bool { c.data::<RIo>(v).is_some() }
fn rio_mut(c: &mut Ctx, v: Value) -> Option<&mut RIo> { c.data_mut::<RIo>(v) }
/// An IO already checked to be one.
fn io_mut(c: &mut Ctx, v: Value) -> &mut RIo { c.data_mut::<RIo>(v).expect("saruby: not an IO") }
fn lossy(b: &[u8]) -> String { String::from_utf8_lossy(b).into_owned() }

/// raise_errno whose detail ends with a path, kept as bytes (CRuby's message carries the path as given).
pub fn raise_errno_at(c: &mut Ctx, no: i64, pre: &str, path: &[u8]) -> Esc {
    let k = crate::builtins::exception::errno_class(c, no);
    let mut m = crate::builtins::exception::errno_text(Value::fix(no)).into_bytes();
    if !pre.is_empty() || !path.is_empty() { m.extend_from_slice(b" @ "); m.extend_from_slice(pre.as_bytes()); m.extend_from_slice(path); }
    let e = make_exception(c, k, "");
    let ms = c.str_new(&m);
    ivar_set_raw(c, e, s::mesg, ms);
    ivar_set_raw(c, e, s::errno, Value::fix(no));
    raise_exc(c, e)
}
/// A path argument: the String's bytes (any but NUL, as CRuby takes them).
pub struct PathArg(pub Vec<u8>);
impl PathArg {
    pub fn as_bytes(&self) -> &[u8] { &self.0 }
    pub fn path(&self) -> &std::path::Path { std::path::Path::new(std::ffi::OsStr::from_bytes(&self.0)) }
    pub fn cstr(&self) -> std::ffi::CString { sys::cstr(&self.0) }
}
impl AsRef<std::path::Path> for PathArg { fn as_ref(&self) -> &std::path::Path { self.path() } }
impl std::fmt::Display for PathArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&String::from_utf8_lossy(&self.0)) }
}

fn blank() -> RIo {
    RIo { fd: -1, wfd: -1, path: vec![], rbuf: vec![], rpos: 0, eof: false, wbuf: vec![], closed: false, init: false, lineno: 0, writable: false,
          readable: false, binmode: false, sync: false, pid: 0, enc: None, enc2: None, ecflags: 0, autoclose: true, has_path: false, data: false, rerr: 0, coop: false, wouldblock: false }
}
fn opened(fd: i32, path: &[u8], r: bool, w: bool) -> RIo {
    let mut io = blank();
    io.fd = fd; io.path = path.to_vec(); io.readable = r; io.writable = w; io.init = true;
    io
}
pub fn new_io(c: &mut Ctx, klass: Value, fd: i32, path: &[u8], r: bool, w: bool) -> Value {
    let io = opened(fd, path, r, w);
    c.alloc(klass, Body::Data(Box::new(io)))
}
/// IO#dup / #clone: a blank IO of the class; initialize_copy dups the descriptor.
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if !is_io(c, v) { return Ok(None); }
    let k = real_class_of(c, v);
    Ok(Some(c.alloc(k, Body::Data(Box::new(blank())))))
}
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if crate::builtins::inherits(c, klass, c.vm().c_io.get()) { return Ok(Some(c.alloc(klass, Body::Data(Box::new(blank()))))); }
    Ok(None)
}

/// Threads waiting on these fds get IOError (before the numbers can be reused).
fn cancel_waits(c: &mut Ctx, fd: i32, wfd: i32) { crate::builtins::blop::cancel_fd(c, fd); if wfd != fd { crate::builtins::blop::cancel_fd(c, wfd); } }
fn close_fds(io: &mut RIo) {
    if io.fd >= 0 { sys::close(io.fd); } if io.wfd >= 0 && io.wfd != io.fd { sys::close(io.wfd); }
    io.fd = -1; io.wfd = -1;
}
fn out_fd(io: &RIo) -> i32 { if io.wfd >= 0 { io.wfd } else { io.fd } }

pub fn flush_one(io: &mut RIo) -> Option<i64> { io_flush_one(io) }
fn io_flush_one(io: &mut RIo) -> Option<i64> {
    if io.wbuf.is_empty() { return None; }
    let b = std::mem::take(&mut io.wbuf);
    sys::write_fd(out_fd(io), &b).err()
}

pub fn flush_all(c: &mut Ctx) {
    out_flush();
    crate::gc::each_object(|v| { if let Some(io) = c.data_mut::<RIo>(v) { if io.init && !io.closed { io_flush_one(io); } } });
}

/// IOError / TypeError unless `v` is an open IO (and writable, if asked).
fn check_open(c: &mut Ctx, v: Value, want_w: bool) -> Result<(), Esc> {
    let st = match c.data::<RIo>(v) {
        None => 0, Some(i) if !i.init => 1, Some(i) if i.closed => 2, Some(i) if want_w && !i.writable => 3, _ => return Ok(()),
    };
    match st {
        0 => type_err(c, format!("wrong argument type {} (expected IO)", class_name_for_err(c, v))),
        1 => err(c, c.vm().c_io_error.get(), "uninitialized stream"),
        2 => err(c, c.vm().c_io_error.get(), "closed stream"),
        _ => err(c, c.vm().c_io_error.get(), "not opened for writing"),
    }
}
/// The open stream behind `v` (IOError when closed / uninitialized).
pub fn get_io(c: &mut Ctx, v: Value) -> Result<&mut RIo, Esc> { check_open(c, v, false)?; Ok(io_mut(c, v)) }
fn readable(c: &mut Ctx, v: Value) -> Result<&mut RIo, Esc> { readable_check(c, v)?; Ok(io_mut(c, v)) }
/// An open readable stream, without waiting for data.
fn readable_nowait(c: &mut Ctx, v: Value) -> Result<&mut RIo, Esc> {
    check_open(c, v, false)?;
    let io = io_mut(c, v);
    if !io.readable { return err(c, c.vm().c_io_error.get(), "not opened for reading"); }
    if !io.wbuf.is_empty() { io_flush_one(io); }
    if io.rerr != 0 { let e = std::mem::take(&mut io.rerr); let p = io.path.clone(); return Err(raise_errno_at(c, e, "io_fread - ", &p)); }
    Ok(io_mut(c, v))
}
fn readable_check(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    let io = readable_nowait(c, v)?;
    let (empty, fd) = (io.rpos >= io.rbuf.len() && !io.data, io.fd);
    if empty && (crate::builtins::thread::active(c) || io_timeout(c, v).is_some()) { coop_fill(c, v, fd)?; }
    Ok(())
}
// ---- IO#timeout: kept as a hidden ivar; looked up only once some IO has one -------------------
static TIMEOUTS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn timeout_sym() -> Sym { intern(b"__io_timeout") }
/// The IO#timeout of `v`, if set.
pub fn io_timeout(c: &Ctx, v: Value) -> Option<std::time::Duration> {
    if !TIMEOUTS.load(std::sync::atomic::Ordering::Relaxed) { return None; }
    let t = ivar_get(c, v, intern(b"__io_timeout_ns"));
    if t.is_fixnum() { Some(std::time::Duration::from_nanos(t.as_fix().max(0) as u64)) } else { None }
}
/// IO::TimeoutError (made on first use: saruby's boot does not define it).
pub fn timeout_error_class(c: &mut Ctx) -> Value {
    let io = c.vm().c_io.get();
    match const_get_at(c, io, intern(b"TimeoutError")) { Some(k) => k, None => define_class_under(c, io, "TimeoutError", c.vm().c_io_error.get()) }
}
fn io_get_timeout(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { get_io(c, slf)?; Ok(if TIMEOUTS.load(std::sync::atomic::Ordering::Relaxed) { ivar_get(c, slf, timeout_sym()) } else { NIL }) }
fn io_set_timeout(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let t = a[0];
    if t != NIL {
        if !is_kind_of(c, t, c.vm().c_numeric.get()) { return type_err(c, format!("can't convert {} into time interval", class_name_for_err(c, t))); }
        let f = crate::builtins::numeric_methods::to_f(c, t)?;
        if f < 0.0 { return arg_err(c, "time interval must not be negative"); }
        TIMEOUTS.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    get_io(c, slf)?;
    if t != NIL || TIMEOUTS.load(std::sync::atomic::Ordering::Relaxed) {
        let ns = if t == NIL { NIL } else { let f = crate::builtins::numeric_methods::to_f(c, t)?; Value::fix((f.min(1e9) * 1e9) as i64) };
        ivar_set_raw(c, slf, timeout_sym(), t);
        ivar_set_raw(c, slf, intern(b"__io_timeout_ns"), ns);
    }
    Ok(t)
}
thread_local! { static SPARE: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) }; }
/// With other threads running, an empty read buffer is filled by a READ blop: this thread parks,
/// the engine reads when the fd is ready, and the bytes are appended to rbuf.
fn coop_fill(c: &mut Ctx, v: Value, fd: i32) -> Result<(), Esc> {
    if fd_ready(fd, sys::POLLIN) { return Ok(()); }   // fill reads it now
    if let Some(t) = io_timeout(c, v) {
        if wait_one(c, fd, sys::POLLIN, Some(std::time::Instant::now() + t))?.is_none() {
            let k = timeout_error_class(c);
            return err(c, k, "Blocking operation timed out!");
        }
        return check_open(c, v, false);
    }
    if fd == 0 { out_flush(); }
    let mut buf = SPARE.with(|s| std::mem::take(&mut *s.borrow_mut()));
    if buf.len() < 65536 { buf.resize(65536, 0); }
    // a non-blocking fd (a socket) is read first, as CRuby does: a listening socket never polls readable but fails at once
    let first = if sys::fcntl(fd, sys::F_GETFL, 0) & sys::O_NONBLOCK != 0 {
        let n = sys::read(fd, &mut buf);
        if n >= 0 { Some(Ok(n as usize)) } else { match sys::errno() { sys::EAGAIN | sys::EINTR => None, e => Some(Err(e)) } }
    } else { None };
    let (buf, r) = match first { Some(r) => (buf, r), None => crate::builtins::thread::read_op(c, fd, buf)? };
    let r = check_open(c, v, false).map(|_| {
        let io = io_mut(c, v);
        match r {
            Ok(n) => { if io.rpos >= io.rbuf.len() { io.rbuf.clear(); io.rpos = 0; } io.rbuf.extend_from_slice(&buf[..n]); }
            Err(e) => io.rerr = e,
        }
    });
    SPARE.with(|s| *s.borrow_mut() = buf);
    r
}
fn discard_rbuf(io: &mut RIo) {
    // give unread bytes back to the kernel offset before writing / seeking
    let unread = (io.rbuf.len() - io.rpos) as i64;
    if unread > 0 && !io.data { sys::lseek(io.fd, -unread, 1); }
    if !io.data { io.rbuf.clear(); io.rpos = 0; }
}

/// Append to the stream (stdout / stderr go to the process buffers); Some(errno) on a failed flush.
// buffer_write's answer when the flush must let other threads run (coop_flush)
const FLUSH_COOP: i64 = -1;
fn buffer_write(io: &mut RIo, b: &[u8], coop: bool) -> Option<i64> {
    let fd = out_fd(io);
    if fd == 1 { out_write(b); return None; }
    if fd == 2 { err_write(b); return None; }
    if !io.rbuf.is_empty() { discard_rbuf(io); }
    io.wbuf.extend_from_slice(b);
    if io.wbuf.len() > 1 << 16 || io.sync { return if coop { Some(FLUSH_COOP) } else { io_flush_one(io) }; }
    None
}
/// Flush with other threads running while the fd is not writable (a full pipe would block them all).
#[cold]
fn coop_flush(c: &mut Ctx, io_v: Value) -> Result<(), Esc> {
    loop {
        let fd = { let io = io_mut(c, io_v); if io.wbuf.is_empty() { return Ok(()); } out_fd(io) };
        // POLLOUT on a pipe promises PIPE_BUF bytes, so a write of that much does not block
        let w = if fd_ready(fd, sys::POLLOUT) {
            let io = io_mut(c, io_v);
            let n = io.wbuf.len().min(4096);
            let w = sys::write(fd, &io.wbuf[..n]);
            if w < 0 { Err(sys::errno()) } else { Ok(w as usize) }
        } else {
            // a WRITE blop: the engine writes once the fd is writable
            let chunk = { let io = io_mut(c, io_v); io.wbuf[..io.wbuf.len().min(4096)].to_vec() };
            crate::builtins::thread::write_op(c, fd, chunk)?
        };
        check_open(c, io_v, true)?;
        let io = io_mut(c, io_v);
        match w {
            Ok(w) => { io.wbuf.drain(..w.min(io.wbuf.len())); }
            Err(e) if e == sys::EINTR || e == sys::EAGAIN => {}
            Err(e) => { io.wbuf.clear(); return Err(raise_errno(c, e, "")); }
        }
    }
}
fn writable(io: &RIo) -> bool { io.init && !io.closed && io.writable }
/// The error a write to `io_v` raises (not an open writable IO, or errno `e` of the flush).
#[cold]
fn write_err(c: &mut Ctx, io_v: Value, e: Option<i64>) -> Esc {
    if let Some(e) = e { return raise_errno(c, e, ""); }
    match check_open(c, io_v, true) { Err(x) => x, Ok(()) => raise_errno(c, 9, "") }
}
pub fn io_write_bytes(c: &mut Ctx, io_v: Value, b: &[u8]) -> Result<(), Esc> {
    // one payload lookup on the common path
    let coop = crate::builtins::thread::active(c);
    let e = match c.data_mut::<RIo>(io_v) { Some(io) if writable(io) => match buffer_write(io, b, coop) { None => return out_check(c), e => e }, _ => None };
    if e == Some(FLUSH_COOP) { return coop_flush(c, io_v); }
    Err(write_err(c, io_v, e))
}
/// The encoding a written String is converted to (make_writeconv), if any.
fn wconv(io: &RIo) -> Option<u8> {
    if io.ecflags & EC_UNSUPPORTED != 0 { return None; }
    match (io.enc, io.enc2) { (None, _) | (Some(ENC_BINARY), None) => None, (Some(e), None) | (_, Some(e)) => Some(e) }
}
// IO conversion options kept as flags: invalid: / undef: :replace; others (xml:, newline:, replace:,
// fallback:) are not done by saruby's IO, which then does not convert (its old behaviour)
const EC_INVALID: u8 = 1;
const EC_UNDEF: u8 = 2;
const EC_UNSUPPORTED: u8 = 4;
fn ecflags_of(o: &crate::builtins::encoding::TcOpts) -> u8 {
    (if o.invalid_replace { EC_INVALID } else { 0 }) | (if o.undef_replace { EC_UNDEF } else { 0 })
        | (if o.xml != 0 || o.newline != 0 || o.replace.is_some() || o.fallback.is_some() { EC_UNSUPPORTED } else { 0 })
}
/// Convert `b` for an IO, or None where saruby has no converter for the pair (left as it is).
fn io_transcode(c: &mut Ctx, b: &[u8], from: u8, to: u8, ec: u8) -> Result<Option<Vec<u8>>, Esc> {
    use crate::builtins::encoding as en;
    // a BINARY source goes through UTF-8 (its non-ASCII bytes are undefined there)
    if ec & EC_UNSUPPORTED != 0 || (from != ENC_BINARY && !en::can_convert(from, to)) { return Ok(None); }
    let o = en::TcOpts { invalid_replace: ec & EC_INVALID != 0, undef_replace: ec & EC_UNDEF != 0, ..Default::default() };
    Ok(Some(en::transcode(c, b, from, to, &o)?))
}
/// `sv` converted for writing to `io_v` (do_writeconv); None: written as it is.
fn write_conv(c: &mut Ctx, io_v: Value, sv: Value) -> Result<Option<Vec<u8>>, Esc> {
    use crate::builtins::encoding as en;
    let Some((to, ec)) = rio(c, io_v).and_then(|io| Some((wconv(io)?, io.ecflags))) else { return Ok(None) };
    let from = c.str_enc(sv);
    if from == to || en::ascii_compat(from) && en::ascii_compat(to) && is_ascii(c.str_bytes(sv)) { return Ok(None); }
    let b = c.str_bytes(sv).to_vec();
    io_transcode(c, &b, from, to, ec)
}
/// Write String `sv` without copying it out first; its byte count.
fn io_write_str(c: &mut Ctx, io_v: Value, sv: Value) -> Result<usize, Esc> {
    if let Some(b) = write_conv(c, io_v, sv)? { io_write_bytes(c, io_v, &b)?; return Ok(b.len()); }
    let coop = crate::builtins::thread::active(c);
    let e = match c.data_mut_and::<RIo>(io_v) {
        Some((io, o)) if writable(io) => {
            let b = o.str_bytes(sv).unwrap_or(&[]);   // sv is a String, never the IO
            let n = b.len();
            match buffer_write(io, b, coop) { None => { out_check(c)?; return Ok(n) } Some(FLUSH_COOP) => Err(n), e => Ok(e) }
        }
        _ => Ok(None),
    };
    let e = match e { Ok(e) => e, Err(n) => { coop_flush(c, io_v)?; return Ok(n); } };
    Err(write_err(c, io_v, e))
}

fn fill(io: &mut RIo) -> bool {
    if io.rpos < io.rbuf.len() { return true; }
    if io.eof || io.data { return false; }
    if io.coop && !fd_ready(io.fd, sys::POLLIN) { io.wouldblock = true; return false; }
    io.rbuf.clear(); io.rpos = 0;
    if io.fd == 0 { out_flush(); }
    io.rbuf.resize(65536, 0);
    let n = match sys::read_fd(io.fd, &mut io.rbuf) { Ok(n) => n, Err(e) => { io.rerr = e; 0 } };
    io.rbuf.truncate(n);
    // pipes / ttys reach EOF only on a zero read; regular files may grow, so never latch eof
    n > 0
}
fn fd_ready(fd: i32, ev: i16) -> bool { let mut p = [sys::PollFd { fd, events: ev, revents: 0 }]; sys::poll(&mut p, 0) > 0 }
/// Give consumed bytes back to the read buffer (a read that would block is redone after the wait).
fn unread(io: &mut RIo, mut b: Vec<u8>) {
    if b.is_empty() { return; }
    b.extend_from_slice(&io.rbuf[io.rpos..]);
    io.rbuf = b; io.rpos = 0;
}
/// Run read op `f` on `v`; while other threads exist, a fill that would block undoes `f`
/// (it hands back what it consumed), waits for the fd with the others running, and runs `f` again.
fn coop_read<T>(c: &mut Ctx, v: Value, mut f: impl FnMut(&mut RIo) -> (T, Vec<u8>)) -> Result<T, Esc> {
    loop {
        let coop = crate::builtins::thread::active(c) || io_timeout(c, v).is_some();
        let io = readable(c, v)?;
        io.coop = coop; io.wouldblock = false;
        let (r, used) = f(io);
        io.coop = false;
        if !io.wouldblock { return Ok(r); }
        io.wouldblock = false;
        let fd = io.fd;
        // what `f` consumed is given back after the wait; a timeout drops it, as CRuby's read does
        coop_fill(c, v, fd)?;
        unread(io_mut(c, v), used);
    }
}
fn find_sub(h: &[u8], n: &[u8]) -> Option<usize> { if n.len() == 1 { h.iter().position(|&b| b == n[0]) } else { h.windows(n.len()).position(|w| w == n) } }
fn read_line(io: &mut RIo, sep: Option<&[u8]>, limit: Option<usize>) -> Option<Vec<u8>> {
    let mut line = vec![];
    if limit == Some(0) { return Some(line); }
    let para = sep == Some(b"");
    let sp: Option<&[u8]> = if para { Some(b"\n\n") } else { sep };
    if para { while fill(io) && io.rbuf[io.rpos] == b'\n' { io.rpos += 1; } }
    loop {
        if !fill(io) { break; }
        let avail = &io.rbuf[io.rpos..];
        let mut take = avail.len();
        let mut done = false;
        if let Some(sp) = sp {
            // the separator may straddle the previous chunk
            let back = (sp.len() - 1).min(line.len());
            let start = line.len() - back;
            let mut probe = line[start..].to_vec(); probe.extend_from_slice(avail);
            if let Some(p) = find_sub(&probe, sp) { take = p + sp.len() - back; done = true; }
        }
        if let Some(l) = limit { if line.len() + take >= l { take = l - line.len(); done = true; } }
        line.extend_from_slice(&avail[..take]);
        io.rpos += take;
        if done { break; }
    }
    if let Some(l) = limit {
        // do not split a multibyte character at the limit
        if line.len() == l && enc_of(io) == ENC_UTF8 {
            let mut k = 0;
            while k < 4 && std::str::from_utf8(&line).is_err_and(|e| e.error_len().is_none()) && fill(io) { line.push(io.rbuf[io.rpos]); io.rpos += 1; k += 1; }
        }
    }
    if para { while fill(io) && io.rbuf[io.rpos] == b'\n' { io.rpos += 1; } }
    if line.is_empty() { None } else { Some(line) }
}
fn read_all(io: &mut RIo) -> Vec<u8> {
    let mut v = vec![];
    while fill(io) { v.extend_from_slice(&io.rbuf[io.rpos..]); io.rpos = io.rbuf.len(); }
    v
}
/// io_read_encoding.
fn enc_of(io: &RIo) -> u8 { io.enc.unwrap_or_else(crate::builtins::encoding::default_external) }

fn io_write(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.len() > 1 && !rio(c, slf).is_some_and(|io| wconv(io).is_some()) {
        let mut out = vec![];
        for &x in a { let sv = crate::rt::srb_tostr(c, x)?; out.extend_from_slice(c.str_bytes(sv)); }
        io_write_bytes(c, slf, &out)?;
        return Ok(Value::fix(out.len() as i64));
    }
    let mut n = 0;
    for &x in a { let sv = crate::rt::srb_tostr(c, x)?; n += io_write_str(c, slf, sv)?; }
    Ok(Value::fix(n as i64))
}
fn io_puts(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if rio(c, slf).is_some_and(|io| wconv(io).is_some()) {
        // each line is written (and converted) on its own
        for &x in a {
            if is_string(x) { io_write_str(c, slf, x)?; if !c.str_bytes(x).ends_with(b"\n") { io_write_bytes(c, slf, b"\n")?; } continue; }
            let out = puts_bytes(c, &[x])?; let sv = c.str_from_vec(out); io_write_str(c, slf, sv)?;
        }
        if a.is_empty() { io_write_bytes(c, slf, b"\n")?; }
        return Ok(NIL);
    }
    let out = puts_bytes(c, a)?; io_write_bytes(c, slf, &out)?; Ok(NIL)
}
fn io_print(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mut out = vec![];
    if a.is_empty() { let l = global_get("$_"); let sv = crate::rt::srb_tostr(c, l)?; out.extend_from_slice(c.str_bytes(sv)); }
    for &x in a {
        let sv = crate::rt::srb_tostr(c, x)?;
        if let Some(b) = write_conv(c, slf, sv)? { out.extend_from_slice(&b); } else { out.extend_from_slice(c.str_bytes(sv)); }
    }
    io_write_bytes(c, slf, &out)?; Ok(NIL)
}
fn io_ltlt(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { io_write(c, slf, a, b)?; Ok(slf) }
fn io_printf(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return Ok(NIL); }
    let s2 = crate::builtins::sprintf::format(c, a[0], &a[1..])?;
    io_write_str(c, slf, s2)?; Ok(NIL)
}
fn io_p(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mut out = vec![];
    for &x in a { let s2 = inspect_value(c, x)?; out.extend_from_slice(c.str_bytes(s2)); out.push(b'\n'); }
    io_write_bytes(c, slf, &out)?; Ok(NIL)
}
fn io_putc(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let b = if is_string(a[0]) {
        let sb = c.str_bytes(a[0]); let l = char_len_at(sb, 0).min(sb.len());
        let ch = c.str_new_like(a[0], sb[..l].to_vec());
        match write_conv(c, slf, ch)? { Some(b) => b, None => c.str_bytes(ch).to_vec() }
    } else { vec![num_to_long(c, a[0])? as u8] };
    io_write_bytes(c, slf, &b)?; Ok(a[0])
}
fn io_flush(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let io = get_io(c, slf)?;
    if out_fd(io) == 1 { out_flush(); } else if let Some(e) = io_flush_one(io) { return Err(raise_errno(c, e, "")); }
    Ok(slf)
}
fn io_fsync(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { io_flush(c, slf, a, b)?; let io = get_io(c, slf)?; sys::fsync(out_fd(io)); Ok(Value::fix(0)) }
fn io_sync(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = get_io(c, slf)?; Ok(Value::bool(io.sync || out_fd(io) == 2)) }
fn io_set_sync(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let io = get_io(c, slf)?;
    io.sync = a[0].truthy();
    if out_fd(io) == 1 { STDOUT_SYNC.store(io.sync, std::sync::atomic::Ordering::Relaxed); if io.sync { out_flush(); } } else if io.sync { io_flush_one(io); }
    Ok(a[0])
}

/// gets-style arguments: (sep = $/, limit = nil, chomp: false)
fn line_args(c: &mut Ctx, a: &[Value]) -> Result<(Option<Vec<u8>>, Option<usize>, bool), Esc> {
    let mut sep: Option<Vec<u8>> = Some(b"\n".to_vec()); let mut chomp = false; let mut limit = None;
    let mut pos = a;
    if let Some(&l) = a.last() { if is_hash(l) { if let Some(v) = crate::hash::hash_get_opt(c, l, sym_value("chomp"))? { chomp = v.truthy(); } pos = &a[..a.len() - 1]; } }
    if pos.len() > 2 { return argc_err(c, pos.len(), 0, 2); }
    let set_limit = |c: &mut Ctx, x: Value| -> Result<Option<usize>, Esc> { let n = num_to_long(c, x)?; Ok(if n < 0 { None } else { Some(n as usize) }) };
    match pos.len() {
        1 => { let x = pos[0]; if x == NIL { sep = None; } else if is_string(x) { sep = Some(c.str_bytes(x).to_vec()); } else if x.is_fixnum() { limit = set_limit(c, x)?; } else { let sv = expect_str(c, x)?; sep = Some(c.str_bytes(sv).to_vec()); } }
        2 => { let x = pos[0]; if x == NIL { sep = None; } else { let sv = expect_str(c, x)?; sep = Some(c.str_bytes(sv).to_vec()); } if pos[1] != NIL { limit = set_limit(c, pos[1])?; } }
        _ => { let rs = global_get("$/"); if rs == NIL { sep = None; } else if is_string(rs) { sep = Some(c.str_bytes(rs).to_vec()); } }
    }
    Ok((sep, limit, chomp))
}
/// Next line as bytes (lineno counted, chomped as asked).
fn gets_raw(io: &mut RIo, la: &(Option<Vec<u8>>, Option<usize>, bool)) -> Option<Vec<u8>> {
    let (sep, limit, chomp) = la;
    let mut l = read_line(io, sep.as_deref(), *limit)?;
    if io.wouldblock { unread(io, l); return None; }
    io.lineno += 1;
    if *chomp { if let Some(sp) = sep {
        if sp.is_empty() { while l.ends_with(b"\n") { l.pop(); } }
        else if l.ends_with(sp) { l.truncate(l.len() - sp.len()); if sp == b"\n" && l.ends_with(b"\r") { l.pop(); } }
    } }
    Some(l)
}
/// One line read from `slf` as a String (nil at EOF), and the stream's lineno after it.
fn gets_value(c: &mut Ctx, slf: Value, la: &(Option<Vec<u8>>, Option<usize>, bool)) -> Result<(Value, i64), Esc> {
    let l = coop_read(c, slf, |io| (gets_raw(io, la), vec![]))?;
    let io = io_mut(c, slf);
    let (enc, ec, lineno) = ((io.enc, io.enc2), io.ecflags, io.lineno);
    if io.rerr != 0 { let e = std::mem::take(&mut io.rerr); let (m, p) = (format!("io_fillbuf - fd:{} ", io.fd), io.path.clone()); return Err(raise_errno_at(c, e, &m, &p)); }
    Ok((match l { Some(l) => read_str(c, enc, ec, l)?, None => NIL }, lineno))
}
fn io_gets(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let la = line_args(c, a)?;
    let (v, lineno) = gets_value(c, slf, &la)?;
    if v != NIL { global_set("$.", Value::fix(lineno)); }
    c.vm().globals.borrow_mut().insert(intern(b"$_"), v);
    Ok(v)
}
fn eof_err<T>(c: &mut Ctx) -> Result<T, Esc> { err(c, c.vm().c_eof_error.get(), "end of file reached") }
fn io_readline(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let v = io_gets(c, slf, a, b)?;
    if v == NIL { return eof_err(c); }
    Ok(v)
}
fn io_each_line(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_line"), a, NIL); }
    let la = line_args(c, a)?;
    if la.1 == Some(0) { return arg_err(c, "invalid limit: 0 for each_line"); }
    loop {
        let (l, lineno) = gets_value(c, slf, &la)?;
        if l == NIL { break; }
        global_set("$.", Value::fix(lineno));
        yield1(c, blk, l)?;
    }
    Ok(slf)
}
fn io_readlines(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let la = line_args(c, a)?;
    if la.1 == Some(0) { return arg_err(c, "invalid limit: 0 for readlines"); }
    let out = c.ary_new();
    loop { let (l, _) = gets_value(c, slf, &la)?; if l == NIL { break; } c.ary_push(out, l); }
    Ok(out)
}
/// Store `v` into the caller's buffer string (read(n, buf) forms).
/// An outbuf is emptied when a read returns nil at EOF.
fn clear_outbuf(c: &mut Ctx, buf: Option<Value>) -> Result<(), Esc> {
    if let Some(b) = buf { if b != NIL { let b = expect_str(c, b)?; check_frozen(c, b)?; c.rstring_mut(b).b.clear(); } }
    Ok(())
}
pub fn into_outbuf(c: &mut Ctx, buf: Option<Value>, v: Vec<u8>, enc: u8) -> RV {
    match buf {
        Some(b) if b != NIL => { let b = expect_str(c, b)?; check_frozen(c, b)?; c.rstring_mut(b).b = v.into(); Ok(b) }
        _ => Ok(c.str_from_vec_enc(v, enc)),
    }
}
fn io_read(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 2)?;
    // the length is converted before the stream is looked at (CRuby's order; to_int may run Ruby code)
    let n = match a.first() { Some(&n) if n != NIL => { let n = num_to_long(c, n)?; if n < 0 { return arg_err(c, format!("negative length {} given", n)); } Some(n as usize) } _ => None };
    if n == Some(0) { readable_nowait(c, slf)?; return into_outbuf(c, a.get(1).copied(), vec![], ENC_BINARY); }
    if let Some(n) = n {
        let v = coop_read(c, slf, |io| {
            let mut v = vec![];
            while v.len() < n && fill(io) { let take = (n - v.len()).min(io.rbuf.len() - io.rpos); v.extend_from_slice(&io.rbuf[io.rpos..io.rpos + take]); io.rpos += take; }
            if io.wouldblock { (vec![], v) } else { (v, vec![]) }
        })?;
        if v.is_empty() && n > 0 {
            let io = io_mut(c, slf);
            if io.rerr != 0 { let e = std::mem::take(&mut io.rerr); let p = io.path.clone(); return Err(raise_errno_at(c, e, "io_fread - ", &p)); }
            if let Some(&b) = a.get(1) { if b != NIL { let b = expect_str(c, b)?; check_frozen(c, b)?; c.rstring_mut(b).b.clear(); } }
            return Ok(NIL);
        }
        return into_outbuf(c, a.get(1).copied(), v, ENC_BINARY);
    }
    let v = coop_read(c, slf, |io| { let v = read_all(io); if io.wouldblock { (vec![], v) } else { (v, vec![]) } })?;
    let io = io_mut(c, slf);
    if io.rerr != 0 { let e = std::mem::take(&mut io.rerr); let p = io.path.clone(); return Err(raise_errno_at(c, e, "io_fread - ", &p)); }
    let (enc, ec) = ((io.enc, io.enc2), io.ecflags);
    let sv = read_str(c, enc, ec, v)?;
    match a.get(1) {
        Some(&b) if b != NIL => { let b = expect_str(c, b)?; check_frozen(c, b)?; let (nb, ne) = (c.str_bytes(sv).to_vec(), c.str_enc(sv)); let r = c.rstring_mut(b); r.b = nb.into(); r.enc = ne; Ok(b) }
        _ => Ok(sv),
    }
}
fn io_readpartial(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, format!("negative length {} given", n)); }
    let io = readable(c, slf)?;
    if n == 0 { return into_outbuf(c, a.get(1).copied(), vec![], ENC_BINARY); }
    if let Some(r) = read_unbuffered(io, n as usize) {
        return match r { Ok(v) if v.is_empty() => eof_err(c), Ok(v) => into_outbuf(c, a.get(1).copied(), v, ENC_BINARY), Err(e) => Err(raise_errno(c, e, "")) };
    }
    if !fill(io) { return eof_err(c); }
    let take = (n as usize).min(io.rbuf.len() - io.rpos);
    let v = io.rbuf[io.rpos..io.rpos + take].to_vec(); io.rpos += take;
    into_outbuf(c, a.get(1).copied(), v, ENC_BINARY)
}
/// With nothing buffered, read up to n bytes straight from the fd (CRuby's io_getpartial; no 64 KiB fill).
fn read_unbuffered(io: &mut RIo, n: usize) -> Option<Result<Vec<u8>, i64>> {
    if io.rpos < io.rbuf.len() || io.data || io.eof { return None; }
    if io.fd == 0 { out_flush(); }
    let mut v = vec![0u8; n];
    Some(sys::read_fd(io.fd, &mut v).map(|k| { v.truncate(k); v }))
}
fn wait_fd(fd: i32, ev: i16, ms: i32) -> bool { let mut p = [sys::PollFd { fd, events: ev, revents: 0 }]; sys::poll(&mut p, ms) > 0 }
fn exception_opt(c: &mut Ctx, a: &[Value]) -> Result<bool, Esc> {
    if let Some(&l) = a.last() { if is_hash(l) { if let Some(v) = crate::hash::hash_get_opt(c, l, sym_value("exception"))? { return Ok(v.truthy()); } } }
    Ok(true)
}
fn wait_error(c: &mut Ctx, name: &str, msg: &str) -> Esc {
    let k = const_get_at(c, c.vm().c_io.get(), intern(name.as_bytes())).unwrap();
    let e = make_exception(c, k, &format!("Resource temporarily unavailable - {}", msg));
    ivar_set_raw(c, e, s::errno, Value::fix(11));
    raise_exc(c, e)
}
fn io_read_nonblock(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let exc = exception_opt(c, a)?;
    let pos: Vec<Value> = a.iter().copied().filter(|x| !is_hash(*x)).collect();
    let n = match pos.first() { Some(&n) => num_to_long(c, n)?, None => return argc_err(c, 0, 1, 2) };
    if n < 0 { return arg_err(c, format!("negative length {} given", n)); }
    // no wait for other threads here: this read must not block
    check_open(c, slf, false)?;
    if !io_mut(c, slf).readable { return err(c, c.vm().c_io_error.get(), "not opened for reading"); }
    if n == 0 { return into_outbuf(c, pos.get(1).copied(), vec![], ENC_BINARY); }
    let io = io_mut(c, slf);
    if !io.wbuf.is_empty() { io_flush_one(io); }
    if io.rpos >= io.rbuf.len() && !io.data && !wait_fd(io.fd, sys::POLLIN, 0) {
        if !exc { return Ok(sym_value("wait_readable")); }
        return Err(wait_error(c, "EAGAINWaitReadable", "read would block"));
    }
    {
        let io = io_mut(c, slf);
        if let Some(r) = read_unbuffered(io, n as usize) {
            return match r {
                Ok(v) if v.is_empty() => if !exc { clear_outbuf(c, pos.get(1).copied())?; Ok(NIL) } else { eof_err(c) },
                Ok(v) => into_outbuf(c, pos.get(1).copied(), v, ENC_BINARY),
                Err(e) if e == sys::EAGAIN => if !exc { Ok(sym_value("wait_readable")) } else { Err(wait_error(c, "EAGAINWaitReadable", "read would block")) },
                Err(e) => Err(raise_errno(c, e, "")),
            };
        }
    }
    if !fill(io_mut(c, slf)) { if !exc { clear_outbuf(c, pos.get(1).copied())?; return Ok(NIL); } return eof_err(c); }
    io_readpartial(c, slf, &pos, b)
}
fn io_write_nonblock(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let exc = exception_opt(c, a)?;
    let sv = crate::rt::srb_tostr(c, a[0])?;
    check_open(c, slf, true)?;
    let io = io_mut(c, slf);
    io_flush_one(io);
    let fd = out_fd(io);
    if fd == 1 { out_flush(); }
    // the fd stays in non-blocking mode, as rb_io_set_nonblock leaves it
    let fl = sys::fcntl(fd, sys::F_GETFL, 0);
    if fl >= 0 && fl & sys::O_NONBLOCK == 0 { sys::fcntl(fd, sys::F_SETFL, fl | sys::O_NONBLOCK); }
    let n = sys::write(fd, c.str_bytes(sv));
    if n < 0 {
        let e = sys::errno();
        if e == sys::EAGAIN {
            if !exc { return Ok(sym_value("wait_writable")); }
            return Err(wait_error(c, "EAGAINWaitWritable", "write would block"));
        }
        return Err(raise_errno(c, e, ""));
    }
    Ok(Value::fix(n as i64))
}
fn io_sysread(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, format!("negative length {} given", n)); }
    let io = readable_nowait(c, slf)?;
    if io.rpos < io.rbuf.len() { return err(c, c.vm().c_io_error.get(), "sysread for buffered IO"); }
    let fd = io.fd;
    if n == 0 { return into_outbuf(c, a.get(1).copied(), vec![], ENC_BINARY); }
    if crate::builtins::thread::active(c) { crate::builtins::thread::wait_readable(c, fd)?; check_open(c, slf, false)?; }
    let mut v = vec![0u8; n as usize];
    match sys::read_fd(fd, &mut v) {
        Ok(0) => eof_err(c),
        Ok(k) => { v.truncate(k); into_outbuf(c, a.get(1).copied(), v, ENC_BINARY) }
        Err(e) => Err(raise_errno(c, e, "")),
    }
}
fn io_syswrite(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let sv = crate::rt::srb_tostr(c, a[0])?;
    check_open(c, slf, true)?;
    let io = io_mut(c, slf);
    let fd = out_fd(io);
    if fd == 1 { out_flush(); }
    io_flush_one(io);
    // one write(2) on a non-blocking fd: the count may be short (rb_io_write_memory)
    if sys::fcntl(fd, sys::F_GETFL, 0) & sys::O_NONBLOCK != 0 {
        loop {
            let n = sys::write(fd, c.str_bytes(sv));
            if n >= 0 { return Ok(Value::fix(n as i64)); }
            let e = sys::errno();
            if e != sys::EINTR { return Err(raise_errno(c, e, "")); }
        }
    }
    match sys::write_fd(fd, c.str_bytes(sv)) { Ok(()) => Ok(Value::fix(c.str_bytes(sv).len() as i64)), Err(e) => Err(raise_errno(c, e, "")) }
}
fn io_pread(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 2, 3)?;
    let n = num_to_long(c, a[0])?; let off = num_to_long(c, a[1])?;
    if n < 0 { return arg_err(c, "negative string size (or size too big)"); }
    let n = n as usize;
    let fd = get_io(c, slf)?.fd;
    let mut v = vec![0u8; n];
    let k = sys::pread(fd, &mut v, off);
    if k < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    if k == 0 && n > 0 { return eof_err(c); }
    v.truncate(k as usize);
    into_outbuf(c, a.get(2).copied(), v, ENC_BINARY)
}
fn io_pwrite(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let sv = crate::rt::srb_tostr(c, a[0])?; let off = num_to_long(c, a[1])?;
    let io = get_io(c, slf)?;
    io_flush_one(io);
    let fd = out_fd(io);
    let k = sys::pwrite(fd, c.str_bytes(sv), off);
    if k < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    Ok(Value::fix(k as i64))
}
fn next_char(io: &mut RIo) -> Option<Vec<u8>> {
    if !fill(io) { return None; }
    let enc = io.enc2.unwrap_or_else(|| enc_of(io));
    let want = if enc == ENC_UTF8 { match io.rbuf[io.rpos] { 0xc2..=0xdf => 2, 0xe0..=0xef => 3, 0xf0..=0xf4 => 4, _ => 1 } } else { crate::builtins::encoding::max_len(enc) };
    // pull more bytes when the character straddles the buffer end
    while io.rbuf.len() - io.rpos < want {
        let mut more = [0u8; 8];
        match sys::read_fd(io.fd, &mut more[..want - (io.rbuf.len() - io.rpos)]) { Ok(n) if n > 0 && !io.data => io.rbuf.extend_from_slice(&more[..n]), _ => break }
    }
    let l = enc_char_len(&io.rbuf, io.rpos, enc).clamp(1, io.rbuf.len() - io.rpos);
    let v = io.rbuf[io.rpos..io.rpos + l].to_vec(); io.rpos += l;
    Some(v)
}
fn io_getc(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let io = readable(c, slf)?;
    let ch = next_char(io);
    let (enc, ec) = ((io.enc, io.enc2), io.ecflags);
    Ok(match ch { Some(v) => read_str(c, enc, ec, v)?, None => NIL })
}
fn io_readchar(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { let v = io_getc(c, slf, a, b)?; if v == NIL { return eof_err(c); } Ok(v) }
fn io_getbyte(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let io = readable(c, slf)?;
    if !fill(io) { return Ok(NIL); }
    io.rpos += 1;
    Ok(Value::fix(io.rbuf[io.rpos - 1] as i64))
}
fn io_readbyte(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { let v = io_getbyte(c, slf, a, b)?; if v == NIL { return eof_err(c); } Ok(v) }
fn io_each_byte(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_byte"), a, NIL); }
    loop { let v = io_getbyte(c, slf, &[], std::ptr::null())?; if v == NIL { break; } yield1(c, blk, v)?; }
    Ok(slf)
}
fn io_each_char(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_char"), a, NIL); }
    loop { let v = io_getc(c, slf, &[], std::ptr::null())?; if v == NIL { break; } yield1(c, blk, v)?; }
    Ok(slf)
}
fn unget(io: &mut RIo, b: &[u8]) {
    if io.rpos >= b.len() && !io.data { io.rpos -= b.len(); let p = io.rpos; io.rbuf[p..p + b.len()].copy_from_slice(b); }
    else { let mut nb = b.to_vec(); nb.extend_from_slice(&io.rbuf[io.rpos..]); io.rbuf = nb; io.rpos = 0; }
}
fn io_ungetc(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0] == NIL { return Ok(NIL); }
    let b = if a[0].is_fixnum() { let mut v = vec![]; encode_utf8(a[0].as_fix() as u32, &mut v); v } else { let sv = expect_str(c, a[0])?; c.str_bytes(sv).to_vec() };
    let io = readable_nowait(c, slf)?;
    unget(io, &b);
    Ok(NIL)
}
fn io_ungetbyte(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0] == NIL { return Ok(NIL); }
    let b = if a[0].is_fixnum() { vec![a[0].as_fix() as u8] } else { let sv = expect_str(c, a[0])?; c.str_bytes(sv).to_vec() };
    let io = readable_nowait(c, slf)?;
    unget(io, &b);
    Ok(NIL)
}
fn io_eof(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = readable(c, slf)?; Ok(Value::bool(!fill(io))) }

pub fn close_io(c: &mut Ctx, slf: Value) -> RV {
    let Some(io) = rio_mut(c, slf) else { return Ok(NIL) };
    if !io.init || io.closed { return Ok(NIL); }
    if out_fd(io) == 1 { out_flush(); }
    let e = io_flush_one(io);
    io.closed = true;
    let (fd, wfd) = (io.fd, io.wfd);
    if fd > 2 || wfd > 2 { cancel_waits(c, fd, wfd); let io = io_mut(c, slf); close_fds(io); }
    let io = io_mut(c, slf);
    let pid = std::mem::take(&mut io.pid);
    if pid > 0 { crate::builtins::process::reap_child(c, pid)?; }
    if let Some(e) = e { return Err(raise_errno(c, e, "")); }
    Ok(NIL)
}
fn io_close(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { close_io(c, slf) }
fn io_close_write(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let Some(io) = rio_mut(c, slf) else { return Ok(NIL) };
    if io.closed || !io.writable { return Ok(NIL); }
    if io.wfd >= 0 && io.wfd != io.fd { io_flush_one(io); let w = io.wfd; cancel_waits(c, -1, w); let io = io_mut(c, slf); sys::close(io.wfd); io.wfd = -1; io.writable = false; return Ok(NIL); }
    if io.readable { return err(c, c.vm().c_io_error.get(), "closing non-duplex IO for writing"); }
    close_io(c, slf)
}
fn io_close_read(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let Some(io) = rio_mut(c, slf) else { return Ok(NIL) };
    if io.closed || !io.readable { return Ok(NIL); }
    if io.wfd >= 0 && io.wfd != io.fd { let r = io.fd; cancel_waits(c, r, -1); let io = io_mut(c, slf); sys::close(io.fd); io.fd = io.wfd; io.readable = false; return Ok(NIL); }
    if io.writable { return err(c, c.vm().c_io_error.get(), "closing non-duplex IO for reading"); }
    close_io(c, slf)
}
fn io_closed(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let Some(io) = rio(c, slf) else { return Ok(FALSE) };
    if !io.init { return err(c, c.vm().c_io_error.get(), "uninitialized stream"); }
    Ok(Value::bool(io.closed))
}
fn io_fileno(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = get_io(c, slf)?; Ok(Value::fix(io.fd as i64)) }
fn io_tty(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = get_io(c, slf)?; Ok(Value::bool(sys::isatty(io.fd) == 1)) }
fn io_path(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let p = match rio(c, slf) { Some(i) if i.has_path => i.path.clone(), _ => return Ok(NIL) };
    Ok(c.str_new(&p))
}
fn io_binmode(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = get_io(c, slf)?; io.binmode = true; io.enc = Some(ENC_BINARY); io.enc2 = None; Ok(slf) }
fn io_binmode_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = get_io(c, slf)?; Ok(Value::bool(io.binmode)) }
pub fn io_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let k = class_path(c, real_class_of(c, slf));
    let s = match rio(c, slf) {
        None => format!("#<{}>", k),
        Some(i) if !i.init => format!("#<{}: uninitialized>", k),
        Some(i) => {
            let named = i.has_path || !i.path.is_empty();
            if !named && i.closed { format!("#<{}:(closed)>", k) } else {
                let p = if named { lossy(&i.path) } else { format!("fd {}", i.fd) };
                format!("#<{}:{}{}>", k, p, if i.closed { " (closed)" } else { "" })
            }
        }
    };
    Ok(c.str_new(s.as_bytes()))
}
fn io_lineno(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = readable_nowait(c, slf)?; Ok(Value::fix(io.lineno)) }
fn io_set_lineno(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let n = num_to_long(c, a[0])?; let io = readable_nowait(c, slf)?; io.lineno = n; Ok(a[0]) }
fn do_seek(c: &mut Ctx, slf: Value, off: i64, whence: i32) -> Result<i64, Esc> {
    let io = get_io(c, slf)?;
    if io.data { io.rpos = match whence { 0 => off, 1 => io.rpos as i64 + off, _ => io.rbuf.len() as i64 + off }.clamp(0, io.rbuf.len() as i64) as usize; return Ok(io.rpos as i64); }
    io_flush_one(io);
    let off = if whence == 1 { off - (io.rbuf.len() - io.rpos) as i64 } else { off };
    io.rbuf.clear(); io.rpos = 0; io.eof = false;
    let r = sys::lseek(io.fd, off, whence);
    if r < 0 { let (e, p) = (sys::errno(), io.path.clone()); return Err(raise_errno_at(c, e, "", &p)); }
    Ok(r)
}
fn whence_arg(c: &mut Ctx, v: Option<&Value>) -> Result<i32, Esc> {
    Ok(match v { None => 0, Some(&w) if w.is_sym() => match crate::symbol::sym_str(w.as_sym()).as_str() { "SET" => 0, "CUR" => 1, "END" => 2, o => return arg_err(c, format!("unknown whence: {}", o)) }, Some(&w) => num_to_long(c, w)? as i32 })
}
fn io_rewind(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { do_seek(c, slf, 0, 0)?; let io = get_io(c, slf)?; io.lineno = 0; Ok(Value::fix(0)) }
fn io_seek(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { check_argc(c, a, 1, 2)?; let off = num_to_long(c, a[0])?; let w = whence_arg(c, a.get(1))?; do_seek(c, slf, off, w)?; Ok(Value::fix(0)) }
fn io_sysseek(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { check_argc(c, a, 1, 2)?; let off = num_to_long(c, a[0])?; let w = whence_arg(c, a.get(1))?; Ok(Value::fix(do_seek(c, slf, off, w)?)) }
fn io_set_pos(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let off = num_to_long(c, a[0])?; do_seek(c, slf, off, 0)?; Ok(a[0]) }
fn io_pos(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let io = get_io(c, slf)?;
    if io.data { return Ok(Value::fix(io.rpos as i64)); }
    io_flush_one(io);
    let r = sys::lseek(io.fd, 0, 1);
    if r < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    Ok(Value::fix(r - (io.rbuf.len() - io.rpos) as i64))
}
fn io_pid(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = get_io(c, slf)?; Ok(if io.pid > 0 { Value::fix(io.pid as i64) } else { NIL }) }
fn io_to_io(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(slf) }
fn io_ext_enc(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if rio(c, slf).is_none() { get_io(c, slf)?; }
    let io = rio(c, slf).expect("an IO");
    let e = if io.enc2.is_some() { io.enc2 } else if io.writable { io.enc } else { Some(enc_of(io)) };
    Ok(match e { Some(e) => crate::builtins::encoding::encoding_value(c, e), None => NIL })
}
fn io_int_enc(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if rio(c, slf).is_none() { get_io(c, slf)?; }
    let io = rio(c, slf).expect("an IO");
    Ok(if io.enc2.is_none() { NIL } else { crate::builtins::encoding::encoding_value(c, enc_of(io)) })
}
/// An IO's (encs.enc, encs.enc2): the read encoding (None: Encoding.default_external at the time)
/// and, when transcoding, the external one.
pub type Encs = (Option<u8>, Option<u8>);
/// An internal encoding argument: not given (Encoding.default_internal), none ("-"), or one.
#[derive(Clone, Copy, PartialEq)]
enum IntEnc { Default, No, Enc(u8) }
/// rb_io_ext_int_to_encs.
fn ext_int_to_encs(ext: Option<u8>, int: IntEnc, by_bom: bool) -> Encs {
    use crate::builtins::encoding::{default_external, default_internal};
    let default_ext = ext.is_none();
    let ext = ext.unwrap_or_else(default_external);
    let int = if ext == ENC_BINARY { None } else { match int { IntEnc::Default => default_internal(), IntEnc::No => None, IntEnc::Enc(e) => Some(e) } };
    match int {
        Some(i) if by_bom || i != ext => (Some(i), Some(ext)),
        _ => (if default_ext && int != Some(ext) { None } else { Some(ext) }, None),
    }
}
/// parse_mode_enc: "ext[:int]" with "BOM|" and "-" (no transcoding); unknown names are ignored
/// with a warning.  Also whether the encoding is to be set by a BOM.
fn parse_mode_enc(c: &mut Ctx, s: &[u8]) -> Result<(Encs, bool), Esc> {
    use crate::builtins::encoding::find_enc;
    let (e, i) = match s.iter().position(|&x| x == b':') { Some(p) => (&s[..p], Some(&s[p + 1..])), None => (s, None) };
    let has_bom = e.len() > 4 && e[..4].eq_ignore_ascii_case(b"BOM|");
    let e = if has_bom { &e[4..] } else { e };
    let bom = has_bom && e.len() >= 4 && e[..4].eq_ignore_ascii_case(b"UTF-");
    if has_bom && !bom { enc_warn(c, &format!("BOM with non-UTF encoding {} is nonsense", lossy(e)))?; }
    let ext = find_enc(e);
    if ext.is_none() { enc_warn(c, &format!("Unsupported encoding {} ignored", lossy(e)))?; }
    let int = match i {
        None => IntEnc::Default,
        Some(b"-") => IntEnc::No,
        Some(i) => match find_enc(i) {
            None => { enc_warn(c, &format!("Unsupported encoding {} ignored", lossy(i)))?; IntEnc::Default }
            Some(x) if !bom && Some(x) == ext => IntEnc::No,
            Some(x) => IntEnc::Enc(x),
        },
    };
    Ok((ext_int_to_encs(ext, int, bom), bom))
}
/// rb_warn with the caller's location.
fn enc_warn(c: &mut Ctx, msg: &str) -> Result<(), Esc> {
    if c.vm().verbose.get() == NIL { return Ok(()); }
    let loc = backtrace_lines(c, 0).first().map(|l| l.split(":in ").next().unwrap_or("").to_string()).unwrap_or_default();
    crate::builtins::kernel::warning_write(c, format!("{}: warning: {}\n", loc, msg).as_bytes())
}
/// An internal_encoding: option or set_encoding's second argument ("-" is none).
fn int_enc_arg(c: &mut Ctx, v: Value) -> Result<IntEnc, Esc> {
    if is_string(v) && c.str_bytes(v) == b"-" { return Ok(IntEnc::No); }
    Ok(IntEnc::Enc(crate::builtins::encoding::to_enc(c, v)?))
}
/// rb_io_extract_encoding_option: encoding: / external_encoding: / internal_encoding:.
fn enc_options(c: &mut Ctx, h: Value) -> Result<Option<(Encs, bool)>, Esc> {
    let g = |c: &mut Ctx, k: &str| -> RV { Ok(crate::hash::hash_get_opt(c, h, sym_value(k))?.unwrap_or(NIL)) };
    let (enc, ext, int) = (g(c, "encoding")?, g(c, "external_encoding")?, g(c, "internal_encoding")?);
    if ext != NIL || int != NIL {
        let e = if ext != NIL { Some(crate::builtins::encoding::to_enc(c, ext)?) } else { None };
        let mut i = if int != NIL { int_enc_arg(c, int)? } else { IntEnc::Default };
        if let (Some(e), IntEnc::Enc(x)) = (e, i) { if e == x { i = IntEnc::No; } }
        if enc != NIL {
            let sv = crate::rt::srb_tostr(c, enc)?;
            let s = String::from_utf8_lossy(c.str_bytes(sv)).into_owned();
            crate::builtins::kernel::warn_str(c, &format!("warning: Ignoring encoding parameter '{}': {}_encoding is used\n", s, if ext == NIL { "internal" } else { "external" }));
        }
        return Ok(Some((ext_int_to_encs(e, i, false), false)));
    }
    if enc == NIL { return Ok(None); }
    if is_string(enc) { let b = c.str_bytes(enc).to_vec(); return Ok(Some(parse_mode_enc(c, &b)?)); }
    let e = crate::builtins::encoding::to_enc(c, enc)?;
    Ok(Some((ext_int_to_encs(Some(e), IntEnc::Default, false), false)))
}
/// io_encoding_set: set_encoding(ext, int = nil) and IO.pipe's encodings.
fn encoding_set(c: &mut Ctx, v1: Value, v2: Value) -> Result<Encs, Esc> {
    use crate::builtins::encoding::to_enc;
    if v2 != NIL {
        let e2 = to_enc(c, v1)?;
        let (mut enc, mut enc2) = (Some(match int_enc_arg(c, v2)? { IntEnc::Enc(x) => x, _ => e2 }), Some(e2));
        if enc == enc2 { enc2 = None; }
        if enc2 == Some(ENC_BINARY) { enc = enc2; enc2 = None; }
        return Ok((enc, enc2));
    }
    if v1 == NIL { return Ok(ext_int_to_encs(None, IntEnc::Default, false)); }
    let v1 = if !is_string(v1) && !crate::builtins::encoding::is_encoding(c, v1) && v1.is_heap() && respond_to(c, v1, s::to_str, true) { expect_str(c, v1)? } else { v1 };
    if is_string(v1) && crate::builtins::encoding::ascii_compat(c.str_enc(v1)) { let b = c.str_bytes(v1).to_vec(); return Ok(parse_mode_enc(c, &b)?.0); }
    let e = to_enc(c, v1)?;
    Ok(ext_int_to_encs(Some(e), IntEnc::Default, false))
}
fn set_encs(c: &mut Ctx, v: Value, e: Encs) { if let Some(io) = rio_mut(c, v) { io.enc = e.0; io.enc2 = e.1; } }
fn io_set_encoding(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_open(c, slf, false)?;
    let (pos, opts) = match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    if pos.is_empty() || pos.len() > 2 { return argc_err(c, pos.len(), 1, 2); }
    let (nl, ec) = match opts { Some(h) => { let o = crate::builtins::encoding::parse_tc_opts(c, h)?; (o.newline, ecflags_of(&o)) } None => (0, 0) };
    let e = encoding_set(c, pos[0], pos.get(1).copied().unwrap_or(NIL))?;
    let (rd, bin) = rio(c, slf).map(|io| (io.readable, io.binmode)).unwrap_or((false, false));
    if bin && nl != 0 { return arg_err(c, "newline decorator with binary mode"); }
    validate_enc_binmode(c, rd, bin, Some(e))?;
    set_encs(c, slf, e);
    if let Some(io) = rio_mut(c, slf) { io.ecflags = ec; }
    Ok(slf)
}
/// io_strip_bom: the encoding of a BOM at the read position (consumed), if any.
fn strip_bom(io: &mut RIo) -> Option<u8> {
    use crate::builtins::encoding::{ENC_UTF16BE, ENC_UTF16LE, ENC_UTF32BE, ENC_UTF32LE};
    if !fill(io) { return None; }
    while io.rbuf.len() - io.rpos < 4 && !io.data {
        let mut more = [0u8; 4];
        match sys::read_fd(io.fd, &mut more[..4 - (io.rbuf.len() - io.rpos)]) { Ok(n) if n > 0 => io.rbuf.extend_from_slice(&more[..n]), _ => break }
    }
    let b = &io.rbuf[io.rpos..];
    let (e, n) = if b.starts_with(&[0xef, 0xbb, 0xbf]) { (ENC_UTF8, 3) } else if b.starts_with(&[0xfe, 0xff]) { (ENC_UTF16BE, 2) }
        else if b.starts_with(&[0xff, 0xfe, 0, 0]) { (ENC_UTF32LE, 4) } else if b.starts_with(&[0xff, 0xfe]) { (ENC_UTF16LE, 2) }
        else if b.starts_with(&[0, 0, 0xfe, 0xff]) { (ENC_UTF32BE, 4) } else { return None };
    io.rpos += n;
    Some(e)
}
/// io_set_encoding_by_bom: the external encoding from a BOM (the internal one is kept).
fn set_encoding_by_bom(c: &mut Ctx, v: Value) -> Option<u8> {
    let io = io_mut(c, v);
    let found = strip_bom(io);
    match found {
        Some(e) => { let int = if io.enc2.is_some() { io.enc } else { None }; (io.enc, io.enc2) = match int { Some(i) if i != e => (Some(i), Some(e)), _ => (Some(e), None) }; }
        None => io.enc2 = None,
    }
    found
}
fn io_set_encoding_by_bom(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let io = get_io(c, slf)?;
    if !io.binmode { return arg_err(c, "ASCII incompatible encoding needs binmode"); }
    if io.enc2.is_some() { return arg_err(c, "encoding conversion is set"); }
    if let Some(e) = io.enc { if e != ENC_BINARY { return arg_err(c, format!("encoding is set to {} already", crate::builtins::encoding::enc_name(e))); } }
    Ok(match set_encoding_by_bom(c, slf) { Some(e) => crate::builtins::encoding::encoding_value(c, e), None => NIL })
}
/// Bytes read from `io` as a String of its read encoding, converted from the external one if it has both.
fn read_str(c: &mut Ctx, enc: Encs, ec: u8, b: Vec<u8>) -> RV {
    let to = enc.0.unwrap_or_else(crate::builtins::encoding::default_external);
    match enc.1 {
        Some(from) => match io_transcode(c, &b, from, to, ec)? {
            Some(v) => Ok(c.str_from_vec_enc(v, to)),
            None => Ok(c.str_from_vec_enc(b, from)),
        },
        None => Ok(c.str_from_vec_enc(b, to)),
    }
}

fn io_autoclose_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = get_io(c, slf)?; Ok(Value::bool(io.autoclose)) }
fn io_set_autoclose(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let io = get_io(c, slf)?; io.autoclose = a[0].truthy(); Ok(a[0]) }
fn io_cloexec_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = get_io(c, slf)?; Ok(Value::bool(sys::fcntl(io.fd, sys::F_GETFD, 0) & sys::FD_CLOEXEC != 0)) }
fn io_set_cloexec(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let io = get_io(c, slf)?;
    for fd in [io.fd, io.wfd] { if fd >= 0 { let f = sys::fcntl(fd, sys::F_GETFD, 0); sys::fcntl(fd, sys::F_SETFD, if a[0].truthy() { f | sys::FD_CLOEXEC } else { f & !sys::FD_CLOEXEC }); } }
    Ok(a[0])
}
fn io_fcntl(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let cmd = num_to_long(c, a[0])? as i32; let arg = match a.get(1) { Some(&v) if v.is_fixnum() => v.as_fix() as i32, _ => 0 };
    let io = get_io(c, slf)?;
    let r = sys::fcntl(io.fd, cmd, arg);
    if r < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    Ok(Value::fix(r as i64))
}
// io/nonblock
fn set_nonblock(fd: i32, on: bool) -> bool {
    let fl = sys::fcntl(fd, sys::F_GETFL, 0);
    let was = fl & sys::O_NONBLOCK != 0;
    if fl >= 0 && was != on { sys::fcntl(fd, sys::F_SETFL, if on { fl | sys::O_NONBLOCK } else { fl & !sys::O_NONBLOCK }); }
    was
}
fn io_nonblock_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let io = get_io(c, slf)?;
    let fl = sys::fcntl(out_fd(io), sys::F_GETFL, 0);
    if fl < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    Ok(Value::bool(fl & sys::O_NONBLOCK != 0))
}
fn io_set_nonblock(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let io = get_io(c, slf)?; set_nonblock(out_fd(io), a[0].truthy()); Ok(a[0]) }
fn io_nonblock(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    let on = a.first().map(|v| v.truthy()).unwrap_or(true);
    let fd = out_fd(get_io(c, slf)?);
    let was = set_nonblock(fd, on);
    let r = yield_block(c, b, &[slf]);
    if was != on { if let Some(io) = rio(c, slf) { if !io.closed { set_nonblock(out_fd(io), was); } } }
    r
}
/// require "io/nonblock".
pub fn init_nonblock(c: &mut Ctx) {
    let io = c.vm().c_io.get();
    define_cfunc(c, io, "nonblock?", io_nonblock_p, 0); define_cfunc(c, io, "nonblock=", io_set_nonblock, 1); define_cfunc(c, io, "nonblock", io_nonblock, -1);
}
/// The deadline of an IO wait's timeout argument (nil: none).
pub fn wait_deadline(c: &mut Ctx, v: Option<&Value>) -> Result<Option<std::time::Instant>, Esc> {
    Ok(match v { Some(&t) if t != NIL && !is_hash(t) => {
        let f = crate::builtins::numeric_methods::to_f(c, t)?;
        Some(std::time::Instant::now() + std::time::Duration::from_secs_f64(if f.is_finite() { f.clamp(0.0, 1e9) } else { 1e9 }))
    } _ => None })
}
/// Wait for `events` on one fd through the scheduler (other threads run; signals are handled).
pub fn wait_one(c: &mut Ctx, fd: i32, events: i16, deadline: Option<std::time::Instant>) -> Result<Option<i16>, Esc> {
    out_flush();
    let r = crate::builtins::thread::poll_wait(c, vec![sys::PollFd { fd, events, revents: 0 }], deadline)?;
    Ok(r.map(|p| p[0].revents))
}
fn io_wait_readable(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let dl = wait_deadline(c, a.first())?;
    let io = readable_nowait(c, slf)?;
    if io.rpos < io.rbuf.len() { return Ok(slf); }
    let fd = io.fd;
    let r = wait_one(c, fd, sys::POLLIN, dl)?;
    check_open(c, slf, false)?;
    Ok(if r.is_some() { slf } else { NIL })
}
fn io_wait_writable(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let dl = wait_deadline(c, a.first())?;
    let io = get_io(c, slf)?;
    let fd = out_fd(io);
    let r = wait_one(c, fd, sys::POLLOUT, dl)?;
    check_open(c, slf, false)?;
    Ok(if r.is_some() { slf } else { NIL })
}
fn io_ready_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = readable_nowait(c, slf)?; Ok(Value::bool(io.rpos < io.rbuf.len() || wait_fd(io.fd, sys::POLLIN, 0))) }
fn io_nread(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let io = readable_nowait(c, slf)?; Ok(Value::fix((io.rbuf.len() - io.rpos) as i64)) }

// ---- opening --------------------------------------------------------------------------------

pub struct OpenMode { pub flags: i32, pub bin: bool, pub encs: Option<Encs>, pub perm: u32, pub bom: bool, pub ecflags: u8 }
impl OpenMode {
    pub fn new(flags: i32, bin: bool) -> OpenMode { OpenMode { flags, bin, encs: None, perm: 0o666, bom: false, ecflags: 0 } }
}
/// Access flags of a mode string, and whether it says 'b' / 't'.
fn mode_str_flags(c: &mut Ctx, mode: &str) -> Result<(i32, bool, bool), Esc> {
    let m = mode.split(':').next().unwrap_or("r");
    let mut ch = m.chars();
    let mut flags = match ch.next() {
        Some('r') => sys::O_RDONLY, Some('w') => sys::O_WRONLY | sys::O_CREAT | sys::O_TRUNC, Some('a') => sys::O_WRONLY | sys::O_CREAT | sys::O_APPEND,
        _ => return arg_err(c, format!("invalid access mode {}", mode)),
    };
    let (mut bin, mut text) = (false, false);
    for x in ch {
        match x {
            '+' => flags = (flags & !sys::O_ACCMODE) | sys::O_RDWR,
            'b' if !text => bin = true, 't' if !bin => text = true,
            'x' => { if flags & sys::O_TRUNC == 0 { return arg_err(c, format!("invalid access mode {}", mode)); } flags |= sys::O_EXCL; }
            'e' => flags |= sys::O_CLOEXEC,
            _ => return arg_err(c, format!("invalid access mode {}", mode)),
        }
    }
    Ok((flags, bin, text))
}
/// Parse (mode = "r", perm = 0666, **opts) as File.open / IO.new take them (rb_io_extract_modeenc).
pub fn parse_mode(c: &mut Ctx, a: &[Value]) -> Result<OpenMode, Esc> {
    let mut om = OpenMode::new(sys::O_RDONLY, false);
    let (pos, opts) = match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    let mut mode = pos.first().copied().unwrap_or(NIL);
    if let Some(&p) = pos.get(1) { if p != NIL { om.perm = num_to_long(c, p)? as u32; } }
    let g = |c: &mut Ctx, h: Value, k: &str| crate::hash::hash_get_opt(c, h, sym_value(k));
    if let Some(h) = opts {
        if let Some(m) = g(c, h, "mode")? { if m != NIL { if mode != NIL { return arg_err(c, "mode specified twice"); } mode = m; } }
        if let Some(p) = g(c, h, "perm")? { if p != NIL { om.perm = num_to_long(c, p)? as u32; } }
    }
    if mode != NIL && !mode.is_fixnum() && !is_string(mode) {
        mode = match crate::builtins::random::check_to_int(c, mode)? { Some(i) => i, None => expect_str(c, mode)? };
    }
    let mut text = false;
    if mode.is_fixnum() { om.flags |= mode.as_fix() as i32; }
    else if mode != NIL {
        let ms = to_rstring(c, mode)?;
        let (f, b, t) = mode_str_flags(c, &ms)?;
        om.flags |= f; om.bin |= b; text = t;
        if let Some(i) = ms.find(':') { let (e, bom) = parse_mode_enc(c, ms[i + 1..].as_bytes())?; om.encs = Some(e); om.bom = bom; }
    }
    if let Some(h) = opts {
        if let Some(t) = g(c, h, "textmode")? {
            if t != NIL {
                if text { return arg_err(c, "textmode specified twice"); }
                if om.bin { return arg_err(c, "both textmode and binmode specified"); }
                text = t.truthy();
            }
        }
        if let Some(b) = g(c, h, "binmode")? {
            if b != NIL {
                if om.bin { return arg_err(c, "binmode specified twice"); }
                if text { return arg_err(c, "both textmode and binmode specified"); }
                om.bin = b.truthy();
            }
        }
        if let Some(f) = g(c, h, "flags")? { if f != NIL { om.flags |= num_to_long(c, f)? as i32; } }
        let tc = crate::builtins::encoding::parse_tc_opts(c, h)?;
        om.ecflags = ecflags_of(&tc);
        if let Some((e, bom)) = enc_options(c, h)? {
            if om.encs.is_some() { return arg_err(c, "encoding specified twice"); }
            om.encs = Some(e); om.bom = bom;
        }
        if om.bin && tc.newline != 0 { return arg_err(c, "newline decorator with binary mode"); }
    }
    if om.bin && text { return arg_err(c, "both textmode and binmode specified"); }
    let readable = om.flags & sys::O_ACCMODE != sys::O_WRONLY;
    validate_enc_binmode(c, readable, om.bin, om.encs)?;
    Ok(om)
}
/// validate_enc_binmode: reading an ASCII-incompatible encoding without conversion needs binmode.
fn validate_enc_binmode(c: &mut Ctx, readable: bool, bin: bool, encs: Option<Encs>) -> Result<(), Esc> {
    let (enc, enc2) = encs.unwrap_or((None, None));
    let e = enc.unwrap_or_else(crate::builtins::encoding::default_external);
    if readable && enc2.is_none() && !bin && !crate::builtins::encoding::ascii_compat(e) { return arg_err(c, "ASCII incompatible encoding needs binmode"); }
    Ok(())
}

pub fn path_arg(c: &mut Ctx, v: Value) -> Result<PathArg, Esc> {
    let sv = if is_string(v) { v } else if respond_to(c, v, intern(b"to_path"), true) { let p = funcall(c, v, intern(b"to_path"), &[], std::ptr::null())?; expect_str(c, p)? } else { expect_str(c, v)? };
    let b = c.str_bytes(sv);
    if b.contains(&0) { return arg_err(c, "path name contains null byte"); }   // rb_get_path
    Ok(PathArg(b.to_vec()))
}
fn setup_io(c: &mut Ctx, v: Value, fd: i32, path: Option<&[u8]>, om: &OpenMode) -> Result<(), Esc> {
    let io = io_mut(c, v);
    if io.init && !io.closed && io.fd > 2 { close_fds(io); }
    *io = blank();
    io.fd = fd; io.init = true;
    let acc = om.flags & sys::O_ACCMODE;
    io.readable = acc != sys::O_WRONLY; io.writable = acc != sys::O_RDONLY;
    io.binmode = om.bin;
    (io.enc, io.enc2) = om.encs.unwrap_or_else(|| if om.bin { (Some(ENC_BINARY), None) } else { ext_int_to_encs(None, IntEnc::Default, false) });
    io.ecflags = om.ecflags;
    if let Some(p) = path { io.path = p.to_vec(); io.has_path = true; }
    if om.bom && io.readable { set_encoding_by_bom(c, v); }
    Ok(())
}
pub fn sysopen(c: &mut Ctx, path: &PathArg, om: &OpenMode) -> Result<i32, Esc> {
    let cp = path.cstr();
    if om.flags & sys::O_NONBLOCK == 0 && crate::builtins::thread::active(c) && is_fifo(path) { return open_fifo_coop(c, path, om); }
    loop {
        let fd = sys::open(&cp, om.flags | sys::O_CLOEXEC, om.perm);
        if fd >= 0 { return Ok(fd); }
        let e = sys::errno();
        if e == sys::EINTR { continue; }
        return Err(raise_errno_at(c, e, "rb_sysopen - ", path.as_bytes()));
    }
}
fn is_fifo(path: &PathArg) -> bool { use std::os::unix::fs::FileTypeExt; std::fs::metadata(path.path()).is_ok_and(|m| m.file_type().is_fifo()) }
/// open(2) of a FIFO with other threads running: O_NONBLOCK so it cannot block them all; a writer
/// without a reader (ENXIO) retries after a back-off sleep. The fd is then made blocking again.
/// (A reader no longer waits for a writer at open; its first read waits instead.)
fn open_fifo_coop(c: &mut Ctx, path: &PathArg, om: &OpenMode) -> Result<i32, Esc> {
    let cp = path.cstr();
    let mut nap = std::time::Duration::from_millis(1);
    loop {
        let fd = sys::open(&cp, om.flags | sys::O_CLOEXEC | sys::O_NONBLOCK, om.perm);
        if fd >= 0 { let fl = sys::fcntl(fd, sys::F_GETFL, 0); sys::fcntl(fd, sys::F_SETFL, fl & !sys::O_NONBLOCK); return Ok(fd); }
        let e = sys::errno();
        if e == sys::EINTR { continue; }
        if e != 6 { return Err(raise_errno_at(c, e, "rb_sysopen - ", path.as_bytes())); }   // ENXIO: no reader yet
        crate::builtins::thread::sleep_for(c, nap)?;
        nap = (nap * 2).min(std::time::Duration::from_millis(50));
    }
}
/// File#initialize(path_or_fd, mode, perm, **opts)
fn file_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, 3); }
    if a[0].is_fixnum() { return io_initialize(c, slf, a, _b); }
    let p = path_arg(c, a[0])?;
    let om = parse_mode(c, &a[1..])?;
    let fd = sysopen(c, &p, &om)?;
    // an O_TMPFILE file has no path
    let path = if om.flags & 0o20000000 != 0 { None } else { Some(p.as_bytes()) };
    setup_io(c, slf, fd, path, &om)?;
    Ok(NIL)
}
fn io_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, 2); }
    let fd = num_to_long(c, a[0])? as i32;
    let fl = sys::fcntl(fd, sys::F_GETFL, 0);
    if fl < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    let mut om = parse_mode(c, &a[1..])?;
    if a.len() < 2 || is_hash(a[1]) && crate::hash::hash_get_opt(c, a[1], sym_value("mode"))?.is_none() { om.flags = (om.flags & !sys::O_ACCMODE) | (fl & sys::O_ACCMODE); }
    else if (fl & sys::O_ACCMODE) != sys::O_RDWR && (om.flags & sys::O_ACCMODE) != (fl & sys::O_ACCMODE) { return Err(raise_errno(c, 22, "")); }
    setup_io(c, slf, fd, None, &om)?;
    if let Some(&l) = a.last() { if is_hash(l) {
        if let Some(v) = crate::hash::hash_get_opt(c, l, sym_value("autoclose"))? { if let Some(io) = rio_mut(c, slf) { io.autoclose = v.truthy(); } }
        if let Some(v) = crate::hash::hash_get_opt(c, l, sym_value("path"))? {
            if v != NIL { let ps = expect_str(c, v)?; let b = c.str_bytes(ps).to_vec(); if let Some(io) = rio_mut(c, slf) { io.path = b; io.has_path = true; } }
        }
    } }
    Ok(NIL)
}
fn io_init_copy(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let src = get_io(c, a[0])?;
    io_flush_one(src);
    let fd = sys::fcntl(src.fd, 1030, 3); // F_DUPFD_CLOEXEC
    if fd < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    let mut io = opened(fd, &src.path, src.readable, src.writable);
    io.binmode = src.binmode; io.enc = src.enc; io.enc2 = src.enc2; io.ecflags = src.ecflags; io.has_path = src.has_path; io.lineno = src.lineno; io.sync = src.sync;
    if src.wfd >= 0 && src.wfd != src.fd { io.wfd = sys::fcntl(src.wfd, 1030, 3); }
    *io_mut(c, slf) = io;
    Ok(slf)
}
/// Class.open-style: new + yield + close.
pub fn open_with_block(c: &mut Ctx, klass: Value, a: &[Value], blk: *const Block) -> RV {
    let f = crate::builtins::object::c_new(c, klass, a, std::ptr::null())?;
    if blk.is_null() { return Ok(f); }
    let r = yield1(c, blk, f);
    let (ek, ev) = (c.esc, c.esc_val);
    let cr = funcall(c, f, intern(b"close"), &[], std::ptr::null());
    if r.is_err() { c.esc = ek; c.esc_val = ev; return r; }
    cr?;
    r
}
fn io_s_open(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV { open_with_block(c, slf, a, blk) }
/// Kernel#open: a file, "|command", or anything with #to_open.
fn k_open(c: &mut Ctx, _s: Value, a: &[Value], blk: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    if !is_string(a[0]) && respond_to(c, a[0], intern(b"to_open"), true) {
        let r = funcall(c, a[0], intern(b"to_open"), &a[1..], std::ptr::null())?;
        return if blk.is_null() { Ok(r) } else { yield1(c, blk, r) };
    }
    open_with_block(c, c.vm().c_file.get(), a, blk)
}
fn io_s_sysopen(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    let om = parse_mode(c, &a[1..])?;
    Ok(Value::fix(sysopen(c, &p, &om)? as i64))
}
fn io_s_pipe(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    let mut fds = [0i32; 2];
    if sys::pipe2(&mut fds, sys::O_CLOEXEC) < 0 { return Err(raise_errno(c, sys::errno(), "")); }
    let r = new_io(c, slf, fds[0], b"", true, false);
    let w = new_io(c, slf, fds[1], b"", false, true);
    io_mut(c, w).sync = true;
    let pos = match a.last() { Some(&l) if is_hash(l) => &a[..a.len() - 1], _ => a };
    if !pos.is_empty() { let e = encoding_set(c, pos[0], pos.get(1).copied().unwrap_or(NIL))?; set_encs(c, r, e); }
    let pair = c.ary_new_from(&[r, w]);
    if blk.is_null() { return Ok(pair); }
    let res = yield_block(c, blk, &[r, w]);
    let (ek, ev) = (c.esc, c.esc_val);
    let _ = close_io(c, r); let _ = close_io(c, w);
    if res.is_err() { c.esc = ek; c.esc_val = ev; }
    res
}
fn io_s_popen(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    let (mut pos, opts): (Vec<Value>, Option<Value>) = match a.last() { Some(&l) if is_hash(l) && a.len() > 1 => (a[..a.len() - 1].to_vec(), Some(l)), _ => (a.to_vec(), None) };
    if pos.is_empty() { return argc_err(c, 0, 1, 3); }
    let env = if is_hash(pos[0]) && pos.len() > 1 { Some(pos.remove(0)) } else { None };
    let cmd = pos[0];
    let mode = if pos.len() > 1 { to_rstring(c, pos[1])? } else { "r".into() };
    let (flags, bin, _) = mode_str_flags(c, &mode)?;
    let acc = flags & sys::O_ACCMODE;
    let (rd, wr) = (acc != sys::O_WRONLY, acc != sys::O_RDONLY);
    let mut pr = [-1i32; 2]; let mut pw = [-1i32; 2];
    if rd { sys::pipe2(&mut pr, sys::O_CLOEXEC); } if wr { sys::pipe2(&mut pw, sys::O_CLOEXEC); }
    let fork_dash = is_string(cmd) && c.str_bytes(cmd) == b"-";
    let pid = if fork_dash {
        flush_all(c);
        let pid = sys::fork();
        if pid == 0 {
            if rd { sys::dup2(pr[1], 1); } if wr { sys::dup2(pw[0], 0); } for fd in [pr[0], pr[1], pw[0], pw[1]] { if fd > 2 { sys::close(fd); } }
            crate::builtins::process::after_fork_child(c);
            if blk.is_null() { return Ok(NIL); }
            let r = yield1(c, blk, NIL);
            crate::builtins::process::child_exit(c, r);
        }
        pid
    } else {
        let (env, words, opts) = if is_array(cmd) {
            let mut w = c.ary(cmd).to_vec();
            let e2 = if !w.is_empty() && is_hash(w[0]) { Some(w.remove(0)) } else { env };
            let o2 = if w.len() > 1 && is_hash(w[w.len() - 1]) { w.pop() } else { opts };
            (e2, w, o2)
        } else { (env, vec![cmd], opts) };
        let mut spec = crate::builtins::process::spawn_spec(c, env, &words, opts)?;
        // the pipe goes first so err: [:child, :out] refers to it
        if rd { spec.redirect.insert(0, (1, pr[1])); }
        if wr { spec.redirect.insert(0, (0, pw[0])); }
        crate::builtins::process::spawn_exec(c, &spec)?
    };
    if rd { sys::close(pr[1]); } if wr { sys::close(pw[0]); }
    if pid < 0 { return Err(raise_errno(c, sys::errno(), "fork(2)")); }
    let io = new_io(c, slf, if rd { pr[0] } else { pw[1] }, b"", rd, wr);
    { let r = io_mut(c, io); if rd && wr { r.wfd = pw[1]; } r.pid = pid; r.sync = true; r.binmode = bin; if bin { r.enc = Some(ENC_BINARY); } }
    if let Some(i) = mode.find(':') { let e = parse_mode_enc(c, mode[i + 1..].as_bytes())?.0; set_encs(c, io, e); }
    if blk.is_null() { return Ok(io); }
    let r = yield1(c, blk, io);
    let (ek, ev) = (c.esc, c.esc_val);
    let _ = close_io(c, io);
    if r.is_err() { c.esc = ek; c.esc_val = ev; }
    r
}
/// rb_time_interval for IO.select: milliseconds, -1 for none (nil) or an infinite one.
fn select_timeout(c: &mut Ctx, v: Option<&Value>) -> Result<i32, Esc> {
    let t = match v { Some(&t) if t != NIL => t, _ => return Ok(-1) };
    if !crate::numeric::is_numeric(t) { return type_err(c, format!("can't convert {} into time interval", crate::builtins::kernel::class_name_for_err(c, t))); }
    let f = crate::builtins::numeric_methods::to_f(c, t)?;
    if f.is_nan() { return err(c, c.vm().c_range_error.get(), "NaN out of Time range"); }
    if f < 0.0 { return arg_err(c, "time interval must not be negative"); }
    Ok(if f * 1000.0 >= i32::MAX as f64 { -1 } else { (f * 1000.0).ceil() as i32 })
}
fn io_s_select(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let ms = select_timeout(c, a.get(3))?;
    let mut pfds: Vec<sys::PollFd> = vec![];
    let mut who: Vec<(usize, Value)> = vec![];
    let out = [c.ary_new(), c.ary_new(), c.ary_new()];
    let res = c.ary_new_from(&out);
    let mut ready_now = false;
    for k in 0..3 {
        let Some(&l) = a.get(k) else { continue };
        if l == NIL { continue; }
        let l = expect_array(c, l)?;
        let mut i = 0;
        while i < c.ary_len(l) {
            let v = c.ary_get(l, i); i += 1;
            let iov = if is_io(c, v) { v } else { funcall(c, v, intern(b"to_io"), &[], std::ptr::null())? };
            let io = get_io(c, iov)?;
            if k == 0 && io.rpos < io.rbuf.len() { c.ary_push(out[0], v); ready_now = true; continue; }
            pfds.push(sys::PollFd { fd: if k == 1 { out_fd(io) } else { io.fd }, events: [sys::POLLIN, sys::POLLOUT, 2][k], revents: 0 });
            who.push((k, v));
        }
    }
    out_flush();
    if ready_now { sys::poll(&mut pfds, 0); }
    else {
        // a POLL blop: other threads run and trap handlers fire while this one waits
        let deadline = if ms < 0 { None } else { Some(std::time::Instant::now() + std::time::Duration::from_millis(ms as u64)) };
        match crate::builtins::thread::poll_wait(c, pfds, deadline)? { Some(p) => pfds = p, None => return Ok(NIL) }
    }
    for (p, (k, v)) in pfds.iter().zip(who.iter()) { if p.revents != 0 { c.ary_push(out[*k], *v); } }
    Ok(res)
}
/// IO.read-style helpers: open path, run f, close.
fn with_file<R>(c: &mut Ctx, klass: Value, path: Value, om: OpenMode, f: impl FnOnce(&mut Ctx, Value) -> Result<R, Esc>) -> Result<R, Esc> {
    let p = path_arg(c, path)?;
    let fd = sysopen(c, &p, &om)?;
    let k = if klass == c.vm().c_io.get() { c.vm().c_file.get() } else { klass };
    let io = new_io(c, k, -1, p.as_bytes(), true, true);
    setup_io(c, io, fd, Some(p.as_bytes()), &om)?;
    let r = f(c, io);
    let (ek, ev) = (c.esc, c.esc_val);
    let cr = close_io(c, io);
    if r.is_err() { c.esc = ek; c.esc_val = ev; return r; }
    cr?;
    r
}
fn split_opts(a: &[Value]) -> (&[Value], Option<Value>) { match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) } }
fn read_mode(c: &mut Ctx, opts: Option<Value>, bin: bool) -> Result<OpenMode, Esc> {
    let mut om = match opts { Some(h) => parse_mode(c, &[h])?, None => OpenMode::new(0, false) };
    om.bin |= bin;
    Ok(om)
}
fn io_s_read_impl(c: &mut Ctx, slf: Value, a: &[Value], bin: bool) -> RV {
    let (pos, opts) = split_opts(a);
    if pos.is_empty() || pos.len() > 3 { return argc_err(c, pos.len(), 1, 3); }
    let om = read_mode(c, opts, bin)?;
    let (len, off) = (pos.get(1).copied().unwrap_or(NIL), pos.get(2).copied().unwrap_or(NIL));
    if len != NIL { let n = num_to_long(c, len)?; if n < 0 { return arg_err(c, format!("negative length {} given", n)); } }
    with_file(c, slf, pos[0], om, |c, io| {
        if off != NIL { let o = num_to_long(c, off)?; if o < 0 { if bin { let p = rio(c, io).map(|i| lossy(&i.path)).unwrap_or_default(); return Err(raise_errno(c, 22, &p)); } return arg_err(c, format!("negative offset {} given", o)); } do_seek(c, io, o, 0)?; }
        let r = io_read(c, io, if len == NIL { &[] } else { std::slice::from_ref(&len) }, std::ptr::null())?;
        Ok(r)
    })
}
fn io_s_read(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { io_s_read_impl(c, slf, a, false) }
fn io_s_binread(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { io_s_read_impl(c, slf, a, true) }
/// (path, sep/limit args) plus the chomp: option as a Hash, for readlines / foreach.
fn line_opts(c: &mut Ctx, pos: &[Value], opts: Option<Value>) -> Result<Vec<Value>, Esc> {
    let chomp = match opts { Some(h) => crate::hash::hash_get_opt(c, h, sym_value("chomp"))?, None => None };
    let mut la: Vec<Value> = pos[1..].to_vec();
    if let Some(v) = chomp { let h = c.hash_new(); crate::hash::hash_set(c, h, sym_value("chomp"), v)?; la.push(h); }
    Ok(la)
}
fn io_s_readlines(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, opts) = split_opts(a);
    if pos.is_empty() { return argc_err(c, 0, 1, 3); }
    let om = read_mode(c, opts, false)?;
    let la = line_opts(c, pos, opts)?;
    let keep = c.ary_new_from(&la);   // the chomp Hash lives only in `la` while the file is opened
    let r = with_file(c, slf, pos[0], om, |c, io| io_readlines(c, io, &la, std::ptr::null()));
    std::hint::black_box(keep);
    r
}
fn io_s_foreach(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"foreach"), a, NIL); }
    let (pos, opts) = split_opts(a);
    if pos.is_empty() { return argc_err(c, 0, 1, 3); }
    let om = read_mode(c, opts, false)?;
    let la = line_opts(c, pos, opts)?;
    let keep = c.ary_new_from(&la);
    let r = with_file(c, slf, pos[0], om, |c, io| io_each_line(c, io, &la, blk));
    std::hint::black_box(keep);
    r?;
    Ok(NIL)
}
fn io_s_write_impl(c: &mut Ctx, slf: Value, a: &[Value], bin: bool) -> RV {
    let (pos, opts) = split_opts(a);
    if pos.len() < 2 || pos.len() > 3 { return argc_err(c, pos.len(), 2, 3); }
    let off = pos.get(2).copied().unwrap_or(NIL);
    let mut om = match opts { Some(h) => parse_mode(c, &[h])?, None => OpenMode::new(0, false) };
    if om.flags == 0 { om.flags = sys::O_WRONLY | sys::O_CREAT | if off == NIL { sys::O_TRUNC } else { 0 }; }
    om.bin |= bin;
    let data = pos[1];
    with_file(c, slf, pos[0], om, |c, io| {
        if off != NIL { let o = num_to_long(c, off)?; do_seek(c, io, o, 0)?; }
        io_write(c, io, &[data], std::ptr::null())
    })
}
fn io_s_write(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { io_s_write_impl(c, slf, a, false) }
fn io_s_binwrite(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { io_s_write_impl(c, slf, a, true) }
fn io_s_copy_stream(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 2, 4)?;
    let open_side = |c: &mut Ctx, v: Value, w: bool| -> Result<(Value, bool), Esc> {
        if is_string(v) || (!is_io(c, v) && respond_to(c, v, intern(b"to_path"), true)) {
            let p = path_arg(c, v)?;
            let om = OpenMode::new(if w { sys::O_WRONLY | sys::O_CREAT | sys::O_TRUNC } else { 0 }, true);
            let fd = sysopen(c, &p, &om)?;
            let io = new_io(c, c.vm().c_file.get(), fd, p.as_bytes(), !w, w);
            Ok((io, true))
        } else { Ok((v, false)) }
    };
    let (src, sc) = open_side(c, a[0], false)?;
    let (dst, dc) = open_side(c, a[1], true)?;
    let len = match a.get(2) { Some(&l) if l != NIL => Some(num_to_long(c, l)?), _ => None };
    let src_io = is_io(c, src);
    if let Some(&o) = a.get(3) { if o != NIL {
        if !src_io { return arg_err(c, "cannot specify src_offset for non-IO"); }
        let o = num_to_long(c, o)?; do_seek(c, src, o, 0)?;
    } }
    // CRuby's copy_stream_fallback: 16 KiB chunks; a duck-typed source gets readpartial(len, buf) or read(len, buf)
    let partial = src_io || respond_to(c, src, intern(b"readpartial"), true);
    let meth = intern(if partial { b"readpartial" } else { b"read" });
    let dst_io = is_io(c, dst);
    let buf = c.str_new_enc(b"", ENC_BINARY);
    let (mut total, mut rest) = (0i64, len.unwrap_or(-1));
    loop {
        // rest == 0 only (CRuby: a source that overshoots gets a negative length)
        let l = match len { None => 16384, Some(_) if rest == 0 => break, Some(_) => rest.min(16384) };
        let r = if src_io { io_readpartial(c, src, &[Value::fix(l), buf], std::ptr::null()) } else { funcall(c, src, meth, &[Value::fix(l), buf], std::ptr::null()) };
        match r {
            Ok(v) => { if !partial && v == NIL { break; } }
            Err(e) => {
                if partial && c.esc == EscKind::Raise && is_kind_of(c, c.esc_val, c.vm().c_eof_error.get()) { c.esc = EscKind::None; break; }
                return Err(e);
            }
        }
        let w = if dst_io { io_write(c, dst, &[buf], std::ptr::null())? } else { funcall(c, dst, intern(b"write"), &[buf], std::ptr::null())? };
        let w = num_to_long(c, w)?;
        total += w; rest -= w;
        if !partial && c.str_bytes(buf).is_empty() { break; }
    }
    if sc { close_io(c, src)?; }
    if dc { close_io(c, dst)?; } else if let Some(d) = rio_mut(c, dst) { if d.init && !d.closed { io_flush_one(d); } }
    Ok(Value::fix(total))
}
fn io_s_try_convert(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    if is_io(c, a[0]) { return Ok(a[0]); }
    if respond_to(c, a[0], intern(b"to_io"), true) { return funcall(c, a[0], intern(b"to_io"), &[], std::ptr::null()); }
    Ok(NIL)
}
fn io_reopen(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, 2); }
    { let io = io_mut(c, slf); if io.init && !io.closed { if out_fd(io) == 1 { out_flush(); } io_flush_one(io); } }
    if let Some(src) = rio(c, a[0]) {
        if !src.init || src.closed { return err(c, c.vm().c_io_error.get(), "closed stream"); }
        let src = io_mut(c, a[0]);
        io_flush_one(src);
        if src.fd == 1 { out_flush(); }
        let (sfd, sr, sw, spath, shas, unread) = (src.fd, src.readable, src.writable, src.path.clone(), src.has_path, src.rbuf.len() - src.rpos);
        let io = io_mut(c, slf);
        if io.init && !io.closed && io.fd >= 0 { sys::dup2(sfd, io.fd); } else { io.fd = sys::dup(sfd); io.init = true; io.closed = false; }
        io.readable = sr; io.writable = sw; io.path = spath; io.has_path = shas; io.rbuf.clear(); io.rpos = 0;
        let pos = sys::lseek(sfd, 0, 1);
        if pos >= 0 && unread > 0 { sys::lseek(io.fd, pos - unread as i64, 0); }
        return Ok(slf);
    }
    let p = path_arg(c, a[0])?;
    let mut om = parse_mode(c, &a[1..])?;
    let (init, rd, wr) = { let io = io_mut(c, slf); (io.init, io.readable, io.writable) };
    if a.len() < 2 && init { om.flags = if rd && wr { sys::O_RDWR } else if wr { sys::O_WRONLY | sys::O_CREAT | sys::O_TRUNC } else { 0 }; }
    let fd = sysopen(c, &p, &om)?;
    let io = io_mut(c, slf);
    if io.init && !io.closed && io.fd >= 0 { sys::dup2(fd, io.fd); sys::close(fd); } else { io.fd = fd; io.init = true; io.closed = false; }
    let acc = om.flags & sys::O_ACCMODE;
    io.readable = acc != sys::O_WRONLY; io.writable = acc != sys::O_RDONLY;
    io.path = p.0; io.has_path = true; io.rbuf.clear(); io.rpos = 0;
    Ok(slf)
}

pub fn setup_data(c: &mut Ctx, src: &[u8], file: &str) {
    if let Some(pos) = find_end_marker(src) {
        let mut r = opened(-1, file.as_bytes(), true, false);
        r.rbuf = src.to_vec(); r.rpos = pos; r.data = true; r.has_path = true; r.lineno = 0;
        let io = c.alloc(c.vm().c_file.get(), Body::Data(Box::new(r)));
        const_set(c, c.vm().c_object.get(), intern(b"DATA"), io);
    }
}
fn find_end_marker(src: &[u8]) -> Option<usize> {
    let mut i = 0;
    for line in src.split(|&b| b == b'\n') {
        if line == b"__END__" || line == b"__END__\r" { let p = i + line.len() + 1; return Some(p.min(src.len())); }
        i += line.len() + 1;
    }
    None
}

/// A frozen String at boot (no context yet: builtins::boot, pattern 16 of safe_heap.md).
pub fn boot_frozen_str(vmr: &Vm, b: &[u8]) -> Value {
    let v = crate::gc::alloc(vmr.c_string.get(), Body::String(RString { b: crate::sbuf::SBuf::from_slice(b), enc: ENC_UTF8 }));
    obj_set_flags(v, FL_FROZEN);
    v
}

pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    // CRuby holds two descriptors from boot (its timer thread's eventfd and epoll fd): taking two here
    // gives a program the same fd numbers (IO.pipe is [5, 6] in both)
    let mut held = [0; 2];
    sys::pipe2(&mut held, sys::O_CLOEXEC);
    let io = define_class(c, "IO", vmr.c_object.get());
    class_set_alloc(c, io, AllocKind::None);
    include_module(c, io, vmr.m_enumerable.get());
    vmr.c_io.set(io);
    let f = define_class(c, "File", io);
    vmr.c_file.set(f);
    let d = |c: &mut Ctx, n: &str, fun: CFunc, a: i32| define_cfunc(c, io, n, fun, a);
    d(c, "write", io_write, -1); d(c, "puts", io_puts, -1); d(c, "print", io_print, -1); d(c, "<<", io_ltlt, 1); d(c, "printf", io_printf, -1); d(c, "p", io_p, -1); d(c, "putc", io_putc, 1);
    d(c, "flush", io_flush, 0); d(c, "fsync", io_fsync, 0); d(c, "fdatasync", io_fsync, 0); d(c, "sync", io_sync, 0); d(c, "sync=", io_set_sync, 1); d(c, "gets", io_gets, -1); d(c, "readline", io_readline, -1);
    d(c, "each_line", io_each_line, -1); d(c, "each", io_each_line, -1); d(c, "readlines", io_readlines, -1); d(c, "read", io_read, -1); d(c, "getc", io_getc, 0);
    d(c, "readchar", io_readchar, 0); d(c, "getbyte", io_getbyte, 0); d(c, "readbyte", io_readbyte, 0); d(c, "each_byte", io_each_byte, 0); d(c, "each_char", io_each_char, 0);
    d(c, "ungetc", io_ungetc, 1); d(c, "ungetbyte", io_ungetbyte, 1); d(c, "readpartial", io_readpartial, -1); d(c, "read_nonblock", io_read_nonblock, -1);
    d(c, "write_nonblock", io_write_nonblock, -1); d(c, "sysread", io_sysread, -1); d(c, "syswrite", io_syswrite, 1); d(c, "pread", io_pread, -1); d(c, "pwrite", io_pwrite, 2);
    d(c, "eof?", io_eof, 0); d(c, "eof", io_eof, 0); d(c, "close", io_close, 0); d(c, "close_read", io_close_read, 0); d(c, "close_write", io_close_write, 0);
    d(c, "closed?", io_closed, 0); d(c, "fileno", io_fileno, 0); d(c, "to_i", io_fileno, 0);
    d(c, "tty?", io_tty, 0); d(c, "isatty", io_tty, 0); d(c, "path", io_path, 0); d(c, "to_path", io_path, 0); d(c, "binmode", io_binmode, 0); d(c, "binmode?", io_binmode_p, 0); d(c, "inspect", io_inspect, 0);
    d(c, "lineno", io_lineno, 0); d(c, "lineno=", io_set_lineno, 1); d(c, "rewind", io_rewind, 0); d(c, "seek", io_seek, -1); d(c, "sysseek", io_sysseek, -1);
    d(c, "pos", io_pos, 0); d(c, "tell", io_pos, 0); d(c, "pos=", io_set_pos, 1); d(c, "pid", io_pid, 0); d(c, "to_io", io_to_io, 0);
    d(c, "external_encoding", io_ext_enc, 0); d(c, "internal_encoding", io_int_enc, 0); d(c, "set_encoding", io_set_encoding, -1);
    d(c, "set_encoding_by_bom", io_set_encoding_by_bom, 0);
    d(c, "autoclose?", io_autoclose_p, 0); d(c, "autoclose=", io_set_autoclose, 1); d(c, "close_on_exec?", io_cloexec_p, 0); d(c, "close_on_exec=", io_set_cloexec, 1);
    d(c, "fcntl", io_fcntl, -1); d(c, "reopen", io_reopen, -1); d(c, "wait_readable", io_wait_readable, -1); d(c, "wait_writable", io_wait_writable, -1);
    d(c, "ready?", io_ready_p, 0); d(c, "nread", io_nread, 0); d(c, "timeout", io_get_timeout, 0); d(c, "timeout=", io_set_timeout, 1);
    define_private_cfunc(c, io, "initialize", io_initialize, -1);
    define_private_cfunc(c, io, "initialize_copy", io_init_copy, 1);
    define_private_cfunc(c, f, "initialize", file_initialize, -1);
    let ds = |c: &mut Ctx, n: &str, fun: CFunc, a: i32| define_singleton_cfunc(c, io, n, fun, a);
    ds(c, "open", io_s_open, -1); ds(c, "for_fd", crate::builtins::object::c_new, -1); ds(c, "sysopen", io_s_sysopen, -1); ds(c, "pipe", io_s_pipe, -1); ds(c, "popen", io_s_popen, -1);
    ds(c, "select", io_s_select, -1); ds(c, "read", io_s_read, -1); ds(c, "binread", io_s_binread, -1); ds(c, "readlines", io_s_readlines, -1); ds(c, "foreach", io_s_foreach, -1);
    ds(c, "write", io_s_write, -1); ds(c, "binwrite", io_s_binwrite, -1); ds(c, "copy_stream", io_s_copy_stream, -1); ds(c, "try_convert", io_s_try_convert, 1);
    define_module_function(c, vmr.m_kernel.get(), "select", io_s_select, -1);
    define_module_function(c, vmr.m_kernel.get(), "open", k_open, -1);
    for (n, v) in [("SEEK_SET", 0), ("SEEK_CUR", 1), ("SEEK_END", 2), ("SEEK_DATA", 3), ("SEEK_HOLE", 4), ("READABLE", 1), ("PRIORITY", 2), ("WRITABLE", 4)] { const_set(c, io, intern(n.as_bytes()), Value::fix(v)); }
    let wr = define_module_under(c, io, "WaitReadable");
    let ww = define_module_under(c, io, "WaitWritable");
    let eagain = crate::builtins::exception::errno_class(c, 11);
    const_set(c, vmr.m_errno.get(), intern(b"EWOULDBLOCK"), eagain);
    let e1 = define_class_under(c, io, "EAGAINWaitReadable", eagain); include_module(c, e1, wr);
    let e2 = define_class_under(c, io, "EAGAINWaitWritable", eagain); include_module(c, e2, ww);
    const_set(c, io, intern(b"EWOULDBLOCKWaitReadable"), e1);
    const_set(c, io, intern(b"EWOULDBLOCKWaitWritable"), e2);
    crate::builtins::file::init(c);
    let so = crate::gc::alloc(io, Body::Data(Box::new(opened(1, b"<STDOUT>", false, true))));
    let mut e = opened(2, b"<STDERR>", false, true); e.sync = true;
    let se = crate::gc::alloc(io, Body::Data(Box::new(e)));
    let si = crate::gc::alloc(io, Body::Data(Box::new(opened(0, b"<STDIN>", true, false))));
    vmr.stdout.set(so); vmr.stderr.set(se); vmr.stdin.set(si);
    const_set(c, vmr.c_object.get(), intern(b"STDOUT"), so);
    const_set(c, vmr.c_object.get(), intern(b"STDERR"), se);
    const_set(c, vmr.c_object.get(), intern(b"STDIN"), si);
    vmr.globals.borrow_mut().insert(intern(b"$stdout"), so);
    vmr.globals.borrow_mut().insert(intern(b"$stderr"), se);
    vmr.globals.borrow_mut().insert(intern(b"$stdin"), si);
    for (a, b) in crate::builtins::kernel::GVAR_SYNONYMS { vmr.global_aliases.borrow_mut().insert(crate::symbol::intern_str(a), crate::symbol::intern_str(b)); }
    let rs = boot_frozen_str(vmr, b"\n");
    vmr.globals.borrow_mut().insert(intern(b"$/"), rs);
    let _ = s::puts;
}
