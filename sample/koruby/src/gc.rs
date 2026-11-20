// Non-moving mark & sweep GC.
//
// Rooting follows CRuby: heap objects are traced precisely, and the machine stack
// (plus callee-saved registers) is scanned conservatively — any word that points
// into a live slot (interior pointers included) keeps it alive.  The VM value
// stack is scanned the same way (slots above the last write may hold stale words).
//
// Rule for Rust code: a Value may live in locals/registers freely, but never only
// in a Rust heap buffer (Vec<Value> etc.) across an allocation — use a Ruby Array.
use crate::object::*;
use crate::value::*;
use crate::fxhash::HashSet;

pub const PAGE_BYTES: usize = 64 * 1024;
pub const SLOT: usize = std::mem::size_of::<RObj>();
pub const PER_PAGE: usize = PAGE_BYTES / SLOT;

pub struct Heap {
    pub pages: Vec<usize>,
    pub page_set: HashSet<usize>,
    pub free: *mut RObj,
    pub free_count: usize,
    pub total: usize,
    pub live_after_gc: usize,
    pub allocs_since_gc: usize,
    pub malloc_since_gc: usize,
    pub malloc_limit: usize,
    pub gc_count: usize,
    pub gc_time: std::time::Duration,
    pub stress: usize,
    pub stress_ctr: usize,
    pub disabled: bool,
    pub stack_base: usize,
    pub extra_stacks: Vec<(usize, usize)>,   // [lo, hi) of suspended fiber stacks
    pub total_allocated: u64,
    pub total_freed: u64,
}

pub static mut HEAP: Option<Heap> = None;

#[allow(static_mut_refs)]
#[inline(always)]
pub fn heap() -> &'static mut Heap { unsafe { HEAP.as_mut().unwrap_unchecked() } }

pub fn init(stack_base: usize) {
    unsafe {
        HEAP = Some(Heap {
            pages: vec![], page_set: HashSet::default(), free: std::ptr::null_mut(), free_count: 0, total: 0,
            live_after_gc: 0, allocs_since_gc: 0, malloc_since_gc: 0, malloc_limit: 64 << 20, gc_count: 0,
            gc_time: std::time::Duration::ZERO, stress: 0, stress_ctr: 0, disabled: false, stack_base: 0, extra_stacks: vec![],
            total_allocated: 0, total_freed: 0,
        });
    }
    let h = heap();
    h.stack_base = stack_base;
    if let Ok(s) = std::env::var("KORUBY_GC_STRESS") { h.stress = s.parse().unwrap_or(1).max(1); }
    for _ in 0..32 { add_page(); }
}

fn add_page() {
    let h = heap();
    let layout = std::alloc::Layout::from_size_align(PAGE_BYTES, PAGE_BYTES).unwrap();
    let base = unsafe { std::alloc::alloc(layout) } as usize;
    if base == 0 { panic!("koruby: out of memory"); }
    h.pages.push(base);
    h.page_set.insert(base);
    for i in (0..PER_PAGE).rev() {
        let p = (base + i * SLOT) as *mut RObj;
        unsafe {
            std::ptr::write(p, RObj { klass: NIL, flags: FL_FREE, _pad: 0, body: Body::Free(h.free) });
        }
        h.free = p;
    }
    h.free_count += PER_PAGE;
    h.total += PER_PAGE;
}

#[inline]
pub fn malloc_increase(n: usize) {
    heap().malloc_since_gc += n;
}

/// Allocate a slot.  May run a GC: every live Value must be reachable from the
/// machine stack, the VM value stack, or the heap.
#[inline]
pub fn alloc(klass: Value, body: Body) -> Value {
    let h = heap();
    if h.free.is_null() || h.stress != 0 || h.malloc_since_gc > h.malloc_limit { slow_path(); }
    let p = h.free;
    unsafe {
        let next = match (*p).body { Body::Free(n) => n, _ => std::hint::unreachable_unchecked() };
        h.free = next;
        std::ptr::write(p, RObj { klass, flags: 0, _pad: 0, body });
    }
    h.free_count -= 1;
    h.allocs_since_gc += 1;
    h.total_allocated += 1;
    Value::from_ptr(p)
}

#[inline(never)]
fn slow_path() {
    let h = heap();
    // KORUBY_GC_STRESS=n: collect every n-th allocation once the VM has booted
    if h.stress != 0 && !h.disabled && unsafe { crate::rt::BOOTED } {
        h.stress_ctr += 1;
        if h.stress_ctr >= h.stress { h.stress_ctr = 0; collect(); }
    }
    if h.malloc_since_gc > h.malloc_limit && !h.disabled { collect(); }
    if h.free.is_null() {
        if !h.disabled && h.allocs_since_gc > h.total / 4 { collect(); }
        // keep the heap at >= 2x the live set: a full mark per cycle is paid by that many allocations
        let h = heap();
        if h.free.is_null() || h.free_count < h.live_after_gc {
            let want = h.live_after_gc.saturating_sub(h.free_count).max(h.total / 8) / PER_PAGE + 1;
            for _ in 0..want { add_page(); }
        }
    }
}

pub struct Marker { pub stack: Vec<*mut RObj> }

impl Marker {
    #[inline]
    pub fn mark(&mut self, v: Value) {
        if !v.is_heap() { return; }
        let p = v.ptr();
        unsafe {
            if (*p).flags & (FL_MARK | FL_FREE) != 0 { return; }
            (*p).flags |= FL_MARK;
        }
        self.stack.push(p);
    }
    /// A word that may or may not be a pointer into the heap (interior pointers allowed).
    #[inline]
    pub fn mark_maybe(&mut self, w: usize) {
        let page = w & !(PAGE_BYTES - 1);
        let h = heap();
        if !h.page_set.contains(&page) { return; }
        let idx = (w - page) / SLOT;
        if idx >= PER_PAGE { return; }
        let p = (page + idx * SLOT) as *mut RObj;
        unsafe {
            if (*p).flags & (FL_MARK | FL_FREE) != 0 { return; }
            (*p).flags |= FL_MARK;
        }
        self.stack.push(p);
    }
    pub fn mark_range(&mut self, lo: usize, hi: usize) {
        let mut a = lo & !7;
        while a + 8 <= hi {
            let w = unsafe { *(a as *const usize) };
            if w != 0 { self.mark_maybe(w); }
            a += 8;
        }
    }
    fn drain(&mut self) {
        while let Some(p) = self.stack.pop() {
            let o = unsafe { &*p };
            self.mark(o.klass);
            if o.flags & FL_EXIVAR != 0 {
                if let Some(iv) = crate::vm::vm().gen_ivars.get(&(p as usize)) {
                    let iv: &Vec<(Sym, Value)> = unsafe { &*(iv as *const _) };
                    for (_, v) in iv { self.mark(*v); }
                }
            }
            match &o.body {
                Body::Object(ivs) => for v in ivs.iter() { self.mark(v) },
                Body::Array(a) => for v in a { self.mark(*v) },
                Body::Hash(h) => {
                    for e in &h.entries { self.mark(e.key); self.mark(e.val); }
                    self.mark(h.default); self.mark(h.default_proc);
                }
                Body::Range(a, b, _) => { self.mark(*a); self.mark(*b); }
                Body::Proc(pd) => { crate::vm::mark_block(self, &pd.blk); self.mark(pd.from_method); }
                Body::Env(vals, parent) => { for v in vals { self.mark(*v) } self.mark(*parent); }
                Body::Method(m) => { self.mark(m.recv); self.mark(m.owner); crate::vm::mark_me(self, m.me); }
                Body::Class(cd) => crate::vm::mark_class(self, cd),
                Body::Data(d) => d.mark(self),
                Body::String(_) | Body::Float(_) | Body::Bignum(_) | Body::Free(_) => {}
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn dump_regs(r: &mut [usize; 8]) {
    unsafe {
        std::arch::asm!(
            "mov [{0}], rbx", "mov [{0} + 8], rbp", "mov [{0} + 16], r12", "mov [{0} + 24], r13",
            "mov [{0} + 32], r14", "mov [{0} + 40], r15", "mov [{0} + 48], rsp",
            in(reg) r.as_mut_ptr(), options(nostack));
    }
}
#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn dump_regs(r: &mut [usize; 12]) {
    unsafe {
        std::arch::asm!(
            "stp x19, x20, [{0}]", "stp x21, x22, [{0}, #16]", "stp x23, x24, [{0}, #32]",
            "stp x25, x26, [{0}, #48]", "stp x27, x28, [{0}, #64]", "stp x29, x30, [{0}, #80]",
            in(reg) r.as_mut_ptr(), options(nostack));
    }
}

#[inline(never)]
pub fn collect() {
    #[cfg(target_arch = "x86_64")]
    let mut regs = [0usize; 8];
    #[cfg(target_arch = "aarch64")]
    let mut regs = [0usize; 12];
    dump_regs(&mut regs);
    std::hint::black_box(&regs);
    collect_inner(&regs as *const _ as usize);
    std::hint::black_box(&regs);
}

#[inline(never)]
fn collect_inner(_regs_addr: usize) {
    let h = heap();
    if h.disabled { return; }
    let t0 = std::time::Instant::now();
    let mut m = Marker { stack: Vec::with_capacity(4096) };
    let here = 0usize;
    let lo = &here as *const usize as usize;
    m.mark_range(lo, crate::fiber::current_stack_top());
    for &(a, b) in h.extra_stacks.clone().iter() { m.mark_range(a, b); }
    crate::vm::mark_roots(&mut m);
    m.drain();
    sweep();
    let h = heap();
    h.gc_count += 1;
    h.allocs_since_gc = 0;
    h.malloc_since_gc = 0;
    h.malloc_limit = (h.live_after_gc * 256).max(64 << 20);
    h.gc_time += t0.elapsed();
}

fn sweep() {
    let h = heap();
    let mut free: *mut RObj = std::ptr::null_mut();
    let mut nfree = 0;
    let mut live = 0;
    let vm = crate::vm::vm();
    for &base in h.pages.iter() {
        for i in (0..PER_PAGE).rev() {
            let p = (base + i * SLOT) as *mut RObj;
            unsafe {
                let f = (*p).flags;
                if f & FL_FREE != 0 {
                    (*p).body = Body::Free(free);
                    free = p; nfree += 1;
                } else if f & FL_MARK != 0 {
                    (*p).flags = f & !FL_MARK;
                    live += 1;
                } else {
                    if f & FL_EXIVAR != 0 { vm.gen_ivars.remove(&(p as usize)); }
                    if let Body::Data(_) = &(*p).body { crate::vm::on_free_data(p); }
                    std::ptr::drop_in_place(&mut (*p).body);
                    std::ptr::write(&mut (*p).body, Body::Free(free));
                    (*p).flags = FL_FREE;
                    (*p).klass = NIL;
                    free = p; nfree += 1;
                    h.total_freed += 1;
                }
            }
        }
    }
    h.free = free;
    h.free_count = nfree;
    h.live_after_gc = live;
}

/// Iterate live objects (ObjectSpace.each_object).
pub fn each_object(mut f: impl FnMut(Value)) {
    let h = heap();
    for &base in h.pages.clone().iter() {
        for i in 0..PER_PAGE {
            let p = (base + i * SLOT) as *mut RObj;
            unsafe { if (*p).flags & FL_FREE == 0 { f(Value::from_ptr(p)); } }
        }
    }
}
