// Random: MT19937 with CRuby's seeding, real/int/range draws (same numbers for the same seed).
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
    fn mark(&self, m: &mut crate::gc::Marker) { m.mark(self.seed); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

static mut DEFAULT: u64 = 0;
fn default_rnd() -> Value { unsafe { Value::from_raw(DEFAULT) } }
fn rr(v: Value) -> &'static mut RRandom { rdata::<RRandom>(v).unwrap() }

/// The generator behind obj: a Random instance, the Random class (default), or None.
fn get_rnd(obj: Value) -> Option<Value> {
    if obj == NIL || obj == vm().c_random { return Some(default_rnd()); }
    if obj.is_heap() && rdata::<RRandom>(obj).is_some() { return Some(obj); }
    None
}

fn fill_random_words(n: usize) -> Vec<u32> {
    let mut buf = vec![0u8; n * 4];
    let ok = std::fs::File::open("/dev/urandom").and_then(|mut f| { use std::io::Read; f.read_exact(&mut buf) }).is_ok();
    if !ok {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(1);
        for (i, b) in buf.iter_mut().enumerate() { *b = (t >> ((i % 16) * 8)) as u8 ^ (std::process::id() as u8).wrapping_mul(i as u8 + 1); }
    }
    buf.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}
fn make_seed_value(mut w: Vec<u32>) -> Value {
    if *w.last().unwrap() <= 1 { w.push(1); }
    big_to_value(BigInt::from_words(false, &w))
}
fn rand_init(mt: &mut Mt, seed: Value) {
    let b = to_big(seed);
    let mut w = b.mag.clone();
    if w.is_empty() { w.push(0); }
    if w.len() == 1 { mt.init_genrand(w[0]); }
    else {
        if *w.last().unwrap() == 1 { w.pop(); }
        mt.init_by_array(&w);
    }
}
fn rand_init_default(mt: &mut Mt) -> Value {
    let w = fill_random_words(4);
    mt.init_by_array(&w);
    make_seed_value(w)
}

// ---- draws (rnd = Some(generator) or None for a duck-typed object using #bytes) ------------

fn obj_bytes(c: &mut Ctx, obj: Value, n: usize) -> Result<Vec<u8>, Esc> {
    let v = funcall_public(c, obj, intern(b"bytes"), &[Value::fix(n as i64)], std::ptr::null())?;
    if !is_string(v) { return type_err(c, format!("wrong argument type {} (expected String)", class_name_for_err(v))); }
    let b = str_bytes(v).to_vec();
    if b.len() < n { return err(c, vm().c_range_error, format!("random data too short {}", b.len())); }
    if b.len() > n { return err(c, vm().c_range_error, format!("random data too long {}", b.len())); }
    Ok(b)
}
fn int32(c: &mut Ctx, obj: Value, rnd: Option<Value>) -> Result<u32, Esc> {
    match rnd { Some(r) => Ok(rr(r).mt.genrand_int32()), None => { let b = obj_bytes(c, obj, 4)?; Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]])) } }
}
fn random_real(c: &mut Ctx, obj: Value, rnd: Option<Value>, excl: bool) -> Result<f64, Esc> {
    let (a, b) = match rnd {
        Some(r) => { let m = &mut rr(r).mt; (m.genrand_int32(), m.genrand_int32()) }
        None => { let x = obj_bytes(c, obj, 8)?; (u32::from_le_bytes([x[0], x[1], x[2], x[3]]), u32::from_le_bytes([x[4], x[5], x[6], x[7]])) }
    };
    Ok(if excl { pair_to_real_excl(a, b) } else { pair_to_real_incl(a, b) })
}
fn make_mask(mut x: u64) -> u64 { x |= x >> 1; x |= x >> 2; x |= x >> 4; x |= x >> 8; x |= x >> 16; x |= x >> 32; x }
fn limited_rand(c: &mut Ctx, obj: Value, rnd: Option<Value>, limit: u64) -> Result<u64, Esc> {
    if limit == 0 { return Ok(0); }
    if rnd.is_none() {
        let w = 64 - limit.leading_zeros();
        let n = if w > 32 { 8 } else { 4 };
        let mask = if w >= 64 { u64::MAX } else { !(u64::MAX << w) };
        let full = if n == 8 { u64::MAX } else { 0xffff_ffff };
        let (mut bits, mut rest) = (0u64, 0u64);
        loop {
            if mask & !rest != 0 {
                let b = obj_bytes(c, obj, n)?;
                let mut a = [0u8; 8]; a[..n].copy_from_slice(&b);
                bits = u64::from_le_bytes(a); rest = full;
            }
            let val = bits & mask;
            bits = if w >= 64 { 0 } else { bits >> w }; rest = if w >= 64 { 0 } else { rest >> w };
            if val <= limit { return Ok(val); }
        }
    }
    Ok(limited_rand_mt(&mut rr(rnd.unwrap()).mt, limit))
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
fn limited_big_rand(c: &mut Ctx, obj: Value, rnd: Option<Value>, limit: &BigInt) -> RV {
    let lim = &limit.mag;
    let len = lim.len();
    if rnd.is_none() {
        // MS-word-first draws from #bytes
        let nlz = lim[len - 1].leading_zeros();
        let mask = u32::MAX >> nlz;
        loop {
            let b = obj_bytes(c, obj, len * 4)?;
            let mut r: Vec<u32> = b.chunks(4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]])).collect();
            r[0] &= mask;
            let msw: Vec<u32> = lim.iter().rev().copied().collect();
            let mut ok = true;
            for i in 0..len { if msw[i] < r[i] { ok = false; break; } if r[i] < msw[i] { break; } }
            if ok { r.reverse(); return Ok(big_to_value(BigInt::from_words(false, &r))); }
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
                r = int32(c, obj, rnd)? & mask;
                if boundary { if l < r { continue 'retry; } if r < l { boundary = false; } }
            }
            out[i] = r;
        }
        return Ok(big_to_value(BigInt::from_words(false, &out)));
    }
}
/// [0, n) for the default generator (Array#shuffle / #sample).
pub fn rand_below(n: u64) -> u64 {
    if n == 0 { return 0; }
    limited_rand_mt(&mut rr(default_rnd()).mt, n - 1)
}

fn rand_int(c: &mut Ctx, obj: Value, rnd: Option<Value>, vmax: Value, restrictive: bool) -> Result<Option<Value>, Esc> {
    if vmax.is_fixnum() {
        let mut max = vmax.as_fix();
        if max == 0 { return Ok(None); }
        if max < 0 { if restrictive { return Ok(None); } max = -max; }
        let r = limited_rand(c, obj, rnd, (max - 1) as u64)?;
        return Ok(Some(int_from_i128(r as i128)));
    }
    let mut b = to_big(vmax);
    if b.is_zero() { return Ok(None); }
    if b.neg { if restrictive { return Ok(None); } b = b.neg(); }
    let b = b.sub(&BigInt::from_i64(1));
    if let Some(m) = b.to_i64() { if Value::fixable(m) {
        if m == -1 { return Ok(None); }
        let r = limited_rand(c, obj, rnd, m as u64)?;
        return Ok(Some(int_from_i128(r as i128)));
    } }
    Ok(Some(limited_big_rand(c, obj, rnd, &b)?))
}

fn domain_error<T>(c: &mut Ctx) -> Result<T, Esc> {
    let k = const_get_at(vm().c_object, intern(b"Errno")).and_then(|e| const_get_at(e, intern(b"EDOM")));
    match k {
        Some(k) => { let e = funcall(c, k, s::new, &[], std::ptr::null())?; Err(raise_exc(c, e)) }
        None => err(c, vm().c_argument_error, "Numerical argument out of domain"),
    }
}
fn float_value(c: &mut Ctx, f: f64) -> Result<f64, Esc> { if !f.is_finite() { return domain_error(c); } Ok(f) }
fn check_to_int(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if is_integer(v) { return Ok(Some(v)); }
    if v.is_heap() && !is_float(v) && respond_to(v, s::to_int, true) {
        let r = funcall(c, v, s::to_int, &[], std::ptr::null())?;
        if is_integer(r) { return Ok(Some(r)); }
    }
    Ok(None)
}
fn check_to_float(c: &mut Ctx, v: Value) -> Result<Option<f64>, Esc> {
    if is_float(v) { return Ok(Some(float_val(v))); }
    if !is_kind_of(v, vm().c_numeric) && !v.is_fixnum() { return Ok(None); }
    if let Some(f) = num_to_f64(v) { return Ok(Some(f)); }
    let r = funcall(c, v, s::to_f, &[], std::ptr::null())?;
    Ok(if is_float(r) { Some(float_val(r)) } else { None })
}

enum Draw { Val(Value), False }

fn rand_range(c: &mut Ctx, obj: Value, rnd: Option<Value>, range: Value) -> Result<Draw, Esc> {
    if !is_range(range) { return Ok(Draw::False); }
    let (beg, end, excl) = crate::range::range_parts(range);
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
                if max >= 0 { let r = limited_rand(c, obj, rnd, max as u64)?; v = int_from_i128(r as i128); }
                break;
            }
            let b = to_big(vmax);
            if !b.neg && !b.is_zero() {
                let b = if excl { b.sub(&BigInt::from_i64(1)) } else { b };
                let bv = big_to_value(b.clone());
                if bv.is_fixnum() { vmax = bv; excl = false; continue; }
                v = limited_big_rand(c, obj, rnd, &b)?;
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
            let r = random_real(c, obj, rnd, excl)?;
            if scale > 1.0 { return Ok(Draw::Val(float_new((r - 0.5) * max * scale + mid))); }
            v = float_new(r * max);
        } else if max == 0.0 && !excl { v = float_new(0.0); }
    }
    if beg.is_fixnum() && v.is_fixnum() { return Ok(Draw::Val(int_from_i64(beg.as_fix() + v.as_fix()))); }
    if v == NIL { return Ok(Draw::Val(NIL)); }
    if is_bignum(v) { return Ok(Draw::Val(funcall(c, v, s::plus, &[beg], std::ptr::null())?)); }
    if is_float(v) { if let Some(f) = check_to_float(c, beg)? { return Ok(Draw::Val(float_new(float_val(v) + f))); } }
    Ok(Draw::Val(funcall(c, beg, s::plus, &[v], std::ptr::null())?))
}

fn rand_random(c: &mut Ctx, obj: Value, rnd: Option<Value>, a: &[Value]) -> Result<Draw, Esc> {
    check_argc(c, a, 0, 1)?;
    if a.is_empty() { return Ok(Draw::Val(float_new(random_real(c, obj, rnd, true)?))); }
    let vmax = a[0];
    if vmax == NIL { return Ok(Draw::Val(NIL)); }
    if !is_float(vmax) {
        if let Some(v) = check_to_int(c, vmax)? { return Ok(Draw::Val(rand_int(c, obj, rnd, v, true)?.unwrap_or(NIL))); }
    }
    if let Some(f) = check_to_float(c, vmax)? {
        let max = float_value(c, f)?;
        if max < 0.0 { return Ok(Draw::Val(NIL)); }
        let mut r = random_real(c, obj, rnd, true)?;
        if max > 0.0 { r *= max; }
        return Ok(Draw::Val(float_new(r)));
    }
    rand_range(c, obj, rnd, vmax)
}
fn invalid_argument<T>(c: &mut Ctx, v: Value) -> Result<T, Esc> {
    let s = funcall(c, v, s::to_s, &[], std::ptr::null())?;
    let st = if is_string(s) { String::from_utf8_lossy(str_bytes(s)).into_owned() } else { String::new() };
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
    let rnd = get_rnd(s);
    let d = rand_random(c, s, rnd, a)?;
    check_random_number(c, d, a)
}
fn r_random_number(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let rnd = get_rnd(s);
    match rand_random(c, s, rnd, a)? {
        Draw::Val(v) if v == NIL => Ok(float_new(random_real(c, s, rnd, true)?)),
        Draw::Val(v) => Ok(v),
        Draw::False => invalid_argument(c, a[0]),
    }
}
/// Kernel#rand
pub fn rand_value(c: &mut Ctx, vmax: Value) -> RV {
    let rnd = Some(default_rnd());
    if vmax != NIL {
        if let Draw::Val(v) = rand_range(c, NIL, rnd, vmax)? { return Ok(v); }
        let vi = if is_float(vmax) {
            let f = float_val(vmax);
            if !f.is_finite() { return err(c, vm().c_float_domain_error, float_to_s(f)); }
            crate::builtins::numeric_methods::f64_to_int(f)
        } else { crate::builtins::numeric_methods::to_integer(c, vmax)? };
        if vi != Value::fix(0) { if let Some(v) = rand_int(c, NIL, rnd, vi, false)? { return Ok(v); } }
    }
    Ok(float_new(random_real(c, NIL, rnd, true)?))
}
fn k_rand(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { check_argc(c, a, 0, 1)?; rand_value(c, a.first().copied().unwrap_or(NIL)) }
fn k_srand(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    let seed = if a.is_empty() { make_seed_value(fill_random_words(4)) } else { crate::builtins::numeric_methods::to_integer(c, a[0])? };
    let d = rr(default_rnd());
    let old = d.seed;
    rand_init(&mut d.mt, seed);
    d.seed = seed;
    Ok(old)
}
/// srand with a plain integer (internal use)
pub fn seed(s: u64) { let d = rr(default_rnd()); let v = int_from_i128(s as i128); rand_init(&mut d.mt, v); d.seed = v; }

fn bytes_of(c: &mut Ctx, r: Value, n: Value) -> RV {
    let n = crate::builtins::numeric_methods::num_to_long(c, n)?;
    if n < 0 { return arg_err(c, format!("negative string size (or size too big)")); }
    let mt = &mut rr(r).mt;
    let mut out = Vec::with_capacity(n as usize);
    let mut left = n as usize;
    while left > 0 { let mut x = mt.genrand_int32(); for _ in 0..left.min(4) { out.push(x as u8); x >>= 8; } left -= left.min(4); }
    Ok(crate::string::str_from_vec_enc(out, ENC_BINARY))
}
fn r_bytes(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let Some(r) = get_rnd(s) else { return type_err(c, "wrong argument type") };
    bytes_of(c, r, a[0])
}
fn r_init(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    if obj(s).flags & FL_FROZEN != 0 { let ins = inspect_str(c, s)?; return err(c, vm().c_frozen_error, format!("can't modify frozen Random: {}", ins)); }
    let Some(_) = rdata::<RRandom>(s) else { return type_err(c, "undefined random interface") };
    if a.is_empty() { let sd = rand_init_default(&mut rr(s).mt); rr(s).seed = sd; }
    else { let sd = crate::builtins::numeric_methods::to_integer(c, a[0])?; rand_init(&mut rr(s).mt, sd); rr(s).seed = sd; }
    Ok(s)
}
fn r_seed(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(rdata::<RRandom>(s).map(|r| r.seed).unwrap_or(NIL)) }
fn r_eq(_c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if real_class_of(s) != real_class_of(o) { return Ok(FALSE); }
    let (Some(x), Some(y)) = (rdata::<RRandom>(s), rdata::<RRandom>(o)) else { return Ok(FALSE) };
    if x.mt.state != y.mt.state || x.mt.next != y.mt.next || x.mt.left != y.mt.left { return Ok(FALSE); }
    Ok(Value::bool(icmp_eq(x.seed, y.seed)))
}
fn icmp_eq(a: Value, b: Value) -> bool { is_integer(a) && is_integer(b) && to_big(a) == to_big(b) }
fn r_init_copy(_c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if let (Some(_), Some(o)) = (rdata::<RRandom>(s), rdata::<RRandom>(a[0])) { let (mt, sd) = (o.mt.clone(), o.seed); rr(s).mt = mt; rr(s).seed = sd; }
    Ok(s)
}
fn s_new_seed(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(make_seed_value(fill_random_words(4))) }
fn s_seed(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(rr(default_rnd()).seed) }
fn s_bytes(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { bytes_of(c, default_rnd(), a[0]) }
fn s_urandom(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative string size (or size too big)"); }
    let w = fill_random_words((n as usize + 3) / 4);
    let b: Vec<u8> = w.iter().flat_map(|x| x.to_le_bytes()).take(n as usize).collect();
    Ok(crate::string::str_from_vec_enc(b, ENC_BINARY))
}

pub fn alloc_data(_c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if crate::builtins::inherits(klass, vm().c_random) {
        let mut mt = Mt::new(); mt.init_genrand(0);
        return Ok(Some(crate::gc::alloc(klass, Body::Data(Box::new(RRandom { mt, seed: Value::fix(0) })))));
    }
    Ok(None)
}
pub fn dup_data(_c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if let Some(r) = rdata::<RRandom>(v) {
        let (mt, seed) = (r.mt.clone(), r.seed);
        return Ok(Some(crate::gc::alloc(real_class_of(v), Body::Data(Box::new(RRandom { mt, seed })))));
    }
    Ok(None)
}

pub fn init() {
    let vm = vm();
    let base = new_class(vm.c_object);
    rclass(base).alloc = AllocKind::None;
    let r = define_class("Random", base);
    rclass(r).alloc = AllocKind::None;
    vm.c_random = r;
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
    define_module_function(vm.m_kernel, "rand", k_rand, -1);
    define_module_function(vm.m_kernel, "srand", k_srand, -1);
    let mut mt = Mt::new();
    let sd = rand_init_default(&mut mt);
    let d = crate::gc::alloc(r, Body::Data(Box::new(RRandom { mt, seed: sd })));
    vm.pinned.push(d);
    unsafe { DEFAULT = d.raw(); }
}
