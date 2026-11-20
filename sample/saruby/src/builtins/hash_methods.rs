// Hash methods, on the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::array::*;
use crate::builtins::kernel::*;
use crate::hash::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

macro_rules! cfn { ($name:ident, |$c:ident, $s:ident, $a:ident, $b:ident| $body:expr) => {
    #[allow(unused_variables)]
    pub fn $name($c: &mut Ctx, $s: Value, $a: &[Value], $b: *const Block) -> RV { $body }
}; }

fn modify(c: &mut Ctx, h: Value) -> Result<(), Esc> { if obj_flags(h) & FL_FROZEN != 0 { return Err(raise_frozen(c, h)); } Ok(()) }
fn enum_or<F: FnOnce(&mut Ctx) -> RV>(c: &mut Ctx, slf: Value, mid: &str, a: &[Value], blk: *const Block, f: F) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, intern(mid.as_bytes()), a); }
    f(c)
}
/// Iterate live entries by slot index (tolerates mutation during iteration).
fn each_entry(c: &mut Ctx, h: Value, mut f: impl FnMut(&mut Ctx, Value, Value) -> Result<bool, Esc>) -> Result<(), Esc> {
    c.hash_iter_enter(h);
    let mut i = 0;
    let mut r = Ok(());
    while let Some((k, v)) = c.hash_entry_at(h, i) {
        if k != UNDEF { match f(c, k, v) { Ok(true) => {}, Ok(false) => break, Err(e) => { r = Err(e); break; } } }
        i += 1;
    }
    c.hash_iter_leave(h);
    r
}
/// Insert the entries of `src` into `dst` (another hash), slot by slot: hashing a key may run Ruby code.
fn copy_entries(c: &mut Ctx, dst: Value, src: Value, dup_keys: bool) -> Result<(), Esc> {
    let mut i = 0;
    while let Some((k, v)) = c.hash_entry_at(src, i) {
        if k != UNDEF { if dup_keys { hash_aset(c, dst, k, v)?; } else { hash_set(c, dst, k, v)?; } }
        i += 1;
    }
    Ok(())
}
/// |k, v| blocks receive two args; |pair| receives [k, v]
fn wants_two(c: &Ctx, blk: *const Block) -> bool {
    matches!(c.block_params(blk), Some(p) if p.lead.len() + p.post.len() + p.opt.len() >= 2 || p.rest.is_some() && p.lead.len() >= 1)
}
fn yield_pair(c: &mut Ctx, blk: *const Block, k: Value, v: Value) -> RV { let two = wants_two(c, blk); yield_kv(c, blk, two, k, v) }
#[inline]
fn yield_kv(c: &mut Ctx, blk: *const Block, two: bool, k: Value, v: Value) -> RV {
    if two { return yield_block(c, blk, &[k, v]); }
    let p = c.ary_new_from(&[k, v]);
    yield_block(c, blk, &[p])
}

cfn!(h_initialize, |c, slf, a, blk| {
    modify(c, slf)?;
    let pos: Vec<Value> = a.iter().copied().filter(|x| kw_arg(a) != Some(*x)).collect();
    if !blk.is_null() {
        if !pos.is_empty() { return argc_err(c, pos.len(), 0, 0); }
        let p = block_to_proc(c, blk)?;
        c.hash_mut(slf).default_proc = p;
    } else if let Some(&d) = pos.first() { c.hash_mut(slf).default = d; }
    if pos.len() > 1 { return argc_err(c, pos.len(), 0, 1); }
    Ok(slf)
});
cfn!(h_s_aref, |c, slf, a, blk| {
    let h = c.hash_new(); obj_set_klass(h, slf);
    if a.len() == 1 {
        if is_hash(a[0]) { copy_entries(c, h, a[0], true)?; return Ok(h); }
        if let Some(arr) = check_array(c, a[0])? {
            for i in 0..c.ary_len(arr) {
                let p = c.ary_get(arr, i);
                let p = match check_array(c, p)? { Some(p) => p, None => return arg_err(c, format!("wrong element type {} at {} (expected array)", class_name_for_err(p), i)) };
                match c.ary(p) { &[k] => hash_aset(c, h, k, NIL)?, &[k, v] => hash_aset(c, h, k, v)?, x => { let n = x.len(); return arg_err(c, format!("invalid number of elements ({} for 1..2)", n)) } }
            }
            return Ok(h);
        }
    }
    if a.len() % 2 != 0 { return arg_err(c, "odd number of arguments for Hash"); }
    for i in (0..a.len()).step_by(2) { hash_aset(c, h, a[i], a[i + 1])?; }
    Ok(h)
});
cfn!(h_aref, |c, slf, a, blk| hash_aref(c, slf, a[0]));
cfn!(h_aset, |c, slf, a, blk| { modify(c, slf)?; hash_aset(c, slf, a[0], a[1])?; Ok(a[1]) });
cfn!(h_fetch, |c, slf, a, blk| {
    check_argc(c, a, 1, 2)?;
    match hash_get_opt(c, slf, a[0])? {
        Some(v) => Ok(v),
        None => {
            if !blk.is_null() { return yield1(c, blk, a[0]); }
            if a.len() > 1 { return Ok(a[1]); }
            let ins = inspect_str(c, a[0])?;
            let e = make_exception(c.vm().c_key_error, &format!("key not found: {}", ins));
            ivar_set_raw(e, s::key, a[0]); ivar_set_raw(e, s::receiver, slf);
            Err(raise_exc(c, e))
        }
    }
});
cfn!(h_key_p, |c, slf, a, blk| Ok(Value::bool(hash_lookup(c, slf, a[0])?.is_some())));
cfn!(h_value_p, |c, slf, a, blk| {
    let mut found = false;
    each_entry(c, slf, |c, _k, v| { if crate::builtins::array_methods::rb_eq(c, v, a[0])? { found = true; return Ok(false); } Ok(true) })?;
    Ok(Value::bool(found))
});
cfn!(h_key, |c, slf, a, blk| {
    let mut found = NIL;
    each_entry(c, slf, |c, k, v| { if crate::builtins::array_methods::rb_eq(c, v, a[0])? { found = k; return Ok(false); } Ok(true) })?;
    Ok(found)
});
cfn!(h_delete, |c, slf, a, blk| {
    modify(c, slf)?;
    match hash_delete(c, slf, a[0])? { Some(v) => Ok(v), None => if blk.is_null() { Ok(NIL) } else { yield1(c, blk, a[0]) } }
});
cfn!(h_size, |c, slf, a, blk| Ok(Value::fix(c.hash_len(slf) as i64)));
cfn!(h_empty, |c, slf, a, blk| Ok(Value::bool(c.hash_len(slf) == 0)));
cfn!(h_keys, |c, slf, a, blk| Ok(c.ary_from_vec(c.hash(slf).entries.iter().filter(|e| e.key != UNDEF).map(|e| e.key).collect())));
cfn!(h_values, |c, slf, a, blk| Ok(c.ary_from_vec(c.hash(slf).entries.iter().filter(|e| e.key != UNDEF).map(|e| e.val).collect())));
cfn!(h_values_at, |c, slf, a, blk| { let out = c.ary_new(); for &k in a { let v = hash_aref(c, slf, k)?; c.ary_push(out, v); } Ok(out) });
cfn!(h_fetch_values, |c, slf, a, blk| { let out = c.ary_new(); for &k in a { let v = h_fetch(c, slf, &[k], blk)?; c.ary_push(out, v); } Ok(out) });
cfn!(h_each, |c, slf, a, blk| enum_or(c, slf, "each", a, blk, |c| { let two = wants_two(c, blk); each_entry(c, slf, |c, k, v| { yield_kv(c, blk, two, k, v)?; Ok(true) })?; Ok(slf) }));
cfn!(h_each_key, |c, slf, a, blk| enum_or(c, slf, "each_key", a, blk, |c| { each_entry(c, slf, |c, k, _v| { yield1(c, blk, k)?; Ok(true) })?; Ok(slf) }));
cfn!(h_each_value, |c, slf, a, blk| enum_or(c, slf, "each_value", a, blk, |c| { each_entry(c, slf, |c, _k, v| { yield1(c, blk, v)?; Ok(true) })?; Ok(slf) }));
cfn!(h_each_with_index, |c, slf, a, blk| if crate::builtins::enum_native::each_overridden(c, slf, h_each) { crate::builtins::enum_native::call_enumerable(c, slf, "each_with_index", a, blk) } else { enum_or(c, slf, "each_with_index", a, blk, |c| {
    let mut i = 0;
    each_entry(c, slf, |c, k, v| { let p = c.ary_new_from(&[k, v]); yield_block(c, blk, &[p, Value::fix(i)])?; i += 1; Ok(true) })?; Ok(slf)
}) });
cfn!(h_map, |c, slf, a, blk| if crate::builtins::enum_native::each_overridden(c, slf, h_each) { crate::builtins::enum_native::call_enumerable(c, slf, "map", a, blk) } else { enum_or(c, slf, "map", a, blk, |c| { let out = c.ary_new(); let two = wants_two(c, blk); each_entry(c, slf, |c, k, v| { let r = yield_kv(c, blk, two, k, v)?; c.ary_push(out, r); Ok(true) })?; Ok(out) }) });
cfn!(h_flat_map, |c, slf, a, blk| if crate::builtins::enum_native::each_overridden(c, slf, h_each) { crate::builtins::enum_native::call_enumerable(c, slf, "flat_map", a, blk) } else { enum_or(c, slf, "flat_map", a, blk, |c| { let out = c.ary_new(); each_entry(c, slf, |c, k, v| { let r = yield_pair(c, blk, k, v)?; if is_array(r) { c.ary_concat(out, r); } else { c.ary_push(out, r); } Ok(true) })?; Ok(out) }) });
fn sel(c: &mut Ctx, slf: Value, blk: *const Block, want: bool) -> RV {
    let out = c.hash_new();
    each_entry(c, slf, |c, k, v| { if yield_block(c, blk, &[k, v])?.truthy() == want { hash_set(c, out, k, v)?; } Ok(true) })?;
    Ok(out)
}
cfn!(h_select, |c, slf, a, blk| enum_or(c, slf, "select", a, blk, |c| sel(c, slf, blk, true)));
cfn!(h_reject, |c, slf, a, blk| enum_or(c, slf, "reject", a, blk, |c| sel(c, slf, blk, false)));
fn sel_b(c: &mut Ctx, slf: Value, blk: *const Block, want: bool) -> Result<bool, Esc> {
    modify(c, slf)?;
    let mut changed = false;
    let mut i = 0;
    c.hash_iter_enter(slf);
    let r: Result<(), Esc> = (|| {
        while i < c.hash_slots(slf) {
            if let Some((k, v)) = c.hash_entry_at(slf, i) {
                if k != UNDEF && yield_block(c, blk, &[k, v])?.truthy() != want {
                    modify(c, slf)?;
                    if c.hash_entry_at(slf, i).map(|e| e.0) == Some(k) { c.hash_delete_slot(slf, i); changed = true; }
                }
            }
            i += 1;
        }
        Ok(())
    })();
    c.hash_iter_leave(slf);
    r?;
    Ok(changed)
}
cfn!(h_select_b, |c, slf, a, blk| enum_or(c, slf, "select!", a, blk, |c| Ok(if sel_b(c, slf, blk, true)? { slf } else { NIL })));
cfn!(h_keep_if, |c, slf, a, blk| enum_or(c, slf, "keep_if", a, blk, |c| { sel_b(c, slf, blk, true)?; Ok(slf) }));
cfn!(h_reject_b, |c, slf, a, blk| enum_or(c, slf, "reject!", a, blk, |c| Ok(if sel_b(c, slf, blk, false)? { slf } else { NIL })));
cfn!(h_delete_if, |c, slf, a, blk| enum_or(c, slf, "delete_if", a, blk, |c| { sel_b(c, slf, blk, false)?; Ok(slf) }));
cfn!(h_filter_map, |c, slf, a, blk| if crate::builtins::enum_native::each_overridden(c, slf, h_each) { crate::builtins::enum_native::call_enumerable(c, slf, "filter_map", a, blk) } else { enum_or(c, slf, "filter_map", a, blk, |c| { let out = c.ary_new(); each_entry(c, slf, |c, k, v| { let r = yield_pair(c, blk, k, v)?; if r.truthy() { c.ary_push(out, r); } Ok(true) })?; Ok(out) }) });
cfn!(h_find, |c, slf, a, blk| enum_or(c, slf, "find", a, blk, |c| { let mut res = NIL; each_entry(c, slf, |c, k, v| { if yield_pair(c, blk, k, v)?.truthy() { res = c.ary_new_from(&[k, v]); return Ok(false); } Ok(true) })?; Ok(res) }));
cfn!(h_any, |c, slf, a, blk| {
    if blk.is_null() && a.is_empty() { return Ok(Value::bool(c.hash_len(slf) > 0)); }
    let mut res = false;
    each_entry(c, slf, |c, k, v| { let t = if !a.is_empty() { let p = c.ary_new_from(&[k, v]); crate::rt::srb_eqq(c, a[0], p, &mut CallCache::default())? } else { yield_pair(c, blk, k, v)?.truthy() }; if t { res = true; return Ok(false); } Ok(true) })?;
    Ok(Value::bool(res))
});
cfn!(h_all, |c, slf, a, blk| {
    let mut res = true;
    each_entry(c, slf, |c, k, v| { let t = if blk.is_null() { true } else { yield_pair(c, blk, k, v)?.truthy() }; if !t { res = false; return Ok(false); } Ok(true) })?;
    Ok(Value::bool(res))
});
cfn!(h_count, |c, slf, a, blk| {
    if blk.is_null() && a.is_empty() { return Ok(Value::fix(c.hash_len(slf) as i64)); }
    let mut n = 0;
    each_entry(c, slf, |c, k, v| { let t = if !a.is_empty() { let p = c.ary_new_from(&[k, v]); crate::builtins::array_methods::rb_eq(c, p, a[0])? } else { yield_pair(c, blk, k, v)?.truthy() }; if t { n += 1; } Ok(true) })?;
    Ok(Value::fix(n))
});
cfn!(h_to_a, |c, slf, a, blk| hash_to_a(c, slf));
cfn!(h_to_h, |c, slf, a, blk| {
    if blk.is_null() { if obj_klass(slf) == c.vm().c_hash { return Ok(slf); } let h = c.hash_dup(slf); obj_set_klass(h, c.vm().c_hash); return Ok(h); }
    let out = c.hash_new();
    each_entry(c, slf, |c, k, v| {
        let r = yield_pair(c, blk, k, v)?;
        let p = match check_array(c, r)? { Some(p) => p, None => return type_err(c, format!("wrong element type {} (expected array)", class_name_for_err(r))) };
        if c.ary_len(p) != 2 { return arg_err(c, format!("element has wrong array length (expected 2, was {})", c.ary_len(p))); }
        let (k, v) = (c.ary_get(p, 0), c.ary_get(p, 1));
        hash_aset(c, out, k, v)?; Ok(true)
    })?;
    Ok(out)
});
cfn!(h_to_hash, |c, slf, a, blk| Ok(slf));
pub fn hash_inspect(c: &mut Ctx, slf: Value) -> RV {
    if c.hash_len(slf) == 0 { return Ok(c.str_new(b"{}")); }
    if !recursion_enter(slf, s::inspect) { return Ok(c.str_new(b"{...}")); }
    let mut out = vec![b'{'];
    let mut first = true;
    let r = each_entry(c, slf, |c, k, v| {
        if !first { out.extend_from_slice(b", "); }
        first = false;
        if k.is_sym() {
            let nm = crate::symbol::sym_bytes(k.as_sym());
            let simple = crate::string::sym_is_simple(nm) && !nm.ends_with(b"=") || (nm.ends_with(b"?") || nm.ends_with(b"!"));
            if simple && !nm.is_empty() && (nm[0].is_ascii_alphabetic() || nm[0] == b'_' || nm[0] >= 0x80) { out.extend_from_slice(nm); out.extend_from_slice(b": "); }
            else { out.extend_from_slice(inspect_bytes(nm, ENC_UTF8).as_bytes()); out.extend_from_slice(b": "); }
        } else {
            let ks = inspect_value(c, k)?;
            out.extend_from_slice(c.str_bytes(ks));
            out.extend_from_slice(b" => ");
        }
        let vs = inspect_value(c, v)?;
        out.extend_from_slice(c.str_bytes(vs));
        Ok(true)
    });
    recursion_leave(slf, s::inspect);
    r?;
    out.push(b'}');
    Ok(c.str_from_vec(out))
}
cfn!(h_inspect, |c, slf, a, blk| hash_inspect(c, slf));
cfn!(h_eq, |c, slf, a, blk| {
    let o = a[0];
    if slf == o { return Ok(TRUE); }
    if !is_hash(o) { if o.is_heap() && respond_to(o, s::to_hash, true) { let r = funcall(c, o, s::eq, &[slf], std::ptr::null())?; return Ok(Value::bool(r.truthy())); } return Ok(FALSE); }
    if c.hash_len(slf) != c.hash_len(o) { return Ok(FALSE); }
    if !recursion_enter(slf, s::eq) { return Ok(TRUE); }
    let mut ok = true;
    let r = each_entry(c, slf, |c, k, v| {
        match hash_get_opt(c, o, k)? { Some(w) => { if !crate::builtins::array_methods::rb_eq(c, v, w)? { ok = false; return Ok(false); } } None => { ok = false; return Ok(false); } }
        Ok(true)
    });
    recursion_leave(slf, s::eq);
    r?;
    Ok(Value::bool(ok))
});
cfn!(h_hash, |c, slf, a, blk| {
    let mut h: u64 = c.hash_len(slf) as u64;
    if !recursion_enter(slf, s::hash) { return Ok(Value::fix((h >> 2) as i64)); }
    let r = each_entry(c, slf, |c, k, v| { let kh = key_hash(c, k)?; let vh = key_hash(c, v)?; h = h.wrapping_add(crate::astrust::hash_merge(kh, vh)); Ok(true) });
    recursion_leave(slf, s::hash);
    r?;
    Ok(Value::fix((h >> 2) as i64))
});
cfn!(h_merge, |c, slf, a, blk| {
    let out = c.hash_dup(slf); obj_set_klass(out, c.vm().c_hash);
    if obj_klass(slf) != c.vm().c_hash { obj_set_klass(out, real_class_of(slf)); }
    h_update(c, out, a, blk)?;
    Ok(out)
});
cfn!(h_update, |c, slf, a, blk| {
    modify(c, slf)?;
    for &x in a {
        let o = match check_hash(c, x)? { Some(o) => o, None => return type_err(c, format!("no implicit conversion of {} into Hash", class_name_for_err(x))) };
        each_entry(c, o, |c, k, v| {
            let nv = if !blk.is_null() { match hash_get_opt(c, slf, k)? { Some(old) => { c.hash_iter_enter(slf); let r = yield_block(c, blk, &[k, old, v]); c.hash_iter_leave(slf); r? } None => v } } else { v };
            modify(c, slf)?;
            hash_aset(c, slf, k, nv)?; Ok(true)
        })?;
    }
    Ok(slf)
});
cfn!(h_clear, |c, slf, a, blk| { modify(c, slf)?; c.hash_clear(slf); Ok(slf) });
cfn!(h_replace, |c, slf, a, blk| {
    modify(c, slf)?;
    let o = match check_hash(c, a[0])? { Some(o) => o, None => return type_err(c, "no implicit conversion into Hash") };
    if o == slf { return Ok(slf); }
    if c.hash_iterating(slf) { return err(c, c.vm().c_runtime_error, "can't replace hash during iteration"); }
    let (d, others) = c.hash_mut_and(slf);
    if let Some(o) = others.hash(o) { d.clear(); d.identity = o.identity; d.default = o.default; d.default_proc = o.default_proc; }
    copy_entries(c, slf, o, false)?;
    Ok(slf)
});
cfn!(h_default, |c, slf, a, blk| {
    let p = c.hash(slf).default_proc;
    if !a.is_empty() && p != NIL { return call_proc(c, p, &[slf, a[0]], std::ptr::null()); }
    Ok(c.hash(slf).default)
});
cfn!(h_set_default, |c, slf, a, blk| { modify(c, slf)?; let r = c.hash_mut(slf); r.default = a[0]; r.default_proc = NIL; Ok(a[0]) });
cfn!(h_default_proc, |c, slf, a, blk| Ok(c.hash(slf).default_proc));
cfn!(h_set_default_proc, |c, slf, a, blk| {
    modify(c, slf)?;
    let mut p = a[0];
    if p != NIL && !is_proc(p) && respond_to(p, intern(b"to_proc"), true) {
        let q = funcall(c, p, intern(b"to_proc"), &[], std::ptr::null())?;
        if is_proc(q) { p = q; }
    }
    if p != NIL && !is_proc(p) { return type_err(c, format!("wrong default_proc type {} (expected Proc)", class_name_for_err(p))); }
    if p != NIL && c.proc_data(p).is_lambda { let ar = crate::builtins::proc::proc_arity(c, p); if ar != 2 && !(ar < 0 && ar >= -3) { return type_err(c, format!("default_proc takes two arguments (2 for {})", ar)); } }
    let r = c.hash_mut(slf); r.default_proc = p; r.default = NIL; Ok(p)
});
cfn!(h_invert, |c, slf, a, blk| { let out = c.hash_new(); each_entry(c, slf, |c, k, v| { hash_aset(c, out, v, k)?; Ok(true) })?; Ok(out) });
cfn!(h_transform_values, |c, slf, a, blk| enum_or(c, slf, "transform_values", a, blk, |c| { let out = c.hash_new(); each_entry(c, slf, |c, k, v| { let nv = yield1(c, blk, v)?; hash_set(c, out, k, nv)?; Ok(true) })?; Ok(out) }));
cfn!(h_transform_values_b, |c, slf, a, blk| enum_or(c, slf, "transform_values!", a, blk, |c| { modify(c, slf)?; each_entry(c, slf, |c, k, v| { let nv = yield1(c, blk, v)?; modify(c, slf)?; hash_set(c, slf, k, nv)?; Ok(true) })?; Ok(slf) }));
cfn!(h_transform_keys, |c, slf, a, blk| {
    let map = a.first().copied().filter(|x| is_hash(*x));
    if blk.is_null() && map.is_none() { return crate::builtins::enumerator::enum_for_sized(c, slf, intern(b"transform_keys"), a); }
    let out = c.hash_new();
    each_entry(c, slf, |c, k, v| {
        let nk = match map { Some(m) => match hash_get_opt(c, m, k)? { Some(x) => x, None => if blk.is_null() { k } else { yield1(c, blk, k)? } }, None => yield1(c, blk, k)? };
        hash_aset(c, out, nk, v)?; Ok(true)
    })?;
    Ok(out)
});
cfn!(h_transform_keys_b, |c, slf, a, blk| {
    modify(c, slf)?;
    let r = h_transform_keys(c, slf, a, blk)?;
    if !is_hash(r) { return Ok(r); }
    c.hash_clear(slf);
    copy_entries(c, slf, r, false)?;
    Ok(slf)
});
cfn!(h_sort_by_fallback, |c, slf, a, blk| Ok(NIL));
cfn!(h_min_by_fallback, |c, slf, a, blk| Ok(NIL));
cfn!(h_group_by, |c, slf, a, blk| enum_or(c, slf, "group_by", a, blk, |c| {
    let out = c.hash_new();
    each_entry(c, slf, |c, k, v| { let g = yield_pair(c, blk, k, v)?; let p = c.ary_new_from(&[k, v]); match hash_get_opt(c, out, g)? { Some(arr) => c.ary_push(arr, p), None => { let arr = c.ary_new_from(&[p]); hash_set(c, out, g, arr)?; } } Ok(true) })?;
    Ok(out)
}));
cfn!(h_partition, |c, slf, a, blk| enum_or(c, slf, "partition", a, blk, |c| {
    let (t, f) = (c.ary_new(), c.ary_new());
    each_entry(c, slf, |c, k, v| { let p = c.ary_new_from(&[k, v]); if yield_pair(c, blk, k, v)?.truthy() { c.ary_push(t, p); } else { c.ary_push(f, p); } Ok(true) })?;
    Ok(c.ary_new_from(&[t, f]))
}));
cfn!(h_sum, |c, slf, a, blk| { let arr = hash_to_a(c, slf)?; crate::builtins::array_methods::a_sum(c, arr, a, blk) });
cfn!(h_min_by, |c, slf, a, blk| { let arr = hash_to_a(c, slf)?; funcall(c, arr, intern(b"min_by"), a, blk) });
cfn!(h_sort_by, |c, slf, a, blk| { let arr = hash_to_a(c, slf)?; funcall(c, arr, s::sort_by, a, blk) });
cfn!(h_slice, |c, slf, a, blk| { let out = c.hash_new(); for &k in a { if let Some(v) = hash_get_opt(c, slf, k)? { hash_set(c, out, k, v)?; } } Ok(out) });
cfn!(h_except, |c, slf, a, blk| { let out = c.hash_dup(slf); obj_set_klass(out, c.vm().c_hash); for &k in a { hash_delete(c, out, k)?; } Ok(out) });
cfn!(h_filter_keys_dig, |c, slf, a, blk| {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let v = hash_aref(c, slf, a[0])?;
    if a.len() == 1 || v == NIL { return Ok(v); }
    if !respond_to(v, intern(b"dig"), true) { return type_err(c, format!("{} does not have #dig method", class_name_for_err(v))); }
    funcall(c, v, intern(b"dig"), &a[1..], std::ptr::null())
});
cfn!(h_shift, |c, slf, a, blk| {
    modify(c, slf)?;
    let mut first = None;
    each_entry(c, slf, |_c, k, v| { first = Some((k, v)); Ok(false) })?;
    match first { Some((k, v)) => { hash_delete(c, slf, k)?; Ok(c.ary_new_from(&[k, v])) } None => Ok(NIL) }
});
cfn!(h_first, |c, slf, a, blk| {
    if a.is_empty() { let mut f = NIL; each_entry(c, slf, |c, k, v| { f = c.ary_new_from(&[k, v]); Ok(false) })?; return Ok(f); }
    let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
    let out = c.ary_new();
    each_entry(c, slf, |c, k, v| { if c.ary_len(out) as i64 >= n { return Ok(false); } let p = c.ary_new_from(&[k, v]); c.ary_push(out, p); Ok(true) })?;
    Ok(out)
});
cfn!(h_compare_by_identity, |c, slf, a, blk| {
    modify(c, slf)?;
    // before the flag changes: a failed rehash would leave the index on the old hash codes
    if c.hash_iterating(slf) { return err(c, c.vm().c_runtime_error, "compare_by_identity during iteration"); }
    c.hash_mut(slf).identity = true;
    hash_rehash(c, slf)?;
    Ok(slf)
});
cfn!(h_compare_by_identity_p, |c, slf, a, blk| Ok(Value::bool(c.hash(slf).identity)));
cfn!(h_rehash, |c, slf, a, blk| { modify(c, slf)?; hash_rehash(c, slf)?; Ok(slf) });
cfn!(h_compact, |c, slf, a, blk| { let out = c.hash_new(); each_entry(c, slf, |c, k, v| { if v != NIL { hash_set(c, out, k, v)?; } Ok(true) })?; Ok(out) });
cfn!(h_compact_b, |c, slf, a, blk| {
    modify(c, slf)?;
    let mut changed = false;
    let mut i = 0;
    // by slot: no key is hashed again, so nothing can run in between
    while let Some((k, v)) = c.hash_entry_at(slf, i) {
        if k != UNDEF && v == NIL { c.hash_delete_slot(slf, i); changed = true; }
        i += 1;
    }
    Ok(if changed { slf } else { NIL })
});
cfn!(h_assoc, |c, slf, a, blk| Ok(match hash_get_opt(c, slf, a[0])? { Some(v) => { let mut key = a[0]; each_entry(c, slf, |c, k, _| { if key_eql(c, k, a[0])? { key = k; return Ok(false); } Ok(true) })?; c.ary_new_from(&[key, v]) } None => NIL }));
cfn!(h_rassoc, |c, slf, a, blk| { let mut r = NIL; each_entry(c, slf, |c, k, v| { if crate::builtins::array_methods::rb_eq(c, v, a[0])? { r = c.ary_new_from(&[k, v]); return Ok(false); } Ok(true) })?; Ok(r) });
cfn!(h_lt_le, |c, slf, a, blk| {
    let o = a[0];
    let mut ok = true;
    each_entry(c, slf, |c, k, v| { match hash_get_opt(c, o, k)? { Some(w) if crate::builtins::array_methods::rb_eq(c, v, w)? => {}, _ => { ok = false; return Ok(false); } } Ok(true) })?;
    Ok(Value::bool(ok))
});
cfn!(h_le, |c, slf, a, blk| { if !is_hash(a[0]) { return type_err(c, "no implicit conversion into Hash"); } h_lt_le(c, slf, a, blk) });
cfn!(h_lt, |c, slf, a, blk| { if !is_hash(a[0]) { return type_err(c, "no implicit conversion into Hash"); } if c.hash_len(slf) >= c.hash_len(a[0]) { return Ok(FALSE); } h_lt_le(c, slf, a, blk) });
cfn!(h_ge, |c, slf, a, blk| { if !is_hash(a[0]) { return type_err(c, "no implicit conversion into Hash"); } h_lt_le(c, a[0], &[slf], blk) });
cfn!(h_gt, |c, slf, a, blk| { if !is_hash(a[0]) { return type_err(c, "no implicit conversion into Hash"); } if c.hash_len(slf) <= c.hash_len(a[0]) { return Ok(FALSE); } h_lt_le(c, a[0], &[slf], blk) });
cfn!(h_freeze, |c, slf, a, blk| { obj_set_flags(slf, FL_FROZEN); Ok(slf) });
cfn!(h_to_proc, |c, slf, a, blk| {
    let b = Block::cfunc(c, |c, d, args, _b| { if args.is_empty() { return argc_err(c, 0, 1, 1); } hash_aref(c, d, args[0]) }, slf);
    let k = c.vm().c_proc;
    let p = c.alloc(k, Body::Proc(Box::new(ProcData { blk: b, is_lambda: true, from_method: NIL })));
    c.proc_data_mut(p).blk.proc_val = p;
    Ok(p)
});
cfn!(h_s_try_convert, |c, slf, a, blk| Ok(check_hash(c, a[0])?.unwrap_or(NIL)));
cfn!(h_s_new, |c, slf, a, blk| {
    let h = c.hash_new(); obj_set_klass(h, slf);
    funcall_kw(c, h, s::initialize, a, blk, kw_arg(a).is_some())?;
    Ok(h)
});
cfn!(h_initialize_copy, |c, slf, a, blk| h_replace(c, slf, a, blk));
cfn!(h_store_kw, |c, slf, a, blk| { modify(c, slf)?; hash_aset(c, slf, a[0], a[1])?; Ok(a[1]) });

cfn!(h_deconstruct_keys, |c, slf, a, blk| Ok(slf));
cfn!(h_flatten, |c, slf, a, blk| {
    check_argc(c, a, 0, 1)?;
    let depth = if a.is_empty() { 1 } else { crate::builtins::numeric_methods::num_to_long(c, a[0])? };
    if depth == 0 { return hash_to_a(c, slf); }
    let flat: Vec<Value> = c.hash(slf).entries.iter().filter(|e| e.key != UNDEF).flat_map(|e| [e.key, e.val]).collect();
    let out = c.ary_from_vec(flat);
    if depth > 1 || depth < 0 {
        let d = if depth < 0 { NIL } else { Value::fix(depth - 1) };
        return funcall(c, out, intern(b"flatten"), &[d], std::ptr::null());
    }
    Ok(out)
});

pub fn init(k: Value) {
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(k, n, f, a);
    define_private_cfunc(k, "initialize", h_initialize, -1);
    define_private_cfunc(k, "initialize_copy", h_initialize_copy, 1);
    define_singleton_cfunc(k, "[]", h_s_aref, -1);
    define_singleton_cfunc(k, "try_convert", h_s_try_convert, 1);
    define_singleton_cfunc(k, "new", h_s_new, -1);
    d("[]", h_aref, 1); d("[]=", h_aset, 2); d("store", h_store_kw, 2); d("fetch", h_fetch, -1); d("key?", h_key_p, 1); d("has_key?", h_key_p, 1);
    d("include?", h_key_p, 1); d("member?", h_key_p, 1); d("value?", h_value_p, 1); d("has_value?", h_value_p, 1); d("key", h_key, 1);
    d("delete", h_delete, 1); d("size", h_size, 0); d("length", h_size, 0); d("empty?", h_empty, 0); d("keys", h_keys, 0);
    d("values", h_values, 0); d("values_at", h_values_at, -1); d("fetch_values", h_fetch_values, -1); d("each", h_each, 0); d("each_pair", h_each, 0);
    d("each_key", h_each_key, 0); d("each_value", h_each_value, 0); d("each_with_index", h_each_with_index, 0); d("map", h_map, 0); d("collect", h_map, 0);
    d("flat_map", h_flat_map, 0); d("select", h_select, 0); d("filter", h_select, 0); d("reject", h_reject, 0); d("select!", h_select_b, 0);
    d("filter!", h_select_b, 0); d("keep_if", h_keep_if, 0); d("reject!", h_reject_b, 0); d("delete_if", h_delete_if, 0); d("filter_map", h_filter_map, 0);
    d("any?", h_any, -1); d("to_a", h_to_a, 0); d("to_h", h_to_h, 0);
    d("to_hash", h_to_hash, 0); d("inspect", h_inspect, 0); d("to_s", h_inspect, 0); d("==", h_eq, 1); d("eql?", h_eq, 1); d("hash", h_hash, 0);
    d("merge", h_merge, -1); d("update", h_update, -1); d("merge!", h_update, -1); d("clear", h_clear, 0); d("replace", h_replace, 1);
    d("default", h_default, -1); d("default=", h_set_default, 1); d("default_proc", h_default_proc, 0); d("default_proc=", h_set_default_proc, 1);
    d("invert", h_invert, 0); d("transform_values", h_transform_values, 0); d("transform_values!", h_transform_values_b, 0);
    d("transform_keys", h_transform_keys, -1); d("transform_keys!", h_transform_keys_b, -1); 
    d("slice", h_slice, -1); d("except", h_except, -1); d("dig", h_filter_keys_dig, -1);
    d("shift", h_shift, 0); d("compare_by_identity", h_compare_by_identity, 0); d("compare_by_identity?", h_compare_by_identity_p, 0);
    d("rehash", h_rehash, 0); d("compact", h_compact, 0); d("compact!", h_compact_b, 0); d("assoc", h_assoc, 1); d("rassoc", h_rassoc, 1);
    d("<=", h_le, 1); d("<", h_lt, 1); d(">=", h_ge, 1); d(">", h_gt, 1); d("freeze", h_freeze, 0); d("to_proc", h_to_proc, 0);
    d("deconstruct_keys", h_deconstruct_keys, 1); d("flatten", h_flatten, -1);
    let _ = (h_sort_by_fallback, h_min_by_fallback, h_find, h_all, h_count, h_group_by, h_partition, h_sum, h_min_by, h_sort_by, h_first);
}
