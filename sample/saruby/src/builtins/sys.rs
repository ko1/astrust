// Raw libc bindings (x86_64/aarch64 Linux) used by IO / File / Process.
#![allow(non_camel_case_types)]
use std::ffi::CString;

pub type c_int = i32;
#[repr(C)] pub struct PollFd { pub fd: c_int, pub events: i16, pub revents: i16 }
#[repr(C)] pub struct Timespec { pub sec: i64, pub nsec: i64 }
#[repr(C)] pub struct Rlimit { pub cur: u64, pub max: u64 }
#[repr(C)] pub struct Tms { pub utime: i64, pub stime: i64, pub cutime: i64, pub cstime: i64 }

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
    pub fn access(path: *const i8, mode: c_int) -> c_int;
    pub fn eaccess(path: *const i8, mode: c_int) -> c_int;
    pub fn utimensat(dirfd: c_int, path: *const i8, ts: *const Timespec, flags: c_int) -> c_int;
    pub fn chdir(path: *const i8) -> c_int;
    pub fn chroot(path: *const i8) -> c_int;
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
    pub fn alarm(s: u32) -> u32;
    fn __errno_location() -> *mut c_int;
}

pub const O_RDONLY: c_int = 0; pub const O_WRONLY: c_int = 1; pub const O_RDWR: c_int = 2; pub const O_ACCMODE: c_int = 3;
pub const O_CREAT: c_int = 0o100; pub const O_EXCL: c_int = 0o200; pub const O_NOCTTY: c_int = 0o400; pub const O_TRUNC: c_int = 0o1000;
pub const O_APPEND: c_int = 0o2000; pub const O_NONBLOCK: c_int = 0o4000; pub const O_CLOEXEC: c_int = 0o2000000;
pub const F_GETFD: c_int = 1; pub const F_SETFD: c_int = 2; pub const F_GETFL: c_int = 3; pub const F_SETFL: c_int = 4; pub const FD_CLOEXEC: c_int = 1;
pub const EINTR: i64 = 4; pub const EAGAIN: i64 = 11;
pub const POLLIN: i16 = 1; pub const POLLOUT: i16 = 4; pub const POLLERR: i16 = 8; pub const POLLHUP: i16 = 16;

#[repr(C)]
struct Tm { sec: i32, min: i32, hour: i32, mday: i32, mon: i32, year: i32, wday: i32, yday: i32, isdst: i32, gmtoff: i64, zone: *const std::os::raw::c_char }
extern "C" { fn localtime_r(t: *const i64, tm: *mut Tm) -> *mut Tm; fn tzset(); }
/// (gmtoff, isdst, zone abbreviation) of the system zone at UTC second t (Time).
pub fn local_tm(t: i64) -> (i64, bool, String) {
    let mut tm = Tm { sec: 0, min: 0, hour: 0, mday: 0, mon: 0, year: 0, wday: 0, yday: 0, isdst: 0, gmtoff: 0, zone: std::ptr::null() };
    unsafe { tzset(); localtime_r(&t, &mut tm); }
    // SAFETY: tm_zone is null or a NUL-terminated name in libc's zone data
    let z = if tm.zone.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(tm.zone) }.to_string_lossy().into_owned() };
    (tm.gmtoff, tm.isdst > 0, z)
}

pub fn errno() -> i64 { unsafe { *__errno_location() as i64 } }
pub fn set_errno(e: i64) { unsafe { *__errno_location() = e as c_int; } }
pub fn cstr(s: &[u8]) -> CString { CString::new(s.iter().copied().take_while(|&b| b != 0).collect::<Vec<u8>>()).unwrap() }
pub fn errmsg(e: i64) -> String { unsafe { std::ffi::CStr::from_ptr(strerror(e as c_int)).to_string_lossy().into_owned() } }

/// read(2) retrying on EINTR; Err(errno) otherwise.
pub fn read_fd(fd: c_int, buf: &mut [u8]) -> Result<usize, i64> {
    loop {
        let n = unsafe { read(fd, buf.as_mut_ptr(), buf.len()) };
        if n >= 0 { return Ok(n as usize); }
        let e = errno();
        if e != EINTR { return Err(e); }
        crate::builtins::process::run_pending_signals_nothrow();
    }
}
/// write(2) of the whole buffer (waits on EAGAIN).
pub fn write_fd(fd: c_int, mut b: &[u8]) -> Result<(), i64> {
    while !b.is_empty() {
        let n = unsafe { write(fd, b.as_ptr(), b.len()) };
        if n < 0 {
            let e = errno();
            if e == EINTR { continue; }
            if e == EAGAIN { let mut p = PollFd { fd, events: POLLOUT, revents: 0 }; unsafe { poll(&mut p, 1, -1); } continue; }
            return Err(e);
        }
        b = &b[n as usize..];
    }
    Ok(())
}
