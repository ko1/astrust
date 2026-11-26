// VM-state helpers without a payload (recursion guard) and boot-time class setup (docs/safe_heap.md).
use crate::heap::obj_set_klass;
#[allow(unused_imports)]
use crate::heap::no_legacy_body::*;   // no object accessor (docs/safe_heap.md)
use crate::value::*;
use crate::vm::Ctx;

/// Recursion guard of inspect / hash / == (VM state only, no payload).  The entries are Values the
/// collector visits (vm::visit_roots), so a moving collection rewrites them.
pub fn recursion_enter(v: Value, tag: crate::value::Sym) -> bool {
    let mut g = crate::vm::vm().recursive_guard.borrow_mut();
    if g.iter().any(|&(x, y, t)| x == v && y == UNDEF && t == tag) { return false; }
    g.push((v, UNDEF, tag));
    true
}
/// rb_exec_recursive_paired's guard: the pair (a, b) under `tag`.
pub fn recursion_enter_paired(a: Value, b: Value, tag: crate::value::Sym) -> bool {
    let mut g = crate::vm::vm().recursive_guard.borrow_mut();
    if g.iter().any(|&(x, y, t)| x == a && y == b && t == tag) { return false; }
    g.push((a, b, tag));
    true
}
pub fn recursion_leave_paired(a: Value, b: Value, tag: crate::value::Sym) {
    let mut g = crate::vm::vm().recursive_guard.borrow_mut();
    if let Some(p) = g.iter().rposition(|&(x, y, t)| x == a && y == b && t == tag) { g.remove(p); }
}
pub fn recursion_leave(v: Value, tag: crate::value::Sym) {
    let mut g = crate::vm::vm().recursive_guard.borrow_mut();
    if let Some(p) = g.iter().rposition(|&(x, y, t)| x == v && y == UNDEF && t == tag) { g.remove(p); }
}

// ---- boot-time class setup for builtins/boot.rs ----
/// `m` becomes the metaclass of `k`.
pub fn boot_metaclass(c: &mut Ctx, k: Value, m: Value) {
    let d = c.class_mut(m);
    d.is_singleton = true; d.attached = k;
    obj_set_klass(k, m);
}
pub fn boot_subclass(c: &mut Ctx, sup: Value, k: Value) { c.class_mut(sup).subclasses.push(k); }
/// Constants `names` of `owner` autoload the native `feature`.
pub fn boot_autoload(c: &mut Ctx, owner: Value, names: &[&str], feature: &str) {
    let path = c.str_new_frozen(feature.as_bytes());
    c.class_mut(owner).autoload.reserve(names.len());
    for n in names { c.class_mut(owner).autoload.insert(crate::symbol::intern(n.as_bytes()), path); }
}
