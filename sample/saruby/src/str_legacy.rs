// Legacy String API (no context), for the modules not converted to the context-borrowing
// accessors yet (docs/safe_heap.md); converted code uses the `Ctx` methods of string.rs and
// cannot name these (`heap::no_legacy_str`).
use crate::object::*;
use crate::sbuf::SBuf;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::vm;

// bytes kept inside the slot are not malloc'ed: only longer strings count toward the GC trigger
#[inline(always)]
fn account(n: usize) { if n > crate::sbuf::INLINE { crate::gc::malloc_increase(n); } }
#[inline]
pub fn str_new(b: &[u8]) -> Value {
    account(b.len());
    crate::gc::alloc_with(vm().c_string.get(), move || Body::String(RString { b: SBuf::from_slice(b), enc: ENC_UTF8 }))
}
#[inline]
pub fn str_new_enc(b: &[u8], enc: u8) -> Value {
    account(b.len());
    crate::gc::alloc_with(vm().c_string.get(), move || Body::String(RString { b: SBuf::from_slice(b), enc }))
}
#[inline]
pub fn str_with_capa(n: usize, enc: u8) -> Value {
    account(n);
    crate::gc::alloc_with(vm().c_string.get(), move || Body::String(RString { b: SBuf::with_capacity(n), enc }))
}
pub fn str_new_frozen(b: &[u8]) -> Value {
    let v = str_new(b);
    obj(v).flags |= FL_FROZEN;
    v
}
pub fn str_eq(a: Value, b: Value) -> bool { rstr(a).b == rstr(b).b }

pub fn str_to_sym(v: Value) -> Value { Value::sym(intern(&rstr(v).b)) }

/// acc << s for a mutable acc of the same encoding (nothing to check).
#[inline]
pub fn str_append_same_enc(acc: Value, s: Value) {
    if acc == s { let b = rstr(s).b.to_vec(); rstr(acc).b.extend_from_slice(&b); return; }
    let (a, b) = (rstr(acc), rstr(s));
    crate::gc::malloc_increase(b.b.len());
    a.b.extend_from_slice(&b.b);
}


