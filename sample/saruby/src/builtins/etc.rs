// The libc part of ext/etc: Etc.sysconf, Etc.confstr, IO#pathconf (the rest is prelude/lib/etc.rb).
#![forbid(unsafe_code)]
use crate::array::*;
use crate::builtins::kernel::class_name_for_err;
use crate::builtins::numeric_methods::num_to_long;
use crate::builtins::sys;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

/// NUM2INT
fn int_arg(c: &mut Ctx, v: Value) -> Result<i32, Esc> {
    if !v.is_fixnum() && !(v.is_heap() && (matches!(c.body_of(v), Body::Bignum(_) | Body::Float(_)) || respond_to(v, intern(b"to_int"), true))) {
        return type_err(c, format!("no implicit conversion of {} into Integer", class_name_for_err(v)));
    }
    Ok(num_to_long(c, v)? as i32)
}
/// rb_sys_fail(func)
fn sys_fail(c: &mut Ctx, func: &str) -> Esc {
    let no = sys::errno();
    let k = crate::builtins::exception::errno_class(c.vm(), no);
    let e = make_exception(k, &format!("{} - {}", crate::builtins::exception::errno_text(Value::fix(no)), func));
    ivar_set_raw(e, crate::symbol::s::errno, Value::fix(no));
    raise_exc(c, e)
}
fn etc_sysconf(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = int_arg(c, a[0])?;
    sys::set_errno(0);
    let r = sys::sysconf(n);
    if r == -1 { if sys::errno() == 0 { return Ok(NIL); } return Err(sys_fail(c, "sysconf")); }
    Ok(Value::fix(r))
}
fn etc_confstr(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = int_arg(c, a[0])?;
    let mut buf = vec![0u8; 512];
    sys::set_errno(0);
    let mut r = sys::confstr(n, &mut buf);
    if r == 0 { if sys::errno() == 0 { return Ok(NIL); } return Err(sys_fail(c, "confstr")); }
    if r > buf.len() { buf = vec![0u8; r]; r = sys::confstr(n, &mut buf); if r == 0 { return Err(sys_fail(c, "confstr")); } }
    buf.truncate(r - 1);
    Ok(c.str_new_enc(&buf, ENC_BINARY))
}
fn io_pathconf(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = int_arg(c, a[0])?;
    let fd = funcall(c, slf, intern(b"fileno"), &[], std::ptr::null())?;
    let fd = num_to_long(c, fd)? as i32;
    sys::set_errno(0);
    let r = sys::fpathconf(fd, n);
    if r == -1 { if sys::errno() == 0 { return Ok(NIL); } return Err(sys_fail(c, "fpathconf")); }
    Ok(Value::fix(r))
}
pub fn init(c: &mut Ctx) {
    let ob = c.vm().c_object.get();
    let m = const_get_at(ob, intern(b"Etc")).unwrap_or_else(|| define_module("Etc"));
    define_module_function(m, "sysconf", etc_sysconf, 1);
    define_module_function(m, "confstr", etc_confstr, 1);
    let io = c.vm().c_io.get();
    define_cfunc(io, "pathconf", io_pathconf, 1);
}
