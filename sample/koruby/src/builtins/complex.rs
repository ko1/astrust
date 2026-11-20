// Complex: real/imag are arbitrary real Numerics (Integer, Float, Rational, ...).
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::rational::*;
use crate::numeric::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

pub struct RComplex { pub re: Value, pub im: Value }
impl RData for RComplex {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.re); m.visit(&mut self.im); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

pub fn is_complex(v: Value) -> bool { v.is_heap() && rdata::<RComplex>(v).is_some() }
pub fn cpx_parts(v: Value) -> (Value, Value) { let r = rdata::<RComplex>(v).unwrap(); (r.re, r.im) }
pub fn cpx_raw(re: Value, im: Value) -> Value {
    let v = crate::gc::alloc(vm().c_complex, Body::Data(Box::new(RComplex { re, im })));
    obj(v).flags |= FL_FROZEN;
    v
}
fn nul() -> *const Block { std::ptr::null() }

// ---- generic helpers (CRuby's f_add & co. with their shortcuts) -----------------------------

fn fz(v: Value) -> bool { v == Value::fix(0) }
fn f_add(c: &mut Ctx, x: Value, y: Value) -> RV {
    if is_integer(x) { if fz(x) { return Ok(y); } if fz(y) { return Ok(x); } }
    else if (is_float(x) || is_rational(x)) && fz(y) { return Ok(x); }
    if x.is_fixnum() && y.is_fixnum() { return Ok(iadd(x, y)); }
    funcall(c, x, s::plus, &[y], nul())
}
fn f_sub(c: &mut Ctx, x: Value, y: Value) -> RV {
    if fz(y) && (is_integer(x) || is_float(x) || is_rational(x)) { return Ok(x); }
    if x.is_fixnum() && y.is_fixnum() { return Ok(isub(x, y)); }
    funcall(c, x, s::minus, &[y], nul())
}
fn f_mul(c: &mut Ctx, x: Value, y: Value) -> RV {
    if fz(y) && is_integer(x) { return Ok(Value::fix(0)); }
    if y == Value::fix(1) { return Ok(x); }
    if fz(x) && is_integer(y) { return Ok(Value::fix(0)); }
    if x == Value::fix(1) { return Ok(y); }
    if x.is_fixnum() && y.is_fixnum() { return Ok(imul(x, y)); }
    funcall(c, x, s::mul, &[y], nul())
}
fn f_quo(c: &mut Ctx, x: Value, y: Value) -> RV {
    if is_integer(x) || is_rational(x) {
        if is_float(y) { return Ok(float_new(to_f64(c, x)? / float_val(y))); }
        if is_integer(y) || is_rational(y) {
            let xr = if is_integer(x) { rat_raw(x, Value::fix(1)) } else { x };
            return rat_div_v(c, xr, y);
        }
    }
    if is_float(x) && (is_integer(y) || is_float(y) || is_rational(y)) { return Ok(float_new(float_val(x) / to_f64(c, y)?)); }
    funcall(c, x, intern(b"quo"), &[y], nul())
}
fn f_fdiv(c: &mut Ctx, x: Value, y: Value) -> RV { funcall(c, x, intern(b"fdiv"), &[y], nul()) }
fn f_neg(c: &mut Ctx, x: Value) -> RV {
    if is_integer(x) { return Ok(ineg(x)); }
    if is_float(x) { return Ok(float_new(-float_val(x))); }
    funcall(c, x, s::uminus, &[], nul())
}
fn f_abs(c: &mut Ctx, x: Value) -> RV {
    if is_integer(x) { return Ok(iabs(x)); }
    if is_float(x) { return Ok(float_new(float_val(x).abs())); }
    funcall(c, x, intern(b"abs"), &[], nul())
}
pub fn f_zero_p(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_integer(x) { return Ok(fz(x)); }
    if is_float(x) { return Ok(float_val(x) == 0.0); }
    if is_rational(x) { return Ok(iszero(rat_parts(x).0)); }
    Ok(funcall(c, x, s::eq, &[Value::fix(0)], nul())?.truthy())
}
fn exact_zero(x: Value) -> bool { fz(x) || (is_rational(x) && iszero(rat_parts(x).0)) }
fn f_eqeq(c: &mut Ctx, x: Value, y: Value) -> Result<bool, Esc> {
    if x.is_fixnum() && y.is_fixnum() { return Ok(x == y); }
    Ok(funcall(c, x, s::eq, &[y], nul())?.truthy())
}
fn f_negative_p(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_integer(x) { return Ok(isneg(x)); }
    if is_float(x) { return Ok(float_val(x) < 0.0); }
    if is_rational(x) { return Ok(isneg(rat_parts(x).0)); }
    Ok(funcall(c, x, s::lt, &[Value::fix(0)], nul())?.truthy())
}
fn f_signbit(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_float(x) { let f = float_val(x); return Ok(!f.is_nan() && f.is_sign_negative()); }
    f_negative_p(c, x)
}
/// real (non-Complex) Numeric?
fn f_real_p(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_integer(x) || is_float(x) || is_rational(x) { return Ok(true); }
    if is_complex(x) { return Ok(false); }
    Ok(funcall(c, x, intern(b"real?"), &[], nul())?.truthy())
}
fn k_numeric(x: Value) -> bool { is_integer(x) || is_float(x) || is_kind_of(x, vm().c_numeric) }
fn rat_canon(x: Value) -> Value { if is_rational(x) { let (n, d) = rat_parts(x); if d == Value::fix(1) { return n; } } x }
fn to_f64(c: &mut Ctx, x: Value) -> Result<f64, Esc> { crate::builtins::numeric_methods::to_f(c, x) }

fn real_check(c: &mut Ctx, x: Value) -> RV {
    if is_integer(x) || is_float(x) || is_rational(x) { return Ok(x); }
    if is_complex(x) { let (re, im) = cpx_parts(x); if f_zero_p(c, im)? { return Ok(re); } }
    if !k_numeric(x) || !f_real_p(c, x)? { return type_err(c, "not a real"); }
    Ok(x)
}
fn canonicalize(c: &mut Ctx, re: Value, im: Value) -> RV {
    match (is_complex(re), is_complex(im)) {
        (false, false) => Ok(cpx_raw(re, im)),
        (false, true) => { let (ir, ii) = cpx_parts(im); let r = f_sub(c, re, ii)?; let i = f_add(c, Value::fix(0), ir)?; Ok(cpx_raw(r, i)) }
        (true, false) => { let (rr, ri) = cpx_parts(re); let i = f_add(c, ri, im)?; Ok(cpx_raw(rr, i)) }
        (true, true) => { let (rr, ri) = cpx_parts(re); let (ir, ii) = cpx_parts(im); let r = f_sub(c, rr, ii)?; let i = f_add(c, ri, ir)?; Ok(cpx_raw(r, i)) }
    }
}

pub fn polar(c: &mut Ctx, x: Value, y: Value) -> RV {
    if f_zero_p(c, x)? || f_zero_p(c, y)? { return Ok(cpx_raw(x, float_new(0.0))); }
    if is_float(y) {
        let arg = float_val(y);
        let (re, im);
        if arg == std::f64::consts::PI { re = f_neg(c, x)?; im = float_new(0.0); }
        else if arg == std::f64::consts::FRAC_PI_2 { re = float_new(0.0); im = x; }
        else if arg == -std::f64::consts::FRAC_PI_2 { re = float_new(0.0); im = f_neg(c, x)?; }
        else if is_float(x) { let a = float_val(x); re = float_new(a * arg.cos()); im = float_new(a * arg.sin()); }
        else { im = f_mul(c, x, float_new(arg.sin()))?; re = f_mul(c, x, float_new(arg.cos()))?; }
        return Ok(cpx_raw(re, im));
    }
    let a = to_f64(c, y)?;
    let re = f_mul(c, x, float_new(a.cos()))?; let im = f_mul(c, x, float_new(a.sin()))?;
    canonicalize(c, re, im)
}
/// abs * exp(i * pi * ang)
pub fn polar_pi(abs: f64, ang: f64) -> Value {
    let fi = ang.trunc(); let fr = ang - fi;
    let pos = fr == 0.5;
    if pos || fr == -0.5 {
        let h = fi / 2.0;
        let mut a = abs;
        if ((h - h.trunc()) != fr) ^ pos { a = -a; }
        return cpx_raw(float_new(0.0), float_new(a));
    }
    if fr == 0.0 {
        let h = fi / 2.0;
        return float_new(if h - h.trunc() != 0.0 { -abs } else { abs });
    }
    let t = std::f64::consts::PI * ang;
    cpx_raw(float_new(abs * t.cos()), float_new(abs * t.sin()))
}

// ---- arithmetic ----------------------------------------------------------------------------

fn coerce(c: &mut Ctx, s: Value, o: Value, op: Sym) -> RV { crate::builtins::numeric_methods::coerce_bin(c, s, o, op, false) }

fn cpx_plus(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0]; let (re, im) = cpx_parts(s);
    if is_complex(o) { let (or, oi) = cpx_parts(o); let r = f_add(c, re, or)?; let i = f_add(c, im, oi)?; return Ok(cpx_raw(r, i)); }
    if k_numeric(o) && f_real_p(c, o)? { let r = f_add(c, re, o)?; return Ok(cpx_raw(r, im)); }
    coerce(c, s, o, s::plus)
}
fn cpx_minus(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0]; let (re, im) = cpx_parts(s);
    if is_complex(o) { let (or, oi) = cpx_parts(o); let r = f_sub(c, re, or)?; let i = f_sub(c, im, oi)?; return Ok(cpx_raw(r, i)); }
    if k_numeric(o) && f_real_p(c, o)? { let r = f_sub(c, re, o)?; return Ok(cpx_raw(r, im)); }
    coerce(c, s, o, s::minus)
}
fn safe_mul(c: &mut Ctx, a: Value, b: Value, az: bool, bz: bool) -> RV {
    let mut a = a; let mut b = b;
    if !az && bz && is_float(a) { let v = float_val(a); if !v.is_nan() { a = float_new(if v.is_sign_negative() { -1.0 } else { 1.0 }); } }
    if !bz && az && is_float(b) { let v = float_val(b); if !v.is_nan() { b = float_new(if v.is_sign_negative() { -1.0 } else { 1.0 }); } }
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
    let (re, im) = cpx_parts(s);
    if is_complex(o) { let (or, oi) = cpx_parts(o); let (r, i) = comp_mul(c, re, im, or, oi)?; return Ok(cpx_raw(r, i)); }
    if k_numeric(o) && f_real_p(c, o)? { let r = f_mul(c, re, o)?; let i = f_mul(c, im, o)?; return Ok(cpx_raw(r, i)); }
    coerce(c, s, o, s::mul)
}
fn cpx_mul(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { cpx_mul_v(c, s, a[0]) }
fn f_gt(c: &mut Ctx, x: Value, y: Value) -> Result<bool, Esc> {
    if x.is_fixnum() && y.is_fixnum() { return Ok(x.as_fix() > y.as_fix()); }
    Ok(funcall(c, x, s::gt, &[y], nul())?.truthy())
}
fn divide(c: &mut Ctx, s: Value, o: Value, fdiv: bool) -> RV {
    let func = if fdiv { f_fdiv } else { f_quo };
    let (ar, ai) = cpx_parts(s);
    if is_complex(o) {
        let (br, bi) = cpx_parts(o);
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
        return Ok(if flo { cpx_raw(x, y) } else { cpx_raw(rat_canon(x), rat_canon(y)) });
    }
    if k_numeric(o) && f_real_p(c, o)? {
        let x = func(c, ar, o)?; let y = func(c, ai, o)?;
        return Ok(cpx_raw(rat_canon(x), rat_canon(y)));
    }
    coerce(c, s, o, if fdiv { intern(b"fdiv") } else { s::div })
}
fn cpx_div(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { divide(c, s, a[0], false) }
fn cpx_fdiv(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { divide(c, s, a[0], true) }
fn cpx_uminus(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let (re, im) = cpx_parts(s); let r = f_neg(c, re)?; let i = f_neg(c, im)?; Ok(cpx_raw(r, i)) }

fn num_pow(c: &mut Ctx, x: Value, y: Value) -> RV { funcall(c, x, s::pow, &[y], nul()) }
fn zero_for(x: Value) -> Value { if is_float(x) { float_new(0.0) } else if is_rational(x) { rat_raw(Value::fix(0), Value::fix(1)) } else { Value::fix(0) } }
fn pow_special_angle(c: &mut Ctx, s: Value, o: Value) -> Result<Option<Value>, Esc> {
    if !is_integer(o) { return Ok(None); }
    let (re, im) = cpx_parts(s);
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
        let half = idiv(o, Value::fix(2));
        let mut z = num_pow(c, sq, half)?;
        if iodd(o) { z = funcall(c, z, s::mul, &[x], nul())?; }
        z
    };
    const DIRS: [[i32; 2]; 8] = [[1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1], [0, -1], [1, -1]];
    let zd = imod(imul(Value::fix(dir), o), Value::fix(8)).as_fix() as usize;
    let pick = |c: &mut Ctx, k: i32| -> RV { match k { 0 => Ok(zero_for(zx)), 1 => Ok(zx), _ => f_neg(c, zx) } };
    let zr = pick(c, DIRS[zd][0])?; let zi = pick(c, DIRS[zd][1])?;
    Ok(Some(cpx_raw(zr, zi)))
}
fn cpx_abs_v(c: &mut Ctx, s: Value) -> RV {
    let (re, im) = cpx_parts(s);
    if f_zero_p(c, re)? { let a = f_abs(c, im)?; return if is_float(re) && !is_float(im) { Ok(float_new(to_f64(c, a)?)) } else { Ok(a) }; }
    if f_zero_p(c, im)? { let a = f_abs(c, re)?; return if !is_float(re) && is_float(im) { Ok(float_new(to_f64(c, a)?)) } else { Ok(a) }; }
    let x = to_f64(c, re)?; let y = to_f64(c, im)?;
    Ok(float_new(x.hypot(y)))
}
fn cpx_arg_v(c: &mut Ctx, s: Value) -> RV {
    let (re, im) = cpx_parts(s);
    let y = to_f64(c, im)?; let x = to_f64(c, re)?;
    Ok(float_new(y.atan2(x)))
}
fn cpx_pow(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let mut o = a[0];
    if k_numeric(o) && exact_zero(o) { return Ok(cpx_raw(Value::fix(1), Value::fix(0))); }
    if is_rational(o) && rat_parts(o).1 == Value::fix(1) { o = rat_parts(o).0; }
    if is_complex(o) { let (or, oi) = cpx_parts(o); if exact_zero(oi) { o = or; } }
    if o == Value::fix(1) { let (re, im) = cpx_parts(s); return Ok(cpx_raw(re, im)); }
    if let Some(r) = pow_special_angle(c, s, o)? { return Ok(r); }
    if is_complex(o) {
        let (or, oi) = cpx_parts(o);
        let r = cpx_abs_v(c, s)?; let theta = cpx_arg_v(c, s)?;
        let lr = float_new(to_f64(c, r)?.ln());
        let t1 = f_mul(c, or, lr)?; let t2 = f_mul(c, oi, theta)?; let e = f_sub(c, t1, t2)?;
        let nr = float_new(to_f64(c, e)?.exp());
        let t1 = f_mul(c, theta, or)?; let t2 = f_mul(c, oi, lr)?; let nt = f_add(c, t1, t2)?;
        return polar(c, nr, nt);
    }
    if o.is_fixnum() {
        let mut n = o.as_fix();
        let mut base = s;
        if n < 0 { base = f_quo(c, Value::fix(1), s)?; n = -n; }
        let on = Value::fix(n);
        let (xr0, xi0) = if is_complex(base) { cpx_parts(base) } else { (base, Value::fix(0)) };
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
        return Ok(cpx_raw(zr, zi));
    }
    if k_numeric(o) && f_real_p(c, o)? {
        let r = cpx_abs_v(c, s)?; let theta = cpx_arg_v(c, s)?;
        let nr = num_pow(c, r, o)?; let nt = f_mul(c, theta, o)?;
        return polar(c, nr, nt);
    }
    coerce(c, s, o, s::pow)
}

fn cpx_eq(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0]; let (re, im) = cpx_parts(s);
    if is_complex(o) { let (or, oi) = cpx_parts(o); return Ok(Value::bool(f_eqeq(c, re, or)? && f_eqeq(c, im, oi)?)); }
    if k_numeric(o) && f_real_p(c, o)? { return Ok(Value::bool(f_eqeq(c, re, o)? && f_zero_p(c, im)?)); }
    let r = funcall(c, o, s::eq, &[s], nul())?; Ok(Value::bool(r.truthy()))
}
fn cpx_cmp(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0]; let (re, im) = cpx_parts(s);
    if !k_numeric(o) {
        if o.is_heap() && respond_to(o, s::coerce, true) && !is_string(o) {
            return match funcall(c, o, s::coerce, &[s], nul()) {
                Ok(r) if is_array(r) && ary_len(r) == 2 => { let x = ary_get(r, 0); funcall(c, x, s::cmp, &[ary_get(r, 1)], nul()) }
                Ok(_) => Ok(NIL), Err(_) => { c.esc = EscKind::None; Ok(NIL) } };
        }
        return Ok(NIL);
    }
    if !f_zero_p(c, im)? { return Ok(NIL); }
    if is_complex(o) {
        let (or, oi) = cpx_parts(o);
        if f_zero_p(c, oi)? { return funcall(c, re, s::cmp, &[or], nul()); }
        return Ok(NIL);
    }
    funcall(c, re, s::cmp, &[o], nul())
}
fn cpx_coerce(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if is_complex(o) { return Ok(ary_new_from(&[o, s])); }
    if k_numeric(o) && f_real_p(c, o)? { return Ok(ary_new_from(&[cpx_raw(o, Value::fix(0)), s])); }
    type_err(c, format!("{} can't be coerced into Complex", class_name_for_err(o)))
}

// ---- accessors / predicates -----------------------------------------------------------------

fn cpx_real(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(cpx_parts(s).0) }
fn cpx_imag(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(cpx_parts(s).1) }
fn cpx_abs(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { cpx_abs_v(c, s) }
fn cpx_abs2(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (re, im) = cpx_parts(s); let x = f_mul(c, re, re)?; let y = f_mul(c, im, im)?; f_add(c, x, y)
}
fn cpx_arg(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { cpx_arg_v(c, s) }
fn cpx_rect(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let (re, im) = cpx_parts(s); Ok(ary_new_from(&[re, im])) }
fn cpx_polar_m(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let r = cpx_abs_v(c, s)?; let t = cpx_arg_v(c, s)?; Ok(ary_new_from(&[r, t])) }
fn cpx_conj(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let (re, im) = cpx_parts(s); let i = f_neg(c, im)?; Ok(cpx_raw(re, i)) }
fn cpx_real_p(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(FALSE) }
fn f_finite(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_integer(x) || is_rational(x) { return Ok(true); }
    if is_float(x) { return Ok(float_val(x).is_finite()); }
    Ok(funcall(c, x, intern(b"finite?"), &[], nul())?.truthy())
}
fn f_infinite(c: &mut Ctx, x: Value) -> Result<bool, Esc> {
    if is_integer(x) || is_rational(x) { return Ok(false); }
    if is_float(x) { return Ok(float_val(x).is_infinite()); }
    Ok(funcall(c, x, intern(b"infinite?"), &[], nul())?.truthy())
}
fn cpx_finite_p(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let (re, im) = cpx_parts(s); Ok(Value::bool(f_finite(c, re)? && f_finite(c, im)?)) }
fn cpx_infinite_p(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let (re, im) = cpx_parts(s); Ok(if f_infinite(c, re)? || f_infinite(c, im)? { Value::fix(1) } else { NIL }) }
fn cpx_hash(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (re, im) = cpx_parts(s);
    let h = crate::astrust::hash_merge(crate::hash::key_hash(c, re)?, crate::hash::key_hash(c, im)? ^ 0x436f);
    Ok(Value::fix((h >> 2) as i64))
}
fn cpx_eql(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if !is_complex(o) { return Ok(FALSE); }
    let (re, im) = cpx_parts(s); let (or, oi) = cpx_parts(o);
    if real_class_of(re) != real_class_of(or) || real_class_of(im) != real_class_of(oi) { return Ok(FALSE); }
    cpx_eq(c, s, a, std::ptr::null())
}

fn format(c: &mut Ctx, s: Value, insp: bool) -> Result<String, Esc> {
    let (re, im) = cpx_parts(s);
    let mut out = if insp { inspect_str(c, re)? } else { let v = funcall(c, re, s::to_s, &[], nul())?; String::from_utf8_lossy(str_bytes(v)).into_owned() };
    let neg = f_signbit(c, im)?;
    out.push(if neg { '-' } else { '+' });
    let ai = f_abs(c, im)?;
    let t = if insp { inspect_str(c, ai)? } else { let v = funcall(c, ai, s::to_s, &[], nul())?; String::from_utf8_lossy(str_bytes(v)).into_owned() };
    out.push_str(&t);
    if !t.as_bytes().last().map(|b| b.is_ascii_digit()).unwrap_or(false) { out.push('*'); }
    out.push('i');
    Ok(out)
}
fn cpx_to_s(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let t = format(c, s, false)?; Ok(str_new(t.as_bytes())) }
fn cpx_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let t = format(c, s, true)?; Ok(str_new(format!("({})", t).as_bytes())) }

fn range_err_conv<T>(c: &mut Ctx, s: Value, into: &str) -> Result<T, Esc> {
    let t = format(c, s, false)?;
    err(c, vm().c_range_error, format!("can't convert {} into {}", t, into))
}
fn cpx_to_i(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (re, im) = cpx_parts(s);
    if !exact_zero(im) { return range_err_conv(c, s, "Integer"); }
    funcall(c, re, s::to_i, &[], nul())
}
fn cpx_to_f(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (re, im) = cpx_parts(s);
    if !exact_zero(im) { return range_err_conv(c, s, "Float"); }
    funcall(c, re, s::to_f, &[], nul())
}
fn cpx_to_r(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (re, im) = cpx_parts(s);
    if !(is_float(im) && float_val(im) == 0.0) && !exact_zero(im) { return range_err_conv(c, s, "Rational"); }
    funcall(c, re, intern(b"to_r"), &[], nul())
}
fn cpx_rationalize(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    let (re, im) = cpx_parts(s);
    if !(is_float(im) && float_val(im) == 0.0) && !exact_zero(im) { return range_err_conv(c, s, "Rational"); }
    funcall(c, re, intern(b"rationalize"), a, nul())
}
fn cpx_to_c(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(s) }
fn cpx_denominator(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (re, im) = cpx_parts(s);
    let a = funcall(c, re, intern(b"denominator"), &[], nul())?;
    let b = funcall(c, im, intern(b"denominator"), &[], nul())?;
    funcall(c, a, intern(b"lcm"), &[b], nul())
}
fn cpx_numerator(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (re, im) = cpx_parts(s);
    let cd = cpx_denominator(c, s, &[], nul())?;
    let rn = funcall(c, re, intern(b"numerator"), &[], nul())?; let rd = funcall(c, re, intern(b"denominator"), &[], nul())?;
    let q = funcall(c, cd, s::div, &[rd], nul())?; let r = f_mul(c, rn, q)?;
    let inn = funcall(c, im, intern(b"numerator"), &[], nul())?; let id = funcall(c, im, intern(b"denominator"), &[], nul())?;
    let q = funcall(c, cd, s::div, &[id], nul())?; let i = f_mul(c, inn, q)?;
    Ok(cpx_raw(r, i))
}

// ---- construction ---------------------------------------------------------------------------

fn s_rect(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let re = real_check(c, a[0])?;
    let im = if a.len() > 1 { real_check(c, a[1])? } else { Value::fix(0) };
    Ok(cpx_raw(re, im))
}
fn s_polar(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
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
    let v = if flo { float_new(txt.parse::<f64>().unwrap_or(0.0)) }
        else if rat { match parse_rational(c, txt.as_bytes(), true)? { Some(r) => r, None => return Ok(None) } }
        else { big_to_int(&txt) };
    Ok(Some(v))
}
fn big_to_int(t: &str) -> Value { crate::bignum::big_to_value(crate::bignum::BigInt::parse(t, 10).unwrap_or(crate::bignum::BigInt::zero())) }
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
    if let Some((sg, k)) = sign_unit(b, i) { return Ok((cpx_raw(zero, Value::fix(sg)), fin(k))); }
    let Some(n1) = read_real(c, b, &mut i)? else { return Ok((cpx_raw(zero, zero), false)) };
    if i < b.len() && is_imag_unit(b[i]) { i += 1; return Ok((cpx_raw(zero, n1), fin(i))); }
    if i < b.len() && b[i] == b'@' {
        let mut k = i + 1;
        let Some(n2) = read_real(c, b, &mut k)? else { return Ok((cpx_raw(n1, zero), false)) };
        let r = polar(c, n1, n2)?;
        return Ok((r, fin(k)));
    }
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        if let Some((sg, k)) = sign_unit(b, i) { return Ok((cpx_raw(n1, Value::fix(sg)), fin(k))); }
        let mut k = i;
        let Some(n2) = read_real(c, b, &mut k)? else { return Ok((cpx_raw(n1, zero), false)) };
        if k < b.len() && is_imag_unit(b[k]) { return Ok((cpx_raw(n1, n2), fin(k + 1))); }
        return Ok((cpx_raw(n1, zero), false));
    }
    Ok((cpx_raw(n1, zero), fin(i)))
}
fn str_to_c(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let b = str_bytes(s).to_vec();
    Ok(parse_complex(c, &b)?.0)
}
fn str_to_c_strict(c: &mut Ctx, v: Value, raise: bool) -> RV {
    let b = str_bytes(v).to_vec();
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
    if is_complex(a1) { let (re, im) = cpx_parts(a1); if exact_zero(im) { a1 = re; } }
    if let Some(x) = a2 { if is_complex(x) { let (re, im) = cpx_parts(x); if exact_zero(im) { a2 = Some(re); } } }
    if is_complex(a1) && a2.map(|x| exact_zero(x)).unwrap_or(true) { return Ok(a1); }
    match a2 {
        None => {
            if k_numeric(a1) && !f_real_p(c, a1)? { return Ok(a1); }
            if !k_numeric(a1) {
                let r: RV = (|| {
                    if a1.is_heap() && respond_to(a1, intern(b"to_c"), true) {
                        let r = funcall(c, a1, intern(b"to_c"), &[], nul())?;
                        if is_complex(r) { return Ok(r); }
                        let cn = class_name_for_err(a1);
                        return type_err(c, format!("can't convert {} to Complex ({}#to_c gives {})", cn, cn, class_name_for_err(r)));
                    }
                    type_err(c, format!("can't convert {} into Complex", class_name_for_err(a1)))
                })();
                return match r { Err(e) => { if !raise && c.esc == EscKind::Raise { c.esc = EscKind::None; return Ok(NIL); } Err(e) } ok => ok };
            }
        }
        Some(b2) => {
            if k_numeric(a1) && k_numeric(b2) && (!f_real_p(c, a1)? || !f_real_p(c, b2)?) {
                let iu = cpx_raw(Value::fix(0), Value::fix(1));
                let m = funcall(c, b2, s::mul, &[iu], nul())?;
                return funcall(c, a1, s::plus, &[m], nul());
            }
        }
    }
    let r: RV = (|| {
        let re = real_check(c, a1)?;
        let im = match a2 { Some(x) => real_check(c, x)?, None => Value::fix(0) };
        canonicalize(c, re, im)
    })();
    match r { Err(e) => { if !raise && c.esc == EscKind::Raise { c.esc = EscKind::None; return Ok(NIL); } Err(e) } ok => ok }
}
fn k_complex(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (n, raise) = kw_exception(c, a)?;
    let a = &a[..n];
    check_argc(c, a, 1, 2)?;
    convert(c, a[0], a.get(1).copied(), raise)
}

// ---- Numeric side ---------------------------------------------------------------------------

fn num_to_c(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(cpx_raw(s, Value::fix(0))) }
fn num_i(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(cpx_raw(Value::fix(0), s)) }
fn num_polar(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let a = funcall(c, s, intern(b"abs"), &[], nul())?;
    let t = funcall(c, s, intern(b"arg"), &[], nul())?;
    Ok(ary_new_from(&[a, t]))
}
fn num_abs2(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { funcall(c, s, s::mul, &[s], nul()) }
fn flo_arg(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let f = float_val(s);
    if f.is_nan() { return Ok(s); }
    Ok(if f.is_sign_negative() { float_new(std::f64::consts::PI) } else { Value::fix(0) })
}
fn nil_to_c(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(cpx_raw(Value::fix(0), Value::fix(0))) }

pub fn init() {
    let vm = vm();
    let k = define_class("Complex", vm.c_numeric);
    rclass(k).alloc = AllocKind::None;
    vm.c_complex = k;
    let sk = singleton_class(k);
    define_method_me(sk, s::new, new_me(s::new, sk, VISI_PUBLIC, MKind::Undef));
    define_singleton_cfunc(k, "rect", s_rect, -1);
    define_singleton_cfunc(k, "rectangular", s_rect, -1);
    define_singleton_cfunc(k, "polar", s_polar, -1);
    let d = |nm: &str, f: CFunc, ar: i32| define_cfunc(k, nm, f, ar);
    d("+", cpx_plus, 1); d("-", cpx_minus, 1); d("*", cpx_mul, 1); d("/", cpx_div, 1); d("quo", cpx_div, 1);
    d("fdiv", cpx_fdiv, 1); d("**", cpx_pow, 1); d("-@", cpx_uminus, 0); d("==", cpx_eq, 1); d("<=>", cpx_cmp, 1);
    d("coerce", cpx_coerce, 1); d("real", cpx_real, 0); d("imaginary", cpx_imag, 0); d("imag", cpx_imag, 0);
    d("abs", cpx_abs, 0); d("magnitude", cpx_abs, 0); d("abs2", cpx_abs2, 0); d("arg", cpx_arg, 0); d("angle", cpx_arg, 0);
    d("phase", cpx_arg, 0); d("rectangular", cpx_rect, 0); d("rect", cpx_rect, 0); d("polar", cpx_polar_m, 0);
    d("conjugate", cpx_conj, 0); d("conj", cpx_conj, 0); d("real?", cpx_real_p, 0); d("finite?", cpx_finite_p, 0);
    d("infinite?", cpx_infinite_p, 0); d("hash", cpx_hash, 0); d("eql?", cpx_eql, 1); d("to_s", cpx_to_s, 0);
    d("inspect", cpx_inspect, 0); d("to_i", cpx_to_i, 0); d("to_f", cpx_to_f, 0); d("to_r", cpx_to_r, 0);
    d("rationalize", cpx_rationalize, -1); d("to_c", cpx_to_c, 0); d("numerator", cpx_numerator, 0);
    d("denominator", cpx_denominator, 0);
    for nm in ["%", "<", "<=", ">", ">=", "between?", "clamp", "div", "divmod", "floor", "ceil", "modulo", "remainder",
               "round", "step", "truncate", "i", "positive?", "negative?"] {
        let id = intern(nm.as_bytes());
        define_method_me(k, id, new_me(id, k, VISI_PUBLIC, MKind::Undef));
    }
    const_set(k, intern(b"I"), cpx_raw(Value::fix(0), Value::fix(1)));
    let n = vm.c_numeric;
    define_cfunc(n, "to_c", num_to_c, 0);
    define_cfunc(n, "i", num_i, 0);
    define_cfunc(n, "polar", num_polar, 0);
    define_cfunc(n, "abs2", num_abs2, 0);
    define_cfunc(vm.c_float, "arg", flo_arg, 0);
    define_cfunc(vm.c_float, "angle", flo_arg, 0);
    define_cfunc(vm.c_float, "phase", flo_arg, 0);
    define_cfunc(vm.c_nil, "to_c", nil_to_c, 0);
    define_cfunc(vm.c_string, "to_c", str_to_c, 0);
    define_module_function(vm.m_kernel, "Complex", k_complex, -1);
}
