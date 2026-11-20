// Arbitrary-precision integers: sign + magnitude (base 2^32 limbs, little-endian).
// Values that fit in a Fixnum are always normalized back to Fixnum.
use crate::object::*;
use crate::value::*;
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BigInt { pub neg: bool, pub mag: Vec<u32> }

fn trim(v: &mut Vec<u32>) { while v.last() == Some(&0) { v.pop(); } }

fn cmp_mag(a: &[u32], b: &[u32]) -> Ordering {
    if a.len() != b.len() { return a.len().cmp(&b.len()); }
    for i in (0..a.len()).rev() { if a[i] != b[i] { return a[i].cmp(&b[i]); } }
    Ordering::Equal
}
fn add_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (a, b) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut r = Vec::with_capacity(a.len() + 1);
    let mut carry = 0u64;
    for i in 0..a.len() {
        let s = a[i] as u64 + *b.get(i).unwrap_or(&0) as u64 + carry;
        r.push(s as u32); carry = s >> 32;
    }
    if carry > 0 { r.push(carry as u32); }
    r
}
// a >= b
fn sub_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut r = Vec::with_capacity(a.len());
    let mut borrow = 0i64;
    for i in 0..a.len() {
        let mut d = a[i] as i64 - *b.get(i).unwrap_or(&0) as i64 - borrow;
        if d < 0 { d += 1 << 32; borrow = 1; } else { borrow = 0; }
        r.push(d as u32);
    }
    trim(&mut r);
    r
}
fn mul_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    if a.is_empty() || b.is_empty() { return vec![]; }
    if a.len() > 48 && b.len() > 48 { return karatsuba(a, b); }
    let mut r = vec![0u32; a.len() + b.len()];
    for i in 0..a.len() {
        let mut carry = 0u64;
        let x = a[i] as u64;
        if x == 0 { continue; }
        for j in 0..b.len() {
            let t = x * b[j] as u64 + r[i + j] as u64 + carry;
            r[i + j] = t as u32; carry = t >> 32;
        }
        let mut k = i + b.len();
        while carry > 0 { let t = r[k] as u64 + carry; r[k] = t as u32; carry = t >> 32; k += 1; }
    }
    trim(&mut r);
    r
}
fn karatsuba(a: &[u32], b: &[u32]) -> Vec<u32> {
    let m = a.len().max(b.len()) / 2;
    let (a0, a1) = if a.len() > m { (&a[..m], &a[m..]) } else { (a, &[][..]) };
    let (b0, b1) = if b.len() > m { (&b[..m], &b[m..]) } else { (b, &[][..]) };
    let mut a0v = a0.to_vec(); trim(&mut a0v);
    let mut b0v = b0.to_vec(); trim(&mut b0v);
    let z0 = mul_mag(&a0v, &b0v);
    let z2 = mul_mag(a1, b1);
    let z1 = mul_mag(&add_mag(&a0v, a1), &add_mag(&b0v, b1));
    let z1 = sub_mag(&sub_mag(&z1, &z0), &z2);
    let mut r = vec![0u32; a.len() + b.len() + 1];
    let addat = |r: &mut Vec<u32>, x: &[u32], off: usize| {
        let mut carry = 0u64;
        let mut i = 0;
        while i < x.len() || carry > 0 {
            if off + i >= r.len() { r.push(0); }
            let t = r[off + i] as u64 + *x.get(i).unwrap_or(&0) as u64 + carry;
            r[off + i] = t as u32; carry = t >> 32; i += 1;
        }
    };
    addat(&mut r, &z0, 0);
    addat(&mut r, &z1, m);
    addat(&mut r, &z2, 2 * m);
    trim(&mut r);
    r
}
fn divmod_small(a: &[u32], d: u32) -> (Vec<u32>, u32) {
    let mut q = vec![0u32; a.len()];
    let mut rem = 0u64;
    for i in (0..a.len()).rev() {
        let cur = (rem << 32) | a[i] as u64;
        q[i] = (cur / d as u64) as u32;
        rem = cur % d as u64;
    }
    trim(&mut q);
    (q, rem as u32)
}
// Knuth algorithm D: magnitude division
fn divmod_mag(a: &[u32], b: &[u32]) -> (Vec<u32>, Vec<u32>) {
    if cmp_mag(a, b) == Ordering::Less { return (vec![], a.to_vec()); }
    if b.len() == 1 { let (q, r) = divmod_small(a, b[0]); return (q, if r == 0 { vec![] } else { vec![r] }); }
    let s = b[b.len() - 1].leading_zeros();
    let b = shl_mag(b, s as usize);
    let mut a = shl_mag(a, s as usize);
    if a.len() == a.len() { a.push(0); }
    let n = b.len();
    let m = a.len() - n - 1;
    let mut q = vec![0u32; m + 1];
    let btop = b[n - 1] as u64;
    let bsec = b[n - 2] as u64;
    for j in (0..=m).rev() {
        let num = ((a[j + n] as u64) << 32) | a[j + n - 1] as u64;
        let mut qhat = num / btop;
        let mut rhat = num % btop;
        while qhat >= (1 << 32) || qhat * bsec > ((rhat << 32) | a[j + n - 2] as u64) {
            qhat -= 1; rhat += btop;
            if rhat >= (1 << 32) { break; }
        }
        let mut borrow: i64 = 0;
        let mut carry: u64 = 0;
        for i in 0..n {
            let p = qhat * b[i] as u64 + carry;
            carry = p >> 32;
            let t = a[i + j] as i64 - (p & 0xffffffff) as i64 - borrow;
            a[i + j] = t as u32;
            borrow = if t < 0 { 1 } else { 0 };
        }
        let t = a[j + n] as i64 - carry as i64 - borrow;
        a[j + n] = t as u32;
        if t < 0 {
            qhat -= 1;
            let mut c = 0u64;
            for i in 0..n {
                let s2 = a[i + j] as u64 + b[i] as u64 + c;
                a[i + j] = s2 as u32; c = s2 >> 32;
            }
            a[j + n] = (a[j + n] as u64 + c) as u32;
        }
        q[j] = qhat as u32;
    }
    trim(&mut q);
    a.truncate(n);
    let mut r = shr_mag(&a, s as usize);
    trim(&mut r);
    (q, r)
}
fn shl_mag(a: &[u32], bits: usize) -> Vec<u32> {
    let words = bits / 32; let b = bits % 32;
    let mut r = vec![0u32; words];
    if b == 0 { r.extend_from_slice(a); } else {
        let mut carry = 0u32;
        for &x in a { r.push((x << b) | carry); carry = x >> (32 - b); }
        if carry > 0 { r.push(carry); }
    }
    trim(&mut r);
    r
}
fn shr_mag(a: &[u32], bits: usize) -> Vec<u32> {
    let words = bits / 32; let b = bits % 32;
    if words >= a.len() { return vec![]; }
    let a = &a[words..];
    let mut r = Vec::with_capacity(a.len());
    if b == 0 { r.extend_from_slice(a); } else {
        for i in 0..a.len() {
            let hi = if i + 1 < a.len() { a[i + 1] << (32 - b) } else { 0 };
            r.push((a[i] >> b) | hi);
        }
    }
    trim(&mut r);
    r
}

impl BigInt {
    pub fn zero() -> BigInt { BigInt { neg: false, mag: vec![] } }
    pub fn from_i64(v: i64) -> BigInt {
        let neg = v < 0;
        let u = v.unsigned_abs();
        let mut mag = vec![u as u32, (u >> 32) as u32];
        trim(&mut mag);
        BigInt { neg, mag }
    }
    pub fn from_u64(u: u64) -> BigInt { let mut mag = vec![u as u32, (u >> 32) as u32]; trim(&mut mag); BigInt { neg: false, mag } }
    pub fn from_i128(v: i128) -> BigInt {
        let neg = v < 0; let u = v.unsigned_abs();
        let mut mag = vec![u as u32, (u >> 32) as u32, (u >> 64) as u32, (u >> 96) as u32];
        trim(&mut mag);
        BigInt { neg, mag }
    }
    pub fn from_words(neg: bool, w: &[u32]) -> BigInt { let mut mag = w.to_vec(); trim(&mut mag); BigInt { neg: neg && !mag.is_empty(), mag } }
    pub fn is_zero(&self) -> bool { self.mag.is_empty() }
    fn norm(mut self) -> BigInt { trim(&mut self.mag); if self.mag.is_empty() { self.neg = false; } self }
    pub fn to_i64(&self) -> Option<i64> {
        if self.mag.len() > 2 { return None; }
        let u = self.mag.first().copied().unwrap_or(0) as u64 | (self.mag.get(1).copied().unwrap_or(0) as u64) << 32;
        if self.neg { if u <= 1u64 << 63 { Some((u as i64).wrapping_neg()) } else { None } }
        else if u <= i64::MAX as u64 { Some(u as i64) } else { None }
    }
    /// CRuby's big2dbl (bit-for-bit, including its rounding).
    pub fn to_f64(&self) -> f64 {
        let ds = &self.mag;
        let mut i = ds.len();
        let mut d = 0.0f64;
        if i > 0 {
            let mut bits = self.bit_length_mag();
            if bits > 53 + 1024 { d = f64::INFINITY; } else {
                let mut lo = 0;
                if bits > 54 { bits -= 54; lo = bits / 32; } else { bits = 0; }
                loop { i -= 1; if i <= lo { break; } d = ds[i] as f64 + 4294967296.0 * d; }
                let mut dl = ds[i];
                let round = bits != 0;
                bits %= 32;
                if round && (dl & (1u32 << bits)) != 0 {
                    let mut carry = (dl & !(u32::MAX << bits)) != 0;
                    if !carry { let mut k = i; while k > 0 { k -= 1; if ds[k] != 0 { carry = true; break; } } }
                    if carry {
                        dl &= u32::MAX << bits;
                        let (s, ov) = dl.overflowing_add(1u32 << bits);
                        dl = s;
                        if ov && dl == 0 { d += 1.0; }
                    }
                }
                d = dl as f64 + 4294967296.0 * d;
                if lo > 0 { d = ldexp(d, (lo * 32) as i32); }
            }
        }
        if self.neg { -d } else { d }
    }
    fn bit_length_mag(&self) -> usize {
        match self.mag.last() { None => 0, Some(&top) => (self.mag.len() - 1) * 32 + (32 - top.leading_zeros() as usize) }
    }
    fn low_bits_nonzero(&self, n: usize) -> bool {
        let w = n / 32;
        if self.mag[..w.min(self.mag.len())].iter().any(|&x| x != 0) { return true; }
        let b = n % 32;
        b > 0 && w < self.mag.len() && self.mag[w] & ((1u32 << b) - 1) != 0
    }
    pub fn trailing_zeros(&self) -> usize {
        let mut n = 0;
        for &w in &self.mag { if w == 0 { n += 32; } else { return n + w.trailing_zeros() as usize; } }
        n
    }
    pub fn gcd(&self, o: &BigInt) -> BigInt {
        let (mut a, mut b) = (self.abs(), o.abs());
        while !b.is_zero() {
            if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                let (mut x, mut y) = (x as u64, y as u64);
                while y != 0 { let r = x % y; x = y; y = r; }
                return BigInt::from_u64(x);
            }
            let r = a.divmod(&b).1; a = b; b = r;
        }
        a
    }
    fn fixable(&self) -> bool { self.to_i64().map(crate::value::Value::fixable).unwrap_or(false) }
    fn accurate_in_double(&self) -> bool { self.bit_length_mag() <= 53 }
    /// floor shift: n > 0 right, n < 0 left
    fn shift(&self, n: i64) -> BigInt { if n > 0 { self.shr(n as usize) } else if n < 0 { self.shl((-n) as usize) } else { self.clone() } }
    fn big_fdiv(x: &BigInt, y: &BigInt, ey: i64) -> f64 {
        let mut ex = x.bit_length_mag() as i64 - 128;
        if ex > 32 { ex -= 32; } else if ex > 0 { ex = 0; }
        let x = x.shift(ex);
        let z = x.divrem_trunc(y).0;
        let l = ex - ey;
        if l > i32::MAX as i64 { return f64::INFINITY; }
        if l < i32::MIN as i64 { return 0.0; }
        ldexp(z.to_f64(), l as i32)
    }
    fn big_fdiv_int(x: &BigInt, y: &BigInt) -> f64 {
        let ey = y.bit_length_mag() as i64 - 64;
        let y = y.shift(ey);
        BigInt::big_fdiv(x, &y, ey)
    }
    fn ddiv(x: f64, y: f64) -> f64 {
        if y != 0.0 { x / y } else if x == 0.0 { f64::NAN } else { x * (if y.is_sign_negative() { -1.0 } else { 1.0 }) * f64::INFINITY }
    }
    /// CRuby's rb_int_fdiv_double (Integer#fdiv(Integer), Rational#to_f).
    pub fn ratio_to_f64(n: &BigInt, d: &BigInt) -> f64 {
        let (mut x, mut y) = (n.clone(), d.clone());
        if !y.is_zero() && !(x.accurate_in_double() && y.accurate_in_double()) {
            let g = x.gcd(&y);
            if !g.is_zero() && g != BigInt::from_i64(1) { x = x.divrem_trunc(&g).0; y = y.divrem_trunc(&g).0; }
        }
        if x.fixable() {
            if y.fixable() && y.accurate_in_double() { return BigInt::ddiv(x.to_i64().unwrap() as f64, y.to_i64().unwrap() as f64); }
            return BigInt::big_fdiv_int(&x, &y);
        }
        let dx = x.to_f64();
        if y.fixable() {
            if dx.is_infinite() { return BigInt::big_fdiv_int(&x, &y); }
            return BigInt::ddiv(dx, y.to_i64().unwrap() as f64);
        }
        BigInt::big_fdiv_int(&x, &y)
    }
    pub fn from_f64(f: f64) -> BigInt {
        let neg = f < 0.0;
        let mut x = f.abs().trunc();
        let mut mag = vec![];
        while x >= 1.0 { let r = x % 4294967296.0; mag.push(r as u32); x = ((x - r) / 4294967296.0).trunc(); }
        BigInt { neg, mag }.norm()
    }
    pub fn neg(&self) -> BigInt { BigInt { neg: !self.neg, mag: self.mag.clone() }.norm() }
    pub fn abs(&self) -> BigInt { BigInt { neg: false, mag: self.mag.clone() } }
    pub fn add(&self, o: &BigInt) -> BigInt {
        if self.neg == o.neg { return BigInt { neg: self.neg, mag: add_mag(&self.mag, &o.mag) }.norm(); }
        match cmp_mag(&self.mag, &o.mag) {
            Ordering::Less => BigInt { neg: o.neg, mag: sub_mag(&o.mag, &self.mag) }.norm(),
            _ => BigInt { neg: self.neg, mag: sub_mag(&self.mag, &o.mag) }.norm(),
        }
    }
    pub fn sub(&self, o: &BigInt) -> BigInt { self.add(&o.neg()) }
    pub fn mul(&self, o: &BigInt) -> BigInt { BigInt { neg: self.neg != o.neg, mag: mul_mag(&self.mag, &o.mag) }.norm() }
    /// Truncating division
    pub fn divrem_trunc(&self, o: &BigInt) -> (BigInt, BigInt) {
        let (q, r) = divmod_mag(&self.mag, &o.mag);
        (BigInt { neg: self.neg != o.neg, mag: q }.norm(), BigInt { neg: self.neg, mag: r }.norm())
    }
    /// Floored division (Ruby semantics)
    pub fn divmod(&self, o: &BigInt) -> (BigInt, BigInt) {
        let (q, r) = self.divrem_trunc(o);
        if !r.is_zero() && (r.neg != o.neg) { (q.sub(&BigInt::from_i64(1)), r.add(o)) } else { (q, r) }
    }
    pub fn cmp(&self, o: &BigInt) -> Ordering {
        match (self.neg, o.neg) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_mag(&self.mag, &o.mag),
            (true, true) => cmp_mag(&o.mag, &self.mag),
        }
    }
    pub fn pow(&self, mut e: u64) -> BigInt {
        let mut base = self.clone();
        let mut r = BigInt::from_i64(1);
        while e > 0 { if e & 1 == 1 { r = r.mul(&base); } e >>= 1; if e > 0 { base = base.mul(&base); } }
        r
    }
    pub fn shl(&self, n: usize) -> BigInt {
        if !self.neg { return BigInt { neg: false, mag: shl_mag(&self.mag, n) }; }
        BigInt { neg: true, mag: shl_mag(&self.mag, n) }.norm()
    }
    /// Arithmetic shift right (floor)
    pub fn shr(&self, n: usize) -> BigInt {
        if !self.neg { return BigInt { neg: false, mag: shr_mag(&self.mag, n) }; }
        // floor for negatives: -((|x| - 1) >> n) - 1
        let m1 = BigInt { neg: false, mag: sub_mag(&self.mag, &[1]) };
        let s = BigInt { neg: false, mag: shr_mag(&m1.mag, n) };
        s.add(&BigInt::from_i64(1)).neg()
    }
    /// Two's complement words of length n (for bit ops)
    fn twos(&self, n: usize) -> Vec<u32> {
        let mut v = self.mag.clone(); v.resize(n, 0);
        if self.neg {
            for x in v.iter_mut() { *x = !*x; }
            let mut carry = 1u64;
            for x in v.iter_mut() { let t = *x as u64 + carry; *x = t as u32; carry = t >> 32; if carry == 0 { break; } }
        }
        v
    }
    fn from_twos(mut v: Vec<u32>) -> BigInt {
        let neg = v.last().map(|x| x >> 31 == 1).unwrap_or(false);
        if neg {
            for x in v.iter_mut() { *x = !*x; }
            let mut carry = 1u64;
            for x in v.iter_mut() { let t = *x as u64 + carry; *x = t as u32; carry = t >> 32; if carry == 0 { break; } }
        }
        BigInt { neg, mag: v }.norm()
    }
    pub fn bitop(&self, o: &BigInt, op: u8) -> BigInt {
        let n = self.mag.len().max(o.mag.len()) + 1;
        let a = self.twos(n); let b = o.twos(n);
        let r: Vec<u32> = a.iter().zip(b.iter()).map(|(x, y)| match op { b'&' => x & y, b'|' => x | y, _ => x ^ y }).collect();
        BigInt::from_twos(r)
    }
    pub fn not(&self) -> BigInt { self.neg().sub(&BigInt::from_i64(1)) }
    pub fn bit(&self, i: usize) -> u32 {
        let n = self.mag.len().max(i / 32 + 1) + 1;
        let t = self.twos(n);
        (t[i / 32] >> (i % 32)) & 1
    }
    pub fn bit_length(&self) -> usize {
        let m = if self.neg { BigInt { neg: false, mag: sub_mag(&self.mag, &[1]) } } else { self.clone() };
        match m.mag.last() { None => 0, Some(&top) => (m.mag.len() - 1) * 32 + (32 - top.leading_zeros() as usize) }
    }
    pub fn to_string_radix(&self, radix: u32) -> String {
        if self.mag.is_empty() { return "0".into(); }
        let mut digits: Vec<u8> = vec![];
        let (chunk, per) = match radix { 10 => (1_000_000_000u32, 9), 16 => (1 << 28, 7), 2 => (1 << 30, 30), 8 => (1 << 30, 10), _ => {
            let mut p = radix; let mut k = 1; while (p as u64) * (radix as u64) < (1u64 << 31) { p *= radix; k += 1; } (p, k) } };
        let mut cur = self.mag.clone();
        while !cur.is_empty() {
            let (q, mut r) = divmod_small(&cur, chunk);
            for _ in 0..per {
                let d = r % radix; r /= radix;
                digits.push(std::char::from_digit(d, radix).unwrap() as u8);
                if q.is_empty() && r == 0 { break; }
            }
            cur = q;
        }
        while digits.len() > 1 && *digits.last().unwrap() == b'0' { digits.pop(); }
        if self.neg { digits.push(b'-'); }
        digits.reverse();
        String::from_utf8(digits).unwrap()
    }
    pub fn parse(s: &str, radix: u32) -> Option<BigInt> {
        let s = s.trim();
        let (neg, body) = if let Some(r) = s.strip_prefix('-') { (true, r) } else if let Some(r) = s.strip_prefix('+') { (false, r) } else { (false, s) };
        if body.is_empty() { return None; }
        let mut mag: Vec<u32> = vec![];
        for ch in body.chars() {
            if ch == '_' { continue; }
            let d = ch.to_digit(radix)?;
            // mag = mag * radix + d
            let mut carry = d as u64;
            for x in mag.iter_mut() { let t = *x as u64 * radix as u64 + carry; *x = t as u32; carry = t >> 32; }
            if carry > 0 { mag.push(carry as u32); }
        }
        Some(BigInt { neg, mag }.norm())
    }
    pub fn isqrt(&self) -> BigInt {
        if self.is_zero() { return BigInt::zero(); }
        let mut x = BigInt::from_i64(1).shl((self.bit_length() + 1) / 2 + 1);
        loop {
            let (q, _) = self.divmod(&x);
            let y = x.add(&q).shr(1);
            if y.cmp(&x) != Ordering::Less { return x; }
            x = y;
        }
    }
    pub fn is_odd(&self) -> bool { self.mag.first().map(|x| x & 1 == 1).unwrap_or(false) }
}

/// x * 2^e without intermediate overflow/underflow.
pub fn ldexp(mut x: f64, mut e: i32) -> f64 {
    while e > 1000 { x *= 2f64.powi(1000); e -= 1000; if x.is_infinite() { return x; } }
    while e < -1000 { x *= 2f64.powi(-1000); e += 1000; if x == 0.0 { return x; } }
    x * 2f64.powi(e)
}

impl std::fmt::Display for BigInt {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "{}", self.to_string_radix(10)) }
}

/// Normalize: Fixnum when it fits, else a Bignum object (frozen).
pub fn big_to_value(b: BigInt) -> Value {
    if let Some(i) = b.to_i64() { if Value::fixable(i) { return Value::fix(i); } }
    let v = crate::gc::alloc(crate::vm::vm().c_integer, Body::Bignum(Box::new(b)));
    obj(v).flags |= FL_FROZEN;
    v
}

pub fn to_big(v: Value) -> BigInt {
    if v.is_fixnum() { BigInt::from_i64(v.as_fix()) } else { rbig(v).clone() }
}

pub fn int_from_i128(i: i128) -> Value {
    if i >= crate::value::FIXNUM_MIN as i128 && i <= crate::value::FIXNUM_MAX as i128 { Value::fix(i as i64) } else { big_to_value(BigInt::from_i128(i)) }
}
pub fn int_from_i64(i: i64) -> Value { if Value::fixable(i) { Value::fix(i) } else { big_to_value(BigInt::from_i64(i)) } }
