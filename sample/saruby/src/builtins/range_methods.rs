// Range, on the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::array::*;
use crate::builtins::kernel::*;
use crate::range::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

macro_rules! cfn { ($name:ident, |$c:ident, $s:ident, $a:ident, $b:ident| $body:expr) => {
    #[allow(unused_variables)]
    pub fn $name($c: &mut Ctx, $s: Value, $a: &[Value], $b: Blk) -> RV { $body }
}; }

/// RANGE_FL_INIT (a bit of the exception-only FL_PRIVATE_CALL); a range with an end is initialized too
const FL_RANGE_INIT: u32 = crate::object::FL_PRIVATE_CALL;
cfn!(r_initialize, |c, slf, a, blk| {
    check_argc(c, a, 2, 3)?;
    // range_modify
    check_frozen(c, slf)?;
    let (b0, e0, _) = c.range(slf);
    if obj_flags(slf) & FL_RANGE_INIT != 0 || b0 != NIL || e0 != NIL {
        let e = make_name_error(c, c.vm().c_name_error.get(), "'initialize' called twice", Value::sym(s::initialize), slf)?;
        return Err(raise_exc(c, e));
    }
    range_check(c, a[0], a[1])?;
    c.range_set(slf, a[0], a[1], a.len() > 2 && a[2].truthy());
    obj_set_flags(slf, FL_RANGE_INIT);
    if obj_klass(slf) == c.vm().c_range.get() { obj_set_flags(slf, FL_FROZEN); }
    Ok(NIL)
});
cfn!(r_begin, |c, slf, a, blk| Ok(c.range_parts(slf).0));
cfn!(r_end, |c, slf, a, blk| Ok(c.range_parts(slf).1));
cfn!(r_excl, |c, slf, a, blk| Ok(Value::bool(c.range_parts(slf).2)));
fn int_end(c: &mut Ctx, e: Value) -> Result<Option<i64>, Esc> {
    if e.is_fixnum() { return Ok(Some(e.as_fix())); }
    if is_float(e) { let f = float_val(e); if f.is_infinite() && f > 0.0 { return Ok(None); } return Ok(Some(f.floor() as i64)); }
    if e == NIL { return Ok(None); }
    Ok(Some(crate::builtins::numeric_methods::num_to_long(c, e)?))
}
pub fn range_each(c: &mut Ctx, slf: Value, mut f: impl FnMut(&mut Ctx, Value) -> Result<bool, Esc>) -> Result<(), Esc> {
    let (b, e, excl) = c.range_parts(slf);
    // Integer..Float is range.c's generic path: rb_check_string_type(beg), then <=> and succ per element
    let generic = is_float(e) && crate::rt::redef(crate::rt::R_INT_CMP | crate::rt::R_INT_SUCC | crate::rt::R_IMM_CONV);
    if b.is_fixnum() && !generic && (e.is_fixnum() || e == NIL || is_float(e)) {
        let mut i = b.as_fix();
        let last = int_end(c, e)?;
        let excl = excl && e.is_fixnum() || (is_float(e) && excl && float_val(e).fract() == 0.0);
        loop {
            if let Some(l) = last { if if excl { i >= l } else { i > l } { break; } }
            if !f(c, Value::fix(i))? { break; }
            i += 1;
        }
        return Ok(());
    }
    if b.is_sym() && e.is_sym() {
        // Symbol ranges iterate like their String forms (String#upto)
        let (bb, eb) = (crate::symbol::sym_bytes(b.as_sym()).to_vec(), crate::symbol::sym_bytes(e.as_sym()).to_vec());
        if bb.len() == 1 && eb.len() == 1 && bb[0] < 0x80 && eb[0] < 0x80 {
            let (lo, hi) = (bb[0], if excl { eb[0] as i32 - 1 } else { eb[0] as i32 });
            let mut ch = lo as i32;
            while ch <= hi { if !f(c, Value::sym(intern(&[ch as u8])))? { break; } ch += 1; }
            return Ok(());
        }
        let (bs, es) = (c.str_new(&bb), c.str_new(&eb));
        let blk_data = c.ary_new();
        let collect = Block::cfunc(c, |c, d, args, _b, _o| { c.ary_push(d, args[0]); Ok(NIL) }, blk_data);
        let ev = if excl { TRUE } else { FALSE };
        funcall(c, bs, s::upto, &[es, ev], &collect)?;
        for i in 0..c.ary_len(blk_data) { let v = Value::sym(intern(c.str_bytes(c.ary_get(blk_data, i)))); if !f(c, v)? { break; } }
        return Ok(());
    }
    if is_string(b) && (is_string(e) || e == NIL) {
        if e == NIL {
            let mut cur = c.str_bytes(b).to_vec();
            loop { let s2 = c.str_new(&cur); if !f(c, s2)? { return Ok(()); } cur = succ_bytes(&cur); }
        }
        let (bb, eb) = (c.str_bytes(b).to_vec(), c.str_bytes(e).to_vec());
        if bb.len() == 1 && eb.len() == 1 && bb[0] < 0x80 && eb[0] < 0x80 {
            let (lo, hi) = (bb[0] as i32, if excl { eb[0] as i32 - 1 } else { eb[0] as i32 });
            let mut ch = lo;
            while ch <= hi { let s1 = c.str_new(&[ch as u8]); if !f(c, s1)? { break; } ch += 1; }
            return Ok(());
        }
        let blk_data = c.ary_new();
        let collect = Block::cfunc(c, |c, d, args, _b, _o| { c.ary_push(d, args[0]); Ok(NIL) }, blk_data);
        let ev = if excl { TRUE } else { FALSE };
        funcall(c, b, s::upto, &[e, ev], &collect)?;
        for i in 0..c.ary_len(blk_data) { let v = c.ary_get(blk_data, i); if !f(c, v)? { break; } }
        return Ok(());
    }
    if generic && respond_to(c, b, s::to_str, true) {
        let tmp = funcall(c, b, s::to_str, &[], Blk::NONE)?;
        if is_string(tmp) {
            let blk_data = c.ary_new();
            let collect = Block::cfunc(c, |c, d, args, _b, _o| { c.ary_push(d, args[0]); Ok(NIL) }, blk_data);
            let ev = if excl { TRUE } else { FALSE };
            funcall(c, tmp, s::upto, &[e, ev], &collect)?;
            for i in 0..c.ary_len(blk_data) { let v = c.ary_get(blk_data, i); if !f(c, v)? { break; } }
            return Ok(());
        }
    }
    if b == NIL || is_float(b) { return type_err(c, format!("can't iterate from {}", class_path(c, real_class_of(c, b)))); }
    if !respond_to(c, b, s::succ, true) { return type_err(c, format!("can't iterate from {}", class_path(c, real_class_of(c, b)))); }
    // range_each_func: no #succ past the end
    let mut cur = b;
    loop {
        let mut last = false;
        if e != NIL {
            let v = r_less(c, cur, e)?;
            if v > 0 || (excl && v == 0) { break; }
            last = v == 0;
        }
        if !f(c, cur)? || last { break; }
        cur = funcall(c, cur, s::succ, &[], std::ptr::null())?;
    }
    Ok(())
}
// Range#each is identified by its entry (vm.ids.r_each.get()), not by function pointer
/// The loop of Range#each over fixnums, yielding through `y` (inline_iter.rs); None: other bounds.
#[inline(always)]
pub fn r_each_loop(c: &mut Ctx, slf: Value, y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> Option<RV> {
    let (b, e, excl) = c.range_parts(slf);
    if !b.is_fixnum() || !e.is_fixnum() { return None; }
    let mut i = b.as_fix(); let l = e.as_fix();
    let l = if excl { l - 1 } else { l };
    while i <= l { if let Err(e) = y(c, &[Value::fix(i)]) { return Some(Err(e)); } i += 1; }
    Some(Ok(slf))
}
cfn!(r_each, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, s::each, a); }
    if let Some(r) = r_each_loop(c, slf, &mut |c, a| yield1(c, blk, a[0])) { return r; }
    range_each(c, slf, |c, v| { yield1(c, blk, v)?; Ok(true) })?;
    Ok(slf)
});
fn cant_iterate<T>(c: &mut Ctx, v: Value) -> Result<T, Esc> { type_err(c, format!("can't iterate from {}", class_path(c, real_class_of(c, v)))) }
fn discrete(c: &mut Ctx, v: Value) -> bool { v.is_fixnum() || respond_to(c, v, s::succ, false) }
fn is_numeric_obj(c: &Ctx, v: Value) -> bool { crate::numeric::is_numeric(v) || (v.is_heap() && is_kind_of(c, v, c.vm().c_numeric.get())) }
/// range_reverse_size
fn reverse_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk, _o: Blk) -> RV {
    let (b, e, x) = c.range_parts(slf);
    if e == NIL { return cant_iterate(c, e); }
    if is_integer(b) { if is_numeric_obj(c, e) { return crate::builtins::numeric_methods::interval_step_size(c, b, e, Value::fix(1), x); } return cant_iterate(c, e); }
    if b == NIL { if is_integer(e) { return Ok(c.float_new(f64::INFINITY)); } return cant_iterate(c, e); }
    if !discrete(c, b) { return cant_iterate(c, e); }
    Ok(NIL)
}
cfn!(r_reverse_each, |c, slf, a, blk| {
    if blk.is_null() { let sz = crate::builtins::numeric_methods::size_proc(c, reverse_size, slf); return crate::builtins::enumerator::enum_for(c, slf, intern(b"reverse_each"), a, sz); }
    let (b, e, excl) = c.range_parts(slf);
    if e.is_fixnum() && (b.is_fixnum() || b == NIL) {
        let mut i = if excl { e.as_fix() - 1 } else { e.as_fix() };
        loop { if b != NIL && i < b.as_fix() { break; } yield1(c, blk, Value::fix(i))?; i -= 1; }
        return Ok(slf);
    }
    if e == NIL { return type_err(c, format!("can't iterate from {}", class_path(c, real_class_of(c, e)))); }
    if is_integer(e) && (is_integer(b) || b == NIL) {
        // bignum bounds: count down as range.c does, without building the array
        use crate::bignum::BigInt;
        let one = BigInt::from_i64(1);
        let mut cur = c.to_big(e);
        if excl { cur = cur.sub(&one); }
        let lo = if b == NIL { None } else { Some(c.to_big(b)) };
        loop {
            if let Some(l) = &lo { if cur.cmp(l) == std::cmp::Ordering::Less { break; } }
            let v = c.big_to_value(cur.clone());
            yield1(c, blk, v)?;
            cur = cur.sub(&one);
        }
        return Ok(slf);
    }
    let arr = r_to_a(c, slf, &[], Blk::NONE)?;
    let n = c.ary_len(arr);
    for i in (0..n).rev() { let v = c.ary_get(arr, i); yield1(c, blk, v)?; }
    Ok(slf)
});
cfn!(r_to_a, |c, slf, a, blk| {
    let (b, e, _) = c.range_parts(slf);
    if e == NIL { return err(c, c.vm().c_range_error.get(), "cannot convert endless range to an array"); }
    // range_to_a is Enumerable#to_a: a redefined or subclass #each is called
    if (obj_klass(slf) != c.vm().c_range.get() || crate::rt::redef(crate::rt::R_RANGE_EACH)) && crate::builtins::enum_native::each_overridden(c, slf, r_each) {
        return crate::builtins::enum_native::call_enumerable(c, slf, "to_a", a, blk);
    }
    let out = c.ary_new();
    range_each(c, slf, |c, v| { c.ary_push(out, v); Ok(true) })?;
    let _ = b;
    Ok(out)
});
// range_size
cfn!(r_size, |c, slf, a, blk| {
    let (b, e, excl) = c.range_parts(slf);
    if b.is_fixnum() && e.is_fixnum() { let n = e.as_fix() - b.as_fix() + if excl { 0 } else { 1 }; return Ok(Value::fix(n.max(0))); }
    if is_integer(b) {
        if is_numeric_obj(c, e) { return crate::builtins::numeric_methods::interval_step_size(c, b, e, Value::fix(1), excl); }
        if e == NIL { return Ok(c.float_new(f64::INFINITY)); }
    }
    if !discrete(c, b) { return cant_iterate(c, b); }
    Ok(NIL)
});
cfn!(r_count, |c, slf, a, blk| {
    if a.is_empty() && blk.is_null() {
        let (b, e, _) = c.range_parts(slf);
        if e == NIL || (b == NIL && e != NIL) { return Ok(c.float_new(f64::INFINITY)); }
        if is_integer(b) { return r_size(c, slf, a, blk); }
    }
    crate::builtins::enum_native::call_enumerable(c, slf, "count", a, blk)   // range_count: rb_call_super
});
/// range.c r_less: <=> as -1/0/1, or i64::MAX when incomparable (nil).
fn r_less(c: &mut Ctx, a: Value, b: Value) -> Result<i64, Esc> {
    if a.is_fixnum() && b.is_fixnum() && !crate::rt::redef(crate::rt::R_INT_CMP) { return Ok(a.as_fix().cmp(&b.as_fix()) as i64); }
    let r = funcall(c, a, s::cmp, &[b], std::ptr::null())?;
    if r == NIL { return Ok(i64::MAX); }
    crate::builtins::array_methods::cmp_result(c, r, a, b)
}
/// range.c r_cover_p.
fn cover_in(c: &mut Ctx, b: Value, e: Value, excl: bool, v: Value) -> Result<bool, Esc> {
    if b == NIL || r_less(c, b, v)? <= 0 {
        if e == NIL || r_less(c, v, e)? <= -(excl as i64) { return Ok(true); }
    }
    Ok(false)
}
fn cover(c: &mut Ctx, slf: Value, v: Value) -> Result<bool, Esc> {
    let (b, e, excl) = c.range_parts(slf);
    cover_in(c, b, e, excl, v)
}
/// range.c linear_object_p: Numeric or Time.
fn linear(c: &Ctx, v: Value) -> bool { crate::numeric::is_numeric(v) || (v.is_heap() && (is_kind_of(c, v, c.vm().c_numeric.get()) || is_kind_of(c, v, c.vm().c_time.get()))) }
cfn!(r_include, |c, slf, a, blk| {
    check_argc(c, a, 1, 1)?;
    let (b, e, excl) = c.range_parts(slf);
    let v = a[0];
    if b.is_fixnum() || e.is_fixnum() || linear(c, b) || linear(c, e)
        || crate::builtins::random::check_to_int(c, b)?.is_some() || crate::builtins::random::check_to_int(c, e)?.is_some() {
        return Ok(Value::bool(cover_in(c, b, e, excl, v)?));
    }
    if is_string(b) && is_string(e) {
        // rb_str_include_range_p
        if v == NIL { return Ok(FALSE); }
        let v = if is_string(v) { v } else if v.is_heap() && respond_to(c, v, s::to_str, true) { expect_str(c, v)? } else { return Ok(FALSE); };
        let (bb, eb, vb) = (c.str_bytes(b).to_vec(), c.str_bytes(e).to_vec(), c.str_bytes(v).to_vec());
        if bb.len() == 1 && eb.len() == 1 {
            if vb.len() != 1 { return Ok(FALSE); }
            let (x, y, z) = (bb[0], eb[0], vb[0]);
            if x.is_ascii() && y.is_ascii() && z.is_ascii() { return Ok(Value::bool((x <= z && z < y) || (!excl && z == y))); }
        }
        let mut found = false;
        range_each(c, slf, |c, x| { if crate::builtins::array_methods::rb_eq(c, x, v)? { found = true; return Ok(false); } Ok(true) })?;
        return Ok(Value::bool(found));
    }
    if b == NIL && e == NIL && linear(c, v) { return Ok(TRUE); }
    if b == NIL || e == NIL { return type_err(c, "cannot determine inclusion in beginless/endless ranges"); }
    // Enumerable#include?
    let mut found = false;
    range_each(c, slf, |c, x| { if crate::builtins::array_methods::rb_eq(c, x, v)? { found = true; return Ok(false); } Ok(true) })?;
    Ok(Value::bool(found))
});
/// range.c r_cover_range_p.
fn cover_range(c: &mut Ctx, slf: Value, val: Value) -> Result<bool, Esc> {
    let (b, e, excl) = c.range_parts(slf);
    let (vb, ve, vx) = c.range_parts(val);
    if e != NIL && ve == NIL { return Ok(false); }
    if b != NIL && vb == NIL { return Ok(false); }
    if vb != NIL && ve != NIL && r_less(c, vb, ve)? > (if vx { -1 } else { 0 }) { return Ok(false); }
    if vb != NIL && !cover_in(c, b, e, excl, vb)? { return Ok(false); }
    let cmp_end = if ve != NIL && e != NIL {
        let r = funcall(c, e, s::cmp, &[ve], std::ptr::null())?;
        if r == NIL { return Ok(false); }
        crate::builtins::array_methods::cmp_result(c, r, e, ve)?
    } else { r_less(c, e, ve)? };
    if excl == vx { return Ok(cmp_end >= 0); }
    if excl { return Ok(cmp_end > 0); }
    if cmp_end >= 0 { return Ok(true); }
    // val.max, a TypeError counting as "no max"
    let m = match funcall(c, val, intern(b"max"), &[], std::ptr::null()) {
        Ok(m) => m,
        Err(err) => {
            if c.esc == EscKind::Raise && is_kind_of(c, c.esc_val, c.vm().c_type_error.get()) { c.esc = EscKind::None; NIL } else { return Err(err); }
        }
    };
    if m == NIL { return Ok(false); }
    Ok(r_less(c, e, m)? >= 0)
}
cfn!(r_cover, |c, slf, a, blk| {
    check_argc(c, a, 1, 1)?;
    if is_range(a[0]) { return Ok(Value::bool(cover_range(c, slf, a[0])?)); }
    Ok(Value::bool(cover(c, slf, a[0])?))
});
cfn!(r_eqq, |c, slf, a, blk| Ok(Value::bool(cover(c, slf, a[0])?)));
cfn!(r_eq, |c, slf, a, blk| {
    if !is_range(a[0]) { return Ok(FALSE); }
    let (b1, e1, x1) = c.range_parts(slf); let (b2, e2, x2) = c.range_parts(a[0]);
    Ok(Value::bool(x1 == x2 && crate::builtins::array_methods::rb_eq(c, b1, b2)? && crate::builtins::array_methods::rb_eq(c, e1, e2)?))
});
cfn!(r_eql, |c, slf, a, blk| {
    if !is_range(a[0]) { return Ok(FALSE); }
    let (b1, e1, x1) = c.range_parts(slf); let (b2, e2, x2) = c.range_parts(a[0]);
    Ok(Value::bool(x1 == x2 && crate::hash::key_eql(c, b1, b2)? && crate::hash::key_eql(c, e1, e2)?))
});
cfn!(r_hash, |c, slf, a, blk| {
    let (b, e, x) = c.range_parts(slf);
    let h = crate::astrust::hash_merge(crate::astrust::hash_merge(crate::hash::key_hash(c, b)?, crate::hash::key_hash(c, e)?), (x as u64) << 24);
    Ok(Value::fix((h >> 2) as i64))
});
/// range_to_s / inspect_range: the encodings follow the parts (rb_str_dup + rb_str_append).
fn range_to_s(c: &mut Ctx, slf: Value, insp: bool) -> RV {
    let (b, e, x) = c.range_parts(slf);
    let f = |c: &mut Ctx, v: Value| -> RV { if insp { crate::builtins::kernel::inspect_value(c, v) } else { crate::rt::srb_tostr(c, v) } };
    let acc = if !insp || b != NIL || e == NIL { let s = f(c, b)?; c.str_dup(s) } else { c.str_new_enc(b"", ENC_BINARY) };
    obj_set_klass(acc, c.vm().c_string.get());
    c.rstring_mut(acc).b.extend_from_slice(if x { b"..." } else { b".." });
    if !insp || b == NIL || e != NIL { let s = f(c, e)?; crate::string::str_append_str(c, acc, s)?; }
    Ok(acc)
}
cfn!(r_to_s, |c, slf, a, blk| range_to_s(c, slf, false));
cfn!(r_inspect, |c, slf, a, blk| range_to_s(c, slf, true));
cfn!(r_first, |c, slf, a, blk| { if a.len() > 1 { return argc_err(c, a.len(), 1, 1); }
    let (b, _, _) = c.range_parts(slf);
    if a.is_empty() { if b == NIL { return err(c, c.vm().c_range_error.get(), "cannot get the first element of beginless range"); } return Ok(b); }
    let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative array size (or size too big)"); }
    if n >= 1 << 60 { return arg_err(c, "array size too big"); }
    let out = c.ary_new();
    if n == 0 { return Ok(out); }
    range_each(c, slf, |c, v| { c.ary_push(out, v); Ok((c.ary_len(out) as i64) < n) })?;
    Ok(out)
});
cfn!(r_last, |c, slf, a, blk| { check_argc(c, a, 0, 1)?;
    let (b, e, x) = c.range_parts(slf);
    if e == NIL { return err(c, c.vm().c_range_error.get(), "cannot get the last element of endless range"); }
    if a.is_empty() {
        return Ok(e);
    }
    // rb_int_range_last: counted back from the end, never materialized
    let each_user = (obj_klass(slf) != c.vm().c_range.get() || crate::rt::redef(crate::rt::R_RANGE_EACH)) && crate::builtins::enum_native::each_overridden(c, slf, r_each);
    if a.len() == 1 && is_integer(b) && is_integer(e) && !each_user {
        let (zero, one) = (Value::fix(0), Value::fix(1));
        let last = if x { funcall(c, e, s::minus, &[one], std::ptr::null())? } else { e };
        let span = funcall(c, last, s::minus, &[b], std::ptr::null())?;
        let len = funcall(c, span, s::plus, &[one], std::ptr::null())?;
        if !funcall(c, len, intern(b">"), &[zero], std::ptr::null())?.truthy() { return Ok(c.ary_new()); }
        let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
        if n < 0 { return arg_err(c, "negative array size"); }
        let nv = c.int_from_i64(n);
        let cntv = if funcall(c, len, intern(b">"), &[nv], std::ptr::null())?.truthy() { nv } else { len };
        if !cntv.is_fixnum() || cntv.as_fix() >= 1 << 60 { return arg_err(c, "array size too big"); }
        let cnt = cntv.as_fix();
        if cnt > 1 << 31 { return err(c, c.vm().c_no_memory_error.get(), "failed to allocate memory"); }
        let back = funcall(c, cntv, s::minus, &[one], std::ptr::null())?;
        let mut cur = funcall(c, last, s::minus, &[back], std::ptr::null())?;
        let out = c.ary_new();
        for _ in 0..cnt { c.ary_push(out, cur); cur = funcall(c, cur, s::plus, &[one], std::ptr::null())?; }
        return Ok(out);
    }
    let arr = r_to_a(c, slf, &[], Blk::NONE)?;
    let _ = (b, x);
    funcall(c, arr, intern(b"last"), a, std::ptr::null())
});
cfn!(r_min, |c, slf, a, blk| {
    let (b, e, x) = c.range_parts(slf);
    if b == NIL { return err(c, c.vm().c_range_error.get(), "cannot get the minimum of beginless range"); }
    if !blk.is_null() || !a.is_empty() {
        if a.len() > 1 && !blk.is_null() { return crate::builtins::enum_native::call_enumerable(c, slf, "min", a, blk); }   // rb_call_super
        check_argc(c, a, 0, 1)?;
        if let Some(&n) = a.first() { if n != NIL { let nn = crate::builtins::numeric_methods::num_to_long(c, n)?; if nn < 0 { return arg_err(c, "negative array size (or size too big)"); } if nn >= 1 << 60 { return arg_err(c, "array size too big"); } } }
        if blk.is_null() { return r_first(c, slf, a, blk); }
        if e == NIL { return err(c, c.vm().c_range_error.get(), "cannot get the minimum of endless range with custom comparison method"); }
        return crate::builtins::enum_native::call_enumerable(c, slf, "min", a, blk);
    }
    if e == NIL { return Ok(b); }
    let r = funcall(c, b, s::cmp, &[e], std::ptr::null())?;
    if r == NIL || r.as_fix() > 0 || (x && r.as_fix() == 0) { return Ok(NIL); }
    Ok(b)
});
cfn!(r_max, |c, slf, a, blk| {
    let (b, e, x) = c.range_parts(slf);
    if e == NIL { return err(c, c.vm().c_range_error.get(), "cannot get the maximum of endless range"); }
    if !blk.is_null() || !a.is_empty() {
        if a.len() > 1 && !blk.is_null() { return crate::builtins::enum_native::call_enumerable(c, slf, "max", a, blk); }   // rb_call_super
        check_argc(c, a, 0, 1)?;
        if let Some(&n) = a.first() { if n != NIL { let nn = crate::builtins::numeric_methods::num_to_long(c, n)?; if nn < 0 { return arg_err(c, "negative array size (or size too big)"); } if nn >= 1 << 60 { return arg_err(c, "array size too big"); } } }
        if blk.is_null() && e.is_fixnum() && (b == NIL || b.is_fixnum()) {
            let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
            let out = c.ary_new();
            let mut i = if x { e.as_fix() - 1 } else { e.as_fix() };
            while (c.ary_len(out) as i64) < n && (b == NIL || i >= b.as_fix()) { c.ary_push(out, Value::fix(i)); i -= 1; }
            return Ok(out);
        }
        // a Bignum end: counted down from it, not iterated up to it
        if blk.is_null() && is_integer(e) && (b == NIL || is_integer(b)) {
            let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
            let out = c.ary_new();
            let one = Value::fix(1);
            let mut i = if x { funcall(c, e, s::minus, &[one], std::ptr::null())? } else { e };
            while (c.ary_len(out) as i64) < n {
                if b != NIL && !funcall(c, i, intern(b">="), &[b], std::ptr::null())?.truthy() { break; }
                c.ary_push(out, i);
                i = funcall(c, i, s::minus, &[one], std::ptr::null())?;
            }
            return Ok(out);
        }
        if b == NIL && !blk.is_null() { return err(c, c.vm().c_range_error.get(), "cannot get the maximum of beginless range with custom comparison method"); }
        if b == NIL { return type_err(c, "can't iterate from NilClass"); }
        return crate::builtins::enum_native::call_enumerable(c, slf, "max", a, blk);
    }
    // range_max: an exclusive non-numeric range is iterated
    if x && !crate::numeric::is_numeric(e) {
        if b == NIL { return err(c, c.vm().c_range_error.get(), "cannot get the maximum of beginless range with custom comparison method"); }
        return crate::builtins::enum_native::call_enumerable(c, slf, "max", a, blk);
    }
    let r = if b == NIL { -1 } else { let r = funcall(c, b, s::cmp, &[e], std::ptr::null())?; if r == NIL { return Ok(NIL); } r.as_fix() };
    if r > 0 { return Ok(NIL); }
    if x {
        if !is_integer(e) { return type_err(c, "cannot exclude non Integer end value"); }
        if r == 0 { return Ok(NIL); }
        if b != NIL && !is_integer(b) { return type_err(c, "cannot exclude end value with non Integer begin value"); }
        if e.is_fixnum() { return Ok(Value::fix(e.as_fix() - 1)); }
        return funcall(c, e, s::minus, &[Value::fix(1)], std::ptr::null());
    }
    Ok(e)
});
cfn!(r_minmax, |c, slf, a, blk| {
    if !blk.is_null() {
        let (b, e, _) = c.range_parts(slf);
        if b == NIL { return err(c, c.vm().c_range_error.get(), "cannot get the minimum of beginless range"); }
        if e == NIL { return err(c, c.vm().c_range_error.get(), "cannot get the maximum of endless range"); }
        return crate::builtins::enum_native::call_enumerable(c, slf, "minmax", a, blk);
    }
    let lo = r_min(c, slf, &[], Blk::NONE)?;
    let hi = r_max(c, slf, &[], Blk::NONE)?;
    Ok(c.ary_new_from(&[lo, hi]))
});
cfn!(r_sum, |c, slf, a, blk| {
    let (b, e, x) = c.range_parts(slf);
    if blk.is_null() && b.is_fixnum() && e.is_fixnum() {
        let lo = b.as_fix() as i128; let hi = (if x { e.as_fix() - 1 } else { e.as_fix() }) as i128;
        let init = if a.is_empty() { Value::fix(0) } else { a[0] };
        if hi < lo { return Ok(init); }
        let s2 = (lo + hi) * (hi - lo + 1) / 2;
        let n = c.int_from_i128(s2);
        // int_range_sum: rb_int_plus, no call of a redefined Integer#+
        if is_integer(init) { return crate::builtins::numeric_methods::int_plus(c, init, &[n], Blk::NONE); }
        return funcall(c, init, s::plus, &[n], std::ptr::null());
    }
    if blk.is_null() && is_integer(b) && is_integer(e) {
        // int_range_sum with a bignum end: (hi - lo + 1) * (hi + lo) / 2
        use crate::bignum::BigInt;
        let (lo, mut hi) = (c.to_big(b), c.to_big(e));
        if x { hi = hi.sub(&BigInt::from_i64(1)); }
        let init = if a.is_empty() { Value::fix(0) } else { a[0] };
        if hi.cmp(&lo) == std::cmp::Ordering::Less { return Ok(init); }
        let s2 = hi.sub(&lo).add(&BigInt::from_i64(1)).mul(&hi.add(&lo)).divmod(&BigInt::from_i64(2)).0;
        let n = c.big_to_value(s2);
        if is_integer(init) { return crate::builtins::numeric_methods::int_plus(c, init, &[n], Blk::NONE); }
        return funcall(c, init, s::plus, &[n], std::ptr::null());
    }
    let arr = r_to_a(c, slf, &[], Blk::NONE)?;
    crate::builtins::array_methods::a_sum(c, arr, a, blk)
});
// range_step of range.c (3.4+): numeric ranges step either way, others need a step that moves toward the end.
cfn!(r_step, |c, slf, a, blk| {
    let (b, e, x) = c.range_parts(slf);
    let numk = c.vm().c_numeric.get();
    let isnum = |c: &Ctx, v: Value| v.is_fixnum() || v.is_flonum() || (v.is_heap() && is_kind_of(c, v, numk));
    let (b_num, e_num) = (isnum(c, b), isnum(c, e));
    let str_b = if is_string(b) { Some(b) } else { None };
    // rb_check_string_type(b): a numeric begin that answers to_str is asked too
    if b_num && crate::rt::conv_target(b) && respond_to(c, b, s::to_str, true) { funcall(c, b, s::to_str, &[], std::ptr::null())?; }
    check_argc(c, a, 0, 1)?;
    let st = match a.first() {
        Some(&v) => v,
        None if b_num || str_b.is_some() || b.is_sym() || (b == NIL && e_num) => Value::fix(1),
        None => return arg_err(c, "step is required for non-numeric ranges"),
    };
    let st_num = isnum(c, st);
    if st_num && b_num && funcall(c, st, s::eq, &[Value::fix(0)], std::ptr::null())?.truthy() { return arg_err(c, "step can't be 0"); }
    if blk.is_null() {
        if st_num && ((b_num && (e == NIL || e_num)) || (b == NIL && e_num)) {
            return crate::builtins::enumerator::arith_seq_from(c, slf, b, e, st, x, mid_of(a), a);
        }
        if b == NIL { return arg_err(c, "#step for non-numeric beginless ranges is meaningless"); }
        return crate::builtins::enumerator::enum_for(c, slf, mid_of(a), a, NIL);
    }
    if b == NIL { return arg_err(c, "#step iteration for beginless ranges is meaningless"); }
    if b.is_fixnum() && e == NIL && st.is_fixnum() {
        let mut i = b;
        loop { yield1(c, blk, i)?; i = funcall(c, i, s::plus, &[st], std::ptr::null())?; }
    }
    if b.is_fixnum() && e.is_fixnum() && st.is_fixnum() {
        let (mut i, unit) = (b.as_fix() as i128, st.as_fix() as i128);
        let end = e.as_fix() as i128;
        if unit < 0 {
            let end = if x { end } else { end - 1 };
            while i > end { yield1(c, blk, Value::fix(i as i64))?; i += unit; }
        } else {
            let end = if x { end } else { end + 1 };
            while i < end { yield1(c, blk, Value::fix(i as i64))?; i += unit; }
        }
        return Ok(slf);
    }
    if b_num && st_num && (is_float(b) || is_float(e) || is_float(st)) {
        let (Some(beg), Some(unit)) = (c.num_to_f64(b), c.num_to_f64(st)) else { return type_err(c, "no implicit conversion to float"); };
        let end = if e == NIL { if unit < 0.0 { f64::NEG_INFINITY } else { f64::INFINITY } } else { match c.num_to_f64(e) { Some(v) => v, None => return type_err(c, "no implicit conversion to float") } };
        let n = crate::builtins::numeric_methods::float_step_size(beg, end, unit, x);
        if unit.is_infinite() { if n != 0.0 { let v = c.float_new(beg); yield1(c, blk, v)?; } }
        else {
            let mut i = 0.0;
            while i < n {
                let mut d = i * unit + beg;
                if if unit >= 0.0 { end < d } else { d < end } { d = end; }
                let v = c.float_new(d); yield1(c, blk, v)?;
                i += 1.0;
            }
        }
        return Ok(slf);
    }
    if (str_b.is_some() || b.is_sym()) && st.is_fixnum() {
        // step_i: every k-th; a k below 1 yields the first only
        let k = st.as_fix();
        let mut idx = 0;
        range_each(c, slf, |c, v| { if idx == 0 || (k > 0 && idx % k == 0) { yield1(c, blk, v)?; } idx += 1; Ok(true) })?;
        return Ok(slf);
    }
    let mut v = b;
    if e == NIL { loop { yield1(c, blk, v)?; v = funcall(c, v, s::plus, &[st], std::ptr::null())?; } }
    if b_num && st_num && r_less(c, st, Value::fix(0))? < 0 {
        // backwards, as ArithmeticSequence
        loop {
            let r = r_less(c, e, v)?;
            if x { if r >= 0 { break; } } else if r > 0 { break; }
            yield1(c, blk, v)?;
            if r == 0 { break; }
            v = funcall(c, v, s::plus, &[st], std::ptr::null())?;
        }
        return Ok(slf);
    }
    let dir = r_less(c, b, e)?;
    if dir == 0 { if !x { yield1(c, blk, v)?; } return Ok(slf); }
    let next = funcall(c, b, s::plus, &[st], std::ptr::null())?;
    if dir == r_less(c, b, next)? {
        loop {
            let r = r_less(c, v, e)?;
            if x { if r != dir { break; } } else if r != dir && r != 0 { break; }
            yield1(c, blk, v)?;
            if r == 0 { break; }
            v = funcall(c, v, s::plus, &[st], std::ptr::null())?;
        }
    }
    Ok(slf)
});

thread_local!(static PCT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
fn mid_of(_a: &[Value]) -> crate::value::Sym { if PCT.with(|p| p.get()) { intern(b"%") } else { s::step } }
cfn!(r_pct, |c, slf, a, blk| { PCT.with(|p| p.set(true)); let r = r_step(c, slf, a, blk); PCT.with(|p| p.set(false)); r });
// Range#bsearch after CRuby range.c (same probe order: BSEARCH / bsearch_integer_range / doubling for endless).
enum Probe { Found(Value), Smaller(bool) }
fn bs_check(c: &mut Ctx, blk: Blk, val: Value, satisfied: &mut Value) -> Result<Probe, Esc> {
    let v = yield1(c, blk, val)?;
    if v.is_fixnum() {
        if v.as_fix() == 0 { return Ok(Probe::Found(val)); }
        return Ok(Probe::Smaller(v.as_fix() < 0));
    }
    if v == TRUE { *satisfied = val; return Ok(Probe::Smaller(true)); }
    if v.falsy() { return Ok(Probe::Smaller(false)); }
    if crate::numeric::is_numeric(v) {
        let z = funcall(c, v, s::cmp, &[Value::fix(0)], std::ptr::null())?;
        let n = crate::builtins::array_methods::cmp_result(c, z, v, Value::fix(0))?;
        if n == 0 { return Ok(Probe::Found(val)); }
        return Ok(Probe::Smaller(n < 0));
    }
    type_err(c, format!("wrong argument type {} (must be numeric, true, false or nil)", class_name_for_err(c, v)))
}
fn bs_i64(c: &mut Ctx, blk: Blk, mut low: i64, mut high: i64, excl: bool, conv: &dyn Fn(&mut Ctx, i64) -> Value) -> RV {
    let mut satisfied = NIL;
    if !excl { high += 1; }
    low -= 1;
    while low + 1 < high {
        let mid = if (high < 0) == (low < 0) { low + (high - low) / 2 } else { (low + high) / 2 };
        let probe = conv(c, mid);
        match bs_check(c, blk, probe, &mut satisfied)? {
            Probe::Found(v) => return Ok(v),
            Probe::Smaller(true) => high = mid,
            Probe::Smaller(false) => low = mid,
        }
    }
    Ok(satisfied)
}
fn bs_int_range(c: &mut Ctx, blk: Blk, beg: Value, end: Value, excl: bool) -> RV {
    if beg.is_fixnum() && end.is_fixnum() && end.as_fix() < i64::MAX / 4 && beg.as_fix() > i64::MIN / 4 {
        return bs_i64(c, blk, beg.as_fix(), end.as_fix(), excl, &|_c, i| Value::fix(i));
    }
    let one = Value::fix(1);
    let mut satisfied = NIL;
    let mut low = funcall(c, beg, s::minus, &[one], std::ptr::null())?;
    let mut high = if excl { end } else { funcall(c, end, s::plus, &[one], std::ptr::null())? };
    loop {
        let l1 = funcall(c, low, s::plus, &[one], std::ptr::null())?;
        if !funcall(c, l1, s::lt, &[high], std::ptr::null())?.truthy() { break; }
        let sum = funcall(c, low, s::plus, &[high], std::ptr::null())?;
        let mid = funcall(c, sum, intern(b"div"), &[Value::fix(2)], std::ptr::null())?;
        match bs_check(c, blk, mid, &mut satisfied)? {
            Probe::Found(v) => return Ok(v),
            Probe::Smaller(true) => high = mid,
            Probe::Smaller(false) => low = mid,
        }
    }
    Ok(satisfied)
}
fn dbl_as_i64(d: f64) -> i64 { let i = d.abs().to_bits() as i64; if d < 0.0 { -i } else { i } }
fn i64_as_dbl(c: &mut Ctx, i: i64) -> Value { if i < 0 { c.float_new(-f64::from_bits((-i) as u64)) } else { c.float_new(f64::from_bits(i as u64)) } }
cfn!(r_bsearch, |c, slf, a, blk| {
    let (b, e, x) = c.range_parts(slf);
    if blk.is_null() {
        // RETURN_ENUMERATOR only for the searchable kinds
        let ok = is_float(b) || is_float(e) || (is_integer(b) && (is_integer(e) || e == NIL)) || (b == NIL && is_integer(e));
        if !ok { return type_err(c, format!("can't do binary search for {}", class_path(c, real_class_of(c, b)))); }
        return crate::builtins::enumerator::enum_for(c, slf, intern(b"bsearch"), a, NIL);
    }
    if b.is_fixnum() && e.is_fixnum() { return bs_int_range(c, blk, b, e, x); }
    if is_float(b) || is_float(e) {
        let tof = |c: &mut Ctx, v: Value, dflt: f64| -> Result<f64, Esc> {
            if v == NIL { return Ok(dflt); }
            if let Some(f) = c.num_to_f64(v) { return Ok(f); }
            let r = funcall(c, v, intern(b"to_f"), &[], std::ptr::null())?;
            Ok(c.num_to_f64(r).unwrap_or(0.0))
        };
        let lo = tof(c, b, f64::NEG_INFINITY)?; let hi = tof(c, e, f64::INFINITY)?;
        return bs_i64(c, blk, dbl_as_i64(lo), dbl_as_i64(hi), x, &i64_as_dbl);
    }
    if is_integer(b) && is_integer(e) { return bs_int_range(c, blk, b, e, x); }
    if is_integer(b) && e == NIL {
        let mut diff = Value::fix(1);
        let mut satisfied = NIL;
        loop {
            let mid = funcall(c, b, s::plus, &[diff], std::ptr::null())?;
            match bs_check(c, blk, mid, &mut satisfied)? {
                Probe::Found(v) => return Ok(v),
                Probe::Smaller(true) => return bs_int_range(c, blk, b, mid, false),
                Probe::Smaller(false) => {}
            }
            diff = funcall(c, diff, s::mul, &[Value::fix(2)], std::ptr::null())?;
        }
    }
    if b == NIL && is_integer(e) {
        let mut diff = Value::fix(-1);
        let mut satisfied = NIL;
        loop {
            let mid = funcall(c, e, s::plus, &[diff], std::ptr::null())?;
            match bs_check(c, blk, mid, &mut satisfied)? {
                Probe::Found(v) => return Ok(v),
                Probe::Smaller(false) => return bs_int_range(c, blk, mid, e, x),
                Probe::Smaller(true) => {}
            }
            diff = funcall(c, diff, s::mul, &[Value::fix(2)], std::ptr::null())?;
        }
    }
    type_err(c, format!("can't do binary search for {}", class_name_for_err(c, b)))
});
cfn!(r_overlap, |c, slf, a, blk| {
    if !is_range(a[0]) { return type_err(c, format!("wrong argument type {} (expected Range)", class_name_for_err(c, a[0]))); }
    let (b1, e1, x1) = c.range_parts(slf); let (b2, e2, x2) = c.range_parts(a[0]);
    let lt = |c: &mut Ctx, p: Value, q: Value, excl: bool| -> Result<bool, Esc> { if p == NIL || q == NIL { return Ok(true); } let r = funcall(c, p, s::cmp, &[q], std::ptr::null())?; Ok(r.is_fixnum() && (r.as_fix() < 0 || (!excl && r.as_fix() == 0))) };
    let empty = |c: &mut Ctx, b: Value, e: Value, x: bool| -> Result<bool, Esc> { if b == NIL || e == NIL { return Ok(false); } let r = funcall(c, b, s::cmp, &[e], std::ptr::null())?; Ok(!r.is_fixnum() || r.as_fix() > 0 || (x && r.as_fix() == 0)) };
    if empty(c, b1, e1, x1)? || empty(c, b2, e2, x2)? { return Ok(FALSE); }
    Ok(Value::bool(lt(c, b2, e1, x1)? && lt(c, b1, e2, x2)?))
});

// range.c: an endless range has no set; Enumerable#to_set(klass, *args) of 4.0 warns
cfn!(r_to_set, |c, slf, a, blk| {
    if c.range_parts(slf).1 == NIL { return err(c, c.vm().c_range_error.get(), "cannot convert endless range to a set"); }
    crate::builtins::enum_native::call_enumerable(c, slf, "to_set", a, blk)
});
/// Defines the methods of Range `k`; the entry of Range#each, for `vm.ids`.
pub fn init(c: &mut Ctx, k: Value) -> *const MethodEntry {
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, k, n, f, a);
    define_private_cfunc(c, k, "initialize", r_initialize, -1);
    d(c, "begin", r_begin, 0); d(c, "first", r_first, -1); d(c, "end", r_end, 0); d(c, "last", r_last, -1); d(c, "exclude_end?", r_excl, 0);
    d(c, "each", r_each, 0);
    let each = search_method(c, k, s::each);
    d(c, "reverse_each", r_reverse_each, 0); d(c, "to_a", r_to_a, 0); d(c, "entries", r_to_a, 0); d(c, "size", r_size, 0); d(c, "count", r_count, -1);
    d(c, "include?", r_include, 1); d(c, "member?", r_include, 1); d(c, "cover?", r_cover, 1); d(c, "===", r_eqq, 1); d(c, "==", r_eq, 1); d(c, "eql?", r_eql, 1);
    d(c, "hash", r_hash, 0); d(c, "to_s", r_to_s, 0); d(c, "inspect", r_inspect, 0); d(c, "min", r_min, -1); d(c, "minmax", r_minmax, 0); d(c, "max", r_max, -1); 
    d(c, "step", r_step, -1); d(c, "%", r_pct, 1); d(c, "bsearch", r_bsearch, 0); d(c, "overlap?", r_overlap, 1); d(c, "to_set", r_to_set, -1);
    each
}
