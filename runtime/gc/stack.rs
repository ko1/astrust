// Ambiguous roots from machine stacks: the words of a stack range, and the running
// stack together with the callee-saved registers.  A binding that keeps references
// in native locals passes these to `RootVisitor::visit_ambiguous`.

/// The aligned words of `[lo, hi)`.
///
/// # Safety
/// The range is readable for `'a` (a mapped stack, running or suspended).
#[inline]
pub unsafe fn words<'a>(lo: usize, hi: usize) -> &'a [usize] {
    let a = lo & !7;
    if hi < a + 8 { return &[]; }
    // SAFETY: the caller vouches for the range; stack words are plain integers to us
    unsafe { std::slice::from_raw_parts(a as *const usize, (hi - a) / 8) }
}

#[cfg(target_arch = "x86_64")]
const NREGS: usize = 8;
#[cfg(target_arch = "aarch64")]
const NREGS: usize = 12;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("runtime/gc/stack.rs: no callee-saved register dump for this target");

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn dump_regs(r: &mut [usize; NREGS]) {
    // SAFETY: stores seven registers into the eight-word array `r`
    unsafe {
        std::arch::asm!(
            "mov [{0}], rbx", "mov [{0} + 8], rbp", "mov [{0} + 16], r12", "mov [{0} + 24], r13",
            "mov [{0} + 32], r14", "mov [{0} + 40], r15", "mov [{0} + 48], rsp",
            in(reg) r.as_mut_ptr(), options(nostack));
    }
}
#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn dump_regs(r: &mut [usize; NREGS]) {
    // SAFETY: stores twelve registers into the twelve-word array `r`
    unsafe {
        std::arch::asm!(
            "stp x19, x20, [{0}]", "stp x21, x22, [{0}, #16]", "stp x23, x24, [{0}, #32]",
            "stp x25, x26, [{0}, #48]", "stp x27, x28, [{0}, #64]", "stp x29, x30, [{0}, #80]",
            in(reg) r.as_mut_ptr(), options(nostack));
    }
}

/// Call `f` with the words of the running stack from this frame up to `top`,
/// callee-saved registers included (they are spilled into this frame first).
///
/// # Safety
/// `top` is the end (highest address) of the stack the caller is running on.
#[inline(never)]
pub unsafe fn with_current_stack<R>(top: usize, f: impl FnOnce(&[usize]) -> R) -> R {
    let mut regs = [0usize; NREGS];
    dump_regs(&mut regs);
    std::hint::black_box(&regs);
    // SAFETY: `regs` is in the running frame, so [regs, top) is the live part of this stack
    let r = f(unsafe { words(regs.as_ptr() as usize, top) });
    std::hint::black_box(&regs);
    r
}
