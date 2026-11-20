// Array core API.  Methods are in builtins/array_methods.rs.
pub use crate::object::*;
use crate::value::*;
use crate::vm::*;

#[inline]
pub fn ary_new() -> Value { crate::gc::alloc(vm().c_array, Body::Array(Vec::new())) }
#[inline]
pub fn ary_new_capa(n: usize) -> Value {
    crate::gc::malloc_increase(n * 8);
    crate::gc::alloc(vm().c_array, Body::Array(Vec::with_capacity(n)))
}
#[inline]
pub fn ary_new_from(v: &[Value]) -> Value {
    crate::gc::malloc_increase(v.len() * 8);
    crate::gc::alloc(vm().c_array, Body::Array(v.to_vec()))
}
pub fn ary_from_vec(v: Vec<Value>) -> Value {
    crate::gc::malloc_increase(v.len() * 8);
    crate::gc::alloc(vm().c_array, Body::Array(v))
}
#[inline]
pub fn ary_push(a: Value, v: Value) {
    let x = rary(a);
    if x.len() == x.capacity() { crate::gc::malloc_increase(x.len() * 8 + 32); }
    x.push(v);
}
#[inline] pub fn ary_len(a: Value) -> usize { rary(a).len() }
#[inline] pub fn ary_get(a: Value, i: usize) -> Value { rary(a).get(i).copied().unwrap_or(NIL) }
pub fn ary_to_vec(a: Value) -> Vec<Value> { rary(a).clone() }

/// Implicit conversion to Array (to_ary); None when not convertible.
pub fn check_array(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if is_array(v) { return Ok(Some(v)); }
    if v.is_heap() && respond_to(v, crate::symbol::s::to_ary, true) {
        let r = funcall(c, v, crate::symbol::s::to_ary, &[], std::ptr::null())?;
        if is_array(r) { return Ok(Some(r)); }
        if r == NIL { return Ok(None); }
        let cls = class_path(real_class_of(v));
        return type_err(c, format!("can't convert {} to Array ({}#to_ary gives {})", cls, cls, class_path(real_class_of(r))));
    }
    Ok(None)
}
pub fn expect_array(c: &mut Ctx, v: Value) -> RV {
    match check_array(c, v)? {
        Some(a) => Ok(a),
        None => type_err(c, format!("no implicit conversion of {} into Array", crate::builtins::kernel::class_name_for_err(v))),
    }
}
