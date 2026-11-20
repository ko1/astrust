// Global allocator: per-size-class free lists for small blocks allocated on the
// interpreter thread (String / Array / ivar / Hash buffers churn through malloc
// otherwise).  Everything else (other threads, large or over-aligned blocks) goes
// to the system allocator.  Arena memory is recycled per class, never unmapped.
use std::alloc::{GlobalAlloc, Layout, System};

const MAX: usize = 1024;              // largest arena block
const NCLASS: usize = MAX / 16 + 1;   // class c holds 16*c-byte blocks
const ARENA: usize = 1 << 36;         // reserved address range
const RUN: usize = 32 * 1024;         // carved from the arena per refill

static mut OWNER: usize = 0;          // thread pointer of the interpreter thread
static mut BASE: usize = 0;
static mut LEN: usize = 0;
static mut BUMP: usize = 0;
static mut FREE: [*mut u8; NCLASS] = [std::ptr::null_mut(); NCLASS];

pub struct KAlloc;

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn thread_ptr() -> usize {
    let t: usize;
    unsafe { std::arch::asm!("mov {}, fs:[0]", out(reg) t, options(nostack, readonly, preserves_flags)); }
    t
}
#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn thread_ptr() -> usize {
    let t: usize;
    unsafe { std::arch::asm!("mrs {}, tpidr_el0", out(reg) t, options(nostack, nomem, preserves_flags)); }
    t
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline(always)]
fn thread_ptr() -> usize { 1 }

extern "C" { fn mmap(a: *mut u8, l: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut u8; }

/// Make the calling thread the arena's owner (call once, on the interpreter thread).
pub fn adopt() {
    if std::env::var_os("KORUBY_NO_KALLOC").is_some() { return; }
    // PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS|MAP_NORESERVE
    let mut len = ARENA;
    let p = loop {
        let p = unsafe { mmap(std::ptr::null_mut(), len, 3, 0x02 | 0x20 | 0x4000, -1, 0) } as usize;
        if p != usize::MAX && p != 0 { break p; }
        len >>= 2;   // restricted address space (ulimit -v, valgrind): settle for less
        if len < 1 << 26 { return; }
    };
    unsafe { BASE = p; BUMP = p; LEN = len; OWNER = thread_ptr(); }
}

#[inline(always)]
unsafe fn free_slot(c: usize) -> *mut *mut u8 { (&raw mut FREE as *mut *mut u8).add(c) }

#[inline(never)]
#[cold]
unsafe fn refill(c: usize, l: Layout) -> *mut u8 {
    let sz = c * 16;
    let n = RUN / sz;
    if BUMP + n * sz > BASE + LEN { return System.alloc(l); }
    let run = BUMP;
    BUMP += n * sz;
    // block 0 is returned; the rest are chained
    let mut next: *mut u8 = std::ptr::null_mut();
    for i in (1..n).rev() {
        let p = (run + i * sz) as *mut u8;
        *(p as *mut *mut u8) = next;
        next = p;
    }
    *free_slot(c) = next;
    run as *mut u8
}

#[inline(always)]
fn small(l: Layout) -> bool { l.size() <= MAX && l.align() <= 16 }
#[inline(always)]
unsafe fn in_arena(p: *mut u8) -> bool { (p as usize).wrapping_sub(BASE) < LEN }

unsafe impl GlobalAlloc for KAlloc {
    #[inline]
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if small(l) && thread_ptr() == OWNER {
            let c = (l.size() + 15) >> 4;
            let s = free_slot(c);
            let p = *s;
            if !p.is_null() { *s = *(p as *mut *mut u8); return p; }
            return refill(c, l);
        }
        System.alloc(l)
    }
    #[inline]
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if in_arena(p) {
            // a block freed by another thread is leaked (only happens at exit)
            if thread_ptr() == OWNER {
                let s = free_slot((l.size() + 15) >> 4);
                *(p as *mut *mut u8) = *s;
                *s = p;
            }
            return;
        }
        System.dealloc(p, l)
    }
    #[inline]
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if small(l) && thread_ptr() == OWNER {
            let p = self.alloc(l);
            if !p.is_null() { std::ptr::write_bytes(p, 0, l.size()); }
            return p;
        }
        System.alloc_zeroed(l)
    }
    #[inline]
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new_size: usize) -> *mut u8 {
        if in_arena(p) {
            if (new_size + 15) >> 4 == (l.size() + 15) >> 4 { return p; }
        } else if new_size > MAX {
            return System.realloc(p, l, new_size);
        }
        let nl = Layout::from_size_align_unchecked(new_size, l.align());
        let q = self.alloc(nl);
        if !q.is_null() {
            std::ptr::copy_nonoverlapping(p, q, l.size().min(new_size));
            self.dealloc(p, l);
        }
        q
    }
}

#[global_allocator]
static GLOBAL: KAlloc = KAlloc;
