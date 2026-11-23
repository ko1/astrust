// Backend "bitmap": non-moving mark & sweep that never touches the object header.
// Each page starts with two bitmaps (mark, live), one bit per slot.  Allocation takes
// free slots straight from the live bitmap, a 64-slot word at a time; sweep is
// `live = mark` per word, visiting only dead slots.  Same policy as "marksweep"
// (granted pages are added when every slot is taken, `reserve`).
// It exists to show that a binding does not depend on a backend's object layout.
use super::pages::Pages;
use super::{Control, EdgeVisitor, GcBackend, GcLang, HeapStats, RootVisitor, WeakVisitor};
use std::ptr::NonNull;

const R: usize = 1024;            // bytes reserved per page: 2 bitmaps of 4096 bits
const WORDS: usize = R / 16;
const INITIAL_PAGES: usize = 32;

#[repr(C)]
struct Bits { mark: [u64; WORDS], live: [u64; WORDS] }

/// The bitmaps of the page at `base`.
///
/// # Safety
/// `base` is a page of a `Pages<_, R>`, and no other reference to its bitmaps is live.
#[inline(always)]
unsafe fn bits<'a>(base: usize) -> &'a mut Bits {
    // SAFETY: the first R bytes of a page are a zero-initialized Bits
    unsafe { &mut *(base as *mut Bits) }
}

pub struct MarkBitmap<L: GcLang> {
    cur_bits: u64,                // slots of the current word not handed out yet (reserved in `live`)
    cur_slot0: usize,             // address of the current word's slot 0
    cur_page: usize,
    cur_word: usize,
    next_page: usize,             // where the search for free slots continues
    next_word: usize,
    free_count: usize,
    total: usize,                 // slots of the pages added
    reserve: usize,               // pages granted and not added yet
    live_after_gc: usize,
    allocs_since_gc: usize,
    external_since_gc: usize,
    external_limit: usize,
    total_allocated: u64,
    total_freed: u64,
    pages: Pages<L::Obj, R>,
}

impl<L: GcLang> MarkBitmap<L> {
    const SLOT: usize = Pages::<L::Obj, R>::SLOT;
    const PER_PAGE: usize = {
        assert!(Pages::<L::Obj, R>::PER_PAGE <= WORDS * 64);
        Pages::<L::Obj, R>::PER_PAGE
    };
    const NWORDS: usize = Self::PER_PAGE.div_ceil(64);

    /// The bits of word `w` that stand for slots.
    #[inline(always)]
    fn valid(w: usize) -> u64 {
        if w + 1 == Self::NWORDS && Self::PER_PAGE % 64 != 0 { (1u64 << (Self::PER_PAGE % 64)) - 1 } else { !0 }
    }

    fn add_page(&mut self) {
        self.pages.add();
        self.free_count += Self::PER_PAGE;
        self.total += Self::PER_PAGE;
    }

    /// Reserve the next word that has free slots.
    fn refill(&mut self) -> bool {
        while self.next_page < self.pages.len() {
            let base = self.pages.bases()[self.next_page];
            while self.next_word < Self::NWORDS {
                let w = self.next_word;
                self.next_word += 1;
                // SAFETY: base is one of our pages; the reference ends with this iteration
                let live = unsafe { &mut bits(base).live[w] };
                let free = !*live & Self::valid(w);
                if free != 0 {
                    *live |= free;
                    self.cur_bits = free;
                    self.cur_slot0 = Pages::<L::Obj, R>::slot(base, w * 64) as usize;
                    self.cur_page = self.next_page;
                    self.cur_word = w;
                    return true;
                }
            }
            self.next_page += 1;
            self.next_word = 0;
        }
        false
    }

    /// Give back the slots reserved and not handed out, so `live` means "holds an object".
    fn unreserve(&mut self) {
        if self.cur_bits == 0 { return; }
        // SAFETY: cur_page is one of our pages; no other bitmap reference is live
        unsafe { bits(self.pages.bases()[self.cur_page]).live[self.cur_word] &= !self.cur_bits; }
        self.cur_bits = 0;
        self.next_page = self.cur_page;
        self.next_word = self.cur_word;
    }

    #[inline(never)]
    fn slow(&mut self, lang: &mut L, ctl: &mut Control, mut obj: L::Obj) -> L::Obj {
        if self.external_since_gc > self.external_limit && !ctl.disabled() { ctl.collect(self, lang, Some(&mut obj)); }
        if self.cur_bits != 0 { return obj; }
        while !self.refill() {
            // every slot is taken
            if self.reserve > 0 { self.reserve -= 1; self.add_page(); continue; }
            let collect = !ctl.disabled() && self.allocs_since_gc > self.total / 4;
            if collect { ctl.collect(self, lang, Some(&mut obj)); }
            if !collect || self.free_count < self.live_after_gc.max(1) {
                let want = self.live_after_gc.saturating_sub(self.free_count).max(self.total / 8) / Self::PER_PAGE + 1;
                self.reserve = want - 1;
                self.add_page();
            }
        }
        obj
    }

    fn sweep(&mut self, lang: &mut L) {
        let mut live = 0;
        for &base in self.pages.bases() {
            for w in 0..Self::NWORDS {
                // SAFETY: base is one of our pages; finalize does not reach the bitmaps
                let b = unsafe { bits(base) };
                let mut dead = b.live[w] & !b.mark[w];
                b.live[w] = b.mark[w];
                b.mark[w] = 0;
                live += b.live[w].count_ones() as usize;
                while dead != 0 {
                    let p = Pages::<L::Obj, R>::slot(base, w * 64 + dead.trailing_zeros() as usize);
                    dead &= dead - 1;
                    // SAFETY: live and unmarked: an initialized object nothing refers to
                    unsafe { lang.finalize(NonNull::new_unchecked(p)); }
                    self.total_freed += 1;
                }
            }
        }
        self.free_count = self.total - live;
        self.live_after_gc = live;
        self.next_page = 0;
        self.next_word = 0;
    }
}

struct Marker<'a, L: GcLang> {
    stack: Vec<*mut L::Obj>,
    pages: &'a Pages<L::Obj, R>,
}

impl<L: GcLang> Marker<'_, L> {
    #[inline(always)]
    fn mark(&mut self, base: usize, i: usize) {
        let (live, mark) = mark_word(base, i);
        let m = 1u64 << (i % 64);
        if live & m == 0 || *mark & m != 0 { return; }
        *mark |= m;
        self.stack.push(Pages::<L::Obj, R>::slot(base, i));
    }

    // out of line: scan_edges is inlined into drain only
    #[inline(never)]
    fn pending(&mut self, lang: &mut L, obj: &mut L::Obj) { lang.scan_edges(obj, self); }

    fn drain(&mut self, lang: &mut L) {
        while let Some(p) = self.stack.pop() {
            // SAFETY: a live bit means an initialized object; the mutator is stopped
            lang.scan_edges(unsafe { &mut *p }, self);
        }
    }
}

impl<L: GcLang> EdgeVisitor<L> for Marker<'_, L> {
    #[inline]
    fn visit(&mut self, slot: &mut L::Ref) {
        if let Some(p) = L::as_object(*slot) {
            // GcLang::as_object contract: p is a slot of this heap
            let (base, i) = Pages::<L::Obj, R>::locate(p.as_ptr());
            self.mark(base, i);
        }
    }
}

impl<L: GcLang> RootVisitor<L> for Marker<'_, L> {
    fn visit_ambiguous(&mut self, words: &[usize]) {
        for &w in words {
            if w == 0 { continue; }
            if let Some((base, i)) = self.pages.resolve(w) { self.mark(base, i); }
        }
    }
}

/// The live word and the mark word of slot `i` of page `base`, one of our pages.
#[inline(always)]
fn mark_word<'a>(base: usize, i: usize) -> (u64, &'a mut u64) {
    // SAFETY: base is one of our pages; the caller's use of the reference ends before the next call
    let b = unsafe { bits(base) };
    (b.live[i / 64], &mut b.mark[i / 64])
}

// between mark and sweep: a set mark bit means alive
struct Survivors;
impl<L: GcLang> WeakVisitor<L> for Survivors {
    fn alive(&mut self, slot: &mut L::Ref) -> bool {
        L::as_object(*slot).is_none_or(|p| {
            // GcLang::as_object contract: p is a slot of this heap
            let (base, i) = Pages::<L::Obj, R>::locate(p.as_ptr());
            *mark_word(base, i).1 & 1u64 << (i % 64) != 0
        })
    }
}

impl<L: GcLang> GcBackend<L> for MarkBitmap<L> {
    const NAME: &'static str = "bitmap";
    const MOVING: bool = false;
    const AMBIGUOUS_ROOTS: bool = true;

    fn new() -> Self {
        MarkBitmap {
            cur_bits: 0, cur_slot0: 0, cur_page: 0, cur_word: 0, next_page: 0, next_word: 0,
            free_count: 0, total: 0, reserve: INITIAL_PAGES, live_after_gc: 0, allocs_since_gc: 0,
            external_since_gc: 0, external_limit: 64 << 20, total_allocated: 0, total_freed: 0, pages: Pages::new(),
        }
    }

    #[inline(always)]
    fn alloc<F: FnOnce() -> L::Obj>(&mut self, lang: &mut L, ctl: &mut Control, make: F) -> NonNull<L::Obj> {
        let mut obj = make();
        if self.cur_bits == 0 || self.external_since_gc > self.external_limit { obj = self.slow(lang, ctl, obj); }
        let p = (self.cur_slot0 + self.cur_bits.trailing_zeros() as usize * Self::SLOT) as *mut L::Obj;
        self.cur_bits &= self.cur_bits - 1;
        // SAFETY: after slow() cur_bits is not zero; p is a slot reserved for us with room for an Obj
        unsafe { p.write(obj); }
        self.free_count -= 1;
        self.allocs_since_gc += 1;
        self.total_allocated += 1;
        // SAFETY: slots are never null
        unsafe { NonNull::new_unchecked(p) }
    }

    fn collect(&mut self, lang: &mut L, pending: Option<&mut L::Obj>) {
        self.unreserve();
        {
            let mut m = Marker::<L> { stack: Vec::with_capacity(4096), pages: &self.pages };
            lang.visit_roots(&mut m);
            if let Some(o) = pending { m.pending(lang, o); }
            m.drain(lang);
        }
        lang.sweep_weak(&mut Survivors);
        self.sweep(lang);
        self.allocs_since_gc = 0;
        self.external_since_gc = 0;
        self.external_limit = (self.live_after_gc * 256).max(64 << 20);
    }

    #[inline(always)]
    fn account_external(&mut self, bytes: usize) { self.external_since_gc += bytes; }

    fn each_object<F: FnMut(NonNull<L::Obj>)>(&mut self, mut f: F) {
        self.unreserve();
        // collected first: an embedder with a global heap may reserve slots from `f`
        let mut objs = vec![];
        for &base in self.pages.bases() {
            for w in 0..Self::NWORDS {
                // SAFETY: base is one of our pages; the reference ends with this statement
                let mut live = unsafe { bits(base).live[w] };
                while live != 0 {
                    objs.push(Pages::<L::Obj, R>::slot(base, w * 64 + live.trailing_zeros() as usize));
                    live &= live - 1;
                }
            }
        }
        // SAFETY: slots are never null
        for p in objs { f(unsafe { NonNull::new_unchecked(p) }); }
    }

    fn stats(&self) -> HeapStats {
        let r = self.reserve * Self::PER_PAGE;
        HeapStats { slots: self.total + r, free: self.free_count + r, allocated: self.total_allocated, freed: self.total_freed }
    }
}
