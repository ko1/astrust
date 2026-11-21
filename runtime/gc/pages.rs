// Page-aligned slabs of fixed-size slots, shared by the slot-based backends.
// `Pages<T, R>`: 64 KiB pages aligned to their size, the first `R` bytes of each
// reserved for the backend (side tables), the rest cut into slots of size_of::<T>().
// Any machine word can be resolved to the slot it points into (conservative roots).
// Pages are never returned to the system.
use std::alloc::Layout;
use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hasher};
use std::marker::PhantomData;

pub const PAGE_BYTES: usize = 64 * 1024;
/// Pages taken from the system allocator at a time (one mapping instead of one per page).
const SLAB_PAGES: usize = 16;

// multiply-rotate hash of one address (SipHash would dominate conservative scanning)
#[derive(Default, Clone, Copy)]
pub struct AddrHasher(u64);
impl Hasher for AddrHasher {
    fn write(&mut self, bytes: &[u8]) { for &b in bytes { self.write_usize(b as usize); } }
    #[inline] fn write_usize(&mut self, i: usize) { self.0 = (self.0.rotate_left(26) ^ i as u64).wrapping_mul(0xf1357aea2e62a9c5); }
    #[inline] fn finish(&self) -> u64 { self.0.rotate_left(26) }
}

pub struct Pages<T, const R: usize> {
    bases: Vec<usize>,
    set: HashSet<usize, BuildHasherDefault<AddrHasher>>,
    slab: usize,               // the next page of the current slab not handed out
    slab_left: usize,
    _t: PhantomData<*mut T>,
}

impl<T, const R: usize> Pages<T, R> {
    pub const SLOT: usize = std::mem::size_of::<T>();
    pub const PER_PAGE: usize = {
        assert!(std::mem::size_of::<T>() >= 16 && std::mem::size_of::<T>() % 8 == 0 && std::mem::align_of::<T>() <= 4096);
        assert!(R % std::mem::align_of::<T>() == 0 && R % 8 == 0 && R < PAGE_BYTES / 2);
        (PAGE_BYTES - R) / std::mem::size_of::<T>()
    };

    pub fn new() -> Self { Pages { bases: vec![], set: HashSet::default(), slab: 0, slab_left: 0, _t: PhantomData } }

    /// One more page, and its base.  Slot memory is uninitialized (and untouched); the
    /// reserved bytes are zero.
    pub fn add(&mut self) -> usize {
        if self.slab_left == 0 {
            let layout = Layout::from_size_align(PAGE_BYTES * SLAB_PAGES, PAGE_BYTES).unwrap();
            // SAFETY: the layout has a non-zero size; each page's first R bytes are inside the block
            self.slab = unsafe {
                let p = std::alloc::alloc(layout);
                if R != 0 && !p.is_null() { for i in 0..SLAB_PAGES { p.add(i * PAGE_BYTES).write_bytes(0, R); } }
                p
            } as usize;
            if self.slab == 0 { std::alloc::handle_alloc_error(layout); }
            self.slab_left = SLAB_PAGES;
        }
        let base = self.slab;
        self.slab += PAGE_BYTES;
        self.slab_left -= 1;
        self.bases.push(base);
        self.set.insert(base);
        base
    }

    #[inline(always)] pub fn bases(&self) -> &[usize] { &self.bases }
    #[inline(always)] pub fn len(&self) -> usize { self.bases.len() }

    /// Slot `i` of the page at `base` (an address computation; dereferencing is the caller's business).
    #[inline(always)]
    pub fn slot(base: usize, i: usize) -> *mut T { (base + R + i * Self::SLOT) as *mut T }

    /// The page and slot index of a slot pointer.
    #[inline(always)]
    pub fn locate(p: *mut T) -> (usize, usize) {
        let base = p as usize & !(PAGE_BYTES - 1);
        (base, (p as usize - base - R) / Self::SLOT)
    }

    /// The (page, slot index) that address `w` falls into, if it is inside one of our slots.
    #[inline]
    pub fn resolve(&self, w: usize) -> Option<(usize, usize)> {
        let base = w & !(PAGE_BYTES - 1);
        if !self.set.contains(&base) { return None; }
        let off = w - base;
        if off < R { return None; }
        let i = (off - R) / Self::SLOT;
        if i >= Self::PER_PAGE { return None; }
        Some((base, i))
    }
}
