// Array core API on the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
// Methods are in builtins/array_methods.rs.
#![forbid(unsafe_code)]
pub use crate::heap::{obj_clear_flags, obj_flags, obj_klass, obj_set_flags, obj_set_klass};
// the legacy accessors, for the modules not converted yet
pub use crate::object::*;
pub use imp::*;

// a module of its own: `no_legacy` must not meet the re-export above
mod imp {
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
use crate::heap::obj_flags;
use crate::avec::AVec;
use crate::object::*;
use crate::value::*;
use crate::vm::*;

impl Ctx {
    #[inline]
    pub fn ary_new(&mut self) -> Value { let k = self.vm().c_array; self.alloc_with(k, move || Body::Array(AVec::new())) }
    #[inline]
    pub fn ary_new_capa(&mut self, n: usize) -> Value {
        self.malloc_increase(n * 8);
        let k = self.vm().c_array;
        self.alloc_with(k, move || Body::Array(AVec::with_capacity(n)))
    }
    #[inline]
    pub fn ary_new_from(&mut self, v: &[Value]) -> Value {
        self.malloc_increase(v.len() * 8);
        let k = self.vm().c_array;
        self.alloc_with(k, move || Body::Array(AVec::from_slice(v)))
    }
    /// An Array of the first n (<= avec::INLINE) of `w`.
    #[inline(always)]
    pub fn ary_new_small(&mut self, n: usize, w: [Value; crate::avec::INLINE]) -> Value {
        let k = self.vm().c_array;
        self.alloc_with(k, move || Body::Array(AVec::small(n, w)))
    }
    /// An Array of the values of stack slots (which keep them rooted meanwhile).
    #[inline]
    pub fn ary_from_slots(&mut self, s: &[std::cell::Cell<Value>]) -> Value {
        if s.len() > crate::avec::INLINE { return self.ary_from_vec(s.iter().map(|x| x.get()).collect()); }
        let k = self.vm().c_array;
        self.alloc_with(k, || Body::Array(AVec::from_slots(s)))
    }
    /// The Vec becomes the Array's payload; its Values are roots during the allocation.
    #[inline]
    pub fn ary_from_vec(&mut self, v: Vec<Value>) -> Value {
        self.malloc_increase(v.len() * 8);
        let k = self.vm().c_array;
        if v.len() <= crate::avec::INLINE { return self.alloc_with(k, || Body::Array(AVec::from_slice(&v))); }
        // empty first, then the Vec stored field by field: moving a ready Body::Array into the slot
        // is compiled as an unaligned copy from the stack that stalls on its own stores
        let a = self.alloc_with(k, || Body::Array(AVec::new()));
        std::mem::forget(std::mem::replace(self.ary_mut(a), AVec::Heap(v)));
        a
    }
    #[inline(always)]
    pub fn ary_push(&mut self, a: Value, v: Value) {
        if !self.ary_mut(a).try_push(v) { self.ary_push_grow(a, v); }
    }
    #[inline(never)]
    fn ary_push_grow(&mut self, a: Value, v: Value) {
        let x = self.ary_mut(a);
        let grow = if x.len() == x.capacity() { x.len() * 8 + 32 } else { 0 };
        x.push(v);
        if grow != 0 { self.malloc_increase(grow); }
    }
    #[inline]
    pub fn ary_len(&self, a: Value) -> usize { self.ary(a).len() }
    /// Element i, nil beyond the end.
    #[inline]
    pub fn ary_get(&self, a: Value, i: usize) -> Value { self.ary(a).get(i).copied().unwrap_or(NIL) }
    /// Append the elements of `src` to `dst`; they may be the same Array.
    pub fn ary_concat(&mut self, dst: Value, src: Value) {
        let (d, others) = self.ary_mut_and(dst);
        match others.ary(src) { Some(s) => d.extend_from_slice(s), None => d.extend_from_within(..) }
    }
    /// Array `v` for writing; FrozenError when frozen.
    #[inline]
    pub fn ary_modify(&mut self, v: Value) -> Result<&mut AVec, Esc> {
        if obj_flags(v) & FL_FROZEN != 0 { return Err(raise_frozen(self, v)); }
        Ok(self.ary_mut(v))
    }
}

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
}
