// File (class methods, constants), File::Stat, FileTest, Dir, fnmatch / glob.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::array::*;
use crate::builtins::exception::raise_errno;
use crate::builtins::io::{path_arg, rio, PathArg};
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::builtins::sys;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicI32, Ordering::Relaxed};

pub struct RStat { pub md: Option<std::fs::Metadata> }
impl RData for RStat {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn io_errno(e: &std::io::Error) -> i64 { e.raw_os_error().map(|x| x as i64).unwrap_or(5) }
fn os_err(c: &mut Ctx, e: &std::io::Error, detail: &str) -> Esc { raise_errno(c, io_errno(e), detail) }
fn os_err_at(c: &mut Ctx, e: &std::io::Error, pre: &str, p: &[u8]) -> Esc { crate::builtins::io::raise_errno_at(c, io_errno(e), pre, p) }
fn last_err_at(c: &mut Ctx, pre: &str, p: &[u8]) -> Esc { let e = sys::errno(); crate::builtins::io::raise_errno_at(c, e, pre, p) }
/// "(x, y)" of two paths, for the messages of rename / link
fn pair(x: &PathArg, y: &PathArg) -> Vec<u8> { let mut v = b"(".to_vec(); v.extend_from_slice(x.as_bytes()); v.extend_from_slice(b", "); v.extend_from_slice(y.as_bytes()); v.push(b')'); v }
fn new_stat(c: &mut Ctx, md: std::fs::Metadata) -> Value { init_stat(c); c.alloc(c.vm().c_stat.get(), Body::Data(Box::new(RStat { md: Some(md) }))) }
fn fd_meta(fd: i32) -> std::io::Result<std::fs::Metadata> {
    sys::fd_metadata(fd)
}
/// Metadata of a path or an IO argument.
fn meta_of(c: &mut Ctx, v: Value, follow: bool) -> Result<Result<std::fs::Metadata, (std::io::Error, PathArg)>, Esc> {
    if let Some(io) = rio(c, v) {
        if io.closed { return err(c, c.vm().c_io_error.get(), "closed stream"); }
        return Ok(fd_meta(io.fd).map_err(|e| (e, PathArg(io.path.clone()))));
    }
    let p = path_arg(c, v)?;
    Ok(if follow { std::fs::metadata(&p) } else { std::fs::symlink_metadata(&p) }.map_err(|e| (e, p)))
}
/// The Metadata of a File::Stat (a copy: the payload is not borrowed across what follows).
fn stat_md(c: &mut Ctx, v: Value) -> Result<std::fs::Metadata, Esc> {
    match c.data::<RStat>(v).and_then(|s| s.md.clone()) { Some(m) => Ok(m), None => type_err(c, "uninitialized File::Stat") }
}
/// A local Time at sec + nsec.
fn time_of(c: &mut Ctx, sec: i64, nsec: i64) -> Value {
    use crate::builtins::time::{RTime, LOCAL};
    let w = c.int_from_i128(sec as i128 * 1_000_000_000 + nsec as i128);
    c.alloc_with(c.vm().c_time.get(), move || Body::Data(Box::new(RTime { w, mode: LOCAL, off: NIL, zone: NIL, dst: false })))
}
fn access_ok(md: &std::fs::Metadata, bit: u32, effective: bool) -> bool {
    let (uid, gid) = if effective { (sys::geteuid(), sys::getegid()) } else { (sys::getuid(), sys::getgid()) };
    if uid == 0 { return bit != 1 || md.mode() & 0o111 != 0 || md.is_dir(); }
    let m = md.mode();
    if md.uid() == uid { m & (bit << 6) != 0 } else if md.gid() == gid { m & (bit << 3) != 0 } else { m & bit != 0 }
}
fn ftype(md: &std::fs::Metadata) -> &'static str {
    match md.mode() & 0o170000 { 0o100000 => "file", 0o040000 => "directory", 0o120000 => "link", 0o020000 => "characterSpecial", 0o060000 => "blockSpecial", 0o010000 => "fifo", 0o140000 => "socket", _ => "unknown" }
}

macro_rules! stat_fn { ($name:ident, |$c:ident, $m:ident| $e:expr) => { fn $name($c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let md = stat_md($c, s)?; let $m = &md; Ok($e) } }; }
stat_fn!(st_size, |c, m| Value::fix(m.len() as i64));
stat_fn!(st_mode, |c, m| Value::fix(m.mode() as i64));
stat_fn!(st_uid, |c, m| Value::fix(m.uid() as i64));
stat_fn!(st_gid, |c, m| Value::fix(m.gid() as i64));
stat_fn!(st_ino, |c, m| Value::fix(m.ino() as i64));
stat_fn!(st_dev, |c, m| Value::fix(m.dev() as i64));
stat_fn!(st_dev_major, |c, m| Value::fix(((m.dev() >> 8) & 0xfff) as i64));
stat_fn!(st_dev_minor, |c, m| Value::fix(((m.dev() & 0xff) | ((m.dev() >> 12) & 0xfff00)) as i64));
stat_fn!(st_rdev, |c, m| Value::fix(m.rdev() as i64));
stat_fn!(st_nlink, |c, m| Value::fix(m.nlink() as i64));
stat_fn!(st_blksize, |c, m| Value::fix(m.blksize() as i64));
stat_fn!(st_blocks, |c, m| Value::fix(m.blocks() as i64));
stat_fn!(st_atime, |c, m| time_of(c, m.atime(), m.atime_nsec()));
stat_fn!(st_mtime, |c, m| time_of(c, m.mtime(), m.mtime_nsec()));
stat_fn!(st_ctime, |c, m| time_of(c, m.ctime(), m.ctime_nsec()));
stat_fn!(st_file, |c, m| Value::bool(m.is_file()));
stat_fn!(st_dir, |c, m| Value::bool(m.is_dir()));
stat_fn!(st_symlink, |c, m| Value::bool(m.file_type().is_symlink()));
stat_fn!(st_pipe, |c, m| Value::bool(m.mode() & 0o170000 == 0o010000));
stat_fn!(st_socket, |c, m| Value::bool(m.mode() & 0o170000 == 0o140000));
stat_fn!(st_chardev, |c, m| Value::bool(m.mode() & 0o170000 == 0o020000));
stat_fn!(st_blockdev, |c, m| Value::bool(m.mode() & 0o170000 == 0o060000));
stat_fn!(st_readable, |c, m| Value::bool(access_ok(m, 4, true)));
stat_fn!(st_writable, |c, m| Value::bool(access_ok(m, 2, true)));
stat_fn!(st_executable, |c, m| Value::bool(access_ok(m, 1, true)));
stat_fn!(st_readable_real, |c, m| Value::bool(access_ok(m, 4, false)));
stat_fn!(st_writable_real, |c, m| Value::bool(access_ok(m, 2, false)));
stat_fn!(st_executable_real, |c, m| Value::bool(access_ok(m, 1, false)));
stat_fn!(st_world_readable, |c, m| if m.mode() & 4 != 0 { Value::fix((m.mode() & 0o777) as i64) } else { NIL });
stat_fn!(st_world_writable, |c, m| if m.mode() & 2 != 0 { Value::fix((m.mode() & 0o777) as i64) } else { NIL });
stat_fn!(st_zero, |c, m| Value::bool(m.len() == 0));
stat_fn!(st_size_p, |c, m| if m.len() == 0 { NIL } else { Value::fix(m.len() as i64) });
stat_fn!(st_ftype, |c, m| c.str_new(ftype(m).as_bytes()));
stat_fn!(st_setuid, |c, m| Value::bool(m.mode() & 0o4000 != 0));
stat_fn!(st_setgid, |c, m| Value::bool(m.mode() & 0o2000 != 0));
stat_fn!(st_sticky, |c, m| Value::bool(m.mode() & 0o1000 != 0));
stat_fn!(st_owned, |c, m| Value::bool(m.uid() == sys::geteuid()));
stat_fn!(st_grpowned, |c, m| Value::bool(m.gid() == sys::getegid()));
fn st_birthtime(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let m = stat_md(c, s)?;
    birthtime(c, &m)
}
fn st_cmp(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let m = stat_md(c, s)?;
    let Some(o) = c.data::<RStat>(a[0]).and_then(|s| s.md.as_ref()) else { return Ok(NIL) };
    Ok(Value::fix((m.mtime(), m.mtime_nsec()).cmp(&(o.mtime(), o.mtime_nsec())) as i64))
}
fn st_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let k = class_path(c, real_class_of(c, s));
    let Some(m) = c.data::<RStat>(s).and_then(|s| s.md.clone()) else { return Ok(c.str_new(format!("#<{}: uninitialized>", k).as_bytes())) };
    let mut t = vec![];
    for (sec, ns) in [(m.atime(), m.atime_nsec()), (m.mtime(), m.mtime_nsec()), (m.ctime(), m.ctime_nsec())] { let tv = time_of(c, sec, ns); t.push(inspect_str(c, tv)?); }
    Ok(c.str_new(format!("#<{} dev=0x{:x}, ino={}, mode=0{:o}, nlink={}, uid={}, gid={}, rdev=0x{:x}, size={}, blksize={}, blocks={}, atime={}, mtime={}, ctime={}>",
        k, m.dev(), m.ino(), m.mode(), m.nlink(), m.uid(), m.gid(), m.rdev(), m.len(), m.blksize(), m.blocks(), t[0], t[1], t[2]).as_bytes()))
}
fn st_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?;
    match std::fs::metadata(&p) { Ok(m) => { if let Some(st) = c.data_mut::<RStat>(s) { st.md = Some(m); } Ok(NIL) } Err(e) => Err(os_err_at(c, &e, "rb_file_s_stat - ", p.as_bytes())) }
}
fn st_init_copy(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    if let Some(md) = c.data::<RStat>(a[0]).map(|o| o.md.clone()) { if let Some(st) = c.data_mut::<RStat>(s) { st.md = md; } }
    Ok(s)
}
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let st = c.vm().c_stat.get();
    if st != NIL && crate::builtins::inherits(c, klass, st) { return Ok(Some(c.alloc(klass, Body::Data(Box::new(RStat { md: None }))))); }
    Ok(None)
}

// ---- File class methods -----------------------------------------------------------------------

fn f_s_stat(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    match meta_of(c, a[0], true)? { Ok(m) => Ok(new_stat(c, m)), Err((e, p)) => Err(os_err_at(c, &e, "rb_file_s_stat - ", p.as_bytes())) }
}
fn f_s_lstat(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    match meta_of(c, a[0], false)? { Ok(m) => Ok(new_stat(c, m)), Err((e, p)) => Err(os_err_at(c, &e, "rb_file_s_lstat - ", p.as_bytes())) }
}
fn f_stat(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let fd = crate::builtins::io::get_io(c, s)?.fd;
    match fd_meta(fd) { Ok(m) => Ok(new_stat(c, m)), Err(e) => Err(os_err(c, &e, "")) }
}
fn f_lstat(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let p = PathArg(crate::builtins::io::get_io(c, s)?.path.clone());
    match std::fs::symlink_metadata(&p) { Ok(m) => Ok(new_stat(c, m)), Err(e) => Err(os_err_at(c, &e, "rb_file_lstat - ", p.as_bytes())) }
}
macro_rules! test_fn {
    ($name:ident, $follow:expr, |$m:ident| $e:expr) => {
        fn $name(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { Ok(match meta_of(c, a[0], $follow)? { Ok($m) => $e, Err(_) => FALSE }) }
    };
}
test_fn!(f_s_exist, true, |_m| TRUE);
test_fn!(f_s_file_p, true, |m| Value::bool(m.is_file()));
test_fn!(f_s_dir_p, true, |m| Value::bool(m.is_dir()));
test_fn!(f_s_symlink_p, false, |m| Value::bool(m.file_type().is_symlink()));
test_fn!(f_s_pipe_p, true, |m| Value::bool(m.mode() & 0o170000 == 0o010000));
test_fn!(f_s_socket_p, true, |m| Value::bool(m.mode() & 0o170000 == 0o140000));
test_fn!(f_s_chardev_p, true, |m| Value::bool(m.mode() & 0o170000 == 0o020000));
test_fn!(f_s_blockdev_p, true, |m| Value::bool(m.mode() & 0o170000 == 0o060000));
test_fn!(f_s_readable, true, |m| Value::bool(access_ok(&m, 4, true)));
test_fn!(f_s_writable, true, |m| Value::bool(access_ok(&m, 2, true)));
test_fn!(f_s_executable, true, |m| Value::bool(access_ok(&m, 1, true)));
test_fn!(f_s_readable_real, true, |m| Value::bool(access_ok(&m, 4, false)));
test_fn!(f_s_writable_real, true, |m| Value::bool(access_ok(&m, 2, false)));
test_fn!(f_s_executable_real, true, |m| Value::bool(access_ok(&m, 1, false)));
test_fn!(f_s_zero, true, |m| Value::bool(m.len() == 0));
test_fn!(f_s_setuid, true, |m| Value::bool(m.mode() & 0o4000 != 0));
test_fn!(f_s_setgid, true, |m| Value::bool(m.mode() & 0o2000 != 0));
test_fn!(f_s_sticky, true, |m| Value::bool(m.mode() & 0o1000 != 0));
test_fn!(f_s_owned, true, |m| Value::bool(m.uid() == sys::geteuid()));
test_fn!(f_s_grpowned, true, |m| Value::bool(m.gid() == sys::getegid()));
fn f_s_size_p(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { Ok(match meta_of(c, a[0], true)? { Ok(m) if m.len() > 0 => Value::fix(m.len() as i64), _ => NIL }) }
fn f_s_world_readable(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { Ok(match meta_of(c, a[0], true)? { Ok(m) if m.mode() & 4 != 0 => Value::fix((m.mode() & 0o777) as i64), _ => NIL }) }
fn f_s_world_writable(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { Ok(match meta_of(c, a[0], true)? { Ok(m) if m.mode() & 2 != 0 => Value::fix((m.mode() & 0o777) as i64), _ => NIL }) }
fn f_s_identical(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    Ok(match (meta_of(c, a[0], true)?, meta_of(c, a[1], true)?) { (Ok(x), Ok(y)) => Value::bool(x.dev() == y.dev() && x.ino() == y.ino()), _ => FALSE })
}
macro_rules! meta_fn {
    ($name:ident, $what:expr, |$c:ident, $m:ident| $e:expr) => {
        fn $name($c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { match meta_of($c, a[0], true)? { Ok($m) => Ok($e), Err((e, p)) => Err(os_err($c, &e, &format!("{} - {}", $what, p))) } }
    };
}
meta_fn!(f_s_size, "rb_file_s_size", |c, m| Value::fix(m.len() as i64));
meta_fn!(f_s_mtime, "rb_file_s_mtime", |c, m| time_of(c, m.mtime(), m.mtime_nsec()));
meta_fn!(f_s_atime, "rb_file_s_atime", |c, m| time_of(c, m.atime(), m.atime_nsec()));
meta_fn!(f_s_ctime, "rb_file_s_ctime", |c, m| time_of(c, m.ctime(), m.ctime_nsec()));
fn f_s_birthtime(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    match meta_of(c, a[0], true)? {
        Ok(m) => birthtime(c, &m),
        Err((e, p)) => Err(os_err_at(c, &e, "rb_file_s_birthtime - ", p.as_bytes())),
    }
}
fn birthtime(c: &mut Ctx, m: &std::fs::Metadata) -> RV {
    match m.created().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()) { Some(d) => Ok(time_of(c, d.as_secs() as i64, d.subsec_nanos() as i64)), None => err(c, c.vm().c_not_impl_error.get(), "birthtime() function is unimplemented on this machine") }
}
fn f_s_ftype(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    match meta_of(c, a[0], false)? { Ok(m) => Ok(c.str_new(ftype(&m).as_bytes())), Err((e, p)) => Err(os_err_at(c, &e, "rb_file_s_ftype - ", p.as_bytes())) }
}
fn f_s_delete(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    for &x in a { let p = path_arg(c, x)?; if let Err(e) = std::fs::remove_file(&p) { return Err(os_err_at(c, &e, "rb_file_s_unlink - ", p.as_bytes())); } }
    Ok(Value::fix(a.len() as i64))
}
fn f_s_rename(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let (x, y) = (path_arg(c, a[0])?, path_arg(c, a[1])?);
    match std::fs::rename(&x, &y) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err_at(c, &e, "rb_file_s_rename - ", &pair(&x, &y))) }
}
fn f_s_symlink(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let (x, y) = (path_arg(c, a[0])?, path_arg(c, a[1])?);
    match std::os::unix::fs::symlink(&x, &y) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err_at(c, &e, "rb_file_s_symlink - ", &pair(&x, &y))) }
}
fn f_s_link(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let (x, y) = (path_arg(c, a[0])?, path_arg(c, a[1])?);
    match std::fs::hard_link(&x, &y) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err_at(c, &e, "rb_file_s_link - ", &pair(&x, &y))) }
}
fn f_s_readlink(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?;
    match std::fs::read_link(&p) { Ok(t) => Ok(c.str_new(t.as_os_str().as_bytes())), Err(e) => Err(os_err_at(c, &e, "rb_readlink - ", p.as_bytes())) }
}
fn mode_arg(c: &mut Ctx, v: Value) -> Result<u32, Esc> { Ok(num_to_long(c, v)? as u32) }
fn f_s_chmod(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let m = mode_arg(c, a[0])?;
    for &x in &a[1..] { let p = path_arg(c, x)?; if sys::chmod(&p.cstr(), m) < 0 { return Err(last_err_at(c, "rb_file_s_chmod - ", p.as_bytes())); } }
    Ok(Value::fix(a.len() as i64 - 1))
}
fn id_arg(c: &mut Ctx, v: Value) -> Result<u32, Esc> { if v == NIL { Ok(u32::MAX) } else { Ok(num_to_long(c, v)? as u32) } }
fn f_s_chown(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.len() < 2 { return argc_err(c, a.len(), 2, -1); }
    let (u, g) = (id_arg(c, a[0])?, id_arg(c, a[1])?);
    for &x in &a[2..] { let p = path_arg(c, x)?; if sys::chown(&p.cstr(), u, g) < 0 { return Err(last_err_at(c, "rb_file_s_chown - ", p.as_bytes())); } }
    Ok(Value::fix(a.len() as i64 - 2))
}
fn f_s_lchown(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.len() < 2 { return argc_err(c, a.len(), 2, -1); }
    let (u, g) = (id_arg(c, a[0])?, id_arg(c, a[1])?);
    for &x in &a[2..] { let p = path_arg(c, x)?; if sys::lchown(&p.cstr(), u, g) < 0 { return Err(last_err_at(c, "rb_file_s_lchown - ", p.as_bytes())); } }
    Ok(Value::fix(a.len() as i64 - 2))
}
fn f_s_lchmod(c: &mut Ctx, _slf: Value, _a: &[Value], _b: Blk) -> RV { err(c, c.vm().c_not_impl_error.get(), "lchmod() function is unimplemented on this machine") }
fn f_s_truncate(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?; let n = num_to_long(c, a[1])?;
    if sys::truncate(&p.cstr(), n) < 0 { return Err(last_err_at(c, "rb_file_s_truncate - ", p.as_bytes())); }
    Ok(Value::fix(0))
}
fn f_s_umask(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    if let Some(&m) = a.first() { let m = mode_arg(c, m)?; return Ok(Value::fix(sys::umask(m) as i64)); }
    let old = sys::umask(0); sys::umask(old);
    Ok(Value::fix(old as i64))
}
fn f_s_mkfifo(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?;
    let m = match a.get(1) { Some(&m) => mode_arg(c, m)?, None => 0o666 };
    if sys::mkfifo(&p.cstr(), m) < 0 { return Err(last_err_at(c, "rb_file_s_mkfifo - ", p.as_bytes())); }
    Ok(Value::fix(0))
}
fn timespec_arg(c: &mut Ctx, v: Value) -> Result<sys::Timespec, Esc> {
    if v == NIL { return Ok(sys::Timespec { sec: 0, nsec: (1 << 30) - 1 }); } // UTIME_NOW
    if let Some((s, ns)) = crate::builtins::time::time_to_ns(c, v) { return Ok(sys::Timespec { sec: s, nsec: ns }); }
    let f = crate::builtins::numeric_methods::to_f(c, v)?;
    Ok(sys::Timespec { sec: f.floor() as i64, nsec: ((f - f.floor()) * 1e9) as i64 })
}
fn utime_impl(c: &mut Ctx, a: &[Value], nofollow: bool) -> RV {
    if a.len() < 2 { return argc_err(c, a.len(), 2, -1); }
    let ts = [timespec_arg(c, a[0])?, timespec_arg(c, a[1])?];
    for &x in &a[2..] { let p = path_arg(c, x)?; if sys::utimensat_cwd(&p.cstr(), &ts, if nofollow { 0x100 } else { 0 }) < 0 { return Err(last_err_at(c, "rb_file_s_utime - ", p.as_bytes())); } }
    Ok(Value::fix(a.len() as i64 - 2))
}
fn f_s_utime(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { utime_impl(c, a, false) }
fn f_s_lutime(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { utime_impl(c, a, true) }

// paths are bytes (CRuby takes any bytes but NUL)
fn trim_end(b: &[u8]) -> &[u8] { let mut n = b.len(); while n > 0 && b[n - 1] == b'/' { n -= 1; } &b[..n] }
fn trim_start(b: &[u8]) -> &[u8] { let n = b.iter().position(|&x| x != b'/').unwrap_or(b.len()); &b[n..] }
fn last_comp(b: &[u8]) -> &[u8] { match b.iter().rposition(|&x| x == b'/') { Some(i) => &b[i + 1..], None => b } }
fn str_like(c: &mut Ctx, src: Value, b: &[u8]) -> Value { if is_string(src) { let e = c.str_enc(src); c.str_new_enc(b, e) } else { c.str_new(b) } }
fn f_s_basename(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let p = path_arg(c, a[0])?.0;
    let t = trim_end(&p);
    let mut base = if t.is_empty() && p.starts_with(b"/") { b"/".to_vec() } else { last_comp(t).to_vec() };
    if a.len() > 1 {
        let suf = path_arg(c, a[1])?.0;
        if suf == b".*" { if let Some(i) = base.iter().rposition(|&x| x == b'.') { if i > 0 { base.truncate(i); } } }
        else if base.ends_with(&suf) && base != suf { base.truncate(base.len() - suf.len()); }
    }
    Ok(str_like(c, a[0], &base))
}
fn dirname_of(p: &[u8]) -> Vec<u8> {
    let t = trim_end(p);
    let d: &[u8] = match t.iter().rposition(|&x| x == b'/') { Some(i) => { let d = trim_end(&t[..i]); if d.is_empty() { b"/" } else { d } } None => if p.starts_with(b"/") { b"/" } else { b"." } };
    // a run of leading slashes is one root
    if d.starts_with(b"//") { let mut o = b"/".to_vec(); o.extend_from_slice(trim_start(d)); o } else { d.to_vec() }
}
fn f_s_dirname(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let mut p = path_arg(c, a[0])?.0;
    let n = match a.get(1) { Some(&l) => { let n = num_to_long(c, l)?; if n < 0 { return arg_err(c, format!("negative level: {}", n)); } n } None => 1 };
    for _ in 0..n { let d = dirname_of(&p); if d == p { break; } p = d; }
    Ok(str_like(c, a[0], &p))
}
fn f_s_extname(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?.0;
    let base = last_comp(trim_end(&p));
    let stem = &base[base.iter().position(|&x| x != b'.').unwrap_or(base.len())..];
    let e: &[u8] = match stem.iter().rposition(|&x| x == b'.') { Some(i) if i + 1 < stem.len() => &stem[i..], Some(i) if i > 0 => b".", _ => b"" };
    Ok(str_like(c, a[0], e))
}
fn f_s_split(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let d = f_s_dirname(c, slf, &a[..1], b)?;
    let bn = f_s_basename(c, slf, &a[..1], b)?;
    Ok(c.ary_new_from(&[d, bn]))
}
fn f_s_join(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut parts: Vec<Vec<u8>> = vec![];
    fn flat(c: &mut Ctx, v: Value, out: &mut Vec<Vec<u8>>, depth: usize) -> Result<(), Esc> {
        if depth > 1000 { return arg_err(c, "recursive array"); }
        if is_array(v) {
            let mut i = 0;
            while i < c.ary_len(v) { let x = c.ary_get(v, i); i += 1; if x == v { return arg_err(c, "recursive array"); } flat(c, x, out, depth + 1)?; }
            Ok(())
        } else { out.push(path_arg(c, v)?.0); Ok(()) }
    }
    for &x in a { flat(c, x, &mut parts, 0)?; }
    let mut out: Vec<u8> = vec![];
    for (i, p) in parts.iter().enumerate() {
        if i == 0 { out.extend_from_slice(p); continue; }
        match (out.ends_with(b"/"), p.starts_with(b"/")) { (true, true) => { while out.ends_with(b"/") { out.pop(); } out.extend_from_slice(p) } (false, false) => { out.push(b'/'); out.extend_from_slice(p); } _ => out.extend_from_slice(p) }
    }
    Ok(c.str_new(&out))
}
fn home_of(c: &mut Ctx, user: &str) -> Result<Vec<u8>, Esc> {
    if user.is_empty() { return match std::env::var_os("HOME") { Some(h) => Ok(h.into_vec()), None => arg_err(c, "couldn't find HOME environment -- expanding '~'") }; }
    if let Ok(pw) = std::fs::read_to_string("/etc/passwd") { for l in pw.lines() { let f: Vec<&str> = l.split(':').collect(); if f.len() > 5 && f[0] == user { return Ok(f[5].as_bytes().to_vec()); } } }
    arg_err(c, format!("user {} doesn't exist", user))
}
fn cwd() -> Vec<u8> { std::env::current_dir().map(|d| d.into_os_string().into_vec()).unwrap_or(b"/".to_vec()) }
pub(crate) fn expand(c: &mut Ctx, a: &[Value], tilde: bool) -> Result<Vec<u8>, Esc> {
    let mut p = path_arg(c, a[0])?.0;
    if tilde && p.starts_with(b"~") {
        let end = p.iter().position(|&x| x == b'/').unwrap_or(p.len());
        let user = String::from_utf8_lossy(&p[1..end]).into_owned();
        let mut h = home_of(c, &user)?;
        if !h.starts_with(b"/") { return arg_err(c, "non-absolute home"); }
        h.extend_from_slice(&p[end..]);
        p = h;
    }
    let full = if p.starts_with(b"/") { p } else {
        let mut base = if a.len() > 1 && a[1] != NIL { expand(c, &a[1..2], tilde)? } else { cwd() };
        base.push(b'/'); base.extend_from_slice(&p);
        base
    };
    let mut comps: Vec<&[u8]> = vec![];
    for x in full.split(|&b| b == b'/') { match x { b"" | b"." => {} b".." => { comps.pop(); } y => comps.push(y) } }
    let mut out = b"/".to_vec(); out.extend_from_slice(&comps.join(&b'/'));
    Ok(out)
}
fn f_s_expand_path(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { check_argc(c, a, 1, 2)?; let r = expand(c, a, true)?; Ok(c.str_new(&r)) }
fn f_s_absolute_path(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { check_argc(c, a, 1, 2)?; let r = expand(c, a, false)?; Ok(c.str_new(&r)) }
fn f_s_absolute_path_p(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { let p = path_arg(c, a[0])?; Ok(Value::bool(p.as_bytes().starts_with(b"/"))) }
fn realpath_impl(c: &mut Ctx, a: &[Value], strict: bool) -> RV {
    check_argc(c, a, 1, 2)?;
    let p = PathArg(expand(c, a, false)?);
    match std::fs::canonicalize(&p) {
        Ok(r) => Ok(c.str_new(r.as_os_str().as_bytes())),
        Err(e) => {
            if !strict { // the last component may be missing
                let d = PathArg(dirname_of(p.as_bytes()));
                if let Ok(r) = std::fs::canonicalize(&d) {
                    let mut o = trim_end(r.as_os_str().as_bytes()).to_vec(); o.push(b'/'); o.extend_from_slice(last_comp(p.as_bytes()));
                    return Ok(c.str_new(&o));
                }
            }
            Err(os_err_at(c, &e, "rb_check_realpath_internal - ", p.as_bytes()))
        }
    }
}
fn f_s_realpath(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { realpath_impl(c, a, true) }
fn f_s_realdirpath(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { realpath_impl(c, a, false) }
fn f_s_path(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { let p = path_arg(c, a[0])?; Ok(str_like(c, a[0], p.as_bytes())) }
fn f_s_open(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV { crate::builtins::io::open_with_block(c, slf, a, blk) }

// ---- File instance methods --------------------------------------------------------------------

fn f_size(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    funcall(c, s, intern(b"flush"), &[], std::ptr::null())?;
    let fd = crate::builtins::io::get_io(c, s)?.fd;
    match fd_meta(fd) { Ok(m) => Ok(Value::fix(m.len() as i64)), Err(e) => Err(os_err(c, &e, "")) }
}
fn f_truncate(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let n = num_to_long(c, a[0])?;
    funcall(c, s, intern(b"flush"), &[], std::ptr::null())?;
    let io = crate::builtins::io::get_io(c, s)?;
    if !io.writable { return err(c, c.vm().c_io_error.get(), "not opened for writing"); }
    if sys::ftruncate(io.fd, n) < 0 { let p = io.path.clone(); return Err(last_err_at(c, "", &p)); }
    Ok(Value::fix(0))
}
fn f_flock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let op = num_to_long(c, a[0])? as i32;
    funcall(c, s, intern(b"flush"), &[], std::ptr::null())?;
    // with other threads, a lock that would block is retried after a back-off sleep (no fd to wait on)
    let coop = crate::builtins::thread::active(c) && op & 4 == 0;
    let mut nap = std::time::Duration::from_millis(1);
    loop {
        let io = crate::builtins::io::get_io(c, s)?;
        if sys::flock(io.fd, if coop { op | 4 } else { op }) == 0 { return Ok(Value::fix(0)); }
        match sys::errno() {
            4 => continue,
            11 if coop => { crate::builtins::thread::sleep_for(c, nap)?; nap = (nap * 2).min(std::time::Duration::from_millis(50)); }
            11 if op & 4 != 0 => return Ok(FALSE),
            _ => { let p = io.path.clone(); return Err(last_err_at(c, "", &p)); }
        }
    }
}
fn f_chmod(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let m = mode_arg(c, a[0])?;
    let io = crate::builtins::io::get_io(c, s)?;
    if sys::fchmod(io.fd, m) < 0 { let p = io.path.clone(); return Err(last_err_at(c, "", &p)); }
    Ok(Value::fix(0))
}
fn f_chown(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (u, g) = (id_arg(c, a[0])?, id_arg(c, a[1])?);
    let io = crate::builtins::io::get_io(c, s)?;
    if sys::fchown(io.fd, u, g) < 0 { let p = io.path.clone(); return Err(last_err_at(c, "", &p)); }
    Ok(Value::fix(0))
}
macro_rules! fmeta_fn {
    ($name:ident, |$c:ident, $m:ident| $e:expr) => {
        fn $name($c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let fd = crate::builtins::io::get_io($c, s)?.fd; match fd_meta(fd) { Ok($m) => Ok($e), Err(e) => Err(os_err($c, &e, "")) } }
    };
}
fmeta_fn!(f_mtime, |c, m| time_of(c, m.mtime(), m.mtime_nsec()));
fmeta_fn!(f_atime, |c, m| time_of(c, m.atime(), m.atime_nsec()));
fmeta_fn!(f_ctime, |c, m| time_of(c, m.ctime(), m.ctime_nsec()));

// ---- fnmatch / glob ---------------------------------------------------------------------------

const FNM_NOESCAPE: i64 = 1; const FNM_PATHNAME: i64 = 2; const FNM_DOTMATCH: i64 = 4; const FNM_CASEFOLD: i64 = 8; const FNM_EXTGLOB: i64 = 16;

fn eq_ch(a: u8, b: u8, fold: bool) -> bool { a == b || (fold && a.eq_ignore_ascii_case(&b)) }
/// Match a bracket expression at p (after '['); returns (matched, rest of pattern).
fn bracket<'a>(p: &'a [u8], ch: &[u8], flags: i64) -> Option<(bool, &'a [u8])> {
    let fold = flags & FNM_CASEFOLD != 0;
    let mut i = 0;
    let neg = matches!(p.first(), Some(b'!') | Some(b'^'));
    if neg { i += 1; }
    let mut ok = false;
    let next = |i: &mut usize| -> Option<&'a [u8]> {
        if *i >= p.len() { return None; }
        if p[*i] == b'\\' && flags & FNM_NOESCAPE == 0 && *i + 1 < p.len() { *i += 1; }
        let l = char_len_at(p, *i).min(p.len() - *i);
        let r = &p[*i..*i + l]; *i += l; Some(r)
    };
    loop {
        if i >= p.len() { return None; }
        if p[i] == b']' { i += 1; break; }
        let lo = next(&mut i)?;
        let hi = if i + 1 < p.len() && p[i] == b'-' && p[i + 1] != b']' { i += 1; next(&mut i)? } else { lo };
        let inr = |x: &[u8]| -> bool { if x.len() == 1 && lo.len() == 1 && hi.len() == 1 { lo[0] <= x[0] && x[0] <= hi[0] } else { lo <= x && x <= hi } };
        if inr(ch) || (fold && ch.len() == 1 && (inr(&[ch[0].to_ascii_lowercase()]) || inr(&[ch[0].to_ascii_uppercase()]))) { ok = true; }
    }
    Some((ok != neg, &p[i..]))
}
fn fnm(p: &[u8], s: &[u8], flags: i64, lead: bool) -> bool {
    let path = flags & FNM_PATHNAME != 0;
    let dot = flags & FNM_DOTMATCH != 0;
    let fold = flags & FNM_CASEFOLD != 0;
    let (mut p, mut s, mut lead) = (p, s, lead);
    loop {
        let Some(&pc) = p.first() else { return s.is_empty() };
        match pc {
            b'*' => {
                if path && lead && p.starts_with(b"**/") {
                    // zero or more directories (hidden ones only with DOTMATCH)
                    let rest = &p[3..];
                    let mut t = s;
                    loop {
                        if fnm(rest, t, flags, true) { return true; }
                        if t.first() == Some(&b'.') && !dot { return false; }
                        match t.iter().position(|&b| b == b'/') { Some(i) => t = &t[i + 1..], None => return false }
                    }
                }
                while p.first() == Some(&b'*') { p = &p[1..]; }
                if lead && s.first() == Some(&b'.') && !dot { return false; }
                let mut i = 0;
                loop {
                    if fnm(p, &s[i..], flags, false) { return true; }
                    if i >= s.len() || (path && s[i] == b'/') { return false; }
                    i += 1;
                }
            }
            b'?' => {
                if s.is_empty() || (path && s[0] == b'/') || (lead && s[0] == b'.' && !dot) { return false; }
                let l = char_len_at(s, 0).min(s.len());
                p = &p[1..]; s = &s[l..]; lead = false;
            }
            b'[' => {
                if s.is_empty() { return false; }
                let l = char_len_at(s, 0).min(s.len());
                match bracket(&p[1..], &s[..l], flags) {
                    Some((ok, rest)) => {
                        if !ok || (path && s[0] == b'/') || (lead && s[0] == b'.' && !dot) { return false; }
                        p = rest; s = &s[l..]; lead = false;
                    }
                    None => return false,
                }
            }
            _ => {
                let mut q = p;
                if pc == b'\\' && flags & FNM_NOESCAPE == 0 && p.len() > 1 { q = &p[1..]; }
                if s.is_empty() || !eq_ch(q[0], s[0], fold) { return false; }
                lead = path && s[0] == b'/';
                p = &q[1..]; s = &s[1..];
            }
        }
    }
}
/// Expand {a,b} alternatives (nested).
fn brace_expand(p: &[u8], noescape: bool, out: &mut Vec<Vec<u8>>) {
    let mut i = 0; let mut depth = 0; let mut lb = None; let mut rb = None;
    while i < p.len() {
        match p[i] {
            b'\\' if !noescape => { i += 1; }
            b'{' => { if depth == 0 { lb = Some(i); } depth += 1; }
            b'}' if depth > 0 => { depth -= 1; if depth == 0 { rb = Some(i); break; } }
            _ => {}
        }
        i += 1;
    }
    let (Some(l), Some(r)) = (lb, rb) else { out.push(p.to_vec()); return };
    let mut start = l + 1; let mut d = 0; let mut k = l + 1;
    while k <= r {
        let ch = p[k];
        if ch == b'\\' && !noescape { k += 2; continue; }
        if ch == b'{' { d += 1; }
        if (ch == b',' && d == 0) || k == r {
            let mut alt = p[..l].to_vec(); alt.extend_from_slice(&p[start..k]); alt.extend_from_slice(&p[r + 1..]);
            brace_expand(&alt, noescape, out);
            start = k + 1;
        } else if ch == b'}' { d -= 1; }
        k += 1;
    }
}
fn f_s_fnmatch(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 2, 3)?;
    let pv = expect_str(c, a[0])?;
    let s = path_arg(c, a[1])?;
    let flags = match a.get(2) { Some(&f) => num_to_long(c, f)?, None => 0 };
    let pat = c.str_bytes(pv);
    if flags & FNM_EXTGLOB != 0 {
        let mut alts = vec![];
        brace_expand(pat, flags & FNM_NOESCAPE != 0, &mut alts);
        return Ok(Value::bool(alts.iter().any(|p| fnm(p, s.as_bytes(), flags, true))));
    }
    Ok(Value::bool(fnm(pat, s.as_bytes(), flags, true)))
}

/// Directory entries in readdir order (with "." and ".."), like CRuby.
fn read_dir_raw(path: &[u8]) -> Result<Vec<(Vec<u8>, u8)>, i64> { sys::read_dir_raw(&sys::cstr(path)) }
fn has_magic(seg: &[u8]) -> bool { seg.iter().any(|&b| matches!(b, b'*' | b'?' | b'[')) }
fn unescape(seg: &[u8], noescape: bool) -> Vec<u8> {
    if noescape { return seg.to_vec(); }
    let mut o = vec![]; let mut i = 0;
    while i < seg.len() { if seg[i] == b'\\' && i + 1 < seg.len() { i += 1; } o.push(seg[i]); i += 1; }
    o
}
fn is_dir_at(base: &str, rel: &[u8], follow: bool) -> bool {
    let p = join_base(base, rel);
    if follow { std::fs::metadata(&p).map(|m| m.is_dir()).unwrap_or(false) } else { std::fs::symlink_metadata(&p).map(|m| m.is_dir()).unwrap_or(false) }
}
fn join_base(base: &str, rel: &[u8]) -> String {
    let r = String::from_utf8_lossy(rel);
    if rel.starts_with(b"/") || base.is_empty() { if rel.is_empty() { ".".into() } else { r.into_owned() } } else if rel.is_empty() { base.to_string() } else { format!("{}/{}", base, r) }
}
fn glob_walk(base: &str, prefix: &[u8], segs: &[&[u8]], flags: i64, out: &mut Vec<Vec<u8>>) {
    let Some(&seg) = segs.first() else { return };
    let rest = &segs[1..];
    let noesc = flags & FNM_NOESCAPE != 0;
    let dirp = if prefix.is_empty() { b"".to_vec() } else { prefix.to_vec() };
    if seg.is_empty() {
        // trailing slash: directories only
        if rest.is_empty() { if is_dir_at(base, &dirp, true) { out.push(dirp); } } else { glob_walk(base, prefix, rest, flags, out); }
        return;
    }
    if seg == b"**" && !rest.is_empty() {
        glob_walk(base, prefix, rest, flags, out);
        let dp = join_base(base, &dirp);
        let Ok(mut ents) = read_dir_raw(dp.as_bytes()) else { return };
        ents.sort();
        for (n, t) in ents {
            if n == b"." || n == b".." || (n[0] == b'.' && flags & FNM_DOTMATCH == 0) { continue; }
            let mut np = dirp.clone(); np.extend_from_slice(&n);
            let isd = if t == 4 { true } else if t == 0 { is_dir_at(base, &np, false) } else { false };
            if !isd { continue; }
            np.push(b'/');
            glob_walk(base, &np, segs, flags, out);
        }
        return;
    }
    if !has_magic(seg) {
        let mut np = dirp.clone(); np.extend_from_slice(&unescape(seg, noesc));
        if rest.is_empty() { if std::fs::symlink_metadata(join_base(base, &np)).is_ok() { out.push(np); } }
        else if is_dir_at(base, &np, true) { np.push(b'/'); glob_walk(base, &np, rest, flags, out); }
        return;
    }
    let dp = join_base(base, &dirp);
    let Ok(mut ents) = read_dir_raw(dp.as_bytes()) else { return };
    ents.sort();
    for (n, t) in ents {
        if n == b".." { continue; }
        if n == b"." && !(rest.is_empty() && (seg[0] == b'.' || flags & FNM_DOTMATCH != 0)) { continue; }
        if !fnm(seg, &n, flags & !FNM_PATHNAME, true) { continue; }
        let mut np = dirp.clone(); np.extend_from_slice(&n);
        if rest.is_empty() { out.push(np); continue; }
        let isd = if t == 4 { true } else if t == 8 { false } else { is_dir_at(base, &np, true) };
        if isd { np.push(b'/'); glob_walk(base, &np, rest, flags, out); }
    }
}
fn glob_one(pat: &[u8], flags: i64, base: &str, out: &mut Vec<Vec<u8>>) {
    let mut alts = vec![];
    brace_expand(pat, flags & FNM_NOESCAPE != 0, &mut alts);
    for p in alts {
        if p.is_empty() { continue; }
        let (prefix, body): (&[u8], &[u8]) = if p[0] == b'/' { let n = p.iter().position(|&b| b != b'/').unwrap_or(p.len()); (b"/", &p[n..]) } else { (b"", &p[..]) };
        // collapse "//" except for the trailing-slash marker
        let mut segs: Vec<&[u8]> = vec![];
        let parts: Vec<&[u8]> = body.split(|&b| b == b'/').collect();
        for (i, s) in parts.iter().enumerate() { if s.is_empty() && i + 1 != parts.len() { continue; } if *s == b"**" && segs.last() == Some(&&b"**"[..]) { continue; } segs.push(s); }
        if segs.is_empty() || (segs.len() == 1 && segs[0].is_empty()) { if !prefix.is_empty() { out.push(prefix.to_vec()); } continue; }
        let mut found = vec![];
        glob_walk(base, prefix, &segs, flags, &mut found);
        for f in found { let f = if segs.last().map(|s| s.is_empty()).unwrap_or(false) && !f.ends_with(b"/") { let mut g = f; g.push(b'/'); g } else { f }; out.push(f); }
    }
}
fn d_s_glob(c: &mut Ctx, _slf: Value, a: &[Value], blk: Blk) -> RV {
    let (pos, opts) = match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    if pos.is_empty() { return argc_err(c, 0, 1, 2); }
    let mut flags = match pos.get(1) { Some(&f) if !is_array(pos[0]) || pos.len() == 2 && f.is_fixnum() => num_to_long(c, f)?, _ => 0 };
    let mut base = String::new();
    let mut sort = true;
    if let Some(h) = opts {
        let mut unknown = vec![];
        let mut i = 0;
        while let Some((k, _)) = c.hash_entry_at(h, i) {
            i += 1;
            if k == UNDEF || k == sym_value("base") || k == sym_value("flags") || k == sym_value("sort") { continue; }
            unknown.push(inspect_str(c, k)?);
        }
        if !unknown.is_empty() { return arg_err(c, format!("unknown keyword{}: {}", if unknown.len() > 1 { "s" } else { "" }, unknown.join(", "))); }
        if let Some(b) = crate::hash::hash_get_opt(c, h, sym_value("base"))? { if b != NIL { base = String::from_utf8_lossy(path_arg(c, b)?.as_bytes()).into_owned(); } }
        if let Some(f) = crate::hash::hash_get_opt(c, h, sym_value("flags"))? { if f != NIL { flags |= num_to_long(c, f)?; } }
        if let Some(s) = crate::hash::hash_get_opt(c, h, sym_value("sort"))? { sort = s.truthy(); }
    }
    let _ = sort;
    let mut pats: Vec<Vec<u8>> = vec![];
    if is_array(pos[0]) { let mut i = 0; while i < c.ary_len(pos[0]) { let x = c.ary_get(pos[0], i); i += 1; pats.push(path_arg(c, x)?.0); } }
    else { let p = path_arg(c, pos[0])?; pats.extend(p.as_bytes().split(|&b| b == 0).map(|x| x.to_vec())); }
    let mut found = vec![];
    for p in &pats { glob_one(p, flags, &base, &mut found); }
    let out = c.ary_of_strs(found, crate::builtins::encoding::default_external());
    if blk.is_null() { return Ok(out); }
    each_elem(c, out, blk)?;
    Ok(NIL)
}
fn d_s_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, opts) = match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    let mut args = vec![c.ary_new_from(pos)];
    if let Some(o) = opts { args.push(o); }
    d_s_glob(c, slf, &args, Blk::NONE)
}

// ---- Dir ----------------------------------------------------------------------------------------

fn d_s_pwd(c: &mut Ctx, _slf: Value, _a: &[Value], _b: Blk) -> RV {
    match std::env::current_dir() { Ok(d) => Ok(c.str_new(d.as_os_str().as_bytes())), Err(e) => Err(os_err(c, &e, "rb_dir_getwd")) }
}
static CHDIR_DEPTH: AtomicI32 = AtomicI32::new(0);
fn chdir_to(c: &mut Ctx, p: &PathArg) -> Result<(), Esc> {
    if sys::chdir(&p.cstr()) < 0 { return Err(last_err_at(c, "rb_dir_s_chdir - ", p.as_bytes())); }
    Ok(())
}
fn d_s_chdir(c: &mut Ctx, _slf: Value, a: &[Value], blk: Blk) -> RV {
    let p = match a.first() { Some(&v) => path_arg(c, v)?, None => PathArg(home_of(c, "")?) };
    if blk.is_null() {
        if CHDIR_DEPTH.load(Relaxed) > 0 { crate::builtins::kernel::warn_str(c, "warning: conflicting chdir during another chdir block\n"); }
        chdir_to(c, &p)?; return Ok(Value::fix(0));
    }
    let old = PathArg(cwd());
    chdir_to(c, &p)?;
    CHDIR_DEPTH.fetch_add(1, Relaxed);
    let arg = c.str_new(p.as_bytes());
    let r = yield1(c, blk, arg);
    CHDIR_DEPTH.fetch_sub(1, Relaxed);
    let (ek, ev) = (c.esc, c.esc_val);
    let back = chdir_to(c, &old);
    if r.is_err() { c.esc = ek; c.esc_val = ev; return r; }
    back?;
    r
}
fn d_s_mkdir(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let p = path_arg(c, a[0])?;
    let mode = match a.get(1) { Some(&m) if m != NIL => mode_arg(c, m)?, _ => 0o777 };
    use std::os::unix::fs::DirBuilderExt;
    match std::fs::DirBuilder::new().mode(mode).create(&p) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err_at(c, &e, "rb_dir_s_mkdir - ", p.as_bytes())) }
}
fn d_s_rmdir(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?;
    match std::fs::remove_dir(&p) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err_at(c, &e, "rb_dir_s_rmdir - ", p.as_bytes())) }
}
fn dir_list(c: &mut Ctx, v: Value, children: bool) -> Result<Vec<Vec<u8>>, Esc> {
    let p = path_arg(c, v)?;
    match read_dir_raw(p.as_bytes()) {
        Ok(e) => Ok(e.into_iter().map(|x| x.0).filter(|n| !children || (n != b"." && n != b"..")).collect()),
        Err(e) => Err(crate::builtins::io::raise_errno_at(c, e, "rb_dir_initialize - ", p.as_bytes())),
    }
}
fn strs(c: &mut Ctx, v: Vec<Vec<u8>>) -> Value { c.ary_of_strs(v, crate::builtins::encoding::default_external()) }
/// yield each element of Array `l`, re-reading its length at every step
fn each_elem(c: &mut Ctx, l: Value, blk: Blk) -> Result<(), Esc> {
    let mut i = 0;
    while let Some(&x) = c.ary(l).get(i) { yield1(c, blk, x)?; i += 1; }
    Ok(())
}
fn d_s_entries(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { let l = dir_list(c, a[0], false)?; Ok(strs(c, l)) }
fn d_s_children(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { let l = dir_list(c, a[0], true)?; Ok(strs(c, l)) }
fn d_s_each_child(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"each_child"), a, NIL); }
    let l = dir_list(c, a[0], true)?; let l = strs(c, l);
    each_elem(c, l, blk)?;
    Ok(NIL)
}
fn d_s_foreach(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"foreach"), a, NIL); }
    let l = dir_list(c, a[0], false)?; let l = strs(c, l);
    each_elem(c, l, blk)?;
    Ok(NIL)
}
fn d_s_empty(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?;
    Ok(Value::bool(read_dir_raw(p.as_bytes()).map(|e| e.len() <= 2).unwrap_or(false)))
}
fn d_s_home(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let u = match a.first() { Some(&v) if v != NIL => to_rstring(c, v)?, _ => String::new() };
    let h = home_of(c, &u)?;
    Ok(c.str_new(&h))
}
fn d_s_exist(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { f_s_dir_p(c, s, a, b) }
fn iv(n: &str) -> crate::value::Sym { intern(n.as_bytes()) }
fn d_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, _) = match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    if pos.len() != 1 { return argc_err(c, pos.len(), 1, 1); }
    let p = path_arg(c, pos[0])?;
    let l = dir_list(c, pos[0], false)?; let l = strs(c, l);
    let ps = c.str_new(p.as_bytes());
    ivar_set_raw(c, s, iv("@path"), ps); ivar_set_raw(c, s, iv("@ents"), l); ivar_set_raw(c, s, iv("@pos"), Value::fix(0)); ivar_set_raw(c, s, iv("@closed"), FALSE);
    Ok(NIL)
}
fn d_check(c: &mut Ctx, s: Value) -> Result<(Value, usize), Esc> {
    let e = ivar_get(c, s, iv("@ents"));
    if e == NIL { return err(c, c.vm().c_io_error.get(), "uninitialized Dir"); }
    if ivar_get(c, s, iv("@closed")).truthy() { return err(c, c.vm().c_io_error.get(), "closed directory"); }
    Ok((e, ivar_get(c, s, iv("@pos")).as_fix() as usize))
}
fn d_read(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (e, p) = d_check(c, s)?;
    if p >= c.ary_len(e) { return Ok(NIL); }
    ivar_set_raw(c, s, iv("@pos"), Value::fix(p as i64 + 1));
    let x = c.ary_get(e, p);
    Ok(c.str_dup(x))
}
fn d_each(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"each"), a, NIL); }
    loop { let v = d_read(c, s, &[], Blk::NONE)?; if v == NIL { break; } yield1(c, blk, v)?; }
    Ok(s)
}
fn d_children(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (e, _) = d_check(c, s)?;
    let out = c.ary_new();
    let mut i = 0;
    while let Some(&x) = c.ary(e).get(i) { i += 1; if c.str_bytes(x) != b"." && c.str_bytes(x) != b".." { let d = c.str_dup(x); c.ary_push(out, d); } }
    Ok(out)
}
fn d_each_child(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"each_child"), a, NIL); }
    let l = d_children(c, s, &[], Blk::NONE)?;
    each_elem(c, l, blk)?;
    Ok(s)
}
fn d_rewind(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { d_check(c, s)?; ivar_set_raw(c, s, iv("@pos"), Value::fix(0)); Ok(s) }
fn d_tell(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (_, p) = d_check(c, s)?; Ok(Value::fix(p as i64)) }
fn d_seek(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { d_check(c, s)?; let n = num_to_long(c, a[0])?; ivar_set_raw(c, s, iv("@pos"), Value::fix(n.max(0))); Ok(s) }
fn d_set_pos(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { d_seek(c, s, a, b)?; Ok(a[0]) }
fn d_close(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { ivar_set_raw(c, s, iv("@closed"), TRUE); Ok(NIL) }
fn d_path(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let p = ivar_get(c, s, iv("@path")); Ok(if p == NIL { NIL } else { c.str_dup(p) }) }
fn d_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let p = ivar_get(c, s, iv("@path"));
    let k = class_path(c, real_class_of(c, s));
    let s = if p == NIL { format!("#<{}: uninitialized>", k) } else { format!("#<{}:{}>", k, String::from_utf8_lossy(c.str_bytes(p))) };
    Ok(c.str_new(s.as_bytes()))
}
fn d_chdir(c: &mut Ctx, s: Value, _a: &[Value], blk: Blk) -> RV { d_check(c, s)?; let p = ivar_get(c, s, iv("@path")); d_s_chdir(c, s, &[p], blk) }
fn d_s_open(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let d = crate::builtins::object::c_new(c, slf, a, Blk::NONE)?;
    if blk.is_null() { return Ok(d); }
    let r = yield1(c, blk, d);
    ivar_set_raw(c, d, iv("@closed"), TRUE);
    r
}

// predicates shared by File and FileTest
const TESTS: &[(&str, CFunc, i32)] = &[("exist?", f_s_exist, 1), ("file?", f_s_file_p, 1), ("directory?", f_s_dir_p, 1), ("symlink?", f_s_symlink_p, 1), ("pipe?", f_s_pipe_p, 1),
        ("socket?", f_s_socket_p, 1), ("chardev?", f_s_chardev_p, 1), ("blockdev?", f_s_blockdev_p, 1), ("readable?", f_s_readable, 1), ("writable?", f_s_writable, 1),
        ("executable?", f_s_executable, 1), ("readable_real?", f_s_readable_real, 1), ("writable_real?", f_s_writable_real, 1), ("executable_real?", f_s_executable_real, 1),
        ("zero?", f_s_zero, 1), ("empty?", f_s_zero, 1), ("size?", f_s_size_p, 1), ("size", f_s_size, 1), ("setuid?", f_s_setuid, 1), ("setgid?", f_s_setgid, 1), ("sticky?", f_s_sticky, 1),
        ("owned?", f_s_owned, 1), ("grpowned?", f_s_grpowned, 1), ("world_readable?", f_s_world_readable, 1), ("world_writable?", f_s_world_writable, 1), ("identical?", f_s_identical, 2)];
pub fn init_filetest(c: &mut Ctx) {
    if c.vm().m_file_test.get() != NIL { return; }
    let ft = define_module(c, "FileTest");
    c.vm().m_file_test.set(ft);
    for &(n, fun, ar) in TESTS { define_module_function(c, ft, n, fun, ar); }
}
pub fn init_stat(c: &mut Ctx) {
    if c.vm().c_stat.get() != NIL { return; }
    let f = c.vm().c_file.get();
    let st = define_class_under(c, f, "Stat", c.vm().c_object.get());
    class_set_alloc(c, st, AllocKind::None);
    include_module(c, st, c.vm().m_comparable.get());
    c.vm().c_stat.set(st);
    let sd = |c: &mut Ctx, n: &str, fun: CFunc, a: i32| define_cfunc(c, st, n, fun, a);
    define_private_cfunc(c, st, "initialize", st_initialize, 1);
    define_private_cfunc(c, st, "initialize_copy", st_init_copy, 1);
    sd(c, "size", st_size, 0); sd(c, "mode", st_mode, 0); sd(c, "uid", st_uid, 0); sd(c, "gid", st_gid, 0); sd(c, "ino", st_ino, 0); sd(c, "dev", st_dev, 0);
    sd(c, "dev_major", st_dev_major, 0); sd(c, "dev_minor", st_dev_minor, 0); sd(c, "rdev", st_rdev, 0); sd(c, "nlink", st_nlink, 0); sd(c, "blksize", st_blksize, 0);
    sd(c, "blocks", st_blocks, 0); sd(c, "atime", st_atime, 0); sd(c, "mtime", st_mtime, 0); sd(c, "ctime", st_ctime, 0); sd(c, "birthtime", st_birthtime, 0);
    sd(c, "file?", st_file, 0); sd(c, "directory?", st_dir, 0); sd(c, "symlink?", st_symlink, 0); sd(c, "pipe?", st_pipe, 0); sd(c, "socket?", st_socket, 0);
    sd(c, "chardev?", st_chardev, 0); sd(c, "blockdev?", st_blockdev, 0); sd(c, "readable?", st_readable, 0); sd(c, "writable?", st_writable, 0);
    sd(c, "executable?", st_executable, 0); sd(c, "readable_real?", st_readable_real, 0); sd(c, "writable_real?", st_writable_real, 0);
    sd(c, "executable_real?", st_executable_real, 0); sd(c, "world_readable?", st_world_readable, 0); sd(c, "world_writable?", st_world_writable, 0);
    sd(c, "zero?", st_zero, 0); sd(c, "size?", st_size_p, 0); sd(c, "ftype", st_ftype, 0); sd(c, "setuid?", st_setuid, 0); sd(c, "setgid?", st_setgid, 0);
    sd(c, "sticky?", st_sticky, 0); sd(c, "owned?", st_owned, 0); sd(c, "grpowned?", st_grpowned, 0); sd(c, "<=>", st_cmp, 1); sd(c, "inspect", st_inspect, 0);
}
pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let (io, f) = (vmr.c_io.get(), vmr.c_file.get());
    crate::builtins::prelude::lazy_const(c, f, "Stat", "saruby/stat");
    crate::builtins::prelude::lazy_const(c, vmr.c_object.get(), "FileTest", "saruby/filetest");
    for &(n, fun, ar) in TESTS { define_singleton_cfunc(c, f, n, fun, ar); }
    let df = |c: &mut Ctx, n: &str, fun: CFunc, a: i32| define_singleton_cfunc(c, f, n, fun, a);
    df(c, "open", f_s_open, -1); df(c, "stat", f_s_stat, 1); df(c, "lstat", f_s_lstat, 1); df(c, "mtime", f_s_mtime, 1); df(c, "atime", f_s_atime, 1); df(c, "ctime", f_s_ctime, 1);
    df(c, "birthtime", f_s_birthtime, 1); df(c, "ftype", f_s_ftype, 1); df(c, "delete", f_s_delete, -1); df(c, "unlink", f_s_delete, -1); df(c, "rename", f_s_rename, 2);
    df(c, "symlink", f_s_symlink, 2); df(c, "link", f_s_link, 2); df(c, "readlink", f_s_readlink, 1); df(c, "chmod", f_s_chmod, -1); df(c, "chown", f_s_chown, -1);
    df(c, "lchown", f_s_lchown, -1); df(c, "lchmod", f_s_lchmod, -1); df(c, "truncate", f_s_truncate, 2); df(c, "umask", f_s_umask, -1); df(c, "mkfifo", f_s_mkfifo, -1);
    df(c, "utime", f_s_utime, -1); df(c, "lutime", f_s_lutime, -1); df(c, "basename", f_s_basename, -1); df(c, "dirname", f_s_dirname, -1); df(c, "extname", f_s_extname, 1);
    df(c, "split", f_s_split, 1); df(c, "join", f_s_join, -1); df(c, "expand_path", f_s_expand_path, -1); df(c, "absolute_path", f_s_absolute_path, -1);
    df(c, "absolute_path?", f_s_absolute_path_p, 1); df(c, "realpath", f_s_realpath, -1); df(c, "realdirpath", f_s_realdirpath, -1); df(c, "path", f_s_path, 1);
    df(c, "fnmatch", f_s_fnmatch, -1); df(c, "fnmatch?", f_s_fnmatch, -1);
    let di = |c: &mut Ctx, n: &str, fun: CFunc, a: i32| define_cfunc(c, f, n, fun, a);
    di(c, "size", f_size, 0); di(c, "truncate", f_truncate, 1); di(c, "flock", f_flock, 1); di(c, "chmod", f_chmod, 1); di(c, "chown", f_chown, 2); di(c, "mtime", f_mtime, 0);
    di(c, "atime", f_atime, 0); di(c, "ctime", f_ctime, 0); di(c, "lstat", f_lstat, 0);
    define_cfunc(c, io, "stat", f_stat, 0);
    let sep = c.str_new_frozen(b"/");
    const_set(c, f, intern(b"SEPARATOR"), sep);
    let sep = c.str_new_frozen(b"/");
    const_set(c, f, intern(b"Separator"), sep);
    const_set(c, f, intern(b"ALT_SEPARATOR"), NIL);
    let sep = c.str_new_frozen(b":");
    const_set(c, f, intern(b"PATH_SEPARATOR"), sep);
    let fc = define_module_under(c, f, "Constants");
    for (n, v) in [("RDONLY", 0), ("WRONLY", 1), ("RDWR", 2), ("APPEND", 0o2000), ("CREAT", 0o100), ("EXCL", 0o200), ("NONBLOCK", 0o4000), ("TRUNC", 0o1000),
                   ("NOCTTY", 0o400), ("BINARY", 0), ("SHARE_DELETE", 0), ("SYNC", 0o4010000), ("DSYNC", 0o10000), ("NOFOLLOW", 0o400000), ("DIRECTORY", 0o200000),
                   ("TMPFILE", 0o20200000), ("NOATIME", 0o1000000), ("CLOEXEC", 0o2000000), ("LOCK_SH", 1), ("LOCK_EX", 2), ("LOCK_NB", 4), ("LOCK_UN", 8),
                   ("FNM_NOESCAPE", 1), ("FNM_PATHNAME", 2), ("FNM_DOTMATCH", 4), ("FNM_CASEFOLD", 8), ("FNM_EXTGLOB", 16), ("FNM_SYSCASE", 0), ("FNM_SHORTNAME", 0)] {
        const_set(c, fc, intern(n.as_bytes()), Value::fix(v));
    }
    let null = c.str_new_frozen(b"/dev/null");
    const_set(c, fc, intern(b"NULL"), null);
    include_module(c, io, fc);
    let dir = define_class(c, "Dir", vmr.c_object.get());
    include_module(c, dir, vmr.m_enumerable.get());
    vmr.c_dir.set(dir);
    let dd = |c: &mut Ctx, n: &str, fun: CFunc, a: i32| define_singleton_cfunc(c, dir, n, fun, a);
    dd(c, "pwd", d_s_pwd, 0); dd(c, "getwd", d_s_pwd, 0); dd(c, "glob", d_s_glob, -1); dd(c, "[]", d_s_aref, -1); dd(c, "exist?", d_s_exist, 1); dd(c, "mkdir", d_s_mkdir, -1);
    dd(c, "rmdir", d_s_rmdir, 1); dd(c, "delete", d_s_rmdir, 1); dd(c, "unlink", d_s_rmdir, 1); dd(c, "entries", d_s_entries, -1); dd(c, "children", d_s_children, -1);
    dd(c, "each_child", d_s_each_child, -1); dd(c, "foreach", d_s_foreach, -1); dd(c, "empty?", d_s_empty, 1); dd(c, "home", d_s_home, -1); dd(c, "chdir", d_s_chdir, -1);
    dd(c, "open", d_s_open, -1);
    let dm = |c: &mut Ctx, n: &str, fun: CFunc, a: i32| define_cfunc(c, dir, n, fun, a);
    define_private_cfunc(c, dir, "initialize", d_initialize, -1);
    dm(c, "read", d_read, 0); dm(c, "each", d_each, 0); dm(c, "children", d_children, 0); dm(c, "each_child", d_each_child, 0); dm(c, "rewind", d_rewind, 0);
    dm(c, "tell", d_tell, 0); dm(c, "pos", d_tell, 0); dm(c, "seek", d_seek, 1); dm(c, "pos=", d_set_pos, 1); dm(c, "close", d_close, 0); dm(c, "path", d_path, 0);
    dm(c, "to_path", d_path, 0); dm(c, "inspect", d_inspect, 0); dm(c, "chdir", d_chdir, 0);
}
