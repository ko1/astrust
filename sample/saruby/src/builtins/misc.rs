// Assorted built-ins: pack/unpack, GC, ObjectSpace, Warning, RbConfig, constants.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{obj_klass, obj_set_flags, Global};
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::hash::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

// ---- Encoding (the methods and objects are encoding.rs's) --------------------------------

pub struct REnc { pub id: u8 }
impl RData for REnc {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
pub fn encoding_value(c: &mut Ctx, id: u8) -> RV { Ok(crate::builtins::encoding::encoding_value(c, id)) }

// ---- pack / unpack ----------------------------------------------------------------------

fn parse_tmpl(t: &[u8]) -> Vec<(u8, Option<usize>, bool, bool)> {
    let mut out = vec![];
    let mut i = 0;
    while i < t.len() {
        let ch = t[i]; i += 1;
        if ch.is_ascii_whitespace() { continue; }
        let mut little = false; let mut big = false;
        while i < t.len() && (t[i] == b'_' || t[i] == b'!' || t[i] == b'<' || t[i] == b'>') { if t[i] == b'<' { little = true; } if t[i] == b'>' { big = true; } i += 1; }
        let mut cnt = None; let mut star = false;
        if i < t.len() && t[i] == b'*' { star = true; i += 1; }
        else { let st = i; while i < t.len() && t[i].is_ascii_digit() { i += 1; } if i > st { cnt = Some(std::str::from_utf8(&t[st..i]).unwrap().parse().unwrap()); } }
        let _ = little;
        out.push((ch, if star { Some(usize::MAX) } else { cnt }, big, star));
    }
    out
}
pub fn pack(c: &mut Ctx, ary: Value, a: &[Value]) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, 1); }
    let t = expect_str(c, a[0])?;
    let tmpl = parse_tmpl(c.str_bytes(t));
    let mut out: Vec<u8> = vec![];
    let mut idx = 0;
    let mut binary = false;
    for (ch, cnt, big, _) in tmpl {
        let next = |c: &mut Ctx, idx: &mut usize| -> RV { if *idx >= c.ary_len(ary) { return arg_err(c, "too few arguments"); } let v = c.ary_get(ary, *idx); *idx += 1; Ok(v) };
        match ch {
            b'a' | b'A' | b'Z' => {
                let v = next(c, &mut idx)?; let sv = expect_str(c, v)?; let bs = c.str_bytes(sv).to_vec();
                let n = match cnt { None => 1, Some(usize::MAX) => bs.len() + if ch == b'Z' { 1 } else { 0 }, Some(n) => n };
                let pad = if ch == b'A' { b' ' } else { 0 };
                for k in 0..n { out.push(*bs.get(k).unwrap_or(&pad)); }
            }
            b'U' => {
                let n = match cnt { None => 1, Some(usize::MAX) => c.ary_len(ary).saturating_sub(idx), Some(n) => n };
                for _ in 0..n { let v = next(c, &mut idx)?; let cp = num_to_long(c, v)?; if cp < 0 { return err(c, c.vm().c_range_error, "pack(U): value out of range"); } encode_utf8(cp as u32, &mut out); }
            }
            b'C' | b'c' | b'S' | b's' | b'L' | b'l' | b'Q' | b'q' | b'N' | b'n' | b'V' | b'v' | b'J' | b'j' => {
                binary = true;
                let n = match cnt { None => 1, Some(usize::MAX) => c.ary_len(ary).saturating_sub(idx), Some(n) => n };
                for _ in 0..n {
                    let v = next(c, &mut idx)?;
                    let x = if crate::object::is_bignum(v) { c.big(v).bitop(&crate::bignum::BigInt::from_u64(u64::MAX), b'&').to_i64().unwrap_or(-1) } else { num_to_long(c, v)? } as u64;
                    let (sz, be) = match ch { b'C' | b'c' => (1, false), b'S' | b's' => (2, big), b'L' | b'l' => (4, big), b'Q' | b'q' | b'J' | b'j' => (8, big), b'N' => (4, true), b'n' => (2, true), b'V' => (4, false), _ => (2, false) };
                    let bytes = x.to_le_bytes();
                    if be { for k in (0..sz).rev() { out.push(bytes[k]); } } else { out.extend_from_slice(&bytes[..sz]); }
                }
            }
            b'E' | b'e' | b'D' | b'd' | b'F' | b'f' | b'G' | b'g' => {
                binary = true;
                let n = match cnt { None => 1, Some(usize::MAX) => c.ary_len(ary).saturating_sub(idx), Some(n) => n };
                for _ in 0..n {
                    let v = next(c, &mut idx)?; let f = crate::builtins::numeric_methods::to_f(c, v)?;
                    match ch { b'E' | b'D' | b'd' => out.extend_from_slice(&f.to_le_bytes()), b'G' => out.extend_from_slice(&f.to_be_bytes()),
                        b'e' | b'F' | b'f' => out.extend_from_slice(&(f as f32).to_le_bytes()), _ => out.extend_from_slice(&(f as f32).to_be_bytes()) }
                }
            }
            b'H' | b'h' => {
                let v = next(c, &mut idx)?; let sv = expect_str(c, v)?; let hs = c.str_bytes(sv).to_vec();
                let n = match cnt { None => 1, Some(usize::MAX) => hs.len(), Some(n) => n };
                let mut k = 0;
                while k < n { let hi = hex_v(*hs.get(k).unwrap_or(&b'0')); let lo = hex_v(*hs.get(k + 1).unwrap_or(&b'0')); out.push(if ch == b'H' { hi << 4 | if k + 1 < n { lo } else { 0 } } else { lo << 4 | hi }); k += 2; }
            }
            b'm' => {
                let v = next(c, &mut idx)?; let sv = expect_str(c, v)?;
                let enc = base64(c.str_bytes(sv));
                let line = match cnt { Some(0) => usize::MAX, Some(n) if n != usize::MAX => (n / 3 * 4).max(4), _ => 60 };
                if line == usize::MAX { out.extend_from_slice(enc.as_bytes()); } else { for ch2 in enc.as_bytes().chunks(line) { out.extend_from_slice(ch2); out.push(b'\n'); } }
            }
            b'x' => { let n = cnt.unwrap_or(1); for _ in 0..n { out.push(0); } }
            b'w' => {
                let n = match cnt { None => 1, Some(usize::MAX) => c.ary_len(ary).saturating_sub(idx), Some(n) => n };
                for _ in 0..n { let v = next(c, &mut idx)?; let mut x = num_to_long(c, v)? as u64; let mut tmp = vec![(x & 0x7f) as u8]; x >>= 7; while x > 0 { tmp.push((x & 0x7f) as u8 | 0x80); x >>= 7; } tmp.reverse(); out.extend(tmp); }
            }
            b'M' => {
                let v = next(c, &mut idx)?; let sv = crate::rt::srb_tostr(c, v)?;
                let len = match cnt { Some(n) if n > 1 && n != usize::MAX => n, _ => 72 };
                let (mut n, mut prev) = (0usize, 0u8);
                for &x in c.str_bytes(sv) {
                    if x > 126 || (x < 32 && x != b'\n' && x != b'\t') || x == b'=' { out.extend_from_slice(format!("={:02X}", x).as_bytes()); n += 3; prev = 0; }
                    else if x == b'\n' { if prev == b' ' || prev == b'\t' { out.push(b'='); out.push(x); } out.push(x); n = 0; prev = x; }
                    else { out.push(x); n += 1; prev = x; }
                    if n > len { out.extend_from_slice(b"=\n"); n = 0; prev = b'\n'; }
                }
                if n > 0 { out.extend_from_slice(b"=\n"); }
            }
            b'u' => {
                let v = next(c, &mut idx)?; let sv = expect_str(c, v)?;
                let line = match cnt { Some(n) if n >= 3 && n != usize::MAX => n / 3 * 3, _ => 45 };
                let enc = |v: u8| if v == 0 { b'`' } else { v + 32 };
                for chunk in c.str_bytes(sv).chunks(line) {
                    out.push(enc(chunk.len() as u8));
                    for g in chunk.chunks(3) {
                        let (a0, a1, a2) = (g[0], *g.get(1).unwrap_or(&0), *g.get(2).unwrap_or(&0));
                        out.extend_from_slice(&[enc(a0 >> 2), enc((a0 << 4 | a1 >> 4) & 63), enc((a1 << 2 | a2 >> 6) & 63), enc(a2 & 63)]);
                    }
                    out.push(b'\n');
                }
            }
            b'b' | b'B' => {
                let v = next(c, &mut idx)?; let sv = expect_str(c, v)?; let bits = c.str_bytes(sv);
                let n = match cnt { None => 1, Some(usize::MAX) => bits.len(), Some(n) => n };
                let start = out.len();
                // CRuby pads a short bit string with (missing + 1) / 2 NUL bytes
                let m = n.min(bits.len());
                out.resize(start + (m + 7) / 8 + (n - m + 1) / 2, 0);
                for k in 0..m { if bits[k] & 1 == 1 { out[start + k / 8] |= if ch == b'B' { 0x80 >> (k % 8) } else { 1 << (k % 8) }; } }
                binary = true;
            }
            b'X' => { let n = match cnt { None => 1, Some(usize::MAX) => 0, Some(n) => n }; if n > out.len() { return arg_err(c, "X outside of string"); } out.truncate(out.len() - n); }
            b'@' => { let n = match cnt { None => 1, Some(usize::MAX) => 0, Some(n) => n }; out.resize(n, 0); }
            _ => return arg_err(c, format!("unknown pack directive '{}'", ch as char)),
        }
    }
    Ok(c.str_from_vec_enc(out, if binary { ENC_BINARY } else { ENC_UTF8 }))
}
fn hex_v(x: u8) -> u8 { (x as char).to_digit(16).unwrap_or(0) as u8 }
pub fn base64(b: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for ch in b.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char); out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if ch.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if ch.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}
pub fn unbase64(s2: &[u8]) -> Vec<u8> {
    let mut out = vec![]; let mut buf = 0u32; let mut bits = 0;
    for &x in s2 {
        let v = match x { b'A'..=b'Z' => x - b'A', b'a'..=b'z' => x - b'a' + 26, b'0'..=b'9' => x - b'0' + 52, b'+' | b'-' => 62, b'/' | b'_' => 63, _ => continue } as u32;
        buf = buf << 6 | v; bits += 6;
        if bits >= 8 { bits -= 8; out.push((buf >> bits) as u8); }
    }
    out
}
pub fn unpack(c: &mut Ctx, sv: Value, a: &[Value]) -> RV {
    let t = expect_str(c, a[0])?;
    let tmpl = parse_tmpl(c.str_bytes(t));
    let b = c.str_bytes(sv).to_vec();
    let mut pos = 0;
    let out = c.ary_new();
    for (ch, cnt, big, _) in tmpl {
        match ch {
            b'a' | b'A' | b'Z' => {
                let n = match cnt { None => 1, Some(usize::MAX) => b.len().saturating_sub(pos), Some(n) => n };
                let e = (pos + n).min(b.len());
                let mut v = b[pos..e].to_vec(); pos = e;
                if ch == b'A' { while v.last().map(|x| *x == b' ' || *x == 0).unwrap_or(false) { v.pop(); } }
                if ch == b'Z' { if let Some(z) = v.iter().position(|&x| x == 0) { v.truncate(z); } }
                let s = c.str_from_vec_enc(v, if ch == b'a' || ch == b'Z' || ch == b'A' { ENC_BINARY } else { ENC_UTF8 }); c.ary_push(out, s);
            }
            b'U' => {
                let n = match cnt { None => 1, Some(usize::MAX) => usize::MAX, Some(n) => n };
                let mut k = 0;
                while k < n && pos < b.len() { let (cp, l) = decode_utf8(&b, pos); c.ary_push(out, Value::fix(cp as i64)); pos += l; k += 1; }
            }
            b'C' | b'c' | b'S' | b's' | b'L' | b'l' | b'Q' | b'q' | b'N' | b'n' | b'V' | b'v' | b'J' | b'j' => {
                let (sz, be, signed) = match ch { b'C' => (1, false, false), b'c' => (1, false, true), b'S' => (2, big, false), b's' => (2, big, true), b'L' => (4, big, false), b'l' => (4, big, true),
                    b'Q' | b'J' => (8, big, false), b'q' | b'j' => (8, big, true), b'N' => (4, true, false), b'n' => (2, true, false), b'V' => (4, false, false), _ => (2, false, false) };
                let n = match cnt { None => 1, Some(usize::MAX) => (b.len() - pos.min(b.len())) / sz, Some(n) => n };
                for _ in 0..n {
                    if pos + sz > b.len() { c.ary_push(out, NIL); continue; }
                    let mut x: u64 = 0;
                    for k in 0..sz { let byte = if be { b[pos + k] } else { b[pos + sz - 1 - k] }; x = x << 8 | byte as u64; }
                    pos += sz;
                    let v = if signed { let sh = 64 - sz * 8; ((x << sh) as i64) >> sh } else { x as i64 };
                    let iv = if !signed && sz == 8 { c.int_from_i128(x as i128) } else { Value::fix(v) };
                    c.ary_push(out, iv);
                }
            }
            b'E' | b'D' | b'd' | b'G' | b'e' | b'F' | b'f' | b'g' => {
                let sz = if matches!(ch, b'E' | b'D' | b'd' | b'G') { 8 } else { 4 };
                let n = match cnt { None => 1, Some(usize::MAX) => (b.len() - pos.min(b.len())) / sz, Some(n) => n };
                for _ in 0..n {
                    if pos + sz > b.len() { c.ary_push(out, NIL); continue; }
                    let sl = &b[pos..pos + sz]; pos += sz;
                    let f = match ch { b'E' | b'D' | b'd' => f64::from_le_bytes(sl.try_into().unwrap()), b'G' => f64::from_be_bytes(sl.try_into().unwrap()),
                        b'g' => f32::from_be_bytes(sl.try_into().unwrap()) as f64, _ => f32::from_le_bytes(sl.try_into().unwrap()) as f64 };
                    let fv = c.float_new(f); c.ary_push(out, fv);
                }
            }
            b'H' | b'h' => {
                let n = match cnt { None => 1, Some(usize::MAX) => (b.len() - pos) * 2, Some(n) => n };
                let mut s2 = String::new();
                let mut k = 0;
                while k < n && pos + k / 2 < b.len() { let byte = b[pos + k / 2]; let nib = if (k % 2 == 0) == (ch == b'H') { byte >> 4 } else { byte & 15 }; s2.push(std::char::from_digit(nib as u32, 16).unwrap()); k += 1; }
                pos += (k + 1) / 2;
                let s = c.str_new(s2.as_bytes()); c.ary_push(out, s);
            }
            b'm' => { let s = c.str_from_vec_enc(unbase64(&b[pos..]), ENC_BINARY); c.ary_push(out, s); pos = b.len(); }
            b'x' => { pos += cnt.unwrap_or(1); }
            b'w' => {
                let n = match cnt { None => 1, Some(usize::MAX) => usize::MAX, Some(n) => n };
                let mut k = 0;
                while k < n && pos < b.len() { let mut x: u64 = 0; loop { let byte = b[pos]; pos += 1; x = x << 7 | (byte & 0x7f) as u64; if byte & 0x80 == 0 || pos >= b.len() { break; } } c.ary_push(out, Value::fix(x as i64)); k += 1; }
            }
            b'M' => {
                let mut v = vec![]; let s2 = &b[pos.min(b.len())..]; let mut i = 0;
                while i < s2.len() {
                    if s2[i] == b'=' {
                        if i + 1 >= s2.len() { v.extend_from_slice(&s2[i..]); break; }
                        if s2[i + 1] == b'\n' { i += 2; continue; }
                        if s2[i + 1] == b'\r' && s2.get(i + 2) == Some(&b'\n') { i += 3; continue; }
                        if i + 2 < s2.len() && s2[i + 1].is_ascii_hexdigit() && s2[i + 2].is_ascii_hexdigit() { v.push(hex_v(s2[i + 1]) << 4 | hex_v(s2[i + 2])); i += 3; continue; }
                        v.extend_from_slice(&s2[i..]);
                        break;
                    }
                    v.push(s2[i]); i += 1;
                }
                pos = b.len();
                let s = c.str_from_vec_enc(v, ENC_BINARY); c.ary_push(out, s);
            }
            b'u' => {
                let mut v = vec![]; let s2 = &b[pos.min(b.len())..];
                for line in s2.split(|&x| x == b'\n') {
                    if line.is_empty() { continue; }
                    let n = ((line[0].wrapping_sub(32)) & 63) as usize;
                    if n == 0 { continue; }
                    let d: Vec<u8> = line[1..].iter().map(|&x| x.wrapping_sub(32) & 63).collect();
                    let mut got = vec![];
                    for g in d.chunks(4) { let q = [g[0], *g.get(1).unwrap_or(&0), *g.get(2).unwrap_or(&0), *g.get(3).unwrap_or(&0)]; got.extend_from_slice(&[q[0] << 2 | q[1] >> 4, q[1] << 4 | q[2] >> 2, q[2] << 6 | q[3]]); }
                    got.truncate(n);
                    v.extend(got);
                }
                pos = b.len();
                let s = c.str_from_vec_enc(v, ENC_BINARY); c.ary_push(out, s);
            }
            b'b' | b'B' => {
                let avail = b.len().saturating_sub(pos) * 8;
                let n = match cnt { None => 1, Some(usize::MAX) => avail, Some(n) => n.min(avail) };
                let mut s2 = Vec::with_capacity(n);
                for k in 0..n { let byte = b[pos + k / 8]; let bit = if ch == b'B' { byte >> (7 - k % 8) } else { byte >> (k % 8) } & 1; s2.push(b'0' + bit); }
                pos += (n + 7) / 8;
                let s = c.str_from_vec_enc(s2, ENC_ASCII); c.ary_push(out, s);
            }
            b'X' => { let n = match cnt { None => 1, Some(usize::MAX) => 0, Some(n) => n }; if n > pos { return arg_err(c, "X outside of string"); } pos -= n; }
            b'@' => { let n = match cnt { None => 1, Some(usize::MAX) => 0, Some(n) => n }; if n > b.len() { return arg_err(c, "@ outside of string"); } pos = n; }
            _ => return arg_err(c, format!("unknown unpack directive '{}'", ch as char)),
        }
    }
    Ok(out)
}

// ---- GC / ObjectSpace -----------------------------------------------------------------------

fn gc_start(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { crate::gc::collect(); Ok(NIL) }
fn gc_count(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(crate::gc::heap().ctl.collections() as i64)) }
fn gc_disable(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(crate::gc::heap().set_disabled(true))) }
fn gc_enable(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(crate::gc::heap().set_disabled(false))) }
fn gc_stat(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (n, h) = (crate::gc::heap().ctl.collections() as i64, crate::gc::heap().stats());
    let st = c.hash_new();
    for (k, v) in [("count", n), ("heap_live_slots", (h.slots - h.free) as i64), ("heap_free_slots", h.free as i64),
                   ("total_allocated_objects", h.allocated as i64), ("total_freed_objects", h.freed as i64), ("heap_available_slots", h.slots as i64), ("major_gc_count", n), ("minor_gc_count", 0)] {
        hash_set(c, st, sym_value(k), Value::fix(v))?;
    }
    if let Some(&k) = a.first() { if k.is_sym() { return Ok(hash_get_opt(c, st, k)?.unwrap_or(Value::fix(0))); } }
    Ok(st)
}
fn os_each_object(c: &mut Ctx, _s: Value, a: &[Value], blk: *const Block) -> RV {
    let k = a.first().copied();
    // nothing allocates between the walk and the Array that roots the result
    let mut vs = vec![];
    crate::gc::each_object(|v| vs.push(v));
    vs.retain(|&v| {
        if matches!(c.body_of(v), Body::Env(..)) || obj_klass(v) == NIL { return false; }
        if is_class_or_module(v) && c.class(v).is_singleton { return false; }
        match k { Some(k) => is_class_or_module(obj_klass(v)) && is_kind_of(v, k), None => true }
    });
    let n = vs.len();
    let list = c.ary_from_vec(vs);
    if blk.is_null() { return funcall(c, list, s::each, &[], std::ptr::null()); }
    for i in 0..n { let x = c.ary_get(list, i); yield1(c, blk, x)?; }
    Ok(Value::fix(n as i64))
}
fn os_count_objects(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let h = crate::gc::heap().stats();
    let st = c.hash_new();
    hash_set(c, st, sym_value("TOTAL"), Value::fix(h.slots as i64))?;
    hash_set(c, st, sym_value("FREE"), Value::fix(h.free as i64))?;
    Ok(st)
}
fn os_garbage_collect(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { crate::gc::collect(); Ok(NIL) }

fn warning_warn(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let b = to_rstring(c, a[0])?; err_write(b.as_bytes()); Ok(NIL) }
fn objspace_wm_init(c: &mut Ctx, s2: Value, _a: &[Value], _b: *const Block) -> RV { let h = c.hash_new(); c.hash_mut(h).identity = true; ivar_set_raw(s2, intern(b"@h"), h); Ok(NIL) }
fn wm_aref(c: &mut Ctx, s2: Value, a: &[Value], _b: *const Block) -> RV { let h = ivar_get(s2, intern(b"@h")); Ok(hash_get_opt(c, h, a[0])?.unwrap_or(NIL)) }
fn wm_aset(c: &mut Ctx, s2: Value, a: &[Value], _b: *const Block) -> RV { let h = ivar_get(s2, intern(b"@h")); hash_set(c, h, a[0], a[1])?; Ok(a[1]) }
fn wm_key(c: &mut Ctx, s2: Value, a: &[Value], _b: *const Block) -> RV { let h = ivar_get(s2, intern(b"@h")); Ok(Value::bool(hash_lookup(c, h, a[0])?.is_some())) }

static FINALIZERS: Global = Global::new(); // Array of [object, callable]; run at exit
fn finalizers(c: &mut Ctx) -> Value {
    let f = FINALIZERS.get(c);
    if f != NIL { return f; }
    let a = c.ary_new(); FINALIZERS.set(c, a); a
}
fn os_define_finalizer(c: &mut Ctx, _s: Value, a: &[Value], blk: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let o = a[0];
    if !o.is_heap() || is_float(o) || is_bignum(o) { return arg_err(c, format!("cannot define finalizer for {}", class_path(real_class_of(o)))); }
    if is_frozen(o) { return Err(raise_frozen(c, o)); }
    let p = if a.len() > 1 { a[1] } else if !blk.is_null() { block_to_proc(c, blk)? } else { return arg_err(c, "tried to create Proc object without a block"); };
    if !respond_to(p, intern(b"call"), true) { let d = inspect_str(c, p)?; return arg_err(c, format!("wrong type argument {} (should be callable)", d)); }
    let pair = c.ary_new_from(&[o, p]);
    let f = finalizers(c);
    c.ary_push(f, pair);
    Ok(c.ary_new_from(&[Value::fix(0), p]))
}
fn os_undefine_finalizer(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0].is_heap() && is_frozen(a[0]) { return Err(raise_frozen(c, a[0])); }
    let f = finalizers(c);
    let (v, others) = c.ary_mut_and(f);
    v.retain(|&p| others.ary(p).and_then(|x| x.first()) != Some(&a[0]));
    Ok(a[0])
}
/// Finalizers of objects still registered at exit.
pub fn run_finalizers(c: &mut Ctx) {
    let f = FINALIZERS.get(c);
    if f == NIL { return; }
    loop {
        let Some(pair) = c.ary_mut(f).pop() else { break };
        let (o, p) = (c.ary_get(pair, 0), c.ary_get(pair, 1));
        let id = Value::fix(object_id(o) as i64);
        if funcall(c, p, intern(b"call"), &[id], std::ptr::null()).is_err() { c.esc = EscKind::None; }
    }
}
fn rbconfig_ruby(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "saruby".into());
    Ok(c.str_new(exe.as_bytes()))
}
pub fn rbconfig_init(c: &mut Ctx) {
    if const_get_at(c.vm().c_object, intern(b"RbConfig")).is_some() { return; }
    let rbc = define_module("RbConfig");
    let cfg = c.hash_new_capa(32);
    let exe = std::env::current_exe().unwrap_or_else(|_| "saruby".into());
    let bindir = exe.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| ".".into());
    let name = exe.file_name().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "saruby".into());
    let (lib, _gem) = crate::builtins::prelude::stdlib_dirs().unwrap_or_default();
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" };
    let archname = format!("{}-linux", arch);
    let archdir = format!("{}/{}", lib, archname);
    let entries: &[(&str, &str)] = &[("ruby_install_name", &name), ("RUBY_INSTALL_NAME", &name), ("RUBY_BASE_NAME", "ruby"), ("bindir", &bindir), ("EXEEXT", ""),
        ("host_os", "linux"), ("target_os", "linux"), ("host_cpu", arch), ("target_cpu", arch), ("arch", &archname), ("host", &archname),
        ("MAJOR", "3"), ("MINOR", "4"), ("TEENY", "0"), ("PATCHLEVEL", "0"), ("ruby_version", "3.4.0"), ("RUBY_PROGRAM_VERSION", "3.4.0"),
        ("RUBY_API_VERSION", "3.4"), ("rubylibdir", &lib), ("archdir", &archdir), ("rubyarchdir", &archdir), ("DLEXT", "so"), ("SOEXT", "so"),
        ("LIBEXT", "a"), ("OBJEXT", "o"), ("PATH_SEPARATOR", ":"), ("SHELL", "/bin/sh"), ("CC", "gcc"), ("CPP", "gcc -E"), ("ENABLE_SHARED", "yes"),
        ("build_os", "linux"), ("build_cpu", arch), ("build", &archname), ("LIBRUBY_SO", "libsaruby.so"), ("prefix", &bindir), ("exec_prefix", &bindir),
        ("libdir", &bindir), ("sitedir", ""), ("sitelibdir", ""), ("sitearchdir", ""), ("vendordir", ""), ("vendorlibdir", ""), ("vendorarchdir", ""),
        ("datadir", ""), ("mandir", ""), ("sysconfdir", "/etc"), ("localstatedir", "/var"), ("configure_args", ""), ("CFLAGS", ""), ("LDFLAGS", "")];
    for (k, v) in entries { let ks = c.str_new_frozen(k.as_bytes()); let vs = c.str_new(v.as_bytes()); let _ = hash_set(c, cfg, ks, vs); }
    const_set(rbc, intern(b"CONFIG"), cfg);
    const_set(rbc, intern(b"MAKEFILE_CONFIG"), cfg);
    let top = c.str_new(bindir.as_bytes());
    const_set(rbc, intern(b"TOPDIR"), top);
    define_singleton_cfunc(rbc, "ruby", rbconfig_ruby, 0);
}

pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    Ok(if c.data::<REnc>(v).is_some() { Some(v) } else { None })
}
pub fn alloc_data(_c: &mut Ctx, _klass: Value) -> Result<Option<Value>, Esc> { Ok(None) }

/// Boot: no context yet, so the frozen constant Strings are allocated directly.
fn boot_str(vmr: &Vm, b: &[u8]) -> Value {
    let s = crate::gc::alloc(vmr.c_string, Body::String(RString { b: crate::sbuf::SBuf::from_slice(b), enc: ENC_UTF8 }));
    obj_set_flags(s, FL_FROZEN);
    s
}

pub fn init(vmr: &mut Vm) {
    let ob = vmr.c_object;
    // GC / ObjectSpace
    let gc = define_module("GC");
    vmr.m_gc = gc;
    define_module_function(gc, "start", gc_start, -1);
    define_module_function(gc, "count", gc_count, 0);
    define_module_function(gc, "disable", gc_disable, 0);
    define_module_function(gc, "enable", gc_enable, 0);
    define_module_function(gc, "stat", gc_stat, -1);
    define_module_function(gc, "compact", gc_start, 0);
    define_module_function(gc, "stress", |_c, _s, _a, _b| Ok(FALSE), 0);
    define_module_function(gc, "stress=", |_c, _s, a, _b| Ok(a[0]), 1);
    let os = define_module("ObjectSpace");
    vmr.m_object_space = os;
    define_module_function(os, "each_object", os_each_object, -1);
    define_module_function(os, "count_objects", os_count_objects, -1);
    define_module_function(os, "garbage_collect", os_garbage_collect, -1);
    define_module_function(os, "define_finalizer", os_define_finalizer, -1);
    define_module_function(os, "undefine_finalizer", os_undefine_finalizer, 1);
    let wm = define_class_under(os, "WeakMap", ob);
    vmr.c_object_space_weak_map = wm;
    define_private_cfunc(wm, "initialize", objspace_wm_init, 0);
    define_cfunc(wm, "[]", wm_aref, 1);
    define_cfunc(wm, "[]=", wm_aset, 2);
    define_cfunc(wm, "key?", wm_key, 1);
    define_cfunc(wm, "include?", wm_key, 1);
    let wn = define_module("Warning");
    vmr.m_warning = wn;
    define_module_function(wn, "warn", warning_warn, -1);
    define_singleton_cfunc(wn, "[]", |_c, _s, _a, _b| Ok(FALSE), 1);
    define_singleton_cfunc(wn, "[]=", |_c, _s, a, _b| Ok(a[1]), 2);
    // constants
    let set = |n: &str, v: Value| const_set(ob, intern(n.as_bytes()), v);
    for (n, v) in [("RUBY_VERSION", "3.4.0"), ("RUBY_RELEASE_DATE", "2026-11-20"), ("RUBY_PLATFORM", "x86_64-linux"), ("RUBY_ENGINE", "saruby"),
                   ("RUBY_ENGINE_VERSION", "0.1.0"), ("RUBY_COPYRIGHT", "saruby - Copyright (C) 2026 Koichi Sasada")] { set(n, boot_str(vmr, v.as_bytes())); }
    set("RUBY_DESCRIPTION", boot_str(vmr, crate::version().as_bytes()));
    set("RUBY_PATCHLEVEL", Value::fix(0));
    set("ARGV", vmr.argv);
    set("TOPLEVEL_BINDING", NIL);
    let _ = (intern, num_to_long);
    vmr.c_location = crate::builtins::exception::init_location(vmr);
}
