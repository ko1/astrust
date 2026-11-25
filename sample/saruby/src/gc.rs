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
pub use crate::astrust_gc::{EdgeVisitor, GcHeader, RootVisitor, WeakVisitor};
use crate::heap::{obj_flags, obj_klass};
use crate::object::*;
use crate::value::*;
use crate::vm::Ctx;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// The language binding is the context that allocates or collects: the collector scans that
/// context through the caller's borrow.
pub type Saruby = Ctx;

#[cfg(all(not(feature = "gc-bitmap"), not(feature = "gc_check")))]
pub type Backend = rt::mark_sweep::MarkSweep<Saruby>;
// make FEATURES=gc_check: quarantine + poison, and precise zones (docs/gc_moving.md 2.5, 11)
#[cfg(all(not(feature = "gc-bitmap"), feature = "gc_check"))]
pub type Backend = rt::mark_sweep::MarkSweep<Saruby, true>;
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

/// Top of the main machine stack.
#[inline(always)]
pub fn stack_base() -> usize { STACK_BASE.load(Relaxed) }

static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
/// Seconds from process start to `t` (GC::Profiler's invoke time).
pub fn since_start(t: std::time::Instant) -> f64 { t.saturating_duration_since(*START.get_or_init(std::time::Instant::now)).as_secs_f64() }

pub fn init(stack_base: usize) {
    START.get_or_init(std::time::Instant::now);
    unsafe { HEAP = Some(Heap::new()); }
    STACK_BASE.store(stack_base, Relaxed);
    if let Ok(s) = std::env::var("SARUBY_GC_STRESS") { STRESS.store(s.parse().unwrap_or(1).max(1), Relaxed); }
}

/// The VM has booted: SARUBY_GC_STRESS=n (collect at every n-th allocation) takes effect.
pub fn booted() { heap(&mut Scope).set_stress(STRESS.load(Relaxed)); }

#[inline]
pub fn malloc_increase(n: usize) { heap(&mut Scope).account_external(n); }

/// Allocate an object by `c` (Ctx::alloc).  May run a GC, which scans `c` through this borrow:
/// every live Value must be reachable from the machine stack, the VM value stack, the heap, or `body`.
#[inline(always)]
pub fn alloc(c: &mut Ctx, klass: Value, body: Body) -> Value { alloc_by(c, klass, body) }
#[inline(always)]
pub fn alloc_with(c: &mut Ctx, klass: Value, make: impl FnOnce() -> Body) -> Value { alloc_with_by(c, klass, 0, make) }
#[inline(always)]
pub fn alloc_with_flags(c: &mut Ctx, klass: Value, flags: u32, make: impl FnOnce() -> Body) -> Value { alloc_with_by(c, klass, flags, make) }

#[inline]
fn alloc_by(c: &mut Ctx, klass: Value, body: Body) -> Value {
    let p = heap(&mut Scope).alloc(c, move || RObj { klass, flags: 0, gc: GcHeader::new(), body });
    // SAFETY: the object just allocated
    unsafe { Value::from_ptr(p.as_ptr()) }
}

/// alloc, with the body built by `make` once the slot is known: a body built first and passed
/// in sits on the stack and is copied with wide loads its narrow stores cannot forward to.
#[inline(always)]
fn alloc_with_by(c: &mut Ctx, klass: Value, flags: u32, make: impl FnOnce() -> Body) -> Value {
    let p = heap(&mut Scope).alloc(c, move || RObj { klass, flags, gc: GcHeader::new(), body: make() });
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

/// GC.stress=: collect at every allocation (0: off; SARUBY_GC_STRESS's n is replaced).
pub fn set_stress(n: usize) { heap(&mut Scope).set_stress(n); }
/// GC::Profiler.enable / .disable (records are dropped when disabled).
pub fn set_profile(on: bool) {
    let mut s = Scope;
    let h = heap(&mut s);
    if on != h.ctl.profile.is_some() { h.ctl.profile = if on { Some(vec![]) } else { None }; }
}
/// GC::Profiler records: None while disabled.
pub fn profile_records() -> Option<Vec<rt::GcRecord>> { heap(&mut Scope).ctl.profile.clone() }
pub fn profile_clear() { if let Some(r) = &mut heap(&mut Scope).ctl.profile { r.clear(); } }

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

/// Call `GcLang::allocated` (objspace's allocation tracer) for every allocation, or stop.
pub fn set_alloc_hook(on: bool) { heap(&mut Scope).set_alloc_hook(on); }

// Heap references in visiting order, once each; ambiguous words are left out.
struct Collect(Vec<Value>);
impl EdgeVisitor<Saruby> for Collect {
    fn visit(&mut self, s: &mut Value) { if s.is_heap() && !self.0.contains(s) { self.0.push(*s); } }
}
impl RootVisitor<Saruby> for Collect { fn visit_ambiguous(&mut self, _: &[usize]) {} }

/// The exact roots (ObjectSpace.reachable_objects_from_root, dump_all): machine stacks are not resolved.
pub fn exact_roots(c: &mut Ctx) -> Vec<Value> {
    let mut col = Collect(vec![]);
    crate::vm::visit_roots(&mut col, c);
    col.0
}

/// The objects `v` refers to (ObjectSpace.reachable_objects_from): what marking would visit, once each.
pub fn reachable_from(c: &mut Ctx, v: Value) -> Vec<Value> {
    let mut col = Collect(vec![]);
    let (mut k, fl) = (obj_klass(v), obj_flags(v));
    col.visit(&mut k);
    if fl & FL_EXIVAR != 0 { visit_gen_ivars(v.raw() as usize, &mut col); }
    // a weak table's entries are not references
    if fl & (FL_WEAK_KEYS | FL_WEAK_VALS) == 0 { scan_body(c.body_mut_of(v), &mut col); }
    col.0
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
fn visit_gen_ivars<V: EdgeVisitor<Saruby>>(addr: usize, v: &mut V) {
    if let Some(iv) = crate::vm::vm().gen_ivars.borrow_mut().get_mut(&addr) {
        for (_, x) in iv.iter_mut() { v.visit(x); }
    }
}

thread_local! {
    // the weak tables marked by the collection running (scan_edges), pruned by sweep_weak
    static WEAK_SEEN: std::cell::RefCell<Vec<Value>> = const { std::cell::RefCell::new(Vec::new()) };
}

// The edges behind rare flags: generic ivars, a weak table (true: the body is not to be scanned).
#[cold]
#[inline(never)]
fn rare_edges<V: EdgeVisitor<Saruby>>(o: &mut RObj, v: &mut V) -> bool {
    if o.flags & FL_EXIVAR != 0 { visit_gen_ivars(o as *mut RObj as usize, v); }
    if o.flags & (FL_WEAK_KEYS | FL_WEAK_VALS) == 0 { return false; }
    let Body::Hash(h) = &mut o.body else { return false };
    v.visit(&mut h.default); v.visit(&mut h.default_proc);
    if o.flags & FL_WEAK_VALS == 0 { for e in &mut h.entries { v.visit(&mut e.val); } }
    let t = <Saruby as GcLang>::from_object(NonNull::from(o));
    WEAK_SEEN.with_borrow_mut(|w| w.push(t));
    true
}

// What a dead object leaves in side tables (generic ivars, the allocation tracer, object ids).
#[cold]
#[inline(never)]
fn forget(p: NonNull<RObj>) {
    // the flags are read here, not passed: finalize then tests them in memory (one instruction)
    let (addr, flags) = (p.as_ptr() as usize, obj_flags(<Saruby as GcLang>::from_object(p)));
    let vm = crate::vm::vm();
    if flags & FL_EXIVAR != 0 { vm.gen_ivars.borrow_mut().remove(&addr); }
    if flags & FL_TRACED != 0 { crate::builtins::objspace::forget(addr); }
    if flags & FL_HAS_ID != 0 { vm.object_ids.borrow_mut().remove(&(addr as u64)); }
}

// The finalizer table: an entry whose object died goes to the queue (its callables stay marked).
#[cold]
#[inline(never)]
fn sweep_finalizers<W: WeakVisitor<Saruby>>(c: &mut Ctx, t: Value, w: &mut W) {
    let mut dead = vec![];
    c.hash_mut(t).prune(|e| w.alive(&mut e.key) || { dead.push(e.val); false });
    if !dead.is_empty() { crate::builtins::misc::fin_enqueue(c, dead); }
}

/// The edges of a body (scan_edges without the class and the rare flags).
#[inline(always)]
fn scan_body<V: RootVisitor<Saruby>>(b: &mut Body, v: &mut V) {
    match b {
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
            Body::Class(x) => { crate::vm::class_freed(x); d(x) }
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
        let top = crate::fiber::current_stack_top();
        unsafe { rt::stack::with_current_stack(top, |w| scan_running_stack(v, w, top)); }
        crate::vm::visit_roots(v, self);
    }

    #[inline(always)]
    fn scan_edges<V: RootVisitor<Self>>(&mut self, o: &mut RObj, v: &mut V) {
        v.visit(&mut o.klass);
        if o.flags & (FL_EXIVAR | FL_WEAK_KEYS | FL_WEAK_VALS) != 0 && rare_edges(o, v) { return; }
        scan_body(&mut o.body, v);
    }

    /// Weak tables (WeakMap, WeakKeyMap) lose the entries whose weak key or value died.
    fn sweep_weak<W: WeakVisitor<Self>>(&mut self, w: &mut W) {
        let seen = WEAK_SEEN.take();
        for t in seen {
            if t == crate::builtins::misc::fin_table_value(self) { sweep_finalizers(self, t, w); continue; }
            let vals = obj_flags(t) & FL_WEAK_VALS != 0;
            self.hash_mut(t).prune(|e| w.alive(&mut e.key) && (!vals || w.alive(&mut e.val)));
        }
    }

    #[inline(never)]
    fn allocated(&mut self, p: NonNull<RObj>, ctl: &rt::Control) {
        crate::builtins::objspace::allocated(self, Self::from_object(p), ctl.collections());
    }

    #[inline(always)]
    unsafe fn finalize(&mut self, obj: NonNull<RObj>) {
        let p = obj.as_ptr();
        // SAFETY: the collector hands us a dead, initialized object exactly once
        unsafe {
            if (*p).flags & (FL_EXIVAR | FL_TRACED | FL_HAS_ID) != 0 { forget(obj); }
            if !owns_nothing(&(*p).body) { drop_body(p); }
            // poison: a stale reference finds a Free body, not a dropped payload
            std::ptr::write(&mut (*p).body, Body::Free(std::ptr::null_mut()));
        }
    }

    #[cfg(feature = "gc_check")]
    unsafe fn poison(&mut self, obj: NonNull<RObj>) {
        // SAFETY: a dead slot the backend keeps off the free list; its klass is ours to overwrite
        unsafe { (*obj.as_ptr()).klass = poison_klass(); }
    }
}

// A class word that faults when used: one PROT_NONE page (gc_check builds only).
#[cfg(feature = "gc_check")]
fn poison_klass() -> Value {
    static P: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    extern "C" { fn mmap(a: *mut u8, l: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut u8; }
    // SAFETY: an anonymous PROT_NONE mapping, never unmapped; its address is only compared or faulted on
    let p = *P.get_or_init(|| unsafe { mmap(std::ptr::null_mut(), 65536, 0, 0x22, -1, 0) } as usize);
    // SAFETY: page-aligned, so it has a heap Value's tag; dereferencing it faults, which is the point
    unsafe { Value::from_ptr(p as *mut RObj) }
}

/// GC.__check_rooting(mode), gc_check builds: the harness self-test (docs/gc_moving.md 2.5).
/// 0: a String parked across allocations (survives); 1: held in a Rust local inside a precise zone
/// (must fault under SARUBY_GC_STRESS=1); 2: in a local outside any zone (the stack scan keeps it).
#[cfg(feature = "gc_check")]
pub fn check_rooting(c: &mut Ctx, _s: Value, a: &[Value], _b: crate::vm::Blk) -> crate::vm::RV {
    fn churn(c: &mut Ctx) { for _ in 0..64 { std::hint::black_box(c.str_new(b"churn churn churn churn churn churn")); } }
    fn reread(c: &mut Ctx, s: Value) -> Value { let b = c.str_bytes(s).to_vec(); c.str_new(&b) }
    let mode = a.first().map_or(0, |v| v.as_fix());
    match mode {
        0 => c.park(1, |c, p| precise(c, |c| { let s = c.str_new(b"parked"); p.set(0, s); churn(c); Ok(reread(c, p.get(0))) })),
        1 => precise(c, |c| { let s = c.str_new(b"local"); churn(c); Ok(reread(c, std::hint::black_box(s))) }),
        _ => { let s = c.str_new(b"conservative"); churn(c); Ok(reread(c, std::hint::black_box(s))) }
    }
}

// ---- precise zones (docs/gc_moving.md section 11) ---------------------------------------------
// The running stack is scanned conservatively only outside precise zones: converted code keeps no
// root in Rust locals, so its frames (and those it calls, until a `conservative` zone) are skipped.

#[cfg(feature = "gc_check")]
thread_local! {
    // (stack top, fence, precise), in entry order: on one stack the fences descend
    static ZONES: std::cell::RefCell<Vec<(usize, usize, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Run `f` as a precise zone: values it keeps in Rust locals across a GC point are not roots.
/// Without `gc_check` this is `f(c)`.
#[inline(always)]
pub fn precise<R>(c: &mut Ctx, f: impl FnOnce(&mut Ctx) -> R) -> R {
    #[cfg(feature = "gc_check")] { zone(c, true, f) }
    #[cfg(not(feature = "gc_check"))] { f(c) }
}

/// Inside a precise zone, run code that is not converted (its frames are scanned again).
#[inline(always)]
pub fn conservative<R>(c: &mut Ctx, f: impl FnOnce(&mut Ctx) -> R) -> R {
    #[cfg(feature = "gc_check")] { zone(c, false, f) }
    #[cfg(not(feature = "gc_check"))] { f(c) }
}

#[cfg(feature = "gc_check")]
fn zone<R>(c: &mut Ctx, precise: bool, f: impl FnOnce(&mut Ctx) -> R) -> R {
    // removes its own entry: fibers interleave entries of different stacks
    struct Guard(usize, usize);
    impl Drop for Guard {
        fn drop(&mut self) {
            ZONES.with_borrow_mut(|z| if let Some(i) = z.iter().rposition(|e| e.0 == self.0 && e.1 == self.1) { z.remove(i); });
        }
    }
    rt::stack::with_fence(move |fence| {
        let top = crate::fiber::current_stack_top();
        ZONES.with_borrow_mut(|z| z.push((top, fence, precise)));
        let _g = Guard(top, fence);
        f(c)
    })
}

// The ambiguous words of the running stack (`w` runs from the collector's frame up to `top`).
#[inline(always)]
fn scan_running_stack<V: RootVisitor<Saruby>>(v: &mut V, w: &[usize], top: usize) {
    #[cfg(not(feature = "gc_check"))] { let _ = top; v.visit_ambiguous(w); }
    #[cfg(feature = "gc_check")] {
        let lo = w.as_ptr() as usize;
        let part = |a: usize, b: usize| &w[((a - lo) / 8).min(w.len())..((b.max(a) - lo) / 8).min(w.len())];
        ZONES.with_borrow(|z| {
            let (mut hi, mut cons) = (top, true);
            for &(_, fence, prec) in z.iter().filter(|e| e.0 == top) {
                if fence < lo || fence >= hi { continue; }
                if cons && prec { v.visit_ambiguous(part(fence, hi)); cons = false; }
                else if !cons && !prec { hi = fence; cons = true; }
            }
            if cons { v.visit_ambiguous(part(lo, hi)); }
        });
    }
}

/// Value-stack slots reserved for one scope (docs/gc_moving.md 2.2, idiom C): roots while the
/// scope runs.  The brand keeps a `Park` from outliving it.
pub struct Park<'s> { st: crate::stack::Stk, _brand: std::marker::PhantomData<fn(&'s ()) -> &'s ()> }
impl<'s> Park<'s> {
    #[inline(always)] pub fn get(&self, i: usize) -> Value { self.st[i].get() }
    #[inline(always)] pub fn set(&self, i: usize, v: Value) { self.st[i].set(v) }
}

impl Ctx {
    /// Reserve `n` nil slots above sp for `f`; sp is restored when `f` returns.
    pub fn park<R>(&mut self, n: usize, f: impl for<'s> FnOnce(&mut Ctx, Park<'s>) -> Result<R, crate::vm::Esc>) -> Result<R, crate::vm::Esc> {
        let m = self.sp_mark();
        let st = crate::stack::stage(self, n)?;
        for i in 0..n { st[i].set(NIL); }
        let r = f(self, Park { st, _brand: std::marker::PhantomData });
        self.sp_reset(m);
        r
    }

    /// An Array of new Strings: each is in the parked Array before the next allocation (a Vec of
    /// them is no root: docs/gc_moving.md 2.4).
    pub fn ary_of_new_strs<B: AsRef<[u8]>>(&mut self, items: impl IntoIterator<Item = B>) -> crate::vm::RV {
        self.park(1, |c, p| {
            let a = c.ary_new();
            p.set(0, a);
            for b in items { let s = c.str_new(b.as_ref()); c.ary_push(p.get(0), s); }
            Ok(p.get(0))
        })
    }

    /// A converted cfunc's entry (docs/gc_moving.md 11): recv and args parked in slots 0..=argc,
    /// the body run as a precise zone.  The body re-reads them from the park after each GC point.
    pub fn precise_cfunc<R>(&mut self, recv: Value, args: &[Value], f: impl for<'s> FnOnce(&mut Ctx, Park<'s>) -> Result<R, crate::vm::Esc>) -> Result<R, crate::vm::Esc> {
        self.park(1 + args.len(), |c, p| {
            p.set(0, recv);
            for (i, &a) in args.iter().enumerate() { p.set(1 + i, a); }
            precise(c, |c| f(c, p))
        })
    }
}
