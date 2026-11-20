// Inline fast-path helpers for node.def (operator nodes).
use crate::rt::*;
use crate::value::*;

/// Operands as doubles when Float op Float / Float op Integer / Integer op Float applies
/// (flonum and fixnum only); `rf` / `ri` are the redefinition flags of Float#op / Integer#op.
#[inline(always)]
pub fn flo2(a: Value, b: Value, rf: u32, ri: u32) -> Option<(f64, f64)> {
    if a.is_flonum() {
        if redef(rf) { return None; }
        if b.is_flonum() { return Some((a.flonum_val(), b.flonum_val())); }
        if b.is_fixnum() { return Some((a.flonum_val(), b.as_fix() as f64)); }
    } else if a.is_fixnum() && b.is_flonum() && !redef(ri) {
        return Some((a.as_fix() as f64, b.flonum_val()));
    }
    None
}

// an Integer is compared exactly: only fixnums a double represents exactly take the fast path
#[inline(always)]
fn exact(i: i64) -> bool { (i + (1i64 << 53)) as u64 <= 1u64 << 54 }

/// flo2 for comparisons.
#[inline(always)]
pub fn flo2_cmp(a: Value, b: Value, rf: u32, ri: u32) -> Option<(f64, f64)> {
    if a.is_flonum() {
        if redef(rf) { return None; }
        if b.is_flonum() { return Some((a.flonum_val(), b.flonum_val())); }
        if b.is_fixnum() && exact(b.as_fix()) { return Some((a.flonum_val(), b.as_fix() as f64)); }
    } else if a.is_fixnum() && b.is_flonum() && exact(a.as_fix()) && !redef(ri) {
        return Some((a.as_fix() as f64, b.flonum_val()));
    }
    None
}

/// `pat === subj` when identity alone decides it (immediate patterns).
#[inline(always)]
pub fn eqq_imm(p: Value, subj: Value) -> Option<bool> {
    if p.is_fixnum() { return if subj.is_fixnum() { Some(p == subj) } else { None }; }
    if p.is_special() { return Some(p == subj); }
    None
}
