// BigDecimal (bigdecimal.so): a port of bigdecimal 4.1.3 ext/bigdecimal/bigdecimal.c (and div.h).
// The Vp* routines are kept line for line so that precision and rounding match CRuby exactly;
// lib/bigdecimal.rb of the gem (power, sqrt, BigMath.exp/log) runs on top unchanged.
// Copyright of the original: (C) 2002 Shigeo Kobayashi, under the Ruby license (BSD-2-Clause).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::bignum::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::{num_to_long, to_integer};
use crate::heap::{obj_set_flags, Global};
use crate::string::expect_str;
use crate::symbol::sym_str;
use crate::object::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

// ---- the Vp core -----------------------------------------------------------------------------

const BASE: u32 = 1_000_000_000;
const BASE_FIG: usize = 9;
const BASE1: u32 = BASE / 10;
const HALF_BASE: u32 = BASE / 2;
const DOUBLE_FIG: usize = 16;
const VP_EXPONENT_MAX: isize = isize::MAX / BASE_FIG as isize;
const VP_EXPONENT_MIN: isize = (isize::MIN + BASE_FIG as isize - 1) / BASE_FIG as isize;
const EXPONENT_MAX: isize = VP_EXPONENT_MAX * BASE_FIG as isize;
const EXPONENT_MIN: isize = VP_EXPONENT_MIN * BASE_FIG as isize - (BASE_FIG as isize - 1);
const NEWTON_RAPHSON_DIVISION_THRESHOLD: usize = 100;

const EXC_ALL: u16 = 0xff;
const EXC_INFINITY: u16 = 0x01;
const EXC_NAN: u16 = 0x02;
const EXC_UNDERFLOW: u16 = 0x04;
const EXC_OVERFLOW: u16 = 0x01;
const EXC_ZERODIVIDE: u16 = 0x10;
const EXC_OP: u16 = 0x20;
const ROUND_MODE: u16 = 0x100;
const ROUND_UP: u16 = 1;
const ROUND_DOWN: u16 = 2;
const ROUND_HALF_UP: u16 = 3;
const ROUND_HALF_DOWN: u16 = 4;
const ROUND_CEIL: u16 = 5;
const ROUND_FLOOR: u16 = 6;
const ROUND_HALF_EVEN: u16 = 7;

const SIGN_NAN: i16 = 0;
const SIGN_POSITIVE_ZERO: i16 = 1;
const SIGN_NEGATIVE_ZERO: i16 = -1;
const SIGN_POSITIVE_FINITE: i16 = 2;
const SIGN_NEGATIVE_FINITE: i16 = -2;
const SIGN_POSITIVE_INFINITE: i16 = 3;
const SIGN_NEGATIVE_INFINITE: i16 = -3;

fn roomof(n: usize, m: usize) -> usize { n.div_ceil(m) }

/// The thread-local settings a Vp routine reads (BigDecimal.limit / mode).
#[derive(Clone, Copy)]
pub struct Env { limit: usize, round: u16, exc: u16 }

/// A FloatDomainError raised by VpException, or a NoMemoryError for an impossible size.
pub enum VpErr { Fd(&'static str), NoMem }
type VR<T> = Result<T, VpErr>;

fn vp_exception(env: &Env, f: u16, s: &'static str, always: bool) -> VR<bool> {
    if always || f == EXC_OP || env.exc & f != 0 { return Err(VpErr::Fd(s)); }
    Ok(false)
}

/// struct Real: 0.frac[0] frac[1] ... * BASE**exponent; frac has max_prec usable words.
#[derive(Clone)]
pub struct Real { max_prec: usize, prec: usize, exponent: isize, sign: i16, frac: Vec<u32> }

impl Real {
    fn alloc(n: usize) -> Real { Real { max_prec: n, prec: 0, exponent: 0, sign: 0, frac: vec![0; n.max(1) + 1] } }
    fn zero_wrap(sign: i32, digits: usize) -> Real { let mut r = Real::alloc(roomof(digits, BASE_FIG)); r.set_zero(sign); r }
    fn is_nan(&self) -> bool { self.sign == SIGN_NAN }
    fn is_pos_inf(&self) -> bool { self.sign == SIGN_POSITIVE_INFINITE }
    fn is_neg_inf(&self) -> bool { self.sign == SIGN_NEGATIVE_INFINITE }
    fn is_inf(&self) -> bool { self.is_pos_inf() || self.is_neg_inf() }
    fn is_def(&self) -> bool { !(self.is_nan() || self.is_inf()) }
    fn is_pos_zero(&self) -> bool { self.sign == SIGN_POSITIVE_ZERO }
    fn is_neg_zero(&self) -> bool { self.sign == SIGN_NEGATIVE_ZERO }
    fn is_zero(&self) -> bool { self.is_pos_zero() || self.is_neg_zero() }
    fn is_one(&self) -> bool { self.prec == 1 && self.frac[0] == 1 && self.exponent == 1 }
    fn has_val(&self) -> bool { self.frac[0] != 0 }
    fn get_sign(&self) -> i32 { if self.sign > 0 { 1 } else { -1 } }
    fn set_sign(&mut self, s: i32) { self.sign = if s > 0 { SIGN_POSITIVE_FINITE } else { SIGN_NEGATIVE_FINITE }; }
    fn change_sign(&mut self, s: i32) { self.sign = if s > 0 { self.sign.abs() } else { -self.sign.abs() }; }
    fn set_special(&mut self, sign: i16) { self.frac[0] = 0; self.prec = 1; self.exponent = 0; self.sign = sign; }
    fn set_zero(&mut self, s: i32) { self.set_special(if s > 0 { SIGN_POSITIVE_ZERO } else { SIGN_NEGATIVE_ZERO }); }
    fn set_nan(&mut self) { self.set_special(SIGN_NAN); }
    fn set_inf(&mut self, s: i32) { self.set_special(if s > 0 { SIGN_POSITIVE_INFINITE } else { SIGN_NEGATIVE_INFINITE }); }
    fn set_one(&mut self) { self.prec = 1; self.exponent = 1; self.frac[0] = 1; self.sign = SIGN_POSITIVE_FINITE; }
    fn ensure(&mut self, n: usize) { if self.frac.len() < n { self.frac.resize(n, 0); } }
    /// zero_wrap for a computed size: CRuby's allocation fails with NoMemoryError long before the 8GB cap here
    fn zw(sign: i32, digits: usize) -> VR<Real> {
        if roomof(digits, BASE_FIG) >= (1usize << 31) { return Err(VpErr::NoMem); }
        Ok(Real::zero_wrap(sign, digits))
    }
    fn one() -> Real { let mut r = Real::alloc(1); r.set_one(); r }
}

fn add_exponent(a: &mut Real, n: isize, env: &Env) -> VR<bool> {
    let e = a.exponent;
    let m = e.wrapping_add(n);
    let over = if e > 0 && n > 0 { if n > VP_EXPONENT_MAX - e { Some(true) } else { None } }
        else if e < 0 && n < 0 { if n < VP_EXPONENT_MIN - e { Some(false) } else { None } }
        else if m > VP_EXPONENT_MAX { Some(true) } else if m < VP_EXPONENT_MIN { Some(false) } else { None };
    match over {
        None => { a.exponent = m; Ok(true) }
        Some(false) => { let s = a.get_sign(); a.set_zero(s); vp_exception(env, EXC_UNDERFLOW, "Exponent underflow", false) }
        Some(true) => { let s = a.get_sign(); a.set_inf(s); vp_exception(env, EXC_OVERFLOW, "Exponent overflow", false) }
    }
}

fn vp_nmlz(a: &mut Real, env: &Env) -> VR<bool> {
    if !a.is_def() || a.is_zero() { a.frac[0] = 0; a.prec = 1; a.exponent = 0; return Ok(false); }
    let mut ind = a.prec;
    while ind > 0 {
        ind -= 1;
        if a.frac[ind] != 0 {
            a.prec = ind + 1;
            let mut i = 0;
            while a.frac[i] == 0 { i += 1; }
            if i > 0 {
                a.prec -= i;
                if !add_exponent(a, -(i as isize), env)? { return Ok(false); }
                let p = a.prec;
                a.frac.copy_within(i..i + p, 0);
            }
            return Ok(true);
        }
    }
    let s = a.get_sign();
    a.set_zero(s);
    Ok(false)
}

fn vp_asgn(c: &mut Real, a: &Real, isw: i32, env: &Env) -> VR<usize> {
    if a.is_nan() { c.set_nan(); return Ok(0); }
    if a.is_inf() { c.set_inf(isw * a.get_sign()); return Ok(0); }
    if !a.is_zero() {
        c.exponent = a.exponent;
        c.set_sign(isw * a.get_sign());
        let n = a.prec.min(c.max_prec);
        c.prec = n;
        c.ensure(n + 1);
        c.frac[..n].copy_from_slice(&a.frac[..n]);
        if isw != 10 && isw != -10 {
            if c.prec < a.prec { vp_internal_round(c, n, if n > 0 { a.frac[n - 1] } else { 0 }, a.frac[n], env)?; }
            else { vp_limit_round(c, 0, env)?; }
        }
    } else {
        c.set_zero(isw * a.get_sign());
        return Ok(1);
    }
    Ok(c.prec * BASE_FIG)
}

#[derive(Clone, Copy, PartialEq)]
enum Op { Add, Sub, Mult, Div }

fn vp_is_def_op(c: &mut Real, a: &Real, b: &Real, sw: Op, env: &Env) -> VR<bool> {
    enum R { Nan, Inf }
    let r = 'r: {
        if a.is_nan() || b.is_nan() { c.set_nan(); break 'r R::Nan; }
        if a.is_inf() {
            if b.is_inf() {
                match sw {
                    Op::Add => { if a.get_sign() == b.get_sign() { c.set_inf(a.get_sign()); break 'r R::Inf; } c.set_nan(); break 'r R::Nan; }
                    Op::Sub => { if a.get_sign() != b.get_sign() { c.set_inf(a.get_sign()); break 'r R::Inf; } c.set_nan(); break 'r R::Nan; }
                    Op::Mult => { c.set_inf(a.get_sign() * b.get_sign()); break 'r R::Inf; }
                    Op::Div => { c.set_nan(); break 'r R::Nan; }
                }
            }
            match sw {
                Op::Add | Op::Sub => c.set_inf(a.get_sign()),
                Op::Mult => { if b.is_zero() { c.set_nan(); break 'r R::Nan; } c.set_inf(a.get_sign() * b.get_sign()); }
                Op::Div => c.set_inf(a.get_sign() * b.get_sign()),
            }
            break 'r R::Inf;
        }
        if b.is_inf() {
            match sw {
                Op::Add => c.set_inf(b.get_sign()),
                Op::Sub => c.set_inf(-b.get_sign()),
                Op::Mult => { if a.is_zero() { c.set_nan(); break 'r R::Nan; } c.set_inf(a.get_sign() * b.get_sign()); }
                Op::Div => c.set_zero(a.get_sign() * b.get_sign()),
            }
            break 'r R::Inf;
        }
        return Ok(true);
    };
    match r {
        R::Inf => if c.is_pos_inf() { vp_exception(env, EXC_INFINITY, "Computation results to 'Infinity'", false) }
                  else { vp_exception(env, EXC_INFINITY, "Computation results to '-Infinity'", false) },
        R::Nan => vp_exception(env, EXC_NAN, "Computation results to 'NaN'", false),
    }
}

fn vp_add_sub(c: &mut Real, a: &Real, b: &Real, operation: i32, env: &Env) -> VR<usize> {
    if !vp_is_def_op(c, a, b, if operation > 0 { Op::Add } else { Op::Sub }, env)? { return Ok(0); }
    if a.is_zero() {
        if !b.is_zero() { vp_asgn(c, b, operation, env)?; }
        else {
            if a.get_sign() < 0 && operation * b.get_sign() < 0 { c.set_zero(-1); } else { c.set_zero(1); }
            return Ok(1);
        }
        return Ok(c.prec * BASE_FIG);
    }
    if b.is_zero() { vp_asgn(c, a, 1, env)?; return Ok(c.prec * BASE_FIG); }
    let sw: i32 = if operation < 0 { -1 } else { 1 };
    let a_is_first: bool;
    if a.exponent > b.exponent { a_is_first = true; }
    else if a.exponent < b.exponent { a_is_first = false; }
    else {
        let (na, nb) = (a.prec, b.prec);
        let n = na.min(nb);
        let mut decided = None;
        for i in 0..n {
            if a.frac[i] > b.frac[i] { decided = Some(true); break; }
            if a.frac[i] < b.frac[i] { decided = Some(false); break; }
        }
        a_is_first = match decided {
            Some(x) => x,
            None if na > nb => true,
            None if na < nb => false,
            None => {
                if a.get_sign() + sw * b.get_sign() == 0 { c.set_zero(1); return Ok(c.prec * BASE_FIG); }
                true
            }
        };
    }
    let (ap, bp) = if a_is_first { (a, b) } else { (b, a) };
    let isw = a.get_sign() + sw * b.get_sign();
    let sign;
    let mrv;
    if isw != 0 {
        c.set_sign(1);
        mrv = vp_add_abs(ap, bp, c, env)?;
        sign = isw / 2;
    } else {
        c.set_sign(1);
        mrv = vp_sub_abs(ap, bp, c, env)?;
        sign = if a_is_first { a.get_sign() } else { ap.get_sign() * sw };
    }
    if c.is_inf() { c.set_inf(sign); }
    else {
        c.set_sign(sign);
        let prev = if c.prec > 0 { c.frac[c.prec - 1] } else { 0 };
        vp_internal_round(c, 0, prev, mrv, env)?;
    }
    Ok(c.prec * BASE_FIG)
}

struct Ptr { word_shift: usize, a_pos: usize, b_pos: usize, c_pos: usize, av: u32, bv: u32 }

fn vp_set_ptr(a: &Real, b: &Real, c: &mut Real, env: &Env) -> VR<Ptr> {
    let round_limit = (env.limit + BASE_FIG - 1) / BASE_FIG;
    c.frac[0] = 0;
    let (mut av, mut bv) = (0, 0);
    let word_shift = (a.exponent.wrapping_sub(b.exponent)) as usize;
    let mut left_word = b.prec.wrapping_add(word_shift);
    let mut right_word = a.prec.max(left_word);
    left_word = c.max_prec.wrapping_sub(1);
    let (a_pos, b_pos, c_pos);
    if right_word > left_word {
        right_word = left_word + 1;
        c_pos = right_word;
        if a.prec >= c.max_prec {
            a_pos = left_word;
            if a_pos <= round_limit { av = a.frac[a_pos]; }
        } else { a_pos = a.prec; }
        if b.prec.wrapping_add(word_shift) >= c.max_prec {
            if c.max_prec >= word_shift.wrapping_add(1) {
                b_pos = c.max_prec - word_shift - 1;
                if b_pos + word_shift <= round_limit { bv = b.frac[b_pos]; }
            } else { b_pos = usize::MAX; }
        } else { b_pos = b.prec; }
    } else {
        b_pos = b.prec;
        a_pos = a.prec;
        c_pos = right_word + 1;
    }
    c.prec = c_pos;
    c.ensure(c_pos + 1);
    c.exponent = a.exponent;
    if !add_exponent(c, 1, env)? { return Ok(Ptr { word_shift: usize::MAX, a_pos, b_pos, c_pos, av, bv }); }
    Ok(Ptr { word_shift, a_pos, b_pos, c_pos, av, bv })
}

fn vp_add_abs(a: &Real, b: &Real, c: &mut Real, env: &Env) -> VR<u32> {
    let Ptr { mut word_shift, mut a_pos, mut b_pos, mut c_pos, av, bv } = vp_set_ptr(a, b, c, env)?;
    if word_shift == usize::MAX { return Ok(0); }
    if b_pos == usize::MAX { vp_asgn(c, a, 1, env)?; return Ok(0); }
    let mrv = av.wrapping_add(bv);
    if b_pos > 0 {
        while b_pos > 0 && b_pos + word_shift > a_pos { c_pos -= 1; b_pos -= 1; c.frac[c_pos] = b.frac[b_pos]; }
    }
    if b_pos == 0 && word_shift > a_pos {
        loop {
            let go = word_shift > a_pos;
            word_shift = word_shift.wrapping_sub(1);
            if !go { break; }
            c_pos -= 1; c.frac[c_pos] = 0;
        }
    }
    let bpws = b_pos.wrapping_add(word_shift);
    while a_pos > bpws { c_pos -= 1; a_pos -= 1; c.frac[c_pos] = a.frac[a_pos]; }
    let mut carry = 0u32;
    while b_pos > 0 {
        c_pos -= 1; a_pos -= 1; b_pos -= 1;
        let s = a.frac[a_pos].wrapping_add(b.frac[b_pos]).wrapping_add(carry);
        if s >= BASE { c.frac[c_pos] = s - BASE; carry = 1; } else { c.frac[c_pos] = s; carry = 0; }
    }
    while a_pos > 0 {
        c_pos -= 1; a_pos -= 1;
        let s = a.frac[a_pos].wrapping_add(carry);
        if s >= BASE { c.frac[c_pos] = s - BASE; carry = 1; } else { c.frac[c_pos] = s; carry = 0; }
    }
    if c_pos > 0 { c.frac[c_pos - 1] = c.frac[c_pos - 1].wrapping_add(carry); }
    Ok(mrv)
}

fn vp_sub_abs(a: &Real, b: &Real, c: &mut Real, env: &Env) -> VR<u32> {
    let Ptr { mut word_shift, mut a_pos, mut b_pos, mut c_pos, av, bv } = vp_set_ptr(a, b, c, env)?;
    if word_shift == usize::MAX { return Ok(0); }
    if b_pos == usize::MAX { vp_asgn(c, a, 1, env)?; return Ok(0); }
    let (mrv, mut borrow) = if av >= bv { (av - bv, 0u32) } else { (0, 1u32) };
    if b_pos + word_shift > a_pos {
        while b_pos > 0 && b_pos + word_shift > a_pos {
            c_pos -= 1; b_pos -= 1;
            c.frac[c_pos] = BASE.wrapping_sub(b.frac[b_pos]).wrapping_sub(borrow);
            borrow = 1;
        }
        if b_pos == 0 {
            while word_shift > a_pos { word_shift -= 1; c_pos -= 1; c.frac[c_pos] = BASE - borrow; borrow = 1; }
        }
    }
    let bpws = b_pos.wrapping_add(word_shift);
    while a_pos > bpws { c_pos -= 1; a_pos -= 1; c.frac[c_pos] = a.frac[a_pos]; }
    while b_pos > 0 {
        c_pos -= 1; a_pos -= 1; b_pos -= 1;
        if a.frac[a_pos] < b.frac[b_pos].wrapping_add(borrow) {
            c.frac[c_pos] = BASE.wrapping_add(a.frac[a_pos]).wrapping_sub(b.frac[b_pos]).wrapping_sub(borrow);
            borrow = 1;
        } else {
            c.frac[c_pos] = a.frac[a_pos].wrapping_sub(b.frac[b_pos]).wrapping_sub(borrow);
            borrow = 0;
        }
    }
    while a_pos > 0 {
        c_pos -= 1; a_pos -= 1;
        if a.frac[a_pos] < borrow { c.frac[c_pos] = BASE.wrapping_add(a.frac[a_pos]).wrapping_sub(borrow); borrow = 1; }
        else { c.frac[c_pos] = a.frac[a_pos] - borrow; borrow = 0; }
    }
    if c_pos > 0 { c.frac[c_pos - 1] = c.frac[c_pos - 1].wrapping_sub(borrow); }
    Ok(mrv)
}

/// Exact product of the word arrays (what both the batched loop and the NTT path compute).
fn mul_words(a: &[u32], b: &[u32], out: &mut [u32]) {
    if a.len() < KARATSUBA_MIN || b.len() < KARATSUBA_MIN {
        for x in out.iter_mut() { *x = 0; }
        for i in (0..a.len()).rev() {
            let ai = a[i] as u64;
            if ai == 0 { continue; }
            let mut carry = 0u64;
            for j in (0..b.len()).rev() {
                let t = out[i + j + 1] as u64 + ai * b[j] as u64 + carry;
                out[i + j + 1] = (t % BASE as u64) as u32;
                carry = t / BASE as u64;
            }
            out[i] = (out[i] as u64 + carry) as u32;
        }
        return;
    }
    // big-endian words -> little-endian for Karatsuba
    let la: Vec<u32> = a.iter().rev().copied().collect();
    let lb: Vec<u32> = b.iter().rev().copied().collect();
    let p = mul_le(&la, &lb);
    let n = out.len();
    for (i, x) in out.iter_mut().enumerate() { *x = p.get(n - 1 - i).copied().unwrap_or(0); }
}

const KARATSUBA_MIN: usize = 48;

fn school_le(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut r = vec![0u32; a.len() + b.len()];
    for (i, &x) in a.iter().enumerate() {
        if x == 0 { continue; }
        let mut carry = 0u64;
        for (j, &y) in b.iter().enumerate() {
            let t = r[i + j] as u64 + x as u64 * y as u64 + carry;
            r[i + j] = (t % BASE as u64) as u32;
            carry = t / BASE as u64;
        }
        let mut k = i + b.len();
        while carry > 0 { let t = r[k] as u64 + carry; r[k] = (t % BASE as u64) as u32; carry = t / BASE as u64; k += 1; }
    }
    r
}
/// dst += src (little-endian base 1e9, dst long enough)
fn add_le(dst: &mut [u32], src: &[u32]) {
    let mut carry = 0u32;
    let mut i = 0;
    while i < src.len() || carry > 0 {
        let t = dst[i] + carry + src.get(i).copied().unwrap_or(0);
        if t >= BASE { dst[i] = t - BASE; carry = 1; } else { dst[i] = t; carry = 0; }
        i += 1;
    }
}
/// dst -= src (dst >= src)
fn sub_le(dst: &mut [u32], src: &[u32]) {
    let mut borrow = 0u32;
    let mut i = 0;
    while i < src.len() || borrow > 0 {
        let s = src.get(i).copied().unwrap_or(0) + borrow;
        if dst[i] >= s { dst[i] -= s; borrow = 0; } else { dst[i] = dst[i] + BASE - s; borrow = 1; }
        i += 1;
    }
}
fn mul_le(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (a, b) = if a.len() < b.len() { (b, a) } else { (a, b) };
    if b.len() < KARATSUBA_MIN { return school_le(a, b); }
    let mut r = vec![0u32; a.len() + b.len() + 1];
    if a.len() >= 2 * b.len() {
        // unbalanced: b times slices of a
        let mut off = 0;
        while off < a.len() {
            let e = (off + b.len()).min(a.len());
            let p = mul_le(&a[off..e], b);
            add_le(&mut r[off..], &p);
            off = e;
        }
        r.truncate(a.len() + b.len());
        return r;
    }
    let k = a.len() / 2;
    let (a0, a1) = a.split_at(k);
    let (b0, b1) = b.split_at(k.min(b.len()));
    let z0 = mul_le(a0, b0);
    let z2 = mul_le(a1, b1);
    let mut sa = a0.to_vec(); sa.push(0); add_le(&mut sa, a1);
    let mut sb = b0.to_vec(); sb.resize(b0.len().max(b1.len()) + 1, 0); add_le(&mut sb, b1);
    let mut z1 = mul_le(&sa, &sb);
    sub_le(&mut z1, &z0);
    sub_le(&mut z1, &z2);
    add_le(&mut r, &z0);
    let z1n = z1.iter().rposition(|&x| x != 0).map(|p| p + 1).unwrap_or(0);
    add_le(&mut r[k..], &z1[..z1n]);
    add_le(&mut r[2 * k..], &z2);
    r.truncate(a.len() + b.len());
    r
}

fn vp_mult(c: &mut Real, a: &Real, b: &Real, env: &Env) -> VR<usize> {
    if !vp_is_def_op(c, a, b, Op::Mult, env)? { return Ok(0); }
    if a.is_zero() || b.is_zero() { c.set_zero(a.get_sign() * b.get_sign()); return Ok(1); }
    if a.is_one() { vp_asgn(c, b, 10 * a.get_sign(), env)?; return Ok(c.prec * BASE_FIG); }
    if b.is_one() { vp_asgn(c, a, 10 * b.get_sign(), env)?; return Ok(c.prec * BASE_FIG); }
    let (a, b) = if b.prec > a.prec { (b, a) } else { (a, b) };
    c.exponent = a.exponent;
    c.set_sign(a.get_sign() * b.get_sign());
    if !add_exponent(c, b.exponent, env)? { return Ok(0); }
    c.prec = a.prec + b.prec;
    c.ensure(c.prec + 1);
    let p = c.prec;
    mul_words(&a.frac[..a.prec], &b.frac[..b.prec], &mut c.frac[..p]);
    vp_nmlz(c, env)?;
    Ok(c.prec * BASE_FIG)
}

fn vp_divd(c: &mut Real, r: &mut Real, a: &Real, b: &Real, env: &Env) -> VR<usize> {
    r.set_nan();
    if !vp_is_def_op(c, a, b, Op::Div, env)? { return Ok(c.prec * BASE_FIG); }
    if a.is_zero() && b.is_zero() { c.set_nan(); vp_exception(env, EXC_NAN, "Computation results to 'NaN'", false)?; return Ok(0); }
    if b.is_zero() { c.set_inf(a.get_sign() * b.get_sign()); vp_exception(env, EXC_ZERODIVIDE, "Divide by zero", false)?; return Ok(0); }
    if a.is_zero() {
        c.set_zero(a.get_sign() * b.get_sign());
        r.set_zero(a.get_sign() * b.get_sign());
        return Ok(c.prec * BASE_FIG);
    }
    let word_a = a.prec;
    let word_b = b.prec;
    let word_c = c.max_prec;
    let word_r = r.max_prec;
    if word_a > word_r || word_b + word_c - 2 >= word_r { panic!("ERROR(VpDivd): space for remainder too small."); }
    if word_c >= NEWTON_RAPHSON_DIVISION_THRESHOLD && word_b >= NEWTON_RAPHSON_DIVISION_THRESHOLD {
        vp_divd_newton(c, r, a, b, env)?;
        return Ok(c.prec * BASE_FIG);
    }
    r.ensure(word_r + 1);
    c.ensure(word_c + 1);
    r.frac[..word_a].copy_from_slice(&a.frac[..word_a]);
    for i in word_a..word_r { r.frac[i] = 0; }
    for i in 0..word_c { c.frac[i] = 0; }
    let base = BASE as u64;
    let b1 = b.frac[0] as u64;
    let mut b1p1 = b1;
    let (b1b2, b1b2p1);
    if b.prec <= 1 { b1b2 = b1p1 * base; b1b2p1 = b1b2; }
    else {
        b1p1 = b1 + 1;
        b1b2 = b1 * base + b.frac[1] as u64;
        b1b2p1 = if b.prec > 2 { b1b2 + 1 } else { b1b2 };
    }
    let n_loop = word_c.min(word_r);
    let mut ind_c = 0;
    'outer: while ind_c < n_loop {
        if r.frac[ind_c] == 0 { ind_c += 1; continue; }
        let r1r2 = r.frac[ind_c] as u64 * base + if ind_c + 1 < word_r { r.frac[ind_c + 1] as u64 } else { 0 };
        let q: u64;
        let mut ind_r: usize;
        // 0: q = 1 at ind_c, 1: q = r1r2 / b1b2p1 at ind_c, 2: div_b1p1
        let mut path = 2;
        if r1r2 == b1b2 {
            let (mut ind_b, mut ind_a) = (2, ind_c + 2);
            path = 0;
            while ind_b < word_b {
                if r.frac[ind_a] < b.frac[ind_b] { path = 2; break; }
                if r.frac[ind_a] > b.frac[ind_b] { break; }
                ind_a += 1; ind_b += 1;
            }
        } else if r1r2 >= b1b2p1 { path = 1; }
        match path {
            0 => { q = 1; c.frac[ind_c] = c.frac[ind_c].wrapping_add(1); ind_r = b.prec + ind_c - 1; }
            1 => { q = r1r2 / b1b2p1; c.frac[ind_c] = c.frac[ind_c].wrapping_add(q as u32); ind_r = b.prec + ind_c - 1; }
            _ => {
                if ind_c + 1 >= word_c { break 'outer; }
                q = r1r2 / b1p1;
                c.frac[ind_c + 1] = c.frac[ind_c + 1].wrapping_add(q as u32);
                ind_r = b.prec + ind_c;
            }
        }
        // sub_mult
        let mut borrow2: u32 = 0;
        let mut ind_b = word_b.wrapping_sub(1);
        if ind_r >= word_r { panic!("ERROR(VpDivd): space for remainder too small."); }
        let n = ind_b;
        for _ in 0..=n {
            let mut qb = q * b.frac[ind_b] as u64;
            let borrow1: u32;
            if qb < base { borrow1 = 0; } else { borrow1 = (qb / base) as u32; qb -= borrow1 as u64 * base; }
            if (r.frac[ind_r] as u64) < qb {
                r.frac[ind_r] = r.frac[ind_r].wrapping_add((base - qb) as u32);
                borrow2 = borrow2.wrapping_add(borrow1).wrapping_add(1);
            } else {
                r.frac[ind_r] = r.frac[ind_r].wrapping_sub(qb as u32);
                borrow2 = borrow2.wrapping_add(borrow1);
            }
            if borrow2 != 0 {
                if r.frac[ind_r - 1] < borrow2 { r.frac[ind_r - 1] = r.frac[ind_r - 1].wrapping_add(BASE.wrapping_sub(borrow2)); borrow2 = 1; }
                else { r.frac[ind_r - 1] -= borrow2; borrow2 = 0; }
            }
            ind_r = ind_r.wrapping_sub(1);
            ind_b = ind_b.wrapping_sub(1);
        }
        if borrow2 != 0 { r.frac[ind_r] = r.frac[ind_r].wrapping_sub(borrow2); }
    }
    // out_side
    c.prec = word_c;
    c.exponent = a.exponent;
    c.set_sign(a.get_sign() * b.get_sign());
    if !add_exponent(c, 1, env)? { return Ok(0); }
    if !add_exponent(c, -b.exponent, env)? { return Ok(0); }
    vp_nmlz(c, env)?;
    r.prec = word_r;
    r.exponent = a.exponent;
    r.set_sign(a.get_sign());
    vp_nmlz(r, env)?;
    Ok(c.prec * BASE_FIG)
}

fn words_to_big(w: &[u32]) -> BigInt {
    let mut s = String::with_capacity(w.len() * BASE_FIG);
    for (i, x) in w.iter().enumerate() { if i == 0 { s.push_str(&x.to_string()); } else { s.push_str(&format!("{:09}", x)); } }
    BigInt::parse(&s, 10).unwrap_or_else(BigInt::zero)
}
/// A non-negative integer as a normalized Real.
fn big_to_real(b: &BigInt, env: &Env) -> VR<Real> {
    let s = b.to_string_radix(10);
    Ok(vp_alloc(s.as_bytes(), true, env)?.ok().expect("digits"))
}

// VpDivdNewton computes the exact integer divmod (div.h); the same quotient and remainder by BigInt.
fn vp_divd_newton(c: &mut Real, r: &mut Real, a: &Real, b: &Real, env: &Env) -> VR<()> {
    let env2 = Env { limit: 0, round: ROUND_HALF_UP, exc: env.exc };
    let div_prec = c.max_prec - 1;
    let base_prec = b.prec;
    let ex = base_prec + div_prec;
    // a2 = |a| as an integer of `ex` words (plus a fraction), b2 = |b| as an integer of b.prec words
    let (int_w, frac_w): (Vec<u32>, Vec<u32>) = if a.prec > ex { (a.frac[..ex].to_vec(), a.frac[ex..a.prec].to_vec()) }
        else { let mut v = a.frac[..a.prec].to_vec(); v.resize(ex, 0); (v, vec![]) };
    let a2 = words_to_big(&int_w);
    let b2 = words_to_big(&b.frac[..b.prec]);
    let (q, m) = a2.divmod(&b2);
    let c2 = big_to_real(&q, &env2)?;
    let mut r2 = big_to_real(&m, &env2)?;
    if !frac_w.is_empty() {
        let mut f = Real::alloc(frac_w.len());
        f.frac[..frac_w.len()].copy_from_slice(&frac_w);
        f.prec = frac_w.len(); f.exponent = 0; f.set_sign(1);
        vp_nmlz(&mut f, &env2)?;
        if f.is_zero() { f.set_zero(1); }
        let mx = get_add_sub_prec(&r2, &f);
        let mut s = Real::zw(1, (mx + 1) * BASE_FIG)?;
        vp_add_sub(&mut s, &r2, &f, 1, &env2)?;
        r2 = s;
    }
    vp_asgn(c, &c2, a.get_sign() * b.get_sign(), &env2)?;
    vp_asgn(r, &r2, a.get_sign(), &env2)?;
    if !c.is_zero() { add_exponent(c, a.exponent - b.exponent - div_prec as isize, &env2)?; }
    if !r.is_zero() { add_exponent(r, a.exponent - ex as isize, &env2)?; }
    Ok(())
}

fn vp_comp(a: &Real, b: &Real) -> i32 {
    if a.is_nan() || b.is_nan() { return 999; }
    if !a.is_def() {
        let e = if !b.is_def() { a.sign as i32 - b.sign as i32 } else { a.sign as i32 };
        return e.signum();
    }
    if !b.is_def() { return if -(b.sign as i32) > 0 { 1 } else { -1 }; }
    let val: i32 = 'v: {
        if a.is_zero() { if b.is_zero() { return 0; } break 'v -b.get_sign(); }
        if b.is_zero() { break 'v a.get_sign(); }
        if a.get_sign() > b.get_sign() { break 'v 1; }
        if a.get_sign() < b.get_sign() { break 'v -1; }
        if a.exponent > b.exponent { break 'v a.get_sign(); }
        if a.exponent < b.exponent { break 'v -b.get_sign(); }
        let mx = a.prec.min(b.prec);
        for i in 0..mx {
            if a.frac[i] > b.frac[i] { break 'v a.get_sign(); }
            if a.frac[i] < b.frac[i] { break 'v -b.get_sign(); }
        }
        if a.prec > b.prec { a.get_sign() } else if a.prec < b.prec { -b.get_sign() } else { 0 }
    };
    val.clamp(-1, 1)
}

fn vp_exponent10(a: &Real) -> isize {
    if !a.has_val() { return 0; }
    let mut ex = a.exponent.wrapping_mul(BASE_FIG as isize);
    let mut n = BASE1;
    while a.frac[0] / n == 0 { ex -= 1; n /= 10; }
    ex
}

fn push_digits(out: &mut Vec<u8>, a: &Real, zero_sup: &mut bool) {
    for i in 0..a.prec {
        let mut m = BASE1;
        let mut e = a.frac[i];
        while m != 0 {
            let nn = e / m;
            if !*zero_sup || nn != 0 { out.push(b'0' + nn as u8); *zero_sup = false; }
            e -= nn * m;
            m /= 10;
        }
    }
}

fn vp_sz_mantissa(a: &Real) -> Vec<u8> {
    if a.is_nan() { return b"NaN".to_vec(); }
    if a.is_pos_inf() { return b"Infinity".to_vec(); }
    if a.is_neg_inf() { return b"-Infinity".to_vec(); }
    if a.is_zero() { return if a.is_pos_zero() { b"0".to_vec() } else { b"-0".to_vec() }; }
    let mut out = vec![];
    if a.sign < 0 { out.push(b'-'); }
    let mut zs = true;
    push_digits(&mut out, a, &mut zs);
    while out.last() == Some(&b'0') { out.pop(); }
    out
}

fn vp_to_special_string(a: &Real, f_plus: i32) -> Option<Vec<u8>> {
    if a.is_nan() { return Some(b"NaN".to_vec()); }
    if a.is_pos_inf() { return Some(match f_plus { 1 => b" Infinity".to_vec(), 2 => b"+Infinity".to_vec(), _ => b"Infinity".to_vec() }); }
    if a.is_neg_inf() { return Some(b"-Infinity".to_vec()); }
    if a.is_zero() {
        return Some(if a.is_pos_zero() { match f_plus { 1 => b" 0.0".to_vec(), 2 => b"+0.0".to_vec(), _ => b"0.0".to_vec() } } else { b"-0.0".to_vec() });
    }
    None
}

fn vp_format_st(s: &mut Vec<u8>, f_fmt: usize) {
    if f_fmt == 0 { return; }
    let idig = match s.iter().position(|&c| c == b'.') { Some(p) => p + 1, None => 0 };
    let iexp = s.iter().position(|&c| c == b'E').or_else(|| s.iter().position(|&c| c == b'e')).unwrap_or(0);
    if idig == 0 || idig > iexp { return; }
    let nspaces = (iexp - idig).wrapping_sub(1) / f_fmt;
    for i in (1..=nspaces).rev() { s.insert(idig + i * f_fmt, b' '); }
}

fn vp_to_string(a: &Real, f_fmt: usize, f_plus: i32) -> Vec<u8> {
    if let Some(s) = vp_to_special_string(a, f_plus) { return s; }
    let mut p = vec![];
    if a.sign < 0 { p.push(b'-'); } else if f_plus == 1 { p.push(b' '); } else if f_plus == 2 { p.push(b'+'); }
    p.extend_from_slice(b"0.");
    let start = 0;
    let mut zs = true;
    push_digits(&mut p, a, &mut zs);
    let ex = vp_exponent10(a);
    while p.len() > start + 1 && p.last() == Some(&b'0') { p.pop(); }
    p.push(b'e');
    p.extend_from_slice(ex.to_string().as_bytes());
    vp_format_st(&mut p, f_fmt);
    p
}

fn vp_to_fstring(a: &Real, f_fmt: usize, f_plus: i32) -> Vec<u8> {
    if let Some(s) = vp_to_special_string(a, f_plus) { return s; }
    let mut p: Vec<u8> = vec![];
    let mut delim = f_fmt;
    let append = |p: &mut Vec<u8>, ch: u8, group: bool, delim: &mut usize| {
        if group && *delim == 0 { p.push(b' '); }
        p.push(ch);
        if group { *delim = (*delim + 1) % f_fmt; }
    };
    if a.sign < 0 { append(&mut p, b'-', false, &mut delim); }
    else if f_plus == 1 { append(&mut p, b' ', false, &mut delim); }
    else if f_plus == 2 { append(&mut p, b'+', false, &mut delim); }
    let n = a.prec;
    let mut ex = a.exponent;
    let g = f_fmt > 0;
    if ex <= 0 { append(&mut p, b'0', false, &mut delim); append(&mut p, b'.', false, &mut delim); }
    while ex < 0 { for _ in 0..BASE_FIG { append(&mut p, b'0', g, &mut delim); } ex += 1; }
    for i in 0..n {
        let mut m = BASE1;
        let mut e = a.frac[i];
        if i == 0 && ex > 0 {
            delim = 0;
            while e / m == 0 { m /= 10; delim += 1; }
            if f_fmt > 0 { delim = 2 * f_fmt - ((ex as usize * BASE_FIG - delim) % f_fmt); }
        }
        while m != 0 && (e != 0 || i < n - 1 || ex > 0) {
            append(&mut p, (e / m) as u8 + b'0', g, &mut delim);
            e %= m;
            m /= 10;
        }
        ex -= 1;
        if ex == 0 { append(&mut p, b'.', false, &mut delim); delim = f_fmt; }
    }
    while ex > 0 {
        for _ in 0..BASE_FIG { append(&mut p, b'0', g, &mut delim); }
        ex -= 1;
        if ex == 0 { append(&mut p, b'.', false, &mut delim); }
    }
    if p.len() > 1 && p.last() == Some(&b'.') { p.push(b'0'); }
    p
}

fn mul_overflow(a: isize, b: isize) -> bool { a.checked_mul(b).is_none() }
fn add_overflow(a: isize, b: isize) -> bool { a.checked_add(b).is_none() }

/// VpCtoV: `int_chr` may start with the sign; `frac`/`exp_chr` are digit runs.
fn vp_ctov(a: &mut Real, int_chr: &[u8], mut ni: usize, frac: &[u8], nf: usize, exp_chr: &[u8], ne: usize, env: &Env) -> VR<bool> {
    let at = |s: &[u8], i: usize| -> u8 { s.get(i).copied().unwrap_or(0) };
    let mut e: isize = 0;
    let ma = a.max_prec;
    let mut mi = ni;
    let mut me = ne;
    let mut signe: isize = 1;
    let mut exponent_overflow = false;
    for x in a.frac[..ma.max(1)].iter_mut() { *x = 0; }
    if ne > 0 {
        let mut i = 0;
        if at(exp_chr, 0) == b'-' { signe = -1; i += 1; me += 1; }
        else if at(exp_chr, 0) == b'+' { i += 1; me += 1; }
        while i < me {
            let dig = (at(exp_chr, i) as isize) - b'0' as isize;
            if mul_overflow(e, 10) || add_overflow(e * 10, signe * dig) { exponent_overflow = true; break; }
            e = e * 10 + signe * dig;
            i += 1;
        }
    }
    let mut i = 0;
    let mut sign = 1;
    if at(int_chr, 0) == b'-' { sign = -1; i += 1; mi += 1; }
    else if at(int_chr, 0) == b'+' { i += 1; mi += 1; }
    while i < mi && at(int_chr, i) == b'0' { i += 1; ni -= 1; }
    if add_overflow(e, ni as isize) { exponent_overflow = true; } else { e += ni as isize; }
    let mut j = ((BASE_FIG as isize - e % BASE_FIG as isize) % BASE_FIG as isize) as usize;
    if add_overflow(e, j as isize) { exponent_overflow = true; } else { e += j as isize; }
    if exponent_overflow || !(EXPONENT_MIN..=EXPONENT_MAX).contains(&e) {
        let mut zero = true;
        while i < mi && zero { zero = at(int_chr, i) == b'0'; i += 1; }
        let mut k = 0;
        while k < nf && zero { zero = at(frac, k) == b'0'; k += 1; }
        if !zero && e > 0 { a.set_inf(sign); vp_exception(env, EXC_INFINITY, "exponent overflow", false)?; }
        else { a.set_zero(sign); }
        return Ok(true);
    }
    let mut ind_a = 0;
    // VpAlloc's nalloc always leaves room: C's "overflow (last few digits discarded)" warning is unreachable
    'fill: {
        while i < mi {
            a.frac[ind_a] = 0;
            while j < BASE_FIG && i < mi { a.frac[ind_a] = a.frac[ind_a] * 10 + (at(int_chr, i) - b'0') as u32; j += 1; i += 1; }
            if i < mi { ind_a += 1; if ind_a >= ma { break 'fill; } j = 0; }
        }
        let mut i = 0;
        while i < nf {
            while j < BASE_FIG && i < nf { a.frac[ind_a] = a.frac[ind_a] * 10 + (at(frac, i) - b'0') as u32; j += 1; i += 1; }
            if i < nf { ind_a += 1; if ind_a >= ma { break 'fill; } j = 0; }
        }
    }
    if ind_a >= ma { ind_a = ma - 1; }
    while j < BASE_FIG { a.frac[ind_a] *= 10; j += 1; }
    a.prec = ind_a + 1;
    a.exponent = e / BASE_FIG as isize;
    a.set_sign(sign);
    vp_nmlz(a, env)?;
    Ok(true)
}

/// VpAlloc: Ok(Ok(real)), or Ok(Err(())) for an invalid string (strict) / `exc` false.
fn vp_alloc(sz: &[u8], strict: bool, env: &Env) -> VR<Result<Real, ()>> {
    let at = |i: usize| -> u8 { sz.get(i).copied().unwrap_or(0) };
    let is_space = |c: u8| matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r');
    let mut j = 0;
    while is_space(at(j)) { j += 1; }
    if let Some(r) = parse_special(&sz[j.min(sz.len())..]) { return Ok(Ok(r)); }
    if at(j) == b'#' { j += 1; }
    let mut psz: Vec<u8> = vec![0; sz.len() + 2];
    let mut i = 0;
    let mut v = at(j);
    psz[i] = v;
    let mut sign = 1;
    if v == b'-' || v == b'+' { sign = if v == b'-' { -1 } else { 0 }; i += 1; j += 1; }
    let mut ni = 0;
    'parse: {
        loop {
            v = at(j); psz[i] = v;
            if v == 0 { break; }
            if !strict && is_space(v) { v = 0; psz[i] = 0; break; }
            if v == b'_' {
                if ni > 0 {
                    let w = at(j + 1);
                    if w == 0 || is_space(w) || w.is_ascii_digit() { j += 1; continue; }
                    if !strict { v = 0; psz[i] = 0; break; }
                }
                break 'parse;
            }
            if !v.is_ascii_digit() { break; }
            ni += 1; i += 1; j += 1;
        }
        let (mut nf, mut ne, mut ipf, mut ipe, mut exp_seen) = (0, 0, 0, 0, false);
        if v != 0 {
            psz[i] = at(j);
            if psz[i] == b'.' {
                i += 1; j += 1;
                ipf = i;
                loop {
                    v = at(j); psz[i] = v;
                    if v == 0 { break; }
                    if !strict && is_space(v) { v = 0; psz[i] = 0; break; }
                    if v == b'_' {
                        if nf > 0 && at(j + 1).is_ascii_digit() { j += 1; continue; }
                        if !strict { v = 0; psz[i] = 0; break; }
                        break 'parse;
                    }
                    if !v.is_ascii_digit() { break; }
                    i += 1; j += 1; nf += 1;
                }
            }
            if v != 0 {
                psz[i] = at(j);
                match psz[i] {
                    0 => {}
                    b'e' | b'E' | b'd' | b'D' => {
                        exp_seen = true;
                        i += 1; j += 1;
                        ipe = i;
                        v = at(j); psz[i] = v;
                        if v == b'-' || v == b'+' { i += 1; j += 1; }
                        loop {
                            v = at(j); psz[i] = v;
                            if v == 0 { break; }
                            if !strict && is_space(v) { v = 0; psz[i] = 0; break; }
                            if v == b'_' {
                                if ne > 0 && at(j + 1).is_ascii_digit() { j += 1; continue; }
                                if !strict { v = 0; psz[i] = 0; if ne == 0 { exp_seen = false; } break; }
                                break 'parse;
                            }
                            if !v.is_ascii_digit() { break; }
                            i += 1; j += 1; ne += 1;
                        }
                    }
                    _ => {}
                }
            }
            if v != 0 {
                while is_space(at(j)) { j += 1; }
                if at(j) != 0 && strict { break 'parse; }
            }
        }
        psz[i] = 0;
        if strict && ((ni == 0 && nf == 0) || (exp_seen && ne == 0)) { break 'parse; }
        let nalloc = (ni + nf + BASE_FIG - 1) / BASE_FIG + 1;
        let mut vp = Real::alloc(nalloc);
        vp.set_zero(sign);
        let int_chr = psz.clone();
        let frac = psz[ipf..].to_vec();
        let exp = psz[ipe..].to_vec();
        vp_ctov(&mut vp, &int_chr, ni, &frac, nf, &exp, ne, env)?;
        return Ok(Ok(vp));
    }
    if !strict { return Ok(Ok(Real::zero_wrap(1, 1))); }
    Ok(Err(()))
}

fn parse_special(s: &[u8]) -> Option<Real> {
    for (pat, sign) in [(&b"Infinity"[..], SIGN_POSITIVE_INFINITE), (b"+Infinity", SIGN_POSITIVE_INFINITE), (b"-Infinity", SIGN_NEGATIVE_INFINITE), (b"NaN", SIGN_NAN)] {
        if !s.starts_with(pat) { continue; }
        let mut p = pat.len();
        while p < s.len() && matches!(s[p], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') { p += 1; }
        if p == s.len() || s[p] == 0 {
            let mut r = Real::alloc(1);
            r.set_special(sign);
            return Some(r);
        }
    }
    None
}

fn vp_vtod(m: &Real) -> (f64, isize, i32) {
    if m.is_nan() { return (f64::NAN, 0, -1); }
    if m.is_pos_zero() { return (0.0, 0, 0); }
    if m.is_neg_zero() { return (-0.0, 0, 0); }
    if m.is_pos_inf() { return (f64::INFINITY, 0, 2); }
    if m.is_neg_inf() { return (f64::NEG_INFINITY, 0, 2); }
    let fig = roomof(DOUBLE_FIG, BASE_FIG);
    let mm = fig.min(m.prec);
    let mut d = 0.0f64;
    let mut div = 1.0f64;
    for i in 0..mm { div /= BASE as f64; d += m.frac[i] as f64 * div; }
    (d * m.get_sign() as f64, m.exponent.wrapping_mul(BASE_FIG as isize), 1)
}

fn vp_mid_round(y: &mut Real, f: u16, nf: isize, env: &Env) -> VR<bool> {
    let mut nf = nf.wrapping_add(y.exponent.wrapping_mul(BASE_FIG as isize));
    let mut exptoadd: isize = 0;
    if nf < 0 {
        if f != ROUND_CEIL && f != ROUND_FLOOR { let s = y.get_sign(); y.set_zero(s); return Ok(false); }
        exptoadd = -nf;
        nf = 0;
    }
    let ix = (nf / BASE_FIG as isize) as usize;
    if ix >= y.prec { return Ok(false); }
    let mut v = y.frac[ix];
    let ioffset = nf as usize - ix * BASE_FIG;
    let n = BASE_FIG - ioffset - 1;
    let shifter = 10u32.pow(n as u32);
    let mut fracf = v % (shifter * 10) > 0;
    let mut fracf_1further = v % shifter > 0;
    v /= shifter;
    let mut div = (v / 10) as u64;
    let v = v - (div as u32) * 10;
    for i in ix + 1..y.prec {
        if y.frac[i] % BASE != 0 { fracf = true; fracf_1further = true; break; }
    }
    for i in ix + 1..y.prec { y.frac[i] = 0; }
    match f {
        ROUND_DOWN => {}
        ROUND_UP => if fracf { div += 1; },
        ROUND_HALF_UP => if v >= 5 { div += 1; },
        ROUND_HALF_DOWN => if v > 5 || (v == 5 && fracf_1further) { div += 1; },
        ROUND_CEIL => if fracf && y.sign > 0 { div += 1; },
        ROUND_FLOOR => if fracf && y.sign < 0 { div += 1; },
        ROUND_HALF_EVEN => {
            if v > 5 { div += 1; }
            else if v == 5 {
                if fracf_1further { div += 1; }
                else if ioffset == 0 { if ix > 0 && y.frac[ix - 1] % 2 == 1 { div += 1; } }
                else if div % 2 == 1 { div += 1; }
            }
        }
        _ => {}
    }
    for _ in 0..=n { div *= 10; }
    if div >= BASE as u64 {
        if ix > 0 { y.frac[ix] = 0; vp_rdup(y, ix, env)?; }
        else {
            let s = y.get_sign();
            let e = y.exponent;
            y.set_one();
            y.set_sign(s);
            y.exponent = e + 1;
        }
    } else {
        y.frac[ix] = div as u32;
        vp_nmlz(y, env)?;
    }
    if exptoadd > 0 && !y.is_zero() {
        y.exponent += exptoadd / BASE_FIG as isize;
        exptoadd %= BASE_FIG as isize;
        for _ in 0..exptoadd {
            y.frac[0] = y.frac[0].wrapping_mul(10);
            if y.frac[0] >= BASE { y.frac[0] /= BASE; y.exponent += 1; }
        }
    }
    Ok(true)
}

fn vp_left_round(y: &mut Real, f: u16, nf: isize, env: &Env) -> VR<bool> {
    if !y.has_val() { return Ok(false); }
    let mut v = y.frac[0];
    let mut nf = nf.wrapping_sub(y.exponent.wrapping_mul(BASE_FIG as isize));
    loop { v /= 10; if v == 0 { break; } nf -= 1; }
    nf += BASE_FIG as isize - 1;
    vp_mid_round(y, f, nf, env)
}

fn vp_active_round(y: &mut Real, x: &Real, f: u16, nf: isize, env: &Env) -> VR<bool> {
    if vp_asgn(y, x, 10, env)? <= 1 { return Ok(false); }
    vp_mid_round(y, f, nf, env)
}

fn vp_limit_round(c: &mut Real, ix_digit: usize, env: &Env) -> VR<i32> {
    let ix = env.limit;
    if !vp_nmlz(c, env)? { return Ok(-1); }
    if ix == 0 { return Ok(0); }
    let ix_digit = if ix_digit == 0 { c.prec - 1 } else { ix_digit };
    if (ix + BASE_FIG - 1) / BASE_FIG > ix_digit + 1 { return Ok(0); }
    Ok(vp_left_round(c, env.round, ix as isize, env)? as i32)
}

fn vp_internal_round(c: &mut Real, ix_digit: usize, v_prev: u32, v: u32, env: &Env) -> VR<()> {
    if vp_limit_round(c, ix_digit, env)? != 0 { return Ok(()); }
    if v == 0 { return Ok(()); }
    let v = v / BASE1;
    let f = match env.round {
        ROUND_DOWN => false,
        ROUND_UP => v != 0,
        ROUND_HALF_UP => v >= 5,
        ROUND_HALF_DOWN => v >= 6,
        ROUND_CEIL => v != 0 && c.sign > 0,
        ROUND_FLOOR => v != 0 && c.sign < 0,
        ROUND_HALF_EVEN => v > 5 || (v == 5 && v_prev % 2 == 1),
        _ => false,
    };
    if f { vp_rdup(c, ix_digit, env)?; vp_nmlz(c, env)?; }
    Ok(())
}

fn vp_rdup(m: &mut Real, ind_m: usize, env: &Env) -> VR<bool> {
    let mut ind = if ind_m == 0 { m.prec } else { ind_m };
    let mut carry = 1u32;
    while carry > 0 && ind > 0 {
        ind -= 1;
        m.frac[ind] = m.frac[ind].wrapping_add(carry);
        if m.frac[ind] >= BASE { m.frac[ind] -= BASE; } else { carry = 0; }
    }
    if carry > 0 {
        if !add_exponent(m, 1, env)? { return Ok(false); }
        m.prec = 1; m.frac[0] = 1;
    } else { vp_nmlz(m, env)?; }
    Ok(true)
}

fn vp_frac(y: &mut Real, x: &Real, env: &Env) -> VR<()> {
    if !x.has_val() { vp_asgn(y, x, 10, env)?; return Ok(()); }
    if x.exponent > 0 && x.exponent as usize >= x.prec { y.set_zero(x.get_sign()); return Ok(()); }
    if x.exponent <= 0 { vp_asgn(y, x, 10, env)?; return Ok(()); }
    y.prec = (x.prec - x.exponent as usize).min(y.max_prec);
    y.exponent = 0;
    y.set_sign(x.get_sign());
    let my = y.prec;
    y.ensure(my + 1);
    let ix = x.exponent as usize;
    y.frac[..my].copy_from_slice(&x.frac[ix..ix + my]);
    vp_nmlz(y, env)?;
    Ok(())
}

fn get_add_sub_prec(a: &Real, b: &Real) -> usize {
    if a.is_zero() || b.is_zero() { return a.prec.max(b.prec); }
    let min_a = a.exponent - a.prec as isize;
    let min_b = b.exponent - b.prec as isize;
    (a.exponent.max(b.exponent) - min_a.min(min_b)) as usize
}

fn count_precision_and_scale(p: &Real) -> (isize, isize) {
    if p.is_zero() || !p.is_def() { return (0, 0); }
    let mut n = p.prec as isize;
    while n > 0 && p.frac[n as usize - 1] == 0 { n -= 1; }
    if n == 0 { return (0, 0); }
    let mut nlz = BASE_FIG as isize;
    let mut x = p.frac[0];
    while x > 0 { nlz -= 1; x /= 10; }
    let mut ntz = 0;
    let mut x = p.frac[n as usize - 1];
    while x > 0 && x % 10 == 0 { ntz += 1; x /= 10; }
    let mut ex = p.exponent;
    let mut n_digits_head = BASE_FIG as isize;
    if ex < 0 { n_digits_head += (-ex) * BASE_FIG as isize; ex = 0; }
    else if ex > 0 { n_digits_head -= nlz; }
    let mut precision = n_digits_head;
    if ex > n { precision += (ex - 1) * BASE_FIG as isize; }
    else if n > 0 { precision += (n - 1) * BASE_FIG as isize; if ex < n { precision -= ntz; } }
    let mut scale = 0;
    if p.exponent < 0 { scale = n_digits_head + (n - 1) * BASE_FIG as isize - ntz; }
    else if n > p.exponent { scale = (n - p.exponent) * BASE_FIG as isize - ntz; }
    (precision, scale)
}

// ---- Ruby objects --------------------------------------------------------------------------

pub struct RBigDecimal { r: Real }
impl RData for RBigDecimal {
    fn visit(&mut self, _m: &mut crate::gc::Visitor) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

static CLASS: Global = Global::new();

fn real_of(c: &Ctx, v: Value) -> Option<Real> { c.data::<RBigDecimal>(v).map(|d| d.r.clone()) }
fn is_bd(c: &Ctx, v: Value) -> bool { c.data::<RBigDecimal>(v).is_some() }

fn wrap(c: &mut Ctx, r: Real) -> Value {
    let k = CLASS.get(c);
    let v = c.alloc(k, Body::Data(Box::new(RBigDecimal { r })));
    obj_set_flags(v, FL_FROZEN);
    v
}

/// A Real and the object holding it (the object is reused when no copy is made).
#[derive(Clone)]
struct Bdv { v: Value, r: Real }

fn vperr(c: &mut Ctx, e: VpErr) -> Esc {
    let m = match e {
        VpErr::Fd(s) => make_exception(c, c.vm().c_float_domain_error.get(), s),
        VpErr::NoMem => make_exception(c, c.vm().c_no_memory_error.get(), "failed to allocate memory"),
    };
    raise_exc(c, m)
}
macro_rules! vp { ($c:expr, $e:expr) => { match $e { Ok(x) => x, Err(e) => return Err(vperr($c, e)) } }; }

fn tl_key(name: &str) -> Value { sym_value(name) }
fn tl_get(c: &mut Ctx, name: &str, default: Value) -> RV {
    let t = crate::builtins::thread::current_thread(c);
    let v = funcall(c, t, intern(b"[]"), &[tl_key(name)], std::ptr::null())?;
    if v == NIL { tl_set(c, name, default)?; return Ok(default); }
    Ok(v)
}
fn tl_set(c: &mut Ctx, name: &str, v: Value) -> Result<(), Esc> {
    let t = crate::builtins::thread::current_thread(c);
    funcall(c, t, intern(b"[]="), &[tl_key(name), v], std::ptr::null())?;
    Ok(())
}
const K_EXC: &str = "BigDecimal.exception_mode";
const K_ROUND: &str = "BigDecimal.rounding_mode";
const K_LIMIT: &str = "BigDecimal.precision_limit";
fn get_exc(c: &mut Ctx) -> Result<u16, Esc> { let v = tl_get(c, K_EXC, Value::fix(0))?; Ok(num_to_long(c, v)? as u16) }
fn get_round(c: &mut Ctx) -> Result<u16, Esc> { let v = tl_get(c, K_ROUND, Value::fix(ROUND_HALF_UP as i64))?; Ok(num_to_long(c, v)? as u16) }
fn get_limit(c: &mut Ctx) -> Result<usize, Esc> { let v = tl_get(c, K_LIMIT, Value::fix(0))?; Ok(num_to_long(c, v)? as usize) }
fn env(c: &mut Ctx) -> Result<Env, Esc> { Ok(Env { exc: get_exc(c)?, round: get_round(c)?, limit: get_limit(c)? }) }

/// VpCheckException
fn check_exception(c: &mut Ctx, p: &Real, always: bool, env: &Env) -> Result<(), Esc> {
    let r = if p.is_nan() { vp_exception(env, EXC_NAN, "Computation results in 'NaN' (Not a Number)", always) }
        else if p.is_pos_inf() { vp_exception(env, EXC_INFINITY, "Computation results in 'Infinity'", always) }
        else if p.is_neg_inf() { vp_exception(env, EXC_INFINITY, "Computation results in '-Infinity'", always) }
        else { Ok(false) };
    vp!(c, r);
    Ok(())
}
/// CheckGetValue on a fresh result.
fn check_get(c: &mut Ctx, r: Real, env: &Env) -> RV { check_exception(c, &r, false, env)?; Ok(wrap(c, r)) }
fn check_get_v(c: &mut Ctx, b: Bdv, env: &Env) -> RV { check_exception(c, &b.r, false, env)?; Ok(b.v) }

fn num2int(c: &mut Ctx, v: Value) -> Result<i32, Esc> {
    let n = num_to_long(c, v)?;
    if n > i32::MAX as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too big to convert to 'int'", n)); }
    if n < i32::MIN as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too small to convert to 'int'", n)); }
    Ok(n as i32)
}
fn check_int_precision(c: &mut Ctx, v: Value) -> Result<usize, Esc> {
    let n = num_to_long(c, v)?;
    if n < 0 { return arg_err(c, "negative precision"); }
    Ok(n as usize)
}

fn from_u64(u: u64) -> Real {
    if u == 0 { let mut r = Real::alloc(1); r.prec = 1; r.exponent = 1; r.set_zero(1); r.frac[0] = 0; return r; }
    let mut words = vec![];
    let mut x = u;
    while x > 0 { words.push((x % BASE as u64) as u32); x /= BASE as u64; }
    let ntz = words.iter().take_while(|&&w| w == 0).count();
    let len = words.len() - ntz;
    let mut r = Real::alloc(len);
    r.prec = len; r.exponent = words.len() as isize; r.set_sign(1);
    for k in 0..len { r.frac[k] = words[words.len() - 1 - k]; }
    r
}
fn int_to_real(c: &mut Ctx, v: Value, env: &Env) -> Result<Real, Esc> {
    if v.is_fixnum() {
        let i = v.as_fix();
        let mut r = from_u64(i.unsigned_abs());
        if i < 0 { r.set_sign(-1); }
        return Ok(r);
    }
    let b = c.to_big(v);
    if let Some(i) = b.to_i64() { let mut r = from_u64(i.unsigned_abs()); if i < 0 { r.set_sign(-1); } return Ok(r); }
    if !b.neg && b.bit_length() <= 64 { let s = b.to_string_radix(10); return Ok(from_u64(s.parse().unwrap())); }
    let s = b.to_string_radix(10);
    let r = vp!(c, vp_alloc(s.as_bytes(), true, env)).ok().expect("digits");
    check_exception(c, &r, false, env)?;
    Ok(r)
}

/// rb_float_convert_to_BigDecimal; `digs` 0 is the shortest representation.
fn float_to_real(c: &mut Ctx, d: f64, digs: usize, raise: bool, env: &Env) -> Result<Option<Real>, Esc> {
    if d.is_nan() || d.is_infinite() {
        let mut r = Real::alloc(1);
        if d.is_nan() { r.set_nan(); } else { r.set_inf(if d > 0.0 { 1 } else { -1 }); }
        check_exception(c, &r, false, env)?;
        return Ok(Some(r));
    }
    if d == 0.0 { return Ok(Some(Real::zero_wrap(if d.is_sign_negative() { -1 } else { 1 }, 1))); }
    if digs > DOUBLE_FIG {
        if !raise { return Ok(None); }
        return arg_err(c, "precision too large.");
    }
    let (mut digits, decpt) = if digs == 0 { dtoa_mode0(d.abs()) } else { dtoa_mode2(d.abs(), digs) };
    while digits.len() > 1 && digits.last() == Some(&b'0') { digits.pop(); }
    digits.truncate(DOUBLE_FIG);
    let mut t = Vec::from(&b"0."[..]);
    t.extend_from_slice(&digits);
    t.extend_from_slice(format!("e{}", decpt).as_bytes());
    let mut r = vp!(c, vp_alloc(&t, true, env)).ok().expect("digits");
    if d < 0.0 { r.set_sign(-1); }
    Ok(Some(r))
}

const TENS: [f64; 23] = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22];
const BIGTENS: [f64; 5] = [1e16, 1e32, 1e64, 1e128, 1e256];

/// dtoa(d, 2, ndigits) of bigdecimal's missing/dtoa.c for d > 0: (digit chars, decpt). Its floating-point
/// fast path (ndigits <= Quick_max 14) is kept: in its uncertain zone it rounds half-even on the last digit.
fn dtoa_mode2(d: f64, ndigits: usize) -> (Vec<u8>, i64) {
    let bits = d.to_bits();
    let (w0, w1) = ((bits >> 32) as u32, bits as u32);
    let ef = ((bits >> 52) & 0x7ff) as i32;
    let (mant, e2) = if ef != 0 { ((bits & ((1u64 << 52) - 1)) | (1u64 << 52), ef - 1075) } else { (bits & ((1u64 << 52) - 1), -1074) };
    let tz = mant.trailing_zeros() as i32;
    let be = e2 + tz;
    let bbits = 64 - (mant >> tz).leading_zeros() as i32;
    let (d2, i) = if ef != 0 {
        (f64::from_bits((bits & ((1u64 << 52) - 1)) | (1023u64 << 52)), ef - 1023)
    } else {
        let i = bbits + be + (1023 + 52 - 1);
        let x: u32 = if i > 32 { (w0 << (64 - i)) | (w1 >> (i - 32)) } else { w1 << (32 - i) };
        (f64::from_bits((x as f64).to_bits() - (31u64 << 52)), i - (1023 + 52 - 1) - 1)
    };
    let ds = (d2 - 1.5) * 0.289529654602168 + 0.1760912590558 + i as f64 * 0.301029995663981;
    let mut k = ds as i32;
    if ds < 0.0 && ds != k as f64 { k -= 1; }
    let mut k_check = true;
    if (0..=22).contains(&k) { if d < TENS[k as usize] { k -= 1; } k_check = false; }
    let n = ndigits.max(1) as i32;
    let mut half = false;
    if n <= 14 {
        let (mut dd, mut ilim, mut kk, mut ieps, mut bi) = (d, n, k, 2, 0usize);
        if kk > 0 {
            let mut dsv = TENS[(kk & 0xf) as usize];
            let mut j = kk >> 4;
            if j & 0x10 != 0 { j &= 0xf; dd /= BIGTENS[4]; ieps += 1; }
            while j != 0 { if j & 1 != 0 { ieps += 1; dsv *= BIGTENS[bi]; } j >>= 1; bi += 1; }
            dd /= dsv;
        } else if kk != 0 {
            let j1 = -kk;
            dd *= TENS[(j1 & 0xf) as usize];
            let mut j = j1 >> 4;
            while j != 0 { if j & 1 != 0 { ieps += 1; dd *= BIGTENS[bi]; } j >>= 1; bi += 1; }
        }
        if k_check && dd < 1.0 && ilim > 0 { kk -= 1; dd *= 10.0; ieps += 1; }
        let mut eps = ieps as f64 * dd + 7.0;
        eps = f64::from_bits(eps.to_bits().wrapping_sub(52u64 << 52));
        eps *= TENS[(ilim - 1) as usize];
        let mut s: Vec<u8> = vec![];
        let mut i = 1;
        loop {
            let l = dd as i64;
            dd -= l as f64;
            if dd == 0.0 { ilim = i; }
            s.push(b'0' + l as u8);
            if i == ilim {
                if dd > 0.5 + eps { return bump_up(s, kk); }
                if dd < 0.5 - eps { while s.last() == Some(&b'0') { s.pop(); } return (s, kk as i64 + 1); }
                half = true;
                if (s.last().unwrap() - b'0') & 1 == 1 { return bump_up(s, kk); }
                break;
            }
            i += 1;
            dd *= 10.0;
        }
    }
    // the exact path: all significant decimal places of d (at most 767), rounded half-even
    let ex = format!("{:.800e}", d);
    let (m, e) = ex.split_once('e').unwrap();
    let all: Vec<u8> = m.bytes().filter(|b| b.is_ascii_digit()).collect();
    let k = e.parse::<i64>().unwrap();
    let n = n as usize;
    let mut s = all[..n].to_vec();
    let rest = &all[n..];
    let j = match rest.first() { None => -1, Some(&f) if f > b'5' => 1, Some(&f) if f < b'5' => -1, _ => if rest[1..].iter().any(|&x| x != b'0') { 1 } else { 0 } };
    // the "small integer" branch rounds without consulting `half`
    let small_int = be >= 0 && k <= 14;
    if j > 0 || (j == 0 && (s[n - 1] - b'0') & 1 == 1) {
        while s.last() == Some(&b'9') { s.pop(); }
        if s.is_empty() { return (vec![b'1'], k + 2); }
        let last = s.len() - 1;
        if small_int || !half || (s[last] - b'0') & 1 == 1 { s[last] += 1; }
    } else {
        while s.len() > 1 && s.last() == Some(&b'0') { s.pop(); }
    }
    (s, k + 1)
}
/// dtoa(d, 0): the shortest round-tripping digits; of two equally near candidates dtoa takes the even one.
fn dtoa_mode0(d: f64) -> (Vec<u8>, i64) {
    let s = format!("{:e}", d);
    let (mant, ex) = s.split_once('e').unwrap();
    let short: Vec<u8> = mant.bytes().filter(|b| b.is_ascii_digit()).collect();
    let k = ex.parse::<i64>().unwrap();
    let n = short.len();
    let exact = format!("{:.800e}", d);
    let (m, e) = exact.split_once('e').unwrap();
    if e.parse::<i64>().unwrap() == k {
        let all: Vec<u8> = m.bytes().filter(|b| b.is_ascii_digit()).collect();
        let rest = &all[n..];
        let tie = rest.first() == Some(&b'5') && rest[1..].iter().all(|&x| x == b'0');
        if tie && (short[n - 1] - b'0') & 1 == 1 {
            let mut alt = all[..n].to_vec();
            if (alt[n - 1] - b'0') & 1 == 0 {
                let t = format!("{}.{}e{}", alt[0] as char, std::str::from_utf8(&alt[1..]).unwrap(), k);
                if t.parse::<f64>() == Ok(d) { while alt.len() > 1 && alt.last() == Some(&b'0') { alt.pop(); } return (alt, k + 1); }
            }
        }
    }
    (short, k + 1)
}
fn bump_up(mut s: Vec<u8>, k: i32) -> (Vec<u8>, i64) {
    while s.last() == Some(&b'9') { s.pop(); }
    if s.is_empty() { return (vec![b'1'], k as i64 + 2); }
    *s.last_mut().unwrap() += 1;
    (s, k as i64 + 1)
}

fn rational_to_real(c: &mut Ctx, v: Value, digs: Option<usize>, raise: bool, env: &Env) -> Result<Option<Bdv>, Esc> {
    let Some(digs) = digs else {
        if !raise { return Ok(None); }
        return arg_err(c, "can't omit precision for a Rational.");
    };
    let (num, den) = crate::builtins::rational::rat_parts(c, v);
    let nr = int_to_real(c, num, env)?;
    let nv = wrap(c, nr.clone());
    let r = div2(c, Bdv { v: nv, r: nr }, den, digs, env)?;
    Ok(Some(r))
}

/// GetBDValueWithPrecInternal: None when `v` is not convertible (and !must).
fn get_bd(c: &mut Ctx, v: Value, prec: usize, must: bool, env: &Env) -> Result<Option<Bdv>, Esc> {
    if let Some(r) = real_of(c, v) { return Ok(Some(Bdv { v, r })); }
    if is_integer(v) { let r = int_to_real(c, v, env)?; let w = wrap(c, r.clone()); return Ok(Some(Bdv { v: w, r })); }
    if is_float(v) { let r = float_to_real(c, float_val(v), 0, true, env)?.unwrap(); let w = wrap(c, r.clone()); return Ok(Some(Bdv { v: w, r })); }
    if crate::builtins::rational::is_rational(c, v) { return rational_to_real(c, v, Some(prec), true, env); }
    if must { return Err(cannot_coerce(c, v)); }
    Ok(None)
}
fn get_bd_must(c: &mut Ctx, v: Value, prec: usize, env: &Env) -> Result<Bdv, Esc> { Ok(get_bd(c, v, prec, true, env)?.unwrap()) }
fn cannot_coerce(c: &mut Ctx, v: Value) -> Esc {
    let s = if !v.is_heap() { match inspect_str(c, v) { Ok(s) => s, Err(e) => return e } } else { class_path(c, real_class_of(c, v)) };
    let m = make_exception(c, c.vm().c_type_error.get(), &format!("{} can't be coerced into BigDecimal", s));
    raise_exc(c, m)
}
fn slf(c: &mut Ctx, v: Value) -> Result<Bdv, Esc> {
    match real_of(c, v) { Some(r) => Ok(Bdv { v, r }), None => Err(cannot_coerce(c, v)) }
}
fn is_coerceable(c: &Ctx, v: Value) -> bool { is_bd(c, v) || is_integer(v) || is_float(v) || crate::builtins::rational::is_rational(c, v) }
fn get_coerce_prec(a: &Real, prec: usize) -> usize {
    let p = if prec == 0 { a.prec * BASE_FIG } else { prec };
    p.max(2 * DOUBLE_FIG)
}

// rb_num_coerce_*
fn do_coerce(c: &mut Ctx, x: Value, y: Value, err_: bool) -> Result<Option<(Value, Value)>, Esc> {
    if !respond_to(c, y, intern(b"coerce"), true) {
        if err_ { let s = if !y.is_heap() { inspect_str(c, y)? } else { class_path(c, real_class_of(c, y)) }; return type_err(c, format!("{} can't be coerced into BigDecimal", s)); }
        return Ok(None);
    }
    let a = funcall(c, y, intern(b"coerce"), &[x], std::ptr::null())?;
    if !err_ && a == NIL { return Ok(None); }
    if !is_array(a) || c.ary_len(a) != 2 { return type_err(c, "coerce must return [x, y]"); }
    Ok(Some((c.ary_get(a, 0), c.ary_get(a, 1))))
}
fn coerce_bin(c: &mut Ctx, x: Value, y: Value, op: &str) -> RV {
    let (a, b) = do_coerce(c, x, y, true)?.unwrap();
    funcall(c, a, intern(op.as_bytes()), &[b], std::ptr::null())
}
fn coerce_cmp(c: &mut Ctx, x: Value, y: Value, op: &str) -> RV {
    match do_coerce(c, x, y, false)? { Some((a, b)) => funcall(c, a, intern(op.as_bytes()), &[b], std::ptr::null()), None => Ok(NIL) }
}
fn coerce_relop(c: &mut Ctx, x: Value, y: Value, op: &str) -> RV {
    if let Some((a, b)) = do_coerce(c, x, y, false)? {
        let r = funcall(c, a, intern(op.as_bytes()), &[b], std::ptr::null())?;
        if r != NIL { return Ok(r); }
    }
    let s = if !y.is_heap() || is_float(y) { inspect_str(c, y)? } else { class_path(c, real_class_of(c, y)) };
    arg_err(c, format!("comparison of BigDecimal with {} failed: coercion was not possible", s))
}

// ---- arithmetic ------------------------------------------------------------------------------

fn addsub(c: &mut Ctx, s: Value, r: Value, prec: usize, op: i32) -> RV {
    let env = env(c)?;
    let mut a = slf(c, s)?;
    let mut b = get_bd_must(c, r, get_coerce_prec(&a.r, prec), &env)?;
    if a.r.is_nan() { return check_get_v(c, a, &env); }
    if b.r.is_nan() { return check_get_v(c, b, &env); }
    let mut cr;
    if a.r.is_inf() || b.r.is_inf() {
        cr = Real::zero_wrap(1, BASE_FIG);
        vp!(c, vp_add_sub(&mut cr, &a.r, &b.r, op, &env));
    } else {
        if prec != 0 && !a.r.is_zero() && !b.r.is_zero() {
            let room = roomof(prec, BASE_FIG) as isize;
            let lim_a = a.r.exponent - (a.r.prec as isize).max(room) - 1;
            let lim_b = b.r.exponent - (b.r.prec as isize).max(room) - 1;
            if lim_a > b.r.exponent {
                let mut b2 = Real::zero_wrap(1, BASE_FIG); b2.set_one(); b2.set_sign(b.r.sign as i32); b2.exponent = lim_a;
                b = Bdv { v: NIL, r: b2 };
            } else if lim_b > a.r.exponent {
                let mut a2 = Real::zero_wrap(1, BASE_FIG); a2.set_one(); a2.set_sign(a.r.sign as i32); a2.exponent = lim_b;
                a = Bdv { v: NIL, r: a2 };
            }
        }
        let mx = get_add_sub_prec(&a.r, &b.r);
        cr = vp!(c, Real::zw(1, (mx + 1) * BASE_FIG));
        let mut e2 = env;
        if prec != 0 { e2.limit = prec; }
        vp!(c, vp_add_sub(&mut cr, &a.r, &b.r, op, &e2));
    }
    check_get(c, cr, &env)
}

fn mult_with_coerce(c: &mut Ctx, s: Value, r: Value, prec: usize) -> RV {
    let env = env(c)?;
    let a = slf(c, s)?;
    let b = get_bd_must(c, r, get_coerce_prec(&a.r, prec), &env)?;
    let mut cr = vp!(c, Real::zw(1, (a.r.prec + b.r.prec) * BASE_FIG));
    vp!(c, vp_mult(&mut cr, &a.r, &b.r, &env));
    if prec != 0 { vp!(c, vp_left_round(&mut cr, env.round, prec as isize, &env)); }
    else { vp!(c, vp_limit_round(&mut cr, 0, &env)); }
    check_get(c, cr, &env)
}

/// BigDecimal_div2 with an explicit precision (0: automatic).
fn div2(c: &mut Ctx, av: Bdv, b: Value, ix: usize, env: &Env) -> Result<Bdv, Esc> {
    let bv = get_bd_must(c, b, get_coerce_prec(&av.r, ix), env)?;
    let mut ix = ix;
    if ix == 0 {
        let (a_prec, _) = count_precision_and_scale(&av.r);
        let (b_prec, _) = count_precision_and_scale(&bv.r);
        ix = (a_prec.max(b_prec) + DOUBLE_FIG as isize) as usize;
        if 2 * DOUBLE_FIG > ix { ix = 2 * DOUBLE_FIG; }
        if env.limit != 0 && env.limit < ix { ix = env.limit; }
    }
    let mut cv = vp!(c, Real::zw(1, ix + 1 + 2 * BASE_FIG - 1));
    let rp = av.r.prec.max(bv.r.prec + cv.max_prec - 1);
    let mut res = vp!(c, Real::zw(1, rp * BASE_FIG));
    vp!(c, vp_divd(&mut cv, &mut res, &av.r, &bv.r, env));
    if !res.is_zero() {
        let idx = roomof(ix, BASE_FIG);
        cv.ensure(idx + 2);
        while cv.prec <= idx { let p = cv.prec; cv.frac[p] = 0; cv.prec += 1; }
        if cv.frac[idx] == 0 || cv.frac[idx] == HALF_BASE { cv.frac[idx] += 1; }
    }
    vp!(c, vp_left_round(&mut cv, env.round, ix as isize, env));
    check_exception(c, &cv, false, env)?;
    let v = wrap(c, cv.clone());
    Ok(Bdv { v, r: cv })
}

/// BigDecimal_DoDivmod: None when `r` is not convertible.
fn do_divmod(c: &mut Ctx, s: Value, r: Value, truncate: bool, env: &Env) -> Result<Option<(Real, Real)>, Esc> {
    let a = slf(c, s)?;
    let Some(b) = get_bd(c, r, get_coerce_prec(&a.r, 0), false, env)? else { return Ok(None) };
    let nan = || { let mut x = Real::alloc(1); x.set_nan(); x };
    if a.r.is_nan() || b.r.is_nan() || (a.r.is_inf() && b.r.is_inf()) { return Ok(Some((nan(), nan()))); }
    if b.r.is_zero() { return err(c, c.vm().c_zero_div_error.get(), "divided by 0"); }
    if a.r.is_inf() {
        let mut d = Real::alloc(1);
        d.set_inf(if a.r.get_sign() == b.r.get_sign() { 1 } else { -1 });
        return Ok(Some((d, nan())));
    }
    if a.r.is_zero() { return Ok(Some((Real::zero_wrap(1, 1), a.r))); }
    if b.r.is_inf() {
        if !truncate && a.r.get_sign() * b.r.get_sign() < 0 {
            let mut m1 = Real::zero_wrap(1, BASE_FIG); m1.set_one(); m1.set_sign(-1);
            return Ok(Some((m1, b.r)));
        }
        return Ok(Some((Real::zero_wrap(1, 1), a.r)));
    }
    let a_exponent = vp_exponent10(&a.r);
    let b_exponent = vp_exponent10(&b.r);
    let mx = if a_exponent > b_exponent { (a_exponent - b_exponent + 1) as usize } else { 1 };
    let mut dv = vp!(c, Real::zw(1, mx + 2 * BASE_FIG - 1));
    let rx = a.r.prec.max(b.r.prec + dv.max_prec - 1);
    let mx = dv.prec + b.r.prec;
    let mut res = vp!(c, Real::zw(1, rx.max(mx) * BASE_FIG));
    let mut md = vp!(c, Real::zw(1, (res.max_prec + 1) * BASE_FIG));
    vp!(c, vp_divd(&mut dv, &mut res, &a.r, &b.r, env));
    vp!(c, vp_mid_round(&mut dv, ROUND_DOWN, 0, env));
    vp!(c, vp_mult(&mut res, &dv, &b.r, env));
    let e0 = Env { limit: 0, ..*env };
    vp!(c, vp_add_sub(&mut md, &a.r, &res, -1, &e0));
    if !truncate && !md.is_zero() && a.r.get_sign() * b.r.get_sign() < 0 {
        let mut dv2 = vp!(c, Real::zw(1, (dv.max_prec + 1) * BASE_FIG));
        let mut md2 = vp!(c, Real::zw(1, (get_add_sub_prec(&md, &b.r) + 1) * BASE_FIG));
        vp!(c, vp_add_sub(&mut dv2, &dv, &Real::one(), -1, &e0));
        vp!(c, vp_add_sub(&mut md2, &md, &b.r, 1, &e0));
        return Ok(Some((dv2, md2)));
    }
    Ok(Some((dv, md)))
}

/// BigDecimal_to_i
fn to_i_real(c: &mut Ctx, r: &Real, env: &Env) -> RV {
    check_exception(c, r, true, env)?;
    if r.exponent <= 0 { return Ok(Value::fix(0)); }
    if r.exponent == 1 { return Ok(c.int_from_i64(r.get_sign() as i64 * r.frac[0] as i64)); }
    // digits * 10**dpower through Integer#**, which owns the "exponent is too large" limit
    let ex = r.exponent as usize;
    let w: Vec<u32> = r.frac[..r.prec.min(ex)].to_vec();
    let mut b = words_to_big(&w);
    let mut dpower = vp_exponent10(r) - b.to_string_radix(10).len() as isize;
    let digits = b.to_string_radix(10);
    let stripped = digits.trim_end_matches('0');
    if stripped.len() != digits.len() { b = BigInt::parse(stripped, 10).unwrap(); dpower += (digits.len() - stripped.len()) as isize; }
    if r.sign < 0 { b = b.neg(); }
    let mut ret = c.big_to_value(b);
    if dpower != 0 {
        let e = c.int_from_i64(dpower as i64);
        let p10 = funcall(c, Value::fix(10), intern(b"**"), &[e], std::ptr::null())?;
        if is_float(p10) { return err(c, c.vm().c_float_domain_error.get(), "Infinity"); }
        ret = funcall(c, ret, intern(b"*"), &[p10], std::ptr::null())?;
    }
    Ok(ret)
}

// ---- methods -----------------------------------------------------------------------------------

type M = fn(&mut Ctx, Value, &[Value], Blk) -> RV;

fn m_precision(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let a = slf(c, s)?; Ok(c.int_from_i64(count_precision_and_scale(&a.r).0 as i64)) }
fn m_scale(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let a = slf(c, s)?; Ok(c.int_from_i64(count_precision_and_scale(&a.r).1 as i64)) }
fn m_precision_scale(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let a = slf(c, s)?;
    let (p, sc) = count_precision_and_scale(&a.r);
    let (p, sc) = (c.int_from_i64(p as i64), c.int_from_i64(sc as i64));
    Ok(c.ary_new_from(&[p, sc]))
}
fn m_n_significant_digits(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let a = slf(c, s)?.r;
    if a.is_zero() || !a.is_def() { return Ok(Value::fix(0)); }
    let mut n = a.prec;
    while n > 0 && a.frac[n - 1] == 0 { n -= 1; }
    if n == 0 { return Ok(Value::fix(0)); }
    let mut nlz = BASE_FIG as i64; let mut x = a.frac[0]; while x > 0 { nlz -= 1; x /= 10; }
    let mut ntz = 0i64; let mut x = a.frac[n - 1]; while x > 0 && x % 10 == 0 { ntz += 1; x /= 10; }
    Ok(c.int_from_i64(BASE_FIG as i64 * n as i64 - nlz - ntz))
}
fn m_hash(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let a = slf(c, s)?.r;
    let mut h = a.sign as i64 as u64;
    if a.sign == 2 || a.sign == -2 {
        let mut x: u64 = 0xcbf2_9ce4_8422_2325;
        for &w in &a.frac[..a.prec] { for b in w.to_le_bytes() { x ^= b as u64; x = x.wrapping_mul(0x100_0000_01b3); } }
        h ^= x;
        h = h.wrapping_add(a.exponent as u64);
    }
    Ok(Value::fix((h as i64) >> 1))
}
fn m_dump(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let r = slf(c, s)?.r;
    let mut out = format!("{}:", r.prec * BASE_FIG).into_bytes();
    out.extend_from_slice(&vp_to_string(&r, 0, 0));
    Ok(c.str_new_enc(&out, ENC_BINARY))
}
fn cstr(c: &mut Ctx, v: Value) -> Result<Vec<u8>, Esc> {
    let v = expect_str(c, v)?;
    let b = c.str_bytes(v).to_vec();
    if b.contains(&0) { return arg_err(c, "string contains null byte"); }
    Ok(b)
}
fn invalid_value(c: &mut Ctx, s: &[u8]) -> Esc {
    let m = format!("invalid value for BigDecimal(): \"{}\"", String::from_utf8_lossy(s));
    let e = make_exception(c, c.vm().c_argument_error.get(), &m);
    raise_exc(c, e)
}
fn s_load(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let b = cstr(c, a[0])?;
    let mut p = 0;
    while p < b.len() {
        let ch = b[p]; p += 1;
        if ch == b':' { break; }
        if !ch.is_ascii_digit() { return type_err(c, "load failed: invalid character in the marshaled string"); }
    }
    let env = env(c)?;
    match vp!(c, vp_alloc(&b[p..], true, &env)) {
        Ok(r) => check_get(c, r, &env),
        Err(()) => Err(invalid_value(c, &b[p..])),
    }
}

fn rounding_mode_sym(name: &str) -> Option<u16> {
    Some(match name {
        "up" => ROUND_UP, "down" | "truncate" => ROUND_DOWN, "half_up" | "default" => ROUND_HALF_UP, "half_down" => ROUND_HALF_DOWN,
        "ceil" | "ceiling" => ROUND_CEIL, "floor" => ROUND_FLOOR, "half_even" | "banker" => ROUND_HALF_EVEN, _ => return None,
    })
}
fn check_rounding_mode(c: &mut Ctx, v: Value) -> Result<u16, Esc> {
    if v.is_sym() {
        if let Some(m) = rounding_mode_sym(&sym_str(v.as_sym())) { return Ok(m); }
        return arg_err(c, format!("invalid rounding mode ({})", sym_str(v.as_sym())));
    }
    // NUM2USHORT: rb_to_int's message for nil
    let iv = if v == NIL { to_integer(c, v)? } else { v };
    let n = num_to_long(c, iv)?;
    if n > u16::MAX as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too big to convert to 'unsigned short'", n)); }
    if n < i16::MIN as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too small to convert to 'unsigned short'", n)); }
    if !(1..=7).contains(&n) { let s = to_s_value(c, v)?; let s = String::from_utf8_lossy(c.str_bytes(s)).into_owned(); return arg_err(c, format!("invalid rounding mode ({})", s)); }
    Ok(n as u16)
}
fn check_rounding_mode_option(c: &mut Ctx, opts: Value, env: &Env) -> Result<u16, Esc> {
    let Some(mode) = crate::hash::hash_get_opt(c, opts, sym_value("half"))? else { return Ok(env.round) };
    if mode == NIL { return Ok(env.round); }
    let s = if mode.is_sym() { Some(sym_str(mode.as_sym()).into_bytes()) }
        else if is_string(mode) { Some(c.str_bytes(mode).to_vec()) }
        else if mode.is_heap() && respond_to(c, mode, intern(b"to_str"), true) { let t = funcall(c, mode, intern(b"to_str"), &[], std::ptr::null())?; if is_string(t) { Some(c.str_bytes(t).to_vec()) } else { None } }
        else { None };
    if let Some(s) = &s {
        let l = s.to_ascii_lowercase();
        match &l[..] { b"up" => return Ok(ROUND_HALF_UP), b"even" => return Ok(ROUND_HALF_EVEN), b"down" => return Ok(ROUND_HALF_DOWN), _ => {} }
    }
    let shown = match s { Some(s) => String::from_utf8_lossy(&s[..]).into_owned(), None => { let t = to_s_value(c, mode)?; String::from_utf8_lossy(c.str_bytes(t)).into_owned() } };
    arg_err(c, format!("invalid rounding mode ({})", shown))
}

fn s_mode(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let which = a[0];
    let val = a.get(1).copied().unwrap_or(NIL);
    let f = num2int(c, which)? as u32 as u64;
    if f & EXC_ALL as u64 != 0 {
        let mut fo = get_exc(c)?;
        if val == NIL { return Ok(Value::fix(fo as i64)); }
        if val != FALSE && val != TRUE { return arg_err(c, "second argument must be true or false"); }
        for bit in [EXC_INFINITY, EXC_NAN, EXC_UNDERFLOW, EXC_ZERODIVIDE] {
            if f & bit as u64 != 0 { fo = if val == TRUE { fo | bit } else { fo & !bit }; tl_set(c, K_EXC, Value::fix(fo as i64))?; }
            fo = get_exc(c)?;
        }
        return Ok(Value::fix(fo as i64));
    }
    if f == ROUND_MODE as u64 {
        let fo = get_round(c)?;
        if val == NIL { return Ok(Value::fix(fo as i64)); }
        let sw = check_rounding_mode(c, val)?;
        tl_set(c, K_ROUND, Value::fix(sw as i64))?;
        return Ok(Value::fix(sw as i64));
    }
    type_err(c, "first argument for BigDecimal.mode invalid")
}
fn s_limit(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let cur = get_limit(c)?;
    let ncur = c.int_from_i64(cur as i64);
    if let Some(&n) = a.first() {
        if n == NIL { return Ok(ncur); }
        let nf = num2int(c, n)?;
        if nf < 0 { return arg_err(c, "argument must be positive"); }
        tl_set(c, K_LIMIT, Value::fix(nf as i64))?;
    }
    Ok(ncur)
}
fn s_double_fig(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(DOUBLE_FIG as i64)) }
fn save_and_yield(c: &mut Ctx, key: &str, b: Blk) -> RV {
    let saved = tl_get(c, key, match key { K_ROUND => Value::fix(ROUND_HALF_UP as i64), _ => Value::fix(0) })?;
    let r = yield_block(c, b, &[NIL]);
    let (ek, ev) = (c.esc, c.esc_val);
    tl_set(c, key, saved)?;
    if r.is_err() { c.esc = ek; c.esc_val = ev; }
    r
}
fn s_save_exception_mode(c: &mut Ctx, _s: Value, _a: &[Value], b: Blk) -> RV { save_and_yield(c, K_EXC, b) }
fn s_save_rounding_mode(c: &mut Ctx, _s: Value, _a: &[Value], b: Blk) -> RV { save_and_yield(c, K_ROUND, b) }
fn s_save_limit(c: &mut Ctx, _s: Value, _a: &[Value], b: Blk) -> RV { save_and_yield(c, K_LIMIT, b) }

fn s_interpret_loosely(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let v = expect_str(c, a[0])?;
    let mut b = c.str_bytes(v).to_vec();
    if let Some(p) = b.iter().position(|&x| x == 0) { b.truncate(p); }
    let env = env(c)?;
    match vp!(c, vp_alloc(&b, false, &env)) { Ok(r) => check_get(c, r, &env), Err(()) => Ok(NIL) }
}

/// rb_convert_to_BigDecimal; digs None is "omitted" (SIZE_MAX).
fn convert(c: &mut Ctx, val: Value, digs: Option<usize>, raise: bool) -> RV {
    if val == NIL || val == TRUE || val == FALSE {
        if raise { return type_err(c, format!("can't convert {} into BigDecimal", if val == NIL { "nil" } else if val == TRUE { "true" } else { "false" })); }
        return Ok(NIL);
    }
    let env = env(c)?;
    if let Some(r) = real_of(c, val) {
        if digs.is_none() { check_exception(c, &r, false, &env)?; return Ok(val); }
        return check_get(c, r, &env);
    }
    if is_integer(val) { let r = int_to_real(c, val, &env)?; return Ok(wrap(c, r)); }
    if is_float(val) {
        return match float_to_real(c, float_val(val), digs.unwrap_or(0), raise, &env)? { Some(r) => Ok(wrap(c, r)), None => Ok(NIL) };
    }
    if crate::builtins::rational::is_rational(c, val) {
        return match rational_to_real(c, val, digs, raise, &env)? { Some(b) => Ok(b.v), None => Ok(NIL) };
    }
    if crate::builtins::complex::is_complex(c, val) {
        let (re, im) = crate::builtins::complex::cpx_parts(c, val);
        let zero = if im.is_fixnum() { im.as_fix() == 0 } else if is_bignum(im) { false }
            else if crate::builtins::rational::is_rational(c, im) { crate::builtins::rational::rat_parts(c, im).0 == Value::fix(0) }
            else { funcall(c, im, intern(b"=="), &[Value::fix(0)], std::ptr::null())?.truthy() };
        if !zero {
            if !raise { return Ok(NIL); }
            return arg_err(c, "Unable to make a BigDecimal from non-zero imaginary number");
        }
        return convert(c, re, digs, raise);
    }
    let s = if is_string(val) { val } else if val.is_heap() && respond_to(c, val, intern(b"to_str"), true) {
        let s = funcall(c, val, intern(b"to_str"), &[], std::ptr::null())?;
        if !is_string(s) { let cn = class_path(c, real_class_of(c, val)); return type_err(c, format!("can't convert {} to String ({}#to_str gives {})", cn, cn, class_path(c, real_class_of(c, s)))); }
        s
    } else {
        if raise { return type_err(c, format!("can't convert {} into BigDecimal", class_path(c, real_class_of(c, val)))); }
        return Ok(NIL);
    };
    let enc = c.rstring(s).enc;
    if !crate::builtins::encoding::ascii_compat(enc) {
        let n = crate::builtins::encoding::enc_name(enc);
        return err(c, c.vm().c_encoding_error.get(), format!("ASCII incompatible encoding: {}", n));
    }
    let b = c.str_bytes(s).to_vec();
    if b.contains(&0) {
        if !raise { return Ok(NIL); }
        return arg_err(c, "string contains null byte");
    }
    match vp!(c, vp_alloc(&b, true, &env)) {
        Ok(r) => check_get(c, r, &env),
        Err(()) => if raise { Err(invalid_value(c, &b)) } else { Ok(NIL) },
    }
}

fn k_bigdecimal(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (n, raise) = crate::builtins::rational::kw_exception(c, a)?;
    let a = &a[..n];
    check_argc(c, a, 1, 2)?;
    let mut digs = None;
    if a.len() > 1 {
        let d = to_integer(c, a[1])?;
        let neg = if d.is_fixnum() { d.as_fix() < 0 } else { c.big(d).neg };
        if neg { if !raise { return Ok(NIL); } return arg_err(c, "negative precision"); }
        digs = Some(if d.is_fixnum() { d.as_fix() as usize } else { usize::MAX - 1 });
    }
    convert(c, a[0], digs, raise)
}

fn m_add(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    if !is_coerceable(c, a[0]) { return coerce_bin(c, s, a[0], "+"); }
    addsub(c, s, a[0], 0, 1)
}
fn m_sub(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    if !is_coerceable(c, a[0]) { return coerce_bin(c, s, a[0], "-"); }
    addsub(c, s, a[0], 0, -1)
}
fn m_mult(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    if !is_coerceable(c, a[0]) { return coerce_bin(c, s, a[0], "*"); }
    mult_with_coerce(c, s, a[0], 0)
}
fn m_add2(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { let p = check_int_precision(c, a[1])?; addsub(c, s, a[0], p, 1) }
fn m_sub2(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { let p = check_int_precision(c, a[1])?; addsub(c, s, a[0], p, -1) }
fn m_mult2(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { let p = check_int_precision(c, a[1])?; mult_with_coerce(c, s, a[0], p) }

/// BigDecimal_div2(self, b, n)
fn div2_value(c: &mut Ctx, s: Value, b: Value, n: Value) -> RV {
    let env = env(c)?;
    if n == NIL {
        return match do_divmod(c, s, b, false, &env)? {
            Some((d, _)) => { check_exception(c, &d, false, &env)?; to_i_real(c, &d, &env) }
            None => coerce_bin(c, s, b, "div"),
        };
    }
    let ix = check_int_precision(c, n)?;
    let av = slf(c, s)?;
    Ok(div2(c, av, b, ix, &env)?.v)
}
fn m_divide(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    if !is_coerceable(c, a[0]) { return coerce_bin(c, s, a[0], "/"); }
    div2_value(c, s, a[0], Value::fix(0))
}
fn m_div(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    div2_value(c, s, a[0], a.get(1).copied().unwrap_or(NIL))
}
fn m_quo(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let n = if a.len() > 1 { check_int_precision(c, a[1])? as i64 } else { -1 };
    if n > 0 { return div2_value(c, s, a[0], a[1]); }
    m_divide(c, s, &a[..1], Blk::NONE)
}
fn m_mod(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let env = env(c)?;
    match do_divmod(c, s, a[0], false, &env)? { Some((_, m)) => check_get(c, m, &env), None => coerce_bin(c, s, a[0], "%") }
}
fn m_remainder(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let env = env(c)?;
    match do_divmod(c, s, a[0], true, &env)? { Some((_, m)) => check_get(c, m, &env), None => coerce_bin(c, s, a[0], "remainder") }
}
fn m_divmod(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let env = env(c)?;
    match do_divmod(c, s, a[0], false, &env)? {
        Some((d, m)) => {
            // rb_assoc_new's arguments: gcc evaluates the second (mod) first
            let mv = check_get(c, m, &env)?;
            check_exception(c, &d, false, &env)?;
            let q = to_i_real(c, &d, &env)?;
            Ok(c.ary_new_from(&[q, mv]))
        }
        None => coerce_bin(c, s, a[0], "divmod"),
    }
}

fn cmp(c: &mut Ctx, s: Value, r: Value, op: u8) -> RV {
    let env = env(c)?;
    let a = slf(c, s)?;
    let Some(b) = get_bd(c, r, get_coerce_prec(&a.r, 0), false, &env)? else {
        return match op {
            b'*' => coerce_cmp(c, s, r, "<=>"),
            b'=' => Ok(Value::bool(coerce_cmp(c, s, r, "==")?.truthy())),
            b'G' => coerce_relop(c, s, r, ">="),
            b'L' => coerce_relop(c, s, r, "<="),
            b'>' => coerce_relop(c, s, r, ">"),
            _ => coerce_relop(c, s, r, "<"),
        };
    };
    let e = vp_comp(&a.r, &b.r);
    if e == 999 { return Ok(if op == b'*' { NIL } else { FALSE }); }
    Ok(match op {
        b'*' => Value::fix(e as i64),
        b'=' => Value::bool(e == 0),
        b'G' => Value::bool(e >= 0),
        b'>' => Value::bool(e > 0),
        b'L' => Value::bool(e <= 0),
        _ => Value::bool(e < 0),
    })
}
fn m_cmp(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { cmp(c, s, a[0], b'*') }
fn m_eq(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { cmp(c, s, a[0], b'=') }
fn m_lt(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { cmp(c, s, a[0], b'<') }
fn m_le(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { cmp(c, s, a[0], b'L') }
fn m_gt(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { cmp(c, s, a[0], b'>') }
fn m_ge(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { cmp(c, s, a[0], b'G') }

fn m_zero_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(slf(c, s)?.r.is_zero())) }
fn m_nonzero_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(if slf(c, s)?.r.is_zero() { NIL } else { s }) }
fn m_coerce(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let env = env(c)?;
    let p = slf(c, s)?;
    let b = get_bd_must(c, a[0], get_coerce_prec(&p.r, 0), &env)?;
    let bv = check_get_v(c, b, &env)?;
    Ok(c.ary_new_from(&[bv, s]))
}
fn m_uplus(_c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(s) }
fn m_neg(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let env = env(c)?;
    let a = slf(c, s)?.r;
    let mut r = vp!(c, Real::zw(1, a.prec * BASE_FIG));
    vp!(c, vp_asgn(&mut r, &a, -10, &env));
    check_get(c, r, &env)
}
fn m_abs(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let env = env(c)?;
    let a = slf(c, s)?.r;
    let mut r = vp!(c, Real::zw(1, a.prec * BASE_FIG));
    vp!(c, vp_asgn(&mut r, &a, 10, &env));
    r.change_sign(1);
    check_get(c, r, &env)
}
fn fix_real(a: &Real, env: &Env) -> VR<Real> {
    let mut r = Real::zw(1, (a.prec + 1) * BASE_FIG)?;
    vp_active_round(&mut r, a, ROUND_DOWN, 0, env)?;
    Ok(r)
}
fn m_fix(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let env = env(c)?;
    let a = slf(c, s)?.r;
    let r = vp!(c, fix_real(&a, &env));
    check_get(c, r, &env)
}
fn m_frac(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let env = env(c)?;
    let a = slf(c, s)?.r;
    let mut r = vp!(c, Real::zw(1, (a.prec + 1) * BASE_FIG));
    vp!(c, vp_frac(&mut r, &a, &env));
    check_get(c, r, &env)
}
fn m_round(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 2)?;
    let env = env(c)?;
    let mut sw = env.round;
    let mut iloc = 0;
    let mut to_int = false;
    match a.len() {
        0 => { to_int = true; }
        1 => {
            if is_hash(a[0]) { sw = check_rounding_mode_option(c, a[0], &env)?; }
            else { iloc = num2int(c, a[0])?; if iloc < 1 { to_int = true; } }
        }
        _ => {
            iloc = num2int(c, a[0])?;
            sw = if is_hash(a[1]) { check_rounding_mode_option(c, a[1], &env)? } else { check_rounding_mode(c, a[1])? };
        }
    }
    let x = slf(c, s)?.r;
    let mut r = vp!(c, Real::zw(1, (x.prec + 1) * BASE_FIG));
    vp!(c, vp_active_round(&mut r, &x, sw, iloc as isize, &env));
    if to_int { check_exception(c, &r, false, &env)?; return to_i_real(c, &r, &env); }
    check_get(c, r, &env)
}
fn trunc_floor_ceil(c: &mut Ctx, s: Value, a: &[Value], mode: u16) -> RV {
    check_argc(c, a, 0, 1)?;
    let env = env(c)?;
    let iloc = if a.is_empty() { 0 } else { num2int(c, a[0])? };
    let x = slf(c, s)?.r;
    let mut r = vp!(c, Real::zw(1, (x.prec + 1) * BASE_FIG));
    vp!(c, vp_active_round(&mut r, &x, mode, iloc as isize, &env));
    if a.is_empty() { check_exception(c, &r, false, &env)?; return to_i_real(c, &r, &env); }
    check_get(c, r, &env)
}
fn m_truncate(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { trunc_floor_ceil(c, s, a, ROUND_DOWN) }
fn m_floor(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { trunc_floor_ceil(c, s, a, ROUND_FLOOR) }
fn m_ceil(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { trunc_floor_ceil(c, s, a, ROUND_CEIL) }

fn m_to_s(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let r = slf(c, s)?.r;
    let (mut fmt, mut f_plus, mut mc) = (false, 0, 0usize);
    if let Some(&f) = a.first() {
        if is_string(f) {
            let b = cstr(c, f)?;
            let mut p = 0;
            if b.first() == Some(&b' ') { f_plus = 1; p += 1; } else if b.first() == Some(&b'+') { f_plus = 2; p += 1; }
            while p < b.len() {
                let ch = b[p]; p += 1;
                if matches!(ch, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') { continue; }
                if !ch.is_ascii_digit() { if ch == b'F' || ch == b'f' { fmt = true; } break; }
                mc = mc.wrapping_mul(10).wrapping_add((ch - b'0') as usize);
            }
        } else {
            let m = num2int(c, f)?;
            if m <= 0 { return arg_err(c, "argument must be positive"); }
            mc = m as usize;
        }
    }
    let out = if fmt { vp_to_fstring(&r, mc, f_plus) } else { vp_to_string(&r, mc, f_plus) };
    Ok(c.str_new_enc(&out, ENC_ASCII))
}
fn m_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let r = slf(c, s)?.r;
    let out = vp_to_string(&r, 0, 0);
    Ok(c.str_new_enc(&out, ENC_BINARY))
}
fn m_split(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let r = slf(c, s)?.r;
    let mut m = vp_sz_mantissa(&r);
    let mut sg = 1;
    if m.first() == Some(&b'-') { m.remove(0); sg = -1; }
    if m.first() == Some(&b'N') { sg = 0; }
    let e = vp_exponent10(&r);
    let ms = c.str_new_enc(&m, ENC_BINARY);
    let ev = c.int_from_i64(e as i64);
    Ok(c.ary_new_from(&[Value::fix(sg), ms, Value::fix(10), ev]))
}
fn m_exponent(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let r = slf(c, s)?.r; Ok(c.int_from_i64(vp_exponent10(&r) as i64)) }
fn m_sign(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(slf(c, s)?.r.sign as i64)) }
fn m_nan_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(slf(c, s)?.r.is_nan())) }
fn m_infinite_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let r = slf(c, s)?.r;
    Ok(if r.is_pos_inf() { Value::fix(1) } else if r.is_neg_inf() { Value::fix(-1) } else { NIL })
}
fn m_finite_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let r = slf(c, s)?.r; Ok(Value::bool(!r.is_nan() && !r.is_inf())) }
fn m_to_i(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let env = env(c)?; let r = slf(c, s)?.r; to_i_real(c, &r, &env) }
fn m_to_f(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let env = env(c)?;
    let r = slf(c, s)?.r;
    let neg = r.sign < 0;
    let (d, e, f) = vp_vtod(&r);
    if f != 1 { return Ok(c.float_new(d)); }
    let over = |c: &mut Ctx| -> RV { vp!(c, vp_exception(&env, EXC_OVERFLOW, "BigDecimal to Float conversion", false)); Ok(c.float_new(if neg { f64::NEG_INFINITY } else { f64::INFINITY })) };
    let under = |c: &mut Ctx| -> RV { vp!(c, vp_exception(&env, EXC_UNDERFLOW, "BigDecimal to Float conversion", false)); Ok(c.float_new(if neg { -0.0 } else { 0.0 })) };
    if e > 308 + BASE_FIG as isize { return over(c); }
    if e < -307 - 15 { return under(c); }
    let s = String::from_utf8(vp_to_string(&r, 0, 0)).unwrap();
    let d: f64 = s.parse().unwrap_or(0.0);
    if d.is_infinite() { return over(c); }
    if d == 0.0 { return under(c); }
    Ok(c.float_new(d))
}
fn m_to_r(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let env = env(c)?;
    let r = slf(c, s)?.r;
    check_exception(c, &r, true, &env)?;
    let power = vp_exponent10(&r);
    let mut m = vp_sz_mantissa(&r);
    if m.first() == Some(&b'-') { m.remove(0); }
    let dp = power - m.len() as isize;
    let mut num = BigInt::parse(std::str::from_utf8(&m).unwrap(), 10).unwrap_or_else(BigInt::zero);
    if r.get_sign() < 0 { num = num.neg(); }
    let ten = BigInt::from_i64(10);
    let (n, d) = if dp < 0 { (num, ten.pow((-dp) as u64)) } else { (num.mul(&ten.pow(dp as u64)), BigInt::from_i64(1)) };
    let (n, d) = (c.big_to_value(n), c.big_to_value(d));
    let k = c.vm().m_kernel.get();
    funcall(c, k, intern(b"Rational"), &[n, d], std::ptr::null())
}
fn m_decimal_shift(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let env = env(c)?;
    let av = slf(c, s)?;
    let iv = to_integer(c, a[0])?;
    let mut shift = num_to_long(c, iv)? as isize;
    let ar = &av.r;
    if ar.is_zero() || ar.is_nan() || ar.is_inf() || shift == 0 { return check_get_v(c, av, &env); }
    let mut exponent_shift = if shift > 0 { shift / BASE_FIG as isize } else { (shift + 1) / BASE_FIG as isize - 1 };
    shift -= exponent_shift * BASE_FIG as isize;
    let ex = 10u32.pow(shift as u32);
    let shift_down = ar.frac[0] as u64 * ex as u64 >= BASE as u64;
    let iex = BASE / ex;
    let mut prec = ar.prec + shift_down as usize;
    let mut r = vp!(c, Real::zw(1, prec * BASE_FIG));
    r.ensure(prec + 1);
    if shift == 0 { vp!(c, vp_asgn(&mut r, ar, 10, &env)); }
    else if shift_down {
        let mut carry = 0u32;
        exponent_shift += 1;
        for i in 0..ar.prec { let v = ar.frac[i]; r.frac[i] = carry * ex + v / iex; carry = v % iex; }
        r.frac[ar.prec] = carry * ex;
    } else {
        let mut carry = 0u32;
        for i in (0..ar.prec).rev() { let v = ar.frac[i]; r.frac[i] = v % iex * ex + carry; carry = v / iex; }
    }
    while r.frac[prec - 1] == 0 { prec -= 1; }
    r.prec = prec;
    r.sign = ar.sign;
    r.exponent = ar.exponent;
    vp!(c, add_exponent(&mut r, exponent_shift, &env));
    check_get(c, r, &env)
}
fn m_clone(_c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(s) }

/// Run by the first `require "bigdecimal.so"`.
pub fn init(c: &mut Ctx) {
    if CLASS.get(c) != NIL { return; }
    let k = define_class(c, "BigDecimal", c.vm().c_numeric.get());
    class_set_alloc(c, k, AllocKind::None);
    CLASS.set(c, k);
    let kernel = c.vm().m_kernel.get();
    define_module_function(c, kernel, "BigDecimal", k_bigdecimal, -1);
    let sk = singleton_class(c, k);
    let _ = crate::builtins::object::undef_method(c, sk, intern(b"new"));
    let sd = |c: &mut Ctx, n: &str, f: M, a: i32| define_singleton_cfunc(c, k, n, f, a);
    sd(c, "interpret_loosely", s_interpret_loosely, 1); sd(c, "mode", s_mode, -1); sd(c, "limit", s_limit, -1); sd(c, "double_fig", s_double_fig, 0);
    sd(c, "_load", s_load, 1); sd(c, "save_exception_mode", s_save_exception_mode, 0); sd(c, "save_rounding_mode", s_save_rounding_mode, 0);
    sd(c, "save_limit", s_save_limit, 0);
    let v = c.str_new_frozen(b"4.1.3");
    const_set(c, k, intern(b"VERSION"), v);
    for (n, x) in [("BASE", BASE as i64), ("EXCEPTION_ALL", EXC_ALL as i64), ("EXCEPTION_NaN", EXC_NAN as i64), ("EXCEPTION_INFINITY", EXC_INFINITY as i64),
        ("EXCEPTION_UNDERFLOW", EXC_UNDERFLOW as i64), ("EXCEPTION_OVERFLOW", EXC_OVERFLOW as i64), ("EXCEPTION_ZERODIVIDE", EXC_ZERODIVIDE as i64),
        ("ROUND_MODE", ROUND_MODE as i64), ("ROUND_UP", ROUND_UP as i64), ("ROUND_DOWN", ROUND_DOWN as i64), ("ROUND_HALF_UP", ROUND_HALF_UP as i64),
        ("ROUND_HALF_DOWN", ROUND_HALF_DOWN as i64), ("ROUND_CEILING", ROUND_CEIL as i64), ("ROUND_FLOOR", ROUND_FLOOR as i64), ("ROUND_HALF_EVEN", ROUND_HALF_EVEN as i64),
        ("SIGN_NaN", SIGN_NAN as i64), ("SIGN_POSITIVE_ZERO", SIGN_POSITIVE_ZERO as i64), ("SIGN_NEGATIVE_ZERO", SIGN_NEGATIVE_ZERO as i64),
        ("SIGN_POSITIVE_FINITE", SIGN_POSITIVE_FINITE as i64), ("SIGN_NEGATIVE_FINITE", SIGN_NEGATIVE_FINITE as i64),
        ("SIGN_POSITIVE_INFINITE", SIGN_POSITIVE_INFINITE as i64), ("SIGN_NEGATIVE_INFINITE", SIGN_NEGATIVE_INFINITE as i64)] {
        const_set(c, k, intern(n.as_bytes()), Value::fix(x));
    }
    let special = |c: &mut Ctx, sign: i16| { let mut r = Real::alloc(1); r.set_special(sign); wrap(c, r) };
    let pi = special(c, SIGN_POSITIVE_INFINITE);
    const_set(c, k, intern(b"INFINITY"), pi);
    let nan = special(c, SIGN_NAN);
    const_set(c, k, intern(b"NAN"), nan);
    let d = |c: &mut Ctx, n: &str, f: M, a: i32| define_cfunc(c, k, n, f, a);
    d(c, "precision", m_precision, 0); d(c, "scale", m_scale, 0); d(c, "precision_scale", m_precision_scale, 0); d(c, "n_significant_digits", m_n_significant_digits, 0);
    d(c, "add", m_add2, 2); d(c, "sub", m_sub2, 2); d(c, "mult", m_mult2, 2); d(c, "div", m_div, -1); d(c, "hash", m_hash, 0); d(c, "to_s", m_to_s, -1);
    d(c, "to_i", m_to_i, 0); d(c, "to_int", m_to_i, 0); d(c, "to_r", m_to_r, 0); d(c, "split", m_split, 0); d(c, "+", m_add, 1); d(c, "-", m_sub, 1);
    d(c, "+@", m_uplus, 0); d(c, "-@", m_neg, 0); d(c, "*", m_mult, 1); d(c, "/", m_divide, 1); d(c, "quo", m_quo, -1); d(c, "%", m_mod, 1); d(c, "modulo", m_mod, 1);
    d(c, "remainder", m_remainder, 1); d(c, "divmod", m_divmod, 1); d(c, "clone", m_clone, 0); d(c, "dup", m_clone, 0); d(c, "to_f", m_to_f, 0);
    d(c, "abs", m_abs, 0); d(c, "fix", m_fix, 0); d(c, "round", m_round, -1); d(c, "frac", m_frac, 0); d(c, "floor", m_floor, -1); d(c, "ceil", m_ceil, -1);
    d(c, "<=>", m_cmp, 1); d(c, "==", m_eq, 1); d(c, "===", m_eq, 1); d(c, "eql?", m_eq, 1); d(c, "<", m_lt, 1); d(c, "<=", m_le, 1); d(c, ">", m_gt, 1); d(c, ">=", m_ge, 1);
    d(c, "zero?", m_zero_p, 0); d(c, "nonzero?", m_nonzero_p, 0); d(c, "coerce", m_coerce, 1); d(c, "inspect", m_inspect, 0); d(c, "exponent", m_exponent, 0);
    d(c, "sign", m_sign, 0); d(c, "nan?", m_nan_p, 0); d(c, "infinite?", m_infinite_p, 0); d(c, "finite?", m_finite_p, 0); d(c, "truncate", m_truncate, -1);
    d(c, "_decimal_shift", m_decimal_shift, 1); d(c, "_dump", m_dump, -1);
}
