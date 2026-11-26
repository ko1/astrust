// Complex: real/imag are arbitrary real Numerics (Integer, Float, Rational, ...).
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::builtins::kernel::*;
use crate::builtins::rational::*;
use crate::heap::obj_set_flags;
use crate::symbol::{intern, s};
use crate::object::*;
use crate::value::*;
use crate::vm::*;

pub struct RComplex { pub re: Value, pub im: Value }
impl RData for RComplex {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.re); m.visit(&mut self.im); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

pub fn is_complex(c: &Ctx, v: Value) -> bool { c.data::<RComplex>(v).is_some() }
pub fn cpx_parts(c: &Ctx, v: Value) -> (Value, Value) { let r = c.data::<RComplex>(v).expect("a Complex"); (r.re, r.im) }
pub fn cpx_raw(c: &mut Ctx, re: Value, im: Value) -> Value {
    let k = c.vm().c_complex.get();
    let v = c.alloc_with(k, move || Body::Data(Box::new(RComplex { re, im })));
    obj_set_flags(v, FL_FROZEN);
    v
}
fn nul() -> Blk<'static> { Blk::NONE }

// ---- generic helpers (CRuby's f_add & co. with their shortcuts) -----------------------------

fn fz(v: Value) -> bool { v == Value::fix(0) }
// complex.c's shortcuts hold while the receiver's operator is the builtin (rb_method_basic_definition_p)
fn op_redef(x: Value, int_bit: u64, flo_bit: u64) -> bool {
    (is_integer(x) && crate::rt::redef(int_bit)) || (is_float(x) && crate::rt::redef(flo_bit))
}
fn f_add(c: &mut Ctx, x: Value, y: Value) -> RV {
    if op_redef(x, crate::rt::R_INT_PLUS, crate::rt::R_FLO_PLUS) { return funcall(c, x, s::plus, &[y], nul()); }
    if is_integer(x) { if fz(x) { return Ok(y); } if fz(y) { return Ok(x); } }
    else if (is_float(x) || is_rational(c, x)) && fz(y) { return Ok(x); }
    if x.is_fixnum() && y.is_fixnum() { return Ok(iadd(c, x, y)); }
    funcall(c, x, s::plus, &[y], nul())
}
fn f_sub(c: &mut Ctx, x: Value, y: Value) -> RV {
    if op_redef(x, crate::rt::R_INT_MINUS, crate::rt::R_FLO_MINUS) { return funcall(c, x, s::minus, &[y], nul()); }
    if fz(y) && (is_integer(x) || is_float(x) || is_rational(c, x)) { return Ok(x); }
    if x.is_fixnum() && y.is_fixnum() { return Ok(isub(c, x, y)); }
    funcall(c, x, s::minus, &[y], nul())
}
fn f_mul(c: &mut Ctx, x: Value, y: Value) -> RV {
    if op_redef(x, crate::rt::R_INT_MUL, crate::rt::R_FLO_MUL) { return funcall(c, x, s::mul, &[y], nul()); }
    if fz(y) && is_integer(x) { return Ok(Value::fix(0)); }
    if y == Value::fix(1) { return Ok(x); }
    if fz(x) && is_integer(y) { return Ok(Value::fix(0)); }
    if x == Value::fix(1) { return Ok(y); }
    if x.is_fixnum() && y.is_fixnum() { return Ok(imul(c, x, y)); }
    funcall(c, x, s::mul, &[y], nul())
}
fn f_quo(c: &mut Ctx, x: Value, y: Value) -> RV {
    if is_integer(x) || is_rational(c, x) {
        if is_float(y) { let f = to_f64(c, x)?; return Ok(c.float_new(f / float_val(y))); }
        if is_integer(y) || is_rational(c, y) {
            let xr = if is_integer(x) { rat_raw(c, x, Value::fix(1)) } else { x };
            return rat_div_v(c, xr, y);
        }
    }
    if is_float(x) && (is_integer(y) || is_float(y) || is_rational(c, y)) { let f = to_f64(c, y)?; return Ok(c.float_new(float_val(x) / f)); }
    funcall(c, x, intern(b"quo"), &[y], nul())
}
fn f_fdiv(c: &mut Ctx, x: Value, y: Value) -> RV { funcall(c, x, intern(b"fdiv"), &[y], nul()) }
fn f_neg(c: &mut Ctx, x: Value) -> RV {
    if is_integer(x) { return Ok(ineg(c, x)); }
    if is_float(x) { return Ok(c.float_new(-float_val(x))); }
    funcall(c, x, s::uminus, &[], nul())
}
fn f_abs(c: &mut Ctx, x: Value) -> RV {
    if is_integer(x) { return Ok(iabs(c, x)); }
    if is_float(x) { return Ok(c.float_new(float_val(x).abs())); }
    funcall(c, x, intern(b"abs"), &[], nul())
}
pub fn f_zero_p(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_integer(x) { return Ok(fz(x)); }
    if is_float(x) { return Ok(float_val(x) == 0.0); }
    if is_rational(c, x) { return Ok(iszero(rat_parts(c, x).0)); }
    Ok(funcall(c, x, s::eq, &[Value::fix(0)], nul())?.truthy())
}
fn exact_zero(c: &Ctx, x: Value) -> bool { fz(x) || (is_rational(c, x) && iszero(rat_parts(c, x).0)) }
fn f_eqeq(c: &mut Ctx, x: Value, y: Value) -> Result<bool, Esc> {
    if x.is_fixnum() && y.is_fixnum() { return Ok(x == y); }
    Ok(funcall(c, x, s::eq, &[y], nul())?.truthy())
}
fn f_negative_p(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_integer(x) { return Ok(isneg(c, x)); }
    if is_float(x) { return Ok(float_val(x) < 0.0); }
    if is_rational(c, x) { return Ok(isneg(c, rat_parts(c, x).0)); }
    Ok(funcall(c, x, s::lt, &[Value::fix(0)], nul())?.truthy())
}
fn f_signbit(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_float(x) { let f = float_val(x); return Ok(!f.is_nan() && f.is_sign_negative()); }
    f_negative_p(c, x)
}
/// real (non-Complex) Numeric?
fn f_real_p(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_integer(x) || is_float(x) || is_rational(c, x) { return Ok(true); }
    if is_complex(c, x) { return Ok(false); }
    Ok(funcall(c, x, intern(b"real?"), &[], nul())?.truthy())
}
fn k_numeric(c: &Ctx, x: Value) -> bool { is_integer(x) || is_float(x) || is_kind_of(c, x, c.vm().c_numeric.get()) }
fn rat_canon(c: &Ctx, x: Value) -> Value { if is_rational(c, x) { let (n, d) = rat_parts(c, x); if d == Value::fix(1) { return n; } } x }
fn to_f64(c: &mut Ctx, x: Value) -> Result<f64, Esc> { crate::builtins::numeric_methods::to_f(c, x) }

fn real_check(c: &mut Ctx, x: Value) -> RV {
    if is_integer(x) || is_float(x) || is_rational(c, x) { return Ok(x); }
    if is_complex(c, x) { let (re, im) = cpx_parts(c, x); if f_zero_p(c, im)? { return Ok(re); } }
    if !k_numeric(c, x) || !f_real_p(c, x)? { return type_err(c, "not a real"); }
    Ok(x)
}
fn canonicalize(c: &mut Ctx, re: Value, im: Value) -> RV {
    match (is_complex(c, re), is_complex(c, im)) {
        (false, false) => Ok(cpx_raw(c, re, im)),
        (false, true) => { let (ir, ii) = cpx_parts(c, im); let r = f_sub(c, re, ii)?; let i = f_add(c, Value::fix(0), ir)?; Ok(cpx_raw(c, r, i)) }
        (true, false) => { let (rr, ri) = cpx_parts(c, re); let i = f_add(c, ri, im)?; Ok(cpx_raw(c, rr, i)) }
        (true, true) => { let (rr, ri) = cpx_parts(c, re); let (ir, ii) = cpx_parts(c, im); let r = f_sub(c, rr, ii)?; let i = f_add(c, ri, ir)?; Ok(cpx_raw(c, r, i)) }
    }
}

pub fn polar(c: &mut Ctx, x: Value, y: Value) -> RV {
    if f_zero_p(c, x)? || f_zero_p(c, y)? { let z = c.float_new(0.0); return Ok(cpx_raw(c, x, z)); }
    if is_float(y) {
        let arg = float_val(y);
        let (re, im);
        if arg == std::f64::consts::PI { re = f_neg(c, x)?; im = c.float_new(0.0); }
        else if arg == std::f64::consts::FRAC_PI_2 { re = c.float_new(0.0); im = x; }
        else if arg == -std::f64::consts::FRAC_PI_2 { re = c.float_new(0.0); im = f_neg(c, x)?; }
        else if is_float(x) { let a = float_val(x); re = c.float_new(a * arg.cos()); im = c.float_new(a * arg.sin()); }
        else { let sn = c.float_new(arg.sin()); im = f_mul(c, x, sn)?; let cs = c.float_new(arg.cos()); re = f_mul(c, x, cs)?; }
        return Ok(cpx_raw(c, re, im));
    }
    let a = to_f64(c, y)?;
    let cs = c.float_new(a.cos()); let re = f_mul(c, x, cs)?; let sn = c.float_new(a.sin()); let im = f_mul(c, x, sn)?;
    canonicalize(c, re, im)
}
/// abs * exp(i * pi * ang)
pub fn polar_pi(c: &mut Ctx, abs: f64, ang: f64) -> Value {
    let fi = ang.trunc(); let fr = ang - fi;
    let pos = fr == 0.5;
    if pos || fr == -0.5 {
        let h = fi / 2.0;
        let mut a = abs;
        if ((h - h.trunc()) != fr) ^ pos { a = -a; }
        let (z, i) = (c.float_new(0.0), c.float_new(a));
        return cpx_raw(c, z, i);
    }
    if fr == 0.0 {
        let h = fi / 2.0;
        return c.float_new(if h - h.trunc() != 0.0 { -abs } else { abs });
    }
    let t = std::f64::consts::PI * ang;
    let (r, i) = (c.float_new(abs * t.cos()), c.float_new(abs * t.sin()));
    cpx_raw(c, r, i)
}

// ---- arithmetic ----------------------------------------------------------------------------

fn coerce(c: &mut Ctx, s: Value, o: Value, op: Sym) -> RV { crate::builtins::numeric_methods::coerce_bin(c, s, o, op, false) }

fn cpx_plus(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0]; let (re, im) = cpx_parts(c, s);
    if is_complex(c, o) { let (or, oi) = cpx_parts(c, o); let r = f_add(c, re, or)?; let i = f_add(c, im, oi)?; return Ok(cpx_raw(c, r, i)); }
    if k_numeric(c, o) && f_real_p(c, o)? { let r = f_add(c, re, o)?; return Ok(cpx_raw(c, r, im)); }
    coerce(c, s, o, s::plus)
}
fn cpx_minus(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0]; let (re, im) = cpx_parts(c, s);
    if is_complex(c, o) { let (or, oi) = cpx_parts(c, o); let r = f_sub(c, re, or)?; let i = f_sub(c, im, oi)?; return Ok(cpx_raw(c, r, i)); }
    if k_numeric(c, o) && f_real_p(c, o)? { let r = f_sub(c, re, o)?; return Ok(cpx_raw(c, r, im)); }
    coerce(c, s, o, s::minus)
}
fn safe_mul(c: &mut Ctx, a: Value, b: Value, az: bool, bz: bool) -> RV {
    let mut a = a; let mut b = b;
    if !az && bz && is_float(a) { let v = float_val(a); if !v.is_nan() { a = c.float_new(if v.is_sign_negative() { -1.0 } else { 1.0 }); } }
    if !bz && az && is_float(b) { let v = float_val(b); if !v.is_nan() { b = c.float_new(if v.is_sign_negative() { -1.0 } else { 1.0 }); } }
    f_mul(c, a, b)
}
fn comp_mul(c: &mut Ctx, ar: Value, ai: Value, br: Value, bi: Value) -> Result<(Value, Value), Esc> {
    let (arz, aiz, brz, biz) = (f_zero_p(c, ar)?, f_zero_p(c, ai)?, f_zero_p(c, br)?, f_zero_p(c, bi)?);
    let x = safe_mul(c, ar, br, arz, brz)?; let y = safe_mul(c, ai, bi, aiz, biz)?;
    let re = f_sub(c, x, y)?;
    let x = safe_mul(c, ar, bi, arz, biz)?; let y = safe_mul(c, ai, br, aiz, brz)?;
    let im = f_add(c, x, y)?;
    Ok((re, im))
}
fn cpx_mul_v(c: &mut Ctx, s: Value, o: Value) -> RV {
    let (re, im) = cpx_parts(c, s);
    if is_complex(c, o) { let (or, oi) = cpx_parts(c, o); let (r, i) = comp_mul(c, re, im, or, oi)?; return Ok(cpx_raw(c, r, i)); }
    // CRuby's binary (gcc) evaluates f_complex_new2's arguments imaginary part first
    if k_numeric(c, o) && f_real_p(c, o)? { let i = f_mul(c, im, o)?; let r = f_mul(c, re, o)?; return Ok(cpx_raw(c, r, i)); }
    coerce(c, s, o, s::mul)
}
fn cpx_mul(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { cpx_mul_v(c, s, a[0]) }
fn f_gt(c: &mut Ctx, x: Value, y: Value) -> Result<bool, Esc> {
    if x.is_fixnum() && y.is_fixnum() { return Ok(x.as_fix() > y.as_fix()); }
    Ok(funcall(c, x, s::gt, &[y], nul())?.truthy())
}
fn divide(c: &mut Ctx, s: Value, o: Value, fdiv: bool) -> RV {
    let func = if fdiv { f_fdiv } else { f_quo };
    let (ar, ai) = cpx_parts(c, s);
    if is_complex(c, o) {
        let (br, bi) = cpx_parts(c, o);
        let flo = is_float(ar) || is_float(ai) || is_float(br) || is_float(bi);
        let (x, y);
        let abr = f_abs(c, br)?; let abi = f_abs(c, bi)?;
        if f_gt(c, abr, abi)? {
            let r = func(c, bi, br)?;
            let rr = f_mul(c, r, r)?; let t = f_add(c, Value::fix(1), rr)?; let n = f_mul(c, br, t)?;
            let t = f_mul(c, ai, r)?; let t = f_add(c, ar, t)?; x = func(c, t, n)?;
            let t = f_mul(c, ar, r)?; let t = f_sub(c, ai, t)?; y = func(c, t, n)?;
        } else {
            let r = func(c, br, bi)?;
            let rr = f_mul(c, r, r)?; let t = f_add(c, Value::fix(1), rr)?; let n = f_mul(c, bi, t)?;
            let t = f_mul(c, ar, r)?; let t = f_add(c, t, ai)?; x = func(c, t, n)?;
            let t = f_mul(c, ai, r)?; let t = f_sub(c, t, ar)?; y = func(c, t, n)?;
        }
        return Ok(if flo { cpx_raw(c, x, y) } else { cpx_raw(c, rat_canon(c, x), rat_canon(c, y)) });
    }
    if k_numeric(c, o) && f_real_p(c, o)? {
        let x = func(c, ar, o)?; let y = func(c, ai, o)?;
        return Ok(cpx_raw(c, rat_canon(c, x), rat_canon(c, y)));
    }
    coerce(c, s, o, if fdiv { intern(b"fdiv") } else { intern(b"quo") })
}
fn cpx_div(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { divide(c, s, a[0], false) }
fn cpx_fdiv(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { divide(c, s, a[0], true) }
fn cpx_uminus(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (re, im) = cpx_parts(c, s); let r = f_neg(c, re)?; let i = f_neg(c, im)?; Ok(cpx_raw(c, r, i)) }

fn num_pow(c: &mut Ctx, x: Value, y: Value) -> RV { funcall(c, x, s::pow, &[y], nul()) }
fn zero_for(c: &mut Ctx, x: Value) -> Value { if is_float(x) { c.float_new(0.0) } else if is_rational(c, x) { rat_raw(c, Value::fix(0), Value::fix(1)) } else { Value::fix(0) } }
fn pow_special_angle(c: &mut Ctx, s: Value, o: Value) -> Result<Option<Value>, Esc> {
    if !is_integer(o) { return Ok(None); }
    let (re, im) = cpx_parts(c, s);
    let mut dir;
    let mut x;
    if f_zero_p(c, im)? { x = re; dir = 0; }
    else if f_zero_p(c, re)? { x = im; dir = 2; }
    else if f_eqeq(c, re, im)? { x = re; dir = 1; }
    else { let ni = f_neg(c, im)?; if f_eqeq(c, re, ni)? { x = im; dir = 3; } else { return Ok(None); } }
    if f_negative_p(c, x)? { x = f_neg(c, x)?; dir += 4; }
    let zx = if dir % 2 == 0 { num_pow(c, x, o)? } else {
        let two_x = f_mul(c, Value::fix(2), x)?;
        let sq = funcall(c, two_x, s::mul, &[x], nul())?;
        let half = idiv(c, o, Value::fix(2));
        let mut z = num_pow(c, sq, half)?;
        if iodd(c, o) { z = funcall(c, z, s::mul, &[x], nul())?; }
        z
    };
    const DIRS: [[i32; 2]; 8] = [[1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1], [0, -1], [1, -1]];
    let dm = imul(c, Value::fix(dir), o); let zd = imod(c, dm, Value::fix(8)).as_fix() as usize;
    let pick = |c: &mut Ctx, k: i32| -> RV { match k { 0 => Ok(zero_for(c, zx)), 1 => Ok(zx), _ => f_neg(c, zx) } };
    let zr = pick(c, DIRS[zd][0])?; let zi = pick(c, DIRS[zd][1])?;
    Ok(Some(cpx_raw(c, zr, zi)))
}
fn cpx_abs_v(c: &mut Ctx, s: Value) -> RV {
    let (re, im) = cpx_parts(c, s);
    if f_zero_p(c, re)? { let a = f_abs(c, im)?; return if is_float(re) && !is_float(im) { let f = to_f64(c, a)?; Ok(c.float_new(f)) } else { Ok(a) }; }
    if f_zero_p(c, im)? { let a = f_abs(c, re)?; return if !is_float(re) && is_float(im) { let f = to_f64(c, a)?; Ok(c.float_new(f)) } else { Ok(a) }; }
    let x = to_f64(c, re)?; let y = to_f64(c, im)?;
    Ok(c.float_new(x.hypot(y)))
}
fn cpx_arg_v(c: &mut Ctx, s: Value) -> RV {
    let (re, im) = cpx_parts(c, s);
    let y = to_f64(c, im)?; let x = to_f64(c, re)?;
    Ok(c.float_new(y.atan2(x)))
}
fn cpx_pow(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let mut o = a[0];
    if k_numeric(c, o) && exact_zero(c, o) { return Ok(cpx_raw(c, Value::fix(1), Value::fix(0))); }
    if is_rational(c, o) && rat_parts(c, o).1 == Value::fix(1) { o = rat_parts(c, o).0; }
    if is_complex(c, o) { let (or, oi) = cpx_parts(c, o); if exact_zero(c, oi) { o = or; } }
    if o == Value::fix(1) { let (re, im) = cpx_parts(c, s); return Ok(cpx_raw(c, re, im)); }
    if let Some(r) = pow_special_angle(c, s, o)? { return Ok(r); }
    if is_complex(c, o) {
        let (or, oi) = cpx_parts(c, o);
        let r = cpx_abs_v(c, s)?; let theta = cpx_arg_v(c, s)?;
        let f = to_f64(c, r)?; let lr = c.float_new(f.ln());
        let t1 = f_mul(c, or, lr)?; let t2 = f_mul(c, oi, theta)?; let e = f_sub(c, t1, t2)?;
        let f = to_f64(c, e)?; let nr = c.float_new(f.exp());
        let t1 = f_mul(c, theta, or)?; let t2 = f_mul(c, oi, lr)?; let nt = f_add(c, t1, t2)?;
        return polar(c, nr, nt);
    }
    if o.is_fixnum() {
        let mut n = o.as_fix();
        let mut base = s;
        if n < 0 { base = f_quo(c, Value::fix(1), s)?; n = -n; }
        let on = Value::fix(n);
        let (xr0, xi0) = if is_complex(c, base) { cpx_parts(c, base) } else { (base, Value::fix(0)) };
        let (mut xr, mut xi) = (xr0, xi0);
        let (mut zr, mut zi) = (xr, xi);
        if f_zero_p(c, xi)? { zr = num_pow(c, zr, on)?; }
        else if f_zero_p(c, xr)? {
            zi = num_pow(c, zi, on)?;
            if n & 2 != 0 { zi = f_neg(c, zi)?; }
            if n & 1 == 0 { std::mem::swap(&mut zr, &mut zi); }
        } else {
            n -= 1;
            while n > 0 {
                while n % 2 == 0 {
                    let a1 = f_mul(c, xr, xr)?; let a2 = f_mul(c, xi, xi)?; let tmp = f_sub(c, a1, a2)?;
                    let t = f_mul(c, Value::fix(2), xr)?; xi = f_mul(c, t, xi)?;
                    xr = tmp; n /= 2;
                }
                let (r, i) = comp_mul(c, zr, zi, xr, xi)?; zr = r; zi = i;
                n -= 1;
            }
        }
        return Ok(cpx_raw(c, zr, zi));
    }
    if k_numeric(c, o) && f_real_p(c, o)? {
        let r = cpx_abs_v(c, s)?; let theta = cpx_arg_v(c, s)?;
        let nr = num_pow(c, r, o)?; let nt = f_mul(c, theta, o)?;
        return polar(c, nr, nt);
    }
    coerce(c, s, o, s::pow)
}

fn cpx_eq(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0]; let (re, im) = cpx_parts(c, s);
    if is_complex(c, o) { let (or, oi) = cpx_parts(c, o); return Ok(Value::bool(f_eqeq(c, re, or)? && f_eqeq(c, im, oi)?)); }
    if k_numeric(c, o) && f_real_p(c, o)? { return Ok(Value::bool(f_eqeq(c, re, o)? && f_zero_p(c, im)?)); }
    let r = funcall(c, o, s::eq, &[s], nul())?; Ok(Value::bool(r.truthy()))
}
fn cpx_cmp(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0]; let (re, im) = cpx_parts(c, s);
    if !k_numeric(c, o) {
        if o.is_heap() && respond_to(c, o, s::coerce, true) && !is_string(o) {
            return match funcall(c, o, s::coerce, &[s], nul()) {
                Ok(r) if is_array(r) && c.ary_len(r) == 2 => { let (x, y) = (c.ary(r)[0], c.ary(r)[1]); funcall(c, x, s::cmp, &[y], nul()) }
                Ok(_) => Ok(NIL), Err(_) => { c.esc = EscKind::None; Ok(NIL) } };
        }
        return Ok(NIL);
    }
    if !f_zero_p(c, im)? { return Ok(NIL); }
    if is_complex(c, o) {
        let (or, oi) = cpx_parts(c, o);
        if f_zero_p(c, oi)? { return funcall(c, re, s::cmp, &[or], nul()); }
        return Ok(NIL);
    }
    funcall(c, re, s::cmp, &[o], nul())
}
fn cpx_coerce(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0];
    if is_complex(c, o) { return Ok(c.ary_new_from(&[o, s])); }
    if k_numeric(c, o) && f_real_p(c, o)? { let x = cpx_raw(c, o, Value::fix(0)); return Ok(c.ary_new_from(&[x, s])); }
    type_err(c, format!("{} can't be coerced into Complex", class_name_for_err(c, o)))
}

// ---- accessors / predicates -----------------------------------------------------------------

fn cpx_real(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(cpx_parts(c, s).0) }
fn cpx_imag(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(cpx_parts(c, s).1) }
fn cpx_abs(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { cpx_abs_v(c, s) }
fn cpx_abs2(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (re, im) = cpx_parts(c, s); let x = f_mul(c, re, re)?; let y = f_mul(c, im, im)?; f_add(c, x, y)
}
fn cpx_arg(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { cpx_arg_v(c, s) }
fn cpx_rect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (re, im) = cpx_parts(c, s); Ok(c.ary_new_from(&[re, im])) }
fn cpx_polar_m(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let r = cpx_abs_v(c, s)?; let t = cpx_arg_v(c, s)?; Ok(c.ary_new_from(&[r, t])) }
fn cpx_conj(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (re, im) = cpx_parts(c, s); let i = f_neg(c, im)?; Ok(cpx_raw(c, re, i)) }
fn cpx_real_p(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(FALSE) }
fn f_finite(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_integer(x) || is_rational(c, x) { return Ok(true); }
    if is_float(x) { return Ok(float_val(x).is_finite()); }
    Ok(funcall(c, x, intern(b"finite?"), &[], nul())?.truthy())
}
fn f_infinite(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_integer(x) || is_rational(c, x) { return Ok(false); }
    if is_float(x) { return Ok(float_val(x).is_infinite()); }
    Ok(funcall(c, x, intern(b"infinite?"), &[], nul())?.truthy())
}
fn cpx_finite_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (re, im) = cpx_parts(c, s); Ok(Value::bool(f_finite(c, re)? && f_finite(c, im)?)) }
fn cpx_infinite_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let (re, im) = cpx_parts(c, s); Ok(if f_infinite(c, re)? || f_infinite(c, im)? { Value::fix(1) } else { NIL }) }
fn cpx_hash(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (re, im) = cpx_parts(c, s);
    let h = crate::astrust::hash_merge(crate::hash::key_hash(c, re)?, crate::hash::key_hash(c, im)? ^ 0x436f);
    Ok(Value::fix((h >> 2) as i64))
}
fn cpx_eql(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0];
    if !is_complex(c, o) { return Ok(FALSE); }
    let (re, im) = cpx_parts(c, s); let (or, oi) = cpx_parts(c, o);
    if real_class_of(c, re) != real_class_of(c, or) || real_class_of(c, im) != real_class_of(c, oi) { return Ok(FALSE); }
    cpx_eq(c, s, a, Blk::NONE)
}

fn format(c: &mut Ctx, s: Value, insp: bool) -> Result<String, Esc> {
    let (re, im) = cpx_parts(c, s);
    let mut out = if insp { inspect_str(c, re)? } else { let v = funcall(c, re, s::to_s, &[], nul())?; String::from_utf8_lossy(c.str_bytes(v)).into_owned() };
    let neg = f_signbit(c, im)?;
    out.push(if neg { '-' } else { '+' });
    let ai = f_abs(c, im)?;
    let t = if insp { inspect_str(c, ai)? } else { let v = funcall(c, ai, s::to_s, &[], nul())?; String::from_utf8_lossy(c.str_bytes(v)).into_owned() };
    out.push_str(&t);
    if !t.as_bytes().last().map(|b| b.is_ascii_digit()).unwrap_or(false) { out.push('*'); }
    out.push('i');
    Ok(out)
}
fn cpx_to_s(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let t = format(c, s, false)?; Ok(c.str_new_enc(t.as_bytes(), if t.is_ascii() { ENC_ASCII } else { ENC_UTF8 })) }
fn cpx_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let t = format(c, s, true)?; Ok(c.str_new_enc(format!("({})", t).as_bytes(), if t.is_ascii() { ENC_ASCII } else { ENC_UTF8 })) }

fn range_err_conv<T>(c: &mut Ctx, s: Value, into: &str) -> Result<T, Esc> {
    let t = format(c, s, false)?;
    err(c, c.vm().c_range_error.get(), format!("can't convert {} into {}", t, into))
}
/// k_exact_zero_p: not a Float, and == 0
fn k_exact_zero(c: &mut Ctx, x: Value) -> Result<bool, Esc> { Ok(!is_float(x) && f_zero_p(c, x)?) }
fn cpx_to_i(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (re, im) = cpx_parts(c, s);
    if !k_exact_zero(c, im)? { return range_err_conv(c, s, "Integer"); }
    funcall(c, re, s::to_i, &[], nul())
}
fn cpx_to_f(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (re, im) = cpx_parts(c, s);
    if !k_exact_zero(c, im)? { return range_err_conv(c, s, "Float"); }
    funcall(c, re, s::to_f, &[], nul())
}
fn cpx_to_r(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (re, im) = cpx_parts(c, s);
    if !(is_float(im) && float_val(im) == 0.0) && !k_exact_zero(c, im)? {
        // the imaginary part's #to_r may still be an exact zero
        let r = if im.is_heap() && respond_to(c, im, intern(b"to_r"), true) { funcall(c, im, intern(b"to_r"), &[], nul())? } else { NIL };
        if r == NIL || !k_exact_zero(c, r)? { return range_err_conv(c, s, "Rational"); }
    }
    funcall(c, re, intern(b"to_r"), &[], nul())
}
fn cpx_rationalize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let (re, im) = cpx_parts(c, s);
    if !k_exact_zero(c, im)? { return range_err_conv(c, s, "Rational"); }
    funcall(c, re, intern(b"rationalize"), a, nul())
}
fn cpx_to_c(_c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(s) }
fn cpx_denominator(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (re, im) = cpx_parts(c, s);
    let a = funcall(c, re, intern(b"denominator"), &[], nul())?;
    let b = funcall(c, im, intern(b"denominator"), &[], nul())?;
    funcall(c, a, intern(b"lcm"), &[b], nul())
}
fn cpx_numerator(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let (re, im) = cpx_parts(c, s);
    let cd = cpx_denominator(c, s, &[], nul())?;
    let rn = funcall(c, re, intern(b"numerator"), &[], nul())?; let rd = funcall(c, re, intern(b"denominator"), &[], nul())?;
    let q = funcall(c, cd, s::div, &[rd], nul())?; let r = f_mul(c, rn, q)?;
    let inn = funcall(c, im, intern(b"numerator"), &[], nul())?; let id = funcall(c, im, intern(b"denominator"), &[], nul())?;
    let q = funcall(c, cd, s::div, &[id], nul())?; let i = f_mul(c, inn, q)?;
    Ok(cpx_raw(c, r, i))
}

// ---- construction ---------------------------------------------------------------------------

fn s_rect(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let re = real_check(c, a[0])?;
    let im = if a.len() > 1 { real_check(c, a[1])? } else { Value::fix(0) };
    Ok(cpx_raw(c, re, im))
}
fn s_polar(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let abs = real_check(c, a[0])?;
    let arg = if a.len() > 1 { real_check(c, a[1])? } else { Value::fix(0) };
    polar(c, abs, arg)
}

fn is_imag_unit(b: u8) -> bool { matches!(b, b'i' | b'I' | b'j' | b'J') }
/// one real number token at b[i..]: returns value (Integer/Float/Rational) and advances i.
fn read_real(c: &mut Ctx, b: &[u8], i: &mut usize) -> Result<Option<Value>, Esc> {
    let st = *i; let mut j = *i;
    if j < b.len() && (b[j] == b'+' || b[j] == b'-') { j += 1; }
    let body = j;
    let digits = |b: &[u8], j: &mut usize| -> bool { let s0 = *j; while *j < b.len() && (b[*j].is_ascii_digit() || (b[*j] == b'_' && *j > s0 && *j + 1 < b.len() && b[*j + 1].is_ascii_digit())) { *j += 1; } *j > s0 };
    let mut flo = false; let mut rat = false;
    let ip = digits(b, &mut j);
    if j < b.len() && b[j] == b'.' && j + 1 < b.len() && b[j + 1].is_ascii_digit() { j += 1; digits(b, &mut j); flo = true; }
    else if !ip { return Ok(None); }
    let exp = |b: &[u8], j: &mut usize| -> bool {
        if *j + 1 < b.len() && (b[*j] == b'e' || b[*j] == b'E') {
            let mut k = *j + 1; if k < b.len() && (b[k] == b'+' || b[k] == b'-') { k += 1; }
            let s0 = k; while k < b.len() && b[k].is_ascii_digit() { k += 1; }
            if k > s0 { *j = k; return true; }
        }
        false
    };
    if exp(b, &mut j) { flo = true; }
    if !flo && j < b.len() && b[j] == b'/' && j + 1 < b.len() && b[j + 1].is_ascii_digit() {
        j += 1; digits(b, &mut j); rat = true;
    }
    let _ = body;
    let txt: String = b[st..j].iter().filter(|&&ch| ch != b'_').map(|&ch| ch as char).collect();
    *i = j;
    let v = if flo { c.float_new(txt.parse::<f64>().unwrap_or(0.0)) }
        else if rat { match parse_rational(c, txt.as_bytes(), true, true)? { Some(r) => r, None => return Ok(None) } }
        else { big_to_int(c, &txt) };
    Ok(Some(v))
}
fn big_to_int(c: &mut Ctx, t: &str) -> Value { c.big_to_value(crate::bignum::BigInt::parse(t, 10).unwrap_or(crate::bignum::BigInt::zero())) }
fn ws(b: &[u8], mut i: usize) -> usize { while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) { i += 1; } i }
/// Returns (value, fully_ok)
fn parse_complex(c: &mut Ctx, b: &[u8]) -> Result<(Value, bool), Esc> {
    let zero = Value::fix(0);
    let mut i = ws(b, 0);
    let sign_unit = |b: &[u8], i: usize| -> Option<(i64, usize)> {
        let mut k = i; let mut sg = 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') { if b[k] == b'-' { sg = -1; } k += 1; }
        if k < b.len() && is_imag_unit(b[k]) { Some((sg, k + 1)) } else { None }
    };
    let fin = |i: usize| ws(b, i) == b.len();
    if let Some((sg, k)) = sign_unit(b, i) { return Ok((cpx_raw(c, zero, Value::fix(sg)), fin(k))); }
    let Some(n1) = read_real(c, b, &mut i)? else { return Ok((cpx_raw(c, zero, zero), false)) };
    if i < b.len() && is_imag_unit(b[i]) { i += 1; return Ok((cpx_raw(c, zero, n1), fin(i))); }
    if i < b.len() && b[i] == b'@' {
        let mut k = i + 1;
        let Some(n2) = read_real(c, b, &mut k)? else { return Ok((cpx_raw(c, n1, zero), false)) };
        let r = polar(c, n1, n2)?;
        return Ok((r, fin(k)));
    }
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        if let Some((sg, k)) = sign_unit(b, i) { return Ok((cpx_raw(c, n1, Value::fix(sg)), fin(k))); }
        let mut k = i;
        let Some(n2) = read_real(c, b, &mut k)? else { return Ok((cpx_raw(c, n1, zero), false)) };
        if k < b.len() && is_imag_unit(b[k]) { return Ok((cpx_raw(c, n1, n2), fin(k + 1))); }
        return Ok((cpx_raw(c, n1, zero), false));
    }
    Ok((cpx_raw(c, n1, zero), fin(i)))
}
fn str_to_c(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let e = c.str_enc(s);
    if !crate::builtins::encoding::ascii_compat(e) { return err(c, c.vm().c_compat_error.get(), format!("ASCII incompatible encoding: {}", crate::builtins::encoding::enc_name(e))); }
    let b = c.str_bytes(s).to_vec();
    Ok(parse_complex(c, &b)?.0)
}
fn str_to_c_strict(c: &mut Ctx, v: Value, raise: bool) -> RV {
    let e = c.str_enc(v);
    if !crate::builtins::encoding::ascii_compat(e) { return err(c, c.vm().c_compat_error.get(), format!("ASCII incompatible encoding: {}", crate::builtins::encoding::enc_name(e))); }
    let b = c.str_bytes(v).to_vec();
    if b.contains(&0) { if raise { return arg_err(c, "string contains null byte"); } return Ok(NIL); }
    let (r, ok) = parse_complex(c, &b)?;
    if !ok { if !raise { return Ok(NIL); } let ins = inspect_str(c, v)?; return arg_err(c, format!("invalid value for convert(): {}", ins)); }
    Ok(r)
}
fn convert(c: &mut Ctx, a1: Value, a2: Option<Value>, raise: bool) -> RV {
    if a1 == NIL || a2 == Some(NIL) { if !raise { return Ok(NIL); } return type_err(c, "can't convert nil into Complex"); }
    let mut a1 = a1; let mut a2 = a2;
    if is_string(a1) { a1 = str_to_c_strict(c, a1, raise)?; if a1 == NIL { return Ok(NIL); } }
    if let Some(x) = a2 { if is_string(x) { let r = str_to_c_strict(c, x, raise)?; if r == NIL { return Ok(NIL); } a2 = Some(r); } }
    if is_complex(c, a1) { let (re, im) = cpx_parts(c, a1); if exact_zero(c, im) { a1 = re; } }
    if let Some(x) = a2 { if is_complex(c, x) { let (re, im) = cpx_parts(c, x); if exact_zero(c, im) { a2 = Some(re); } } }
    if is_complex(c, a1) && a2.map(|x| exact_zero(c, x)).unwrap_or(true) { return Ok(a1); }
    match a2 {
        None => {
            if k_numeric(c, a1) && !f_real_p(c, a1)? { return Ok(a1); }
            if !k_numeric(c, a1) {
                let r: RV = (|| {
                    if a1.is_heap() && respond_to(c, a1, intern(b"to_c"), true) {
                        let r = funcall(c, a1, intern(b"to_c"), &[], nul())?;
                        if is_complex(c, r) { return Ok(r); }
                        let cn = class_name_for_err(c, a1);
                        return type_err(c, format!("can't convert {} to Complex ({}#to_c gives {})", cn, cn, class_name_for_err(c, r)));
                    }
                    type_err(c, format!("can't convert {} into Complex", class_name_for_err(c, a1)))
                })();
                return match r { Err(e) => { if !raise && c.esc == EscKind::Raise { c.esc = EscKind::None; return Ok(NIL); } Err(e) } ok => ok };
            }
        }
        Some(b2) => {
            if k_numeric(c, a1) && k_numeric(c, b2) && (!f_real_p(c, a1)? || !f_real_p(c, b2)?) {
                let iu = cpx_raw(c, Value::fix(0), Value::fix(1));
                let m = funcall(c, b2, s::mul, &[iu], nul())?;
                return funcall(c, a1, s::plus, &[m], nul());
            }
        }
    }
    if let Some(x) = a2 { if !raise && !is_integer(x) && !is_float(x) && !is_rational(c, x) { return Ok(NIL); } }
    let re = real_check(c, a1)?;
    let im = match a2 { Some(x) => real_check(c, x)?, None => Value::fix(0) };
    canonicalize(c, re, im)
}
fn k_complex(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (n, raise) = kw_exception(c, a)?;
    let a = &a[..n];
    check_argc(c, a, 1, 2)?;
    convert(c, a[0], a.get(1).copied(), raise)
}

// ---- Numeric side ---------------------------------------------------------------------------

fn num_to_c(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(cpx_raw(c, s, Value::fix(0))) }
fn num_i(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(cpx_raw(c, Value::fix(0), s)) }
fn num_polar(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let a = funcall(c, s, intern(b"abs"), &[], nul())?;
    let t = funcall(c, s, intern(b"arg"), &[], nul())?;
    Ok(c.ary_new_from(&[a, t]))
}
fn num_abs2(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { funcall(c, s, s::mul, &[s], nul()) }
fn flo_arg(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let f = float_val(s);
    if f.is_nan() { return Ok(s); }
    Ok(if f.is_sign_negative() { c.float_new(std::f64::consts::PI) } else { Value::fix(0) })
}
fn nil_to_c(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(cpx_raw(c, Value::fix(0), Value::fix(0))) }

/// Boot: defines Complex (returned) and the conversions on the other classes.
pub fn init(c: &mut Ctx) -> Value {
    let vmr = c.vm();
    let k = define_class(c, "Complex", vmr.c_numeric.get());
    class_set_alloc(c, k, AllocKind::None);
    let sk = singleton_class(c, k);
    define_method_me(c, sk, s::new, new_me(s::new, sk, VISI_PUBLIC, MKind::Undef));
    define_singleton_cfunc(c, k, "rect", s_rect, -1);
    define_singleton_cfunc(c, k, "rectangular", s_rect, -1);
    define_singleton_cfunc(c, k, "polar", s_polar, -1);
    let d = |c: &mut Ctx, nm: &str, f: CFunc, ar: i32| define_cfunc(c, k, nm, f, ar);
    d(c, "+", cpx_plus, 1); d(c, "-", cpx_minus, 1); d(c, "*", cpx_mul, 1); d(c, "/", cpx_div, 1); d(c, "quo", cpx_div, 1);
    d(c, "fdiv", cpx_fdiv, 1); d(c, "**", cpx_pow, 1); d(c, "-@", cpx_uminus, 0); d(c, "==", cpx_eq, 1); d(c, "<=>", cpx_cmp, 1);
    d(c, "coerce", cpx_coerce, 1); d(c, "real", cpx_real, 0); d(c, "imaginary", cpx_imag, 0); d(c, "imag", cpx_imag, 0);
    d(c, "abs", cpx_abs, 0); d(c, "magnitude", cpx_abs, 0); d(c, "abs2", cpx_abs2, 0); d(c, "arg", cpx_arg, 0); d(c, "angle", cpx_arg, 0);
    d(c, "phase", cpx_arg, 0); d(c, "rectangular", cpx_rect, 0); d(c, "rect", cpx_rect, 0); d(c, "polar", cpx_polar_m, 0);
    d(c, "conjugate", cpx_conj, 0); d(c, "conj", cpx_conj, 0); d(c, "real?", cpx_real_p, 0); d(c, "finite?", cpx_finite_p, 0);
    d(c, "infinite?", cpx_infinite_p, 0); d(c, "hash", cpx_hash, 0); d(c, "eql?", cpx_eql, 1); d(c, "to_s", cpx_to_s, 0);
    d(c, "inspect", cpx_inspect, 0); d(c, "to_i", cpx_to_i, 0); d(c, "to_f", cpx_to_f, 0); d(c, "to_r", cpx_to_r, 0);
    d(c, "rationalize", cpx_rationalize, -1); d(c, "to_c", cpx_to_c, 0); d(c, "numerator", cpx_numerator, 0);
    d(c, "denominator", cpx_denominator, 0);
    define_private_cfunc(c, k, "marshal_dump", |c, s, _a, _b| { let (re, im) = cpx_parts(c, s); Ok(c.ary_new_from(&[re, im])) }, 0);
    for nm in ["%", "<", "<=", ">", ">=", "between?", "clamp", "div", "divmod", "floor", "ceil", "modulo", "remainder",
               "round", "step", "truncate", "i", "positive?", "negative?"] {
        let id = intern(nm.as_bytes());
        define_method_me(c, k, id, new_me(id, k, VISI_PUBLIC, MKind::Undef));
    }
    let iu = c.alloc(k, Body::Data(Box::new(RComplex { re: Value::fix(0), im: Value::fix(1) })));
    obj_set_flags(iu, FL_FROZEN);
    const_set(c, k, intern(b"I"), iu);
    let n = vmr.c_numeric.get();
    define_cfunc(c, n, "to_c", num_to_c, 0);
    define_cfunc(c, n, "i", num_i, 0);
    define_cfunc(c, n, "polar", num_polar, 0);
    define_cfunc(c, n, "abs2", num_abs2, 0);
    define_cfunc(c, vmr.c_float.get(), "arg", flo_arg, 0);
    define_cfunc(c, vmr.c_float.get(), "angle", flo_arg, 0);
    define_cfunc(c, vmr.c_float.get(), "phase", flo_arg, 0);
    define_cfunc(c, vmr.c_nil.get(), "to_c", nil_to_c, 0);
    define_cfunc(c, vmr.c_string.get(), "to_c", str_to_c, 0);
    define_module_function(c, vmr.m_kernel.get(), "Complex", k_complex, -1);
    k
}
