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

pub fn init_classes(vmr: &mut Vm) {
    let ob = vmr.c_object;
    let ex = define_class("Exception", ob);
    class_set_alloc(ex, AllocKind::Exception);
    vmr.c_exception = ex;
    let d = |n: &str, sup: Value| define_class(n, sup);
    vmr.c_no_memory_error = d("NoMemoryError", ex);
    vmr.c_script_error = d("ScriptError", ex);
    vmr.c_load_error = d("LoadError", vmr.c_script_error);
    vmr.c_not_impl_error = d("NotImplementedError", vmr.c_script_error);
    vmr.c_syntax_error = d("SyntaxError", vmr.c_script_error);
    vmr.c_security_error = d("SecurityError", ex);
    vmr.c_signal_exception = d("SignalException", ex);
    vmr.c_interrupt = d("Interrupt", vmr.c_signal_exception);
    vmr.c_system_exit = d("SystemExit", ex);
    vmr.c_system_stack_error = d("SystemStackError", ex);
    let se = d("StandardError", ex);
    vmr.c_standard_error = se;
    vmr.c_argument_error = d("ArgumentError", se);
    vmr.c_uncaught_throw_error = d("UncaughtThrowError", vmr.c_argument_error);
    vmr.c_encoding_error = d("EncodingError", se);
    vmr.c_fiber_error = d("FiberError", se);
    vmr.c_io_error = d("IOError", se);
    vmr.c_eof_error = d("EOFError", vmr.c_io_error);
    vmr.c_index_error = d("IndexError", se);
    vmr.c_key_error = d("KeyError", vmr.c_index_error);
    vmr.c_stop_iteration = d("StopIteration", vmr.c_index_error);
    vmr.c_closed_queue_error = d("ClosedQueueError", vmr.c_stop_iteration);
    vmr.c_local_jump_error = d("LocalJumpError", se);
    vmr.c_name_error = d("NameError", se);
    vmr.c_no_method_error = d("NoMethodError", vmr.c_name_error);
    vmr.c_range_error = d("RangeError", se);
    vmr.c_float_domain_error = d("FloatDomainError", vmr.c_range_error);
    vmr.c_regexp_error = d("RegexpError", se);
    vmr.c_runtime_error = d("RuntimeError", se);
    vmr.c_frozen_error = d("FrozenError", vmr.c_runtime_error);
    vmr.c_system_call_error = d("SystemCallError", se);
    vmr.c_thread_error = d("ThreadError", se);
    vmr.c_type_error = d("TypeError", se);
    vmr.c_zero_div_error = d("ZeroDivisionError", se);
    vmr.c_no_matching_pattern_error = d("NoMatchingPatternError", se);
    vmr.c_no_matching_pattern_key_error = d("NoMatchingPatternKeyError", vmr.c_no_matching_pattern_error);
    let m = define_module("Math");
    vmr.m_math = m;
    vmr.c_math_domain_error = define_class_under(m, "DomainError", vmr.c_argument_error);
    let enc = define_class("Encoding", ob);
    vmr.c_encoding = enc;
    vmr.c_compat_error = define_class_under(enc, "CompatibilityError", vmr.c_encoding_error);
    vmr.c_invalid_byte_seq_error = define_class_under(enc, "InvalidByteSequenceError", vmr.c_encoding_error);
    vmr.c_undef_conv_error = define_class_under(enc, "UndefinedConversionError", vmr.c_encoding_error);
    vmr.c_converter_not_found_error = define_class_under(enc, "ConverterNotFoundError", vmr.c_encoding_error);
    vmr.m_errno = define_module("Errno");
    // Errno::* classes are built on first use (autoload of an internal feature) to keep boot short
    let names: Vec<&str> = ERRNOS.iter().map(|e| e.0).chain(ERRNO_ALIASES.iter().map(|a| a.0)).collect();
    crate::builtins::lazy_consts(vmr.m_errno, &names, "saruby/errno");
}
const ERRNO_ALIASES: &[(&str, &str)] = &[("EOPNOTSUPP", "ENOTSUP"), ("EDEADLOCK", "EDEADLK")];
// Errno::X: its number is the constant `Errno`; its message comes from ERRNOS by that number.
fn define_errno(vmr: &Vm, name: &str, no: i64) -> Value {
    if let Some(k) = const_get_at(vmr.m_errno, intern(name.as_bytes())) { return k; }
    let k = define_class_under(vmr.m_errno, name, vmr.c_system_call_error);
    const_set(k, intern(b"Errno"), Value::fix(no));
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
    let m = c.vm().m_errno;
    for (name, no, _) in ERRNOS { define_errno(c.vm(), name, *no); }
    for (alias, name) in ERRNO_ALIASES {
        let k = const_get_at(m, intern(name.as_bytes())).unwrap();
        if const_get_at(m, intern(alias.as_bytes())).is_none() { const_set(m, intern(alias.as_bytes()), k); }
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
];

/// Errno class of `no`, made on first use (through vm.rs: no payload is touched here).
pub fn errno_class(vmr: &Vm, no: i64) -> Value {
    // ENOTSUP and friends share numbers: the first table entry names the class
    for (name, n, _) in ERRNOS { if *n == no { return define_errno(vmr, name, *n); } }
    vmr.c_system_call_error
}

pub fn raise_errno(c: &mut Ctx, no: i64, detail: &str) -> Esc {
    let k = errno_class(c.vm(), no);
    let base = errno_text(Value::fix(no));
    let msg = if detail.is_empty() { base } else { format!("{} @ {}", base, detail) };
    let e = make_exception(k, &msg);
    ivar_set_raw(e, s::errno, Value::fix(no));
    raise_exc(c, e)
}

/// Errno with CRuby's rb_sys_fail text: "<strerror> - <detail>".
pub fn raise_errno_plain(c: &mut Ctx, no: i64, detail: &str) -> Esc {
    let k = errno_class(c.vm(), no);
    let base = errno_text(Value::fix(no));
    let e = make_exception(k, &format!("{} - {}", base, detail));
    ivar_set_raw(e, s::errno, Value::fix(no));
    raise_exc(c, e)
}

pub fn io_err_to_errno(e: &std::io::Error) -> i64 { e.raw_os_error().map(|x| x as i64).unwrap_or(5) }

fn exc_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    ivar_set_raw(slf, s::mesg, if a.is_empty() { NIL } else { a[0] });
    Ok(NIL)
}
fn exc_s_exception(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { crate::builtins::object::c_new(c, slf, a, b) }
fn exc_exception(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() || a[0] == slf { return Ok(slf); }
    let e = crate::builtins::object::obj_dup(c, slf, true, None)?;
    ivar_set_raw(e, s::mesg, a[0]);
    ivar_set_raw(e, s::bt, NIL);
    Ok(e)
}
pub fn exc_message_str(c: &mut Ctx, e: Value) -> Result<String, Esc> {
    let m = funcall(c, e, s::message, &[], std::ptr::null())?;
    if is_string(m) { Ok(String::from_utf8_lossy(c.str_bytes(m)).into_owned()) } else { to_rstring(c, m) }
}
fn exc_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let m = ivar_get(slf, s::mesg);
    if m == NIL { return Ok(c.str_new(class_path(real_class_of(slf)).as_bytes())); }
    if is_string(m) { return Ok(m); }
    to_s_value(c, m)
}
fn exc_message(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { funcall(c, slf, s::to_s, &[], std::ptr::null()) }
fn exc_detailed_message(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let m = exc_message_str(c, slf)?;
    let highlight = match kw_arg(a) { Some(h) => crate::hash::hash_get_opt(c, h, sym_value("highlight"))?.map(|v| v.truthy()).unwrap_or(false), None => false };
    let cls = class_path(real_class_of(slf));
    if m.is_empty() { return Ok(c.str_new(if highlight { format!("\x1b[1;4m{}\x1b[m", cls) } else { cls }.as_bytes())); }
    let mut lines = m.splitn(2, '\n');
    let first = lines.next().unwrap_or("");
    let rest = lines.next();
    let mut s = if highlight { format!("\x1b[1m{} (\x1b[1;4m{}\x1b[m\x1b[1m)\x1b[m", first, cls) } else { format!("{} ({})", first, cls) };
    if let Some(r) = rest { s.push('\n'); s.push_str(&if highlight { format!("\x1b[1m{}\x1b[m", r) } else { r.to_string() }); }
    Ok(c.str_new(s.as_bytes()))
}
fn exc_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let cls = class_path(real_class_of(slf));
    let m = exc_message_str(c, slf)?;
    if m.is_empty() { return Ok(c.str_new(cls.as_bytes())); }
    if m.contains('\n') { return Ok(c.str_new(format!("#<{}:{}>", cls, inspect_bytes(m.as_bytes(), ENC_UTF8)).as_bytes())); }
    Ok(c.str_new(format!("#<{}: {}>", cls, m).as_bytes()))
}
fn exc_backtrace(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(exc_bt(slf)) }
fn exc_set_backtrace(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let v = a[0];
    let bt = if is_string(v) { c.ary_new_from(&[v]) } else if v == NIL || is_array(v) { v } else { return type_err(c, "backtrace must be an Array of String or an Array of Thread::Backtrace::Location"); };
    let loc = c.vm().c_location;
    if is_array(bt) && !c.ary(bt).iter().all(|&x| is_string(x)) && !c.ary(bt).iter().all(|&x| x.is_heap() && is_kind_of(x, loc)) {
        return type_err(c, "backtrace must be an Array of String or an Array of Thread::Backtrace::Location");
    }
    if is_array(bt) && c.ary_len(bt) > 0 && c.ary(bt).iter().all(|&x| x.is_heap() && is_kind_of(x, loc)) {
        let strs = c.ary_new();
        let mut i = 0;
        while let Some(&x) = c.ary(bt).get(i) { let s = funcall(c, x, s::to_s, &[], std::ptr::null())?; c.ary_push(strs, s); i += 1; }
        ivar_set_raw(slf, s::bt, strs);
        ivar_set_raw(slf, intern(b"__bt_locs"), bt);
        return Ok(bt);
    }
    ivar_set_raw(slf, s::bt, bt);
    ivar_set_raw(slf, intern(b"__bt_locs"), NIL);
    ivar_set_raw(slf, intern(b"__bt_set"), TRUE);
    Ok(bt)
}
fn exc_backtrace_locations(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let locs = ivar_get(slf, intern(b"__bt_locs"));
    if locs != NIL { return Ok(locs); }
    let bt = exc_bt(slf);
    if bt == NIL || ivar_get(slf, intern(b"__bt_set")) == TRUE { return Ok(NIL); }
    let out = c.ary_new();
    let mut i = 0;
    while let Some(&l) = c.ary(bt).get(i) {
        if is_string(l) { let s = String::from_utf8_lossy(c.str_bytes(l)).into_owned(); let loc = location_from_str(c, &s); c.ary_push(out, loc); }
        else { c.ary_push(out, l); }
        i += 1;
    }
    ivar_set_raw(slf, intern(b"__bt_locs"), out);
    Ok(out)
}

// ---- Thread::Backtrace::Location ----------------------------------------------------------

/// "path:line:in 'label'" -> Location
pub fn location_from_str(c: &mut Ctx, s: &str) -> Value {
    let (head, label) = match s.rfind(":in '") { Some(p) => (&s[..p], s[p + 5..].trim_end_matches('\'')), None => (s, "") };
    let (path, line) = match head.rsplit_once(':') { Some((p, l)) if l.parse::<i64>().is_ok() => (p, l.parse::<i64>().unwrap()), _ => (head, 0) };
    let o = new_object(c.vm().c_location);
    let p = c.str_new(path.as_bytes());
    ivar_set_raw(o, intern(b"path"), p);
    ivar_set_raw(o, intern(b"lineno"), Value::fix(line));
    let l = c.str_new(label.as_bytes());
    ivar_set_raw(o, intern(b"label"), l);
    o
}
pub fn locations_from_lines(c: &mut Ctx, lines: Vec<String>) -> Value {
    let out = c.ary_new();
    for l in lines { let v = location_from_str(c, &l); c.ary_push(out, v); }
    out
}
fn loc_path(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(slf, intern(b"path"))) }
fn loc_abs_path(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let p = ivar_get(slf, intern(b"path"));
    let ps = String::from_utf8_lossy(c.str_bytes(p)).into_owned();
    if ps.starts_with('<') || ps.starts_with('(') || ps == "-e" { return Ok(NIL); }
    match std::fs::canonicalize(&ps) { Ok(a) => Ok(c.str_new(a.to_string_lossy().as_bytes())), Err(_) => Ok(p) }
}
fn loc_lineno(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(slf, intern(b"lineno"))) }
fn loc_label(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(slf, intern(b"label"))) }
fn loc_base_label(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let l = ivar_get(slf, intern(b"label"));
    let mut s = String::from_utf8_lossy(c.str_bytes(l)).into_owned();
    if let Some(p) = s.find(" in ") { if s.starts_with("block") || s.starts_with("rescue") || s.starts_with("ensure") { s = s[p + 4..].to_string(); } }
    if !s.starts_with('<') { if let Some(p) = s.rfind(['#', '.']) { s = s[p + 1..].to_string(); } }
    Ok(c.str_new(s.as_bytes()))
}
fn loc_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let p = ivar_get(slf, intern(b"path")); let l = ivar_get(slf, intern(b"label"));
    let mut s = String::from_utf8_lossy(c.str_bytes(p)).into_owned();
    s.push_str(&format!(":{}:in '{}'", ivar_get(slf, intern(b"lineno")).as_fix(), String::from_utf8_lossy(c.str_bytes(l))));
    Ok(c.str_new(s.as_bytes()))
}
fn loc_inspect(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let s = loc_to_s(c, slf, a, b)?;
    funcall(c, s, intern(b"inspect"), &[], std::ptr::null())
}
fn k_caller_locations(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let (start, len) = caller_range(c, args)?;
    let Some(lines) = backtrace_lines_n(c, 1 + start, len) else { return Ok(NIL) };
    Ok(locations_from_lines(c, lines))
}
/// caller / caller_locations arguments: (start = 1, length = nil) or (range)
pub fn caller_range(c: &mut Ctx, args: &[Value]) -> Result<(usize, Option<usize>), Esc> {
    if args.is_empty() { return Ok((1, None)); }
    let a = args[0];
    if a.is_fixnum() {
        if a.as_fix() < 0 { return arg_err(c, format!("negative level ({})", a.as_fix())); }
        let len = match args.get(1) { Some(&l) if l.is_fixnum() => { if l.as_fix() < 0 { return arg_err(c, format!("negative size ({})", l.as_fix())); } Some(l.as_fix() as usize) }, _ => None };
        return Ok((a.as_fix() as usize, len));
    }
    if is_kind_of(a, c.vm().c_range) {
        let b = funcall(c, a, intern(b"begin"), &[], std::ptr::null())?;
        let e = funcall(c, a, intern(b"end"), &[], std::ptr::null())?;
        let excl = funcall(c, a, intern(b"exclude_end?"), &[], std::ptr::null())?.truthy();
        let b = if b == NIL { 0 } else { b.as_fix().max(0) as usize };
        if e == NIL { return Ok((b, None)); }
        let e = e.as_fix();
        if e < 0 { return Ok((b, None)); }
        let n = (e as usize + if excl { 0 } else { 1 }).saturating_sub(b);
        return Ok((b, Some(n)));
    }
    Ok((crate::builtins::numeric_methods::num_to_long(c, a)?.max(0) as usize, None))
}

/// Thread::Backtrace::Location; returns the class.
pub fn init_location(vmr: &Vm) -> Value {
    let bt = define_class_under(vmr.c_thread, "Backtrace", vmr.c_object);
    let k = define_class_under(bt, "Location", vmr.c_object);
    define_cfunc(k, "path", loc_path, 0);
    define_cfunc(k, "absolute_path", loc_abs_path, 0);
    define_cfunc(k, "lineno", loc_lineno, 0);
    define_cfunc(k, "label", loc_label, 0);
    define_cfunc(k, "base_label", loc_base_label, 0);
    define_cfunc(k, "to_s", loc_to_s, 0);
    define_cfunc(k, "inspect", loc_inspect, 0);
    define_module_function(vmr.m_kernel, "caller_locations", k_caller_locations, -1);
    k
}
fn exc_cause(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(slf, s::cause)) }
fn exc_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if slf == o { return Ok(TRUE); }
    if real_class_of(slf) != real_class_of(o) { return Ok(FALSE); }
    let m1 = funcall(c, slf, s::message, &[], std::ptr::null())?;
    let m2 = funcall(c, o, s::message, &[], std::ptr::null())?;
    let e = funcall(c, m1, s::eq, &[m2], std::ptr::null())?;
    if !e.truthy() { return Ok(FALSE); }
    let b1 = exc_bt(slf); let b2 = exc_bt(o);
    let e2 = funcall(c, b1, s::eq, &[b2], std::ptr::null())?;
    Ok(Value::bool(e2.truthy()))
}

pub fn full_message(c: &mut Ctx, e: Value, _highlight: bool) -> Result<String, Esc> {
    let bt = exc_bt(e);
    let mut lines: Vec<String> = vec![];
    if is_array(bt) { for &l in c.ary(bt) { if is_string(l) { lines.push(String::from_utf8_lossy(c.str_bytes(l)).into_owned()); } } }
    let dm = funcall(c, e, s::detailed_message, &[], std::ptr::null())?;
    let msg = if is_string(dm) { String::from_utf8_lossy(c.str_bytes(dm)).into_owned() } else { exc_message_str(c, e)? };
    let mut out = String::new();
    let first = lines.first().cloned().unwrap_or_else(|| "-e:1:in '<main>'".into());
    if lines.is_empty() { out.push_str(&format!("{}: {}\n", c.vm().main_file, msg)); }
    else { out.push_str(&format!("{}: {}\n", first, msg)); }
    for l in lines.iter().skip(1) { out.push_str(&format!("\tfrom {}\n", l)); }
    let cause = ivar_get(e, s::cause);
    if cause != NIL && cause != e && is_exception(cause) {
        let cs = full_message(c, cause, false)?;
        out.push_str(&cs);
    }
    Ok(out)
}
fn exc_full_message(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = full_message(c, slf, false)?;
    Ok(c.str_new(s.as_bytes()))
}
fn exc_s_to_tty(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(FALSE) }

// NameError / NoMethodError / KeyError / FrozenError / StopIteration / SystemExit / etc.
fn name_err_init(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, kw) = match kw_arg(a) { Some(h) => (&a[..a.len() - 1], Some(h)), None => (a, None) };
    ivar_set_raw(slf, s::mesg, pos.first().copied().unwrap_or(NIL));
    ivar_set_raw(slf, s::name, pos.get(1).copied().unwrap_or(NIL));
    let nme = c.vm().c_no_method_error;
    if real_class_of(slf) == nme || is_kind_of(slf, nme) {
        let args = match pos.get(2) { Some(&x) => x, None => c.ary_new() };
        ivar_set_raw(slf, intern(b"args"), args);
    }
    if let Some(h) = kw {
        if let Some(r) = crate::hash::hash_get_opt(c, h, sym_value("receiver"))? { ivar_set_raw(slf, s::receiver, r); ivar_set_raw(slf, intern(b"__has_recv"), TRUE); }
    }
    Ok(NIL)
}
fn name_err_name(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(slf, s::name)) }
fn name_err_receiver(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if !ivar_defined(slf, s::receiver) { return arg_err(c, "no receiver is available"); }
    Ok(ivar_get(slf, s::receiver))
}
fn nm_err_args(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let v = ivar_get(slf, intern(b"args")); Ok(if v == NIL { c.ary_new() } else { v }) }
fn key_err_init(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, kw) = match kw_arg(a) { Some(h) => (&a[..a.len() - 1], Some(h)), None => (a, None) };
    ivar_set_raw(slf, s::mesg, pos.first().copied().unwrap_or(NIL));
    if let Some(h) = kw {
        if let Some(r) = crate::hash::hash_get_opt(c, h, sym_value("receiver"))? { ivar_set_raw(slf, s::receiver, r); }
        if let Some(k) = crate::hash::hash_get_opt(c, h, sym_value("key"))? { ivar_set_raw(slf, s::key, k); }
    }
    Ok(NIL)
}
fn key_err_key(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if !ivar_defined(slf, s::key) { return arg_err(c, "no key is available"); }
    Ok(ivar_get(slf, s::key))
}
fn frozen_err_init(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, kw) = match kw_arg(a) { Some(h) => (&a[..a.len() - 1], Some(h)), None => (a, None) };
    ivar_set_raw(slf, s::mesg, pos.first().copied().unwrap_or(NIL));
    if let Some(h) = kw { if let Some(r) = crate::hash::hash_get_opt(c, h, sym_value("receiver"))? { ivar_set_raw(slf, s::receiver, r); } }
    Ok(NIL)
}
fn stop_iter_result(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(slf, intern(b"result"))) }
fn sysexit_init(_c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mut st = 0;
    let mut i = 0;
    if let Some(&v) = a.first() {
        if v == TRUE { st = 0; i = 1; } else if v == FALSE { st = 1; i = 1; } else if v.is_fixnum() { st = v.as_fix(); i = 1; }
    }
    let msg = a.get(i).copied().unwrap_or(NIL);   // none: the message is the class name, as CRuby
    ivar_set_raw(slf, s::mesg, msg);
    ivar_set_raw(slf, s::status, Value::fix(st));
    Ok(NIL)
}
fn sysexit_status(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let v = ivar_get(slf, s::status); Ok(if v == NIL { Value::fix(0) } else { v }) }
fn sysexit_success(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let v = ivar_get(slf, s::status); Ok(Value::bool(v == NIL || v == Value::fix(0))) }
fn uncaught_tag(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(slf, intern(b"tag"))) }
fn uncaught_value(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(slf, intern(b"value"))) }
fn syscallerr_init(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let k = real_class_of(slf);
    let mut no = NIL;
    for &kk in ancestors(k).iter() { if let Some(v) = const_get_at(kk, intern(b"Errno")) { if v.is_fixnum() { no = v; break; } } }
    let mut detail = a.first().copied().unwrap_or(NIL);
    if k == c.vm().c_system_call_error {
        if a.len() == 1 && detail.is_fixnum() { no = detail; detail = NIL; } else if a.len() >= 2 && a[1].is_fixnum() { no = a[1]; }
        // a known errno turns the instance into Errno::X, as CRuby
        if no.is_fixnum() && errno_message(no.as_fix()).is_some() { let ek = errno_class(c.vm(), no.as_fix()); obj_set_klass(slf, ek); }
    }
    let mut msg = errno_text(no);
    if detail != NIL { let ds = to_rstring(c, detail)?; msg = format!("{} - {}", msg, ds); }
    let m = c.str_new(msg.as_bytes());
    ivar_set_raw(slf, s::mesg, m);
    ivar_set_raw(slf, s::errno, no);
    Ok(NIL)
}
fn syscallerr_errno(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(slf, s::errno)) }

pub fn init(vmr: &Vm) {
    let e = vmr.c_exception;
    define_private_cfunc(e, "initialize", exc_initialize, -1);
    define_singleton_cfunc(e, "exception", exc_s_exception, -1);
    define_singleton_cfunc(e, "to_tty?", exc_s_to_tty, 0);
    define_cfunc(e, "exception", exc_exception, -1);
    define_cfunc(e, "to_s", exc_to_s, 0);
    define_cfunc(e, "message", exc_message, 0);
    define_cfunc(e, "detailed_message", exc_detailed_message, -1);
    define_cfunc(e, "inspect", exc_inspect, 0);
    define_cfunc(e, "backtrace", exc_backtrace, 0);
    define_cfunc(e, "set_backtrace", exc_set_backtrace, 1);
    define_cfunc(e, "backtrace_locations", exc_backtrace_locations, 0);
    define_cfunc(e, "cause", exc_cause, 0);
    define_cfunc(e, "==", exc_eq, 1);
    define_cfunc(e, "full_message", exc_full_message, -1);
    let ne = vmr.c_name_error;
    define_private_cfunc(ne, "initialize", name_err_init, -1);
    define_cfunc(ne, "name", name_err_name, 0);
    define_cfunc(ne, "receiver", name_err_receiver, 0);
    define_cfunc(vmr.c_no_method_error, "args", nm_err_args, 0);
    define_private_cfunc(vmr.c_key_error, "initialize", key_err_init, -1);
    define_cfunc(vmr.c_key_error, "key", key_err_key, 0);
    define_cfunc(vmr.c_key_error, "receiver", name_err_receiver, 0);
    define_private_cfunc(vmr.c_frozen_error, "initialize", frozen_err_init, -1);
    define_cfunc(vmr.c_frozen_error, "receiver", name_err_receiver, 0);
    define_cfunc(vmr.c_no_matching_pattern_key_error, "key", key_err_key, 0);
    define_cfunc(vmr.c_no_matching_pattern_key_error, "matchee", name_err_receiver, 0);
    define_cfunc(vmr.c_stop_iteration, "result", stop_iter_result, 0);
    define_private_cfunc(vmr.c_system_exit, "initialize", sysexit_init, -1);
    define_cfunc(vmr.c_system_exit, "status", sysexit_status, 0);
    define_cfunc(vmr.c_system_exit, "success?", sysexit_success, 0);
    define_cfunc(vmr.c_uncaught_throw_error, "tag", uncaught_tag, 0);
    define_cfunc(vmr.c_uncaught_throw_error, "value", uncaught_value, 0);
    define_private_cfunc(vmr.c_system_call_error, "initialize", syscallerr_init, -1);
    define_cfunc(vmr.c_system_call_error, "errno", syscallerr_errno, 0);
    define_cfunc(vmr.c_local_jump_error, "reason", |_c, s, _a, _b| { let r = ivar_get(s, intern(b"__reason")); Ok(if r == NIL { sym_value("noreason") } else { r }) }, 0);
    define_cfunc(vmr.c_local_jump_error, "exit_value", |_c, s, _a, _b| Ok(ivar_get(s, intern(b"__exit_value"))), 0);
    let _ = sym_str;
}
