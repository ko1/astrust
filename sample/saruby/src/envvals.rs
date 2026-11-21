// The values of an Env.  The first INLINE live in the GC slot, the rest (if any) in one boxed Vec,
// so an Env of up to INLINE values needs no malloc and frees nothing at sweep.  Baked code indexes
// with a constant, so the inline/out-of-line choice folds away.  3 words: Body stays 40 bytes.
#![forbid(unsafe_code)]
use crate::value::*;

pub const INLINE: usize = 2;

#[derive(Clone)]
pub struct EnvVals { head: [Value; INLINE], rest: Option<Box<Vec<Value>>> }

const _: () = assert!(std::mem::size_of::<EnvVals>() == 24);

impl EnvVals {
    /// n values, all nil.
    #[inline(always)]
    pub fn new(n: usize) -> EnvVals {
        EnvVals { head: [NIL; INLINE], rest: if n > INLINE { Some(Box::new(vec![NIL; n - INLINE])) } else { None } }
    }
    #[inline(always)]
    pub fn is_heap(&self) -> bool { self.rest.is_some() }
    #[inline(always)]
    pub fn get(&self, i: usize) -> Value {
        if i < INLINE { self.head[i] } else { self.rest_get(i) }
    }
    #[inline(always)]
    pub fn set(&mut self, i: usize, v: Value) {
        if i < INLINE { self.head[i] = v } else { self.rest_set(i, v) }
    }
    fn rest_get(&self, i: usize) -> Value { self.rest.as_ref().map_or(NIL, |r| r[i - INLINE]) }
    fn rest_set(&mut self, i: usize, v: Value) { if let Some(r) = &mut self.rest { r[i - INLINE] = v } }
    /// Every value (inline slots past a small Env's size hold nil).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        self.head.iter_mut().chain(self.rest.iter_mut().flat_map(|r| r.iter_mut()))
    }
}
