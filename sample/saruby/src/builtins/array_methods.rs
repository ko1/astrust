// Array methods (the hot Enumerable ones natively; the rest come from prelude.rb).
// Heap access goes through the context (heap.rs, docs/safe_heap.md): a payload borrow cannot
// be held across a call that takes `c`, so loops over `slf` re-read its length every step.
#![forbid(unsafe_code)]
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::hash::*;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

macro_rules! cfn { ($(#[$m:meta])* $name:ident, |$c:ident, $s:ident, $a:ident, $b:ident| $body:expr) => {
    $(#[$m])* #[allow(unused_variables)]
    pub fn $name($c: &mut Ctx, $s: Value, $a: &[Value], $b: *const Block) -> RV { $body }
}; }

/// Must an Enumerable method of slf go through its #each (a subclass's or a redefined Array#each)?
#[inline]
fn ary_each_redef(c: &Ctx, slf: Value) -> bool {
    (obj_klass(slf) != c.vm().c_array.get() || crate::rt::redef(crate::rt::R_ARY_EACH)) && crate::builtins::enum_native::each_overridden(c, slf, a_each)
}
fn enum_or<F: FnOnce(&mut Ctx) -> RV>(c: &mut Ctx, slf: Value, mid: &str, a: &[Value], blk: *const Block, f: F) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, intern(mid.as_bytes()), a); }
    f(c)
}

/// Ruby equality (==) with rb_equal's fast paths (identity, then the unredefined builtins).
pub fn rb_eq(c: &mut Ctx, a: Value, b: Value) -> Result<bool, Esc> {
    use crate::rt::*;
    if a == b { return Ok(!(a.is_flonum() && a.flonum_val().is_nan())); }
    if a.is_fixnum() {
        if (b.is_fixnum() || b.is_sym() || b == NIL || b == TRUE || b == FALSE) && !redef(R_INT_EQ) { return Ok(false); }
    } else if a.is_sym() || a == NIL || a == TRUE || a == FALSE {
        if !redef(R_IMM_EQ) { return Ok(false); }
    } else if is_string(a) && is_string(b) && obj_klass(a) == c.vm().c_string.get() && !redef(R_STR_EQ) { return Ok(c.str_eq(a, b)); }
    let r = funcall(c, a, s::eq, &[b], std::ptr::null())?;
    Ok(r.truthy())
}

/// <=> returning an Ordering (raises ArgumentError when incomparable): CRuby's OPTIMIZED_CMP.
pub fn rb_cmp(c: &mut Ctx, a: Value, b: Value) -> Result<std::cmp::Ordering, Esc> {
    use crate::rt::*;
    use std::cmp::Ordering;
    let r = redef_word();
    if a.is_fixnum() && b.is_fixnum() && r & R_INT_CMP == 0 { return Ok(a.as_fix().cmp(&b.as_fix())); }
    if is_string(a) && is_string(b) && r & R_STR_CMP == 0 { return Ok(c.str_bytes(a).cmp(c.str_bytes(b))); }
    if a.is_flonum() && b.is_flonum() && r & R_FLO_CMP == 0 { if let Some(o) = a.flonum_val().partial_cmp(&b.flonum_val()) { return Ok(o); } }
    let r = funcall(c, a, s::cmp, &[b], std::ptr::null())?;
    cmp_result(c, r, a, b).map(|x| if x < 0 { Ordering::Less } else if x > 0 { Ordering::Greater } else { Ordering::Equal })
}
pub fn cmp_result(c: &mut Ctx, r: Value, a: Value, b: Value) -> Result<i64, Esc> {
    if r.is_fixnum() { return Ok(r.as_fix().signum()); }
    if r == NIL {
        let an = class_path(c, real_class_of(c, a));
        let bn = if b.is_heap() && !is_float(b) { class_path(c, real_class_of(c, b)) } else { inspect_str(c, b)? };
        return arg_err(c, format!("comparison of {} with {} failed", an, bn));
    }
    if is_bignum(r) { return Ok(if c.big(r).neg { -1 } else { 1 }); }
    let z = funcall(c, r, s::gt, &[Value::fix(0)], std::ptr::null())?;
    if z.truthy() { return Ok(1); }
    let z = funcall(c, r, s::lt, &[Value::fix(0)], std::ptr::null())?;
    Ok(if z.truthy() { -1 } else { 0 })
}

/// glibc msort (CRuby sorts through qsort_r): top-down merge, left wins unless cmp(l, r) > 0,
/// so inconsistent comparators see the same call sequence as CRuby.
pub fn sort_values(c: &mut Ctx, v: &mut Vec<Value>, cmp: &mut dyn FnMut(&mut Ctx, Value, Value) -> Result<std::cmp::Ordering, Esc>) -> Result<(), Esc> {
    fn msort(c: &mut Ctx, v: &mut [Value], tmp: &mut [Value], cmp: &mut dyn FnMut(&mut Ctx, Value, Value) -> Result<std::cmp::Ordering, Esc>) -> Result<(), Esc> {
        let n = v.len();
        if n <= 1 { return Ok(()); }
        let n1 = n / 2;
        {
            let (l, r) = v.split_at_mut(n1);
            let (tl, tr) = tmp.split_at_mut(n1);
            msort(c, l, tl, cmp)?;
            msort(c, r, tr, cmp)?;
        }
        let (mut i, mut j, mut k) = (0, n1, 0);
        while i < n1 && j < n {
            if cmp(c, v[i], v[j])? != std::cmp::Ordering::Greater { tmp[k] = v[i]; i += 1; } else { tmp[k] = v[j]; j += 1; }
            k += 1;
        }
        while i < n1 { tmp[k] = v[i]; i += 1; k += 1; }
        // the rest of the right run is already in place
        v[..k].copy_from_slice(&tmp[..k]);
        Ok(())
    }
    if v.len() < 2 { return Ok(()); }
    let mut tmp = v.clone();
    msort(c, v, &mut tmp, cmp)
}

pub fn sort_array(c: &mut Ctx, v: &mut Vec<Value>, blk: *const Block) -> Result<(), Esc> {
    if blk.is_null() {
        use crate::rt::*;
        let r = redef_word();
        if r & R_INT_CMP == 0 && v.iter().all(|x| x.is_fixnum()) { v.sort_by_key(|x| x.as_fix()); return Ok(()); }
        if r & R_STR_CMP == 0 && v.iter().all(|x| is_string(*x)) { v.sort_by(|a, b| c.str_bytes(*a).cmp(c.str_bytes(*b))); return Ok(()); }
        if r & R_FLO_CMP == 0 && v.iter().all(|x| x.is_flonum() && !x.flonum_val().is_nan()) { v.sort_by(|a, b| a.flonum_val().partial_cmp(&b.flonum_val()).unwrap()); return Ok(()); }
        sort_values(c, v, &mut |c, a, b| rb_cmp(c, a, b))
    } else {
        sort_values(c, v, &mut |c, a, b| { let r = yield_block(c, blk, &[a, b])?; cmp_result(c, r, a, b).map(|x| x.cmp(&0)) })
    }
}

/// CRuby's ARY_MAX_SIZE.
const ARY_MAX: i64 = i64::MAX / 8;
fn alloc_check(c: &mut Ctx, n: i64) -> Result<(), Esc> {
    if n > ARY_MAX { return arg_err(c, "argument too big"); }
    if n > 1 << 31 { return err(c, c.vm().c_no_memory_error.get(), "failed to allocate memory"); }
    Ok(())
}
fn norm_index(i: i64, n: usize) -> Option<usize> {
    let i = if i < 0 { i + n as i64 } else { i };
    if i < 0 { None } else { Some(i as usize) }
}

pub fn ary_aref(c: &mut Ctx, slf: Value, a: &[Value]) -> RV {
    check_argc(c, a, 1, 2)?;
    let n = c.ary_len(slf);
    if a.len() == 2 {
        let st = num_to_long(c, a[0])?; let l = num_to_long(c, a[1])?; let n = c.ary_len(slf);
        let st = if st < 0 { st + n as i64 } else { st };
        if st < 0 || st as usize > n || l < 0 { return Ok(NIL); }
        let e = (st as usize + l as usize).min(n);
        return Ok(c.ary_from_vec(c.ary(slf)[st as usize..e].to_vec()));
    }
    let x = a[0];
    if x.is_fixnum() { return Ok(ary_entry(c, slf, x.as_fix())); }
    if is_range(x) {
        return match crate::range::range_beg_len(c, x, n as i64, 0)? {
            Some((st, l)) => { let n = c.ary_len(slf) as i64; if st > n { return Ok(NIL); } Ok(c.ary_from_vec(c.ary(slf)[st as usize..(st + l).min(n) as usize].to_vec())) }
            None => Ok(NIL),
        };
    }
    if crate::builtins::enumerator::is_arith_seq(c, x) { return crate::builtins::enumerator::arith_seq_slice(c, x, slf); }
    let i = num_to_long(c, x)?;
    Ok(ary_entry(c, slf, i))
}

/// Array#[] of an Integer index (also the operator node's).
#[inline]
pub fn ary_entry(c: &Ctx, slf: Value, i: i64) -> Value {
    let v = c.ary(slf);
    // a negative i is out of range as usize: from the end, off the common path
    match v.get(i as usize) { Some(&x) => x, None => ary_entry_neg(v, i) }
}
fn ary_entry_neg(v: &[Value], i: i64) -> Value {
    if i >= 0 { return NIL; }
    v.get(i.wrapping_add(v.len() as i64) as usize).copied().unwrap_or(NIL)
}

/// Array#[]= of an Integer index inside the Array of a non-frozen one; false: ary_store.
#[inline]
pub fn ary_set_in(c: &mut Ctx, slf: Value, i: i64, v: Value) -> bool {
    if obj_flags(slf) & FL_FROZEN != 0 { return false; }
    match c.ary_mut(slf).get_mut(i as usize) { Some(x) => { *x = v; true } None => false }
}

/// Array#[]= of an Integer index (also the operator node's, which raises without a frame of its own, as CRuby).
pub fn ary_store(c: &mut Ctx, slf: Value, i: i64, val: Value) -> RV {
    if ary_set_in(c, slf, i, val) { return Ok(val); }
    c.ary_modify(slf)?;
    let n = c.ary_len(slf);
    let i2 = if i < 0 { i + n as i64 } else { i };
    if i2 < 0 { return err(c, c.vm().c_index_error.get(), format!("index {} too small for array; minimum: -{}", i, n)); }
    if i2 >= ARY_MAX { return err(c, c.vm().c_index_error.get(), format!("index {} too big", i)); }
    if i2 > n as i64 { alloc_check(c, i2 + 1)?; }
    let i2 = i2 as usize;
    let v = c.ary_mut(slf);
    if i2 >= v.len() { v.resize(i2 + 1, NIL); }
    v[i2] = val;
    Ok(val)
}

cfn!(a_s_new, |c, slf, a, blk| crate::builtins::object::c_new(c, slf, a, blk));
cfn!(a_initialize, |c, slf, a, blk| {
    check_argc(c, a, 0, 2)?;
    let v = c.ary_modify(slf)?; v.clear();
    if a.is_empty() { return Ok(slf); }
    if a.len() == 1 && is_array(a[0]) { let src = c.ary(a[0]).to_vec(); *c.ary_mut(slf) = src.into(); return Ok(slf); }
    if a.len() == 1 && !a[0].is_fixnum() { if let Some(arr) = check_array(c, a[0])? { let src = c.ary(arr).to_vec(); *c.ary_mut(slf) = src.into(); return Ok(slf); } }
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative array size"); }
    if n > 1 << 32 { return arg_err(c, "array size too big"); }
    if !blk.is_null() {
        for i in 0..n { let x = yield1(c, blk, Value::fix(i))?; c.ary_push(slf, x); }
    } else {
        let fillv = if a.len() > 1 { a[1] } else { NIL };
        *c.ary_mut(slf) = vec![fillv; n as usize].into();
    }
    Ok(slf)
});
cfn!(a_s_aref, |c, slf, a, blk| { let v = c.ary_new_from(a); obj_set_klass(v, slf); Ok(v) });
cfn!(a_aref, |c, slf, a, blk| ary_aref(c, slf, a));
cfn!(a_aset, |c, slf, a, blk| {
    check_argc(c, a, 2, 3)?;
    c.ary_modify(slf)?;
    let n = c.ary_len(slf);
    let (st, l, val) = if a.len() == 3 {
        let st = num_to_long(c, a[0])?; let l = num_to_long(c, a[1])?; let n = c.ary_len(slf);
        if l < 0 { return err(c, c.vm().c_index_error.get(), format!("negative length ({})", l)); }
        let st2 = if st < 0 { st + n as i64 } else { st };
        if st2 < 0 { return err(c, c.vm().c_index_error.get(), format!("index {} too small for array; minimum: -{}", st, n)); }
        (st2 as usize, l as usize, a[2])
    } else if is_range(a[0]) {
        match crate::range::range_beg_len(c, a[0], n as i64, 1)? { Some((st, l)) => (st as usize, l as usize, a[1]), None => return Ok(a[1]) }
    } else {
        let i = num_to_long(c, a[0])?;
        return ary_store(c, slf, i, a[1]);
    };
    if st as i64 >= ARY_MAX { return err(c, c.vm().c_index_error.get(), format!("index {} too big", st)); }
    if st > c.ary_len(slf) { alloc_check(c, st as i64 + 1)?; }
    // a[st, l] = an Array of l elements inside a: in place (no Vec, a stays inline)
    if is_array(val) && l <= 16 && c.ary_len(val) == l && st + l <= c.ary_len(slf) {
        let mut buf = [NIL; 16];
        buf[..l].copy_from_slice(c.ary(val));
        c.ary_mut(slf)[st..st + l].copy_from_slice(&buf[..l]);
        return Ok(val);
    }
    let repl: Vec<Value> = if is_array(val) { c.ary(val).to_vec() } else { vec![val] };
    let v = c.ary_mut(slf);
    if st > v.len() { v.resize(st, NIL); }
    let e = (st + l).min(v.len());
    v.splice(st..e, repl);
    Ok(val)
});
cfn!(a_at, |c, slf, a, blk| { let i = num_to_long(c, a[0])?; Ok(norm_index(i, c.ary_len(slf)).and_then(|i| c.ary(slf).get(i).copied()).unwrap_or(NIL)) });
cfn!(a_fetch, |c, slf, a, blk| {
    check_argc(c, a, 1, 2)?;
    let i = num_to_long(c, a[0])?;
    match norm_index(i, c.ary_len(slf)).and_then(|i| c.ary(slf).get(i).copied()) {
        Some(v) => Ok(v),
        None => {
            if !blk.is_null() { return yield1(c, blk, a[0]); }
            if a.len() > 1 { return Ok(a[1]); }
            let n = c.ary_len(slf) as i64;
            err(c, c.vm().c_index_error.get(), format!("index {} outside of array bounds: {}...{}", i, -n, n))
        }
    }
});
cfn!(#[inline] a_first, |c, slf, a, blk| {
    if a.is_empty() { return Ok(c.ary_get(slf, 0)); }
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative array size"); }
    Ok(c.ary_from_vec(c.ary(slf).iter().take(n as usize).copied().collect()))
});
cfn!(#[inline] a_last, |c, slf, a, blk| {
    let l = c.ary_len(slf);
    if a.is_empty() { return Ok(if l == 0 { NIL } else { c.ary_get(slf, l - 1) }); }
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative array size"); }
    let n = (n as usize).min(l);
    Ok(c.ary_from_vec(c.ary(slf)[l - n..].to_vec()))
});
// the growth out of line: a baked site runs this cfunc itself (inline_builtin.rs)
cfn!(#[inline] a_push, |c, slf, a, blk| { let v = c.ary_modify(slf)?; if a.len() == 1 { if !v.try_push(a[0]) { push_grow(v, a[0]); } } else { v.extend_from_slice(a); } Ok(slf) });
#[inline(never)]
fn push_grow(v: &mut crate::avec::AVec, x: Value) { v.push(x) }
cfn!(#[inline] a_ltlt, |c, slf, a, blk| { c.ary_modify(slf)?; c.ary_push(slf, a[0]); Ok(slf) });
cfn!(a_pop, |c, slf, a, blk| {
    let v = c.ary_modify(slf)?;
    if a.is_empty() { return Ok(v.pop().unwrap_or(NIL)); }
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative array size"); }
    let v = c.ary_mut(slf); let k = (n as usize).min(v.len());
    let tail = v.split_off(v.len() - k);
    Ok(c.ary_from_vec(tail))
});
cfn!(a_shift, |c, slf, a, blk| {
    let v = c.ary_modify(slf)?;
    if a.is_empty() { return Ok(if v.is_empty() { NIL } else { v.remove(0) }); }
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative array size"); }
    let v = c.ary_mut(slf); let k = (n as usize).min(v.len());
    let head: Vec<Value> = v.drain(..k).collect();
    Ok(c.ary_from_vec(head))
});
cfn!(a_unshift, |c, slf, a, blk| { let v = c.ary_modify(slf)?; v.splice(0..0, a.iter().copied()); Ok(slf) });
cfn!(a_insert, |c, slf, a, blk| {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    c.ary_modify(slf)?;
    let i = num_to_long(c, a[0])?;
    if a.len() == 1 { return Ok(slf); }
    let v = c.ary_modify(slf)?;
    let n = v.len() as i64;
    let pos = if i < 0 { i + n + 1 } else { i };
    if pos < 0 { return err(c, c.vm().c_index_error.get(), format!("index {} too small for array; minimum: -{}", i, n + 1)); }
    let pos = pos as usize;
    if pos > v.len() { v.resize(pos, NIL); }
    v.splice(pos..pos, a[1..].iter().copied());
    Ok(slf)
});
cfn!(#[inline] a_length, |c, slf, a, blk| Ok(Value::fix(c.ary_len(slf) as i64)));
cfn!(#[inline] a_empty, |c, slf, a, blk| Ok(Value::bool(c.ary_len(slf) == 0)));
// The loops of Array's iterators, yielding through `y`: the builtin's block, or at a baked site the block's
// entry compiled in (inline_iter.rs).  The Array is read again at every step (the block may change it).
/// Array#each
#[inline(always)]
pub fn a_each_loop(c: &mut Ctx, slf: Value, y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> RV {
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) { y(c, &[x])?; i += 1; }
    Ok(slf)
}
/// Array#map
#[inline(always)]
pub fn a_map_loop(c: &mut Ctx, slf: Value, y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> RV {
    let out = c.ary_new_capa(c.ary_len(slf));
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) { let r = y(c, &[x])?; c.ary_push(out, r); i += 1; }
    Ok(out)
}
/// Array#select (keep: true) / #reject (false)
#[inline(always)]
pub fn a_select_loop(c: &mut Ctx, slf: Value, keep: bool, y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> RV {
    let out = c.ary_new();
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) {
        if y(c, &[x])?.truthy() == keep {
            // as CRuby: select reads the element again after the block (rb_ary_elt), nil if it is gone
            let v = if keep { c.ary(slf).get(i).copied().unwrap_or(NIL) } else { x };
            c.ary_push(out, v);
        }
        i += 1;
    }
    Ok(out)
}
/// Array#each_with_index
#[inline(always)]
pub fn a_each_wi_loop(c: &mut Ctx, slf: Value, y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> RV {
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) { y(c, &[x, Value::fix(i as i64)])?; i += 1; }
    Ok(slf)
}
/// Array#inject with a block and at most an initial value (None: more arguments)
#[inline(always)]
pub fn a_inject_loop(c: &mut Ctx, slf: Value, a: &[Value], y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> Option<RV> {
    if a.len() > 1 { return None; }
    let (mut acc, mut i) = match a.first() { Some(&v) => (v, 0), None => match c.ary(slf).first() { Some(&x) => (x, 1), None => return Some(Ok(NIL)) } };
    while let Some(&x) = c.ary(slf).get(i) { acc = match y(c, &[acc, x]) { Ok(v) => v, Err(e) => return Some(Err(e)) }; i += 1; }
    Some(Ok(acc))
}
cfn!(a_each, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, s::each, a); }
    a_each_loop(c, slf, &mut |c, a| yield1(c, blk, a[0]))
});
cfn!(a_each_index, |c, slf, a, blk| enum_or(c, slf, "each_index", a, blk, |c| { let mut i = 0; while i < c.ary_len(slf) { yield1(c, blk, Value::fix(i as i64))?; i += 1; } Ok(slf) }));
cfn!(a_each_with_index, |c, slf, a, blk| if ary_each_redef(c, slf) { crate::builtins::enum_native::call_enumerable(c, slf, "each_with_index", a, blk) } else { enum_or(c, slf, "each_with_index", a, blk, |c| a_each_wi_loop(c, slf, &mut |c, a| yield_block(c, blk, a))) });
cfn!(a_reverse_each, |c, slf, a, blk| enum_or(c, slf, "reverse_each", a, blk, |c| { let mut i = c.ary_len(slf); while i > 0 { i -= 1; if i >= c.ary_len(slf) { i = c.ary_len(slf); continue; } let x = c.ary_get(slf, i); yield1(c, blk, x)?; } Ok(slf) }));
cfn!(a_map, |c, slf, a, blk| enum_or(c, slf, "map", a, blk, |c| a_map_loop(c, slf, &mut |c, a| yield1(c, blk, a[0]))));
cfn!(a_map_b, |c, slf, a, blk| enum_or(c, slf, "map!", a, blk, |c| {
    c.ary_modify(slf)?;
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) { let y = yield1(c, blk, x)?; c.ary_modify(slf)?; if i < c.ary_len(slf) { c.ary_mut(slf)[i] = y; } i += 1; }
    Ok(slf)
}));
cfn!(a_select, |c, slf, a, blk| enum_or(c, slf, "select", a, blk, |c| a_select_loop(c, slf, true, &mut |c, a| yield1(c, blk, a[0]))));
cfn!(a_reject, |c, slf, a, blk| enum_or(c, slf, "reject", a, blk, |c| a_select_loop(c, slf, false, &mut |c, a| yield1(c, blk, a[0]))));
fn filter_in_place(c: &mut Ctx, slf: Value, blk: *const Block, keep_truthy: bool) -> Result<bool, Esc> {
    c.ary_modify(slf)?;
    let mut keep = vec![]; let mut i = 0; let mut changed = false;
    let r: Result<(), Esc> = (|| {
        while let Some(&x) = c.ary(slf).get(i) { let t = yield1(c, blk, x)?.truthy(); if t == keep_truthy { keep.push(x); } else { changed = true; } i += 1; }
        Ok(())
    })();
    if r.is_err() { let rest: Vec<Value> = c.ary(slf)[i.min(c.ary_len(slf))..].to_vec(); keep.extend(rest); *c.ary_mut(slf) = keep.into(); return r.map(|_| false); }
    *c.ary_mut(slf) = keep.into();
    Ok(changed)
}
cfn!(a_select_b, |c, slf, a, blk| enum_or(c, slf, "select!", a, blk, |c| { let ch = filter_in_place(c, slf, blk, true)?; Ok(if ch { slf } else { NIL }) }));
cfn!(a_keep_if, |c, slf, a, blk| enum_or(c, slf, "keep_if", a, blk, |c| { filter_in_place(c, slf, blk, true)?; Ok(slf) }));
cfn!(a_reject_b, |c, slf, a, blk| enum_or(c, slf, "reject!", a, blk, |c| { let ch = filter_in_place(c, slf, blk, false)?; Ok(if ch { slf } else { NIL }) }));
cfn!(a_delete_if, |c, slf, a, blk| enum_or(c, slf, "delete_if", a, blk, |c| { filter_in_place(c, slf, blk, false)?; Ok(slf) }));
cfn!(a_index, |c, slf, a, blk| {
    if a.is_empty() && blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"index"), a, NIL); }
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) {
        let hit = if a.is_empty() { yield1(c, blk, x)?.truthy() } else { rb_eq(c, x, a[0])? };
        if hit { return Ok(Value::fix(i as i64)); }
        i += 1;
    }
    Ok(NIL)
});
cfn!(a_rindex, |c, slf, a, blk| {
    if a.is_empty() && blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"rindex"), a, NIL); }
    let mut i = c.ary_len(slf);
    while i > 0 {
        i -= 1;
        if i >= c.ary_len(slf) { continue; }
        let x = c.ary_get(slf, i);
        let hit = if a.is_empty() { yield1(c, blk, x)?.truthy() } else { rb_eq(c, x, a[0])? };
        if hit { return Ok(Value::fix(i as i64)); }
    }
    Ok(NIL)
});
cfn!(a_include, |c, slf, a, blk| {
    let t = a[0];
    let mut i = 0;
    // a fixnum element answers Integer#== without a call (a different fixnum: false)
    let fix = t.is_fixnum() && !crate::rt::redef(crate::rt::R_INT_EQ);
    while let Some(&x) = c.ary(slf).get(i) {
        if x == t { return Ok(TRUE); }
        if !(fix && x.is_fixnum()) && rb_eq(c, x, t)? { return Ok(TRUE); }
        i += 1;
    }
    Ok(FALSE)
});
cfn!(a_delete, |c, slf, a, blk| {
    let mut found = NIL; let mut keep = vec![]; let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) { if rb_eq(c, x, a[0])? { found = x; } else { keep.push(x); } i += 1; }
    if found == NIL && !keep.len() == c.ary_len(slf) { }
    if keep.len() == c.ary_len(slf) { return if blk.is_null() { Ok(NIL) } else { yield1(c, blk, a[0]) }; }
    c.ary_modify(slf)?;
    *c.ary_mut(slf) = keep.into();
    Ok(if found == NIL { a[0] } else { found })
});
cfn!(a_delete_at, |c, slf, a, blk| {
    let i = num_to_long(c, a[0])?;
    let v = c.ary_modify(slf)?;
    match norm_index(i, v.len()) { Some(i) if i < v.len() => Ok(v.remove(i)), _ => Ok(NIL) }
});
cfn!(a_compact, |c, slf, a, blk| Ok(c.ary_from_vec(c.ary(slf).iter().copied().filter(|x| *x != NIL).collect())));
cfn!(a_compact_b, |c, slf, a, blk| { let v = c.ary_modify(slf)?; let n = v.len(); v.retain(|x| *x != NIL); Ok(if v.len() == n { NIL } else { slf }) });
fn flatten_into(c: &mut Ctx, v: Value, depth: i64, out: Value, stack: &mut Vec<Value>) -> Result<bool, Esc> {
    let mut changed = false;
    let n = c.ary_len(v);
    for i in 0..n {
        let x = c.ary_get(v, i);
        let sub = if depth != 0 { if is_array(x) { Some(x) } else if crate::rt::conv_target(x) && !is_hash(x) && !is_string(x) { check_array(c, x)? } else { None } } else { None };
        match sub {
            Some(arr) => {
                if depth < 0 && stack.contains(&arr) { return arg_err(c, "tried to flatten recursive array"); }
                stack.push(arr);
                flatten_into(c, arr, depth - 1, out, stack)?;
                stack.pop();
                changed = true;
            }
            None => c.ary_push(out, x),
        }
    }
    Ok(changed)
}
cfn!(a_flatten, |c, slf, a, blk| {
    let depth = if a.is_empty() || a[0] == NIL { -1 } else { num_to_long(c, a[0])? };
    let out = c.ary_new();
    let mut st = vec![slf];
    flatten_into(c, slf, depth, out, &mut st)?;
    Ok(out)
});
cfn!(a_flatten_b, |c, slf, a, blk| {
    c.ary_modify(slf)?;
    let depth = if a.is_empty() || a[0] == NIL { -1 } else { num_to_long(c, a[0])? };
    let out = c.ary_new();
    let mut st = vec![slf];
    let ch = flatten_into(c, slf, depth, out, &mut st)?;
    *c.ary_mut(slf) = c.ary(out).to_vec().into();
    Ok(if ch { slf } else { NIL })
});
fn uniq_vec(c: &mut Ctx, slf: Value, blk: *const Block) -> Result<Vec<Value>, Esc> {
    let seen = c.hash_new();
    let mut out = vec![];
    let keep = c.ary_new();
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) {
        let k = if blk.is_null() { x } else { yield1(c, blk, x)? };
        c.ary_push(keep, k);
        if hash_add_new(c, seen, k, TRUE)? { out.push(x); }
        i += 1;
    }
    Ok(out)
}
cfn!(a_uniq, |c, slf, a, blk| { let v = uniq_vec(c, slf, blk)?; Ok(c.ary_from_vec(v)) });
cfn!(a_uniq_b, |c, slf, a, blk| { c.ary_modify(slf)?; let v = uniq_vec(c, slf, blk)?; let ch = v.len() != c.ary_len(slf); *c.ary_mut(slf) = v.into(); Ok(if ch { slf } else { NIL }) });
cfn!(a_sort, |c, slf, a, blk| { let mut v = c.ary(slf).to_vec(); let keep = c.ary_from_vec(v.clone()); sort_array(c, &mut v, blk)?; std::hint::black_box(keep); Ok(c.ary_from_vec(v)) });
cfn!(a_sort_b, |c, slf, a, blk| { c.ary_modify(slf)?; let mut v = c.ary(slf).to_vec(); let keep = c.ary_new_from(&v); sort_array(c, &mut v, blk)?; std::hint::black_box(keep); *c.ary_modify(slf)? = v.into(); Ok(slf) });
cfn!(a_sort_by_b, |c, slf, a, blk| enum_or(c, slf, "sort_by!", a, blk, |c| {
    c.ary_modify(slf)?;
    let r = funcall(c, slf, s::sort_by, &[], blk)?;
    *c.ary_mut(slf) = c.ary(r).to_vec().into();
    Ok(slf)
}));
cfn!(a_reverse, |c, slf, a, blk| { let mut v = c.ary(slf).to_vec(); v.reverse(); Ok(c.ary_from_vec(v)) });
cfn!(a_reverse_b, |c, slf, a, blk| { c.ary_modify(slf)?.reverse(); Ok(slf) });
cfn!(a_rotate, |c, slf, a, blk| {
    let k = if a.is_empty() { 1 } else { num_to_long(c, a[0])? };
    let mut v = c.ary(slf).to_vec();
    if !v.is_empty() { let n = v.len() as i64; let r = ((k % n) + n) % n; v.rotate_left(r as usize); }
    Ok(c.ary_from_vec(v))
});
cfn!(a_rotate_b, |c, slf, a, blk| {
    let k = if a.is_empty() { 1 } else { num_to_long(c, a[0])? };
    let v = c.ary_modify(slf)?;
    if !v.is_empty() { let n = v.len() as i64; let r = ((k % n) + n) % n; v.rotate_left(r as usize); }
    Ok(slf)
});
pub fn ary_join(c: &mut Ctx, slf: Value, sep: &[u8], out: &mut Vec<u8>, enc: &mut u8) -> Result<(), Esc> {
    if !rec_enter(c, slf, s::join)? { return arg_err(c, "recursive array join"); }
    let mut i = 0;
    let r: Result<(), Esc> = (|| {
        while i < c.ary_len(slf) {
            if i > 0 { out.extend_from_slice(sep); }
            let x = c.ary_get(slf, i);
            if is_string(x) { out.extend_from_slice(c.str_bytes(x)); if c.rstring(x).enc == ENC_BINARY && !is_ascii(c.str_bytes(x)) { *enc = ENC_BINARY; } }
            else if is_array(x) { ary_join(c, x, sep, out, enc)?; }
            else {
                let conv = crate::rt::conv_target(x);
                let sub = if conv { check_str_conv(c, x)? } else { None };
                if let Some(s2) = sub { out.extend_from_slice(c.str_bytes(s2)); }
                else if let Some(arr) = if conv { check_array(c, x)? } else { None } { ary_join(c, arr, sep, out, enc)?; }
                else { let s2 = crate::rt::srb_tostr(c, x)?; out.extend_from_slice(c.str_bytes(s2)); }
            }
            i += 1;
        }
        Ok(())
    })();
    rec_leave(c, slf, s::join);
    r
}
fn check_str_conv(c: &mut Ctx, x: Value) -> Result<Option<Value>, Esc> {
    if respond_to(c, x, s::to_str, true) { return Ok(Some(expect_str(c, x)?)); }
    Ok(None)
}
cfn!(a_join, |c, slf, a, blk| {
    check_argc(c, a, 0, 1)?;
    if c.ary_len(slf) == 0 { return Ok(c.str_new(b"")); }
    let sepv = if a.is_empty() || a[0] == NIL { gvar_get(c, intern(b"$,"))? } else { a[0] };
    let sep = if sepv == NIL { vec![] } else { let s2 = expect_str(c, sepv)?; c.str_bytes(s2).to_vec() };
    let mut out = vec![]; let mut enc = ENC_UTF8;
    // rb_ary_join's first pass: the first non-String is asked for to_str before anything is joined
    if let Some(&x) = c.ary(slf).iter().find(|x| !is_string(**x)) {
        if !is_array(x) && crate::rt::conv_target(x) && respond_to(c, x, s::to_str, true) { funcall(c, x, s::to_str, &[], std::ptr::null())?; }
    }
    ary_join(c, slf, &sep, &mut out, &mut enc)?;
    Ok(c.str_from_vec_enc(out, enc))
});
cfn!(a_to_a, |c, slf, a, blk| if obj_klass(slf) == c.vm().c_array.get() { Ok(slf) } else { Ok(c.ary_from_vec(c.ary(slf).to_vec())) });
cfn!(a_to_ary, |c, slf, a, blk| Ok(slf));
cfn!(a_entries, |c, slf, a, blk| Ok(c.ary_from_vec(c.ary(slf).to_vec())));
cfn!(a_to_h, |c, slf, a, blk| {
    let h = c.hash_new();
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) {
        let pair = if blk.is_null() { x } else { yield1(c, blk, x)? };
        let p = match check_array(c, pair)? { Some(p) => p, None => return type_err(c, format!("wrong element type {} at {} (expected array)", class_name_for_err(c, pair), i)) };
        if c.ary_len(p) != 2 { return arg_err(c, format!("wrong array length at {} (expected 2, was {})", i, c.ary_len(p))); }
        hash_aset(c, h, c.ary_get(p, 0), c.ary_get(p, 1))?;
        i += 1;
    }
    Ok(h)
});
pub fn ary_inspect(c: &mut Ctx, slf: Value) -> RV {
    if c.ary_len(slf) == 0 { return Ok(c.str_new(b"[]")); }
    if !rec_enter(c, slf, s::inspect)? { return Ok(c.str_new(b"[...]")); }
    let mut out = vec![b'['];
    let mut i = 0;
    let r: Result<(), Esc> = (|| {
        while i < c.ary_len(slf) {
            if i > 0 { out.extend_from_slice(b", "); }
            let s2 = inspect_value(c, c.ary_get(slf, i))?;
            out.extend_from_slice(c.str_bytes(s2));
            i += 1;
        }
        Ok(())
    })();
    rec_leave(c, slf, s::inspect);
    r?;
    out.push(b']');
    Ok(c.str_from_vec(out))
}
cfn!(a_inspect, |c, slf, a, blk| ary_inspect(c, slf));
cfn!(a_eq, |c, slf, a, blk| {
    let o = a[0];
    if slf == o { return Ok(TRUE); }
    if !is_array(o) { if crate::builtins::kernel::rb_respond_to(c, o, s::to_ary)? { let r = funcall(c, o, s::eq, &[slf], std::ptr::null())?; return Ok(Value::bool(r.truthy())); } return Ok(FALSE); }
    if c.ary_len(slf) != c.ary_len(o) { return Ok(FALSE); }
    if !rec_enter(c, slf, s::eq)? { return Ok(TRUE); }
    let mut i = 0;
    let r: Result<bool, Esc> = (|| {
        while i < c.ary_len(slf).min(c.ary_len(o)) { if !rb_eq(c, c.ary_get(slf, i), c.ary_get(o, i))? { return Ok(false); } i += 1; }
        Ok(c.ary_len(slf) == c.ary_len(o))
    })();
    rec_leave(c, slf, s::eq);
    Ok(Value::bool(r?))
});
cfn!(a_eql, |c, slf, a, blk| {
    let o = a[0];
    if slf == o { return Ok(TRUE); }
    if !is_array(o) || c.ary_len(slf) != c.ary_len(o) { return Ok(FALSE); }
    if !rec_enter(c, slf, s::eql_p)? { return Ok(TRUE); }
    let mut i = 0;
    let r: Result<bool, Esc> = (|| {
        while i < c.ary_len(slf).min(c.ary_len(o)) { if !rb_eql(c, c.ary_get(slf, i), c.ary_get(o, i))? { return Ok(false); } i += 1; }
        Ok(c.ary_len(slf) == c.ary_len(o))
    })();
    rec_leave(c, slf, s::eql_p);
    Ok(Value::bool(r?))
});
cfn!(a_hash, |c, slf, a, blk| { let h = ary_hash(c, slf)?; Ok(Value::fix((h >> 2) as i64)) });
cfn!(a_cmp, |c, slf, a, blk| {
    let o = if is_array(a[0]) { a[0] } else if a[0].is_heap() { match check_array(c, a[0])? { Some(o) => o, None => return Ok(NIL) } } else { return Ok(NIL) };
    if slf == o { return Ok(Value::fix(0)); }
    // recursive_cmp under rb_exec_recursive_paired: a recursive pair compares by length
    if rec_enter(c, slf, s::cmp)? {
        let r = (|| -> RV {
            let mut i = 0;
            while i < c.ary_len(slf) && i < c.ary_len(o) {
                let r = funcall(c, c.ary_get(slf, i), s::cmp, &[c.ary_get(o, i)], std::ptr::null())?;
                if r != Value::fix(0) { return Ok(r); }
                i += 1;
            }
            Ok(UNDEF)
        })();
        rec_leave(c, slf, s::cmp);
        let r = r?;
        if r != UNDEF { return Ok(r); }
    }
    Ok(Value::fix((c.ary_len(slf) as i64 - c.ary_len(o) as i64).signum()))
});
cfn!(a_plus, |c, slf, a, blk| { let o = expect_array(c, a[0])?; let mut v = c.ary(slf).to_vec(); v.extend_from_slice(c.ary(o)); Ok(c.ary_from_vec(v)) });
/// rb_eql: identity, opt_equality_specialized (unless == is redefined), then #eql?.
pub fn rb_eql(c: &mut Ctx, a: Value, b: Value) -> Result<bool, Esc> {
    use crate::rt::{redef, R_INT_EQ, R_FLO_EQ, R_IMM_EQ, R_STR_EQ};
    if a == b { return Ok(true); }
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_EQ) { return Ok(false); }
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_EQ) { return Ok(false); }
    if a.is_sym() && b.is_sym() && !redef(R_IMM_EQ) { return Ok(false); }
    if a.is_heap() {
        let k = obj_klass(a);
        if k == c.vm().c_float.get() && is_float(b) && !redef(R_FLO_EQ) { return Ok(float_val(a) == float_val(b)); }
        if k == c.vm().c_string.get() && is_string(b) && !redef(R_STR_EQ) { return Ok(c.str_bytes(a) == c.str_bytes(b)); }
    }
    Ok(funcall(c, a, s::eql_p, &[b], std::ptr::null())?.truthy())
}
/// rb_ary_includes_by_eql
fn includes_by_eql(c: &mut Ctx, ary: Value, x: Value) -> Result<bool, Esc> {
    let mut i = 0;
    while i < c.ary_len(ary) { let e = c.ary_get(ary, i); if rb_eql(c, x, e)? { return Ok(true); } i += 1; }
    Ok(false)
}
// array.c's set operations: arrays up to SMALL_ARRAY_LEN are scanned with rb_eql, longer ones hashed
const SMALL_ARRAY_LEN: usize = 16;
fn ary_set_hash(c: &mut Ctx, o: Value) -> RV {
    let h = c.hash_new();
    let mut i = 0;
    while i < c.ary_len(o) { let x = c.ary_get(o, i); hash_add_new(c, h, x, x)?; i += 1; }
    Ok(h)
}
cfn!(a_minus, |c, slf, a, blk| {
    let o = expect_array(c, a[0])?;
    if c.ary_len(o) == 0 { return Ok(c.ary_from_vec(c.ary(slf).to_vec())); }
    let out = c.ary_new();
    if c.ary_len(slf) <= SMALL_ARRAY_LEN || c.ary_len(o) <= SMALL_ARRAY_LEN {
        let mut i = 0;
        while let Some(&x) = c.ary(slf).get(i) { if !includes_by_eql(c, o, x)? { c.ary_push(out, x); } i += 1; }
        return Ok(out);
    }
    let h = ary_set_hash(c, o)?;
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) { if hash_lookup(c, h, x)?.is_none() { c.ary_push(out, x); } i += 1; }
    Ok(out)
});
// rb_ary_difference_multi
cfn!(a_difference, |c, slf, a, blk| {
    let len = c.ary_len(slf);
    let mut others = Vec::with_capacity(a.len());
    let keep = c.ary_new();   // roots the hashes
    for &x in a {
        let o = expect_array(c, x)?;
        let e = if len > SMALL_ARRAY_LEN && c.ary_len(o) > SMALL_ARRAY_LEN { (ary_set_hash(c, o)?, true) } else { (o, false) };
        c.ary_push(keep, e.0);
        others.push(e);
    }
    let out = c.ary_new();
    let mut i = 0;
    'el: while let Some(&x) = c.ary(slf).get(i) {
        i += 1;
        for &(o, hashed) in &others {
            if if hashed { hash_lookup(c, o, x)?.is_some() } else { includes_by_eql(c, o, x)? } { continue 'el; }
        }
        c.ary_push(out, x);
    }
    std::hint::black_box(keep);
    Ok(out)
});
cfn!(a_mul, |c, slf, a, blk| {
    if is_string(a[0]) { return a_join(c, slf, a, blk); }
    if a[0].is_heap() && !a[0].is_fixnum() { if let Some(sv) = check_str_conv(c, a[0])? { return a_join(c, slf, &[sv], blk); } }
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative argument"); }
    let l = c.ary_len(slf) as i64;
    if l > 0 { match l.checked_mul(n) { Some(t) => alloc_check(c, t)?, None => return arg_err(c, "argument too big") } }
    let v = c.ary(slf);
    Ok(c.ary_from_vec(v.iter().copied().cycle().take(v.len() * n as usize).collect()))
});
cfn!(a_and, |c, slf, a, blk| {
    let o = expect_array(c, a[0])?;
    let out = c.ary_new();
    if c.ary_len(slf) == 0 || c.ary_len(o) == 0 { return Ok(out); }
    if c.ary_len(slf) <= SMALL_ARRAY_LEN && c.ary_len(o) <= SMALL_ARRAY_LEN {
        let mut i = 0;
        while let Some(&x) = c.ary(slf).get(i) {
            i += 1;
            if !includes_by_eql(c, o, x)? || includes_by_eql(c, out, x)? { continue; }
            c.ary_push(out, x);
        }
        return Ok(out);
    }
    let h = ary_set_hash(c, o)?;
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) { i += 1; if hash_delete(c, h, x)?.is_some() { c.ary_push(out, x); } }
    Ok(out)
});
cfn!(a_intersection, |c, slf, a, blk| { let mut cur = c.ary_from_vec(c.ary(slf).to_vec()); for &x in a { cur = a_and(c, cur, &[x], blk)?; } Ok(cur) });
cfn!(a_intersect_p, |c, slf, a, blk| {
    let o = expect_array(c, a[0])?;
    if c.ary_len(slf) == 0 || c.ary_len(o) == 0 { return Ok(FALSE); }
    if c.ary_len(slf) <= SMALL_ARRAY_LEN && c.ary_len(o) <= SMALL_ARRAY_LEN {
        let mut i = 0;
        while let Some(&x) = c.ary(slf).get(i) { i += 1; if includes_by_eql(c, o, x)? { return Ok(TRUE); } }
        return Ok(FALSE);
    }
    let (short, long) = if c.ary_len(slf) > c.ary_len(o) { (o, slf) } else { (slf, o) };
    let h = ary_set_hash(c, short)?;
    let mut i = 0;
    while let Some(&x) = c.ary(long).get(i) { i += 1; if hash_lookup(c, h, x)?.is_some() { return Ok(TRUE); } }
    Ok(FALSE)
});
/// rb_ary_union: append the elements of `ary` not yet in `out` (rb_eql scan)
fn union_scan(c: &mut Ctx, out: Value, ary: Value) -> Result<(), Esc> {
    let mut i = 0;
    while let Some(&x) = c.ary(ary).get(i) { i += 1; if !includes_by_eql(c, out, x)? { c.ary_push(out, x); } }
    Ok(())
}
cfn!(a_or, |c, slf, a, blk| a_union(c, slf, &a[..1], blk));
cfn!(a_union, |c, slf, a, blk| {
    let mut os = Vec::with_capacity(a.len());
    let mut sum = c.ary_len(slf);
    let keep = c.ary_new();   // roots converted arguments
    for &x in a { let o = expect_array(c, x)?; sum += c.ary_len(o); c.ary_push(keep, o); os.push(o); }
    let r = if sum <= SMALL_ARRAY_LEN {
        let out = c.ary_new();
        union_scan(c, out, slf)?;
        for &o in &os { union_scan(c, out, o)?; }
        out
    } else {
        let mut v = c.ary(slf).to_vec();
        for &o in &os { v.extend_from_slice(c.ary(o)); }
        let all = c.ary_from_vec(v);
        a_uniq(c, all, &[], std::ptr::null())?
    };
    std::hint::black_box(keep);
    Ok(r)
});
cfn!(a_concat, |c, slf, a, blk| {
    c.ary_modify(slf)?;
    let mut add = vec![];
    for &x in a { let o = expect_array(c, x)?; add.extend_from_slice(c.ary(o)); }
    c.ary_mut(slf).extend(add);
    Ok(slf)
});
cfn!(a_clear, |c, slf, a, blk| { c.ary_modify(slf)?.clear(); Ok(slf) });
cfn!(a_replace, |c, slf, a, blk| {
    let o = expect_array(c, a[0])?;
    c.ary_modify(slf)?;
    let (dst, others) = c.ary_mut_and(slf);
    if let Some(src) = others.ary(o) { dst.clear(); dst.extend_from_slice(src); }
    Ok(slf)
});
cfn!(a_fill, |c, slf, a, blk| {
    c.ary_modify(slf)?;
    let (val, rest) = if blk.is_null() { if a.is_empty() { return argc_err(c, 0, 1, 3); } (a[0], &a[1..]) } else { (NIL, a) };
    let n = c.ary_len(slf) as i64;
    let (st, e) = if rest.is_empty() { (0, n) } else if is_range(rest[0]) {
        match crate::range::range_beg_len(c, rest[0], n, 1)? { Some((s2, l)) => (s2, s2 + l), None => return Ok(slf) }
    } else {
        let s2 = if rest[0] == NIL { 0 } else { num_to_long(c, rest[0])? };
        let s2 = if s2 < 0 { (s2 + n).max(0) } else { s2 };
        let l = if rest.len() > 1 && rest[1] != NIL { num_to_long(c, rest[1])? } else { n - s2 };
        (s2, s2 + l.max(0))
    };
    if e > n { alloc_check(c, e)?; }
    for i in st..e {
        let v = if blk.is_null() { val } else { yield1(c, blk, Value::fix(i))? };
        let arr = c.ary_mut(slf);
        if i as usize >= arr.len() { arr.resize(i as usize + 1, NIL); }
        arr[i as usize] = v;
    }
    Ok(slf)
});
cfn!(a_slice_b, |c, slf, a, blk| {
    c.ary_modify(slf)?;
    let n = c.ary_len(slf);
    if a.len() == 2 {
        let st = num_to_long(c, a[0])?; let l = num_to_long(c, a[1])?; let n = c.ary_len(slf);
        let st = if st < 0 { st + n as i64 } else { st };
        if st < 0 || st as usize > n || l < 0 { return Ok(NIL); }
        let e = (st as usize + l as usize).min(n);
        let removed: Vec<Value> = c.ary_mut(slf).drain(st as usize..e).collect();
        return Ok(c.ary_from_vec(removed));
    }
    if is_range(a[0]) {
        return match crate::range::range_beg_len(c, a[0], n as i64, 0)? {
            Some((st, l)) => { let n = c.ary_len(slf) as i64; if st > n { return Ok(NIL); } let e = (st + l).min(n) as usize; let removed: Vec<Value> = c.ary_mut(slf).drain(st as usize..e).collect(); Ok(c.ary_from_vec(removed)) }
            None => Ok(NIL),
        };
    }
    let i = num_to_long(c, a[0])?;
    match norm_index(i, n) { Some(i) if i < n => Ok(c.ary_mut(slf).remove(i)), _ => Ok(NIL) }
});
pub fn minmax_impl(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block, want_max: bool) -> RV {
    let cmpf = |c: &mut Ctx, x: Value, y: Value| -> Result<std::cmp::Ordering, Esc> {
        if blk.is_null() { rb_cmp(c, x, y) } else { let r = yield_block(c, blk, &[x, y])?; cmp_result(c, r, x, y).map(|v| v.cmp(&0)) }
    };
    if !a.is_empty() && a[0] != NIL {
        let Some(mut d) = Nmin::new(c, a[0], false, want_max, blk)? else { return Ok(c.ary_new()); };
        let mut i = 0;
        while let Some(&x) = c.ary(slf).get(i) { d.add(c, x, NIL)?; i += 1; }
        return d.finish(c);
    }
    let mut best = NIL; let mut first = true;
    let mut i = 0;
    if blk.is_null() && c.ary(slf).first().is_some_and(|v| v.is_fixnum()) && !crate::rt::redef(crate::rt::R_INT_CMP) {
        // ary_max_opt_fixnum / ary_min_opt_fixnum: up to the first non-fixnum, then the generic loop
        let v = c.ary(slf);
        best = v[0]; first = false; i = 1;
        while let Some(&x) = v.get(i) {
            if !x.is_fixnum() { break; }
            if (want_max && x.as_fix() > best.as_fix()) || (!want_max && x.as_fix() < best.as_fix()) { best = x; }
            i += 1;
        }
    }
    while let Some(&x) = c.ary(slf).get(i) {
        if first { best = x; first = false; }
        else if blk.is_null() {
            // ary_max_generic / ary_min_generic: the current best is the receiver of <=>
            let o = rb_cmp(c, best, x)?; if (want_max && o.is_lt()) || (!want_max && o.is_gt()) { best = x; }
        } else { let o = cmpf(c, x, best)?; if (want_max && o.is_gt()) || (!want_max && o.is_lt()) { best = x; } }
        i += 1;
    }
    Ok(best)
}
/// rb_nmin_run (enum.c): the n smallest (rev: largest) of the values given to `add`, compared in
/// CRuby's order (a quickselect over a buffer of 4n, then a sort).  by: the values come with keys.
pub struct Nmin { n: usize, bufmax: usize, buf: Value, curlen: usize, limit: Option<Value>, rev: bool, by: bool, blk: *const Block }
impl Nmin {
    /// None when n is 0 (nothing to collect, as CRuby returns before iterating).
    pub fn new(c: &mut Ctx, num: Value, by: bool, rev: bool, blk: *const Block) -> Result<Option<Nmin>, Esc> {
        let n = num_to_long(c, num)?;
        if n < 0 { return arg_err(c, format!("negative size ({})", n)); }
        if n == 0 { return Ok(None); }
        if i64::MAX / 4 / (if by { 2 } else { 1 }) < n { return arg_err(c, "too big size"); }
        let buf = c.ary_new();
        Ok(Some(Nmin { n: n as usize, bufmax: n as usize * 4, buf, curlen: 0, limit: None, rev, by, blk }))
    }
    /// The state as a Ruby Array, for a block function that gets only a Value (enum_native.rs);
    /// the block is not in it: `load` takes the Cfunc block's outer block.
    pub fn save(&self, c: &mut Ctx) -> Value {
        let l = self.limit.unwrap_or(NIL);
        let fl = (self.limit.is_some() as i64) | (self.rev as i64) << 1 | (self.by as i64) << 2;
        c.ary_new_from(&[self.buf, Value::fix(self.n as i64), Value::fix(self.curlen as i64), l, Value::fix(fl)])
    }
    pub fn load(c: &Ctx, d: Value, blk: *const Block) -> Nmin {
        let n = c.ary_get(d, 1).as_fix() as usize;
        let fl = c.ary_get(d, 4).as_fix();
        Nmin { n, bufmax: n * 4, buf: c.ary_get(d, 0), curlen: c.ary_get(d, 2).as_fix() as usize,
               limit: if fl & 1 != 0 { Some(c.ary_get(d, 3)) } else { None }, rev: fl & 2 != 0, by: fl & 4 != 0, blk }
    }
    pub fn store(&self, c: &mut Ctx, d: Value) {
        let fl = (self.limit.is_some() as i64) | (self.rev as i64) << 1 | (self.by as i64) << 2;
        let a = c.ary_mut(d);
        a[2] = Value::fix(self.curlen as i64); a[3] = self.limit.unwrap_or(NIL); a[4] = Value::fix(fl);
    }
    pub fn blk(&self) -> *const Block { self.blk }
    fn cmp(&self, c: &mut Ctx, a: Value, b: Value) -> Result<i64, Esc> {
        if !self.by && !self.blk.is_null() { let r = yield_block(c, self.blk, &[a, b])?; return cmp_result(c, r, a, b); }
        Ok(rb_cmp(c, a, b)? as i64)
    }
    pub fn add(&mut self, c: &mut Ctx, v: Value, key: Value) -> Result<(), Esc> {
        let cv = if self.by { key } else { v };
        if let Some(l) = self.limit {
            let k = self.cmp(c, cv, l)?;
            if (if self.rev { -k } else { k }) >= 0 { return Ok(()); }
        }
        if self.by { c.ary_push(self.buf, key); }
        c.ary_push(self.buf, v);
        self.curlen += 1;
        if self.curlen == self.bufmax { self.filter(c)?; }
        Ok(())
    }
    fn filter(&mut self, c: &mut Ctx) -> Result<(), Esc> {
        if self.curlen <= self.n { return Ok(()); }
        let (n, w, buf) = (self.n, if self.by { 2 } else { 1 }, self.buf);
        let swap = |c: &mut Ctx, i: usize, j: usize| { let b = c.ary_mut(buf); for k in 0..w { b.swap(i * w + k, j * w + k); } };
        let (mut left, mut right) = (0usize, self.curlen - 1);
        let store_index;
        loop {
            let mut pivot = left + (right - left) / 2;
            let mut pivots = 1;
            swap(c, pivot, right);
            pivot = right;
            let mut store = left;
            let mut i = left;
            while i + pivots <= right {
                let (x, y) = (c.ary_get(buf, i * w), c.ary_get(buf, pivot * w));
                let k = self.cmp(c, x, y)?;
                let k = if self.rev { -k } else { k };
                if k == 0 { swap(c, i, right - pivots); pivots += 1; continue; }
                if k < 0 { swap(c, i, store); store += 1; }
                i += 1;
            }
            let mut j = store;
            let mut i = right;
            while right < i + pivots { if i <= j { break; } swap(c, j, i); j += 1; i -= 1; }
            if store <= n && n <= store + pivots { store_index = store; break; }
            if n < store { right = store - 1; } else { left = store + pivots; }
        }
        self.limit = Some(c.ary_get(buf, store_index * w));
        self.curlen = n;
        c.ary_mut(buf).truncate(n * w);
        Ok(())
    }
    pub fn finish(mut self, c: &mut Ctx) -> RV {
        self.filter(c)?;
        let (buf, w) = (self.buf, if self.by { 2 } else { 1 });
        let mut idx: Vec<Value> = (0..c.ary_len(buf) / w).map(|i| Value::fix(i as i64)).collect();
        sort_values(c, &mut idx, &mut |c, x, y| {
            let (a, b) = (c.ary_get(buf, x.as_fix() as usize * w), c.ary_get(buf, y.as_fix() as usize * w));
            Ok(self.cmp(c, a, b)?.cmp(&0))
        })?;
        let mut out: Vec<Value> = idx.iter().map(|i| c.ary_get(buf, i.as_fix() as usize * w + w - 1)).collect();
        if self.rev { out.reverse(); }
        Ok(c.ary_from_vec(out))
    }
}
cfn!(a_min, |c, slf, a, blk| minmax_impl(c, slf, a, blk, false));
cfn!(a_max, |c, slf, a, blk| minmax_impl(c, slf, a, blk, true));
cfn!(a_sum, |c, slf, a, blk| {
    let mut acc = if a.is_empty() { Value::fix(0) } else { a[0] };
    let mut fsum: Option<(f64, f64)> = None; // Kahan-Babuska for floats
    let mut i = 0;
    // a run of fixnums with no block runs no Ruby code: one borrow of the elements for the whole run
    if blk.is_null() && acc.is_fixnum() {
        let mut sum = acc.as_fix();
        for &x in c.ary(slf) {
            if !x.is_fixnum() { break; }
            match sum.checked_add(x.as_fix()) { Some(r) if Value::fixable(r) => { sum = r; i += 1; } _ => break }
        }
        acc = Value::fix(sum);
    }
    'elems: while let Some(&x0) = c.ary(slf).get(i) {
        let mut x = x0;
        if !blk.is_null() { x = yield1(c, blk, x)?; }
        // the same x again after switching to the float sum: its block must not run twice
        loop {
        if let Some((s2, comp)) = fsum.as_mut() {
            if let Some(f) = c.num_to_f64(x) {
                // CRuby's float_sum: NaN / Inf short-circuit the compensation
                if s2.is_nan() {}
                else if f.is_nan() { *s2 = f; }
                else if f.is_infinite() { *s2 = if s2.is_infinite() && s2.is_sign_negative() != f.is_sign_negative() { f64::NAN } else { f }; }
                else if s2.is_infinite() {}
                else { let t = *s2 + f; if s2.abs() >= f.abs() { *comp += (*s2 - t) + f; } else { *comp += (f - t) + *s2; } *s2 = t; }
                i += 1; continue 'elems;
            }
            acc = c.float_new(*s2 + *comp); fsum = None;
        }
        if acc.is_fixnum() && x.is_fixnum() { if let Some(r) = acc.as_fix().checked_add(x.as_fix()) { if Value::fixable(r) { acc = Value::fix(r); i += 1; continue 'elems; } } }
        if is_float(x) && crate::numeric::is_numeric(acc) && fsum.is_none() { fsum = Some((c.num_to_f64(acc).unwrap(), 0.0)); continue; }
        acc = funcall(c, acc, s::plus, &[x], std::ptr::null())?;
        i += 1;
        continue 'elems;
        }
    }
    if let Some((s2, comp)) = fsum { return Ok(c.float_new(if s2.is_finite() { s2 + comp } else { s2 })); }
    Ok(acc)
});
cfn!(a_count, |c, slf, a, blk| {
    if a.is_empty() && blk.is_null() { return Ok(Value::fix(c.ary_len(slf) as i64)); }
    let mut n = 0; let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) { let hit = if !a.is_empty() { rb_eq(c, x, a[0])? } else { yield1(c, blk, x)?.truthy() }; if hit { n += 1; } i += 1; }
    Ok(Value::fix(n))
});
cfn!(a_take, |c, slf, a, blk| { let n = num_to_long(c, a[0])?; if n < 0 { return arg_err(c, "attempt to take negative size"); } Ok(c.ary_from_vec(c.ary(slf).iter().take(n as usize).copied().collect())) });
cfn!(a_drop, |c, slf, a, blk| { let n = num_to_long(c, a[0])?; if n < 0 { return arg_err(c, "attempt to drop negative size"); } Ok(c.ary_from_vec(c.ary(slf).iter().skip(n as usize).copied().collect())) });
cfn!(a_take_while, |c, slf, a, blk| enum_or(c, slf, "take_while", a, blk, |c| { let out = c.ary_new(); let mut i = 0; while let Some(&x) = c.ary(slf).get(i) { if !yield1(c, blk, x)?.truthy() { break; } c.ary_push(out, x); i += 1; } Ok(out) }));
cfn!(a_drop_while, |c, slf, a, blk| enum_or(c, slf, "drop_while", a, blk, |c| { let mut i = 0; while let Some(&x) = c.ary(slf).get(i) { if !yield1(c, blk, x)?.truthy() { break; } i += 1; } Ok(c.ary_from_vec(c.ary(slf)[i.min(c.ary_len(slf))..].to_vec())) }));
cfn!(a_zip, |c, slf, a, blk| {
    if c.ary_len(slf) == 0 { return Ok(if blk.is_null() { c.ary_new() } else { NIL }); }
    let mut others = vec![];
    for &x in a {
        let arr = if is_array(x) { x } else if let Some(arr) = check_array(c, x)? { arr } else if respond_to(c, x, s::each, true) { { let n = Value::fix(c.ary_len(slf) as i64); if respond_to(c, x, intern(b"first"), true) { funcall(c, x, intern(b"first"), &[n], std::ptr::null())? } else { funcall(c, x, s::to_a, &[], std::ptr::null())? } } } else { return type_err(c, format!("wrong argument type {} (must respond to :each)", class_name_for_err(c, x))); };
        others.push(arr);
    }
    let keep = c.ary_from_vec(others.clone());
    let out = c.ary_new();
    for i in 0..c.ary_len(slf) {
        let row = c.ary_new_from(&[c.ary_get(slf, i)]);
        for &o in &others { c.ary_push(row, c.ary_get(o, i)); }
        if blk.is_null() { c.ary_push(out, row); } else { yield1(c, blk, row)?; }
    }
    std::hint::black_box(keep);
    Ok(if blk.is_null() { out } else { NIL })
});
cfn!(a_product, |c, slf, a, blk| {
    let mut lists = vec![slf];
    for &x in a { lists.push(expect_array(c, x)?); }
    let keep = c.ary_from_vec(lists.clone());
    let mut total: i64 = 1;
    for l in &lists { total = match total.checked_mul(c.ary_len(*l) as i64) { Some(t) => t, None => return err(c, c.vm().c_range_error.get(), "too big to product length") }; }
    if blk.is_null() { alloc_check(c, total)?; }
    let mut res: Vec<Vec<Value>> = vec![vec![]];
    for l in &lists { let mut nr = vec![]; for r in &res { for &x in c.ary(*l).iter() { let mut t = r.clone(); t.push(x); nr.push(t); } } res = nr; }
    let out = c.ary_new();
    for r in res { let row = c.ary_from_vec(r); if blk.is_null() { c.ary_push(out, row); } else { yield1(c, blk, row)?; } }
    std::hint::black_box(keep);
    Ok(if blk.is_null() { out } else { slf })
});
fn combos(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = vec![]; let mut cur = vec![];
    fn rec(st: usize, n: usize, k: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) { if cur.len() == k { out.push(cur.clone()); return; } for i in st..n { cur.push(i); rec(i + 1, n, k, cur, out); cur.pop(); } }
    rec(0, n, k, &mut cur, &mut out); out
}
fn perms(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = vec![]; let mut cur = vec![]; let mut used = vec![false; n];
    fn rec(n: usize, k: usize, cur: &mut Vec<usize>, used: &mut Vec<bool>, out: &mut Vec<Vec<usize>>) { if cur.len() == k { out.push(cur.clone()); return; } for i in 0..n { if !used[i] { used[i] = true; cur.push(i); rec(n, k, cur, used, out); cur.pop(); used[i] = false; } } }
    rec(n, k, &mut cur, &mut used, &mut out); out
}
fn emit_index_lists(c: &mut Ctx, slf: Value, lists: Vec<Vec<usize>>, blk: *const Block, mid: &str, a: &[Value]) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(mid.as_bytes()), a, Value::fix(lists.len() as i64)); }
    let snapshot = c.ary_from_vec(c.ary(slf).to_vec());
    for l in lists { let row = c.ary_from_vec(l.iter().map(|&i| c.ary_get(snapshot, i)).collect()); yield1(c, blk, row)?; }
    Ok(slf)
}
cfn!(a_combination, |c, slf, a, blk| { let k = num_to_long(c, a[0])?; let l = if k < 0 || k as usize > c.ary_len(slf) { vec![] } else { combos(c.ary_len(slf), k as usize) }; emit_index_lists(c, slf, l, blk, "combination", a) });
cfn!(a_permutation, |c, slf, a, blk| { let k = if a.is_empty() { c.ary_len(slf) as i64 } else { num_to_long(c, a[0])? }; let l = if k < 0 || k as usize > c.ary_len(slf) { vec![] } else { perms(c.ary_len(slf), k as usize) }; emit_index_lists(c, slf, l, blk, "permutation", a) });
cfn!(a_repeated_combination, |c, slf, a, blk| {
    let k = num_to_long(c, a[0])?; let n = c.ary_len(slf);
    let mut out = vec![];
    if k >= 0 { let mut cur = vec![]; fn rec(st: usize, n: usize, k: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) { if cur.len() == k { out.push(cur.clone()); return; } for i in st..n { cur.push(i); rec(i, n, k, cur, out); cur.pop(); } } rec(0, n, k as usize, &mut cur, &mut out); }
    emit_index_lists(c, slf, out, blk, "repeated_combination", a)
});
cfn!(a_repeated_permutation, |c, slf, a, blk| {
    let k = num_to_long(c, a[0])?; let n = c.ary_len(slf);
    let mut out = vec![];
    if k >= 0 { let mut cur = vec![]; fn rec(n: usize, k: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) { if cur.len() == k { out.push(cur.clone()); return; } for i in 0..n { cur.push(i); rec(n, k, cur, out); cur.pop(); } } rec(n, k as usize, &mut cur, &mut out); }
    emit_index_lists(c, slf, out, blk, "repeated_permutation", a)
});
cfn!(a_transpose, |c, slf, a, blk| {
    let n = c.ary_len(slf);
    if n == 0 { return Ok(c.ary_new()); }
    let mut rows = vec![];
    for i in 0..n { rows.push(expect_array(c, c.ary_get(slf, i))?); }
    let w = c.ary_len(rows[0]);
    for (i, r) in rows.iter().enumerate() { if c.ary_len(*r) != w { return err(c, c.vm().c_index_error.get(), format!("element size differs ({} should be {})", c.ary_len(*r), w)); } let _ = i; }
    let out = c.ary_new();
    for j in 0..w { let col = c.ary_from_vec(rows.iter().map(|r| c.ary_get(*r, j)).collect()); c.ary_push(out, col); }
    Ok(out)
});
cfn!(a_assoc, |c, slf, a, blk| { let mut i = 0; while let Some(&x) = c.ary(slf).get(i) { if let Some(x) = check_array(c, x)? { if c.ary_len(x) > 0 && rb_eq(c, c.ary_get(x, 0), a[0])? { return Ok(x); } } i += 1; } Ok(NIL) });
cfn!(a_rassoc, |c, slf, a, blk| { let mut i = 0; while let Some(&x) = c.ary(slf).get(i) { if let Some(x) = check_array(c, x)? { if c.ary_len(x) > 1 && rb_eq(c, c.ary_get(x, 1), a[0])? { return Ok(x); } } i += 1; } Ok(NIL) });
cfn!(a_values_at, |c, slf, a, blk| {
    let out = c.ary_new();
    for &x in a {
        if is_range(x) {
            let n = c.ary_len(slf) as i64;
            let (b, e, excl) = c.range_parts(x);
            let st0 = if b == NIL { 0 } else { num_to_long(c, b)? };
            let st = if st0 < 0 { st0 + n } else { st0 };
            if st < 0 { let s2 = inspect_str(c, x)?; return err(c, c.vm().c_range_error.get(), format!("{} out of range", s2)); }
            let en = if e == NIL { n } else { let e2 = num_to_long(c, e)?; let e2 = if e2 < 0 { e2 + n } else { e2 }; if excl { e2 } else { e2 + 1 } };
            for i in st..en.max(st) { c.ary_push(out, c.ary_get(slf, i as usize)); }
        } else { let i = num_to_long(c, x)?; c.ary_push(out, norm_index(i, c.ary_len(slf)).map(|i| c.ary_get(slf, i)).unwrap_or(NIL)); }
    }
    Ok(out)
});
cfn!(a_dig, |c, slf, a, blk| {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let v = ary_aref(c, slf, &a[..1])?;
    if a.len() == 1 || v == NIL { return Ok(v); }
    if !respond_to(c, v, intern(b"dig"), true) { return type_err(c, format!("{} does not have #dig method", class_name_for_err(c, v))); }
    funcall(c, v, intern(b"dig"), &a[1..], std::ptr::null())
});
cfn!(a_first_n_compat, |c, slf, a, blk| Ok(NIL));
// CRuby's ary_sample: same random draws, so a seeded generator picks the same elements
cfn!(a_sample, |c, slf, a, blk| {
    let kw = kw_arg(a);
    let pos = if kw.is_some() { &a[..a.len() - 1] } else { a };
    check_argc(c, pos, 0, 1)?;
    let gen = match kw { Some(h) => hash_get_opt(c, h, sym_value("random"))?.unwrap_or(c.vm().c_random.get()), None => c.vm().c_random.get() };
    let upto = |c: &mut Ctx, m: usize| -> Result<usize, Esc> { Ok(crate::builtins::random::ulong_limited(c, gen, m as u64 - 1)? as usize) };
    let mut len = c.ary_len(slf);
    if pos.is_empty() {
        let i = if len < 2 { 0 } else { upto(c, len)? };
        return Ok(c.ary(slf).get(i).copied().unwrap_or(NIL));
    }
    let n = num_to_long(c, pos[0])?;
    if n < 0 { return arg_err(c, "negative sample number"); }
    let mut n = (n as usize).min(len);
    if n <= 10 {
        let mut rnds = [0usize; 10];
        for i in 0..n { rnds[i] = upto(c, len - i)?; }
        let k = len;
        len = c.ary_len(slf);
        if len < k && (0..n).any(|i| rnds[i] >= len.saturating_sub(i)) { return Ok(c.ary_new()); }
        n = n.min(len);
        let at = |c: &Ctx, i: usize| c.ary_get(slf, i);
        let idx: Vec<usize> = match n {
            0 => vec![],
            1 => vec![rnds[0]],
            2 => { let (i, mut j) = (rnds[0], rnds[1]); if j >= i { j += 1; } vec![i, j] }
            3 => {
                let (i, mut j, mut k) = (rnds[0], rnds[1], rnds[2]);
                let (mut l, mut g) = (j, i);
                if j >= i { l = i; j += 1; g = j; }
                if k >= l { k += 1; if k >= g { k += 1; } }
                vec![i, j, k]
            }
            _ => {
                let mut sorted: Vec<usize> = Vec::with_capacity(n);
                let mut idx = Vec::with_capacity(n);
                for &r in &rnds[..n] {
                    let mut k = r;
                    let mut j = 0;
                    while j < sorted.len() { if k < sorted[j] { break; } k += 1; j += 1; }
                    sorted.insert(j, k);
                    idx.push(k);
                }
                idx
            }
        };
        let v: Vec<Value> = idx.into_iter().map(|i| at(c, i)).collect();
        return Ok(c.ary_from_vec(v));
    }
    let memo_threshold = if len < 2560 { len / 128 } else if len < 5120 { len / 64 } else if len < 10240 { len / 32 } else { len / 16 };
    if n <= memo_threshold / 2 {
        // a partial Fisher-Yates over a sparse map of the swapped slots: no copy of self
        let mut picks = Vec::with_capacity(n);
        let mut max_idx = 0;
        for i in 0..n { let r = upto(c, len - i)? + i; max_idx = max_idx.max(r); picks.push(r); }
        len = c.ary_len(slf);
        if len <= max_idx { n = 0; } else { n = n.min(len); }
        // open addressing (<= n keys, load <= 1/2): a HashMap instance here moved rustc's codegen split and slowed map/select
        let cap = (2 * n + 2).next_power_of_two();
        let (mut keys, mut vals) = (vec![usize::MAX; cap], vec![0usize; cap]);
        let slot = |keys: &[usize], k: usize| { let mut p = (k.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 40) & (cap - 1); while keys[p] != usize::MAX && keys[p] != k { p = (p + 1) & (cap - 1); } p };
        let mut out = Vec::with_capacity(n);
        for (i, &j) in picks[..n].iter().enumerate() {
            let (pi, pj) = (slot(&keys, i), slot(&keys, j));
            let i2 = if keys[pi] == i { vals[pi] } else { i };
            let j2 = if keys[pj] == j { vals[pj] } else { j };
            keys[pj] = j; vals[pj] = i2;
            out.push(c.ary_get(slf, j2));
        }
        return Ok(c.ary_from_vec(out));
    }
    let mut v = c.ary(slf).to_vec();
    for i in 0..n { let j = upto(c, len - i)? + i; v.swap(i, j); }
    v.truncate(n);
    Ok(c.ary_from_vec(v))
});
/// The `random:` keyword (the Random class when absent); no positional arguments.
fn random_kw(c: &mut Ctx, a: &[Value]) -> RV {
    let kw = kw_arg(a);
    let npos = a.len() - kw.is_some() as usize;
    if npos != 0 { return argc_err(c, npos, 0, 0); }
    let Some(h) = kw else { return Ok(c.vm().c_random.get()) };
    let key = Value::sym(intern(b"random"));
    let mut gen = c.vm().c_random.get();
    let mut unknown = vec![];
    let mut i = 0;
    while let Some((k, v)) = c.hash_entry_at(h, i) {
        i += 1;
        if k == UNDEF { continue; }
        if k == key { gen = v; } else { unknown.push(inspect_str(c, k)?); }
    }
    if !unknown.is_empty() { return arg_err(c, format!("unknown keyword{}: {}", if unknown.len() > 1 { "s" } else { "" }, unknown.join(", "))); }
    Ok(gen)
}
cfn!(a_shuffle, |c, slf, a, blk| {
    let gen = random_kw(c, a)?;
    let v = c.ary(slf).to_vec();
    let out = c.ary_from_vec(v);   // a #rand of gen may run Ruby: keep the elements in a Ruby Array
    // CRuby's order: i from len down to 1, j = rand(i), swap(i - 1, j)
    let mut i = c.ary_len(out);
    while i > 0 {
        let j = crate::builtins::random::ulong_limited(c, gen, i as u64 - 1)? as usize;
        i -= 1;
        c.ary_mut(out).swap(i, j);
    }
    Ok(out)
});
cfn!(a_shuffle_b, |c, slf, a, blk| { let r = a_shuffle(c, slf, a, blk)?; *c.ary_modify(slf)? = c.ary(r).to_vec().into(); Ok(slf) });
/// Enumerator#size marker of Array#cycle's enumerators.
pub const ARY_CYCLE_SIZE: &[u8] = b"__ary_cycle_size";
/// rb_ary_cycle_size.
pub fn ary_cycle_size(c: &mut Ctx, ary: Value, args: Value) -> RV {
    let l = c.ary_len(ary);
    if l == 0 { return Ok(Value::fix(0)); }
    let n = if c.ary_len(args) > 0 { c.ary_get(args, 0) } else { NIL };
    if n == NIL { return Ok(c.float_new(f64::INFINITY)); }
    let m = num_to_long(c, n)?;
    Ok(if m <= 0 { Value::fix(0) } else { c.int_from_i128(l as i128 * m as i128) })
}
cfn!(a_cycle, |c, slf, a, blk| {
    check_argc(c, a, 0, 1)?;
    // the size is computed when asked (rb_ary_cycle_size): a count that is no long fails only then
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"cycle"), a, Value::sym(intern(ARY_CYCLE_SIZE))); }
    enum_or(c, slf, "cycle", a, blk, |c| {
    let n = if a.is_empty() || a[0] == NIL { -1 } else { num_to_long(c, a[0])? };
    let mut k = 0;
    while n < 0 || k < n { if c.ary_len(slf) == 0 { break; } let mut i = 0; while i < c.ary_len(slf) { yield1(c, blk, c.ary_get(slf, i))?; i += 1; } k += 1; }
    Ok(NIL)
}) });
cfn!(a_bsearch, |c, slf, a, blk| enum_or(c, slf, "bsearch", a, blk, |c| {
    let (mut lo, mut hi) = (0usize, c.ary_len(slf));
    let mut found = NIL;
    while lo < hi {
        let mid = (lo + hi) / 2;
        let x = c.ary_get(slf, mid);
        let r = yield1(c, blk, x)?;
        if r == TRUE { found = x; hi = mid; }
        else if r == FALSE || r == NIL { lo = mid + 1; }
        else if r.is_fixnum() || is_float(r) {
            let v = c.num_to_f64(r).unwrap();
            if v == 0.0 { return Ok(x); }
            if v < 0.0 { hi = mid; } else { lo = mid + 1; }
        } else { return type_err(c, format!("wrong argument type {} (must be numeric, true, false or nil)", class_name_for_err(c, r))); }
    }
    Ok(found)
}));
cfn!(a_pack, |c, slf, a, blk| crate::builtins::pack::pack(c, slf, a));
cfn!(a_frozen_dup, |c, slf, a, blk| Ok(slf));
cfn!(a_flat_map, |c, slf, a, blk| if ary_each_redef(c, slf) { crate::builtins::enum_native::call_enumerable(c, slf, "flat_map", a, blk) } else { enum_or(c, slf, "flat_map", a, blk, |c| {
    let out = c.ary_new(); let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) { let y = yield1(c, blk, x)?; if is_array(y) { c.ary_concat(out, y); } else if let Some(arr) = check_array(c, y)? { c.ary_concat(out, arr); } else { c.ary_push(out, y); } i += 1; }
    Ok(out)
}) });
cfn!(a_any, |c, slf, a, blk| {
    check_argc(c, a, 0, 1)?;
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) {
        let t = if !a.is_empty() { crate::rt::srb_eqq(c, a[0], x, &mut CallCache::default())? } else if blk.is_null() { x.truthy() } else { yield1(c, blk, x)?.truthy() };
        if t { return Ok(TRUE); }
        i += 1;
    }
    Ok(FALSE)
});
cfn!(a_all, |c, slf, a, blk| {
    check_argc(c, a, 0, 1)?;
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) {
        let t = if !a.is_empty() { crate::rt::srb_eqq(c, a[0], x, &mut CallCache::default())? } else if blk.is_null() { x.truthy() } else { yield1(c, blk, x)?.truthy() };
        if !t { return Ok(FALSE); }
        i += 1;
    }
    Ok(TRUE)
});
cfn!(a_none, |c, slf, a, blk| { let r = a_any(c, slf, a, blk)?; Ok(Value::bool(r == FALSE)) });
cfn!(a_find, |c, slf, a, blk| enum_or(c, slf, "find", a, blk, |c| { let mut i = 0; while let Some(&x) = c.ary(slf).get(i) { if yield1(c, blk, x)?.truthy() { return Ok(x); } i += 1; } Ok(NIL) }));
/// limit_by_enum_size / enum_size_over_p's rb_check_funcall(ary, :size), made only when it is not the builtin.
fn ary_size_call(c: &mut Ctx, slf: Value) -> Result<Option<Value>, Esc> {
    if obj_klass(slf) == c.vm().c_array.get() && !crate::rt::redef(crate::rt::R_ARY_SIZE) { return Ok(None); }
    if !respond_to(c, slf, s::size, true) { return Ok(None); }
    Ok(Some(funcall(c, slf, s::size, &[], std::ptr::null())?))
}
cfn!(a_each_slice, |c, slf, a, blk| if ary_each_redef(c, slf) { crate::builtins::enum_native::call_enumerable(c, slf, "each_slice", a, blk) } else { if blk.is_null() { crate::builtins::enumerator::enum_for(c, slf, intern(b"each_slice"), a, NIL) } else { enum_or(c, slf, "each_slice", a, blk, |c| {
    let n = num_to_long(c, a[0])?;
    if n <= 0 { return arg_err(c, "invalid slice size"); }
    ary_size_call(c, slf)?;
    let mut i = 0;
    while i < c.ary_len(slf) { let e = (i + n as usize).min(c.ary_len(slf)); let sl = c.ary_from_vec(c.ary(slf)[i..e].to_vec()); yield1(c, blk, sl)?; i = e; }
    Ok(slf)
}) } });
cfn!(a_inject, |c, slf, a, blk| {
    if ary_each_redef(c, slf) { return crate::builtins::enum_native::call_enumerable(c, slf, "inject", a, blk); }
    if !blk.is_null() { if let Some(r) = a_inject_loop(c, slf, a, &mut |c, a| yield_block(c, blk, a)) { return r; } }
    let (mut acc, mut i, op) = match (a.len(), blk.is_null()) {
        (0, _) => { if c.ary_len(slf) == 0 { return Ok(NIL); } (c.ary_get(slf, 0), 1, None) }
        (1, false) => (a[0], 0, None),
        (1, true) => { if c.ary_len(slf) == 0 { return Ok(NIL); } (c.ary_get(slf, 0), 1, Some(crate::builtins::object::to_sym_arg(c, a[0])?)) }
        _ => (a[0], 0, Some(crate::builtins::object::to_sym_arg(c, a[1])?)),
    };
    while let Some(&x) = c.ary(slf).get(i) {
        acc = match op {
            Some(m) => { if m == s::plus && acc.is_fixnum() && x.is_fixnum() && !crate::rt::redef(crate::rt::R_INT_PLUS) { if let Some(r) = acc.as_fix().checked_add(x.as_fix()) { if Value::fixable(r) { i += 1; acc = Value::fix(r); continue; } } } funcall(c, acc, m, &[x], std::ptr::null())? }
            None => yield_block(c, blk, &[acc, x])?,
        };
        i += 1;
    }
    Ok(acc)
});
cfn!(a_sort_by_native, |c, slf, a, blk| if ary_each_redef(c, slf) { crate::builtins::enum_native::call_enumerable(c, slf, "sort_by", a, blk) } else { enum_or(c, slf, "sort_by", a, blk, |c| sort_by_body(c, slf, blk)) });
/// The elements of slf ordered by the block's keys (sort_by_cmp: OPTIMIZED_CMP on the keys).
pub fn sort_by_body(c: &mut Ctx, slf: Value, blk: *const Block) -> RV {
    let n = c.ary_len(slf);
    let keys = c.ary_new_capa(n);
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) { let k = yield1(c, blk, x)?; c.ary_push(keys, k); i += 1; }
    let mut idx: Vec<Value> = (0..c.ary_len(keys).min(c.ary_len(slf))).map(|i| Value::fix(i as i64)).collect();
    let src = c.ary_from_vec(c.ary(slf).to_vec());
    if !crate::rt::redef(crate::rt::R_INT_CMP) && c.ary(keys).iter().all(|k| k.is_fixnum()) { let ks = c.ary(keys); idx.sort_by_key(|i| ks[i.as_fix() as usize].as_fix()); }
    else { sort_values(c, &mut idx, &mut |c, x, y| rb_cmp(c, c.ary_get(keys, x.as_fix() as usize), c.ary_get(keys, y.as_fix() as usize)))?; }
    Ok(c.ary_from_vec(idx.iter().map(|i| c.ary_get(src, i.as_fix() as usize)).collect()))
}
cfn!(a_each_cons, |c, slf, a, blk| if ary_each_redef(c, slf) { crate::builtins::enum_native::call_enumerable(c, slf, "each_cons", a, blk) } else { if blk.is_null() { crate::builtins::enumerator::enum_for(c, slf, intern(b"each_cons"), a, NIL) } else { enum_or(c, slf, "each_cons", a, blk, |c| {
    let n = num_to_long(c, a[0])?;
    if n <= 0 { return arg_err(c, "invalid size"); }
    let n = n as usize;
    if let Some(sz) = ary_size_call(c, slf)? { if sz.is_fixnum() && n as u64 > sz.as_fix() as u64 { return Ok(slf); } }
    let mut i = 0;
    while i + n <= c.ary_len(slf) { let sl = c.ary_from_vec(c.ary(slf)[i..i + n].to_vec()); yield1(c, blk, sl)?; i += 1; }
    Ok(slf)
}) } });
cfn!(a_filter_map, |c, slf, a, blk| if ary_each_redef(c, slf) { crate::builtins::enum_native::call_enumerable(c, slf, "filter_map", a, blk) } else { enum_or(c, slf, "filter_map", a, blk, |c| { let out = c.ary_new(); let mut i = 0; while let Some(&x) = c.ary(slf).get(i) { let y = yield1(c, blk, x)?; if y.truthy() { c.ary_push(out, y); } i += 1; } Ok(out) }) });
// Without a block rb_ary_minmax is [min, max]; with one, CRuby's pairwise minmax (enum.c minmax_ii).
cfn!(a_minmax, |c, slf, a, blk| {
    if blk.is_null() {
        // rb_assoc_new(rb_ary_min(..), rb_ary_max(..)): CRuby's build runs the max first
        let mx = minmax_impl(c, slf, &[], blk, true)?;
        let mn = minmax_impl(c, slf, &[], blk, false)?;
        return Ok(c.ary_new_from(&[mn, mx]));
    }
    minmax_pairs(c, slf, blk)
});
/// Enumerable#minmax (enum.c minmax_i / minmax_ii) over the elements of slf: taken in pairs.
pub fn minmax_pairs(c: &mut Ctx, slf: Value, blk: *const Block) -> RV {
    let cmp = |c: &mut Ctx, x: Value, y: Value| -> Result<i64, Esc> {
        if blk.is_null() { Ok(rb_cmp(c, x, y)? as i64) } else { let r = yield_block(c, blk, &[x, y])?; cmp_result(c, r, x, y) }
    };
    let (mut mn, mut mx) = (UNDEF, UNDEF);
    let mut last = UNDEF;
    let mut i = 0;
    while let Some(&x) = c.ary(slf).get(i) {
        i += 1;
        if last == UNDEF { last = x; continue; }
        let (mut lo, mut hi) = (x, last);
        last = UNDEF;
        let n = cmp(c, hi, lo)?;
        if n == 0 { lo = hi; } else if n < 0 { std::mem::swap(&mut lo, &mut hi); }
        if mn == UNDEF { mn = lo; mx = hi; } else {
            if cmp(c, lo, mn)? < 0 { mn = lo; }
            if cmp(c, hi, mx)? > 0 { mx = hi; }
        }
    }
    if last != UNDEF {
        if mn == UNDEF { mn = last; mx = last; } else {
            if cmp(c, last, mn)? < 0 { mn = last; }
            if cmp(c, last, mx)? > 0 { mx = last; }
        }
    }
    if mn == UNDEF { return Ok(c.ary_new_from(&[NIL, NIL])); }
    Ok(c.ary_new_from(&[mn, mx]))
}
cfn!(a_fetch_values, |c, slf, a, blk| {
    let out = c.ary_new_capa(a.len());
    for &x in a { let v = a_fetch(c, slf, &[x], blk)?; c.ary_push(out, v); }
    Ok(out)
});
cfn!(a_rfind, |c, slf, a, blk| {
    check_argc(c, a, 0, 1)?;
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"rfind"), a, NIL); }
    let mut i = c.ary_len(slf);
    while i > 0 {
        i -= 1;
        if i >= c.ary_len(slf) { i = c.ary_len(slf); continue; }
        let x = c.ary_get(slf, i);
        if yield1(c, blk, x)?.truthy() { return Ok(x); }
    }
    if let Some(&ifnone) = a.first() { if ifnone != NIL { return funcall(c, ifnone, s::call, &[], std::ptr::null()); } }
    Ok(NIL)
});
cfn!(a_bsearch_index, |c, slf, a, blk| enum_or(c, slf, "bsearch_index", a, blk, |c| {
    let (mut lo, mut hi) = (0usize, c.ary_len(slf));
    let mut found = NIL;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let x = c.ary_get(slf, mid);
        let r = yield1(c, blk, x)?;
        if r == TRUE { found = Value::fix(mid as i64); hi = mid; }
        else if r == FALSE || r == NIL { lo = mid + 1; }
        else if r.is_fixnum() || is_float(r) || is_bignum(r) {
            let v = c.num_to_f64(r).unwrap_or(0.0);
            if v == 0.0 { return Ok(Value::fix(mid as i64)); }
            if v < 0.0 { hi = mid; } else { lo = mid + 1; }
        } else { return type_err(c, format!("wrong argument type {} (must be numeric, true, false or nil)", class_name_for_err(c, r))); }
    }
    Ok(found)
}));
cfn!(a_freeze, |c, slf, a, blk| { obj_set_flags(slf, FL_FROZEN); Ok(slf) });
cfn!(a_s_try_convert, |c, slf, a, blk| Ok(check_array(c, a[0])?.unwrap_or(NIL)));

pub fn init(c: &mut Ctx, k: Value) {
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, k, n, f, a);
    define_private_cfunc(c, k, "initialize", a_initialize, -1);
    define_singleton_cfunc(c, k, "[]", a_s_aref, -1);
    define_singleton_cfunc(c, k, "try_convert", a_s_try_convert, 1);
    d(c, "[]", a_aref, -1); d(c, "slice", a_aref, -1); d(c, "[]=", a_aset, -1); d(c, "at", a_at, 1); d(c, "fetch", a_fetch, -1); d(c, "first", a_first, -1); d(c, "last", a_last, -1);
    d(c, "push", a_push, -1); d(c, "append", a_push, -1); d(c, "<<", a_ltlt, 1); d(c, "pop", a_pop, -1); d(c, "shift", a_shift, -1); d(c, "unshift", a_unshift, -1);
    d(c, "prepend", a_unshift, -1); d(c, "insert", a_insert, -1); d(c, "length", a_length, 0); d(c, "size", a_length, 0); d(c, "empty?", a_empty, 0);
    d(c, "each", a_each, 0); d(c, "each_index", a_each_index, 0); d(c, "each_with_index", a_each_with_index, 0); d(c, "reverse_each", a_reverse_each, 0);
    d(c, "map", a_map, 0); d(c, "collect", a_map, 0); d(c, "map!", a_map_b, 0); d(c, "collect!", a_map_b, 0); d(c, "select", a_select, 0); d(c, "filter", a_select, 0);
    d(c, "reject", a_reject, 0); d(c, "select!", a_select_b, 0); d(c, "filter!", a_select_b, 0); d(c, "keep_if", a_keep_if, 0); d(c, "reject!", a_reject_b, 0);
    d(c, "delete_if", a_delete_if, 0); d(c, "index", a_index, -1); d(c, "find_index", a_index, -1); d(c, "rindex", a_rindex, -1); d(c, "include?", a_include, 1);
    d(c, "member?", a_include, 1); d(c, "delete", a_delete, 1); d(c, "delete_at", a_delete_at, 1); d(c, "compact", a_compact, 0); d(c, "compact!", a_compact_b, 0);
    d(c, "flatten", a_flatten, -1); d(c, "flatten!", a_flatten_b, -1); d(c, "uniq", a_uniq, 0); d(c, "uniq!", a_uniq_b, 0); d(c, "sort", a_sort, 0); d(c, "sort!", a_sort_b, 0);
    d(c, "sort_by!", a_sort_by_b, 0); d(c, "reverse", a_reverse, 0); d(c, "reverse!", a_reverse_b, 0); d(c, "rotate", a_rotate, -1); d(c, "rotate!", a_rotate_b, -1);
    d(c, "join", a_join, -1); d(c, "to_a", a_to_a, 0); d(c, "entries", a_entries, 0); d(c, "to_ary", a_to_ary, 0); d(c, "to_h", a_to_h, 0); d(c, "inspect", a_inspect, 0); d(c, "to_s", a_inspect, 0);
    d(c, "==", a_eq, 1); d(c, "eql?", a_eql, 1); d(c, "hash", a_hash, 0); d(c, "<=>", a_cmp, 1); d(c, "+", a_plus, 1); d(c, "-", a_minus, 1); d(c, "difference", a_difference, -1);
    d(c, "*", a_mul, 1); d(c, "&", a_and, 1); d(c, "intersection", a_intersection, -1); d(c, "intersect?", a_intersect_p, 1); d(c, "|", a_or, 1); d(c, "union", a_union, -1);
    d(c, "concat", a_concat, -1); d(c, "clear", a_clear, 0); d(c, "replace", a_replace, 1); d(c, "initialize_copy", a_replace, 1); d(c, "fill", a_fill, -1); d(c, "slice!", a_slice_b, -1);
    d(c, "min", a_min, -1); d(c, "max", a_max, -1); d(c, "sum", a_sum, -1); d(c, "count", a_count, -1); d(c, "take", a_take, 1); d(c, "drop", a_drop, 1);
    d(c, "take_while", a_take_while, 0); d(c, "drop_while", a_drop_while, 0); d(c, "zip", a_zip, -1); d(c, "product", a_product, -1); d(c, "combination", a_combination, 1);
    d(c, "permutation", a_permutation, -1); d(c, "repeated_combination", a_repeated_combination, 1); d(c, "repeated_permutation", a_repeated_permutation, 1);
    d(c, "transpose", a_transpose, 0); d(c, "assoc", a_assoc, 1); d(c, "rassoc", a_rassoc, 1); d(c, "values_at", a_values_at, -1); d(c, "dig", a_dig, -1);
    d(c, "sample", a_sample, -1); d(c, "shuffle", a_shuffle, -1); d(c, "shuffle!", a_shuffle_b, -1); d(c, "cycle", a_cycle, -1); d(c, "bsearch", a_bsearch, 0);
    d(c, "pack", a_pack, -1); d(c, "flat_map", a_flat_map, 0); d(c, "collect_concat", a_flat_map, 0); d(c, "any?", a_any, -1); d(c, "all?", a_all, -1); d(c, "none?", a_none, -1);
    d(c, "find", a_find, -1); d(c, "detect", a_find, -1); d(c, "each_slice", a_each_slice, 1); d(c, "inject", a_inject, -1); d(c, "reduce", a_inject, -1);
    d(c, "sort_by", a_sort_by_native, 0); d(c, "each_cons", a_each_cons, 1); d(c, "filter_map", a_filter_map, 0); d(c, "minmax", a_minmax, 0); d(c, "freeze", a_freeze, 0);
    d(c, "fetch_values", a_fetch_values, -1); d(c, "rfind", a_rfind, -1); d(c, "bsearch_index", a_bsearch_index, 0); d(c, "deconstruct", a_to_ary, 0);
    let _ = (a_frozen_dup, a_first_n_compat, a_s_new);
}
