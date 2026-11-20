// Numeric core helpers (Float boxing, conversions, formatting).
use crate::object::*;
use crate::value::*;
use crate::vm::*;

#[inline]
pub fn float_new(d: f64) -> Value {
    match Value::flonum(d) { Some(v) => v, None => float_new_heap(d) }
}
#[cold]
#[inline(never)]
fn float_new_heap(d: f64) -> Value {
    let v = crate::gc::alloc(vm().c_float, Body::Float(d));
    obj(v).flags |= FL_FROZEN;
    v
}

// out of line: with a constant exponent LLVM turns powf(x, 0.5) into sqrt(x), which is not libm's pow bit for bit
#[inline(never)]
pub fn libm_pow(x: f64, y: f64) -> f64 { x.powf(std::hint::black_box(y)) }

/// Float#to_s
pub fn float_to_s(d: f64) -> String {
    if d.is_nan() { return "NaN".into(); }
    if d.is_infinite() { return if d > 0.0 { "Infinity".into() } else { "-Infinity".into() }; }
    if d == 0.0 { return if d.is_sign_negative() { "-0.0".into() } else { "0.0".into() }; }
    // shortest round-trip digits via Rust's formatter in exponent form
    let e = format!("{:e}", d);
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let sign = if neg { "-" } else { "" };
    let decpt = exp + 1;
    if decpt > -4 && (decpt <= 0 || (decpt as usize) < digits.len() || decpt <= 15) {
        // fixed notation
        let decpt = exp + 1;
        let s = if decpt <= 0 {
            format!("0.{}{}", "0".repeat((-decpt) as usize), digits)
        } else if decpt as usize >= digits.len() {
            format!("{}{}.0", digits, "0".repeat(decpt as usize - digits.len()))
        } else {
            format!("{}.{}", &digits[..decpt as usize], &digits[decpt as usize..])
        };
        format!("{}{}", sign, s)
    } else {
        let frac = if digits.len() > 1 { &digits[1..] } else { "0" };
        format!("{}{}.{}e{}{:02}", sign, &digits[..1], frac, if exp < 0 { "-" } else { "+" }, exp.abs())
    }
}

/// Numeric value as f64 (Integer / Float), None otherwise.
pub fn num_to_f64(v: Value) -> Option<f64> {
    if v.is_fixnum() { Some(v.as_fix() as f64) }
    else if is_float(v) { Some(float_val(v)) }
    else if is_bignum(v) { Some(rbig(v).to_f64()) }
    else { None }
}

pub fn is_numeric(v: Value) -> bool { v.is_fixnum() || is_float(v) || is_bignum(v) }
