// Backend "marksweep": non-moving mark & sweep over fixed-size slots.
// Mark and free bits live in the object's GcHeader; free slots are chained through
// a word outside the header.  Ambiguous roots are supported (nothing moves).
//
// Policy: collect when the free list is empty and a quarter of the heap has been
// allocated since the last collection, or when external memory passes its limit;
// grow so that the heap stays at least three times the live set.
use super::pages::Pages;
use super::{header, Control, EdgeVisitor, GcBackend, GcLang, HeapStats, RootVisitor};
use std::ptr::NonNull;

const MARK: u32 = 1;
const FREE: u32 = 2;
const INITIAL_PAGES: usize = 32;

pub struct MarkSweep<L: GcLang> {
    free: *mut L::Obj,
    free_count: usize,
    total: usize,
    live_after_gc: usize,
    allocs_since_gc: usize,
    external_since_gc: usize,
    external_limit: usize,
    total_allocated: u64,
    total_freed: u64,
    pages: Pages<L::Obj, 0>,
}

impl<L: GcLang> MarkSweep<L> {
    const PER_PAGE: usize = Pages::<L::Obj, 0>::PER_PAGE;
    // the free-list link is the first word of a slot that does not overlap the header
    const LINK: usize = if L::HEADER_OFFSET >= 8 { 0 } else { 8 };

    /// The link word of free slot `p`.
    ///
    /// # Safety
    /// `p` is one of our slots.
    #[inline(always)]
    unsafe fn link(p: *mut L::Obj) -> *mut *mut L::Obj {
        // SAFETY: LINK is inside the slot and 8-aligned
        unsafe { (p as *mut u8).add(Self::LINK) as *mut *mut L::Obj }
    }

    fn add_page(&mut self) {
        let base = self.pages.add();
        for i in (0..Self::PER_PAGE).rev() {
            let p = Pages::<L::Obj, 0>::slot(base, i);
            // SAFETY: a slot of the page just mapped; we initialize its header and link
            unsafe {
                std::ptr::write((p as *mut u8).add(L::HEADER_OFFSET) as *mut u32, FREE);
                Self::link(p).write(self.free);
            }
            self.free = p;
        }
        self.free_count += Self::PER_PAGE;
        self.total += Self::PER_PAGE;
    }

    #[inline(never)]
    fn slow(&mut self, lang: &mut L, ctl: &mut Control, mut obj: L::Obj) -> L::Obj {
        if self.external_since_gc > self.external_limit && !ctl.disabled() { ctl.collect(self, lang, Some(&mut obj)); }
        if self.free.is_null() {
            if !ctl.disabled() && self.allocs_since_gc > self.total / 4 { ctl.collect(self, lang, Some(&mut obj)); }
            // heap >= 3x the live set: each full mark is paid by 2x live allocations (2x cost gen_gc 11% more cycles)
            if self.free.is_null() || self.free_count < 2 * self.live_after_gc {
                let want = (2 * self.live_after_gc).saturating_sub(self.free_count).max(self.total / 8) / Self::PER_PAGE + 1;
                for _ in 0..want { self.add_page(); }
            }
        }
        obj
    }

    fn sweep(&mut self, lang: &mut L) {
        let mut free: *mut L::Obj = std::ptr::null_mut();
        let mut nfree = 0;
        for &base in self.pages.bases() {
            for i in (0..Self::PER_PAGE).rev() {
                let p = Pages::<L::Obj, 0>::slot(base, i);
                // SAFETY: every slot of our pages has an initialized header
                let h = unsafe { header::<L>(p) };
                let f = h.get();
                if f & FREE != 0 {
                    // SAFETY: the slot is free; its link word is ours
                    unsafe { Self::link(p).write(free); }
                    free = p;
                    nfree += 1;
                } else if f & MARK != 0 {
                    h.set(f & !MARK);
                } else {
                    // SAFETY: allocated and unmarked: an initialized object nothing refers to; then the slot is ours
                    unsafe {
                        lang.finalize(NonNull::new_unchecked(p));
                        h.set(FREE);
                        Self::link(p).write(free);
                    }
                    free = p;
                    nfree += 1;
                }
            }
        }
        // every slot that was free before is on the old list, so the rest of `nfree` died now
        self.total_freed += (nfree - self.free_count) as u64;
        self.free = free;
        self.free_count = nfree;
        self.live_after_gc = self.total - nfree;
    }
}

struct Marker<'a, L: GcLang> {
    stack: Vec<*mut L::Obj>,
    pages: &'a Pages<L::Obj, 0>,
}

impl<L: GcLang> Marker<'_, L> {
    #[inline(always)]
    fn mark(&mut self, p: *mut L::Obj) {
        // SAFETY: p is a slot of this heap (GcLang::as_object contract, or resolved from our pages)
        let h = unsafe { header::<L>(p) };
        let f = h.get();
        if f & (MARK | FREE) != 0 { return; }
        h.set(f | MARK);
        self.stack.push(p);
    }

    // out of line: scan_edges is inlined into drain only
    #[inline(never)]
    fn pending(&mut self, lang: &mut L, obj: &mut L::Obj) { lang.scan_edges(obj, self); }

    fn drain(&mut self, lang: &mut L) {
        while let Some(p) = self.stack.pop() {
            // SAFETY: marked objects are allocated and initialized; the mutator is stopped
            lang.scan_edges(unsafe { &mut *p }, self);
        }
    }
}

impl<L: GcLang> EdgeVisitor<L> for Marker<'_, L> {
    #[inline]
    fn visit(&mut self, slot: &mut L::Ref) {
        if let Some(p) = L::as_object(*slot) { self.mark(p.as_ptr()); }
    }
}

impl<L: GcLang> RootVisitor<L> for Marker<'_, L> {
    fn visit_ambiguous(&mut self, words: &[usize]) {
        for &w in words {
            if w == 0 { continue; }
            if let Some((base, i)) = self.pages.resolve(w) { self.mark(Pages::<L::Obj, 0>::slot(base, i)); }
        }
    }
}

impl<L: GcLang> MarkSweep<L> {
    #[cold]
    #[inline(never)]
    fn alloc_cold(&mut self, lang: &mut L, ctl: &mut Control, obj: L::Obj) -> NonNull<L::Obj> {
        let obj = self.slow(lang, ctl, obj);
        self.place(obj)
    }

    #[inline(always)]
    fn place(&mut self, obj: L::Obj) -> NonNull<L::Obj> {
        let p = self.free;
        // SAFETY: after slow() the free list is not empty; p is a free slot: it holds a link and has room for an Obj
        unsafe {
            self.free = Self::link(p).read();
            p.write(obj);
        }
        self.free_count -= 1;
        self.allocs_since_gc += 1;
        self.total_allocated += 1;
        // SAFETY: slots are never null
        unsafe { NonNull::new_unchecked(p) }
    }
}

impl<L: GcLang> GcBackend<L> for MarkSweep<L> {
    const NAME: &'static str = "marksweep";
    const MOVING: bool = false;
    const AMBIGUOUS_ROOTS: bool = true;

    fn new() -> Self {
        let mut h = MarkSweep {
            free: std::ptr::null_mut(), free_count: 0, total: 0, live_after_gc: 0, allocs_since_gc: 0,
            external_since_gc: 0, external_limit: 64 << 20, total_allocated: 0, total_freed: 0, pages: Pages::new(),
        };
        for _ in 0..INITIAL_PAGES { h.add_page(); }
        h
    }

    #[inline(always)]
    fn alloc<F: FnOnce() -> L::Obj>(&mut self, lang: &mut L, ctl: &mut Control, make: F) -> NonNull<L::Obj> {
        // no call between the caller building `obj` and the stores into the slot: an object kept on the
        // stack across slow() is copied by a wide load that stalls on its narrow stores
        if self.free.is_null() || self.external_since_gc > self.external_limit { return self.alloc_cold(lang, ctl, make()); }
        self.place(make())
    }

    fn collect(&mut self, lang: &mut L, pending: Option<&mut L::Obj>) {
        {
            let mut m = Marker::<L> { stack: Vec::with_capacity(4096), pages: &self.pages };
            lang.visit_roots(&mut m);
            if let Some(o) = pending { m.pending(lang, o); }
            m.drain(lang);
        }
        self.sweep(lang);
        self.allocs_since_gc = 0;
        self.external_since_gc = 0;
        self.external_limit = (self.live_after_gc * 256).max(64 << 20);
    }

    #[inline(always)]
    fn account_external(&mut self, bytes: usize) { self.external_since_gc += bytes; }

    fn each_object<F: FnMut(NonNull<L::Obj>)>(&mut self, mut f: F) {
        // a copy: an embedder with a global heap may add pages from `f`
        for base in self.pages.bases().to_vec() {
            for i in 0..Self::PER_PAGE {
                let p = Pages::<L::Obj, 0>::slot(base, i);
                // SAFETY: every slot of our pages has an initialized header; slots are never null
                unsafe { if header::<L>(p).get() & FREE == 0 { f(NonNull::new_unchecked(p)); } }
            }
        }
    }

    fn stats(&self) -> HeapStats {
        HeapStats { slots: self.total, free: self.free_count, allocated: self.total_allocated, freed: self.total_freed }
    }
}
