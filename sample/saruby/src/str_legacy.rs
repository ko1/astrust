// Legacy String API (no context), for the modules not converted to the context-borrowing
// accessors yet (docs/safe_heap.md); converted code uses the `Ctx` methods of string.rs and
// cannot name these (`heap::no_legacy_str`).
use crate::object::*;
use crate::sbuf::SBuf;
use crate::string::{char_offsets_of, enc_strlen};
use crate::symbol::intern;
use crate::value::*;
use crate::vm::vm;

// bytes kept inside the slot are not malloc'ed: only longer strings count toward the GC trigger
#[inline(always)]
fn account(n: usize) { if n > crate::sbuf::INLINE { crate::gc::malloc_increase(n); } }
#[inline]
pub fn str_new(b: &[u8]) -> Value {
    account(b.len());
    crate::gc::alloc_with(vm().c_string, move || Body::String(RString { b: SBuf::from_slice(b), enc: ENC_UTF8 }))
}
#[inline]
pub fn str_new_enc(b: &[u8], enc: u8) -> Value {
    account(b.len());
    crate::gc::alloc_with(vm().c_string, move || Body::String(RString { b: SBuf::from_slice(b), enc }))
}
#[inline]
pub fn str_with_capa(n: usize, enc: u8) -> Value {
    account(n);
    crate::gc::alloc_with(vm().c_string, move || Body::String(RString { b: SBuf::with_capacity(n), enc }))
}
pub fn str_from_vec(b: Vec<u8>) -> Value {
    account(b.len());
    crate::gc::alloc_with(vm().c_string, move || Body::String(RString { b: SBuf::from_vec(b), enc: ENC_UTF8 }))
}
pub fn str_from_vec_enc(b: Vec<u8>, enc: u8) -> Value {
    account(b.len());
    crate::gc::alloc_with(vm().c_string, move || Body::String(RString { b: SBuf::from_vec(b), enc }))
}
pub fn str_new_frozen(b: &[u8]) -> Value {
    let v = str_new(b);
    obj(v).flags |= FL_FROZEN;
    v
}
pub fn str_enc(v: Value) -> u8 { rstr(v).enc }
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

/// Char boundaries (byte offsets) for a string, honoring its encoding.
pub fn char_offsets(v: Value) -> Vec<usize> { let r = rstr(v); char_offsets_of(&r.b, r.enc) }
pub fn str_char_len(v: Value) -> usize { let r = rstr(v); enc_strlen(&r.b, r.enc) }
pub fn str_dup(v: Value) -> Value {
    let r = rstr(v);
    let n = str_from_vec_enc(r.b.to_vec(), r.enc);
    obj(n).klass = obj(v).klass;
    n
}

pub fn ary_of_strs(items: Vec<Vec<u8>>, enc: u8) -> Value {
    let a = ary_new_capa(items.len());
    for it in items { let s = str_from_vec_enc(it, enc); ary_push(a, s); }
    a
}
