// Legacy Range / numeric API (no context), for the modules not converted yet (docs/safe_heap.md).
// Converted code uses the `Ctx` methods of range.rs, numeric.rs, bignum.rs.
use crate::heap::{obj_set_flags, obj_set_klass};
use crate::object::*;
use crate::value::*;

pub fn range_new(b: Value, e: Value, excl: bool) -> Value { crate::gc::alloc(crate::vm::vm().c_range.get(), Body::Range(b, e, excl)) }

#[inline]
pub fn float_new(d: f64) -> Value {
    match Value::flonum(d) { Some(v) => v, None => float_new_heap(d) }
}
#[cold]
#[inline(never)]
fn float_new_heap(d: f64) -> Value {
    let v = crate::gc::alloc(crate::vm::vm().c_float.get(), Body::Float(d));
    obj_set_flags(v, FL_FROZEN);
    v
}

pub fn big_to_value(b: crate::bignum::BigInt) -> Value {
    if let Some(i) = b.to_i64() { if Value::fixable(i) { return Value::fix(i); } }
    let v = crate::gc::alloc(crate::vm::vm().c_integer.get(), Body::Bignum(Box::new(b)));
    obj_set_flags(v, FL_FROZEN);
    v
}

pub fn int_from_i128(i: i128) -> Value {
    if i >= FIXNUM_MIN as i128 && i <= FIXNUM_MAX as i128 { Value::fix(i as i64) } else { big_to_value(crate::bignum::BigInt::from_i128(i)) }
}
pub fn int_from_i64(i: i64) -> Value { if Value::fixable(i) { Value::fix(i) } else { big_to_value(crate::bignum::BigInt::from_i64(i)) } }
/// Float truncated to an Integer.
pub fn f64_to_int(f: f64) -> Value {
    let t = f.trunc();
    if t.abs() < 4.0e18 { int_from_i64(t as i64) } else { big_to_value(crate::bignum::BigInt::from_f64(t)) }
}
/// Recursion guard of inspect / hash / == (VM state only, no payload).
pub fn recursion_enter(v: Value, tag: crate::value::Sym) -> bool {
    let mut g = crate::vm::vm().recursive_guard.borrow_mut();
    if g.iter().any(|&(x, t)| x == v.raw() && t == tag) { return false; }
    g.push((v.raw(), tag));
    true
}
pub fn recursion_leave(v: Value, tag: crate::value::Sym) {
    let mut g = crate::vm::vm().recursive_guard.borrow_mut();
    if let Some(p) = g.iter().rposition(|&(x, t)| x == v.raw() && t == tag) { g.remove(p); }
}

// ---- boot-time class setup for builtins/boot.rs (no context exists yet) ----
/// `m` becomes the metaclass of `k`.
pub fn boot_metaclass(k: Value, m: Value) {
    let d = rclass(m);
    d.is_singleton = true; d.attached = k;
    obj_set_klass(k, m);
}
pub fn boot_subclass(sup: Value, k: Value) { rclass(sup).subclasses.push(k); }
/// Constants `names` of `owner` autoload the native `feature`.
pub fn boot_autoload(owner: Value, names: &[&str], feature: &str) {
    let path = crate::str_legacy::str_new_frozen(feature.as_bytes());
    for n in names { rclass(owner).autoload.insert(crate::symbol::intern(n.as_bytes()), path); }
}
