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
fn e_int(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let n = num_to_long(c, a[0])?;
    Ok(c.int_from_i64(n))
}
// Enumerable.__hash(h): implicit Hash conversion (to_hash) or TypeError.
fn e_hash(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    match crate::hash::check_hash(c, a[0])? {
        Some(h) => Ok(h),
        None => type_err(c, format!("no implicit conversion of {} into Hash", crate::builtins::kernel::class_name_for_err(c, a[0]))),
    }
}
// Enumerable.__argc(given, min, max): ArgumentError with CRuby's wording (max -1 = unlimited).
fn e_argc(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    argc_err(c, a[0].as_fix() as usize, a[1].as_fix() as i32, a[2].as_fix() as i32)
}
// Enumerable.__kw(hash, *known): rb_get_kwargs' "unknown keyword(s)" ArgumentError.
fn e_kw(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let names: Vec<String> = a[1..].iter().filter(|v| v.is_sym()).map(|v| crate::symbol::sym_str(v.as_sym())).collect();
    let known: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
    kw_unknown(c, a[0], &known)?;
    Ok(NIL)
}

// ---- Comparable (compar.c): x <=> y always called, rb_cmpint of its answer ----
fn cmpint(c: &mut Ctx, x: Value, y: Value) -> Result<i64, Esc> {
    let r = funcall(c, x, crate::symbol::s::cmp, &[y], std::ptr::null())?;
    crate::builtins::array_methods::cmp_result(c, r, x, y)
}
fn cmp_gt(c: &mut Ctx, x: Value, a: &[Value], _b: Blk) -> RV { Ok(Value::bool(cmpint(c, x, a[0])? > 0)) }
fn cmp_ge(c: &mut Ctx, x: Value, a: &[Value], _b: Blk) -> RV { Ok(Value::bool(cmpint(c, x, a[0])? >= 0)) }
fn cmp_lt(c: &mut Ctx, x: Value, a: &[Value], _b: Blk) -> RV { Ok(Value::bool(cmpint(c, x, a[0])? < 0)) }
fn cmp_le(c: &mut Ctx, x: Value, a: &[Value], _b: Blk) -> RV { Ok(Value::bool(cmpint(c, x, a[0])? <= 0)) }
fn cmp_between(c: &mut Ctx, x: Value, a: &[Value], _b: Blk) -> RV {
    Ok(Value::bool(cmpint(c, x, a[0])? >= 0 && cmpint(c, x, a[1])? <= 0))
}
fn cmp_clamp(c: &mut Ctx, x: Value, a: &[Value], _b: Blk) -> RV {
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
// Enumerable.__to_a(obj): obj's #each collected (enum_to_a), for the Enumerable methods that sort
fn e_to_a(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let acc = c.ary_new();
    run_each(c, a[0], |c, d, args, _, _o| { let v = pack(c, args); c.ary_push(d, v); Ok(NIL) }, acc, &[], Blk::NONE)?;
    Ok(acc)
}
#[inline(never)]
fn en_sort_by_gen(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, crate::symbol::intern(b"sort_by"), a); }
    let acc = c.ary_new();
    run_each(c, slf, |c, d, args, _, _o| { let v = pack(c, args); c.ary_push(d, v); Ok(NIL) }, acc, &[], Blk::NONE)?;
    crate::builtins::array_methods::sort_by_body(c, acc, blk)
}
pub fn init_comparable(c: &mut Ctx, m: Value) {
    define_cfunc(c, m, ">", cmp_gt, 1); define_cfunc(c, m, ">=", cmp_ge, 1);
    define_cfunc(c, m, "<", cmp_lt, 1); define_cfunc(c, m, "<=", cmp_le, 1);
    define_cfunc(c, m, "between?", cmp_between, 2); define_cfunc(c, m, "clamp", cmp_clamp, -1);
}

// Enumerable.__ivget / __ivset: hidden (non-@) instance slots for Data members; set ignores frozen.
fn e_ivget(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { Ok(ivar_get(c, a[0], a[1].as_sym())) }
fn e_ivset(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
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
pub fn call_enumerable(c: &mut Ctx, recv: Value, mid: &str, args: &[Value], blk: Blk) -> RV {
    let m = c.vm().m_enumerable.get();
    call_method_of(c, m, recv, crate::symbol::intern(mid.as_bytes()), args, blk)
}
/// Call `klass`'s implementation of mid (found from klass up) on recv, as rb_call_super does.
pub fn call_method_of(c: &mut Ctx, klass: Value, recv: Value, mid: Sym, args: &[Value], blk: Blk) -> RV {
    let me = lookup_method(c, klass, mid);
    // the forwarding cfunc's own arguments: its keywords go on
    let kw = kw_arg(c, args).is_some();
    if me.is_null() || !c.has_room(args.len() + 2) { return funcall_kw(c, recv, mid, args, blk, kw); }
    let base = c.top();
    base[0].set(recv);
    base[1].set(NIL);
    for (i, a) in args.iter().enumerate() { base[2 + i].set(*a); }
    c.set_sp(base, 2 + args.len());
    let r = invoke(c, me, recv, base, args.len() as u32, blk, CF_FCALL | if kw { CF_KW } else { 0 });
    c.set_sp(base, 0);
    r
}


// ---- native Enumerable (hot methods): iterate with #each and a Rust block ----------------------
// The packed element is CRuby's ENUM_WANT_SVALUE: 0 args -> nil, 1 -> it, n -> [args].
use crate::array::*;
use crate::symbol::s;

/// enum.c enum_yield: one packed element; from a multi-value #each a lambda block gets it as rb_yield_force_blockarg.
#[inline(always)]
fn enum_yield(c: &mut Ctx, o: Blk, argc: usize, v: Value) -> RV {
    if argc == 1 { yield1(c, o, v) } else { enum_yield_n(c, o, argc, v) }
}
#[cold]
#[inline(never)]
fn enum_yield_n(c: &mut Ctx, o: Blk, argc: usize, v: Value) -> RV {
    if argc > 1 { crate::builtins::hash_methods::yield_forced(c, o, v) } else { yield_block(c, o, &[]) }
}
#[inline]
fn pack(c: &mut Ctx, args: &[Value]) -> Value { match args.len() { 0 => NIL, 1 => args[0], _ => c.ary_new_from(args) } }
/// Run recv.each with a Rust block; Ok(true) if it was stopped early by `enum_stop`.
/// `f` gets `blk` as its last argument: the Cfunc block's outer block, promoted with it if #each keeps it.
fn run_each(c: &mut Ctx, recv: Value, f: CBlockFn, data: Value, args: &[Value], blk: Blk) -> Result<bool, Esc> {
    let r = if let (true, Some((lo, hi))) = (args.is_empty(), int_range(c, recv)) {
        // Integer ranges: no #each dispatch, no Ruby block frames
        let mut i = lo;
        let mut r = Ok(NIL);
        while i <= hi { if let Err(e) = f(c, data, &[Value::fix(i)], Blk::NONE, blk) { r = Err(e); break; } i += 1; }
        r
    } else {
        let b = Block::cfunc_for(c, f, data, blk);
        let kw = kw_arg(c, args).is_some();
        funcall_kw(c, recv, s::each, args, &b, kw)
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
/// rb_iter_break for a run_each whose data Array ends in a live flag (`run_each_stop`); a kept block
/// called after the method returned gets CRuby's LocalJumpError instead of a throw nobody catches.
fn enum_stop(c: &mut Ctx, data: Value) -> RV {
    if c.ary_get(data, c.ary_len(data) - 1) != TRUE { return Err(crate::rt::raise_local_jump(c, "unexpected break", "break", NIL)); }
    c.esc = EscKind::Throw; c.esc_tag_v = data; c.esc_val = NIL; Err(Esc)
}
/// run_each for a block that may `enum_stop`: `data` gets the live flag for the call's duration.
fn run_each_stop(c: &mut Ctx, recv: Value, f: CBlockFn, data: Value, blk: Blk) -> Result<bool, Esc> {
    c.ary_push(data, TRUE);
    let r = run_each(c, recv, f, data, &[], blk);
    let n = c.ary_len(data) - 1;
    c.ary_mut(data)[n] = FALSE;
    r
}

/// Array#zip's take_items for a non-Array: the first n values of obj.each (packed); None if no #each.
pub fn take_items(c: &mut Ctx, obj: Value, n: usize) -> Result<Option<Value>, Esc> {
    if !crate::rt::check_respond(c, obj, s::each)? { return Ok(None); }
    let out = c.ary_new();
    if n == 0 { return Ok(Some(out)); }
    let d = c.ary_new_from(&[out, Value::fix(n as i64)]);
    run_each_stop(c, obj, |c, d, args, _, _| {
        let v = pack(c, args);
        let out = c.ary_get(d, 0);
        c.ary_push(out, v);
        if c.ary_len(out) as i64 >= c.ary_get(d, 1).as_fix() { return enum_stop(c, d); }
        Ok(NIL)
    }, d, Blk::NONE)?;
    Ok(Some(out))
}

// CRuby owns these on Enumerable; a plain Array / Hash (its #each the builtin) is read directly
use crate::builtins::{array_methods as am, hash_methods as hm};
#[inline(always)]
fn ary_direct(c: &Ctx, s: Value) -> bool { is_array(s) && !am::ary_each_redef(c, s) }
#[inline(always)]
fn hash_direct(c: &Ctx, s: Value) -> bool { is_hash(s) && !each_overridden(c, s, hm::h_each) }

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
#[inline(never)]
fn en_map_gen(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return no_blk(c, slf, "map", a); }
    if let Some(r) = en_map_range_loop(c, slf, &mut |c, a| yield_block(c, blk, a)) { return r; }
    let d = c.ary_new();
    run_each(c, slf, map_blk, d, &[], blk)?;
    Ok(d)
}
/// Enumerable#map's #each block; Proc#arity of it is the outer block's (rb_lambda_call).
pub fn map_blk(c: &mut Ctx, d: Value, args: &[Value], _: Blk, o: Blk) -> RV { let r = yield_block(c, o, args)?; c.ary_push(d, r); Ok(NIL) }
#[inline(never)]
fn en_flat_map_gen(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return no_blk(c, slf, "flat_map", a); }
    let d = c.ary_new();
    run_each(c, slf, |c, out, args, _, o| {
        let r = yield_block(c, o, args)?;
        match check_array(c, r)? { Some(ra) => c.ary_concat(out, ra), None => c.ary_push(out, r) }
        Ok(NIL)
    }, d, &[], blk)?;
    Ok(d)
}
#[inline(never)]
fn en_filter_map_gen(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return no_blk(c, slf, "filter_map", a); }
    let d = c.ary_new();
    run_each(c, slf, |c, d, args, _, o| { let r = yield_block(c, o, args)?; if r.truthy() { c.ary_push(d, r); } Ok(NIL) }, d, &[], blk)?;
    Ok(d)
}
fn select_impl(c: &mut Ctx, slf: Value, blk: Blk, want: bool) -> RV {
    let acc0 = c.ary_new();
    let d = c.ary_new_from(&[acc0, Value::bool(want)]);
    run_each(c, slf, |c, d, args, _, o| {
        let v = pack(c, args);
        if enum_yield(c, o, args.len(), v)?.truthy() == c.ary_get(d, 1).truthy() { c.ary_push(c.ary_get(d, 0), v); }
        Ok(NIL)
    }, d, &[], blk)?;
    Ok(c.ary_get(d, 0))
}
fn en_select(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return no_blk(c, slf, "select", a); }
    select_impl(c, slf, blk, true)
}
fn en_reject(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return no_blk(c, slf, "reject", a); }
    select_impl(c, slf, blk, false)
}
#[inline(never)]
fn en_to_a_gen(c: &mut Ctx, slf: Value, a: &[Value], _blk: Blk) -> RV {
    let acc0 = c.ary_new();
    let d = c.ary_new_from(&[acc0]);
    run_each(c, slf, |c, d, args, _, _o| { let v = pack(c, args); c.ary_push(c.ary_get(d, 0), v); Ok(NIL) }, d, a, Blk::NONE)?;
    Ok(c.ary_get(d, 0))
}
#[inline(never)]
fn en_each_with_index_gen(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return no_blk(c, slf, "each_with_index", a); }
    let d = c.ary_new_from(&[Value::fix(0)]);
    run_each(c, slf, |c, d, args, _, o| {
        let i = c.ary_get(d, 0);
        c.ary_mut(d)[0] = Value::fix(i.as_fix() + 1);
        let v = pack(c, args);
        yield_block(c, o, &[v, i])
    }, d, a, blk)?;
    Ok(slf)
}
fn en_each_with_object(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return no_blk(c, slf, "each_with_object", a); }
    run_each(c, slf, |c, m, args, _, o| { let v = pack(c, args); yield_block(c, o, &[v, m]) }, a[0], &[], blk)?;
    Ok(a[0])
}
#[inline(never)]
fn en_include_gen(c: &mut Ctx, slf: Value, a: &[Value], _blk: Blk) -> RV {
    let d = c.ary_new_from(&[a[0]]);
    let hit = run_each_stop(c, slf, |c, d, args, _, _o| {
        let v = pack(c, args);
        if crate::builtins::array_methods::rb_eq(c, v, c.ary_get(d, 0))? { return enum_stop(c, d); }
        Ok(NIL)
    }, d, Blk::NONE)?;
    Ok(Value::bool(hit))
}
#[cold]
#[inline(never)]
fn find_multi(c: &mut Ctx, d: Value, args: &[Value], o: Blk) -> RV {
    let v = pack(c, args);
    if enum_yield_n(c, o, args.len(), v)?.truthy() { c.ary_mut(d)[0] = v; return enum_stop(c, d); }
    Ok(NIL)
}
fn en_find(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, crate::symbol::intern(b"find"), a, NIL); }
    if let Some(r) = crate::builtins::hash_methods::find_lambda(c, slf, a, blk) { return r; }
    let d = c.ary_new_from(&[NIL]);
    let hit = run_each_stop(c, slf, |c, d, args, _, o| {
        if let &[v] = args {
            if yield1(c, o, v)?.truthy() { c.ary_mut(d)[0] = v; return enum_stop(c, d); }
            return Ok(NIL);
        }
        find_multi(c, d, args, o)
    }, d, blk)?;
    if hit { return Ok(c.ary_get(d, 0)); }
    match a.first() { Some(&ifnone) if ifnone != NIL => funcall(c, ifnone, s::call, &[], std::ptr::null()), _ => Ok(NIL) }
}
fn en_count(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    if !a.is_empty() && !blk.is_null() { crate::builtins::kernel::gvar_warn(c, "given block not used", false)?; }
    let mode = if !a.is_empty() { 1 } else if !blk.is_null() { 2 } else { 0 };
    let d = c.ary_new_from(&[Value::fix(0), Value::fix(mode), a.first().copied().unwrap_or(NIL)]);
    run_each(c, slf, |c, d, args, _, o| {
        let hit = match c.ary_get(d, 1).as_fix() {
            0 => true,
            1 => { let v = pack(c, args); crate::builtins::array_methods::rb_eq(c, v, c.ary_get(d, 2))? }
            _ => yield_block(c, o, args)?.truthy(),
        };
        if hit { c.ary_mut(d)[0] = Value::fix(c.ary_get(d, 0).as_fix() + 1); }
        Ok(NIL)
    }, d, &[], blk)?;
    Ok(c.ary_get(d, 0))
}
// enum.c minmax_by: elements taken in pairs, the smaller key against the minimum, the larger against the maximum.
// d: [unused, min, max, min_key, max_key, last, last_key, state (bit 0: min/max set, bit 1: last set)]
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
fn en_minmax_by(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return no_blk(c, slf, "minmax_by", a); }
    let d = c.ary_new_from(&[NIL, NIL, NIL, NIL, NIL, NIL, NIL, Value::fix(0)]);
    run_each(c, slf, |c, d, args, _, o| {
        let mut i = pack(c, args);
        let mut vi = enum_yield(c, o, args.len(), i)?;
        let st = c.ary_get(d, 7).as_fix();
        if st & 2 == 0 { c.ary_mut(d)[5] = i; c.ary_mut(d)[6] = vi; c.ary_mut(d)[7] = Value::fix(st | 2); return Ok(NIL); }
        let (mut j, mut vj) = (c.ary_get(d, 5), c.ary_get(d, 6));
        c.ary_mut(d)[7] = Value::fix(st & !2);
        let n = crate::builtins::array_methods::rb_cmp(c, vj, vi)?;
        if n.is_eq() { i = j; vi = vj; } else if n.is_lt() { std::mem::swap(&mut i, &mut j); std::mem::swap(&mut vi, &mut vj); }
        minmax_by_update(c, d, vi, vj, i, j)?;
        Ok(NIL)
    }, d, &[], blk)?;
    if c.ary_get(d, 7).as_fix() & 2 != 0 { let (l, lv) = (c.ary_get(d, 5), c.ary_get(d, 6)); minmax_by_update(c, d, lv, lv, l, l)?; }
    Ok(c.ary_new_from(&[c.ary_get(d, 1), c.ary_get(d, 2)]))
}
// enum.c min_by / max_by: OPTIMIZED_CMP(key, best key); with n, rb_nmin_run over (key, value) pairs
fn minmax_by_one(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk, mx: bool) -> RV {
    use crate::builtins::array_methods::Nmin;
    check_argc(c, a, 0, 1)?;
    let name = if mx { "max_by" } else { "min_by" };
    if blk.is_null() { return no_blk(c, slf, name, a); }
    if let Some(&n) = a.first().filter(|n| **n != NIL) {
        let Some(nm) = Nmin::new(c, n, true, mx, blk)? else { return Ok(c.ary_new()); };
        let d = nm.save(c);
        run_each(c, slf, |c, d, args, _, o| {
            let mut nm = Nmin::load(c, d, o);
            let v = pack(c, args);
            let k = enum_yield(c, o, args.len(), v)?;
            nm.add(c, v, k)?;
            nm.store(c, d);
            Ok(NIL)
        }, d, &[], blk)?;
        return Nmin::load(c, d, blk).finish(c);
    }
    // d: [unused, best, best key, set?, max?]
    let d = c.ary_new_from(&[NIL, NIL, NIL, FALSE, Value::bool(mx)]);
    run_each(c, slf, |c, d, args, _, o| {
        let v = pack(c, args);
        let k = enum_yield(c, o, args.len(), v)?;
        let take = if c.ary_get(d, 3) == FALSE { true } else {
            let o = crate::builtins::array_methods::rb_cmp(c, k, c.ary_get(d, 2))?;
            if c.ary_get(d, 4) == TRUE { o.is_gt() } else { o.is_lt() }
        };
        if take { let a = c.ary_mut(d); a[1] = v; a[2] = k; a[3] = TRUE; }
        Ok(NIL)
    }, d, &[], blk)?;
    Ok(c.ary_get(d, 1))
}
// Enumerable#min / #max: min_i (OPTIMIZED_CMP(element, best)) / min_ii; with n, rb_nmin_run
fn en_min_max(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk, mx: bool) -> RV {
    use crate::builtins::array_methods::{cmp_result, minmax_impl, rb_cmp};
    check_argc(c, a, 0, 1)?;
    let all = e_to_a(c, NIL, &[slf], Blk::NONE)?;
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
fn en_min(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV { en_min_max(c, slf, a, blk, false) }
fn en_max(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV { en_min_max(c, slf, a, blk, true) }
fn en_minmax(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV {
    let all = e_to_a(c, NIL, &[slf], Blk::NONE)?;
    crate::builtins::array_methods::minmax_pairs(c, all, blk)
}
fn en_min_by(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV { minmax_by_one(c, slf, a, blk, false) }
fn en_max_by(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV { minmax_by_one(c, slf, a, blk, true) }
fn en_group_by(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return no_blk(c, slf, "group_by", a); }
    let acc0 = c.hash_new();
    run_each(c, slf, |c, h, args, _, o| {
        let v = pack(c, args);
        let k = enum_yield(c, o, args.len(), v)?;
        // group_by_i: rb_hash_aref (a redefined Hash#default answers a miss)
        let got = match crate::hash::hash_get_opt(c, h, k)? { Some(x) => x, None => crate::hash::hash_default_value(c, h, k)? };
        if is_array(got) { c.ary_push(got, v); } else { let arr = c.ary_new_from(&[v]); crate::hash::hash_aset(c, h, k, arr)?; }
        Ok(NIL)
    }, acc0, &[], blk)?;
    Ok(acc0)
}
#[inline(never)]
fn en_inject_gen(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { check_argc(c, a, 1, 2)?; } else { check_argc(c, a, 0, 2)?; if a.len() == 2 && c.vm().verbose.get() == TRUE { crate::builtins::kernel::gvar_warn(c, "given block not used", false)?; } }
    // d = [acc (UNDEF = first element pending), op (nil = block)]
    let (acc, op) = match (a.len(), blk.is_null()) {
        (2, _) => (a[0], a[1]),
        (1, true) => (UNDEF, a[0]),
        (1, false) => (a[0], NIL),
        _ => (UNDEF, NIL),
    };
    let op = if op == NIL || op.is_sym() { op } else { Value::sym(crate::builtins::object::to_sym_arg(c, op)?) };
    let d = c.ary_new_from(&[acc, op]);
    run_each(c, slf, |c, d, args, _, o| {
        let v = pack(c, args);
        let acc = c.ary_get(d, 0);
        if acc == UNDEF { c.ary_mut(d)[0] = v; return Ok(NIL); }
        let op = c.ary_get(d, 1);
        let r = if op == NIL { yield_block(c, o, &[acc, v])? } else {
            let m = op.as_sym();
            if m == s::plus && acc.is_fixnum() && v.is_fixnum() {
                match acc.as_fix().checked_add(v.as_fix()) { Some(x) if Value::fixable(x) => Value::fix(x), _ => funcall_public(c, acc, m, &[v], std::ptr::null())? }
            } else { funcall_public(c, acc, m, &[v], std::ptr::null())? }
        };
        c.ary_mut(d)[0] = r;
        Ok(NIL)
    }, d, &[], blk)?;
    let r = c.ary_get(d, 0);
    Ok(if r == UNDEF { NIL } else { r })
}

fn slice_cons(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk, cons: bool) -> RV {
    let n = num_to_long(c, a[0])?;
    if n <= 0 { return arg_err(c, if cons { "invalid size" } else { "invalid slice size" }); }
    let mid = if cons { "each_cons" } else { "each_slice" };
    // the size is asked for when wanted (enum_each_slice_size), not now
    if blk.is_null() { let nv = c.int_from_i64(n); return crate::builtins::enumerator::enum_for(c, slf, crate::symbol::intern(mid.as_bytes()), &[nv], NIL); }
    // limit_by_enum_size / enum_size_over_p: rb_check_funcall(obj, :size); each_cons stops on a short one
    let sz = if respond_to(c, slf, s::size, true) { funcall(c, slf, s::size, &[], std::ptr::null())? } else { NIL };
    if cons && sz.is_fixnum() && (n as u64) > sz.as_fix() as u64 { return Ok(slf); }
    let acc0 = c.ary_new();
    let d = c.ary_new_from(&[acc0, Value::fix(n.min(1 << 60)), Value::bool(cons)]);
    run_each(c, slf, |c, d, args, _, o| {
        let v = pack(c, args);
        let buf = c.ary_get(d, 0);
        let n = c.ary_get(d, 1).as_fix() as usize;
        c.ary_push(buf, v);
        if c.ary_get(d, 2).truthy() {
            if c.ary_len(buf) > n { c.ary_mut(buf).remove(0); }
            if c.ary_len(buf) == n { let cp = c.ary_from_vec(c.ary(buf).to_vec()); return yield1(c, o, cp); }
        } else if c.ary_len(buf) == n {
            c.ary_mut(d)[0] = c.ary_new();
            return yield1(c, o, buf);
        }
        Ok(NIL)
    }, d, &[], blk)?;
    let buf = c.ary_get(d, 0);
    if !cons && c.ary_len(buf) > 0 { yield1(c, blk, buf)?; }
    Ok(slf)
}
#[inline(never)]
fn en_each_slice_gen(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV { slice_cons(c, slf, a, blk, false) }
#[inline(never)]
fn en_each_cons_gen(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV { slice_cons(c, slf, a, blk, true) }

// Enumerable's own entries of the methods Array / Hash answer natively: a thin test, then a tail call
// to the Array / Hash body or the generic one (so the call costs what the Array / Hash method did)
fn en_map(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if hash_direct(c, slf) { hm::h_map(c, slf, a, blk) } else { en_map_gen(c, slf, a, blk) }
}
fn en_flat_map(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if ary_direct(c, slf) { am::a_flat_map(c, slf, a, blk) } else if hash_direct(c, slf) { hm::h_flat_map(c, slf, a, blk) } else { en_flat_map_gen(c, slf, a, blk) }
}
fn en_filter_map(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if ary_direct(c, slf) { am::a_filter_map(c, slf, a, blk) } else if hash_direct(c, slf) { hm::h_filter_map(c, slf, a, blk) } else { en_filter_map_gen(c, slf, a, blk) }
}
fn en_to_a(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if a.is_empty() && ary_direct(c, slf) { am::a_entries(c, slf, a, blk) } else { en_to_a_gen(c, slf, a, blk) }
}
fn en_each_with_index(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if a.is_empty() && ary_direct(c, slf) { am::a_each_with_index(c, slf, a, blk) }
    else if a.is_empty() && hash_direct(c, slf) { hm::h_each_with_index(c, slf, a, blk) } else { en_each_with_index_gen(c, slf, a, blk) }
}
fn en_include(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if ary_direct(c, slf) { am::a_include(c, slf, a, blk) } else { en_include_gen(c, slf, a, blk) }
}
fn en_inject(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if ary_direct(c, slf) { am::a_inject(c, slf, a, blk) } else { en_inject_gen(c, slf, a, blk) }
}
fn en_sort_by(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if ary_direct(c, slf) { am::a_sort_by(c, slf, a, blk) } else { en_sort_by_gen(c, slf, a, blk) }
}
fn en_each_slice(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if ary_direct(c, slf) { am::a_each_slice(c, slf, a, blk) } else { en_each_slice_gen(c, slf, a, blk) }
}
fn en_each_cons(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if ary_direct(c, slf) { am::a_each_cons(c, slf, a, blk) } else { en_each_cons_gen(c, slf, a, blk) }
}

/// (Enumerable's entry, of Hash?, the body, its arity): what fill_cc caches for an Array / Hash site (direct_body)
static DIRECT: std::sync::OnceLock<Vec<(usize, bool, CFunc, i32)>> = std::sync::OnceLock::new();
fn init_direct(c: &Ctx, m: Value) {
    let ary: [(&str, CFunc, i32); 11] = [("each_slice", am::a_each_slice, 1), ("each_cons", am::a_each_cons, 1), ("inject", am::a_inject, -1),
        ("reduce", am::a_inject, -1), ("each_with_index", am::a_each_with_index, 0), ("sort_by", am::a_sort_by, 0), ("flat_map", am::a_flat_map, 0),
        ("collect_concat", am::a_flat_map, 0), ("filter_map", am::a_filter_map, 0), ("member?", am::a_include, 1), ("entries", am::a_entries, 0)];
    let hash: [(&str, CFunc, i32); 6] = [("map", hm::h_map, 0), ("collect", hm::h_map, 0), ("flat_map", hm::h_flat_map, 0),
        ("collect_concat", hm::h_flat_map, 0), ("filter_map", hm::h_filter_map, 0), ("each_with_index", hm::h_each_with_index, 0)];
    let me = |n: &str| c.method_entry(m, crate::symbol::intern(n.as_bytes())).map_or(0, |e| e as *const MethodEntry as usize);
    let t = ary.iter().map(|&(n, f, a)| (me(n), false, f, a)).chain(hash.iter().map(|&(n, f, a)| (me(n), true, f, a))).collect();
    let _ = DIRECT.set(t);
}
/// fill_cc of a site whose cache gets Enumerable's entry `me` for an Array / Hash receiver (klass exactly the class,
/// its #each the builtin; a redefinition refills every cache): the body that entry's cfunc would call, with the
/// arity it takes (a site passing other arguments calls the entry).
pub fn direct_body(c: &Ctx, me: *const MethodEntry, klass: Value) -> Option<(CFunc, i32)> {
    let v = c.vm();
    let hash = if klass == v.c_array.get() { if crate::rt::redef(crate::rt::R_ARY_EACH) { return None; } false }
        else if klass == v.c_hash.get() { if !matches!(c.method_entry(klass, s::each), Some(e) if matches!(e.kind, MKind::Cfunc(f, _) if f as *const () as usize == hm::h_each as *const () as usize)) { return None; } true }
        else { return None };
    DIRECT.get()?.iter().find(|&&(m, h, _, _)| m == me as usize && h == hash).map(|&(_, _, f, a)| (f, a))
}

// enum.c partition_i / tally_i (no Ruby-level packing: ENUM_WANT_SVALUE is `pack`)
fn en_partition(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return no_blk(c, slf, "partition", a); }
    let (t, f) = (c.ary_new(), c.ary_new());
    let d = c.ary_new_from(&[t, f]);
    run_each(c, slf, |c, d, args, _, o| {
        let v = pack(c, args);
        let i = if enum_yield(c, o, args.len(), v)?.truthy() { 0 } else { 1 };
        c.ary_push(c.ary_get(d, i), v);
        Ok(NIL)
    }, d, &[], blk)?;
    Ok(c.ary_new_from(&[t, f]))
}
fn en_tally(c: &mut Ctx, slf: Value, a: &[Value], _blk: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let h = if a.is_empty() { c.hash_new() } else {
        let h = e_hash(c, NIL, a, Blk::NONE)?;
        if is_frozen(h) { return Err(raise_frozen(c, h)); }
        h
    };
    run_each(c, slf, |c, h, args, _, _o| { let v = pack(c, args); crate::hash::hash_tally_up(c, h, v)?; Ok(NIL) }, h, &[], Blk::NONE)?;
    Ok(h)
}

// Enumerable.__each(obj, *args) { |v| }: obj.each(*args) with each yield packed (ENUM_WANT_SVALUE)
fn e_each(c: &mut Ctx, _s: Value, a: &[Value], blk: Blk) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    run_each(c, a[0], |c, _d, args, _, o| { let v = pack(c, args); enum_yield(c, o, args.len(), v) }, NIL, &a[1..], blk)?;
    Ok(a[0])
}
// Enumerable.__set_init(h, enum) { |x| }: set.c set_i_initialize (an Array is read directly, no #each)
fn e_set_init(c: &mut Ctx, _s: Value, a: &[Value], blk: Blk) -> RV {
    let (h, e) = (a[0], a[1]);
    set_merge_one(c, h, e, blk)
}
/// set_merge_enum_into (a Set's table or an Array read directly), or set_i_initialize with a block
fn set_merge_one(c: &mut Ctx, h: Value, e: Value, blk: Blk) -> RV {
    if blk.is_null() && e.is_heap() && !is_array(e) {
        let src = ivar_get(c, e, crate::symbol::intern(b"__h"));
        if is_hash(src) {
            let mut slot = 0;
            while let Some((k, _)) = c.hash_entry_at(src, slot) { slot += 1; if k != UNDEF { crate::hash::hash_aset(c, h, k, TRUE)?; } }
            return Ok(NIL);
        }
    }
    if is_array(e) {
        let mut i = 0;
        while i < c.ary_len(e) {
            let x = c.ary_get(e, i);
            let k = if blk.is_null() { x } else { yield1(c, blk, x)? };
            crate::hash::hash_aset(c, h, k, TRUE)?;
            i += 1;
        }
        return Ok(NIL);
    }
    let ee = crate::symbol::intern(b"each_entry");
    let has_ee = respond_to(c, e, ee, true);
    if !has_ee && !respond_to(c, e, s::each, true) { return arg_err(c, "value must be enumerable"); }
    let f: CBlockFn = |c, h, args, _, o| {
        let v = pack(c, args);
        let k = if o.is_null() { v } else { yield1(c, o, v)? };
        crate::hash::hash_aset(c, h, k, TRUE)?;
        Ok(NIL)
    };
    // enum_method_id: #each_entry when it responds (Enumerable's own is #each with the same packing)
    if has_ee && !c.me_ref(lookup_method(c, class_of(e), ee)).is_some_and(|m| m.owner == c.vm().m_enumerable.get()) {
        let b = Block::cfunc_for(c, f, h, blk);
        funcall(c, e, ee, &[], &b)?;
    } else { run_each(c, e, f, h, &[], blk)?; }
    Ok(NIL)
}
// Enumerable.__set_merge(h, enums): set_i_merge's loop over its arguments
// Enumerable.__set_like(set): set.class.new, compare_by_identity when set is (4.0.10's &, classify, divide, flatten)
fn e_set_like(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let k = funcall(c, a[0], crate::symbol::intern(b"class"), &[], std::ptr::null())?;
    let n = funcall(c, k, crate::symbol::intern(b"new"), &[], std::ptr::null())?;
    let h = ivar_get(c, a[0], crate::symbol::intern(b"__h"));
    if is_hash(h) && c.hash(h).identity { funcall(c, n, crate::symbol::intern(b"compare_by_identity"), &[], std::ptr::null())?; }
    Ok(n)
}
fn e_set_merge(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let mut i = 0;
    while i < c.ary_len(a[1]) { let e = c.ary_get(a[1], i); set_merge_one(c, a[0], e, Blk::NONE)?; i += 1; }
    Ok(NIL)
}
// Enumerable.__set_eq(h1, h2): set_i_eq after the class test (sizes, identity, then each key looked up)
fn e_set_eq(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (h1, h2) = (a[0], a[1]);
    if c.hash_len(h1) != c.hash_len(h2) || c.hash(h1).identity != c.hash(h2).identity { return Ok(FALSE); }
    let mut slot = 0;
    while let Some((k, _)) = c.hash_entry_at(h1, slot) {
        slot += 1;
        if k == UNDEF { continue; }
        if crate::hash::hash_lookup(c, h2, k)?.is_none() { return Ok(FALSE); }
    }
    Ok(TRUE)
}

fn define_enumerable_natives(c: &mut Ctx, m: Value) {
    define_cfunc(c, m, "partition", en_partition, 0); define_cfunc(c, m, "tally", en_tally, -1);
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
    define_cfunc(c, m, "sort_by", en_sort_by, 0);
    init_direct(c, m);
}

/// Defines the native methods of Enumerable `m`.
pub fn init(c: &mut Ctx, m: Value) {
    define_enumerable_natives(c, m);
    // the prelude's helpers (`__saruby__.__each(..)`): on the internal module, not on Enumerable
    let m = c.vm().m_internal.get();
    define_singleton_cfunc(c, m, "__int", e_int, 1);
    define_singleton_cfunc(c, m, "__hash", e_hash, 1);
    define_singleton_cfunc(c, m, "__argc", e_argc, 3);
    define_singleton_cfunc(c, m, "__kw", e_kw, -1);
    define_singleton_cfunc(c, m, "__to_a", e_to_a, 1);
    define_singleton_cfunc(c, m, "__range_sum", |c, _s, a, b| crate::builtins::range_methods::r_sum(c, a[0], &a[1..], b), 2);
    define_singleton_cfunc(c, m, "__ivget", e_ivget, 2);
    define_singleton_cfunc(c, m, "__set_init", e_set_init, 2);
    define_singleton_cfunc(c, m, "__each", e_each, -1);
    define_singleton_cfunc(c, m, "__set_eq", e_set_eq, 2);
    define_singleton_cfunc(c, m, "__set_merge", e_set_merge, 2);
    define_singleton_cfunc(c, m, "__set_like", e_set_like, 1);
    define_singleton_cfunc(c, m, "__ivset", e_ivset, 3);
}
