// Fibers (to be implemented): GC hook for suspended fiber stacks.
pub fn mark_fibers(_m: &mut crate::gc::Marker) {}
use crate::value::*;
use crate::vm::*;
/// External enumeration (Enumerator#next) through a fiber; None = not available (buffer instead).
pub fn enum_next(_c: &mut Ctx, _e: Value) -> Result<Option<Value>, Esc> { Ok(None) }
pub fn enum_has_fiber(_e: Value) -> bool { false }
pub fn enum_peek(_c: &mut Ctx, _e: Value) -> RV { Ok(NIL) }
