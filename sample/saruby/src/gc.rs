// saruby's binding to the ASTrust GC framework (runtime/gc, docs/gc.md): the heap
// instance, the backend choice, and what the collector asks of the language —
// roots (vm.rs, fiber.rs), the edges of an object, finalization.
//
// Roots today: heap objects are traced exactly; the machine stacks (with the
// callee-saved registers) and the VM value stacks are ambiguous roots, so nothing
// they point at can move.
//
// Rule for Rust code: a Value may live in locals/registers freely, but never only
// in a Rust heap buffer (Vec<Value> etc.) across an allocation — use a Ruby Array.
// (The body passed to `alloc` is the exception: its values are roots during that call.)
use crate::astrust_gc::{self as rt, Gc, GcLang};
pub use crate::astrust_gc::{EdgeVisitor, GcHeader, RootVisitor};
use crate::object::*;
use crate::value::*;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// The language binding.  Stateless: saruby's roots are globals (`vm()`, the fiber list).
pub struct Saruby;

#[cfg(not(feature = "gc-bitmap"))]
pub type Backend = rt::mark_sweep::MarkSweep<Saruby>;
#[cfg(feature = "gc-bitmap")]
pub type Backend = rt::mark_bitmap::MarkBitmap<Saruby>;

pub type Heap = Gc<Saruby, Backend>;
/// What `RData::visit` receives (object-safe form of the visitor; ambiguous words for fiber stacks).
pub type Visitor<'a> = dyn RootVisitor<Saruby> + 'a;

static mut HEAP: Option<Heap> = None;
static STACK_BASE: AtomicUsize = AtomicUsize::new(0);
static STRESS: AtomicUsize = AtomicUsize::new(0);

// Private: each `&mut Heap` lives inside one function below, and none re-enters: the collector's
// callbacks (visit_roots, scan_edges, finalize, Data visit / drop) call no gc.rs function.
// (a closure form, with_heap(|h| ..), measured +23 % instructions on gcchurn AOT)
#[allow(static_mut_refs)]
#[inline(always)]
fn heap() -> &'static mut Heap { unsafe { HEAP.as_mut().unwrap_unchecked() } }   // SAFETY: init runs first; see above

/// Top of the main machine stack.
#[inline(always)]
pub fn stack_base() -> usize { STACK_BASE.load(Relaxed) }

pub fn init(stack_base: usize) {
    unsafe { HEAP = Some(Heap::new()); }
    STACK_BASE.store(stack_base, Relaxed);
    if let Ok(s) = std::env::var("SARUBY_GC_STRESS") { STRESS.store(s.parse().unwrap_or(1).max(1), Relaxed); }
}

/// The VM has booted: SARUBY_GC_STRESS=n (collect at every n-th allocation) takes effect.
pub fn booted() { heap().set_stress(STRESS.load(Relaxed)); }

#[inline]
pub fn malloc_increase(n: usize) { heap().account_external(n); }

/// Allocate an object.  May run a GC: every live Value must be reachable from the
/// machine stack, the VM value stack, the heap, or `body`.
#[inline]
pub fn alloc(klass: Value, body: Body) -> Value {
    let p = heap().alloc(&mut Saruby, move || RObj { klass, flags: 0, gc: GcHeader::new(), body });
    // SAFETY: the object just allocated
    unsafe { Value::from_ptr(p.as_ptr()) }
}

/// alloc, with the body built by `make` once the slot is known: a body built first and passed
/// in sits on the stack and is copied with wide loads its narrow stores cannot forward to.
#[inline(always)]
pub fn alloc_with(klass: Value, make: impl FnOnce() -> Body) -> Value {
    let p = heap().alloc(&mut Saruby, move || RObj { klass, flags: 0, gc: GcHeader::new(), body: make() });
    // SAFETY: the object just allocated
    unsafe { Value::from_ptr(p.as_ptr()) }
}

/// GC.start: a full collection, GC.disable or not (as CRuby: GC.disable stops only the automatic ones).
pub fn collect() { heap().collect(&mut Saruby); }

/// A collection the runtime asks for (as an allocation would), unless GC.disable is in effect.
pub fn collect_auto() {
    let h = heap();
    if !h.ctl.disabled() { h.collect(&mut Saruby); }
}

/// Collections so far.
pub fn collections() -> usize { heap().ctl.collections() }
/// Minor collections so far.
pub fn minor_collections() -> usize { heap().ctl.minor_collections() }
/// Time spent collecting.
pub fn total_time() -> std::time::Duration { heap().ctl.time() }
/// Slot counts (GC.stat, ObjectSpace.count_objects).
pub fn stats() -> rt::HeapStats { heap().stats() }
pub fn backend_name() -> &'static str { heap().backend_name() }
/// GC.disable / GC.enable: the previous state.
pub fn set_disabled(d: bool) -> bool { heap().set_disabled(d) }

/// GC.start(full_mark: false), GC.disable or not.
pub fn collect_minor() { heap().collect_minor(&mut Saruby); }

/// Store `v` into `slot`, a reference slot of heap object `holder`, with the write barrier.
/// No backend needs the barrier yet and saruby's stores do not go through here; a
/// generational or incremental backend requires every such store to (docs/gc.md).
#[inline(always)]
pub fn store(holder: Value, slot: &mut Value, v: Value) {
    debug_assert!(holder.is_heap());
    // SAFETY: a heap Value is a non-null object pointer
    heap().store(unsafe { NonNull::new_unchecked(holder.ptr()) }, slot, v);
}

/// Iterate live objects (ObjectSpace.each_object).  `f` must not allocate objects.
pub fn each_object(mut f: impl FnMut(Value)) {
    // SAFETY: the backend enumerates allocated objects
    heap().each_object(|p| f(unsafe { Value::from_ptr(p.as_ptr()) }));
}

// Generic ivars (objects without an ivar table of their own) live in a side table keyed by
// address.  Rare, so kept out of the mark and sweep loops.
#[cold]
#[inline(never)]
fn visit_gen_ivars<V: EdgeVisitor<Saruby>>(o: *mut RObj, v: &mut V) {
    if let Some(iv) = crate::vm::vm().gen_ivars.get_mut(&(o as usize)) {
        for (_, x) in iv.iter_mut() { v.visit(x); }
    }
}
#[cold]
#[inline(never)]
fn remove_gen_ivars(p: *mut RObj) { crate::vm::vm().gen_ivars.remove(&(p as usize)); }

// The kinds that die most often own nothing when small; drop_in_place would dispatch through a jump table.
#[inline(always)]
fn owns_nothing(b: &Body) -> bool {
    match b {
        Body::Array(a) => !a.is_heap(),
        Body::String(s) => !s.b.is_heap(),
        Body::Object(iv) => iv.is_inline(),
        Body::Env(e, _) => !e.is_heap(),
        Body::Float(_) | Body::Range(..) | Body::Free(_) => true,
        _ => false,
    }
}

// SAFETY (caller): p is a dead, initialized object, finalized exactly once
#[inline(never)]
unsafe fn drop_body(p: *mut RObj) {
    unsafe {
        if let Body::Data(_) = &(*p).body { crate::vm::on_free_data(p); }
        std::ptr::drop_in_place(&mut (*p).body);
    }
}

// SAFETY: RObj carries its GcHeader in `gc` and is 8-aligned; heap Values are made only by `alloc`.
unsafe impl GcLang for Saruby {
    type Ref = Value;
    type Obj = RObj;
    const HEADER_OFFSET: usize = std::mem::offset_of!(RObj, gc);
    const AMBIGUOUS_ROOTS: bool = true;

    #[inline(always)]
    fn as_object(v: Value) -> Option<NonNull<RObj>> {
        // SAFETY: no Value is zero
        if v.is_heap() { Some(unsafe { NonNull::new_unchecked(v.ptr()) }) } else { None }
    }
    #[inline(always)]
    fn from_object(p: NonNull<RObj>) -> Value { unsafe { Value::from_ptr(p.as_ptr()) } }   // SAFETY: an object of this heap (GcLang contract)

    fn visit_roots<V: RootVisitor<Self>>(&mut self, v: &mut V) {
        // SAFETY: current_stack_top() is the end of the stack this code runs on
        unsafe { rt::stack::with_current_stack(crate::fiber::current_stack_top(), |w| v.visit_ambiguous(w)); }
        crate::vm::visit_roots(v);
    }

    #[inline(always)]
    fn scan_edges<V: RootVisitor<Self>>(&mut self, o: &mut RObj, v: &mut V) {
        v.visit(&mut o.klass);
        if o.flags & FL_EXIVAR != 0 { visit_gen_ivars(o, v); }
        match &mut o.body {
            Body::Object(ivs) => for x in ivs.iter_mut() { v.visit(x) },
            Body::Array(a) => for x in a { v.visit(x) },
            Body::Hash(h) => {
                for e in &mut h.entries { v.visit(&mut e.key); v.visit(&mut e.val); }
                v.visit(&mut h.default); v.visit(&mut h.default_proc);
            }
            Body::Range(a, b, _) => { v.visit(a); v.visit(b); }
            Body::Proc(pd) => { crate::vm::visit_block(v, &mut pd.blk); v.visit(&mut pd.from_method); }
            Body::Env(vals, parent) => { for x in vals.iter_mut() { v.visit(x) } v.visit(parent); }
            Body::Method(m) => { v.visit(&mut m.recv); v.visit(&mut m.owner); crate::vm::visit_me(v, m.me); }
            Body::Class(cd) => crate::vm::visit_class(v, cd),
            Body::Data(d) => d.visit(v),
            Body::String(_) | Body::Float(_) | Body::Bignum(_) | Body::Free(_) => {}
        }
    }

    #[inline(always)]
    unsafe fn finalize(&mut self, p: NonNull<RObj>) {
        let p = p.as_ptr();
        // SAFETY: the collector hands us a dead, initialized object exactly once
        unsafe {
            if (*p).flags & FL_EXIVAR != 0 { remove_gen_ivars(p); }
            if !owns_nothing(&(*p).body) { drop_body(p); }
            // poison: a stale reference finds a Free body, not a dropped payload
            std::ptr::write(&mut (*p).body, Body::Free(std::ptr::null_mut()));
        }
    }
}
