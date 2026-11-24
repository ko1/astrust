// The VM value stack, unchecked backend (the default): raw pointers.
// Same API as stack.rs, which checks everything; docs/safe_stack.md compares the two.
//
// All the unsafe of the value stack is in this file.  Its functions are safe to call only
// under these rules, which the checked backend enforces (so: run the tests with it):
//   R1  an index into a frame is below its iseq's nlocals; into a staged area, below the
//       count given to stage(), or covered by has_room();
//   R2  a Stk / Fp is used only while its frame or staged call is live;
//   R3  sp is at or above every live slot (stage / set_sp / enter keep it so): staging never
//       overwrites one and live() covers them all;
//   R4  nothing writes the slots with_args lends to a cfunc while it runs (they are below sp).
pub use crate::slots::*;
use crate::value::*;
use crate::vm::{raise_stack_overflow, Ctx, Esc};
use std::ops::{Index, Range};

/// A position in a stack (a frame base, a staged call).
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Stk(*const Slot);
pub type Fp = Stk;
pub type FpHead = Stk;
/// An allocated stack.
#[derive(Clone, Copy)]
pub struct Stack(&'static [Slot]);
/// Ctx.sp: the first free slot.
pub type Sp = *const Slot;
/// Ctx.stk: the stack's bounds; `end` is SLACK slots below the real end.
#[derive(Clone, Copy)]
pub struct StackRef { end: *const Slot, base: *const Slot }
pub const NO_SP: Sp = std::ptr::null();
pub const NO_STACK: StackRef = StackRef { end: std::ptr::null(), base: std::ptr::null() };

/// Staging is not checked; frame entries are, and leave this much room for it.
const SLACK: usize = 64;

impl Index<usize> for Stk {
    type Output = Slot;
    #[inline(always)]
    fn index(&self, i: usize) -> &Slot { unsafe { &*self.0.add(i) } }
}
impl Index<Range<usize>> for Stk {
    type Output = [Slot];
    #[inline(always)]
    fn index(&self, r: Range<usize>) -> &[Slot] {
        unsafe { std::slice::from_raw_parts(self.0.add(r.start), r.end.saturating_sub(r.start)) }
    }
}

pub fn stack_new(n: usize) -> Stack { Stack(slots_new(n)) }
pub fn stack_new_at(n: usize, at: usize) -> Stack { Stack(slots_at(slots_new(n), at)) }
pub fn stack_release(s: Stack) { slots_release(s.0) }

#[inline(always)]
pub fn head(st: Stk) -> FpHead { st }

#[inline(always)]
pub fn slot(fp: Fp, i: u32) -> &'static Slot { unsafe { &*fp.0.add(i as usize) } }

#[inline(always)]
pub fn frame(_c: &mut Ctx, st: Stk) -> Result<Fp, Esc> { Ok(st) }

impl Ctx {
    pub fn attach_stack(&mut self, s: Stack) {
        let base = s.0.as_ptr();
        self.sp = base;
        self.stk = StackRef { end: base.wrapping_add(s.0.len().saturating_sub(SLACK)), base };
    }
    pub fn bottom(&self) -> Stk { Stk(self.stk.base) }
    #[inline(always)]
    pub fn top(&self) -> Stk { Stk(self.sp) }
    pub fn live(&self) -> &'static [Slot] {
        if self.stk.base.is_null() { return &[]; }
        unsafe { std::slice::from_raw_parts(self.stk.base, self.sp.offset_from(self.stk.base) as usize) }
    }
    #[inline(always)]
    pub fn set_sp(&mut self, st: Stk, n: usize) { self.sp = st.0.wrapping_add(n); }
    #[inline(always)]
    pub fn has_room(&self, n: usize) -> bool { self.sp.wrapping_add(n) < self.stk.end }
    #[inline(always)]
    pub fn sp_mark(&self) -> Sp { self.sp }
    #[inline(always)]
    pub fn sp_reset(&mut self, m: Sp) { self.sp = m; }
}

#[inline(always)]
pub fn stage(c: &mut Ctx, n: usize) -> Result<Stk, Esc> {
    let b = c.sp;
    c.sp = b.wrapping_add(n);
    Ok(Stk(b))
}

#[inline(always)]
pub fn at_entry(_c: &mut Ctx, _st: Stk, _argc: usize) {}

/// Method entry: the overflow check, and sp goes above the frame's n slots.
#[inline(always)]
pub fn enter(c: &mut Ctx, fp: Fp, n: u32, _nparams: u32) -> Result<(), Esc> {
    let top = fp.0.wrapping_add(n as usize);
    if top.wrapping_add(16) >= c.stk.end { return Err(raise_stack_overflow(c)); }
    c.sp = top;
    Ok(())
}

/// Block entry: the same, from the frame's base.
#[inline(always)]
pub fn enter_base(c: &mut Ctx, fp: Fp, n: u32) -> Result<(), Esc> { enter(c, fp, n, 0) }

#[inline(always)]
pub fn put(st: Stk, at: usize, vals: &[Value]) {
    for (i, v) in vals.iter().enumerate() { st[at + i].set(*v); }
}

#[inline(always)]
pub fn nil_fill(st: Stk, from: usize, to: usize) {
    let mut i = from;
    while i < to { st[i].set(NIL); i += 1; }
}

/// The staged arguments themselves (Cell<Value> has the layout of Value).
#[inline(always)]
pub fn with_args<R>(st: Stk, argc: usize, f: impl FnOnce(&[Value]) -> R) -> R {
    f(unsafe { std::slice::from_raw_parts(st.0.add(2) as *const Value, argc) })
}
