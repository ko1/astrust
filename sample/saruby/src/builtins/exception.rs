// Exception hierarchy and methods.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::obj_set_klass;
use crate::array::*;
use crate::builtins::kernel::*;
use crate::string::*;
use crate::symbol::{intern, s, sym_str};
use crate::value::*;
use crate::vm::*;

pub fn init_classes(c: &mut Ctx) {
    let vmr = c.vm();
    let ob = vmr.c_object.get();
    let ex = define_class(c, "Exception", ob);
    class_set_alloc(c, ex, AllocKind::Exception);
    vmr.c_exception.set(ex);
    let d = |c: &mut Ctx, n: &str, sup: Value| define_class(c, n, sup);
    vmr.c_no_memory_error.set(d(c, "NoMemoryError", ex));
    vmr.c_script_error.set(d(c, "ScriptError", ex));
    vmr.c_load_error.set(d(c, "LoadError", vmr.c_script_error.get()));
    vmr.c_not_impl_error.set(d(c, "NotImplementedError", vmr.c_script_error.get()));
    vmr.c_syntax_error.set(d(c, "SyntaxError", vmr.c_script_error.get()));
    vmr.c_security_error.set(d(c, "SecurityError", ex));
    vmr.c_signal_exception.set(d(c, "SignalException", ex));
    vmr.c_interrupt.set(d(c, "Interrupt", vmr.c_signal_exception.get()));
    vmr.c_system_exit.set(d(c, "SystemExit", ex));
    vmr.c_system_stack_error.set(d(c, "SystemStackError", ex));
    let se = d(c, "StandardError", ex);
    vmr.c_standard_error.set(se);
    vmr.c_argument_error.set(d(c, "ArgumentError", se));
    vmr.c_uncaught_throw_error.set(d(c, "UncaughtThrowError", vmr.c_argument_error.get()));
    vmr.c_encoding_error.set(d(c, "EncodingError", se));
    vmr.c_fiber_error.set(d(c, "FiberError", se));
    vmr.c_io_error.set(d(c, "IOError", se));
    vmr.c_eof_error.set(d(c, "EOFError", vmr.c_io_error.get()));
    vmr.c_index_error.set(d(c, "IndexError", se));
    vmr.c_key_error.set(d(c, "KeyError", vmr.c_index_error.get()));
    vmr.c_stop_iteration.set(d(c, "StopIteration", vmr.c_index_error.get()));
    vmr.c_closed_queue_error.set(d(c, "ClosedQueueError", vmr.c_stop_iteration.get()));
    vmr.c_local_jump_error.set(d(c, "LocalJumpError", se));
    vmr.c_name_error.set(d(c, "NameError", se));
    vmr.c_no_method_error.set(d(c, "NoMethodError", vmr.c_name_error.get()));
    vmr.c_range_error.set(d(c, "RangeError", se));
    vmr.c_float_domain_error.set(d(c, "FloatDomainError", vmr.c_range_error.get()));
    vmr.c_regexp_error.set(d(c, "RegexpError", se));
    vmr.c_runtime_error.set(d(c, "RuntimeError", se));
    vmr.c_frozen_error.set(d(c, "FrozenError", vmr.c_runtime_error.get()));
    vmr.c_system_call_error.set(d(c, "SystemCallError", se));
    vmr.c_thread_error.set(d(c, "ThreadError", se));
    vmr.c_type_error.set(d(c, "TypeError", se));
    vmr.c_zero_div_error.set(d(c, "ZeroDivisionError", se));
    vmr.c_no_matching_pattern_error.set(d(c, "NoMatchingPatternError", se));
    vmr.c_no_matching_pattern_key_error.set(d(c, "NoMatchingPatternKeyError", vmr.c_no_matching_pattern_error.get()));
    let m = define_module(c, "Math");
    vmr.m_math.set(m);
    vmr.c_math_domain_error.set(define_class_under(c, m, "DomainError", vmr.c_argument_error.get()));
    let enc = define_class(c, "Encoding", ob);
    vmr.c_encoding.set(enc);
    vmr.c_compat_error.set(define_class_under(c, enc, "CompatibilityError", vmr.c_encoding_error.get()));
    vmr.c_invalid_byte_seq_error.set(define_class_under(c, enc, "InvalidByteSequenceError", vmr.c_encoding_error.get()));
    vmr.c_undef_conv_error.set(define_class_under(c, enc, "UndefinedConversionError", vmr.c_encoding_error.get()));
    {
        use crate::builtins::transcode as en;
        for k in [vmr.c_invalid_byte_seq_error.get(), vmr.c_undef_conv_error.get()] {
            define_cfunc(c, k, "source_encoding", en::exc_source_encoding, 0);
            define_cfunc(c, k, "destination_encoding", en::exc_destination_encoding, 0);
            define_cfunc(c, k, "source_encoding_name", en::exc_source_encoding_name, 0);
            define_cfunc(c, k, "destination_encoding_name", en::exc_destination_encoding_name, 0);
        }
        define_cfunc(c, vmr.c_invalid_byte_seq_error.get(), "error_bytes", en::exc_error_bytes, 0);
        define_cfunc(c, vmr.c_invalid_byte_seq_error.get(), "readagain_bytes", en::exc_readagain_bytes, 0);
        define_cfunc(c, vmr.c_invalid_byte_seq_error.get(), "incomplete_input?", en::exc_incomplete_input, 0);
        define_cfunc(c, vmr.c_undef_conv_error.get(), "error_char", en::exc_error_char, 0);
    }
    vmr.c_converter_not_found_error.set(define_class_under(c, enc, "ConverterNotFoundError", vmr.c_encoding_error.get()));
    vmr.m_errno.set(define_module(c, "Errno"));
    // Errno::* classes are built on first use (autoload of an internal feature) to keep boot short
    let names: Vec<&str> = ERRNOS[..BOOT_ERRNOS].iter().map(|e| e.0).chain(ERRNO_ALIASES[..2].iter().map(|a| a.0)).collect();
    crate::builtins::lazy_consts(c, vmr.m_errno.get(), &names, "saruby/errno");
}
/// Entries of ERRNOS (and the first two ERRNO_ALIASES) named at boot; interning the rest costs startup.
const BOOT_ERRNOS: usize = 66;
/// The less common Errno names, made reachable on require "socket" (saruby/errno defines them all).
pub fn register_rare_errnos(c: &mut Ctx) {
    let m = c.vm().m_errno.get();
    let names: Vec<&str> = ERRNOS[BOOT_ERRNOS..].iter().map(|e| e.0).chain(ERRNO_ALIASES[2..].iter().map(|a| a.0))
        .filter(|n| const_get_at(c, m, intern(n.as_bytes())).is_none()).collect();
    crate::builtins::lazy_consts(c, m, &names, "saruby/errno");
}
const ERRNO_ALIASES: &[(&str, &str)] = &[("EOPNOTSUPP", "ENOTSUP"), ("EDEADLOCK", "EDEADLK"),
    // not on Linux: CRuby makes them Errno::NOERROR
    ("EAUTH", "NOERROR"), ("EBADARCH", "NOERROR"), ("EBADEXEC", "NOERROR"), ("EBADMACHO", "NOERROR"), ("EBADRPC", "NOERROR"), ("ECAPMODE", "NOERROR"), ("EDEVERR", "NOERROR"), ("EDOOFUS", "NOERROR"), ("EFTYPE", "NOERROR"), ("EIPSEC", "NOERROR"), ("ELAST", "NOERROR"), ("ENEEDAUTH", "NOERROR"), ("ENOATTR", "NOERROR"), ("ENOPOLICY", "NOERROR"), ("ENOTCAPABLE", "NOERROR"), ("EPROCLIM", "NOERROR"), ("EPROCUNAVAIL", "NOERROR"), ("EPROGMISMATCH", "NOERROR"), ("EPROGUNAVAIL", "NOERROR"), ("EPWROFF", "NOERROR"), ("EQFULL", "NOERROR"), ("ERPCMISMATCH", "NOERROR"), ("ESHLIBVERS", "NOERROR")];
// Errno::X: its number is the constant `Errno`; its message comes from ERRNOS by that number.
fn define_errno(c: &mut Ctx, name: &str, no: i64) -> Value {
    let vmr = c.vm();
    if let Some(k) = const_get_at(c, vmr.m_errno.get(), intern(name.as_bytes())) { return k; }
    let k = define_class_under(c, vmr.m_errno.get(), name, vmr.c_system_call_error.get());
    const_set(c, k, intern(b"Errno"), Value::fix(no));
    k
}
/// strerror text of errno `no`.
pub fn errno_message(no: i64) -> Option<&'static str> { ERRNOS.iter().find(|e| e.1 == no).map(|e| e.2) }
/// Message of SystemCallError for errno `no` (nil: none given).
pub fn errno_text(no: Value) -> String {
    if !no.is_fixnum() { return "unknown error".into(); }
    match errno_message(no.as_fix()) { Some(m) => m.into(), None => format!("Unknown error {}", no.as_fix()) }
}
/// Every Errno class (the "saruby/errno" feature).
pub fn define_all_errnos(c: &mut Ctx) {
    let m = c.vm().m_errno.get();
    for (name, no, _) in ERRNOS { define_errno(c, name, *no); }
    for (alias, name) in ERRNO_ALIASES {
        let k = const_get_at(c, m, intern(name.as_bytes())).unwrap();
        if const_get_at(c, m, intern(alias.as_bytes())).is_none() { const_set(c, m, intern(alias.as_bytes()), k); }
    }
}

pub const ERRNOS: &[(&str, i64, &str)] = &[
    ("NOERROR", 0, "Success"), ("ENXIO", 6, "No such device or address"), ("E2BIG", 7, "Argument list too long"), ("ENOEXEC", 8, "Exec format error"),
    ("EFAULT", 14, "Bad address"), ("ENOTBLK", 15, "Block device required"), ("EBUSY", 16, "Device or resource busy"), ("ENODEV", 19, "No such device"),
    ("ENFILE", 23, "Too many open files in system"), ("ENOTTY", 25, "Inappropriate ioctl for device"), ("ETXTBSY", 26, "Text file busy"),
    ("EFBIG", 27, "File too large"), ("EROFS", 30, "Read-only file system"), ("EMLINK", 31, "Too many links"), ("EDEADLK", 35, "Resource deadlock avoided"),
    ("ENOLCK", 37, "No locks available"), ("ENOSYS", 38, "Function not implemented"), ("EPROTO", 71, "Protocol error"),
    ("EOVERFLOW", 75, "Value too large for defined data type"), ("EILSEQ", 84, "Invalid or incomplete multibyte or wide character"),
    ("ENOTSOCK", 88, "Socket operation on non-socket"), ("EDESTADDRREQ", 89, "Destination address required"), ("EMSGSIZE", 90, "Message too long"),
    ("EPROTONOSUPPORT", 93, "Protocol not supported"), ("EAFNOSUPPORT", 97, "Address family not supported by protocol"),
    ("EADDRINUSE", 98, "Address already in use"), ("EADDRNOTAVAIL", 99, "Cannot assign requested address"), ("ENETUNREACH", 101, "Network is unreachable"),
    ("ECONNABORTED", 103, "Software caused connection abort"), ("ECONNRESET", 104, "Connection reset by peer"), ("ENOBUFS", 105, "No buffer space available"),
    ("EISCONN", 106, "Transport endpoint is already connected"), ("ENOTCONN", 107, "Transport endpoint is not connected"),
    ("ETIMEDOUT", 110, "Connection timed out"), ("EHOSTUNREACH", 113, "No route to host"), ("EALREADY", 114, "Operation already in progress"),
    ("EINPROGRESS", 115, "Operation now in progress"), ("ESTALE", 116, "Stale file handle"), ("EDQUOT", 122, "Disk quota exceeded"),
    ("ECANCELED", 125, "Operation canceled"),
    ("EPERM", 1, "Operation not permitted"), ("ENOENT", 2, "No such file or directory"), ("ESRCH", 3, "No such process"),
    ("EINTR", 4, "Interrupted system call"), ("EIO", 5, "Input/output error"), ("EBADF", 9, "Bad file descriptor"),
    ("ECHILD", 10, "No child processes"), ("EAGAIN", 11, "Resource temporarily unavailable"), ("ENOMEM", 12, "Cannot allocate memory"),
    ("EACCES", 13, "Permission denied"), ("EEXIST", 17, "File exists"), ("ENOTDIR", 20, "Not a directory"),
    ("EISDIR", 21, "Is a directory"), ("EINVAL", 22, "Invalid argument"), ("EMFILE", 24, "Too many open files"),
    ("ENOSPC", 28, "No space left on device"), ("ESPIPE", 29, "Illegal seek"), ("EPIPE", 32, "Broken pipe"),
    ("EDOM", 33, "Numerical argument out of domain"), ("ERANGE", 34, "Numerical result out of range"),
    ("ENOTEMPTY", 39, "Directory not empty"), ("ECONNREFUSED", 111, "Connection refused"), ("ENOTSUP", 95, "Operation not supported"),
    ("ELOOP", 40, "Too many levels of symbolic links"), ("ENAMETOOLONG", 36, "File name too long"), ("EXDEV", 18, "Invalid cross-device link"),
    // the rest of Linux's, as CRuby defines them
    ("EADV", 68, "Advertise error"), ("EBADE", 52, "Invalid exchange"), ("EBADFD", 77, "File descriptor in bad state"), ("EBADMSG", 74, "Bad message"),
    ("EBADR", 53, "Invalid request descriptor"), ("EBADRQC", 56, "Invalid request code"), ("EBADSLT", 57, "Invalid slot"), ("EBFONT", 59, "Bad font file format"),
    ("ECHRNG", 44, "Channel number out of range"), ("ECOMM", 70, "Communication error on send"), ("EDOTDOT", 73, "RFS specific error"), ("EHOSTDOWN", 112, "Host is down"),
    ("EHWPOISON", 133, "Memory page has hardware error"), ("EIDRM", 43, "Identifier removed"), ("EISNAM", 120, "Is a named type file"), ("EKEYEXPIRED", 127, "Key has expired"),
    ("EKEYREJECTED", 129, "Key was rejected by service"), ("EKEYREVOKED", 128, "Key has been revoked"), ("EL2HLT", 51, "Level 2 halted"), ("EL2NSYNC", 45, "Level 2 not synchronized"),
    ("EL3HLT", 46, "Level 3 halted"), ("EL3RST", 47, "Level 3 reset"), ("ELIBACC", 79, "Can not access a needed shared library"), ("ELIBBAD", 80, "Accessing a corrupted shared library"),
    ("ELIBEXEC", 83, "Cannot exec a shared library directly"), ("ELIBMAX", 82, "Attempting to link in too many shared libraries"), ("ELIBSCN", 81, ".lib section in a.out corrupted"), ("ELNRNG", 48, "Link number out of range"),
    ("EMEDIUMTYPE", 124, "Wrong medium type"), ("EMULTIHOP", 72, "Multihop attempted"), ("ENAVAIL", 119, "No XENIX semaphores available"), ("ENETDOWN", 100, "Network is down"),
    ("ENETRESET", 102, "Network dropped connection on reset"), ("ENOANO", 55, "No anode"), ("ENOCSI", 50, "No CSI structure available"), ("ENODATA", 61, "No data available"),
    ("ENOKEY", 126, "Required key not available"), ("ENOLINK", 67, "Link has been severed"), ("ENOMEDIUM", 123, "No medium found"), ("ENOMSG", 42, "No message of desired type"),
    ("ENONET", 64, "Machine is not on the network"), ("ENOPKG", 65, "Package not installed"), ("ENOPROTOOPT", 92, "Protocol not available"), ("ENOSR", 63, "Out of streams resources"),
    ("ENOSTR", 60, "Device not a stream"), ("ENOTNAM", 118, "Not a XENIX named type file"), ("ENOTRECOVERABLE", 131, "State not recoverable"), ("ENOTUNIQ", 76, "Name not unique on network"),
    ("EOWNERDEAD", 130, "Owner died"), ("EPFNOSUPPORT", 96, "Protocol family not supported"), ("EPROTOTYPE", 91, "Protocol wrong type for socket"), ("EREMCHG", 78, "Remote address changed"),
    ("EREMOTE", 66, "Object is remote"), ("EREMOTEIO", 121, "Remote I/O error"), ("ERESTART", 85, "Interrupted system call should be restarted"), ("ERFKILL", 132, "Operation not possible due to RF-kill"),
    ("ESHUTDOWN", 108, "Cannot send after transport endpoint shutdown"), ("ESOCKTNOSUPPORT", 94, "Socket type not supported"), ("ESRMNT", 69, "Srmount error"), ("ESTRPIPE", 86, "Streams pipe error"),
    ("ETIME", 62, "Timer expired"), ("ETOOMANYREFS", 109, "Too many references: cannot splice"), ("EUCLEAN", 117, "Structure needs cleaning"), ("EUNATCH", 49, "Protocol driver not attached"),
    ("EUSERS", 87, "Too many users"), ("EXFULL", 54, "Exchange full"),
];

/// Errno class of `no`, made on first use (through vm.rs: no payload is touched here).
pub fn errno_class(c: &mut Ctx, no: i64) -> Value {
    let vmr = c.vm();
    // ENOTSUP and friends share numbers: the first table entry names the class
    for (name, n, _) in ERRNOS { if *n == no { return define_errno(c, name, *n); } }
    vmr.c_system_call_error.get()
}

pub fn raise_errno(c: &mut Ctx, no: i64, detail: &str) -> Esc {
    let k = errno_class(c, no);
    let base = errno_text(Value::fix(no));
    let msg = if detail.is_empty() { base } else { format!("{} @ {}", base, detail) };
    let e = make_exception(c, k, &msg);
    ivar_set_raw(c, e, s::errno, Value::fix(no));
    raise_exc(c, e)
}

/// Errno with CRuby's rb_sys_fail text: "<strerror> - <detail>".
pub fn raise_errno_plain(c: &mut Ctx, no: i64, detail: &str) -> Esc {
    let k = errno_class(c, no);
    let base = errno_text(Value::fix(no));
    let e = make_exception(c, k, &format!("{} - {}", base, detail));
    ivar_set_raw(c, e, s::errno, Value::fix(no));
    raise_exc(c, e)
}

pub fn io_err_to_errno(e: &std::io::Error) -> i64 { e.raw_os_error().map(|x| x as i64).unwrap_or(5) }

fn exc_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    ivar_set_raw(c, slf, s::mesg, if a.is_empty() { NIL } else { a[0] });
    Ok(NIL)
}
fn exc_s_exception(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { crate::builtins::object::c_new(c, slf, a, b) }
fn exc_exception(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.is_empty() || a[0] == slf { return Ok(slf); }
    let e = crate::builtins::object::obj_dup(c, slf, true, None)?;
    ivar_set_raw(c, e, s::mesg, a[0]);
    ivar_set_raw(c, e, s::bt, NIL);
    Ok(e)
}
pub fn exc_message_str(c: &mut Ctx, e: Value) -> Result<String, Esc> {
    let m = funcall(c, e, s::message, &[], std::ptr::null())?;
    if is_string(m) { Ok(String::from_utf8_lossy(c.str_bytes(m)).into_owned()) } else { to_rstring(c, m) }
}
fn exc_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let m = ivar_get(c, slf, s::mesg);
    if m == NIL { return Ok(c.str_new(class_path(c, real_class_of(c, slf)).as_bytes())); }
    if is_string(m) { return Ok(m); }
    to_s_value(c, m)
}
fn exc_message(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { funcall(c, slf, s::to_s, &[], std::ptr::null()) }
/// rb_check_string_type
fn check_str(c: &mut Ctx, v: Value) -> RV {
    if is_string(v) { return Ok(v); }
    if v.is_heap() && respond_to(c, v, s::to_str, true) { let r = funcall(c, v, s::to_str, &[], std::ptr::null())?; if is_string(r) { return Ok(r); } }
    Ok(NIL)
}
/// check_highlight_keyword: nil when not given.
fn highlight_kw(c: &mut Ctx, opt: Option<Value>) -> RV {
    let Some(h) = opt else { return Ok(NIL) };
    let v = crate::hash::hash_get_opt(c, h, sym_value("highlight"))?.unwrap_or(NIL);
    if v != TRUE && v != FALSE && v != NIL { let ins = inspect_str(c, v)?; return arg_err(c, format!("expected true or false as highlight: {}", ins)); }
    Ok(v)
}
fn stderr_tty() -> bool { crate::builtins::sys::isatty(2) == 1 }
const BOLD: &[u8] = b"\x1b[1m";
const UNDERLINE: &[u8] = b"\x1b[1;4m";
const RESET: &[u8] = b"\x1b[m";
fn underlined(o: &mut Vec<u8>, t: &[u8], highlight: bool) { if highlight { o.extend_from_slice(UNDERLINE); } o.extend_from_slice(t); if highlight { o.extend_from_slice(RESET); } }
/// rb_decorate_message (eval_error.c)
fn decorate_message(c: &mut Ctx, e: Value, emesg: Value, highlight: bool) -> Vec<u8> {
    let msg: Vec<u8> = if is_string(emesg) && crate::builtins::encoding::ascii_compat(c.str_enc(emesg)) { c.str_bytes(emesg).to_vec() } else { vec![] };
    let klass = real_class_of(c, e);
    let mut o = vec![];
    if klass == c.vm().c_runtime_error.get() && msg.is_empty() { underlined(&mut o, b"unhandled exception", highlight); return o; }
    let epath = class_path(c, klass);
    if msg.is_empty() { underlined(&mut o, epath.as_bytes(), highlight); return o; }
    if highlight { o.extend_from_slice(BOLD); }
    let nl = msg.iter().position(|&b| b == b'\n');
    o.extend_from_slice(&msg[..nl.unwrap_or(msg.len())]);
    if !epath.starts_with('#') {
        o.extend_from_slice(b" (");
        if highlight { o.extend_from_slice(UNDERLINE); }
        o.extend_from_slice(epath.as_bytes());
        if highlight { o.extend_from_slice(RESET); o.extend_from_slice(BOLD); }
        o.push(b')');
        if highlight { o.extend_from_slice(RESET); }
    }
    let Some(nl) = nl else { return o };
    let tail = &msg[nl + 1..];
    if tail.is_empty() { return o; }
    o.push(b'\n');
    if !highlight { o.extend_from_slice(tail); return o; }
    // each line bold, the newlines outside
    let mut t = tail;
    while !t.is_empty() {
        let n = t.iter().position(|&b| b == b'\n').unwrap_or(t.len());
        if n > 0 { o.extend_from_slice(BOLD); o.extend_from_slice(&t[..n]); o.extend_from_slice(RESET); }
        let m = t[n..].iter().take_while(|&&b| b == b'\n').count();
        o.extend_from_slice(&t[n..n + m]);
        t = &t[n + m..];
    }
    o
}
pub fn exc_detailed_message(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let highlight = highlight_kw(c, kw_arg(c, a))?.truthy();
    let m = funcall(c, slf, s::message, &[], std::ptr::null())?;
    let m = check_str(c, m)?;
    let o = decorate_message(c, slf, m, highlight);
    Ok(c.str_new(&o))
}
fn exc_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let cls = class_path(c, real_class_of(c, slf));
    // rb_obj_as_string(exc): #to_s, not #message
    let sv = to_s_value(c, slf)?;
    let m = String::from_utf8_lossy(c.str_bytes(sv)).into_owned();
    if m.is_empty() { return Ok(c.str_new(cls.as_bytes())); }
    if m.contains('\n') { return Ok(c.str_new(format!("#<{}:{}>", cls, inspect_bytes(m.as_bytes(), ENC_UTF8)).as_bytes())); }
    Ok(c.str_new(format!("#<{}: {}>", cls, m).as_bytes()))
}
fn exc_backtrace(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(exc_bt(c, slf)) }
pub fn exc_set_backtrace(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let v = a[0];
    // a captured backtrace still tells NameError#local_variables where it was raised
    let old = ivar_get(c, slf, s::bt);
    if c.data::<RawBt>(old).is_some() { keep_raw_bt(c, slf, old); }
    let bt = if is_string(v) { c.ary_new_from(&[v]) } else if v == NIL || is_array(v) { v } else { return type_err(c, "backtrace must be an Array of String or an Array of Thread::Backtrace::Location"); };
    let loc = c.vm().c_location.get();
    if is_array(bt) && !c.ary(bt).iter().all(|&x| is_string(x)) && !c.ary(bt).iter().all(|&x| x.is_heap() && is_kind_of(c, x, loc)) {
        return type_err(c, "backtrace must be an Array of String or an Array of Thread::Backtrace::Location");
    }
    if is_array(bt) && c.ary_len(bt) > 0 && c.ary(bt).iter().all(|&x| x.is_heap() && is_kind_of(c, x, loc)) {
        let strs = c.ary_new();
        let mut i = 0;
        while let Some(&x) = c.ary(bt).get(i) { let s = funcall(c, x, s::to_s, &[], std::ptr::null())?; c.ary_push(strs, s); i += 1; }
        ivar_set_raw(c, slf, s::bt, strs);
        ivar_set_raw(c, slf, intern(b"__bt_locs"), bt);
        return Ok(bt);
    }
    ivar_set_raw(c, slf, s::bt, bt);
    ivar_set_raw(c, slf, intern(b"__bt_locs"), NIL);
    ivar_set_raw(c, slf, intern(b"__bt_set"), TRUE);
    Ok(bt)
}
fn exc_backtrace_locations(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let locs = ivar_get(c, slf, intern(b"__bt_locs"));
    if locs != NIL { return Ok(locs); }
    let bt = exc_bt(c, slf);
    if bt == NIL || ivar_get(c, slf, intern(b"__bt_set")) == TRUE { return Ok(NIL); }
    let out = c.ary_new();
    let nids = ivar_get(c, slf, intern(b"__bt_nids"));
    let mut i = 0;
    while let Some(&l) = c.ary(bt).get(i) {
        if is_string(l) {
            let s = String::from_utf8_lossy(c.str_bytes(l)).into_owned();
            let loc = location_from_str(c, &s);
            if is_array(nids) { let n = c.ary_get(nids, i); ivar_set_raw(c, loc, intern(b"__nid"), n); }
            c.ary_push(out, loc);
        }
        else { c.ary_push(out, l); }
        i += 1;
    }
    ivar_set_raw(c, slf, intern(b"__bt_locs"), out);
    Ok(out)
}

// ---- Thread::Backtrace::Location ----------------------------------------------------------

/// "path:line:in 'label'" -> Location
pub fn location_from_str(c: &mut Ctx, s: &str) -> Value {
    let (head, label) = match s.rfind(":in '") { Some(p) => (&s[..p], s[p + 5..].trim_end_matches('\'')), None => (s, "") };
    let (path, line) = match head.rsplit_once(':') { Some((p, l)) if l.parse::<i64>().is_ok() => (p, l.parse::<i64>().unwrap()), _ => (head, 0) };
    location_new(c, path, line, label, NIL)
}
/// A Location of its parts; nid its prism node id (nil: unknown, false: code from eval).
fn location_new(c: &mut Ctx, path: &str, line: i64, label: &str, nid: Value) -> Value {
    static SYMS: std::sync::OnceLock<[crate::value::Sym; 4]> = std::sync::OnceLock::new();
    let [sp, sl, sb, sn] = *SYMS.get_or_init(|| [intern(b"path"), intern(b"lineno"), intern(b"label"), intern(b"__nid")]);
    let o = new_object(c, c.vm().c_location.get());
    let p = c.str_new(path.as_bytes());
    ivar_set_raw(c, o, sp, p);
    ivar_set_raw(c, o, sl, Value::fix(line));
    let l = c.str_new(label.as_bytes());
    ivar_set_raw(c, o, sb, l);
    if nid != NIL { ivar_set_raw(c, o, sn, nid); }
    o
}
pub fn locations_from_lines(c: &mut Ctx, lines: Vec<String>) -> Value {
    let out = c.ary_new();
    for l in lines { let v = location_from_str(c, &l); c.ary_push(out, v); }
    out
}
/// Locations with their prism node ids (error_highlight).
pub fn locations_from_locs(c: &mut Ctx, locs: Vec<BtLine>) -> Value {
    let out = c.ary_new_capa(locs.len());
    for l in locs { let v = location_new(c, l.file, l.line as i32 as i64, &l.label, nid_value(l.nid)); c.ary_push(out, v); }
    out
}
fn loc_path(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, slf, intern(b"path"))) }
/// Real paths of the files loaded from disk, taken when they were read (an iseq's realpath).
static REALPATHS: std::sync::Mutex<Vec<(String, Option<String>)>> = std::sync::Mutex::new(vec![]);
pub fn note_realpath(file: &str) {
    let rp = std::fs::canonicalize(file).ok().map(|p| p.to_string_lossy().into_owned());
    let mut t = REALPATHS.lock().unwrap_or_else(|e| e.into_inner());
    match t.iter_mut().find(|e| e.0 == file) { Some(e) => e.1 = rp, None => t.push((file.to_string(), rp)) }
}
/// A loaded file's real path as of its load (else as of now).
pub fn realpath(file: &str) -> Option<String> {
    let known = REALPATHS.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|e| e.0 == file).map(|e| e.1.clone());
    known.unwrap_or_else(|| std::fs::canonicalize(file).ok().map(|p| p.to_string_lossy().into_owned()))
}
fn loc_abs_path(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let p = ivar_get(c, slf, intern(b"path"));
    let ps = String::from_utf8_lossy(c.str_bytes(p)).into_owned();
    if ps.starts_with('<') || ps.starts_with('(') || ps == "-e" || ivar_get(c, slf, intern(b"__nid")) == FALSE { return Ok(NIL); }
    let known = REALPATHS.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|e| e.0 == ps).map(|e| e.1.clone());
    if let Some(rp) = known { return Ok(match rp { Some(r) => c.str_new(r.as_bytes()), None => p }); }
    match std::fs::canonicalize(&ps) { Ok(a) => Ok(c.str_new(a.to_string_lossy().as_bytes())), Err(_) => Ok(p) }
}
fn loc_lineno(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, slf, intern(b"lineno"))) }
fn loc_label(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, slf, intern(b"label"))) }
fn loc_base_label(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let l = ivar_get(c, slf, intern(b"label"));
    let mut s = String::from_utf8_lossy(c.str_bytes(l)).into_owned();
    if let Some(p) = s.find(" in ") { if s.starts_with("block") || s.starts_with("rescue") || s.starts_with("ensure") { s = s[p + 4..].to_string(); } }
    if !s.starts_with('<') { if let Some(p) = s.rfind(['#', '.']) { s = s[p + 1..].to_string(); } }
    Ok(c.str_new(s.as_bytes()))
}
fn loc_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let p = ivar_get(c, slf, intern(b"path")); let l = ivar_get(c, slf, intern(b"label"));
    let mut s = String::from_utf8_lossy(c.str_bytes(p)).into_owned();
    s.push_str(&format!(":{}:in '{}'", ivar_get(c, slf, intern(b"lineno")).as_fix(), String::from_utf8_lossy(c.str_bytes(l))));
    Ok(c.str_new(s.as_bytes()))
}
fn loc_inspect(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let s = loc_to_s(c, slf, a, b)?;
    funcall(c, s, intern(b"inspect"), &[], std::ptr::null())
}
fn k_caller_locations(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    let (start, len, drop) = caller_range(c, args)?;
    let Some(locs) = backtrace_locs_n(c, 1 + start, len) else { return Ok(NIL) };
    Ok(locations_from_locs(c, drop_last(locs, drop)))
}
/// Thread.each_caller_location(start = 1, length = nil) { |loc| }
fn th_each_caller_location(c: &mut Ctx, _s: Value, args: &[Value], b: Blk) -> RV {
    if let Some(h) = kw_arg(c, args) {
        let k = c.hash_pairs(h).first().map(|p| p.0).unwrap_or(NIL);
        let ks = inspect_str(c, k)?;
        return arg_err(c, format!("unknown keyword: {}", ks));
    }
    if b.is_null() { return err(c, c.vm().c_local_jump_error.get(), "no block given"); }
    let (start, len, drop) = caller_range(c, args)?;
    let Some(locs) = backtrace_locs_n(c, 1 + start, len) else { return Ok(NIL) };
    let a = locations_from_locs(c, drop_last(locs, drop));
    let mut i = 0;
    while let Some(&l) = c.ary(a).get(i) { i += 1; yield1(c, b, l)?; }
    Ok(NIL)
}
/// caller / caller_locations arguments: (start = 1, length = nil) or (range); the third part is how many
/// frames a range with a negative end leaves off the end.
pub fn caller_range(c: &mut Ctx, args: &[Value]) -> Result<(usize, Option<usize>, usize), Esc> {
    if args.is_empty() { return Ok((1, None, 0)); }
    let a = args[0];
    let long = crate::builtins::numeric_methods::num_to_long;
    if args.len() == 1 && is_kind_of(c, a, c.vm().c_range.get()) {
        let b = funcall(c, a, intern(b"begin"), &[], std::ptr::null())?;
        let e = funcall(c, a, intern(b"end"), &[], std::ptr::null())?;
        let excl = funcall(c, a, intern(b"exclude_end?"), &[], std::ptr::null())?.truthy();
        let b = if b == NIL { 0 } else { long(c, b)? };
        if b < 0 { return arg_err(c, format!("negative level ({})", b)); }
        let b = b as usize;
        if e == NIL { return Ok((b, None, 0)); }
        let e = long(c, e)?;
        if e < 0 { return Ok((b, None, (-e) as usize - if excl { 0 } else { 1 })); }
        let n = (e as usize + if excl { 0 } else { 1 }).saturating_sub(b);
        return Ok((b, Some(n), 0));
    }
    let lev = long(c, a)?;
    if lev < 0 { return arg_err(c, format!("negative level ({})", lev)); }
    let len = match args.get(1) {
        Some(&l) if l != NIL => { let n = long(c, l)?; if n < 0 { return arg_err(c, format!("negative size ({})", n)); } Some(n as usize) }
        _ => None,
    };
    Ok((lev as usize, len, 0))
}
/// Drop `drop` entries off the end (a range with a negative end).
pub fn drop_last<T>(mut v: Vec<T>, drop: usize) -> Vec<T> { let n = v.len().saturating_sub(drop); v.truncate(n); v }

/// Thread::Backtrace::Location; returns the class.
pub fn init_location(c: &mut Ctx) -> Value {
    let vmr = c.vm();
    let bt = define_class_under(c, vmr.c_thread.get(), "Backtrace", vmr.c_object.get());
    let k = define_class_under(c, bt, "Location", vmr.c_object.get());
    define_cfunc(c, k, "path", loc_path, 0);
    define_cfunc(c, k, "absolute_path", loc_abs_path, 0);
    define_cfunc(c, k, "lineno", loc_lineno, 0);
    define_cfunc(c, k, "label", loc_label, 0);
    define_cfunc(c, k, "base_label", loc_base_label, 0);
    define_cfunc(c, k, "to_s", loc_to_s, 0);
    define_cfunc(c, k, "inspect", loc_inspect, 0);
    define_module_function(c, vmr.m_kernel.get(), "caller_locations", k_caller_locations, -1);
    define_singleton_cfunc(c, vmr.c_thread.get(), "each_caller_location", th_each_caller_location, -1);
    define_singleton_cfunc(c, bt, "limit", |_c, _s, _a, _b| Ok(Value::fix(BT_LIMIT.load(std::sync::atomic::Ordering::Relaxed))), 0);
    k
}
fn exc_cause(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, slf, s::cause)) }
fn exc_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0];
    if slf == o { return Ok(TRUE); }
    if real_class_of(c, slf) != real_class_of(c, o) { return Ok(FALSE); }
    let m1 = funcall(c, slf, s::message, &[], std::ptr::null())?;
    let m2 = funcall(c, o, s::message, &[], std::ptr::null())?;
    let e = funcall(c, m1, s::eq, &[m2], std::ptr::null())?;
    if !e.truthy() { return Ok(FALSE); }
    let b1 = exc_bt(c, slf); let b2 = exc_bt(c, o);
    let e2 = funcall(c, b1, s::eq, &[b2], std::ptr::null())?;
    Ok(Value::bool(e2.truthy()))
}

/// error_pos (eval_error.c): the caller's position and the running method.
fn error_pos(c: &Ctx) -> String {
    let callee = c.cur_frame().and_then(|f| c.me_ref(f.me)).map(|m| m.name);
    let mut f = c.cur_frame();
    while let Some(x) = f { if x.kind != FK_CFUNC && !x.iseq().is_null() { break; } f = c.prev_frame(x); }
    let Some(x) = f else { return format!("{}: ", c.vm().main_file.borrow()) };
    match (x.line(), callee) {
        (0, _) => format!("{}: ", frame_file(x)),
        (l, Some(n)) => format!("{}:{}:in '{}': ", frame_file(x), l, sym_str(n)),
        (l, None) => format!("{}:{}: ", frame_file(x), l),
    }
}
/// print_errinfo
fn print_errinfo(c: &mut Ctx, o: &mut Vec<u8>, e: Value, bt: &[Vec<u8>], emesg: Value, highlight: bool) {
    match bt.first() { Some(l) => { o.extend_from_slice(l); o.extend_from_slice(b": "); } None => o.extend_from_slice(error_pos(c).as_bytes()) }
    let msg_empty = !is_string(emesg) || c.str_bytes(emesg).is_empty();
    let klass = real_class_of(c, e);
    // (4.0.2 has no "unhandled exception" here, only in decorate_message)
    if msg_empty { let p = class_path(c, klass); underlined(o, p.as_bytes(), highlight); }
    else { o.extend_from_slice(c.str_bytes(emesg)); }
    o.push(b'\n');
}
/// print_backtrace
/// --backtrace-limit (-1: none)
pub static BT_LIMIT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(-1);
fn print_backtrace(c: &Ctx, o: &mut Vec<u8>, e: Value, bt: &[Vec<u8>], reverse: bool) {
    let len = bt.len();
    let width = if len <= 1 { 0 } else { (len - 1).to_string().len() };
    let (mut skip_start, mut skip_len) = (usize::MAX, 0);
    if is_kind_of(c, e, c.vm().c_system_stack_error.get()) && len > 9 + 4 + 5 { skip_start = 9; skip_len = len - (9 + 4 + 5) + 5; }
    let lim = BT_LIMIT.load(std::sync::atomic::Ordering::Relaxed);
    if lim >= 0 && len as i64 > lim + 2 { skip_start = lim as usize + 1; skip_len = len - skip_start; }
    let mut i = 1;
    while i < len {
        if i == skip_start {
            o.extend_from_slice(format!("\t ... {} levels...\n", skip_len).as_bytes());
            i += skip_len;
            if i >= len { break; }
        }
        let l = &bt[if reverse { len - i } else { i }];
        o.push(b'\t');
        if reverse { o.extend_from_slice(format!("{:>w$}: ", len - i, w = width).as_bytes()); }
        o.extend_from_slice(b"from "); o.extend_from_slice(l); o.push(b'\n');
        i += 1;
    }
}
fn bt_lines(c: &mut Ctx, e: Value) -> Result<Vec<Vec<u8>>, Esc> {
    let bt = funcall(c, e, s::backtrace, &[], std::ptr::null())?;
    if !is_array(bt) { return Ok(vec![]); }
    Ok(c.ary(bt).iter().map_while(|&l| if is_string(l) { Some(c.str_bytes(l).to_vec()) } else { None }).collect())
}
/// rb_get_detailed_message
fn get_detailed_message(c: &mut Ctx, e: Value, opt: Value) -> RV {
    if !respond_to(c, e, s::detailed_message, true) { return Ok(NIL); }
    let m = funcall_kw(c, e, s::detailed_message, &[opt], std::ptr::null(), true)?;
    check_str(c, m)
}
/// show_cause
fn show_cause(c: &mut Ctx, o: &mut Vec<u8>, e: Value, opt: Value, highlight: bool, reverse: bool, shown: &mut Vec<Value>) -> Result<(), Esc> {
    let cause = ivar_get(c, e, s::cause);
    if cause == NIL || !is_exception(c, cause) || shown.contains(&cause) { return Ok(()); }
    shown.push(cause);
    let bt = bt_lines(c, cause)?;
    let m = get_detailed_message(c, cause, opt)?;
    if reverse {
        show_cause(c, o, cause, opt, highlight, reverse, shown)?;
        print_backtrace(c, o, cause, &bt, true);
        print_errinfo(c, o, cause, &bt, m, highlight);
    } else {
        print_errinfo(c, o, cause, &bt, m, highlight);
        print_backtrace(c, o, cause, &bt, false);
        show_cause(c, o, cause, opt, highlight, reverse, shown)?;
    }
    Ok(())
}
/// rb_error_write
fn error_write(c: &mut Ctx, e: Value, opt: Value, highlight: bool, reverse: bool, protect: bool) -> Result<Vec<u8>, Esc> {
    let bt = bt_lines(c, e)?;
    let saved = (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v, c.errinfo);
    let m = match get_detailed_message(c, e, opt) {
        // the uncaught report falls back to the class name (rb_ec_error_print's EC_PUSH_TAG)
        Err(_) if protect => { (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v, c.errinfo) = saved; NIL }
        r => r?,
    };
    let mut o = vec![];
    let mut shown = vec![e];
    if reverse {
        if highlight { o.extend_from_slice(BOLD); o.extend_from_slice(b"Traceback"); o.extend_from_slice(RESET); } else { o.extend_from_slice(b"Traceback"); }
        o.extend_from_slice(b" (most recent call last):\n");
        show_cause(c, &mut o, e, opt, highlight, reverse, &mut shown)?;
        print_backtrace(c, &mut o, e, &bt, true);
        print_errinfo(c, &mut o, e, &bt, m, highlight);
    } else {
        print_errinfo(c, &mut o, e, &bt, m, highlight);
        print_backtrace(c, &mut o, e, &bt, false);
        show_cause(c, &mut o, e, opt, highlight, reverse, &mut shown)?;
    }
    Ok(o)
}
/// The report of an uncaught exception (rb_ec_error_print): highlighted when stderr is a tty.
pub fn full_message(c: &mut Ctx, e: Value) -> Result<String, Esc> {
    let hl = stderr_tty();
    let opt = c.hash_new();
    crate::hash::hash_set(c, opt, sym_value("highlight"), Value::bool(hl))?;
    let o = error_write(c, e, opt, hl, false, true)?;
    Ok(String::from_utf8_lossy(&o).into_owned())
}
fn exc_full_message(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let kw = kw_arg(c, a);
    let mut hl = highlight_kw(c, kw)?;
    if hl == NIL { hl = Value::bool(stderr_tty()); }
    let mut reverse = false;
    if let Some(h) = kw {
        let ord = crate::hash::hash_get_opt(c, h, sym_value("order"))?.unwrap_or(NIL);
        if ord != NIL {
            let id = if ord.is_sym() { Some(ord) } else if is_string(ord) { Some(Value::sym(intern(c.str_bytes(ord)))) } else { None };
            if id == Some(sym_value("bottom")) { reverse = true; }
            else if id != Some(sym_value("top")) { let ins = inspect_str(c, ord)?; return arg_err(c, format!("expected :top or :bottom as order: {}", ins)); }
        }
    }
    let opt = c.hash_new();
    if let Some(h) = kw { for (k, v) in c.hash_pairs(h) { crate::hash::hash_set(c, opt, k, v)?; } }
    crate::hash::hash_set(c, opt, sym_value("highlight"), hl)?;
    let o = error_write(c, slf, opt, hl.truthy(), reverse, false)?;
    Ok(c.str_new(&o))
}
fn exc_s_to_tty(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(stderr_tty())) }

// NameError / NoMethodError / KeyError / FrozenError / StopIteration / SystemExit / etc.
fn name_err_init(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, kw) = match kw_arg(c, a) { Some(h) => (&a[..a.len() - 1], Some(h)), None => (a, None) };
    ivar_set_raw(c, slf, s::mesg, pos.first().copied().unwrap_or(NIL));
    ivar_set_raw(c, slf, s::name, pos.get(1).copied().unwrap_or(NIL));
    let nme = c.vm().c_no_method_error.get();
    if real_class_of(c, slf) == nme || is_kind_of(c, slf, nme) {
        let args = match pos.get(2) { Some(&x) => x, None => c.ary_new() };
        ivar_set_raw(c, slf, intern(b"args"), args);
        if pos.get(3).is_some_and(|p| p.truthy()) { crate::heap::obj_set_flags(slf, crate::object::FL_PRIVATE_CALL); }
    }
    if let Some(h) = kw {
        if let Some(r) = crate::hash::hash_get_opt(c, h, sym_value("receiver"))? { ivar_set_raw(c, slf, s::receiver, r); ivar_set_raw(c, slf, intern(b"__has_recv"), TRUE); }
    }
    let is = ruby_level_iseq(c);
    if !is.is_null() { let at = c.alloc(c.vm().c_object.get(), Body::Data(Box::new(LvAt(is)))); ivar_set_raw(c, slf, intern(b"__lv_at"), at); }
    Ok(NIL)
}
fn name_err_name(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, slf, s::name)) }

/// The iseq of the Ruby-level frame a NameError was made in (CRuby keeps it at NameError.new).
struct LvAt(*const Iseq);
impl RData for LvAt {
    fn visit(&mut self, _m: &mut crate::gc::Visitor) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn ruby_level_iseq(c: &Ctx) -> *const Iseq {
    let mut f = c.cur_frame();
    while let Some(x) = f { if x.kind != FK_CFUNC && !x.iseq().is_null() { return x.iseq(); } f = c.prev_frame(x); }
    std::ptr::null()
}
/// NameError#local_variables: the locals where it was made (`__lv_at`: NameError.new's LvAt, or the captured backtrace
/// of a raise by the VM once it is formatted); one raised by the VM and not formatted: its backtrace's top Ruby frame.
fn name_err_local_variables(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let at = ivar_get(c, slf, intern(b"__lv_at"));
    let is = match c.data::<LvAt>(at) {
        Some(l) => l.0,
        None => { let r = raw_bt_ruby_iseq(c, at); if r.is_null() { raw_bt_ruby_iseq(c, ivar_get(c, slf, s::bt)) } else { r } }
    };
    let names = crate::builtins::binding::iseq_var_names(c, is);
    let a = c.ary_new_capa(names.len());
    for n in names { c.ary_push(a, Value::sym(n)); }
    Ok(a)
}
/// A NameError's captured backtrace about to be replaced: kept for local_variables unless NameError.new kept the place.
pub fn keep_raw_bt(c: &mut Ctx, e: Value, raw: Value) {
    let k = intern(b"__lv_at");
    if ivar_get(c, e, k) == NIL && is_kind_of(c, e, c.vm().c_name_error.get()) { ivar_set_raw(c, e, k, raw); }
}
fn name_err_receiver(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if !ivar_defined(c, slf, s::receiver) { return arg_err(c, "no receiver is available"); }
    Ok(ivar_get(c, slf, s::receiver))
}
fn nm_err_args(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let v = ivar_get(c, slf, intern(b"args")); Ok(if v == NIL { c.ary_new() } else { v }) }
fn key_err_init(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, kw) = match kw_arg(c, a) { Some(h) => (&a[..a.len() - 1], Some(h)), None => (a, None) };
    ivar_set_raw(c, slf, s::mesg, pos.first().copied().unwrap_or(NIL));
    if let Some(h) = kw {
        if let Some(r) = crate::hash::hash_get_opt(c, h, sym_value("receiver"))? { ivar_set_raw(c, slf, s::receiver, r); }
        if let Some(k) = crate::hash::hash_get_opt(c, h, sym_value("key"))? { ivar_set_raw(c, slf, s::key, k); }
    }
    Ok(NIL)
}
/// NoMatchingPatternKeyError.new(msg = nil, matchee: , key: )
fn nmpk_err_init(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, kw) = match kw_arg(c, a) { Some(h) => (&a[..a.len() - 1], Some(h)), None => (a, None) };
    check_argc(c, pos, 0, 1)?;
    ivar_set_raw(c, slf, s::mesg, pos.first().copied().unwrap_or(NIL));
    if let Some(h) = kw {
        if let Some(r) = crate::hash::hash_get_opt(c, h, sym_value("matchee"))? { ivar_set_raw(c, slf, s::receiver, r); }
        if let Some(k) = crate::hash::hash_get_opt(c, h, sym_value("key"))? { ivar_set_raw(c, slf, s::key, k); }
    }
    Ok(NIL)
}
fn nmpk_err_matchee(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if !ivar_defined(c, slf, s::receiver) { return arg_err(c, "no matchee is available"); }
    Ok(ivar_get(c, slf, s::receiver))
}
fn key_err_key(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if !ivar_defined(c, slf, s::key) { return arg_err(c, "no key is available"); }
    Ok(ivar_get(c, slf, s::key))
}
fn frozen_err_init(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, kw) = match kw_arg(c, a) { Some(h) => (&a[..a.len() - 1], Some(h)), None => (a, None) };
    ivar_set_raw(c, slf, s::mesg, pos.first().copied().unwrap_or(NIL));
    if let Some(h) = kw { if let Some(r) = crate::hash::hash_get_opt(c, h, sym_value("receiver"))? { ivar_set_raw(c, slf, s::receiver, r); } }
    Ok(NIL)
}
fn stop_iter_result(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, slf, intern(b"result"))) }
fn sysexit_init(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut st = 0;
    let mut i = 0;
    if let Some(&v) = a.first() {
        if v == TRUE { st = 0; i = 1; } else if v == FALSE { st = 1; i = 1; } else if v.is_fixnum() { st = v.as_fix(); i = 1; }
    }
    let msg = a.get(i).copied().unwrap_or(NIL);   // none: the message is the class name, as CRuby
    ivar_set_raw(c, slf, s::mesg, msg);
    ivar_set_raw(c, slf, s::status, Value::fix(st));
    Ok(NIL)
}
fn sysexit_status(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let v = ivar_get(c, slf, s::status); Ok(if v == NIL { Value::fix(0) } else { v }) }
fn sysexit_success(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let v = ivar_get(c, slf, s::status); Ok(Value::bool(v == NIL || v == Value::fix(0))) }
fn uncaught_tag(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, slf, intern(b"tag"))) }
fn uncaught_value(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, slf, intern(b"value"))) }
fn syscallerr_init(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    // syserr_initialize
    let k = real_class_of(c, slf);
    let (mut mesg, mut error, func);
    if k == c.vm().c_system_call_error.get() {
        check_argc(c, a, 1, 3)?;
        (mesg, error, func) = (a[0], a.get(1).copied().unwrap_or(NIL), a.get(2).copied().unwrap_or(NIL));
        if a.len() == 1 && mesg.is_fixnum() { error = mesg; mesg = NIL; }
        if error != NIL {
            // a known errno turns the instance into Errno::X
            let n = crate::builtins::numeric_methods::num_to_long(c, error)?;
            if errno_message(n).is_some() { let ek = errno_class(c, n); obj_set_klass(slf, ek); }
        }
    } else {
        check_argc(c, a, 0, 2)?;
        (mesg, func) = (a.first().copied().unwrap_or(NIL), a.get(1).copied().unwrap_or(NIL));
        error = NIL;
        for &kk in ancestors(c, k).iter() { if let Some(v) = const_get_at(c, kk, intern(b"Errno")) { error = v; break; } }
    }
    let mut msg = if error == NIL { "unknown error".to_string() } else { let n = crate::builtins::numeric_methods::num_to_long(c, error)?; errno_text(Value::fix(n)) };
    if mesg != NIL {
        let s = expect_str(c, mesg)?;
        let ds = String::from_utf8_lossy(c.str_bytes(s)).into_owned();
        if func != NIL { let f = to_rstring(c, func)?; msg = format!("{} @ {}", msg, f); }
        msg = format!("{} - {}", msg, ds);
    }
    let m = c.str_new(msg.as_bytes());
    ivar_set_raw(c, slf, s::mesg, m);
    ivar_set_raw(c, slf, s::errno, error);
    Ok(NIL)
}
fn syscallerr_errno(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, slf, s::errno)) }

pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let e = vmr.c_exception.get();
    define_private_cfunc(c, e, "initialize", exc_initialize, -1);
    define_singleton_cfunc(c, e, "exception", exc_s_exception, -1);
    define_singleton_cfunc(c, e, "to_tty?", exc_s_to_tty, 0);
    define_cfunc(c, e, "exception", exc_exception, -1);
    define_cfunc(c, e, "to_s", exc_to_s, 0);
    define_cfunc(c, e, "message", exc_message, 0);
    define_cfunc(c, e, "detailed_message", exc_detailed_message, -1);
    define_cfunc(c, e, "inspect", exc_inspect, 0);
    define_cfunc(c, e, "backtrace", exc_backtrace, 0);
    define_cfunc(c, e, "set_backtrace", exc_set_backtrace, 1);
    define_cfunc(c, e, "backtrace_locations", exc_backtrace_locations, 0);
    define_cfunc(c, e, "cause", exc_cause, 0);
    define_cfunc(c, e, "==", exc_eq, 1);
    define_cfunc(c, e, "full_message", exc_full_message, -1);
    let ne = vmr.c_name_error.get();
    define_private_cfunc(c, ne, "initialize", name_err_init, -1);
    define_cfunc(c, ne, "name", name_err_name, 0);
    define_cfunc(c, ne, "receiver", name_err_receiver, 0);
    define_cfunc(c, ne, "local_variables", name_err_local_variables, 0);
    define_cfunc(c, vmr.c_no_method_error.get(), "args", nm_err_args, 0);
    define_private_cfunc(c, vmr.c_key_error.get(), "initialize", key_err_init, -1);
    define_cfunc(c, vmr.c_key_error.get(), "key", key_err_key, 0);
    define_cfunc(c, vmr.c_key_error.get(), "receiver", name_err_receiver, 0);
    define_private_cfunc(c, vmr.c_frozen_error.get(), "initialize", frozen_err_init, -1);
    define_cfunc(c, vmr.c_frozen_error.get(), "receiver", name_err_receiver, 0);
    define_cfunc(c, vmr.c_no_matching_pattern_key_error.get(), "key", key_err_key, 0);
    define_private_cfunc(c, vmr.c_no_matching_pattern_key_error.get(), "initialize", nmpk_err_init, -1);
    define_cfunc(c, vmr.c_no_matching_pattern_key_error.get(), "matchee", nmpk_err_matchee, 0);
    define_cfunc(c, vmr.c_stop_iteration.get(), "result", stop_iter_result, 0);
    define_private_cfunc(c, vmr.c_system_exit.get(), "initialize", sysexit_init, -1);
    define_cfunc(c, vmr.c_system_exit.get(), "status", sysexit_status, 0);
    define_cfunc(c, vmr.c_system_exit.get(), "success?", sysexit_success, 0);
    define_cfunc(c, vmr.c_uncaught_throw_error.get(), "tag", uncaught_tag, 0);
    define_cfunc(c, vmr.c_uncaught_throw_error.get(), "value", uncaught_value, 0);
    define_private_cfunc(c, vmr.c_system_call_error.get(), "initialize", syscallerr_init, -1);
    define_cfunc(c, vmr.c_system_call_error.get(), "errno", syscallerr_errno, 0);
    define_cfunc(c, vmr.c_local_jump_error.get(), "reason", |c, s, _a, _b| { let r = ivar_get(c, s, intern(b"__reason")); Ok(if r == NIL { sym_value("noreason") } else { r }) }, 0);
    define_cfunc(c, vmr.c_local_jump_error.get(), "exit_value", |c, s, _a, _b| Ok(ivar_get(c, s, intern(b"__exit_value"))), 0);
    let _ = sym_str;
}
