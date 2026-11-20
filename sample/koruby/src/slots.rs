// Value stack memory, shared by the two stack backends (stack.rs, stack_raw.rs).
#![forbid(unsafe_code)]
use crate::value::*;
use std::cell::{Cell, RefCell};

/// A stack slot.  A Cell: slots are shared by nature (a caller's staged arguments are its
/// callee's parameters, and the GC reads every live slot).
pub type Slot = Cell<Value>;

/// The most slots a frame may have (compile.rs keeps to it).  2^16, so a u16 index is in range.
pub const FRAME: usize = 1 << 16;

thread_local! { static POOL: RefCell<Vec<&'static [Slot]>> = const { RefCell::new(Vec::new()) }; }

/// Memory for a stack of n slots.  Never freed, only recycled: that is what makes it 'static.
/// A recycled one comes back as it was left; like any slot above sp, its contents are dead.
pub fn slots_new(n: usize) -> &'static [Slot] {
    let old = POOL.with(|p| {
        let mut p = p.borrow_mut();
        p.iter().position(|s| s.len() == n).map(|i| p.swap_remove(i))
    });
    old.unwrap_or_else(|| Box::leak(vec![Cell::new(NIL); n].into_boxed_slice()))
}

pub fn slots_release(s: &'static [Slot]) {
    if !s.is_empty() { POOL.with(|p| p.borrow_mut().push(s)); }
}

pub fn to_vec(st: &[Slot]) -> Vec<Value> { st.iter().map(Cell::get).collect() }
