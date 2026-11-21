// Instance-variable slots of an object.  Slot i belongs to the class's i-th ivar; unset = UNDEF.
//
//   up to INLINE slots   in the GC slot itself (no malloc)
//   more                 all of them in one heap block (the inline ones are copied there when it is made)
//
// One test (is there a block?) and one bounds check reach any slot, whatever its index: a site reads
// the index from its cache at run time (a baked index would depend on the program's ivar order).
// No unsafe, and no self pointer (the value may be moved, as a moving collector would).
use crate::value::*;

pub const INLINE: usize = 2;

pub struct IvBuf { w: [Value; INLINE], ext: Option<Box<[Value]>> }

impl IvBuf {
    /// At least n slots, all unset.
    #[inline(always)]
    pub fn filled(n: usize) -> IvBuf {
        IvBuf { w: [UNDEF; INLINE], ext: if n <= INLINE { None } else { Some(ext_filled(n)) } }
    }
    /// True when every slot is in the GC slot (nothing to free).
    #[inline(always)]
    pub fn is_inline(&self) -> bool { self.ext.is_none() }
    #[inline]
    pub fn len(&self) -> usize {
        match &self.ext { None => INLINE, Some(e) => e.len() }
    }
    /// Slot i (UNDEF: unset); None beyond len().
    #[inline(always)]
    pub fn get(&self, i: usize) -> Option<Value> {
        match &self.ext { None => self.w.get(i).copied(), Some(e) => e.get(i).copied() }
    }
    /// Store into an existing slot; false beyond len().
    #[inline(always)]
    pub fn set(&mut self, i: usize, v: Value) -> bool {
        match self.slot_mut(i) { Some(x) => { *x = v; true } None => false }
    }
    /// Slot i in place (a read and a write with one test); None beyond len().
    #[inline(always)]
    pub fn slot_mut(&mut self, i: usize) -> Option<&mut Value> {
        match &mut self.ext { None => self.w.get_mut(i), Some(e) => e.get_mut(i) }
    }
    /// Make it at least n slots long (new slots unset).
    #[inline(never)]
    pub fn resize(&mut self, n: usize) {
        if n <= self.len() { return; }
        let mut e = ext_filled(n);
        for (i, v) in self.iter().enumerate() { e[i] = v; }
        self.ext = Some(e);
        self.w = [UNDEF; INLINE];
    }
    /// The slots in index order (unset ones as UNDEF).
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Value> + '_ {
        match &self.ext { None => self.w.iter(), Some(e) => e.iter() }.copied()
    }
    /// The slots in place (a moving collector's edge update).
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Value> + '_ {
        match &mut self.ext { None => self.w.iter_mut(), Some(e) => e.iter_mut() }
    }
}

// out of line: an allocation, not on a site's path
#[inline(never)]
fn ext_filled(n: usize) -> Box<[Value]> { vec![UNDEF; n].into_boxed_slice() }

impl Clone for IvBuf {
    fn clone(&self) -> IvBuf { IvBuf { w: self.w, ext: self.ext.clone() } }
}
