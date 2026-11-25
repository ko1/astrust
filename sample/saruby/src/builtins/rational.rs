// Rational: num/den Integers, den > 0, gcd(num, den) == 1.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::bignum::*;
use crate::builtins::complex::{cpx_parts, cpx_raw, is_complex};
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::{coerce_bin, round_half_mode};
use crate::heap::obj_set_flags;
use crate::numeric::*;
use crate::symbol::{intern, s};
use crate::object::*;
use crate::value::*;
use crate::vm::*;
use std::cmp::Ordering;

pub struct RRational { pub num: Value, pub den: Value }
impl RData for RRational {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.num); m.visit(&mut self.den); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

// ---- Integer helpers on Values (fixnum fast paths) ----------------------------------------
// A helper that can make a bignum takes `&mut Ctx`; nested uses name their intermediates.

pub fn iadd(c: &mut Ctx, a: Value, b: Value) -> Value {
    if a.is_fixnum() && b.is_fixnum() { return c.int_from_i64(a.as_fix() + b.as_fix()); }
    let r = c.to_big(a).add(&c.to_big(b)); c.big_to_value(r)
}
pub fn isub(c: &mut Ctx, a: Value, b: Value) -> Value {
    if a.is_fixnum() && b.is_fixnum() { return c.int_from_i64(a.as_fix() - b.as_fix()); }
    let r = c.to_big(a).sub(&c.to_big(b)); c.big_to_value(r)
}
pub fn imul(c: &mut Ctx, a: Value, b: Value) -> Value {
    if a.is_fixnum() && b.is_fixnum() { return c.int_from_i128(a.as_fix() as i128 * b.as_fix() as i128); }
    let r = c.to_big(a).mul(&c.to_big(b)); c.big_to_value(r)
}
/// floored division; b != 0
pub fn idiv(c: &mut Ctx, a: Value, b: Value) -> Value {
    if a.is_fixnum() && b.is_fixnum() { let (x, y) = (a.as_fix(), b.as_fix()); let q = x / y; return c.int_from_i64(if x % y != 0 && ((x < 0) != (y < 0)) { q - 1 } else { q }); }
    let r = c.to_big(a).divmod(&c.to_big(b)).0; c.big_to_value(r)
}
pub fn imod(c: &mut Ctx, a: Value, b: Value) -> Value { let q = idiv(c, a, b); let m = imul(c, q, b); isub(c, a, m) }
pub fn icmp(c: &Ctx, a: Value, b: Value) -> Ordering {
    if a.is_fixnum() && b.is_fixnum() { return a.as_fix().cmp(&b.as_fix()); }
    c.to_big(a).cmp(&c.to_big(b))
}
pub fn ineg(c: &mut Ctx, a: Value) -> Value { if a.is_fixnum() { c.int_from_i64(-a.as_fix()) } else { let r = c.big(a).neg(); c.big_to_value(r) } }
pub fn isneg(c: &Ctx, a: Value) -> bool { if a.is_fixnum() { a.as_fix() < 0 } else { c.big(a).neg } }
pub fn iszero(a: Value) -> bool { a == Value::fix(0) }
pub fn iabs(c: &mut Ctx, a: Value) -> Value { if isneg(c, a) { ineg(c, a) } else { a } }
pub fn igcd(c: &mut Ctx, a: Value, b: Value) -> Value {
    if a.is_fixnum() && b.is_fixnum() {
        let (mut x, mut y) = (a.as_fix().unsigned_abs(), b.as_fix().unsigned_abs());
        while y != 0 { let r = x % y; x = y; y = r; }
        return c.int_from_i128(x as i128);
    }
    let r = c.to_big(a).gcd(&c.to_big(b)); c.big_to_value(r)
}
pub fn ipow(c: &mut Ctx, a: Value, e: u64) -> Value {
    if a.is_fixnum() { if let Ok(e32) = u32::try_from(e) { if let Some(r) = a.as_fix().checked_pow(e32) { return c.int_from_i64(r); } } }
    let r = c.to_big(a).pow(e); c.big_to_value(r)
}
pub fn iodd(c: &Ctx, a: Value) -> bool { if a.is_fixnum() { a.as_fix() & 1 == 1 } else { c.big(a).is_odd() } }
pub fn istr(c: &Ctx, a: Value) -> String { if a.is_fixnum() { a.as_fix().to_string() } else { c.big(a).to_string_radix(10) } }
pub fn int_to_f(c: &Ctx, a: Value) -> f64 { if a.is_fixnum() { a.as_fix() as f64 } else { c.big(a).to_f64() } }
pub fn ratio_f64(c: &Ctx, n: Value, d: Value) -> f64 {
    if n.is_fixnum() && d.is_fixnum() && n.as_fix().unsigned_abs() < (1 << 53) && d.as_fix() < (1 << 53) { return n.as_fix() as f64 / d.as_fix() as f64; }
    BigInt::ratio_to_f64(&c.to_big(n), &c.to_big(d))
}

// ---- construction --------------------------------------------------------------------------

pub fn is_rational(c: &Ctx, v: Value) -> bool { c.data::<RRational>(v).is_some() }
pub fn rat_parts(c: &Ctx, v: Value) -> (Value, Value) { let r = c.data::<RRational>(v).expect("a Rational"); (r.num, r.den) }

/// Already-normalized parts.
pub fn rat_raw(c: &mut Ctx, num: Value, den: Value) -> Value {
    let k = c.vm().c_rational.get();
    // built after the slot is found: num and den stay on the stack (rooted) through a collection
    let v = c.alloc_with(k, move || Body::Data(Box::new(RRational { num, den })));
    obj_set_flags(v, FL_FROZEN);
    v
}
/// num/den (den != 0), reduced.
pub fn rat_new(c: &mut Ctx, num: Value, den: Value) -> Value {
    let (mut n, mut d) = (num, den);
    if isneg(c, d) { n = ineg(c, n); d = ineg(c, d); }
    if d == Value::fix(1) { return rat_raw(c, n, d); }
    let g = igcd(c, n, d);
    if g != Value::fix(1) && !iszero(g) { n = idiv(c, n, g); d = idiv(c, d, g); }
    rat_raw(c, n, d)
}
fn zerodiv<T>(c: &mut Ctx) -> Result<T, Esc> { err(c, c.vm().c_zero_div_error.get(), "divided by 0") }
pub fn rat_new_checked(c: &mut Ctx, num: Value, den: Value) -> RV {
    if iszero(den) { return zerodiv(c); }
    Ok(rat_new(c, num, den))
}

/// Exact (num, den) of a finite float.
pub fn float_parts(c: &mut Ctx, f: f64) -> (Value, Value) {
    if f == 0.0 { return (Value::fix(0), Value::fix(1)); }
    let bits = f.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let mut m = bits & ((1u64 << 52) - 1);
    let mut e = if exp == 0 { -1074 } else { m |= 1 << 52; exp - 1075 };
    let tz = (m.trailing_zeros() as i64).min(if e < 0 { -e } else { 0 });
    m >>= tz; e += tz;
    let mut n = BigInt::from_u64(m);
    if f < 0.0 { n = n.neg(); }
    if e >= 0 { (c.big_to_value(n.shl(e as usize)), Value::fix(1)) }
    else { let nv = c.big_to_value(n); (nv, c.big_to_value(BigInt::from_i64(1).shl((-e) as usize))) }
}
pub fn float_to_r(c: &mut Ctx, f: f64) -> RV {
    if !f.is_finite() { return err(c, c.vm().c_float_domain_error.get(), float_to_s(f)); }
    let (n, d) = float_parts(c, f);
    Ok(rat_raw(c, n, d))
}

/// (num, den) for Integer / Rational
fn exact_parts(c: &Ctx, v: Value) -> Option<(Value, Value)> {
    if is_integer(v) { Some((v, Value::fix(1))) } else { c.data::<RRational>(v).map(|r| (r.num, r.den)) }
}
pub fn rat_to_f(c: &Ctx, v: Value) -> f64 { let (n, d) = rat_parts(c, v); ratio_f64(c, n, d) }

// ---- exact (Integer / Rational) helpers for other builtins; results canonical ----------------

pub fn ex_canon(c: &Ctx, v: Value) -> Value { if let Some((n, d)) = exact_parts(c, v) { if d == Value::fix(1) { return n; } } v }
fn ep(c: &Ctx, v: Value) -> (Value, Value) { exact_parts(c, v).expect("exact number") }
pub fn ex_add(c: &mut Ctx, a: Value, b: Value) -> Value { if a.is_fixnum() && b.is_fixnum() { return iadd(c, a, b); } let r = add_parts(c, ep(c, a), ep(c, b), false); ex_canon(c, r) }
pub fn ex_sub(c: &mut Ctx, a: Value, b: Value) -> Value { if a.is_fixnum() && b.is_fixnum() { return isub(c, a, b); } let r = add_parts(c, ep(c, a), ep(c, b), true); ex_canon(c, r) }
pub fn ex_mul(c: &mut Ctx, a: Value, b: Value) -> Value { if a.is_fixnum() && b.is_fixnum() { return imul(c, a, b); } let r = mul_parts(c, ep(c, a), ep(c, b)); ex_canon(c, r) }
/// b != 0
pub fn ex_div(c: &mut Ctx, a: Value, b: Value) -> Value { let (n, d) = ep(c, b); let r = mul_parts(c, ep(c, a), (d, n)); ex_canon(c, r) }
pub fn ex_cmp(c: &mut Ctx, a: Value, b: Value) -> Ordering {
    if is_integer(a) && is_integer(b) { return icmp(c, a, b); }
    let ((an, ad), (bn, bd)) = (ep(c, a), ep(c, b));
    let x = imul(c, an, bd); let y = imul(c, bn, ad);
    icmp(c, x, y)
}
pub fn ex_floor(c: &mut Ctx, a: Value) -> Value { if is_integer(a) { a } else { let (n, d) = rat_parts(c, a); idiv(c, n, d) } }
/// a mod m (m > 0 exact)
pub fn ex_mod(c: &mut Ctx, a: Value, m: Value) -> Value { let q = ex_div(c, a, m); let q = ex_floor(c, q); let p = ex_mul(c, q, m); ex_sub(c, a, p) }
pub fn ex_to_f(c: &Ctx, a: Value) -> f64 { if is_integer(a) { int_to_f(c, a) } else { rat_to_f(c, a) } }
pub fn ex_to_s(c: &Ctx, a: Value) -> String { if is_integer(a) { istr(c, a) } else { let (n, d) = rat_parts(c, a); format!("{}/{}", istr(c, n), istr(c, d)) } }

// ---- arithmetic ----------------------------------------------------------------------------

fn add_parts(c: &mut Ctx, a: (Value, Value), b: (Value, Value), sub: bool) -> Value {
    let (an, ad) = a; let (bn, bd) = b;
    let bn = if sub { ineg(c, bn) } else { bn };
    if ad == bd { let n = iadd(c, an, bn); return rat_new(c, n, ad); }
    let x = imul(c, an, bd); let y = imul(c, bn, ad);
    let n = iadd(c, x, y); let d = imul(c, ad, bd);
    rat_new(c, n, d)
}
fn mul_parts(c: &mut Ctx, a: (Value, Value), b: (Value, Value)) -> Value {
    let (an, ad) = a; let (bn, bd) = b;
    if iszero(an) || iszero(bn) { return rat_raw(c, Value::fix(0), Value::fix(1)); }
    let g1 = igcd(c, an, bd); let g2 = igcd(c, bn, ad);
    let (x, y) = (idiv(c, an, g1), idiv(c, bn, g2));
    let n = imul(c, x, y);
    let (x, y) = (idiv(c, ad, g2), idiv(c, bd, g1));
    let d = imul(c, x, y);
    if isneg(c, d) { let (n, d) = (ineg(c, n), ineg(c, d)); rat_raw(c, n, d) } else { rat_raw(c, n, d) }
}

fn rat_plus(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0];
    if let Some(p) = exact_parts(c, o) { return Ok(add_parts(c, rat_parts(c, s), p, false)); }
    if is_float(o) { return Ok(c.float_new(rat_to_f(c, s) + float_val(o))); }
    coerce_bin(c, s, o, s::plus, false)
}
fn rat_minus(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0];
    if let Some(p) = exact_parts(c, o) { return Ok(add_parts(c, rat_parts(c, s), p, true)); }
    if is_float(o) { return Ok(c.float_new(rat_to_f(c, s) - float_val(o))); }
    coerce_bin(c, s, o, s::minus, false)
}
fn rat_mul(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0];
    if let Some(p) = exact_parts(c, o) { return Ok(mul_parts(c, rat_parts(c, s), p)); }
    if is_float(o) { return Ok(c.float_new(rat_to_f(c, s) * float_val(o))); }
    coerce_bin(c, s, o, s::mul, false)
}
pub fn rat_div_v(c: &mut Ctx, s: Value, o: Value) -> RV {
    if let Some((n, d)) = exact_parts(c, o) {
        if iszero(n) { return zerodiv(c); }
        return Ok(mul_parts(c, rat_parts(c, s), (d, n)));
    }
    if is_float(o) { return Ok(c.float_new(rat_to_f(c, s) / float_val(o))); }
    coerce_bin(c, s, o, s::div, false)
}
fn rat_div(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { rat_div_v(c, s, a[0]) }
fn rat_fdiv(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0];
    if exact_zero(c, o) { return Ok(c.float_new(rat_to_f(c, s) / 0.0)); }
    let d = rat_div_v(c, s, o)?;
    if is_rational(c, d) { return Ok(c.float_new(rat_to_f(c, d))); }
    if is_float(d) { return Ok(d); }
    funcall(c, d, s::to_f, &[], std::ptr::null())
}
fn rat_uminus(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (n, d) = rat_parts(c, s); let n = ineg(c, n); Ok(rat_raw(c, n, d)) }
fn rat_abs(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (n, d) = rat_parts(c, s); Ok(if isneg(c, n) { let n = ineg(c, n); rat_raw(c, n, d) } else { s }) }

fn rat_pow(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let mut o = a[0];
    if exact_zero(c, o) { return Ok(rat_raw(c, Value::fix(1), Value::fix(1))); }
    if is_rational(c, o) && rat_parts(c, o).1 == Value::fix(1) { o = rat_parts(c, o).0; }
    let (n, d) = rat_parts(c, s);
    // 0**n and 1**n of an exact n
    if d == Value::fix(1) && (is_integer(o) || is_rational(c, o)) {
        if n == Value::fix(1) { return Ok(rat_raw(c, Value::fix(1), Value::fix(1))); }
        if n == Value::fix(-1) && is_integer(o) { let x = Value::fix(if iodd(c, o) { -1 } else { 1 }); return Ok(rat_raw(c, x, Value::fix(1))); }
        if iszero(n) {
            let neg = if is_integer(o) { isneg(c, o) } else { isneg(c, rat_parts(c, o).0) };
            if neg { return zerodiv(c); }
            return Ok(rat_raw(c, Value::fix(0), Value::fix(1)));
        }
    }
    if is_integer(o) {
        if !o.is_fixnum() { return arg_err(c, "exponent is too large"); }
        let e = o.as_fix();
        let ea = e.unsigned_abs();
        let bits = (c.to_big(n).bit_length().max(c.to_big(d).bit_length()) as u64).saturating_mul(ea);
        if bits > 1u64 << 34 { return arg_err(c, "exponent is too large"); }
        let (b0, b1) = if e > 0 { (n, d) } else { (d, n) };
        let (mut rn, mut rd) = (ipow(c, b0, ea), ipow(c, b1, ea));
        if isneg(c, rd) { rn = ineg(c, rn); rd = ineg(c, rd); }
        return Ok(rat_raw(c, rn, rd));
    }
    if is_float(o) || is_rational(c, o) {
        let f = c.float_new(rat_to_f(c, s));
        let y = if is_rational(c, o) { c.float_new(rat_to_f(c, o)) } else { o };
        return funcall(c, f, s::pow, &[y], std::ptr::null());
    }
    coerce_bin(c, s, o, s::pow, false)
}

pub fn rat_cmp_v(c: &mut Ctx, s: Value, o: Value) -> RV {
    let (n, d) = rat_parts(c, s);
    if let Some((on, od)) = exact_parts(c, o) {
        let ord = if d == od { icmp(c, n, on) } else { let x = imul(c, n, od); let y = imul(c, on, d); icmp(c, x, y) };
        return Ok(Value::fix(ord as i64));
    }
    if is_float(o) {
        let (x, y) = (rat_to_f(c, s), float_val(o));
        return Ok(match x.partial_cmp(&y) { Some(ord) => Value::fix(ord as i64), None => NIL });
    }
    if o.is_heap() && respond_to(c, o, s::coerce, true) && !is_string(o) {
        // rb_num_coerce_cmp: an exception in #coerce goes through
        let r = funcall(c, o, s::coerce, &[s], std::ptr::null())?;
        if r == NIL { return Ok(NIL); }
        if !is_array(r) || c.ary_len(r) != 2 { return type_err(c, "coerce must return [x, y]"); }
        let (x, y) = (c.ary(r)[0], c.ary(r)[1]);
        return funcall(c, x, s::cmp, &[y], std::ptr::null());
    }
    Ok(NIL)
}
fn rat_cmp(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { rat_cmp_v(c, s, a[0]) }
fn rat_eq(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0];
    let (n, d) = rat_parts(c, s);
    if is_integer(o) { return Ok(Value::bool(d == Value::fix(1) && icmp(c, n, o) == Ordering::Equal)); }
    if is_rational(c, o) { let (on, od) = rat_parts(c, o); return Ok(Value::bool(icmp(c, n, on).is_eq() && icmp(c, d, od).is_eq())); }
    if is_float(o) { return Ok(Value::bool(rat_to_f(c, s) == float_val(o))); }
    let r = funcall(c, o, s::eq, &[s], std::ptr::null())?;
    Ok(Value::bool(r.truthy()))
}
fn rat_coerce(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0];
    if is_integer(o) { let r = rat_raw(c, o, Value::fix(1)); return Ok(c.ary_new_from(&[r, s])); }
    if is_float(o) { let f = c.float_new(rat_to_f(c, s)); return Ok(c.ary_new_from(&[o, f])); }
    if is_rational(c, o) { return Ok(c.ary_new_from(&[o, s])); }
    if is_complex(c, o) {
        let (re, im) = cpx_parts(c, o);
        if exact_zero(c, im) {
            let r = if is_integer(re) { rat_raw(c, re, Value::fix(1)) } else { re };
            if is_rational(c, r) { return Ok(c.ary_new_from(&[r, s])); }
        }
        let x = cpx_raw(c, s, Value::fix(0));
        return Ok(c.ary_new_from(&[o, x]));
    }
    type_err(c, format!("{} can't be coerced into Rational", class_name_for_err(c, o)))
}

// ---- rounding -------------------------------------------------------------------------------

fn r_floor(c: &mut Ctx, n: Value, d: Value) -> Value { idiv(c, n, d) }
fn r_ceil(c: &mut Ctx, n: Value, d: Value) -> Value { let m = ineg(c, n); let q = idiv(c, m, d); ineg(c, q) }
fn r_trunc(c: &mut Ctx, n: Value, d: Value) -> Value { if isneg(c, n) { r_ceil(c, n, d) } else { r_floor(c, n, d) } }
fn r_round(c: &mut Ctx, n: Value, d: Value, half: u8) -> Value {
    let neg = isneg(c, n);
    let an = iabs(c, n);
    let two = Value::fix(2);
    let an2 = imul(c, an, two);
    let dd = imul(c, d, two);
    let q = match half {
        2 => { let x = isub(c, an2, d); let y = isub(c, dd, Value::fix(1)); let t = iadd(c, x, y); idiv(c, t, dd) }
        1 => {
            let x = iadd(c, an2, d);
            let q = idiv(c, x, dd);
            let m = imod(c, x, dd);
            if iszero(m) && iodd(c, q) { isub(c, q, Value::fix(1)) } else { q }
        }
        _ => { let x = iadd(c, an2, d); idiv(c, x, dd) }
    };
    if neg { ineg(c, q) } else { q }
}
fn r_by_mode(c: &mut Ctx, mode: u8, half: u8, n: Value, d: Value) -> Value {
    match mode { b'f' => r_floor(c, n, d), b'c' => r_ceil(c, n, d), b't' => r_trunc(c, n, d), _ => r_round(c, n, d, half) }
}
fn rat_round_common(c: &mut Ctx, s: Value, a: &[Value], mode: u8) -> RV {
    let half = if mode == b'r' { round_half_mode(c, a)? } else { 0 };
    let (pos, _) = if mode == b'r' { kw_split(c, a) } else { (a, None) };
    if pos.len() > 1 { return arg_err(c, format!("wrong number of arguments (given {}, expected 0..1)", pos.len())); }
    let (n, d) = rat_parts(c, s);
    if pos.is_empty() { return Ok(r_by_mode(c, mode, half, n, d)); }
    let nd = pos[0];
    if !is_integer(nd) { return type_err(c, "not an integer"); }
    let nd = match if nd.is_fixnum() { Some(nd.as_fix()) } else { c.to_big(nd).to_i64() } { Some(k) => k, None => return arg_err(c, "exponent is too large") };
    if nd.unsigned_abs().saturating_mul(4) > 1u64 << 34 { return arg_err(c, "exponent is too large"); }
    Ok(rat_round_nd(c, s, nd, mode, half))
}
/// round/floor/ceil/truncate (mode r/f/c/t) of a Rational to k decimal digits
pub fn rat_round_nd(c: &mut Ctx, s: Value, k: i64, mode: u8, half: u8) -> Value {
    let (n, d) = rat_parts(c, s);
    let p = ipow(c, Value::fix(10), k.unsigned_abs());
    let sc = if k >= 0 { mul_parts(c, (n, d), (p, Value::fix(1))) } else { mul_parts(c, (n, d), (Value::fix(1), p)) };
    let (sn, sd) = rat_parts(c, sc);
    let r = r_by_mode(c, mode, half, sn, sd);
    let res = if k >= 0 { rat_new(c, r, p) } else { let x = imul(c, r, p); rat_raw(c, x, Value::fix(1)) };
    if k < 1 { let (rn, rd) = rat_parts(c, res); return r_trunc(c, rn, rd); }
    res
}
fn rat_floor(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { rat_round_common(c, s, a, b'f') }
fn rat_ceil(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { rat_round_common(c, s, a, b'c') }
fn rat_truncate(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { rat_round_common(c, s, a, b't') }
fn rat_round(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { rat_round_common(c, s, a, b'r') }

// ---- conversions ----------------------------------------------------------------------------

fn rat_to_s(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (n, d) = rat_parts(c, s);
    let t = format!("{}/{}", istr(c, n), istr(c, d));
    Ok(c.str_new_enc(t.as_bytes(), crate::object::ENC_ASCII))
}
fn rat_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (n, d) = rat_parts(c, s);
    let t = format!("({}/{})", istr(c, n), istr(c, d));
    Ok(c.str_new_enc(t.as_bytes(), crate::object::ENC_ASCII))
}
fn rat_to_f_m(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.float_new(rat_to_f(c, s))) }
fn rat_to_i(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (n, d) = rat_parts(c, s); Ok(r_trunc(c, n, d)) }
fn rat_to_r(_c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(s) }
fn rat_num(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(rat_parts(c, s).0) }
fn rat_den(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(rat_parts(c, s).1) }
fn rat_pos_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let n = rat_parts(c, s).0; Ok(Value::bool(!isneg(c, n) && !iszero(n))) }
fn rat_neg_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(isneg(c, rat_parts(c, s).0))) }
fn rat_hash(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (n, d) = rat_parts(c, s);
    let h = crate::astrust::hash_merge(crate::hash::key_hash(c, n)?, crate::hash::key_hash(c, d)? ^ 0x5241);
    Ok(Value::fix((h >> 2) as i64))
}

/// Simplest rational within [a, b] (a <= b), both as (num, den) with positive den.
fn simplest_between(c: &mut Ctx, a: (Value, Value), b: (Value, Value)) -> (Value, Value) {
    let (mut a, mut b) = (a, b);
    let (mut p0, mut p1, mut q0, mut q1) = (Value::fix(0), Value::fix(1), Value::fix(1), Value::fix(0));
    let one = Value::fix(1);
    let cc;
    loop {
        let ce = r_ceil(c, a.0, a.1);
        // ce < b ?
        let x = imul(c, ce, b.1);
        if icmp(c, x, b.0) == Ordering::Less { cc = ce; break; }
        let k = isub(c, ce, one);
        let t = imul(c, k, p1); let p2 = iadd(c, t, p0);
        let t = imul(c, k, q1); let q2 = iadd(c, t, q0);
        // t = 1/(b - k); b = 1/(a - k); a = t
        let t = imul(c, k, b.1); let bk = (isub(c, b.0, t), b.1);
        let t = imul(c, k, a.1); let ak = (isub(c, a.0, t), a.1);
        let t = (bk.1, bk.0);
        b = (ak.1, ak.0);
        a = t;
        p0 = p1; q0 = q1; p1 = p2; q1 = q2;
    }
    let t = imul(c, cc, p1); let p = iadd(c, t, p0);
    let t = imul(c, cc, q1); let q = iadd(c, t, q0);
    (p, q)
}
fn norm_pair(c: &mut Ctx, n: Value, d: Value) -> (Value, Value) { let r = rat_new(c, n, d); rat_parts(c, r) }
fn to_exact_pair(c: &mut Ctx, v: Value) -> Result<(Value, Value), Esc> {
    if let Some(p) = exact_parts(c, v) { return Ok(p); }
    if is_float(v) { let f = float_val(v); if !f.is_finite() { return err(c, c.vm().c_float_domain_error.get(), float_to_s(f)); } return Ok(float_parts(c, f)); }
    let r = funcall(c, v, intern(b"to_r"), &[], std::ptr::null())?;
    match exact_parts(c, r) { Some(p) => Ok(p), None => type_err(c, format!("can't convert {} into Rational", class_name_for_err(c, v))) }
}
fn rationalize_with(c: &mut Ctx, x: (Value, Value), e: Value) -> RV {
    let e = to_exact_pair(c, e)?;
    let e = (iabs(c, e.0), e.1);
    let neg = isneg(c, x.0);
    let xa = (iabs(c, x.0), x.1);
    let (t1, t2) = (imul(c, xa.0, e.1), imul(c, e.0, xa.1));
    let den = imul(c, xa.1, e.1);
    let lo = isub(c, t1, t2); let a = norm_pair(c, lo, den);
    let hi = iadd(c, t1, t2); let b = norm_pair(c, hi, den);
    if icmp(c, a.0, b.0).is_eq() && icmp(c, a.1, b.1).is_eq() { return Ok(rat_new(c, x.0, x.1)); }
    let (p, q) = simplest_between(c, a, b);
    let p = if neg { ineg(c, p) } else { p };
    Ok(rat_new(c, p, q))
}
fn rat_rationalize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    if a.is_empty() { return Ok(s); }
    rationalize_with(c, rat_parts(c, s), a[0])
}
fn flo_to_r(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { float_to_r(c, float_val(s)) }
fn flo_rationalize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let f = float_val(s);
    if !f.is_finite() { return err(c, c.vm().c_float_domain_error.get(), float_to_s(f)); }
    if !a.is_empty() { let p = float_parts(c, f); return rationalize_with(c, p, a[0]); }
    if f == 0.0 { return Ok(rat_raw(c, Value::fix(0), Value::fix(1))); }
    // f = m * 2^e with a 53-bit mantissa
    let (fr, ex) = frexp(f.abs());
    let m = (fr * 9007199254740992.0) as i64; let e = ex - 53;
    if e >= 0 {
        let mut b = BigInt::from_i64(m).shl(e as usize);
        if f < 0.0 { b = b.neg(); }
        let n = c.big_to_value(b);
        return Ok(rat_raw(c, n, Value::fix(1)));
    }
    let den = c.big_to_value(BigInt::from_i64(1).shl((1 - e) as usize));
    let a2 = norm_pair(c, Value::fix(2 * m - 1), den);
    let b2 = norm_pair(c, Value::fix(2 * m + 1), den);
    let (p, q) = simplest_between(c, a2, b2);
    let p = if f < 0.0 { ineg(c, p) } else { p };
    Ok(rat_new(c, p, q))
}
pub fn frexp(f: f64) -> (f64, i64) {
    if f == 0.0 || !f.is_finite() { return (f, 0); }
    let bits = f.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i64;
    if exp == 0 { let (m, e) = frexp(f * 2f64.powi(64)); return (m, e - 64); }
    let m = f64::from_bits((bits & !(0x7ffu64 << 52)) | (1022u64 << 52));
    (m, exp - 1022)
}
fn flo_numerator(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let f = float_val(s);
    if f.is_nan() || f.is_infinite() { return Ok(s); }
    let r = float_to_r(c, f)?; Ok(rat_parts(c, r).0)
}
fn flo_denominator(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let f = float_val(s);
    if !f.is_finite() { return Ok(Value::fix(1)); }
    let r = float_to_r(c, f)?; Ok(rat_parts(c, r).1)
}

// ---- parsing --------------------------------------------------------------------------------

fn skip_ws(b: &[u8], mut i: usize) -> usize { while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) { i += 1; } i }
/// digits with single underscores between digits
fn read_digits(b: &[u8], i: &mut usize) -> Option<(BigInt, usize)> {
    let st = *i;
    if st >= b.len() || !b[st].is_ascii_digit() { return None; }
    let mut j = st; let mut s = String::new(); let mut cnt = 0;
    while j < b.len() {
        if b[j].is_ascii_digit() { s.push(b[j] as char); cnt += 1; j += 1; }
        else if b[j] == b'_' && j + 1 < b.len() && b[j + 1].is_ascii_digit() && j > st { j += 1; }
        else { break; }
    }
    *i = j;
    Some((BigInt::parse(&s, 10).unwrap(), cnt))
}
/// returns (num, exp10) meaning num * 10**exp10 (exp10 saturates far beyond fixnum range)
fn read_num(b: &[u8], i: &mut usize) -> Option<(BigInt, i128)> {
    let mut ok = false;
    let mut n = BigInt::zero(); let mut nexp: i128 = 0;
    if *i < b.len() && b[*i] != b'.' {
        let (x, _) = read_digits(b, i)?;
        n = x; ok = true;
    }
    if *i < b.len() && b[*i] == b'.' {
        *i += 1;
        match read_digits(b, i) {
            None => return Some((n, nexp)),
            Some((fp, cnt)) => { n = n.mul(&BigInt::from_i64(10).pow(cnt as u64)).add(&fp); nexp = -(cnt as i128); ok = true; }
        }
    }
    if !ok { return None; }
    if *i + 1 < b.len() && (b[*i] == b'e' || b[*i] == b'E') {
        let save = *i;
        *i += 1;
        let mut neg = false;
        let signed = b[*i] == b'-' || b[*i] == b'+';
        if signed { neg = b[*i] == b'-'; *i += 1; }
        match read_digits(b, i) {
            // CRuby keeps a consumed "e+" / "e-" even without digits (Rational("1e+") is 1)
            None => { if !signed { *i = save; } return Some((n, nexp)); }
            Some((e, _)) => {
                let e: i128 = e.to_i64().map(|x| x as i128).unwrap_or(1i128 << 100);
                nexp += if neg { -e } else { e };
            }
        }
    }
    Some((n, nexp))
}
const FIXNUM_LIM: i128 = (1i128 << 62) - 1;
/// 10**e as CRuby's Integer#** would compute it (ArgumentError when too large).
fn pow10(c: &mut Ctx, e: i128) -> Result<BigInt, Esc> {
    if e * 4 > 1i128 << 34 { return arg_err(c, "exponent is too large"); }
    Ok(BigInt::from_i64(10).pow(e as u64))
}
/// Ok(None) = parse failure.  The value may be a Float (+-0.0 / Infinity) for out-of-range exponents.
pub fn parse_rational(c: &mut Ctx, b: &[u8], strict: bool, raise: bool) -> Result<Option<Value>, Esc> {
    let mut i = skip_ws(b, 0);
    let mut neg = false;
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') { neg = b[i] == b'-'; i += 1; }
    let Some((mut num, mut e)) = read_num(b, &mut i) else { return Ok(None) };
    let mut den = BigInt::from_i64(1);
    if i < b.len() && b[i] == b'/' {
        i += 1;
        match read_num(b, &mut i) {
            None if strict => return Ok(None),
            None => {} // to_r: "3/" and "1/-2" keep the numerator alone
            Some((dn, de)) => {
                if dn.is_zero() { if !raise { return Ok(None); } return zerodiv(c); }
                if strict && skip_ws(b, i) != b.len() { return Ok(None); }
                den = dn; e -= de;
            }
        }
    } else if strict && skip_ws(b, i) != b.len() { return Ok(None); }
    if e > 0 {
        if e > FIXNUM_LIM { return Ok(Some(c.float_new(if neg { f64::NEG_INFINITY } else { f64::INFINITY }))); }
        num = num.mul(&pow10(c, e)?);
    } else if e < 0 {
        if -e > FIXNUM_LIM { return Ok(Some(c.float_new(if neg { -0.0 } else { 0.0 }))); }
        den = den.mul(&pow10(c, -e)?);
    }
    if neg { num = num.neg(); }
    let n = c.big_to_value(num); let d = c.big_to_value(den);
    Ok(Some(rat_new(c, n, d)))
}
fn str_to_r(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let b = c.str_bytes(s).to_vec();
    match parse_rational(c, &b, false, true)? {
        None => Ok(rat_raw(c, Value::fix(0), Value::fix(1))),
        Some(v) if is_float(v) => { if float_val(v).is_infinite() { return err(c, c.vm().c_float_domain_error.get(), "Infinity"); } Ok(v) }
        Some(v) => Ok(v),
    }
}

// ---- Kernel#Rational ------------------------------------------------------------------------

fn exact_zero(c: &Ctx, v: Value) -> bool { v == Value::fix(0) || (is_rational(c, v) && iszero(rat_parts(c, v).0)) }
fn exact_one(c: &Ctx, v: Value) -> bool { v == Value::fix(1) || (is_rational(c, v) && rat_parts(c, v) == (Value::fix(1), Value::fix(1))) }

fn conv_arg(c: &mut Ctx, v: Value, raise: bool) -> Result<Option<Value>, Esc> {
    let mut v = v;
    if is_complex(c, v) { let (re, im) = cpx_parts(c, v); if exact_zero(c, im) { v = re; } }
    if is_float(v) { return Ok(Some(float_to_r(c, float_val(v))?)); }
    if is_string(v) {
        let b = c.str_bytes(v).to_vec();
        return match parse_rational(c, &b, true, raise)? {
            Some(r) if is_float(r) => {
                if float_val(r) == 0.0 { return Ok(Some(rat_raw(c, Value::fix(0), Value::fix(1)))); }
                if !raise { return Ok(None); }
                err(c, c.vm().c_float_domain_error.get(), "Infinity")
            }
            Some(r) => Ok(Some(r)),
            None => { if !raise { return Ok(None); } let ins = inspect_str(c, v)?; arg_err(c, format!("invalid value for convert(): {}", ins)) }
        };
    }
    if !is_kind_of(c, v, c.vm().c_numeric.get()) && v.is_heap() {
        if respond_to(c, v, intern(b"to_r"), true) {
            // exception: false also swallows what #to_r raises (rb_protect)
            let r = match funcall(c, v, intern(b"to_r"), &[], std::ptr::null()) {
                Err(_) if !raise && c.esc == EscKind::Raise => { c.esc = EscKind::None; return Ok(None); }
                r => r?,
            };
            if is_rational(c, r) { return Ok(Some(r)); }
            let cn = class_name_for_err(c, v);
            return type_err(c, format!("can't convert {} to Rational ({}#to_r gives {})", cn, cn, class_name_for_err(c, r)));
        }
        if respond_to(c, v, s::to_int, true) {
            // rb_check_to_int: an exception from to_int means "not an Integer"
            match funcall(c, v, s::to_int, &[], std::ptr::null()) {
                Ok(r) if is_integer(r) => return Ok(Some(r)),
                Ok(_) => {}
                Err(_) => { c.esc = EscKind::None; }
            }
        }
    }
    Ok(Some(v))
}
fn int_value(c: &mut Ctx, v: Value) -> RV {
    if is_integer(v) { return Ok(v); }
    if !is_numeric(v) && !is_rational(c, v) && v.is_heap() && respond_to(c, v, s::to_int, true) {
        match funcall(c, v, s::to_int, &[], std::ptr::null()) { Ok(r) if is_integer(r) => return Ok(r), Ok(_) => {} Err(_) => c.esc = EscKind::None }
    }
    type_err(c, "not an integer")
}
pub fn convert_rational(c: &mut Ctx, a1: Value, a2: Option<Value>, raise: bool) -> RV {
    if a1 == NIL || a2 == Some(NIL) { if !raise { return Ok(NIL); } return type_err(c, "can't convert nil into Rational"); }
    let Some(a1) = conv_arg(c, a1, raise)? else { return Ok(NIL) };
    let a2 = match a2 { None => None, Some(v) => match conv_arg(c, v, raise)? { Some(x) => Some(x), None => return Ok(NIL) } };
    if is_rational(c, a1) && a2.map(|v| exact_one(c, v)).unwrap_or(true) { return Ok(a1); }
    let Some(a2) = a2 else {
        if is_integer(a1) { return Ok(rat_raw(c, a1, Value::fix(1))); }
        if !raise {
            if !respond_to(c, a1, intern(b"to_r"), true) { return Ok(NIL); }
            return match funcall(c, a1, intern(b"to_r"), &[], std::ptr::null()) { Ok(r) if is_rational(c, r) => Ok(r), Ok(_) => Ok(NIL), Err(_) => { c.esc = EscKind::None; Ok(NIL) } };
        }
        if a1.is_heap() && respond_to(c, a1, intern(b"to_r"), true) {
            let r = funcall(c, a1, intern(b"to_r"), &[], std::ptr::null())?;
            if is_rational(c, r) { return Ok(r); }
            let cn = class_name_for_err(c, a1);
            return type_err(c, format!("can't convert {} to Rational ({}#to_r gives {})", cn, cn, class_name_for_err(c, r)));
        }
        return type_err(c, format!("can't convert {} into Rational", class_name_for_err(c, a1)));
    };
    let kn = c.vm().c_numeric.get();
    let numeric = |v: Value| is_kind_of(c, v, kn);
    if numeric(a1) && numeric(a2) && (!is_integer(a1) || !is_integer(a2)) {
        let x = if is_integer(a1) { rat_raw(c, a1, Value::fix(1)) } else { a1 };
        return funcall(c, x, s::div, &[a2], std::ptr::null());
    }
    let r: RV = (|| { let n = int_value(c, a1)?; let d = int_value(c, a2)?; rat_new_checked(c, n, d) })();
    match r { Err(e) => { if !raise && c.esc == EscKind::Raise { c.esc = EscKind::None; return Ok(NIL); } Err(e) } ok => ok }
}
/// Split off a trailing `exception:` keyword.
pub fn kw_exception(c: &mut Ctx, a: &[Value]) -> Result<(usize, bool), Esc> {
    if let Some(h) = kw_arg(c, a) {
        if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("exception"))? {
            if v != TRUE && v != FALSE { let ins = inspect_str(c, v)?; return arg_err(c, format!("expected true or false as exception: {}", ins)); }
            return Ok((a.len() - 1, v == TRUE));
        }
        return Ok((a.len() - 1, true));
    }
    Ok((a.len(), true))
}
fn k_rational(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (n, raise) = kw_exception(c, a)?;
    let a = &a[..n];
    check_argc(c, a, 1, 2)?;
    convert_rational(c, a[0], a.get(1).copied(), raise)
}

fn int_to_r(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(rat_raw(c, s, Value::fix(1))) }
fn int_rationalize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { check_argc(c, a, 0, 1)?; Ok(rat_raw(c, s, Value::fix(1))) }
fn nil_to_r(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(rat_raw(c, Value::fix(0), Value::fix(1))) }
fn nil_rationalize(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { check_argc(c, a, 0, 1)?; Ok(rat_raw(c, Value::fix(0), Value::fix(1))) }

/// Boot: defines Rational (returned) and the conversions on the other classes.
pub fn init(c: &mut Ctx) -> Value {
    let vmr = c.vm();
    let k = define_class(c, "Rational", vmr.c_numeric.get());
    class_set_alloc(c, k, AllocKind::None);
    let sk = singleton_class(c, k);
    define_method_me(c, sk, s::new, new_me(s::new, sk, VISI_PUBLIC, MKind::Undef));
    let d = |c: &mut Ctx, nm: &str, f: CFunc, ar: i32| define_cfunc(c, k, nm, f, ar);
    d(c, "+", rat_plus, 1); d(c, "-", rat_minus, 1); d(c, "*", rat_mul, 1); d(c, "/", rat_div, 1); d(c, "quo", rat_div, 1);
    d(c, "fdiv", rat_fdiv, 1); d(c, "**", rat_pow, 1); d(c, "-@", rat_uminus, 0); d(c, "abs", rat_abs, 0); d(c, "magnitude", rat_abs, 0);
    d(c, "<=>", rat_cmp, 1); d(c, "==", rat_eq, 1);
    d(c, "coerce", rat_coerce, 1);
    d(c, "floor", rat_floor, -1); d(c, "ceil", rat_ceil, -1); d(c, "truncate", rat_truncate, -1); d(c, "round", rat_round, -1);
    d(c, "to_s", rat_to_s, 0); d(c, "inspect", rat_inspect, 0); d(c, "to_f", rat_to_f_m, 0); d(c, "to_i", rat_to_i, 0); d(c, "to_r", rat_to_r, 0);
    d(c, "numerator", rat_num, 0); d(c, "denominator", rat_den, 0); d(c, "positive?", rat_pos_p, 0);
    d(c, "negative?", rat_neg_p, 0); d(c, "hash", rat_hash, 0); d(c, "rationalize", rat_rationalize, -1);
    define_private_cfunc(c, k, "marshal_dump", |c, s, _a, _b| { let (n, d) = rat_parts(c, s); Ok(c.ary_new_from(&[n, d])) }, 0);
    define_cfunc(c, vmr.c_float.get(), "to_r", flo_to_r, 0);
    define_cfunc(c, vmr.c_float.get(), "rationalize", flo_rationalize, -1);
    define_cfunc(c, vmr.c_float.get(), "numerator", flo_numerator, 0);
    define_cfunc(c, vmr.c_float.get(), "denominator", flo_denominator, 0);
    define_cfunc(c, vmr.c_integer.get(), "to_r", int_to_r, 0);
    define_cfunc(c, vmr.c_integer.get(), "rationalize", int_rationalize, -1);
    define_cfunc(c, vmr.c_nil.get(), "to_r", nil_to_r, 0);
    define_cfunc(c, vmr.c_nil.get(), "rationalize", nil_rationalize, -1);
    define_cfunc(c, vmr.c_string.get(), "to_r", str_to_r, 0);
    define_module_function(c, vmr.m_kernel.get(), "Rational", k_rational, -1);
    k
}
