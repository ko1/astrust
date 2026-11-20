// Math.
use crate::builtins::numeric_methods::to_f;
use crate::numeric::float_new;
use crate::value::*;
use crate::vm::*;

macro_rules! m1 { ($name:ident, $f:expr, $dom:expr, $nm:expr) => {
    fn $name(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
        let x = to_f(c, a[0])?;
        if !$dom(x) { return err(c, vm().c_math_domain_error, format!("Numerical argument is out of domain - \"{}\"", $nm)); }
        Ok(float_new($f(x)))
    }
}; }
fn any(_: f64) -> bool { true }
m1!(m_sin, f64::sin, any, "sin"); m1!(m_cos, f64::cos, any, "cos"); m1!(m_tan, f64::tan, any, "tan");
m1!(m_asin, f64::asin, |x: f64| (-1.0..=1.0).contains(&x) || x.is_nan(), "asin"); m1!(m_acos, f64::acos, |x: f64| (-1.0..=1.0).contains(&x) || x.is_nan(), "acos");
m1!(m_atan, f64::atan, any, "atan"); m1!(m_exp, f64::exp, any, "exp"); m1!(m_sqrt, f64::sqrt, |x: f64| x >= 0.0 || x.is_nan(), "sqrt");
m1!(m_cbrt, f64::cbrt, any, "cbrt"); m1!(m_sinh, f64::sinh, any, "sinh"); m1!(m_cosh, f64::cosh, any, "cosh"); m1!(m_tanh, f64::tanh, any, "tanh");
m1!(m_asinh, f64::asinh, any, "asinh"); m1!(m_acosh, f64::acosh, |x: f64| x >= 1.0 || x.is_nan(), "acosh"); m1!(m_atanh, f64::atanh, |x: f64| (-1.0..=1.0).contains(&x) || x.is_nan(), "atanh");
m1!(m_log2, f64::log2, |x: f64| x >= 0.0 || x.is_nan(), "log2"); m1!(m_log10, f64::log10, |x: f64| x >= 0.0 || x.is_nan(), "log10");
m1!(m_erf, erf, any, "erf"); m1!(m_erfc, |x| 1.0 - erf(x), any, "erfc");
m1!(m_gamma, gamma, |x: f64| !(x < 0.0 && x.fract() == 0.0) && x != f64::NEG_INFINITY, "gamma");
fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.5 * x.abs());
    let y = 1.0 - t * (-x * x - 1.26551223 + t * (1.00002368 + t * (0.37409196 + t * (0.09678418 + t * (-0.18628806 + t * (0.27886807 + t * (-1.13520398 + t * (1.48851587 + t * (-0.82215223 + t * 0.17087277))))))))).exp();
    if x >= 0.0 { y } else { -y }
}
fn gamma(x: f64) -> f64 {
    if x.fract() == 0.0 && x > 0.0 && x < 24.0 { let mut r = 1.0; let mut i = 2.0; while i < x { r *= i; i += 1.0; } return r; }
    if x < 0.5 { return std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma(1.0 - x)); }
    let g = 7.0;
    let cs = [0.99999999999980993, 676.5203681218851, -1259.1392167224028, 771.32342877765313, -176.61502916214059, 12.507343278686905, -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7];
    let x = x - 1.0;
    let mut a = cs[0];
    let t = x + g + 0.5;
    for (i, &cc) in cs.iter().enumerate().skip(1) { a += cc / (x + i as f64); }
    (2.0 * std::f64::consts::PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * a
}
fn m_log(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let x = if crate::object::is_bignum(a[0]) { let b = crate::object::rbig(a[0]); if b.neg { return err(c, vm().c_math_domain_error, "Numerical argument is out of domain - \"log\""); } let bl = b.bit_length(); if bl > 1000 { let sh = b.shr(bl - 60); let l = sh.to_f64().ln() + ((bl - 60) as f64) * std::f64::consts::LN_2; return Ok(float_new(if a.len() > 1 { l / to_f(c, a[1])?.ln() } else { l })); } b.to_f64() } else { to_f(c, a[0])? };
    if x < 0.0 { return err(c, vm().c_math_domain_error, "Numerical argument is out of domain - \"log\""); }
    let r = x.ln();
    if a.len() > 1 { let b = to_f(c, a[1])?; return Ok(float_new(r / b.ln())); }
    Ok(float_new(r))
}
fn m_atan2(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let y = to_f(c, a[0])?; let x = to_f(c, a[1])?; Ok(float_new(y.atan2(x))) }
fn m_hypot(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let y = to_f(c, a[0])?; let x = to_f(c, a[1])?; Ok(float_new(y.hypot(x))) }
fn m_pow_ldexp(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let y = to_f(c, a[0])?; let x = crate::builtins::numeric_methods::num_to_long(c, a[1])?; Ok(float_new(y * 2f64.powi(x as i32))) }
fn m_frexp(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let x = to_f(c, a[0])?;
    if x == 0.0 || !x.is_finite() { return Ok(crate::array::ary_new_from(&[float_new(x), Value::fix(0)])); }
    let e = x.abs().log2().floor() as i32 + 1;
    Ok(crate::array::ary_new_from(&[float_new(x / 2f64.powi(e)), Value::fix(e as i64)]))
}
fn m_lgamma(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let x = to_f(c, a[0])?;
    let g = gamma(x);
    Ok(crate::array::ary_new_from(&[float_new(g.abs().ln()), Value::fix(if g < 0.0 { -1 } else { 1 })]))
}

pub fn init() {
    let m = vm().m_math;
    let d = |n: &str, f: CFunc, a: i32| define_module_function(m, n, f, a);
    d("sin", m_sin, 1); d("cos", m_cos, 1); d("tan", m_tan, 1); d("asin", m_asin, 1); d("acos", m_acos, 1); d("atan", m_atan, 1);
    d("exp", m_exp, 1); d("sqrt", m_sqrt, 1); d("cbrt", m_cbrt, 1); d("sinh", m_sinh, 1); d("cosh", m_cosh, 1); d("tanh", m_tanh, 1);
    d("asinh", m_asinh, 1); d("acosh", m_acosh, 1); d("atanh", m_atanh, 1); d("log2", m_log2, 1); d("log10", m_log10, 1); d("log", m_log, -1);
    d("atan2", m_atan2, 2); d("hypot", m_hypot, 2); d("ldexp", m_pow_ldexp, 2); d("frexp", m_frexp, 1); d("erf", m_erf, 1); d("erfc", m_erfc, 1);
    d("gamma", m_gamma, 1); d("lgamma", m_lgamma, 1);
    const_set(m, crate::symbol::intern(b"PI"), float_new(std::f64::consts::PI));
    const_set(m, crate::symbol::intern(b"E"), float_new(std::f64::consts::E));
}
