// Numeric / Integer / Float, on the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::array::*;
use crate::bignum::*;
use crate::builtins::kernel::*;
use crate::numeric::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

pub fn f64_to_int(c: &mut Ctx, f: f64) -> Value {
    let t = f.trunc();
    if t.abs() < 4.0e18 { c.int_from_i64(t as i64) } else { c.big_to_value(BigInt::from_f64(t)) }
}

/// Integer-like argument to i64 (to_int conversion; Float truncated).
pub fn num_to_long(c: &mut Ctx, v: Value) -> Result<i64, Esc> {
    if v.is_fixnum() { return Ok(v.as_fix()); }
    if is_bignum(v) { return match c.big(v).to_i64() { Some(i) => Ok(i), None => err(c, c.vm().c_range_error.get(), format!("bignum too big to convert into 'long'")) }; }
    if is_float(v) {
        let f = float_val(v);
        if f.is_nan() || f.is_infinite() { return err(c, c.vm().c_float_domain_error.get(), float_to_s(f)); }
        // FIXABLE range of rb_num2long: -2**63 <= f < 2**63
        if !(f >= -9223372036854775808.0 && f < 9223372036854775808.0) { return err(c, c.vm().c_range_error.get(), format!("float {} out of range of integer", crate::builtins::sprintf::fmt_g10(f))); }
        return Ok(f as i64);
    }
    if v == NIL { return type_err(c, "no implicit conversion of nil into Integer"); }
    if let Some(r) = if v.is_heap() { crate::string::check_funcall(c, v, s::to_int)? } else { None } {
        if r.is_fixnum() { return Ok(r.as_fix()); }
        if is_bignum(r) { return num_to_long(c, r); }
        let cls = class_name_for_err(c, v);
        return type_err(c, format!("can't convert {} to Integer ({}#to_int gives {})", cls, cls, class_name_for_err(c, r)));
    }
    type_err(c, format!("no implicit conversion of {} into Integer", class_name_for_err(c, v)))
}

/// rb_to_int: an Integer as is, else #to_int, which must give an Integer.
pub fn rb_to_int(c: &mut Ctx, v: Value) -> RV {
    if is_integer(v) { return Ok(v); }
    if is_float(v) { let f = float_val(v); check_finite(c, f)?; return Ok(f64_to_int(c, f)); }
    if let Some(r) = if v.is_heap() { crate::string::check_funcall(c, v, s::to_int)? } else { None } {
        if is_integer(r) { return Ok(r); }
        let cls = class_name_for_err(c, v);
        return type_err(c, format!("can't convert {} to Integer ({}#to_int gives {})", cls, cls, class_name_for_err(c, r)));
    }
    type_err(c, format!("no implicit conversion of {} into Integer", class_name_for_err(c, v)))
}
/// NUM2INT: an `int` argument (ndigits).
pub fn num_to_int32(c: &mut Ctx, v: Value) -> Result<i64, Esc> {
    let n = num_to_long(c, v)?;
    if n < i32::MIN as i64 || n > i32::MAX as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too {} to convert to 'int'", n, if n < 0 { "small" } else { "big" })); }
    Ok(n)
}

pub fn to_integer(c: &mut Ctx, v: Value) -> RV {
    if is_integer(v) { return Ok(v); }
    if is_float(v) { return Ok(f64_to_int(c, float_val(v))); }
    if v.is_heap() && respond_to(c, v, s::to_int, true) {
        let r = funcall(c, v, s::to_int, &[], std::ptr::null())?;
        if is_integer(r) { return Ok(r); }
    }
    type_err(c, format!("no implicit conversion of {} into Integer", class_name_for_err(c, v)))
}

/// rb_num2dbl.
pub fn num2dbl(c: &mut Ctx, v: Value) -> Result<f64, Esc> {
    if v.is_fixnum() { return Ok(v.as_fix() as f64); }
    if is_float(v) { return Ok(float_val(v)); }
    if is_bignum(v) { return Ok(c.big(v).to_f64()); }
    if v == NIL || v == TRUE || v == FALSE { return type_err(c, format!("no implicit conversion to float from {}", class_name_for_err(c, v))); }
    if is_string(v) { return type_err(c, "no implicit conversion of String into Float"); }   // master (4.0: "... to float from string")
    if respond_to(c, v, s::to_f, true) {
        let r = funcall(c, v, s::to_f, &[], std::ptr::null())?;
        if is_float(r) { return Ok(float_val(r)); }
        let k = class_name_for_err(c, v);
        return type_err(c, format!("can't convert {} to Float ({}#to_f gives {})", k, k, class_name_for_err(c, r)));
    }
    type_err(c, format!("can't convert {} into Float", class_name_for_err(c, v)))
}
pub fn to_f(c: &mut Ctx, v: Value) -> Result<f64, Esc> {
    if let Some(f) = c.num_to_f64(v) { return Ok(f); }
    if v == NIL { return type_err(c, "can't convert nil into Float"); }
    if v.is_heap() && respond_to(c, v, s::to_f, true) && !is_string(v) {
        let r = funcall(c, v, s::to_f, &[], std::ptr::null())?;
        if let Some(f) = c.num_to_f64(r) { return Ok(f); }
    }
    type_err(c, format!("can't convert {} into Float", class_name_for_err(c, v)))
}

fn coerce_fail<T>(c: &mut Ctx, a: Value, b: Value, cmp: bool) -> Result<T, Esc> {
    let bdesc = if b == NIL || b == TRUE || b == FALSE { class_name_for_err(c, b) } else if is_float(b) || b.is_fixnum() || b.is_sym() { inspect_str(c, b)? } else { class_path(c, real_class_of(c, b)) };
    // rb_cmperr: special constants and Floats by inspect, others by class
    if cmp { let bs = if !b.is_heap() || is_float(b) { inspect_str(c, b).unwrap_or(bdesc) } else { class_path(c, real_class_of(c, b)) }; return arg_err(c, format!("comparison of {} with {} failed: coercion was not possible", class_path(c, real_class_of(c, a)), bs)); }
    type_err(c, format!("{} can't be coerced into {}", bdesc, class_path(c, real_class_of(c, a))))
}

/// a op b via b.coerce(a)
pub fn coerce_bin(c: &mut Ctx, a: Value, b: Value, op: Sym, cmp: bool) -> RV {
    if b.is_heap() && respond_to(c, b, s::coerce, true) && !is_string(b) {
        let r = funcall(c, b, s::coerce, &[a], std::ptr::null())?;
        if cmp && r == NIL { return coerce_fail(c, a, b, true); }
        if !is_array(r) || c.ary_len(r) != 2 { return type_err(c, "coerce must return [x, y]"); }
        let x = c.ary_get(r, 0); let y = c.ary_get(r, 1);
        return funcall(c, x, op, &[y], std::ptr::null());
    }
    coerce_fail(c, a, b, cmp)
}

#[derive(Clone, Copy, PartialEq)]
enum Op { Add, Sub, Mul, Div, Mod, Pow }

fn int_arith(c: &mut Ctx, a: Value, b: Value, op: Op) -> RV {
    if a.is_fixnum() && b.is_fixnum() {
        let (x, y) = (a.as_fix() as i128, b.as_fix() as i128);
        let r = match op {
            Op::Add => x + y, Op::Sub => x - y, Op::Mul => x * y,
            Op::Div => { if y == 0 { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); } let q = x / y; if (x % y != 0) && ((x < 0) != (y < 0)) { q - 1 } else { q } }
            Op::Mod => { if y == 0 { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); } let m = x % y; if m != 0 && ((m < 0) != (y < 0)) { m + y } else { m } }
            Op::Pow => return int_pow(c, a, b),
        };
        return Ok(c.int_from_i128(r));
    }
    if op == Op::Mul {
        // Bignum * one-limb Integer (either order)
        let (big, small) = if is_bignum(a) && b.is_fixnum() { (a, b.as_fix()) } else if a.is_fixnum() && is_bignum(b) { (b, a.as_fix()) } else { (NIL, 0) };
        if big != NIL && small != 0 && small.unsigned_abs() <= u32::MAX as u64 {
            return Ok(c.big_to_value(c.big(big).mul_small(small.unsigned_abs() as u32, small < 0)));
        }
    }
    if is_integer(b) {
        let (x, y) = (c.to_big(a), c.to_big(b));
        return Ok(match op {
            Op::Add => c.big_to_value(x.add(&y)), Op::Sub => c.big_to_value(x.sub(&y)), Op::Mul => c.big_to_value(x.mul(&y)),
            Op::Div => { if y.is_zero() { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); } c.big_to_value(x.divmod(&y).0) }
            Op::Mod => { if y.is_zero() { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); } c.big_to_value(x.divmod(&y).1) }
            Op::Pow => return int_pow(c, a, b),
        });
    }
    if is_float(b) {
        if op == Op::Pow { return int_pow(c, a, b); }
        let x = c.num_to_f64(a).unwrap(); let y = float_val(b);
        if op == Op::Mod && y == 0.0 { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); }
        return Ok(c.float_new(match op { Op::Add => x + y, Op::Sub => x - y, Op::Mul => x * y, Op::Div => x / y, Op::Mod => fmod_ruby(x, y), Op::Pow => unreachable!() }));
    }
    // fix_plus / fix_mul compute Complex op Fixnum directly (keeps exact parts)
    if a.is_fixnum() && (op == Op::Add || op == Op::Mul) && crate::builtins::complex::is_complex(c, b) {
        return funcall(c, b, if op == Op::Add { s::plus } else { s::mul }, &[a], std::ptr::null());
    }
    let sym = match op { Op::Add => s::plus, Op::Sub => s::minus, Op::Mul => s::mul, Op::Div => s::div, Op::Mod => s::modulo, Op::Pow => s::pow };
    coerce_bin(c, a, b, sym, false)
}

pub fn fmod_ruby(x: f64, y: f64) -> f64 {
    if y.is_infinite() && x.is_finite() { if x == 0.0 || (x > 0.0) == (y > 0.0) { return x; } return y; }
    let m = x % y;
    if m != 0.0 && ((m < 0.0) != (y < 0.0)) { m + y } else { m }
}

fn int_pow(c: &mut Ctx, a: Value, b: Value) -> RV {
    if is_float(b) {
        let x = c.num_to_f64(a).unwrap(); let y = float_val(b);
        if y == 0.0 { return Ok(c.float_new(1.0)); }
        if x < 0.0 && y != y.round() { return Ok(crate::builtins::complex::polar_pi(c, (-x).powf(y), y)); }
        return Ok(c.float_new(x.powf(y)));
    }
    if !is_integer(b) { return coerce_bin(c, a, b, s::pow, false); }
    let e = c.to_big(b);
    if e.neg && (a == Value::fix(1) || a == Value::fix(-1)) { return Ok(Value::fix(if a == Value::fix(-1) && e.is_odd() { -1 } else { 1 })); }
    if e.neg {
        if let Some(_) = const_get_at(c, c.vm().c_object.get(), intern(b"Rational")) {
            let r = funcall(c, c.vm().main_obj.get(), intern(b"Rational"), &[a, Value::fix(1)], std::ptr::null())?;
            return funcall(c, r, s::pow, &[b], std::ptr::null());
        }
        return Ok(c.float_new(c.num_to_f64(a).unwrap().powf(c.num_to_f64(b).unwrap())));
    }
    let x = c.to_big(a);
    if x.mag.len() <= 1 && x.mag.first().copied().unwrap_or(0) <= 1 {
        if x.is_zero() { return Ok(Value::fix(if e.is_zero() { 1 } else { 0 })); }
        if !x.neg { return Ok(Value::fix(1)); }
        return Ok(Value::fix(if e.is_odd() { -1 } else { 1 }));
    }
    let ei = match e.to_i64() { Some(i) => i as u64, None => { return arg_err(c, "exponent is too large"); } };
    if (x.bit_length() as u64).saturating_mul(ei) > 1u64 << 34 {
        return arg_err(c, "exponent is too large");
    }
    if a.is_fixnum() {
        if let Ok(ei32) = u32::try_from(ei) { if let Some(r) = a.as_fix().checked_pow(ei32) { return Ok(c.int_from_i64(r)); } }
    }
    Ok(c.big_to_value(x.pow(ei)))
}

fn int_cmp_val(c: &mut Ctx, a: Value, b: Value) -> Result<Option<std::cmp::Ordering>, Esc> {
    if a.is_fixnum() && b.is_fixnum() { return Ok(Some(a.as_fix().cmp(&b.as_fix()))); }
    if is_integer(b) { return Ok(Some(c.to_big(a).cmp(&c.to_big(b)))); }
    if is_float(b) {
        let y = float_val(b);
        if y.is_nan() { return Ok(None); }
        if y.is_infinite() { return Ok(Some(if y > 0.0 { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater })); }
        let x = c.num_to_f64(a).unwrap();
        if x != y { return Ok(x.partial_cmp(&y)); }
        // exact comparison for big values
        return Ok(Some(c.to_big(a).cmp(&BigInt::from_f64(y)).then_with(|| if y.fract() > 0.0 { std::cmp::Ordering::Less } else if y.fract() < 0.0 { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Equal })));
    }
    let _ = c;
    Ok(None)
}

macro_rules! cfn { ($(#[$m:meta])* $name:ident, |$c:ident, $s:ident, $a:ident| $body:expr) => {
    $(#[$m])* #[allow(unused_variables)]
    pub fn $name($c: &mut Ctx, $s: Value, $a: &[Value], _b: Blk) -> RV { $body }
}; }

cfn!(int_plus, |c, s, a| int_arith(c, s, a[0], Op::Add));
cfn!(int_minus, |c, s, a| int_arith(c, s, a[0], Op::Sub));
cfn!(int_mul, |c, s, a| int_arith(c, s, a[0], Op::Mul));
cfn!(int_div, |c, s, a| int_arith(c, s, a[0], Op::Div));
cfn!(int_mod, |c, s, a| int_arith(c, s, a[0], Op::Mod));
cfn!(int_pow_m, |c, s, a| { check_argc(c, a, 1, 2)?;
    if a.len() == 2 {
        // modular exponentiation
        if !is_integer(a[0]) { return type_err(c, "Integer#pow() 2nd argument not allowed unless a 1st argument is integer"); }
        let mut e = c.to_big(a[0]);
        if e.neg { return err(c, c.vm().c_range_error.get(), "Integer#pow() 1st argument cannot be negative when 2nd argument specified"); }
        if !is_integer(a[1]) { return type_err(c, "Integer#pow() 2nd argument not allowed unless all arguments are integers"); }
        let m = c.to_big(a[1]);
        // bignum.c: 0**x is 0 (x != 0) before the modulus is looked at
        if s == Value::fix(0) && !e.is_zero() { return Ok(Value::fix(0)); }
        if m.is_zero() { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); }
        let mut base = c.to_big(s).divmod(&m).1;
        let mut r = BigInt::from_i64(1).divmod(&m).1;
        while !e.is_zero() {
            if e.is_odd() { r = r.mul(&base).divmod(&m).1; }
            e = e.shr(1);
            base = base.mul(&base).divmod(&m).1;
        }
        return Ok(c.big_to_value(r));
    }
    int_arith(c, s, a[0], Op::Pow)
});
cfn!(#[inline] int_uminus, |c, s, _a| Ok(if s.is_fixnum() { c.int_from_i128(-(s.as_fix() as i128)) } else { c.big_to_value(c.big(s).neg()) }));
cfn!(int_not, |c, s, _a| Ok(if s.is_fixnum() { Value::fix(!s.as_fix()) } else { c.big_to_value(c.big(s).not()) }));
fn int_bitop(c: &mut Ctx, a: Value, b: Value, op: u8) -> RV {
    if a.is_fixnum() && b.is_fixnum() {
        let (x, y) = (a.as_fix(), b.as_fix());
        return Ok(Value::fix(match op { b'&' => x & y, b'|' => x | y, _ => x ^ y }));
    }
    if is_integer(b) { return Ok(c.big_to_value(c.to_big(a).bitop(&c.to_big(b), op))); }
    let sym = match op { b'&' => s::and, b'|' => s::or, _ => s::xor };
    if b.is_heap() && respond_to(c, b, s::coerce, true) && !is_string(b) {
        let r = funcall(c, b, s::coerce, &[a], std::ptr::null())?;
        if !is_array(r) || c.ary_len(r) != 2 { return type_err(c, "coerce must return [x, y]"); }
        let (x, y) = (c.ary_get(r, 0), c.ary_get(r, 1));
        if is_integer(x) && is_integer(y) { return int_bitop(c, x, y, op); }
        if !is_integer(y) { return coerce_fail(c, a, y, false); }
        return funcall(c, x, sym, &[y], std::ptr::null());
    }
    coerce_fail(c, a, b, false)
}
cfn!(int_and, |c, s, a| int_bitop(c, s, a[0], b'&'));
cfn!(int_or, |c, s, a| int_bitop(c, s, a[0], b'|'));
cfn!(int_xor, |c, s, a| int_bitop(c, s, a[0], b'^'));
fn int_shift(c: &mut Ctx, a: Value, n: i64) -> RV {
    if n >= 0 {
        if a.is_fixnum() && n < 62 {
            let x = a.as_fix() as i128;
            return Ok(c.int_from_i128(x << n));
        }
        if a == Value::fix(0) { return Ok(a); }
        // CRuby allocates the result; only a size malloc refuses is NoMemoryError (no fixed cap)
        let words = (n as u64 / 32 + 4) as usize;
        if Vec::<u32>::new().try_reserve_exact(words).is_err() { return err(c, c.vm().c_no_memory_error.get(), "failed to allocate memory"); }
        Ok(c.big_to_value(c.to_big(a).shl(n as usize)))
    } else {
        let m = (-n) as u64;
        if a.is_fixnum() { return Ok(Value::fix(if m >= 63 { if a.as_fix() < 0 { -1 } else { 0 } } else { a.as_fix() >> m })); }
        Ok(c.big_to_value(c.to_big(a).shr(m.min(1 << 30) as usize)))
    }
}
// a Bignum width: right gives 0 / -1, left of 0 gives 0, any other left cannot be represented
fn int_shift_big(c: &mut Ctx, a: Value, w: Value, left: bool) -> RV {
    let right = left == c.big(w).neg;
    let neg = if a.is_fixnum() { a.as_fix() < 0 } else { c.to_big(a).neg };
    if right { return Ok(Value::fix(if neg { -1 } else { 0 })); }
    if a == Value::fix(0) { return Ok(a); }
    // bignum.c big_shift2/3: RangeError only when the 64-bit digit count overflows; otherwise the allocation fails
    let bits = c.big(w).bit_length();
    let xn = if a.is_fixnum() { 1 } else { (c.big(a).bit_length() as u128 + 63) / 64 };
    let y = c.big(w).mag.iter().take(4).rev().fold(0u128, |acc, &d| (acc << 32) | d as u128);
    if bits >= 68 || (y >> 6) + xn >= 1u128 << 60 { return err(c, c.vm().c_range_error.get(), "shift width too big"); }
    err(c, c.vm().c_no_memory_error.get(), "failed to allocate memory")
}
fn shift_by(c: &mut Ctx, s: Value, w: Value, left: bool) -> RV {
    let w = if w.is_fixnum() { w } else { rb_to_int(c, w)? };
    if is_bignum(w) { return int_shift_big(c, s, w, left); }
    int_shift(c, s, if left { w.as_fix() } else { -w.as_fix() })
}
cfn!(int_lshift, |c, s, a| shift_by(c, s, a[0], true));
cfn!(int_rshift, |c, s, a| shift_by(c, s, a[0], false));
/// generate_mask: (1 << len) - 1
fn gen_mask(c: &mut Ctx, len: Value) -> RV { let one = shift_by(c, Value::fix(1), len, true)?; int_arith(c, one, Value::fix(1), Op::Sub) }
/// int_aref2 / rb_big_aref2
fn int_aref2(c: &mut Ctx, num: Value, beg: Value, len: Value) -> RV {
    if !is_bignum(num) {
        let sh = shift_by(c, num, beg, false)?;
        let m = gen_mask(c, len)?;
        return int_bitop(c, sh, m, b'&');
    }
    let (bv, lv) = (rb_to_int(c, beg)?, rb_to_int(c, len)?);
    for v in [bv, lv] { if is_bignum(v) && c.big(v).to_i64().is_none() { return err(c, c.vm().c_range_error.get(), "bignum too big to convert into 'long long'"); } }
    let (b, l) = (num_to_long(c, bv)?, num_to_long(c, lv)?);
    let Some(end) = b.checked_add(l) else { return err(c, c.vm().c_range_error.get(), "bignum too big to convert into 'long long'") };
    if l < 0 { return shift_by(c, num, bv, false); }
    if l == 0 || end <= 0 { return Ok(Value::fix(0)); }
    let sh = shift_by(c, num, bv, false)?;
    // a positive one has no bits above its length
    let bits = if c.big(num).neg { l } else { l.min((c.big(num).bit_length() as i64 + 1).saturating_sub(b)) };
    let m = gen_mask(c, Value::fix(bits))?;
    int_bitop(c, sh, m, b'&')
}
/// rb_fix_aref / rb_big_aref: one bit
fn int_bit(c: &mut Ctx, s: Value, idx: Value) -> RV {
    let idx = rb_to_int(c, idx)?;
    let sneg = if s.is_fixnum() { s.as_fix() < 0 } else { c.big(s).neg };
    if is_bignum(idx) { return Ok(Value::fix(if !c.big(idx).neg && sneg { 1 } else { 0 })); }
    let i = idx.as_fix();
    if i < 0 { return Ok(Value::fix(0)); }
    if s.is_fixnum() { return Ok(Value::fix(if i >= 63 { sneg as i64 } else { (s.as_fix() >> i) & 1 })); }
    Ok(Value::fix(c.big(s).bit(i as usize) as i64))
}
cfn!(int_aref, |c, s, a| {
    check_argc(c, a, 1, 2)?;
    if a.len() == 2 { return int_aref2(c, s, a[0], a[1]); }
    if a[0].is_fixnum() { return int_bit(c, s, a[0]); }
    if !is_range(a[0]) { return int_bit(c, s, a[0]); }
    let (b, e, x) = c.range_parts(a[0]);
    if b == NIL {
        if neg_int_p(c, e)? { return Ok(Value::fix(0)); }
        let e = if x { e } else { funcall(c, e, s::plus, &[Value::fix(1)], std::ptr::null())? };
        let m = gen_mask(c, e)?;
        if int_bitop(c, s, m, b'&')? == Value::fix(0) { return Ok(Value::fix(0)); }
        return arg_err(c, "The beginless range for Integer#[] results in infinity");
    }
    // compare_indexes: <=> (nil: incomparable)
    let cmp = if b.is_fixnum() && e.is_fixnum() { b.as_fix().cmp(&e.as_fix()) as i64 } else {
        let r = funcall(c, b, s::cmp, &[e], std::ptr::null())?;
        if r == NIL { i64::MAX } else { crate::builtins::array_methods::cmp_result(c, r, b, e)? }
    };
    if e != NIL && cmp < 0 {
        let mut len = funcall(c, e, s::minus, &[b], std::ptr::null())?;
        if !x { len = funcall(c, len, s::plus, &[Value::fix(1)], std::ptr::null())?; }
        return int_aref2(c, s, b, len);
    }
    if cmp == 0 { return if x { Ok(Value::fix(0)) } else { int_bit(c, s, b) }; }
    shift_by(c, s, b, false)
});
cfn!(int_eq, |c, s, a| {
    let b = a[0];
    if s.is_fixnum() && b.is_fixnum() { return Ok(Value::bool(s == b)); }
    if is_integer(b) { return Ok(Value::bool(c.to_big(s) == c.to_big(b))); }
    if is_float(b) { return Ok(Value::bool(int_cmp_val(c, s, b)? == Some(std::cmp::Ordering::Equal))); }
    let r = funcall(c, b, s::eq, &[s], std::ptr::null())?;
    Ok(Value::bool(r.truthy()))
});
cfn!(int_eql, |c, s, a| {
    // num_eql: rb_equal for fixnums, which calls a redefined Integer#==
    if s.is_fixnum() && a[0].is_fixnum() && s != a[0] && crate::rt::redef(crate::rt::R_INT_EQ) { return Ok(Value::bool(funcall(c, s, s::eq, &[a[0]], std::ptr::null())?.truthy())); }
    Ok(Value::bool(is_integer(a[0]) && c.to_big(s) == c.to_big(a[0])))
});
cfn!(int_cmp, |c, s, a| {
    match int_cmp_val(c, s, a[0])? {
        Some(o) => Ok(Value::fix(o as i64)),
        None => {
            if a[0].is_heap() && !is_string(a[0]) {
                // do_coerce(err = false): rb_check_funcall; nil is nil, an exception goes through
                let Some(r) = crate::string::check_funcall_args(c, a[0], s::coerce, &[s])? else { return Ok(NIL) };
                if r == NIL { return Ok(NIL); }
                if !is_array(r) || c.ary_len(r) != 2 { return type_err(c, "coerce must return [x, y]"); }
                let x = c.ary_get(r, 0); return funcall(c, x, s::cmp, &[c.ary_get(r, 1)], std::ptr::null());
            }
            Ok(NIL)
        }
    }
});
fn int_rel(c: &mut Ctx, s: Value, b: Value, f: fn(std::cmp::Ordering) -> bool, op: Sym) -> RV {
    match int_cmp_val(c, s, b)? {
        Some(o) => Ok(Value::bool(f(o))),
        None => if is_float(b) { Ok(FALSE) } else { coerce_bin(c, s, b, op, true) },
    }
}
cfn!(int_lt, |c, s, a| int_rel(c, s, a[0], |o| o.is_lt(), s::lt));
cfn!(int_le, |c, s, a| int_rel(c, s, a[0], |o| o.is_le(), s::le));
cfn!(int_gt, |c, s, a| int_rel(c, s, a[0], |o| o.is_gt(), s::gt));
cfn!(int_ge, |c, s, a| int_rel(c, s, a[0], |o| o.is_ge(), s::ge));
cfn!(#[inline] int_abs, |c, s, _a| Ok(if s.is_fixnum() { c.int_from_i64(s.as_fix().abs()) } else { c.big_to_value(c.big(s).abs()) }));
cfn!(int_to_s, |c, s, a| { check_argc(c, a, 0, 1)?;
    let base = if a.is_empty() { 10 } else { num_to_int32(c, a[0])? };
    if !(2..=36).contains(&base) { return arg_err(c, format!("invalid radix {}", base)); }
    let st = if s.is_fixnum() && base == 10 { s.as_fix().to_string() } else { c.to_big(s).to_string_radix(base as u32) };
    let v = c.str_new(st.as_bytes());
    c.rstring_mut(v).enc = ENC_ASCII;
    Ok(v)
});
cfn!(#[inline] int_to_f, |c, s, _a| Ok(c.float_new(c.num_to_f64(s).unwrap())));
cfn!(#[inline] int_to_i, |c, s, _a| Ok(s));
/// rb_enc_uint_chr: the character `cp` in `enc`; Err(true): "out of char range", Err(false): "invalid codepoint".
pub fn enc_chr(enc: u8, cp: u64) -> Result<Vec<u8>, bool> {
    use crate::builtins::encoding::{self as en, EK};
    if cp > u32::MAX as u64 { return Err(true); }
    let cp = cp as u32;
    let sur = (0xd800..0xe000).contains(&cp);
    // 4.0.10: a single-byte code_to_mbclen rejects > 0xff as an invalid code point
    if en::is_dummy(enc) || en::is_single_byte(enc) {
        if cp > 0xff || (en::kind(enc) == EK::Ascii && cp > 0x7f) { return Err(false); }
        return Ok(vec![cp as u8]);
    }
    match en::kind(enc) {
        // Onigmo's code_to_mbclen: 0xfffffffe / 0xffffffff are one (invalid) byte in UTF-8
        EK::Utf8 => if cp >= 0xffff_fffe || sur { Err(false) } else if cp > 0x10ffff { Err(true) } else { en::code_to_bytes(enc, cp).ok_or(false) },
        EK::U16LE | EK::U16BE => if cp > 0x10ffff || sur { Err(false) } else { en::code_to_bytes(enc, cp).ok_or(false) },
        EK::Cesu8 => if sur { Err(false) } else if cp > 0x10ffff { Err(true) } else { en::code_to_bytes(enc, cp).ok_or(false) },
        EK::U32LE | EK::U32BE => {
            if cp > 0x10ffff || sur { return Err(false); }
            Ok(if en::kind(enc) == EK::U32LE { cp.to_le_bytes().to_vec() } else { cp.to_be_bytes().to_vec() })
        }
        k => {
            let max: u32 = match k { EK::Gb18030 | EK::EucTw | EK::EmacsMule => u32::MAX, EK::EucJp(_) => 0xff_ffff, _ => 0xffff };
            if cp > max { return Err(true); }
            en::code_to_bytes(enc, cp).ok_or(false)
        }
    }
}
cfn!(int_chr, |c, s, a| {
    if a.len() > 1 { return argc_err(c, a.len(), 0, 1); }
    let i = if s.is_fixnum() { s.as_fix() } else { -1 };
    let range = |c: &mut Ctx| -> RV { let n = if s.is_fixnum() { s.as_fix().to_string() } else { "bignum".into() }; err(c, c.vm().c_range_error.get(), format!("{} out of char range", n)) };
    if i < 0 { return range(c); }
    let enc = match a.first() {
        Some(&e) => crate::builtins::encoding::to_enc(c, e)?,
        None if i < 0x100 => return Ok(c.str_new_enc(&[i as u8], if i < 128 { ENC_ASCII } else { ENC_BINARY })),
        None => match crate::builtins::encoding::default_internal() { Some(e) => e, None => return range(c) },
    };
    match enc_chr(enc, i as u64) {
        Ok(b) => Ok(c.str_new_enc(&b, enc)),
        Err(true) => range(c),
        Err(false) => err(c, c.vm().c_range_error.get(), format!("invalid codepoint 0x{:X} in {}", i, crate::builtins::encoding::enc_name(enc))),
    }
});
cfn!(int_ord, |c, s, _a| Ok(s));
/// The fixnum loop of Integer#times, yielding through `y` (inline_iter.rs: a baked site's block); None: not a fixnum.
#[inline(always)]
pub fn int_times_loop(c: &mut Ctx, s: Value, y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> Option<RV> {
    if !s.is_fixnum() || crate::rt::redef(crate::rt::R_INT_LT | crate::rt::R_INT_SUCC) { return None; }
    let n = s.as_fix();
    let mut i = 0;
    while i < n { if let Err(e) = y(c, &[Value::fix(i)]) { return Some(Err(e)); } i += 1; }
    Some(Ok(s))
}
/// The fixnum loops of Integer#upto / #downto (dir 1 / -1); None: not two fixnums.
#[inline(always)]
pub fn int_upto_loop(c: &mut Ctx, s: Value, e: Value, dir: i64, y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> Option<RV> {
    if !s.is_fixnum() || !e.is_fixnum() { return None; }
    let mut i = s.as_fix(); let e = e.as_fix();
    while if dir > 0 { i <= e } else { i >= e } { if let Err(e) = y(c, &[Value::fix(i)]) { return Some(Err(e)); } i += dir; }
    Some(Ok(s))
}
fn int_times(c: &mut Ctx, s: Value, _a: &[Value], b: Blk) -> RV {
    if b.is_null() { return crate::builtins::enumerator::enum_for(c, s, s::times, &[], NIL); }
    if let Some(r) = int_times_loop(c, s, &mut |c, a| yield1(c, b, a[0])) { return r; }
    if crate::rt::redef(crate::rt::R_INT_LT | crate::rt::R_INT_SUCC) {
        // numeric.rb: `while i < self; yield i; i = i.succ; end`
        let mut i = Value::fix(0);
        while funcall(c, i, s::lt, &[s], std::ptr::null())?.truthy() { yield1(c, b, i)?; i = funcall(c, i, s::succ, &[], std::ptr::null())?; }
        return Ok(s);
    }
    let mut i = Value::fix(0);
    loop {
        if int_cmp_val(c, i, s)? != Some(std::cmp::Ordering::Less) { break; }
        yield1(c, b, i)?;
        i = int_arith(c, i, Value::fix(1), Op::Add)?;
    }
    Ok(s)
}
/// A Proc running `f(data, enumerator args)`: an Enumerator's size function.
pub fn size_proc(c: &mut Ctx, f: CBlockFn, data: Value) -> Value {
    let b = Block::cfunc(c, f, data);
    let k = c.vm().c_proc.get();
    let p = c.alloc(k, Body::Proc(Box::new(ProcData { blk: b, is_lambda: true, from_method: NIL })));
    c.proc_data_mut(p).blk.proc_val = p;
    p
}
/// ruby_num_interval_step_size
pub fn interval_step_size(c: &mut Ctx, from: Value, to: Value, step: Value, excl: bool) -> RV {
    if from.is_fixnum() && to.is_fixnum() && step.is_fixnum() {
        let mut d = step.as_fix() as i128;
        if d == 0 { return Ok(c.float_new(f64::INFINITY)); }
        let mut delta = to.as_fix() as i128 - from.as_fix() as i128;
        if d < 0 { d = -d; delta = -delta; }
        if excl { delta -= 1; }
        if delta < 0 { return Ok(Value::fix(0)); }
        return Ok(c.int_from_i128(delta / d + 1));
    }
    if is_float(from) || is_float(to) || is_float(step) {
        let n = float_step_size(to_f(c, from)?, to_f(c, to)?, to_f(c, step)?, excl);
        if n.is_infinite() { return Ok(c.float_new(n)); }
        return Ok(f64_to_int(c, n));
    }
    let r = funcall(c, step, s::cmp, &[Value::fix(0)], std::ptr::null())?;
    if r == NIL { return coerce_fail(c, step, Value::fix(0), true); }
    let cmp = match crate::builtins::array_methods::cmp_result(c, r, step, Value::fix(0))? { 0 => return Ok(c.float_new(f64::INFINITY)), -1 => s::lt, _ => s::gt };
    if funcall(c, from, cmp, &[to], std::ptr::null())?.truthy() { return Ok(Value::fix(0)); }
    let d = funcall(c, to, s::minus, &[from], std::ptr::null())?;
    let mut res = funcall(c, d, intern(b"div"), &[step], std::ptr::null())?;
    let add = !excl || { let m = funcall(c, res, s::mul, &[step], std::ptr::null())?; let l = funcall(c, from, s::plus, &[m], std::ptr::null())?; funcall(c, to, cmp, &[l], std::ptr::null())?.truthy() };
    if add { res = funcall(c, res, s::plus, &[Value::fix(1)], std::ptr::null())?; }
    Ok(res)
}
fn upto_size(c: &mut Ctx, d: Value, a: &[Value], _b: Blk, _o: Blk) -> RV { interval_step_size(c, d, a[0], Value::fix(1), false) }
fn downto_size(c: &mut Ctx, d: Value, a: &[Value], _b: Blk, _o: Blk) -> RV { interval_step_size(c, d, a[0], Value::fix(-1), false) }
fn int_upto(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    if b.is_null() { let sz = if s.is_fixnum() && a[0].is_fixnum() { NIL } else { size_proc(c, upto_size, s) }; return crate::builtins::enumerator::enum_for(c, s, s::upto, a, sz); }
    if let Some(r) = int_upto_loop(c, s, a[0], 1, &mut |c, a| yield1(c, b, a[0])) { return r; }
    if is_float(a[0]) {
        let e = float_val(a[0]); let mut i = s;
        while c.num_to_f64(i).unwrap() <= e { yield1(c, b, i)?; i = int_arith(c, i, Value::fix(1), Op::Add)?; }
        return Ok(s);
    }
    let mut i = s;
    loop {
        match int_cmp_val(c, i, a[0])? { Some(o) if o.is_le() => {} Some(_) => break, None => return coerce_fail(c, s, a[0], true) }
        yield1(c, b, i)?;
        i = int_arith(c, i, Value::fix(1), Op::Add)?;
    }
    Ok(s)
}
fn int_downto(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    if b.is_null() { let sz = if s.is_fixnum() && a[0].is_fixnum() { NIL } else { size_proc(c, downto_size, s) }; return crate::builtins::enumerator::enum_for(c, s, s::downto, a, sz); }
    if let Some(r) = int_upto_loop(c, s, a[0], -1, &mut |c, a| yield1(c, b, a[0])) { return r; }
    let mut i = s;
    loop {
        match int_cmp_val(c, i, a[0])? { Some(o) if o.is_ge() => {} Some(_) => break, None => return coerce_fail(c, s, a[0], true) }
        yield1(c, b, i)?;
        i = int_arith(c, i, Value::fix(1), Op::Sub)?;
    }
    Ok(s)
}
cfn!(int_succ, |c, s, _a| int_arith(c, s, Value::fix(1), Op::Add));
cfn!(int_pred, |c, s, _a| int_arith(c, s, Value::fix(1), Op::Sub));
cfn!(#[inline] int_zero, |c, s, _a| Ok(Value::bool(s == Value::fix(0))));
cfn!(#[inline] int_even, |c, s, _a| Ok(Value::bool(if s.is_fixnum() { s.as_fix() & 1 == 0 } else { !c.big(s).is_odd() })));
cfn!(#[inline] int_odd, |c, s, _a| Ok(Value::bool(if s.is_fixnum() { s.as_fix() & 1 == 1 } else { c.big(s).is_odd() })));
cfn!(int_positive, |c, s, _a| Ok(Value::bool(if s.is_fixnum() { s.as_fix() > 0 } else { !c.big(s).neg })));
cfn!(int_negative, |c, s, _a| Ok(Value::bool(if s.is_fixnum() { s.as_fix() < 0 } else { c.big(s).neg })));
cfn!(int_divmod, |c, s, a| {
    if is_float(a[0]) { let f = c.float_new(c.num_to_f64(s).unwrap()); return flo_divmod(c, f, a, Blk::NONE); }
    if !is_integer(a[0]) { return coerce_bin(c, s, a[0], s::divmod, false); }
    let q = int_arith(c, s, a[0], Op::Div)?; let m = int_arith(c, s, a[0], Op::Mod)?;
    Ok(c.ary_new_from(&[q, m]))
});
cfn!(int_idiv, |c, s, a| {
    if is_float(a[0]) {
        let y = float_val(a[0]);
        if y == 0.0 { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); }
        return Ok(f64_to_int(c, (c.num_to_f64(s).unwrap() / y).floor()));
    }
    if !is_integer(a[0]) { return coerce_bin(c, s, a[0], intern(b"div"), false); }
    int_arith(c, s, a[0], Op::Div)
});
cfn!(int_fdiv, |c, s, a| {
    let y = a[0];
    if is_integer(y) { return Ok(c.float_new(BigInt::ratio_to_f64(&c.to_big(s), &c.to_big(y)))); }
    if is_float(y) {
        let (x, dy) = (c.num_to_f64(s).unwrap(), float_val(y));
        if dy.is_nan() { return Ok(y); }
        return Ok(c.float_new(if dy != 0.0 { x / dy } else if x == 0.0 { f64::NAN } else { x * dy.signum() * f64::INFINITY }));
    }
    coerce_bin(c, s, y, intern(b"fdiv"), false)
});
cfn!(int_remainder, |c, s, a| {
    if is_float(a[0]) { if float_val(a[0]) == 0.0 { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); } return Ok(c.float_new(c.num_to_f64(s).unwrap() % float_val(a[0]))); }
    if !is_integer(a[0]) { return num_remainder(c, s, a, Blk::NONE); }
    let y = c.to_big(a[0]);
    if y.is_zero() { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); }
    Ok(c.big_to_value(c.to_big(s).divrem_trunc(&y).1))
});
fn gcd_big(a: BigInt, b: BigInt) -> BigInt {
    let (mut a, mut b) = (a.abs(), b.abs());
    while !b.is_zero() { let r = a.divmod(&b).1; a = b; b = r; }
    a
}
cfn!(int_gcd, |c, s, a| {
    if !is_integer(a[0]) { return type_err(c, "not an integer"); }
    if s.is_fixnum() && a[0].is_fixnum() { let (mut x, mut y) = (s.as_fix().unsigned_abs(), a[0].as_fix().unsigned_abs()); while y != 0 { let r = x % y; x = y; y = r; } return Ok(c.int_from_i128(x as i128)); }
    Ok(c.big_to_value(gcd_big(c.to_big(s), c.to_big(a[0]))))
});
cfn!(int_lcm, |c, s, a| {
    if !is_integer(a[0]) { return type_err(c, "not an integer"); }
    let (x, y) = (c.to_big(s), c.to_big(a[0]));
    if x.is_zero() || y.is_zero() { return Ok(Value::fix(0)); }
    let g = gcd_big(x.clone(), y.clone());
    Ok(c.big_to_value(x.mul(&y).abs().divmod(&g).0))
});
fn round_int(c: &mut Ctx, s: Value, a: &[Value], mode: u8) -> RV {
    // only round takes keywords (half:)
    let (pos, _) = if mode == b'r' { kw_split(c, a) } else { (a, None) };
    if pos.len() > 1 { return argc_err(c, pos.len(), 0, 1); }
    let Some(&d) = pos.first() else { return Ok(s) };
    let digits = num_to_int32(c, d)?;
    let half = if mode == b'r' { round_half_mode(c, a)? } else { 0 };
    if digits >= 0 { return Ok(s); }
    let x = c.to_big(s);
    // int_round_zero_p: 10**-digits / 2 > |self|
    let bytes = if s.is_fixnum() { 8 } else { (x.mag.len() as i64 + 1) / 2 * 8 };
    if (mode == b'r' || mode == b't') && -0.415241 * digits as f64 - 0.125 > bytes as f64 { return Ok(Value::fix(0)); }
    if -digits as u64 * 4 > 1u64 << 34 {
        // floor / ceil toward 0 are 0; the other way is 10**-digits itself
        if (mode == b'f' && !x.neg) || (mode == b'c' && x.neg) || x.is_zero() { return Ok(Value::fix(0)); }
        return arg_err(c, "exponent is too large");
    }
    let m = BigInt::from_i64(10).pow((-digits) as u64);
    let (q, r) = x.divmod(&m);
    let res = match mode {
        b'f' => q.mul(&m),
        b'c' => if r.is_zero() { x } else { q.add(&BigInt::from_i64(1)).mul(&m) },
        b't' => if x.neg && !r.is_zero() { q.add(&BigInt::from_i64(1)).mul(&m) } else { q.mul(&m) },
        _ => {
            let twice = r.mul(&BigInt::from_i64(2));
            let up = match twice.cmp(&m) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Equal => match half { 1 => q.is_odd(), 2 => x.neg, _ => !x.neg },
            };
            if up { q.add(&BigInt::from_i64(1)).mul(&m) } else { q.mul(&m) }
        }
    };
    Ok(c.big_to_value(res))
}
/// half: mode from kw (0 = up (default), 1 = even, 2 = down)
pub fn round_half_mode(c: &mut Ctx, a: &[Value]) -> Result<u8, Esc> {
    if let Some(h) = kw_arg(c, a) {
        if let Some(v) = crate::hash::hash_get_opt(c, h, crate::vm::sym_value("half"))? {
            if v == NIL { return Ok(0); }
            let sv = if v.is_sym() { Some(c.str_new(crate::symbol::sym_str(v.as_sym()).as_bytes())) } else if is_string(v) { Some(v) }
                     else if v.is_heap() && respond_to(c, v, s::to_str, true) { let r = funcall(c, v, s::to_str, &[], std::ptr::null())?; if is_string(r) { Some(r) } else { None } } else { None };
            if let Some(sv) = sv {
                let b = c.str_bytes(sv).to_vec();
                if b.eq_ignore_ascii_case(b"up") { return Ok(0); }
                if b.eq_ignore_ascii_case(b"even") { return Ok(1); }
                if b.eq_ignore_ascii_case(b"down") { return Ok(2); }
            }
            let t = funcall(c, v, s::to_s, &[], std::ptr::null())?;
            let ts = if is_string(t) { String::from_utf8_lossy(c.str_bytes(t)).into_owned() } else { String::new() };
            return arg_err(c, format!("invalid rounding mode: {}", ts));
        }
    }
    Ok(0)
}
cfn!(int_round, |c, s, a| round_int(c, s, a, b'r'));
cfn!(int_floor, |c, s, a| round_int(c, s, a, b'f'));
cfn!(int_ceil, |c, s, a| round_int(c, s, a, b'c'));
cfn!(int_truncate, |c, s, a| round_int(c, s, a, b't'));
cfn!(int_digits, |c, s, a| { check_argc(c, a, 0, 1)?;
    // rb_num_negative_int_p (`self < 0` once Integer#< is redefined) before the base
    let neg = if crate::rt::redef(crate::rt::R_INT_LT) { funcall(c, s, s::lt, &[Value::fix(0)], std::ptr::null())?.truthy() } else { c.to_big(s).neg };
    if neg { return err(c, c.vm().c_math_domain_error.get(), "out of domain"); }
    let b = if a.is_empty() { BigInt::from_i64(10) } else {
        let bv = rb_to_int(c, a[0])?;
        let b = c.to_big(bv);
        if b.neg { return arg_err(c, "negative radix"); }
        if let Some(i) = b.to_i64() { if i < 2 { return arg_err(c, format!("invalid radix {}", i)); } }
        b
    };
    let out = c.ary_new();
    let mut x = c.to_big(s);
    if x.is_zero() { c.ary_push(out, Value::fix(0)); }
    while !x.is_zero() { let (q, r) = x.divmod(&b); let d = c.big_to_value(r); c.ary_push(out, d); x = q; }
    Ok(out)
});
cfn!(int_bit_length, |c, s, _a| Ok(Value::fix(if s.is_fixnum() { let x = s.as_fix(); (64 - if x < 0 { (!x).leading_zeros() } else { x.leading_zeros() }) as i64 } else { c.big(s).bit_length() as i64 })));
// rb_big_size_m: the bytes of |self|
cfn!(int_size, |c, s, _a| Ok(Value::fix(if s.is_fixnum() { 8 } else {
    let m = &c.big(s).mag;
    match m.last() { None => 0, Some(&top) => (((m.len() - 1) * 32 + 32 - top.leading_zeros() as usize + 7) / 8) as i64 }
})));
cfn!(int_s_try_convert, |c, _s, a| {
    let v = a[0];
    if is_integer(v) { return Ok(v); }
    if is_float(v) { return rb_to_int(c, v); }
    let Some(r) = (if v.is_heap() { crate::string::check_funcall(c, v, s::to_int)? } else { None }) else { return Ok(NIL) };
    if r == NIL || is_integer(r) { return Ok(r); }
    let cls = class_name_for_err(c, v);
    type_err(c, format!("can't convert {} to Integer ({}#to_int gives {})", cls, cls, class_name_for_err(c, r)))
});
cfn!(int_hash, |c, s, _a| Ok(Value::fix((crate::hash::key_hash(c, s)? >> 2) as i64)));
/// Kernel#Float semantics (rb_Float)
pub fn rb_float(c: &mut Ctx, v: Value) -> Result<f64, Esc> {
    if let Some(f) = c.num_to_f64(v) { return Ok(f); }
    let r = funcall(c, c.vm().main_obj.get(), intern(b"Float"), &[v], std::ptr::null())?;
    to_f(c, r)
}
cfn!(int_coerce, |c, s, a| {
    if is_integer(a[0]) { return Ok(c.ary_new_from(&[a[0], s])); }
    let x = rb_float(c, a[0])?;
    let (x, y) = (c.float_new(x), c.float_new(c.num_to_f64(s).unwrap()));
    Ok(c.ary_new_from(&[x, y]))
});
cfn!(int_gcdlcm, |c, s, a| {
    if !is_integer(a[0]) { return type_err(c, "not an integer"); }
    let g = int_gcd(c, s, a, Blk::NONE)?;
    let l = int_lcm(c, s, a, Blk::NONE)?;
    Ok(c.ary_new_from(&[g, l]))
});
cfn!(int_allbits, |c, s, a| { let m = to_integer(c, a[0])?; let r = int_bitop(c, s, m, b'&')?; Ok(Value::bool(c.to_big(r) == c.to_big(m))) });
cfn!(int_anybits, |c, s, a| { let m = to_integer(c, a[0])?; let r = int_bitop(c, s, m, b'&')?; Ok(Value::bool(r != Value::fix(0))) });
cfn!(int_nobits, |c, s, a| { let m = to_integer(c, a[0])?; let r = int_bitop(c, s, m, b'&')?; Ok(Value::bool(r == Value::fix(0))) });
cfn!(int_s_sqrt, |c, _s, a| {
    let v = to_integer(c, a[0])?;
    let b = c.to_big(v);
    if b.neg { return err(c, c.vm().c_math_domain_error.get(), "Numerical argument is out of domain - \"isqrt\""); }
    Ok(c.big_to_value(b.isqrt()))
});
cfn!(int_ceildiv, |c, s, a| {
    let neg = int_arith(c, s, Value::fix(-1), Op::Mul)?;
    let q = funcall(c, neg, intern(b"div"), &[a[0]], std::ptr::null())?;
    funcall(c, q, s::uminus, &[], std::ptr::null())
});
cfn!(int_integer_p, |_c, _s, _a| Ok(TRUE));

// ---- Float -------------------------------------------------------------------------------

fn flo_arith(c: &mut Ctx, a: Value, b: Value, op: Op) -> RV {
    let x = float_val(a);
    let y = match c.num_to_f64(b) { Some(y) => y, None => {
        let sym = match op { Op::Add => s::plus, Op::Sub => s::minus, Op::Mul => s::mul, Op::Div => s::div, Op::Mod => s::modulo, Op::Pow => s::pow };
        return coerce_bin(c, a, b, sym, false);
    } };
    if op == Op::Mod && y == 0.0 { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); }
    Ok(c.float_new(match op {
        Op::Add => x + y, Op::Sub => x - y, Op::Mul => x * y, Op::Div => x / y,
        Op::Mod => fmod_ruby(x, y),
        Op::Pow => {
            if x < 0.0 && y != y.round() { return Ok(crate::builtins::complex::polar_pi(c, (-x).powf(y), y)); }
            x.powf(y)
        }
    }))
}
cfn!(flo_plus, |c, s, a| flo_arith(c, s, a[0], Op::Add));
cfn!(flo_minus, |c, s, a| flo_arith(c, s, a[0], Op::Sub));
cfn!(flo_mul, |c, s, a| flo_arith(c, s, a[0], Op::Mul));
cfn!(flo_div, |c, s, a| flo_arith(c, s, a[0], Op::Div));
cfn!(flo_mod, |c, s, a| flo_arith(c, s, a[0], Op::Mod));
cfn!(flo_pow, |c, s, a| flo_arith(c, s, a[0], Op::Pow));
cfn!(flo_uminus, |c, s, _a| Ok(c.float_new(-float_val(s))));
fn flo_cmp_val(c: &mut Ctx, a: Value, b: Value) -> Result<Option<std::cmp::Ordering>, Esc> {
    let x = float_val(a);
    if is_integer(b) { return Ok(int_cmp_val(c, b, a)?.map(|o| o.reverse())); }
    match c.num_to_f64(b) { Some(y) => Ok(x.partial_cmp(&y)), None => Ok(None) }
}
cfn!(flo_eq, |c, s, a| {
    if let Some(_) = c.num_to_f64(a[0]) { return Ok(Value::bool(flo_cmp_val(c, s, a[0])? == Some(std::cmp::Ordering::Equal))); }
    let r = funcall(c, a[0], s::eq, &[s], std::ptr::null())?; Ok(Value::bool(r.truthy()))
});
cfn!(flo_eql, |c, s, a| Ok(Value::bool(is_float(a[0]) && float_val(a[0]) == float_val(s))));
cfn!(flo_cmp, |c, s, a| {
    let x = float_val(s);
    if x.is_infinite() && a[0].is_heap() && !is_float(a[0]) && !is_bignum(a[0]) && respond_to(c, a[0], intern(b"infinite?"), true) {
        let r = funcall(c, a[0], intern(b"infinite?"), &[], std::ptr::null())?;
        if !r.truthy() { return Ok(Value::fix(if x > 0.0 { 1 } else { -1 })); }
        // flo_cmp: both infinite; equal when the signs agree (rb_cmpint of infinite?)
        let j = if r.is_fixnum() { r.as_fix().signum() } else if funcall(c, r, s::gt, &[Value::fix(0)], std::ptr::null())?.truthy() { 1 }
            else if funcall(c, r, s::lt, &[Value::fix(0)], std::ptr::null())?.truthy() { -1 } else { 0 };
        return Ok(Value::fix(if x > 0.0 { if j > 0 { 0 } else { 1 } } else if j < 0 { 0 } else { -1 }));
    }
    match flo_cmp_val(c, s, a[0])? {
        Some(o) => Ok(Value::fix(o as i64)),
        None => if c.num_to_f64(a[0]).is_none() && a[0].is_heap() && respond_to(c, a[0], s::coerce, true) { coerce_bin(c, s, a[0], s::cmp, false) } else { Ok(NIL) },
    }
});
fn flo_rel(c: &mut Ctx, s: Value, b: Value, f: fn(std::cmp::Ordering) -> bool, op: Sym) -> RV {
    match flo_cmp_val(c, s, b)? {
        Some(o) => Ok(Value::bool(f(o))),
        None => if c.num_to_f64(b).is_some() { Ok(FALSE) } else { coerce_bin(c, s, b, op, true) },
    }
}
cfn!(flo_lt, |c, s, a| flo_rel(c, s, a[0], |o| o.is_lt(), s::lt));
cfn!(flo_le, |c, s, a| flo_rel(c, s, a[0], |o| o.is_le(), s::le));
cfn!(flo_gt, |c, s, a| flo_rel(c, s, a[0], |o| o.is_gt(), s::gt));
cfn!(flo_ge, |c, s, a| flo_rel(c, s, a[0], |o| o.is_ge(), s::ge));
cfn!(flo_to_s, |c, s, _a| { let v = c.str_new(float_to_s(float_val(s)).as_bytes()); c.rstring_mut(v).enc = ENC_ASCII; Ok(v) });
fn check_finite(c: &mut Ctx, f: f64) -> Result<(), Esc> {
    if f.is_nan() || f.is_infinite() { return err(c, c.vm().c_float_domain_error.get(), float_to_s(f)); }
    Ok(())
}
cfn!(flo_to_i, |c, s, _a| { let f = float_val(s); check_finite(c, f)?; Ok(f64_to_int(c, f)) });
cfn!(flo_abs, |c, s, _a| Ok(c.float_new(float_val(s).abs())));
cfn!(flo_nan, |c, s, _a| Ok(Value::bool(float_val(s).is_nan())));
cfn!(flo_infinite, |c, s, _a| { let f = float_val(s); Ok(if f.is_infinite() { Value::fix(if f > 0.0 { 1 } else { -1 }) } else { NIL }) });
cfn!(flo_finite, |c, s, _a| Ok(Value::bool(float_val(s).is_finite())));
cfn!(flo_zero, |c, s, _a| Ok(Value::bool(float_val(s) == 0.0)));
cfn!(flo_positive, |c, s, _a| Ok(Value::bool(float_val(s) > 0.0)));
cfn!(flo_negative, |c, s, _a| Ok(Value::bool(float_val(s) < 0.0)));
fn flo_divmod(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let x = float_val(s);
    let Some(y) = c.num_to_f64(a[0]) else { return coerce_bin(c, s, a[0], s::divmod, false) };
    let (q, m) = flodivmod(c, x, y)?;
    check_finite(c, q)?;
    let (q, m) = (f64_to_int(c, q), c.float_new(m));
    Ok(c.ary_new_from(&[q, m]))
}
/// CRuby's flodivmod
fn flodivmod(c: &mut Ctx, x: f64, y: f64) -> Result<(f64, f64), Esc> {
    if y.is_nan() { return Ok((y, y)); }
    if y == 0.0 { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); }
    let mut m = if x == 0.0 || (y.is_infinite() && !x.is_infinite()) { x } else { x % y };
    let mut d = if x.is_infinite() && !y.is_infinite() { x } else { ((x - m) / y).round() };
    if y * m < 0.0 { m += y; d -= 1.0; }
    Ok((d, m))
}
cfn!(flo_coerce, |c, s, a| { let y = rb_float(c, a[0])?; let y = c.float_new(y); Ok(c.ary_new_from(&[y, s])) });
cfn!(flo_hash, |c, s, _a| { let f = float_val(s); let b = if f == 0.0 { 0 } else { f.to_bits() }; Ok(Value::fix((crate::astrust::hash_u64(b) >> 2) as i64)) });

// ---- Float rounding: ports of CRuby's round_half_* / rb_float_floor / rb_float_ceil -------------

fn frexp_exp(x: f64) -> i32 { if x == 0.0 || !x.is_finite() { 0 } else { crate::builtins::rational::frexp(x).1 as i32 } }
fn float_round_overflow(nd: i64, binexp: i32) -> bool { nd >= (15 + 2) - (if binexp > 0 { binexp / 4 } else { binexp / 3 - 1 }) as i64 }
fn float_round_underflow(nd: i64, binexp: i32) -> bool { nd < -(if binexp > 0 { binexp / 3 + 1 } else { binexp / 4 }) as i64 }
fn c_round(x: f64) -> f64 { x.round() }
fn round_half_up(x: f64, s: f64) -> f64 {
    let xs = x * s;
    let mut f = c_round(xs);
    if s == 1.0 { return f; }
    if x > 0.0 { if (f + 0.5) / s <= x { f += 1.0; } f } else { if (f - 0.5) / s >= x { f -= 1.0; } f }
}
fn round_half_down(x: f64, s: f64) -> f64 {
    let xs = x * s;
    let mut f = c_round(xs);
    if x > 0.0 { if (f - 0.5) / s >= x { f -= 1.0; } f } else { if (f + 0.5) / s <= x { f += 1.0; } f }
}
fn round_half_even(x: f64, s: f64) -> f64 {
    let u = x.trunc(); let v = x - u;
    let us = u * s; let vs = v * s;
    let r;
    if x > 0.0 {
        let f = vs.floor(); let uf = us + f; let mut d = vs - f;
        if d > 0.5 { d = 1.0; } else if d == 0.5 || (uf + 0.5) / s <= x { d = uf % 2.0; } else { d = 0.0; }
        r = f + d;
    } else if x < 0.0 {
        let f = vs.ceil(); let uf = us + f; let mut d = f - vs;
        if d > 0.5 { d = 1.0; } else if d == 0.5 || (uf - 0.5) / s >= x { d = (-uf) % 2.0; } else { d = 0.0; }
        r = f - d;
    } else { r = x; }
    us + r
}
fn round_by_mode(x: f64, s: f64, half: u8) -> f64 { match half { 1 => round_half_even(x, s), 2 => round_half_down(x, s), _ => round_half_up(x, s) } }
fn dbl2ival(c: &mut Ctx, f: f64) -> RV { check_finite(c, f)?; Ok(f64_to_int(c, f)) }
fn flo_by_rational(c: &mut Ctx, f: f64, nd: i64, mode: u8, half: u8) -> RV {
    let r = crate::builtins::rational::float_to_r(c, f)?;
    let x = crate::builtins::rational::rat_round_nd(c, r, nd, mode, half);
    Ok(c.float_new(if is_integer(x) { crate::builtins::rational::int_to_f(c, x) } else { crate::builtins::rational::rat_to_f(c, x) }))
}
fn flo_ndigits(c: &mut Ctx, a: &[Value]) -> Result<i64, Esc> {
    let (pos, _) = kw_split(c, a);
    if pos.len() > 1 { return arg_err(c, format!("wrong number of arguments (given {}, expected 0..1)", pos.len())); }
    match pos.first() {
        None => Ok(0),
        Some(&v) => num_to_int32(c, v),
    }
}
fn flo_floor_nd(c: &mut Ctx, s: Value, nd: i64) -> RV {
    let x = float_val(s);
    if x == 0.0 { return Ok(if nd > 0 { s } else { Value::fix(0) }); }
    if nd > 0 {
        let be = frexp_exp(x);
        if float_round_overflow(nd, be) { return Ok(s); }
        if x > 0.0 && float_round_underflow(nd, be) { return Ok(c.float_new(0.0)); }
        if nd >= 15 { return flo_by_rational(c, x, nd, b'f', 0); }
        let f = 10f64.powi(nd as i32);
        let mul = (x * f).floor();
        let mut res = (mul + 1.0) / f;
        if res > x { res = mul / f; }
        return Ok(c.float_new(res));
    }
    let i = dbl2ival(c, x.floor())?;
    if nd < 0 { return round_int(c, i, &[Value::fix(nd)], b'f'); }
    Ok(i)
}
fn flo_ceil_nd(c: &mut Ctx, s: Value, nd: i64) -> RV {
    let x = float_val(s);
    if x == 0.0 { return Ok(if nd > 0 { s } else { Value::fix(0) }); }
    if nd > 0 {
        let be = frexp_exp(x);
        if float_round_overflow(nd, be) { return Ok(s); }
        if x < 0.0 && float_round_underflow(nd, be) { return Ok(c.float_new(0.0)); }
        if nd >= 15 { return flo_by_rational(c, x, nd, b'c', 0); }
        let f = 10f64.powi(nd as i32);
        return Ok(c.float_new((x * f).ceil() / f));
    }
    let i = dbl2ival(c, x.ceil())?;
    if nd < 0 { return round_int(c, i, &[Value::fix(nd)], b'c'); }
    Ok(i)
}
cfn!(flo_round, |c, s, a| {
    let nd = flo_ndigits(c, a)?;
    let half = round_half_mode(c, a)?;
    let x = float_val(s);
    if x == 0.0 { return Ok(if nd > 0 { s } else { Value::fix(0) }); }
    if nd < 0 {
        let i = dbl2ival(c, x.trunc())?;
        // this frame's keywords, if any, go on with nd
        let kw = kw_arg(c, a);
        let args = [Value::fix(nd), kw.unwrap_or(NIL)];
        return round_int(c, i, &args[..1 + kw.is_some() as usize], b'r');
    }
    if nd == 0 { return dbl2ival(c, round_by_mode(x, 1.0, half)); }
    if x.is_finite() {
        let be = frexp_exp(x);
        if float_round_overflow(nd, be) { return Ok(s); }
        if float_round_underflow(nd, be) { return Ok(c.float_new(0.0)); }
        if nd >= 15 { return flo_by_rational(c, x, nd, b'r', half); }
        let f = 10f64.powi(nd as i32);
        return Ok(c.float_new(round_by_mode(x, f, half) / f));
    }
    Ok(s)
});
cfn!(flo_floor, |c, s, a| { let nd = flo_ndigits(c, a)?; flo_floor_nd(c, s, nd) });
cfn!(flo_ceil, |c, s, a| { let nd = flo_ndigits(c, a)?; flo_ceil_nd(c, s, nd) });
cfn!(flo_truncate, |c, s, a| {
    let nd = flo_ndigits(c, a)?;
    if float_val(s).is_sign_negative() { flo_ceil_nd(c, s, nd) } else { flo_floor_nd(c, s, nd) }
});
cfn!(flo_next_float, |c, s, _a| { let f = float_val(s); Ok(c.float_new(next_after(f, f64::INFINITY))) });
cfn!(flo_prev_float, |c, s, _a| { let f = float_val(s); Ok(c.float_new(next_after(f, f64::NEG_INFINITY))) });
fn next_after(f: f64, dir: f64) -> f64 {
    if f.is_nan() { return f; }
    if f == dir { return f; }
    if f == 0.0 { return if dir > 0.0 { f64::from_bits(1) } else { -f64::from_bits(1) }; }
    let b = f.to_bits();
    let nb = if (dir > f) == (f > 0.0) { b + 1 } else { b - 1 };
    f64::from_bits(nb)
}

// ---- Numeric (generic) -----------------------------------------------------------------------

/// CRuby's ruby_float_step_size
pub fn float_step_size(beg: f64, end: f64, unit: f64, excl: bool) -> f64 {
    if unit == 0.0 { return f64::INFINITY; }
    if unit.is_infinite() { return if unit > 0.0 { (beg <= end) as i32 as f64 } else { (beg >= end) as i32 as f64 }; }
    let mut n = (end - beg) / unit;
    let mut err = (beg.abs() + end.abs() + (end - beg).abs()) / unit.abs() * f64::EPSILON;
    if err > 0.5 { err = 0.5; }
    if excl {
        if n <= 0.0 { return 0.0; }
        if n < 1.0 { n = 0.0; } else { n = (n - err).floor(); }
        let d = (n + 1.0) * unit + beg;
        if beg < end { if d < end { n += 1.0; } } else if beg > end { if d > end { n += 1.0; } }
    } else {
        if n < 0.0 { return 0.0; }
        n = (n + err).floor();
    }
    n + 1.0
}
/// The loop of `s.step(lim, st)` with three fixnums and st != 0; None: another case.
#[inline(always)]
pub fn num_step_loop(c: &mut Ctx, s: Value, lim: Value, st: Value, y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> Option<RV> {
    if !s.is_fixnum() || !lim.is_fixnum() || !st.is_fixnum() || st == Value::fix(0) { return None; }
    let (mut i, d, e) = (s.as_fix(), st.as_fix(), lim.as_fix());
    // the count of steps first: one test per step, whichever the direction (|i|, |d|, |e| < 2^62: no overflow)
    let mut n = if d > 0 { if i > e { 0 } else { (e - i) / d + 1 } } else if i < e { 0 } else { (i - e) / -d + 1 };
    while n > 0 { if let Err(e) = y(c, &[Value::fix(i)]) { return Some(Err(e)); } i += d; n -= 1; }
    Some(Ok(s))
}
fn num_step(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    let kw = kw_arg(c, a);
    let pos = if kw.is_some() { &a[..a.len() - 1] } else { a };
    if pos.len() > 2 { return arg_err(c, format!("wrong number of arguments (given {}, expected 0..2)", pos.len())); }
    let mut lim = pos.first().copied().unwrap_or(NIL);
    let mut st = pos.get(1).copied().unwrap_or(NIL);
    if let Some(h) = kw {
        let mut unknown = vec![];
        for (k, _) in c.hash_pairs(h) { if k != sym_value("to") && k != sym_value("by") { unknown.push(inspect_str(c, k)?); } }
        if !unknown.is_empty() { return arg_err(c, format!("unknown keyword{}: {}", if unknown.len() > 1 { "s" } else { "" }, unknown.join(", "))); }
        if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("to"))? { if !pos.is_empty() { return arg_err(c, "to is given twice"); } lim = v; }
        if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("by"))? { if pos.len() > 1 { return arg_err(c, "step is given twice"); } st = v; }
    }
    if st == NIL { st = Value::fix(1); }
    // rb_equal(step, 0): Integer#== once redefined
    else if st == Value::fix(0) || ((!st.is_fixnum() || crate::rt::redef(crate::rt::R_INT_EQ)) && funcall(c, st, s::eq, &[Value::fix(0)], std::ptr::null())?.truthy()) { return arg_err(c, "step can't be 0"); }
    let numc = c.vm().c_numeric.get();
    let numeric = |v: Value| v.is_fixnum() || is_float(v) || is_kind_of(c, v, numc);
    if b.is_null() {
        if (lim == NIL || numeric(lim)) && numeric(st) { return crate::builtins::enumerator::arith_seq_from(c, s, s, lim, st, false, s::step, a); }
        // a non-Numeric limit or step: a plain Enumerator (numeric.c num_step)
        let sz = size_proc(c, step_size, s);
        return crate::builtins::enumerator::enum_for(c, s, s::step, &[lim, st], sz);
    }
    // num_step_negative_p: `step > 0` decides the direction (and rejects e.g. a String step)
    if !numeric(st) { funcall(c, st, s::gt, &[Value::fix(0)], std::ptr::null())?; }
    else if st.is_fixnum() && crate::rt::redef(crate::rt::R_INT_LT | crate::rt::R_INT_EQ) {
        // num_step_negative_p asks `step > 0` once Integer#< is redefined, then num_step's own rb_equal(step, 0)
        if crate::rt::redef(crate::rt::R_INT_LT) { funcall(c, st, s::gt, &[Value::fix(0)], std::ptr::null())?; }
        if crate::rt::redef(crate::rt::R_INT_EQ) { funcall(c, st, s::eq, &[Value::fix(0)], std::ptr::null())?; }
    }
    if s.is_fixnum() && st.is_fixnum() && (lim.is_fixnum() || lim == NIL) {
        let (mut i, d) = (s.as_fix(), st.as_fix());
        if lim == NIL {
            loop { yield1(c, b, Value::fix(i))?; i += d; if !Value::fixable(i) { break; } }
            let from = c.int_from_i64(i);
            return num_step_generic(c, s, from, lim, st, b);
        }
        if let Some(r) = num_step_loop(c, s, lim, st, &mut |c, a| yield1(c, b, a[0])) { return r; }
    }
    if (is_float(s) || is_float(lim) || is_float(st)) && !crate::builtins::complex::is_complex(c, st) {
        let x = to_f(c, s)?; let d = to_f(c, st)?;
        let l = if lim == NIL { if d < 0.0 { f64::NEG_INFINITY } else { f64::INFINITY } } else { to_f(c, lim)? };
        let n = float_step_size(x, l, d, false);
        // ruby_float_step: an infinite step yields self once
        if d.is_infinite() {
            if n != 0.0 { let fv = c.float_new(x); yield1(c, b, fv)?; }
            return Ok(s);
        }
        let mut i = 0.0;
        while i < n {
            let mut v = i * d + x;
            if if d >= 0.0 { l < v } else { v < l } { v = l; }
            let fv = c.float_new(v);
            yield1(c, b, fv)?;
            i += 1.0;
        }
        return Ok(s);
    }
    num_step_generic(c, s, s, lim, st, b)
}
/// num_step_size: num_step_negative_p (`step > 0`) first
fn step_size(c: &mut Ctx, d: Value, a: &[Value], _b: Blk, _o: Blk) -> RV {
    let (lim, st) = (a[0], a[1]);
    let neg = if is_integer(st) && !crate::rt::redef(crate::rt::R_INT_LT) { if st.is_fixnum() { st.as_fix() < 0 } else { c.big(st).neg } }
        else if respond_to(c, st, s::gt, true) { !funcall(c, st, s::gt, &[Value::fix(0)], std::ptr::null())?.truthy() } else { return coerce_fail(c, st, Value::fix(0), false) };
    let lim = if lim == NIL { c.float_new(if neg { f64::NEG_INFINITY } else { f64::INFINITY }) } else { lim };
    interval_step_size(c, d, lim, st, false)
}
fn num_step_generic(c: &mut Ctx, s: Value, from: Value, lim: Value, st: Value, b: Blk) -> RV {
    let neg = lim != NIL && funcall(c, st, s::lt, &[Value::fix(0)], std::ptr::null())?.truthy();
    let mut i = from;
    loop {
        if lim != NIL {
            let done = funcall(c, i, if neg { s::lt } else { s::gt }, &[lim], std::ptr::null())?;
            if done.truthy() { break; }
        }
        yield1(c, b, i)?;
        i = funcall(c, i, s::plus, &[st], std::ptr::null())?;
    }
    Ok(s)
}
cfn!(num_integer_p, |_c, _s, _a| Ok(FALSE));
cfn!(num_zero_p, |c, s, _a| { let r = funcall(c, s, s::eq, &[Value::fix(0)], std::ptr::null())?; Ok(Value::bool(r.truthy())) });
cfn!(num_nonzero_p, |c, s, _a| { let r = funcall(c, s, s::zero_p, &[], std::ptr::null())?; Ok(if r.truthy() { NIL } else { s }) });
cfn!(num_positive_p, |c, s, a| if is_integer(s) { int_positive(c, s, a, Blk::NONE) } else { funcall(c, s, s::gt, &[Value::fix(0)], std::ptr::null()) });
cfn!(num_negative_p, |c, s, a| if is_integer(s) { int_negative(c, s, a, Blk::NONE) } else { funcall(c, s, s::lt, &[Value::fix(0)], std::ptr::null()) });
cfn!(num_abs, |c, s, _a| { let neg = funcall(c, s, s::lt, &[Value::fix(0)], std::ptr::null())?; if neg.truthy() { funcall(c, s, s::uminus, &[], std::ptr::null()) } else { Ok(s) } });
cfn!(num_uplus, |c, s, _a| Ok(s));
// num_clone: Numerics are immutable; freeze: false cannot be honoured
cfn!(num_clone, |c, s, a| {
    if let Some(h) = kw_arg(c, a) {
        if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("freeze"))? {
            if v != NIL && v != TRUE && v != FALSE { let i = class_path(c, real_class_of(c, v)); return arg_err(c, format!("unexpected value for freeze: {}", i)); }
            if v == FALSE { return arg_err(c, format!("can't unfreeze {}", class_path(c, real_class_of(c, s)))); }
        }
    }
    Ok(s)
});
cfn!(num_uminus, |c, s, _a| {
    let r = funcall(c, Value::fix(0), s::coerce, &[s], std::ptr::null());
    match r { Ok(r) if is_array(r) => { let x = c.ary_get(r, 0); funcall(c, x, s::minus, &[c.ary_get(r, 1)], std::ptr::null()) }
              _ => { c.esc = EscKind::None; let z = Value::fix(0); funcall(c, z, s::minus, &[s], std::ptr::null()) } }
});
cfn!(num_coerce, |c, s, a| {
    if real_class_of(c, s) == real_class_of(c, a[0]) { return Ok(c.ary_new_from(&[a[0], s])); }
    let x = rb_float(c, a[0])?; let y = rb_float(c, s)?;
    let (x, y) = (c.float_new(x), c.float_new(y));
    Ok(c.ary_new_from(&[x, y]))
});
cfn!(num_floor, |c, s, a| { let f = rb_float(c, s)?; let f = c.float_new(f); flo_floor(c, f, a, Blk::NONE) });
cfn!(num_ceil, |c, s, a| { let f = rb_float(c, s)?; let f = c.float_new(f); flo_ceil(c, f, a, Blk::NONE) });
cfn!(num_round, |c, s, a| { let f = rb_float(c, s)?; let f = c.float_new(f); flo_round(c, f, a, Blk::NONE) });
cfn!(num_truncate, |c, s, a| { let f = rb_float(c, s)?; let f = c.float_new(f); flo_truncate(c, f, a, Blk::NONE) });
cfn!(num_div, |c, s, a| {
    if a[0] == Value::fix(0) || funcall(c, Value::fix(0), s::eq, &[a[0]], std::ptr::null())?.truthy() { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); }
    let q = funcall(c, s, s::div, &[a[0]], std::ptr::null())?;
    funcall(c, q, intern(b"floor"), &[], std::ptr::null())
});
cfn!(num_modulo, |c, s, a| {
    let q = funcall(c, s, intern(b"div"), &[a[0]], std::ptr::null())?;
    let m = funcall(c, a[0], s::mul, &[q], std::ptr::null())?;
    funcall(c, s, s::minus, &[m], std::ptr::null())
});
cfn!(num_divmod, |c, s, a| {
    let q = funcall(c, s, intern(b"div"), &[a[0]], std::ptr::null())?;
    let m = funcall(c, s, s::modulo, &[a[0]], std::ptr::null())?;
    Ok(c.ary_new_from(&[q, m]))
});
fn cmp_zero(c: &mut Ctx, v: Value, op: Sym) -> Result<bool, Esc> {
    if !respond_to(c, v, op, true) { return coerce_fail(c, v, Value::fix(0), true); }
    Ok(funcall(c, v, op, &[Value::fix(0)], std::ptr::null())?.truthy())
}
fn neg_int_p(c: &mut Ctx, v: Value) -> Result<bool, Esc> {
    if is_integer(v) { return Ok(if v.is_fixnum() { v.as_fix() < 0 } else { c.big(v).neg }); }
    cmp_zero(c, v, s::lt)
}
fn pos_int_p(c: &mut Ctx, v: Value) -> Result<bool, Esc> {
    if is_integer(v) { return Ok(if v.is_fixnum() { v.as_fix() > 0 } else { !c.big(v).neg }); }
    cmp_zero(c, v, s::gt)
}
cfn!(num_remainder, |c, s, a| {
    let (mut x, mut y) = (s, a[0]);
    if !is_kind_of(c, y, c.vm().c_numeric.get()) && !is_numeric(y) {
        if !(y.is_heap() && respond_to(c, y, s::coerce, true)) { return coerce_fail(c, x, y, false); }
        let r = funcall(c, y, s::coerce, &[x], std::ptr::null())?;
        if !is_array(r) || c.ary_len(r) != 2 { return type_err(c, "coerce must return [x, y]"); }
        x = c.ary_get(r, 0); y = c.ary_get(r, 1);
    }
    let z = funcall(c, x, s::modulo, &[y], std::ptr::null())?;
    let zz = funcall(c, z, s::eq, &[Value::fix(0)], std::ptr::null())?.truthy();
    if !zz && ((neg_int_p(c, x)? && pos_int_p(c, y)?) || (pos_int_p(c, x)? && neg_int_p(c, y)?)) {
        if is_float(y) && float_val(y).is_infinite() { return Ok(x); }
        return funcall(c, z, s::minus, &[y], std::ptr::null());
    }
    let _ = (&mut x, &mut y);
    Ok(z)
});
cfn!(num_fdiv, |c, s, a| { let x = to_f(c, s)?; let y = to_f(c, a[0])?; Ok(c.float_new(x / y)) });
cfn!(num_quo, |c, s, a| {
    if crate::builtins::complex::is_complex(c, s) { return funcall(c, s, s::div, &[a[0]], std::ptr::null()); }
    if is_float(a[0]) { let x = to_f(c, s)?; return Ok(c.float_new(x / float_val(a[0]))); }
    let r = if is_integer(s) { crate::builtins::rational::rat_raw(c, s, Value::fix(1)) }
        else if crate::builtins::rational::is_rational(c, s) { s }
        else if respond_to(c, s, intern(b"to_r"), true) {
            let r = funcall(c, s, intern(b"to_r"), &[], std::ptr::null())?;
            if !crate::builtins::rational::is_rational(c, r) { let cn = class_path(c, real_class_of(c, s)); return type_err(c, format!("can't convert {} to Rational ({}#to_r gives {})", cn, cn, class_path(c, real_class_of(c, r)))); }
            r
        } else { return type_err(c, format!("can't convert {} into Rational", class_path(c, real_class_of(c, s)))); };
    funcall(c, r, s::div, &[a[0]], std::ptr::null())
});
cfn!(num_to_int, |c, s, _a| funcall(c, s, s::to_i, &[], std::ptr::null()));
cfn!(num_numerator, |c, s, _a| { let r = funcall(c, s, intern(b"to_r"), &[], std::ptr::null())?; funcall(c, r, intern(b"numerator"), &[], std::ptr::null()) });
cfn!(num_denominator, |c, s, _a| { let r = funcall(c, s, intern(b"to_r"), &[], std::ptr::null())?; funcall(c, r, intern(b"denominator"), &[], std::ptr::null()) });
// num_sadded: Numerics have no singleton methods
cfn!(num_sadded, |c, s, a| {
    let mid = if a[0].is_sym() { a[0].as_sym() } else { return type_err(c, "can't define singleton") };
    let k = singleton_class(c, s);
    funcall(c, k, intern(b"remove_method"), &[a[0]], std::ptr::null())?;
    type_err(c, format!("can't define singleton method \"{}\" for {}", crate::symbol::sym_str(mid), class_path(c, real_class_of(c, s))))
});
cfn!(num_real_p, |_c, _s, _a| Ok(TRUE));
cfn!(num_finite_p, |_c, _s, _a| Ok(TRUE));
cfn!(num_infinite_p, |_c, _s, _a| Ok(NIL));
cfn!(num_eql, |c, s, a| {
    if is_integer(s) { return int_eql(c, s, a, Blk::NONE); }
    if real_class_of(c, s) != real_class_of(c, a[0]) { return Ok(FALSE); }
    funcall(c, s, s::eq, &[a[0]], std::ptr::null())
});
cfn!(num_i, |c, s, _a| funcall(c, c.vm().main_obj.get(), intern(b"Complex"), &[Value::fix(0), s], std::ptr::null()));
cfn!(num_to_c, |c, s, _a| funcall(c, c.vm().main_obj.get(), intern(b"Complex"), &[s, Value::fix(0)], std::ptr::null()));
cfn!(num_real, |c, s, _a| Ok(s));
cfn!(num_imag, |_c, _s, _a| Ok(Value::fix(0)));
cfn!(num_conj, |c, s, _a| Ok(s));
cfn!(num_rect, |c, s, _a| Ok(c.ary_new_from(&[s, Value::fix(0)])));
cfn!(num_arg, |c, s, _a| { let neg = funcall(c, s, s::lt, &[Value::fix(0)], std::ptr::null())?; Ok(if neg.truthy() { c.float_new(std::f64::consts::PI) } else { Value::fix(0) }) });

/// Float constants that are not flonums: defined by builtins::init, which may allocate at boot.
pub const FLOAT_CONSTS: [(&str, f64); 5] = [("INFINITY", f64::INFINITY), ("NAN", f64::NAN), ("EPSILON", f64::EPSILON), ("MAX", f64::MAX), ("MIN", f64::MIN_POSITIVE)];
/// Defines the methods of Numeric `n`, Integer `k` and Float `f`.
pub fn init(c: &mut Ctx, n: Value, k: Value, f: Value) {
    define_cfunc(c, n, "step", num_step, -1);
    define_cfunc(c, n, "floor", num_floor, -1);
    define_cfunc(c, n, "ceil", num_ceil, -1);
    define_cfunc(c, n, "round", num_round, -1);
    define_cfunc(c, n, "truncate", num_truncate, -1);
    define_cfunc(c, n, "integer?", num_integer_p, 0);
    define_cfunc(c, n, "zero?", num_zero_p, 0);
    define_cfunc(c, n, "nonzero?", num_nonzero_p, 0);
    define_cfunc(c, n, "positive?", num_positive_p, 0);
    define_cfunc(c, n, "negative?", num_negative_p, 0);
    define_cfunc(c, n, "abs", num_abs, 0);
    define_cfunc(c, n, "magnitude", num_abs, 0);
    define_cfunc(c, n, "+@", num_uplus, 0);
    define_cfunc(c, n, "dup", num_uplus, 0);
    define_cfunc(c, n, "clone", num_clone, -1);
    define_cfunc(c, n, "-@", num_uminus, 0);
    define_cfunc(c, n, "coerce", num_coerce, 1);
    define_cfunc(c, n, "div", num_div, 1);
    define_cfunc(c, n, "modulo", num_modulo, 1);
    define_cfunc(c, n, "%", num_modulo, 1);
    define_cfunc(c, n, "divmod", num_divmod, 1);
    define_cfunc(c, n, "fdiv", num_fdiv, 1);
    define_cfunc(c, n, "remainder", num_remainder, 1);
    define_cfunc(c, n, "quo", num_quo, 1);
    define_cfunc(c, n, "to_int", num_to_int, 0);
    define_cfunc(c, n, "numerator", num_numerator, 0);
    define_cfunc(c, n, "denominator", num_denominator, 0);
    define_cfunc(c, n, "singleton_method_added", num_sadded, 1);
    define_cfunc(c, n, "real?", num_real_p, 0);
    define_cfunc(c, n, "finite?", num_finite_p, 0);
    define_cfunc(c, n, "infinite?", num_infinite_p, 0);
    define_cfunc(c, n, "eql?", num_eql, 1);
    // num_cmp: 0 for the same object, else nil
    define_cfunc(c, n, "<=>", |_c, s, a, _b| Ok(if s == a[0] { Value::fix(0) } else { NIL }), 1);
    define_cfunc(c, n, "i", num_i, 0);
    define_cfunc(c, n, "to_c", num_to_c, 0);
    define_cfunc(c, n, "real", num_real, 0);
    define_cfunc(c, n, "imaginary", num_imag, 0);
    define_cfunc(c, n, "conjugate", num_conj, 0);
    crate::builtins::object::alias_builtin(c, n, "imag", "imaginary");
    crate::builtins::object::alias_builtin(c, n, "conj", "conjugate");
    define_cfunc(c, n, "rectangular", num_rect, 0);
    define_cfunc(c, n, "rect", num_rect, 0);
    define_cfunc(c, n, "arg", num_arg, 0);
    define_cfunc(c, n, "angle", num_arg, 0);
    define_cfunc(c, n, "phase", num_arg, 0);

    let d = |c: &mut Ctx, nm: &str, f: CFunc, a: i32| define_cfunc(c, k, nm, f, a);
    d(c, "+", int_plus, 1); d(c, "-", int_minus, 1); d(c, "*", int_mul, 1); d(c, "/", int_div, 1); d(c, "%", int_mod, 1);
    d(c, "modulo", int_mod, 1); d(c, "**", int_pow_m, 1); d(c, "pow", int_pow_m, -1); d(c, "-@", int_uminus, 0);
    d(c, "~", int_not, 0); d(c, "&", int_and, 1); d(c, "|", int_or, 1); d(c, "^", int_xor, 1); d(c, "<<", int_lshift, 1); d(c, ">>", int_rshift, 1);
    d(c, "[]", int_aref, -1); d(c, "==", int_eq, 1); d(c, "===", int_eq, 1); d(c, "<=>", int_cmp, 1);
    d(c, "<", int_lt, 1); d(c, "<=", int_le, 1); d(c, ">", int_gt, 1); d(c, ">=", int_ge, 1); d(c, "abs", int_abs, 0); crate::builtins::object::alias_builtin(c, k, "magnitude", "abs");
    d(c, "to_s", int_to_s, -1); crate::builtins::object::alias_builtin(c, k, "inspect", "to_s"); d(c, "to_f", int_to_f, 0); d(c, "to_i", int_to_i, 0); d(c, "to_int", int_to_i, 0);
    d(c, "chr", int_chr, -1); d(c, "ord", int_ord, 0); d(c, "times", int_times, 0); d(c, "upto", int_upto, 1); d(c, "downto", int_downto, 1);
    d(c, "succ", int_succ, 0); d(c, "next", int_succ, 0); d(c, "pred", int_pred, 0); d(c, "zero?", int_zero, 0); d(c, "even?", int_even, 0);
    d(c, "odd?", int_odd, 0); d(c, "divmod", int_divmod, 1);
    d(c, "div", int_idiv, 1); d(c, "fdiv", int_fdiv, 1); d(c, "remainder", int_remainder, 1); d(c, "gcd", int_gcd, 1); d(c, "lcm", int_lcm, 1);
    d(c, "round", int_round, -1); d(c, "floor", int_floor, -1); d(c, "ceil", int_ceil, -1); d(c, "truncate", int_truncate, -1);
    d(c, "digits", int_digits, -1); d(c, "bit_length", int_bit_length, 0); d(c, "size", int_size, 0);
    d(c, "coerce", int_coerce, 1); d(c, "allbits?", int_allbits, 1); d(c, "anybits?", int_anybits, 1); d(c, "nobits?", int_nobits, 1);
    d(c, "integer?", int_integer_p, 0); d(c, "ceildiv", int_ceildiv, 1); d(c, "gcdlcm", int_gcdlcm, 1);
    define_singleton_cfunc(c, k, "sqrt", int_s_sqrt, 1);
    define_singleton_cfunc(c, k, "try_convert", int_s_try_convert, 1);
    // rb_cmpint for the prelude's Comparable#==
    let im = c.vm().m_internal.get();
    define_singleton_cfunc(c, im, "__cmpint", |c, _s, a, _b| Ok(Value::fix(crate::builtins::array_methods::cmp_result(c, a[0], NIL, NIL)?)), 1);
    // rb_undef_method(CLASS_OF(rb_cInteger / rb_cFloat), "new")
    for kk in [k, f] { let sk = singleton_class(c, kk); define_method_me(c, sk, s::new, new_me(s::new, sk, VISI_PUBLIC, MKind::Undef)); }

    let d = |c: &mut Ctx, nm: &str, fun: CFunc, a: i32| define_cfunc(c, f, nm, fun, a);
    d(c, "+", flo_plus, 1); d(c, "-", flo_minus, 1); d(c, "*", flo_mul, 1); d(c, "/", flo_div, 1); d(c, "%", flo_mod, 1); d(c, "modulo", flo_mod, 1);
    d(c, "**", flo_pow, 1); d(c, "-@", flo_uminus, 0); d(c, "==", flo_eq, 1); d(c, "===", flo_eq, 1); d(c, "eql?", flo_eql, 1);
    d(c, "<=>", flo_cmp, 1); d(c, "<", flo_lt, 1); d(c, "<=", flo_le, 1); d(c, ">", flo_gt, 1); d(c, ">=", flo_ge, 1);
    d(c, "to_s", flo_to_s, 0); crate::builtins::object::alias_builtin(c, f, "inspect", "to_s"); d(c, "to_i", flo_to_i, 0); d(c, "to_int", flo_to_i, 0);
    d(c, "abs", flo_abs, 0); crate::builtins::object::alias_builtin(c, f, "magnitude", "abs"); d(c, "nan?", flo_nan, 0); d(c, "infinite?", flo_infinite, 0); d(c, "finite?", flo_finite, 0);
    d(c, "zero?", flo_zero, 0); d(c, "positive?", flo_positive, 0); d(c, "negative?", flo_negative, 0); d(c, "divmod", flo_divmod, 1);
    d(c, "coerce", flo_coerce, 1); d(c, "hash", flo_hash, 0); d(c, "round", flo_round, -1); d(c, "floor", flo_floor, -1);
    d(c, "ceil", flo_ceil, -1); d(c, "truncate", flo_truncate, -1); d(c, "next_float", flo_next_float, 0); d(c, "prev_float", flo_prev_float, 0);
    d(c, "to_f", |_c, s, _a, _b| Ok(s), 0);
    d(c, "fdiv", flo_div, 1);
    d(c, "quo", flo_div, 1);
    for (nm, v) in [("DIG", 15), ("MANT_DIG", 53), ("MAX_EXP", 1024), ("MIN_EXP", -1021), ("MAX_10_EXP", 308), ("MIN_10_EXP", -307), ("RADIX", 2)] {
        const_set(c, f, intern(nm.as_bytes()), Value::fix(v));
    }
}
