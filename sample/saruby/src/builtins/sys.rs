// libc (x86_64/aarch64 Linux) behind safe wrappers: the only `unsafe` of IO / File / Process /
// Thread / ENV. Each wrapper takes Rust types (slices, &CStr, &mut) so the pointers it passes are
// valid for the call; the remaining obligation is the OS interface itself ("can't be helped").
#![allow(non_camel_case_types)]
use std::ffi::{CStr, CString};

pub type c_int = i32;
#[repr(C)] pub struct PollFd { pub fd: c_int, pub events: i16, pub revents: i16 }
#[repr(C)] pub struct Timespec { pub sec: i64, pub nsec: i64 }
/// struct epoll_event (packed on x86-64, as the kernel's).
#[cfg_attr(target_arch = "x86_64", repr(C, packed))]
#[cfg_attr(not(target_arch = "x86_64"), repr(C))]
#[derive(Clone, Copy)]
pub struct EpollEvent { pub events: u32, pub data: u64 }
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
        pub fn ppoll(fds: *mut PollFd, n: u64, ts: *const Timespec, sigmask: *const u8) -> c_int;
        pub fn epoll_create1(flags: c_int) -> c_int;
        pub fn epoll_ctl(epfd: c_int, op: c_int, fd: c_int, ev: *mut EpollEvent) -> c_int;
        pub fn epoll_pwait2(epfd: c_int, evs: *mut EpollEvent, n: c_int, ts: *const Timespec, sigmask: *const u8) -> c_int;
        pub fn eventfd(init: u32, flags: c_int) -> c_int;
        pub fn syscall(n: i64, ...) -> i64;
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
/// ppoll(2) with no signal mask change; None waits without a time limit.
pub fn ppoll(fds: &mut [PollFd], timeout: Option<&Timespec>) -> c_int {
    let ts = timeout.map_or(std::ptr::null(), |t| t as *const Timespec);
    unsafe { ffi::ppoll(fds.as_mut_ptr(), fds.len() as u64, ts, std::ptr::null()) }   // SAFETY: live slice; ts null or live
}
pub const EPOLL_CLOEXEC: c_int = O_CLOEXEC;
pub const EPOLL_CTL_ADD: c_int = 1; pub const EPOLL_CTL_DEL: c_int = 2; pub const EPOLL_CTL_MOD: c_int = 3;
pub const EFD_CLOEXEC: c_int = O_CLOEXEC; pub const EFD_NONBLOCK: c_int = O_NONBLOCK;
pub fn epoll_create1(flags: c_int) -> c_int { unsafe { ffi::epoll_create1(flags) } }   // SAFETY: integers
/// epoll_ctl(2); `ev` is ignored by EPOLL_CTL_DEL.
pub fn epoll_ctl(epfd: c_int, op: c_int, fd: c_int, mut ev: EpollEvent) -> c_int { unsafe { ffi::epoll_ctl(epfd, op, fd, &mut ev) } }   // SAFETY: ev is a live local
/// epoll_pwait2(2) into `evs`; None waits without a time limit.
pub fn epoll_pwait2(epfd: c_int, evs: &mut [EpollEvent], timeout: Option<&Timespec>) -> c_int {
    let ts = timeout.map_or(std::ptr::null(), |t| t as *const Timespec);
    unsafe { ffi::epoll_pwait2(epfd, evs.as_mut_ptr(), evs.len().min(i32::MAX as usize) as c_int, ts, std::ptr::null()) }   // SAFETY: live slice of its length; ts null or live
}
pub fn eventfd(init: u32, flags: c_int) -> c_int { unsafe { ffi::eventfd(init, flags) } }   // SAFETY: integers
/// pidfd_open(2) (Linux 5.3; no glibc wrapper before 2.36).
pub fn pidfd_open(pid: c_int) -> c_int { unsafe { ffi::syscall(434, pid as i64, 0i64) as c_int } }   // SAFETY: SYS_pidfd_open takes two integers
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
        // a non-blocking fd (sockets are) read as a blocking one, as rb_io_wait does
        if e == EAGAIN { let mut p = [PollFd { fd, events: POLLIN, revents: 0 }]; poll(&mut p, -1); continue; }
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

// ---- sockets (builtins/socket.rs): a sockaddr is a byte slice in the kernel's layout ----------
#[repr(C)] struct AddrInfoRaw { flags: c_int, family: c_int, socktype: c_int, protocol: c_int, addrlen: u32, addr: *const u8, canonname: *const i8, next: *const AddrInfoRaw }
#[repr(C)] struct IoVec { base: *mut u8, len: usize }
#[repr(C)] struct MsgHdr { name: *mut u8, namelen: u32, iov: *mut IoVec, iovlen: usize, control: *mut u8, controllen: usize, flags: c_int }
#[repr(C)] struct IfAddrsRaw { next: *const IfAddrsRaw, name: *const i8, flags: u32, addr: *const u8, netmask: *const u8, ifu: *const u8, data: *const u8 }
mod sffi {
    use super::*;
    extern "C" {
        pub fn getaddrinfo(node: *const i8, serv: *const i8, hints: *const AddrInfoRaw, res: *mut *const AddrInfoRaw) -> c_int;
        pub fn freeaddrinfo(ai: *const AddrInfoRaw);
        pub fn getnameinfo(sa: *const u8, len: u32, host: *mut u8, hl: u32, serv: *mut u8, sl: u32, flags: c_int) -> c_int;
        pub fn gai_strerror(e: c_int) -> *const i8;
        pub fn getifaddrs(out: *mut *const IfAddrsRaw) -> c_int;
        pub fn freeifaddrs(p: *const IfAddrsRaw);
    }
}
// socket calls go through syscall(2) (already imported): each new libc symbol costs startup time in ld.so
#[cfg(target_arch = "x86_64")]
mod nr { pub const SOCKET: i64 = 41; pub const CONNECT: i64 = 42; pub const SENDTO: i64 = 44; pub const RECVFROM: i64 = 45; pub const SENDMSG: i64 = 46; pub const RECVMSG: i64 = 47;
    pub const SHUTDOWN: i64 = 48; pub const BIND: i64 = 49; pub const LISTEN: i64 = 50; pub const GETSOCKNAME: i64 = 51; pub const GETPEERNAME: i64 = 52; pub const SOCKETPAIR: i64 = 53;
    pub const SETSOCKOPT: i64 = 54; pub const GETSOCKOPT: i64 = 55; pub const UNAME: i64 = 63; pub const ACCEPT4: i64 = 288; }
#[cfg(target_arch = "aarch64")]
mod nr { pub const SOCKET: i64 = 198; pub const SOCKETPAIR: i64 = 199; pub const BIND: i64 = 200; pub const LISTEN: i64 = 201; pub const CONNECT: i64 = 203; pub const GETSOCKNAME: i64 = 204;
    pub const GETPEERNAME: i64 = 205; pub const SENDTO: i64 = 206; pub const RECVFROM: i64 = 207; pub const SETSOCKOPT: i64 = 208; pub const GETSOCKOPT: i64 = 209; pub const SHUTDOWN: i64 = 210;
    pub const SENDMSG: i64 = 211; pub const RECVMSG: i64 = 212; pub const UNAME: i64 = 160; pub const ACCEPT4: i64 = 242; }
// SAFETY (each one-line wrapper below): the system call takes integers and pointers to live slices or
// locals whose lengths are the counts passed; the kernel writes at most that many bytes and reports the length.
pub fn socket(d: c_int, t: c_int, p: c_int) -> c_int { unsafe { ffi::syscall(nr::SOCKET, d as i64, t as i64, p as i64) as c_int } }
pub fn socketpair(d: c_int, t: c_int, p: c_int, sv: &mut [c_int; 2]) -> c_int { unsafe { ffi::syscall(nr::SOCKETPAIR, d as i64, t as i64, p as i64, sv.as_mut_ptr()) as c_int } }
pub fn bind(fd: c_int, sa: &[u8]) -> c_int { unsafe { ffi::syscall(nr::BIND, fd as i64, sa.as_ptr(), sa.len() as i64) as c_int } }
pub fn connect(fd: c_int, sa: &[u8]) -> c_int { unsafe { ffi::syscall(nr::CONNECT, fd as i64, sa.as_ptr(), sa.len() as i64) as c_int } }
pub fn listen(fd: c_int, n: c_int) -> c_int { unsafe { ffi::syscall(nr::LISTEN, fd as i64, n as i64) as c_int } }
fn len_out(r: i64, l: u32) -> Result<usize, i64> { if r < 0 { Err(errno()) } else { Ok(l as usize) } }
/// accept4(2) into `sa`; (fd, address length).
pub fn accept4(fd: c_int, sa: &mut [u8], flags: c_int) -> Result<(c_int, usize), i64> {
    let mut l = sa.len() as u32;
    let r = unsafe { ffi::syscall(nr::ACCEPT4, fd as i64, sa.as_mut_ptr(), &mut l as *mut u32, flags as i64) };
    if r < 0 { Err(errno()) } else { Ok((r as c_int, l as usize)) }
}
/// getsockname(2) into `sa`; the address length (may exceed sa.len(): truncated).
pub fn getsockname(fd: c_int, sa: &mut [u8]) -> Result<usize, i64> { let mut l = sa.len() as u32; let r = unsafe { ffi::syscall(nr::GETSOCKNAME, fd as i64, sa.as_mut_ptr(), &mut l as *mut u32) }; len_out(r, l) }
pub fn getpeername(fd: c_int, sa: &mut [u8]) -> Result<usize, i64> { let mut l = sa.len() as u32; let r = unsafe { ffi::syscall(nr::GETPEERNAME, fd as i64, sa.as_mut_ptr(), &mut l as *mut u32) }; len_out(r, l) }
pub fn setsockopt(fd: c_int, lv: c_int, opt: c_int, v: &[u8]) -> c_int { unsafe { ffi::syscall(nr::SETSOCKOPT, fd as i64, lv as i64, opt as i64, v.as_ptr(), v.len() as i64) as c_int } }
pub fn getsockopt(fd: c_int, lv: c_int, opt: c_int, v: &mut [u8]) -> Result<usize, i64> { let mut l = v.len() as u32; let r = unsafe { ffi::syscall(nr::GETSOCKOPT, fd as i64, lv as i64, opt as i64, v.as_mut_ptr(), &mut l as *mut u32) }; len_out(r, l) }
pub fn shutdown(fd: c_int, how: c_int) -> c_int { unsafe { ffi::syscall(nr::SHUTDOWN, fd as i64, how as i64) as c_int } }
/// sendto(2); an empty `sa` sends to the connected peer.
pub fn sendto(fd: c_int, b: &[u8], flags: c_int, sa: &[u8]) -> isize {
    let p = if sa.is_empty() { std::ptr::null() } else { sa.as_ptr() };
    unsafe { ffi::syscall(nr::SENDTO, fd as i64, b.as_ptr(), b.len() as i64, flags as i64, p, sa.len() as i64) as isize }   // SAFETY: as above (null with length 0)
}
/// recvfrom(2) into `b`, the source address into `sa`; (bytes, address length).
pub fn recvfrom(fd: c_int, b: &mut [u8], flags: c_int, sa: &mut [u8]) -> Result<(usize, usize), i64> {
    let mut l = sa.len() as u32;
    let r = unsafe { ffi::syscall(nr::RECVFROM, fd as i64, b.as_mut_ptr(), b.len() as i64, flags as i64, sa.as_mut_ptr(), &mut l as *mut u32) };
    if r < 0 { Err(errno()) } else { Ok((r as usize, l as usize)) }
}
/// sendmsg(2) of one buffer with an optional destination and control data.
pub fn sendmsg(fd: c_int, data: &[u8], name: &[u8], control: &[u8], flags: c_int) -> isize {
    let mut iov = IoVec { base: data.as_ptr() as *mut u8, len: data.len() };
    let nul = |s: &[u8]| if s.is_empty() { std::ptr::null_mut() } else { s.as_ptr() as *mut u8 };
    let m = MsgHdr { name: nul(name), namelen: name.len() as u32, iov: &mut iov, iovlen: 1, control: nul(control), controllen: control.len(), flags: 0 };
    unsafe { ffi::syscall(nr::SENDMSG, fd as i64, &m as *const MsgHdr, flags as i64) as isize }   // SAFETY: every pointer in `m` is a live slice or local of the given length; sendmsg only reads them
}
/// recvmsg(2) into one buffer; (bytes, name length, control length, msg_flags).
pub fn recvmsg(fd: c_int, data: &mut [u8], name: &mut [u8], control: &mut [u8], flags: c_int) -> Result<(usize, usize, usize, c_int), i64> {
    let mut iov = IoVec { base: data.as_mut_ptr(), len: data.len() };
    let mut m = MsgHdr { name: name.as_mut_ptr(), namelen: name.len() as u32, iov: &mut iov, iovlen: 1, control: control.as_mut_ptr(), controllen: control.len(), flags: 0 };
    let r = unsafe { ffi::syscall(nr::RECVMSG, fd as i64, &mut m as *mut MsgHdr, flags as i64) };   // SAFETY: as sendmsg; the kernel writes within each length and updates them
    if r < 0 { Err(errno()) } else { Ok((r as usize, m.namelen as usize, m.controllen, m.flags)) }
}
/// uname(2)'s nodename (what gethostname(3) returns on Linux).
pub fn nodename() -> Result<Vec<u8>, i64> {
    let mut u = [0u8; 65 * 6];
    if unsafe { ffi::syscall(nr::UNAME, u.as_mut_ptr()) } < 0 { return Err(errno()); }   // SAFETY: struct utsname is six 65-byte fields
    let n = &u[65..130];
    Ok(n[..n.iter().position(|&b| b == 0).unwrap_or(65)].to_vec())
}
/// One getaddrinfo(3) result: (flags, family, socktype, protocol, sockaddr, canonname).
pub type AddrInfo = (c_int, c_int, c_int, c_int, Vec<u8>, Option<Vec<u8>>);
/// getaddrinfo(3) with hints (flags, family, socktype, protocol); Err(EAI_* code).
pub fn getaddrinfo(node: Option<&CStr>, serv: Option<&CStr>, hints: (c_int, c_int, c_int, c_int)) -> Result<Vec<AddrInfo>, c_int> {
    let h = AddrInfoRaw { flags: hints.0, family: hints.1, socktype: hints.2, protocol: hints.3, addrlen: 0, addr: std::ptr::null(), canonname: std::ptr::null(), next: std::ptr::null() };
    let mut res: *const AddrInfoRaw = std::ptr::null();
    let cp = |s: Option<&CStr>| s.map_or(std::ptr::null(), |s| s.as_ptr());
    let r = unsafe { sffi::getaddrinfo(cp(node), cp(serv), &h, &mut res) };   // SAFETY: C strings or null, a live hints struct, an out pointer
    if r != 0 { return Err(r); }
    let mut out = vec![];
    let mut p = res;
    // SAFETY: `res` is the list getaddrinfo returned, valid until freeaddrinfo; each node's addr has addrlen bytes, canonname is null or a C string; all is copied out
    while let Some(ai) = unsafe { p.as_ref() } {
        let sa = if ai.addr.is_null() { vec![] } else { unsafe { std::slice::from_raw_parts(ai.addr, ai.addrlen as usize) }.to_vec() };
        let cn = if ai.canonname.is_null() { None } else { Some(unsafe { CStr::from_ptr(ai.canonname) }.to_bytes().to_vec()) };
        out.push((ai.flags, ai.family, ai.socktype, ai.protocol, sa, cn));
        p = ai.next;
    }
    if !res.is_null() { unsafe { sffi::freeaddrinfo(res) }; }   // SAFETY: freed once, nothing points into it any more
    Ok(out)
}
/// getnameinfo(3): (host, service); Err(EAI_* code).
pub fn getnameinfo(sa: &[u8], flags: c_int) -> Result<(Vec<u8>, Vec<u8>), c_int> {
    let (mut h, mut s) = ([0u8; 1025], [0u8; 32]);
    let r = unsafe { sffi::getnameinfo(sa.as_ptr(), sa.len() as u32, h.as_mut_ptr(), h.len() as u32, s.as_mut_ptr(), s.len() as u32, flags) };   // SAFETY: live buffers of the given lengths; results are NUL-terminated within them
    if r != 0 { return Err(r); }
    let z = |b: &[u8]| b[..b.iter().position(|&x| x == 0).unwrap_or(b.len())].to_vec();
    Ok((z(&h), z(&s)))
}
// SAFETY: gai_strerror returns a static NUL-terminated message
pub fn gai_strerror(e: c_int) -> String { unsafe { CStr::from_ptr(sffi::gai_strerror(e)) }.to_string_lossy().into_owned() }
/// One interface address of getifaddrs(3): (name, flags, addr, netmask, broadcast/destination address).
pub type IfAddr = (Vec<u8>, u32, Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>);
/// getifaddrs(3); `salen(family)` gives the sockaddr length of a family.
pub fn getifaddrs(salen: impl Fn(u16) -> usize) -> Result<Vec<IfAddr>, i64> {
    let mut res: *const IfAddrsRaw = std::ptr::null();
    if unsafe { sffi::getifaddrs(&mut res) } != 0 { return Err(errno()); }   // SAFETY: an out pointer
    // SAFETY (this function's other blocks): the list getifaddrs returned, valid until freeifaddrs; a sockaddr starts with its u16 family and is copied with that family's length; ifa_name is a C string
    let copy = |p: *const u8| -> Option<Vec<u8>> {
        if p.is_null() { return None; }
        let fam = u16::from_ne_bytes(unsafe { [*p, *p.add(1)] });
        Some(unsafe { std::slice::from_raw_parts(p, salen(fam).max(2)) }.to_vec())
    };
    let mut out = vec![];
    let mut p = res;
    while let Some(i) = unsafe { p.as_ref() } {
        let name = unsafe { CStr::from_ptr(i.name) }.to_bytes().to_vec();
        out.push((name, i.flags, copy(i.addr), copy(i.netmask), copy(i.ifu)));
        p = i.next;
    }
    unsafe { sffi::freeifaddrs(res) };   // SAFETY: freed once
    Ok(out)
}
