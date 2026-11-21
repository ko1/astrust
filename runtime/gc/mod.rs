// ASTrust GC framework: the language-independent half of a tracing collector.
// The language implements `GcLang` (roots, edges, finalization, "is this word a
// reference"); a backend implements `GcBackend` (heap layout and algorithm); `Gc`
// ties one of each together at compile time and owns what every backend shares
// (stress mode, GC.disable, collection count and time).  See docs/gc.md.
//
// An embedder includes this directory as a module:
//   #[path = ".../runtime/gc/mod.rs"] pub mod astrust_gc;
#![allow(dead_code)]                 // one backend is instantiated per build
#![deny(unsafe_op_in_unsafe_fn)]
use std::cell::Cell;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::time::{Duration, Instant};

pub mod pages;
pub mod stack;
pub mod mark_sweep;
pub mod mark_bitmap;

/// One word in every object, owned by the backend (mark / free bits today,
/// forwarding state for a moving backend).  Only `new()` makes one, so an object
/// handed to `alloc` always carries a fresh header.
#[repr(transparent)]
pub struct GcHeader(Cell<u32>);

impl GcHeader {
    #[inline(always)] pub const fn new() -> GcHeader { GcHeader(Cell::new(0)) }
    #[inline(always)] fn get(&self) -> u32 { self.0.get() }
    #[inline(always)] fn set(&self, v: u32) { self.0.set(v) }
}

/// What a language tells the collector.
///
/// # Safety
/// Backends dereference what the binding hands them, so the implementation must guarantee:
/// * `Obj` has a `GcHeader` field at byte offset `HEADER_OFFSET`, and its alignment is at least 8;
/// * `as_object(r)` is `None` or a pointer returned earlier by `Gc::alloc` of the same heap
///   (a pointer to an object that has died since is tolerated by the non-moving backends);
/// * `visit_roots` and `scan_edges` visit only slots the caller may write.
pub unsafe trait GcLang: Sized {
    /// A reference word as stored in objects and roots (tagged pointer, NaN box, ...).
    type Ref: Copy;
    /// The heap object.  Every object is one `Obj`; larger payloads are owned by it.
    type Obj;
    const HEADER_OFFSET: usize;
    /// `visit_roots` calls `RootVisitor::visit_ambiguous`.  A backend that cannot pin
    /// refuses such a binding at compile time.
    const AMBIGUOUS_ROOTS: bool;

    /// "Is this word a heap reference": the object it designates, `None` for an immediate.
    fn as_object(r: Self::Ref) -> Option<NonNull<Self::Obj>>;
    /// The reference word designating `p` (how a moving backend rewrites a slot).
    fn from_object(p: NonNull<Self::Obj>) -> Self::Ref;
    /// Visit every root slot.
    fn visit_roots<V: RootVisitor<Self>>(&mut self, v: &mut V);
    /// Visit every reference slot of one object.  With `AMBIGUOUS_ROOTS`, also the ambiguous
    /// words the object owns (a suspended machine stack: a fiber), through `visit_ambiguous`.
    fn scan_edges<V: RootVisitor<Self>>(&mut self, obj: &mut Self::Obj, v: &mut V);
    /// Release what a dead object owns.  Must not allocate or touch other heap objects.
    ///
    /// # Safety
    /// `obj` is an initialized object that no visited slot referred to; it is not used again.
    unsafe fn finalize(&mut self, obj: NonNull<Self::Obj>);
}

/// Receives reference slots.  The same enumeration serves marking, forwarding and
/// pointer update: a moving backend writes the new reference back through `slot`.
pub trait EdgeVisitor<L: GcLang> {
    fn visit(&mut self, slot: &mut L::Ref);
}

/// Receives roots: exact slots (`visit`) and, optionally, ambiguous words.
pub trait RootVisitor<L: GcLang>: EdgeVisitor<L> {
    /// Words that may or may not be references (interior pointers included), such as a
    /// machine stack.  What they designate stays alive and is never moved; the words
    /// themselves are never rewritten.
    fn visit_ambiguous(&mut self, words: &[usize]);
}

/// A heap layout and collection algorithm for the objects of `L`.
pub trait GcBackend<L: GcLang>: Sized {
    const NAME: &'static str;
    /// Objects can change address in a collection.
    const MOVING: bool;
    /// `RootVisitor::visit_ambiguous` is honoured (referents are pinned).
    const AMBIGUOUS_ROOTS: bool;

    fn new() -> Self;
    /// Put `obj` in the heap.  A collection, if one is due, goes through `ctl.collect`
    /// with `obj` as the pending object.  The common case must be inlinable.
    /// `make` builds the object: called after the fast-path check so it is written straight into the
    /// slot (an object built first sits on the stack and is copied with loads its stores cannot forward to).
    fn alloc<F: FnOnce() -> L::Obj>(&mut self, lang: &mut L, ctl: &mut Control, make: F) -> NonNull<L::Obj>;
    /// One full collection (called through `Control::collect` only).  `pending` is an
    /// object being allocated and not in the heap yet: its slots are roots.
    fn collect(&mut self, lang: &mut L, pending: Option<&mut L::Obj>);
    /// Write barrier: `holder` now refers to `v`.
    #[inline(always)]
    fn barrier(&mut self, _holder: NonNull<L::Obj>, _v: L::Ref) {}
    /// Write barrier without the value: some slots of `holder` were overwritten.
    #[inline(always)]
    fn barrier_holder(&mut self, _holder: NonNull<L::Obj>) {}
    /// `obj` keeps its address from now on.
    fn pin(&mut self, _obj: NonNull<L::Obj>) {}
    /// Objects grew by `bytes` of memory outside the heap (a collection trigger).
    #[inline(always)]
    fn account_external(&mut self, _bytes: usize) {}
    /// Call `f` on every object that is allocated and not yet swept.
    fn each_object<F: FnMut(NonNull<L::Obj>)>(&mut self, f: F);
    fn stats(&self) -> HeapStats;
}

#[derive(Clone, Copy, Default, Debug)]
pub struct HeapStats {
    pub slots: usize,          // capacity in objects
    pub free: usize,
    pub allocated: u64,        // since start
    pub freed: u64,
}

/// What every backend shares: stress mode, GC.disable, collection count and time.
pub struct Control {
    stress: usize,             // collect at every `stress`-th allocation; 0 = off
    stress_count: usize,
    disabled: bool,
    collections: usize,
    minors: usize,             // minor collections that found nothing young (collect_minor)
    allocated_at_gc: u64,      // HeapStats::allocated when the last collection ended
    time: Duration,
}

impl Control {
    /// GC.disable: allocation-driven triggers must not collect (backends check this).
    #[inline(always)] pub fn disabled(&self) -> bool { self.disabled }
    pub fn collections(&self) -> usize { self.collections }
    pub fn minor_collections(&self) -> usize { self.minors }
    pub fn time(&self) -> Duration { self.time }

    /// Run one collection of `b` and account for it.
    pub fn collect<L: GcLang, B: GcBackend<L>>(&mut self, b: &mut B, lang: &mut L, pending: Option<&mut L::Obj>) {
        let t0 = Instant::now();
        b.collect(lang, pending);
        self.collections += 1;
        self.allocated_at_gc = b.stats().allocated;
        self.time += t0.elapsed();
    }
}

/// A heap: backend `B` collecting the objects of language `L`.
pub struct Gc<L: GcLang, B: GcBackend<L>> {
    pub ctl: Control,
    backend: B,
    _lang: PhantomData<fn(L)>,
}

impl<L: GcLang, B: GcBackend<L>> Gc<L, B> {
    const CHECK: () = assert!(!L::AMBIGUOUS_ROOTS || B::AMBIGUOUS_ROOTS, "this backend cannot pin the referents of ambiguous roots");

    pub fn new() -> Self {
        let () = Self::CHECK;
        Gc { ctl: Control { stress: 0, stress_count: 0, disabled: false, collections: 0, minors: 0, allocated_at_gc: 0, time: Duration::ZERO }, backend: B::new(), _lang: PhantomData }
    }

    /// Put `obj` in the heap.  May collect: everything the caller still needs must be
    /// reachable from the roots or from `obj` itself (its slots are roots during this call).
    /// The returned pointer is valid until a collection finds the object unreachable.
    #[inline(always)]
    pub fn alloc<F: FnOnce() -> L::Obj>(&mut self, lang: &mut L, make: F) -> NonNull<L::Obj> {
        if self.ctl.stress != 0 { return self.alloc_stressed(lang, make()); }
        self.backend.alloc(lang, &mut self.ctl, make)
    }

    #[cold]
    #[inline(never)]
    fn alloc_stressed(&mut self, lang: &mut L, obj: L::Obj) -> NonNull<L::Obj> {
        let obj = self.stress_alloc(lang, obj);
        self.backend.alloc(lang, &mut self.ctl, move || obj)
    }

    #[cold]
    #[inline(never)]
    fn stress_alloc(&mut self, lang: &mut L, mut obj: L::Obj) -> L::Obj {
        if !self.ctl.disabled {
            self.ctl.stress_count += 1;
            if self.ctl.stress_count >= self.ctl.stress {
                self.ctl.stress_count = 0;
                self.ctl.collect(&mut self.backend, lang, Some(&mut obj));
            }
        }
        obj
    }

    /// Collect now, GC.disable or not.
    pub fn collect(&mut self, lang: &mut L) { self.ctl.collect(&mut self.backend, lang, None) }

    /// A minor collection (young = allocated since the last collection).  No backend has
    /// generations, so it is a full one, except when nothing is young: then there is nothing to free.
    pub fn collect_minor(&mut self, lang: &mut L) {
        if self.backend.stats().allocated == self.ctl.allocated_at_gc {
            self.ctl.collections += 1;
            self.ctl.minors += 1;
        } else {
            self.collect(lang);
        }
    }

    /// Stress mode: collect at every `n`-th allocation (0 = off).  The test for the rooting discipline.
    pub fn set_stress(&mut self, n: usize) { self.ctl.stress = n; self.ctl.stress_count = 0; }
    pub fn stress(&self) -> usize { self.ctl.stress }
    /// GC.disable / GC.enable; returns the previous state.
    pub fn set_disabled(&mut self, d: bool) -> bool { std::mem::replace(&mut self.ctl.disabled, d) }

    /// Store a reference into a slot of `holder`, with the backend's write barrier.
    /// Mandatory for every such store once a generational or incremental backend is used;
    /// today's backends have an empty barrier.
    #[inline(always)]
    pub fn store(&mut self, holder: NonNull<L::Obj>, slot: &mut L::Ref, v: L::Ref) {
        *slot = v;
        self.backend.barrier(holder, v);
    }
    /// The barrier alone, for slots of `holder` already written (a bulk copy, a side table).
    #[inline(always)]
    pub fn barrier_holder(&mut self, holder: NonNull<L::Obj>) { self.backend.barrier_holder(holder) }

    pub fn pin(&mut self, obj: NonNull<L::Obj>) { self.backend.pin(obj) }
    #[inline(always)]
    pub fn account_external(&mut self, bytes: usize) { self.backend.account_external(bytes) }
    /// `f` must not allocate from this heap or collect.
    pub fn each_object<F: FnMut(NonNull<L::Obj>)>(&mut self, f: F) { self.backend.each_object(f) }
    pub fn stats(&self) -> HeapStats { self.backend.stats() }
    pub fn backend_name(&self) -> &'static str { B::NAME }
}

/// The header of the object in slot `p`.
///
/// # Safety
/// `p` is a slot of a heap of `L` whose header word is initialized.
#[inline(always)]
unsafe fn header<'a, L: GcLang>(p: *mut L::Obj) -> &'a GcHeader {
    // SAFETY: GcLang puts a GcHeader at this offset; slot memory is never unmapped
    unsafe { &*((p as *const u8).add(L::HEADER_OFFSET) as *const GcHeader) }
}
