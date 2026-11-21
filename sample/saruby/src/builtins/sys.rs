// libc (x86_64/aarch64 Linux) behind safe wrappers: the only `unsafe` of IO / File / Process /
// Thread / ENV. Each wrapper takes Rust types (slices, &CStr, &mut) so the pointers it passes are
// valid for the call; the remaining obligation is the OS interface itself ("can't be helped").
#![allow(non_camel_case_types)]
use std::ffi::{CStr, CString};

pub type c_int = i32;
#[repr(C)] pub struct PollFd { pub fd: c_int, pub events: i16, pub revents: i16 }
#[repr(C)] pub struct Timespec { pub sec: i64, pub nsec: i64 }
#[repr(C)] pub struct Rlimit { pub cur: u64, pub max: u64 }
#[repr(C)] pub struct Tms { pub utime: i64, pub stime: i64, pub cutime: i64, pub cstime: i64 }

mod ffi {
    use super::*;
    extern "C" {
        pub fn read(fd: c_int, buf: *mut u8, n: usize) -> isize;
        pub fn write(fd: c_int, buf: *const u8, n: usize) -> isize;
        pub fn pread(fd: c_int, buf: *mut u8, n: usize, off: i64) -> isize;
        pub fn pwrite(fd: c_int, buf: *const u8, n: usize, off: i64) -> isize;
        pub fn open(path: *const i8, flags: c_int, mode: u32) -> c_int;
        pub fn close(fd: c_int) -> c_int;
        pub fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
        pub fn pipe2(fds: *mut c_int, flags: c_int) -> c_int;
        pub fn dup(fd: c_int) -> c_int;
        pub fn dup2(a: c_int, b: c_int) -> c_int;
        pub fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
        pub fn isatty(fd: c_int) -> c_int;
        pub fn fsync(fd: c_int) -> c_int;
        pub fn ftruncate(fd: c_int, len: i64) -> c_int;
        pub fn truncate(path: *const i8, len: i64) -> c_int;
        pub fn flock(fd: c_int, op: c_int) -> c_int;
        pub fn fchmod(fd: c_int, mode: u32) -> c_int;
        pub fn chmod(path: *const i8, mode: u32) -> c_int;
        pub fn chown(path: *const i8, u: u32, g: u32) -> c_int;
        pub fn lchown(path: *const i8, u: u32, g: u32) -> c_int;
        pub fn fchown(fd: c_int, u: u32, g: u32) -> c_int;
        pub fn umask(m: u32) -> u32;
        pub fn mkfifo(path: *const i8, mode: u32) -> c_int;
        pub fn utimensat(dirfd: c_int, path: *const i8, ts: *const Timespec, flags: c_int) -> c_int;
        pub fn chdir(path: *const i8) -> c_int;
        pub fn fork() -> c_int;
        pub fn execvp(file: *const i8, argv: *const *const i8) -> c_int;
        pub fn _exit(st: c_int) -> !;
        pub fn waitpid(pid: c_int, st: *mut c_int, opts: c_int) -> c_int;
        pub fn kill(pid: c_int, sig: c_int) -> c_int;
        pub fn getpid() -> c_int;
        pub fn getppid() -> c_int;
        pub fn getuid() -> u32;
        pub fn geteuid() -> u32;
        pub fn getgid() -> u32;
        pub fn getegid() -> u32;
        pub fn getpgid(pid: c_int) -> c_int;
        pub fn setpgid(pid: c_int, pg: c_int) -> c_int;
        pub fn setsid() -> c_int;
        pub fn getsid(pid: c_int) -> c_int;
        pub fn signal(sig: c_int, h: usize) -> usize;
        pub fn poll(fds: *mut PollFd, n: u64, timeout: c_int) -> c_int;
        pub fn nanosleep(req: *const Timespec, rem: *mut Timespec) -> c_int;
        pub fn clock_gettime(id: c_int, ts: *mut Timespec) -> c_int;
        pub fn clock_getres(id: c_int, ts: *mut Timespec) -> c_int;
        pub fn getrlimit(res: c_int, r: *mut Rlimit) -> c_int;
        pub fn setrlimit(res: c_int, r: *const Rlimit) -> c_int;
        pub fn times(t: *mut Tms) -> i64;
        pub fn strerror(e: c_int) -> *const i8;
        pub fn setenv(k: *const i8, v: *const i8, o: c_int) -> c_int;
        pub fn unsetenv(k: *const i8) -> c_int;
        pub fn clearenv() -> c_int;
        pub fn sigfillset(set: *mut u8) -> c_int;
        pub fn pthread_sigmask(how: c_int, set: *const u8, old: *mut u8) -> c_int;
        pub fn setitimer(which: c_int, new: *const [i64; 4], old: *mut [i64; 4]) -> c_int;
        pub fn opendir(p: *const i8) -> *mut u8;
        pub fn readdir(d: *mut u8) -> *const Dirent;
        pub fn closedir(d: *mut u8) -> c_int;
        pub fn erf(x: f64) -> f64;
        pub fn erfc(x: f64) -> f64;
        pub fn cbrt(x: f64) -> f64;
        pub fn tgamma(x: f64) -> f64;
        pub fn lgamma_r(x: f64, sign: *mut c_int) -> f64;
        pub fn __errno_location() -> *mut c_int;
    }
    #[repr(C)] pub struct Dirent { pub ino: u64, pub off: i64, pub reclen: u16, pub typ: u8, pub name: [u8; 256] }
}

pub const O_RDONLY: c_int = 0; pub const O_WRONLY: c_int = 1; pub const O_RDWR: c_int = 2; pub const O_ACCMODE: c_int = 3;
pub const O_CREAT: c_int = 0o100; pub const O_EXCL: c_int = 0o200; pub const O_NOCTTY: c_int = 0o400; pub const O_TRUNC: c_int = 0o1000;
pub const O_APPEND: c_int = 0o2000; pub const O_NONBLOCK: c_int = 0o4000; pub const O_CLOEXEC: c_int = 0o2000000;
pub const F_GETFD: c_int = 1; pub const F_SETFD: c_int = 2; pub const F_GETFL: c_int = 3; pub const F_SETFL: c_int = 4; pub const FD_CLOEXEC: c_int = 1;
pub const EINTR: i64 = 4; pub const EAGAIN: i64 = 11;
pub const POLLIN: i16 = 1; pub const POLLOUT: i16 = 4; pub const POLLERR: i16 = 8; pub const POLLHUP: i16 = 16;

// ---- file descriptors: integers to the kernel; a bad fd is EBADF, not memory unsafety --------
// SAFETY (each `unsafe` in this block): the buffer, when there is one, is a live Rust slice
// whose length is the count passed; everything else is an integer.
pub fn read(fd: c_int, buf: &mut [u8]) -> isize { unsafe { ffi::read(fd, buf.as_mut_ptr(), buf.len()) } }
pub fn write(fd: c_int, buf: &[u8]) -> isize { unsafe { ffi::write(fd, buf.as_ptr(), buf.len()) } }
pub fn pread(fd: c_int, buf: &mut [u8], off: i64) -> isize { unsafe { ffi::pread(fd, buf.as_mut_ptr(), buf.len(), off) } }
pub fn pwrite(fd: c_int, buf: &[u8], off: i64) -> isize { unsafe { ffi::pwrite(fd, buf.as_ptr(), buf.len(), off) } }
pub fn close(fd: c_int) -> c_int { unsafe { ffi::close(fd) } }
pub fn lseek(fd: c_int, off: i64, whence: c_int) -> i64 { unsafe { ffi::lseek(fd, off, whence) } }
pub fn pipe2(fds: &mut [c_int; 2], flags: c_int) -> c_int { unsafe { ffi::pipe2(fds.as_mut_ptr(), flags) } }
pub fn dup(fd: c_int) -> c_int { unsafe { ffi::dup(fd) } }
pub fn dup2(a: c_int, b: c_int) -> c_int { unsafe { ffi::dup2(a, b) } }
/// fcntl with an integer argument (F_GETFD, F_SETFD, F_GETFL, F_SETFL, F_DUPFD_CLOEXEC).
pub fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int { unsafe { ffi::fcntl(fd, cmd, arg) } }
pub fn isatty(fd: c_int) -> c_int { unsafe { ffi::isatty(fd) } }
pub fn fsync(fd: c_int) -> c_int { unsafe { ffi::fsync(fd) } }
pub fn ftruncate(fd: c_int, len: i64) -> c_int { unsafe { ffi::ftruncate(fd, len) } }
pub fn flock(fd: c_int, op: c_int) -> c_int { unsafe { ffi::flock(fd, op) } }
pub fn fchmod(fd: c_int, mode: u32) -> c_int { unsafe { ffi::fchmod(fd, mode) } }
pub fn fchown(fd: c_int, u: u32, g: u32) -> c_int { unsafe { ffi::fchown(fd, u, g) } }
pub fn poll(fds: &mut [PollFd], timeout: c_int) -> c_int { unsafe { ffi::poll(fds.as_mut_ptr(), fds.len() as u64, timeout) } }
/// Metadata of an open fd without taking ownership of it.
pub fn fd_metadata(fd: c_int) -> std::io::Result<std::fs::Metadata> {
    use std::os::unix::io::FromRawFd;
    // SAFETY: ManuallyDrop: the File never closes the fd it borrows; a bad fd is an Err (EBADF)
    let f = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(fd) });
    f.metadata()
}

// ---- paths: NUL-terminated by &CStr ---------------------------------------------------------
// SAFETY (each): `p` is a valid C string for the call.
pub fn open(p: &CStr, flags: c_int, mode: u32) -> c_int { unsafe { ffi::open(p.as_ptr(), flags, mode) } }
pub fn truncate(p: &CStr, len: i64) -> c_int { unsafe { ffi::truncate(p.as_ptr(), len) } }
pub fn chmod(p: &CStr, mode: u32) -> c_int { unsafe { ffi::chmod(p.as_ptr(), mode) } }
pub fn chown(p: &CStr, u: u32, g: u32) -> c_int { unsafe { ffi::chown(p.as_ptr(), u, g) } }
pub fn lchown(p: &CStr, u: u32, g: u32) -> c_int { unsafe { ffi::lchown(p.as_ptr(), u, g) } }
pub fn mkfifo(p: &CStr, mode: u32) -> c_int { unsafe { ffi::mkfifo(p.as_ptr(), mode) } }
pub fn chdir(p: &CStr) -> c_int { unsafe { ffi::chdir(p.as_ptr()) } }
/// utimensat(AT_FDCWD, p, [atime, mtime], flags).
pub fn utimensat_cwd(p: &CStr, ts: &[Timespec; 2], flags: c_int) -> c_int { unsafe { ffi::utimensat(-100, p.as_ptr(), ts.as_ptr(), flags) } }
pub fn umask(m: u32) -> u32 { unsafe { ffi::umask(m) } }   // SAFETY: integer in and out
/// Directory entries (name, d_type) in readdir order, "." and ".." included.
pub fn read_dir_raw(p: &CStr) -> Result<Vec<(Vec<u8>, u8)>, i64> {
    let d = unsafe { ffi::opendir(p.as_ptr()) };   // SAFETY: valid C string
    if d.is_null() { return Err(errno()); }
    let mut out = vec![];
    // SAFETY: `d` is the open DIR*; readdir's entry is valid until the next readdir/closedir on it, and is copied out before
    while let Some(e) = unsafe { ffi::readdir(d).as_ref() } {
        let n = e.name.iter().position(|&b| b == 0).unwrap_or(256);
        out.push((e.name[..n].to_vec(), e.typ));
    }
    unsafe { ffi::closedir(d); }   // SAFETY: `d` is open and closed once
    Ok(out)
}

// ---- processes ------------------------------------------------------------------------------
/// fork(2). SAFETY: the external interface can't be helped; the child either execs at once or keeps running
/// the interpreter thread alone (the main thread only waits for it, and no lock is held across fork).
pub fn fork() -> c_int { unsafe { ffi::fork() } }
/// execvp(prog, argv); returns only on failure.
pub fn execvp(prog: &CStr, argv: &[CString]) -> c_int {
    let mut ptrs: Vec<*const i8> = argv.iter().map(|a| a.as_ptr()).collect();
    ptrs.push(std::ptr::null());
    unsafe { ffi::execvp(prog.as_ptr(), ptrs.as_ptr()) }   // SAFETY: C strings and a NULL-terminated array alive for the call
}
pub fn _exit(st: c_int) -> ! { unsafe { ffi::_exit(st) } }   // SAFETY: ends the process; nothing is unwound
pub fn waitpid(pid: c_int, st: &mut c_int, opts: c_int) -> c_int { unsafe { ffi::waitpid(pid, st, opts) } }   // SAFETY: st is a live int
pub fn kill(pid: c_int, sig: c_int) -> c_int { unsafe { ffi::kill(pid, sig) } }   // SAFETY: integers
// SAFETY (each): no arguments or integers only.
pub fn getpid() -> c_int { unsafe { ffi::getpid() } }
pub fn getppid() -> c_int { unsafe { ffi::getppid() } }
pub fn getuid() -> u32 { unsafe { ffi::getuid() } }
pub fn geteuid() -> u32 { unsafe { ffi::geteuid() } }
pub fn getgid() -> u32 { unsafe { ffi::getgid() } }
pub fn getegid() -> u32 { unsafe { ffi::getegid() } }
pub fn getpgid(pid: c_int) -> c_int { unsafe { ffi::getpgid(pid) } }
pub fn setpgid(pid: c_int, pg: c_int) -> c_int { unsafe { ffi::setpgid(pid, pg) } }
pub fn setsid() -> c_int { unsafe { ffi::setsid() } }
pub fn getsid(pid: c_int) -> c_int { unsafe { ffi::getsid(pid) } }
pub fn getrlimit(res: c_int, r: &mut Rlimit) -> c_int { unsafe { ffi::getrlimit(res, r) } }   // SAFETY: r is live
pub fn setrlimit(res: c_int, r: &Rlimit) -> c_int { unsafe { ffi::setrlimit(res, r) } }   // SAFETY: r is live
pub fn times(t: &mut Tms) -> i64 { unsafe { ffi::times(t) } }   // SAFETY: t is live

// ---- signals and time -----------------------------------------------------------------------
/// Disposition for `signal`.
pub enum SigAction { Default, Ignore, Catch(extern "C" fn(c_int)) }
/// signal(2). SAFETY: a handler is an `extern "C" fn`, so the address is valid; what it may do
/// is its own contract (the one handler, process::on_signal, only sets an atomic).
pub fn signal(sig: c_int, a: SigAction) {
    let h = match a { SigAction::Default => 0, SigAction::Ignore => 1, SigAction::Catch(f) => f as usize };
    unsafe { ffi::signal(sig, h); }
}
/// Block every signal in the calling thread.
pub fn block_all_signals() {
    let mut set = [0u8; 128];
    // SAFETY: a sigset_t is 128 bytes on Linux; both pointers are to that live buffer
    unsafe { ffi::sigfillset(set.as_mut_ptr()); ffi::pthread_sigmask(0, set.as_ptr(), std::ptr::null_mut()); }
}
/// setitimer(ITIMER_REAL) to fire once after sec + usec.
pub fn set_itimer_real(sec: i64, usec: i64) -> c_int {
    let v: [i64; 4] = [0, 0, sec, usec];
    unsafe { ffi::setitimer(0, &v, std::ptr::null_mut()) }   // SAFETY: struct itimerval is four longs; old value not asked for
}
pub fn nanosleep(ts: &Timespec) -> c_int { unsafe { ffi::nanosleep(ts, std::ptr::null_mut()) } }   // SAFETY: ts is live
pub fn clock_gettime(id: c_int, t: &mut Timespec) -> c_int { unsafe { ffi::clock_gettime(id, t) } }   // SAFETY: t is live
pub fn clock_getres(id: c_int, t: &mut Timespec) -> c_int { unsafe { ffi::clock_getres(id, t) } }   // SAFETY: t is live

// ---- environment ----------------------------------------------------------------------------
// SAFETY (each): valid C strings; and setenv / unsetenv / clearenv race with getenv in another
// thread, which the external interface can't help: only the interpreter thread reads or writes
// the environment (the main thread is blocked joining it).
pub fn setenv(k: &CStr, v: &CStr) -> c_int { unsafe { ffi::setenv(k.as_ptr(), v.as_ptr(), 1) } }
pub fn unsetenv(k: &CStr) -> c_int { unsafe { ffi::unsetenv(k.as_ptr()) } }
pub fn clearenv() -> c_int { unsafe { ffi::clearenv() } }

// ---- libm: pure functions of a double -------------------------------------------------------
// SAFETY (each): no pointers but the live `sign`.
pub fn erf(x: f64) -> f64 { unsafe { ffi::erf(x) } }
pub fn erfc(x: f64) -> f64 { unsafe { ffi::erfc(x) } }
pub fn cbrt(x: f64) -> f64 { unsafe { ffi::cbrt(x) } }
pub fn tgamma(x: f64) -> f64 { unsafe { ffi::tgamma(x) } }
pub fn lgamma_r(x: f64, sign: &mut c_int) -> f64 { unsafe { ffi::lgamma_r(x, sign) } }

#[repr(C)]
struct Tm { sec: i32, min: i32, hour: i32, mday: i32, mon: i32, year: i32, wday: i32, yday: i32, isdst: i32, gmtoff: i64, zone: *const std::os::raw::c_char }
extern "C" { fn localtime_r(t: *const i64, tm: *mut Tm) -> *mut Tm; fn tzset(); }
/// (gmtoff, isdst, zone abbreviation) of the system zone at UTC second t (Time).
pub fn local_tm(t: i64) -> (i64, bool, String) {
    let mut tm = Tm { sec: 0, min: 0, hour: 0, mday: 0, mon: 0, year: 0, wday: 0, yday: 0, isdst: 0, gmtoff: 0, zone: std::ptr::null() };
    unsafe { tzset(); localtime_r(&t, &mut tm); }   // SAFETY: both pointers are live locals
    // SAFETY: tm_zone is null or a NUL-terminated name in libc's zone data
    let z = if tm.zone.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(tm.zone) }.to_string_lossy().into_owned() };
    (tm.gmtoff, tm.isdst > 0, z)
}

// SAFETY: __errno_location is the calling thread's errno, always valid
pub fn errno() -> i64 { unsafe { *ffi::__errno_location() as i64 } }
pub fn set_errno(e: i64) { unsafe { *ffi::__errno_location() = e as c_int; } }   // SAFETY: as errno
pub fn cstr(s: &[u8]) -> CString { CString::new(s.iter().copied().take_while(|&b| b != 0).collect::<Vec<u8>>()).unwrap() }
// SAFETY: strerror returns a NUL-terminated message (copied before any other strerror call on this thread)
pub fn errmsg(e: i64) -> String { unsafe { std::ffi::CStr::from_ptr(ffi::strerror(e as c_int)).to_string_lossy().into_owned() } }

/// read(2) retrying on EINTR; Err(errno) otherwise.
pub fn read_fd(fd: c_int, buf: &mut [u8]) -> Result<usize, i64> {
    loop {
        let n = read(fd, buf);
        if n >= 0 { return Ok(n as usize); }
        let e = errno();
        if e != EINTR { return Err(e); }
        crate::builtins::process::run_pending_signals_nothrow();
    }
}
/// write(2) of the whole buffer (waits on EAGAIN).
pub fn write_fd(fd: c_int, mut b: &[u8]) -> Result<(), i64> {
    while !b.is_empty() {
        let n = write(fd, b);
        if n < 0 {
            let e = errno();
            if e == EINTR { continue; }
            if e == EAGAIN { let mut p = [PollFd { fd, events: POLLOUT, revents: 0 }]; poll(&mut p, -1); continue; }
            return Err(e);
        }
        b = &b[n as usize..];
    }
    Ok(())
}
