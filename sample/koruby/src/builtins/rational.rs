// Rational: num/den Integers, den > 0, gcd(num, den) == 1.
use crate::array::*;
use crate::bignum::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::{coerce_bin, round_half_mode};
use crate::numeric::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;
use std::cmp::Ordering;

pub struct RRational { pub num: Value, pub den: Value }
impl RData for RRational {
    fn mark(&self, m: &mut crate::gc::Marker) { m.mark(self.num); m.mark(self.den); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

// ---- Integer helpers on Values (fixnum fast paths) ----------------------------------------

pub fn iadd(a: Value, b: Value) -> Value {
    if a.is_fixnum() && b.is_fixnum() { return int_from_i64(a.as_fix() + b.as_fix()); }
    big_to_value(to_big(a).add(&to_big(b)))
}
pub fn isub(a: Value, b: Value) -> Value {
    if a.is_fixnum() && b.is_fixnum() { return int_from_i64(a.as_fix() - b.as_fix()); }
    big_to_value(to_big(a).sub(&to_big(b)))
}
pub fn imul(a: Value, b: Value) -> Value {
    if a.is_fixnum() && b.is_fixnum() { return int_from_i128(a.as_fix() as i128 * b.as_fix() as i128); }
    big_to_value(to_big(a).mul(&to_big(b)))
}
/// floored division; b != 0
pub fn idiv(a: Value, b: Value) -> Value {
    if a.is_fixnum() && b.is_fixnum() { let (x, y) = (a.as_fix(), b.as_fix()); let q = x / y; return int_from_i64(if x % y != 0 && ((x < 0) != (y < 0)) { q - 1 } else { q }); }
    big_to_value(to_big(a).divmod(&to_big(b)).0)
}
pub fn imod(a: Value, b: Value) -> Value { isub(a, imul(idiv(a, b), b)) }
pub fn icmp(a: Value, b: Value) -> Ordering {
    if a.is_fixnum() && b.is_fixnum() { return a.as_fix().cmp(&b.as_fix()); }
    to_big(a).cmp(&to_big(b))
}
pub fn ineg(a: Value) -> Value { if a.is_fixnum() { int_from_i64(-a.as_fix()) } else { big_to_value(rbig(a).neg()) } }
pub fn isneg(a: Value) -> bool { if a.is_fixnum() { a.as_fix() < 0 } else { rbig(a).neg } }
pub fn iszero(a: Value) -> bool { a == Value::fix(0) }
pub fn iabs(a: Value) -> Value { if isneg(a) { ineg(a) } else { a } }
pub fn igcd(a: Value, b: Value) -> Value {
    if a.is_fixnum() && b.is_fixnum() {
        let (mut x, mut y) = (a.as_fix().unsigned_abs(), b.as_fix().unsigned_abs());
        while y != 0 { let r = x % y; x = y; y = r; }
        return int_from_i128(x as i128);
    }
    big_to_value(to_big(a).gcd(&to_big(b)))
}
pub fn ipow(a: Value, e: u64) -> Value {
    if a.is_fixnum() { if let Ok(e32) = u32::try_from(e) { if let Some(r) = a.as_fix().checked_pow(e32) { return int_from_i64(r); } } }
    big_to_value(to_big(a).pow(e))
}
pub fn iodd(a: Value) -> bool { if a.is_fixnum() { a.as_fix() & 1 == 1 } else { rbig(a).is_odd() } }
pub fn istr(a: Value) -> String { if a.is_fixnum() { a.as_fix().to_string() } else { rbig(a).to_string_radix(10) } }
pub fn int_to_f(a: Value) -> f64 { if a.is_fixnum() { a.as_fix() as f64 } else { rbig(a).to_f64() } }
pub fn ratio_f64(n: Value, d: Value) -> f64 {
    if n.is_fixnum() && d.is_fixnum() && n.as_fix().unsigned_abs() < (1 << 53) && d.as_fix() < (1 << 53) { return n.as_fix() as f64 / d.as_fix() as f64; }
    BigInt::ratio_to_f64(&to_big(n), &to_big(d))
}

// ---- construction --------------------------------------------------------------------------

pub fn rational_class() -> Value { vm().c_rational }
pub fn is_rational(v: Value) -> bool { v.is_heap() && rdata::<RRational>(v).is_some() }
pub fn rat_parts(v: Value) -> (Value, Value) { let r = rdata::<RRational>(v).unwrap(); (r.num, r.den) }

/// Already-normalized parts.
pub fn rat_raw(num: Value, den: Value) -> Value {
    let v = crate::gc::alloc(rational_class(), Body::Data(Box::new(RRational { num, den })));
    obj(v).flags |= FL_FROZEN;
    v
}
/// num/den (den != 0), reduced.
pub fn rat_new(num: Value, den: Value) -> Value {
    let (mut n, mut d) = (num, den);
    if isneg(d) { n = ineg(n); d = ineg(d); }
    if d == Value::fix(1) { return rat_raw(n, d); }
    let g = igcd(n, d);
    if g != Value::fix(1) && !iszero(g) { n = idiv(n, g); d = idiv(d, g); }
    rat_raw(n, d)
}
fn zerodiv<T>(c: &mut Ctx) -> Result<T, Esc> { err(c, vm().c_zero_div_error, "divided by 0") }
pub fn rat_new_checked(c: &mut Ctx, num: Value, den: Value) -> RV {
    if iszero(den) { return zerodiv(c); }
    Ok(rat_new(num, den))
}

/// Exact (num, den) of a finite float.
pub fn float_parts(f: f64) -> (Value, Value) {
    if f == 0.0 { return (Value::fix(0), Value::fix(1)); }
    let bits = f.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let mut m = bits & ((1u64 << 52) - 1);
    let mut e = if exp == 0 { -1074 } else { m |= 1 << 52; exp - 1075 };
    let tz = (m.trailing_zeros() as i64).min(if e < 0 { -e } else { 0 });
    m >>= tz; e += tz;
    let mut n = BigInt::from_u64(m);
    if f < 0.0 { n = n.neg(); }
    if e >= 0 { (big_to_value(n.shl(e as usize)), Value::fix(1)) }
    else { (big_to_value(n), big_to_value(BigInt::from_i64(1).shl((-e) as usize))) }
}
pub fn float_to_r(c: &mut Ctx, f: f64) -> RV {
    if !f.is_finite() { return err(c, vm().c_float_domain_error, float_to_s(f)); }
    let (n, d) = float_parts(f);
    Ok(rat_raw(n, d))
}

/// (num, den) for Integer / Rational
fn exact_parts(v: Value) -> Option<(Value, Value)> {
    if is_integer(v) { Some((v, Value::fix(1))) } else if is_rational(v) { Some(rat_parts(v)) } else { None }
}
pub fn rat_to_f(v: Value) -> f64 { let (n, d) = rat_parts(v); ratio_f64(n, d) }

// ---- exact (Integer / Rational) helpers for other builtins; results canonical ----------------

pub fn ex_canon(v: Value) -> Value { if is_rational(v) { let (n, d) = rat_parts(v); if d == Value::fix(1) { return n; } } v }
fn ep(v: Value) -> (Value, Value) { exact_parts(v).expect("exact number") }
pub fn ex_add(a: Value, b: Value) -> Value { if a.is_fixnum() && b.is_fixnum() { return iadd(a, b); } ex_canon(add_parts(ep(a), ep(b), false)) }
pub fn ex_sub(a: Value, b: Value) -> Value { if a.is_fixnum() && b.is_fixnum() { return isub(a, b); } ex_canon(add_parts(ep(a), ep(b), true)) }
pub fn ex_mul(a: Value, b: Value) -> Value { if a.is_fixnum() && b.is_fixnum() { return imul(a, b); } ex_canon(mul_parts(ep(a), ep(b))) }
/// b != 0
pub fn ex_div(a: Value, b: Value) -> Value { let (n, d) = ep(b); ex_canon(mul_parts(ep(a), (d, n))) }
pub fn ex_cmp(a: Value, b: Value) -> Ordering {
    if is_integer(a) && is_integer(b) { return icmp(a, b); }
    let ((an, ad), (bn, bd)) = (ep(a), ep(b));
    icmp(imul(an, bd), imul(bn, ad))
}
pub fn ex_floor(a: Value) -> Value { if is_integer(a) { a } else { let (n, d) = rat_parts(a); idiv(n, d) } }
/// a mod m (m > 0 exact)
pub fn ex_mod(a: Value, m: Value) -> Value { let q = ex_floor(ex_div(a, m)); ex_sub(a, ex_mul(q, m)) }
pub fn ex_to_f(a: Value) -> f64 { if is_integer(a) { int_to_f(a) } else { rat_to_f(a) } }
pub fn ex_to_s(a: Value) -> String { if is_integer(a) { istr(a) } else { let (n, d) = rat_parts(a); format!("{}/{}", istr(n), istr(d)) } }

// ---- arithmetic ----------------------------------------------------------------------------

fn add_parts(a: (Value, Value), b: (Value, Value), sub: bool) -> Value {
    let (an, ad) = a; let (bn, bd) = b;
    let bn = if sub { ineg(bn) } else { bn };
    if ad == bd { return rat_new(iadd(an, bn), ad); }
    rat_new(iadd(imul(an, bd), imul(bn, ad)), imul(ad, bd))
}
fn mul_parts(a: (Value, Value), b: (Value, Value)) -> Value {
    let (an, ad) = a; let (bn, bd) = b;
    if iszero(an) || iszero(bn) { return rat_raw(Value::fix(0), Value::fix(1)); }
    let g1 = igcd(an, bd); let g2 = igcd(bn, ad);
    let n = imul(idiv(an, g1), idiv(bn, g2));
    let d = imul(idiv(ad, g2), idiv(bd, g1));
    if isneg(d) { rat_raw(ineg(n), ineg(d)) } else { rat_raw(n, d) }
}

fn rat_plus(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if let Some(p) = exact_parts(o) { return Ok(add_parts(rat_parts(s), p, false)); }
    if is_float(o) { return Ok(float_new(rat_to_f(s) + float_val(o))); }
    coerce_bin(c, s, o, s::plus, false)
}
fn rat_minus(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if let Some(p) = exact_parts(o) { return Ok(add_parts(rat_parts(s), p, true)); }
    if is_float(o) { return Ok(float_new(rat_to_f(s) - float_val(o))); }
    coerce_bin(c, s, o, s::minus, false)
}
fn rat_mul(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if let Some(p) = exact_parts(o) { return Ok(mul_parts(rat_parts(s), p)); }
    if is_float(o) { return Ok(float_new(rat_to_f(s) * float_val(o))); }
    coerce_bin(c, s, o, s::mul, false)
}
pub fn rat_div_v(c: &mut Ctx, s: Value, o: Value) -> RV {
    if let Some((n, d)) = exact_parts(o) {
        if iszero(n) { return zerodiv(c); }
        return Ok(mul_parts(rat_parts(s), (d, n)));
    }
    if is_float(o) { return Ok(float_new(rat_to_f(s) / float_val(o))); }
    coerce_bin(c, s, o, s::div, false)
}
fn rat_div(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { rat_div_v(c, s, a[0]) }
fn rat_fdiv(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if o == Value::fix(0) || (is_rational(o) && iszero(rat_parts(o).0)) { return Ok(float_new(rat_to_f(s) / 0.0)); }
    let d = rat_div_v(c, s, o)?;
    if is_rational(d) { return Ok(float_new(rat_to_f(d))); }
    if is_float(d) { return Ok(d); }
    funcall(c, d, s::to_f, &[], std::ptr::null())
}
fn rat_uminus(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let (n, d) = rat_parts(s); Ok(rat_raw(ineg(n), d)) }
fn rat_abs(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let (n, d) = rat_parts(s); Ok(if isneg(n) { rat_raw(ineg(n), d) } else { s }) }

fn rat_pow(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let mut o = a[0];
    if o == Value::fix(0) || (is_rational(o) && iszero(rat_parts(o).0)) { return Ok(rat_raw(Value::fix(1), Value::fix(1))); }
    if is_rational(o) && rat_parts(o).1 == Value::fix(1) { o = rat_parts(o).0; }
    let (n, d) = rat_parts(s);
    if is_integer(o) {
        if d == Value::fix(1) {
            if n == Value::fix(1) { return Ok(rat_raw(Value::fix(1), Value::fix(1))); }
            if n == Value::fix(-1) { return Ok(rat_raw(Value::fix(if iodd(o) { -1 } else { 1 }), Value::fix(1))); }
            if iszero(n) { if isneg(o) { return zerodiv(c); } return Ok(rat_raw(Value::fix(0), Value::fix(1))); }
        }
        if !o.is_fixnum() { return arg_err(c, "exponent is too large"); }
        let e = o.as_fix();
        let ea = e.unsigned_abs();
        let bits = (to_big(n).bit_length().max(to_big(d).bit_length()) as u64).saturating_mul(ea);
        if bits > 1u64 << 34 { return arg_err(c, "exponent is too large"); }
        let (mut rn, mut rd) = if e > 0 { (ipow(n, ea), ipow(d, ea)) } else { (ipow(d, ea), ipow(n, ea)) };
        if isneg(rd) { rn = ineg(rn); rd = ineg(rd); }
        return Ok(rat_raw(rn, rd));
    }
    if is_float(o) || is_rational(o) {
        let f = float_new(rat_to_f(s));
        let y = if is_rational(o) { float_new(rat_to_f(o)) } else { o };
        return funcall(c, f, s::pow, &[y], std::ptr::null());
    }
    coerce_bin(c, s, o, s::pow, false)
}

pub fn rat_cmp_v(c: &mut Ctx, s: Value, o: Value) -> RV {
    let (n, d) = rat_parts(s);
    if let Some((on, od)) = exact_parts(o) {
        let ord = if d == od { icmp(n, on) } else { icmp(imul(n, od), imul(on, d)) };
        return Ok(Value::fix(ord as i64));
    }
    if is_float(o) {
        let (x, y) = (rat_to_f(s), float_val(o));
        return Ok(match x.partial_cmp(&y) { Some(ord) => Value::fix(ord as i64), None => NIL });
    }
    if o.is_heap() && respond_to(o, s::coerce, true) && !is_string(o) {
        let r = funcall(c, o, s::coerce, &[s], std::ptr::null());
        return match r {
            Ok(r) if is_array(r) && ary_len(r) == 2 => { let x = ary_get(r, 0); funcall(c, x, s::cmp, &[ary_get(r, 1)], std::ptr::null()) }
            Ok(_) => type_err(c, "coerce must return [x, y]"),
            Err(_) => { c.esc = EscKind::None; Ok(NIL) }
        };
    }
    Ok(NIL)
}
fn rat_cmp(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { rat_cmp_v(c, s, a[0]) }
fn rat_rel(c: &mut Ctx, s: Value, o: Value, op: Sym, f: fn(Ordering) -> bool) -> RV {
    if exact_parts(o).is_some() || is_float(o) {
        let r = rat_cmp_v(c, s, o)?;
        if r == NIL { return Ok(FALSE); }
        return Ok(Value::bool(f(r.as_fix().cmp(&0))));
    }
    coerce_bin(c, s, o, op, true)
}
fn rat_lt(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { rat_rel(c, s, a[0], s::lt, |o| o.is_lt()) }
fn rat_le(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { rat_rel(c, s, a[0], s::le, |o| o.is_le()) }
fn rat_gt(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { rat_rel(c, s, a[0], s::gt, |o| o.is_gt()) }
fn rat_ge(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { rat_rel(c, s, a[0], s::ge, |o| o.is_ge()) }
fn rat_eq(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    let (n, d) = rat_parts(s);
    if is_integer(o) { return Ok(Value::bool(d == Value::fix(1) && icmp(n, o) == Ordering::Equal)); }
    if is_rational(o) { let (on, od) = rat_parts(o); return Ok(Value::bool(icmp(n, on).is_eq() && icmp(d, od).is_eq())); }
    if is_float(o) { return Ok(Value::bool(rat_to_f(s) == float_val(o))); }
    let r = funcall(c, o, s::eq, &[s], std::ptr::null())?;
    Ok(Value::bool(r.truthy()))
}
fn rat_coerce(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if is_integer(o) { return Ok(ary_new_from(&[rat_raw(o, Value::fix(1)), s])); }
    if is_float(o) { return Ok(ary_new_from(&[o, float_new(rat_to_f(s))])); }
    if is_rational(o) { return Ok(ary_new_from(&[o, s])); }
    if crate::builtins::complex::is_complex(o) {
        let (re, im) = crate::builtins::complex::cpx_parts(o);
        if im == Value::fix(0) || (is_rational(im) && iszero(rat_parts(im).0)) {
            let r = if is_integer(re) { rat_raw(re, Value::fix(1)) } else { re };
            if is_rational(r) { return Ok(ary_new_from(&[r, s])); }
        }
        return Ok(ary_new_from(&[o, crate::builtins::complex::cpx_raw(s, Value::fix(0))]));
    }
    type_err(c, format!("{} can't be coerced into Rational", class_name_for_err(o)))
}

// ---- rounding -------------------------------------------------------------------------------

fn r_floor(n: Value, d: Value) -> Value { idiv(n, d) }
fn r_ceil(n: Value, d: Value) -> Value { ineg(idiv(ineg(n), d)) }
fn r_trunc(n: Value, d: Value) -> Value { if isneg(n) { r_ceil(n, d) } else { r_floor(n, d) } }
fn r_round(n: Value, d: Value, half: u8) -> Value {
    let neg = isneg(n);
    let an = iabs(n);
    let two = Value::fix(2);
    let q = match half {
        2 => { let x = isub(imul(an, two), d); let dd = imul(d, two); idiv(iadd(x, isub(dd, Value::fix(1))), dd) }
        1 => {
            let x = iadd(imul(an, two), d); let dd = imul(d, two);
            let q = idiv(x, dd);
            if iszero(imod(x, dd)) && iodd(q) { isub(q, Value::fix(1)) } else { q }
        }
        _ => { let x = iadd(imul(an, two), d); idiv(x, imul(d, two)) }
    };
    if neg { ineg(q) } else { q }
}
fn rat_round_common(c: &mut Ctx, s: Value, a: &[Value], mode: u8) -> RV {
    let half = if mode == b'r' { round_half_mode(c, a)? } else { 0 };
    let pos: Vec<Value> = a.iter().copied().filter(|v| !is_hash(*v)).collect();
    let f = |n: Value, d: Value| -> Value { match mode { b'f' => r_floor(n, d), b'c' => r_ceil(n, d), b't' => r_trunc(n, d), _ => r_round(n, d, half) } };
    if pos.len() > 1 { return arg_err(c, format!("wrong number of arguments (given {}, expected 0..1)", pos.len())); }
    let (n, d) = rat_parts(s);
    if pos.is_empty() { return Ok(f(n, d)); }
    let nd = pos[0];
    if !is_integer(nd) { return type_err(c, "not an integer"); }
    if !nd.is_fixnum() { return if isneg(nd) { Ok(Value::fix(0)) } else { Ok(s) }; }
    Ok(rat_round_nd(s, nd.as_fix(), mode, half))
}
/// round/floor/ceil/truncate (mode r/f/c/t) of a Rational to k decimal digits
pub fn rat_round_nd(s: Value, k: i64, mode: u8, half: u8) -> Value {
    let f = |n: Value, d: Value| -> Value { match mode { b'f' => r_floor(n, d), b'c' => r_ceil(n, d), b't' => r_trunc(n, d), _ => r_round(n, d, half) } };
    let (n, d) = rat_parts(s);
    let p = ipow(Value::fix(10), k.unsigned_abs());
    let sc = if k >= 0 { mul_parts((n, d), (p, Value::fix(1))) } else { mul_parts((n, d), (Value::fix(1), p)) };
    let (sn, sd) = rat_parts(sc);
    let r = f(sn, sd);
    let res = if k >= 0 { rat_new(r, p) } else { rat_raw(imul(r, p), Value::fix(1)) };
    if k < 1 { let (rn, rd) = rat_parts(res); return r_trunc(rn, rd); }
    res
}
fn rat_floor(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { rat_round_common(c, s, a, b'f') }
fn rat_ceil(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { rat_round_common(c, s, a, b'c') }
fn rat_truncate(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { rat_round_common(c, s, a, b't') }
fn rat_round(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { rat_round_common(c, s, a, b'r') }

// ---- conversions ----------------------------------------------------------------------------

fn rat_to_s(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (n, d) = rat_parts(s);
    let v = str_new(format!("{}/{}", istr(n), istr(d)).as_bytes()); rstr(v).enc = crate::object::ENC_ASCII; Ok(v)
}
fn rat_inspect(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (n, d) = rat_parts(s);
    let v = str_new(format!("({}/{})", istr(n), istr(d)).as_bytes()); rstr(v).enc = crate::object::ENC_ASCII; Ok(v)
}
fn rat_to_f_m(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(float_new(rat_to_f(s))) }
fn rat_to_i(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let (n, d) = rat_parts(s); Ok(r_trunc(n, d)) }
fn rat_to_r(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(s) }
fn rat_num(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(rat_parts(s).0) }
fn rat_den(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(rat_parts(s).1) }
fn rat_zero_p(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(iszero(rat_parts(s).0))) }
fn rat_pos_p(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let n = rat_parts(s).0; Ok(Value::bool(!isneg(n) && !iszero(n))) }
fn rat_neg_p(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(isneg(rat_parts(s).0))) }
fn rat_hash(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (n, d) = rat_parts(s);
    let h = crate::astrust::hash_merge(crate::hash::key_hash(c, n)?, crate::hash::key_hash(c, d)? ^ 0x5241);
    Ok(Value::fix((h >> 2) as i64))
}

/// Simplest rational within [a, b] (a <= b), both as (num, den) with positive den.
fn simplest_between(a: (Value, Value), b: (Value, Value)) -> (Value, Value) {
    let (mut a, mut b) = (a, b);
    let (mut p0, mut p1, mut q0, mut q1) = (Value::fix(0), Value::fix(1), Value::fix(1), Value::fix(0));
    let one = Value::fix(1);
    let c;
    loop {
        let cc = r_ceil(a.0, a.1);
        // cc < b ?
        if icmp(imul(cc, b.1), b.0) == Ordering::Less { c = cc; break; }
        let k = isub(cc, one);
        let p2 = iadd(imul(k, p1), p0);
        let q2 = iadd(imul(k, q1), q0);
        // t = 1/(b - k); b = 1/(a - k); a = t
        let bk = (isub(b.0, imul(k, b.1)), b.1);
        let ak = (isub(a.0, imul(k, a.1)), a.1);
        let t = (bk.1, bk.0);
        b = (ak.1, ak.0);
        a = t;
        p0 = p1; q0 = q1; p1 = p2; q1 = q2;
    }
    (iadd(imul(c, p1), p0), iadd(imul(c, q1), q0))
}
fn norm_pair(n: Value, d: Value) -> (Value, Value) { let r = rat_new(n, d); rat_parts(r) }
fn to_exact_pair(c: &mut Ctx, v: Value) -> Result<(Value, Value), Esc> {
    if let Some(p) = exact_parts(v) { return Ok(p); }
    if is_float(v) { let f = float_val(v); if !f.is_finite() { return err(c, vm().c_float_domain_error, float_to_s(f)); } return Ok(float_parts(f)); }
    let r = funcall(c, v, intern(b"to_r"), &[], std::ptr::null())?;
    match exact_parts(r) { Some(p) => Ok(p), None => type_err(c, format!("can't convert {} into Rational", class_name_for_err(v))) }
}
fn rationalize_with(c: &mut Ctx, x: (Value, Value), e: Value) -> RV {
    let e = to_exact_pair(c, e)?;
    let e = (iabs(e.0), e.1);
    let neg = isneg(x.0);
    let xa = (iabs(x.0), x.1);
    let a = norm_pair(isub(imul(xa.0, e.1), imul(e.0, xa.1)), imul(xa.1, e.1));
    let b = norm_pair(iadd(imul(xa.0, e.1), imul(e.0, xa.1)), imul(xa.1, e.1));
    if icmp(a.0, b.0).is_eq() && icmp(a.1, b.1).is_eq() { return Ok(rat_new(x.0, x.1)); }
    let (p, q) = simplest_between(a, b);
    Ok(rat_new(if neg { ineg(p) } else { p }, q))
}
fn rat_rationalize(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    if a.is_empty() { return Ok(s); }
    rationalize_with(c, rat_parts(s), a[0])
}
fn flo_to_r(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { float_to_r(c, float_val(s)) }
fn flo_rationalize(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    let f = float_val(s);
    if !f.is_finite() { return err(c, vm().c_float_domain_error, float_to_s(f)); }
    if !a.is_empty() { return rationalize_with(c, float_parts(f), a[0]); }
    if f == 0.0 { return Ok(rat_raw(Value::fix(0), Value::fix(1))); }
    // f = m * 2^e with a 53-bit mantissa
    let (fr, ex) = frexp(f.abs());
    let m = (fr * 9007199254740992.0) as i64; let e = ex - 53;
    if e >= 0 { let r = rat_raw(big_to_value(BigInt::from_i64(m).shl(e as usize)), Value::fix(1)); return Ok(if f < 0.0 { rat_raw(ineg(rat_parts(r).0), Value::fix(1)) } else { r }); }
    let den = big_to_value(BigInt::from_i64(1).shl((1 - e) as usize));
    let a2 = norm_pair(Value::fix(2 * m - 1), den);
    let b2 = norm_pair(Value::fix(2 * m + 1), den);
    let (p, q) = simplest_between(a2, b2);
    Ok(rat_new(if f < 0.0 { ineg(p) } else { p }, q))
}
pub fn frexp(f: f64) -> (f64, i64) {
    if f == 0.0 || !f.is_finite() { return (f, 0); }
    let bits = f.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i64;
    if exp == 0 { let (m, e) = frexp(f * 2f64.powi(64)); return (m, e - 64); }
    let m = f64::from_bits((bits & !(0x7ffu64 << 52)) | (1022u64 << 52));
    (m, exp - 1022)
}
fn flo_numerator(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let f = float_val(s);
    if f.is_nan() || f.is_infinite() { return Ok(s); }
    let r = float_to_r(c, f)?; Ok(rat_parts(r).0)
}
fn flo_denominator(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let f = float_val(s);
    if f.is_nan() { return Ok(s); }
    if f.is_infinite() { return Ok(float_new(1.0)); }
    let r = float_to_r(c, f)?; Ok(rat_parts(r).1)
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
        if b[*i] == b'-' || b[*i] == b'+' { neg = b[*i] == b'-'; *i += 1; }
        match read_digits(b, i) {
            None => { *i = save; return Some((n, nexp)); }
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
pub fn parse_rational(c: &mut Ctx, b: &[u8], strict: bool) -> Result<Option<Value>, Esc> {
    let mut i = skip_ws(b, 0);
    let mut neg = false;
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') { neg = b[i] == b'-'; i += 1; }
    let Some((mut num, mut e)) = read_num(b, &mut i) else { return Ok(None) };
    let mut den = BigInt::from_i64(1);
    if i < b.len() && b[i] == b'/' {
        i += 1;
        let Some((dn, de)) = read_num(b, &mut i) else { return Ok(None) };
        if dn.is_zero() { return zerodiv(c); }
        if strict && skip_ws(b, i) != b.len() { return Ok(None); }
        den = dn; e -= de;
    } else if strict && skip_ws(b, i) != b.len() { return Ok(None); }
    if e > 0 {
        if e > FIXNUM_LIM { return Ok(Some(float_new(if neg { f64::NEG_INFINITY } else { f64::INFINITY }))); }
        num = num.mul(&pow10(c, e)?);
    } else if e < 0 {
        if -e > FIXNUM_LIM { return Ok(Some(float_new(if neg { -0.0 } else { 0.0 }))); }
        den = den.mul(&pow10(c, -e)?);
    }
    if neg { num = num.neg(); }
    Ok(Some(rat_new(big_to_value(num), big_to_value(den))))
}
fn str_to_r(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let b = str_bytes(s).to_vec();
    match parse_rational(c, &b, false)? {
        None => Ok(rat_raw(Value::fix(0), Value::fix(1))),
        Some(v) if is_float(v) => { if float_val(v).is_infinite() { return err(c, vm().c_float_domain_error, "Infinity"); } Ok(v) }
        Some(v) => Ok(v),
    }
}

// ---- Kernel#Rational ------------------------------------------------------------------------

fn exact_zero(v: Value) -> bool { v == Value::fix(0) || (is_rational(v) && iszero(rat_parts(v).0)) }
fn exact_one(v: Value) -> bool { v == Value::fix(1) || (is_rational(v) && rat_parts(v) == (Value::fix(1), Value::fix(1))) }

fn conv_arg(c: &mut Ctx, v: Value, raise: bool) -> Result<Option<Value>, Esc> {
    let mut v = v;
    if crate::builtins::complex::is_complex(v) { let (re, im) = crate::builtins::complex::cpx_parts(v); if exact_zero(im) { v = re; } }
    if is_float(v) { return Ok(Some(float_to_r(c, float_val(v))?)); }
    if is_string(v) {
        let b = str_bytes(v).to_vec();
        if b.contains(&0) { if raise { return arg_err(c, "string contains null byte"); } return Ok(None); }
        return match parse_rational(c, &b, true)? {
            Some(r) if is_float(r) => {
                if float_val(r) == 0.0 { return Ok(Some(rat_raw(Value::fix(0), Value::fix(1)))); }
                if !raise { return Ok(None); }
                err(c, vm().c_float_domain_error, "Infinity")
            }
            Some(r) => Ok(Some(r)),
            None => { if !raise { return Ok(None); } let ins = inspect_str(c, v)?; arg_err(c, format!("invalid value for convert(): {}", ins)) }
        };
    }
    if !is_kind_of(v, vm().c_numeric) && v.is_heap() {
        if respond_to(v, intern(b"to_r"), true) {
            let r = funcall(c, v, intern(b"to_r"), &[], std::ptr::null())?;
            if is_rational(r) { return Ok(Some(r)); }
            let cn = class_name_for_err(v);
            return type_err(c, format!("can't convert {} to Rational ({}#to_r gives {})", cn, cn, class_name_for_err(r)));
        }
        if respond_to(v, s::to_int, true) {
            let r = funcall(c, v, s::to_int, &[], std::ptr::null())?;
            if is_integer(r) { return Ok(Some(r)); }
        }
    }
    Ok(Some(v))
}
fn int_value(c: &mut Ctx, v: Value) -> RV {
    if is_integer(v) { return Ok(v); }
    if !is_numeric(v) && !is_rational(v) && v.is_heap() && respond_to(v, s::to_int, true) {
        let r = funcall(c, v, s::to_int, &[], std::ptr::null())?; if is_integer(r) { return Ok(r); }
    }
    type_err(c, "not an integer")
}
pub fn convert_rational(c: &mut Ctx, a1: Value, a2: Option<Value>, raise: bool) -> RV {
    if a1 == NIL || a2 == Some(NIL) { if !raise { return Ok(NIL); } return type_err(c, "can't convert nil into Rational"); }
    let Some(a1) = conv_arg(c, a1, raise)? else { return Ok(NIL) };
    let a2 = match a2 { None => None, Some(v) => match conv_arg(c, v, raise)? { Some(x) => Some(x), None => return Ok(NIL) } };
    if is_rational(a1) && a2.map(exact_one).unwrap_or(true) { return Ok(a1); }
    let Some(a2) = a2 else {
        if is_integer(a1) { return Ok(rat_raw(a1, Value::fix(1))); }
        if !raise {
            if !respond_to(a1, intern(b"to_r"), true) { return Ok(NIL); }
            return match funcall(c, a1, intern(b"to_r"), &[], std::ptr::null()) { Ok(r) if is_rational(r) => Ok(r), Ok(_) => Ok(NIL), Err(_) => { c.esc = EscKind::None; Ok(NIL) } };
        }
        if a1.is_heap() && respond_to(a1, intern(b"to_r"), true) {
            let r = funcall(c, a1, intern(b"to_r"), &[], std::ptr::null())?;
            if is_rational(r) { return Ok(r); }
            let cn = class_name_for_err(a1);
            return type_err(c, format!("can't convert {} to Rational ({}#to_r gives {})", cn, cn, class_name_for_err(r)));
        }
        return type_err(c, format!("can't convert {} into Rational", class_name_for_err(a1)));
    };
    let numeric = |v: Value| is_kind_of(v, vm().c_numeric);
    if numeric(a1) && numeric(a2) && (!is_integer(a1) || !is_integer(a2)) {
        let x = if is_integer(a1) { rat_raw(a1, Value::fix(1)) } else { a1 };
        return funcall(c, x, s::div, &[a2], std::ptr::null());
    }
    let r: RV = (|| { let n = int_value(c, a1)?; let d = int_value(c, a2)?; rat_new_checked(c, n, d) })();
    match r { Err(e) => { if !raise && c.esc == EscKind::Raise { c.esc = EscKind::None; return Ok(NIL); } Err(e) } ok => ok }
}
/// Split off a trailing `exception:` keyword.
pub fn kw_exception(c: &mut Ctx, a: &[Value]) -> Result<(usize, bool), Esc> {
    if let Some(h) = kw_arg(a) {
        if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("exception"))? {
            if v != TRUE && v != FALSE { let ins = inspect_str(c, v)?; return arg_err(c, format!("expected true or false as exception: {}", ins)); }
            return Ok((a.len() - 1, v == TRUE));
        }
        return Ok((a.len() - 1, true));
    }
    Ok((a.len(), true))
}
fn k_rational(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (n, raise) = kw_exception(c, a)?;
    let a = &a[..n];
    check_argc(c, a, 1, 2)?;
    convert_rational(c, a[0], a.get(1).copied(), raise)
}

fn int_to_r(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(rat_raw(s, Value::fix(1))) }
fn int_rationalize(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { check_argc(c, a, 0, 1)?; Ok(rat_raw(s, Value::fix(1))) }
fn nil_to_r(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(rat_raw(Value::fix(0), Value::fix(1))) }
fn nil_rationalize(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { check_argc(c, a, 0, 1)?; Ok(rat_raw(Value::fix(0), Value::fix(1))) }

pub fn init() {
    let vm = vm();
    let k = define_class("Rational", vm.c_numeric);
    rclass(k).alloc = AllocKind::None;
    vm.c_rational = k;
    let sk = singleton_class(k);
    define_method_me(sk, s::new, new_me(s::new, sk, VISI_PUBLIC, MKind::Undef));
    let d = |nm: &str, f: CFunc, ar: i32| define_cfunc(k, nm, f, ar);
    d("+", rat_plus, 1); d("-", rat_minus, 1); d("*", rat_mul, 1); d("/", rat_div, 1); d("quo", rat_div, 1);
    d("fdiv", rat_fdiv, 1); d("**", rat_pow, 1); d("-@", rat_uminus, 0); d("abs", rat_abs, 0); d("magnitude", rat_abs, 0);
    d("<=>", rat_cmp, 1); d("==", rat_eq, 1); d("<", rat_lt, 1); d("<=", rat_le, 1); d(">", rat_gt, 1); d(">=", rat_ge, 1);
    d("coerce", rat_coerce, 1);
    d("floor", rat_floor, -1); d("ceil", rat_ceil, -1); d("truncate", rat_truncate, -1); d("round", rat_round, -1);
    d("to_s", rat_to_s, 0); d("inspect", rat_inspect, 0); d("to_f", rat_to_f_m, 0); d("to_i", rat_to_i, 0); d("to_r", rat_to_r, 0);
    d("numerator", rat_num, 0); d("denominator", rat_den, 0); d("zero?", rat_zero_p, 0); d("positive?", rat_pos_p, 0);
    d("negative?", rat_neg_p, 0); d("hash", rat_hash, 0); d("rationalize", rat_rationalize, -1);
    define_cfunc(vm.c_float, "to_r", flo_to_r, 0);
    define_cfunc(vm.c_float, "rationalize", flo_rationalize, -1);
    define_cfunc(vm.c_float, "numerator", flo_numerator, 0);
    define_cfunc(vm.c_float, "denominator", flo_denominator, 0);
    define_cfunc(vm.c_integer, "to_r", int_to_r, 0);
    define_cfunc(vm.c_integer, "rationalize", int_rationalize, -1);
    define_cfunc(vm.c_nil, "to_r", nil_to_r, 0);
    define_cfunc(vm.c_nil, "rationalize", nil_rationalize, -1);
    define_cfunc(vm.c_string, "to_r", str_to_r, 0);
    define_module_function(vm.m_kernel, "Rational", k_rational, -1);
}
