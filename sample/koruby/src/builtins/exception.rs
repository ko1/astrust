// Exception hierarchy and methods.
use crate::array::*;
use crate::builtins::kernel::*;
use crate::string::*;
use crate::symbol::{intern, s, sym_str};
use crate::value::*;
use crate::vm::*;

pub fn init_classes() {
    let vm = vm();
    let ob = vm.c_object;
    let ex = define_class("Exception", ob);
    rclass(ex).alloc = AllocKind::Exception;
    vm.c_exception = ex;
    let d = |n: &str, sup: Value| define_class(n, sup);
    vm.c_no_memory_error = d("NoMemoryError", ex);
    vm.c_script_error = d("ScriptError", ex);
    vm.c_load_error = d("LoadError", vm.c_script_error);
    vm.c_not_impl_error = d("NotImplementedError", vm.c_script_error);
    vm.c_syntax_error = d("SyntaxError", vm.c_script_error);
    vm.c_security_error = d("SecurityError", ex);
    vm.c_signal_exception = d("SignalException", ex);
    vm.c_interrupt = d("Interrupt", vm.c_signal_exception);
    vm.c_system_exit = d("SystemExit", ex);
    vm.c_system_stack_error = d("SystemStackError", ex);
    let se = d("StandardError", ex);
    vm.c_standard_error = se;
    vm.c_argument_error = d("ArgumentError", se);
    vm.c_uncaught_throw_error = d("UncaughtThrowError", vm.c_argument_error);
    vm.c_encoding_error = d("EncodingError", se);
    vm.c_fiber_error = d("FiberError", se);
    vm.c_io_error = d("IOError", se);
    vm.c_eof_error = d("EOFError", vm.c_io_error);
    vm.c_index_error = d("IndexError", se);
    vm.c_key_error = d("KeyError", vm.c_index_error);
    vm.c_stop_iteration = d("StopIteration", vm.c_index_error);
    vm.c_closed_queue_error = d("ClosedQueueError", vm.c_stop_iteration);
    vm.c_local_jump_error = d("LocalJumpError", se);
    vm.c_name_error = d("NameError", se);
    vm.c_no_method_error = d("NoMethodError", vm.c_name_error);
    vm.c_range_error = d("RangeError", se);
    vm.c_float_domain_error = d("FloatDomainError", vm.c_range_error);
    vm.c_regexp_error = d("RegexpError", se);
    vm.c_runtime_error = d("RuntimeError", se);
    vm.c_frozen_error = d("FrozenError", vm.c_runtime_error);
    vm.c_system_call_error = d("SystemCallError", se);
    vm.c_thread_error = d("ThreadError", se);
    vm.c_type_error = d("TypeError", se);
    vm.c_zero_div_error = d("ZeroDivisionError", se);
    vm.c_no_matching_pattern_error = d("NoMatchingPatternError", se);
    vm.c_no_matching_pattern_key_error = d("NoMatchingPatternKeyError", vm.c_no_matching_pattern_error);
    let m = define_module("Math");
    vm.m_math = m;
    vm.c_math_domain_error = define_class_under(m, "DomainError", vm.c_argument_error);
    let enc = define_class("Encoding", ob);
    vm.c_encoding = enc;
    vm.c_compat_error = define_class_under(enc, "CompatibilityError", vm.c_encoding_error);
    vm.c_invalid_byte_seq_error = define_class_under(enc, "InvalidByteSequenceError", vm.c_encoding_error);
    vm.c_undef_conv_error = define_class_under(enc, "UndefinedConversionError", vm.c_encoding_error);
    vm.c_converter_not_found_error = define_class_under(enc, "ConverterNotFoundError", vm.c_encoding_error);
    vm.m_errno = define_module("Errno");
    for (name, no, msg) in ERRNOS {
        let k = define_class_under(vm.m_errno, name, vm.c_system_call_error);
        const_set(k, intern(b"Errno"), Value::fix(*no));
        ivar_set_raw(k, intern(b"__errmsg"), str_new_frozen(msg.as_bytes()));
    }
}

pub const ERRNOS: &[(&str, i64, &str)] = &[
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

pub fn errno_class(no: i64) -> Value {
    for (name, n, _) in ERRNOS { if *n == no { return const_get_at(vm().m_errno, intern(name.as_bytes())).unwrap(); } }
    vm().c_system_call_error
}

pub fn raise_errno(c: &mut Ctx, no: i64, detail: &str) -> Esc {
    let k = errno_class(no);
    let base = ivar_get(k, intern(b"__errmsg"));
    let base = if base == NIL { "Unknown error".to_string() } else { String::from_utf8_lossy(str_bytes(base)).into_owned() };
    let msg = if detail.is_empty() { base } else { format!("{} @ {}", base, detail) };
    let e = make_exception(k, &msg);
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
    if is_string(m) { Ok(String::from_utf8_lossy(str_bytes(m)).into_owned()) } else { to_rstring(c, m) }
}
fn exc_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let m = ivar_get(slf, s::mesg);
    if m == NIL { return Ok(str_new(class_path(real_class_of(slf)).as_bytes())); }
    if is_string(m) { return Ok(m); }
    to_s_value(c, m)
}
fn exc_message(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { funcall(c, slf, s::to_s, &[], std::ptr::null()) }
fn exc_detailed_message(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let m = exc_message_str(c, slf)?;
    let highlight = match kw_arg(a) { Some(h) => crate::hash::hash_get_opt(c, h, sym_value("highlight"))?.map(|v| v.truthy()).unwrap_or(false), None => false };
    let cls = class_path(real_class_of(slf));
    if m.is_empty() { return Ok(str_new(if highlight { format!("\x1b[1;4m{}\x1b[m", cls) } else { cls }.as_bytes())); }
    let mut lines = m.splitn(2, '\n');
    let first = lines.next().unwrap_or("");
    let rest = lines.next();
    let mut s = if highlight { format!("\x1b[1m{} (\x1b[1;4m{}\x1b[m\x1b[1m)\x1b[m", first, cls) } else { format!("{} ({})", first, cls) };
    if let Some(r) = rest { s.push('\n'); s.push_str(&if highlight { format!("\x1b[1m{}\x1b[m", r) } else { r.to_string() }); }
    Ok(str_new(s.as_bytes()))
}
fn exc_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let cls = class_path(real_class_of(slf));
    let m = exc_message_str(c, slf)?;
    if m.is_empty() { return Ok(str_new(cls.as_bytes())); }
    if m.contains('\n') { return Ok(str_new(format!("#<{}: {}>", cls, inspect_bytes(m.as_bytes(), ENC_UTF8)).as_bytes())); }
    Ok(str_new(format!("#<{}: {}>", cls, m).as_bytes()))
}
fn exc_backtrace(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(slf, s::bt)) }
fn exc_set_backtrace(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let v = a[0];
    let bt = if is_string(v) { ary_new_from(&[v]) } else if v == NIL || is_array(v) { v } else { return type_err(c, "backtrace must be an Array of String or an Array of Thread::Backtrace::Location"); };
    ivar_set_raw(slf, s::bt, bt);
    Ok(bt)
}
fn exc_backtrace_locations(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let bt = ivar_get(slf, s::bt);
    if bt == NIL { return Ok(NIL); }
    Ok(bt)
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
    let b1 = ivar_get(slf, s::bt); let b2 = ivar_get(o, s::bt);
    let e2 = funcall(c, b1, s::eq, &[b2], std::ptr::null())?;
    Ok(Value::bool(e2.truthy()))
}

pub fn full_message(c: &mut Ctx, e: Value, _highlight: bool) -> Result<String, Esc> {
    let bt = ivar_get(e, s::bt);
    let mut lines: Vec<String> = vec![];
    if is_array(bt) { for i in 0..ary_len(bt) { let l = ary_get(bt, i); if is_string(l) { lines.push(String::from_utf8_lossy(str_bytes(l)).into_owned()); } } }
    let dm = funcall(c, e, s::detailed_message, &[], std::ptr::null())?;
    let msg = if is_string(dm) { String::from_utf8_lossy(str_bytes(dm)).into_owned() } else { exc_message_str(c, e)? };
    let mut out = String::new();
    let first = lines.first().cloned().unwrap_or_else(|| "-e:1:in '<main>'".into());
    if lines.is_empty() { out.push_str(&format!("{}: {}\n", vm().main_file, msg)); }
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
    Ok(str_new(s.as_bytes()))
}
fn exc_s_to_tty(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(FALSE) }

// NameError / NoMethodError / KeyError / FrozenError / StopIteration / SystemExit / etc.
fn name_err_init(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, kw) = match kw_arg(a) { Some(h) => (&a[..a.len() - 1], Some(h)), None => (a, None) };
    ivar_set_raw(slf, s::mesg, pos.first().copied().unwrap_or(NIL));
    ivar_set_raw(slf, s::name, pos.get(1).copied().unwrap_or(NIL));
    if real_class_of(slf) == vm().c_no_method_error || is_kind_of(slf, vm().c_no_method_error) {
        ivar_set_raw(slf, intern(b"args"), pos.get(2).copied().unwrap_or_else(ary_new));
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
fn nm_err_args(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let v = ivar_get(slf, intern(b"args")); Ok(if v == NIL { ary_new() } else { v }) }
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
fn sysexit_init(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mut st = 0; let mut msg = str_new(b"exit");
    let mut i = 0;
    if let Some(&v) = a.first() {
        if v == TRUE { st = 0; i = 1; } else if v == FALSE { st = 1; i = 1; } else if v.is_fixnum() { st = v.as_fix(); i = 1; }
    }
    if let Some(&m) = a.get(i) { msg = m; }
    let _ = c;
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
    for &kk in ancestors(k).iter() { if let Some(v) = const_get_at(kk, intern(b"Errno")) { no = v; break; } }
    let base = { let mut m = NIL; for &kk in ancestors(k).iter() { let v = ivar_get(kk, intern(b"__errmsg")); if v != NIL { m = v; break; } } m };
    let mut msg = if base != NIL { String::from_utf8_lossy(str_bytes(base)).into_owned() } else { "unknown error".into() };
    if k == vm().c_system_call_error && a.len() >= 2 && a[1].is_fixnum() { no = a[1]; let ek = errno_class(no.as_fix()); let v = ivar_get(ek, intern(b"__errmsg")); if v != NIL { msg = String::from_utf8_lossy(str_bytes(v)).into_owned(); } }
    if let Some(&d) = a.first() { if d != NIL { let ds = to_rstring(c, d)?; msg = format!("{} - {}", msg, ds); } }
    ivar_set_raw(slf, s::mesg, str_new(msg.as_bytes()));
    ivar_set_raw(slf, s::errno, no);
    Ok(NIL)
}
fn syscallerr_errno(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(slf, s::errno)) }

pub fn init() {
    let vm = vm();
    let e = vm.c_exception;
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
    let ne = vm.c_name_error;
    define_private_cfunc(ne, "initialize", name_err_init, -1);
    define_cfunc(ne, "name", name_err_name, 0);
    define_cfunc(ne, "receiver", name_err_receiver, 0);
    define_cfunc(vm.c_no_method_error, "args", nm_err_args, 0);
    define_private_cfunc(vm.c_key_error, "initialize", key_err_init, -1);
    define_cfunc(vm.c_key_error, "key", key_err_key, 0);
    define_cfunc(vm.c_key_error, "receiver", name_err_receiver, 0);
    define_private_cfunc(vm.c_frozen_error, "initialize", frozen_err_init, -1);
    define_cfunc(vm.c_frozen_error, "receiver", name_err_receiver, 0);
    define_cfunc(vm.c_no_matching_pattern_key_error, "key", key_err_key, 0);
    define_cfunc(vm.c_no_matching_pattern_key_error, "matchee", name_err_receiver, 0);
    define_cfunc(vm.c_stop_iteration, "result", stop_iter_result, 0);
    define_private_cfunc(vm.c_system_exit, "initialize", sysexit_init, -1);
    define_cfunc(vm.c_system_exit, "status", sysexit_status, 0);
    define_cfunc(vm.c_system_exit, "success?", sysexit_success, 0);
    define_cfunc(vm.c_uncaught_throw_error, "tag", uncaught_tag, 0);
    define_cfunc(vm.c_uncaught_throw_error, "value", uncaught_value, 0);
    define_private_cfunc(vm.c_system_call_error, "initialize", syscallerr_init, -1);
    define_cfunc(vm.c_system_call_error, "errno", syscallerr_errno, 0);
    define_cfunc(vm.c_local_jump_error, "reason", |_c, _s, _a, _b| Ok(sym_value("break")), 0);
    define_cfunc(vm.c_local_jump_error, "exit_value", |_c, _s, _a, _b| Ok(NIL), 0);
    let _ = sym_str;
}
