// Math, on the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::builtins::kernel::class_name_for_err;
use crate::value::*;
use crate::vm::*;

/// rb_num_to_dbl (Get_Double): Integers and Floats directly, other Numerics by #to_f, the rest a TypeError.
pub fn num_to_dbl(c: &mut Ctx, v: Value) -> Result<f64, Esc> {
    if let Some(f) = c.num_to_f64(v) { return Ok(f); }
    if !v.is_heap() || !is_kind_of(c, v, c.vm().c_numeric.get()) {
        return type_err(c, format!("can't convert {} into Float", class_name_for_err(c, v)));
    }
    let r = funcall(c, v, crate::symbol::s::to_f, &[], std::ptr::null())?;
    if let Some(f) = c.num_to_f64(r).filter(|_| crate::object::is_float(r)) { return Ok(f); }
    let cls = class_name_for_err(c, v);
    type_err(c, format!("can't convert {} to Float ({}#to_f gives {})", cls, cls, class_name_for_err(c, r)))
}
pub fn domain_err<T>(c: &mut Ctx, nm: &str) -> Result<T, Esc> { err(c, c.vm().c_math_domain_error.get(), format!("Numerical argument is out of domain - {}", nm)) }
/// get_double_rshift: a huge positive Bignum as (its top bits as a double, the bits shifted out)
fn dbl_rshift(c: &mut Ctx, v: Value) -> Result<(f64, i64), Esc> {
    if crate::object::is_bignum(v) && !c.big(v).neg {
        let n = c.big(v).bit_length() as i64;
        if n >= 1024 { let sh = n - 53; return Ok((c.big(v).shr(sh as usize).to_f64(), sh)); }
    }
    Ok((num_to_dbl(c, v)?, 0))
}

macro_rules! m1 { ($name:ident, $f:expr, $dom:expr, $nm:expr) => {
    fn $name(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
        let x = num_to_dbl(c, a[0])?;
        if !$dom(x) { return domain_err(c, $nm); }
        Ok(c.float_new($f(x)))
    }
}; }
fn any(_: f64) -> bool { true }
m1!(m_sin, f64::sin, any, "sin"); m1!(m_cos, f64::cos, any, "cos"); m1!(m_tan, f64::tan, any, "tan");
m1!(m_asin, f64::asin, |x: f64| (-1.0..=1.0).contains(&x) || x.is_nan(), "asin"); m1!(m_acos, f64::acos, |x: f64| (-1.0..=1.0).contains(&x) || x.is_nan(), "acos");
m1!(m_atan, f64::atan, any, "atan"); m1!(m_exp, f64::exp, any, "exp"); m1!(m_sqrt, f64::sqrt, |x: f64| x >= 0.0 || x.is_nan(), "sqrt");
m1!(m_cbrt, f64::cbrt, any, "cbrt"); m1!(m_sinh, f64::sinh, any, "sinh"); m1!(m_cosh, f64::cosh, any, "cosh"); m1!(m_tanh, f64::tanh, any, "tanh");
m1!(m_asinh, f64::asinh, any, "asinh"); m1!(m_acosh, f64::acosh, |x: f64| x >= 1.0 || x.is_nan(), "acosh"); m1!(m_atanh, f64::atanh, |x: f64| (-1.0..=1.0).contains(&x) || x.is_nan(), "atanh");
fn m_log2(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (d, n) = dbl_rshift(c, a[0])?;
    if d < 0.0 { return domain_err(c, "log2"); }
    Ok(c.float_new(if d == 0.0 { f64::NEG_INFINITY } else { d.log2() + n as f64 }))
}
fn m_log10(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (d, n) = dbl_rshift(c, a[0])?;
    if d < 0.0 { return domain_err(c, "log10"); }
    Ok(c.float_new(if d == 0.0 { f64::NEG_INFINITY } else { d.log10() + n as f64 * 2f64.log10() }))
}
/// math_log_split
fn log_split(c: &mut Ctx, v: Value) -> Result<(f64, i64), Esc> {
    let (d, n) = dbl_rshift(c, v)?;
    if d < 0.0 { return domain_err(c, "log"); }
    Ok((d, n))
}
fn m_log(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let (d, n) = log_split(c, a[0])?;
    if a.len() == 2 {
        let (b, n2) = log_split(c, a[1])?;
        if d == 0.0 { return Ok(c.float_new(if b != 0.0 { f64::NEG_INFINITY } else { f64::NAN })); }
        if b == 0.0 { return Ok(c.float_new(-0.0)); }
        return Ok(c.float_new(d.log2() / b.log2() + (n - n2) as f64 / b.log2()));
    }
    Ok(c.float_new(if d == 0.0 { f64::NEG_INFINITY } else { d.ln() + n as f64 * std::f64::consts::LN_2 }))
}
pub fn m_log1p(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (d, n) = dbl_rshift(c, a[0])?;
    if n != 0 {
        let x1 = crate::builtins::numeric_methods::int_plus(c, a[0], &[Value::fix(1)], Blk::NONE)?;
        let (d, n) = log_split(c, x1)?;
        return Ok(c.float_new(if d == 0.0 { f64::NEG_INFINITY } else { d.ln() + n as f64 * std::f64::consts::LN_2 }));
    }
    if d < -1.0 { return domain_err(c, "log1p"); }
    Ok(c.float_new(if d == -1.0 { f64::NEG_INFINITY } else { d.ln_1p() }))
}
fn m_atan2(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { let y = num_to_dbl(c, a[0])?; let x = num_to_dbl(c, a[1])?; Ok(c.float_new(y.atan2(x))) }
fn m_hypot(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { let y = num_to_dbl(c, a[0])?; let x = num_to_dbl(c, a[1])?; Ok(c.float_new(y.hypot(x))) }
fn m_ldexp(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let y = num_to_dbl(c, a[0])?;
    let x = crate::builtins::numeric_methods::num_to_int32(c, a[1])?;
    Ok(c.float_new(crate::bignum::ldexp(y, x as i32)))
}
fn m_frexp(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let x = num_to_dbl(c, a[0])?;
    if x == 0.0 || !x.is_finite() { let f = c.float_new(x); return Ok(c.ary_new_from(&[f, Value::fix(0)])); }
    let e = x.abs().log2().floor() as i32 + 1;
    let f = c.float_new(x / 2f64.powi(e));
    Ok(c.ary_new_from(&[f, Value::fix(e as i64)]))
}

/// Defines the functions and constants of Math `m`.
pub fn init(c: &mut Ctx, m: Value) {
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_module_function(c, m, n, f, a);
    d(c, "sin", m_sin, 1); d(c, "cos", m_cos, 1); d(c, "tan", m_tan, 1); d(c, "asin", m_asin, 1); d(c, "acos", m_acos, 1); d(c, "atan", m_atan, 1);
    d(c, "exp", m_exp, 1); d(c, "sqrt", m_sqrt, 1); d(c, "cbrt", m_cbrt, 1); d(c, "sinh", m_sinh, 1); d(c, "cosh", m_cosh, 1); d(c, "tanh", m_tanh, 1);
    d(c, "asinh", m_asinh, 1); d(c, "acosh", m_acosh, 1); d(c, "atanh", m_atanh, 1); d(c, "log2", m_log2, 1); d(c, "log10", m_log10, 1); d(c, "log", m_log, -1);
    d(c, "atan2", m_atan2, 2); d(c, "hypot", m_hypot, 2); d(c, "ldexp", m_ldexp, 2); d(c, "frexp", m_frexp, 1); d(c, "log1p", m_log1p, 1);
    // both are flonums: nothing is allocated
    for (nm, v) in [("PI", std::f64::consts::PI), ("E", std::f64::consts::E)] {
        const_set(c, m, crate::symbol::intern(nm.as_bytes()), Value::flonum(v).expect("a flonum"));
    }
}
