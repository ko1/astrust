// socket.so (ext/socket) natively: BasicSocket and its subclasses are IO objects (RIo), so
// read / write / close / select are io.rs's. Blocking calls wait through the blop layer
// (io::wait_one); getaddrinfo / getnameinfo run on a worker thread while other threads exist.
// lib/socket.rb of the reference ruby is loaded on top (require "socket").
#![forbid(unsafe_code)]
use crate::array::*;
use crate::builtins::exception::raise_errno_plain;
use crate::builtins::io::{self, RIo};
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::builtins::sys;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::Global;
use crate::string::*;
use crate::symbol::{intern, s, sym_bytes};
use crate::value::*;
use crate::vm::*;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

static C_BASIC: Global = Global::new();
static C_SOCKET: Global = Global::new();
static C_TCPSOCKET: Global = Global::new();
static C_UNIXSOCKET: Global = Global::new();
static C_ADDRINFO: Global = Global::new();
static C_OPTION: Global = Global::new();
static C_ANCDATA: Global = Global::new();
static C_IFADDR: Global = Global::new();
static E_SOCKET: Global = Global::new();
static E_RESOLUTION: Global = Global::new();
static NOREVLOOKUP: AtomicBool = AtomicBool::new(true);

// ---- constants (socket_consts.rs, generated from CRuby) --------------------------------------
fn consts() -> &'static std::collections::HashMap<&'static str, i64> {
    static M: std::sync::OnceLock<std::collections::HashMap<&'static str, i64>> = std::sync::OnceLock::new();
    M.get_or_init(|| all_consts().collect())
}
/// The platform value of a Socket::Constants name (-1 when the platform lacks it).
/// (name, value) in mkconstants.rb order (one string and one array: no relocations at load time).
fn all_consts() -> impl Iterator<Item = (&'static str, i64)> {
    crate::builtins::socket_consts::NAMES.split(' ').zip(crate::builtins::socket_consts::VALUES.iter().copied())
}
fn k(name: &str) -> i32 { consts().get(name).map(|&v| v as i32).unwrap_or(-1) }
const AF_UNSPEC: i32 = 0;
const AF_UNIX: i32 = 1;
const AF_INET: i32 = 2;
const AF_INET6: i32 = 10;
const AF_PACKET: i32 = 17;
const SOCK_STREAM: i32 = 1;
const SOCK_DGRAM: i32 = 2;
const SOL_SOCKET: i32 = 1;
const MSG_CMSG_CLOEXEC: i32 = 0x4000_0000;   // Linux; not a Socket constant
const EINPROGRESS: i64 = 115;
const ETIMEDOUT: i64 = 110;
const SIN_LEN: usize = 16;
const SIN6_LEN: usize = 28;
const SUN_LEN: usize = 110;
const SS_LEN: usize = 128;
fn is_ip(f: i32) -> bool { f == AF_INET || f == AF_INET6 }

/// The name groups of mkconstants.rb: which names qualify, and the prefixes that may be left out.
struct Grp { pat: fn(&str) -> bool, pre: &'static [&'static str] }
const G_FAMILY: Grp = Grp { pat: |n| n.starts_with("AF_") || n.starts_with("PF_"), pre: &["AF_"] };
const G_SOCKTYPE: Grp = Grp { pat: |n| n.starts_with("SOCK_"), pre: &["SOCK_"] };
const G_IPPROTO: Grp = Grp { pat: |n| n.starts_with("IPPROTO_"), pre: &["IPPROTO_"] };
const G_ULEVEL: Grp = Grp { pat: |n| n == "SOL_SOCKET", pre: &["SOL_"] };
const G_IPLEVEL: Grp = Grp { pat: |n| n == "SOL_SOCKET" || n.starts_with("IPPROTO_"), pre: &["SOL_", "IPPROTO_"] };
const G_SO: Grp = Grp { pat: |n| n.starts_with("SO_"), pre: &["SO_"] };
const G_IP: Grp = Grp { pat: |n| n.starts_with("IP_"), pre: &["IP_"] };
const G_IPV6: Grp = Grp { pat: |n| n.starts_with("IPV6_"), pre: &["IPV6_"] };
const G_TCP: Grp = Grp { pat: |n| n.starts_with("TCP_"), pre: &["TCP_"] };
const G_UDP: Grp = Grp { pat: |n| n.starts_with("UDP_"), pre: &["UDP_"] };
const G_SHUT: Grp = Grp { pat: |n| n.starts_with("SHUT_"), pre: &["SHUT_"] };
const G_SCM: Grp = Grp { pat: |n| n.starts_with("SCM_"), pre: &["SCM_"] };
const G_LOCAL: Grp = Grp { pat: |n| n.starts_with("LOCAL_"), pre: &["LOCAL_"] };
const G_AF: Grp = Grp { pat: |n| n.starts_with("AF_"), pre: &[] };
const G_AF_NOPRE: Grp = Grp { pat: |n| n.starts_with("AF_"), pre: &["AF_"] };
const G_PF: Grp = Grp { pat: |n| n.starts_with("PF_"), pre: &[] };
const G_SOCKTYPE_NAME: Grp = Grp { pat: |n| n.starts_with("SOCK_"), pre: &[] };
const G_IPPROTO_NAME: Grp = Grp { pat: |n| n.starts_with("IPPROTO_"), pre: &[] };

/// name -> value (rsock_*_to_int).
fn name_to_int(g: &Grp, name: &[u8]) -> Option<i32> {
    let name = std::str::from_utf8(name).ok()?;
    for (n, v) in all_consts() {
        if !(g.pat)(n) { continue; }
        if n == name { return Some(v as i32); }
        if g.pre.iter().any(|p| n.strip_prefix(p) == Some(name)) { return Some(v as i32); }
    }
    None
}
/// value -> name (rsock_intern_*): the first name in table order, shortened if a prefix applies.
fn int_to_name(g: &Grp, v: i32) -> Option<String> {
    let mut first = None;
    for (n, x) in all_consts() {
        if x as i32 != v || !(g.pat)(n) { continue; }
        if let Some(s) = g.pre.iter().find_map(|p| n.strip_prefix(p)) { return Some(s.to_string()); }
        if first.is_none() { first = Some(n.to_string()); }
    }
    first
}

// ---- small helpers -----------------------------------------------------------------------------
fn bin(c: &mut Ctx, b: &[u8]) -> Value { c.str_new_enc(b, ENC_BINARY) }
fn usascii(c: &mut Ctx, b: &str) -> Value { c.str_new_enc(b.as_bytes(), ENC_ASCII) }
fn to_i32(c: &mut Ctx, v: Value) -> Result<i32, Esc> {
    let n = num_to_long(c, v)?;
    if n > i32::MAX as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too big to convert to 'int'", n)); }
    if n < i32::MIN as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too small to convert to 'int'", n)); }
    Ok(n as i32)
}
/// StringValueCStr: the bytes of a String (to_str), without NUL.
fn cstr_arg(c: &mut Ctx, v: Value) -> Result<Vec<u8>, Esc> {
    let s = expect_str(c, v)?;
    let b = c.str_bytes(s).to_vec();
    if b.contains(&0) { return arg_err(c, "string contains null byte"); }
    Ok(b)
}
fn sym_arg(v: Value) -> Option<Vec<u8>> { if v.is_sym() { Some(sym_bytes(v.as_sym()).to_vec()) } else { None } }
fn blk_given(b: Blk) -> bool { !b.is_null() }
fn kwget(c: &mut Ctx, h: Option<Value>, name: &str) -> Result<Value, Esc> {
    match h { Some(h) => Ok(crate::hash::hash_get_opt(c, h, sym_value(name))?.unwrap_or(NIL)), None => Ok(NIL) }
}
fn u16_at(b: &[u8], i: usize) -> u32 { u16::from_ne_bytes([b[i], b[i + 1]]) as u32 }
fn be16_at(b: &[u8], i: usize) -> u32 { u16::from_be_bytes([b[i], b[i + 1]]) as u32 }
fn u64_at(b: &[u8], i: usize) -> u64 { let mut x = [0u8; 8]; x.copy_from_slice(&b[i..i + 8]); u64::from_ne_bytes(x) }
fn i32_at(b: &[u8], i: usize) -> i32 { i32::from_ne_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) }
fn afamily(sa: &[u8]) -> i32 { if sa.len() >= 2 { u16_at(sa, 0) as i32 } else { AF_UNSPEC } }

// ---- errors ------------------------------------------------------------------------------------
/// rb_warn: "file:line: warning: msg" unless $VERBOSE is nil.
fn rb_warn(c: &mut Ctx, msg: &str) -> Result<(), Esc> {
    if c.vm().verbose.get() == NIL { return Ok(()); }
    let loc = c.cur_frame().map(|f| format!("{}:{}: ", frame_file(f), f.line())).unwrap_or_default();
    warning_write(c, format!("{}warning: {}\n", loc, msg).as_bytes())
}
fn sys_fail(c: &mut Ctx, e: i64, msg: &str) -> Esc { raise_errno_plain(c, e, msg) }
/// rb_readwrite_syserr_fail: IO::<E>Wait{Readable,Writable}.
fn rw_fail(c: &mut Ctx, readable: bool, e: i64, msg: &str) -> Esc {
    let pre = match e { sys::EAGAIN => "EAGAIN", EINPROGRESS => "EINPROGRESS", _ => return sys_fail(c, e, msg) };
    let name = format!("{}Wait{}", pre, if readable { "Readable" } else { "Writable" });
    let k = const_get_at(c, c.vm().c_io.get(), intern(name.as_bytes())).unwrap_or(c.vm().c_io_error.get());
    let x = make_exception(c, k, &format!("{} - {}", crate::builtins::exception::errno_text(Value::fix(e)), msg));
    ivar_set_raw(c, x, s::errno, Value::fix(e));
    raise_exc(c, x)
}
fn socket_error<T>(c: &mut Ctx, msg: impl AsRef<str>) -> Result<T, Esc> { let k = E_SOCKET.get(c); err(c, k, msg) }
/// rsock_raise_resolution_error.
fn resolution_error(c: &mut Ctx, reason: &str, code: i32, eno: i64) -> Esc {
    if code == k("EAI_SYSTEM") && eno != 0 { return sys_fail(c, eno, reason); }
    let k = E_RESOLUTION.get(c);
    let x = make_exception(c, k, &format!("{}: {}", reason, sys::gai_strerror(code)));
    ivar_set_raw(c, x, intern(b"error_code"), Value::fix(code as i64));
    raise_exc(c, x)
}
fn timeout_error(c: &mut Ctx, msg: &str) -> Esc {
    let k = io::timeout_error_class(c);
    raise_msg(c, k, msg)
}
/// rsock_syserr_fail_host_port: "<mesg> for <host.inspect> port <port>".
fn fail_host_port(c: &mut Ctx, e: i64, mesg: &str, host: Value, port: Value) -> Esc {
    let h = match inspect_str(c, host) { Ok(h) => h, Err(x) => return x };
    let p = match to_s_value(c, port) { Ok(p) => String::from_utf8_lossy(c.str_bytes(p)).into_owned(), Err(x) => return x };
    let m = format!("{} for {} port {}", mesg, h, p);
    if e == ETIMEDOUT { return timeout_error(c, &m); }
    sys_fail(c, e, &m)
}
/// rsock_syserr_fail_path: "<mesg> for <path>" when the IO has a path.
fn fail_path(c: &mut Ctx, e: i64, mesg: &str, path: Option<&[u8]>) -> Esc {
    match path { Some(p) => { let m = format!("{} for {}", mesg, String::from_utf8_lossy(p)); sys_fail(c, e, &m) } None => sys_fail(c, e, mesg) }
}
fn io_path(c: &Ctx, v: Value) -> Option<Vec<u8>> { io::rio(c, v).filter(|i| i.has_path).map(|i| i.path.clone()) }
/// rsock_syserr_fail_sockaddr: "<mesg> for <inspect_sockaddr>".
fn fail_sockaddr(c: &mut Ctx, e: i64, mesg: &str, sa: &[u8]) -> Esc {
    let s = match inspect_sockaddr(c, sa) { Ok(s) => s, Err(x) => return x };
    let m = format!("{} for {}", mesg, String::from_utf8_lossy(&s));
    sys_fail(c, e, &m)
}

// ---- name resolution ------------------------------------------------------------------------
/// Run `f` on a worker thread while other Ruby threads exist (they keep running); inline otherwise.
fn offload<T: Send + 'static>(c: &mut Ctx, f: impl FnOnce() -> T + Send + 'static) -> Result<T, Esc> {
    if !crate::builtins::thread::active(c) { return Ok(f()); }
    let efd = sys::eventfd(0, sys::EFD_CLOEXEC);
    if efd < 0 { return Ok(f()); }
    let wfd = sys::dup(efd);
    if wfd < 0 { sys::close(efd); return Ok(f()); }
    let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let s2 = slot.clone();
    // the worker owns wfd and closes it; this thread owns efd, so neither number is reused under the other
    let started = spawn_job(Box::new(move || {
        let r = f();
        if let Ok(mut g) = s2.lock() { *g = Some(r); }
        sys::write(wfd, &1u64.to_ne_bytes());
        sys::close(wfd);
    }));
    if !started { sys::close(wfd); sys::close(efd); return Err(sys_fail(c, sys::EAGAIN, "can't create resolver thread")); }
    loop {
        if let Err(e) = crate::builtins::thread::poll_wait(c, vec![sys::PollFd { fd: efd, events: sys::POLLIN, revents: 0 }], None) { sys::close(efd); return Err(e); }
        let v = slot.lock().ok().and_then(|mut g| g.take());
        if let Some(v) = v { sys::close(efd); return Ok(v); }
    }
}
/// One non-generic thread start (a generic spawn per call site perturbs the crate's code generation).
#[inline(never)]
fn spawn_job(job: Box<dyn FnOnce() + Send>) -> bool {
    std::thread::Builder::new().spawn(move || { sys::block_all_signals(); job() }).is_ok()
}
type GaiRes = Result<Vec<sys::AddrInfo>, (i32, i64)>;
fn real_getaddrinfo(c: &mut Ctx, node: Option<Vec<u8>>, serv: Option<Vec<u8>>, h: (i32, i32, i32, i32)) -> Result<GaiRes, Esc> {
    let n = node.map(|b| sys::cstr(&b));
    let s = serv.map(|b| sys::cstr(&b));
    offload(c, move || sys::getaddrinfo(n.as_deref(), s.as_deref(), h).map_err(|e| (e, sys::errno())))
}
fn sockaddr_in(addr: [u8; 4], port: u16) -> Vec<u8> {
    let mut v = vec![0u8; SIN_LEN];
    v[0..2].copy_from_slice(&(AF_INET as u16).to_ne_bytes()); v[2..4].copy_from_slice(&port.to_be_bytes()); v[4..8].copy_from_slice(&addr);
    v
}
fn sockaddr_in6(addr: [u8; 16], port: u16) -> Vec<u8> {
    let mut v = vec![0u8; SIN6_LEN];
    v[0..2].copy_from_slice(&(AF_INET6 as u16).to_ne_bytes()); v[2..4].copy_from_slice(&port.to_be_bytes()); v[8..24].copy_from_slice(&addr);
    v
}
/// numeric_getaddrinfo of raddrinfo.c: numeric hosts are answered without the resolver.
fn numeric_getaddrinfo(node: Option<&[u8]>, serv: Option<&[u8]>, h: (i32, i32, i32, i32)) -> Option<Vec<sys::AddrInfo>> {
    let node = std::str::from_utf8(node?).ok()?;
    let port: u16 = match serv {
        None => 0,
        Some(s) => { if s.is_empty() || !s.iter().all(|b| b.is_ascii_digit()) { if !s.is_empty() { return None; } } let n: u64 = std::str::from_utf8(s).ok()?.parse().unwrap_or(0); if n >= 0x10000 { return None; } n as u16 }
    };
    let (fam, st, pr) = (h.1, h.2, h.3);
    let list = [(SOCK_STREAM, 6), (SOCK_DGRAM, 17), (3, 0)];
    let mut out = vec![];
    let mut push = |f: i32, sa: Vec<u8>| {
        for &(t, p) in &list {
            if (st == 0 || st == t) && (pr == 0 || p == 0 || pr == p) { out.push((0, f, t, if pr != 0 { pr } else { p }, sa.clone(), None)); }
        }
    };
    let v6 = if (fam == AF_UNSPEC || fam == AF_INET6) && node.bytes().all(|b| b.is_ascii_hexdigit() || b == b'.' || b == b':') { parse_ipv6(node) } else { None };
    if let Some(a) = v6 { push(AF_INET6, sockaddr_in6(a, port)); }
    else if (fam == AF_UNSPEC || fam == AF_INET) && node.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        if let Some(a) = parse_ipv4(node) { push(AF_INET, sockaddr_in(a, port)); }
    }
    if out.is_empty() { None } else { Some(out) }
}
fn ipv4_str(a: u32) -> String { let b = a.to_be_bytes(); format!("{}.{}.{}.{}", b[0] as u32, b[1] as u32, b[2] as u32, b[3] as u32) }
fn hex2(b: u8) -> String { const H: &[u8; 16] = b"0123456789abcdef"; String::from_utf8(vec![H[(b >> 4) as usize], H[(b & 15) as usize]]).unwrap_or_default() }
/// inet_pton(AF_INET): four decimal parts, no leading zeros.
fn parse_ipv4(s: &str) -> Option<[u8; 4]> {
    let mut out = [0u8; 4];
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 { return None; }
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() || p.len() > 3 || !p.bytes().all(|b| b.is_ascii_digit()) || (p.len() > 1 && p.starts_with('0')) { return None; }
        let n: u32 = p.bytes().fold(0, |a, b| a * 10 + (b - b'0') as u32);
        if n > 255 { return None; }
        out[i] = n as u8;
    }
    Some(out)
}
/// inet_pton(AF_INET6).
fn parse_ipv6(s: &str) -> Option<[u8; 16]> {
    let (head, tail) = match s.find("::") { Some(i) => (&s[..i], Some(&s[i + 2..])), None => (s, None) };
    if tail.is_some_and(|t| t.contains("::")) { return None; }
    let groups = |part: &str, last: bool| -> Option<Vec<u16>> {
        let mut v = vec![];
        if part.is_empty() { return Some(v); }
        let items: Vec<&str> = part.split(':').collect();
        for (i, it) in items.iter().enumerate() {
            if last && i + 1 == items.len() && it.contains('.') { let a = parse_ipv4(it)?; v.push(u16::from_be_bytes([a[0], a[1]])); v.push(u16::from_be_bytes([a[2], a[3]])); continue; }
            if it.is_empty() || it.len() > 4 || !it.bytes().all(|b| b.is_ascii_hexdigit()) { return None; }
            v.push(u16::from_str_radix(it, 16).ok()?);
        }
        Some(v)
    };
    let h = groups(head, tail.is_none())?;
    let t = match tail { Some(t) => groups(t, true)?, None => vec![] };
    let n = h.len() + t.len();
    if (tail.is_none() && n != 8) || (tail.is_some() && n > 7) { return None; }
    let mut g = h; g.extend(std::iter::repeat_n(0u16, 8 - n)); g.extend(t);
    let mut out = [0u8; 16];
    for (i, x) in g.iter().enumerate() { out[2 * i..2 * i + 2].copy_from_slice(&x.to_be_bytes()); }
    Some(out)
}
fn is_ip_text(s: &str) -> bool { parse_ipv4(s).is_some() || parse_ipv6(s).is_some() }
/// if_nametoindex(3) through /sys/class/net (no extra libc symbol).
fn if_nametoindex(name: &[u8]) -> u32 {
    let n = String::from_utf8_lossy(name);
    if n.is_empty() || n.contains('/') { return 0; }
    std::fs::read_to_string(format!("/sys/class/net/{}/ifindex", n)).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}
fn if_indextoname(idx: u32) -> Option<Vec<u8>> {
    for e in std::fs::read_dir("/sys/class/net").ok()?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if if_nametoindex(name.as_bytes()) == idx { return Some(name.into_bytes()); }
    }
    None
}
/// getservbyname(3) / getservbyport(3) from /etc/services ("files" in nsswitch): (name, port).
fn getservby(name: Option<&[u8]>, port: u32, proto: &[u8]) -> Option<(Vec<u8>, u32)> {
    let text = std::fs::read("/etc/services").ok()?;
    for line in text.split(|&b| b == b'\n') {
        let line = line.split(|&b| b == b'#').next().unwrap_or(&[]);
        let mut it = line.split(|b| b.is_ascii_whitespace()).filter(|w| !w.is_empty());
        let (Some(n), Some(pp)) = (it.next(), it.next()) else { continue };
        let Some(slash) = pp.iter().position(|&b| b == b'/') else { continue };
        if &pp[slash + 1..] != proto { continue; }
        let Ok(pn) = String::from_utf8_lossy(&pp[..slash]).parse::<u32>() else { continue };
        let hit = match name { Some(want) => n == want || it.any(|a| a == want), None => pn == port };
        if hit { return Some((n.to_vec(), pn)); }
    }
    None
}
/// raddrinfo_host_str.
fn host_str(c: &mut Ctx, host: Value, flags: &mut i32) -> Result<Option<Vec<u8>>, Esc> {
    if host == NIL { return Ok(None); }
    if is_integer(host) {
        let n = num_to_long(c, host)?;
        if n < 0 || n > u32::MAX as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too big to convert to 'unsigned int'", n)); }
        *flags |= k("AI_NUMERICHOST");
        return Ok(Some(ipv4_str(n as u32).into_bytes()));
    }
    let b = cstr_arg(c, host)?;
    if b.is_empty() || b == b"<any>" { *flags |= k("AI_NUMERICHOST"); return Ok(Some(b"0.0.0.0".to_vec())); }
    if b == b"<broadcast>" { *flags |= k("AI_NUMERICHOST"); return Ok(Some(b"255.255.255.255".to_vec())); }
    if b.len() >= 1025 { return arg_err(c, format!("hostname too long ({})", b.len())); }
    Ok(Some(b))
}
/// raddrinfo_port_str.
fn port_str(c: &mut Ctx, port: Value, flags: &mut i32) -> Result<Option<Vec<u8>>, Esc> {
    if port == NIL { return Ok(None); }
    if port.is_fixnum() { *flags |= k("AI_NUMERICSERV"); return Ok(Some(port.as_fix().to_string().into_bytes())); }
    let b = cstr_arg(c, port)?;
    if b.len() >= 32 { return arg_err(c, format!("service name too long ({})", b.len())); }
    Ok(Some(b))
}
/// rsock_getaddrinfo; hints = (flags, family, socktype, protocol).
fn getaddrinfo(c: &mut Ctx, host: Value, port: Value, mut h: (i32, i32, i32, i32), socktype_hack: bool, reason: &str) -> Result<Vec<sys::AddrInfo>, Esc> {
    let mut extra = 0;
    let hp = host_str(c, host, &mut extra)?;
    let pp = port_str(c, port, &mut extra)?;
    if socktype_hack && h.2 == 0 && pp.as_ref().is_some_and(|p| !p.is_empty() && p.iter().all(|b| b.is_ascii_digit())) { h.2 = SOCK_DGRAM; }
    h.0 |= extra;
    if let Some(r) = numeric_getaddrinfo(hp.as_deref(), pp.as_deref(), h) { return Ok(r); }
    // a non-blocking fiber's scheduler resolves the name (address_resolve), each address then numerically
    let s = crate::builtins::thread::scheduler_current(c);
    if s != NIL && hp.is_some() && h.0 & k("AI_NUMERICHOST") == 0 {
        if let Some(res) = crate::builtins::thread::sched_hook(c, s, "address_resolve", &[host])? {
            let mut out = vec![];
            let ips = if is_array(res) { c.ary(res).to_vec() } else { vec![] };
            for ip in ips {
                let mut x = 0;
                let ib = host_str(c, ip, &mut x)?;
                if let Some(r) = numeric_getaddrinfo(ib.as_deref(), pp.as_deref(), h) { out.extend(r); }
            }
            std::hint::black_box(res);   // keeps the addresses alive while they are read
            if out.is_empty() { return Err(resolution_error(c, reason, k("EAI_NONAME"), 0)); }
            return Ok(out);
        }
    }
    match real_getaddrinfo(c, hp.clone(), pp, h)? {
        Ok(r) => Ok(r),
        Err((code, eno)) => {
            if hp.as_ref().is_some_and(|h| h.last() == Some(&b'\n')) { return socket_error(c, "newline at the end of hostname"); }
            Err(resolution_error(c, reason, code, eno))
        }
    }
}
/// rsock_addrinfo.
fn addrinfo_list(c: &mut Ctx, host: Value, port: Value, family: i32, socktype: i32, flags: i32) -> Result<Vec<sys::AddrInfo>, Esc> {
    getaddrinfo(c, host, port, (flags, family, socktype, 0), true, "getaddrinfo")
}
fn getnameinfo(c: &mut Ctx, sa: &[u8], flags: i32) -> Result<Result<(Vec<u8>, Vec<u8>), (i32, i64)>, Esc> {
    let numeric = k("NI_NUMERICHOST");
    if flags & numeric != 0 { return Ok(sys::getnameinfo(sa, flags).map_err(|e| (e, sys::errno()))); }
    let sa = sa.to_vec();
    offload(c, move || sys::getnameinfo(&sa, flags).map_err(|e| (e, sys::errno())))
}
fn numeric_flags() -> i32 { k("NI_NUMERICHOST") | k("NI_NUMERICSERV") }
/// rsock_make_ipaddr: the numeric host of a sockaddr.
fn make_ipaddr(c: &mut Ctx, sa: &[u8]) -> RV {
    match getnameinfo(c, sa, k("NI_NUMERICHOST"))? {
        Ok((h, _)) => Ok(bin(c, &h)),
        Err((e, eno)) => Err(resolution_error(c, "getnameinfo", e, eno)),
    }
}
/// rsock_ipaddr: [family, port, host, numeric host].
fn ipaddr_ary(c: &mut Ctx, sa: &[u8], norev: bool) -> RV { let v = ipaddr_vec(c, sa, norev)?; Ok(c.ary_new_from(&v)) }
fn ipaddr_vec(c: &mut Ctx, sa: &[u8], norev: bool) -> Result<Vec<Value>, Esc> {
    let f = afamily(sa);
    let fam = match int_to_name(&G_AF, f) { Some(n) => usascii(c, &n), None => usascii(c, &format!("unknown:{}", f)) };
    let mut a1 = None;
    if !norev { if let Ok((h, _)) = getnameinfo(c, sa, 0)? { a1 = Some(h); } }
    let (h, p) = match getnameinfo(c, sa, numeric_flags())? { Ok(r) => r, Err((e, eno)) => return Err(resolution_error(c, "getnameinfo", e, eno)) };
    let a2 = bin(c, &h);
    let a1 = match a1 { Some(x) => bin(c, &x), None => a2 };
    let port: i64 = String::from_utf8_lossy(&p).parse().unwrap_or(0);
    Ok(vec![fam, Value::fix(port), a1, a2])
}
/// unixsocket_len: the path length of a sockaddr_un (trailing NULs dropped); negative when too short.
fn unix_path_len(sa: &[u8]) -> isize {
    let mut e = sa.len();
    while e > 2 && sa[e - 1] == 0 { e -= 1; }
    e as isize - 2
}
fn unix_path(sa: &[u8]) -> &[u8] { let n = unix_path_len(sa); if n > 0 { &sa[2..2 + n as usize] } else { &[] } }
fn unixaddr(c: &mut Ctx, sa: &[u8]) -> Value {
    let f = usascii(c, "AF_UNIX");
    let p = bin(c, unix_path(sa));
    c.ary_new_from(&[f, p])
}
/// sockaddr_un for `path` with rsock_unix_sockaddr_len's length.
fn make_sockaddr_un(c: &mut Ctx, path: &[u8], what: &str) -> Result<Vec<u8>, Esc> {
    if path.len() > SUN_LEN - 2 {
        return arg_err(c, match what {
            "init" => format!("too long unix socket path ({}bytes given but {}bytes max)", path.len(), SUN_LEN - 2),
            _ => format!("too long unix socket path ({} bytes given but {} bytes max)", path.len(), SUN_LEN - 2),
        });
    }
    let mut v = vec![0u8; SUN_LEN];
    v[0..2].copy_from_slice(&(AF_UNIX as u16).to_ne_bytes());
    v[2..2 + path.len()].copy_from_slice(path);
    let len = if path.is_empty() { 2 } else if path[0] == 0 { 2 + path.len() } else { SUN_LEN };
    v.truncate(len);
    Ok(v)
}

// ---- constant arguments (constants.c) -------------------------------------------------------
fn constant_arg(c: &mut Ctx, v: Value, g: &Grp, msg: &str) -> Result<i32, Esc> {
    let name = if let Some(n) = sym_arg(v) { Some(n) } else if is_string(v) || (v.is_heap() && respond_to(c, v, s::to_str, true) && !is_integer(v)) {
        let s = expect_str(c, v)?; Some(c.str_bytes(s).to_vec())
    } else { None };
    match name {
        Some(n) => match name_to_int(g, &n) {
            Some(x) => Ok(x),
            None => { let n = String::from_utf8_lossy(n.split(|&b| b == 0).next().unwrap_or(&[])).into_owned(); socket_error(c, format!("{}: {}", msg, n)) }
        },
        None => to_i32(c, v),
    }
}
fn family_arg(c: &mut Ctx, v: Value) -> Result<i32, Esc> { constant_arg(c, v, &G_FAMILY, "unknown socket domain") }
fn socktype_arg(c: &mut Ctx, v: Value) -> Result<i32, Esc> { constant_arg(c, v, &G_SOCKTYPE, "unknown socket type") }
fn level_arg(c: &mut Ctx, family: i32, v: Value) -> Result<i32, Esc> {
    if is_ip(family) { constant_arg(c, v, &G_IPLEVEL, "unknown protocol level") } else { constant_arg(c, v, &G_ULEVEL, "unknown protocol level") }
}
fn optname_arg(c: &mut Ctx, family: i32, level: i32, v: Value) -> Result<i32, Esc> {
    if level == SOL_SOCKET { return constant_arg(c, v, &G_SO, "unknown socket level option name"); }
    if is_ip(family) {
        if level == 0 { return constant_arg(c, v, &G_IP, "unknown IP level option name"); }
        if level == 41 { return constant_arg(c, v, &G_IPV6, "unknown IPv6 level option name"); }
        if level == 6 { return constant_arg(c, v, &G_TCP, "unknown TCP level option name"); }
        if level == 17 { return constant_arg(c, v, &G_UDP, "unknown UDP level option name"); }
    }
    to_i32(c, v)
}
fn cmsg_type_arg(c: &mut Ctx, family: i32, level: i32, v: Value) -> Result<i32, Esc> {
    if level == SOL_SOCKET { return constant_arg(c, v, &G_SCM, "unknown UNIX control message"); }
    if is_ip(family) {
        if level == 0 { return constant_arg(c, v, &G_IP, "unknown IP control message"); }
        if level == 41 { return constant_arg(c, v, &G_IPV6, "unknown IPv6 control message"); }
        if level == 6 { return constant_arg(c, v, &G_TCP, "unknown TCP control message"); }
        if level == 17 { return constant_arg(c, v, &G_UDP, "unknown UDP control message"); }
    }
    to_i32(c, v)
}

// ---- Addrinfo ---------------------------------------------------------------------------------
pub struct RAddr { init: bool, sa: Vec<u8>, pfamily: i32, socktype: i32, protocol: i32, canonname: Value, inspectname: Value }
impl RData for RAddr {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.canonname); m.visit(&mut self.inspectname); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
#[derive(Clone)]
struct Ai { sa: Vec<u8>, pfamily: i32, socktype: i32, protocol: i32, canonname: Value, inspectname: Value }
fn blank_addr() -> RAddr { RAddr { init: false, sa: vec![], pfamily: 0, socktype: 0, protocol: 0, canonname: NIL, inspectname: NIL } }
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let k = C_ADDRINFO.get(c);
    if k != NIL && crate::builtins::inherits(c, klass, k) { return Ok(Some(c.alloc(klass, Body::Data(Box::new(blank_addr()))))); }
    Ok(None)
}
fn is_addrinfo(c: &Ctx, v: Value) -> bool { c.data::<RAddr>(v).is_some_and(|a| a.init) }
fn get_ai(c: &mut Ctx, v: Value) -> Result<Ai, Esc> {
    match c.data::<RAddr>(v) {
        Some(a) if a.init => Ok(Ai { sa: a.sa.clone(), pfamily: a.pfamily, socktype: a.socktype, protocol: a.protocol, canonname: a.canonname, inspectname: a.inspectname }),
        _ => type_err(c, "uninitialized socket address"),
    }
}
fn set_ai(c: &mut Ctx, v: Value, ai: Ai) -> Result<(), Esc> {
    if ai.sa.len() > SS_LEN { return arg_err(c, "sockaddr string too big"); }
    let a = c.data_mut::<RAddr>(v).expect("an Addrinfo");
    *a = RAddr { init: true, sa: ai.sa, pfamily: ai.pfamily, socktype: ai.socktype, protocol: ai.protocol, canonname: ai.canonname, inspectname: ai.inspectname };
    Ok(())
}
fn addrinfo_new(c: &mut Ctx, ai: Ai) -> RV {
    let k = C_ADDRINFO.get(c);
    let v = c.alloc(k, Body::Data(Box::new(blank_addr())));
    set_ai(c, v, ai)?;
    Ok(v)
}
fn ai_simple(sa: Vec<u8>, pf: i32, st: i32, pr: i32) -> Ai { Ai { sa, pfamily: pf, socktype: st, protocol: pr, canonname: NIL, inspectname: NIL } }
/// rsock_inspect_sockaddr.
fn inspect_sockaddr(c: &mut Ctx, sa: &[u8]) -> Result<Vec<u8>, Esc> {
    let n = sa.len();
    let mut r = String::new();
    if n == 0 { return Ok(b"empty-sockaddr".to_vec()); }
    if n < 2 { return Ok(b"too-short-sockaddr".to_vec()); }
    match afamily(sa) {
        AF_UNSPEC => r.push_str("UNSPEC"),
        AF_INET => {
            for i in 0..4 { if i > 0 { r.push('.'); } if 4 + i < n { r.push_str(&sa[4 + i].to_string()); } else { r.push('?'); } }
            if 4 < n { let p = be16_at(sa, 2); if p != 0 { r.push_str(&format!(":{}", p)); } } else { r.push_str(":?"); }
            if n != SIN_LEN { r.push_str(&format!(" ({} bytes for {} bytes sockaddr_in)", n, SIN_LEN)); }
        }
        AF_INET6 => {
            if n < SIN6_LEN { r.push_str(&format!("too-short-AF_INET6-sockaddr {} bytes", n)); } else {
                let h = match getnameinfo(c, sa, numeric_flags())? { Ok((h, _)) => h, Err((e, eno)) => return Err(resolution_error(c, "getnameinfo", e, eno)) };
                let h = String::from_utf8_lossy(&h).into_owned();
                let p = be16_at(sa, 2);
                if p == 0 { r.push_str(&h); } else { r.push_str(&format!("[{}]:{}", h, p)); }
                if n > SIN6_LEN { r.push_str(&format!("(sockaddr {} bytes too long)", n - SIN6_LEN)); }
            }
        }
        AF_UNIX => {
            let len = unix_path_len(sa);
            if len < 0 { r.push_str("too-short-AF_UNIX-sockaddr"); } else if len == 0 { r.push_str("empty-path-AF_UNIX-sockaddr"); } else {
                let p = &sa[2..2 + len as usize];
                if p.iter().all(|&b| (0x21..0x7f).contains(&b)) {
                    if p[0] != b'/' { r.push_str("UNIX "); }
                    let mut out = r.into_bytes(); out.extend_from_slice(p); return Ok(out);
                }
                r.push_str("UNIX");
                for b in p { r.push_str(&format!(":{}", hex2(*b))); }
            }
        }
        AF_PACKET => {
            let mut sep = "[";
            let mut cat = |r: &mut String, s: &str| { r.push_str(sep); sep = " "; r.push_str(s); };
            r.push_str("PACKET");
            if n >= 4 { cat(&mut r, &format!("protocol={}", be16_at(sa, 2))); }
            if n >= 8 { let i = i32_at(sa, 4); match if_indextoname(i as u32) { Some(nm) => cat(&mut r, &String::from_utf8_lossy(&nm)), None => cat(&mut r, &format!("ifindex={}", i)) } }
            if n >= 10 { cat(&mut r, &format!("hatype={}", u16_at(sa, 8))); }
            if n >= 11 {
                let s = match sa[10] { 0 => "HOST".to_string(), 1 => "BROADCAST".into(), 2 => "MULTICAST".into(), 3 => "OTHERHOST".into(), 4 => "OUTGOING".into(), x => format!("pkttype={}", x) };
                cat(&mut r, &s);
            }
            let halen = if n >= 12 { sa[11] as usize } else { 0 };
            if n != 12 + halen && n >= 12 { cat(&mut r, &format!("halen={}", halen)); }
            if n > 12 {
                let len = halen.min(n - 12);
                let mut s = "hwaddr".to_string();
                for i in 0..len { s.push_str(if i == 0 { "=" } else { ":" }); s.push_str(&hex2(sa[12 + i])); }
                cat(&mut r, &s);
            }
            if n < 12 || 12 + halen != n { cat(&mut r, &format!("({} bytes for {} bytes sockaddr_ll)", n, 20)); }
            r.push(']');
        }
        f => match int_to_name(&G_AF, f) { Some(nm) => r.push_str(&format!("{} address format unknown", nm)), None => r.push_str(&format!("unknown address family {}", f)) },
    }
    Ok(r.into_bytes())
}
fn ai_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let ai = get_ai(c, slf)?;
    let mut r = format!("#<{}: ", class_path(c, real_class_of(c, slf))).into_bytes();
    r.extend(inspect_sockaddr(c, &ai.sa)?);
    let mut t = String::new();
    if ai.pfamily != 0 && afamily(&ai.sa) != ai.pfamily {
        match int_to_name(&G_PF, ai.pfamily) { Some(n) => t.push_str(&format!(" {}", n)), None => t.push_str(&format!(" PF_???({})", ai.pfamily)) }
    }
    let inet = is_ip(ai.pfamily);
    if inet && ai.socktype == SOCK_STREAM && (ai.protocol == 0 || ai.protocol == 6) { t.push_str(" TCP"); }
    else if inet && ai.socktype == SOCK_DGRAM && (ai.protocol == 0 || ai.protocol == 17) { t.push_str(" UDP"); }
    else {
        if ai.socktype != 0 { match int_to_name(&G_SOCKTYPE_NAME, ai.socktype) { Some(n) => t.push_str(&format!(" {}", n)), None => t.push_str(&format!(" SOCK_???({})", ai.socktype)) } }
        if ai.protocol != 0 {
            match if inet { int_to_name(&G_IPPROTO_NAME, ai.protocol) } else { None } { Some(n) => t.push_str(&format!(" {}", n)), None => t.push_str(&format!(" UNKNOWN_PROTOCOL({})", ai.protocol)) }
        }
    }
    r.extend_from_slice(t.as_bytes());
    if ai.canonname != NIL { r.push(b' '); let s = expect_str(c, ai.canonname)?; r.extend_from_slice(c.str_bytes(s)); }
    if ai.inspectname != NIL { r.extend_from_slice(b" ("); let s = expect_str(c, ai.inspectname)?; r.extend_from_slice(c.str_bytes(s)); r.push(b')'); }
    r.push(b'>');
    Ok(bin(c, &r))
}
fn ai_inspect_sockaddr(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let ai = get_ai(c, slf)?; let s = inspect_sockaddr(c, &ai.sa)?; Ok(bin(c, &s)) }
/// make_inspectname.
fn make_inspectname(c: &mut Ctx, node: Value, service: Value, first: Option<&sys::AddrInfo>) -> RV {
    let (mut node, mut service) = (node, service);
    if let Some(r) = first {
        if let Ok(Ok((h, p))) = getnameinfo(c, &r.4, numeric_flags()) {
            if is_string(node) && c.str_bytes(node) == h.as_slice() { node = NIL; }
            if is_string(service) && c.str_bytes(service) == p.as_slice() { service = NIL; }
            else if service.is_fixnum() && String::from_utf8_lossy(&p).parse::<i64>().unwrap_or(0) == service.as_fix() as i32 as i64 { service = NIL; }
        }
    }
    let mut out: Option<Vec<u8>> = if is_string(node) { Some(c.str_bytes(node).to_vec()) } else { None };
    if is_string(service) {
        let sv = cstr_arg(c, service)?;
        let o = out.get_or_insert_with(Vec::new); o.push(b':'); o.extend_from_slice(&sv);
    } else if service.is_fixnum() && service.as_fix() as i32 != 0 {
        let o = out.get_or_insert_with(Vec::new); o.extend_from_slice(format!(":{}", service.as_fix() as i32).as_bytes());
    }
    Ok(match out { Some(b) => { let enc = if is_string(node) { c.str_enc(node) } else { ENC_BINARY }; let v = c.str_new_enc(&b, enc); obj_freeze(v); v } None => NIL })
}
fn frozen_bin(c: &mut Ctx, b: &[u8]) -> Value { let v = bin(c, b); obj_freeze(v); v }
fn obj_freeze(v: Value) { crate::heap::obj_set_flags(v, FL_FROZEN); }
/// call_getaddrinfo: hints from Ruby values (nil: unspecified).
fn call_getaddrinfo(c: &mut Ctx, node: Value, service: Value, family: Value, socktype: Value, protocol: Value, flags: Value, hack: bool) -> Result<Vec<sys::AddrInfo>, Esc> {
    let f = if family == NIL { AF_UNSPEC } else { family_arg(c, family)? };
    let st = if socktype == NIL { 0 } else { socktype_arg(c, socktype)? };
    let pr = if protocol == NIL { 0 } else { to_i32(c, protocol)? };
    let fl = if flags == NIL { 0 } else { to_i32(c, flags)? };
    let r = getaddrinfo(c, node, service, (fl, f, st, pr), hack, "getaddrinfo")?;
    if r.is_empty() { return socket_error(c, "host not found"); }
    Ok(r)
}
fn canon(c: &mut Ctx, r: &sys::AddrInfo) -> Value { match &r.5 { Some(n) => { let n = n.clone(); frozen_bin(c, &n) } None => NIL } }
fn addrinfo_firstonly(c: &mut Ctx, node: Value, service: Value, family: i32, socktype: i32, protocol: i32) -> RV {
    let res = call_getaddrinfo(c, node, service, Value::fix(family as i64), Value::fix(socktype as i64), Value::fix(protocol as i64), Value::fix(0), false)?;
    let iname = make_inspectname(c, node, service, res.first())?;
    let r = &res[0];
    let cn = canon(c, r);
    addrinfo_new(c, Ai { sa: r.4.clone(), pfamily: r.1, socktype: r.2, protocol: r.3, canonname: cn, inspectname: iname })
}
fn ai_s_getaddrinfo(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (a, _kw) = kw_split(c, a);
    check_argc(c, a, 2, 6)?;
    let g = |i: usize| a.get(i).copied().unwrap_or(NIL);
    let res = call_getaddrinfo(c, a[0], a[1], g(2), g(3), g(4), g(5), false)?;
    let iname = make_inspectname(c, a[0], a[1], res.first())?;
    let out = c.ary_new();
    for r in &res {
        let cn = canon(c, r);
        let v = addrinfo_new(c, Ai { sa: r.4.clone(), pfamily: r.1, socktype: r.2, protocol: r.3, canonname: cn, inspectname: iname })?;
        c.ary_push(out, v);
    }
    Ok(out)
}
fn ai_s_ip(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let v = addrinfo_firstonly(c, a[0], NIL, AF_UNSPEC, 0, 0)?;
    let ad = c.data_mut::<RAddr>(v).unwrap(); ad.socktype = 0; ad.protocol = 0;
    Ok(v)
}
fn ai_s_tcp(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { addrinfo_firstonly(c, a[0], a[1], AF_UNSPEC, SOCK_STREAM, 6) }
fn ai_s_udp(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { addrinfo_firstonly(c, a[0], a[1], AF_UNSPEC, SOCK_DGRAM, 17) }
fn unix_ai(c: &mut Ctx, path: Value, socktype: i32) -> Result<Ai, Esc> {
    let p = expect_str(c, path)?;
    let b = c.str_bytes(p).to_vec();
    let sa = make_sockaddr_un(c, &b, "ai")?;
    Ok(ai_simple(sa, AF_UNIX, socktype, 0))
}
fn ai_s_unix(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let st = match a.get(1) { Some(&v) if v != NIL => socktype_arg(c, v)?, _ => SOCK_STREAM };
    let ai = unix_ai(c, a[0], st)?;
    addrinfo_new(c, ai)
}
fn ai_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if is_addrinfo(c, slf) { return type_err(c, "already initialized socket address"); }
    check_argc(c, a, 1, 4)?;
    let g = |i: usize| a.get(i).copied().unwrap_or(NIL);
    let pf = if g(1) == NIL { AF_UNSPEC } else { family_arg(c, g(1))? };
    let st = if g(2) == NIL { 0 } else { socktype_arg(c, g(2))? };
    let pr = if g(3) == NIL { 0 } else { to_i32(c, g(3))? };
    if let Some(ary) = check_array(c, a[0])? {
        let fam = expect_str(c, c.ary_get(ary, 0))?;
        let fb = c.str_bytes(fam).to_vec();
        let af = match name_to_int(&G_FAMILY, &fb) { Some(x) => x, None => { let n = String::from_utf8_lossy(&fb).into_owned(); return socket_error(c, format!("unknown address family: {}", n)); } };
        let ai = match af {
            AF_INET | AF_INET6 => {
                let svc = c.ary_get(ary, 1);
                let service = Value::fix(to_i32(c, svc)? as i64);
                let nodename = c.ary_get(ary, 2);
                if nodename != NIL { expect_str(c, nodename)?; }
                let numeric = expect_str(c, c.ary_get(ary, 3))?;
                let flags = k("AI_NUMERICHOST") | k("AI_NUMERICSERV");
                let res = call_getaddrinfo(c, numeric, service, Value::fix(if pf != 0 { pf } else { af } as i64), Value::fix(st as i64), Value::fix(pr as i64), Value::fix(flags as i64), true)?;
                let same = is_string(nodename) && c.str_bytes(nodename) == c.str_bytes(numeric);
                let iname = if same { NIL } else { make_inspectname(c, nodename, service, res.first())? };
                let cn = canon(c, &res[0]);
                Ai { sa: res[0].4.clone(), pfamily: if pf != 0 { pf } else { af }, socktype: st, protocol: pr, canonname: cn, inspectname: iname }
            }
            AF_UNIX => { let p = c.ary_get(ary, 1); let p = expect_str(c, p)?; let mut ai = unix_ai(c, p, SOCK_STREAM)?; ai.pfamily = AF_UNIX; ai }
            _ => return socket_error(c, "unexpected address family"),
        };
        set_ai(c, slf, ai)?;
    } else {
        let s = expect_str(c, a[0])?;
        let sa = c.str_bytes(s).to_vec();
        set_ai(c, slf, ai_simple(sa, pf, st, pr))?;
    }
    Ok(slf)
}
fn ai_int(c: &mut Ctx, slf: Value, f: fn(&Ai) -> i32) -> RV { let ai = get_ai(c, slf)?; Ok(Value::fix(f(&ai) as i64)) }
fn ai_afamily(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { ai_int(c, s, |a| afamily(&a.sa)) }
fn ai_pfamily(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { ai_int(c, s, |a| a.pfamily) }
fn ai_socktype(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { ai_int(c, s, |a| a.socktype) }
fn ai_protocol(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { ai_int(c, s, |a| a.protocol) }
fn ai_to_sockaddr(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let ai = get_ai(c, s)?; Ok(bin(c, &ai.sa)) }
fn ai_canonname(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(get_ai(c, s)?.canonname) }
fn ai_ip_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let ai = get_ai(c, s)?; Ok(Value::bool(is_ip(afamily(&ai.sa)))) }
fn ai_ipv4_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let ai = get_ai(c, s)?; Ok(Value::bool(afamily(&ai.sa) == AF_INET)) }
fn ai_ipv6_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let ai = get_ai(c, s)?; Ok(Value::bool(afamily(&ai.sa) == AF_INET6)) }
fn ai_unix_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let ai = get_ai(c, s)?; Ok(Value::bool(afamily(&ai.sa) == AF_UNIX)) }
fn ai_getnameinfo(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let ai = get_ai(c, slf)?;
    let mut fl = match a.first() { Some(&v) if v != NIL => to_i32(c, v)?, _ => 0 };
    if ai.socktype == SOCK_DGRAM { fl |= k("NI_DGRAM"); }
    match getnameinfo(c, &ai.sa, fl)? {
        Ok((h, p)) => { let (h, p) = (bin(c, &h), bin(c, &p)); Ok(c.ary_new_from(&[h, p])) }
        Err((e, eno)) => Err(resolution_error(c, "getnameinfo", e, eno)),
    }
}
fn need_ip(c: &mut Ctx, ai: &Ai) -> Result<(), Esc> { if !is_ip(afamily(&ai.sa)) { return socket_error(c, "need IPv4 or IPv6 address"); } Ok(()) }
fn ai_ip_unpack(c: &mut Ctx, slf: Value, _a: &[Value], b: Blk) -> RV {
    let ai = get_ai(c, slf)?; need_ip(c, &ai)?;
    let r = ai_getnameinfo(c, slf, &[Value::fix(numeric_flags() as i64)], b)?;
    let (h, p) = (c.ary_get(r, 0), c.ary_get(r, 1));
    let n: i64 = String::from_utf8_lossy(c.str_bytes(p)).parse().unwrap_or(0);
    Ok(c.ary_new_from(&[h, Value::fix(n)]))
}
fn ai_ip_address(c: &mut Ctx, slf: Value, _a: &[Value], b: Blk) -> RV {
    let ai = get_ai(c, slf)?; need_ip(c, &ai)?;
    let r = ai_getnameinfo(c, slf, &[Value::fix(numeric_flags() as i64)], b)?;
    Ok(c.ary_get(r, 0))
}
fn ai_ip_port(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let ai = get_ai(c, slf)?; need_ip(c, &ai)?;
    let f = afamily(&ai.sa);
    if f == AF_INET && ai.sa.len() != SIN_LEN { return socket_error(c, "unexpected sockaddr size for IPv4"); }
    if f == AF_INET6 && ai.sa.len() != SIN6_LEN { return socket_error(c, "unexpected sockaddr size for IPv6"); }
    Ok(Value::fix(be16_at(&ai.sa, 2) as i64))
}
fn in4(ai: &Ai) -> Option<u32> { if afamily(&ai.sa) == AF_INET && ai.sa.len() >= 8 { Some(u32::from_be_bytes([ai.sa[4], ai.sa[5], ai.sa[6], ai.sa[7]])) } else { None } }
fn in6(ai: &Ai) -> Option<[u8; 16]> { if afamily(&ai.sa) == AF_INET6 && ai.sa.len() >= 24 { let mut x = [0u8; 16]; x.copy_from_slice(&ai.sa[8..24]); Some(x) } else { None } }
macro_rules! ai_pred4 { ($n:ident, $f:expr) => { fn $n(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let ai = get_ai(c, s)?; Ok(Value::bool(in4(&ai).is_some_and($f))) } }; }
macro_rules! ai_pred6 { ($n:ident, $f:expr) => { fn $n(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let ai = get_ai(c, s)?; Ok(Value::bool(in6(&ai).is_some_and(|a: [u8; 16]| $f(&a)))) } }; }
ai_pred4!(ai_ipv4_private, |a: u32| a & 0xff000000 == 0x0a000000 || a & 0xfff00000 == 0xac100000 || a & 0xffff0000 == 0xc0a80000);
ai_pred4!(ai_ipv4_loopback, |a: u32| a & 0xff000000 == 0x7f000000);
ai_pred4!(ai_ipv4_multicast, |a: u32| a & 0xf0000000 == 0xe0000000);
fn v6_zero(a: &[u8], n: usize) -> bool { a[..n].iter().all(|&b| b == 0) }
ai_pred6!(ai_ipv6_unspecified, |a: &[u8; 16]| v6_zero(a, 16));
ai_pred6!(ai_ipv6_loopback, |a: &[u8; 16]| v6_zero(a, 15) && a[15] == 1);
ai_pred6!(ai_ipv6_multicast, |a: &[u8; 16]| a[0] == 0xff);
ai_pred6!(ai_ipv6_linklocal, |a: &[u8; 16]| a[0] == 0xfe && a[1] & 0xc0 == 0x80);
ai_pred6!(ai_ipv6_sitelocal, |a: &[u8; 16]| a[0] == 0xfe && a[1] & 0xc0 == 0xc0);
ai_pred6!(ai_ipv6_unique_local, |a: &[u8; 16]| a[0] & 0xfe == 0xfc);
ai_pred6!(ai_ipv6_v4mapped, |a: &[u8; 16]| v6_zero(a, 10) && a[10] == 0xff && a[11] == 0xff);
ai_pred6!(ai_ipv6_v4compat, |a: &[u8; 16]| v6_zero(a, 12) && !(v6_zero(a, 15) && a[15] <= 1));
ai_pred6!(ai_ipv6_mc_nodelocal, |a: &[u8; 16]| a[0] == 0xff && a[1] & 0xf == 1);
ai_pred6!(ai_ipv6_mc_linklocal, |a: &[u8; 16]| a[0] == 0xff && a[1] & 0xf == 2);
ai_pred6!(ai_ipv6_mc_sitelocal, |a: &[u8; 16]| a[0] == 0xff && a[1] & 0xf == 5);
ai_pred6!(ai_ipv6_mc_orglocal, |a: &[u8; 16]| a[0] == 0xff && a[1] & 0xf == 8);
ai_pred6!(ai_ipv6_mc_global, |a: &[u8; 16]| a[0] == 0xff && a[1] & 0xf == 0xe);
fn ai_ipv6_to_ipv4(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let ai = get_ai(c, s)?;
    let Some(a) = in6(&ai) else { return Ok(NIL) };
    let mapped = v6_zero(&a, 10) && a[10] == 0xff && a[11] == 0xff;
    let compat = v6_zero(&a, 12) && !(v6_zero(&a, 15) && a[15] <= 1);
    if !mapped && !compat { return Ok(NIL); }
    let sa = sockaddr_in([a[12], a[13], a[14], a[15]], 0);
    addrinfo_new(c, Ai { sa, pfamily: AF_INET, socktype: ai.socktype, protocol: ai.protocol, canonname: ai.canonname, inspectname: ai.inspectname })
}
fn ai_unix_path(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let ai = get_ai(c, s)?;
    if afamily(&ai.sa) != AF_UNIX { return socket_error(c, "need AF_UNIX address"); }
    let n = unix_path_len(&ai.sa);
    if n < 0 { return socket_error(c, format!("too short AF_UNIX address: {} bytes given for minimum 2 bytes.", ai.sa.len())); }
    if n as usize > SUN_LEN - 2 { return socket_error(c, format!("too long AF_UNIX path ({} bytes given but {} bytes max)", n, SUN_LEN - 2)); }
    Ok(bin(c, unix_path(&ai.sa)))
}
fn ai_mdump(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let ai = get_ai(c, s)?;
    let af = afamily(&ai.sa);
    let pf = match int_to_name(&G_PF, ai.pfamily) { Some(n) => usascii(c, &n), None => return socket_error(c, format!("unknown protocol family: {}", ai.pfamily)) };
    let st = if ai.socktype == 0 { Value::fix(0) } else { match int_to_name(&G_SOCKTYPE_NAME, ai.socktype) { Some(n) => usascii(c, &n), None => return socket_error(c, format!("unknown socktype: {}", ai.socktype)) } };
    let pr = if ai.protocol == 0 { Value::fix(0) } else if is_ip(af) {
        match int_to_name(&G_IPPROTO_NAME, ai.protocol) { Some(n) => usascii(c, &n), None => return socket_error(c, format!("unknown IP protocol: {}", ai.protocol)) }
    } else { return socket_error(c, format!("unknown protocol: {}", ai.protocol)); };
    let afs = match int_to_name(&G_AF, af) { Some(n) => usascii(c, &n), None => return socket_error(c, format!("unknown address family: {}", af)) };
    let sa = if af == AF_UNIX { bin(c, unix_path(&ai.sa)) } else {
        match getnameinfo(c, &ai.sa, numeric_flags())? {
            Ok((h, p)) => { let (h, p) = (bin(c, &h), bin(c, &p)); c.ary_new_from(&[h, p]) }
            Err((e, eno)) => return Err(resolution_error(c, "getnameinfo", e, eno)),
        }
    };
    Ok(c.ary_new_from(&[afs, sa, pf, st, pr, ai.canonname, ai.inspectname]))
}
fn ai_mload(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if is_addrinfo(c, slf) { return type_err(c, "already initialized socket address"); }
    let ary = crate::array::expect_array(c, a[0])?;
    let fam_name = |c: &mut Ctx, v: Value| -> Result<Option<i32>, Esc> { let s = expect_str(c, v)?; Ok(name_to_int(&G_FAMILY, c.str_bytes(s))) };
    let v0 = c.ary_get(ary, 0);
    let Some(af) = fam_name(c, v0)? else { return type_err(c, "unexpected address family") };
    let v2 = c.ary_get(ary, 2);
    let Some(pf) = fam_name(c, v2)? else { return type_err(c, "unexpected protocol family") };
    let v3 = c.ary_get(ary, 3);
    let st = if v3 == Value::fix(0) { 0 } else { let s = expect_str(c, v3)?; match name_to_int(&G_SOCKTYPE, c.str_bytes(s)) { Some(x) => x, None => return type_err(c, "unexpected socktype") } };
    let v4 = c.ary_get(ary, 4);
    let pr = if v4 == Value::fix(0) { 0 } else {
        let s = expect_str(c, v4)?;
        if !is_ip(af) { return type_err(c, "unexpected protocol"); }
        match name_to_int(&G_IPPROTO, c.str_bytes(s)) { Some(x) => x, None => return type_err(c, "unexpected protocol") }
    };
    let v5 = c.ary_get(ary, 5);
    let cn = if v5 == NIL { NIL } else { expect_str(c, v5)? };
    let v6 = c.ary_get(ary, 6);
    let iname = if v6 == NIL { NIL } else { expect_str(c, v6)? };
    let v1 = c.ary_get(ary, 1);
    let sa = if af == AF_UNIX {
        let s = expect_str(c, v1)?;
        let p = c.str_bytes(s).to_vec();
        if p.len() > SUN_LEN - 2 { return socket_error(c, format!("too long AF_UNIX path ({} bytes given but {} bytes max)", p.len(), SUN_LEN - 2)); }
        let mut v = vec![0u8; SUN_LEN]; v[0..2].copy_from_slice(&(AF_UNIX as u16).to_ne_bytes()); v[2..2 + p.len()].copy_from_slice(&p);
        v
    } else {
        let pair = crate::array::expect_array(c, v1)?;
        let (h, p) = (c.ary_get(pair, 0), c.ary_get(pair, 1));
        let flags = k("AI_NUMERICHOST") | k("AI_NUMERICSERV");
        let res = call_getaddrinfo(c, h, p, Value::fix(pf as i64), Value::fix(st as i64), Value::fix(pr as i64), Value::fix(flags as i64), true)?;
        res[0].4.clone()
    };
    set_ai(c, slf, Ai { sa, pfamily: pf, socktype: st, protocol: pr, canonname: cn, inspectname: iname })?;
    Ok(slf)
}
/// SockAddrStringValueWithAddrinfo: (sockaddr bytes, the Addrinfo if one was given).
fn sockaddr_arg(c: &mut Ctx, v: Value) -> Result<(Vec<u8>, Option<Value>), Esc> {
    if is_addrinfo(c, v) { let ai = get_ai(c, v)?; return Ok((ai.sa, Some(v))); }
    let s = expect_str(c, v)?;
    Ok((c.str_bytes(s).to_vec(), None))
}
/// rsock_fd_socket_addrinfo.
fn fd_socket_addrinfo(c: &mut Ctx, fd: i32, sa: &[u8]) -> RV {
    let mut b = [0u8; 4];
    if let Err(e) = sys::getsockopt(fd, SOL_SOCKET, k("SO_TYPE"), &mut b) { return Err(sys_fail(c, e, "getsockopt(SO_TYPE)")); }
    addrinfo_new(c, ai_simple(sa.to_vec(), afamily(sa), i32::from_ne_bytes(b), 0))
}

// ---- sockets ----------------------------------------------------------------------------------
fn norev_sym() -> Sym { intern(b"__norevlookup") }
/// rsock_init_sock: a read-write, binary, synchronized stream on fd.
fn init_sock(c: &mut Ctx, v: Value, fd: i32) -> RV {
    let Some(io) = c.data_mut::<RIo>(v) else { return type_err(c, "not an IO") };
    io.fd = fd; io.wfd = -1; io.readable = true; io.writable = true; io.init = true; io.closed = false; io.eof = false;
    io.rbuf.clear(); io.rpos = 0; io.wbuf.clear(); io.binmode = true; io.enc = Some(ENC_BINARY); io.enc2 = None; io.sync = true;
    io.autoclose = true; io.path.clear(); io.has_path = false; io.lineno = 0; io.rerr = 0;
    crate::builtins::io::conv_note(io);
    ivar_set_raw(c, v, norev_sym(), Value::bool(NOREVLOOKUP.load(Relaxed)));
    Ok(v)
}
fn new_sock(c: &mut Ctx, klass: Value, fd: i32) -> RV {
    let v = crate::builtins::object::alloc_instance(c, klass)?;
    init_sock(c, v, fd)
}
fn sock_norev(c: &Ctx, v: Value) -> bool { ivar_get(c, v, norev_sym()) != FALSE }
fn rsock_socket(d: i32, t: i32, p: i32) -> i32 { sys::socket(d, t | k("SOCK_CLOEXEC") | k("SOCK_NONBLOCK"), p) }
fn sfd(c: &mut Ctx, v: Value) -> Result<i32, Esc> { Ok(io::get_io(c, v)?.fd) }
fn getfamily(fd: i32) -> i32 { let mut b = [0u8; SS_LEN]; match sys::getsockname(fd, &mut b) { Ok(n) => afamily(&b[..n.min(SS_LEN)]), Err(_) => AF_UNSPEC } }
fn is_dgram(c: &mut Ctx, fd: i32) -> Result<bool, Esc> {
    let mut b = [0u8; 4];
    if let Err(e) = sys::getsockopt(fd, SOL_SOCKET, k("SO_TYPE"), &mut b) { return Err(sys_fail(c, e, "getsockopt(SO_TYPE)")); }
    Ok(i32::from_ne_bytes(b) == SOCK_DGRAM)
}
/// Wait for `ev` on the socket's fd (other threads run); IOError if it was closed meanwhile.
fn wait(c: &mut Ctx, v: Value, ev: i16) -> Result<(), Esc> {
    let fd = sfd(c, v)?;
    let dl = io::io_timeout(c, v).map(|t| std::time::Instant::now() + t);
    let tv = io::io_timeout_value(c, v);
    let r = io::wait_one(c, v, fd, ev, dl, tv)?;
    sfd(c, v)?;
    // rb_io_wait with the IO's timeout
    if r.is_none() { return Err(timeout_error(c, if ev == sys::POLLOUT { "Timed out waiting for IO to become writable!" } else { "Timed out waiting for IO to become readable!" })); }
    Ok(())
}
fn so_error(fd: i32) -> Result<i64, i64> { let mut b = [0u8; 4]; sys::getsockopt(fd, SOL_SOCKET, k("SO_ERROR"), &mut b)?; Ok(i32::from_ne_bytes(b) as i64) }
/// rsock_connect: Ok(Err(errno)) when connect(2) failed.
fn do_connect(c: &mut Ctx, v: Value, sa: &[u8], deadline: Option<std::time::Instant>) -> Result<Result<(), i64>, Esc> {
    let deadline = deadline.or_else(|| io::io_timeout(c, v).map(|t| std::time::Instant::now() + t));
    let fd = sfd(c, v)?;
    if sys::connect(fd, sa) == 0 { return Ok(Ok(())); }
    let e = sys::errno();
    if e != sys::EINTR && e != sys::EAGAIN && e != EINPROGRESS { return Ok(Err(e)); }
    // wait_connectable
    match so_error(fd) { Err(e) => return Ok(Err(e)), Ok(0) => {} Ok(x) if [114, 106, 111, 113].contains(&x) => return Ok(Err(x)), Ok(_) => {} }
    let r = io::wait_one(c, v, fd, sys::POLLIN | sys::POLLOUT, deadline, UNDEF)?;
    let fd = sfd(c, v)?;
    if r.is_none() {
        let s = inspect_sockaddr(c, sa)?;
        return Err(timeout_error(c, &format!("user specified timeout for {}", String::from_utf8_lossy(&s))));
    }
    match so_error(fd) { Err(e) => Ok(Err(e)), Ok(x) if [0, sys::EINTR, sys::EAGAIN, EINPROGRESS, 114, 106].contains(&x) => Ok(Ok(())), Ok(x) => Ok(Err(x)) }
}
/// rsock_s_accept: (fd, peer sockaddr).
fn do_accept(c: &mut Ctx, srv: Value) -> Result<(i32, Vec<u8>), Esc> {
    loop {
        let fd = sfd(c, srv)?;
        let mut b = [0u8; SS_LEN];
        match sys::accept4(fd, &mut b, k("SOCK_CLOEXEC") | k("SOCK_NONBLOCK")) {
            Ok((nfd, n)) => return Ok((nfd, b[..n.min(SS_LEN)].to_vec())),
            Err(e) if e == sys::EAGAIN || e == sys::EINTR || e == 103 || e == 71 => { if e != sys::EINTR { wait(c, srv, sys::POLLIN)?; } }
            Err(e) => return Err(sys_fail(c, e, "accept(2)")),
        }
    }
}
fn accept_nonblock(c: &mut Ctx, srv: Value, ex: Value) -> Result<Result<(i32, Vec<u8>), Value>, Esc> {
    let fd = sfd(c, srv)?;
    let mut b = [0u8; SS_LEN];
    match sys::accept4(fd, &mut b, k("SOCK_CLOEXEC") | k("SOCK_NONBLOCK")) {
        Ok((nfd, n)) => Ok(Ok((nfd, b[..n.min(SS_LEN)].to_vec()))),
        Err(e) if e == sys::EAGAIN || e == 103 || e == 71 => {
            if ex == FALSE { return Ok(Err(sym_value("wait_readable"))); }
            Err(rw_fail(c, true, e, "accept(2) would block"))
        }
        Err(e) => Err(sys_fail(c, e, "accept(2)")),
    }
}

// BasicSocket
fn bs_s_for_fd(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let fd = to_i32(c, a[0])?;
    use std::os::unix::fs::FileTypeExt;
    match sys::fd_metadata(fd) {
        Err(e) => return Err(sys_fail(c, crate::builtins::exception::io_err_to_errno(&e), "fstat(2)")),
        Ok(m) if !m.file_type().is_socket() => return Err(sys_fail(c, 9, "not a socket file descriptor")),
        Ok(_) => {}
    }
    new_sock(c, slf, fd)
}
/// rb_io_s_new: a block is ignored with a warning.
fn bs_s_new(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if blk_given(b) { let n = class_path(c, slf); rb_warn(c, &format!("{}::new() does not take block; use {}::open() instead", n, n))?; }
    crate::builtins::object::c_new(c, slf, a, Blk::NONE)
}
fn bs_s_norev(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(NOREVLOOKUP.load(Relaxed))) }
fn bs_s_set_norev(_c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { NOREVLOOKUP.store(a[0].truthy(), Relaxed); Ok(a[0]) }
fn bs_norev(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { io::get_io(c, s)?; Ok(Value::bool(sock_norev(c, s))) }
fn bs_set_norev(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { io::get_io(c, s)?; ivar_set_raw(c, s, norev_sym(), Value::bool(a[0].truthy())); Ok(s) }
fn bs_shutdown(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let how = match a.first() {
        Some(&v) if v != NIL => {
            let h = constant_arg(c, v, &G_SHUT, "unknown shutdown argument")?;
            if !(0..=2).contains(&h) { return arg_err(c, "`how' should be either :SHUT_RD, :SHUT_WR, :SHUT_RDWR"); }
            h
        }
        _ => 2,
    };
    let fd = sfd(c, s)?;
    if sys::shutdown(fd, how) < 0 { return Err(sys_fail(c, sys::errno(), "shutdown(2)")); }
    Ok(Value::fix(0))
}
fn bs_close_read(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let io = io::get_io(c, s)?;
    sys::shutdown(io.fd, 0);
    if !io.writable { return io::close_io(c, s); }
    io.readable = false;
    Ok(NIL)
}
fn bs_close_write(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let io = io::get_io(c, s)?;
    if !io.readable { return io::close_io(c, s); }
    io::flush_one(io);
    sys::shutdown(io.fd, 1);
    io.writable = false;
    Ok(NIL)
}
fn bs_setsockopt(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (lev, opt, val) = if a.len() == 1 {
        (funcall(c, a[0], intern(b"level"), &[], std::ptr::null())?, funcall(c, a[0], intern(b"optname"), &[], std::ptr::null())?, funcall(c, a[0], intern(b"data"), &[], std::ptr::null())?)
    } else { check_argc(c, a, 3, 3)?; (a[0], a[1], a[2]) };
    let fd = sfd(c, s)?;
    let family = getfamily(fd);
    let level = level_arg(c, family, lev)?;
    let option = optname_arg(c, family, level, opt)?;
    let data: Vec<u8> = if val.is_fixnum() { (val.as_fix() as i32).to_ne_bytes().to_vec() } else if val == FALSE { 0i32.to_ne_bytes().to_vec() } else if val == TRUE { 1i32.to_ne_bytes().to_vec() }
        else { let v = expect_str(c, val)?; c.str_bytes(v).to_vec() };
    let fd = sfd(c, s)?;
    if sys::setsockopt(fd, level, option, &data) < 0 { let p = io_path(c, s); return Err(fail_path(c, sys::errno(), "setsockopt(2)", p.as_deref())); }
    Ok(Value::fix(0))
}
fn bs_getsockopt(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let fd = sfd(c, s)?;
    let family = getfamily(fd);
    let level = level_arg(c, family, a[0])?;
    let option = optname_arg(c, family, level, a[1])?;
    let mut b = vec![0u8; 256];
    let fd = sfd(c, s)?;
    match sys::getsockopt(fd, level, option, &mut b) {
        Ok(n) => { b.truncate(n.min(256)); let d = bin(c, &b); option_new(c, family, level, option, d) }
        Err(e) => { let p = io_path(c, s); Err(fail_path(c, e, "getsockopt(2)", p.as_deref())) }
    }
}
fn name_of(c: &mut Ctx, s: Value, peer: bool) -> Result<Vec<u8>, Esc> {
    let fd = sfd(c, s)?;
    let mut b = [0u8; SS_LEN];
    let r = if peer { sys::getpeername(fd, &mut b) } else { sys::getsockname(fd, &mut b) };
    match r { Ok(n) => Ok(b[..n.min(SS_LEN)].to_vec()), Err(e) => Err(sys_fail(c, e, if peer { "getpeername(2)" } else { "getsockname(2)" })) }
}
fn bs_getsockname(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let b = name_of(c, s, false)?; Ok(bin(c, &b)) }
fn bs_getpeername(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let b = name_of(c, s, true)?; Ok(bin(c, &b)) }
fn bs_local_address(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let b = name_of(c, s, false)?; let fd = sfd(c, s)?; fd_socket_addrinfo(c, fd, &b) }
fn bs_remote_address(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let b = name_of(c, s, true)?; let fd = sfd(c, s)?; fd_socket_addrinfo(c, fd, &b) }
fn bs_getpeereid(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let fd = sfd(c, s)?;
    let mut b = [0u8; 12];
    if let Err(e) = sys::getsockopt(fd, SOL_SOCKET, k("SO_PEERCRED"), &mut b) { return Err(sys_fail(c, e, "getsockopt(SO_PEERCRED)")); }
    Ok(c.ary_new_from(&[Value::fix(u32::from_ne_bytes([b[4], b[5], b[6], b[7]]) as i64), Value::fix(u32::from_ne_bytes([b[8], b[9], b[10], b[11]]) as i64)]))
}
/// sendto(2) waiting while the socket is not writable.
fn send_all(c: &mut Ctx, s: Value, msg: &[u8], flags: i32, to: &[u8]) -> Result<Result<usize, i64>, Esc> {
    loop {
        let fd = sfd(c, s)?;
        let n = sys::sendto(fd, msg, flags, to);
        if n >= 0 { return Ok(Ok(n as usize)); }
        let e = sys::errno();
        if e == sys::EAGAIN { wait(c, s, sys::POLLOUT)?; continue; }
        if e == sys::EINTR { continue; }
        return Ok(Err(e));
    }
}
fn bs_send(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 2, 3)?;
    let m = expect_str(c, a[0])?;
    let to = match a.get(2) { Some(&t) if t != NIL => sockaddr_arg(c, t)?.0, _ => vec![] };
    sfd(c, s)?;
    let flags = to_i32(c, a[1])?;
    let msg = c.str_bytes(m).to_vec();
    match send_all(c, s, &msg, flags, &to)? {
        Ok(n) => Ok(Value::fix(n as i64)),
        Err(e) => Err(sys_fail(c, e, if to.is_empty() { "send(2)" } else { "sendto(2)" })),
    }
}
#[derive(Clone, Copy, PartialEq)]
enum From { Recv, Ip, Unix, Socket }
fn strbuf_len(c: &mut Ctx, v: Value) -> Result<usize, Esc> {
    let n = to_i32(c, v)?;
    if n < 0 { return arg_err(c, "negative string size (or size too big)"); }
    Ok(n as usize)
}
fn check_outbuf(c: &mut Ctx, v: Option<&Value>) -> Result<Option<Value>, Esc> {
    match v { Some(&b) if b != NIL => { let b = expect_str(c, b)?; check_frozen(c, b)?; Ok(Some(b)) } _ => Ok(None) }
}
fn recv_result(c: &mut Ctx, sock: Value, from: From, str: Value, sa: &[u8], alen: usize) -> RV {
    match from {
        From::Recv => Ok(str),
        From::Ip => { let a = if alen != 0 && alen != SS_LEN { let nr = sock_norev(c, sock); ipaddr_ary(c, &sa[..alen], nr)? } else { NIL }; Ok(c.ary_new_from(&[str, a])) }
        From::Unix => { let a = unixaddr(c, &sa[..alen]); Ok(c.ary_new_from(&[str, a])) }
        From::Socket => { let fd = sfd(c, sock)?; let a = fd_socket_addrinfo(c, fd, &sa[..alen])?; Ok(c.ary_new_from(&[str, a])) }
    }
}
/// rsock_s_recvfrom.
fn s_recvfrom(c: &mut Ctx, sock: Value, a: &[Value], from: From) -> RV {
    check_argc(c, a, 1, 3)?;
    let flags = match a.get(1) { Some(&f) if f != NIL => to_i32(c, f)?, _ => 0 };
    let len = strbuf_len(c, a[0])?;
    let out = check_outbuf(c, a.get(2))?;
    let io = io::get_io(c, sock)?;
    if io.rpos < io.rbuf.len() { return err(c, c.vm().c_io_error.get(), "recv for buffered IO"); }
    let mut buf = vec![0u8; len];
    let mut sa = [0u8; SS_LEN];
    let (n, alen) = loop {
        let fd = sfd(c, sock)?;
        match sys::recvfrom(fd, &mut buf, flags, &mut sa) {
            Ok(r) => break r,
            Err(e) if e == sys::EAGAIN => wait(c, sock, sys::POLLIN)?,
            Err(e) if e == sys::EINTR => {}
            Err(e) => return Err(sys_fail(c, e, "recvfrom(2)")),
        }
    };
    let fd = sfd(c, sock)?;
    if n == 0 && !is_dgram(c, fd)? { return Ok(NIL); }
    buf.truncate(n);
    let str = io::into_outbuf(c, out, buf, ENC_BINARY)?;
    recv_result(c, sock, from, str, &sa, alen.min(SS_LEN))
}
/// rsock_s_recvfrom_nonblock.
fn s_recvfrom_nonblock(c: &mut Ctx, sock: Value, a: &[Value], from: From) -> RV {
    let flags = to_i32(c, a[1])? | k("MSG_DONTWAIT");
    let len = strbuf_len(c, a[0])?;
    let out = check_outbuf(c, Some(&a[2]))?;
    let io = io::get_io(c, sock)?;
    if io.rpos < io.rbuf.len() { return err(c, c.vm().c_io_error.get(), "recvfrom for buffered IO"); }
    let fd = io.fd;
    let mut buf = vec![0u8; len];
    let mut sa = [0u8; SS_LEN];
    let (n, alen) = match sys::recvfrom(fd, &mut buf, flags, &mut sa) {
        Ok(r) => r,
        Err(e) if e == sys::EAGAIN => { if a[3] == FALSE { return Ok(sym_value("wait_readable")); } return Err(rw_fail(c, true, e, "recvfrom(2) would block")); }
        Err(e) => return Err(sys_fail(c, e, "recvfrom(2)")),
    };
    if n == 0 && !is_dgram(c, fd)? { return Ok(NIL); }
    buf.truncate(n);
    let str = io::into_outbuf(c, out, buf, ENC_BINARY)?;
    if from == From::Ip && !(alen != 0 && alen != SS_LEN) { return Ok(c.ary_new_from(&[str, NIL])); }
    recv_result(c, sock, from, str, &sa, alen.min(SS_LEN))
}
fn bs_recv(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { s_recvfrom(c, s, a, From::Recv) }
fn bs_recv_nonblock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { s_recvfrom_nonblock(c, s, a, From::Recv) }
fn bs_read_nonblock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let len = num_to_long(c, a[0])?;
    if len < 0 { return arg_err(c, format!("negative length {} given", len)); }
    let out = check_outbuf(c, Some(&a[1]))?;
    let io = io::get_io(c, s)?;
    if len == 0 { return io::into_outbuf(c, out, vec![], ENC_BINARY); }
    let len = len as usize;
    let data = if io.rpos < io.rbuf.len() {
        let n = len.min(io.rbuf.len() - io.rpos);
        let d = io.rbuf[io.rpos..io.rpos + n].to_vec(); io.rpos += n; d
    } else {
        let fd = io.fd;
        let mut buf = vec![0u8; len];
        let mut sa = [0u8; 0];
        match sys::recvfrom(fd, &mut buf, k("MSG_DONTWAIT"), &mut sa) {
            Ok((n, _)) => { buf.truncate(n); buf }
            Err(e) if e == sys::EAGAIN => { if a[2] == FALSE { return Ok(sym_value("wait_readable")); } return Err(rw_fail(c, true, e, "read would block")); }
            Err(e) => { let p = io_path(c, s); return Err(match p { Some(p) => sys_fail(c, e, &String::from_utf8_lossy(&p)), None => crate::builtins::exception::raise_errno(c, e, "") }); }
        }
    };
    if data.is_empty() { if a[2] == FALSE { return Ok(NIL); } return err(c, c.vm().c_eof_error.get(), "end of file reached"); }
    io::into_outbuf(c, out, data, ENC_BINARY)
}
fn bs_write_nonblock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let m = if is_string(a[0]) { a[0] } else { to_s_value(c, a[0])? };
    let io = io::get_io(c, s)?;
    if !io.writable { return err(c, c.vm().c_io_error.get(), "not opened for writing"); }
    io::flush_one(io);
    let fd = io.fd;
    let msg = c.str_bytes(m).to_vec();
    let n = sys::sendto(fd, &msg, k("MSG_DONTWAIT"), &[]);
    if n < 0 {
        let e = sys::errno();
        if e == sys::EAGAIN { if a[1] == FALSE { return Ok(sym_value("wait_writable")); } return Err(rw_fail(c, false, e, "write would block")); }
        return Err(crate::builtins::exception::raise_errno(c, e, ""));
    }
    Ok(Value::fix(n as i64))
}

// sendmsg / recvmsg (ancdata.c), with control messages as Socket::AncillaryData
fn cmsg_align(n: usize) -> usize { (n + 7) & !7 }
const CMSG_HDR: usize = 16;
fn ancdata_new(c: &mut Ctx, family: i32, level: i32, typ: i32, data: Value) -> RV {
    let k = C_ANCDATA.get(c);
    let o = new_object(c, k);
    ivar_set_raw(c, o, intern(b"family"), Value::fix(family as i64));
    ivar_set_raw(c, o, intern(b"level"), Value::fix(level as i64));
    ivar_set_raw(c, o, intern(b"type"), Value::fix(typ as i64));
    ivar_set_raw(c, o, intern(b"data"), data);
    Ok(o)
}
fn sendmsg_common(c: &mut Ctx, s: Value, a: &[Value], nonblock: bool) -> RV {
    let m = expect_str(c, a[0])?;
    let mut flags = if a[1] == NIL { 0 } else { to_i32(c, a[1])? };
    let dest = if a[2] == NIL { vec![] } else { sockaddr_arg(c, a[2])?.0 };
    let fd = sfd(c, s)?;
    let family = getfamily(fd);
    let mut ctl: Vec<u8> = vec![];
    let controls = a[3];
    for i in 0..c.ary_len(controls) {
        let e = c.ary_get(controls, i);
        let (level, typ, data) = if let Some(arr) = check_array(c, e)? {
            if c.ary_len(arr) != 3 { return arg_err(c, "an element of controls should be 3-elements array"); }
            let (l, t, d) = (c.ary_get(arr, 0), c.ary_get(arr, 1), c.ary_get(arr, 2));
            let level = level_arg(c, family, l)?;
            let typ = cmsg_type_arg(c, family, level, t)?;
            (level, typ, d)
        } else {
            let l = funcall(c, e, intern(b"level"), &[], std::ptr::null())?;
            let t = funcall(c, e, intern(b"type"), &[], std::ptr::null())?;
            let d = funcall(c, e, intern(b"data"), &[], std::ptr::null())?;
            (to_i32(c, l)?, to_i32(c, t)?, d)
        };
        let d = expect_str(c, data)?;
        let db = c.str_bytes(d).to_vec();
        let start = ctl.len();
        ctl.resize(start + cmsg_align(CMSG_HDR + db.len()), 0);
        ctl[start..start + 8].copy_from_slice(&((CMSG_HDR + db.len()) as u64).to_ne_bytes());
        ctl[start + 8..start + 12].copy_from_slice(&level.to_ne_bytes());
        ctl[start + 12..start + 16].copy_from_slice(&typ.to_ne_bytes());
        ctl[start + 16..start + 16 + db.len()].copy_from_slice(&db);
    }
    if nonblock { flags |= k("MSG_DONTWAIT"); }
    let msg = c.str_bytes(m).to_vec();
    loop {
        let fd = sfd(c, s)?;
        let n = sys::sendmsg(fd, &msg, &dest, &ctl, flags);
        if n >= 0 { return Ok(Value::fix(n as i64)); }
        let e = sys::errno();
        if e == sys::EAGAIN {
            if nonblock { if a.get(4) == Some(&FALSE) { return Ok(sym_value("wait_writable")); } return Err(rw_fail(c, false, e, "sendmsg(2) would block")); }
            wait(c, s, sys::POLLOUT)?; continue;
        }
        if e == sys::EINTR { continue; }
        return Err(sys_fail(c, e, "sendmsg(2)"));
    }
}
fn bs_sendmsg(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { sendmsg_common(c, s, a, false) }
fn bs_sendmsg_nonblock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { sendmsg_common(c, s, a, true) }
/// Close the descriptors of SCM_RIGHTS messages in `ctl` (rsock_discard_cmsg_resource).
fn discard_rights(ctl: &[u8]) {
    let mut off = 0;
    while off + CMSG_HDR <= ctl.len() {
        let len = u64_at(ctl, off) as usize;
        if len < CMSG_HDR || off + len > ctl.len() { break; }
        if i32_at(ctl, off + 8) == SOL_SOCKET && i32_at(ctl, off + 12) == k("SCM_RIGHTS") {
            for ch in ctl[off + CMSG_HDR..off + len].chunks_exact(4) { sys::close(i32_at(ch, 0)); }
        }
        off += cmsg_align(len);
    }
}
/// rsock_bsock_recvmsg_internal: without a size, peek and grow the buffers until the message fits.
fn recvmsg_common(c: &mut Ctx, s: Value, a: &[Value], nonblock: bool) -> RV {
    let mut maxdat = if a[0] == NIL { 4096 } else { strbuf_len(c, a[0])? };
    let mut flags = to_i32(c, a[1])?;
    let mut maxctl = if a[2] == NIL { 4096 } else { strbuf_len(c, a[2])? };
    let scm_rights = a[3].truthy();
    let ex = a.get(4) != Some(&FALSE);
    let mut grow = a[0] == NIL || a[2] == NIL;
    let io = io::get_io(c, s)?;
    if io.rpos < io.rbuf.len() { return err(c, c.vm().c_io_error.get(), "recvmsg for buffered IO"); }
    if nonblock { flags |= k("MSG_DONTWAIT"); }
    let orig = flags;
    let (data, name, ctl, mflags) = loop {
        let mut data = vec![0u8; maxdat];
        let mut name = [0u8; SS_LEN];
        let mut ctl = vec![0u8; maxctl];
        if grow { flags |= k("MSG_PEEK"); }
        let fd = sfd(c, s)?;
        let (n, nl, cl, mf) = match sys::recvmsg(fd, &mut data, &mut name, &mut ctl, flags | MSG_CMSG_CLOEXEC) {
            Ok(r) => r,
            Err(e) if e == sys::EAGAIN && !nonblock => { wait(c, s, sys::POLLIN)?; continue; }
            Err(e) if e == sys::EINTR => continue,
            Err(e) if e == sys::EAGAIN => { if !ex { return Ok(sym_value("wait_readable")); } return Err(rw_fail(c, true, e, "recvmsg(2) would block")); }
            Err(e) => return Err(sys_fail(c, e, "recvmsg(2)")),
        };
        if n == 0 && !is_dgram(c, fd)? { return Ok(NIL); }
        ctl.truncate(cl.min(maxctl));
        if grow {
            let mut grown = false;
            if a[0] == NIL && n == maxdat { maxdat *= 2; grown = true; }
            if a[2] == NIL && mf & k("MSG_CTRUNC") != 0 { maxctl *= 2; grown = true; }
            discard_rights(&ctl);
            if !grown { grow = false; flags = orig; }
            continue;
        }
        data.truncate(n);
        break (data, name[..nl.min(SS_LEN)].to_vec(), ctl, mf);
    };
    let dv = bin(c, &data);
    let fd = sfd(c, s)?;
    let sender = fd_socket_addrinfo(c, fd, &name)?;
    let out = c.ary_new_from(&[dv, sender, Value::fix(mflags as i64)]);
    let family = getfamily(fd);
    let mut off = 0;
    while off + CMSG_HDR <= ctl.len() {
        let len = u64_at(&ctl, off) as usize;
        if len < CMSG_HDR || off + len > ctl.len() { break; }
        let level = i32_at(&ctl, off + 8);
        let typ = i32_at(&ctl, off + 12);
        let d = ctl[off + CMSG_HDR..off + len].to_vec();
        let dv = bin(c, &d);
        let anc = ancdata_new(c, family, level, typ, dv)?;
        if level == SOL_SOCKET && typ == k("SCM_RIGHTS") {
            let ios = c.ary_new();
            for ch in d.chunks_exact(4) {
                let nfd = i32_at(ch, 0);
                if scm_rights { let io = io_for_fd(c, nfd)?; c.ary_push(ios, io); } else { sys::close(nfd); }
            }
            if scm_rights { ivar_set_raw(c, anc, intern(b"unix_rights"), ios); }
        }
        c.ary_push(out, anc);
        off += cmsg_align(len);
    }
    Ok(out)
}
fn bs_recvmsg(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { recvmsg_common(c, s, a, false) }
fn bs_recvmsg_nonblock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { recvmsg_common(c, s, a, true) }
/// A received descriptor as an IO of the class the kind of file asks for (rsock_..._for_fd).
fn io_for_fd(c: &mut Ctx, fd: i32) -> RV {
    use std::os::unix::fs::FileTypeExt;
    let sock = sys::fd_metadata(fd).map(|m| m.file_type().is_socket()).unwrap_or(false);
    if sock { let k = C_SOCKET.get(c); return new_sock(c, k, fd); }
    let k = c.vm().c_io.get();
    Ok(io::new_io(c, k, fd, b"", true, true))
}

// IPSocket
fn ip_inspect(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    let base = io::io_inspect(c, s, a, b)?;
    let mut r = c.str_bytes(base).to_vec();
    let fd = io::rio(c, s).map(|i| if i.init && !i.closed { i.fd } else { -1 }).unwrap_or(-1);
    if fd >= 0 {
        let mut sa = [0u8; SS_LEN];
        if let Ok(n) = sys::getsockname(fd, &mut sa) {
            let sa = &sa[..n.min(SS_LEN)];
            if let Some(fam) = int_to_name(&G_AF, afamily(sa)) {
                let last = r.len() > 1 && r.last() == Some(&b'>');
                if last { r.pop(); }
                r.extend_from_slice(b", "); r.extend_from_slice(fam.as_bytes());
                if let Ok((h, p)) = sys::getnameinfo(sa, numeric_flags()) { r.extend_from_slice(b", "); r.extend_from_slice(&h); r.extend_from_slice(b", "); r.extend_from_slice(&p); }
                if last { r.push(b'>'); }
            }
        }
    }
    Ok(bin(c, &r))
}
/// rsock_revlookup_flag: Some(norevlookup) for true / false / :numeric / :hostname.
fn revlookup_flag(c: &mut Ctx, v: Value) -> Result<Option<bool>, Esc> {
    if v == TRUE { return Ok(Some(false)); }
    if v == FALSE { return Ok(Some(true)); }
    if v == NIL { return Ok(None); }
    if !v.is_sym() { return type_err(c, format!("wrong argument type {} (expected Symbol)", class_name_for_err(c, v))); }
    match sym_bytes(v.as_sym()) {
        b"numeric" => Ok(Some(true)),
        b"hostname" => Ok(Some(false)),
        n => { let n = String::from_utf8_lossy(n).into_owned(); arg_err(c, format!("invalid reverse_lookup flag: :{}", n)) }
    }
}
fn ip_addr_common(c: &mut Ctx, s: Value, a: &[Value], peer: bool) -> RV {
    check_argc(c, a, 0, 1)?;
    let nr = match a.first() { Some(&v) => revlookup_flag(c, v)?, None => None };
    io::get_io(c, s)?;
    let nr = nr.unwrap_or_else(|| sock_norev(c, s));
    let sa = name_of(c, s, peer)?;
    ipaddr_ary(c, &sa, nr)
}
fn ip_addr(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { ip_addr_common(c, s, a, false) }
fn ip_peeraddr(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { ip_addr_common(c, s, a, true) }
fn ip_recvfrom(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { s_recvfrom(c, s, a, From::Ip) }
fn ip_s_getaddress(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let r = addrinfo_list(c, a[0], NIL, AF_UNSPEC, SOCK_STREAM, 0)?;
    let sa = r[0].4.clone();
    make_ipaddr(c, &sa)
}

// TCPSocket / TCPServer (init_inetsock_internal)
fn timeout_deadline(c: &mut Ctx, t: Value) -> Result<Option<std::time::Instant>, Esc> { if t == NIL { Ok(None) } else { io::wait_deadline(c, Some(&t)) } }
fn init_inetsock(c: &mut Ctx, slf: Value, rhost: Value, rserv: Value, lhost: Value, lserv: Value, server: bool, kw: Option<Value>) -> RV {
    let resolv_t = kwget(c, kw, "resolv_timeout")?;
    let connect_t = kwget(c, kw, "connect_timeout")?;
    let open_t = kwget(c, kw, "open_timeout")?;
    if open_t != NIL && (resolv_t != NIL || connect_t != NIL) { return arg_err(c, "Cannot specify open_timeout along with connect_timeout or resolv_timeout"); }
    let start = std::time::Instant::now();
    let open_dl = timeout_deadline(c, open_t)?;
    // the fast-fallback path (a host name, not an address) reports resolution errors as getaddrinfo(3)
    let mut fl = 0;
    let named = !server && host_str(c, rhost, &mut fl)?.is_some_and(|h| { let h = String::from_utf8_lossy(&h).into_owned(); !is_ip_text(&h) });
    let ff = kwget(c, kw, "fast_fallback")?;
    let ff = if ff == NIL { let s = C_SOCKET.get(c); ivar_get(c, s, intern(b"tcp_fast_fallback")) != FALSE } else { ff.truthy() };
    let reason = if named && ff { "getaddrinfo(3)" } else { "getaddrinfo" };
    let res = getaddrinfo(c, rhost, rserv, (if server { k("AI_PASSIVE") } else { 0 }, AF_UNSPEC, SOCK_STREAM, 0), true, reason)?;
    let lres = if !server && (lhost != NIL || lserv != NIL) { Some(getaddrinfo(c, lhost, lserv, (0, AF_UNSPEC, SOCK_STREAM, 0), true, reason)?) } else { None };
    let _ = start;
    let (mut status, mut local, mut error, mut syscall) = (0i32, 0i32, 0i64, "");
    let mut ok = false;
    for (i, r) in res.iter().enumerate() {
        let mut lr: Option<&sys::AddrInfo> = None;
        if let Some(l) = &lres {
            lr = l.iter().find(|x| x.1 == r.1);
            if lr.is_none() {
                if i + 1 < res.len() || status < 0 { continue; }
                lr = l.first();
            }
        }
        let fd = rsock_socket(r.1, r.2, r.3);
        syscall = "socket(2)";
        if fd < 0 { error = sys::errno(); status = -1; continue; }
        init_sock(c, slf, fd)?;
        if server {
            sys::setsockopt(fd, SOL_SOCKET, k("SO_REUSEADDR"), &1i32.to_ne_bytes());
            status = sys::bind(fd, &r.4); syscall = "bind(2)";
            if status < 0 { error = sys::errno(); }
        } else {
            status = 0;
            if let Some(l) = lr {
                sys::setsockopt(fd, SOL_SOCKET, k("SO_REUSEADDR"), &1i32.to_ne_bytes());
                status = sys::bind(fd, &l.4); local = status; syscall = "bind(2)";
                if status < 0 { error = sys::errno(); }
            }
            if status >= 0 {
                let dl = if open_dl.is_some() { open_dl } else { timeout_deadline(c, connect_t)? };
                if let Some(d) = open_dl { if std::time::Instant::now() >= d {
                    let s = inspect_sockaddr(c, &r.4)?; io::close_io(c, slf)?;
                    return Err(timeout_error(c, &format!("user specified timeout for {}", String::from_utf8_lossy(&s))));
                } }
                syscall = "connect(2)";
                match do_connect(c, slf, &r.4, dl) {
                    Ok(Ok(())) => status = 0,
                    Ok(Err(e)) => { status = -1; error = e; }
                    Err(x) => { let _ = io::close_io(c, slf); return Err(x); }
                }
            }
        }
        if status < 0 { io::close_io(c, slf)?; continue; }
        ok = true;
        break;
    }
    if !ok {
        let (h, p) = if local < 0 { (lhost, lserv) } else { (rhost, rserv) };
        return Err(fail_host_port(c, error, syscall, h, p));
    }
    if server {
        let fd = sfd(c, slf)?;
        if sys::listen(fd, k("SOMAXCONN")) < 0 { let e = sys::errno(); io::close_io(c, slf)?; return Err(sys_fail(c, e, "listen(2)")); }
    }
    Ok(slf)
}
fn tcp_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (a, kw) = kw_split(c, a);
    check_argc(c, a, 2, 4)?;
    let g = |i: usize| a.get(i).copied().unwrap_or(NIL);
    init_inetsock(c, slf, a[0], a[1], g(2), g(3), false, kw)
}
fn tcp_s_gethostbyname(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    rb_warn(c, "TCPSocket.gethostbyname is deprecated; use Addrinfo.getaddrinfo instead.")?;
    make_hostent(c, a[0], false)
}
fn make_hostent(c: &mut Ctx, host: Value, raw: bool) -> RV {
    let res = getaddrinfo(c, host, NIL, (k("AI_CANONNAME"), AF_UNSPEC, SOCK_STREAM, 0), true, "getaddrinfo")?;
    let name = match &res[0].5 { Some(n) => n.clone(), None => { let mut f = 0; host_str(c, host, &mut f)?.unwrap_or_default() } };
    let nv = bin(c, &name);
    let aliases = c.ary_new();
    let out = c.ary_new_from(&[nv, aliases, Value::fix(res[0].1 as i64)]);
    for r in &res {
        let v = if raw {
            match afamily(&r.4) { AF_INET => bin(c, &r.4[4..8]), AF_INET6 => bin(c, &r.4[8..24]), f => return socket_error(c, format!("unknown socket family:{}", f)) }
        } else { make_ipaddr(c, &r.4)? };
        c.ary_push(out, v);
    }
    Ok(out)
}
fn tcp_svr_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let (h, p) = if a.len() == 2 { (a[0], a[1]) } else { (NIL, a[0]) };
    init_inetsock(c, slf, h, p, NIL, NIL, true, None)
}
fn tcp_accept(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (fd, _) = do_accept(c, s)?; let k = C_TCPSOCKET.get(c); new_sock(c, k, fd) }
fn tcp_accept_nonblock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    match accept_nonblock(c, s, a[0])? { Ok((fd, _)) => { let k = C_TCPSOCKET.get(c); new_sock(c, k, fd) } Err(sym) => Ok(sym) }
}
fn sysaccept(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (fd, _) = do_accept(c, s)?; Ok(Value::fix(fd as i64)) }
fn sock_listen(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let n = to_i32(c, a[0])?;
    let fd = sfd(c, s)?;
    if sys::listen(fd, n) < 0 { return Err(sys_fail(c, sys::errno(), "listen(2)")); }
    Ok(Value::fix(0))
}

// UDPSocket
fn udp_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let f = match a.first() { Some(&v) => family_arg(c, v)?, None => AF_INET };
    let fd = rsock_socket(f, SOCK_DGRAM, 0);
    if fd < 0 { return Err(sys_fail(c, sys::errno(), "socket(2) - udp")); }
    init_sock(c, slf, fd)
}
fn fd_family(fd: i32) -> i32 { getfamily(fd) }
fn udp_connect(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let fd = sfd(c, s)?;
    let res = addrinfo_list(c, a[0], a[1], fd_family(fd), SOCK_DGRAM, 0)?;
    let mut last = 0;
    for r in &res { match do_connect(c, s, &r.4, None)? { Ok(()) => return Ok(Value::fix(0)), Err(e) => last = e } }
    Err(fail_host_port(c, last, "connect(2)", a[0], a[1]))
}
fn udp_bind(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let fd = sfd(c, s)?;
    let res = addrinfo_list(c, a[0], a[1], fd_family(fd), SOCK_DGRAM, 0)?;
    let mut last = 0;
    for r in &res { let fd = sfd(c, s)?; if sys::bind(fd, &r.4) == 0 { return Ok(Value::fix(0)); } last = sys::errno(); }
    Err(fail_host_port(c, last, "bind(2)", a[0], a[1]))
}
fn udp_send(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    if a.len() == 2 || a.len() == 3 { return bs_send(c, s, a, b); }
    check_argc(c, a, 4, 4)?;
    let m = expect_str(c, a[0])?;
    let fd = sfd(c, s)?;
    let flags = to_i32(c, a[1])?;
    let res = addrinfo_list(c, a[2], a[3], fd_family(fd), SOCK_DGRAM, 0)?;
    let msg = c.str_bytes(m).to_vec();
    let mut last = 0;
    for r in &res { match send_all(c, s, &msg, flags, &r.4)? { Ok(n) => return Ok(Value::fix(n as i64)), Err(e) => last = e } }
    Err(fail_host_port(c, last, "sendto(2)", a[2], a[3]))
}
fn udp_recvfrom_nonblock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { s_recvfrom_nonblock(c, s, a, From::Ip) }

// UNIXSocket / UNIXServer
fn unix_path_value(c: &mut Ctx, v: Value) -> Result<Vec<u8>, Esc> {
    if is_string(v) { let b = c.str_bytes(v); if b.is_empty() || b[0] == 0 { return Ok(b.to_vec()); } }
    let p = io::path_arg(c, v)?;
    Ok(p.0)
}
fn init_unixsock(c: &mut Ctx, slf: Value, path: Value, server: bool) -> RV {
    let p = unix_path_value(c, path)?;
    let sa = make_sockaddr_un(c, &p, "init")?;
    let fd = rsock_socket(AF_UNIX, SOCK_STREAM, 0);
    if fd < 0 { return Err(fail_path(c, sys::errno(), "socket(2)", Some(&p))); }
    init_sock(c, slf, fd)?;
    let r = if server { if sys::bind(fd, &sa) == 0 { Ok(()) } else { Err(sys::errno()) } } else {
        match do_connect(c, slf, &sa, None) { Ok(r) => r, Err(x) => { let _ = io::close_io(c, slf); return Err(x); } }
    };
    if let Err(e) = r { io::close_io(c, slf)?; return Err(fail_path(c, e, "connect(2)", Some(&p))); }
    if server {
        if sys::listen(fd, k("SOMAXCONN")) < 0 { let e = sys::errno(); io::close_io(c, slf)?; return Err(fail_path(c, e, "listen(2)", Some(&p))); }
        let io = c.data_mut::<RIo>(slf).unwrap(); io.path = p; io.has_path = true;
    }
    Ok(slf)
}
fn unix_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { init_unixsock(c, s, a[0], false) }
fn unix_svr_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { init_unixsock(c, s, a[0], true) }
fn unix_path_m(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let io = io::get_io(c, s)?;
    if !io.has_path {
        let fd = io.fd;
        let mut sa = [0u8; SS_LEN];
        let n = match sys::getsockname(fd, &mut sa) { Ok(n) => n.min(SUN_LEN), Err(e) => return Err(sys_fail(c, e, "getsockname(2)")) };
        let p = unix_path(&sa[..n]).to_vec();
        let io = c.data_mut::<RIo>(s).unwrap(); io.path = p; io.has_path = true;
    }
    let p = io::rio(c, s).unwrap().path.clone();
    Ok(bin(c, &p))
}
fn unix_addr_common(c: &mut Ctx, s: Value, peer: bool) -> RV {
    let fd = sfd(c, s)?;
    let mut sa = [0u8; SUN_LEN];
    let r = if peer { sys::getpeername(fd, &mut sa) } else { sys::getsockname(fd, &mut sa) };
    match r {
        Ok(n) => Ok(unixaddr(c, &sa[..n.min(SUN_LEN)])),
        Err(e) => { let p = io_path(c, s); Err(fail_path(c, e, if peer { "getpeername(2)" } else { "getsockname(2)" }, p.as_deref())) }
    }
}
fn unix_addr(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { unix_addr_common(c, s, false) }
fn unix_peeraddr(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { unix_addr_common(c, s, true) }
fn unix_recvfrom(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { s_recvfrom(c, s, a, From::Unix) }
fn unix_send_io(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let v = a[0];
    let fd = if io::is_io(c, v) { io::get_io(c, v)?.fd } else if v.is_fixnum() { v.as_fix() as i32 } else { return type_err(c, "neither IO nor file descriptor"); };
    let mut ctl = vec![0u8; cmsg_align(CMSG_HDR + 4)];
    ctl[0..8].copy_from_slice(&((CMSG_HDR + 4) as u64).to_ne_bytes());
    ctl[8..12].copy_from_slice(&SOL_SOCKET.to_ne_bytes());
    ctl[12..16].copy_from_slice(&k("SCM_RIGHTS").to_ne_bytes());
    ctl[16..20].copy_from_slice(&fd.to_ne_bytes());
    ctl.truncate(CMSG_HDR + 4);
    loop {
        let sf = sfd(c, s)?;
        if sys::sendmsg(sf, &[0], &[], &ctl, 0) >= 0 { return Ok(NIL); }
        let e = sys::errno();
        if e == sys::EAGAIN { wait(c, s, sys::POLLOUT)?; continue; }
        if e == sys::EINTR { continue; }
        let p = io_path(c, s);
        return Err(fail_path(c, e, "sendmsg(2)", p.as_deref()));
    }
}
fn unix_recv_io(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 2)?;
    let klass = if a.is_empty() { c.vm().c_io.get() } else { a[0] };
    let mode = a.get(1).copied().unwrap_or(NIL);
    let space = cmsg_align(CMSG_HDR + 4);
    let (cl, ctl) = loop {
        let sf = sfd(c, s)?;
        let mut d = [0u8; 1];
        let mut ctl = vec![0u8; space];
        match sys::recvmsg(sf, &mut d, &mut [], &mut ctl, MSG_CMSG_CLOEXEC) {
            Ok((_, _, cl, _)) => break (cl, ctl),
            Err(e) if e == sys::EAGAIN => wait(c, s, sys::POLLIN)?,
            Err(e) if e == sys::EINTR => {}
            Err(e) => { let p = io_path(c, s); return Err(fail_path(c, e, "recvmsg(2)", p.as_deref())); }
        }
    };
    if cl < CMSG_HDR { return socket_error(c, format!("file descriptor was not passed (msg_controllen={} smaller than sizeof(struct cmsghdr)={})", cl, CMSG_HDR)); }
    let (len, level, typ) = (u64_at(&ctl, 0) as usize, i32_at(&ctl, 8), i32_at(&ctl, 12));
    if level != SOL_SOCKET { return socket_error(c, format!("file descriptor was not passed (cmsg_level={}, {} expected)", level, SOL_SOCKET)); }
    if typ != k("SCM_RIGHTS") { return socket_error(c, format!("file descriptor was not passed (cmsg_type={}, {} expected)", typ, k("SCM_RIGHTS"))); }
    if cl < CMSG_HDR + 4 { return socket_error(c, format!("file descriptor was not passed (msg_controllen={} smaller than CMSG_LEN(sizeof(int))={})", cl, CMSG_HDR + 4)); }
    if cl > space { return socket_error(c, format!("file descriptor was not passed (msg_controllen={} bigger than CMSG_SPACE(sizeof(int))={})", cl, space)); }
    if len != CMSG_HDR + 4 { return socket_error(c, format!("file descriptor was not passed (cmsg_len={}, {} expected)", len, CMSG_HDR + 4)); }
    let fd = i32_at(&ctl, 16);
    sys::fcntl(fd, sys::F_SETFD, sys::FD_CLOEXEC);
    if klass == NIL { return Ok(Value::fix(fd as i64)); }
    let args: Vec<Value> = if mode == NIL { vec![Value::fix(fd as i64)] } else { vec![Value::fix(fd as i64), mode] };
    funcall(c, klass, intern(b"for_fd"), &args, std::ptr::null())
}
fn unix_accept(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (fd, _) = do_accept(c, s)?; let k = C_UNIXSOCKET.get(c); new_sock(c, k, fd) }
fn unix_accept_nonblock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    match accept_nonblock(c, s, a[0])? { Ok((fd, _)) => { let k = C_UNIXSOCKET.get(c); new_sock(c, k, fd) } Err(sym) => Ok(sym) }
}
fn unix_s_socketpair(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    check_argc(c, a, 0, 2)?;
    let t = a.first().copied().unwrap_or(Value::fix(SOCK_STREAM as i64));
    let p = a.get(1).copied().unwrap_or(Value::fix(0));
    socketpair(c, slf, Value::fix(AF_UNIX as i64), t, p, b)
}

// Socket
fn sock_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 2, 3)?;
    let d = family_arg(c, a[0])?;
    let t = socktype_arg(c, a[1])?;
    let p = match a.get(2) { Some(&v) if v != NIL => to_i32(c, v)?, _ => 0 };
    let fd = rsock_socket(d, t, p);
    if fd < 0 { return Err(sys_fail(c, sys::errno(), "socket(2)")); }
    init_sock(c, slf, fd)
}
fn socketpair(c: &mut Ctx, klass: Value, d: Value, t: Value, p: Value, b: Blk) -> RV {
    let d = family_arg(c, d)?;
    let t = socktype_arg(c, t)?;
    let p = if p == NIL { 0 } else { to_i32(c, p)? };
    let mut sv = [0i32; 2];
    if sys::socketpair(d, t | k("SOCK_CLOEXEC") | k("SOCK_NONBLOCK"), p, &mut sv) < 0 { return Err(sys_fail(c, sys::errno(), "socketpair(2)")); }
    let s1 = new_sock(c, klass, sv[0])?;
    let s2 = new_sock(c, klass, sv[1])?;
    let r = c.ary_new_from(&[s1, s2]);
    if blk_given(b) {
        let res = yield_block(c, b, &[r]);
        for s in [s2, s1] {
            if res.is_err() { let (e, v) = (c.esc, c.esc_val); let _ = funcall(c, s, intern(b"close"), &[], std::ptr::null()); c.esc = e; c.esc_val = v; }
            else if let Err(x) = funcall(c, s, intern(b"close"), &[], std::ptr::null()) { let _ = x; let k = c.vm().c_io_error.get(); if !is_kind_of(c, c.esc_val, k) { return Err(Esc); } }
        }
        return res;
    }
    Ok(r)
}
fn sock_s_socketpair(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    check_argc(c, a, 2, 3)?;
    socketpair(c, slf, a[0], a[1], a.get(2).copied().unwrap_or(NIL), b)
}
fn sock_connect(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (sa, _) = sockaddr_arg(c, a[0])?;
    match do_connect(c, s, &sa, None)? { Ok(()) => Ok(Value::fix(0)), Err(e) => Err(fail_sockaddr(c, e, "connect(2)", &sa)) }
}
fn sock_connect_nonblock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (sa, _) = sockaddr_arg(c, a[0])?;
    let fd = sfd(c, s)?;
    if sys::connect(fd, &sa) == 0 { return Ok(Value::fix(0)); }
    let e = sys::errno();
    if e == EINPROGRESS { if a[1] == FALSE { return Ok(sym_value("wait_writable")); } return Err(rw_fail(c, false, e, "connect(2) would block")); }
    if e == 106 && a[1] == FALSE { return Ok(Value::fix(0)); }
    Err(fail_sockaddr(c, e, "connect(2)", &sa))
}
fn sock_bind(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (sa, _) = sockaddr_arg(c, a[0])?;
    let fd = sfd(c, s)?;
    if sys::bind(fd, &sa) < 0 { return Err(fail_sockaddr(c, sys::errno(), "bind(2)", &sa)); }
    Ok(Value::fix(0))
}
fn sock_accept(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (fd, sa) = do_accept(c, s)?;
    let k = C_SOCKET.get(c);
    let p = new_sock(c, k, fd)?;
    let ai = fd_socket_addrinfo(c, fd, &sa)?;
    Ok(c.ary_new_from(&[p, ai]))
}
fn sock_accept_nonblock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    match accept_nonblock(c, s, a[0])? {
        Ok((fd, sa)) => { let k = C_SOCKET.get(c); let p = new_sock(c, k, fd)?; let ai = fd_socket_addrinfo(c, fd, &sa)?; Ok(c.ary_new_from(&[p, ai])) }
        Err(sym) => Ok(sym),
    }
}
fn sock_sysaccept(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (fd, sa) = do_accept(c, s)?;
    let ai = fd_socket_addrinfo(c, fd, &sa)?;
    Ok(c.ary_new_from(&[Value::fix(fd as i64), ai]))
}
fn sock_recvfrom(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { s_recvfrom(c, s, a, From::Socket) }
fn sock_recvfrom_nonblock(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { s_recvfrom_nonblock(c, s, a, From::Socket) }
fn sock_s_gethostname(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    match sys::nodename() { Ok(b) => Ok(bin(c, &b)), Err(e) => Err(sys_fail(c, e, "gethostname(3)")) }
}
fn sock_s_gethostbyname(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    rb_warn(c, "Socket.gethostbyname is deprecated; use Addrinfo.getaddrinfo instead.")?;
    make_hostent(c, a[0], true)
}
fn sock_s_gethostbyaddr(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    rb_warn(c, "Socket.gethostbyaddr is deprecated; use Addrinfo#getnameinfo instead.")?;
    check_argc(c, a, 1, 2)?;
    let s = expect_str(c, a[0])?;
    let addr = c.str_bytes(s).to_vec();
    let fam = match a.get(1) { Some(&f) if f != NIL => family_arg(c, f)?, _ => if addr.len() == 16 { AF_INET6 } else { AF_INET } };
    let sa = if fam == AF_INET6 && addr.len() == 16 { let mut x = [0u8; 16]; x.copy_from_slice(&addr); sockaddr_in6(x, 0) }
        else if fam == AF_INET && addr.len() == 4 { sockaddr_in([addr[0], addr[1], addr[2], addr[3]], 0) }
        else { return socket_error(c, "host not found"); };
    // nsswitch "files dns": /etc/hosts gives the name and its aliases, else a reverse lookup
    let (name, aliases) = match hosts_entry(&addr) {
        Some(x) => x,
        None => match getnameinfo(c, &sa, k("NI_NAMEREQD"))? { Ok((h, _)) => (h, vec![]), Err(_) => return socket_error(c, "host not found") },
    };
    let h = bin(c, &name);
    let al = c.ary_new();
    for x in &aliases { let v = bin(c, x); c.ary_push(al, v); }
    let av = bin(c, &addr);
    Ok(c.ary_new_from(&[h, al, Value::fix(fam as i64), av]))
}
/// The first /etc/hosts line for an address: (name, aliases).
fn hosts_entry(addr: &[u8]) -> Option<(Vec<u8>, Vec<Vec<u8>>)> {
    let text = std::fs::read_to_string("/etc/hosts").ok()?;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("");
        let mut it = line.split_whitespace();
        let Some(a) = it.next() else { continue };
        let parsed: Option<Vec<u8>> = if addr.len() == 4 { parse_ipv4(a).map(|x| x.to_vec()) } else { parse_ipv6(a).map(|x| x.to_vec()) };
        if parsed.as_deref() != Some(addr) { continue; }
        let Some(name) = it.next() else { continue };
        return Some((name.as_bytes().to_vec(), it.map(|x| x.as_bytes().to_vec()).collect()));
    }
    None
}
fn sock_s_getservbyname(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let svc = cstr_arg(c, a[0])?;
    let proto = match a.get(1) { Some(&p) if p != NIL => cstr_arg(c, p)?, _ => b"tcp".to_vec() };
    if let Some((_, port)) = getservby(Some(&svc), 0, &proto) { return Ok(Value::fix(port as i64)); }
    let s = String::from_utf8_lossy(&svc).into_owned();
    let n = if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) { u64::from_str_radix(h, 16).ok() } else if s.len() > 1 && s.starts_with('0') { u64::from_str_radix(&s[1..], 8).ok() } else { s.parse::<u64>().ok() };
    match n { Some(n) => Ok(Value::fix(n as i64)), None => socket_error(c, format!("no such service {}/{}", s, String::from_utf8_lossy(&proto))) }
}
fn sock_s_getservbyport(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let n = num_to_long(c, a[0])?;
    if n != (n as u16) as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too {} to convert into `int16_t'", n, if n > 0 { "big" } else { "small" })); }
    let proto = match a.get(1) { Some(&p) if p != NIL => cstr_arg(c, p)?, _ => b"tcp".to_vec() };
    match getservby(None, n as u32, &proto) {
        Some((name, _)) => Ok(bin(c, &name)),
        None => socket_error(c, format!("no such service for port {}/{}", n, String::from_utf8_lossy(&proto))),
    }
}
fn sock_s_getaddrinfo(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 2, 7)?;
    let g = |i: usize| a.get(i).copied().unwrap_or(NIL);
    let f = if g(2) == NIL { AF_UNSPEC } else { family_arg(c, g(2))? };
    let st = if g(3) == NIL { 0 } else { socktype_arg(c, g(3))? };
    let pr = if g(4) == NIL { 0 } else { to_i32(c, g(4))? };
    let fl = if g(5) == NIL { 0 } else { to_i32(c, g(5))? };
    let nr = match if g(6) == NIL { None } else { revlookup_flag(c, g(6))? } { Some(x) => x, None => NOREVLOOKUP.load(Relaxed) };
    let res = getaddrinfo(c, a[0], a[1], (fl, f, st, pr), false, "getaddrinfo")?;
    let out = c.ary_new();
    for r in &res {
        let mut v = ipaddr_vec(c, &r.4, nr)?;
        if let Some(cn) = &r.5 { let cn = cn.clone(); v[2] = bin(c, &cn); }
        for x in [r.1, r.2, r.3] { v.push(Value::fix(x as i64)); }
        let ary = c.ary_new_from(&v);
        c.ary_push(out, ary);
    }
    Ok(out)
}
fn sock_s_getnameinfo(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let fl = match a.get(1) { Some(&f) if f != NIL => to_i32(c, f)?, _ => 0 };
    let sa = a[0];
    let addrs: Vec<Vec<u8>> = if is_addrinfo(c, sa) || is_string(sa) || (sa.is_heap() && !is_array(sa) && respond_to(c, sa, s::to_str, true)) {
        let (b, _) = sockaddr_arg(c, sa)?;
        if b.len() > SS_LEN { return type_err(c, "sockaddr length too big"); }
        let need = match afamily(&b) { AF_INET => SIN_LEN, AF_INET6 => SIN6_LEN, _ => 0 };
        if need != 0 && b.len() < need { return type_err(c, "sockaddr size differs - should not happen"); }
        vec![b]
    } else if let Some(ary) = check_array(c, sa)? {
        let n = c.ary_len(ary);
        let (af, port, mut hfl) = (c.ary_get(ary, 0), c.ary_get(ary, 1), 0);
        let mut host;
        if n == 3 { host = c.ary_get(ary, 2); }
        else if n >= 4 { host = c.ary_get(ary, 3); if host == NIL { host = c.ary_get(ary, 2); } else { hfl |= k("AI_NUMERICHOST"); } }
        else { return arg_err(c, format!("array size should be 3 or 4, {} given", n)); }
        let st = if fl & k("NI_DGRAM") != 0 { SOCK_DGRAM } else { SOCK_STREAM };
        let f = if af == NIL { AF_UNSPEC } else { family_arg(c, af)? };
        getaddrinfo(c, host, port, (hfl, f, st, 0), false, "getaddrinfo")?.into_iter().map(|r| r.4).collect()
    } else { return type_err(c, "expecting String or Array"); };
    let mut first: Option<(Vec<u8>, Vec<u8>)> = None;
    for sa in &addrs {
        match getnameinfo(c, sa, fl)? {
            Ok(r) => match &first { None => first = Some(r), Some(f) => if *f != r { return socket_error(c, "sockaddr resolved to multiple nodename"); } },
            Err((e, eno)) => return Err(resolution_error(c, "getnameinfo", e, eno)),
        }
    }
    let (h, p) = first.unwrap_or_default();
    let (h, p) = (bin(c, &h), bin(c, &p));
    Ok(c.ary_new_from(&[h, p]))
}
fn sock_s_pack_sockaddr_in(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let r = addrinfo_list(c, a[1], a[0], AF_UNSPEC, 0, 0)?;
    let sa = r[0].4.clone();
    Ok(bin(c, &sa))
}
fn sock_s_unpack_sockaddr_in(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (sa, _) = sockaddr_arg(c, a[0])?;
    if sa.len() < 2 { return arg_err(c, "too short sockaddr"); }
    let f = afamily(&sa);
    if f != AF_INET && f != AF_INET6 { return arg_err(c, "not an AF_INET/AF_INET6 sockaddr"); }
    let h = make_ipaddr(c, &sa)?;
    let port = if sa.len() >= 4 { be16_at(&sa, 2) } else { 0 };
    Ok(c.ary_new_from(&[Value::fix(port as i64), h]))
}
fn sock_s_pack_sockaddr_un(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let p = expect_str(c, a[0])?;
    let b = c.str_bytes(p).to_vec();
    let sa = make_sockaddr_un(c, &b, "pack")?;
    Ok(bin(c, &sa))
}
fn sock_s_unpack_sockaddr_un(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (sa, _) = sockaddr_arg(c, a[0])?;
    if sa.len() < 2 { return arg_err(c, "too short sockaddr"); }
    if afamily(&sa) != AF_UNIX { return arg_err(c, "not an AF_UNIX sockaddr"); }
    if sa.len() > SUN_LEN { return type_err(c, format!("too long sockaddr_un - {} longer than {}", sa.len(), SUN_LEN)); }
    Ok(bin(c, unix_path(&sa)))
}
/// sockaddr_len of socket.c for a sockaddr of getifaddrs.
fn salen(f: u16) -> usize { match f as i32 { AF_INET => SIN_LEN, AF_INET6 => SIN6_LEN, AF_UNIX => SUN_LEN, AF_PACKET => 20, _ => 2 } }
fn sockaddr_obj(c: &mut Ctx, sa: Option<&Vec<u8>>) -> RV {
    let Some(sa) = sa else { return Ok(NIL) };
    let mut sa = sa.clone();
    if afamily(&sa) == AF_PACKET && sa.len() >= 12 { sa.truncate(12 + sa[11] as usize); }
    let f = afamily(&sa);
    addrinfo_new(c, ai_simple(sa, f, 0, 0))
}
fn sock_s_ip_address_list(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let l = match sys::getifaddrs(salen) { Ok(l) => l, Err(e) => return Err(sys_fail(c, e, "getifaddrs")) };
    let out = c.ary_new();
    for i in &l { if let Some(sa) = &i.2 { if is_ip(afamily(sa)) { let v = sockaddr_obj(c, Some(sa))?; c.ary_push(out, v); } } }
    Ok(out)
}
fn sock_s_getifaddrs(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let l = match sys::getifaddrs(salen) { Ok(l) => l, Err(e) => return Err(sys_fail(c, e, "getifaddrs")) };
    let out = c.ary_new();
    let k = C_IFADDR.get(c);
    for i in &l {
        let o = new_object(c, k);
        let name = bin(c, &i.0);
        let idx = if_nametoindex(&i.0);
        ivar_set_raw(c, o, intern(b"name"), name);
        ivar_set_raw(c, o, intern(b"ifindex"), Value::fix(idx as i64));
        ivar_set_raw(c, o, intern(b"flags"), Value::fix(i.1 as i64));
        let (a, n, x) = (sockaddr_obj(c, i.2.as_ref())?, sockaddr_obj(c, i.3.as_ref())?, sockaddr_obj(c, i.4.as_ref())?);
        ivar_set_raw(c, o, intern(b"addr"), a); ivar_set_raw(c, o, intern(b"netmask"), n); ivar_set_raw(c, o, intern(b"ifu"), x);
        c.ary_push(out, o);
    }
    Ok(out)
}
fn ifa_get(c: &mut Ctx, s: Value, n: &str) -> RV { let _ = c; Ok(ivar_get(c, s, intern(n.as_bytes()))) }
fn ifa_name(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { ifa_get(c, s, "name") }
fn ifa_ifindex(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { ifa_get(c, s, "ifindex") }
fn ifa_flags(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { ifa_get(c, s, "flags") }
fn ifa_addr(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { ifa_get(c, s, "addr") }
fn ifa_netmask(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { ifa_get(c, s, "netmask") }
const IFF_BROADCAST: i64 = 2;
const IFF_POINTOPOINT: i64 = 0x10;
fn ifa_broadaddr(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let f = ivar_get(c, s, intern(b"flags")).as_fix(); if f & IFF_BROADCAST != 0 { ifa_get(c, s, "ifu") } else { Ok(NIL) } }
fn ifa_dstaddr(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let f = ivar_get(c, s, intern(b"flags")).as_fix(); if f & IFF_POINTOPOINT != 0 { ifa_get(c, s, "ifu") } else { Ok(NIL) } }
fn ifa_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let mut r = format!("#<{} ", class_path(c, real_class_of(c, s))).into_bytes();
    let name = ivar_get(c, s, intern(b"name")); r.extend_from_slice(c.str_bytes(name));
    let flags = ivar_get(c, s, intern(b"flags")).as_fix() as u32;
    const FL: &[(u32, &str)] = &[(0x1, "UP"), (0x2, "BROADCAST"), (0x4, "DEBUG"), (0x8, "LOOPBACK"), (0x10, "POINTOPOINT"), (0x40, "RUNNING"), (0x80, "NOARP"), (0x100, "PROMISC"), (0x20, "NOTRAILERS"), (0x200, "ALLMULTI"), (0x400, "MASTER"), (0x800, "SLAVE"), (0x1000, "MULTICAST"), (0x2000, "PORTSEL"), (0x4000, "AUTOMEDIA"), (0x8000, "DYNAMIC")];
    let mut f = flags; let mut sep = ' ';
    for (bit, n) in FL { if f & bit != 0 { f &= !bit; r.push(sep as u8); r.extend_from_slice(n.as_bytes()); sep = ','; } }
    if f != 0 { r.push(sep as u8); r.extend_from_slice(format!("{:#x}", f).as_bytes()); }
    for (key, label) in [("addr", " "), ("netmask", " netmask="), ("ifu", "")] {
        let v = ivar_get(c, s, intern(key.as_bytes()));
        if v == NIL { continue; }
        let label = if key == "ifu" { if flags as i64 & IFF_BROADCAST != 0 { " broadcast=" } else if flags as i64 & IFF_POINTOPOINT != 0 { " dstaddr=" } else { continue } } else { label };
        let ai = get_ai(c, v)?;
        r.extend_from_slice(label.as_bytes());
        r.extend(inspect_sockaddr(c, &ai.sa)?);
    }
    r.push(b'>');
    Ok(bin(c, &r))
}
fn sock_s_fast_fallback(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let _ = c; Ok(ivar_get(c, s, intern(b"tcp_fast_fallback"))) }
fn sock_s_set_fast_fallback(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { ivar_set_raw(c, s, intern(b"tcp_fast_fallback"), a[0]); Ok(a[0]) }

// ---- Socket::Option ------------------------------------------------------------------------
fn option_new(c: &mut Ctx, family: i32, level: i32, optname: i32, data: Value) -> RV {
    let k = C_OPTION.get(c);
    let o = new_object(c, k);
    let d = expect_str(c, data)?;
    for (n, v) in [("family", Value::fix(family as i64)), ("level", Value::fix(level as i64)), ("optname", Value::fix(optname as i64)), ("data", d)] { ivar_set_raw(c, o, intern(n.as_bytes()), v); }
    Ok(o)
}
fn opt_args(c: &mut Ctx, a: &[Value]) -> Result<(i32, i32, i32), Esc> {
    let f = family_arg(c, a[0])?;
    let l = level_arg(c, f, a[1])?;
    let o = optname_arg(c, f, l, a[2])?;
    Ok((f, l, o))
}
fn opt_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (f, l, o) = opt_args(c, a)?;
    let d = expect_str(c, a[3])?;
    for (n, v) in [("family", Value::fix(f as i64)), ("level", Value::fix(l as i64)), ("optname", Value::fix(o as i64)), ("data", d)] { ivar_set_raw(c, s, intern(n.as_bytes()), v); }
    Ok(s)
}
fn opt_attr(c: &Ctx, s: Value, n: &str) -> Value { ivar_get(c, s, intern(n.as_bytes())) }
fn opt_family(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(opt_attr(c, s, "family")) }
fn opt_level(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let v = opt_attr(c, s, "level"); Ok(Value::fix(to_i32(c, v)? as i64)) }
fn opt_optname(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let v = opt_attr(c, s, "optname"); Ok(Value::fix(to_i32(c, v)? as i64)) }
fn opt_data(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let v = opt_attr(c, s, "data"); expect_str(c, v) }
fn opt_bytes(c: &mut Ctx, s: Value) -> Result<Vec<u8>, Esc> { let v = opt_data(c, s, &[], Blk::NONE)?; Ok(c.str_bytes(v).to_vec()) }
fn check_size(c: &mut Ctx, len: usize, size: usize, name: &str) -> Result<(), Esc> {
    if len != size { return type_err(c, format!("size differ.  expected as {}={} but {}", name, size, len)); }
    Ok(())
}
fn opt_s_int(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (f, l, o) = opt_args(c, a)?;
    let iv = crate::builtins::numeric_methods::to_integer(c, a[3])?;
    let i = to_i32(c, iv)?;
    let d = bin(c, &i.to_ne_bytes());
    option_new(c, f, l, o, d)
}
fn opt_int(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let d = opt_bytes(c, s)?;
    check_size(c, d.len(), 4, "sizeof(int)")?;
    Ok(Value::fix(i32_at(&d, 0) as i64))
}
fn opt_s_byte(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (f, l, o) = opt_args(c, a)?;
    let iv = crate::builtins::numeric_methods::to_integer(c, a[3])?;
    let i = num_to_long(c, iv)?;
    let d = bin(c, &[i as u8]);
    option_new(c, f, l, o, d)
}
fn opt_byte(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let d = opt_bytes(c, s)?;
    check_size(c, d.len(), 1, "sizeof(char)")?;
    Ok(Value::fix(d[0] as i8 as i64))
}
fn opt_s_bool(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (f, l, o) = opt_args(c, a)?;
    let d = bin(c, &(a[3].truthy() as i32).to_ne_bytes());
    option_new(c, f, l, o, d)
}
fn opt_bool(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let d = opt_bytes(c, s)?;
    if d.len() == 1 { return Ok(Value::bool(d[0] != 0)); }
    check_size(c, d.len(), 4, "sizeof(int)")?;
    Ok(Value::bool(i32_at(&d, 0) != 0))
}
fn opt_s_linger(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let onoff = if is_integer(a[0]) || (a[0].is_heap() && respond_to(c, a[0], intern(b"to_int"), true)) {
        let iv = crate::builtins::numeric_methods::to_integer(c, a[0])?; to_i32(c, iv)?
    } else { a[0].truthy() as i32 };
    let secs = to_i32(c, a[1])?;
    let mut d = onoff.to_ne_bytes().to_vec(); d.extend_from_slice(&secs.to_ne_bytes());
    let d = bin(c, &d);
    option_new(c, AF_UNSPEC, SOL_SOCKET, k("SO_LINGER"), d)
}
fn opt_linger(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (l, o) = (opt_attr(c, s, "level"), opt_attr(c, s, "optname"));
    if l != Value::fix(SOL_SOCKET as i64) || o != Value::fix(k("SO_LINGER") as i64) { return type_err(c, "linger socket option expected"); }
    let d = opt_bytes(c, s)?;
    check_size(c, d.len(), 8, "sizeof(struct linger)")?;
    let on = match i32_at(&d, 0) { 0 => FALSE, 1 => TRUE, x => Value::fix(x as i64) };
    Ok(c.ary_new_from(&[on, Value::fix(i32_at(&d, 4) as i64)]))
}
fn opt_s_mc_loop(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let iv = crate::builtins::numeric_methods::to_integer(c, a[0])?;
    let i = to_i32(c, iv)?;
    let d = bin(c, &i.to_ne_bytes());
    option_new(c, AF_INET, 0, k("IP_MULTICAST_LOOP"), d)
}
fn opt_s_mc_ttl(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let iv = crate::builtins::numeric_methods::to_integer(c, a[0])?;
    let i = to_i32(c, iv)?;
    let d = bin(c, &i.to_ne_bytes());
    option_new(c, AF_INET, 0, k("IP_MULTICAST_TTL"), d)
}
fn opt_mc(c: &mut Ctx, s: Value, name: &str) -> RV {
    let (f, l, o) = (opt_attr(c, s, "family"), opt_attr(c, s, "level"), opt_attr(c, s, "optname"));
    if f == Value::fix(AF_INET as i64) && l == Value::fix(0) && o == Value::fix(k(name) as i64) { return opt_int(c, s, &[], Blk::NONE); }
    type_err(c, format!("{} socket option expected", if name == "IP_MULTICAST_LOOP" { "ipv4_multicast_loop" } else { "ipv4_multicast_ttl" }))
}
fn opt_mc_loop(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { opt_mc(c, s, "IP_MULTICAST_LOOP") }
fn opt_mc_ttl(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { opt_mc(c, s, "IP_MULTICAST_TTL") }
fn opt_unpack(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { let d = opt_data(c, s, &[], Blk::NONE)?; funcall(c, d, intern(b"unpack"), &[a[0]], std::ptr::null()) }
fn ntop4(b: &[u8]) -> String { ipv4_str(u32::from_be_bytes([b[0], b[1], b[2], b[3]])) }
fn ntop6(b: &[u8]) -> String { let mut a = [0u8; 16]; a.copy_from_slice(&b[..16]); sys::getnameinfo(&sockaddr_in6(a, 0), 1).map(|(h, _)| String::from_utf8_lossy(&h).into_owned()).unwrap_or_default() }
fn ifname(succ: &str, fail: &str, idx: u32) -> String { match if_indextoname(idx) { Some(n) => format!("{}{}", succ, String::from_utf8_lossy(&n)), None => format!("{}{}", fail, idx) } }
/// The value part of Socket::Option#inspect, when the option has a known layout.
fn inspect_optval(family: i32, level: i32, opt: i32, d: &[u8]) -> Option<String> {
    let int = |d: &[u8]| if d.len() == 4 { Some(format!(" {}", i32_at(d, 0))) } else { None };
    let mreqn = |d: &[u8]| if d.len() == 12 { Some(format!(" {} {}{}", ntop4(&d[0..4]), ntop4(&d[4..8]), ifname(" ", " ifindex:", i32_at(d, 8) as u32))) } else { None };
    let family = if level == SOL_SOCKET { AF_UNSPEC } else { family };
    if family == AF_UNSPEC && level == SOL_SOCKET {
        let n = int_to_name(&G_SO, opt).unwrap_or_default();
        return match n.as_str() {
            "DEBUG" | "ACCEPTCONN" | "BROADCAST" | "REUSEADDR" | "KEEPALIVE" | "OOBINLINE" | "SNDBUF" | "RCVBUF" | "DONTROUTE" | "RCVLOWAT" | "SNDLOWAT" => int(d),
            "ERROR" => if d.len() == 4 { let i = i32_at(d, 0); Some(format!(" {} ({})", sys::errmsg(i as i64), i)) } else { None },
            "TYPE" => if d.len() == 4 { let i = i32_at(d, 0); Some(match int_to_name(&G_SOCKTYPE_NAME, i) { Some(n) => format!(" {}", n), None => format!(" {}", i) }) } else { None },
            "LINGER" => if d.len() == 8 { let on = i32_at(d, 0); Some(format!("{} {}sec", match on { 0 => " off".to_string(), 1 => " on".to_string(), x => format!(" on({})", x) }, i32_at(d, 4))) } else { None },
            "RCVTIMEO" | "SNDTIMEO" => if d.len() == 16 { let s = u64_at(d, 0) as i64; let u = u64_at(d, 8) as i64; Some(format!(" {}.{:06}sec", s, u)) } else { None },
            "PEERCRED" => if d.len() == 12 { Some(format!(" pid={} euid={} egid={} (ucred)", i32_at(d, 0) as u32, i32_at(d, 4) as u32, i32_at(d, 8) as u32)) } else { None },
            _ => None,
        };
    }
    if is_ip(family) {
        if level == 0 {
            let n = int_to_name(&G_IP, opt).unwrap_or_default();
            return match n.as_str() {
                "MULTICAST_IF" => if d.len() == 4 { Some(format!(" {}", ntop4(d))) } else { mreqn(d) },
                "ADD_MEMBERSHIP" | "DROP_MEMBERSHIP" => if d.len() == 8 { Some(format!(" {} {}", ntop4(&d[0..4]), ntop4(&d[4..8]))) } else { mreqn(d) },
                "MULTICAST_LOOP" | "MULTICAST_TTL" => int(d),
                _ => None,
            };
        }
        if level == 41 {
            let n = int_to_name(&G_IPV6, opt).unwrap_or_default();
            return match n.as_str() {
                "MULTICAST_HOPS" | "UNICAST_HOPS" | "V6ONLY" => int(d),
                "MULTICAST_IF" => if d.len() == 4 { Some(ifname(" ", " ", i32_at(d, 0) as u32)) } else { None },
                "MULTICAST_LOOP" => if d.len() == 4 { Some(format!(" {}", i32_at(d, 0) as u32)) } else { None },
                "JOIN_GROUP" | "LEAVE_GROUP" => if d.len() == 20 { Some(format!(" {}{}", ntop6(&d[0..16]), ifname(" ", " interface:", i32_at(d, 16) as u32))) } else { None },
                _ => None,
            };
        }
        if level == 6 && Some(opt) == name_to_int(&G_TCP, b"NODELAY") { return int(d); }
    }
    None
}
fn opt_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let fv = opt_attr(c, s, "family"); let family = to_i32(c, fv)?;
    let lv = opt_attr(c, s, "level"); let level = to_i32(c, lv)?;
    let ov = opt_attr(c, s, "optname"); let opt = to_i32(c, ov)?;
    let d = opt_bytes(c, s)?;
    let mut r = format!("#<{}:", class_path(c, real_class_of(c, s)));
    match int_to_name(&G_AF_NOPRE, family) { Some(n) => r.push_str(&format!(" {}", n)), None => r.push_str(&format!(" family:{}", family)) }
    if level == SOL_SOCKET {
        r.push_str(" SOCKET");
        match int_to_name(&G_SO, opt) { Some(n) => r.push_str(&format!(" {}", n)), None => r.push_str(&format!(" optname:{}", opt)) }
    } else if family == AF_UNIX {
        r.push_str(&format!(" level:{}", level));
        match int_to_name(&G_LOCAL, opt) { Some(n) => r.push_str(&format!(" {}", n)), None => r.push_str(&format!(" optname:{}", opt)) }
    } else if is_ip(family) {
        match int_to_name(&G_IPLEVEL, level) { Some(n) => r.push_str(&format!(" {}", n)), None => r.push_str(&format!(" level:{}", level)) }
        let g = match level { 0 => Some(&G_IP), 41 => Some(&G_IPV6), 6 => Some(&G_TCP), 17 => Some(&G_UDP), _ => None };
        match g.and_then(|g| int_to_name(g, opt)) { Some(n) => r.push_str(&format!(" {}", n)), None => r.push_str(&format!(" optname:{}", opt)) }
    } else {
        r.push_str(&format!(" level:{} optname:{}", level, opt));
    }
    let mut out = r.into_bytes();
    match inspect_optval(family, level, opt, &d) {
        Some(v) => out.extend_from_slice(v.as_bytes()),
        None => { out.push(b' '); let dv = opt_data(c, s, &[], Blk::NONE)?; let dumped = funcall(c, dv, intern(b"dump"), &[], std::ptr::null())?; out.extend_from_slice(c.str_bytes(dumped)); }
    }
    out.push(b'>');
    Ok(bin(c, &out))
}

// ---- Socket::AncillaryData (the parts sendmsg / recvmsg and send_io need) -----------------------
fn anc_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let f = family_arg(c, a[0])?;
    let l = level_arg(c, f, a[1])?;
    let t = cmsg_type_arg(c, f, l, a[2])?;
    let d = expect_str(c, a[3])?;
    for (n, v) in [("family", Value::fix(f as i64)), ("level", Value::fix(l as i64)), ("type", Value::fix(t as i64)), ("data", d)] { ivar_set_raw(c, s, intern(n.as_bytes()), v); }
    Ok(s)
}
fn anc_attr(c: &mut Ctx, s: Value, n: &str) -> RV { Ok(ivar_get(c, s, intern(n.as_bytes()))) }
fn anc_family(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { anc_attr(c, s, "family") }
fn anc_level(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { anc_attr(c, s, "level") }
fn anc_type(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { anc_attr(c, s, "type") }
fn anc_data(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let v = ivar_get(c, s, intern(b"data")); expect_str(c, v) }
fn anc_s_int(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let i = to_i32(c, a[3])?;
    let d = bin(c, &i.to_ne_bytes());
    let o = new_object(c, slf);
    anc_initialize(c, o, &[a[0], a[1], a[2], d], Blk::NONE)
}
fn anc_int(c: &mut Ctx, s: Value, _a: &[Value], b: Blk) -> RV {
    let d = anc_data(c, s, &[], b)?;
    let d = c.str_bytes(d).to_vec();
    if d.len() != 4 { return type_err(c, format!("size differ.  expected as sizeof(int)={} but {}", 4, d.len())); }
    Ok(Value::fix(i32_at(&d, 0) as i64))
}
fn anc_cmsg_is(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let fv = ivar_get(c, s, intern(b"family")); let f = to_i32(c, fv)?;
    let lv = ivar_get(c, s, intern(b"level")); let l0 = to_i32(c, lv)?;
    let tv = ivar_get(c, s, intern(b"type")); let t0 = to_i32(c, tv)?;
    let l = level_arg(c, f, a[0])?;
    let t = cmsg_type_arg(c, f, l, a[1])?;
    Ok(Value::bool(l == l0 && t == t0))
}
fn anc_s_unix_rights(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut d = vec![];
    let ios = c.ary_new();
    for &v in a {
        let io = funcall(c, v, intern(b"to_io"), &[], std::ptr::null()).ok().filter(|&x| io::is_io(c, x));
        let Some(io) = io else { return type_err(c, "IO expected"); };
        let fd = io::get_io(c, io)?.fd;
        d.extend_from_slice(&fd.to_ne_bytes());
        c.ary_push(ios, io);
    }
    let dv = bin(c, &d);
    let o = new_object(c, slf);
    for (n, v) in [("family", Value::fix(AF_UNIX as i64)), ("level", Value::fix(SOL_SOCKET as i64)), ("type", Value::fix(k("SCM_RIGHTS") as i64)), ("data", dv), ("unix_rights", ios)] { ivar_set_raw(c, o, intern(n.as_bytes()), v); }
    Ok(o)
}
fn anc_unix_rights(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (l, t) = (ivar_get(c, s, intern(b"level")), ivar_get(c, s, intern(b"type")));
    if l != Value::fix(SOL_SOCKET as i64) || t != Value::fix(k("SCM_RIGHTS") as i64) { return type_err(c, "SCM_RIGHTS ancillary data expected"); }
    Ok(ivar_get(c, s, intern(b"unix_rights")))
}
fn to_u32(c: &mut Ctx, v: Value) -> Result<u32, Esc> {
    let n = num_to_long(c, v)?;
    if n < i32::MIN as i64 || n > u32::MAX as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too big to convert to 'unsigned int'", n)); }
    Ok(n as u32)
}
fn anc_s_ip_pktinfo(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 2, 3)?;
    let (addr, _) = sockaddr_arg(c, a[0])?;
    let idx = to_u32(c, a[1])?;
    let dst = match a.get(2) { Some(&d) if d != NIL => sockaddr_arg(c, d)?.0, _ => addr.clone() };
    if addr.len() != SIN_LEN { return arg_err(c, "addr size different to AF_INET sockaddr"); }
    if afamily(&addr) != AF_INET { return arg_err(c, "addr is not AF_INET sockaddr"); }
    if dst.len() != SIN_LEN { return arg_err(c, "spec_dat size different to AF_INET sockaddr"); }
    if afamily(&dst) != AF_INET { return arg_err(c, "spec_dst is not AF_INET sockaddr"); }
    let mut d = idx.to_ne_bytes().to_vec(); d.extend_from_slice(&dst[4..8]); d.extend_from_slice(&addr[4..8]);
    let dv = bin(c, &d);
    let o = new_object(c, slf);
    anc_initialize(c, o, &[Value::fix(AF_INET as i64), Value::fix(0), Value::fix(k("IP_PKTINFO") as i64), dv], Blk::NONE)
}
fn anc_fields(c: &mut Ctx, s: Value) -> Result<(i32, i32, Vec<u8>), Esc> {
    let lv = ivar_get(c, s, intern(b"level")); let l = to_i32(c, lv)?;
    let tv = ivar_get(c, s, intern(b"type")); let t = to_i32(c, tv)?;
    let d = anc_data(c, s, &[], Blk::NONE)?;
    Ok((l, t, c.str_bytes(d).to_vec()))
}
fn anc_ip_pktinfo(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (l, t, d) = anc_fields(c, s)?;
    if l != 0 || t != k("IP_PKTINFO") || d.len() != 12 { return type_err(c, "IP_PKTINFO ancillary data expected"); }
    let a = addrinfo_new(c, ai_simple(sockaddr_in([d[8], d[9], d[10], d[11]], 0), AF_INET, 0, 0))?;
    let sd = addrinfo_new(c, ai_simple(sockaddr_in([d[4], d[5], d[6], d[7]], 0), AF_INET, 0, 0))?;
    Ok(c.ary_new_from(&[a, Value::fix(u32::from_ne_bytes([d[0], d[1], d[2], d[3]]) as i64), sd]))
}
fn anc_s_ipv6_pktinfo(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (addr, _) = sockaddr_arg(c, a[0])?;
    let idx = to_u32(c, a[1])?;
    if addr.len() != SIN6_LEN { return arg_err(c, "addr size different to AF_INET6 sockaddr"); }
    if afamily(&addr) != AF_INET6 { return arg_err(c, "addr is not AF_INET6 sockaddr"); }
    let mut d = addr[8..24].to_vec(); d.extend_from_slice(&idx.to_ne_bytes());
    let dv = bin(c, &d);
    let o = new_object(c, slf);
    anc_initialize(c, o, &[Value::fix(AF_INET6 as i64), Value::fix(41), Value::fix(k("IPV6_PKTINFO") as i64), dv], Blk::NONE)
}
fn ipv6_pktinfo(c: &mut Ctx, s: Value) -> Result<(Value, u32), Esc> {
    let (l, t, d) = anc_fields(c, s)?;
    if l != 41 || t != k("IPV6_PKTINFO") || d.len() != 20 { return type_err(c, "IPV6_PKTINFO ancillary data expected"); }
    let mut a = [0u8; 16]; a.copy_from_slice(&d[..16]);
    let idx = u32::from_ne_bytes([d[16], d[17], d[18], d[19]]);
    let mut sa = sockaddr_in6(a, 0);
    if a[0] == 0xfe && a[1] & 0xc0 == 0x80 { sa[24..28].copy_from_slice(&idx.to_ne_bytes()); }
    Ok((addrinfo_new(c, ai_simple(sa, AF_INET6, 0, 0))?, idx))
}
fn anc_timestamp(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (l, t, d) = anc_fields(c, s)?;
    if l == SOL_SOCKET && d.len() == 16 && (t == k("SCM_TIMESTAMP") || t == k("SCM_TIMESTAMPNS")) {
        let tk = const_get_at(c, c.vm().c_object.get(), intern(b"Time")).unwrap_or(NIL);
        let (sec, frac) = (Value::fix(u64_at(&d, 0) as i64), Value::fix(u64_at(&d, 8) as i64));
        let unit = sym_value(if t == k("SCM_TIMESTAMP") { "usec" } else { "nsec" });
        return funcall(c, tk, intern(b"at"), &[sec, frac, unit], std::ptr::null());
    }
    type_err(c, "timestamp ancillary data expected")
}
fn anc_ipv6_pktinfo(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (a, i) = ipv6_pktinfo(c, s)?; Ok(c.ary_new_from(&[a, Value::fix(i as i64)])) }
fn anc_ipv6_pktinfo_addr(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(ipv6_pktinfo(c, s)?.0) }
fn anc_ipv6_pktinfo_ifindex(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(ipv6_pktinfo(c, s)?.1 as i64)) }
fn anc_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let fv = ivar_get(c, s, intern(b"family")); let f = to_i32(c, fv)?;
    let lv = ivar_get(c, s, intern(b"level")); let l = to_i32(c, lv)?;
    let tv = ivar_get(c, s, intern(b"type")); let t = to_i32(c, tv)?;
    let d = anc_data(c, s, &[], Blk::NONE)?;
    let db = c.str_bytes(d).to_vec();
    let mut r = format!("#<{}:", class_path(c, real_class_of(c, s)));
    match int_to_name(&G_AF_NOPRE, f) { Some(n) => r.push_str(&format!(" {}", n)), None => r.push_str(&format!(" family:{}", f)) }
    if l == SOL_SOCKET {
        r.push_str(" SOCKET");
        match int_to_name(&G_SCM, t) { Some(n) => r.push_str(&format!(" {}", n)), None => r.push_str(&format!(" cmsg_type:{}", t)) }
    } else if is_ip(f) {
        match int_to_name(&G_IPLEVEL, l) { Some(n) => r.push_str(&format!(" {}", n)), None => r.push_str(&format!(" cmsg_level:{}", l)) }
        let g = match l { 0 => Some(&G_IP), 41 => Some(&G_IPV6), 6 => Some(&G_TCP), 17 => Some(&G_UDP), _ => None };
        match g.and_then(|g| int_to_name(g, t)) { Some(n) => r.push_str(&format!(" {}", n)), None => r.push_str(&format!(" cmsg_type:{}", t)) }
    } else { r.push_str(&format!(" cmsg_level:{} cmsg_type:{}", l, t)); }
    let mut out = r.into_bytes();
    if l == SOL_SOCKET && t == k("SCM_RIGHTS") && db.len() % 4 == 0 {
        for ch in db.chunks_exact(4) { out.extend_from_slice(format!(" {}", i32_at(ch, 0)).as_bytes()); }
    } else {
        out.push(b' ');
        let dumped = funcall(c, d, intern(b"dump"), &[], std::ptr::null())?;
        out.extend_from_slice(c.str_bytes(dumped));
    }
    out.push(b'>');
    Ok(bin(c, &out))
}

fn resolution_error_code(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, s, intern(b"error_code"))) }

/// require "socket.so".
pub fn init(c: &mut Ctx) {
    if C_SOCKET.get(c) != NIL { return; }
    let (c_obj, io_k, std_err) = (c.vm().c_object.get(), c.vm().c_io.get(), c.vm().c_standard_error.get());
    // core classes of CRuby 4.0 that saruby's IO does not define yet
    io::timeout_error_class(c);
    crate::builtins::exception::register_rare_errnos(c);
    for (n, w) in [("EINPROGRESSWaitReadable", "WaitReadable"), ("EINPROGRESSWaitWritable", "WaitWritable")] {
        if const_get_at(c, io_k, intern(n.as_bytes())).is_none() {
            let base = crate::builtins::exception::errno_class(c, EINPROGRESS);
            let e = define_class_under(c, io_k, n, base);
            let m = const_get_at(c, io_k, intern(w.as_bytes())).unwrap();
            include_module_c(c, e, m);
        }
    }
    let bs = define_class(c, "BasicSocket", io_k);
    C_BASIC.set(c, bs);
    let _ = crate::builtins::object::undef_method(c, bs, intern(b"initialize"));
    let ds = |c: &mut Ctx, k: Value, n: &str, f: CFunc, a: i32| define_singleton_cfunc(c, k, n, f, a);
    ds(c, bs, "new", bs_s_new, -1);
    ds(c, bs, "do_not_reverse_lookup", bs_s_norev, 0); ds(c, bs, "do_not_reverse_lookup=", bs_s_set_norev, 1); ds(c, bs, "for_fd", bs_s_for_fd, 1);
    define_cfunc(c, bs, "close_read", bs_close_read, 0); define_cfunc(c, bs, "close_write", bs_close_write, 0); define_cfunc(c, bs, "shutdown", bs_shutdown, -1); define_cfunc(c, bs, "setsockopt", bs_setsockopt, -1); define_cfunc(c, bs, "getsockopt", bs_getsockopt, 2); define_cfunc(c, bs, "getsockname", bs_getsockname, 0); define_cfunc(c, bs, "getpeername", bs_getpeername, 0); define_cfunc(c, bs, "getpeereid", bs_getpeereid, 0); define_cfunc(c, bs, "local_address", bs_local_address, 0); define_cfunc(c, bs, "remote_address", bs_remote_address, 0); define_cfunc(c, bs, "send", bs_send, -1); define_cfunc(c, bs, "recv", bs_recv, -1); define_cfunc(c, bs, "do_not_reverse_lookup", bs_norev, 0); define_cfunc(c, bs, "do_not_reverse_lookup=", bs_set_norev, 1);
    define_private_cfunc(c, bs, "__recv_nonblock", bs_recv_nonblock, 4); define_private_cfunc(c, bs, "__read_nonblock", bs_read_nonblock, 3); define_private_cfunc(c, bs, "__write_nonblock", bs_write_nonblock, 2); define_private_cfunc(c, bs, "__sendmsg", bs_sendmsg, 4); define_private_cfunc(c, bs, "__sendmsg_nonblock", bs_sendmsg_nonblock, 5); define_private_cfunc(c, bs, "__recvmsg", bs_recvmsg, 4); define_private_cfunc(c, bs, "__recvmsg_nonblock", bs_recvmsg_nonblock, 5);
    let sock = define_class(c, "Socket", bs);
    C_SOCKET.set(c, sock);
    let se = define_class(c, "SocketError", std_err);
    E_SOCKET.set(c, se);
    let re = define_class_under(c, sock, "ResolutionError", se);
    E_RESOLUTION.set(c, re);
    define_cfunc(c, re, "error_code", resolution_error_code, 0);
    // IPSocket, TCPSocket, TCPServer, UDPSocket
    let ip = define_class(c, "IPSocket", bs);
    define_cfunc(c, ip, "inspect", ip_inspect, 0); define_cfunc(c, ip, "addr", ip_addr, -1); define_cfunc(c, ip, "peeraddr", ip_peeraddr, -1); define_cfunc(c, ip, "recvfrom", ip_recvfrom, -1);
    ds(c, ip, "getaddress", ip_s_getaddress, 1);
    let _ = crate::builtins::object::undef_method(c, ip, intern(b"getpeereid"));
    let tcp = define_class(c, "TCPSocket", ip);
    C_TCPSOCKET.set(c, tcp);
    ds(c, tcp, "gethostbyname", tcp_s_gethostbyname, 1);
    define_private_cfunc(c, tcp, "initialize", tcp_initialize, -1);
    let srv = define_class(c, "TCPServer", tcp);
    define_cfunc(c, srv, "accept", tcp_accept, 0); define_private_cfunc(c, srv, "__accept_nonblock", tcp_accept_nonblock, 1); define_cfunc(c, srv, "sysaccept", sysaccept, 0);
    define_private_cfunc(c, srv, "initialize", tcp_svr_initialize, -1); define_cfunc(c, srv, "listen", sock_listen, 1);
    let udp = define_class(c, "UDPSocket", ip);
    define_private_cfunc(c, udp, "initialize", udp_initialize, -1); define_cfunc(c, udp, "connect", udp_connect, 2); define_cfunc(c, udp, "bind", udp_bind, 2); define_cfunc(c, udp, "send", udp_send, -1);
    define_private_cfunc(c, udp, "__recvfrom_nonblock", udp_recvfrom_nonblock, 4);
    // UNIXSocket, UNIXServer
    let us = define_class(c, "UNIXSocket", bs);
    C_UNIXSOCKET.set(c, us);
    define_private_cfunc(c, us, "initialize", unix_initialize, 1); define_cfunc(c, us, "path", unix_path_m, 0); define_cfunc(c, us, "addr", unix_addr, 0); define_cfunc(c, us, "peeraddr", unix_peeraddr, 0); define_cfunc(c, us, "recvfrom", unix_recvfrom, -1); define_cfunc(c, us, "send_io", unix_send_io, 1); define_cfunc(c, us, "recv_io", unix_recv_io, -1);
    ds(c, us, "socketpair", unix_s_socketpair, -1); ds(c, us, "pair", unix_s_socketpair, -1);
    let usv = define_class(c, "UNIXServer", us);
    define_private_cfunc(c, usv, "initialize", unix_svr_initialize, 1); define_cfunc(c, usv, "accept", unix_accept, 0); define_private_cfunc(c, usv, "__accept_nonblock", unix_accept_nonblock, 1);
    define_cfunc(c, usv, "sysaccept", sysaccept, 0); define_cfunc(c, usv, "listen", sock_listen, 1);
    // Socket
    define_private_cfunc(c, sock, "initialize", sock_initialize, -1); define_cfunc(c, sock, "connect", sock_connect, 1); define_cfunc(c, sock, "bind", sock_bind, 1); define_cfunc(c, sock, "listen", sock_listen, 1); define_cfunc(c, sock, "accept", sock_accept, 0); define_cfunc(c, sock, "sysaccept", sock_sysaccept, 0); define_cfunc(c, sock, "recvfrom", sock_recvfrom, -1);
    define_private_cfunc(c, sock, "__connect_nonblock", sock_connect_nonblock, 2); define_private_cfunc(c, sock, "__accept_nonblock", sock_accept_nonblock, 1);
    define_private_cfunc(c, sock, "__recvfrom_nonblock", sock_recvfrom_nonblock, 4);
    ds(c, sock, "socketpair", sock_s_socketpair, -1); ds(c, sock, "pair", sock_s_socketpair, -1); ds(c, sock, "gethostname", sock_s_gethostname, 0); ds(c, sock, "gethostbyname", sock_s_gethostbyname, 1); ds(c, sock, "gethostbyaddr", sock_s_gethostbyaddr, -1); ds(c, sock, "getservbyname", sock_s_getservbyname, -1); ds(c, sock, "getservbyport", sock_s_getservbyport, -1); ds(c, sock, "getaddrinfo", sock_s_getaddrinfo, -1); ds(c, sock, "getnameinfo", sock_s_getnameinfo, -1); ds(c, sock, "sockaddr_in", sock_s_pack_sockaddr_in, 2); ds(c, sock, "pack_sockaddr_in", sock_s_pack_sockaddr_in, 2); ds(c, sock, "unpack_sockaddr_in", sock_s_unpack_sockaddr_in, 1); ds(c, sock, "sockaddr_un", sock_s_pack_sockaddr_un, 1); ds(c, sock, "pack_sockaddr_un", sock_s_pack_sockaddr_un, 1); ds(c, sock, "unpack_sockaddr_un", sock_s_unpack_sockaddr_un, 1); ds(c, sock, "ip_address_list", sock_s_ip_address_list, 0); ds(c, sock, "getifaddrs", sock_s_getifaddrs, 0); ds(c, sock, "tcp_fast_fallback", sock_s_fast_fallback, 0); ds(c, sock, "tcp_fast_fallback=", sock_s_set_fast_fallback, 1);
    let ff = !matches!(std::env::var("RUBY_TCP_NO_FAST_FALLBACK"), Ok(v) if v != "0");
    ivar_set_raw(c, sock, intern(b"tcp_fast_fallback"), Value::bool(ff));
    // Socket::Option, Socket::AncillaryData, Socket::Ifaddr
    let opt = define_class_under(c, sock, "Option", c_obj);
    C_OPTION.set(c, opt);
    define_private_cfunc(c, opt, "initialize", opt_initialize, 4); define_cfunc(c, opt, "family", opt_family, 0); define_cfunc(c, opt, "level", opt_level, 0); define_cfunc(c, opt, "optname", opt_optname, 0); define_cfunc(c, opt, "data", opt_data, 0); define_cfunc(c, opt, "inspect", opt_inspect, 0); define_cfunc(c, opt, "int", opt_int, 0); define_cfunc(c, opt, "byte", opt_byte, 0); define_cfunc(c, opt, "bool", opt_bool, 0); define_cfunc(c, opt, "linger", opt_linger, 0); define_cfunc(c, opt, "ipv4_multicast_ttl", opt_mc_ttl, 0); define_cfunc(c, opt, "ipv4_multicast_loop", opt_mc_loop, 0); define_cfunc(c, opt, "unpack", opt_unpack, 1); define_cfunc(c, opt, "to_s", opt_data, 0);
    ds(c, opt, "int", opt_s_int, 4); ds(c, opt, "byte", opt_s_byte, 4); ds(c, opt, "bool", opt_s_bool, 4); ds(c, opt, "linger", opt_s_linger, 2); ds(c, opt, "ipv4_multicast_ttl", opt_s_mc_ttl, 1); ds(c, opt, "ipv4_multicast_loop", opt_s_mc_loop, 1);
    let anc = define_class_under(c, sock, "AncillaryData", c_obj);
    C_ANCDATA.set(c, anc);
    define_private_cfunc(c, anc, "initialize", anc_initialize, 4); define_cfunc(c, anc, "family", anc_family, 0); define_cfunc(c, anc, "level", anc_level, 0); define_cfunc(c, anc, "type", anc_type, 0); define_cfunc(c, anc, "data", anc_data, 0); define_cfunc(c, anc, "int", anc_int, 0); define_cfunc(c, anc, "cmsg_is?", anc_cmsg_is, 2); define_cfunc(c, anc, "unix_rights", anc_unix_rights, 0); define_cfunc(c, anc, "inspect", anc_inspect, 0);
    ds(c, anc, "int", anc_s_int, 4); ds(c, anc, "unix_rights", anc_s_unix_rights, -1);
    ds(c, anc, "ip_pktinfo", anc_s_ip_pktinfo, -1); ds(c, anc, "ipv6_pktinfo", anc_s_ipv6_pktinfo, 2);
    define_cfunc(c, anc, "timestamp", anc_timestamp, 0); define_cfunc(c, anc, "ip_pktinfo", anc_ip_pktinfo, 0); define_cfunc(c, anc, "ipv6_pktinfo", anc_ipv6_pktinfo, 0);
    define_cfunc(c, anc, "ipv6_pktinfo_addr", anc_ipv6_pktinfo_addr, 0); define_cfunc(c, anc, "ipv6_pktinfo_ifindex", anc_ipv6_pktinfo_ifindex, 0);
    let ifa = define_class_under(c, sock, "Ifaddr", c_obj);
    C_IFADDR.set(c, ifa);
    define_cfunc(c, ifa, "name", ifa_name, 0); define_cfunc(c, ifa, "ifindex", ifa_ifindex, 0); define_cfunc(c, ifa, "flags", ifa_flags, 0); define_cfunc(c, ifa, "addr", ifa_addr, 0); define_cfunc(c, ifa, "netmask", ifa_netmask, 0); define_cfunc(c, ifa, "broadaddr", ifa_broadaddr, 0); define_cfunc(c, ifa, "dstaddr", ifa_dstaddr, 0); define_cfunc(c, ifa, "inspect", ifa_inspect, 0);
    // Addrinfo
    let ai = define_class(c, "Addrinfo", c_obj);
    class_set_alloc(c, ai, AllocKind::None);
    C_ADDRINFO.set(c, ai);
    define_private_cfunc(c, ai, "initialize", ai_initialize, -1); define_cfunc(c, ai, "inspect", ai_inspect, 0); define_cfunc(c, ai, "inspect_sockaddr", ai_inspect_sockaddr, 0); define_cfunc(c, ai, "afamily", ai_afamily, 0); define_cfunc(c, ai, "pfamily", ai_pfamily, 0); define_cfunc(c, ai, "socktype", ai_socktype, 0); define_cfunc(c, ai, "protocol", ai_protocol, 0); define_cfunc(c, ai, "canonname", ai_canonname, 0); define_cfunc(c, ai, "ipv4?", ai_ipv4_p, 0); define_cfunc(c, ai, "ipv6?", ai_ipv6_p, 0); define_cfunc(c, ai, "unix?", ai_unix_p, 0); define_cfunc(c, ai, "ip?", ai_ip_p, 0); define_cfunc(c, ai, "ip_unpack", ai_ip_unpack, 0); define_cfunc(c, ai, "ip_address", ai_ip_address, 0); define_cfunc(c, ai, "ip_port", ai_ip_port, 0); define_cfunc(c, ai, "ipv4_private?", ai_ipv4_private, 0); define_cfunc(c, ai, "ipv4_loopback?", ai_ipv4_loopback, 0); define_cfunc(c, ai, "ipv4_multicast?", ai_ipv4_multicast, 0); define_cfunc(c, ai, "ipv6_unspecified?", ai_ipv6_unspecified, 0); define_cfunc(c, ai, "ipv6_loopback?", ai_ipv6_loopback, 0); define_cfunc(c, ai, "ipv6_multicast?", ai_ipv6_multicast, 0); define_cfunc(c, ai, "ipv6_linklocal?", ai_ipv6_linklocal, 0); define_cfunc(c, ai, "ipv6_sitelocal?", ai_ipv6_sitelocal, 0); define_cfunc(c, ai, "ipv6_unique_local?", ai_ipv6_unique_local, 0); define_cfunc(c, ai, "ipv6_v4mapped?", ai_ipv6_v4mapped, 0); define_cfunc(c, ai, "ipv6_v4compat?", ai_ipv6_v4compat, 0); define_cfunc(c, ai, "ipv6_mc_nodelocal?", ai_ipv6_mc_nodelocal, 0); define_cfunc(c, ai, "ipv6_mc_linklocal?", ai_ipv6_mc_linklocal, 0); define_cfunc(c, ai, "ipv6_mc_sitelocal?", ai_ipv6_mc_sitelocal, 0); define_cfunc(c, ai, "ipv6_mc_orglocal?", ai_ipv6_mc_orglocal, 0); define_cfunc(c, ai, "ipv6_mc_global?", ai_ipv6_mc_global, 0); define_cfunc(c, ai, "ipv6_to_ipv4", ai_ipv6_to_ipv4, 0); define_cfunc(c, ai, "unix_path", ai_unix_path, 0); define_cfunc(c, ai, "to_sockaddr", ai_to_sockaddr, 0); define_cfunc(c, ai, "to_s", ai_to_sockaddr, 0); define_cfunc(c, ai, "getnameinfo", ai_getnameinfo, -1); define_cfunc(c, ai, "marshal_dump", ai_mdump, 0); define_cfunc(c, ai, "marshal_load", ai_mload, 1);
    ds(c, ai, "getaddrinfo", ai_s_getaddrinfo, -1); ds(c, ai, "ip", ai_s_ip, 1); ds(c, ai, "tcp", ai_s_tcp, 2); ds(c, ai, "udp", ai_s_udp, 2); ds(c, ai, "unix", ai_s_unix, -1);
    // constants, on Socket and Socket::Constants
    let cm = define_module_under(c, sock, "Constants");
    for (n, v) in all_consts() {
        let x = Value::fix(v);
        const_set(c, sock, intern(n.as_bytes()), x);
        const_set(c, cm, intern(n.as_bytes()), x);
    }
}
