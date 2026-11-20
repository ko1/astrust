// Hash methods.
use crate::array::*;
use crate::builtins::kernel::*;
use crate::hash::*;
use crate::object::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

macro_rules! cfn { ($name:ident, |$c:ident, $s:ident, $a:ident, $b:ident| $body:expr) => {
    #[allow(unused_variables)]
    pub fn $name($c: &mut Ctx, $s: Value, $a: &[Value], $b: *const Block) -> RV { $body }
}; }

fn modify(c: &mut Ctx, h: Value) -> Result<(), Esc> { if obj(h).flags & FL_FROZEN != 0 { return Err(raise_frozen(c, h)); } Ok(()) }
fn enum_or<F: FnOnce(&mut Ctx) -> RV>(c: &mut Ctx, slf: Value, mid: &str, a: &[Value], blk: *const Block, f: F) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, intern(mid.as_bytes()), a); }
    f(c)
}
/// Iterate live entries by slot index (tolerates mutation during iteration).
fn each_entry(c: &mut Ctx, h: Value, mut f: impl FnMut(&mut Ctx, Value, Value) -> Result<bool, Esc>) -> Result<(), Esc> {
    let mut i = 0;
    while i < hash_slots(h) {
        if let Some((k, v)) = hash_entry_at(h, i) { if k != UNDEF && !f(c, k, v)? { break; } }
        i += 1;
    }
    Ok(())
}
fn yield_pair(c: &mut Ctx, blk: *const Block, k: Value, v: Value) -> RV {
    // |k, v| blocks receive two args; |pair| receives [k, v]
    let b = unsafe { &*blk };
    if let BlockKind::Iseq(is) = b.kind {
        let is = unsafe { &*is };
        if is.params.lead.len() + is.params.post.len() + is.params.opt.len() >= 2 || is.params.rest.is_some() && is.params.lead.len() >= 1 {
            return yield_block(c, blk, &[k, v]);
        }
    }
    let p = ary_new_from(&[k, v]);
    yield_block(c, blk, &[p])
}

cfn!(h_initialize, |c, slf, a, blk| {
    modify(c, slf)?;
    let pos: Vec<Value> = a.iter().copied().filter(|x| kw_arg(a) != Some(*x)).collect();
    if !blk.is_null() {
        if !pos.is_empty() { return argc_err(c, pos.len(), 0, 0); }
        let p = block_to_proc(c, blk)?;
        rhash(slf).default_proc = p;
    } else if let Some(&d) = pos.first() { rhash(slf).default = d; }
    if pos.len() > 1 { return argc_err(c, pos.len(), 0, 1); }
    Ok(slf)
});
cfn!(h_s_aref, |c, slf, a, blk| {
    let h = hash_new(); obj(h).klass = slf;
    if a.len() == 1 {
        if is_hash(a[0]) { for (k, v) in hash_pairs(a[0]) { hash_aset(c, h, k, v)?; } return Ok(h); }
        if let Some(arr) = check_array(c, a[0])? {
            for i in 0..ary_len(arr) {
                let p = ary_get(arr, i);
                let p = match check_array(c, p)? { Some(p) => p, None => return arg_err(c, format!("wrong element type {} at {} (expected array)", class_name_for_err(p), i)) };
                match ary_len(p) { 1 => hash_aset(c, h, ary_get(p, 0), NIL)?, 2 => hash_aset(c, h, ary_get(p, 0), ary_get(p, 1))?, n => return arg_err(c, format!("invalid number of elements ({} for 1..2)", n)) }
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
            let e = make_exception(vm().c_key_error, &format!("key not found: {}", ins));
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
cfn!(h_size, |c, slf, a, blk| Ok(Value::fix(hash_len(slf) as i64)));
cfn!(h_empty, |c, slf, a, blk| Ok(Value::bool(hash_len(slf) == 0)));
cfn!(h_keys, |c, slf, a, blk| Ok(ary_from_vec(hash_pairs(slf).into_iter().map(|p| p.0).collect())));
cfn!(h_values, |c, slf, a, blk| Ok(ary_from_vec(hash_pairs(slf).into_iter().map(|p| p.1).collect())));
cfn!(h_values_at, |c, slf, a, blk| { let out = ary_new(); for &k in a { let v = hash_aref(c, slf, k)?; ary_push(out, v); } Ok(out) });
cfn!(h_fetch_values, |c, slf, a, blk| { let out = ary_new(); for &k in a { let v = h_fetch(c, slf, &[k], blk)?; ary_push(out, v); } Ok(out) });
cfn!(h_each, |c, slf, a, blk| enum_or(c, slf, "each", a, blk, |c| { each_entry(c, slf, |c, k, v| { yield_pair(c, blk, k, v)?; Ok(true) })?; Ok(slf) }));
cfn!(h_each_key, |c, slf, a, blk| enum_or(c, slf, "each_key", a, blk, |c| { each_entry(c, slf, |c, k, _v| { yield1(c, blk, k)?; Ok(true) })?; Ok(slf) }));
cfn!(h_each_value, |c, slf, a, blk| enum_or(c, slf, "each_value", a, blk, |c| { each_entry(c, slf, |c, _k, v| { yield1(c, blk, v)?; Ok(true) })?; Ok(slf) }));
cfn!(h_each_with_index, |c, slf, a, blk| enum_or(c, slf, "each_with_index", a, blk, |c| {
    let mut i = 0;
    each_entry(c, slf, |c, k, v| { let p = ary_new_from(&[k, v]); yield_block(c, blk, &[p, Value::fix(i)])?; i += 1; Ok(true) })?; Ok(slf)
}));
cfn!(h_map, |c, slf, a, blk| enum_or(c, slf, "map", a, blk, |c| { let out = ary_new(); each_entry(c, slf, |c, k, v| { let r = yield_pair(c, blk, k, v)?; ary_push(out, r); Ok(true) })?; Ok(out) }));
cfn!(h_flat_map, |c, slf, a, blk| enum_or(c, slf, "flat_map", a, blk, |c| { let out = ary_new(); each_entry(c, slf, |c, k, v| { let r = yield_pair(c, blk, k, v)?; if is_array(r) { let x = rary(r).clone(); rary(out).extend(x); } else { ary_push(out, r); } Ok(true) })?; Ok(out) }));
fn sel(c: &mut Ctx, slf: Value, blk: *const Block, want: bool) -> RV {
    let out = hash_new();
    each_entry(c, slf, |c, k, v| { if yield_pair(c, blk, k, v)?.truthy() == want { hash_set(c, out, k, v)?; } Ok(true) })?;
    Ok(out)
}
cfn!(h_select, |c, slf, a, blk| enum_or(c, slf, "select", a, blk, |c| sel(c, slf, blk, true)));
cfn!(h_reject, |c, slf, a, blk| enum_or(c, slf, "reject", a, blk, |c| sel(c, slf, blk, false)));
fn sel_b(c: &mut Ctx, slf: Value, blk: *const Block, want: bool) -> Result<bool, Esc> {
    modify(c, slf)?;
    let mut dels = vec![];
    each_entry(c, slf, |c, k, v| { if yield_pair(c, blk, k, v)?.truthy() != want { dels.push(k); } Ok(true) })?;
    let keep = ary_from_vec(dels.clone());
    for k in &dels { hash_delete(c, slf, *k)?; }
    std::hint::black_box(keep);
    Ok(!dels.is_empty())
}
cfn!(h_select_b, |c, slf, a, blk| enum_or(c, slf, "select!", a, blk, |c| Ok(if sel_b(c, slf, blk, true)? { slf } else { NIL })));
cfn!(h_keep_if, |c, slf, a, blk| enum_or(c, slf, "keep_if", a, blk, |c| { sel_b(c, slf, blk, true)?; Ok(slf) }));
cfn!(h_reject_b, |c, slf, a, blk| enum_or(c, slf, "reject!", a, blk, |c| Ok(if sel_b(c, slf, blk, false)? { slf } else { NIL })));
cfn!(h_delete_if, |c, slf, a, blk| enum_or(c, slf, "delete_if", a, blk, |c| { sel_b(c, slf, blk, false)?; Ok(slf) }));
cfn!(h_filter_map, |c, slf, a, blk| enum_or(c, slf, "filter_map", a, blk, |c| { let out = ary_new(); each_entry(c, slf, |c, k, v| { let r = yield_pair(c, blk, k, v)?; if r.truthy() { ary_push(out, r); } Ok(true) })?; Ok(out) }));
cfn!(h_find, |c, slf, a, blk| enum_or(c, slf, "find", a, blk, |c| { let mut res = NIL; each_entry(c, slf, |c, k, v| { if yield_pair(c, blk, k, v)?.truthy() { res = ary_new_from(&[k, v]); return Ok(false); } Ok(true) })?; Ok(res) }));
cfn!(h_any, |c, slf, a, blk| {
    if blk.is_null() && a.is_empty() { return Ok(Value::bool(hash_len(slf) > 0)); }
    let mut res = false;
    each_entry(c, slf, |c, k, v| { let t = if !a.is_empty() { let p = ary_new_from(&[k, v]); crate::rt::rt_eqq(c, a[0], p, &mut CallCache::default())? } else { yield_pair(c, blk, k, v)?.truthy() }; if t { res = true; return Ok(false); } Ok(true) })?;
    Ok(Value::bool(res))
});
cfn!(h_all, |c, slf, a, blk| {
    let mut res = true;
    each_entry(c, slf, |c, k, v| { let t = if blk.is_null() { true } else { yield_pair(c, blk, k, v)?.truthy() }; if !t { res = false; return Ok(false); } Ok(true) })?;
    Ok(Value::bool(res))
});
cfn!(h_count, |c, slf, a, blk| {
    if blk.is_null() && a.is_empty() { return Ok(Value::fix(hash_len(slf) as i64)); }
    let mut n = 0;
    each_entry(c, slf, |c, k, v| { let t = if !a.is_empty() { let p = ary_new_from(&[k, v]); crate::builtins::array_methods::rb_eq(c, p, a[0])? } else { yield_pair(c, blk, k, v)?.truthy() }; if t { n += 1; } Ok(true) })?;
    Ok(Value::fix(n))
});
cfn!(h_to_a, |c, slf, a, blk| hash_to_a(c, slf));
cfn!(h_to_h, |c, slf, a, blk| {
    if blk.is_null() { if obj(slf).klass == vm().c_hash { return Ok(slf); } let h = hash_dup(slf); obj(h).klass = vm().c_hash; return Ok(h); }
    let out = hash_new();
    each_entry(c, slf, |c, k, v| {
        let r = yield_pair(c, blk, k, v)?;
        let p = match check_array(c, r)? { Some(p) => p, None => return type_err(c, format!("wrong element type {} (expected array)", class_name_for_err(r))) };
        if ary_len(p) != 2 { return arg_err(c, format!("element has wrong array length (expected 2, was {})", ary_len(p))); }
        hash_aset(c, out, ary_get(p, 0), ary_get(p, 1))?; Ok(true)
    })?;
    Ok(out)
});
cfn!(h_to_hash, |c, slf, a, blk| Ok(slf));
pub fn hash_inspect(c: &mut Ctx, slf: Value) -> RV {
    if hash_len(slf) == 0 { return Ok(str_new(b"{}")); }
    if !recursion_enter(slf, s::inspect) { return Ok(str_new(b"{...}")); }
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
            out.extend_from_slice(str_bytes(ks));
            out.extend_from_slice(b" => ");
        }
        let vs = inspect_value(c, v)?;
        out.extend_from_slice(str_bytes(vs));
        Ok(true)
    });
    recursion_leave(slf, s::inspect);
    r?;
    out.push(b'}');
    Ok(str_from_vec(out))
}
cfn!(h_inspect, |c, slf, a, blk| hash_inspect(c, slf));
cfn!(h_eq, |c, slf, a, blk| {
    let o = a[0];
    if slf == o { return Ok(TRUE); }
    if !is_hash(o) { if o.is_heap() && respond_to(o, s::to_hash, true) { let r = funcall(c, o, s::eq, &[slf], std::ptr::null())?; return Ok(Value::bool(r.truthy())); } return Ok(FALSE); }
    if hash_len(slf) != hash_len(o) { return Ok(FALSE); }
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
    let mut h: u64 = hash_len(slf) as u64;
    each_entry(c, slf, |c, k, v| { let kh = key_hash(c, k)?; let vh = key_hash(c, v)?; h = h.wrapping_add(crate::astrust::hash_merge(kh, vh)); Ok(true) })?;
    Ok(Value::fix((h >> 2) as i64))
});
cfn!(h_merge, |c, slf, a, blk| {
    let out = hash_dup(slf); obj(out).klass = vm().c_hash;
    if obj(slf).klass != vm().c_hash { obj(out).klass = real_class_of(slf); }
    h_update(c, out, a, blk)?;
    Ok(out)
});
cfn!(h_update, |c, slf, a, blk| {
    modify(c, slf)?;
    for &x in a {
        let o = match check_hash(c, x)? { Some(o) => o, None => return type_err(c, format!("no implicit conversion of {} into Hash", class_name_for_err(x))) };
        each_entry(c, o, |c, k, v| {
            let nv = if !blk.is_null() { match hash_get_opt(c, slf, k)? { Some(old) => yield_block(c, blk, &[k, old, v])?, None => v } } else { v };
            hash_aset(c, slf, k, nv)?; Ok(true)
        })?;
    }
    Ok(slf)
});
cfn!(h_clear, |c, slf, a, blk| { modify(c, slf)?; hash_clear(slf); Ok(slf) });
cfn!(h_replace, |c, slf, a, blk| {
    modify(c, slf)?;
    let o = match check_hash(c, a[0])? { Some(o) => o, None => return type_err(c, "no implicit conversion into Hash") };
    if o == slf { return Ok(slf); }
    let pairs = hash_pairs(o);
    hash_clear(slf);
    for (k, v) in pairs { hash_set(c, slf, k, v)?; }
    rhash(slf).default = rhash(o).default; rhash(slf).default_proc = rhash(o).default_proc;
    Ok(slf)
});
cfn!(h_default, |c, slf, a, blk| {
    if !a.is_empty() && rhash(slf).default_proc != NIL { let p = rhash(slf).default_proc; return call_proc(c, p, &[slf, a[0]], std::ptr::null()); }
    Ok(rhash(slf).default)
});
cfn!(h_set_default, |c, slf, a, blk| { modify(c, slf)?; rhash(slf).default = a[0]; rhash(slf).default_proc = NIL; Ok(a[0]) });
cfn!(h_default_proc, |c, slf, a, blk| Ok(rhash(slf).default_proc));
cfn!(h_set_default_proc, |c, slf, a, blk| {
    modify(c, slf)?;
    let p = a[0];
    if p != NIL && !is_proc(p) { return type_err(c, format!("wrong default_proc type {} (expected Proc)", class_name_for_err(p))); }
    if p != NIL && rproc(p).is_lambda { let ar = crate::builtins::proc::proc_arity(p); if ar != 2 && !(ar < 0 && ar >= -3) { return type_err(c, format!("default_proc takes two arguments (2 for {})", ar)); } }
    rhash(slf).default_proc = p; rhash(slf).default = NIL; Ok(p)
});
cfn!(h_invert, |c, slf, a, blk| { let out = hash_new(); each_entry(c, slf, |c, k, v| { hash_aset(c, out, v, k)?; Ok(true) })?; Ok(out) });
cfn!(h_transform_values, |c, slf, a, blk| enum_or(c, slf, "transform_values", a, blk, |c| { let out = hash_new(); each_entry(c, slf, |c, k, v| { let nv = yield1(c, blk, v)?; hash_set(c, out, k, nv)?; Ok(true) })?; Ok(out) }));
cfn!(h_transform_values_b, |c, slf, a, blk| enum_or(c, slf, "transform_values!", a, blk, |c| { modify(c, slf)?; each_entry(c, slf, |c, k, v| { let nv = yield1(c, blk, v)?; hash_set(c, slf, k, nv)?; Ok(true) })?; Ok(slf) }));
cfn!(h_transform_keys, |c, slf, a, blk| {
    let map = a.first().copied().filter(|x| is_hash(*x));
    if blk.is_null() && map.is_none() { return crate::builtins::enumerator::enum_for_sized(c, slf, intern(b"transform_keys"), a); }
    let out = hash_new();
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
    let pairs = hash_pairs(r);
    hash_clear(slf);
    for (k, v) in pairs { hash_set(c, slf, k, v)?; }
    Ok(slf)
});
cfn!(h_sort_by_fallback, |c, slf, a, blk| Ok(NIL));
cfn!(h_min_by_fallback, |c, slf, a, blk| Ok(NIL));
cfn!(h_group_by, |c, slf, a, blk| enum_or(c, slf, "group_by", a, blk, |c| {
    let out = hash_new();
    each_entry(c, slf, |c, k, v| { let g = yield_pair(c, blk, k, v)?; let p = ary_new_from(&[k, v]); match hash_get_opt(c, out, g)? { Some(arr) => ary_push(arr, p), None => { let arr = ary_new_from(&[p]); hash_set(c, out, g, arr)?; } } Ok(true) })?;
    Ok(out)
}));
cfn!(h_partition, |c, slf, a, blk| enum_or(c, slf, "partition", a, blk, |c| {
    let (t, f) = (ary_new(), ary_new());
    each_entry(c, slf, |c, k, v| { let p = ary_new_from(&[k, v]); if yield_pair(c, blk, k, v)?.truthy() { ary_push(t, p); } else { ary_push(f, p); } Ok(true) })?;
    Ok(ary_new_from(&[t, f]))
}));
cfn!(h_sum, |c, slf, a, blk| { let arr = hash_to_a(c, slf)?; crate::builtins::array_methods::a_sum(c, arr, a, blk) });
cfn!(h_min_by, |c, slf, a, blk| { let arr = hash_to_a(c, slf)?; funcall(c, arr, intern(b"min_by"), a, blk) });
cfn!(h_sort_by, |c, slf, a, blk| { let arr = hash_to_a(c, slf)?; funcall(c, arr, s::sort_by, a, blk) });
cfn!(h_slice, |c, slf, a, blk| { let out = hash_new(); for &k in a { if let Some(v) = hash_get_opt(c, slf, k)? { hash_set(c, out, k, v)?; } } Ok(out) });
cfn!(h_except, |c, slf, a, blk| { let out = hash_dup(slf); obj(out).klass = vm().c_hash; for &k in a { hash_delete(c, out, k)?; } Ok(out) });
cfn!(h_filter_keys_dig, |c, slf, a, blk| {
    let v = hash_aref(c, slf, a[0])?;
    if a.len() == 1 || v == NIL { return Ok(v); }
    funcall(c, v, intern(b"dig"), &a[1..], std::ptr::null())
});
cfn!(h_shift, |c, slf, a, blk| {
    modify(c, slf)?;
    let mut first = None;
    each_entry(c, slf, |_c, k, v| { first = Some((k, v)); Ok(false) })?;
    match first { Some((k, v)) => { hash_delete(c, slf, k)?; Ok(ary_new_from(&[k, v])) } None => Ok(NIL) }
});
cfn!(h_first, |c, slf, a, blk| {
    if a.is_empty() { let mut f = NIL; each_entry(c, slf, |_c, k, v| { f = ary_new_from(&[k, v]); Ok(false) })?; return Ok(f); }
    let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
    let out = ary_new();
    each_entry(c, slf, |_c, k, v| { if ary_len(out) as i64 >= n { return Ok(false); } ary_push(out, ary_new_from(&[k, v])); Ok(true) })?;
    Ok(out)
});
cfn!(h_compare_by_identity, |c, slf, a, blk| { modify(c, slf)?; rhash(slf).identity = true; hash_rehash(c, slf)?; Ok(slf) });
cfn!(h_compare_by_identity_p, |c, slf, a, blk| Ok(Value::bool(rhash(slf).identity)));
cfn!(h_rehash, |c, slf, a, blk| { modify(c, slf)?; hash_rehash(c, slf)?; Ok(slf) });
cfn!(h_compact, |c, slf, a, blk| { let out = hash_new(); each_entry(c, slf, |c, k, v| { if v != NIL { hash_set(c, out, k, v)?; } Ok(true) })?; Ok(out) });
cfn!(h_compact_b, |c, slf, a, blk| { modify(c, slf)?; let mut dels = vec![]; each_entry(c, slf, |_c, k, v| { if v == NIL { dels.push(k); } Ok(true) })?; for k in &dels { hash_delete(c, slf, *k)?; } Ok(if dels.is_empty() { NIL } else { slf }) });
cfn!(h_assoc, |c, slf, a, blk| Ok(match hash_get_opt(c, slf, a[0])? { Some(v) => { let mut key = a[0]; each_entry(c, slf, |c, k, _| { if key_eql(c, k, a[0])? { key = k; return Ok(false); } Ok(true) })?; ary_new_from(&[key, v]) } None => NIL }));
cfn!(h_rassoc, |c, slf, a, blk| { let mut r = NIL; each_entry(c, slf, |c, k, v| { if crate::builtins::array_methods::rb_eq(c, v, a[0])? { r = ary_new_from(&[k, v]); return Ok(false); } Ok(true) })?; Ok(r) });
cfn!(h_lt_le, |c, slf, a, blk| {
    let o = a[0];
    let mut ok = true;
    each_entry(c, slf, |c, k, v| { match hash_get_opt(c, o, k)? { Some(w) if crate::builtins::array_methods::rb_eq(c, v, w)? => {}, _ => { ok = false; return Ok(false); } } Ok(true) })?;
    Ok(Value::bool(ok))
});
cfn!(h_le, |c, slf, a, blk| { if !is_hash(a[0]) { return type_err(c, "no implicit conversion into Hash"); } h_lt_le(c, slf, a, blk) });
cfn!(h_lt, |c, slf, a, blk| { if !is_hash(a[0]) { return type_err(c, "no implicit conversion into Hash"); } if hash_len(slf) >= hash_len(a[0]) { return Ok(FALSE); } h_lt_le(c, slf, a, blk) });
cfn!(h_ge, |c, slf, a, blk| { if !is_hash(a[0]) { return type_err(c, "no implicit conversion into Hash"); } h_lt_le(c, a[0], &[slf], blk) });
cfn!(h_gt, |c, slf, a, blk| { if !is_hash(a[0]) { return type_err(c, "no implicit conversion into Hash"); } if hash_len(slf) <= hash_len(a[0]) { return Ok(FALSE); } h_lt_le(c, a[0], &[slf], blk) });
cfn!(h_freeze, |c, slf, a, blk| { obj(slf).flags |= FL_FROZEN; Ok(slf) });
cfn!(h_to_proc, |c, slf, a, blk| {
    let b = Block::cfunc(c, |c, d, args, _b| { if args.is_empty() { return argc_err(c, 0, 1, 1); } hash_aref(c, d, args[0]) }, slf);
    let p = crate::gc::alloc(vm().c_proc, Body::Proc(Box::new(ProcData { blk: b, is_lambda: true, from_method: NIL })));
    rproc(p).blk.proc_val = p;
    Ok(p)
});
cfn!(h_s_try_convert, |c, slf, a, blk| Ok(check_hash(c, a[0])?.unwrap_or(NIL)));
cfn!(h_s_new, |c, slf, a, blk| {
    let h = hash_new(); obj(h).klass = slf;
    funcall_kw(c, h, s::initialize, a, blk, kw_arg(a).is_some())?;
    Ok(h)
});
cfn!(h_initialize_copy, |c, slf, a, blk| h_replace(c, slf, a, blk));
cfn!(h_store_kw, |c, slf, a, blk| { modify(c, slf)?; hash_aset(c, slf, a[0], a[1])?; Ok(a[1]) });

pub fn init() {
    let k = vm().c_hash;
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(k, n, f, a);
    define_private_cfunc(k, "initialize", h_initialize, -1);
    define_private_cfunc(k, "initialize_copy", h_initialize_copy, 1);
    define_singleton_cfunc(k, "[]", h_s_aref, -1);
    define_singleton_cfunc(k, "try_convert", h_s_try_convert, 1);
    define_singleton_cfunc(k, "new", h_s_new, -1);
    d("[]", h_aref, 1); d("[]=", h_aset, 2); d("store", h_store_kw, 2); d("fetch", h_fetch, -1); d("key?", h_key_p, 1); d("has_key?", h_key_p, 1);
    d("include?", h_key_p, 1); d("member?", h_key_p, 1); d("value?", h_value_p, 1); d("has_value?", h_value_p, 1); d("key", h_key, 1);
    d("delete", h_delete, 1); d("size", h_size, 0); d("length", h_size, 0); d("count", h_count, -1); d("empty?", h_empty, 0); d("keys", h_keys, 0);
    d("values", h_values, 0); d("values_at", h_values_at, -1); d("fetch_values", h_fetch_values, -1); d("each", h_each, 0); d("each_pair", h_each, 0);
    d("each_key", h_each_key, 0); d("each_value", h_each_value, 0); d("each_with_index", h_each_with_index, 0); d("map", h_map, 0); d("collect", h_map, 0);
    d("flat_map", h_flat_map, 0); d("select", h_select, 0); d("filter", h_select, 0); d("reject", h_reject, 0); d("select!", h_select_b, 0);
    d("filter!", h_select_b, 0); d("keep_if", h_keep_if, 0); d("reject!", h_reject_b, 0); d("delete_if", h_delete_if, 0); d("filter_map", h_filter_map, 0);
    d("find", h_find, -1); d("detect", h_find, -1); d("any?", h_any, -1); d("all?", h_all, -1); d("to_a", h_to_a, 0); d("entries", h_to_a, 0); d("to_h", h_to_h, 0);
    d("to_hash", h_to_hash, 0); d("inspect", h_inspect, 0); d("to_s", h_inspect, 0); d("==", h_eq, 1); d("eql?", h_eq, 1); d("hash", h_hash, 0);
    d("merge", h_merge, -1); d("update", h_update, -1); d("merge!", h_update, -1); d("clear", h_clear, 0); d("replace", h_replace, 1);
    d("default", h_default, -1); d("default=", h_set_default, 1); d("default_proc", h_default_proc, 0); d("default_proc=", h_set_default_proc, 1);
    d("invert", h_invert, 0); d("transform_values", h_transform_values, 0); d("transform_values!", h_transform_values_b, 0);
    d("transform_keys", h_transform_keys, -1); d("transform_keys!", h_transform_keys_b, -1); d("group_by", h_group_by, 0); d("partition", h_partition, 0);
    d("sum", h_sum, -1); d("min_by", h_min_by, -1); d("sort_by", h_sort_by, 0); d("slice", h_slice, -1); d("except", h_except, -1); d("dig", h_filter_keys_dig, -1);
    d("shift", h_shift, 0); d("first", h_first, -1); d("compare_by_identity", h_compare_by_identity, 0); d("compare_by_identity?", h_compare_by_identity_p, 0);
    d("rehash", h_rehash, 0); d("compact", h_compact, 0); d("compact!", h_compact_b, 0); d("assoc", h_assoc, 1); d("rassoc", h_rassoc, 1);
    d("<=", h_le, 1); d("<", h_lt, 1); d(">=", h_ge, 1); d(">", h_gt, 1); d("freeze", h_freeze, 0); d("to_proc", h_to_proc, 0);
    let _ = (h_sort_by_fallback, h_min_by_fallback);
}
