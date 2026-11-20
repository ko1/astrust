// File (class methods, constants), File::Stat, FileTest, Dir, fnmatch / glob.
use crate::array::*;
use crate::builtins::exception::raise_errno;
use crate::builtins::io::{path_arg, rio};
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::builtins::sys;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::FromRawFd;

pub struct RStat { pub md: Option<std::fs::Metadata> }
impl RData for RStat {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn io_errno(e: &std::io::Error) -> i64 { e.raw_os_error().map(|x| x as i64).unwrap_or(5) }
fn os_err(c: &mut Ctx, e: &std::io::Error, detail: &str) -> Esc { raise_errno(c, io_errno(e), detail) }
fn last_err(c: &mut Ctx, detail: &str) -> Esc { raise_errno(c, sys::errno(), detail) }
fn new_stat(md: std::fs::Metadata) -> Value { init_stat(); crate::gc::alloc(vm().c_stat, Body::Data(Box::new(RStat { md: Some(md) }))) }
fn fd_meta(fd: i32) -> std::io::Result<std::fs::Metadata> {
    let f = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(fd) });
    f.metadata()
}
/// Metadata of a path or an IO argument.
fn meta_of(c: &mut Ctx, v: Value, follow: bool) -> Result<Result<std::fs::Metadata, (std::io::Error, String)>, Esc> {
    if let Some(io) = rio(v) { let io = if io.closed { return err(c, vm().c_io_error, "closed stream"); } else { io }; return Ok(fd_meta(io.fd).map_err(|e| (e, io.path.clone()))); }
    let p = path_arg(c, v)?;
    Ok(if follow { std::fs::metadata(&p) } else { std::fs::symlink_metadata(&p) }.map_err(|e| (e, p)))
}
fn stat_md(c: &mut Ctx, v: Value) -> Result<&'static std::fs::Metadata, Esc> {
    match rdata::<RStat>(v).and_then(|s| s.md.as_ref()) { Some(m) => Ok(m), None => type_err(c, "uninitialized File::Stat") }
}
fn time_of(sec: i64, nsec: i64) -> Value { crate::builtins::time::time_from_ns(sec, nsec) }
fn access_ok(md: &std::fs::Metadata, bit: u32, effective: bool) -> bool {
    let (uid, gid) = unsafe { if effective { (sys::geteuid(), sys::getegid()) } else { (sys::getuid(), sys::getgid()) } };
    if uid == 0 { return bit != 1 || md.mode() & 0o111 != 0 || md.is_dir(); }
    let m = md.mode();
    if md.uid() == uid { m & (bit << 6) != 0 } else if md.gid() == gid { m & (bit << 3) != 0 } else { m & bit != 0 }
}
fn ftype(md: &std::fs::Metadata) -> &'static str {
    match md.mode() & 0o170000 { 0o100000 => "file", 0o040000 => "directory", 0o120000 => "link", 0o020000 => "characterSpecial", 0o060000 => "blockSpecial", 0o010000 => "fifo", 0o140000 => "socket", _ => "unknown" }
}

macro_rules! stat_fn { ($name:ident, |$m:ident| $e:expr) => { fn $name(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let $m = stat_md(c, s)?; Ok($e) } }; }
stat_fn!(st_size, |m| Value::fix(m.len() as i64));
stat_fn!(st_mode, |m| Value::fix(m.mode() as i64));
stat_fn!(st_uid, |m| Value::fix(m.uid() as i64));
stat_fn!(st_gid, |m| Value::fix(m.gid() as i64));
stat_fn!(st_ino, |m| Value::fix(m.ino() as i64));
stat_fn!(st_dev, |m| Value::fix(m.dev() as i64));
stat_fn!(st_dev_major, |m| Value::fix(((m.dev() >> 8) & 0xfff) as i64));
stat_fn!(st_dev_minor, |m| Value::fix(((m.dev() & 0xff) | ((m.dev() >> 12) & 0xfff00)) as i64));
stat_fn!(st_rdev, |m| Value::fix(m.rdev() as i64));
stat_fn!(st_nlink, |m| Value::fix(m.nlink() as i64));
stat_fn!(st_blksize, |m| Value::fix(m.blksize() as i64));
stat_fn!(st_blocks, |m| Value::fix(m.blocks() as i64));
stat_fn!(st_atime, |m| time_of(m.atime(), m.atime_nsec()));
stat_fn!(st_mtime, |m| time_of(m.mtime(), m.mtime_nsec()));
stat_fn!(st_ctime, |m| time_of(m.ctime(), m.ctime_nsec()));
stat_fn!(st_file, |m| Value::bool(m.is_file()));
stat_fn!(st_dir, |m| Value::bool(m.is_dir()));
stat_fn!(st_symlink, |m| Value::bool(m.file_type().is_symlink()));
stat_fn!(st_pipe, |m| Value::bool(m.mode() & 0o170000 == 0o010000));
stat_fn!(st_socket, |m| Value::bool(m.mode() & 0o170000 == 0o140000));
stat_fn!(st_chardev, |m| Value::bool(m.mode() & 0o170000 == 0o020000));
stat_fn!(st_blockdev, |m| Value::bool(m.mode() & 0o170000 == 0o060000));
stat_fn!(st_readable, |m| Value::bool(access_ok(m, 4, true)));
stat_fn!(st_writable, |m| Value::bool(access_ok(m, 2, true)));
stat_fn!(st_executable, |m| Value::bool(access_ok(m, 1, true)));
stat_fn!(st_readable_real, |m| Value::bool(access_ok(m, 4, false)));
stat_fn!(st_writable_real, |m| Value::bool(access_ok(m, 2, false)));
stat_fn!(st_executable_real, |m| Value::bool(access_ok(m, 1, false)));
stat_fn!(st_world_readable, |m| if m.mode() & 4 != 0 { Value::fix((m.mode() & 0o777) as i64) } else { NIL });
stat_fn!(st_world_writable, |m| if m.mode() & 2 != 0 { Value::fix((m.mode() & 0o777) as i64) } else { NIL });
stat_fn!(st_zero, |m| Value::bool(m.len() == 0));
stat_fn!(st_size_p, |m| if m.len() == 0 { NIL } else { Value::fix(m.len() as i64) });
stat_fn!(st_ftype, |m| str_new(ftype(m).as_bytes()));
stat_fn!(st_setuid, |m| Value::bool(m.mode() & 0o4000 != 0));
stat_fn!(st_setgid, |m| Value::bool(m.mode() & 0o2000 != 0));
stat_fn!(st_sticky, |m| Value::bool(m.mode() & 0o1000 != 0));
stat_fn!(st_owned, |m| Value::bool(m.uid() == unsafe { sys::geteuid() }));
stat_fn!(st_grpowned, |m| Value::bool(m.gid() == unsafe { sys::getegid() }));
fn st_birthtime(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let m = stat_md(c, s)?;
    match m.created().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()) { Some(d) => Ok(time_of(d.as_secs() as i64, d.subsec_nanos() as i64)), None => err(c, vm().c_not_impl_error, "birthtime() function is unimplemented on this machine") }
}
fn st_cmp(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let m = stat_md(c, s)?;
    let Some(o) = rdata::<RStat>(a[0]).and_then(|s| s.md.as_ref()) else { return Ok(NIL) };
    Ok(Value::fix((m.mtime(), m.mtime_nsec()).cmp(&(o.mtime(), o.mtime_nsec())) as i64))
}
fn st_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let k = class_path(real_class_of(s));
    let Some(m) = rdata::<RStat>(s).and_then(|s| s.md.as_ref()) else { return Ok(str_new(format!("#<{}: uninitialized>", k).as_bytes())) };
    let mut t = vec![];
    for (sec, ns) in [(m.atime(), m.atime_nsec()), (m.mtime(), m.mtime_nsec()), (m.ctime(), m.ctime_nsec())] { let tv = time_of(sec, ns); t.push(inspect_str(c, tv)?); }
    Ok(str_new(format!("#<{} dev=0x{:x}, ino={}, mode=0{:o}, nlink={}, uid={}, gid={}, rdev=0x{:x}, size={}, blksize={}, blocks={}, atime={}, mtime={}, ctime={}>",
        k, m.dev(), m.ino(), m.mode(), m.nlink(), m.uid(), m.gid(), m.rdev(), m.len(), m.blksize(), m.blocks(), t[0], t[1], t[2]).as_bytes()))
}
fn st_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    match std::fs::metadata(&p) { Ok(m) => { rdata::<RStat>(s).unwrap().md = Some(m); Ok(NIL) } Err(e) => Err(os_err(c, &e, &format!("rb_file_s_stat - {}", p))) }
}
fn st_init_copy(_c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if let Some(o) = rdata::<RStat>(a[0]) { rdata::<RStat>(s).unwrap().md = o.md.clone(); }
    Ok(s)
}
pub fn alloc_data(_c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if vm().c_stat != NIL && crate::builtins::inherits(klass, vm().c_stat) { return Ok(Some(crate::gc::alloc(klass, Body::Data(Box::new(RStat { md: None }))))); }
    Ok(None)
}

// ---- File class methods -----------------------------------------------------------------------

fn f_s_stat(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    match meta_of(c, a[0], true)? { Ok(m) => Ok(new_stat(m)), Err((e, p)) => Err(os_err(c, &e, &format!("rb_file_s_stat - {}", p))) }
}
fn f_s_lstat(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    match meta_of(c, a[0], false)? { Ok(m) => Ok(new_stat(m)), Err((e, p)) => Err(os_err(c, &e, &format!("rb_file_s_lstat - {}", p))) }
}
fn f_stat(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let io = crate::builtins::io::get_io(c, s)?;
    match fd_meta(io.fd) { Ok(m) => Ok(new_stat(m)), Err(e) => Err(os_err(c, &e, "")) }
}
fn f_lstat(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let io = crate::builtins::io::get_io(c, s)?;
    let p = io.path.clone();
    match std::fs::symlink_metadata(&p) { Ok(m) => Ok(new_stat(m)), Err(e) => Err(os_err(c, &e, &format!("rb_file_lstat - {}", p))) }
}
macro_rules! test_fn {
    ($name:ident, $follow:expr, |$m:ident| $e:expr) => {
        fn $name(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(match meta_of(c, a[0], $follow)? { Ok($m) => $e, Err(_) => FALSE }) }
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
test_fn!(f_s_owned, true, |m| Value::bool(m.uid() == unsafe { sys::geteuid() }));
test_fn!(f_s_grpowned, true, |m| Value::bool(m.gid() == unsafe { sys::getegid() }));
fn f_s_size_p(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(match meta_of(c, a[0], true)? { Ok(m) if m.len() > 0 => Value::fix(m.len() as i64), _ => NIL }) }
fn f_s_world_readable(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(match meta_of(c, a[0], true)? { Ok(m) if m.mode() & 4 != 0 => Value::fix((m.mode() & 0o777) as i64), _ => NIL }) }
fn f_s_world_writable(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(match meta_of(c, a[0], true)? { Ok(m) if m.mode() & 2 != 0 => Value::fix((m.mode() & 0o777) as i64), _ => NIL }) }
fn f_s_identical(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    Ok(match (meta_of(c, a[0], true)?, meta_of(c, a[1], true)?) { (Ok(x), Ok(y)) => Value::bool(x.dev() == y.dev() && x.ino() == y.ino()), _ => FALSE })
}
macro_rules! meta_fn {
    ($name:ident, $what:expr, |$m:ident| $e:expr) => {
        fn $name(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { match meta_of(c, a[0], true)? { Ok($m) => Ok($e), Err((e, p)) => Err(os_err(c, &e, &format!("{} - {}", $what, p))) } }
    };
}
meta_fn!(f_s_size, "rb_file_s_size", |m| Value::fix(m.len() as i64));
meta_fn!(f_s_mtime, "rb_file_s_mtime", |m| time_of(m.mtime(), m.mtime_nsec()));
meta_fn!(f_s_atime, "rb_file_s_atime", |m| time_of(m.atime(), m.atime_nsec()));
meta_fn!(f_s_ctime, "rb_file_s_ctime", |m| time_of(m.ctime(), m.ctime_nsec()));
fn f_s_birthtime(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    match meta_of(c, a[0], true)? {
        Ok(m) => match m.created().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()) { Some(d) => Ok(time_of(d.as_secs() as i64, d.subsec_nanos() as i64)), None => err(c, vm().c_not_impl_error, "birthtime() function is unimplemented on this machine") },
        Err((e, p)) => Err(os_err(c, &e, &format!("rb_file_s_birthtime - {}", p))),
    }
}
fn f_s_ftype(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    match meta_of(c, a[0], false)? { Ok(m) => Ok(str_new(ftype(&m).as_bytes())), Err((e, p)) => Err(os_err(c, &e, &format!("rb_file_s_ftype - {}", p))) }
}
fn f_s_delete(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    for &x in a { let p = path_arg(c, x)?; if let Err(e) = std::fs::remove_file(&p) { return Err(os_err(c, &e, &format!("rb_file_s_unlink - {}", p))); } }
    Ok(Value::fix(a.len() as i64))
}
fn f_s_rename(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (x, y) = (path_arg(c, a[0])?, path_arg(c, a[1])?);
    match std::fs::rename(&x, &y) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err(c, &e, &format!("rb_file_s_rename - ({}, {})", x, y))) }
}
fn f_s_symlink(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (x, y) = (path_arg(c, a[0])?, path_arg(c, a[1])?);
    match std::os::unix::fs::symlink(&x, &y) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err(c, &e, &format!("rb_file_s_symlink - ({}, {})", x, y))) }
}
fn f_s_link(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (x, y) = (path_arg(c, a[0])?, path_arg(c, a[1])?);
    match std::fs::hard_link(&x, &y) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err(c, &e, &format!("rb_file_s_link - ({}, {})", x, y))) }
}
fn f_s_readlink(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    match std::fs::read_link(&p) { Ok(t) => Ok(str_new(t.to_string_lossy().as_bytes())), Err(e) => Err(os_err(c, &e, &format!("rb_readlink - {}", p))) }
}
fn mode_arg(c: &mut Ctx, v: Value) -> Result<u32, Esc> { Ok(num_to_long(c, v)? as u32) }
fn f_s_chmod(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let m = mode_arg(c, a[0])?;
    for &x in &a[1..] { let p = path_arg(c, x)?; if unsafe { sys::chmod(sys::cstr(p.as_bytes()).as_ptr(), m) } < 0 { return Err(last_err(c, &format!("rb_file_s_chmod - {}", p))); } }
    Ok(Value::fix(a.len() as i64 - 1))
}
fn id_arg(c: &mut Ctx, v: Value) -> Result<u32, Esc> { if v == NIL { Ok(u32::MAX) } else { Ok(num_to_long(c, v)? as u32) } }
fn f_s_chown(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.len() < 2 { return argc_err(c, a.len(), 2, -1); }
    let (u, g) = (id_arg(c, a[0])?, id_arg(c, a[1])?);
    for &x in &a[2..] { let p = path_arg(c, x)?; if unsafe { sys::chown(sys::cstr(p.as_bytes()).as_ptr(), u, g) } < 0 { return Err(last_err(c, &format!("rb_file_s_chown - {}", p))); } }
    Ok(Value::fix(a.len() as i64 - 2))
}
fn f_s_lchown(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.len() < 2 { return argc_err(c, a.len(), 2, -1); }
    let (u, g) = (id_arg(c, a[0])?, id_arg(c, a[1])?);
    for &x in &a[2..] { let p = path_arg(c, x)?; if unsafe { sys::lchown(sys::cstr(p.as_bytes()).as_ptr(), u, g) } < 0 { return Err(last_err(c, &format!("rb_file_s_lchown - {}", p))); } }
    Ok(Value::fix(a.len() as i64 - 2))
}
fn f_s_lchmod(c: &mut Ctx, _slf: Value, _a: &[Value], _b: *const Block) -> RV { err(c, vm().c_not_impl_error, "lchmod() function is unimplemented on this machine") }
fn f_s_truncate(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?; let n = num_to_long(c, a[1])?;
    if unsafe { sys::truncate(sys::cstr(p.as_bytes()).as_ptr(), n) } < 0 { return Err(last_err(c, &format!("rb_file_s_truncate - {}", p))); }
    Ok(Value::fix(0))
}
fn f_s_umask(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    unsafe {
        if let Some(&m) = a.first() { let m = mode_arg(c, m)?; return Ok(Value::fix(sys::umask(m) as i64)); }
        let old = sys::umask(0); sys::umask(old);
        Ok(Value::fix(old as i64))
    }
}
fn f_s_mkfifo(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    let m = match a.get(1) { Some(&m) => mode_arg(c, m)?, None => 0o666 };
    if unsafe { sys::mkfifo(sys::cstr(p.as_bytes()).as_ptr(), m) } < 0 { return Err(last_err(c, &format!("rb_file_s_mkfifo - {}", p))); }
    Ok(Value::fix(0))
}
fn timespec_arg(c: &mut Ctx, v: Value) -> Result<sys::Timespec, Esc> {
    if v == NIL { return Ok(sys::Timespec { sec: 0, nsec: (1 << 30) - 1 }); } // UTIME_NOW
    if let Some((s, ns)) = crate::builtins::time::time_to_ns(v) { return Ok(sys::Timespec { sec: s, nsec: ns }); }
    let f = crate::builtins::numeric_methods::to_f(c, v)?;
    Ok(sys::Timespec { sec: f.floor() as i64, nsec: ((f - f.floor()) * 1e9) as i64 })
}
fn utime_impl(c: &mut Ctx, a: &[Value], nofollow: bool) -> RV {
    if a.len() < 2 { return argc_err(c, a.len(), 2, -1); }
    let ts = [timespec_arg(c, a[0])?, timespec_arg(c, a[1])?];
    for &x in &a[2..] { let p = path_arg(c, x)?; if unsafe { sys::utimensat(-100, sys::cstr(p.as_bytes()).as_ptr(), ts.as_ptr(), if nofollow { 0x100 } else { 0 }) } < 0 { return Err(last_err(c, &format!("rb_file_s_utime - {}", p))); } }
    Ok(Value::fix(a.len() as i64 - 2))
}
fn f_s_utime(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { utime_impl(c, a, false) }
fn f_s_lutime(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { utime_impl(c, a, true) }

fn str_like(src: Value, b: &[u8]) -> Value { if is_string(src) { str_new_enc(b, str_enc(src)) } else { str_new(b) } }
fn f_s_basename(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let p = path_arg(c, a[0])?;
    let t = p.trim_end_matches('/');
    let mut base = if t.is_empty() && p.starts_with('/') { "/".to_string() } else { t.rsplit('/').next().unwrap_or("").to_string() };
    if a.len() > 1 {
        let suf = path_arg(c, a[1])?;
        if suf == ".*" { if let Some(i) = base.rfind('.') { if i > 0 { base.truncate(i); } } }
        else if base.ends_with(&suf) && base != suf { base.truncate(base.len() - suf.len()); }
    }
    Ok(str_like(a[0], base.as_bytes()))
}
fn dirname_of(p: &str) -> String {
    let t = p.trim_end_matches('/');
    let d = match t.rfind('/') { Some(i) => { let d = t[..i].trim_end_matches('/'); if d.is_empty() { "/" } else { d } } None => if p.starts_with('/') { "/" } else { "." } };
    // a run of leading slashes is one root
    if d.starts_with("//") { format!("/{}", d.trim_start_matches('/')) } else { d.to_string() }
}
fn f_s_dirname(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let mut p = path_arg(c, a[0])?;
    let n = match a.get(1) { Some(&l) => { let n = num_to_long(c, l)?; if n < 0 { return arg_err(c, format!("negative level: {}", n)); } n } None => 1 };
    for _ in 0..n { let d = dirname_of(&p); if d == p { break; } p = d; }
    Ok(str_like(a[0], p.as_bytes()))
}
fn f_s_extname(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    let base = p.trim_end_matches('/').rsplit('/').next().unwrap_or("");
    let stem = base.trim_start_matches('.');
    let e = match stem.rfind('.') { Some(i) if i + 1 < stem.len() => stem[i..].to_string(), Some(i) if i > 0 => ".".to_string(), _ => String::new() };
    Ok(str_like(a[0], e.as_bytes()))
}
fn f_s_split(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let d = f_s_dirname(c, slf, &a[..1], b)?;
    let bn = f_s_basename(c, slf, &a[..1], b)?;
    Ok(ary_new_from(&[d, bn]))
}
fn f_s_join(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mut parts: Vec<Vec<u8>> = vec![];
    fn flat(c: &mut Ctx, v: Value, out: &mut Vec<Vec<u8>>, depth: usize) -> Result<(), Esc> {
        if depth > 1000 { return arg_err(c, "recursive array"); }
        if is_array(v) { for i in 0..ary_len(v) { let x = ary_get(v, i); if x == v { return arg_err(c, "recursive array"); } flat(c, x, out, depth + 1)?; } Ok(()) } else { out.push(path_arg(c, v)?.into_bytes()); Ok(()) }
    }
    for &x in a { flat(c, x, &mut parts, 0)?; }
    let mut out: Vec<u8> = vec![];
    for (i, p) in parts.iter().enumerate() {
        if i == 0 { out.extend_from_slice(p); continue; }
        match (out.ends_with(b"/"), p.starts_with(b"/")) { (true, true) => { while out.ends_with(b"/") { out.pop(); } out.extend_from_slice(p) } (false, false) => { out.push(b'/'); out.extend_from_slice(p); } _ => out.extend_from_slice(p) }
    }
    Ok(str_new(&out))
}
fn home_of(c: &mut Ctx, user: &str) -> Result<String, Esc> {
    if user.is_empty() { return match std::env::var("HOME") { Ok(h) => Ok(h), Err(_) => arg_err(c, "couldn't find HOME environment -- expanding '~'") }; }
    if let Ok(pw) = std::fs::read_to_string("/etc/passwd") { for l in pw.lines() { let f: Vec<&str> = l.split(':').collect(); if f.len() > 5 && f[0] == user { return Ok(f[5].to_string()); } } }
    arg_err(c, format!("user {} doesn't exist", user))
}
fn cwd() -> String { std::env::current_dir().map(|d| d.to_string_lossy().into_owned()).unwrap_or("/".into()) }
fn expand(c: &mut Ctx, a: &[Value], tilde: bool) -> Result<String, Esc> {
    let mut p = path_arg(c, a[0])?;
    if tilde && p.starts_with('~') {
        let end = p.find('/').unwrap_or(p.len());
        let h = home_of(c, &p[1..end])?;
        if !h.starts_with('/') { return arg_err(c, "non-absolute home"); }
        p = format!("{}{}", h, &p[end..]);
    }
    let full = if p.starts_with('/') { p } else {
        let base = if a.len() > 1 && a[1] != NIL { expand(c, &a[1..2], tilde)? } else { cwd() };
        format!("{}/{}", base, p)
    };
    let mut comps: Vec<&str> = vec![];
    for x in full.split('/') { match x { "" | "." => {} ".." => { comps.pop(); } y => comps.push(y) } }
    Ok(format!("/{}", comps.join("/")))
}
fn f_s_expand_path(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { check_argc(c, a, 1, 2)?; let r = expand(c, a, true)?; Ok(str_new(r.as_bytes())) }
fn f_s_absolute_path(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { check_argc(c, a, 1, 2)?; let r = expand(c, a, false)?; Ok(str_new(r.as_bytes())) }
fn f_s_absolute_path_p(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { let p = path_arg(c, a[0])?; Ok(Value::bool(p.starts_with('/'))) }
fn realpath_impl(c: &mut Ctx, a: &[Value], strict: bool) -> RV {
    check_argc(c, a, 1, 2)?;
    let p = expand(c, a, false)?;
    match std::fs::canonicalize(&p) {
        Ok(r) => Ok(str_new(r.to_string_lossy().as_bytes())),
        Err(e) => {
            if !strict { // the last component may be missing
                let d = dirname_of(&p);
                if let Ok(r) = std::fs::canonicalize(&d) { let base = p.rsplit('/').next().unwrap_or(""); return Ok(str_new(format!("{}/{}", r.to_string_lossy().trim_end_matches('/'), base).as_bytes())); }
            }
            Err(os_err(c, &e, &format!("rb_check_realpath_internal - {}", p)))
        }
    }
}
fn f_s_realpath(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { realpath_impl(c, a, true) }
fn f_s_realdirpath(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { realpath_impl(c, a, false) }
fn f_s_path(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { let p = path_arg(c, a[0])?; Ok(str_like(a[0], p.as_bytes())) }
fn f_s_open(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV { crate::builtins::io::open_with_block(c, slf, a, blk) }

// ---- File instance methods --------------------------------------------------------------------

fn f_size(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    funcall(c, s, intern(b"flush"), &[], std::ptr::null())?;
    let io = crate::builtins::io::get_io(c, s)?;
    match fd_meta(io.fd) { Ok(m) => Ok(Value::fix(m.len() as i64)), Err(e) => Err(os_err(c, &e, "")) }
}
fn f_truncate(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = num_to_long(c, a[0])?;
    funcall(c, s, intern(b"flush"), &[], std::ptr::null())?;
    let io = crate::builtins::io::get_io(c, s)?;
    if !io.writable { return err(c, vm().c_io_error, "not opened for writing"); }
    if unsafe { sys::ftruncate(io.fd, n) } < 0 { let p = io.path.clone(); return Err(last_err(c, &p)); }
    Ok(Value::fix(0))
}
fn f_flock(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let op = num_to_long(c, a[0])? as i32;
    funcall(c, s, intern(b"flush"), &[], std::ptr::null())?;
    let io = crate::builtins::io::get_io(c, s)?;
    loop {
        if unsafe { sys::flock(io.fd, op) } == 0 { return Ok(Value::fix(0)); }
        match sys::errno() { 4 => continue, 11 if op & 4 != 0 => return Ok(FALSE), _ => { let p = io.path.clone(); return Err(last_err(c, &p)); } }
    }
}
fn f_chmod(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let m = mode_arg(c, a[0])?;
    let io = crate::builtins::io::get_io(c, s)?;
    if unsafe { sys::fchmod(io.fd, m) } < 0 { let p = io.path.clone(); return Err(last_err(c, &p)); }
    Ok(Value::fix(0))
}
fn f_chown(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let (u, g) = (id_arg(c, a[0])?, id_arg(c, a[1])?);
    let io = crate::builtins::io::get_io(c, s)?;
    if unsafe { sys::fchown(io.fd, u, g) } < 0 { let p = io.path.clone(); return Err(last_err(c, &p)); }
    Ok(Value::fix(0))
}
macro_rules! fmeta_fn {
    ($name:ident, |$m:ident| $e:expr) => {
        fn $name(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let io = crate::builtins::io::get_io(c, s)?; match fd_meta(io.fd) { Ok($m) => Ok($e), Err(e) => Err(os_err(c, &e, "")) } }
    };
}
fmeta_fn!(f_mtime, |m| time_of(m.mtime(), m.mtime_nsec()));
fmeta_fn!(f_atime, |m| time_of(m.atime(), m.atime_nsec()));
fmeta_fn!(f_ctime, |m| time_of(m.ctime(), m.ctime_nsec()));

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
fn f_s_fnmatch(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 2, 3)?;
    let pv = expect_str(c, a[0])?;
    let s = path_arg(c, a[1])?;
    let flags = match a.get(2) { Some(&f) => num_to_long(c, f)?, None => 0 };
    let pat = str_bytes(pv);
    if flags & FNM_EXTGLOB != 0 {
        let mut alts = vec![];
        brace_expand(pat, flags & FNM_NOESCAPE != 0, &mut alts);
        return Ok(Value::bool(alts.iter().any(|p| fnm(p, s.as_bytes(), flags, true))));
    }
    Ok(Value::bool(fnm(pat, s.as_bytes(), flags, true)))
}

#[repr(C)] struct Dirent { ino: u64, off: i64, reclen: u16, typ: u8, name: [u8; 256] }
extern "C" { fn opendir(p: *const i8) -> *mut u8; fn readdir(d: *mut u8) -> *mut Dirent; fn closedir(d: *mut u8) -> i32; }
/// Directory entries in readdir order (with "." and ".."), like CRuby.
fn read_dir_raw(path: &str) -> Result<Vec<(Vec<u8>, u8)>, i64> {
    let d = unsafe { opendir(sys::cstr(path.as_bytes()).as_ptr()) };
    if d.is_null() { return Err(sys::errno()); }
    let mut out = vec![];
    loop {
        let e = unsafe { readdir(d) };
        if e.is_null() { break; }
        let e = unsafe { &*e };
        let n = e.name.iter().position(|&b| b == 0).unwrap_or(256);
        out.push((e.name[..n].to_vec(), e.typ));
    }
    unsafe { closedir(d); }
    Ok(out)
}
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
        let Ok(mut ents) = read_dir_raw(&dp) else { return };
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
    let Ok(mut ents) = read_dir_raw(&dp) else { return };
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
fn d_s_glob(c: &mut Ctx, _slf: Value, a: &[Value], blk: *const Block) -> RV {
    let (pos, opts) = match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    if pos.is_empty() { return argc_err(c, 0, 1, 2); }
    let mut flags = match pos.get(1) { Some(&f) if !is_array(pos[0]) || pos.len() == 2 && f.is_fixnum() => num_to_long(c, f)?, _ => 0 };
    let mut base = String::new();
    let mut sort = true;
    if let Some(h) = opts {
        if let Some(b) = crate::hash::hash_get_opt(c, h, sym_value("base"))? { if b != NIL { base = path_arg(c, b)?; } }
        if let Some(f) = crate::hash::hash_get_opt(c, h, sym_value("flags"))? { if f != NIL { flags |= num_to_long(c, f)?; } }
        if let Some(s) = crate::hash::hash_get_opt(c, h, sym_value("sort"))? { sort = s.truthy(); }
    }
    let _ = sort;
    let mut pats: Vec<Vec<u8>> = vec![];
    if is_array(pos[0]) { for i in 0..ary_len(pos[0]) { pats.push(path_arg(c, ary_get(pos[0], i))?.into_bytes()); } }
    else { let p = path_arg(c, pos[0])?; pats.extend(p.as_bytes().split(|&b| b == 0).map(|x| x.to_vec())); }
    let mut found = vec![];
    for p in &pats { glob_one(p, flags, &base, &mut found); }
    let out = ary_of_strs(found, crate::builtins::encoding::default_external());
    if blk.is_null() { return Ok(out); }
    for i in 0..ary_len(out) { yield1(c, blk, ary_get(out, i))?; }
    Ok(NIL)
}
fn d_s_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, opts) = match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    let mut args = vec![ary_new_from(pos)];
    if let Some(o) = opts { args.push(o); }
    d_s_glob(c, slf, &args, std::ptr::null())
}

// ---- Dir ----------------------------------------------------------------------------------------

fn d_s_pwd(c: &mut Ctx, _slf: Value, _a: &[Value], _b: *const Block) -> RV {
    match std::env::current_dir() { Ok(d) => Ok(str_new(d.to_string_lossy().as_bytes())), Err(e) => Err(os_err(c, &e, "rb_dir_getwd")) }
}
static mut CHDIR_DEPTH: i32 = 0;
fn chdir_to(c: &mut Ctx, p: &str) -> Result<(), Esc> {
    if unsafe { sys::chdir(sys::cstr(p.as_bytes()).as_ptr()) } < 0 { return Err(last_err(c, &format!("rb_dir_s_chdir - {}", p))); }
    Ok(())
}
fn d_s_chdir(c: &mut Ctx, _slf: Value, a: &[Value], blk: *const Block) -> RV {
    let p = match a.first() { Some(&v) => path_arg(c, v)?, None => home_of(c, "")? };
    if blk.is_null() {
        if unsafe { CHDIR_DEPTH } > 0 { crate::builtins::kernel::warn_str(c, "warning: conflicting chdir during another chdir block\n"); }
        chdir_to(c, &p)?; return Ok(Value::fix(0));
    }
    let old = cwd();
    chdir_to(c, &p)?;
    unsafe { CHDIR_DEPTH += 1; }
    let arg = str_new(p.as_bytes());
    let r = yield1(c, blk, arg);
    unsafe { CHDIR_DEPTH -= 1; }
    let (ek, ev) = (c.esc, c.esc_val);
    let back = chdir_to(c, &old);
    if r.is_err() { c.esc = ek; c.esc_val = ev; return r; }
    back?;
    r
}
fn d_s_mkdir(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let p = path_arg(c, a[0])?;
    let mode = match a.get(1) { Some(&m) if m != NIL => mode_arg(c, m)?, _ => 0o777 };
    use std::os::unix::fs::DirBuilderExt;
    match std::fs::DirBuilder::new().mode(mode).create(&p) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err(c, &e, &format!("rb_dir_s_mkdir - {}", p))) }
}
fn d_s_rmdir(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    match std::fs::remove_dir(&p) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err(c, &e, &format!("rb_dir_s_rmdir - {}", p))) }
}
fn dir_list(c: &mut Ctx, v: Value, children: bool) -> Result<Vec<Vec<u8>>, Esc> {
    let p = path_arg(c, v)?;
    match read_dir_raw(&p) {
        Ok(e) => Ok(e.into_iter().map(|x| x.0).filter(|n| !children || (n != b"." && n != b"..")).collect()),
        Err(e) => Err(raise_errno(c, e, &format!("rb_dir_initialize - {}", p))),
    }
}
fn strs(v: Vec<Vec<u8>>) -> Value { ary_of_strs(v, crate::builtins::encoding::default_external()) }
fn d_s_entries(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(strs(dir_list(c, a[0], false)?)) }
fn d_s_children(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(strs(dir_list(c, a[0], true)?)) }
fn d_s_each_child(c: &mut Ctx, s: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"each_child"), a, NIL); }
    let l = strs(dir_list(c, a[0], true)?);
    for i in 0..ary_len(l) { yield1(c, blk, ary_get(l, i))?; }
    Ok(NIL)
}
fn d_s_foreach(c: &mut Ctx, s: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"foreach"), a, NIL); }
    let l = strs(dir_list(c, a[0], false)?);
    for i in 0..ary_len(l) { yield1(c, blk, ary_get(l, i))?; }
    Ok(NIL)
}
fn d_s_empty(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    Ok(Value::bool(read_dir_raw(&p).map(|e| e.len() <= 2).unwrap_or(false)))
}
fn d_s_home(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let u = match a.first() { Some(&v) if v != NIL => to_rstring(c, v)?, _ => String::new() };
    let h = home_of(c, &u)?;
    Ok(str_new(h.as_bytes()))
}
fn d_s_exist(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { f_s_dir_p(c, s, a, b) }
fn iv(n: &str) -> crate::value::Sym { intern(n.as_bytes()) }
fn d_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, _) = match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    if pos.len() != 1 { return argc_err(c, pos.len(), 1, 1); }
    let p = path_arg(c, pos[0])?;
    let l = strs(dir_list(c, pos[0], false)?);
    ivar_set_raw(s, iv("@path"), str_new(p.as_bytes())); ivar_set_raw(s, iv("@ents"), l); ivar_set_raw(s, iv("@pos"), Value::fix(0)); ivar_set_raw(s, iv("@closed"), FALSE);
    Ok(NIL)
}
fn d_check(c: &mut Ctx, s: Value) -> Result<(Value, usize), Esc> {
    let e = ivar_get(s, iv("@ents"));
    if e == NIL { return err(c, vm().c_io_error, "uninitialized Dir"); }
    if ivar_get(s, iv("@closed")).truthy() { return err(c, vm().c_io_error, "closed directory"); }
    Ok((e, ivar_get(s, iv("@pos")).as_fix() as usize))
}
fn d_read(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (e, p) = d_check(c, s)?;
    if p >= ary_len(e) { return Ok(NIL); }
    ivar_set_raw(s, iv("@pos"), Value::fix(p as i64 + 1));
    Ok(str_dup(ary_get(e, p)))
}
fn d_each(c: &mut Ctx, s: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"each"), a, NIL); }
    loop { let v = d_read(c, s, &[], std::ptr::null())?; if v == NIL { break; } yield1(c, blk, v)?; }
    Ok(s)
}
fn d_children(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (e, _) = d_check(c, s)?;
    let out = ary_new();
    for i in 0..ary_len(e) { let x = ary_get(e, i); if str_bytes(x) != b"." && str_bytes(x) != b".." { ary_push(out, str_dup(x)); } }
    Ok(out)
}
fn d_each_child(c: &mut Ctx, s: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"each_child"), a, NIL); }
    let l = d_children(c, s, &[], std::ptr::null())?;
    for i in 0..ary_len(l) { yield1(c, blk, ary_get(l, i))?; }
    Ok(s)
}
fn d_rewind(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { d_check(c, s)?; ivar_set_raw(s, iv("@pos"), Value::fix(0)); Ok(s) }
fn d_tell(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let (_, p) = d_check(c, s)?; Ok(Value::fix(p as i64)) }
fn d_seek(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { d_check(c, s)?; let n = num_to_long(c, a[0])?; ivar_set_raw(s, iv("@pos"), Value::fix(n.max(0))); Ok(s) }
fn d_set_pos(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { d_seek(c, s, a, b)?; Ok(a[0]) }
fn d_close(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { ivar_set_raw(s, iv("@closed"), TRUE); Ok(NIL) }
fn d_path(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let p = ivar_get(s, iv("@path")); Ok(if p == NIL { NIL } else { str_dup(p) }) }
fn d_inspect(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let p = ivar_get(s, iv("@path"));
    let k = class_path(real_class_of(s));
    Ok(str_new(if p == NIL { format!("#<{}: uninitialized>", k) } else { format!("#<{}:{}>", k, String::from_utf8_lossy(str_bytes(p))) }.as_bytes()))
}
fn d_chdir(c: &mut Ctx, s: Value, _a: &[Value], blk: *const Block) -> RV { d_check(c, s)?; let p = ivar_get(s, iv("@path")); d_s_chdir(c, s, &[p], blk) }
fn d_s_open(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    let d = crate::builtins::object::c_new(c, slf, a, std::ptr::null())?;
    if blk.is_null() { return Ok(d); }
    let r = yield1(c, blk, d);
    ivar_set_raw(d, iv("@closed"), TRUE);
    r
}

// predicates shared by File and FileTest
const TESTS: &[(&str, CFunc, i32)] = &[("exist?", f_s_exist, 1), ("file?", f_s_file_p, 1), ("directory?", f_s_dir_p, 1), ("symlink?", f_s_symlink_p, 1), ("pipe?", f_s_pipe_p, 1),
        ("socket?", f_s_socket_p, 1), ("chardev?", f_s_chardev_p, 1), ("blockdev?", f_s_blockdev_p, 1), ("readable?", f_s_readable, 1), ("writable?", f_s_writable, 1),
        ("executable?", f_s_executable, 1), ("readable_real?", f_s_readable_real, 1), ("writable_real?", f_s_writable_real, 1), ("executable_real?", f_s_executable_real, 1),
        ("zero?", f_s_zero, 1), ("empty?", f_s_zero, 1), ("size?", f_s_size_p, 1), ("size", f_s_size, 1), ("setuid?", f_s_setuid, 1), ("setgid?", f_s_setgid, 1), ("sticky?", f_s_sticky, 1),
        ("owned?", f_s_owned, 1), ("grpowned?", f_s_grpowned, 1), ("world_readable?", f_s_world_readable, 1), ("world_writable?", f_s_world_writable, 1), ("identical?", f_s_identical, 2)];
pub fn init_filetest() {
    let vm = vm();
    if vm.m_file_test != NIL { return; }
    let ft = define_module("FileTest");
    vm.m_file_test = ft;
    for &(n, fun, ar) in TESTS { define_module_function(ft, n, fun, ar); }
}
pub fn init_stat() {
    let vm = vm();
    if vm.c_stat != NIL { return; }
    let f = vm.c_file;
    let st = define_class_under(f, "Stat", vm.c_object);
    rclass(st).alloc = AllocKind::None;
    include_module(st, vm.m_comparable);
    vm.c_stat = st;
    let sd = |n: &str, fun: CFunc, a: i32| define_cfunc(st, n, fun, a);
    define_private_cfunc(st, "initialize", st_initialize, 1);
    define_private_cfunc(st, "initialize_copy", st_init_copy, 1);
    sd("size", st_size, 0); sd("mode", st_mode, 0); sd("uid", st_uid, 0); sd("gid", st_gid, 0); sd("ino", st_ino, 0); sd("dev", st_dev, 0);
    sd("dev_major", st_dev_major, 0); sd("dev_minor", st_dev_minor, 0); sd("rdev", st_rdev, 0); sd("nlink", st_nlink, 0); sd("blksize", st_blksize, 0);
    sd("blocks", st_blocks, 0); sd("atime", st_atime, 0); sd("mtime", st_mtime, 0); sd("ctime", st_ctime, 0); sd("birthtime", st_birthtime, 0);
    sd("file?", st_file, 0); sd("directory?", st_dir, 0); sd("symlink?", st_symlink, 0); sd("pipe?", st_pipe, 0); sd("socket?", st_socket, 0);
    sd("chardev?", st_chardev, 0); sd("blockdev?", st_blockdev, 0); sd("readable?", st_readable, 0); sd("writable?", st_writable, 0);
    sd("executable?", st_executable, 0); sd("readable_real?", st_readable_real, 0); sd("writable_real?", st_writable_real, 0);
    sd("executable_real?", st_executable_real, 0); sd("world_readable?", st_world_readable, 0); sd("world_writable?", st_world_writable, 0);
    sd("zero?", st_zero, 0); sd("size?", st_size_p, 0); sd("ftype", st_ftype, 0); sd("setuid?", st_setuid, 0); sd("setgid?", st_setgid, 0);
    sd("sticky?", st_sticky, 0); sd("owned?", st_owned, 0); sd("grpowned?", st_grpowned, 0); sd("<=>", st_cmp, 1); sd("inspect", st_inspect, 0);
}
pub fn init() {
    let vm = vm();
    let (io, f) = (vm.c_io, vm.c_file);
    crate::builtins::prelude::lazy_const(f, "Stat", "koruby/stat");
    crate::builtins::prelude::lazy_const(vm.c_object, "FileTest", "koruby/filetest");
    for &(n, fun, ar) in TESTS { define_singleton_cfunc(f, n, fun, ar); }
    let df = |n: &str, fun: CFunc, a: i32| define_singleton_cfunc(f, n, fun, a);
    df("open", f_s_open, -1); df("stat", f_s_stat, 1); df("lstat", f_s_lstat, 1); df("mtime", f_s_mtime, 1); df("atime", f_s_atime, 1); df("ctime", f_s_ctime, 1);
    df("birthtime", f_s_birthtime, 1); df("ftype", f_s_ftype, 1); df("delete", f_s_delete, -1); df("unlink", f_s_delete, -1); df("rename", f_s_rename, 2);
    df("symlink", f_s_symlink, 2); df("link", f_s_link, 2); df("readlink", f_s_readlink, 1); df("chmod", f_s_chmod, -1); df("chown", f_s_chown, -1);
    df("lchown", f_s_lchown, -1); df("lchmod", f_s_lchmod, -1); df("truncate", f_s_truncate, 2); df("umask", f_s_umask, -1); df("mkfifo", f_s_mkfifo, -1);
    df("utime", f_s_utime, -1); df("lutime", f_s_lutime, -1); df("basename", f_s_basename, -1); df("dirname", f_s_dirname, -1); df("extname", f_s_extname, 1);
    df("split", f_s_split, 1); df("join", f_s_join, -1); df("expand_path", f_s_expand_path, -1); df("absolute_path", f_s_absolute_path, -1);
    df("absolute_path?", f_s_absolute_path_p, 1); df("realpath", f_s_realpath, -1); df("realdirpath", f_s_realdirpath, -1); df("path", f_s_path, 1);
    df("fnmatch", f_s_fnmatch, -1); df("fnmatch?", f_s_fnmatch, -1);
    let di = |n: &str, fun: CFunc, a: i32| define_cfunc(f, n, fun, a);
    di("size", f_size, 0); di("truncate", f_truncate, 1); di("flock", f_flock, 1); di("chmod", f_chmod, 1); di("chown", f_chown, 2); di("mtime", f_mtime, 0);
    di("atime", f_atime, 0); di("ctime", f_ctime, 0); di("lstat", f_lstat, 0);
    define_cfunc(io, "stat", f_stat, 0);
    const_set(f, intern(b"SEPARATOR"), str_new_frozen(b"/"));
    const_set(f, intern(b"Separator"), str_new_frozen(b"/"));
    const_set(f, intern(b"ALT_SEPARATOR"), NIL);
    const_set(f, intern(b"PATH_SEPARATOR"), str_new_frozen(b":"));
    let fc = define_module_under(f, "Constants");
    for (n, v) in [("RDONLY", 0), ("WRONLY", 1), ("RDWR", 2), ("APPEND", 0o2000), ("CREAT", 0o100), ("EXCL", 0o200), ("NONBLOCK", 0o4000), ("TRUNC", 0o1000),
                   ("NOCTTY", 0o400), ("BINARY", 0), ("SHARE_DELETE", 0), ("SYNC", 0o4010000), ("DSYNC", 0o10000), ("NOFOLLOW", 0o400000), ("DIRECTORY", 0o200000),
                   ("TMPFILE", 0o20200000), ("NOATIME", 0o1000000), ("CLOEXEC", 0o2000000), ("LOCK_SH", 1), ("LOCK_EX", 2), ("LOCK_NB", 4), ("LOCK_UN", 8),
                   ("FNM_NOESCAPE", 1), ("FNM_PATHNAME", 2), ("FNM_DOTMATCH", 4), ("FNM_CASEFOLD", 8), ("FNM_EXTGLOB", 16), ("FNM_SYSCASE", 0), ("FNM_SHORTNAME", 0)] {
        const_set(fc, intern(n.as_bytes()), Value::fix(v));
    }
    const_set(fc, intern(b"NULL"), str_new_frozen(b"/dev/null"));
    include_module(io, fc);
    let dir = define_class("Dir", vm.c_object);
    include_module(dir, vm.m_enumerable);
    vm.c_dir = dir;
    let dd = |n: &str, fun: CFunc, a: i32| define_singleton_cfunc(dir, n, fun, a);
    dd("pwd", d_s_pwd, 0); dd("getwd", d_s_pwd, 0); dd("glob", d_s_glob, -1); dd("[]", d_s_aref, -1); dd("exist?", d_s_exist, 1); dd("mkdir", d_s_mkdir, -1);
    dd("rmdir", d_s_rmdir, 1); dd("delete", d_s_rmdir, 1); dd("unlink", d_s_rmdir, 1); dd("entries", d_s_entries, -1); dd("children", d_s_children, -1);
    dd("each_child", d_s_each_child, -1); dd("foreach", d_s_foreach, -1); dd("empty?", d_s_empty, 1); dd("home", d_s_home, -1); dd("chdir", d_s_chdir, -1);
    dd("open", d_s_open, -1);
    let dm = |n: &str, fun: CFunc, a: i32| define_cfunc(dir, n, fun, a);
    define_private_cfunc(dir, "initialize", d_initialize, -1);
    dm("read", d_read, 0); dm("each", d_each, 0); dm("children", d_children, 0); dm("each_child", d_each_child, 0); dm("rewind", d_rewind, 0);
    dm("tell", d_tell, 0); dm("pos", d_tell, 0); dm("seek", d_seek, 1); dm("pos=", d_set_pos, 1); dm("close", d_close, 0); dm("path", d_path, 0);
    dm("to_path", d_path, 0); dm("inspect", d_inspect, 0); dm("chdir", d_chdir, 0);
}
