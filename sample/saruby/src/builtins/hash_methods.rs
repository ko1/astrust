// Hash methods, on the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::array::*;
use crate::builtins::kernel::*;
use crate::hash::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

macro_rules! cfn { ($(#[$m:meta])* $name:ident, |$c:ident, $s:ident, $a:ident, $b:ident| $body:expr) => {
    $(#[$m])* #[allow(unused_variables)]
    pub fn $name($c: &mut Ctx, $s: Value, $a: &[Value], $b: Blk) -> RV { $body }
}; }

fn modify(c: &mut Ctx, h: Value) -> Result<(), Esc> { if obj_flags(h) & FL_FROZEN != 0 { return Err(raise_frozen(c, h)); } Ok(()) }
fn enum_or<F: FnOnce(&mut Ctx) -> RV>(c: &mut Ctx, slf: Value, mid: &str, a: &[Value], blk: Blk, f: F) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, intern(mid.as_bytes()), a); }
    f(c)
}
/// Iterate live entries by slot index (tolerates mutation during iteration).
fn each_entry(c: &mut Ctx, h: Value, mut f: impl FnMut(&mut Ctx, Value, Value) -> Result<bool, Esc>) -> Result<(), Esc> {
    c.hash_iter_enter(h);
    let mut i = c.hash_first_slot(h);
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
/// rb_lambda_call (Enumerable#map): a Method proc needing 2+ arguments is yielded k, v.
#[cold]
fn method_wants_two(c: &Ctx, blk: Blk) -> bool {
    let Some(m) = c.block_method(blk) else { return false };
    let a = crate::builtins::proc::method_arity(c, c.method_obj(m).me);
    (if a >= 0 { a } else { -a - 1 }) >= 2
}
/// |k, v| blocks receive two args; |pair| receives [k, v]
fn wants_two(c: &Ctx, blk: Blk) -> bool {
    matches!(c.block_params(blk), Some(p) if p.lead.len() + p.post.len() + p.opt.len() >= 2 || p.rest.is_some() && p.lead.len() >= 1)
}
/// Enumerable#map's rb_lambda_call: the block's min arity (2+) makes #each yield two values; a lambda's optional/rest do not count.
fn map_wants_two(c: &Ctx, blk: Blk) -> bool {
    match c.block_params(blk) {
        Some(p) if is_lambda_block(c, blk) => p.lead.len() + p.post.len() >= 2,
        Some(p) => p.lead.len() + p.post.len() + p.opt.len() >= 2 || p.rest.is_some() && p.lead.len() >= 1,
        None => method_wants_two(c, blk),
    }
}
/// A lambda is never given the pair as two values (the packed rb_assoc_new); `wants_two` for the rest.
fn pair_two(c: &Ctx, blk: Blk) -> bool { wants_two(c, blk) && !is_lambda_block(c, blk) }
/// rb_yield_force_blockarg(ary): a lambda iseq takes the elements with block-argument setup, other blocks the array.
pub fn yield_forced(c: &mut Ctx, blk: Blk, ary: Value) -> RV {
    let args: Vec<Value> = match c.block_params(blk) {
        Some(p) if is_lambda_block(c, blk) => {
            let min = p.lead.len() + p.post.len();
            let max = if p.rest.is_some() { usize::MAX } else { min + p.opt.len() };
            let has_kw = !p.kw.is_empty() || p.kwrest.is_some();
            let ambiguous = min == 1 && max == 1 && !has_kw;
            let splat = (min > 0 || !p.opt.is_empty() || has_kw) && !ambiguous && !(has_kw && max == 1);
            let mut a = if splat { c.ary(ary).to_vec() } else { vec![ary] };
            a.truncate(max);
            while a.len() < min { a.push(NIL); }
            a
        }
        _ => vec![ary],
    };
    yield_block(c, blk, &args)
}
/// Enumerable#find on a Hash with a lambda block (enum.c find_i); None for any other block or receiver.
pub fn find_lambda(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> Option<RV> {
    if !is_hash(slf) || blk.is_null() || !is_lambda_block(c, blk) || c.block_params(blk).is_none() || crate::builtins::enum_native::each_overridden(c, slf, h_each) { return None; }
    let mut res = None;
    let r = each_entry(c, slf, |c, k, v| {
        let pr = c.ary_new_from(&[k, v]);
        if yield_forced(c, blk, pr)?.truthy() { res = Some(pr); return Ok(false); }
        Ok(true)
    });
    Some(match (r, res) {
        (Err(e), _) => Err(e),
        (Ok(()), Some(p)) => Ok(p),
        (Ok(()), None) => match a.first() { Some(&f) if f != NIL => funcall(c, f, s::call, &[], std::ptr::null()), _ => Ok(NIL) },
    })
}
/// hash_dup_with_compare_by_id: a Hash with the entries and the identity flag, no defaults.
fn dup_with_identity(c: &mut Ctx, h: Value) -> Value {
    let mut d = c.hash(h).dup();
    d.default = NIL; d.default_proc = NIL;
    let k = c.vm().c_hash.get();
    c.alloc(k, Body::Hash(Box::new(d)))
}
/// An empty Hash with h's identity flag (copy_compare_by_id).
fn new_like(c: &mut Ctx, h: Value) -> Value {
    let id = c.hash(h).identity;
    let o = c.hash_new();
    if id { c.hash_mut(o).identity = true; }
    o
}
/// to_hash (rb_convert_type): a Hash or TypeError.
fn to_hash_conv(c: &mut Ctx, v: Value) -> RV {
    match check_hash(c, v)? { Some(h) => Ok(h), None => type_err(c, format!("no implicit conversion of {} into Hash", class_name_for_err(c, v))) }
}
#[inline]
fn yield_pair(c: &mut Ctx, blk: Blk, k: Value, v: Value) -> RV { let two = wants_two(c, blk); yield_kv(c, blk, two, k, v) }
#[inline]
fn yield_kv(c: &mut Ctx, blk: Blk, two: bool, k: Value, v: Value) -> RV {
    if two { return yield_block(c, blk, &[k, v]); }
    let p = c.ary_new_from(&[k, v]);
    yield_block(c, blk, &[p])
}

cfn!(h_initialize, |c, slf, a, blk| {
    modify(c, slf)?;
    let (pos, kw) = kw_split(c, a);
    if let Some(h) = kw { kw_unknown(c, h, &["capacity"])?; }
    if !blk.is_null() {
        if !pos.is_empty() { return argc_err(c, pos.len(), 0, 0); }
        let p = block_to_proc(c, blk)?;
        let r = c.hash_mut(slf); r.default_proc = p; r.default = NIL;
    } else {
        if pos.len() > 1 { return argc_err(c, pos.len(), 0, 1); }
        let d = pos.first().copied().unwrap_or(NIL);
        let r = c.hash_mut(slf); r.default = d; r.default_proc = NIL;
    }
    Ok(slf)
});
cfn!(h_s_aref, |c, slf, a, blk| {
    let h = c.hash_new(); obj_set_klass(h, slf);
    if a.len() == 1 {
        if let Some(src) = check_hash(c, a[0])? { copy_entries(c, h, src, true)?; return Ok(h); }
        if let Some(arr) = check_array(c, a[0])? {
            for i in 0..c.ary_len(arr) {
                let p = c.ary_get(arr, i);
                let p = match check_array(c, p)? { Some(p) => p, None => return arg_err(c, format!("wrong element type {} at {} (expected array)", class_name_for_err(c, p), i)) };
                match c.ary(p) { &[k] => hash_aset(c, h, k, NIL)?, &[k, v] => hash_aset(c, h, k, v)?, x => { let n = x.len(); return arg_err(c, format!("invalid number of elements ({} for 1..2)", n)) } }
            }
            return Ok(h);
        }
    }
    if a.len() % 2 != 0 { return arg_err(c, "odd number of arguments for Hash"); }
    for i in (0..a.len()).step_by(2) { hash_aset(c, h, a[i], a[i + 1])?; }
    Ok(h)
});
cfn!(#[inline] h_aref, |c, slf, a, blk| h_aref_at(c, slf, a[0], ()));
cfn!(#[inline] h_aset, |c, slf, a, blk| h_aset_at(c, slf, a[0], a[1], ()));
/// Hash#[] / Hash#[]= with the line an operator node stores only where they raise or call out.
#[inline] pub fn h_aref_at(c: &mut Ctx, slf: Value, k: Value, line: impl AtLine) -> RV { crate::hash::hash_aref_at(c, slf, k, line) }
#[inline] pub fn h_aset_at(c: &mut Ctx, slf: Value, k: Value, v: Value, line: impl AtLine) -> RV { crate::hash::hash_aset_at(c, slf, k, v, line)?; Ok(v) }
cfn!(h_fetch, |c, slf, a, blk| {
    check_argc(c, a, 1, 2)?;
    if !blk.is_null() && a.len() == 2 { gvar_warn(c, "block supersedes default value argument", false)?; }
    match hash_get_opt(c, slf, a[0])? {
        Some(v) => Ok(v),
        None => {
            if !blk.is_null() { return yield1(c, blk, a[0]); }
            if a.len() > 1 { return Ok(a[1]); }
            let ins = inspect_str(c, a[0])?;
            let e = make_exception_rb(c, c.vm().c_key_error.get(), &format!("key not found: {}", ins));
            ivar_set_raw(c, e, s::key, a[0]); ivar_set_raw(c, e, s::receiver, slf);
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
cfn!(#[inline] h_size, |c, slf, a, blk| Ok(Value::fix(c.hash_len(slf) as i64)));
cfn!(#[inline] h_empty, |c, slf, a, blk| Ok(Value::bool(c.hash_len(slf) == 0)));
cfn!(h_keys, |c, slf, a, blk| Ok(c.ary_from_vec(c.hash(slf).entries.iter().filter(|e| e.key != UNDEF).map(|e| e.key).collect())));
cfn!(h_values, |c, slf, a, blk| Ok(c.ary_from_vec(c.hash(slf).entries.iter().filter(|e| e.key != UNDEF).map(|e| e.val).collect())));
cfn!(h_values_at, |c, slf, a, blk| { let out = c.ary_new(); for &k in a { let v = hash_aref(c, slf, k)?; c.ary_push(out, v); } Ok(out) });
cfn!(h_fetch_values, |c, slf, a, blk| { let out = c.ary_new(); for &k in a { let v = h_fetch(c, slf, &[k], blk)?; c.ary_push(out, v); } Ok(out) });
/// Hash#each, yielding each pair as two values through `y` (inline_iter.rs: a block with 2+ parameters).
#[inline(always)]
pub fn h_each_loop(c: &mut Ctx, slf: Value, y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> RV {
    each_entry(c, slf, |c, k, v| { y(c, &[k, v])?; Ok(true) })?;
    Ok(slf)
}
cfn!(h_each, |c, slf, a, blk| enum_or(c, slf, "each", a, blk, |c| { let two = wants_two(c, blk) && !is_lambda_block(c, blk); h_each_loop(c, slf, &mut |c, a| yield_kv(c, blk, two, a[0], a[1])) }));
cfn!(h_each_key, |c, slf, a, blk| enum_or(c, slf, "each_key", a, blk, |c| { each_entry(c, slf, |c, k, _v| { yield1(c, blk, k)?; Ok(true) })?; Ok(slf) }));
cfn!(h_each_value, |c, slf, a, blk| enum_or(c, slf, "each_value", a, blk, |c| { each_entry(c, slf, |c, _k, v| { yield1(c, blk, v)?; Ok(true) })?; Ok(slf) }));
cfn!(h_each_with_index, |c, slf, a, blk| if crate::builtins::enum_native::each_overridden(c, slf, h_each) { crate::builtins::enum_native::call_enumerable(c, slf, "each_with_index", a, blk) } else { enum_or(c, slf, "each_with_index", a, blk, |c| {
    let mut i = 0;
    each_entry(c, slf, |c, k, v| { let p = c.ary_new_from(&[k, v]); yield_block(c, blk, &[p, Value::fix(i)])?; i += 1; Ok(true) })?; Ok(slf)
}) });
cfn!(h_map, |c, slf, a, blk| if crate::builtins::enum_native::each_overridden(c, slf, h_each) { crate::builtins::enum_native::call_enumerable(c, slf, "map", a, blk) } else { enum_or(c, slf, "map", a, blk, |c| { let out = c.ary_new(); let two = map_wants_two(c, blk); each_entry(c, slf, |c, k, v| { let r = yield_kv(c, blk, two, k, v)?; c.ary_push(out, r); Ok(true) })?; Ok(out) }) });
cfn!(h_flat_map, |c, slf, a, blk| if crate::builtins::enum_native::each_overridden(c, slf, h_each) { crate::builtins::enum_native::call_enumerable(c, slf, "flat_map", a, blk) } else { enum_or(c, slf, "flat_map", a, blk, |c| { let out = c.ary_new(); let two = pair_two(c, blk); each_entry(c, slf, |c, k, v| { let r = yield_kv(c, blk, two, k, v)?; if is_array(r) { c.ary_concat(out, r); } else { c.ary_push(out, r); } Ok(true) })?; Ok(out) }) });
/// rb_hash_select / reject: the copy is iterated (and pruned), so the receiver may change meanwhile.
fn sel(c: &mut Ctx, slf: Value, blk: Blk, want: bool) -> RV {
    let out = dup_with_identity(c, slf);
    sel_b(c, out, blk, want)?;
    Ok(out)
}
cfn!(h_select, |c, slf, a, blk| enum_or(c, slf, "select", a, blk, |c| sel(c, slf, blk, true)));
cfn!(h_reject, |c, slf, a, blk| enum_or(c, slf, "reject", a, blk, |c| sel(c, slf, blk, false)));
fn sel_b(c: &mut Ctx, slf: Value, blk: Blk, want: bool) -> Result<bool, Esc> {
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
cfn!(h_filter_map, |c, slf, a, blk| if crate::builtins::enum_native::each_overridden(c, slf, h_each) { crate::builtins::enum_native::call_enumerable(c, slf, "filter_map", a, blk) } else { enum_or(c, slf, "filter_map", a, blk, |c| { let out = c.ary_new(); let two = pair_two(c, blk); each_entry(c, slf, |c, k, v| { let r = yield_kv(c, blk, two, k, v)?; if r.truthy() { c.ary_push(out, r); } Ok(true) })?; Ok(out) }) });
cfn!(h_find, |c, slf, a, blk| enum_or(c, slf, "find", a, blk, |c| { let mut res = NIL; each_entry(c, slf, |c, k, v| { if yield_pair(c, blk, k, v)?.truthy() { res = c.ary_new_from(&[k, v]); return Ok(false); } Ok(true) })?; Ok(res) }));
cfn!(h_any, |c, slf, a, blk| {
    check_argc(c, a, 0, 1)?;
    if !a.is_empty() && !blk.is_null() { gvar_warn(c, "given block not used", false)?; }
    if blk.is_null() && a.is_empty() { return Ok(Value::bool(c.hash_len(slf) > 0)); }
    let mut res = false;
    let two = pair_two(c, blk);
    each_entry(c, slf, |c, k, v| { let t = if !a.is_empty() { let p = c.ary_new_from(&[k, v]); crate::rt::srb_eqq(c, a[0], p, &mut CallCache::default())? } else { yield_kv(c, blk, two, k, v)?.truthy() }; if t { res = true; return Ok(false); } Ok(true) })?;
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
    if blk.is_null() { if obj_klass(slf) == c.vm().c_hash.get() { return Ok(slf); } let h = c.hash_dup(slf); obj_set_klass(h, c.vm().c_hash.get()); return Ok(h); }
    let out = c.hash_new();
    each_entry(c, slf, |c, k, v| {
        let r = yield_block(c, blk, &[k, v])?;
        let p = match check_array(c, r)? { Some(p) => p, None => return type_err(c, format!("wrong element type {} (expected array)", class_name_for_err(c, r))) };
        if c.ary_len(p) != 2 { return arg_err(c, format!("element has wrong array length (expected 2, was {})", c.ary_len(p))); }
        let (k, v) = (c.ary_get(p, 0), c.ary_get(p, 1));
        hash_aset(c, out, k, v)?; Ok(true)
    })?;
    Ok(out)
});
cfn!(h_to_hash, |c, slf, a, blk| Ok(slf));
pub fn hash_inspect(c: &mut Ctx, slf: Value) -> RV {
    if c.hash_len(slf) == 0 { return Ok(c.str_new_enc(b"{}", ENC_ASCII)); }
    if !rec_enter(c, slf, s::inspect)? { return Ok(c.str_new_enc(b"{...}", ENC_ASCII)); }
    // inspect_hash: the first key's inspect gives the encoding, the rest is appended (rb_str_buf_append)
    let out = c.str_new_enc(b"{", ENC_BINARY);
    let mut first = true;
    let r = each_entry(c, slf, |c, k, v| {
        if !first { c.rstring_mut(out).b.extend_from_slice(b", "); }
        let ks = if k.is_sym() {
            let nm = crate::symbol::sym_bytes(k.as_sym());
            let enc = crate::builtins::symbol_methods::sym_encoding(k.as_sym());
            // the name is checked in its UTF-8 form (a symbol of another encoding is converted)
            let u8nm = if enc == ENC_UTF8 || enc == ENC_ASCII { Some(nm.to_vec()) } else { crate::builtins::econv::convert_string(crate::builtins::encoding::enc_name(enc), "UTF-8", nm) };
            let simple = u8nm.as_deref().is_some_and(|u| (crate::string::sym_is_simple(u) && !u.ends_with(b"=") || (u.ends_with(b"?") || u.ends_with(b"!"))) && !u.is_empty() && (u[0].is_ascii_alphabetic() || u[0] == b'_' || u[0] >= 0x80));
            // rb_str_symname_p: a non-ASCII name is quoted unless the inspect result encoding is the symbol's
            let printable = crate::string::inspect_resenc() == enc || crate::string::is_ascii(nm);
            let (mut b, e) = if simple && printable { (nm.to_vec(), if nm.is_ascii() { ENC_ASCII } else { enc }) } else {
                let r = crate::string::inspect_resenc();
                (crate::string::inspect_vec(nm, if nm.is_ascii() { ENC_ASCII } else { enc }), if crate::builtins::encoding::ascii_compat(r) { r } else { ENC_ASCII })
            };
            b.extend_from_slice(b": ");
            let e = if b.is_ascii() { ENC_ASCII } else { e };
            c.str_new_enc(&b, e)
        } else {
            let ks = inspect_value(c, k)?;
            if first { let e = c.str_enc(ks); c.rstring_mut(out).enc = e; }
            crate::string::str_append_str(c, out, ks)?;
            c.str_new_enc(b" => ", ENC_ASCII)
        };
        if first && k.is_sym() { let e = c.str_enc(ks); c.rstring_mut(out).enc = e; }
        first = false;
        crate::string::str_append_str(c, out, ks)?;
        let vs = inspect_value(c, v)?;
        crate::string::str_append_str(c, out, vs)?;
        Ok(true)
    });
    rec_leave(c, slf, s::inspect);
    r?;
    c.rstring_mut(out).b.extend_from_slice(b"}");
    Ok(out)
}
cfn!(h_inspect, |c, slf, a, blk| hash_inspect(c, slf));
cfn!(h_eq, |c, slf, a, blk| hash_equal(c, slf, a[0], false));
cfn!(h_eql, |c, slf, a, blk| hash_equal(c, slf, a[0], true));
/// hash_equal: values compared by == or (eql) eql?.
fn hash_equal(c: &mut Ctx, slf: Value, o: Value, eql: bool) -> RV {
    if slf == o { return Ok(TRUE); }
    if !is_hash(o) { if o.is_heap() && respond_to(c, o, s::to_hash, true) { let r = funcall(c, o, s::eq, &[slf], std::ptr::null())?; return Ok(Value::bool(r.truthy())); } return Ok(FALSE); }
    if c.hash_len(slf) != c.hash_len(o) { return Ok(FALSE); }
    if c.hash_len(slf) > 0 && c.hash(slf).identity != c.hash(o).identity { return Ok(FALSE); }
    if !rec_enter(c, slf, s::eq)? { return Ok(TRUE); }
    let mut ok = true;
    let r = each_entry(c, slf, |c, k, v| {
        match hash_get_opt(c, o, k)? {
            Some(w) => { let same = if eql { key_eql(c, v, w)? } else { crate::builtins::array_methods::rb_eq(c, v, w)? }; if !same { ok = false; return Ok(false); } }
            None => { ok = false; return Ok(false); }
        }
        Ok(true)
    });
    rec_leave(c, slf, s::eq);
    r?;
    Ok(Value::bool(ok))
}
cfn!(h_hash, |c, slf, a, blk| {
    let n = c.hash_len(slf) as u64;
    let h = crate::hash::hash_outer(c, slf, n, |c| {
        let mut h = n;
        each_entry(c, slf, |c, k, v| { let kh = crate::hash::ruby_hash(c, k)?; let vh = crate::hash::ruby_hash(c, v)?; h = h.wrapping_add(crate::astrust::hash_merge(kh, vh)); Ok(true) })?;
        Ok(h)
    })?;
    Ok(Value::fix((h >> 2) as i64))
});
cfn!(h_merge, |c, slf, a, blk| {
    let out = c.hash_dup(slf); obj_set_klass(out, c.vm().c_hash.get());
    if obj_klass(slf) != c.vm().c_hash.get() { obj_set_klass(out, real_class_of(c, slf)); }
    h_update(c, out, a, blk)?;
    Ok(out)
});
cfn!(h_update, |c, slf, a, blk| {
    modify(c, slf)?;
    for &x in a {
        let o = match check_hash(c, x)? { Some(o) => o, None => return type_err(c, format!("no implicit conversion of {} into Hash", class_name_for_err(c, x))) };
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
    if c.hash_iterating(slf) { return err(c, c.vm().c_runtime_error.get(), "can't replace hash during iteration"); }
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
    if p != NIL && !is_proc(p) && respond_to(c, p, intern(b"to_proc"), true) {
        let q = funcall(c, p, intern(b"to_proc"), &[], std::ptr::null())?;
        if is_proc(q) { p = q; }
    }
    if p != NIL && !is_proc(p) { return type_err(c, format!("wrong default_proc type {} (expected Proc)", class_name_for_err(c, p))); }
    if p != NIL && c.proc_data(p).is_lambda { let ar = crate::builtins::proc::proc_arity(c, p); if ar != 2 && !(ar < 0 && ar >= -3) { return type_err(c, format!("default_proc takes two arguments (2 for {})", ar)); } }
    let r = c.hash_mut(slf); r.default_proc = p; r.default = NIL; Ok(p)
});
cfn!(h_invert, |c, slf, a, blk| { let out = c.hash_new(); each_entry(c, slf, |c, k, v| { hash_aset(c, out, v, k)?; Ok(true) })?; Ok(out) });
cfn!(h_transform_values, |c, slf, a, blk| enum_or(c, slf, "transform_values", a, blk, |c| { let out = new_like(c, slf); each_entry(c, slf, |c, k, v| { let nv = yield1(c, blk, v)?; hash_set(c, out, k, nv)?; Ok(true) })?; Ok(out) }));
cfn!(h_transform_values_b, |c, slf, a, blk| enum_or(c, slf, "transform_values!", a, blk, |c| { modify(c, slf)?; each_entry(c, slf, |c, k, v| { let nv = yield1(c, blk, v)?; modify(c, slf)?; hash_set(c, slf, k, nv)?; Ok(true) })?; Ok(slf) }));
cfn!(h_transform_keys, |c, slf, a, blk| {
    check_argc(c, a, 0, 1)?;
    if a.is_empty() && blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, intern(b"transform_keys"), a); }
    let map = if a.is_empty() { None } else { Some(to_hash_conv(c, a[0])?) };
    let out = c.hash_new();
    each_entry(c, slf, |c, k, v| {
        let nk = match map { Some(m) => match hash_get_opt(c, m, k)? { Some(x) => x, None => if blk.is_null() { k } else { yield1(c, blk, k)? } }, None => yield1(c, blk, k)? };
        hash_aset(c, out, nk, v)?; Ok(true)
    })?;
    Ok(out)
});
cfn!(h_transform_keys_b, |c, slf, a, blk| {
    check_argc(c, a, 0, 1)?;
    if a.is_empty() && blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, intern(b"transform_keys!"), a); }
    let map = if a.is_empty() { None } else { Some(to_hash_conv(c, a[0])?) };
    modify(c, slf)?;
    // rb_hash_transform_keys_bang: over a snapshot of the pairs; a key is dropped unless it is already a new key
    let pairs: Vec<(Value, Value)> = c.hash_pairs(slf);
    let keep = c.ary_from_vec(pairs.iter().flat_map(|&(k, v)| [k, v]).collect());
    let new_keys = c.hash_new();
    for (k, v) in pairs {
        let nk = match map {
            Some(m) => match hash_get_opt(c, m, k)? { Some(x) => x, None => if blk.is_null() { k } else { yield1(c, blk, k)? } },
            None => yield1(c, blk, k)?,
        };
        if hash_get_opt(c, new_keys, k)?.is_none() { hash_delete(c, slf, k)?; }
        hash_aset(c, slf, nk, v)?;
        hash_aset(c, new_keys, nk, NIL)?;
    }
    std::hint::black_box(keep);
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
cfn!(h_slice, |c, slf, a, blk| { let out = new_like(c, slf); for &k in a { if let Some(v) = hash_get_opt(c, slf, k)? { hash_set(c, out, k, v)?; } } Ok(out) });
cfn!(h_except, |c, slf, a, blk| { let out = dup_with_identity(c, slf); for &k in a { hash_delete(c, out, k)?; } Ok(out) });
cfn!(h_filter_keys_dig, |c, slf, a, blk| {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let v = hash_aref(c, slf, a[0])?;
    if a.len() == 1 || v == NIL { return Ok(v); }
    if !respond_to(c, v, intern(b"dig"), true) { return type_err(c, format!("{} does not have #dig method", class_name_for_err(c, v))); }
    funcall(c, v, intern(b"dig"), &a[1..], std::ptr::null())
});
cfn!(h_shift, |c, slf, a, blk| {
    modify(c, slf)?;
    if c.hash_len(slf) == 0 { return Ok(NIL); }
    // the first live slot, deleted in place: no #hash / #eql? call (CRuby's st_shift)
    let i = c.hash_first_slot(slf);
    let (k, v) = c.hash_entry_at(slf, i).unwrap();
    c.hash_delete_slot(slf, i);
    Ok(c.ary_new_from(&[k, v]))
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
    if c.hash_iterating(slf) { return err(c, c.vm().c_runtime_error.get(), "compare_by_identity during iteration"); }
    c.hash_mut(slf).identity = true;
    hash_rehash(c, slf)?;
    Ok(slf)
});
cfn!(h_compare_by_identity_p, |c, slf, a, blk| Ok(Value::bool(c.hash(slf).identity)));
cfn!(h_rehash, |c, slf, a, blk| { modify(c, slf)?; hash_rehash(c, slf)?; Ok(slf) });
cfn!(h_compact, |c, slf, a, blk| {
    // rb_hash_dup: the class, defaults and identity stay
    let out = c.hash_dup(slf);
    let k = real_class_of(c, slf); obj_set_klass(out, k);
    let mut i = 0;
    while let Some((k, v)) = c.hash_entry_at(out, i) { if k != UNDEF && v == NIL { c.hash_delete_slot(out, i); } i += 1; }
    Ok(out)
});
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
cfn!(h_assoc, |c, slf, a, blk| {
    if c.hash_len(slf) == 0 { return Ok(NIL); }
    let key = a[0];
    // an st table (more than 8 entries): its hash function with == (assoc_cmp)
    if c.hash_len(slf) > 8 && !c.hash(slf).identity {
        let kh = key_hash(c, key)?;
        let mut r = None;
        c.hash_iter_enter(slf);
        let mut i = 0;
        let res: Result<(), Esc> = (|| {
            while i < c.hash_slots(slf) {
                let e = c.hash(slf).entries[i];
                if e.key != UNDEF && e.hash == kh && crate::builtins::array_methods::rb_eq(c, key, e.key)? { r = Some(e.val); break; }
                i += 1;
            }
            Ok(())
        })();
        c.hash_iter_leave(slf);
        res?;
        if let Some(v) = r { return Ok(c.ary_new_from(&[key, v])); }
    }
    let mut r = NIL;
    each_entry(c, slf, |c, k, v| { if crate::builtins::array_methods::rb_eq(c, key, k)? { r = c.ary_new_from(&[k, v]); return Ok(false); } Ok(true) })?;
    Ok(r)
});
cfn!(h_rassoc, |c, slf, a, blk| { let mut r = NIL; each_entry(c, slf, |c, k, v| { if crate::builtins::array_methods::rb_eq(c, v, a[0])? { r = c.ary_new_from(&[k, v]); return Ok(false); } Ok(true) })?; Ok(r) });
cfn!(h_lt_le, |c, slf, a, blk| {
    let o = a[0];
    let mut ok = true;
    each_entry(c, slf, |c, k, v| { match hash_get_opt(c, o, k)? { Some(w) if crate::builtins::array_methods::rb_eq(c, v, w)? => {}, _ => { ok = false; return Ok(false); } } Ok(true) })?;
    Ok(Value::bool(ok))
});
cfn!(h_le, |c, slf, a, blk| { let o = to_hash_conv(c, a[0])?; h_lt_le(c, slf, &[o], blk) });
cfn!(h_lt, |c, slf, a, blk| { let o = to_hash_conv(c, a[0])?; if c.hash_len(slf) >= c.hash_len(o) { return Ok(FALSE); } h_lt_le(c, slf, &[o], blk) });
cfn!(h_ge, |c, slf, a, blk| { let o = to_hash_conv(c, a[0])?; h_lt_le(c, o, &[slf], blk) });
cfn!(h_gt, |c, slf, a, blk| { let o = to_hash_conv(c, a[0])?; if c.hash_len(slf) <= c.hash_len(o) { return Ok(FALSE); } h_lt_le(c, o, &[slf], blk) });
cfn!(h_freeze, |c, slf, a, blk| { obj_set_flags(slf, FL_FROZEN); Ok(slf) });
/// The body of Hash#to_proc's lambda (proc.rs gives it arity 1).
pub fn hash_proc_call(c: &mut Ctx, d: Value, args: &[Value], _b: Blk, _o: Blk) -> RV {
    if args.len() != 1 { return argc_err(c, args.len(), 1, 1); }
    hash_aref(c, d, args[0])
}
cfn!(h_to_proc, |c, slf, a, blk| {
    let b = Block::cfunc(c, hash_proc_call, slf);
    let k = c.vm().c_proc.get();
    let p = c.alloc(k, Body::Proc(Box::new(ProcData { blk: b, is_lambda: true, from_method: NIL })));
    c.proc_data_mut(p).blk.proc_val = p;
    Ok(p)
});
cfn!(h_s_try_convert, |c, slf, a, blk| Ok(check_hash(c, a[0])?.unwrap_or(NIL)));
cfn!(h_s_new, |c, slf, a, blk| {
    let h = c.hash_new(); obj_set_klass(h, slf);
    funcall_kw(c, h, s::initialize, a, blk, kw_arg(c, a).is_some())?;
    Ok(h)
});
cfn!(h_initialize_copy, |c, slf, a, blk| h_replace(c, slf, a, blk));

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

pub fn init(c: &mut Ctx, k: Value) {
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, k, n, f, a);
    define_private_cfunc(c, k, "initialize", h_initialize, -1);
    define_private_cfunc(c, k, "initialize_copy", h_initialize_copy, 1);
    define_singleton_cfunc(c, k, "[]", h_s_aref, -1);
    define_singleton_cfunc(c, k, "try_convert", h_s_try_convert, 1);
    define_singleton_cfunc(c, k, "new", h_s_new, -1);
    d(c, "[]", h_aref, 1); d(c, "[]=", h_aset, 2);  d(c, "fetch", h_fetch, -1); d(c, "key?", h_key_p, 1); d(c, "has_key?", h_key_p, 1);
    d(c, "include?", h_key_p, 1); d(c, "member?", h_key_p, 1); d(c, "value?", h_value_p, 1); d(c, "has_value?", h_value_p, 1); d(c, "key", h_key, 1);
    d(c, "delete", h_delete, 1); d(c, "size", h_size, 0); d(c, "length", h_size, 0); d(c, "empty?", h_empty, 0); d(c, "keys", h_keys, 0);
    d(c, "values", h_values, 0); d(c, "values_at", h_values_at, -1); d(c, "fetch_values", h_fetch_values, -1); d(c, "each", h_each, 0); d(c, "each_pair", h_each, 0);
    d(c, "each_key", h_each_key, 0); d(c, "each_value", h_each_value, 0); d(c, "each_with_index", h_each_with_index, 0); d(c, "map", h_map, 0); d(c, "collect", h_map, 0);
    d(c, "flat_map", h_flat_map, 0); d(c, "select", h_select, 0); d(c, "filter", h_select, 0); d(c, "reject", h_reject, 0); d(c, "select!", h_select_b, 0);
    d(c, "filter!", h_select_b, 0); d(c, "keep_if", h_keep_if, 0); d(c, "reject!", h_reject_b, 0); d(c, "delete_if", h_delete_if, 0); d(c, "filter_map", h_filter_map, 0);
    d(c, "any?", h_any, -1); d(c, "to_a", h_to_a, 0); d(c, "to_h", h_to_h, 0);
    d(c, "to_hash", h_to_hash, 0); d(c, "inspect", h_inspect, 0); d(c, "to_s", h_inspect, 0); d(c, "==", h_eq, 1); d(c, "eql?", h_eql, 1); d(c, "hash", h_hash, 0);
    d(c, "merge", h_merge, -1); d(c, "update", h_update, -1); d(c, "merge!", h_update, -1); d(c, "clear", h_clear, 0); d(c, "replace", h_replace, 1);
    d(c, "default", h_default, -1); d(c, "default=", h_set_default, 1); d(c, "default_proc", h_default_proc, 0); d(c, "default_proc=", h_set_default_proc, 1);
    d(c, "invert", h_invert, 0); d(c, "transform_values", h_transform_values, 0); d(c, "transform_values!", h_transform_values_b, 0);
    d(c, "transform_keys", h_transform_keys, -1); d(c, "transform_keys!", h_transform_keys_b, -1); 
    d(c, "slice", h_slice, -1); d(c, "except", h_except, -1); d(c, "dig", h_filter_keys_dig, -1);
    d(c, "shift", h_shift, 0); d(c, "compare_by_identity", h_compare_by_identity, 0); d(c, "compare_by_identity?", h_compare_by_identity_p, 0);
    d(c, "rehash", h_rehash, 0); d(c, "compact", h_compact, 0); d(c, "compact!", h_compact_b, 0); d(c, "assoc", h_assoc, 1); d(c, "rassoc", h_rassoc, 1);
    d(c, "<=", h_le, 1); d(c, "<", h_lt, 1); d(c, ">=", h_ge, 1); d(c, ">", h_gt, 1); d(c, "freeze", h_freeze, 0); d(c, "to_proc", h_to_proc, 0);
    crate::builtins::object::alias_method(c, k, intern(b"store"), intern(b"[]=")).expect("Hash#store");
    d(c, "deconstruct_keys", h_deconstruct_keys, 1); d(c, "flatten", h_flatten, -1);
    let _ = (h_sort_by_fallback, h_min_by_fallback, h_find, h_all, h_count, h_group_by, h_partition, h_sum, h_min_by, h_sort_by, h_first);
}
