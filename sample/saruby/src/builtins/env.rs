// ENV: the process environment as a Hash-like object.
use crate::array::*;
use crate::builtins::exception::raise_errno_plain;
use crate::builtins::sys;
use crate::hash::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::os::unix::ffi::{OsStrExt, OsStringExt};

fn name_arg(c: &mut Ctx, v: Value, what: &str) -> Result<Vec<u8>, Esc> {
    let s = expect_str(c, v)?;
    let b = str_bytes(s).to_vec();
    if b.contains(&0) { return arg_err(c, format!("bad environment variable {}: contains null byte", what)); }
    Ok(b)
}
fn get(name: &[u8]) -> Option<Vec<u8>> { if name.is_empty() || name.contains(&b'=') { return None; } std::env::var_os(std::ffi::OsStr::from_bytes(name)).map(|v| v.into_vec()) }
fn env_str(b: &[u8]) -> Value { let s = str_new_enc(b, crate::builtins::encoding::default_external()); obj(s).flags |= FL_FROZEN; s }
fn set(c: &mut Ctx, name: &[u8], val: Option<&[u8]>) -> Result<(), Esc> {
    if name.is_empty() || name.contains(&b'=') {
        if val.is_none() { return Ok(()); }
        return Err(raise_errno_plain(c, 22, &format!("setenv({})", String::from_utf8_lossy(name))));
    }
    match val { Some(v) => { sys::setenv(&sys::cstr(name), &sys::cstr(v)); } None => { sys::unsetenv(&sys::cstr(name)); } }
    Ok(())
}
fn pairs() -> Vec<(Vec<u8>, Vec<u8>)> { std::env::vars_os().map(|(k, v)| (k.into_vec(), v.into_vec())).collect() }
fn to_hash(c: &mut Ctx) -> RV {
    let h = hash_new();
    for (k, v) in pairs() { hash_set(c, h, env_str(&k), env_str(&v))?; }
    Ok(h)
}
fn env_aref(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let k = name_arg(c, a[0], "name")?; Ok(get(&k).map(|v| env_str(&v)).unwrap_or(NIL)) }
fn env_fetch(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let k = name_arg(c, a[0], "name")?;
    if let Some(v) = get(&k) { return Ok(env_str(&v)); }
    if !b.is_null() { return yield1(c, b, a[0]); }
    if a.len() > 1 { return Ok(a[1]); }
    let e = make_exception(vm().c_key_error, &format!("key not found: {}", inspect_bytes(&k, ENC_UTF8)));
    ivar_set_raw(e, intern(b"receiver"), s); ivar_set_raw(e, intern(b"key"), a[0]);
    Err(raise_exc(c, e))
}
fn env_aset(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let k = name_arg(c, a[0], "name")?;
    if a[1] == NIL { set(c, &k, None)?; return Ok(NIL); }
    let v = name_arg(c, a[1], "value")?;
    set(c, &k, Some(&v))?;
    Ok(a[1])
}
fn env_delete(c: &mut Ctx, _s: Value, a: &[Value], b: *const Block) -> RV {
    let k = name_arg(c, a[0], "name")?;
    match get(&k) {
        Some(v) => { set(c, &k, None)?; Ok(env_str(&v)) }
        None => if b.is_null() { Ok(NIL) } else { yield1(c, b, a[0]) },
    }
}
fn env_key_p(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let k = name_arg(c, a[0], "name")?; Ok(Value::bool(get(&k).is_some())) }
fn env_value_p(_c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { if !is_string(a[0]) { return Ok(NIL); } let v = str_bytes(a[0]); Ok(Value::bool(pairs().iter().any(|p| p.1 == v))) }
fn env_key(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let v = name_arg(c, a[0], "value")?; Ok(pairs().into_iter().find(|p| p.1 == v).map(|p| env_str(&p.0)).unwrap_or(NIL)) }
fn env_keys(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { let out = ary_new(); for (k, _) in pairs() { let s = env_str(&k); ary_push(out, s); } Ok(out) }
fn env_values(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { let out = ary_new(); for (_, v) in pairs() { let s = env_str(&v); ary_push(out, s); } Ok(out) }
fn env_to_h(c: &mut Ctx, _s: Value, _a: &[Value], b: *const Block) -> RV { let h = to_hash(c)?; if b.is_null() { Ok(h) } else { funcall(c, h, intern(b"to_h"), &[], b) } }
fn env_size(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(pairs().len() as i64)) }
fn env_empty(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(pairs().is_empty())) }
fn env_clear(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { sys::clearenv(); Ok(s) }
fn env_inspect(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { let h = to_hash(c)?; crate::builtins::hash_methods::hash_inspect(c, h) }
fn env_to_s(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(str_new(b"ENV")) }
/// Iterators and queries answered by a snapshot Hash (the method keeps its own name).
fn env_delegate(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV {
    let name = unsafe { (*c.frame().me).name };
    if b.is_null() && matches!(crate::symbol::sym_bytes(name), b"each" | b"each_pair" | b"each_key" | b"each_value" | b"select" | b"filter" | b"reject" | b"delete_if" | b"keep_if" | b"select!" | b"filter!" | b"reject!" | b"filter_map") {
        return crate::builtins::enumerator::enum_for(c, s, name, a, NIL);
    }
    let h = to_hash(c)?;
    let r = funcall(c, h, name, a, b)?;
    Ok(if r == h { s } else { r })
}
/// Write a Hash back as the whole environment.
fn sync_from(c: &mut Ctx, h: Value) -> Result<(), Esc> {
    let want = hash_pairs(h);
    for (k, _) in pairs() { if !want.iter().any(|(wk, _)| is_string(*wk) && str_bytes(*wk) == &k[..]) { set(c, &k, None)?; } }
    for (k, v) in want { let kb = name_arg(c, k, "name")?; let vb = name_arg(c, v, "value")?; set(c, &kb, Some(&vb))?; }
    Ok(())
}
fn env_mutate(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV {
    let name = unsafe { (*c.frame().me).name };
    if b.is_null() && !matches!(crate::symbol::sym_bytes(name), b"update" | b"merge!" | b"replace") { return crate::builtins::enumerator::enum_for(c, s, name, a, NIL); }
    let h = to_hash(c)?;
    let r = funcall(c, h, name, a, b)?;
    sync_from(c, h)?;
    Ok(if r == h { s } else { r })
}
fn env_freeze(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { type_err(c, "cannot freeze ENV") }
fn env_dup(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { type_err(c, "Cannot dup ENV, use ENV.to_h to get a copy of ENV as a hash") }
fn env_clone(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { type_err(c, "Cannot clone ENV, use ENV.to_h to get a copy of ENV as a hash") }

pub fn init() {
    let vm = vm();
    let env = new_object(vm.c_object);
    include_module(singleton_class(env), vm.m_enumerable);
    let d = |n: &str, f: CFunc, a: i32| define_singleton_cfunc(env, n, f, a);
    d("[]", env_aref, 1); d("fetch", env_fetch, -1); d("[]=", env_aset, 2); d("store", env_aset, 2); d("delete", env_delete, 1);
    for n in ["key?", "has_key?", "include?", "member?"] { d(n, env_key_p, 1); }
    for n in ["value?", "has_value?"] { d(n, env_value_p, 1); }
    d("key", env_key, 1); d("keys", env_keys, 0); d("values", env_values, 0); d("to_h", env_to_h, 0); d("to_hash", env_to_h, 0);
    d("size", env_size, 0); d("length", env_size, 0); d("empty?", env_empty, 0); d("clear", env_clear, 0); d("inspect", env_inspect, 0); d("to_s", env_to_s, 0);
    for n in ["each", "each_pair", "each_key", "each_value", "select", "filter", "reject", "slice", "except", "values_at", "assoc", "rassoc", "invert", "to_a", "filter_map"] { d(n, env_delegate, -1); }
    for n in ["update", "merge!", "replace", "delete_if", "reject!", "keep_if", "select!", "filter!", "shift"] { d(n, env_mutate, -1); }
    d("freeze", env_freeze, 0); d("dup", env_dup, 0); d("clone", env_clone, -1); d("rehash", |_c, _s, _a, _b| Ok(NIL), 0);
    const_set(vm.c_object, intern(b"ENV"), env);
}
