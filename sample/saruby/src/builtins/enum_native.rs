// Native helpers for the Ruby-level Enumerable / Set / Data in prelude/core.rb.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::value::*;
use crate::vm::*;

// Enumerable.__int(n): implicit Integer conversion (to_int) with CRuby's errors.
fn e_int(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = num_to_long(c, a[0])?;
    Ok(c.int_from_i64(n))
}
// Enumerable.__hash(h): implicit Hash conversion (to_hash) or TypeError.
fn e_hash(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    match crate::hash::check_hash(c, a[0])? {
        Some(h) => Ok(h),
        None => type_err(c, format!("no implicit conversion of {} into Hash", crate::builtins::kernel::class_name_for_err(c, a[0]))),
    }
}
// Enumerable.__argc(given, min, max): ArgumentError with CRuby's wording (max -1 = unlimited).
fn e_argc(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    argc_err(c, a[0].as_fix() as usize, a[1].as_fix() as i32, a[2].as_fix() as i32)
}

// ---- Comparable (compar.c): x <=> y always called, rb_cmpint of its answer ----
fn cmpint(c: &mut Ctx, x: Value, y: Value) -> Result<i64, Esc> {
    let r = funcall(c, x, crate::symbol::s::cmp, &[y], std::ptr::null())?;
    crate::builtins::array_methods::cmp_result(c, r, x, y)
}
fn cmp_gt(c: &mut Ctx, x: Value, a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(cmpint(c, x, a[0])? > 0)) }
fn cmp_ge(c: &mut Ctx, x: Value, a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(cmpint(c, x, a[0])? >= 0)) }
fn cmp_lt(c: &mut Ctx, x: Value, a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(cmpint(c, x, a[0])? < 0)) }
fn cmp_le(c: &mut Ctx, x: Value, a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(cmpint(c, x, a[0])? <= 0)) }
fn cmp_between(c: &mut Ctx, x: Value, a: &[Value], _b: *const Block) -> RV {
    Ok(Value::bool(cmpint(c, x, a[0])? >= 0 && cmpint(c, x, a[1])? <= 0))
}
fn cmp_clamp(c: &mut Ctx, x: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let (min, max) = if a.len() == 1 {
        let r = a[0];
        let (b, e, excl) = if crate::object::is_range(r) { c.range_parts(r) } else {
            let s = crate::symbol::s::begin;
            let ok = r.is_heap() && respond_to(c, r, s, false) && respond_to(c, r, crate::symbol::s::end_, false) && respond_to(c, r, crate::symbol::intern(b"exclude_end?"), false);
            if !ok { return type_err(c, format!("wrong argument type {} (expected Range)", crate::builtins::kernel::class_name_for_err(c, r))); }
            let b = funcall(c, r, s, &[], std::ptr::null())?;
            let e = funcall(c, r, crate::symbol::s::end_, &[], std::ptr::null())?;
            let x = funcall(c, r, crate::symbol::intern(b"exclude_end?"), &[], std::ptr::null())?.truthy();
            (b, e, x)
        };
        if e != NIL && excl { return arg_err(c, "cannot clamp with an exclusive range"); }
        (b, e)
    } else { (a[0], a[1]) };
    if min != NIL && max != NIL && cmpint(c, min, max)? > 0 { return arg_err(c, "min argument must be less than or equal to max argument"); }
    if min != NIL {
        let n = cmpint(c, x, min)?;
        if n == 0 { return Ok(x); }
        if n < 0 { return Ok(min); }
    }
    if max != NIL && cmpint(c, x, max)? > 0 { return Ok(max); }
    Ok(x)
}
// Enumerable.__sort_by(obj) { |x| key }: obj's #each collected, then ordered by the keys (enum_sort_by)
// Enumerable.__to_a(obj): obj's #each collected (enum_to_a), for the Enumerable methods that sort
fn e_to_a(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let acc = c.ary_new();
    run_each(c, a[0], |c, d, args, _| { let v = pack(c, args); c.ary_push(d, v); Ok(NIL) }, acc, &[])?;
    Ok(acc)
}
fn e_sort_by(c: &mut Ctx, _s: Value, a: &[Value], blk: *const Block) -> RV {
    let acc = c.ary_new();
    run_each(c, a[0], |c, d, args, _| { let v = pack(c, args); c.ary_push(d, v); Ok(NIL) }, acc, &[])?;
    crate::builtins::array_methods::sort_by_body(c, acc, blk)
}
pub fn init_comparable(c: &mut Ctx, m: Value) {
    define_cfunc(c, m, ">", cmp_gt, 1); define_cfunc(c, m, ">=", cmp_ge, 1);
    define_cfunc(c, m, "<", cmp_lt, 1); define_cfunc(c, m, "<=", cmp_le, 1);
    define_cfunc(c, m, "between?", cmp_between, 2); define_cfunc(c, m, "clamp", cmp_clamp, -1);
}

// Enumerable.__ivget / __ivset: hidden (non-@) instance slots for Data members; set ignores frozen.
fn e_ivget(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(c, a[0], a[1].as_sym())) }
fn e_ivset(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if !a[0].is_heap() || !a[1].is_sym() { return type_err(c, "bad hidden ivar"); }
    ivar_set_raw(c, a[0], a[1].as_sym(), a[2]);
    Ok(a[2])
}

/// Does recv's #each differ from `native` (a subclass or singleton overrode it)?
pub fn each_overridden(c: &Ctx, recv: Value, native: CFunc) -> bool {
    match c.method_entry(class_of(recv), crate::symbol::s::each) {
        Some(me) => match me.kind { MKind::Cfunc(f, _) => f as *const () as usize != native as *const () as usize, _ => true },
        None => true,
    }
}

/// Call Enumerable's own implementation of mid on recv (for natives whose #each was overridden).
pub fn call_enumerable(c: &mut Ctx, recv: Value, mid: &str, args: &[Value], blk: *const Block) -> RV {
    let mid = crate::symbol::intern(mid.as_bytes());
    let me = lookup_method(c, c.vm().m_enumerable.get(), mid);
    if me.is_null() { return funcall(c, recv, mid, args, blk); }
    let base = c.top();
    if !c.has_room(args.len() + 2) { return funcall(c, recv, mid, args, blk); }
    base[0].set(recv);
    base[1].set(NIL);
    for (i, a) in args.iter().enumerate() { base[2 + i].set(*a); }
    c.set_sp(base, 2 + args.len());
    let r = invoke(c, me, recv, base, args.len() as u32, blk, CF_FCALL);
    c.set_sp(base, 0);
    r
}


// ---- native Enumerable (hot methods): iterate with #each and a Rust block ----------------------
// The packed element is CRuby's ENUM_WANT_SVALUE: 0 args -> nil, 1 -> it, n -> [args].
use crate::array::*;
use crate::symbol::s;

#[inline]
fn pack(c: &mut Ctx, args: &[Value]) -> Value { match args.len() { 0 => NIL, 1 => args[0], _ => c.ary_new_from(args) } }
#[inline]
fn ublk(c: &Ctx, d: Value, i: usize) -> *const Block { c.ary_get(d, i).as_fix() as usize as *const Block }
#[inline]
fn bptr(b: *const Block) -> Value { Value::fix(b as usize as i64) }

/// Run recv.each with a Rust block; Ok(true) if it was stopped early by `enum_stop`.
fn run_each(c: &mut Ctx, recv: Value, f: CBlockFn, data: Value, args: &[Value]) -> Result<bool, Esc> {
    let r = if let (true, Some((lo, hi))) = (args.is_empty(), int_range(c, recv)) {
        // Integer ranges: no #each dispatch, no Ruby block frames
        let mut i = lo;
        let mut r = Ok(NIL);
        while i <= hi { if let Err(e) = f(c, data, &[Value::fix(i)], std::ptr::null()) { r = Err(e); break; } i += 1; }
        r
    } else {
        let b = Block::cfunc(c, f, data);
        funcall(c, recv, s::each, args, &b)
    };
    match r {
        Ok(_) => Ok(false),
        Err(e) => {
            if c.esc == EscKind::Throw && c.esc_tag_v == data { c.esc = EscKind::None; return Ok(true); }
            Err(e)
        }
    }
}
/// First and last element of an Integer Range whose #each is Range's own.
fn int_range(c: &Ctx, recv: Value) -> Option<(i64, i64)> {
    if !is_range(recv) || obj_klass(recv) != c.vm().c_range.get() || range_each_redefined(c) { return None; }
    match c.range(recv) {
        (b, e, x) if b.is_fixnum() && e.is_fixnum() => Some((b.as_fix(), if x { e.as_fix() - 1 } else { e.as_fix() })),
        _ => None,
    }
}
fn range_each_redefined(c: &Ctx) -> bool {
    !matches!(c.method_entry(c.vm().c_range.get(), s::each), Some(me) if std::ptr::eq(me, c.vm().ids.r_each.get()))
}
fn enum_stop(c: &mut Ctx, data: Value) -> RV { c.esc = EscKind::Throw; c.esc_tag_v = data; c.esc_val = NIL; Err(Esc) }

fn no_blk(c: &mut Ctx, slf: Value, mid: &str, a: &[Value]) -> RV {
    crate::builtins::enumerator::enum_for_sized(c, slf, crate::symbol::intern(mid.as_bytes()), a)
}

/// Enumerable#map on a Range of fixnums whose #each is the builtin, yielding through `y` (inline_iter.rs);
/// None: another receiver.
#[inline(always)]
pub fn en_map_range_loop(c: &mut Ctx, slf: Value, y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> Option<RV> {
    let (lo, hi) = int_range(c, slf)?;
    // the result in a local: the generic path reads it and the block out of `d` at every element
    let out = c.ary_new();
    let mut i = lo;
    while i <= hi { match y(c, &[Value::fix(i)]) { Ok(r) => c.ary_push(out, r), Err(e) => return Some(Err(e)) } i += 1; }
    Some(Ok(out))
}
fn en_map(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "map", a); }
    if let Some(r) = en_map_range_loop(c, slf, &mut |c, a| yield_block(c, blk, a)) { return r; }
    let acc0 = c.ary_new();
    let d = c.ary_new_from(&[acc0, bptr(blk)]);
    run_each(c, slf, |c, d, args, _| { let r = yield_block(c, ublk(c, d, 1), args)?; c.ary_push(c.ary_get(d, 0), r); Ok(NIL) }, d, &[])?;
    Ok(c.ary_get(d, 0))
}
fn en_flat_map(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "flat_map", a); }
    let acc0 = c.ary_new();
    let d = c.ary_new_from(&[acc0, bptr(blk)]);
    run_each(c, slf, |c, d, args, _| {
        let r = yield_block(c, ublk(c, d, 1), args)?;
        let out = c.ary_get(d, 0);
        match if r.is_heap() { check_array(c, r)? } else { None } { Some(ra) => c.ary_concat(out, ra), None => c.ary_push(out, r) }
        Ok(NIL)
    }, d, &[])?;
    Ok(c.ary_get(d, 0))
}
fn en_filter_map(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "filter_map", a); }
    let acc0 = c.ary_new();
    let d = c.ary_new_from(&[acc0, bptr(blk)]);
    run_each(c, slf, |c, d, args, _| { let r = yield_block(c, ublk(c, d, 1), args)?; if r.truthy() { c.ary_push(c.ary_get(d, 0), r); } Ok(NIL) }, d, &[])?;
    Ok(c.ary_get(d, 0))
}
fn select_impl(c: &mut Ctx, slf: Value, blk: *const Block, want: bool) -> RV {
    let acc0 = c.ary_new();
    let d = c.ary_new_from(&[acc0, bptr(blk), Value::bool(want)]);
    run_each(c, slf, |c, d, args, _| {
        let v = pack(c, args);
        if yield1(c, ublk(c, d, 1), v)?.truthy() == c.ary_get(d, 2).truthy() { c.ary_push(c.ary_get(d, 0), v); }
        Ok(NIL)
    }, d, &[])?;
    Ok(c.ary_get(d, 0))
}
fn en_select(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "select", a); }
    select_impl(c, slf, blk, true)
}
fn en_reject(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "reject", a); }
    select_impl(c, slf, blk, false)
}
fn en_to_a(c: &mut Ctx, slf: Value, a: &[Value], _blk: *const Block) -> RV {
    let acc0 = c.ary_new();
    let d = c.ary_new_from(&[acc0]);
    run_each(c, slf, |c, d, args, _| { let v = pack(c, args); c.ary_push(c.ary_get(d, 0), v); Ok(NIL) }, d, a)?;
    Ok(c.ary_get(d, 0))
}
fn en_each_with_index(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "each_with_index", a); }
    let d = c.ary_new_from(&[Value::fix(0), bptr(blk)]);
    run_each(c, slf, |c, d, args, _| {
        let i = c.ary_get(d, 0);
        c.ary_mut(d)[0] = Value::fix(i.as_fix() + 1);
        let v = pack(c, args);
        yield_block(c, ublk(c, d, 1), &[v, i])?;
        Ok(NIL)
    }, d, a)?;
    Ok(slf)
}
fn en_each_with_object(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "each_with_object", a); }
    let d = c.ary_new_from(&[a[0], bptr(blk)]);
    run_each(c, slf, |c, d, args, _| { let v = pack(c, args); yield_block(c, ublk(c, d, 1), &[v, c.ary_get(d, 0)])?; Ok(NIL) }, d, &[])?;
    Ok(a[0])
}
fn en_include(c: &mut Ctx, slf: Value, a: &[Value], _blk: *const Block) -> RV {
    let d = c.ary_new_from(&[a[0]]);
    let hit = run_each(c, slf, |c, d, args, _| {
        let v = pack(c, args);
        if crate::builtins::array_methods::rb_eq(c, v, c.ary_get(d, 0))? { return enum_stop(c, d); }
        Ok(NIL)
    }, d, &[])?;
    Ok(Value::bool(hit))
}
fn en_find(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, crate::symbol::intern(b"find"), a, NIL); }
    let d = c.ary_new_from(&[NIL, bptr(blk)]);
    let hit = run_each(c, slf, |c, d, args, _| {
        let v = pack(c, args);
        if yield1(c, ublk(c, d, 1), v)?.truthy() { c.ary_mut(d)[0] = v; return enum_stop(c, d); }
        Ok(NIL)
    }, d, &[])?;
    if hit { return Ok(c.ary_get(d, 0)); }
    match a.first() { Some(&ifnone) if ifnone != NIL => funcall(c, ifnone, s::call, &[], std::ptr::null()), _ => Ok(NIL) }
}
fn en_count(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    let mode = if !a.is_empty() { 1 } else if !blk.is_null() { 2 } else { 0 };
    let d = c.ary_new_from(&[Value::fix(0), bptr(blk), Value::fix(mode), a.first().copied().unwrap_or(NIL)]);
    run_each(c, slf, |c, d, args, _| {
        let hit = match c.ary_get(d, 2).as_fix() {
            0 => true,
            1 => { let v = pack(c, args); crate::builtins::array_methods::rb_eq(c, v, c.ary_get(d, 3))? }
            _ => yield_block(c, ublk(c, d, 1), args)?.truthy(),
        };
        if hit { c.ary_mut(d)[0] = Value::fix(c.ary_get(d, 0).as_fix() + 1); }
        Ok(NIL)
    }, d, &[])?;
    Ok(c.ary_get(d, 0))
}
// enum.c minmax_by: elements taken in pairs, the smaller key against the minimum, the larger against the maximum.
// d: [blk, min, max, min_key, max_key, last, last_key, state (bit 0: min/max set, bit 1: last set)]
fn minmax_by_update(c: &mut Ctx, d: Value, v1: Value, v2: Value, i1: Value, i2: Value) -> Result<(), Esc> {
    use crate::builtins::array_methods::rb_cmp;
    let st = c.ary_get(d, 7).as_fix();
    if st & 1 == 0 {
        for (k, v) in [(1, i1), (2, i2), (3, v1), (4, v2)] { c.ary_mut(d)[k] = v; }
        c.ary_mut(d)[7] = Value::fix(st | 1);
        return Ok(());
    }
    let mn = c.ary_get(d, 3);
    if rb_cmp(c, v1, mn)?.is_lt() { c.ary_mut(d)[3] = v1; c.ary_mut(d)[1] = i1; }
    let mx = c.ary_get(d, 4);
    if rb_cmp(c, v2, mx)?.is_gt() { c.ary_mut(d)[4] = v2; c.ary_mut(d)[2] = i2; }
    Ok(())
}
fn en_minmax_by(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "minmax_by", a); }
    let d = c.ary_new_from(&[bptr(blk), NIL, NIL, NIL, NIL, NIL, NIL, Value::fix(0)]);
    run_each(c, slf, |c, d, args, _| {
        let mut i = pack(c, args);
        let mut vi = yield1(c, ublk(c, d, 0), i)?;
        let st = c.ary_get(d, 7).as_fix();
        if st & 2 == 0 { c.ary_mut(d)[5] = i; c.ary_mut(d)[6] = vi; c.ary_mut(d)[7] = Value::fix(st | 2); return Ok(NIL); }
        let (mut j, mut vj) = (c.ary_get(d, 5), c.ary_get(d, 6));
        c.ary_mut(d)[7] = Value::fix(st & !2);
        let n = crate::builtins::array_methods::rb_cmp(c, vj, vi)?;
        if n.is_eq() { i = j; vi = vj; } else if n.is_lt() { std::mem::swap(&mut i, &mut j); std::mem::swap(&mut vi, &mut vj); }
        minmax_by_update(c, d, vi, vj, i, j)?;
        Ok(NIL)
    }, d, &[])?;
    if c.ary_get(d, 7).as_fix() & 2 != 0 { let (l, lv) = (c.ary_get(d, 5), c.ary_get(d, 6)); minmax_by_update(c, d, lv, lv, l, l)?; }
    Ok(c.ary_new_from(&[c.ary_get(d, 1), c.ary_get(d, 2)]))
}
// enum.c min_by / max_by: OPTIMIZED_CMP(key, best key); with n, rb_nmin_run over (key, value) pairs
fn minmax_by_one(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block, mx: bool) -> RV {
    use crate::builtins::array_methods::Nmin;
    check_argc(c, a, 0, 1)?;
    let name = if mx { "max_by" } else { "min_by" };
    if blk.is_null() { return no_blk(c, slf, name, a); }
    if let Some(&n) = a.first().filter(|n| **n != NIL) {
        let Some(nm) = Nmin::new(c, n, true, mx, blk)? else { return Ok(c.ary_new()); };
        let d = nm.save(c);
        run_each(c, slf, |c, d, args, _| {
            let mut nm = Nmin::load(c, d);
            let v = pack(c, args);
            let k = yield1(c, nm.blk(), v)?;
            nm.add(c, v, k)?;
            nm.store(c, d);
            Ok(NIL)
        }, d, &[])?;
        return Nmin::load(c, d).finish(c);
    }
    // d: [blk, best, best key, set?, max?]
    let d = c.ary_new_from(&[bptr(blk), NIL, NIL, FALSE, Value::bool(mx)]);
    run_each(c, slf, |c, d, args, _| {
        let v = pack(c, args);
        let k = yield1(c, ublk(c, d, 0), v)?;
        let take = if c.ary_get(d, 3) == FALSE { true } else {
            let o = crate::builtins::array_methods::rb_cmp(c, k, c.ary_get(d, 2))?;
            if c.ary_get(d, 4) == TRUE { o.is_gt() } else { o.is_lt() }
        };
        if take { let a = c.ary_mut(d); a[1] = v; a[2] = k; a[3] = TRUE; }
        Ok(NIL)
    }, d, &[])?;
    Ok(c.ary_get(d, 1))
}
// Enumerable#min / #max: min_i (OPTIMIZED_CMP(element, best)) / min_ii; with n, rb_nmin_run
fn en_min_max(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block, mx: bool) -> RV {
    use crate::builtins::array_methods::{cmp_result, minmax_impl, rb_cmp};
    check_argc(c, a, 0, 1)?;
    let all = e_to_a(c, NIL, &[slf], std::ptr::null())?;
    if a.first().is_some_and(|n| *n != NIL) { return minmax_impl(c, all, a, blk, mx); }
    let mut best = match c.ary(all).first() { Some(&v) => v, None => return Ok(NIL) };
    let mut i = 1;
    while let Some(&x) = c.ary(all).get(i) {
        let k = if blk.is_null() { rb_cmp(c, x, best)? as i64 } else { let r = yield_block(c, blk, &[x, best])?; cmp_result(c, r, x, best)? };
        if (mx && k > 0) || (!mx && k < 0) { best = x; }
        i += 1;
    }
    Ok(best)
}
fn en_min(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV { en_min_max(c, slf, a, blk, false) }
fn en_max(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV { en_min_max(c, slf, a, blk, true) }
fn en_minmax(c: &mut Ctx, slf: Value, _a: &[Value], blk: *const Block) -> RV {
    let all = e_to_a(c, NIL, &[slf], std::ptr::null())?;
    crate::builtins::array_methods::minmax_pairs(c, all, blk)
}
fn en_min_by(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV { minmax_by_one(c, slf, a, blk, false) }
fn en_max_by(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV { minmax_by_one(c, slf, a, blk, true) }
fn en_group_by(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "group_by", a); }
    let acc0 = c.hash_new();
    let d = c.ary_new_from(&[acc0, bptr(blk)]);
    run_each(c, slf, |c, d, args, _| {
        let v = pack(c, args);
        let k = yield1(c, ublk(c, d, 1), v)?;
        let h = c.ary_get(d, 0);
        // group_by_i: rb_hash_aref (a redefined Hash#default answers a miss)
        let got = match crate::hash::hash_get_opt(c, h, k)? { Some(x) => x, None => crate::hash::hash_default_value(c, h, k)? };
        if is_array(got) { c.ary_push(got, v); } else { let arr = c.ary_new_from(&[v]); crate::hash::hash_aset(c, h, k, arr)?; }
        Ok(NIL)
    }, d, &[])?;
    Ok(c.ary_get(d, 0))
}
fn en_inject(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if a.len() > 2 || (a.is_empty() && blk.is_null()) { return argc_err(c, a.len(), 1, 2); }
    // d = [acc (UNDEF = first element pending), blk, op (nil = block)]
    let (acc, op) = match (a.len(), blk.is_null()) {
        (2, _) => (a[0], a[1]),
        (1, true) => (UNDEF, a[0]),
        (1, false) => (a[0], NIL),
        _ => (UNDEF, NIL),
    };
    let op = if op == NIL || op.is_sym() { op } else if is_string(op) { Value::sym(crate::symbol::intern(c.str_bytes(op))) } else {
        let ins = crate::builtins::kernel::inspect_str(c, op)?;
        return type_err(c, format!("{} is not a symbol nor a string", ins));
    };
    let d = c.ary_new_from(&[acc, bptr(blk), op]);
    run_each(c, slf, |c, d, args, _| {
        let v = pack(c, args);
        let acc = c.ary_get(d, 0);
        if acc == UNDEF { c.ary_mut(d)[0] = v; return Ok(NIL); }
        let op = c.ary_get(d, 2);
        let r = if op == NIL { yield_block(c, ublk(c, d, 1), &[acc, v])? } else {
            let m = op.as_sym();
            if m == s::plus && acc.is_fixnum() && v.is_fixnum() {
                match acc.as_fix().checked_add(v.as_fix()) { Some(x) if Value::fixable(x) => Value::fix(x), _ => funcall_public(c, acc, m, &[v], std::ptr::null())? }
            } else { funcall_public(c, acc, m, &[v], std::ptr::null())? }
        };
        c.ary_mut(d)[0] = r;
        Ok(NIL)
    }, d, &[])?;
    let r = c.ary_get(d, 0);
    Ok(if r == UNDEF { NIL } else { r })
}

fn slice_cons(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block, cons: bool) -> RV {
    let n = num_to_long(c, a[0])?;
    if n <= 0 { return arg_err(c, if cons { "invalid size" } else { "invalid slice size" }); }
    let mid = if cons { "each_cons" } else { "each_slice" };
    if blk.is_null() {
        let mut size = NIL;
        if respond_to(c, slf, s::size, true) {
            let sz = funcall(c, slf, s::size, &[], std::ptr::null())?;
            size = if sz.is_fixnum() { Value::fix(if cons { (sz.as_fix() - n + 1).max(0) } else { (sz.as_fix() + n - 1) / n }) } else { sz };
        }
        return crate::builtins::enumerator::enum_for(c, slf, crate::symbol::intern(mid.as_bytes()), &[Value::fix(n)], size);
    }
    let acc0 = c.ary_new();
    let d = c.ary_new_from(&[acc0, bptr(blk), Value::fix(n), Value::bool(cons)]);
    run_each(c, slf, |c, d, args, _| {
        let v = pack(c, args);
        let buf = c.ary_get(d, 0);
        let n = c.ary_get(d, 2).as_fix() as usize;
        c.ary_push(buf, v);
        if c.ary_get(d, 3).truthy() {
            if c.ary_len(buf) > n { c.ary_mut(buf).remove(0); }
            if c.ary_len(buf) == n { let cp = c.ary_from_vec(c.ary(buf).to_vec()); yield1(c, ublk(c, d, 1), cp)?; }
        } else if c.ary_len(buf) == n {
            c.ary_mut(d)[0] = c.ary_new();
            yield1(c, ublk(c, d, 1), buf)?;
        }
        Ok(NIL)
    }, d, &[])?;
    let buf = c.ary_get(d, 0);
    if !cons && c.ary_len(buf) > 0 { yield1(c, blk, buf)?; }
    Ok(slf)
}
fn en_each_slice(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV { slice_cons(c, slf, a, blk, false) }
fn en_each_cons(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV { slice_cons(c, slf, a, blk, true) }

fn define_enumerable_natives(c: &mut Ctx, m: Value) {
    define_cfunc(c, m, "each_slice", en_each_slice, 1); define_cfunc(c, m, "each_cons", en_each_cons, 1);
    define_cfunc(c, m, "map", en_map, 0); define_cfunc(c, m, "collect", en_map, 0);
    define_cfunc(c, m, "flat_map", en_flat_map, 0); define_cfunc(c, m, "collect_concat", en_flat_map, 0);
    define_cfunc(c, m, "filter_map", en_filter_map, 0);
    define_cfunc(c, m, "select", en_select, 0); define_cfunc(c, m, "filter", en_select, 0); define_cfunc(c, m, "find_all", en_select, 0);
    define_cfunc(c, m, "reject", en_reject, 0);
    define_cfunc(c, m, "to_a", en_to_a, -1); define_cfunc(c, m, "entries", en_to_a, -1);
    define_cfunc(c, m, "each_with_index", en_each_with_index, -1);
    define_cfunc(c, m, "each_with_object", en_each_with_object, 1);
    define_cfunc(c, m, "include?", en_include, 1); define_cfunc(c, m, "member?", en_include, 1);
    define_cfunc(c, m, "find", en_find, -1); define_cfunc(c, m, "detect", en_find, -1);
    define_cfunc(c, m, "count", en_count, -1);
    define_cfunc(c, m, "group_by", en_group_by, 0);
    define_cfunc(c, m, "minmax_by", en_minmax_by, 0);
    define_cfunc(c, m, "min_by", en_min_by, -1);
    define_cfunc(c, m, "min", en_min, -1); define_cfunc(c, m, "max", en_max, -1); define_cfunc(c, m, "minmax", en_minmax, 0); define_cfunc(c, m, "max_by", en_max_by, -1);
    define_cfunc(c, m, "inject", en_inject, -1); define_cfunc(c, m, "reduce", en_inject, -1);
}

/// Defines the native methods of Enumerable `m`.
pub fn init(c: &mut Ctx, m: Value) {
    define_enumerable_natives(c, m);
    define_singleton_cfunc(c, m, "__int", e_int, 1);
    define_singleton_cfunc(c, m, "__hash", e_hash, 1);
    define_singleton_cfunc(c, m, "__argc", e_argc, 3);
    define_singleton_cfunc(c, m, "__sort_by", e_sort_by, 1);
    define_singleton_cfunc(c, m, "__to_a", e_to_a, 1);
    define_singleton_cfunc(c, m, "__range_sum", |c, _s, a, b| crate::builtins::range_methods::r_sum(c, a[0], &a[1..], b), 2);
    define_singleton_cfunc(c, m, "__ivget", e_ivget, 2);
    define_singleton_cfunc(c, m, "__ivset", e_ivset, 3);
}
