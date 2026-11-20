// Numeric / Integer / Float.
use crate::array::*;
use crate::bignum::*;
use crate::builtins::kernel::*;
use crate::numeric::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

pub fn f64_to_int(f: f64) -> Value {
    let t = f.trunc();
    if t.abs() < 4.0e18 { int_from_i64(t as i64) } else { big_to_value(BigInt::from_f64(t)) }
}

/// Integer-like argument to i64 (to_int conversion; Float truncated).
pub fn num_to_long(c: &mut Ctx, v: Value) -> Result<i64, Esc> {
    if v.is_fixnum() { return Ok(v.as_fix()); }
    if is_bignum(v) { return match rbig(v).to_i64() { Some(i) => Ok(i), None => err(c, vm().c_range_error, format!("bignum too big to convert into 'long'")) }; }
    if is_float(v) {
        let f = float_val(v);
        if f.is_nan() || f.is_infinite() { return err(c, vm().c_float_domain_error, float_to_s(f)); }
        if f.abs() > 9.2e18 { return err(c, vm().c_range_error, format!("float {} out of range of integer", float_to_s(f))); }
        return Ok(f as i64);
    }
    if v == NIL { return type_err(c, "no implicit conversion from nil to integer"); }
    if v.is_heap() && respond_to(v, s::to_int, true) {
        let r = funcall(c, v, s::to_int, &[], std::ptr::null())?;
        if r.is_fixnum() { return Ok(r.as_fix()); }
        if is_bignum(r) { return num_to_long(c, r); }
        let cls = class_name_for_err(v);
        return type_err(c, format!("can't convert {} to Integer ({}#to_int gives {})", cls, cls, class_name_for_err(r)));
    }
    type_err(c, format!("no implicit conversion of {} into Integer", class_name_for_err(v)))
}

pub fn to_integer(c: &mut Ctx, v: Value) -> RV {
    if is_integer(v) { return Ok(v); }
    if is_float(v) { return Ok(f64_to_int(float_val(v))); }
    if v.is_heap() && respond_to(v, s::to_int, true) {
        let r = funcall(c, v, s::to_int, &[], std::ptr::null())?;
        if is_integer(r) { return Ok(r); }
    }
    type_err(c, format!("no implicit conversion of {} into Integer", class_name_for_err(v)))
}

pub fn to_f(c: &mut Ctx, v: Value) -> Result<f64, Esc> {
    if let Some(f) = num_to_f64(v) { return Ok(f); }
    if v == NIL { return type_err(c, "can't convert nil into Float"); }
    if v.is_heap() && respond_to(v, s::to_f, true) && !is_string(v) {
        let r = funcall(c, v, s::to_f, &[], std::ptr::null())?;
        if let Some(f) = num_to_f64(r) { return Ok(f); }
    }
    type_err(c, format!("can't convert {} into Float", class_name_for_err(v)))
}

fn coerce_fail<T>(c: &mut Ctx, a: Value, b: Value, cmp: bool) -> Result<T, Esc> {
    let bdesc = if b == NIL || b == TRUE || b == FALSE { class_name_for_err(b) } else if is_float(b) || b.is_fixnum() { inspect_str(c, b)? } else { class_path(real_class_of(b)) };
    if cmp { let bs = if b == NIL { "nil".into() } else { inspect_str(c, b).unwrap_or(bdesc) }; return arg_err(c, format!("comparison of {} with {} failed", class_path(real_class_of(a)), bs)); }
    type_err(c, format!("{} can't be coerced into {}", bdesc, class_path(real_class_of(a))))
}

/// a op b via b.coerce(a)
pub fn coerce_bin(c: &mut Ctx, a: Value, b: Value, op: Sym, cmp: bool) -> RV {
    if b.is_heap() && respond_to(b, s::coerce, true) && !is_string(b) {
        let r = funcall(c, b, s::coerce, &[a], std::ptr::null())?;
        if !is_array(r) || ary_len(r) != 2 { return type_err(c, "coerce must return [x, y]"); }
        let x = ary_get(r, 0); let y = ary_get(r, 1);
        return funcall(c, x, op, &[y], std::ptr::null());
    }
    coerce_fail(c, a, b, cmp)
}

#[derive(Clone, Copy, PartialEq)]
enum Op { Add, Sub, Mul, Div, Mod, Pow }

fn int_arith(c: &mut Ctx, a: Value, b: Value, op: Op) -> RV {
    if a.is_fixnum() && b.is_fixnum() {
        let (x, y) = (a.as_fix() as i128, b.as_fix() as i128);
        let r = match op {
            Op::Add => x + y, Op::Sub => x - y, Op::Mul => x * y,
            Op::Div => { if y == 0 { return err(c, vm().c_zero_div_error, "divided by 0"); } let q = x / y; if (x % y != 0) && ((x < 0) != (y < 0)) { q - 1 } else { q } }
            Op::Mod => { if y == 0 { return err(c, vm().c_zero_div_error, "divided by 0"); } let m = x % y; if m != 0 && ((m < 0) != (y < 0)) { m + y } else { m } }
            Op::Pow => return int_pow(c, a, b),
        };
        return Ok(int_from_i128(r));
    }
    if is_integer(b) {
        let (x, y) = (to_big(a), to_big(b));
        return Ok(match op {
            Op::Add => big_to_value(x.add(&y)), Op::Sub => big_to_value(x.sub(&y)), Op::Mul => big_to_value(x.mul(&y)),
            Op::Div => { if y.is_zero() { return err(c, vm().c_zero_div_error, "divided by 0"); } big_to_value(x.divmod(&y).0) }
            Op::Mod => { if y.is_zero() { return err(c, vm().c_zero_div_error, "divided by 0"); } big_to_value(x.divmod(&y).1) }
            Op::Pow => return int_pow(c, a, b),
        });
    }
    if is_float(b) {
        if op == Op::Pow { return int_pow(c, a, b); }
        let x = num_to_f64(a).unwrap(); let y = float_val(b);
        if op == Op::Mod && y == 0.0 { return err(c, vm().c_zero_div_error, "divided by 0"); }
        return Ok(float_new(match op { Op::Add => x + y, Op::Sub => x - y, Op::Mul => x * y, Op::Div => x / y, Op::Mod => fmod_ruby(x, y), Op::Pow => unreachable!() }));
    }
    // fix_plus / fix_mul compute Complex op Fixnum directly (keeps exact parts)
    if a.is_fixnum() && (op == Op::Add || op == Op::Mul) && crate::builtins::complex::is_complex(b) {
        return funcall(c, b, if op == Op::Add { s::plus } else { s::mul }, &[a], std::ptr::null());
    }
    let sym = match op { Op::Add => s::plus, Op::Sub => s::minus, Op::Mul => s::mul, Op::Div => s::div, Op::Mod => s::modulo, Op::Pow => s::pow };
    coerce_bin(c, a, b, sym, false)
}

pub fn fmod_ruby(x: f64, y: f64) -> f64 {
    if y.is_infinite() && x.is_finite() { if x == 0.0 || (x > 0.0) == (y > 0.0) { return x; } return y; }
    let m = x % y;
    if m != 0.0 && ((m < 0.0) != (y < 0.0)) { m + y } else { m }
}

fn int_pow(c: &mut Ctx, a: Value, b: Value) -> RV {
    if is_float(b) {
        let x = num_to_f64(a).unwrap(); let y = float_val(b);
        if y == 0.0 { return Ok(float_new(1.0)); }
        if x < 0.0 && y != y.round() { return Ok(crate::builtins::complex::polar_pi((-x).powf(y), y)); }
        return Ok(float_new(x.powf(y)));
    }
    if !is_integer(b) { return coerce_bin(c, a, b, s::pow, false); }
    let e = to_big(b);
    if e.neg {
        if let Some(_) = const_get_at(vm().c_object, intern(b"Rational")) {
            let r = funcall(c, vm().main_obj, intern(b"Rational"), &[a, Value::fix(1)], std::ptr::null())?;
            return funcall(c, r, s::pow, &[b], std::ptr::null());
        }
        return Ok(float_new(num_to_f64(a).unwrap().powf(num_to_f64(b).unwrap())));
    }
    let x = to_big(a);
    if x.mag.len() <= 1 && x.mag.first().copied().unwrap_or(0) <= 1 {
        if x.is_zero() { return Ok(Value::fix(if e.is_zero() { 1 } else { 0 })); }
        if !x.neg { return Ok(Value::fix(1)); }
        return Ok(Value::fix(if e.is_odd() { -1 } else { 1 }));
    }
    let ei = match e.to_i64() { Some(i) => i as u64, None => { return arg_err(c, "exponent is too large"); } };
    if (x.bit_length() as u64).saturating_mul(ei) > 1u64 << 34 {
        return arg_err(c, "exponent is too large");
    }
    if a.is_fixnum() {
        if let Ok(ei32) = u32::try_from(ei) { if let Some(r) = a.as_fix().checked_pow(ei32) { return Ok(int_from_i64(r)); } }
    }
    Ok(big_to_value(x.pow(ei)))
}

fn int_cmp_val(c: &mut Ctx, a: Value, b: Value) -> Result<Option<std::cmp::Ordering>, Esc> {
    if a.is_fixnum() && b.is_fixnum() { return Ok(Some(a.as_fix().cmp(&b.as_fix()))); }
    if is_integer(b) { return Ok(Some(to_big(a).cmp(&to_big(b)))); }
    if is_float(b) {
        let y = float_val(b);
        if y.is_nan() { return Ok(None); }
        if y.is_infinite() { return Ok(Some(if y > 0.0 { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater })); }
        let x = num_to_f64(a).unwrap();
        if x != y { return Ok(x.partial_cmp(&y)); }
        // exact comparison for big values
        return Ok(Some(to_big(a).cmp(&BigInt::from_f64(y)).then_with(|| if y.fract() > 0.0 { std::cmp::Ordering::Less } else if y.fract() < 0.0 { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Equal })));
    }
    let _ = c;
    Ok(None)
}

macro_rules! cfn { ($name:ident, |$c:ident, $s:ident, $a:ident| $body:expr) => {
    fn $name($c: &mut Ctx, $s: Value, $a: &[Value], _b: *const Block) -> RV { $body }
}; }

cfn!(int_plus, |c, s, a| int_arith(c, s, a[0], Op::Add));
cfn!(int_minus, |c, s, a| int_arith(c, s, a[0], Op::Sub));
cfn!(int_mul, |c, s, a| int_arith(c, s, a[0], Op::Mul));
cfn!(int_div, |c, s, a| int_arith(c, s, a[0], Op::Div));
cfn!(int_mod, |c, s, a| int_arith(c, s, a[0], Op::Mod));
cfn!(int_pow_m, |c, s, a| {
    if a.len() == 2 {
        // modular exponentiation
        let m = to_big(a[1]);
        if m.is_zero() { return err(c, vm().c_zero_div_error, "divided by 0"); }
        let mut base = to_big(s).divmod(&m).1;
        let mut e = to_big(a[0]);
        if e.neg { return err(c, vm().c_range_error, "Integer#pow() 2nd argument not allowed to be negative when 3rd argument specified"); }
        let mut r = BigInt::from_i64(1).divmod(&m).1;
        while !e.is_zero() {
            if e.is_odd() { r = r.mul(&base).divmod(&m).1; }
            e = e.shr(1);
            base = base.mul(&base).divmod(&m).1;
        }
        return Ok(big_to_value(r));
    }
    int_arith(c, s, a[0], Op::Pow)
});
cfn!(int_uminus, |_c, s, _a| Ok(if s.is_fixnum() { int_from_i128(-(s.as_fix() as i128)) } else { big_to_value(rbig(s).neg()) }));
cfn!(int_uplus, |_c, s, _a| Ok(s));
cfn!(int_not, |_c, s, _a| Ok(if s.is_fixnum() { Value::fix(!s.as_fix()) } else { big_to_value(rbig(s).not()) }));
fn int_bitop(c: &mut Ctx, a: Value, b: Value, op: u8) -> RV {
    if a.is_fixnum() && b.is_fixnum() {
        let (x, y) = (a.as_fix(), b.as_fix());
        return Ok(Value::fix(match op { b'&' => x & y, b'|' => x | y, _ => x ^ y }));
    }
    if is_integer(b) { return Ok(big_to_value(to_big(a).bitop(&to_big(b), op))); }
    if is_float(b) { return type_err(c, "can't convert Float into Integer for bitwise arithmetic"); }
    let sym = match op { b'&' => s::and, b'|' => s::or, _ => s::xor };
    coerce_bin(c, a, b, sym, false)
}
cfn!(int_and, |c, s, a| int_bitop(c, s, a[0], b'&'));
cfn!(int_or, |c, s, a| int_bitop(c, s, a[0], b'|'));
cfn!(int_xor, |c, s, a| int_bitop(c, s, a[0], b'^'));
fn int_shift(c: &mut Ctx, a: Value, n: i64) -> RV {
    if n >= 0 {
        if a.is_fixnum() && n < 62 {
            let x = a.as_fix() as i128;
            return Ok(int_from_i128(x << n));
        }
        if n > 1 << 26 { return err(c, vm().c_range_error, "shift width too big"); }
        Ok(big_to_value(to_big(a).shl(n as usize)))
    } else {
        let m = (-n) as u64;
        if a.is_fixnum() { return Ok(Value::fix(if m >= 63 { if a.as_fix() < 0 { -1 } else { 0 } } else { a.as_fix() >> m })); }
        Ok(big_to_value(to_big(a).shr(m.min(1 << 30) as usize)))
    }
}
cfn!(int_lshift, |c, s, a| { let n = num_to_long(c, a[0])?; int_shift(c, s, n) });
cfn!(int_rshift, |c, s, a| { let n = num_to_long(c, a[0])?; int_shift(c, s, -n) });
cfn!(int_aref, |c, s, a| {
    if a.len() == 2 || is_range(a[0]) {
        let (start, len) = if a.len() == 2 { (num_to_long(c, a[0])?, Some(num_to_long(c, a[1])?)) } else {
            let (b, e, x) = crate::range::range_parts(a[0]);
            let b = if b == NIL { 0 } else { num_to_long(c, b)? };
            if e == NIL { (b, None) } else { let e = num_to_long(c, e)?; (b, Some(e - b + if x { 0 } else { 1 })) }
        };
        let shifted = int_shift(c, s, -start)?;
        return match len { None => Ok(shifted), Some(l) => {
            if l <= 0 { return Ok(Value::fix(0)); }
            let mask = big_to_value(BigInt::from_i64(1).shl(l as usize).sub(&BigInt::from_i64(1)));
            int_bitop(c, shifted, mask, b'&')
        } };
    }
    let i = if is_bignum(a[0]) { if rbig(a[0]).neg { return Ok(Value::fix(0)); } else { return Ok(Value::fix(if to_big(s).neg { 1 } else { 0 })); } } else { num_to_long(c, a[0])? };
    if i < 0 { return Ok(Value::fix(0)); }
    if s.is_fixnum() { return Ok(Value::fix(if i >= 63 { if s.as_fix() < 0 { 1 } else { 0 } } else { (s.as_fix() >> i) & 1 })); }
    Ok(Value::fix(rbig(s).bit(i as usize) as i64))
});
cfn!(int_eq, |c, s, a| {
    let b = a[0];
    if s.is_fixnum() && b.is_fixnum() { return Ok(Value::bool(s == b)); }
    if is_integer(b) { return Ok(Value::bool(to_big(s) == to_big(b))); }
    if is_float(b) { return Ok(Value::bool(int_cmp_val(c, s, b)? == Some(std::cmp::Ordering::Equal))); }
    let r = funcall(c, b, s::eq, &[s], std::ptr::null())?;
    Ok(Value::bool(r.truthy()))
});
cfn!(int_eql, |_c, s, a| Ok(Value::bool(is_integer(a[0]) && to_big(s) == to_big(a[0]))));
cfn!(int_cmp, |c, s, a| {
    match int_cmp_val(c, s, a[0])? {
        Some(o) => Ok(Value::fix(o as i64)),
        None => {
            if a[0].is_heap() && respond_to(a[0], s::coerce, true) && !is_string(a[0]) {
                let r = funcall(c, a[0], s::coerce, &[s], std::ptr::null());
                match r { Ok(r) if is_array(r) && ary_len(r) == 2 => { let x = ary_get(r, 0); return funcall(c, x, s::cmp, &[ary_get(r, 1)], std::ptr::null()); }
                          Ok(_) => return Ok(NIL), Err(_) => { c.esc = EscKind::None; return Ok(NIL); } }
            }
            Ok(NIL)
        }
    }
});
fn int_rel(c: &mut Ctx, s: Value, b: Value, f: fn(std::cmp::Ordering) -> bool, op: Sym) -> RV {
    match int_cmp_val(c, s, b)? {
        Some(o) => Ok(Value::bool(f(o))),
        None => if is_float(b) { Ok(FALSE) } else { coerce_bin(c, s, b, op, true) },
    }
}
cfn!(int_lt, |c, s, a| int_rel(c, s, a[0], |o| o.is_lt(), s::lt));
cfn!(int_le, |c, s, a| int_rel(c, s, a[0], |o| o.is_le(), s::le));
cfn!(int_gt, |c, s, a| int_rel(c, s, a[0], |o| o.is_gt(), s::gt));
cfn!(int_ge, |c, s, a| int_rel(c, s, a[0], |o| o.is_ge(), s::ge));
cfn!(int_abs, |_c, s, _a| Ok(if s.is_fixnum() { int_from_i64(s.as_fix().abs()) } else { big_to_value(rbig(s).abs()) }));
cfn!(int_to_s, |c, s, a| {
    let base = if a.is_empty() { 10 } else { num_to_long(c, a[0])? };
    if !(2..=36).contains(&base) { return arg_err(c, format!("invalid radix {}", base)); }
    let st = if s.is_fixnum() && base == 10 { s.as_fix().to_string() } else { to_big(s).to_string_radix(base as u32) };
    let v = str_new(st.as_bytes());
    rstr(v).enc = ENC_ASCII;
    Ok(v)
});
cfn!(int_to_f, |_c, s, _a| Ok(float_new(num_to_f64(s).unwrap())));
cfn!(int_to_i, |_c, s, _a| Ok(s));
cfn!(int_chr, |c, s, a| {
    let i = num_to_long(c, s)?;
    if !a.is_empty() || i > 255 {
        if i < 0 || i > 0x10ffff { return err(c, vm().c_range_error, format!("{} out of char range", i)); }
        let mut b = vec![]; encode_utf8(i as u32, &mut b);
        return Ok(str_from_vec(b));
    }
    if i < 0 { return err(c, vm().c_range_error, format!("{} out of char range", i)); }
    Ok(str_new_enc(&[i as u8], if i < 128 { ENC_ASCII } else { ENC_BINARY }))
});
cfn!(int_ord, |_c, s, _a| Ok(s));
fn int_times(c: &mut Ctx, s: Value, _a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return crate::builtins::enumerator::enum_for(c, s, s::times, &[], NIL); }
    if s.is_fixnum() {
        let n = s.as_fix();
        let mut i = 0;
        while i < n { yield1(c, b, Value::fix(i))?; i += 1; }
        return Ok(s);
    }
    let mut i = Value::fix(0);
    loop {
        if int_cmp_val(c, i, s)? != Some(std::cmp::Ordering::Less) { break; }
        yield1(c, b, i)?;
        i = int_arith(c, i, Value::fix(1), Op::Add)?;
    }
    Ok(s)
}
fn int_upto(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return crate::builtins::enumerator::enum_for(c, s, s::upto, a, NIL); }
    if s.is_fixnum() && a[0].is_fixnum() {
        let mut i = s.as_fix(); let e = a[0].as_fix();
        while i <= e { yield1(c, b, Value::fix(i))?; i += 1; }
        return Ok(s);
    }
    if is_float(a[0]) {
        let e = float_val(a[0]); let mut i = s;
        while num_to_f64(i).unwrap() <= e { yield1(c, b, i)?; i = int_arith(c, i, Value::fix(1), Op::Add)?; }
        return Ok(s);
    }
    let mut i = s;
    loop {
        match int_cmp_val(c, i, a[0])? { Some(o) if o.is_le() => {} Some(_) => break, None => return coerce_fail(c, s, a[0], true) }
        yield1(c, b, i)?;
        i = int_arith(c, i, Value::fix(1), Op::Add)?;
    }
    Ok(s)
}
fn int_downto(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return crate::builtins::enumerator::enum_for(c, s, s::downto, a, NIL); }
    if s.is_fixnum() && a[0].is_fixnum() {
        let mut i = s.as_fix(); let e = a[0].as_fix();
        while i >= e { yield1(c, b, Value::fix(i))?; i -= 1; }
        return Ok(s);
    }
    let mut i = s;
    loop {
        match int_cmp_val(c, i, a[0])? { Some(o) if o.is_ge() => {} Some(_) => break, None => return coerce_fail(c, s, a[0], true) }
        yield1(c, b, i)?;
        i = int_arith(c, i, Value::fix(1), Op::Sub)?;
    }
    Ok(s)
}
cfn!(int_succ, |c, s, _a| int_arith(c, s, Value::fix(1), Op::Add));
cfn!(int_pred, |c, s, _a| int_arith(c, s, Value::fix(1), Op::Sub));
cfn!(int_zero, |_c, s, _a| Ok(Value::bool(s == Value::fix(0))));
cfn!(int_even, |_c, s, _a| Ok(Value::bool(if s.is_fixnum() { s.as_fix() & 1 == 0 } else { !rbig(s).is_odd() })));
cfn!(int_odd, |_c, s, _a| Ok(Value::bool(if s.is_fixnum() { s.as_fix() & 1 == 1 } else { rbig(s).is_odd() })));
cfn!(int_positive, |_c, s, _a| Ok(Value::bool(if s.is_fixnum() { s.as_fix() > 0 } else { !rbig(s).neg })));
cfn!(int_negative, |_c, s, _a| Ok(Value::bool(if s.is_fixnum() { s.as_fix() < 0 } else { rbig(s).neg })));
cfn!(int_divmod, |c, s, a| {
    if is_float(a[0]) { return flo_divmod(c, float_new(num_to_f64(s).unwrap()), a, std::ptr::null()); }
    if !is_integer(a[0]) { return coerce_bin(c, s, a[0], s::divmod, false); }
    let q = int_arith(c, s, a[0], Op::Div)?; let m = int_arith(c, s, a[0], Op::Mod)?;
    Ok(ary_new_from(&[q, m]))
});
cfn!(int_idiv, |c, s, a| {
    if is_float(a[0]) {
        let y = float_val(a[0]);
        if y == 0.0 { return err(c, vm().c_zero_div_error, "divided by 0"); }
        return Ok(f64_to_int((num_to_f64(s).unwrap() / y).floor()));
    }
    int_arith(c, s, a[0], Op::Div)
});
cfn!(int_fdiv, |c, s, a| {
    let y = a[0];
    if is_integer(y) { return Ok(float_new(BigInt::ratio_to_f64(&to_big(s), &to_big(y)))); }
    if is_float(y) {
        let (x, dy) = (num_to_f64(s).unwrap(), float_val(y));
        if dy.is_nan() { return Ok(y); }
        return Ok(float_new(if dy != 0.0 { x / dy } else if x == 0.0 { f64::NAN } else { x * dy.signum() * f64::INFINITY }));
    }
    coerce_bin(c, s, y, intern(b"fdiv"), false)
});
cfn!(int_remainder, |c, s, a| {
    if is_float(a[0]) { if float_val(a[0]) == 0.0 { return err(c, vm().c_zero_div_error, "divided by 0"); } return Ok(float_new(num_to_f64(s).unwrap() % float_val(a[0]))); }
    if !is_integer(a[0]) { return coerce_bin(c, s, a[0], intern(b"remainder"), false); }
    let y = to_big(a[0]);
    if y.is_zero() { return err(c, vm().c_zero_div_error, "divided by 0"); }
    Ok(big_to_value(to_big(s).divrem_trunc(&y).1))
});
fn gcd_big(a: BigInt, b: BigInt) -> BigInt {
    let (mut a, mut b) = (a.abs(), b.abs());
    while !b.is_zero() { let r = a.divmod(&b).1; a = b; b = r; }
    a
}
cfn!(int_gcd, |c, s, a| {
    if !is_integer(a[0]) { return type_err(c, "not an integer"); }
    if s.is_fixnum() && a[0].is_fixnum() { let (mut x, mut y) = (s.as_fix().unsigned_abs(), a[0].as_fix().unsigned_abs()); while y != 0 { let r = x % y; x = y; y = r; } return Ok(int_from_i128(x as i128)); }
    Ok(big_to_value(gcd_big(to_big(s), to_big(a[0]))))
});
cfn!(int_lcm, |c, s, a| {
    if !is_integer(a[0]) { return type_err(c, "not an integer"); }
    let (x, y) = (to_big(s), to_big(a[0]));
    if x.is_zero() || y.is_zero() { return Ok(Value::fix(0)); }
    let g = gcd_big(x.clone(), y.clone());
    Ok(big_to_value(x.mul(&y).abs().divmod(&g).0))
});
fn round_int(c: &mut Ctx, s: Value, a: &[Value], mode: u8) -> RV {
    let digits = match a.iter().find(|v| !is_hash(**v)) { Some(&d) => num_to_long(c, d)?, None => 0 };
    if digits >= 0 { return Ok(s); }
    let half = round_half_mode(c, a)?;
    let m = BigInt::from_i64(10).pow((-digits) as u64);
    let x = to_big(s);
    let (q, r) = x.divmod(&m);
    let res = match mode {
        b'f' => q.mul(&m),
        b'c' => if r.is_zero() { x } else { q.add(&BigInt::from_i64(1)).mul(&m) },
        b't' => if x.neg && !r.is_zero() { q.add(&BigInt::from_i64(1)).mul(&m) } else { q.mul(&m) },
        _ => {
            let twice = r.mul(&BigInt::from_i64(2));
            let up = match twice.cmp(&m) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Equal => match half { 1 => q.is_odd(), 2 => x.neg, _ => !x.neg },
            };
            if up { q.add(&BigInt::from_i64(1)).mul(&m) } else { q.mul(&m) }
        }
    };
    Ok(big_to_value(res))
}
/// half: mode from kw (0 = up (default), 1 = even, 2 = down)
pub fn round_half_mode(c: &mut Ctx, a: &[Value]) -> Result<u8, Esc> {
    if let Some(h) = a.iter().rev().find(|v| is_hash(**v)) {
        if let Some(v) = crate::hash::hash_get_opt(c, *h, crate::vm::sym_value("half"))? {
            if v == NIL { return Ok(0); }
            let nm = if v.is_sym() { crate::symbol::sym_str(v.as_sym()) } else if is_string(v) { String::from_utf8_lossy(str_bytes(v)).into() } else { return arg_err(c, "invalid rounding mode"); };
            return match nm.as_str() { "up" => Ok(0), "even" => Ok(1), "down" => Ok(2), _ => arg_err(c, format!("invalid rounding mode: {}", nm)) };
        }
    }
    Ok(0)
}
cfn!(int_round, |c, s, a| round_int(c, s, a, b'r'));
cfn!(int_floor, |c, s, a| round_int(c, s, a, b'f'));
cfn!(int_ceil, |c, s, a| round_int(c, s, a, b'c'));
cfn!(int_truncate, |c, s, a| round_int(c, s, a, b't'));
cfn!(int_digits, |c, s, a| {
    let base = if a.is_empty() { 10 } else { num_to_long(c, a[0])? };
    if base < 2 { return arg_err(c, format!("invalid radix {}", base)); }
    if to_big(s).neg { return err(c, vm().c_math_domain_error, "out of domain"); }
    let out = ary_new();
    let mut x = to_big(s);
    let b = BigInt::from_i64(base);
    if x.is_zero() { ary_push(out, Value::fix(0)); }
    while !x.is_zero() { let (q, r) = x.divmod(&b); ary_push(out, big_to_value(r)); x = q; }
    Ok(out)
});
cfn!(int_bit_length, |_c, s, _a| Ok(Value::fix(if s.is_fixnum() { let x = s.as_fix(); (64 - if x < 0 { (!x).leading_zeros() } else { x.leading_zeros() }) as i64 } else { rbig(s).bit_length() as i64 })));
cfn!(int_size, |_c, s, _a| Ok(Value::fix(if s.is_fixnum() { 8 } else { ((rbig(s).mag.len() * 4 + 7) / 8 * 8) as i64 })));
cfn!(int_hash, |_c, s, _a| Ok(Value::fix((crate::astrust::hash_str(&to_big(s).to_string_radix(16)) >> 2) as i64)));
cfn!(int_coerce, |c, s, a| {
    if is_integer(a[0]) { return Ok(ary_new_from(&[a[0], s])); }
    let x = to_f(c, a[0])?;
    Ok(ary_new_from(&[float_new(x), float_new(num_to_f64(s).unwrap())]))
});
cfn!(int_allbits, |c, s, a| { let m = to_integer(c, a[0])?; let r = int_bitop(c, s, m, b'&')?; Ok(Value::bool(to_big(r) == to_big(m))) });
cfn!(int_anybits, |c, s, a| { let m = to_integer(c, a[0])?; let r = int_bitop(c, s, m, b'&')?; Ok(Value::bool(r != Value::fix(0))) });
cfn!(int_nobits, |c, s, a| { let m = to_integer(c, a[0])?; let r = int_bitop(c, s, m, b'&')?; Ok(Value::bool(r == Value::fix(0))) });
cfn!(int_s_sqrt, |c, _s, a| {
    let v = to_integer(c, a[0])?;
    let b = to_big(v);
    if b.neg { return err(c, vm().c_math_domain_error, "Numerical argument is out of domain - \"isqrt\""); }
    Ok(big_to_value(b.isqrt()))
});
cfn!(int_ceildiv, |c, s, a| {
    let neg = int_arith(c, s, Value::fix(-1), Op::Mul)?;
    let q = funcall(c, neg, intern(b"div"), &[a[0]], std::ptr::null())?;
    funcall(c, q, s::uminus, &[], std::ptr::null())
});
cfn!(int_integer_p, |_c, _s, _a| Ok(TRUE));

// ---- Float -------------------------------------------------------------------------------

fn flo_arith(c: &mut Ctx, a: Value, b: Value, op: Op) -> RV {
    let x = float_val(a);
    let y = match num_to_f64(b) { Some(y) => y, None => {
        let sym = match op { Op::Add => s::plus, Op::Sub => s::minus, Op::Mul => s::mul, Op::Div => s::div, Op::Mod => s::modulo, Op::Pow => s::pow };
        return coerce_bin(c, a, b, sym, false);
    } };
    if op == Op::Mod && y == 0.0 { return err(c, vm().c_zero_div_error, "divided by 0"); }
    Ok(float_new(match op {
        Op::Add => x + y, Op::Sub => x - y, Op::Mul => x * y, Op::Div => x / y,
        Op::Mod => fmod_ruby(x, y),
        Op::Pow => {
            if x < 0.0 && y != y.round() { return Ok(crate::builtins::complex::polar_pi((-x).powf(y), y)); }
            x.powf(y)
        }
    }))
}
cfn!(flo_plus, |c, s, a| flo_arith(c, s, a[0], Op::Add));
cfn!(flo_minus, |c, s, a| flo_arith(c, s, a[0], Op::Sub));
cfn!(flo_mul, |c, s, a| flo_arith(c, s, a[0], Op::Mul));
cfn!(flo_div, |c, s, a| flo_arith(c, s, a[0], Op::Div));
cfn!(flo_mod, |c, s, a| flo_arith(c, s, a[0], Op::Mod));
cfn!(flo_pow, |c, s, a| flo_arith(c, s, a[0], Op::Pow));
cfn!(flo_uminus, |_c, s, _a| Ok(float_new(-float_val(s))));
fn flo_cmp_val(c: &mut Ctx, a: Value, b: Value) -> Result<Option<std::cmp::Ordering>, Esc> {
    let x = float_val(a);
    if is_integer(b) { return Ok(int_cmp_val(c, b, a)?.map(|o| o.reverse())); }
    match num_to_f64(b) { Some(y) => Ok(x.partial_cmp(&y)), None => Ok(None) }
}
cfn!(flo_eq, |c, s, a| {
    if let Some(_) = num_to_f64(a[0]) { return Ok(Value::bool(flo_cmp_val(c, s, a[0])? == Some(std::cmp::Ordering::Equal))); }
    let r = funcall(c, a[0], s::eq, &[s], std::ptr::null())?; Ok(Value::bool(r.truthy()))
});
cfn!(flo_eql, |_c, s, a| Ok(Value::bool(is_float(a[0]) && float_val(a[0]) == float_val(s))));
cfn!(flo_cmp, |c, s, a| {
    let x = float_val(s);
    if x.is_infinite() && a[0].is_heap() && !is_float(a[0]) && !is_bignum(a[0]) && respond_to(a[0], intern(b"infinite?"), true) {
        let r = funcall(c, a[0], intern(b"infinite?"), &[], std::ptr::null())?;
        if r == NIL { return Ok(Value::fix(if x > 0.0 { 1 } else { -1 })); }
    }
    match flo_cmp_val(c, s, a[0])? {
        Some(o) => Ok(Value::fix(o as i64)),
        None => if num_to_f64(a[0]).is_none() && a[0].is_heap() && respond_to(a[0], s::coerce, true) { coerce_bin(c, s, a[0], s::cmp, false) } else { Ok(NIL) },
    }
});
fn flo_rel(c: &mut Ctx, s: Value, b: Value, f: fn(std::cmp::Ordering) -> bool, op: Sym) -> RV {
    match flo_cmp_val(c, s, b)? {
        Some(o) => Ok(Value::bool(f(o))),
        None => if num_to_f64(b).is_some() { Ok(FALSE) } else { coerce_bin(c, s, b, op, true) },
    }
}
cfn!(flo_lt, |c, s, a| flo_rel(c, s, a[0], |o| o.is_lt(), s::lt));
cfn!(flo_le, |c, s, a| flo_rel(c, s, a[0], |o| o.is_le(), s::le));
cfn!(flo_gt, |c, s, a| flo_rel(c, s, a[0], |o| o.is_gt(), s::gt));
cfn!(flo_ge, |c, s, a| flo_rel(c, s, a[0], |o| o.is_ge(), s::ge));
cfn!(flo_to_s, |_c, s, _a| { let v = str_new(float_to_s(float_val(s)).as_bytes()); rstr(v).enc = ENC_ASCII; Ok(v) });
fn check_finite(c: &mut Ctx, f: f64) -> Result<(), Esc> {
    if f.is_nan() || f.is_infinite() { return err(c, vm().c_float_domain_error, float_to_s(f)); }
    Ok(())
}
cfn!(flo_to_i, |c, s, _a| { let f = float_val(s); check_finite(c, f)?; Ok(f64_to_int(f)) });
cfn!(flo_abs, |_c, s, _a| Ok(float_new(float_val(s).abs())));
cfn!(flo_nan, |_c, s, _a| Ok(Value::bool(float_val(s).is_nan())));
cfn!(flo_infinite, |_c, s, _a| { let f = float_val(s); Ok(if f.is_infinite() { Value::fix(if f > 0.0 { 1 } else { -1 }) } else { NIL }) });
cfn!(flo_finite, |_c, s, _a| Ok(Value::bool(float_val(s).is_finite())));
cfn!(flo_zero, |_c, s, _a| Ok(Value::bool(float_val(s) == 0.0)));
cfn!(flo_positive, |_c, s, _a| Ok(Value::bool(float_val(s) > 0.0)));
cfn!(flo_negative, |_c, s, _a| Ok(Value::bool(float_val(s) < 0.0)));
fn flo_divmod(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let x = float_val(s);
    let Some(y) = num_to_f64(a[0]) else { return coerce_bin(c, s, a[0], s::divmod, false) };
    let (q, m) = flodivmod(c, x, y)?;
    check_finite(c, q)?;
    Ok(ary_new_from(&[f64_to_int(q), float_new(m)]))
}
/// CRuby's flodivmod
fn flodivmod(c: &mut Ctx, x: f64, y: f64) -> Result<(f64, f64), Esc> {
    if y.is_nan() { return Ok((y, y)); }
    if y == 0.0 { return err(c, vm().c_zero_div_error, "divided by 0"); }
    let mut m = if x == 0.0 || (y.is_infinite() && !x.is_infinite()) { x } else { x % y };
    let mut d = if x.is_infinite() && !y.is_infinite() { x } else { ((x - m) / y).round() };
    if y * m < 0.0 { m += y; d -= 1.0; }
    Ok((d, m))
}
cfn!(flo_idiv, |c, s, a| {
    let x = float_val(s); let y = to_f(c, a[0])?;
    if y == 0.0 { return err(c, vm().c_zero_div_error, "divided by 0"); }
    let q = (x / y).floor(); check_finite(c, q)?;
    Ok(f64_to_int(q))
});
cfn!(flo_coerce, |c, s, a| { let y = to_f(c, a[0])?; Ok(ary_new_from(&[float_new(y), s])) });
cfn!(flo_hash, |_c, s, _a| { let f = float_val(s); let b = if f == 0.0 { 0 } else { f.to_bits() }; Ok(Value::fix((crate::astrust::hash_u64(b) >> 2) as i64)) });

/// Decimal rounding of the shortest repr (CRuby-compatible for typical cases).
pub fn round_float_digits(f: f64, nd: i64, mode: u8, half: u8) -> f64 {
    if !f.is_finite() || nd > 17 { return f; }
    let s = format!("{:e}", f.abs());
    let (mant, exp) = s.split_once('e').unwrap();
    let exp: i64 = exp.parse().unwrap();
    let digits: Vec<u8> = mant.bytes().filter(|b| b.is_ascii_digit()).map(|b| b - b'0').collect();
    // value = 0.d1d2d3... * 10^(exp+1); keep digits up to position exp+1+nd
    let keep = exp + 1 + nd;
    if keep < 0 { return match mode { b'c' if f > 0.0 => 10f64.powi(-nd as i32), b'f' if f < 0.0 => -(10f64.powi(-nd as i32)), _ => 0.0 * f.signum() }; }
    let keep = keep as usize;
    if keep >= digits.len() { return f; }
    let mut kept: Vec<u8> = digits[..keep].to_vec();
    let rest = &digits[keep..];
    let rest_nonzero = rest.iter().any(|&d| d != 0);
    let neg = f < 0.0;
    let up = match mode {
        b'f' => neg && rest_nonzero,
        b'c' => !neg && rest_nonzero,
        b't' => false,
        _ => {
            let first = rest[0];
            if first > 5 || (first == 5 && rest[1..].iter().any(|&d| d != 0)) { true }
            else if first == 5 { match half { 1 => kept.last().map(|d| d % 2 == 1).unwrap_or(false), 2 => false, _ => true } }
            else { false }
        }
    };
    let mut e10 = exp + 1;
    if up {
        let mut i = kept.len();
        loop {
            if i == 0 { kept.insert(0, 1); e10 += 1; break; }
            i -= 1;
            if kept[i] == 9 { kept[i] = 0; } else { kept[i] += 1; break; }
        }
    }
    let mut txt = String::from("0.");
    for d in &kept { txt.push((b'0' + d) as char); }
    if kept.is_empty() { txt.push('0'); }
    txt.push_str(&format!("e{}", e10));
    let v: f64 = txt.parse().unwrap_or(0.0);
    if neg { -v } else { v }
}

fn flo_round_impl(c: &mut Ctx, s: Value, a: &[Value], mode: u8) -> RV {
    let f = float_val(s);
    let nd = match a.iter().find(|v| !is_hash(**v)) { Some(&d) => num_to_long(c, d)?, None => 0 };
    let half = round_half_mode(c, a)?;
    if nd > 0 {
        if f.is_infinite() || f.is_nan() { return Ok(s); }
        return Ok(float_new(round_float_digits(f, nd, mode, half)));
    }
    check_finite(c, f)?;
    if nd == 0 {
        let r = match mode {
            b'f' => f.floor(), b'c' => f.ceil(), b't' => f.trunc(),
            _ => {
                let r = f.round();
                if (f - f.trunc()).abs() == 0.5 {
                    match half { 1 => { let t = f.trunc(); if (t as i64) % 2 == 0 { t } else { r } } 2 => f.trunc(), _ => r }
                } else { round_float_digits(f, 0, b'r', half) }
            }
        };
        return Ok(f64_to_int(r));
    }
    let i = f64_to_int(match mode { b'f' => f.floor(), b'c' => f.ceil(), b't' => f.trunc(), _ => f.round() });
    let r = round_float_digits(f, nd, mode, half);
    let _ = i;
    Ok(f64_to_int(r))
}
cfn!(flo_round, |c, s, a| flo_round_impl(c, s, a, b'r'));
cfn!(flo_floor, |c, s, a| flo_round_impl(c, s, a, b'f'));
cfn!(flo_ceil, |c, s, a| flo_round_impl(c, s, a, b'c'));
cfn!(flo_truncate, |c, s, a| flo_round_impl(c, s, a, b't'));
cfn!(flo_next_float, |_c, s, _a| { let f = float_val(s); Ok(float_new(next_after(f, f64::INFINITY))) });
cfn!(flo_prev_float, |_c, s, _a| { let f = float_val(s); Ok(float_new(next_after(f, f64::NEG_INFINITY))) });
fn next_after(f: f64, dir: f64) -> f64 {
    if f.is_nan() { return f; }
    if f == dir { return f; }
    if f == 0.0 { return if dir > 0.0 { f64::from_bits(1) } else { -f64::from_bits(1) }; }
    let b = f.to_bits();
    let nb = if (dir > f) == (f > 0.0) { b + 1 } else { b - 1 };
    f64::from_bits(nb)
}
cfn!(flo_integer_p, |_c, _s, _a| Ok(FALSE));

// ---- Numeric (generic) -----------------------------------------------------------------------

fn num_step(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV {
    let (mut lim, mut st) = (NIL, Value::fix(1));
    let mut pos = vec![];
    for &x in a { if is_hash(x) {
        if let Some(v) = crate::hash::hash_get_opt(c, x, crate::vm::sym_value("by"))? { st = v; }
        if let Some(v) = crate::hash::hash_get_opt(c, x, crate::vm::sym_value("to"))? { lim = v; }
    } else { pos.push(x); } }
    if !pos.is_empty() { lim = pos[0]; }
    if pos.len() > 1 { st = pos[1]; }
    if b.is_null() { return crate::builtins::enumerator::arith_seq(c, s, lim, st, false, s::step); }
    if st == Value::fix(0) { return arg_err(c, "step can't be 0"); }
    if (is_float(s) || is_float(lim) || is_float(st)) && !crate::builtins::complex::is_complex(st) {
        let (x, l, d) = (to_f(c, s)?, if lim == NIL { if to_f(c, st)? > 0.0 { f64::INFINITY } else { f64::NEG_INFINITY } } else { to_f(c, lim)? }, to_f(c, st)?);
        let n = ((l - x) / d + 1e-9).floor();
        if n.is_infinite() { let mut i = 0.0; loop { yield1(c, b, float_new(x + i * d))?; i += 1.0; } }
        let mut i = 0.0;
        while i <= n { yield1(c, b, float_new(x + i * d))?; i += 1.0; }
        return Ok(s);
    }
    let neg = lim != NIL && funcall(c, st, s::lt, &[Value::fix(0)], std::ptr::null())?.truthy();
    let mut i = s;
    loop {
        if lim != NIL {
            let done = funcall(c, i, if neg { s::lt } else { s::gt }, &[lim], std::ptr::null())?;
            if done.truthy() { break; }
        }
        yield1(c, b, i)?;
        i = funcall(c, i, s::plus, &[st], std::ptr::null())?;
    }
    Ok(s)
}
cfn!(num_integer_p, |_c, _s, _a| Ok(FALSE));
cfn!(num_zero_p, |c, s, _a| { let r = funcall(c, s, s::eq, &[Value::fix(0)], std::ptr::null())?; Ok(Value::bool(r.truthy())) });
cfn!(num_nonzero_p, |c, s, _a| { let r = funcall(c, s, s::zero_p, &[], std::ptr::null())?; Ok(if r.truthy() { NIL } else { s }) });
cfn!(num_positive_p, |c, s, _a| funcall(c, s, s::gt, &[Value::fix(0)], std::ptr::null()));
cfn!(num_negative_p, |c, s, _a| funcall(c, s, s::lt, &[Value::fix(0)], std::ptr::null()));
cfn!(num_abs, |c, s, _a| { let neg = funcall(c, s, s::lt, &[Value::fix(0)], std::ptr::null())?; if neg.truthy() { funcall(c, s, s::uminus, &[], std::ptr::null()) } else { Ok(s) } });
cfn!(num_uplus, |_c, s, _a| Ok(s));
cfn!(num_uminus, |c, s, _a| {
    let r = funcall(c, Value::fix(0), s::coerce, &[s], std::ptr::null());
    match r { Ok(r) if is_array(r) => { let x = ary_get(r, 0); funcall(c, x, s::minus, &[ary_get(r, 1)], std::ptr::null()) }
              _ => { c.esc = EscKind::None; let z = Value::fix(0); funcall(c, z, s::minus, &[s], std::ptr::null()) } }
});
cfn!(num_coerce, |c, s, a| {
    if real_class_of(s) == real_class_of(a[0]) { return Ok(ary_new_from(&[a[0], s])); }
    let x = to_f(c, a[0])?; let y = to_f(c, s)?;
    Ok(ary_new_from(&[float_new(x), float_new(y)]))
});
cfn!(num_div, |c, s, a| {
    let q = funcall(c, s, s::div, &[a[0]], std::ptr::null())?;
    funcall(c, q, intern(b"floor"), &[], std::ptr::null())
});
cfn!(num_modulo, |c, s, a| {
    let q = funcall(c, s, intern(b"div"), &[a[0]], std::ptr::null())?;
    let m = funcall(c, a[0], s::mul, &[q], std::ptr::null())?;
    funcall(c, s, s::minus, &[m], std::ptr::null())
});
cfn!(num_divmod, |c, s, a| {
    let q = funcall(c, s, intern(b"div"), &[a[0]], std::ptr::null())?;
    let m = funcall(c, s, s::modulo, &[a[0]], std::ptr::null())?;
    Ok(ary_new_from(&[q, m]))
});
fn neg_int_p(c: &mut Ctx, v: Value) -> Result<bool, Esc> {
    if is_integer(v) { return Ok(if v.is_fixnum() { v.as_fix() < 0 } else { rbig(v).neg }); }
    Ok(funcall(c, v, s::lt, &[Value::fix(0)], std::ptr::null())?.truthy())
}
fn pos_int_p(c: &mut Ctx, v: Value) -> Result<bool, Esc> {
    if is_integer(v) { return Ok(if v.is_fixnum() { v.as_fix() > 0 } else { !rbig(v).neg }); }
    Ok(funcall(c, v, s::gt, &[Value::fix(0)], std::ptr::null())?.truthy())
}
cfn!(num_remainder, |c, s, a| {
    let (mut x, mut y) = (s, a[0]);
    if !is_kind_of(y, vm().c_numeric) && !is_numeric(y) {
        let r = coerce_bin(c, x, y, intern(b"remainder"), false)?;
        return Ok(r);
    }
    let z = funcall(c, x, s::modulo, &[y], std::ptr::null())?;
    let zz = funcall(c, z, s::eq, &[Value::fix(0)], std::ptr::null())?.truthy();
    if !zz && ((neg_int_p(c, x)? && pos_int_p(c, y)?) || (pos_int_p(c, x)? && neg_int_p(c, y)?)) {
        if is_float(y) && float_val(y).is_infinite() { return Ok(x); }
        return funcall(c, z, s::minus, &[y], std::ptr::null());
    }
    let _ = (&mut x, &mut y);
    Ok(z)
});
cfn!(num_fdiv, |c, s, a| { let x = to_f(c, s)?; let y = to_f(c, a[0])?; Ok(float_new(x / y)) });
cfn!(num_quo, |c, s, a| {
    if is_float(s) || is_float(a[0]) { let x = to_f(c, s)?; let y = to_f(c, a[0])?; return Ok(float_new(x / y)); }
    let r = funcall(c, vm().main_obj, intern(b"Rational"), &[s, Value::fix(1)], std::ptr::null())?;
    funcall(c, r, s::div, &[a[0]], std::ptr::null())
});
cfn!(num_to_int, |c, s, _a| funcall(c, s, s::to_i, &[], std::ptr::null()));
cfn!(num_real_p, |_c, _s, _a| Ok(TRUE));
cfn!(num_finite_p, |_c, _s, _a| Ok(TRUE));
cfn!(num_infinite_p, |_c, _s, _a| Ok(NIL));
cfn!(num_eql, |c, s, a| {
    if real_class_of(s) != real_class_of(a[0]) { return Ok(FALSE); }
    funcall(c, s, s::eq, &[a[0]], std::ptr::null())
});
cfn!(num_i, |c, s, _a| funcall(c, vm().main_obj, intern(b"Complex"), &[Value::fix(0), s], std::ptr::null()));
cfn!(num_to_c, |c, s, _a| funcall(c, vm().main_obj, intern(b"Complex"), &[s, Value::fix(0)], std::ptr::null()));
cfn!(num_real, |_c, s, _a| Ok(s));
cfn!(num_imag, |_c, _s, _a| Ok(Value::fix(0)));
cfn!(num_conj, |_c, s, _a| Ok(s));
cfn!(num_rect, |_c, s, _a| Ok(ary_new_from(&[s, Value::fix(0)])));
cfn!(num_arg, |c, s, _a| { let neg = funcall(c, s, s::lt, &[Value::fix(0)], std::ptr::null())?; Ok(if neg.truthy() { float_new(std::f64::consts::PI) } else { Value::fix(0) }) });

pub fn init() {
    let vm = vm();
    let n = vm.c_numeric;
    define_cfunc(n, "step", num_step, -1);
    define_cfunc(n, "integer?", num_integer_p, 0);
    define_cfunc(n, "zero?", num_zero_p, 0);
    define_cfunc(n, "nonzero?", num_nonzero_p, 0);
    define_cfunc(n, "positive?", num_positive_p, 0);
    define_cfunc(n, "negative?", num_negative_p, 0);
    define_cfunc(n, "abs", num_abs, 0);
    define_cfunc(n, "magnitude", num_abs, 0);
    define_cfunc(n, "+@", num_uplus, 0);
    define_cfunc(n, "-@", num_uminus, 0);
    define_cfunc(n, "coerce", num_coerce, 1);
    define_cfunc(n, "div", num_div, 1);
    define_cfunc(n, "modulo", num_modulo, 1);
    define_cfunc(n, "%", num_modulo, 1);
    define_cfunc(n, "divmod", num_divmod, 1);
    define_cfunc(n, "fdiv", num_fdiv, 1);
    define_cfunc(n, "remainder", num_remainder, 1);
    define_cfunc(n, "quo", num_quo, 1);
    define_cfunc(n, "to_int", num_to_int, 0);
    define_cfunc(n, "real?", num_real_p, 0);
    define_cfunc(n, "finite?", num_finite_p, 0);
    define_cfunc(n, "infinite?", num_infinite_p, 0);
    define_cfunc(n, "eql?", num_eql, 1);
    define_cfunc(n, "i", num_i, 0);
    define_cfunc(n, "to_c", num_to_c, 0);
    define_cfunc(n, "real", num_real, 0);
    define_cfunc(n, "imaginary", num_imag, 0);
    define_cfunc(n, "imag", num_imag, 0);
    define_cfunc(n, "conj", num_conj, 0);
    define_cfunc(n, "conjugate", num_conj, 0);
    define_cfunc(n, "rectangular", num_rect, 0);
    define_cfunc(n, "rect", num_rect, 0);
    define_cfunc(n, "arg", num_arg, 0);
    define_cfunc(n, "angle", num_arg, 0);
    define_cfunc(n, "phase", num_arg, 0);

    let k = vm.c_integer;
    let d = |nm: &str, f: CFunc, a: i32| define_cfunc(k, nm, f, a);
    d("+", int_plus, 1); d("-", int_minus, 1); d("*", int_mul, 1); d("/", int_div, 1); d("%", int_mod, 1);
    d("modulo", int_mod, 1); d("**", int_pow_m, -1); d("pow", int_pow_m, -1); d("-@", int_uminus, 0); d("+@", int_uplus, 0);
    d("~", int_not, 0); d("&", int_and, 1); d("|", int_or, 1); d("^", int_xor, 1); d("<<", int_lshift, 1); d(">>", int_rshift, 1);
    d("[]", int_aref, -1); d("==", int_eq, 1); d("===", int_eq, 1); d("eql?", int_eql, 1); d("<=>", int_cmp, 1);
    d("<", int_lt, 1); d("<=", int_le, 1); d(">", int_gt, 1); d(">=", int_ge, 1); d("abs", int_abs, 0); d("magnitude", int_abs, 0);
    d("to_s", int_to_s, -1); d("inspect", int_to_s, -1); d("to_f", int_to_f, 0); d("to_i", int_to_i, 0); d("to_int", int_to_i, 0);
    d("chr", int_chr, -1); d("ord", int_ord, 0); d("times", int_times, 0); d("upto", int_upto, 1); d("downto", int_downto, 1);
    d("succ", int_succ, 0); d("next", int_succ, 0); d("pred", int_pred, 0); d("zero?", int_zero, 0); d("even?", int_even, 0);
    d("odd?", int_odd, 0); d("positive?", int_positive, 0); d("negative?", int_negative, 0); d("divmod", int_divmod, 1);
    d("div", int_idiv, 1); d("fdiv", int_fdiv, 1); d("remainder", int_remainder, 1); d("gcd", int_gcd, 1); d("lcm", int_lcm, 1);
    d("round", int_round, -1); d("floor", int_floor, -1); d("ceil", int_ceil, -1); d("truncate", int_truncate, -1);
    d("digits", int_digits, -1); d("bit_length", int_bit_length, 0); d("size", int_size, 0); d("hash", int_hash, 0);
    d("coerce", int_coerce, 1); d("allbits?", int_allbits, 1); d("anybits?", int_anybits, 1); d("nobits?", int_nobits, 1);
    d("integer?", int_integer_p, 0); d("ceildiv", int_ceildiv, 1);
    define_singleton_cfunc(k, "sqrt", int_s_sqrt, 1);

    let f = vm.c_float;
    let d = |nm: &str, fun: CFunc, a: i32| define_cfunc(f, nm, fun, a);
    d("+", flo_plus, 1); d("-", flo_minus, 1); d("*", flo_mul, 1); d("/", flo_div, 1); d("%", flo_mod, 1); d("modulo", flo_mod, 1);
    d("**", flo_pow, 1); d("pow", flo_pow, 1); d("-@", flo_uminus, 0); d("==", flo_eq, 1); d("===", flo_eq, 1); d("eql?", flo_eql, 1);
    d("<=>", flo_cmp, 1); d("<", flo_lt, 1); d("<=", flo_le, 1); d(">", flo_gt, 1); d(">=", flo_ge, 1);
    d("to_s", flo_to_s, 0); d("inspect", flo_to_s, 0); d("to_i", flo_to_i, 0); d("to_int", flo_to_i, 0);
    d("abs", flo_abs, 0); d("magnitude", flo_abs, 0); d("nan?", flo_nan, 0); d("infinite?", flo_infinite, 0); d("finite?", flo_finite, 0);
    d("zero?", flo_zero, 0); d("positive?", flo_positive, 0); d("negative?", flo_negative, 0); d("divmod", flo_divmod, 1);
    d("div", flo_idiv, 1); d("coerce", flo_coerce, 1); d("hash", flo_hash, 0); d("round", flo_round, -1); d("floor", flo_floor, -1);
    d("ceil", flo_ceil, -1); d("truncate", flo_truncate, -1); d("next_float", flo_next_float, 0); d("prev_float", flo_prev_float, 0);
    d("to_f", |_c, s, _a, _b| Ok(s), 0); d("integer?", flo_integer_p, 0);
    d("fdiv", flo_div, 1);
    d("quo", flo_div, 1);
    for (nm, v) in [("INFINITY", f64::INFINITY), ("NAN", f64::NAN), ("EPSILON", f64::EPSILON), ("MAX", f64::MAX), ("MIN", f64::MIN_POSITIVE)] {
        const_set(f, intern(nm.as_bytes()), float_new(v));
    }
    for (nm, v) in [("DIG", 15), ("MANT_DIG", 53), ("MAX_EXP", 1024), ("MIN_EXP", -1021), ("MAX_10_EXP", 308), ("MIN_10_EXP", -307), ("RADIX", 2)] {
        const_set(f, intern(nm.as_bytes()), Value::fix(v));
    }
}
