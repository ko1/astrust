// Inline fast-path helpers for node.def (operator nodes).
use crate::rt::*;
use crate::symbol::s;
use crate::value::*;
use crate::vm::{Ctx, OpCc, Pos, RV};

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

/// What an operator site's operands were besides fixnums; set by the interpreter, read when the site is baked.
#[derive(Default)]
pub struct TyFb(pub std::cell::Cell<u32>);
impl TyFb {
    #[inline(always)] pub fn add(&self, f: u32) { self.0.set(self.0.get() | f); }
}
pub const TF_FLO: u32 = 1;
pub const TF_HASH: u32 = 2;

/// Does a site with the baked `ty` (0: interpreter, else 1 + TyFb) keep its Float paths inline?
#[inline(always)]
pub fn tf_flo(ty: u32) -> bool { ty == 0 || (ty - 1) & TF_FLO != 0 }
#[inline(always)]
pub fn tf_hash(ty: u32) -> bool { ty == 0 || (ty - 1) & TF_HASH != 0 }

/// The rest of an operator at a baked site whose inline paths did not apply: Float, then the call.
/// A site whose recorded kinds are stale or absent ends up here, slower but right.
#[inline(never)]
#[cold]
pub fn srb_arith_out(c: &mut Ctx, a: Value, b: Value, mid: Sym, line: Pos, cc: &OpCc) -> RV {
    match mid {
        s::plus => if let Some((x, y)) = flo2(a, b, R_FLO_PLUS, R_INT_PLUS) { return Ok(c.float_new(x + y)); }
        s::minus => if let Some((x, y)) = flo2(a, b, R_FLO_MINUS, R_INT_MINUS) { return Ok(c.float_new(x - y)); }
        s::mul => if let Some((x, y)) = flo2(a, b, R_FLO_MUL, R_INT_MUL) { return Ok(c.float_new(x * y)); }
        s::div => if let Some((x, y)) = flo2(a, b, R_FLO_DIV, R_INT_DIV) { return Ok(c.float_new(x / y)); }
        s::lt => if let Some((x, y)) = flo2_cmp(a, b, R_FLO_LT, R_INT_LT) { return Ok(Value::bool(x < y)); }
        s::le => if let Some((x, y)) = flo2_cmp(a, b, R_FLO_LE, R_INT_LE) { return Ok(Value::bool(x <= y)); }
        s::gt => if let Some((x, y)) = flo2_cmp(a, b, R_FLO_GT, R_INT_GT) { return Ok(Value::bool(x > y)); }
        s::ge => if let Some((x, y)) = flo2_cmp(a, b, R_FLO_GE, R_INT_GE) { return Ok(Value::bool(x >= y)); }
        s::neq => if let Some((x, y)) = flo2_cmp(a, b, R_FLO_EQ, R_INT_EQ) { return Ok(Value::bool(x != y)); }
        s::eq => {
            if let Some((x, y)) = flo2_cmp(a, b, R_FLO_EQ, R_INT_EQ) { return Ok(Value::bool(x == y)); }
            return srb_eq(c, a, b, line, cc);
        }
        _ => {}
    }
    srb_binop(c, a, b, mid, line, cc)
}

/// `pat === subj` when identity alone decides it (immediate patterns).
#[inline(always)]
pub fn eqq_imm(p: Value, subj: Value) -> Option<bool> {
    if p.is_fixnum() { return if subj.is_fixnum() { Some(p == subj) } else { None }; }
    if p.is_special() { return Some(p == subj); }
    None
}
