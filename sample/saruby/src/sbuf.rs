// String byte buffer with the Vec<u8> operations String needs, and one spare byte (`aux`: the
// String's encoding) kept by every operation.  Up to INLINE bytes live in the object slot (no
// malloc), longer ones in a Vec.  repr(u8): a tag byte, then `aux` at offset 1 in both variants (aux() is
// one load, no match); 4 words with the encoding inside, so Body stays 40 bytes.
#![forbid(unsafe_code)]
use std::ops::{Bound, Deref, DerefMut, RangeBounds};

pub const INLINE: usize = 29;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Len { L0, L1, L2, L3, L4, L5, L6, L7, L8, L9, L10, L11, L12, L13, L14, L15, L16, L17, L18, L19, L20, L21, L22, L23, L24, L25, L26, L27, L28, L29 }
use Len::*;
const LENS: [Len; INLINE + 1] = [L0, L1, L2, L3, L4, L5, L6, L7, L8, L9, L10, L11, L12, L13, L14, L15, L16, L17, L18, L19, L20, L21, L22, L23, L24, L25, L26, L27, L28, L29];
impl Len {
    // a table load: a match compiles to a jump table
    #[inline(always)]
    fn of(n: usize) -> Len { LENS[n] }
}
const _: () = assert!(Len::L29 as usize == INLINE);

#[repr(u8)]
// Heap = 0: `!is_heap()` is then `tag != 0`, which keeps the GC sweep's other arms a plain cmp + branch
pub enum SBuf { Heap { aux: u8, v: Vec<u8> }, Inline { aux: u8, len: Len, b: [u8; INLINE] } }

const _: () = assert!(std::mem::size_of::<SBuf>() == 32);

impl SBuf {
    #[inline(always)]
    pub const fn new() -> SBuf { SBuf::Inline { len: Len::L0, aux: 0, b: [0; INLINE] } }
    #[inline(always)]
    pub fn is_heap(&self) -> bool { matches!(self, SBuf::Heap { .. }) }
    #[inline(always)]
    pub fn aux(&self) -> u8 { match self { SBuf::Inline { aux, .. } | SBuf::Heap { aux, .. } => *aux } }
    #[inline(always)]
    pub fn set_aux(&mut self, x: u8) { match self { SBuf::Inline { aux, .. } | SBuf::Heap { aux, .. } => *aux = x } }
    /// Replace the bytes, keep `aux`.
    #[inline]
    pub fn set_bytes(&mut self, mut b: SBuf) { b.set_aux(self.aux()); *self = b; }

    // built in place: a separate [u8; 29] would be copied into the enum piecewise (unaligned reloads)
    #[inline(always)]
    pub fn from_slice(s: &[u8]) -> SBuf { if s.len() <= INLINE { SBuf::inline_from(s) } else { SBuf::heap_from(s) } }
    /// `s` (at most INLINE bytes) as an Inline buffer.
    #[inline(always)]
    pub fn inline_from(s: &[u8]) -> SBuf {
        let mut r = SBuf::Inline { len: Len::of(s.len()), aux: 0, b: [0; INLINE] };
        if let SBuf::Inline { b, .. } = &mut r { if s.len() == 1 { b[0] = s[0]; } else { b[..s.len()].copy_from_slice(s); } }
        r
    }
    // out of line (not cold: long Strings dup often), so clone stays small enough to inline
    #[inline(never)]
    fn heap_clone(aux: u8, v: &[u8]) -> SBuf { SBuf::Heap { aux, v: v.to_vec() } }
    /// `s` (longer than INLINE) on the heap.
    #[inline]
    pub fn heap_copy(s: &[u8]) -> SBuf { SBuf::Heap { aux: 0, v: s.to_vec() } }
    // out of line, so the inline path stays small enough to inline into the allocation closure
    #[cold]
    #[inline(never)]
    fn heap_from(s: &[u8]) -> SBuf { SBuf::Heap { aux: 0, v: s.to_vec() } }
    /// Copy `s` (at most INLINE bytes) into an empty inline buffer, in place.
    #[inline(always)]
    pub fn fill_inline(&mut self, s: &[u8]) {
        if let SBuf::Inline { len, b, .. } = self {
            if s.len() == 1 { b[0] = s[0]; } else { b[..s.len()].copy_from_slice(s); }
            *len = Len::of(s.len());
        }
    }
    /// Takes over the Vec's block: copying a short one inline would cost a memcpy and a free now.
    // Always Heap (an empty Vec allocates nothing): an Inline arm made both arms copy the enum's padding bytes.
    #[inline]
    pub fn from_vec(v: Vec<u8>) -> SBuf { SBuf::Heap { aux: 0, v } }
    #[inline]
    pub fn with_capacity(n: usize) -> SBuf {
        if n <= INLINE { SBuf::new() } else { SBuf::Heap { aux: 0, v: Vec::with_capacity(n) } }
    }
    #[inline]
    fn vec(&mut self, cap: usize) -> &mut Vec<u8> {
        if let SBuf::Inline { .. } = self { self.spill(cap); }
        match self { SBuf::Heap { v, .. } => v, SBuf::Inline { .. } => unreachable!() }
    }
    #[cold]
    #[inline(never)]
    fn spill(&mut self, cap: usize) {
        let mut v = Vec::with_capacity(cap.max(64));
        v.extend_from_slice(self);
        *self = SBuf::Heap { aux: self.aux(), v };
    }

    #[inline]
    pub fn capacity(&self) -> usize { match self { SBuf::Inline { .. } => INLINE, SBuf::Heap { v, .. } => v.capacity() } }

    #[inline(always)]
    pub fn extend_from_slice(&mut self, s: &[u8]) {
        match self {
            SBuf::Inline { len, b, .. } => {
                let k = *len as usize;
                let m = k + s.len();
                if m <= INLINE {
                    // split_at_mut: both bounds checks fold against k <= INLINE and m <= INLINE
                    let t = b.split_at_mut(k).1;
                    if s.len() == 1 { t[0] = s[0]; } else { t[..s.len()].copy_from_slice(s); }
                    *len = Len::of(m);
                    return;
                }
                self.spill_extend(s);
            }
            // one byte without the memcpy call; Vec's own capacity check, its growth is out of line
            SBuf::Heap { v, .. } => if s.len() == 1 { v.push(s[0]); } else { v.extend_from_slice(s); },
        }
    }
    #[cold]
    #[inline(never)]
    fn spill_extend(&mut self, s: &[u8]) {
        let mut v = Vec::with_capacity((self.len() + s.len()).max(64));
        v.extend_from_slice(self);
        v.extend_from_slice(s);
        *self = SBuf::Heap { aux: self.aux(), v };
    }
    #[inline]
    pub fn push(&mut self, b: u8) { self.extend_from_slice(&[b]); }
    pub fn extend<I: IntoIterator<Item = u8>>(&mut self, it: I) { for b in it { self.push(b); } }
    #[inline]
    pub fn clear(&mut self) { self.truncate(0); }
    #[inline]
    pub fn truncate(&mut self, k: usize) {
        match self {
            SBuf::Inline { len, .. } => if k < *len as usize { *len = Len::of(k); },
            SBuf::Heap { v, .. } => v.truncate(k),
        }
    }
    pub fn resize(&mut self, k: usize, val: u8) {
        let n = self.len();
        if k <= n { self.truncate(k); return; }
        if let SBuf::Inline { len, b, .. } = self {
            if k <= INLINE { b[n..k].fill(val); *len = Len::of(k); return; }
        }
        self.vec(k).resize(k, val);
    }
    /// Remove the first n bytes (Vec::drain(..n)).
    pub fn remove_front(&mut self, n: usize) {
        let len = self.len();
        let n = n.min(len);
        self.copy_within(n.., 0);
        self.truncate(len - n);
    }
    /// Vec::splice, without the returned iterator.
    pub fn splice<R: RangeBounds<usize>, I: IntoIterator<Item = u8>>(&mut self, range: R, with: I) {
        let lo = match range.start_bound() { Bound::Included(&x) => x, Bound::Excluded(&x) => x + 1, Bound::Unbounded => 0 };
        let hi = match range.end_bound() { Bound::Included(&x) => x + 1, Bound::Excluded(&x) => x, Bound::Unbounded => self.len() };
        let mut v = self.to_vec();
        v.splice(lo..hi, with);
        self.set_bytes(SBuf::from_vec(v));
    }
    pub fn into_vec(self) -> Vec<u8> {
        match self { SBuf::Heap { v, .. } => v, SBuf::Inline { .. } => self.to_vec() }
    }
}


impl Deref for SBuf {
    type Target = [u8];
    #[inline(always)]
    fn deref(&self) -> &[u8] { match self { SBuf::Inline { len, b, .. } => &b[..*len as usize], SBuf::Heap { v, .. } => v } }
}
impl DerefMut for SBuf {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [u8] { match self { SBuf::Inline { len, b, .. } => &mut b[..*len as usize], SBuf::Heap { v, .. } => v } }
}
impl Clone for SBuf {
    /// The bytes and `aux`.
    #[inline]
    fn clone(&self) -> SBuf {
        match self {
            // a fixed-size copy: from_slice's copy of `len` bytes into the array is moved piecewise
            SBuf::Inline { aux, len, b } => SBuf::Inline { aux: *aux, len: *len, b: *b },
            SBuf::Heap { aux, v } if v.len() > INLINE => SBuf::heap_clone(*aux, v),
            SBuf::Heap { aux, v } => { let mut b = SBuf::inline_from(v); b.set_aux(*aux); b }
        }
    }
}
impl Default for SBuf { fn default() -> SBuf { SBuf::new() } }
impl From<Vec<u8>> for SBuf { #[inline] fn from(v: Vec<u8>) -> SBuf { SBuf::from_vec(v) } }
impl From<&[u8]> for SBuf { #[inline] fn from(s: &[u8]) -> SBuf { SBuf::from_slice(s) } }
impl PartialEq for SBuf { #[inline] fn eq(&self, o: &SBuf) -> bool { **self == **o } }
impl Eq for SBuf {}
impl PartialEq<Vec<u8>> for SBuf { fn eq(&self, o: &Vec<u8>) -> bool { **self == **o } }
impl PartialEq<SBuf> for Vec<u8> { fn eq(&self, o: &SBuf) -> bool { **self == **o } }
impl PartialEq<[u8]> for SBuf { fn eq(&self, o: &[u8]) -> bool { **self == *o } }
impl std::fmt::Debug for SBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { (**self).fmt(f) }
}
