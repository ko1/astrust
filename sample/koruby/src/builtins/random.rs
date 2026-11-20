// rand / Random (xorshift64*).
use crate::object::*;
use crate::value::*;
use crate::vm::*;

pub fn seed(s: u64) { vm().rand_state = if s == 0 { 0x9E3779B97F4A7C15 } else { s.wrapping_mul(0x9E3779B97F4A7C15) | 1 }; }
pub fn next_u64() -> u64 {
    let st = &mut vm().rand_state;
    let mut x = *st;
    x ^= x >> 12; x ^= x << 25; x ^= x >> 27;
    *st = x;
    x.wrapping_mul(0x2545F4914F6CDD1D)
}
pub fn rand_below(n: u64) -> u64 { if n == 0 { 0 } else { next_u64() % n } }
pub fn rand_float() -> f64 { (next_u64() >> 11) as f64 / (1u64 << 53) as f64 }

pub fn rand_value(c: &mut Ctx, lim: Value) -> RV {
    if lim == NIL || lim == Value::fix(0) { return Ok(crate::numeric::float_new(rand_float())); }
    if is_float(lim) { let f = float_val(lim); return Ok(crate::numeric::float_new(rand_float() * f.abs())); }
    if is_range(lim) {
        let (b, e, x) = crate::range::range_parts(lim);
        if b.is_fixnum() && e.is_fixnum() {
            let (lo, hi) = (b.as_fix(), if x { e.as_fix() - 1 } else { e.as_fix() });
            if hi < lo { return Ok(NIL); }
            return Ok(Value::fix(lo + rand_below((hi - lo + 1) as u64) as i64));
        }
        let (bf, ef) = (crate::numeric::num_to_f64(b).unwrap_or(0.0), crate::numeric::num_to_f64(e).unwrap_or(1.0));
        return Ok(crate::numeric::float_new(bf + rand_float() * (ef - bf)));
    }
    let n = crate::builtins::numeric_methods::num_to_long(c, lim)?.abs();
    Ok(Value::fix(rand_below(n as u64) as i64))
}

fn r_rand(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { rand_value(c, a.first().copied().unwrap_or(NIL)) }
fn r_bytes(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
    Ok(crate::string::str_from_vec_enc((0..n).map(|_| next_u64() as u8).collect(), ENC_BINARY))
}
fn r_init(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if let Some(&sd) = a.first() { seed(crate::builtins::numeric_methods::num_to_long(c, sd)? as u64); }
    ivar_set_raw(s, crate::symbol::intern(b"@seed"), a.first().copied().unwrap_or(Value::fix(0)));
    Ok(NIL)
}
fn r_new_seed(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix((next_u64() >> 2) as i64)) }

pub fn init() {
    let vm = vm();
    let r = define_class("Random", vm.c_object);
    vm.c_random = r;
    define_private_cfunc(r, "initialize", r_init, -1);
    define_cfunc(r, "rand", r_rand, -1);
    define_cfunc(r, "bytes", r_bytes, 1);
    define_cfunc(r, "seed", |_c, s, _a, _b| Ok(ivar_get(s, crate::symbol::intern(b"@seed"))), 0);
    define_singleton_cfunc(r, "rand", r_rand, -1);
    define_singleton_cfunc(r, "bytes", r_bytes, 1);
    define_singleton_cfunc(r, "new_seed", r_new_seed, 0);
    define_singleton_cfunc(r, "srand", |c, _s, a, _b| { let old = crate::vm::vm().rand_state; if let Some(&v) = a.first() { seed(crate::builtins::numeric_methods::num_to_long(c, v)? as u64); } Ok(crate::bignum::int_from_i128(old as i128)) }, -1);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
    seed(t ^ (std::process::id() as u64) << 32);
}
