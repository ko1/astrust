// Range.
use crate::array::*;
use crate::builtins::kernel::*;
use crate::range::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

macro_rules! cfn { ($name:ident, |$c:ident, $s:ident, $a:ident, $b:ident| $body:expr) => {
    #[allow(unused_variables)]
    pub fn $name($c: &mut Ctx, $s: Value, $a: &[Value], $b: *const Block) -> RV { $body }
}; }

cfn!(r_initialize, |c, slf, a, blk| {
    check_argc(c, a, 2, 3)?;
    if is_frozen(slf) && !matches!(obj(slf).body, Body::Range(b, e, _) if b == NIL && e == NIL) { return Err(raise_frozen(c, slf)); }
    let r = range_new_checked(c, a[0], a[1], a.len() > 2 && a[2].truthy())?;
    let (b, e, x) = range_parts(r);
    obj(slf).body = Body::Range(b, e, x);
    obj(slf).flags |= FL_FROZEN;
    Ok(NIL)
});
cfn!(r_begin, |c, slf, a, blk| Ok(range_parts(slf).0));
cfn!(r_end, |c, slf, a, blk| Ok(range_parts(slf).1));
cfn!(r_excl, |c, slf, a, blk| Ok(Value::bool(range_parts(slf).2)));
fn int_end(c: &mut Ctx, e: Value) -> Result<Option<i64>, Esc> {
    if e.is_fixnum() { return Ok(Some(e.as_fix())); }
    if is_float(e) { let f = float_val(e); if f.is_infinite() && f > 0.0 { return Ok(None); } return Ok(Some(f.floor() as i64)); }
    if e == NIL { return Ok(None); }
    Ok(Some(crate::builtins::numeric_methods::num_to_long(c, e)?))
}
pub fn range_each(c: &mut Ctx, slf: Value, mut f: impl FnMut(&mut Ctx, Value) -> Result<bool, Esc>) -> Result<(), Esc> {
    let (b, e, excl) = range_parts(slf);
    if b.is_fixnum() && (e.is_fixnum() || e == NIL || is_float(e)) {
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
        let (bs, es) = (str_new(&bb), str_new(&eb));
        let blk_data = ary_new();
        let collect = Block::cfunc(c, |_c, d, args, _b| { ary_push(d, args[0]); Ok(NIL) }, blk_data);
        let ev = if excl { TRUE } else { FALSE };
        funcall(c, bs, s::upto, &[es, ev], &collect)?;
        for i in 0..ary_len(blk_data) { let v = Value::sym(intern(str_bytes(ary_get(blk_data, i)))); if !f(c, v)? { break; } }
        return Ok(());
    }
    if is_string(b) && (is_string(e) || e == NIL) {
        if e == NIL {
            let mut cur = str_bytes(b).to_vec();
            loop { let s2 = str_new(&cur); if !f(c, s2)? { return Ok(()); } cur = succ_bytes(&cur); }
        }
        let o = str_bytes(b).cmp(str_bytes(e));
        if o == std::cmp::Ordering::Greater || (excl && o == std::cmp::Ordering::Equal) { return Ok(()); }
        let (bb, eb) = (str_bytes(b).to_vec(), str_bytes(e).to_vec());
        if bb.len() == 1 && eb.len() == 1 && bb[0] < 0x80 && eb[0] < 0x80 {
            let (lo, hi) = (bb[0] as i32, if excl { eb[0] as i32 - 1 } else { eb[0] as i32 });
            let mut ch = lo;
            while ch <= hi { if !f(c, str_new(&[ch as u8]))? { break; } ch += 1; }
            return Ok(());
        }
        let blk_data = ary_new();
        let collect = Block::cfunc(c, |_c, d, args, _b| { ary_push(d, args[0]); Ok(NIL) }, blk_data);
        let ev = if excl { TRUE } else { FALSE };
        funcall(c, b, s::upto, &[e, ev], &collect)?;
        for i in 0..ary_len(blk_data) { if !f(c, ary_get(blk_data, i))? { break; } }
        return Ok(());
    }
    if b == NIL || is_float(b) { return type_err(c, format!("can't iterate from {}", class_name_for_err(b))); }
    if !respond_to(b, s::succ, true) { return type_err(c, format!("can't iterate from {}", class_name_for_err(b))); }
    let mut cur = b;
    loop {
        if e != NIL {
            let cmp = funcall(c, cur, s::cmp, &[e], std::ptr::null())?;
            if cmp == NIL { break; }
            let v = cmp.as_fix();
            if v > 0 || (excl && v == 0) { break; }
        }
        if !f(c, cur)? { break; }
        cur = funcall(c, cur, s::succ, &[], std::ptr::null())?;
    }
    Ok(())
}
cfn!(r_each, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, s::each, a); }
    let (b, e, excl) = range_parts(slf);
    if b.is_fixnum() && e.is_fixnum() {
        let mut i = b.as_fix(); let l = e.as_fix();
        let l = if excl { l - 1 } else { l };
        while i <= l { yield1(c, blk, Value::fix(i))?; i += 1; }
        return Ok(slf);
    }
    range_each(c, slf, |c, v| { yield1(c, blk, v)?; Ok(true) })?;
    Ok(slf)
});
cfn!(r_reverse_each, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, intern(b"reverse_each"), a); }
    let (b, e, excl) = range_parts(slf);
    if e.is_fixnum() && (b.is_fixnum() || b == NIL) {
        let mut i = if excl { e.as_fix() - 1 } else { e.as_fix() };
        loop { if b != NIL && i < b.as_fix() { break; } yield1(c, blk, Value::fix(i))?; i -= 1; }
        return Ok(slf);
    }
    if e == NIL { return type_err(c, format!("can't iterate from {}", class_name_for_err(e))); }
    let arr = r_to_a(c, slf, &[], std::ptr::null())?;
    let n = ary_len(arr);
    for i in (0..n).rev() { yield1(c, blk, ary_get(arr, i))?; }
    Ok(slf)
});
cfn!(r_to_a, |c, slf, a, blk| {
    let (b, e, _) = range_parts(slf);
    if e == NIL { return err(c, vm().c_range_error, "cannot convert endless range to an array"); }
    let out = ary_new();
    range_each(c, slf, |_c, v| { ary_push(out, v); Ok(true) })?;
    let _ = b;
    Ok(out)
});
cfn!(r_size, |c, slf, a, blk| {
    let (b, e, excl) = range_parts(slf);
    if !crate::numeric::is_numeric(b) { if b == NIL && (crate::numeric::is_numeric(e)) { return Ok(crate::numeric::float_new(f64::INFINITY)); } return Ok(NIL); }
    if e == NIL { return Ok(crate::numeric::float_new(f64::INFINITY)); }
    if !crate::numeric::is_numeric(e) { return Ok(NIL); }
    if is_float(b) { return type_err(c, "can't iterate from Float"); }
    if b.is_fixnum() && e.is_fixnum() { let n = e.as_fix() - b.as_fix() + if excl { 0 } else { 1 }; return Ok(Value::fix(n.max(0))); }
    let ef = crate::numeric::num_to_f64(e).unwrap();
    if ef.is_infinite() { return Ok(crate::numeric::float_new(f64::INFINITY)); }
    let bf = crate::numeric::num_to_f64(b).unwrap();
    let mut n = (ef - bf).floor() + 1.0;
    if excl && (ef - bf).fract() == 0.0 { n -= 1.0; }
    if is_integer(b) && is_integer(e) {
        let x = crate::bignum::to_big(e).sub(&crate::bignum::to_big(b)).add(&crate::bignum::BigInt::from_i64(if excl { 0 } else { 1 }));
        return Ok(crate::bignum::big_to_value(if x.neg { crate::bignum::BigInt::zero() } else { x }));
    }
    Ok(Value::fix(n.max(0.0) as i64))
});
cfn!(r_count, |c, slf, a, blk| {
    if a.is_empty() && blk.is_null() {
        let (b, e, _) = range_parts(slf);
        if e == NIL || (b == NIL && e != NIL) { return Ok(crate::numeric::float_new(f64::INFINITY)); }
        if is_integer(b) { return r_size(c, slf, a, blk); }
    }
    let arr = r_to_a(c, slf, &[], std::ptr::null())?;
    funcall(c, arr, intern(b"count"), a, blk)
});
fn cover(c: &mut Ctx, slf: Value, v: Value) -> Result<bool, Esc> {
    let (b, e, excl) = range_parts(slf);
    if b.is_fixnum() && e.is_fixnum() && v.is_fixnum() {
        let x = v.as_fix();
        return Ok(x >= b.as_fix() && if excl { x < e.as_fix() } else { x <= e.as_fix() });
    }
    if b != NIL {
        let r = funcall(c, b, s::cmp, &[v], std::ptr::null())?;
        if r == NIL || !r.is_fixnum() || r.as_fix() > 0 { return Ok(false); }
    }
    if e != NIL {
        let r = funcall(c, v, s::cmp, &[e], std::ptr::null())?;
        if r == NIL || !r.is_fixnum() { return Ok(false); }
        if r.as_fix() > 0 || (excl && r.as_fix() == 0) { return Ok(false); }
    }
    Ok(true)
}
cfn!(r_include, |c, slf, a, blk| {
    let (b, e, _) = range_parts(slf);
    if crate::numeric::is_numeric(b) || crate::numeric::is_numeric(e) || (b == NIL && e == NIL) { return Ok(Value::bool(cover(c, slf, a[0])?)); }
    if is_string(b) || is_string(e) {
        if !is_string(a[0]) { return Ok(FALSE); }
        if b == NIL || e == NIL { return Ok(Value::bool(cover(c, slf, a[0])?)); }
        let mut found = false;
        range_each(c, slf, |c, v| { if crate::builtins::array_methods::rb_eq(c, v, a[0])? { found = true; return Ok(false); } Ok(true) })?;
        return Ok(Value::bool(found));
    }
    if b == NIL || e == NIL { return type_err(c, "cannot determine inclusion in beginless/endless ranges"); }
    let mut found = false;
    range_each(c, slf, |c, v| { if crate::builtins::array_methods::rb_eq(c, v, a[0])? { found = true; return Ok(false); } Ok(true) })?;
    Ok(Value::bool(found))
});
cfn!(r_cover, |c, slf, a, blk| {
    if is_range(a[0]) {
        let (b2, e2, x2) = range_parts(a[0]);
        let (_, e, x) = range_parts(slf);
        if b2 != NIL && !cover(c, slf, b2)? { return Ok(FALSE); }
        if e2 == NIL { return Ok(Value::bool(e == NIL)); }
        if e == NIL { return Ok(TRUE); }
        let r = funcall(c, e2, s::cmp, &[e], std::ptr::null())?;
        if !r.is_fixnum() { return Ok(FALSE); }
        let v = r.as_fix();
        return Ok(Value::bool(v < 0 || (v == 0 && (!x || x2)) || (x2 && !x && false)));
    }
    Ok(Value::bool(cover(c, slf, a[0])?))
});
cfn!(r_eqq, |c, slf, a, blk| Ok(Value::bool(cover(c, slf, a[0]).unwrap_or_else(|_| { c.esc = EscKind::None; false }))));
cfn!(r_eq, |c, slf, a, blk| {
    if !is_range(a[0]) { return Ok(FALSE); }
    let (b1, e1, x1) = range_parts(slf); let (b2, e2, x2) = range_parts(a[0]);
    Ok(Value::bool(x1 == x2 && crate::builtins::array_methods::rb_eq(c, b1, b2)? && crate::builtins::array_methods::rb_eq(c, e1, e2)?))
});
cfn!(r_eql, |c, slf, a, blk| {
    if !is_range(a[0]) { return Ok(FALSE); }
    let (b1, e1, x1) = range_parts(slf); let (b2, e2, x2) = range_parts(a[0]);
    Ok(Value::bool(x1 == x2 && crate::hash::key_eql(c, b1, b2)? && crate::hash::key_eql(c, e1, e2)?))
});
cfn!(r_hash, |c, slf, a, blk| {
    let (b, e, x) = range_parts(slf);
    let h = crate::astrust::hash_merge(crate::hash::key_hash(c, b)?, crate::hash::key_hash(c, e)?) ^ (x as u64);
    Ok(Value::fix((h >> 2) as i64))
});
fn range_to_s(c: &mut Ctx, slf: Value, insp: bool) -> RV {
    let (b, e, x) = range_parts(slf);
    let f = |c: &mut Ctx, v: Value| -> Result<String, Esc> { if insp { if v == NIL { Ok(String::new()) } else { inspect_str(c, v) } } else { to_rstring(c, v) } };
    if insp && b == NIL && e == NIL { return Ok(str_new(if x { b"nil...nil" as &[u8] } else { b"nil..nil" })); }
    let bs = f(c, b)?; let es = f(c, e)?;
    Ok(str_new(format!("{}{}{}", bs, if x { "..." } else { ".." }, es).as_bytes()))
}
cfn!(r_to_s, |c, slf, a, blk| range_to_s(c, slf, false));
cfn!(r_inspect, |c, slf, a, blk| range_to_s(c, slf, true));
cfn!(r_first, |c, slf, a, blk| {
    let (b, _, _) = range_parts(slf);
    if a.is_empty() { if b == NIL { return err(c, vm().c_range_error, "cannot get the first element of beginless range"); } return Ok(b); }
    let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative array size (or size too big)"); }
    let out = ary_new();
    if n == 0 { return Ok(out); }
    range_each(c, slf, |_c, v| { ary_push(out, v); Ok((ary_len(out) as i64) < n) })?;
    Ok(out)
});
cfn!(r_last, |c, slf, a, blk| {
    let (b, e, x) = range_parts(slf);
    if e == NIL { return err(c, vm().c_range_error, "cannot get the last element of endless range"); }
    if a.is_empty() {
        return Ok(e);
    }
    let arr = r_to_a(c, slf, &[], std::ptr::null())?;
    let _ = (b, x);
    funcall(c, arr, intern(b"last"), a, std::ptr::null())
});
cfn!(r_min, |c, slf, a, blk| {
    let (b, e, x) = range_parts(slf);
    if b == NIL { return err(c, vm().c_range_error, "cannot get the minimum of beginless range"); }
    if !blk.is_null() || !a.is_empty() {
        check_argc(c, a, 0, 1)?;
        if let Some(&n) = a.first() { if n != NIL && crate::builtins::numeric_methods::num_to_long(c, n)? < 0 { return arg_err(c, "negative array size (or size too big)"); } }
        if blk.is_null() && is_integer(b) { return r_first(c, slf, a, blk); }
        if e == NIL { return err(c, vm().c_range_error, "cannot get the minimum of endless range with custom comparison method"); }
        return crate::builtins::enum_native::call_enumerable(c, slf, "min", a, blk);
    }
    if e == NIL { return Ok(b); }
    let r = funcall(c, b, s::cmp, &[e], std::ptr::null())?;
    if r == NIL || r.as_fix() > 0 || (x && r.as_fix() == 0) { return Ok(NIL); }
    Ok(b)
});
cfn!(r_max, |c, slf, a, blk| {
    let (b, e, x) = range_parts(slf);
    if e == NIL { return err(c, vm().c_range_error, "cannot get the maximum of endless range"); }
    if !blk.is_null() || !a.is_empty() {
        check_argc(c, a, 0, 1)?;
        if let Some(&n) = a.first() { if n != NIL && crate::builtins::numeric_methods::num_to_long(c, n)? < 0 { return arg_err(c, "negative array size (or size too big)"); } }
        if blk.is_null() && e.is_fixnum() && (b == NIL || b.is_fixnum()) {
            let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
            let out = ary_new();
            let mut i = if x { e.as_fix() - 1 } else { e.as_fix() };
            while (ary_len(out) as i64) < n && (b == NIL || i >= b.as_fix()) { ary_push(out, Value::fix(i)); i -= 1; }
            return Ok(out);
        }
        if b == NIL { return err(c, vm().c_range_error, "cannot get the maximum of beginless range with custom comparison method"); }
        return crate::builtins::enum_native::call_enumerable(c, slf, "max", a, blk);
    }
    if b != NIL {
        let r = funcall(c, b, s::cmp, &[e], std::ptr::null())?;
        if r == NIL || r.as_fix() > 0 || (x && r.as_fix() == 0) { return Ok(NIL); }
    }
    if x {
        if is_float(e) || (b != NIL && is_float(b)) { return type_err(c, "cannot exclude non Integer end value"); }
        if e.is_fixnum() { return Ok(Value::fix(e.as_fix() - 1)); }
        if !is_integer(e) { return type_err(c, "cannot exclude end value with non Integer begin value"); }
        return funcall(c, e, s::minus, &[Value::fix(1)], std::ptr::null());
    }
    Ok(e)
});
cfn!(r_minmax, |c, slf, a, blk| {
    if !blk.is_null() {
        let (b, e, _) = range_parts(slf);
        if b == NIL { return err(c, vm().c_range_error, "cannot get the minimum of beginless range"); }
        if e == NIL { return err(c, vm().c_range_error, "cannot get the maximum of endless range"); }
        return crate::builtins::enum_native::call_enumerable(c, slf, "minmax", a, blk);
    }
    let lo = r_min(c, slf, &[], std::ptr::null())?;
    let hi = r_max(c, slf, &[], std::ptr::null())?;
    Ok(ary_new_from(&[lo, hi]))
});
cfn!(r_sum, |c, slf, a, blk| {
    let (b, e, x) = range_parts(slf);
    if blk.is_null() && b.is_fixnum() && e.is_fixnum() {
        let lo = b.as_fix() as i128; let hi = (if x { e.as_fix() - 1 } else { e.as_fix() }) as i128;
        let init = if a.is_empty() { Value::fix(0) } else { a[0] };
        if hi < lo { return Ok(init); }
        let s2 = (lo + hi) * (hi - lo + 1) / 2;
        return funcall(c, init, s::plus, &[crate::bignum::int_from_i128(s2)], std::ptr::null());
    }
    let arr = r_to_a(c, slf, &[], std::ptr::null())?;
    crate::builtins::array_methods::a_sum(c, arr, a, blk)
});
cfn!(r_step, |c, slf, a, blk| {
    let (b, e, x) = range_parts(slf);
    check_argc(c, a, 0, 1)?;
    let st = if a.is_empty() { Value::fix(1) } else { a[0] };
    if crate::numeric::is_numeric(st) && (b == NIL || crate::numeric::is_numeric(b)) {
        let z = funcall(c, st, s::eq, &[Value::fix(0)], std::ptr::null())?;
        if z.truthy() { return arg_err(c, "step can't be 0"); }
    }
    if blk.is_null() {
        let num = |v: Value| v == NIL || crate::numeric::is_numeric(v);
        if num(b) && num(e) && crate::numeric::is_numeric(st) && !(b == NIL && e == NIL) {
            return crate::builtins::enumerator::arith_seq_from(c, slf, b, e, st, x, mid_of(a), a);
        }
        return crate::builtins::enumerator::enum_for(c, slf, mid_of(a), a, NIL);
    }
    if crate::numeric::is_numeric(st) {
        let z = funcall(c, st, s::eq, &[Value::fix(0)], std::ptr::null())?;
        if z.truthy() { return arg_err(c, "step can't be 0"); }
        let neg = funcall(c, st, s::lt, &[Value::fix(0)], std::ptr::null())?;
        if neg.truthy() { return arg_err(c, "step can't be negative"); }
    }
    if is_float(b) || is_float(e) || is_float(st) {
        let bf = crate::numeric::num_to_f64(b).unwrap(); let sf = crate::numeric::num_to_f64(st).unwrap();
        let ef = if e == NIL { f64::INFINITY } else { crate::numeric::num_to_f64(e).unwrap() };
        let n = ((ef - bf) / sf + 1e-9).floor();
        let mut i = 0.0;
        loop {
            if !n.is_infinite() && i > n { break; }
            let v = bf + i * sf;
            if x && v >= ef { break; }
            yield1(c, blk, crate::numeric::float_new(v))?;
            i += 1.0;
        }
        return Ok(slf);
    }
    if b.is_fixnum() && (e.is_fixnum() || e == NIL) && st.is_fixnum() {
        let mut i = b.as_fix(); let s2 = st.as_fix();
        loop { if e != NIL && (if x { i >= e.as_fix() } else { i > e.as_fix() }) { break; } yield1(c, blk, Value::fix(i))?; i += s2; }
        return Ok(slf);
    }
    if is_string(b) && st.is_fixnum() {
        let k = st.as_fix();
        let mut idx = 0;
        range_each(c, slf, |c, v| { if idx % k == 0 { yield1(c, blk, v)?; } idx += 1; Ok(true) })?;
        return Ok(slf);
    }
    let mut cur = b;
    loop {
        if e != NIL { let r = funcall(c, cur, s::cmp, &[e], std::ptr::null())?; if r == NIL || r.as_fix() > 0 || (x && r.as_fix() == 0) { break; } }
        yield1(c, blk, cur)?;
        cur = funcall(c, cur, s::plus, &[st], std::ptr::null())?;
    }
    Ok(slf)
});
thread_local!(static PCT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
fn mid_of(_a: &[Value]) -> crate::value::Sym { if PCT.with(|p| p.get()) { intern(b"%") } else { s::step } }
cfn!(r_pct, |c, slf, a, blk| { PCT.with(|p| p.set(true)); let r = r_step(c, slf, a, blk); PCT.with(|p| p.set(false)); r });
cfn!(r_dup_frozen, |c, slf, a, blk| Ok(slf));
// Range#bsearch after CRuby range.c (same probe order: BSEARCH / bsearch_integer_range / doubling for endless).
enum Probe { Found(Value), Smaller(bool) }
fn bs_check(c: &mut Ctx, blk: *const Block, val: Value, satisfied: &mut Value) -> Result<Probe, Esc> {
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
    type_err(c, format!("wrong argument type {} (must be numeric, true, false or nil)", class_name_for_err(v)))
}
fn bs_i64(c: &mut Ctx, blk: *const Block, mut low: i64, mut high: i64, excl: bool, conv: &dyn Fn(i64) -> Value) -> RV {
    let mut satisfied = NIL;
    if !excl { high += 1; }
    low -= 1;
    while low + 1 < high {
        let mid = if (high < 0) == (low < 0) { low + (high - low) / 2 } else { (low + high) / 2 };
        match bs_check(c, blk, conv(mid), &mut satisfied)? {
            Probe::Found(v) => return Ok(v),
            Probe::Smaller(true) => high = mid,
            Probe::Smaller(false) => low = mid,
        }
    }
    Ok(satisfied)
}
fn bs_int_range(c: &mut Ctx, blk: *const Block, beg: Value, end: Value, excl: bool) -> RV {
    if beg.is_fixnum() && end.is_fixnum() && end.as_fix() < i64::MAX / 4 && beg.as_fix() > i64::MIN / 4 {
        return bs_i64(c, blk, beg.as_fix(), end.as_fix(), excl, &|i| Value::fix(i));
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
fn i64_as_dbl(i: i64) -> Value { if i < 0 { crate::numeric::float_new(-f64::from_bits((-i) as u64)) } else { crate::numeric::float_new(f64::from_bits(i as u64)) } }
cfn!(r_bsearch, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"bsearch"), a, NIL); }
    let (b, e, x) = range_parts(slf);
    if b.is_fixnum() && e.is_fixnum() { return bs_int_range(c, blk, b, e, x); }
    if is_float(b) || is_float(e) {
        let tof = |c: &mut Ctx, v: Value, dflt: f64| -> Result<f64, Esc> {
            if v == NIL { return Ok(dflt); }
            if let Some(f) = crate::numeric::num_to_f64(v) { return Ok(f); }
            let r = funcall(c, v, intern(b"to_f"), &[], std::ptr::null())?;
            Ok(crate::numeric::num_to_f64(r).unwrap_or(0.0))
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
    type_err(c, format!("can't do binary search for {}", class_name_for_err(b)))
});
cfn!(r_overlap, |c, slf, a, blk| {
    if !is_range(a[0]) { return type_err(c, format!("wrong argument type {} (expected Range)", class_name_for_err(a[0]))); }
    let (b1, e1, x1) = range_parts(slf); let (b2, e2, x2) = range_parts(a[0]);
    let lt = |c: &mut Ctx, p: Value, q: Value, excl: bool| -> Result<bool, Esc> { if p == NIL || q == NIL { return Ok(true); } let r = funcall(c, p, s::cmp, &[q], std::ptr::null())?; Ok(r.is_fixnum() && (r.as_fix() < 0 || (!excl && r.as_fix() == 0))) };
    let empty = |c: &mut Ctx, b: Value, e: Value, x: bool| -> Result<bool, Esc> { if b == NIL || e == NIL { return Ok(false); } let r = funcall(c, b, s::cmp, &[e], std::ptr::null())?; Ok(!r.is_fixnum() || r.as_fix() > 0 || (x && r.as_fix() == 0)) };
    if empty(c, b1, e1, x1)? || empty(c, b2, e2, x2)? { return Ok(FALSE); }
    Ok(Value::bool(lt(c, b2, e1, x1)? && lt(c, b1, e2, x2)?))
});

pub fn init() {
    let k = vm().c_range;
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(k, n, f, a);
    define_private_cfunc(k, "initialize", r_initialize, -1);
    d("begin", r_begin, 0); d("first", r_first, -1); d("end", r_end, 0); d("last", r_last, -1); d("exclude_end?", r_excl, 0);
    d("each", r_each, 0); d("reverse_each", r_reverse_each, 0); d("to_a", r_to_a, 0); d("entries", r_to_a, 0); d("size", r_size, 0); d("count", r_count, -1);
    d("include?", r_include, 1); d("member?", r_include, 1); d("cover?", r_cover, 1); d("===", r_eqq, 1); d("==", r_eq, 1); d("eql?", r_eql, 1);
    d("hash", r_hash, 0); d("to_s", r_to_s, 0); d("inspect", r_inspect, 0); d("min", r_min, -1); d("minmax", r_minmax, 0); d("max", r_max, -1); d("sum", r_sum, -1);
    d("step", r_step, -1); d("%", r_pct, 1); d("dup", r_dup_frozen, 0); d("bsearch", r_bsearch, 0); d("overlap?", r_overlap, 1);
}
