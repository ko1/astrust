// Value stack memory, shared by the two stack backends (stack.rs, stack_raw.rs).
#![forbid(unsafe_code)]
use crate::value::*;
use std::cell::{Cell, RefCell};

/// A stack slot.  A Cell: slots are shared by nature (a caller's staged arguments are its
/// callee's parameters, and the GC reads every live slot).
pub type Slot = Cell<Value>;

/// The most slots a frame may have (compile.rs keeps to it).  2^16, so a u16 index is in range.
pub const FRAME: usize = 1 << 16;

// released stacks, with the collection count at their release
thread_local! { static POOL: RefCell<Vec<(&'static [Slot], usize)>> = const { RefCell::new(Vec::new()) }; }
thread_local! { static EPOCH: Cell<usize> = const { Cell::new(0) }; }

/// A collection ran: stacks released before it may hold Values it freed.
pub fn slots_epoch_bump() { EPOCH.set(EPOCH.get().wrapping_add(1)); }

/// Stacks of at least this many slots are mapped lazily (fiber_os::nil_slots): only touched pages cost memory.
const LAZY: usize = FRAME;

/// Memory for a stack of n slots.  Never freed, only recycled: that is what makes it 'static.
/// A recycled one is nil again if a collection ran since its release (its stale Values may be freed: the
/// exact value-stack scan relies on that, docs/gc_moving.md 12.2); else it comes back as it was left.
pub fn slots_new(n: usize) -> &'static [Slot] {
    let old = POOL.with(|p| {
        let mut p = p.borrow_mut();
        p.iter().rposition(|s| s.0.len() == n).map(|i| p.swap_remove(i))
    });
    let old = old.map(|(s, e)| { if e != EPOCH.get() { crate::fiber_os::slots_clear(s, 0, NIL); } s });
    old.or_else(|| if n >= LAZY { crate::fiber_os::nil_slots(n) } else { None })
        .unwrap_or_else(|| Box::leak(vec![Cell::new(NIL); n].into_boxed_slice()))
}

/// `s` from its first slot at the page offset of `at`.
pub fn slots_at(s: &'static [Slot], at: usize) -> &'static [Slot] {
    &s[(at.wrapping_sub(s.as_ptr() as usize) & 4095) / std::mem::size_of::<Slot>()..]
}

pub fn slots_release(s: &'static [Slot]) {
    if s.is_empty() { return; }
    #[cfg(feature = "gc_check")] crate::gc::parks_forget(s.as_ptr() as usize);
    POOL.with(|p| {
        let mut p = p.borrow_mut();
        if s.len() >= LAZY { crate::fiber_os::nil_slots_reset(s); }
        p.push((s, EPOCH.get()))
    });
}

#[inline(always)]
pub fn to_vec(st: &[Slot]) -> Vec<Value> {
    let mut v = Vec::with_capacity(st.len());
    v.extend(st.iter().map(Cell::get));
    v
}
