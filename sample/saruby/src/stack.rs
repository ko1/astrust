// The VM value stack, checked backend (the default): no unsafe.
// Design, measurements and the comparison with stack_raw.rs: docs/safe_stack.md.
//
// Every access is bounds-checked against the slice or array it goes through, so nothing here
// (or in the code using it) can touch memory outside a stack whatever the sp register holds.
#![forbid(unsafe_code)]
pub use crate::slots::*;
use crate::value::*;
use crate::vm::{raise_stack_overflow, Ctx, Esc};

/// A frame as evaluators see it: fp[0] = self, fp[1] = env, then parameters and locals.
/// A reference to an array: one word, and an index that is a constant or a u16 needs no check.
pub type Fp = &'static [Slot; FRAME];
/// A position in a stack: the slots from there to its end (staged calls, frame bases).
pub type Stk = &'static [Slot];
/// A frame's self and env slots: what a Frame record keeps.
pub type FpHead = &'static [Slot; 2];
/// An allocated stack.
pub type Stack = Stk;
/// Ctx.sp: the number of free slots (sp is that far below the end of the stack).
pub type Sp = usize;
/// Ctx.stk: the whole stack.
pub type StackRef = Stk;
pub const NO_SP: Sp = 0;
pub const NO_STACK: StackRef = &[];

pub fn stack_new(n: usize) -> Stack { slots_new(n) }
pub fn stack_release(s: Stack) { slots_release(s) }

#[cold]
#[inline(never)]
fn no_head() -> ! { panic!("frame without self/env slots") }

#[inline(always)]
pub fn head(st: Stk) -> FpHead {
    match st.first_chunk::<2>() { Some(h) => h, None => no_head() }
}

/// Local slot i of a frame.  No check: FRAME is 2^16.
#[inline(always)]
pub fn slot(fp: Fp, i: u32) -> &'static Slot { &fp[i as u16 as usize] }

/// The frame at `st`.  A stack's last FRAME slots cannot start one: that is the overflow check.
#[inline(always)]
pub fn frame(c: &mut Ctx, st: Stk) -> Result<Fp, Esc> {
    match st.first_chunk::<FRAME>() { Some(f) => Ok(f), None => Err(raise_stack_overflow(c)) }
}

impl Ctx {
    pub fn attach_stack(&mut self, s: Stack) { self.stk = s; self.sp = s.len(); }

    /// The stack from its first slot.
    pub fn bottom(&self) -> Stk { self.stk }

    /// The free part of the stack: from sp to its end.
    #[inline(always)]
    pub fn top(&self) -> Stk { let s = self.stk; &s[s.len() - self.sp..] }

    /// The used part of the stack (the GC's roots).
    pub fn live(&self) -> &'static [Slot] { let s = self.stk; &s[..s.len() - self.sp] }

    /// Move sp to n slots above the base of `st`.
    #[inline(always)]
    pub fn set_sp(&mut self, st: Stk, n: usize) { self.sp = st.len().saturating_sub(n); }

    /// Are n slots (and a frame) free above sp?
    #[inline(always)]
    pub fn has_room(&self, n: usize) -> bool { self.sp >= n + FRAME }

    #[inline(always)]
    pub fn sp_mark(&self) -> Sp { self.sp }
    #[inline(always)]
    pub fn sp_reset(&mut self, m: Sp) { self.sp = m; }
}

/// Reserve n slots at sp, for a call being staged ([recv, -, args...]) or for temporaries.
/// The result runs from the old sp to the end of the stack and can hold the callee's frame.
#[inline(always)]
pub fn stage(c: &mut Ctx, n: usize) -> Result<Stk, Esc> {
    let (s, room) = (c.stk, c.sp);
    if room < n + FRAME || room > s.len() { return Err(raise_stack_overflow(c)); }
    c.sp = room - n;
    Ok(&s[s.len() - room..])
}

/// Before running a method entry node on the frame at `st`: it expects sp just above the
/// argc staged arguments, where stage() left it (so a baked call that just staged skips this).
#[inline(always)]
pub fn at_entry(c: &mut Ctx, st: Stk, argc: usize) { c.sp = st.len().wrapping_sub(2 + argc); }

/// Method entry: sp goes from above the params to above the frame's n slots (frame() made sure they exist).
#[inline(always)]
pub fn enter(c: &mut Ctx, _fp: Fp, n: u32, nparams: u32) -> Result<(), Esc> {
    c.sp = c.sp.wrapping_sub(n.saturating_sub(2 + nparams) as usize);
    Ok(())
}

/// Block entry: sp goes from the frame's base to above its n slots (the caller checked the room).
#[inline(always)]
pub fn enter_base(c: &mut Ctx, _fp: Fp, n: u32) -> Result<(), Esc> {
    c.sp = c.sp.wrapping_sub(n as usize);
    Ok(())
}

/// st[at..] = vals.
#[inline(always)]
pub fn put(st: Stk, at: usize, vals: &[Value]) {
    for (s, v) in st[at..at + vals.len()].iter().zip(vals) { s.set(*v); }
}

/// st[from..to] = nil.  A plain loop: the range is short, often empty.
#[inline(always)]
pub fn nil_fill(st: Stk, from: usize, to: usize) {
    let mut i = from;
    while i < to { st[i].set(NIL); i += 1; }
}

/// The arguments staged at st[2..2 + argc] as the `&[Value]` a cfunc takes.  A copy: a slot
/// is a Cell, so no `&[Value]` can point into the stack.  (The staged originals stay below sp.)
#[inline(always)]
pub fn with_args<R>(st: Stk, argc: usize, f: impl FnOnce(&[Value]) -> R) -> R {
    // fixed-size copies: slots past argc hold stale but valid Values and are not exposed
    fn copy<const N: usize>(st: Stk) -> Option<[Value; N]> {
        let s = st.get(2..2 + N)?;
        Some(std::array::from_fn(|i| s[i].get()))
    }
    if argc == 0 { return f(&[]); }
    if argc <= 2 { if let Some(a) = copy::<2>(st) { return f(&a[..argc]); } }
    if argc <= 6 { if let Some(a) = copy::<6>(st) { return f(&a[..argc]); } }
    f(&to_vec(&st[2..2 + argc]))
}
