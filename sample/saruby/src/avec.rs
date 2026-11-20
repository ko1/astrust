// Array elements with the Vec<Value> operations Array needs.  Up to INLINE elements live in the
// GC slot (no malloc, nothing to free at sweep); a longer array, or one that needed a general Vec
// operation, owns a Vec and keeps it.  The inline length is an enum whose unused values hold the
// Inline/Heap tag, so the whole thing is 4 words and Body stays 40 bytes.
#![forbid(unsafe_code)]
use crate::value::*;
use std::ops::{Deref, DerefMut, RangeBounds};

pub const INLINE: usize = 3;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Len { L0, L1, L2, L3 }

impl Len {
    #[inline(always)]
    fn of(n: usize) -> Len { match n { 0 => Len::L0, 1 => Len::L1, 2 => Len::L2, _ => Len::L3 } }
}

pub enum AVec { Inline(Len, [Value; INLINE]), Heap(Vec<Value>) }

const _: () = assert!(std::mem::size_of::<AVec>() == 32);

impl AVec {
    #[inline(always)]
    pub const fn new() -> AVec { AVec::Inline(Len::L0, [NIL; INLINE]) }
    #[inline]
    pub fn with_capacity(n: usize) -> AVec { if n <= INLINE { AVec::new() } else { AVec::Heap(Vec::with_capacity(n)) } }
    #[inline(always)]
    pub fn from_slice(s: &[Value]) -> AVec {
        match *s {
            [] => AVec::new(),
            [a] => AVec::Inline(Len::L1, [a, NIL, NIL]),
            [a, b] => AVec::Inline(Len::L2, [a, b, NIL]),
            [a, b, c] => AVec::Inline(Len::L3, [a, b, c]),
            _ => AVec::Heap(s.to_vec()),
        }
    }
    /// The first n (<= INLINE) of `w`; the rest of `w` is ignored.
    #[inline(always)]
    pub fn small(n: usize, w: [Value; INLINE]) -> AVec { AVec::Inline(Len::of(n), w) }
    /// The values of stack slots (an Array literal).
    #[inline(always)]
    pub fn from_slots(s: &[std::cell::Cell<Value>]) -> AVec {
        match s {
            [] => AVec::new(),
            [a] => AVec::Inline(Len::L1, [a.get(), NIL, NIL]),
            [a, b] => AVec::Inline(Len::L2, [a.get(), b.get(), NIL]),
            [a, b, c] => AVec::Inline(Len::L3, [a.get(), b.get(), c.get()]),
            _ => AVec::Heap(s.iter().map(|x| x.get()).collect()),
        }
    }
    /// A short Vec is copied into the slot and freed now.
    #[inline]
    pub fn from_vec(v: Vec<Value>) -> AVec { if v.len() <= INLINE { AVec::from_slice(&v) } else { AVec::Heap(v) } }
    /// True when the elements are out of line.
    #[inline(always)]
    pub fn is_heap(&self) -> bool { matches!(self, AVec::Heap(_)) }
    /// The elements as a Vec, for the operations not provided here (the array stays out of line).
    #[inline]
    pub fn vec(&mut self) -> &mut Vec<Value> {
        if let AVec::Inline(..) = self { self.spill(); }
        match self { AVec::Heap(v) => v, AVec::Inline(..) => unreachable!() }
    }
    #[cold]
    #[inline(never)]
    fn spill(&mut self) {
        let mut v = Vec::with_capacity(8);
        v.extend_from_slice(self);
        *self = AVec::Heap(v);
    }
    #[inline]
    pub fn capacity(&self) -> usize { match self { AVec::Inline(..) => INLINE, AVec::Heap(v) => v.capacity() } }
    #[inline]
    pub fn push(&mut self, x: Value) {
        match self {
            AVec::Heap(v) => v.push(x),
            AVec::Inline(n, w) if (*n as usize) < INLINE => { w[*n as usize] = x; *n = Len::of(*n as usize + 1); }
            _ => self.vec().push(x),
        }
    }
    /// push when there is room without allocating; false otherwise (nothing done).
    #[inline(always)]
    pub fn try_push(&mut self, x: Value) -> bool {
        match self {
            AVec::Heap(v) if v.len() < v.capacity() => { v.push(x); true }
            AVec::Inline(n, w) if (*n as usize) < INLINE => { w[*n as usize] = x; *n = Len::of(*n as usize + 1); true }
            _ => false,
        }
    }
    #[inline]
    pub fn pop(&mut self) -> Option<Value> {
        match self {
            AVec::Inline(n, w) => {
                let k = (*n as usize).checked_sub(1)?;
                *n = Len::of(k);
                Some(w[k])
            }
            AVec::Heap(v) => v.pop(),
        }
    }
    #[inline]
    pub fn truncate(&mut self, k: usize) {
        match self {
            AVec::Inline(n, _) => if k < *n as usize { *n = Len::of(k); },
            AVec::Heap(v) => v.truncate(k),
        }
    }
    #[inline]
    pub fn clear(&mut self) { self.truncate(0); }
    #[inline]
    pub fn extend_from_slice(&mut self, s: &[Value]) {
        if self.len() + s.len() <= INLINE { for &x in s { self.push(x); } } else { self.vec().extend_from_slice(s); }
    }
    /// Append a copy of elements `r` of self.
    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, r: R) { self.vec().extend_from_within(r); }
    pub fn extend<I: IntoIterator<Item = Value>>(&mut self, it: I) { self.vec().extend(it); }
    pub fn insert(&mut self, i: usize, x: Value) { self.vec().insert(i, x); }
    pub fn remove(&mut self, i: usize) -> Value { self.vec().remove(i) }
    pub fn retain<F: FnMut(&Value) -> bool>(&mut self, f: F) { self.vec().retain(f); }
    pub fn drain<R: RangeBounds<usize>>(&mut self, r: R) -> std::vec::Drain<'_, Value> { self.vec().drain(r) }
    pub fn resize(&mut self, n: usize, x: Value) { if n <= self.len() { self.truncate(n); } else { self.vec().resize(n, x); } }
    pub fn splice<R: RangeBounds<usize>, I: IntoIterator<Item = Value>>(&mut self, r: R, with: I) { self.vec().splice(r, with); }
    pub fn split_off(&mut self, at: usize) -> Vec<Value> { self.vec().split_off(at) }
    pub fn reserve(&mut self, n: usize) { if self.len() + n > self.capacity() { self.vec().reserve(n); } }
}

impl Deref for AVec {
    type Target = [Value];
    #[inline(always)]
    fn deref(&self) -> &[Value] {
        // the Heap path falls through: long arrays are the ones indexed in loops
        match self { AVec::Heap(v) => v, AVec::Inline(n, w) => { std::hint::cold_path(); &w[..*n as usize] } }
    }
}
impl DerefMut for AVec {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [Value] {
        match self { AVec::Heap(v) => v, AVec::Inline(n, w) => { std::hint::cold_path(); &mut w[..*n as usize] } }
    }
}
impl Clone for AVec {
    #[inline]
    fn clone(&self) -> AVec { AVec::from_slice(self) }
}
impl Default for AVec { fn default() -> AVec { AVec::new() } }
impl From<Vec<Value>> for AVec { #[inline] fn from(v: Vec<Value>) -> AVec { AVec::from_vec(v) } }
impl<'a> IntoIterator for &'a AVec {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;
    fn into_iter(self) -> Self::IntoIter { self.iter() }
}
impl<'a> IntoIterator for &'a mut AVec {
    type Item = &'a mut Value;
    type IntoIter = std::slice::IterMut<'a, Value>;
    fn into_iter(self) -> Self::IntoIter { self.iter_mut() }
}
