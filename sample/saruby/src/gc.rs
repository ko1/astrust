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
use crate::vm::Ctx;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// The language binding is the context that allocates or collects (NO_CTX when none is given):
/// the collector scans that context through the caller's borrow.
pub type Saruby = Ctx;

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

// `heap(&mut Scope)`: the temporary Scope ends with the statement, so no `&mut Heap` outlives it.
// None nests: the collector's callbacks call no gc.rs function (asserted in debug builds).
struct Scope;

#[cfg(debug_assertions)]
static IN_HEAP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(debug_assertions)]
impl Drop for Scope { fn drop(&mut self) { IN_HEAP.store(false, Relaxed); } }

#[inline(always)]
fn heap(_: &mut Scope) -> &mut Heap {
    #[cfg(debug_assertions)]
    assert!(!IN_HEAP.swap(true, Relaxed), "gc: nested heap borrow");
    // SAFETY: init runs first; one borrow at a time (above)
    unsafe { (*&raw mut HEAP).as_mut().unwrap_unchecked() }
}

// What context-free allocations pass: never runs, and the collector reads no object through it.
static mut NO_CTX: Ctx = Ctx {
    sp: crate::stack::NO_SP, frame: std::ptr::null_mut(), serial: 0, call_me: std::ptr::null(), native_limit: 0,
    call_blk: std::ptr::null(), stk: crate::stack::NO_STACK, esc: crate::vm::EscKind::None, esc_val: NIL, esc_tag: 0,
    esc_tag_v: NIL, errinfo: NIL, last_match: NIL, in_ensure: 0, redef: 0,
    // SAFETY: as above, NO_CTX is never a running context's token
    tok: unsafe { crate::heap::Token::mint() },
};

#[inline(always)]
fn no_ctx(_: &mut Scope) -> &mut Ctx {
    // SAFETY: one borrow at a time, as heap()
    unsafe { &mut *&raw mut NO_CTX }
}
/// `c` is NO_CTX: the allocation gave no context.
#[inline(always)]
pub fn is_no_ctx(c: *const Ctx) -> bool { std::ptr::eq(c, &raw const NO_CTX) }

/// Top of the main machine stack.
#[inline(always)]
pub fn stack_base() -> usize { STACK_BASE.load(Relaxed) }

pub fn init(stack_base: usize) {
    unsafe { HEAP = Some(Heap::new()); }
    STACK_BASE.store(stack_base, Relaxed);
    if let Ok(s) = std::env::var("SARUBY_GC_STRESS") { STRESS.store(s.parse().unwrap_or(1).max(1), Relaxed); }
}

/// The VM has booted: SARUBY_GC_STRESS=n (collect at every n-th allocation) takes effect.
pub fn booted() { heap(&mut Scope).set_stress(STRESS.load(Relaxed)); }

#[inline]
pub fn malloc_increase(n: usize) { heap(&mut Scope).account_external(n); }

/// Allocate an object.  May run a GC: every live Value must be reachable from the
/// machine stack, the VM value stack, the heap, or `body`.
/// Context-free (no `&mut Ctx` given): the running context is scanned by place (vm::visit_roots).
#[inline]
pub fn alloc(klass: Value, body: Body) -> Value { alloc_by(no_ctx(&mut Scope), klass, body) }
/// alloc_with, context-free.
#[inline(always)]
pub fn alloc_with(klass: Value, make: impl FnOnce() -> Body) -> Value { alloc_with_by(no_ctx(&mut Scope), klass, make) }
/// alloc by `c` (Ctx::alloc): a collection scans `c` through this borrow.
#[inline(always)]
pub fn alloc_in(c: &mut Ctx, klass: Value, body: Body) -> Value { alloc_by(c, klass, body) }
#[inline(always)]
pub fn alloc_with_in(c: &mut Ctx, klass: Value, make: impl FnOnce() -> Body) -> Value { alloc_with_by(c, klass, make) }

#[inline]
fn alloc_by(c: &mut Ctx, klass: Value, body: Body) -> Value {
    let p = heap(&mut Scope).alloc(c, move || RObj { klass, flags: 0, gc: GcHeader::new(), body });
    // SAFETY: the object just allocated
    unsafe { Value::from_ptr(p.as_ptr()) }
}

/// alloc, with the body built by `make` once the slot is known: a body built first and passed
/// in sits on the stack and is copied with wide loads its narrow stores cannot forward to.
#[inline(always)]
fn alloc_with_by(c: &mut Ctx, klass: Value, make: impl FnOnce() -> Body) -> Value {
    let p = heap(&mut Scope).alloc(c, move || RObj { klass, flags: 0, gc: GcHeader::new(), body: make() });
    // SAFETY: the object just allocated
    unsafe { Value::from_ptr(p.as_ptr()) }
}

/// GC.start: a full collection, GC.disable or not (as CRuby: GC.disable stops only the automatic ones).
pub fn collect(c: &mut Ctx) { heap(&mut Scope).collect(c); }

/// A collection the runtime asks for (as an allocation would), unless GC.disable is in effect.
pub fn collect_auto(c: &mut Ctx) {
    // the condition's Scope ends before the block
    if !heap(&mut Scope).ctl.disabled() { collect(c); }
}

/// Collections so far.
pub fn collections() -> usize { heap(&mut Scope).ctl.collections() }
/// Minor collections so far.
pub fn minor_collections() -> usize { heap(&mut Scope).ctl.minor_collections() }
/// Time spent collecting.
pub fn total_time() -> std::time::Duration { heap(&mut Scope).ctl.time() }
/// Slot counts (GC.stat, ObjectSpace.count_objects).
pub fn stats() -> rt::HeapStats { heap(&mut Scope).stats() }
pub fn backend_name() -> &'static str { heap(&mut Scope).backend_name() }
/// GC.disable / GC.enable: the previous state.
pub fn set_disabled(d: bool) -> bool { heap(&mut Scope).set_disabled(d) }

/// GC.start(full_mark: false), GC.disable or not.
pub fn collect_minor(c: &mut Ctx) { heap(&mut Scope).collect_minor(c); }

/// Store `v` into `slot`, a reference slot of heap object `holder`, with the write barrier.
/// No backend needs the barrier yet and saruby's stores do not go through here; a
/// generational or incremental backend requires every such store to (docs/gc.md).
#[inline(always)]
pub fn store(holder: Value, slot: &mut Value, v: Value) {
    debug_assert!(holder.is_heap());
    // SAFETY: a heap Value is a non-null object pointer
    heap(&mut Scope).store(unsafe { NonNull::new_unchecked(holder.ptr()) }, slot, v);
}

/// Iterate live objects (ObjectSpace.each_object).  `f` must not allocate objects.
pub fn each_object(mut f: impl FnMut(Value)) {
    // SAFETY: the backend enumerates allocated objects
    heap(&mut Scope).each_object(|p| f(unsafe { Value::from_ptr(p.as_ptr()) }));
}

// Generic ivars (objects without an ivar table of their own) live in a side table keyed by
// address.  Rare, so kept out of the mark and sweep loops.
#[cold]
#[inline(never)]
fn visit_gen_ivars<V: EdgeVisitor<Saruby>>(o: *mut RObj, v: &mut V) {
    if let Some(iv) = crate::vm::vm().gen_ivars.borrow_mut().get_mut(&(o as usize)) {
        for (_, x) in iv.iter_mut() { v.visit(x); }
    }
}
#[cold]
#[inline(never)]
fn remove_gen_ivars(p: *mut RObj) { crate::vm::vm().gen_ivars.borrow_mut().remove(&(p as usize)); }

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
    use std::ptr::drop_in_place as d;
    unsafe {
        if let Body::Data(_) = &(*p).body { crate::vm::on_free_data(p); }
        // per variant: Body's drop glue was left out of line by some builds (nesteddata +0.6 % at 3ae9503)
        match &mut (*p).body {
            Body::Free(_) | Body::Float(_) | Body::Range(..) => {}
            Body::Object(x) => d(x),
            Body::Class(x) => d(x),
            Body::String(x) => d(x),
            Body::Array(x) => d(x),
            // in place (entries, index: its only fields with drop glue): Box<RHash>'s glue was left out of line by some partitions
            Body::Hash(x) => {
                let h: *mut RHash = &mut **x;
                d(&mut (*h).entries); d(&mut (*h).index);
                std::alloc::dealloc(h.cast(), std::alloc::Layout::new::<RHash>());
            }
            Body::Bignum(x) => d(x),
            Body::Proc(x) => d(x),
            Body::Env(x, _) => d(x),
            Body::Method(x) => d(x),
            Body::Data(x) => d(x),
        }
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
        crate::vm::visit_roots(v, self);
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
