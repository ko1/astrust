// The machine and OS interface of fibers; fiber.rs is safe code on top of it.
//   NativeStack  an mmap'd machine stack with a guard page, recycled through a small pool
//   switch       save the callee-saved registers, swap rsp (x86-64 System V)
//   FiberCtx     a fiber's own Ctx, handed to its entry function on its own stack
//   nil_slots    value-stack memory that reads as nil, backed lazily by a copy-on-write file
//                mapping, so a stack costs only the pages it touches
use crate::slots::Slot;
use crate::value::{Value, NIL};
use crate::vm::Ctx;
use std::cell::RefCell;
use std::ptr::NonNull;

const PAGE: usize = 4096;
/// Usable bytes of a fiber's native stack (one PROT_NONE page below them).
pub const NATIVE_BYTES: usize = 16 << 20;
/// Released native stacks kept for reuse (virtual memory only, beyond WARM); the rest are unmapped.
const POOL_MAX: usize = 16384;
/// Bytes at the hot end of a released stack (native or value) whose pages are kept resident.
const WARM: usize = 64 << 10;

const PROT_NONE: i32 = 0;
const PROT_RW: i32 = 3;
const MAP_PRIVATE: i32 = 0x02;
const MAP_FIXED: i32 = 0x10;
const MAP_ANON: i32 = 0x20;
const MAP_NORESERVE: i32 = 0x4000;
const MADV_DONTNEED: i32 = 4;
const MFD_CLOEXEC: u32 = 1;
const F_DUPFD_CLOEXEC: i32 = 1030;
const FD_MIN: i32 = 1000;

extern "C" {
    fn mmap(a: *mut u8, l: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut u8;
    fn munmap(a: *mut u8, l: usize) -> i32;
    fn mprotect(a: *mut u8, l: usize, p: i32) -> i32;
    fn madvise(a: *mut u8, l: usize, adv: i32) -> i32;
    fn memfd_create(name: *const u8, flags: u32) -> i32;
}

fn map_anon(len: usize, prot: i32) -> Option<usize> {
    // SAFETY: a new mapping at an address the kernel picks; no existing memory is affected
    let p = unsafe { mmap(std::ptr::null_mut(), len, prot, MAP_PRIVATE | MAP_ANON | MAP_NORESERVE, -1, 0) } as usize;
    if p == usize::MAX { None } else { Some(p) }
}

/// An address in the running frame: where the CPU's stack is now.
#[inline(always)]
fn here() -> usize { let probe = 0u8; std::hint::black_box(&probe) as *const u8 as usize }

// ---- native stacks --------------------------------------------------------------------------

thread_local! { static POOL: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) }; }

/// A fiber's machine stack: [lo + PAGE, lo + PAGE + NATIVE_BYTES), the page at lo is a guard.
pub struct NativeStack { lo: usize }

impl NativeStack {
    /// A stack from the pool, or a new mapping (`fresh`).  None when the mapping fails.
    pub fn take() -> Option<(NativeStack, bool)> {
        if let Some(lo) = POOL.with(|p| p.borrow_mut().pop()) { return Some((NativeStack { lo }, false)); }
        let lo = map_anon(PAGE + NATIVE_BYTES, PROT_RW)?;
        // SAFETY: the lowest page of the mapping just made, unused
        if unsafe { mprotect(lo as *mut u8, PAGE, PROT_NONE) } != 0 { return None; }
        Some((NativeStack { lo }, true))
    }
    pub fn bottom(&self) -> usize { self.lo + PAGE }
    pub fn top(&self) -> usize { self.lo + PAGE + NATIVE_BYTES }
    fn holds(&self, a: usize) -> bool { a >= self.lo && a < self.top() }

    /// Lay out a first switch into this stack that runs `fiber::fiber_main(ctx)` on it; the stack
    /// pointer to switch to.  Refused (None) while the CPU is on this stack.
    pub fn prepare(&mut self, ctx: &mut FiberCtx) -> Option<usize> {
        if self.holds(here()) { return None; }
        let sp = (self.top() & !15) - 8 * 8;
        // popped by switch: r15 r14 r13 r12 rbx rbp, then the return address
        let frame: [usize; 8] = [0, 0, 0, ctx.0.as_ptr() as usize, 0, 0, fiber_start as *const () as usize, 0];
        // SAFETY: [sp, sp + 64) is the top of this mapped stack, which no running code uses (checked above)
        unsafe { std::ptr::copy_nonoverlapping(frame.as_ptr(), sp as *mut usize, 8); }
        Some(sp)
    }

    /// The words of a switched-out stack from its saved sp to its top (a GC root).
    pub fn words(&self, sp: usize) -> &[usize] { stack_words(sp, self.bottom(), self.top()) }
}

impl Drop for NativeStack {
    fn drop(&mut self) {
        // the running stack is never released: leak it (cannot happen when fiber.rs is right)
        if self.holds(here()) { return; }
        let lo = self.lo;
        let kept = POOL.with(|p| { let mut p = p.borrow_mut(); if p.len() < POOL_MAX { p.push(lo); true } else { false } });
        if kept {
            // SAFETY: pages of this unused stack below its warm top; they read as zero afterwards
            unsafe { madvise(self.bottom() as *mut u8, NATIVE_BYTES - WARM, MADV_DONTNEED); }
        } else {
            // SAFETY: the whole mapping made by take(); nothing runs on it and nothing refers into it
            unsafe { munmap(lo as *mut u8, PAGE + NATIVE_BYTES); }
        }
    }
}

/// The words of the main thread's stack from `sp` up to `top` while it is switched out.
pub fn main_stack_words(sp: usize, top: usize, size: usize) -> &'static [usize] { stack_words(sp, top.saturating_sub(size), top) }

// [sp, top) clamped into a stack [bottom, top) that stays mapped: a native stack while its
// NativeStack lives (the borrow), the main thread's for the whole process
fn stack_words<'a>(sp: usize, bottom: usize, top: usize) -> &'a [usize] {
    // SAFETY: the range is clamped into a mapped stack (callers pass its bounds)
    unsafe { crate::astrust_gc::stack::words(sp.clamp(bottom, top), top) }
}

// ---- the switch -------------------------------------------------------------------------------

/// What a switch passes: a Value word and a flag word (rax:rdx on return).
#[repr(C)]
struct Passed(u64, u64);

// rdx, rcx: the words passed; the other side receives them as the return value (or fiber_start's)
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
unsafe extern "C" fn switch_stack(_save: *mut usize, _to: usize, _v: u64, _flag: u64) -> Passed {
    std::arch::naked_asm!(
        "push rbp", "push rbx", "push r12", "push r13", "push r14", "push r15",
        "mov [rdi], rsp",
        "mov rsp, rsi",
        "pop r15", "pop r14", "pop r13", "pop r12", "pop rbx", "pop rbp",
        "mov rax, rdx", "mov rdx, rcx",
        "ret",
    );
}

// first return into a prepared stack: r12 = ctx, rax:rdx = what the switch passed
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
unsafe extern "C" fn fiber_start() {
    std::arch::naked_asm!("mov rdi, r12", "mov rsi, rax", "and rsp, -16", "call {t}", "ud2", t = sym trampoline);
}

extern "C" fn trampoline(ctx: *mut Ctx, v: u64, flag: u64) -> ! {
    // SAFETY: ctx is the FiberCtx this stack was prepared with, its Fiber the current one (a root)
    // while it runs; v is the Value word the switch into this stack was given (see switch)
    let (c, v) = unsafe { (&mut *ctx, Value::from_raw(v)) };
    crate::fiber::fiber_main(c, v, flag != 0)
}

/// Suspend the running code, saving its stack pointer to `*save`, and continue the code
/// suspended at `to`, handing it (v, flag).  Returns what the switch back to `*save` hands over.
///
/// # Safety
/// `to` was saved by `switch` (or made by `NativeStack::prepare`) on a stack still mapped, and is
/// used once; `save` is writable; no reference into anything the other side may touch is held
/// across the call.
#[inline(always)]
pub unsafe fn switch(save: *mut usize, to: usize, v: Value, flag: bool) -> (Value, bool) {
    // SAFETY: the caller's contract; the words handed back are a Value and a flag passed to switch
    unsafe {
        let r = switch_stack(save, to, v.raw(), flag as u64);
        (Value::from_raw(r.0), r.1 != 0)
    }
}

// ---- a fiber's context ------------------------------------------------------------------------

/// The Ctx a fiber runs with, owned by its Fiber.
pub struct FiberCtx(NonNull<Ctx>);

impl FiberCtx {
    pub fn new(native_limit: usize) -> FiberCtx {
        use crate::vm::EscKind;
        let c = Box::new(Ctx {
            sp: crate::stack::NO_SP, stk: crate::stack::NO_STACK, frame: std::ptr::null_mut(),
            esc: EscKind::None, esc_val: NIL, esc_tag: 0, esc_tag_v: NIL, serial: 1 << 40, errinfo: NIL, last_match: NIL,
            native_limit, in_ensure: 0, cblk_kw: false, redef: crate::rt::redef_word(), call_me: std::ptr::null(), call_blk: std::ptr::null(),
            // SAFETY: a fiber's context runs only on its stack, while the one that switched to it is suspended in switch
            tok: unsafe { crate::heap::Token::mint() },
        });
        FiberCtx(NonNull::from(Box::leak(c)))
    }
    /// Which context this is (compared with gc's allocating one).
    pub fn ptr(&self) -> *mut Ctx { self.0.as_ptr() }
    /// The context, for its owner while the fiber does not run, and for the collector.
    pub fn get(&mut self) -> &mut Ctx {
        // SAFETY: the fiber's own code holds the other reference, and is suspended (or is the caller) meanwhile, as with main_ctx
        unsafe { self.0.as_mut() }
    }
}

impl Drop for FiberCtx {
    fn drop(&mut self) {
        // SAFETY: made by Box::leak in new(), dropped once; the fiber no longer runs
        drop(unsafe { Box::from_raw(self.0.as_ptr()) });
    }
}

// ---- value stacks that read as nil ----------------------------------------------------------

/// Bytes of the nil file; a stack maps it as many times as its size needs.
const CHUNK: usize = 1 << 20;   // written at startup: 1 MiB balances that against one mmap per MiB of stack

/// The nil file's descriptor (-2: not tried, -1: failed).
static NILFD: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-2);

thread_local! {
    static MAPPED: RefCell<std::collections::HashSet<usize>> = RefCell::new(Default::default());
}

/// Make the nil file now, before a second thread exists: growing the descriptor table to
/// FD_MIN in a multi-threaded process waits for an RCU grace period (5-8 ms at startup).
pub fn nil_fd_init() { let _ = nil_fd(); }

fn nil_fd() -> Option<i32> {
    use std::sync::atomic::Ordering::Relaxed;
    let fd = NILFD.load(Relaxed);
    if fd != -2 { return if fd >= 0 { Some(fd) } else { None }; }
    // SAFETY: a NUL-terminated name; creates a new file descriptor
    let fd = unsafe { memfd_create(c"saruby-nil".as_ptr() as *const u8, MFD_CLOEXEC) };
    // out of the way of the descriptors a program sees (they match CRuby's)
    let fd = match crate::builtins::sys::fcntl(fd, F_DUPFD_CLOEXEC, FD_MIN) { -1 => fd, hi => { crate::builtins::sys::close(fd); hi } };
    let ok = fd >= 0 && crate::builtins::sys::ftruncate(fd, CHUNK as i64) == 0 && {
        let buf = NIL.raw().to_ne_bytes().repeat(8192);
        (0..CHUNK / buf.len()).all(|i| crate::builtins::sys::pwrite(fd, &buf, (i * buf.len()) as i64) == buf.len() as isize)
    };
    if !ok && fd >= 0 { crate::builtins::sys::close(fd); }
    NILFD.store(if ok { fd } else { -1 }, Relaxed);
    if ok { Some(fd) } else { None }
}

/// n slots of nil whose pages are private copies only once written.  Never unmapped.
pub fn nil_slots(n: usize) -> Option<&'static [Slot]> {
    let fd = nil_fd()?;
    let len = (n * std::mem::size_of::<Slot>()).div_ceil(CHUNK) * CHUNK;
    let base = if len == CHUNK {
        // SAFETY: a new private view of the nil file at an address the kernel picks
        let p = unsafe { mmap(std::ptr::null_mut(), CHUNK, PROT_RW, MAP_PRIVATE, fd, 0) } as usize;
        if p == usize::MAX { return None; }
        p
    } else { map_anon(len, PROT_NONE)? };
    for off in (0..len).step_by(CHUNK).filter(|_| len > CHUNK) {
        // SAFETY: replaces part of the reservation just made with a private view of the nil file
        let p = unsafe { mmap((base + off) as *mut u8, CHUNK, PROT_RW, MAP_PRIVATE | MAP_FIXED, fd, 0) } as usize;
        if p != base + off {
            // SAFETY: the reservation made above, not handed out
            unsafe { munmap(base as *mut u8, len); }
            return None;
        }
    }
    MAPPED.with(|m| m.borrow_mut().insert(base));
    // SAFETY: n slots of mapped memory that reads as NIL (a valid Value) until written; never unmapped, so 'static
    Some(unsafe { std::slice::from_raw_parts(base as *const Slot, n) })
}

/// Give back the written pages of a released stack made by nil_slots, but its first WARM bytes:
/// they read as nil again.  False if `s` was not made by nil_slots.
pub fn nil_slots_reset(s: &'static [Slot]) -> bool {
    let base = s.as_ptr() as usize;
    if !MAPPED.with(|m| m.borrow().contains(&base)) { return false; }
    let len = std::mem::size_of_val(s).next_multiple_of(PAGE);
    if len <= WARM { return true; }
    // SAFETY: a private file mapping of ours: its pages read as the nil file again, Slots are Cells
    unsafe { madvise((base + WARM) as *mut u8, len - WARM, MADV_DONTNEED); }
    true
}

/// Slots cleared by stores above `from` before the pages beyond are given back (docs/gc_moving.md 12.2).
const NEAR: usize = WARM / std::mem::size_of::<Slot>();

/// Make `s[from..]` read as nil: the first NEAR slots by storing `fill` (nil, or gc_check's poison), the whole
/// pages after them, in a nil_slots mapping, by giving them back.
pub fn slots_clear(s: &'static [Slot], from: usize, fill: Value) {
    let from = from.min(s.len());
    let near = s.len().min(from + NEAR);
    for x in &s[from..near] { x.set(fill); }
    let rest = &s[near..];
    if rest.is_empty() { return; }
    let r0 = rest.as_ptr() as usize;
    let (lo, hi) = (r0.next_multiple_of(PAGE), rest.as_ptr_range().end as usize & !(PAGE - 1));
    // a stack made by nil_slots starts in the mapping's first page (slots_at)
    if hi <= lo || !MAPPED.with(|m| m.borrow().contains(&(s.as_ptr() as usize & !(PAGE - 1)))) {
        for x in rest { x.set(NIL); }
        return;
    }
    let sz = std::mem::size_of::<Slot>();
    for x in &rest[..(lo - r0) / sz] { x.set(NIL); }
    for x in &rest[(hi - r0) / sz..] { x.set(NIL); }
    // SAFETY: whole pages of a private view of the nil file (MAPPED): they read as nil again; Slots are Cells
    unsafe { madvise(lo as *mut u8, hi - lo, MADV_DONTNEED); }
}
