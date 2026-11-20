// Legacy Range / numeric API (no context), for the modules not converted yet (docs/safe_heap.md).
// Converted code uses the `Ctx` methods of range.rs, numeric.rs, bignum.rs.
use crate::object::*;
use crate::value::*;

pub fn range_new(b: Value, e: Value, excl: bool) -> Value { crate::gc::alloc(crate::vm::vm().c_range, Body::Range(b, e, excl)) }
#[inline]
pub fn range_parts(r: Value) -> (Value, Value, bool) {
    match &obj(r).body { Body::Range(a, b, x) => (*a, *b, *x), _ => (NIL, NIL, false) }
}

#[inline]
pub fn float_new(d: f64) -> Value {
    match Value::flonum(d) { Some(v) => v, None => float_new_heap(d) }
}
#[cold]
#[inline(never)]
fn float_new_heap(d: f64) -> Value {
    let v = crate::gc::alloc(crate::vm::vm().c_float, Body::Float(d));
    obj(v).flags |= FL_FROZEN;
    v
}
/// Numeric value as f64 (Integer / Float), None otherwise.
pub fn num_to_f64(v: Value) -> Option<f64> {
    if v.is_fixnum() { Some(v.as_fix() as f64) }
    else if is_float(v) { Some(float_val(v)) }
    else if is_bignum(v) { Some(rbig(v).to_f64()) }
    else { None }
}
pub fn big_to_value(b: crate::bignum::BigInt) -> Value {
    if let Some(i) = b.to_i64() { if Value::fixable(i) { return Value::fix(i); } }
    let v = crate::gc::alloc(crate::vm::vm().c_integer, Body::Bignum(Box::new(b)));
    obj(v).flags |= FL_FROZEN;
    v
}
pub fn to_big(v: Value) -> crate::bignum::BigInt {
    if v.is_fixnum() { crate::bignum::BigInt::from_i64(v.as_fix()) } else { rbig(v).clone() }
}
pub fn int_from_i128(i: i128) -> Value {
    if i >= FIXNUM_MIN as i128 && i <= FIXNUM_MAX as i128 { Value::fix(i as i64) } else { big_to_value(crate::bignum::BigInt::from_i128(i)) }
}
pub fn int_from_i64(i: i64) -> Value { if Value::fixable(i) { Value::fix(i) } else { big_to_value(crate::bignum::BigInt::from_i64(i)) } }
