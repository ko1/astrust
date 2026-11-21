// Random: MT19937 with CRuby's seeding, real/int/range draws (same numbers for the same seed).
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::bignum::*;
use crate::builtins::kernel::*;
use crate::numeric::*;
use crate::object::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

const N: usize = 624;
const M: usize = 397;

#[derive(Clone)]
pub struct Mt { state: [u32; N], next: usize, left: i32 }
impl Mt {
    fn new() -> Mt { Mt { state: [0; N], next: N, left: 1 } }
    fn init_genrand(&mut self, s: u32) {
        self.state[0] = s;
        for j in 1..N { self.state[j] = 1812433253u32.wrapping_mul(self.state[j - 1] ^ (self.state[j - 1] >> 30)).wrapping_add(j as u32); }
        self.left = 1; self.next = N;
    }
    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19650218);
        let (mut i, mut j) = (1usize, 0usize);
        let st = &mut self.state;
        let mut k = N.max(key.len());
        while k > 0 {
            st[i] = (st[i] ^ (st[i - 1] ^ (st[i - 1] >> 30)).wrapping_mul(1664525)).wrapping_add(key[j]).wrapping_add(j as u32);
            i += 1; j += 1;
            if i >= N { st[0] = st[N - 1]; i = 1; }
            if j >= key.len() { j = 0; }
            k -= 1;
        }
        k = N - 1;
        while k > 0 {
            st[i] = (st[i] ^ (st[i - 1] ^ (st[i - 1] >> 30)).wrapping_mul(1566083941)).wrapping_sub(i as u32);
            i += 1;
            if i >= N { st[0] = st[N - 1]; i = 1; }
            k -= 1;
        }
        st[0] = 0x80000000;
    }
    fn next_state(&mut self) {
        let st = &mut self.state;
        let twist = |u: u32, v: u32| (((u & 0x80000000) | (v & 0x7fffffff)) >> 1) ^ if v & 1 != 0 { 0x9908b0df } else { 0 };
        for p in 0..N - M { st[p] = st[p + M] ^ twist(st[p], st[p + 1]); }
        for p in N - M..N - 1 { st[p] = st[p + M - N] ^ twist(st[p], st[p + 1]); }
        st[N - 1] = st[M - 1] ^ twist(st[N - 1], st[0]);
        self.left = N as i32; self.next = 0;
    }
    pub fn genrand_int32(&mut self) -> u32 {
        self.left -= 1;
        if self.left <= 0 { self.next_state(); }
        let mut y = self.state[self.next]; self.next += 1;
        y ^= y >> 11; y ^= (y << 7) & 0x9d2c5680; y ^= (y << 15) & 0xefc60000; y ^= y >> 18;
        y
    }
}

const DBL_REDUCE_SCALE: f64 = 1.0 / 9007199254740992.0;
fn pair_to_real_excl(a: u32, b: u32) -> f64 { ((a >> 5) as f64 * 67108864.0 + (b >> 6) as f64) * DBL_REDUCE_SCALE }
fn pair_to_real_incl(a: u32, b: u32) -> f64 {
    let x = ((a as u128) << 32) | b as u128;
    let m = (1u128 << 53) | 1;
    ((x * m) >> 64) as u64 as f64 * DBL_REDUCE_SCALE
}

pub struct RRandom { pub mt: Mt, pub seed: Value }
impl RData for RRandom {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.seed); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

fn default_rnd(c: &Ctx) -> Value { c.vm().default_random }
fn rr(c: &mut Ctx, v: Value) -> &mut RRandom { c.data_mut::<RRandom>(v).expect("a Random") }

/// The generator behind ob: a Random instance, the Random class (default), or None.
fn get_rnd(c: &Ctx, ob: Value) -> Option<Value> {
    if ob == NIL || ob == c.vm().c_random { return Some(default_rnd(c)); }
    if c.data::<RRandom>(ob).is_some() { return Some(ob); }
    None
}

fn fill_random_words(n: usize) -> Vec<u32> {
    let mut buf = vec![0u8; n * 4];
    // SecureRandom asks for a few bytes at a time: read 4 KiB per open (no fd kept, so fd numbers stay CRuby's);
    // the pool is per process, never shared with a forked child
    thread_local! { static POOL: std::cell::RefCell<(i32, Vec<u8>)> = const { std::cell::RefCell::new((0, Vec::new())) }; }
    let ok = POOL.with(|p| {
        let mut p = p.borrow_mut();
        let pid = crate::builtins::sys::getpid();
        if p.0 != pid { *p = (pid, Vec::new()); }
        if p.1.len() < buf.len() {
            let mut more = vec![0u8; buf.len().max(4096)];
            let read = std::fs::File::open("/dev/urandom").and_then(|mut f| { use std::io::Read; f.read_exact(&mut more) }).is_ok();
            if !read { return false; }
            p.1.extend_from_slice(&more);
        }
        let k = p.1.len() - buf.len();
        buf.copy_from_slice(&p.1[k..]);
        p.1.truncate(k);
        true
    });
    if !ok {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(1);
        for (i, b) in buf.iter_mut().enumerate() { *b = (t >> ((i % 16) * 8)) as u8 ^ (std::process::id() as u8).wrapping_mul(i as u8 + 1); }
    }
    buf.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}
fn make_seed_value(c: &mut Ctx, mut w: Vec<u32>) -> Value {
    if *w.last().unwrap() <= 1 { w.push(1); }
    c.big_to_value(BigInt::from_words(false, &w))
}
/// Seeds the generator of `r` with Integer `seed`.
fn rand_init(c: &mut Ctx, r: Value, seed: Value) {
    let b = c.to_big(seed);
    let mt = &mut rr(c, r).mt;
    let mut w = b.mag.clone();
    if w.is_empty() { w.push(0); }
    if w.len() == 1 { mt.init_genrand(w[0]); }
    else {
        if *w.last().unwrap() == 1 { w.pop(); }
        mt.init_by_array(&w);
    }
}
fn rand_init_default(c: &mut Ctx, mt: &mut Mt) -> Value {
    let w = fill_random_words(4);
    mt.init_by_array(&w);
    make_seed_value(c, w)
}

// ---- draws (rnd = Some(generator) or None for a duck-typed object using #bytes) ------------

fn obj_bytes(c: &mut Ctx, ob: Value, n: usize) -> Result<Vec<u8>, Esc> {
    let v = funcall_public(c, ob, intern(b"bytes"), &[Value::fix(n as i64)], std::ptr::null())?;
    if !is_string(v) { return type_err(c, format!("wrong argument type {} (expected String)", class_name_for_err(v))); }
    let b = c.str_bytes(v).to_vec();
    if b.len() < n { return err(c, c.vm().c_range_error, format!("random data too short {}", b.len())); }
    if b.len() > n { return err(c, c.vm().c_range_error, format!("random data too long {}", b.len())); }
    Ok(b)
}
fn int32(c: &mut Ctx, ob: Value, rnd: Option<Value>) -> Result<u32, Esc> {
    match rnd { Some(r) => Ok(rr(c, r).mt.genrand_int32()), None => { let b = obj_bytes(c, ob, 4)?; Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]])) } }
}
fn random_real(c: &mut Ctx, ob: Value, rnd: Option<Value>, excl: bool) -> Result<f64, Esc> {
    let (a, b) = match rnd {
        Some(r) => { let m = &mut rr(c, r).mt; (m.genrand_int32(), m.genrand_int32()) }
        None => { let x = obj_bytes(c, ob, 8)?; (u32::from_le_bytes([x[0], x[1], x[2], x[3]]), u32::from_le_bytes([x[4], x[5], x[6], x[7]])) }
    };
    Ok(if excl { pair_to_real_excl(a, b) } else { pair_to_real_incl(a, b) })
}
fn make_mask(mut x: u64) -> u64 { x |= x >> 1; x |= x >> 2; x |= x >> 4; x |= x >> 8; x |= x >> 16; x |= x >> 32; x }
fn limited_rand(c: &mut Ctx, ob: Value, rnd: Option<Value>, limit: u64) -> Result<u64, Esc> {
    if limit == 0 { return Ok(0); }
    if rnd.is_none() {
        let w = 64 - limit.leading_zeros();
        let n = if w > 32 { 8 } else { 4 };
        let mask = if w >= 64 { u64::MAX } else { !(u64::MAX << w) };
        let full = if n == 8 { u64::MAX } else { 0xffff_ffff };
        let (mut bits, mut rest) = (0u64, 0u64);
        loop {
            if mask & !rest != 0 {
                let b = obj_bytes(c, ob, n)?;
                let mut a = [0u8; 8]; a[..n].copy_from_slice(&b);
                bits = u64::from_le_bytes(a); rest = full;
            }
            let val = bits & mask;
            bits = if w >= 64 { 0 } else { bits >> w }; rest = if w >= 64 { 0 } else { rest >> w };
            if val <= limit { return Ok(val); }
        }
    }
    Ok(limited_rand_mt(&mut rr(c, rnd.unwrap()).mt, limit))
}
fn limited_rand_mt(mt: &mut Mt, limit: u64) -> u64 {
    if limit == 0 { return 0; }
    let mask = make_mask(limit);
    if limit > 0xffff_ffff {
        'retry: loop {
            let mut val = 0u64;
            for i in (0..2).rev() {
                if (mask >> (i * 32)) & 0xffff_ffff != 0 {
                    val |= (mt.genrand_int32() as u64) << (i * 32);
                    val &= mask;
                    if limit < val { continue 'retry; }
                }
            }
            return val;
        }
    }
    loop { let v = mt.genrand_int32() as u64 & mask; if v <= limit { return v; } }
}
fn limited_big_rand(c: &mut Ctx, ob: Value, rnd: Option<Value>, limit: &BigInt) -> RV {
    let lim = &limit.mag;
    let len = lim.len();
    if rnd.is_none() {
        // MS-word-first draws from #bytes
        let nlz = lim[len - 1].leading_zeros();
        let mask = u32::MAX >> nlz;
        loop {
            let b = obj_bytes(c, ob, len * 4)?;
            let mut r: Vec<u32> = b.chunks(4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]])).collect();
            r[0] &= mask;
            let msw: Vec<u32> = lim.iter().rev().copied().collect();
            let mut ok = true;
            for i in 0..len { if msw[i] < r[i] { ok = false; break; } if r[i] < msw[i] { break; } }
            if ok { r.reverse(); return Ok(c.big_to_value(BigInt::from_words(false, &r))); }
        }
    }
    'retry: loop {
        let mut out = vec![0u32; len];
        let mut mask = 0u32; let mut boundary = true;
        for i in (0..len).rev() {
            let l = lim[i];
            mask = if mask != 0 { u32::MAX } else { make_mask(l as u64) as u32 };
            let mut r = 0;
            if mask != 0 {
                r = int32(c, ob, rnd)? & mask;
                if boundary { if l < r { continue 'retry; } if r < l { boundary = false; } }
            }
            out[i] = r;
        }
        return Ok(c.big_to_value(BigInt::from_words(false, &out)));
    }
}
/// rb_random_ulong_limited: [0, limit] from a Random, the Random class, or any object's public #rand(limit + 1).
pub fn ulong_limited(c: &mut Ctx, ob: Value, limit: u64) -> Result<u64, Esc> {
    if let Some(r) = get_rnd(c, ob) { return Ok(limited_rand_mt(&mut rr(c, r).mt, limit)); }
    if limit == 0 { return Ok(0); }
    let lim = c.int_from_i128(limit as i128 + 1);
    let v = funcall_public(c, ob, intern(b"rand"), &[lim], std::ptr::null())?;
    let r = crate::builtins::numeric_methods::num_to_long(c, v)?;
    if r < 0 { return err(c, c.vm().c_range_error, format!("random number too small {}", r)); }
    if r as u64 > limit { return err(c, c.vm().c_range_error, format!("random number too big {}", r)); }
    Ok(r as u64)
}
/// [0, n) for the default generator (Array#shuffle / #sample).
pub fn rand_below(c: &mut Ctx, n: u64) -> u64 {
    if n == 0 { return 0; }
    let d = default_rnd(c);
    limited_rand_mt(&mut rr(c, d).mt, n - 1)
}

fn rand_int(c: &mut Ctx, ob: Value, rnd: Option<Value>, vmax: Value, restrictive: bool) -> Result<Option<Value>, Esc> {
    if vmax.is_fixnum() {
        let mut max = vmax.as_fix();
        if max == 0 { return Ok(None); }
        if max < 0 { if restrictive { return Ok(None); } max = -max; }
        let r = limited_rand(c, ob, rnd, (max - 1) as u64)?;
        return Ok(Some(c.int_from_i128(r as i128)));
    }
    let mut b = c.to_big(vmax);
    if b.is_zero() { return Ok(None); }
    if b.neg { if restrictive { return Ok(None); } b = b.neg(); }
    let b = b.sub(&BigInt::from_i64(1));
    if let Some(m) = b.to_i64() { if Value::fixable(m) {
        if m == -1 { return Ok(None); }
        let r = limited_rand(c, ob, rnd, m as u64)?;
        return Ok(Some(c.int_from_i128(r as i128)));
    } }
    Ok(Some(limited_big_rand(c, ob, rnd, &b)?))
}

fn domain_error<T>(c: &mut Ctx) -> Result<T, Esc> {
    let k = Some(crate::builtins::exception::errno_class(c.vm(), 33));
    match k {
        Some(k) => { let e = funcall(c, k, s::new, &[], std::ptr::null())?; Err(raise_exc(c, e)) }
        None => err(c, c.vm().c_argument_error, "Numerical argument out of domain"),
    }
}
fn float_value(c: &mut Ctx, f: f64) -> Result<f64, Esc> { if !f.is_finite() { return domain_error(c); } Ok(f) }
pub(crate) fn check_to_int(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if is_integer(v) { return Ok(Some(v)); }
    if v.is_heap() && !is_float(v) && respond_to(v, s::to_int, true) {
        let r = funcall(c, v, s::to_int, &[], std::ptr::null())?;
        if is_integer(r) { return Ok(Some(r)); }
    }
    Ok(None)
}
fn check_to_float(c: &mut Ctx, v: Value) -> Result<Option<f64>, Esc> {
    if is_float(v) { return Ok(Some(float_val(v))); }
    if !is_kind_of(v, c.vm().c_numeric) && !v.is_fixnum() { return Ok(None); }
    if let Some(f) = c.num_to_f64(v) { return Ok(Some(f)); }
    let r = funcall(c, v, s::to_f, &[], std::ptr::null())?;
    Ok(if is_float(r) { Some(float_val(r)) } else { None })
}

enum Draw { Val(Value), False }

fn rand_range(c: &mut Ctx, ob: Value, rnd: Option<Value>, range: Value) -> Result<Draw, Esc> {
    if !is_range(range) { return Ok(Draw::False); }
    let (beg, end, excl) = c.range(range);
    if beg == NIL || end == NIL { return domain_error(c); }
    if !respond_to(end, s::minus, true) { return Ok(Draw::False); }
    let vmax = funcall(c, end, s::minus, &[beg], std::ptr::null())?;
    let mut v = NIL;
    let iv = if is_float(vmax) { None } else { check_to_int(c, vmax)? };
    if let Some(vm_) = iv {
        let mut vmax = vm_; let mut excl = excl;
        loop {
            if vmax.is_fixnum() {
                let max = vmax.as_fix() - excl as i64;
                if max >= 0 { let r = limited_rand(c, ob, rnd, max as u64)?; v = c.int_from_i128(r as i128); }
                break;
            }
            let b = c.to_big(vmax);
            if !b.neg && !b.is_zero() {
                let b = if excl { b.sub(&BigInt::from_i64(1)) } else { b };
                let bv = c.big_to_value(b.clone());
                if bv.is_fixnum() { vmax = bv; excl = false; continue; }
                v = limited_big_rand(c, ob, rnd, &b)?;
            }
            break;
        }
    } else if let Some(mut max) = check_to_float(c, vmax)? {
        let (mut scale, mut mid) = (1.0, 0.5);
        if max.is_infinite() {
            let bf = match check_to_float(c, beg)? { Some(f) => f, None => return type_err(c, "can't convert into Float") };
            let ef = match check_to_float(c, end)? { Some(f) => f, None => return type_err(c, "can't convert into Float") };
            let min = float_value(c, bf)? / 2.0;
            max = float_value(c, ef)? / 2.0;
            scale = 2.0; mid = max + min; max -= min;
        } else if max.is_nan() { return domain_error(c); }
        if max > 0.0 {
            let r = random_real(c, ob, rnd, excl)?;
            if scale > 1.0 { return Ok(Draw::Val(c.float_new((r - 0.5) * max * scale + mid))); }
            v = c.float_new(r * max);
        } else if max == 0.0 && !excl { v = c.float_new(0.0); }
    }
    if beg.is_fixnum() && v.is_fixnum() { return Ok(Draw::Val(c.int_from_i64(beg.as_fix() + v.as_fix()))); }
    if v == NIL { return Ok(Draw::Val(NIL)); }
    if is_bignum(v) { return Ok(Draw::Val(funcall(c, v, s::plus, &[beg], std::ptr::null())?)); }
    if is_float(v) { if let Some(f) = check_to_float(c, beg)? { return Ok(Draw::Val(c.float_new(float_val(v) + f))); } }
    Ok(Draw::Val(funcall(c, beg, s::plus, &[v], std::ptr::null())?))
}

fn rand_random(c: &mut Ctx, ob: Value, rnd: Option<Value>, a: &[Value]) -> Result<Draw, Esc> {
    check_argc(c, a, 0, 1)?;
    if a.is_empty() { let r = random_real(c, ob, rnd, true)?; return Ok(Draw::Val(c.float_new(r))); }
    let vmax = a[0];
    if vmax == NIL { return Ok(Draw::Val(NIL)); }
    if !is_float(vmax) {
        if let Some(v) = check_to_int(c, vmax)? { return Ok(Draw::Val(rand_int(c, ob, rnd, v, true)?.unwrap_or(NIL))); }
    }
    if let Some(f) = check_to_float(c, vmax)? {
        let max = float_value(c, f)?;
        if max < 0.0 { return Ok(Draw::Val(NIL)); }
        let mut r = random_real(c, ob, rnd, true)?;
        if max > 0.0 { r *= max; }
        return Ok(Draw::Val(c.float_new(r)));
    }
    rand_range(c, ob, rnd, vmax)
}
fn invalid_argument<T>(c: &mut Ctx, v: Value) -> Result<T, Esc> {
    let s = funcall(c, v, s::to_s, &[], std::ptr::null())?;
    let st = if is_string(s) { String::from_utf8_lossy(c.str_bytes(s)).into_owned() } else { String::new() };
    arg_err(c, format!("invalid argument - {}", st))
}
fn check_random_number(c: &mut Ctx, d: Draw, a: &[Value]) -> RV {
    match d {
        Draw::False => { crate::builtins::numeric_methods::num_to_long(c, a[0])?; Ok(NIL) }
        Draw::Val(v) if v == NIL => invalid_argument(c, a.first().copied().unwrap_or(NIL)),
        Draw::Val(v) => Ok(v),
    }
}

fn r_rand(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let rnd = get_rnd(c, s);
    let d = rand_random(c, s, rnd, a)?;
    check_random_number(c, d, a)
}
fn r_random_number(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let rnd = get_rnd(c, s);
    match rand_random(c, s, rnd, a)? {
        Draw::Val(v) if v == NIL => { let r = random_real(c, s, rnd, true)?; Ok(c.float_new(r)) }
        Draw::Val(v) => Ok(v),
        Draw::False => invalid_argument(c, a[0]),
    }
}
/// Kernel#rand
pub fn rand_value(c: &mut Ctx, vmax: Value) -> RV {
    let rnd = Some(default_rnd(c));
    if vmax != NIL {
        if let Draw::Val(v) = rand_range(c, NIL, rnd, vmax)? { return Ok(v); }
        let vi = if is_float(vmax) {
            let f = float_val(vmax);
            if !f.is_finite() { return err(c, c.vm().c_float_domain_error, float_to_s(f)); }
            crate::builtins::numeric_methods::f64_to_int(c, f)
        } else { crate::builtins::numeric_methods::to_integer(c, vmax)? };
        if vi != Value::fix(0) { if let Some(v) = rand_int(c, NIL, rnd, vi, false)? { return Ok(v); } }
    }
    let r = random_real(c, NIL, rnd, true)?;
    Ok(c.float_new(r))
}
fn k_rand(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { check_argc(c, a, 0, 1)?; rand_value(c, a.first().copied().unwrap_or(NIL)) }
fn k_srand(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    let seed = if a.is_empty() { make_seed_value(c, fill_random_words(4)) } else { crate::builtins::numeric_methods::to_integer(c, a[0])? };
    let d = default_rnd(c);
    let old = rr(c, d).seed;
    rand_init(c, d, seed);
    rr(c, d).seed = seed;
    Ok(old)
}
/// srand with a plain integer (internal use)
pub fn seed(c: &mut Ctx, s: u64) { let d = default_rnd(c); let v = c.int_from_i128(s as i128); rand_init(c, d, v); rr(c, d).seed = v; }

fn bytes_of(c: &mut Ctx, r: Value, n: Value) -> RV {
    let n = crate::builtins::numeric_methods::num_to_long(c, n)?;
    if n < 0 { return arg_err(c, format!("negative string size (or size too big)")); }
    let mt = &mut rr(c, r).mt;
    let mut out = Vec::with_capacity(n as usize);
    let mut left = n as usize;
    while left > 0 { let mut x = mt.genrand_int32(); for _ in 0..left.min(4) { out.push(x as u8); x >>= 8; } left -= left.min(4); }
    Ok(c.str_from_vec_enc(out, ENC_BINARY))
}
fn r_bytes(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let Some(r) = get_rnd(c, s) else { return type_err(c, "wrong argument type") };
    bytes_of(c, r, a[0])
}
fn r_init(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    if is_frozen(s) { let ins = inspect_str(c, s)?; return err(c, c.vm().c_frozen_error, format!("can't modify frozen Random: {}", ins)); }
    if c.data::<RRandom>(s).is_none() { return type_err(c, "undefined random interface") }
    if a.is_empty() {
        let mut mt = Mt::new(); let sd = rand_init_default(c, &mut mt);
        let r = rr(c, s); r.mt = mt; r.seed = sd;
    } else {
        let sd = crate::builtins::numeric_methods::to_integer(c, a[0])?;
        // to_int may have run Ruby code: check the receiver again
        if c.data::<RRandom>(s).is_none() { return type_err(c, "undefined random interface") }
        rand_init(c, s, sd); rr(c, s).seed = sd;
    }
    Ok(s)
}
fn r_seed(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(c.data::<RRandom>(s).map(|r| r.seed).unwrap_or(NIL)) }
fn r_eq(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if real_class_of(s) != real_class_of(o) { return Ok(FALSE); }
    let (Some(x), Some(y)) = (c.data::<RRandom>(s), c.data::<RRandom>(o)) else { return Ok(FALSE) };
    if x.mt.state != y.mt.state || x.mt.next != y.mt.next || x.mt.left != y.mt.left { return Ok(FALSE); }
    Ok(Value::bool(icmp_eq(c, x.seed, y.seed)))
}
fn icmp_eq(c: &Ctx, a: Value, b: Value) -> bool { is_integer(a) && is_integer(b) && c.to_big(a) == c.to_big(b) }
fn r_init_copy(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if let Some((r, others)) = c.data_mut_and::<RRandom>(s) {
        if let Some(o) = others.data::<RRandom>(a[0]) { r.mt = o.mt.clone(); r.seed = o.seed; }
    }
    Ok(s)
}
fn s_new_seed(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(make_seed_value(c, fill_random_words(4))) }
fn s_seed(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { let d = default_rnd(c); Ok(rr(c, d).seed) }
fn s_bytes(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { bytes_of(c, default_rnd(c), a[0]) }
fn s_urandom(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative string size (or size too big)"); }
    let w = fill_random_words((n as usize + 3) / 4);
    let b: Vec<u8> = w.iter().flat_map(|x| x.to_le_bytes()).take(n as usize).collect();
    Ok(c.str_from_vec_enc(b, ENC_BINARY))
}

pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if crate::builtins::inherits(klass, c.vm().c_random) {
        let mut mt = Mt::new(); mt.init_genrand(0);
        return Ok(Some(c.alloc(klass, Body::Data(Box::new(RRandom { mt, seed: Value::fix(0) })))));
    }
    Ok(None)
}
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if let Some(r) = c.data::<RRandom>(v) {
        let (mt, seed) = (r.mt.clone(), r.seed);
        return Ok(Some(c.alloc_with(real_class_of(v), move || Body::Data(Box::new(RRandom { mt, seed })))));
    }
    Ok(None)
}

/// Boot: defines Random; returns (Random, the default generator).
pub fn init(vmr: &Vm) -> (Value, Value) {
    let base = new_class(vmr.c_object);
    class_set_alloc(base, AllocKind::None);
    let r = define_class("Random", base);
    class_set_alloc(r, AllocKind::None);
    const_set(r, intern(b"Base"), base);
    crate::vm::name_class(base, r, intern(b"Base"));
    define_private_cfunc(base, "initialize", r_init, -1);
    define_cfunc(base, "rand", r_rand, -1);
    define_cfunc(base, "bytes", r_bytes, 1);
    define_cfunc(base, "seed", r_seed, 0);
    define_private_cfunc(r, "initialize_copy", r_init_copy, 1);
    define_cfunc(r, "==", r_eq, 1);
    define_singleton_cfunc(r, "srand", k_srand, -1);
    define_singleton_cfunc(r, "rand", r_rand, -1);
    define_singleton_cfunc(r, "bytes", s_bytes, 1);
    define_singleton_cfunc(r, "seed", s_seed, 0);
    define_singleton_cfunc(r, "new_seed", s_new_seed, 0);
    define_singleton_cfunc(r, "urandom", s_urandom, 1);
    let fm = define_module_under(r, "Formatter");
    define_cfunc(fm, "random_number", r_random_number, -1);
    define_cfunc(fm, "rand", r_random_number, -1);
    include_module(base, fm);
    include_module(singleton_class(base), fm);
    define_module_function(vmr.m_kernel, "rand", k_rand, -1);
    define_module_function(vmr.m_kernel, "srand", k_srand, -1);
    let mut mt = Mt::new();
    let w = fill_random_words(4);
    mt.init_by_array(&w);
    let mut w = w; if *w.last().unwrap() <= 1 { w.push(1); }
    // no context at boot (nothing is borrowed yet)
    // top word > 1: at least 2**96, a bignum
    let sd = crate::gc::alloc(vmr.c_integer, Body::Bignum(Box::new(BigInt::from_words(false, &w))));
    crate::heap::obj_set_flags(sd, FL_FROZEN);
    let d = crate::gc::alloc_with(r, move || Body::Data(Box::new(RRandom { mt, seed: sd })));
    (r, d)
}
