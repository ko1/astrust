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
fn os_err_at(c: &mut Ctx, e: &std::io::Error, pre: &str, p: impl crate::builtins::io::ErrPath) -> Esc { crate::builtins::io::raise_errno_at(c, io_errno(e), pre, p) }
fn last_err_at(c: &mut Ctx, pre: &str, p: impl crate::builtins::io::ErrPath) -> Esc { let e = sys::errno(); crate::builtins::io::raise_errno_at(c, e, pre, p) }
/// "(x, y)" of two paths, for the messages of rename / link
fn pair(c: &mut Ctx, x: &PathArg, y: &PathArg) -> Result<PathArg, Esc> {
    let mut v = b"(".to_vec(); v.extend_from_slice(x.as_bytes()); v.extend_from_slice(b", "); v.extend_from_slice(y.as_bytes()); v.push(b')');
    let (xa, ya) = (is_ascii(x.as_bytes()), is_ascii(y.as_bytes()));
    if !xa && !ya && x.1 != y.1 {
        use crate::builtins::encoding::enc_name_msg;
        return err(c, c.vm().c_compat_error.get(), format!("incompatible character encodings: {} and {}", enc_name_msg(x.1), enc_name_msg(y.1)));
    }
    Ok(PathArg(v, if !xa { x.1 } else { y.1 }))
}
fn new_stat(c: &mut Ctx, md: std::fs::Metadata) -> Value { init_stat(c); c.alloc(c.vm().c_stat.get(), Body::Data(Box::new(RStat { md: Some(md) }))) }
fn fd_meta(fd: i32) -> std::io::Result<std::fs::Metadata> {
    sys::fd_metadata(fd)
}
/// Metadata of a path or an IO argument.
fn meta_of(c: &mut Ctx, v: Value, follow: bool) -> Result<Result<std::fs::Metadata, (std::io::Error, PathArg)>, Esc> {
    // rb_stat: an IO, or what to_io gives, is fstat'ed
    let v = if rio(c, v).is_none() && v.is_heap() && !is_string(v) && respond_to(c, v, intern(b"to_io"), true) { funcall(c, v, intern(b"to_io"), &[], std::ptr::null())? } else { v };
    if let Some(io) = rio(c, v) {
        if io.closed { return err(c, c.vm().c_io_error.get(), "closed stream"); }
        return Ok(fd_meta(io.fd).map_err(|e| (e, PathArg(io.path.clone(), io.path_enc))));
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
stat_fn!(st_rdev_major, |c, m| Value::fix((((m.rdev() >> 32) & 0xffff_f000) | ((m.rdev() >> 8) & 0xfff)) as i64));
stat_fn!(st_rdev_minor, |c, m| Value::fix((((m.rdev() >> 12) & 0xffff_ff00) | (m.rdev() & 0xff)) as i64));
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
stat_fn!(st_grpowned, |c, m| Value::bool(group_member(m.gid())));
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
    match std::fs::metadata(&p) { Ok(m) => { if let Some(st) = c.data_mut::<RStat>(s) { st.md = Some(m); } Ok(NIL) } Err(e) => Err(os_err_at(c, &e, "rb_stat_init - ", &p)) }
}
fn st_init_copy(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let Some(md) = c.data::<RStat>(a[0]).map(|o| o.md.clone()) else { return type_err(c, "initialize_copy should take same class object") };
    if let Some(st) = c.data_mut::<RStat>(s) { st.md = md; }
    Ok(s)
}
/// File::Stat#dup / #clone: an empty Stat of the same class; initialize_copy fills it.
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if c.data::<RStat>(v).is_none() { return Ok(None); }
    let k = real_class_of(c, v);
    alloc_data(c, k)
}
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let st = c.vm().c_stat.get();
    if st != NIL && crate::builtins::inherits(c, klass, st) { return Ok(Some(c.alloc(klass, Body::Data(Box::new(RStat { md: None }))))); }
    Ok(None)
}

// ---- File class methods -----------------------------------------------------------------------

fn f_s_stat(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    match meta_of(c, a[0], true)? { Ok(m) => Ok(new_stat(c, m)), Err((e, p)) => Err(os_err_at(c, &e, "rb_file_s_stat - ", &p)) }
}
fn f_s_lstat(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    match meta_of(c, a[0], false)? { Ok(m) => Ok(new_stat(c, m)), Err((e, p)) => Err(os_err_at(c, &e, "rb_file_s_lstat - ", &p)) }
}
fn f_stat(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let fd = crate::builtins::io::get_io(c, s)?.fd;
    match fd_meta(fd) { Ok(m) => Ok(new_stat(c, m)), Err(e) => Err(os_err(c, &e, "")) }
}
fn f_lstat(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let p = { let io = crate::builtins::io::get_io(c, s)?; PathArg(io.path.clone(), io.path_enc) };
    match std::fs::symlink_metadata(&p) { Ok(m) => Ok(new_stat(c, m)), Err(e) => Err(os_err_at(c, &e, "rb_file_lstat - ", &p)) }
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
test_fn!(f_s_grpowned, true, |m| Value::bool(group_member(m.gid())));
/// rb_group_member: the effective gid or one of the supplementary groups.
fn group_member(g: u32) -> bool {
    if g == sys::getegid() { return true; }
    let n = sys::getgroups(&mut []);
    if n <= 0 { return false; }
    let mut l = vec![0u32; n as usize];
    let n = sys::getgroups(&mut l);
    n > 0 && l[..n as usize].contains(&g)
}
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
        Err((e, p)) => Err(os_err_at(c, &e, "rb_file_s_birthtime - ", &p)),
    }
}
fn birthtime(c: &mut Ctx, m: &std::fs::Metadata) -> RV {
    // statx without STATX_BTIME in the answer: the filesystem keeps no birth time
    match m.created().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()) { Some(d) => Ok(time_of(c, d.as_secs() as i64, d.subsec_nanos() as i64)), None => err(c, c.vm().c_not_impl_error.get(), "birthtime is unimplemented on this filesystem") }
}
fn f_birthtime(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let fd = crate::builtins::io::get_io(c, s)?.fd;
    match fd_meta(fd) { Ok(m) => birthtime(c, &m), Err(e) => Err(os_err(c, &e, "")) }
}
fn f_s_ftype(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    match meta_of(c, a[0], false)? { Ok(m) => Ok(c.str_new(ftype(&m).as_bytes())), Err((e, p)) => Err(os_err_at(c, &e, "rb_file_s_ftype - ", &p)) }
}
fn f_s_delete(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    for &x in a { let p = path_arg(c, x)?; if let Err(e) = std::fs::remove_file(&p) { return Err(os_err_at(c, &e, "apply2files - ", &p)); } }
    Ok(Value::fix(a.len() as i64))
}
fn f_s_rename(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let (x, y) = (path_arg(c, a[0])?, path_arg(c, a[1])?);
    match std::fs::rename(&x, &y) { Ok(_) => Ok(Value::fix(0)), Err(e) => { let p = pair(c, &x, &y)?; Err(os_err_at(c, &e, "rb_file_s_rename - ", &p)) } }
}
fn f_s_symlink(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let (x, y) = (path_arg(c, a[0])?, path_arg(c, a[1])?);
    match std::os::unix::fs::symlink(&x, &y) { Ok(_) => Ok(Value::fix(0)), Err(e) => { let p = pair(c, &x, &y)?; Err(os_err_at(c, &e, "rb_file_s_symlink - ", &p)) } }
}
fn f_s_link(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let (x, y) = (path_arg(c, a[0])?, path_arg(c, a[1])?);
    match std::fs::hard_link(&x, &y) { Ok(_) => Ok(Value::fix(0)), Err(e) => { let p = pair(c, &x, &y)?; Err(os_err_at(c, &e, "rb_file_s_link - ", &p)) } }
}
fn f_s_readlink(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?;
    match std::fs::read_link(&p) { Ok(t) => Ok(c.str_new(t.as_os_str().as_bytes())), Err(e) => Err(os_err_at(c, &e, "rb_readlink - ", &p)) }
}
fn mode_arg(c: &mut Ctx, v: Value) -> Result<u32, Esc> { Ok(num_to_long(c, v)? as u32) }
fn f_s_chmod(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let m = mode_arg(c, a[0])?;
    for &x in &a[1..] { let p = path_arg(c, x)?; if sys::chmod(&p.cstr(), m) < 0 { return Err(last_err_at(c, "apply2files - ", &p)); } }
    Ok(Value::fix(a.len() as i64 - 1))
}
fn id_arg(c: &mut Ctx, v: Value) -> Result<u32, Esc> { if v == NIL { Ok(u32::MAX) } else { Ok(num_to_long(c, v)? as u32) } }
fn f_s_chown(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.len() < 2 { return argc_err(c, a.len(), 2, -1); }
    let (u, g) = (id_arg(c, a[0])?, id_arg(c, a[1])?);
    for &x in &a[2..] { let p = path_arg(c, x)?; if sys::chown(&p.cstr(), u, g) < 0 { return Err(last_err_at(c, "apply2files - ", &p)); } }
    Ok(Value::fix(a.len() as i64 - 2))
}
fn f_s_lchown(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.len() < 2 { return argc_err(c, a.len(), 2, -1); }
    let (u, g) = (id_arg(c, a[0])?, id_arg(c, a[1])?);
    for &x in &a[2..] { let p = path_arg(c, x)?; if sys::lchown(&p.cstr(), u, g) < 0 { return Err(last_err_at(c, "apply2files - ", &p)); } }
    Ok(Value::fix(a.len() as i64 - 2))
}
fn f_s_lchmod(c: &mut Ctx, _slf: Value, _a: &[Value], _b: Blk) -> RV { err(c, c.vm().c_not_impl_error.get(), "lchmod() function is unimplemented on this machine") }
fn f_s_truncate(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?; let n = num_to_long(c, a[1])?;
    if sys::truncate(&p.cstr(), n) < 0 { return Err(last_err_at(c, "rb_file_s_truncate - ", &p)); }
    Ok(Value::fix(0))
}
fn f_s_umask(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    if let Some(&m) = a.first() { let m = mode_arg(c, m)?; return Ok(Value::fix(sys::umask(m) as i64)); }
    let old = sys::umask(0); sys::umask(old);
    Ok(Value::fix(old as i64))
}
fn f_s_mkfifo(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?;
    let m = match a.get(1) { Some(&m) => mode_arg(c, m)?, None => 0o666 };
    if sys::mkfifo(&p.cstr(), m) < 0 { return Err(last_err_at(c, "rb_file_s_mkfifo - ", &p)); }
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
    for &x in &a[2..] { let p = path_arg(c, x)?; if sys::utimensat_cwd(&p.cstr(), &ts, if nofollow { 0x100 } else { 0 }) < 0 { return Err(last_err_at(c, "apply2files - ", &p)); } }
    Ok(Value::fix(a.len() as i64 - 2))
}
fn f_s_utime(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { utime_impl(c, a, false) }
fn f_s_lutime(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { utime_impl(c, a, true) }

// paths are bytes (CRuby takes any bytes but NUL)
fn trim_end(b: &[u8]) -> &[u8] { let mut n = b.len(); while n > 0 && b[n - 1] == b'/' { n -= 1; } &b[..n] }
fn trim_start(b: &[u8]) -> &[u8] { let n = b.iter().position(|&x| x != b'/').unwrap_or(b.len()); &b[n..] }
fn last_comp(b: &[u8]) -> &[u8] { match b.iter().rposition(|&x| x == b'/') { Some(i) => &b[i + 1..], None => b } }
fn str_like(c: &mut Ctx, src: Value, b: &[u8]) -> Value { if is_string(src) { let e = c.str_enc(src); c.str_new_enc(b, e) } else { c.str_new(b) } }
/// rb_get_path: the path as a String (to_path / to_str), without NUL, in an ASCII-compatible encoding.
fn path_str(c: &mut Ctx, v: Value) -> RV {
    let sv = path_str_nul_ok(c, v)?;
    if c.str_bytes(sv).contains(&0) { return arg_err(c, "path name contains null byte"); }
    Ok(sv)
}
/// `i` is a character boundary of `b` in `enc` (a suffix must start at one).
fn char_boundary(b: &[u8], enc: u8, i: usize) -> bool {
    if crate::string::is_ascii(b) || crate::builtins::encoding::max_len(enc) == 1 { return true; }
    let mut k = 0;
    while k < i { k += crate::builtins::encoding::mbclen(enc, b, k).max(1); }
    k == i
}
/// rb_get_path without the NUL check (File.join and basename's suffix do their own).
fn path_str_nul_ok(c: &mut Ctx, v: Value) -> RV {
    let sv = if is_string(v) { v } else if respond_to(c, v, intern(b"to_path"), true) { let p = funcall(c, v, intern(b"to_path"), &[], std::ptr::null())?; expect_str(c, p)? } else { expect_str(c, v)? };
    let enc = c.str_enc(sv);
    if !crate::builtins::encoding::ascii_compat(enc) {
        let d = inspect_str(c, sv)?;
        return err(c, c.vm().c_compat_error.get(), format!("path name must be ASCII-compatible ({}): {}", crate::builtins::encoding::enc_name(enc), d));
    }
    Ok(sv)
}
fn f_s_basename(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let sv = path_str(c, a[0])?;
    let p = c.str_bytes(sv).to_vec();
    let t = trim_end(&p);
    let mut base = if t.is_empty() && p.starts_with(b"/") { b"/".to_vec() } else { last_comp(t).to_vec() };
    if a.len() > 1 {
        let ss = path_str_nul_ok(c, a[1])?;
        let suf = c.str_bytes(ss).to_vec();
        let enc = c.str_enc(sv);
        if suf == b".*" { if let Some(i) = base.iter().rposition(|&x| x == b'.') { if i > 0 && char_boundary(&base, enc, i) { base.truncate(i); } } }
        else if base.ends_with(&suf) && base != suf && char_boundary(&base, enc, base.len() - suf.len()) { base.truncate(base.len() - suf.len()); }
    }
    Ok(str_like(c, sv, &base))
}
fn dirname_of(p: &[u8]) -> Vec<u8> {
    let t = trim_end(p);
    let d: &[u8] = match t.iter().rposition(|&x| x == b'/') { Some(i) => { let d = trim_end(&t[..i]); if d.is_empty() { b"/" } else { d } } None => if p.starts_with(b"/") { b"/" } else { b"." } };
    // a run of leading slashes is one root
    if d.starts_with(b"//") { let mut o = b"/".to_vec(); o.extend_from_slice(trim_start(d)); o } else { d.to_vec() }
}
fn f_s_dirname(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let sv = path_str(c, a[0])?;
    let mut p = c.str_bytes(sv).to_vec();
    let n = match a.get(1) { Some(&l) => { let n = num_to_long(c, l)?; if n < 0 { return arg_err(c, format!("negative level: {}", n)); } n } None => 1 };
    for _ in 0..n { let d = dirname_of(&p); if d == p { break; } p = d; }
    Ok(str_like(c, sv, &p))
}
fn f_s_extname(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?.0;
    let base = last_comp(trim_end(&p));
    let stem = &base[base.iter().position(|&x| x != b'.').unwrap_or(base.len())..];
    let e: &[u8] = match stem.iter().rposition(|&x| x == b'.') { Some(i) if i + 1 < stem.len() => &stem[i..], Some(i) if i > 0 => b".", _ => b"" };
    Ok(str_like(c, a[0], e))
}
fn f_s_split(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let sv = path_str(c, a[0])?;
    let d = f_s_dirname(c, slf, &[sv], b)?;
    let bn = f_s_basename(c, slf, &[sv], b)?;
    Ok(c.ary_new_from(&[d, bn]))
}
/// rb_file_join: arrays join recursively; one separator between parts (a part's leading
/// separator replaces the trailing ones before it); the first part's encoding.
fn join_parts(c: &mut Ctx, a: &[Value], depth: usize, enc: &mut Option<u8>) -> Result<Vec<u8>, Esc> {
    if depth > 1000 { return arg_err(c, "recursive array"); }
    let mut out: Vec<u8> = vec![];
    for (i, &x) in a.iter().enumerate() {
        let part = if is_array(x) {
            let items = c.ary(x).to_vec();
            if items.contains(&x) { return arg_err(c, "recursive array"); }
            join_parts(c, &items, depth + 1, enc)?
        } else {
            let sv = path_str_nul_ok(c, x)?;
            if enc.is_none() { *enc = Some(c.str_enc(sv)); }
            let b = c.str_bytes(sv).to_vec();
            if b.contains(&0) { return arg_err(c, "string contains null byte"); }
            b
        };
        if i > 0 {
            if part.first() == Some(&b'/') { while out.last() == Some(&b'/') { out.pop(); } }
            else if out.last() != Some(&b'/') { out.push(b'/'); }
        }
        out.extend_from_slice(&part);
    }
    Ok(out)
}
fn f_s_join(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut enc = None;
    let out = join_parts(c, a, 0, &mut enc)?;
    Ok(c.str_new_enc(&out, enc.unwrap_or(ENC_BINARY)))
}
fn home_of(c: &mut Ctx, user: &str) -> Result<Vec<u8>, Esc> {
    // without HOME: the password entry of the real uid (rb_default_home_dir)
    if user.is_empty() { if let Some(h) = std::env::var_os("HOME") { return Ok(h.into_vec()); } }
    let pw = if user.is_empty() { sys::getpwuid(sys::getuid()) } else { std::ffi::CString::new(user).ok().and_then(|u| sys::getpwnam(&u)) };
    if let Some(pw) = pw { return Ok(pw.dir); }
    if user.is_empty() { return arg_err(c, "couldn't find HOME environment -- expanding '~'"); }
    arg_err(c, format!("user {} doesn't exist", user))
}
fn cwd() -> Vec<u8> { std::env::current_dir().map(|d| d.into_os_string().into_vec()).unwrap_or(b"/".to_vec()) }
/// rb_file_expand_path_internal's normalization (Unix): `buf[..p]` is the root or base directory
/// (a NUL after the end), `s` the rest of the path.
fn expand_norm(mut buf: Vec<u8>, mut p: usize, s: &[u8]) -> Vec<u8> {
    fn at(b: &[u8], i: usize) -> u8 { *b.get(i).unwrap_or(&0) }
    fn put(b: &mut Vec<u8>, i: usize, x: u8) { if b.len() <= i { b.resize(i + 1, 0); } b[i] = x; }
    fn copy(b: &mut Vec<u8>, p: &mut usize, src: &[u8]) {
        let skip = (at(b, *p) != 0) as usize;
        *p += skip;
        b.truncate(*p);
        b.extend_from_slice(src);
        *p += src.len();
    }
    if p > 0 && buf[p - 1] == b'/' { p -= 1; } else { buf.truncate(p); put(&mut buf, p, b'/'); }
    buf.truncate(p + 1);
    put(&mut buf, p + 1, 0);
    let root = 0;
    let (mut i, mut b) = (0, 0);
    while i < s.len() {
        match s[i] {
            b'.' => {
                let begin = b == i;
                i += 1;
                if begin {
                    if i == s.len() { b = i; continue; }
                    match s[i] {
                        b'.' if i + 1 == s.len() || s[i + 1] == b'/' => {
                            // go back to the parent (dir.c strrdirsep over buf[root..p])
                            put(&mut buf, p, 0);
                            let mut cur = p as isize - 1;
                            while cur >= root as isize && buf[cur as usize] == b'/' { cur -= 1; }
                            let mut found = None;
                            while cur >= root as isize {
                                if buf[cur as usize] == b'/' { while cur > root as isize && buf[cur as usize - 1] == b'/' { cur -= 1; } found = Some(cur as usize); break; }
                                cur -= 1;
                            }
                            match found { None => put(&mut buf, p, b'/'), Some(n) => p = n }
                            i += 1; b = i;
                        }
                        b'/' => { i += 1; b = i; }
                        _ => {}
                    }
                }
            }
            b'/' => {
                if i > b { copy(&mut buf, &mut p, &s[b..i]); put(&mut buf, p, b'/'); }
                i += 1; b = i;
            }
            _ => i += 1,
        }
    }
    if i > b { copy(&mut buf, &mut p, &s[b..i]); buf.truncate(p); }
    // keep the root separator
    let end = p + (at(&buf, p) != 0) as usize;
    let mut sr = 0; while sr < end && at(&buf, sr) == b'/' { sr += 1; }
    if sr >= 1 && p == sr - 1 { p += 1; }
    buf.resize(p.max(buf.len()), 0);
    buf.truncate(p);
    buf
}
/// The trailing-separator trim of a base directory (chompdirsep(skiproot(buf))): its end.
fn base_end(b: &[u8]) -> usize {
    let mut i = 0; while i < b.len() && b[i] == b'/' { i += 1; }
    let mut q = i;
    while q < b.len() {
        if b[q] == b'/' { let last = q; while q < b.len() && b[q] == b'/' { q += 1; } if q >= b.len() { return last; } } else { q += 1; }
    }
    q
}
/// File.expand_path (tilde) / absolute_path: (bytes, encoding).
pub(crate) fn expand_enc(c: &mut Ctx, a: &[Value], tilde: bool) -> Result<(Vec<u8>, u8), Esc> {
    let sv = path_str(c, a[0])?;
    let mut enc = c.str_enc(sv);
    let f = c.str_bytes(sv).to_vec();
    let relative = |c: &mut Ctx| -> Result<(), Esc> {
        let de = crate::builtins::encoding::default_external();
        if !crate::builtins::encoding::ascii_compat(de) {
            return err(c, c.vm().c_compat_error.get(), format!("incompatible character encodings: {} and {}", crate::builtins::encoding::enc_name(enc), crate::builtins::encoding::enc_name(de)));
        }
        Ok(())
    };
    let (buf, p, rest): (Vec<u8>, usize, &[u8]) = if tilde && f.first() == Some(&b'~') {
        let end = f.iter().position(|&x| x == b'/').unwrap_or(f.len());
        let user = String::from_utf8_lossy(&f[1..end]).into_owned();
        let h = home_of(c, &user)?;
        if !h.starts_with(b"/") {
            if user.is_empty() { return arg_err(c, "non-absolute home"); }
            return arg_err(c, format!("non-absolute home of {}", user));
        }
        relative(c)?;
        if enc == ENC_ASCII { enc = crate::builtins::encoding::default_external(); }   // the home directory's (filesystem) encoding
        // "~" and "~/" skip the separator; "~user" keeps it for the loop
        let rest = if user.is_empty() { &f[(end + 1).min(f.len())..] } else { &f[end..] };
        let n = h.len();
        (h, n, rest)
    } else if f.first() != Some(&b'/') {
        let base = if a.len() > 1 && a[1] != NIL {
            // fs_enc_check: an ASCII-only path takes the base directory's encoding
            let (d, de) = expand_enc(c, &a[1..2], tilde)?;
            if crate::string::is_ascii(&f) { enc = de; }
            d
        } else { relative(c)?; if enc == ENC_ASCII { enc = crate::builtins::encoding::default_external(); } cwd() };
        let n = base_end(&base);
        (base, n, &f[..])
    } else {
        let n = f.iter().position(|&x| x != b'/').unwrap_or(f.len());
        (vec![b'/'; n], n, &f[n..])
    };
    Ok((expand_norm(buf, p, rest), enc))
}
pub(crate) fn expand(c: &mut Ctx, a: &[Value], tilde: bool) -> Result<Vec<u8>, Esc> { Ok(expand_enc(c, a, tilde)?.0) }
fn f_s_expand_path(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { check_argc(c, a, 1, 2)?; let (r, e) = expand_enc(c, a, true)?; Ok(c.str_new_enc(&r, e)) }
fn f_s_absolute_path(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { check_argc(c, a, 1, 2)?; let (r, e) = expand_enc(c, a, false)?; Ok(c.str_new_enc(&r, e)) }
fn f_s_absolute_path_p(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { let p = path_arg(c, a[0])?; Ok(Value::bool(p.as_bytes().starts_with(b"/"))) }
/// file.c realpath_rec: resolve `unresolved` onto `resolved` component by component; a missing
/// last component is allowed unless `strict`. `seen`: links being or already resolved (None: in progress).
fn realpath_rec(resolved: &mut Vec<u8>, unresolved: &[u8], strict: bool, last: bool, seen: &mut Vec<(Vec<u8>, Option<Vec<u8>>)>) -> Result<(), (i64, Vec<u8>)> {
    let comps: Vec<&[u8]> = unresolved.split(|&b| b == b'/').filter(|x| !x.is_empty()).collect();
    for (k, &comp) in comps.iter().enumerate() {
        if comp == b"." { continue; }
        if comp == b".." {
            if let Some(i) = resolved.iter().rposition(|&b| b == b'/') { resolved.truncate(if i == 0 { 1 } else { i }); }
            continue;
        }
        let mut test = resolved.clone();
        if test.last() != Some(&b'/') { test.push(b'/'); }
        test.extend_from_slice(comp);
        let is_last = last && comps[k + 1..].iter().all(|x| *x == b".");
        if let Some((_, r)) = seen.iter().find(|(p, _)| *p == test) {
            match r { None => return Err((40, test)), Some(r) => { *resolved = r.clone(); continue; } }
        }
        match std::fs::symlink_metadata(std::ffi::OsStr::from_bytes(&test)) {
            Err(e) => {
                let en = e.raw_os_error().unwrap_or(2) as i64;
                if en == 2 && !strict && is_last { *resolved = test; continue; }
                return Err((en, test));
            }
            Ok(m) if m.file_type().is_symlink() => {
                let target = match std::fs::read_link(std::ffi::OsStr::from_bytes(&test)) { Ok(t) => t.into_os_string().into_vec(), Err(e) => return Err((e.raw_os_error().unwrap_or(2) as i64, test)) };
                seen.push((test.clone(), None));
                if target.first() == Some(&b'/') { *resolved = b"/".to_vec(); }
                realpath_rec(resolved, &target, strict, is_last, seen)?;
                if let Some(e) = seen.iter_mut().find(|(p, _)| *p == test) { e.1 = Some(resolved.clone()); }
            }
            Ok(_) => *resolved = test,
        }
    }
    Ok(())
}
fn realpath_impl(c: &mut Ctx, a: &[Value], strict: bool) -> RV {
    check_argc(c, a, 1, 2)?;
    let sv = path_str(c, a[0])?;
    let enc = c.str_enc(sv);
    let orig = c.str_bytes(sv).to_vec();
    let mut ea = a.to_vec(); ea[0] = sv;
    let p = PathArg(expand(c, &ea, false)?, enc);
    if strict {
        return match std::fs::canonicalize(&p) {
            Ok(r) => Ok(c.str_new_enc(r.as_os_str().as_bytes(), enc)),
            Err(e) => Err(os_err_at(c, &e, "rb_check_realpath_internal - ", if a.len() > 1 && a[1] != NIL { (p.as_bytes(), p.1) } else { (&orig[..], enc) })),
        };
    }
    let mut r = b"/".to_vec();
    match realpath_rec(&mut r, p.as_bytes(), false, true, &mut vec![]) {
        // a resolved path the argument's encoding cannot hold stays in the filesystem encoding
        Ok(()) => {
            let fs = crate::builtins::encoding::default_external();
            if crate::string::is_ascii(&r) || enc == fs { return Ok(c.str_new_enc(&r, enc)); }
            match crate::builtins::econv::convert_string(crate::builtins::encoding::enc_name(fs), crate::builtins::encoding::enc_name(enc), &r) { Some(o) => Ok(c.str_new_enc(&o, enc)), None => Ok(c.str_new_enc(&r, fs)) }
        }
        Err((e, path)) => Err(crate::builtins::io::raise_errno_at(c, e, "realpath_rec - ", (&path[..], enc))),
    }
}
fn f_s_realpath(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { realpath_impl(c, a, true) }
fn f_s_realdirpath(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { realpath_impl(c, a, false) }
fn f_s_path(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { let sv = path_str(c, a[0])?; Ok(c.str_dup(sv)) }

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
    crate::builtins::io::discard_rbuf(io);
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
// Dir.glob: dir.c's glob_make_pattern / glob_helper over byte paths.
#[derive(Clone, PartialEq)]
enum GPat { Plain(Vec<u8>), Brace(Vec<u8>), Magical(Vec<u8>), Recursive, MatchAll, MatchDir }
#[derive(Clone, Copy, PartialEq)]
enum PType { Unknown, NoEnt, Dir, Symlink, Other }
/// A position in a pattern list (the node and the ones after it).
type GCur = (std::rc::Rc<Vec<GPat>>, usize);
const FNM_GLOB_SKIPDOT: i64 = 0x80;   // as dir.c: "." of directories below the first one read is skipped

/// dir.c find_dirsep: the next '/' outside brackets (escapes honoured), or the end.
fn find_dirsep(p: &[u8], mut i: usize, flags: i64) -> usize {
    let esc = flags & FNM_NOESCAPE == 0;
    let mut open = false;
    while i < p.len() {
        match p[i] {
            b'[' => open = true,
            b']' => open = false,
            b'/' if !open => return i,
            b'\\' if esc => { i += 1; if i >= p.len() { return p.len(); } }
            _ => {}
        }
        i += 1;
    }
    p.len()
}
/// 0 plain, 2 brace, 3 magical (dir.c has_magic; ALPHA is plain on Linux).
fn glob_magic(seg: &[u8], flags: i64) -> u8 {
    let esc = flags & FNM_NOESCAPE == 0;
    let mut magical = false;
    let mut i = 0;
    while i < seg.len() {
        match seg[i] {
            b'{' => return 2,
            b'*' | b'?' | b'[' => magical = true,
            b'\\' if esc => i += 1,
            _ => {}
        }
        i += 1;
    }
    if magical { 3 } else { 0 }
}
fn glob_make_pattern(p: &[u8], flags: i64) -> Vec<GPat> {
    let mut out = vec![];
    let (mut i, mut dirsep, mut recursive) = (0, false, false);
    while i < p.len() {
        if i + 2 < p.len() && p[i] == b'*' && p[i + 1] == b'*' && p[i + 2] == b'/' {
            // fold continuous RECURSIVEs
            loop { i += 3; while i < p.len() && p[i] == b'/' { i += 1; } if !(i + 2 < p.len() && p[i] == b'*' && p[i + 1] == b'*' && p[i + 2] == b'/') { break; } }
            out.push(GPat::Recursive);
            dirsep = true; recursive = true;
            continue;
        }
        let mut m = find_dirsep(p, i, flags);
        let magic = glob_magic(&p[i..m], flags);
        // consecutive plain segments (before any **/) stay one node
        if magic == 0 && !recursive && m < p.len() {
            loop {
                let m2 = find_dirsep(p, m + 1, flags);
                if glob_magic(&p[m + 1..m2], flags) == 0 && m2 < p.len() { m = m2; } else { break; }
            }
        }
        let s = p[i..m].to_vec();
        out.push(match magic { 3 => GPat::Magical(s), 2 => GPat::Brace(s), _ => GPat::Plain(s) });
        if m < p.len() { dirsep = true; i = m + 1; } else { dirsep = false; i = m; }
    }
    out.push(if dirsep { GPat::MatchDir } else { GPat::MatchAll });
    out
}
/// dir.c join_path_from_pattern: the pattern text from a position.
fn join_pattern(cur: &GCur) -> Vec<u8> {
    let mut path: Option<Vec<u8>> = None;
    for n in &cur.0[cur.1..] {
        let s: &[u8] = match n { GPat::Recursive => b"**", GPat::MatchDir => b"", GPat::MatchAll => continue, GPat::Plain(s) | GPat::Brace(s) | GPat::Magical(s) => s };
        match &mut path { None => path = Some(s.to_vec()), Some(p) => { p.push(b'/'); p.extend_from_slice(s); } }
    }
    path.unwrap_or_default()
}
struct Glob<'a> { flags: i64, sort: bool, baselen: usize, out: &'a mut Vec<Vec<u8>> }
fn ptype_of(m: &std::fs::Metadata) -> PType { let t = m.file_type(); if t.is_dir() { PType::Dir } else if t.is_symlink() { PType::Symlink } else { PType::Other } }
fn os(p: &[u8]) -> &std::ffi::OsStr { std::ffi::OsStr::from_bytes(p) }
fn join_path(path: &[u8], dirsep: bool, name: &[u8]) -> Vec<u8> { let mut b = path.to_vec(); if dirsep { b.push(b'/'); } b.extend_from_slice(name); b }
fn glob_helper(g: &mut Glob, path: &[u8], dirsep: bool, mut pt: PType, list: &[GCur], flags: i64) {
    let (mut plain, mut brace, mut magical, mut recursive, mut match_all, mut match_dir) = (false, false, false, false, false, false);
    for cur in list {
        let mut n = &cur.0[cur.1];
        if *n == GPat::Recursive { recursive = true; n = &cur.0[cur.1 + 1]; }
        match n {
            GPat::Plain(_) => plain = true,
            GPat::Brace(s) => { if !recursive || s.contains(&b'/') { brace = true; } }
            GPat::Magical(_) => magical = true,
            GPat::MatchAll => match_all = true,
            GPat::MatchDir => match_dir = true,
            GPat::Recursive => {}
        }
    }
    if brace {
        let mut alts = vec![];
        brace_expand(&join_pattern(&list[0]), flags & FNM_NOESCAPE != 0, &mut alts);
        for a in alts {
            let l = std::rc::Rc::new(glob_make_pattern(&a, flags));
            glob_helper(g, path, dirsep, pt, &[(l, 0)], flags);
        }
        return;
    }
    if !path.is_empty() {
        if match_all && pt == PType::Unknown { pt = std::fs::symlink_metadata(os(path)).map(|m| ptype_of(&m)).unwrap_or(PType::NoEnt); }
        if match_dir && matches!(pt, PType::Unknown | PType::Symlink) { pt = std::fs::metadata(os(path)).map(|m| ptype_of(&m)).unwrap_or(PType::NoEnt); }
        let seplen = (g.baselen > 0 && path.get(g.baselen) == Some(&b'/')) as usize;
        let sub = &path[(g.baselen + seplen).min(path.len())..];
        if match_all && pt != PType::NoEnt { g.out.push(sub.to_vec()); }
        if match_dir && pt == PType::Dir { g.out.push(join_path(sub, dirsep, b"")); }
    }
    if pt == PType::NoEnt { return; }
    let dot = flags & FNM_DOTMATCH != 0;
    if magical || recursive {
        let dpath: &[u8] = if path.is_empty() { b"." } else { path };
        let Ok(mut ents) = read_dir_raw(dpath) else { return };
        if g.sort { ents.sort(); }
        let skipdot = flags & FNM_GLOB_SKIPDOT != 0;
        let cflags = flags | FNM_GLOB_SKIPDOT;
        for (name, dtype) in ents {
            let mut npt = PType::Unknown;
            let mut dotfile = 0;
            if name[0] == b'.' {
                dotfile += 1;
                if name.len() == 1 {
                    if recursive && !dot { continue; }
                    if skipdot { continue; }
                    dotfile += 1;
                    npt = PType::Dir;
                } else if name == b".." { continue; }
            }
            let buf = join_path(path, dirsep, &name);
            match dtype { 0 => {} 4 => npt = PType::Dir, 10 => npt = PType::Symlink, _ => npt = PType::Other }
            let lim = if dot { 2 } else { 1 };
            if recursive && dotfile < lim && npt == PType::Unknown {
                npt = std::fs::symlink_metadata(os(&buf)).map(|m| ptype_of(&m)).unwrap_or(PType::NoEnt);
            }
            let mut nl: Vec<GCur> = vec![];
            for cur in list {
                let mut k = cur.1;
                if cur.0[k] == GPat::Recursive {
                    if npt == PType::Dir && dotfile < lim { nl.push(cur.clone()); }
                    k += 1;
                }
                let hit = match &cur.0[k] {
                    GPat::Brace(s) => { let mut alts = vec![]; brace_expand(s, flags & FNM_NOESCAPE != 0, &mut alts); alts.iter().any(|a| fnm(a, &name, cflags & !FNM_PATHNAME, true)) }
                    GPat::Plain(s) | GPat::Magical(s) => fnm(s, &name, cflags & !FNM_PATHNAME, true),
                    _ => false,
                };
                if hit { nl.push((cur.0.clone(), k + 1)); }
            }
            glob_helper(g, &buf, true, npt, &nl, cflags);
        }
    } else if plain {
        let mut copy: Vec<Option<GCur>> = list.iter().map(|c| if matches!(c.0[c.1], GPat::Plain(_)) { Some(c.clone()) } else { None }).collect();
        for i in 0..copy.len() {
            let Some(cur) = copy[i].clone() else { continue };
            let GPat::Plain(s) = &cur.0[cur.1] else { continue };
            let name = unescape(s, flags & FNM_NOESCAPE != 0);
            let mut nl = vec![(cur.0.clone(), cur.1 + 1)];
            for j in i + 1..copy.len() {
                if let Some(c2) = &copy[j] { if let GPat::Plain(s2) = &c2.0[c2.1] { if fnm(s2, &name, flags & !FNM_PATHNAME, true) { nl.push((c2.0.clone(), c2.1 + 1)); copy[j] = None; } } }
            }
            let buf = join_path(path, dirsep, &name);
            glob_helper(g, &buf, true, PType::Unknown, &nl, flags);
        }
    }
}
fn unescape(seg: &[u8], noescape: bool) -> Vec<u8> {
    if noescape { return seg.to_vec(); }
    let mut o = vec![]; let mut i = 0;
    // remove_backslashes: a trailing backslash is dropped too
    while i < seg.len() { if seg[i] == b'\\' { i += 1; if i >= seg.len() { break; } } o.push(seg[i]); i += 1; }
    o
}
/// ruby_glob0 for one pattern (braces included).
fn glob_one(pat: &[u8], flags: i64, base: Option<&[u8]>, sort: bool, out: &mut Vec<Vec<u8>>) {
    let (root, start): (&[u8], &[u8]) = if pat.first() == Some(&b'/') { (&pat[1..], b"/") } else { (pat, b"") };
    let (buf, baselen, dirsep) = match base { Some(b) if start.is_empty() => (b.to_vec(), b.len(), true), _ => (start.to_vec(), 0, false) };
    let list = std::rc::Rc::new(glob_make_pattern(root, flags));
    let mut g = Glob { flags, sort, baselen, out };
    let _ = g.flags;
    glob_helper(&mut g, &buf, dirsep, PType::Unknown, &[(list, 0)], flags);
}
fn d_s_glob(c: &mut Ctx, _slf: Value, a: &[Value], blk: Blk) -> RV {
    let (pos, opts) = kw_split(c, a);
    if pos.is_empty() || pos.len() > 2 { return argc_err(c, pos.len(), 1, 2); }
    glob_impl(c, pos[0], pos.get(1).copied(), opts, blk)
}
fn glob_impl(c: &mut Ctx, pats_v: Value, flags_v: Option<Value>, opts: Option<Value>, blk: Blk) -> RV {
    let mut base: Option<Vec<u8>> = None;
    let mut sort = true;
    let mut kwflags = None;
    if let Some(h) = opts {
        let mut unknown = vec![];
        let mut i = 0;
        while let Some((k, _)) = c.hash_entry_at(h, i) {
            i += 1;
            if k == UNDEF || k == sym_value("base") || k == sym_value("flags") || k == sym_value("sort") { continue; }
            unknown.push(inspect_str(c, k)?);
        }
        if !unknown.is_empty() { return arg_err(c, format!("unknown keyword{}: {}", if unknown.len() > 1 { "s" } else { "" }, unknown.join(", "))); }
        if let Some(b) = crate::hash::hash_get_opt(c, h, sym_value("base"))? { if b != NIL { let p = path_arg(c, b)?; if !p.as_bytes().is_empty() { base = Some(p.0); } } }
        if let Some(f) = crate::hash::hash_get_opt(c, h, sym_value("flags"))? { kwflags = Some(f); }
        if let Some(s) = crate::hash::hash_get_opt(c, h, sym_value("sort"))? {
            if s == TRUE || s == FALSE { sort = s == TRUE; } else { let d = inspect_str(c, s)?; return arg_err(c, format!("expected true or false as sort: {}", d)); }
        }
    }
    // the flags: keyword wins over the positional argument
    let flags = match kwflags.or(flags_v) { Some(f) if f != NIL => num_to_long(c, f)?, _ => 0 };
    let items: Vec<Value> = if is_array(pats_v) { c.ary(pats_v).to_vec() } else { vec![pats_v] };
    let keep = c.ary_new_from(&items);
    // q0: the patterns, q1: the result (path_arg / external_str allocate)
    c.park(2, |c, q| {
        q.set(0, keep);
        let out = c.ary_new();
        q.set(1, out);
        for i in 0..c.ary_len(q.get(0)) {
            let x = c.ary_get(q.get(0), i);
            let s = if is_string(x) { x } else { let p = path_arg(c, x)?; c.str_new(p.as_bytes()) };
            let enc = c.str_enc(s);
            if !crate::builtins::encoding::ascii_compat(enc) {
                return err(c, c.vm().c_compat_error.get(), format!("incompatible character encodings: {} and US-ASCII", crate::builtins::encoding::enc_name(enc)));
            }
            let pb = c.str_bytes(s).to_vec();
            if pb.contains(&0) { return arg_err(c, "nul-separated glob pattern is deprecated"); }
            let renc = if enc == crate::object::ENC_ASCII { crate::builtins::encoding::default_external() } else { enc };
            let mut found = vec![];
            glob_one(&pb, flags, base.as_deref(), sort, &mut found);
            for f in found { let v = crate::builtins::encoding::external_str(c, f, renc); c.ary_push(q.get(1), v); }
        }
        if blk.is_null() { return Ok(q.get(1)); }
        each_elem(c, q.get(1), blk)?;
        Ok(NIL)
    })
}
fn d_s_aref(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, opts) = kw_split(c, a);
    let l = c.ary_new_from(pos);
    glob_impl(c, l, None, opts, Blk::NONE)
}

// ---- Dir ----------------------------------------------------------------------------------------

fn d_s_pwd(c: &mut Ctx, _slf: Value, _a: &[Value], _b: Blk) -> RV {
    // rb_dir_getwd: the filesystem encoding, BINARY in place of US-ASCII (not converted to default_internal)
    let fs = crate::builtins::encoding::default_external();
    let fs = if fs == crate::object::ENC_ASCII { crate::object::ENC_BINARY } else { fs };
    match std::env::current_dir() { Ok(d) => Ok(c.str_from_vec_enc(d.as_os_str().as_bytes().to_vec(), fs)), Err(e) => Err(os_err(c, &e, "rb_dir_getwd")) }
}
static CHDIR_DEPTH: AtomicI32 = AtomicI32::new(0);
fn chdir_to(c: &mut Ctx, p: &PathArg) -> Result<(), Esc> {
    if sys::chdir(&p.cstr()) < 0 { return Err(last_err_at(c, "chdir_path - ", p)); }
    Ok(())
}
fn d_s_chdir(c: &mut Ctx, _slf: Value, a: &[Value], blk: Blk) -> RV {
    let p = match a.first() { Some(&v) => path_arg(c, v)?, None => PathArg::sys(home_of(c, "")?) };
    if blk.is_null() {
        if CHDIR_DEPTH.load(Relaxed) > 0 { crate::builtins::kernel::warn_str(c, "warning: conflicting chdir during another chdir block\n"); }
        chdir_to(c, &p)?; return Ok(Value::fix(0));
    }
    let old = PathArg::sys(cwd());
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
    match std::fs::DirBuilder::new().mode(mode).create(&p) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err_at(c, &e, "dir_s_mkdir - ", &p)) }
}
fn d_s_rmdir(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?;
    match std::fs::remove_dir(&p) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(os_err_at(c, &e, "dir_s_rmdir - ", &p)) }
}
/// The `encoding:` keyword of Dir methods, or the filesystem encoding.
fn dir_enc(c: &mut Ctx, opts: Option<Value>) -> Result<u8, Esc> {
    if let Some(h) = opts { if let Some(e) = crate::hash::hash_get_opt(c, h, sym_value("encoding"))? { if e != NIL { return crate::builtins::encoding::to_enc(c, e); } } }
    Ok(crate::builtins::encoding::default_external())
}
fn dir_list(c: &mut Ctx, p: &PathArg, children: bool) -> Result<Vec<Vec<u8>>, Esc> {
    match read_dir_raw(p.as_bytes()) {
        Ok(e) => Ok(e.into_iter().map(|x| x.0).filter(|n| !children || (n != b"." && n != b"..")).collect()),
        Err(e) => Err(crate::builtins::io::raise_errno_at(c, e, "dir_initialize - ", p)),
    }
}
/// Entry names as Strings in `enc` (or default_internal).
fn strs(c: &mut Ctx, v: Vec<Vec<u8>>, enc: u8) -> Value {
    let out = c.ary_new();
    for n in v { let s = crate::builtins::encoding::external_str(c, n, enc); c.ary_push(out, s); }
    out
}
/// yield each element of Array `l`, re-reading its length at every step
fn each_elem(c: &mut Ctx, l: Value, blk: Blk) -> Result<(), Esc> {
    let mut i = 0;
    while let Some(&x) = c.ary(l).get(i) { yield1(c, blk, x)?; i += 1; }
    Ok(())
}
fn dir_list_args(c: &mut Ctx, a: &[Value], children: bool) -> RV {
    let (pos, o) = kw_split(c, a);
    if pos.len() != 1 { return argc_err(c, pos.len(), 1, 1); }
    let p = path_arg(c, pos[0])?;
    let enc = dir_enc(c, o)?;
    let l = dir_list(c, &p, children)?;
    Ok(strs(c, l, enc))
}
fn d_s_entries(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { dir_list_args(c, a, false) }
fn d_s_children(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { dir_list_args(c, a, true) }
fn d_s_each_child(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"each_child"), a, NIL); }
    let l = dir_list_args(c, a, true)?;
    each_elem(c, l, blk)?;
    Ok(NIL)
}
fn d_s_foreach(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"foreach"), a, NIL); }
    let l = dir_list_args(c, a, false)?;
    each_elem(c, l, blk)?;
    Ok(NIL)
}
fn d_s_empty(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?;
    match read_dir_raw(p.as_bytes()) {
        Ok(e) => Ok(Value::bool(e.len() <= 2)),
        Err(20) => Ok(FALSE),
        Err(e) => Err(crate::builtins::io::raise_errno_at(c, e, "rb_dir_s_empty_p - ", &p)),
    }
}
fn d_s_home(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let u = match a.first() { Some(&v) if v != NIL => to_rstring(c, v)?, _ => String::new() };
    let h = home_of(c, &u)?;
    Ok(c.str_new(&h))
}
fn d_s_exist(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { f_s_dir_p(c, s, a, b) }
fn d_s_chroot(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let p = path_arg(c, a[0])?;
    if sys::chroot(&p.cstr()) < 0 { return Err(last_err_at(c, "dir_s_chroot - ", &p)); }
    Ok(Value::fix(0))
}
fn iv(n: &str) -> crate::value::Sym { intern(n.as_bytes()) }
fn dir_setup(c: &mut Ctx, s: Value, path: Value, ents: Value, fd: Value) {
    ivar_set_raw(c, s, iv("@path"), path); ivar_set_raw(c, s, iv("@ents"), ents); ivar_set_raw(c, s, iv("@pos"), Value::fix(0));
    ivar_set_raw(c, s, iv("@closed"), FALSE); ivar_set_raw(c, s, iv("@fd"), fd);
}
fn d_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, o) = kw_split(c, a);
    if pos.len() != 1 { return argc_err(c, pos.len(), 1, 1); }
    let p = path_arg(c, pos[0])?;
    let enc = dir_enc(c, o)?;
    let l = dir_list(c, &p, false)?; let l = strs(c, l, enc);
    let penc = if is_string(pos[0]) { c.str_enc(pos[0]) } else { crate::builtins::encoding::default_external() };
    let ps = c.str_new_enc(p.as_bytes(), penc);
    dir_setup(c, s, ps, l, NIL);
    Ok(NIL)
}
/// Dir.for_fd: a Dir over an open directory descriptor (owned: Dir#close closes it).
fn d_s_for_fd(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let fd = num_to_long(c, a[0])? as i32;
    match sys::fd_metadata(fd) {
        Err(e) => return Err(crate::builtins::exception::raise_errno_plain(c, e.raw_os_error().unwrap_or(9) as i64, "fdopendir")),
        Ok(m) if !m.is_dir() => return Err(crate::builtins::exception::raise_errno_plain(c, 20, "fdopendir")),
        Ok(_) => {}
    }
    let l = match read_dir_raw(format!("/proc/self/fd/{}", fd).as_bytes()) { Ok(e) => e.into_iter().map(|x| x.0).collect(), Err(e) => return Err(crate::builtins::exception::raise_errno_plain(c, e, "fdopendir")) };
    let l = strs(c, l, crate::builtins::encoding::default_external());
    let d = new_object(c, slf);
    dir_setup(c, d, NIL, l, Value::fix(fd as i64));
    Ok(d)
}
fn d_check(c: &mut Ctx, s: Value) -> Result<(Value, usize), Esc> {
    let e = ivar_get(c, s, iv("@ents"));
    if e == NIL { return err(c, c.vm().c_io_error.get(), "uninitialized Dir"); }
    if ivar_get(c, s, iv("@closed")).truthy() { return err(c, c.vm().c_io_error.get(), "closed directory"); }
    Ok((e, ivar_get(c, s, iv("@pos")).as_fix() as usize))
}
/// Dir#fileno: the directory's descriptor, opened on first use (close-on-exec).
fn d_fileno(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    d_check(c, s)?;
    let fd = ivar_get(c, s, iv("@fd"));
    if fd != NIL { return Ok(fd); }
    let p = ivar_get(c, s, iv("@path"));
    let pb = c.str_bytes(p).to_vec();
    let n = sys::open(&sys::cstr(&pb), sys::O_RDONLY | sys::O_CLOEXEC | 0o200000, 0);
    if n < 0 { return Err(last_err_at(c, "dirfd - ", &pb)); }
    ivar_set_raw(c, s, iv("@fd"), Value::fix(n as i64));
    Ok(Value::fix(n as i64))
}
fn d_read(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (e, p) = d_check(c, s)?;
    if p >= c.ary_len(e) { return Ok(NIL); }
    ivar_set_raw(c, s, iv("@pos"), Value::fix(p as i64 + 1));
    let x = c.ary_get(e, p);
    Ok(c.str_dup(x))
}
/// Dir#each / #children / #each_child start from the first entry (rewinddir) and leave the position at the end.
fn d_each(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"each"), a, NIL); }
    d_check(c, s)?;
    ivar_set_raw(c, s, iv("@pos"), Value::fix(0));
    loop { let v = d_read(c, s, &[], Blk::NONE)?; if v == NIL { break; } yield1(c, blk, v)?; }
    Ok(s)
}
fn d_children(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (e, _) = d_check(c, s)?;
    let out = c.ary_new();
    let mut i = 0;
    while let Some(&x) = c.ary(e).get(i) { i += 1; if c.str_bytes(x) != b"." && c.str_bytes(x) != b".." { let d = c.str_dup(x); c.ary_push(out, d); } }
    ivar_set_raw(c, s, iv("@pos"), Value::fix(c.ary_len(e) as i64));
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
fn d_seek(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { d_check(c, s)?; let n = num_to_long(c, a[0])?; let pv = c.int_from_i64(n.max(0)); ivar_set_raw(c, s, iv("@pos"), pv); Ok(s) }
fn d_set_pos(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { d_seek(c, s, a, b)?; Ok(a[0]) }
fn d_close(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    if ivar_get(c, s, iv("@closed")).truthy() { return Ok(NIL); }
    ivar_set_raw(c, s, iv("@closed"), TRUE);
    let fd = ivar_get(c, s, iv("@fd"));
    if fd.is_fixnum() && sys::close(fd.as_fix() as i32) < 0 { return Err(crate::builtins::exception::raise_errno_plain(c, sys::errno(), "closedir")); }
    Ok(NIL)
}
fn d_path(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let p = ivar_get(c, s, iv("@path")); Ok(if p == NIL { NIL } else { c.str_dup(p) }) }
fn d_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let p = ivar_get(c, s, iv("@path"));
    let k = class_path(c, real_class_of(c, s));
    if p != NIL {
        // dir_inspect: rb_str_buf_new + appends, so a non-ASCII path brings its encoding
        let out = c.str_new_enc(format!("#<{}:", k).as_bytes(), crate::object::ENC_BINARY);
        crate::string::str_append_str(c, out, p)?;
        c.rstring_mut(out).b.push(b'>');
        return Ok(out);
    }
    let s = if ivar_get(c, s, iv("@ents")) == NIL { format!("#<{}: uninitialized>", k) } else { crate::builtins::kernel::any_to_s_str(c, s) };
    Ok(c.str_new(s.as_bytes()))
}
/// Dir.fchdir(fd) [{ }]: a block runs there and returns to the directory it came from (by descriptor).
fn d_s_fchdir(c: &mut Ctx, _s: Value, a: &[Value], blk: Blk) -> RV {
    let fd = num_to_long(c, a[0])? as i32;
    if blk.is_null() {
        if sys::fchdir(fd) < 0 { return Err(crate::builtins::exception::raise_errno_plain(c, sys::errno(), "fchdir")); }
        return Ok(Value::fix(0));
    }
    let save = sys::open(c".", sys::O_RDONLY | sys::O_CLOEXEC | 0o200000, 0);
    if save < 0 { return Err(last_err_at(c, "", b".")); }
    if sys::fchdir(fd) < 0 { let e = sys::errno(); sys::close(save); return Err(crate::builtins::exception::raise_errno_plain(c, e, "fchdir")); }
    CHDIR_DEPTH.fetch_add(1, Relaxed);
    let r = yield_block(c, blk, &[]);
    CHDIR_DEPTH.fetch_sub(1, Relaxed);
    let back = sys::fchdir(save);
    let e = sys::errno();
    sys::close(save);
    if r.is_err() { return r; }
    if back < 0 { return Err(crate::builtins::exception::raise_errno_plain(c, e, "fchdir")); }
    r
}
fn d_chdir(c: &mut Ctx, s: Value, _a: &[Value], blk: Blk) -> RV {
    let fd = d_fileno(c, s, &[], Blk::NONE)?;
    let r = d_s_fchdir(c, s, &[fd], blk)?;
    Ok(if blk.is_null() { Value::fix(0) } else { r })
}
fn d_s_open(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let d = crate::builtins::object::c_new(c, slf, a, Blk::NONE)?;
    if blk.is_null() { return Ok(d); }
    let r = yield1(c, blk, d);
    let (ek, ev) = (c.esc, c.esc_val);
    let cl = d_close(c, d, &[], Blk::NONE);
    if r.is_err() { c.esc = ek; c.esc_val = ev; return r; }
    cl?;
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
    sd(c, "dev_major", st_dev_major, 0); sd(c, "dev_minor", st_dev_minor, 0); sd(c, "rdev", st_rdev, 0); sd(c, "rdev_major", st_rdev_major, 0); sd(c, "rdev_minor", st_rdev_minor, 0); sd(c, "nlink", st_nlink, 0); sd(c, "blksize", st_blksize, 0);
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
    df(c, "stat", f_s_stat, 1); df(c, "lstat", f_s_lstat, 1); df(c, "mtime", f_s_mtime, 1); df(c, "atime", f_s_atime, 1); df(c, "ctime", f_s_ctime, 1);
    df(c, "birthtime", f_s_birthtime, 1); df(c, "ftype", f_s_ftype, 1); df(c, "delete", f_s_delete, -1); df(c, "unlink", f_s_delete, -1); df(c, "rename", f_s_rename, 2);
    df(c, "symlink", f_s_symlink, 2); df(c, "link", f_s_link, 2); df(c, "readlink", f_s_readlink, 1); df(c, "chmod", f_s_chmod, -1); df(c, "chown", f_s_chown, -1);
    df(c, "lchown", f_s_lchown, -1); df(c, "lchmod", f_s_lchmod, -1); df(c, "truncate", f_s_truncate, 2); df(c, "umask", f_s_umask, -1); df(c, "mkfifo", f_s_mkfifo, -1);
    df(c, "utime", f_s_utime, -1); df(c, "lutime", f_s_lutime, -1); df(c, "basename", f_s_basename, -1); df(c, "dirname", f_s_dirname, -1); df(c, "extname", f_s_extname, 1);
    df(c, "split", f_s_split, 1); df(c, "join", f_s_join, -1); df(c, "expand_path", f_s_expand_path, -1); df(c, "absolute_path", f_s_absolute_path, -1);
    df(c, "absolute_path?", f_s_absolute_path_p, 1); df(c, "realpath", f_s_realpath, -1); df(c, "realdirpath", f_s_realdirpath, -1); df(c, "path", f_s_path, 1);
    df(c, "fnmatch", f_s_fnmatch, -1); df(c, "fnmatch?", f_s_fnmatch, -1);
    let di = |c: &mut Ctx, n: &str, fun: CFunc, a: i32| define_cfunc(c, f, n, fun, a);
    di(c, "size", f_size, 0); di(c, "truncate", f_truncate, 1); di(c, "flock", f_flock, 1); di(c, "chmod", f_chmod, 1); di(c, "chown", f_chown, 2); di(c, "mtime", f_mtime, 0);
    di(c, "atime", f_atime, 0); di(c, "ctime", f_ctime, 0); di(c, "lstat", f_lstat, 0); di(c, "birthtime", f_birthtime, 0);
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
    dd(c, "open", d_s_open, -1); dd(c, "for_fd", d_s_for_fd, 1); dd(c, "fchdir", d_s_fchdir, 1); dd(c, "chroot", d_s_chroot, 1);
    let dm = |c: &mut Ctx, n: &str, fun: CFunc, a: i32| define_cfunc(c, dir, n, fun, a);
    define_private_cfunc(c, dir, "initialize", d_initialize, -1);
    dm(c, "read", d_read, 0); dm(c, "each", d_each, 0); dm(c, "children", d_children, 0); dm(c, "each_child", d_each_child, 0); dm(c, "rewind", d_rewind, 0);
    dm(c, "tell", d_tell, 0); dm(c, "pos", d_tell, 0); dm(c, "seek", d_seek, 1); dm(c, "pos=", d_set_pos, 1); dm(c, "close", d_close, 0); dm(c, "path", d_path, 0);
    dm(c, "to_path", d_path, 0); dm(c, "inspect", d_inspect, 0); dm(c, "chdir", d_chdir, 0); dm(c, "fileno", d_fileno, 0);
}
