// ENV: the process environment as a Hash-like object.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::obj_set_flags;
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
    let b = c.str_bytes(s).to_vec();
    if b.contains(&0) { return arg_err(c, format!("bad environment variable {}: contains null byte", what)); }
    Ok(b)
}
fn get(name: &[u8]) -> Option<Vec<u8>> { if name.is_empty() || name.contains(&b'=') { return None; } std::env::var_os(std::ffi::OsStr::from_bytes(name)).map(|v| v.into_vec()) }
/// A name or value: in the locale encoding (UTF-8), converted to default_internal when set; frozen.
fn env_str(c: &mut Ctx, b: &[u8]) -> Value { let s = crate::builtins::encoding::external_str(c, b.to_vec(), crate::builtins::encoding::locale_enc()); obj_set_flags(s, FL_FROZEN); s }
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
    let h = c.hash_new();
    for (k, v) in pairs() { let ks = env_str(c, &k); let vs = env_str(c, &v); hash_set(c, h, ks, vs)?; }
    Ok(h)
}
fn env_aref(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { let k = name_arg(c, a[0], "name")?; Ok(match get(&k) { Some(v) => env_str(c, &v), None => NIL }) }
fn env_fetch(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let k = name_arg(c, a[0], "name")?;
    if !b.is_null() && a.len() > 1 { crate::builtins::kernel::gvar_warn(c, "block supersedes default value argument", false)?; }
    if let Some(v) = get(&k) { return Ok(env_str(c, &v)); }
    if !b.is_null() { return yield1(c, b, a[0]); }
    if a.len() > 1 { return Ok(a[1]); }
    let e = make_exception(c, c.vm().c_key_error.get(), &format!("key not found: {}", inspect_bytes(&k, ENC_UTF8)));
    ivar_set_raw(c, e, intern(b"receiver"), s); ivar_set_raw(c, e, intern(b"key"), a[0]);
    Err(raise_exc(c, e))
}
fn env_aset(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let k = name_arg(c, a[0], "name")?;
    if a[1] == NIL { set(c, &k, None)?; return Ok(NIL); }
    let v = name_arg(c, a[1], "value")?;
    set(c, &k, Some(&v))?;
    Ok(a[1])
}
fn env_delete(c: &mut Ctx, _s: Value, a: &[Value], b: Blk) -> RV {
    let k = name_arg(c, a[0], "name")?;
    match get(&k) {
        Some(v) => { set(c, &k, None)?; Ok(env_str(c, &v)) }
        None => if b.is_null() { Ok(NIL) } else { yield1(c, b, a[0]) },
    }
}
fn env_key_p(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { let k = name_arg(c, a[0], "name")?; Ok(Value::bool(get(&k).is_some())) }
/// rb_check_string_type: the String (to_str), or None.
pub fn check_str(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if is_string(v) { return Ok(Some(v)); }
    if !v.is_heap() || !respond_to(c, v, intern(b"to_str"), true) { return Ok(None); }
    let s = funcall(c, v, intern(b"to_str"), &[], std::ptr::null())?;
    if s == NIL { return Ok(None); }
    if !is_string(s) { let k = crate::builtins::kernel::class_name_for_err(c, v); return type_err(c, format!("can't convert {} to String ({}#to_str gives {})", k, k, class_path(c, real_class_of(c, s)))); }
    Ok(Some(s))
}
fn env_value_p(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let Some(v) = check_str(c, a[0])? else { return Ok(NIL) };
    let v = c.str_bytes(v).to_vec();
    Ok(Value::bool(pairs().iter().any(|p| p.1 == v)))
}
fn env_key(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { let v = name_arg(c, a[0], "value")?; Ok(match pairs().into_iter().find(|p| p.1 == v) { Some(p) => env_str(c, &p.0), None => NIL }) }
fn env_keys(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { let out = c.ary_new(); for (k, _) in pairs() { let s = env_str(c, &k); c.ary_push(out, s); } Ok(out) }
fn env_values(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { let out = c.ary_new(); for (_, v) in pairs() { let s = env_str(c, &v); c.ary_push(out, s); } Ok(out) }
fn pair_ary(c: &mut Ctx, k: &[u8], v: &[u8]) -> Value { let (ks, vs) = (env_str(c, k), env_str(c, v)); c.ary_new_from(&[ks, vs]) }
/// ENV.to_h: with a block, each [key, value] it returns (the block gets two values).
fn env_to_h(c: &mut Ctx, _s: Value, _a: &[Value], b: Blk) -> RV {
    if b.is_null() { return to_hash(c); }
    let h = c.hash_new();
    for (k, v) in pairs() {
        let (ks, vs) = (env_str(c, &k), env_str(c, &v));
        let r = yield_block(c, b, &[ks, vs])?;
        let Some(r) = crate::array::check_array(c, r)? else { let n = crate::builtins::kernel::class_name_for_err(c, r); return type_err(c, format!("wrong element type {} (expected array)", n)); };
        if c.ary_len(r) != 2 { return arg_err(c, format!("element has wrong array length (expected 2, was {})", c.ary_len(r))); }
        let (x, y) = (c.ary_get(r, 0), c.ary_get(r, 1));
        hash_aset(c, h, x, y)?;
    }
    Ok(h)
}
fn env_size(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(pairs().len() as i64)) }
fn env_empty(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(pairs().is_empty())) }
fn env_clear(_c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { sys::clearenv(); Ok(s) }
fn env_inspect(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { let h = to_hash(c)?; crate::builtins::hash_methods::hash_inspect(c, h) }
fn env_to_s(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.str_new(b"ENV")) }
fn sized_enum(c: &mut Ctx, s: Value, a: &[Value]) -> RV {
    let Some(name) = c.cur_method_name() else { return Ok(NIL) };
    crate::builtins::enumerator::enum_for_sized(c, s, name, a)
}
fn env_each_pair(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    if b.is_null() { return sized_enum(c, s, a); }
    for (k, v) in pairs() { let p = pair_ary(c, &k, &v); yield1(c, b, p)?; }
    Ok(s)
}
fn env_each_key(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    if b.is_null() { return sized_enum(c, s, a); }
    for (k, _) in pairs() { let x = env_str(c, &k); yield1(c, b, x)?; }
    Ok(s)
}
fn env_each_value(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    if b.is_null() { return sized_enum(c, s, a); }
    for (_, v) in pairs() { let x = env_str(c, &v); yield1(c, b, x)?; }
    Ok(s)
}
/// select / filter (keep = true) and reject: a Hash of the pairs the block (key, value) decides.
fn env_select_impl(c: &mut Ctx, s: Value, a: &[Value], b: Blk, keep: bool) -> RV {
    if b.is_null() { return sized_enum(c, s, a); }
    let h = c.hash_new();
    for (k, v) in pairs() {
        let (ks, vs) = (env_str(c, &k), env_str(c, &v));
        if yield_block(c, b, &[ks, vs])?.truthy() == keep { hash_aset(c, h, ks, vs)?; }
    }
    Ok(h)
}
fn env_select(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { env_select_impl(c, s, a, b, true) }
fn env_reject(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { env_select_impl(c, s, a, b, false) }
/// select! / keep_if (keep = true), reject! / delete_if: delete the others; `bang` answers nil when nothing changed.
fn env_filter_bang(c: &mut Ctx, s: Value, a: &[Value], b: Blk, keep: bool, bang: bool) -> RV {
    if b.is_null() { return sized_enum(c, s, a); }
    let mut changed = false;
    for (k, v) in pairs() {
        let (ks, vs) = (env_str(c, &k), env_str(c, &v));
        if yield_block(c, b, &[ks, vs])?.truthy() != keep { set(c, &k, None)?; changed = true; }
    }
    Ok(if bang && !changed { NIL } else { s })
}
fn env_select_bang(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { env_filter_bang(c, s, a, b, true, true) }
fn env_keep_if(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { env_filter_bang(c, s, a, b, true, false) }
fn env_reject_bang(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { env_filter_bang(c, s, a, b, false, true) }
fn env_delete_if(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { env_filter_bang(c, s, a, b, false, false) }
/// ENV.slice: the values of the names that exist, under the given key objects.
fn env_slice(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let h = c.hash_new();
    for &k in a {
        let n = name_arg(c, k, "name")?;
        if let Some(v) = get(&n) { let vs = env_str(c, &v); hash_aset(c, h, k, vs)?; }
    }
    Ok(h)
}
fn env_except(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let h = to_hash(c)?;
    for &k in a { hash_delete(c, h, k)?; }
    Ok(h)
}
fn env_values_at(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let out = c.ary_new();
    for &k in a { let n = name_arg(c, k, "name")?; let v = match get(&n) { Some(v) => env_str(c, &v), None => NIL }; c.ary_push(out, v); }
    Ok(out)
}
fn env_assoc(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let k = expect_str(c, a[0])?;
    let n = c.str_bytes(k).to_vec();
    Ok(match get(&n) { Some(v) => { let vs = env_str(c, &v); c.ary_new_from(&[k, vs]) } None => NIL })
}
fn env_rassoc(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let Some(v) = check_str(c, a[0])? else { return Ok(NIL) };
    let v = c.str_bytes(v).to_vec();
    Ok(match pairs().into_iter().find(|p| p.1 == v) { Some((k, v)) => pair_ary(c, &k, &v), None => NIL })
}
fn env_invert(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let h = c.hash_new();
    for (k, v) in pairs() { let (ks, vs) = (env_str(c, &k), env_str(c, &v)); hash_aset(c, h, vs, ks)?; }
    Ok(h)
}
fn env_to_a(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let out = c.ary_new();
    for (k, v) in pairs() { let p = pair_ary(c, &k, &v); c.ary_push(out, p); }
    Ok(out)
}
fn env_shift(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let Some((k, v)) = pairs().into_iter().next() else { return Ok(NIL) };
    let p = pair_ary(c, &k, &v);
    set(c, &k, None)?;
    Ok(p)
}
/// ENV.replace: every name and value is checked before the environment changes.
fn env_replace(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let Some(h) = check_hash(c, a[0])? else { return type_err(c, format!("no implicit conversion of {} into Hash", crate::builtins::kernel::class_name_for_err(c, a[0]))) };
    // env_replace_i: each pair is set as it comes (an error keeps the ones before it), the rest deleted after
    let old = pairs();
    let mut want = vec![];
    let mut i = 0;
    while let Some((k, v)) = c.hash_entry_at(h, i) {
        i += 1;
        if k == UNDEF { continue; }
        let kb = name_arg(c, k, "name")?; let vb = name_arg(c, v, "value")?;
        set(c, &kb, Some(&vb))?;
        want.push(kb);
    }
    for (k, _) in old { if !want.contains(&k) { set(c, &k, None)?; } }
    Ok(s)
}
/// ENV.update / merge!: like ENV[k] = v per pair (nil deletes), the block resolves existing names.
fn env_update(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    for &x in a {
        let Some(h) = check_hash(c, x)? else { return type_err(c, format!("no implicit conversion of {} into Hash", crate::builtins::kernel::class_name_for_err(c, x))) };
        let mut i = 0;
        while let Some((k, v)) = c.hash_entry_at(h, i) {
            i += 1;
            if k == UNDEF { continue; }
            let kb = name_arg(c, k, "name")?;
            let mut v = v;
            if !b.is_null() { if let Some(old) = get(&kb) { let o = env_str(c, &old); v = yield_block(c, b, &[k, o, v])?; } }
            if v == NIL { set(c, &kb, None)?; } else { let vb = name_arg(c, v, "value")?; set(c, &kb, Some(&vb))?; }
        }
    }
    Ok(s)
}
fn env_freeze(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { type_err(c, "cannot freeze ENV") }
fn env_dup(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { type_err(c, "Cannot dup ENV, use ENV.to_h to get a copy of ENV as a hash") }
fn env_clone(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    // rb_get_freeze_opt first
    if let Some(h) = kw_arg(c, a) { {
        let mut i = 0;
        while let Some((k, v)) = c.hash_entry_at(h, i) {
            i += 1;
            if k == UNDEF { continue; }
            if k != sym_value("freeze") { let d = crate::builtins::kernel::inspect_str(c, k)?; return arg_err(c, format!("unknown keyword: {}", d)); }
            if v != NIL && v != TRUE && v != FALSE { let n = crate::builtins::kernel::class_name_for_err(c, v); return arg_err(c, format!("unexpected value for freeze: {}", n)); }
        }
    } }
    type_err(c, "Cannot clone ENV, use ENV.to_h to get a copy of ENV as a hash")
}

pub fn init(c: &mut Ctx) {
    let env = new_object(c, c.vm().c_object.get());
    let se = singleton_class(c, env);
    include_module(c, se, c.vm().m_enumerable.get());
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_singleton_cfunc(c, env, n, f, a);
    d(c, "[]", env_aref, 1); d(c, "fetch", env_fetch, -1); d(c, "[]=", env_aset, 2); d(c, "store", env_aset, 2); d(c, "delete", env_delete, 1);
    for n in ["key?", "has_key?", "include?", "member?"] { d(c, n, env_key_p, 1); }
    for n in ["value?", "has_value?"] { d(c, n, env_value_p, 1); }
    d(c, "key", env_key, 1); d(c, "keys", env_keys, 0); d(c, "values", env_values, 0); d(c, "to_h", env_to_h, 0); d(c, "to_hash", env_to_h, 0);
    d(c, "size", env_size, 0); d(c, "length", env_size, 0); d(c, "empty?", env_empty, 0); d(c, "clear", env_clear, 0); d(c, "inspect", env_inspect, 0); d(c, "to_s", env_to_s, 0);
    d(c, "each", env_each_pair, 0); d(c, "each_pair", env_each_pair, 0); d(c, "each_key", env_each_key, 0); d(c, "each_value", env_each_value, 0);
    d(c, "select", env_select, 0); d(c, "filter", env_select, 0); d(c, "reject", env_reject, 0);
    d(c, "select!", env_select_bang, 0); d(c, "filter!", env_select_bang, 0); d(c, "keep_if", env_keep_if, 0); d(c, "reject!", env_reject_bang, 0); d(c, "delete_if", env_delete_if, 0);
    d(c, "slice", env_slice, -1); d(c, "except", env_except, -1); d(c, "values_at", env_values_at, -1); d(c, "assoc", env_assoc, 1); d(c, "rassoc", env_rassoc, 1);
    d(c, "invert", env_invert, 0); d(c, "to_a", env_to_a, 0); d(c, "shift", env_shift, 0); d(c, "replace", env_replace, 1);
    d(c, "update", env_update, -1); d(c, "merge!", env_update, -1);
    d(c, "freeze", env_freeze, 0); d(c, "dup", env_dup, 0); d(c, "clone", env_clone, -1); d(c, "rehash", |_c, _s, _a, _b| Ok(NIL), 0);
    obj_set_flags(env, crate::object::FL_SHAREABLE);   // as CRuby's (Ractor.shareable?)
    const_set(c, c.vm().c_object.get(), intern(b"ENV"), env);
}
