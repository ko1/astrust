// String byte buffer with the Vec<u8> operations String needs; up to 23 bytes live
// inside the object slot (no malloc), longer ones in a Vec-compatible heap block.
//
// Three words, little-endian:  heap   = [ptr, len, cap | HEAP]
//                              inline = 23 data bytes, then the length byte (< 0x80)
use std::mem::ManuallyDrop;
use std::ops::{Bound, Deref, DerefMut, RangeBounds};

#[cfg(not(target_endian = "little"))]
compile_error!("SBuf keeps its tag in the last byte of a little-endian word");

pub const INLINE: usize = 23;
const HEAP: usize = 1 << 63;

#[repr(C)]
pub struct SBuf { w: [usize; 3] }

// one-byte strings ("x", " ", "\n") are common enough to skip the memcpy call
#[inline(always)]
unsafe fn copy(src: &[u8], dst: *mut u8) {
    if src.len() == 1 { *dst = *src.as_ptr(); } else { std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len()); }
}

impl SBuf {
    #[inline(always)]
    pub const fn new() -> SBuf { SBuf { w: [0; 3] } }
    #[inline(always)]
    fn is_heap(&self) -> bool { self.w[2] & HEAP != 0 }
    #[inline(always)]
    fn set_inline_len(&mut self, n: usize) { self.w[2] = (self.w[2] & !(0xff << 56)) | (n << 56); }
    #[inline(always)]
    fn inline_ptr(&mut self) -> *mut u8 { self.w.as_mut_ptr() as *mut u8 }

    #[inline]
    pub fn from_slice(s: &[u8]) -> SBuf {
        if s.len() <= INLINE {
            let mut b = SBuf::new();
            unsafe { copy(s, b.inline_ptr()); }
            b.set_inline_len(s.len());
            b
        } else { SBuf::adopt(s.to_vec()) }
    }
    /// Takes over the Vec's block: copying a short one inline would cost a memcpy and a free now.
    #[inline]
    pub fn from_vec(v: Vec<u8>) -> SBuf {
        if v.capacity() == 0 { SBuf::new() } else { SBuf::adopt(v) }
    }
    #[inline]
    pub fn with_capacity(n: usize) -> SBuf {
        if n <= INLINE { SBuf::new() } else { SBuf::adopt(Vec::with_capacity(n)) }
    }
    // take over a Vec's block
    #[inline]
    fn adopt(v: Vec<u8>) -> SBuf {
        let mut v = ManuallyDrop::new(v);
        SBuf { w: [v.as_mut_ptr() as usize, v.len(), v.capacity() | HEAP] }
    }
    // run f on the heap block as a Vec
    #[inline]
    fn with_vec<R>(&mut self, f: impl FnOnce(&mut Vec<u8>) -> R) -> R {
        debug_assert!(self.is_heap());
        let mut v = ManuallyDrop::new(unsafe { Vec::from_raw_parts(self.w[0] as *mut u8, self.w[1], self.w[2] & !HEAP) });
        let r = f(&mut v);
        self.w = [v.as_mut_ptr() as usize, v.len(), v.capacity() | HEAP];
        r
    }
    // inline -> heap with room for `cap` bytes
    #[cold]
    fn spill(&mut self, cap: usize) {
        let mut v = Vec::with_capacity(cap.max(64));
        v.extend_from_slice(self);
        *self = SBuf::adopt(v);
    }

    #[inline]
    pub fn capacity(&self) -> usize { if self.is_heap() { self.w[2] & !HEAP } else { INLINE } }

    #[inline]
    pub fn extend_from_slice(&mut self, s: &[u8]) {
        if !self.is_heap() {
            let n = self.w[2] >> 56;
            let m = n + s.len();
            if m <= INLINE {
                unsafe { copy(s, self.inline_ptr().add(n)); }
                self.set_inline_len(m);
                return;
            }
            self.spill(m);
        }
        let (len, cap) = (self.w[1], self.w[2] & !HEAP);
        if len + s.len() <= cap {
            unsafe { copy(s, (self.w[0] as *mut u8).add(len)); }
            self.w[1] = len + s.len();
            return;
        }
        self.grow(s);
    }
    #[cold]
    #[inline(never)]
    fn grow(&mut self, s: &[u8]) { self.with_vec(|v| v.extend_from_slice(s)); }
    #[inline]
    pub fn push(&mut self, b: u8) { self.extend_from_slice(&[b]); }
    pub fn extend<I: IntoIterator<Item = u8>>(&mut self, it: I) { for b in it { self.push(b); } }
    #[inline]
    pub fn clear(&mut self) { self.truncate(0); }
    #[inline]
    pub fn truncate(&mut self, n: usize) {
        if n >= self.len() { return; }
        if self.is_heap() { self.w[1] = n; } else { self.set_inline_len(n); }
    }
    pub fn resize(&mut self, n: usize, val: u8) {
        let len = self.len();
        if n <= len { self.truncate(n); return; }
        if !self.is_heap() {
            if n <= INLINE {
                unsafe { std::ptr::write_bytes(self.inline_ptr().add(len), val, n - len); }
                self.set_inline_len(n);
                return;
            }
            self.spill(n);
        }
        self.with_vec(|v| v.resize(n, val));
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
        *self = SBuf::from_vec(v);
    }
    pub fn into_vec(self) -> Vec<u8> {
        if self.is_heap() {
            let me = ManuallyDrop::new(self);
            unsafe { Vec::from_raw_parts(me.w[0] as *mut u8, me.w[1], me.w[2] & !HEAP) }
        } else { self.to_vec() }
    }
}

impl Deref for SBuf {
    type Target = [u8];
    #[inline(always)]
    fn deref(&self) -> &[u8] {
        unsafe {
            if self.is_heap() { std::slice::from_raw_parts(self.w[0] as *const u8, self.w[1]) }
            else { std::slice::from_raw_parts(self.w.as_ptr() as *const u8, self.w[2] >> 56) }
        }
    }
}
impl DerefMut for SBuf {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            if self.is_heap() { std::slice::from_raw_parts_mut(self.w[0] as *mut u8, self.w[1]) }
            else { std::slice::from_raw_parts_mut(self.w.as_mut_ptr() as *mut u8, self.w[2] >> 56) }
        }
    }
}
impl Drop for SBuf {
    #[inline]
    fn drop(&mut self) {
        if self.is_heap() { unsafe { drop(Vec::from_raw_parts(self.w[0] as *mut u8, self.w[1], self.w[2] & !HEAP)); } }
    }
}
impl Clone for SBuf {
    #[inline]
    fn clone(&self) -> SBuf { SBuf::from_slice(self) }
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
