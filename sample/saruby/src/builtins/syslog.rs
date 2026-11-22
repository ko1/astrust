// syslog 0.3.0 (syslog_ext.so): the libc calls; the module itself is prelude/lib/syslog_ext.rb.
#![forbid(unsafe_code)]
use crate::builtins::numeric_methods::num_to_long;
use crate::builtins::sys;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

fn cstr(c: &mut Ctx, v: Value, check_nul: bool) -> Result<std::ffi::CString, Esc> {
    let s = expect_str(c, v)?;
    let b = c.str_bytes(s);
    let b = match b.iter().position(|&x| x == 0) {
        Some(_) if check_nul => return arg_err(c, "string contains null byte"),
        Some(i) => b[..i].to_vec(),
        None => b.to_vec(),
    };
    Ok(std::ffi::CString::new(b).expect("no NUL"))
}
fn s_openlog(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let id = cstr(c, a[0], true)?;
    let (o, f) = (num_to_long(c, a[1])? as i32, num_to_long(c, a[2])? as i32);
    sys::openlog(id, o, f);
    Ok(Value::fix(sys::setlogmask(0) as i64))
}
fn s_syslog(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let p = num_to_long(c, a[0])? as i32;
    let m = cstr(c, a[1], false)?;
    sys::syslog(p, &m);
    Ok(NIL)
}
fn s_closelog(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { sys::closelog(); Ok(NIL) }
fn s_setlogmask(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let m = num_to_long(c, a[0])? as i32; Ok(Value::fix(sys::setlogmask(m) as i64)) }
pub fn init(c: &mut Ctx) {
    let ob = c.vm().c_object;
    let m = const_get_at(ob, intern(b"Syslog")).unwrap_or_else(|| define_module("Syslog"));
    define_singleton_cfunc(m, "__openlog", s_openlog, 3);
    define_singleton_cfunc(m, "__syslog", s_syslog, 2);
    define_singleton_cfunc(m, "__closelog", s_closelog, 0);
    define_singleton_cfunc(m, "__setlogmask", s_setlogmask, 1);
}
