// Array#pack / String#unpack / String#unpack1, ported from CRuby's pack.c (x86_64-linux: little endian,
// int 4, long 8, pointer 8).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::bignum::BigInt;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::{num_to_long, to_integer};
use crate::object::is_string;
use crate::string::*;
use crate::value::*;
use crate::vm::*;

const NATSTR: &[u8] = b"sSiIlLqQjJ";
const TOO_FEW: &str = "too few arguments";
// p / P: the "pointer" is the String's object address; the packed String keeps the Strings in a hidden
// ivar (str_associate), and unpack hands back only those
fn assoc_sym() -> crate::value::Sym { crate::symbol::intern(b"__pack_associated") }
fn associated_pointer(c: &mut Ctx, assoc: Value, t: u64) -> RV {
    if crate::object::is_array(assoc) { for i in 0..c.ary_len(assoc) { let s = c.ary_get(assoc, i); if is_string(s) && s.raw() as u64 == t { return Ok(s); } } }
    arg_err(c, "no associated pointer")
}

fn is_space(b: u8) -> bool { matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') }

fn unknown_directive<T>(c: &mut Ctx, mode: &str, ty: u8, fmt: &[u8]) -> Result<T, Esc> {
    let t = if (0x20..0x7f).contains(&ty) { (ty as char).to_string() } else { format!("\\x{:02x}", ty) };
    let f = if fmt.iter().all(|&b| (0x20..0x7f).contains(&b)) { String::from_utf8_lossy(fmt).into_owned() } else {
        let s = inspect_bytes(fmt, crate::object::ENC_BINARY); s[1..s.len() - 1].to_string()
    };
    arg_err(c, format!("unknown {} directive '{}' in '{}'", mode, t, f))
}

/// One directive: (type, natint, explicit endian, len, star).  len is None when no count was given.
struct Dir { ty: u8, natint: bool, endian: u8, len: Option<usize>, star: bool, at_end: bool }

/// Next directive at `*p`, or None at the end.  Blanks and `#` comments are skipped.
fn next_dir(c: &mut Ctx, f: &[u8], p: &mut usize) -> Result<Option<Dir>, Esc> {
    loop {
        if *p >= f.len() { return Ok(None); }
        let ty = f[*p]; *p += 1;
        if is_space(ty) { continue; }
        if ty == b'#' { while *p < f.len() && f[*p] != b'\n' { *p += 1; } if *p < f.len() { *p += 1; } continue; }
        let (mut natint, mut endian) = (false, 0u8);
        while *p < f.len() {
            match f[*p] {
                b'_' | b'!' => {
                    if !NATSTR.contains(&ty) { return arg_err(c, format!("'{}' allowed only after types {}", f[*p] as char, std::str::from_utf8(NATSTR).unwrap())); }
                    natint = true; *p += 1;
                }
                b'<' | b'>' => {
                    if !NATSTR.contains(&ty) { return arg_err(c, format!("'{}' allowed only after types {}", f[*p] as char, std::str::from_utf8(NATSTR).unwrap())); }
                    if endian != 0 { return err(c, c.vm().c_range_error.get(), "Can't use both '<' and '>'"); }
                    endian = f[*p]; *p += 1;
                }
                _ => break,
            }
        }
        let at_end = *p >= f.len();
        let (mut len, mut star) = (None, false);
        if *p < f.len() && f[*p] == b'*' { star = true; *p += 1; }
        else if *p < f.len() && f[*p].is_ascii_digit() {
            let st = *p; while *p < f.len() && f[*p].is_ascii_digit() { *p += 1; }
            match std::str::from_utf8(&f[st..*p]).unwrap().parse::<u64>() {
                Ok(n) if n <= i64::MAX as u64 => len = Some(n as usize),
                _ => return err(c, c.vm().c_range_error.get(), "pack length too big"),
            }
        }
        return Ok(Some(Dir { ty, natint, endian, len, star, at_end }));
    }
}

/// (size, signed, big endian) of an integer directive.
fn int_kind(d: &Dir) -> Option<(usize, bool, bool)> {
    let (sz, signed, mut be) = match d.ty {
        b'c' => (1, true, false), b'C' => (1, false, false),
        b's' => (2, true, false), b'S' => (2, false, false),
        b'i' => (4, true, false), b'I' => (4, false, false),
        b'l' => (if d.natint { 8 } else { 4 }, true, false), b'L' => (if d.natint { 8 } else { 4 }, false, false),
        b'q' | b'j' => (8, true, false), b'Q' | b'J' => (8, false, false),
        b'n' => (2, false, true), b'N' => (4, false, true), b'v' => (2, false, false), b'V' => (4, false, false),
        _ => return None,
    };
    if d.endian != 0 { be = d.endian == b'>'; }
    Some((sz, signed, be))
}

/// Low 64 bits (two's complement) of an Integer.
fn int_low64(c: &mut Ctx, v: Value) -> u64 {
    if v.is_fixnum() { return v.as_fix() as u64; }
    let b = c.big(v);
    let m = BigInt::from_u64(0xffff_ffff);
    let lo = b.bitop(&m, b'&').to_i64().unwrap_or(0) as u64;
    let hi = b.shr(32).bitop(&m, b'&').to_i64().unwrap_or(0) as u64;
    hi << 32 | lo
}

/// rb_to_float: Numeric only; anything else is a TypeError.
fn to_float(c: &mut Ctx, v: Value) -> Result<f64, Esc> {
    if let Some(f) = c.num_to_f64(v) { return Ok(f); }
    if v.is_heap() && is_kind_of(c, v, c.vm().c_numeric.get()) { return crate::builtins::numeric_methods::to_f(c, v); }
    type_err(c, format!("can't convert {} into Float", class_name_for_err(c, v)))
}
fn to_float32(c: &mut Ctx, v: Value) -> Result<f32, Esc> {
    let d = to_float(c, v)?;
    Ok(if d.is_nan() { f32::NAN } else if d < -(f32::MAX as f64) { f32::NEG_INFINITY } else if d <= f32::MAX as f64 { d as f32 } else { f32::INFINITY })
}

fn uv_to_utf8(c: &mut Ctx, uv: u64, out: &mut Vec<u8>) -> Result<(), Esc> {
    let n = match uv { 0..=0x7f => { out.push(uv as u8); return Ok(()); } 0x80..=0x7ff => 2, 0x800..=0xffff => 3, 0x10000..=0x1fffff => 4,
        0x200000..=0x3ffffff => 5, 0x4000000..=0x7fffffff => 6, _ => return err(c, c.vm().c_range_error.get(), "pack(U): value out of range") };
    let lead: u8 = [0, 0, 0xc0, 0xe0, 0xf0, 0xf8, 0xfc][n];
    out.push(lead | (uv >> (6 * (n - 1))) as u8);
    for k in (0..n - 1).rev() { out.push(0x80 | ((uv >> (6 * k)) & 0x3f) as u8); }
    Ok(())
}

const UU: &[u8] = b"`!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_";
const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encodes(out: &mut Vec<u8>, s: &[u8], ty: u8, tail_lf: bool) {
    let t = if ty == b'u' { UU } else { B64 };
    let pad = if ty == b'u' { b'`' } else { b'=' };
    if ty == b'u' { out.push(s.len() as u8 + b' '); }
    let mut ch = s.chunks_exact(3);
    for g in &mut ch {
        out.extend_from_slice(&[t[(g[0] >> 2) as usize], t[((g[0] << 4 & 0o60) | (g[1] >> 4 & 0o17)) as usize],
                                t[((g[1] << 2 & 0o74) | (g[2] >> 6 & 3)) as usize], t[(g[2] & 0o77) as usize]]);
    }
    match ch.remainder() {
        [a, b] => out.extend_from_slice(&[t[(a >> 2) as usize], t[((a << 4 & 0o60) | (b >> 4 & 0o17)) as usize], t[(b << 2 & 0o74) as usize], pad]),
        [a] => out.extend_from_slice(&[t[(a >> 2) as usize], t[(a << 4 & 0o60) as usize], pad, pad]),
        _ => {}
    }
    if tail_lf { out.push(b'\n'); }
}

fn qpencode(out: &mut Vec<u8>, s: &[u8], len: usize) {
    const HEX: &[u8] = b"0123456789ABCDEF";
    let (mut n, mut prev): (usize, i32) = (0, -1);
    for &x in s {
        if x > 126 || (x < 32 && x != b'\n' && x != b'\t') || x == b'=' {
            out.extend_from_slice(&[b'=', HEX[(x >> 4) as usize], HEX[(x & 15) as usize]]); n += 3; prev = -1;
        } else if x == b'\n' {
            if prev == b' ' as i32 || prev == b'\t' as i32 { out.extend_from_slice(b"=\n"); }
            out.push(x); n = 0; prev = x as i32;
        } else { out.push(x); n += 1; prev = x as i32; }
        if n > len { out.extend_from_slice(b"=\n"); n = 0; prev = b'\n' as i32; }
    }
    if n > 0 { out.extend_from_slice(b"=\n"); }
}

/// Split off a trailing keyword hash and read its only allowed key.
fn one_kw(c: &mut Ctx, a: &[Value], key: &str) -> Result<(usize, Option<Value>), Esc> {
    let Some(h) = kw_arg(c, a) else { return Ok((a.len(), None)); };
    let mut val = None; let mut unknown = vec![]; let mut i = 0;
    while let Some((k, v)) = c.hash_entry_at(h, i) {
        i += 1;
        if k == UNDEF { continue; }
        if k == sym_value(key) { val = Some(v); } else { unknown.push(inspect_str(c, k)?); }
    }
    if !unknown.is_empty() { return arg_err(c, format!("unknown keyword{}: {}", if unknown.len() > 1 { "s" } else { "" }, unknown.join(", "))); }
    Ok((a.len() - 1, val))
}

/// Room for `add` more bytes: ArgumentError past the long range, NoMemoryError when malloc would fail.
fn reserve_more(c: &mut Ctx, res: &mut Vec<u8>, add: usize) -> Result<(), Esc> {
    if res.len().saturating_add(add) >= i64::MAX as usize { return arg_err(c, "string size too big"); }
    if res.try_reserve(add).is_err() { return err(c, c.vm().c_no_memory_error.get(), "failed to allocate memory"); }
    Ok(())
}
/// unpack's trailing nils for a count beyond the data: the Array limits first (rb_ary_store).
fn nil_fill(c: &mut Ctx, ary: Value, avail: usize, extra: usize) -> Result<(), Esc> {
    let req = avail.saturating_add(extra);
    if req - 1 >= (i64::MAX / 8) as usize { return err(c, c.vm().c_index_error.get(), format!("index {} too big", req - 1)); }
    if req > 1 << 31 { return err(c, c.vm().c_no_memory_error.get(), "failed to allocate memory"); }
    for _ in 0..extra { c.ary_push(ary, NIL); }
    Ok(())
}

pub fn pack(c: &mut Ctx, ary: Value, a: &[Value]) -> RV {
    let (n, buffer) = one_kw(c, a, "buffer")?;
    if n != 1 { return argc_err(c, n, 1, 1); }
    let fv = expect_str(c, a[0])?;
    let fmt = c.str_bytes(fv).to_vec();
    let buffer = match buffer {
        None | Some(NIL) => None,
        Some(b) => {
            if !is_string(b) { return type_err(c, format!("buffer must be String, not {}", class_path(c, real_class_of(c, b)))); }
            check_frozen(c, b)?;
            Some(b)
        }
    };
    let mut res: Vec<u8> = match buffer { Some(b) => c.str_bytes(b).to_vec(), None => vec![] };
    let mut idx = 0usize;
    let mut enc_info = 1;   // 0 BINARY, 1 US-ASCII, 2 UTF-8
    let mut p = 0;
    let mut assoc = NIL;
    macro_rules! nextfrom { () => {{ if idx >= c.ary_len(ary) { return arg_err(c, TOO_FEW); } let v = c.ary_get(ary, idx); idx += 1; v }}; }
    while let Some(d) = next_dir(c, &fmt, &mut p)? {
        let ty = d.ty;
        let mut len = if d.star {
            if b"@Xxu".contains(&ty) { 0 } else if b"PMm".contains(&ty) { 1 } else { c.ary_len(ary).saturating_sub(idx) }
        } else { d.len.unwrap_or(1) };
        match ty { b'U' => { if enc_info == 1 { enc_info = 2; } } b'm' | b'M' | b'u' => {} _ => enc_info = 0 }
        if let Some((sz, _, be)) = int_kind(&d) {
            for _ in 0..len {
                let v = nextfrom!();
                let v = to_integer(c, v)?;
                let x = int_low64(c, v).to_le_bytes();
                if be { for k in (0..sz).rev() { res.push(x[k]); } } else { res.extend_from_slice(&x[..sz]); }
            }
            continue;
        }
        match ty {
            b'A' | b'a' | b'Z' | b'B' | b'b' | b'H' | b'h' => {
                let from = nextfrom!();
                let src: Vec<u8> = if from == NIL { vec![] } else { let s = expect_str(c, from)?; c.str_bytes(s).to_vec() };
                let plen = src.len();
                if d.star { len = plen; }
                let grow = match ty {
                    b'a' | b'A' | b'Z' => {
                        if plen >= len { res.extend_from_slice(&src[..len]); if d.star && ty == b'Z' { res.push(0); } }
                        else { reserve_more(c, &mut res, len)?; res.extend_from_slice(&src); res.resize(res.len() + len - plen, if ty == b'A' { b' ' } else { 0 }); }
                        0
                    }
                    b'b' | b'B' => {
                        let mut j = 0;
                        if len > plen { j = (len - plen + 1) / 2; len = plen; }
                        let mut byte: u32 = 0;
                        for i in 1..=len {
                            let bit = (src[i - 1] & 1) as u32;
                            if ty == b'b' { if bit != 0 { byte |= 128; } if i & 7 != 0 { byte >>= 1; } else { res.push(byte as u8); byte = 0; } }
                            else { byte |= bit; if i & 7 != 0 { byte <<= 1; } else { res.push(byte as u8); byte = 0; } }
                        }
                        if len & 7 != 0 { if ty == b'b' { byte >>= 7 - (len & 7); } else { byte <<= 7 - (len & 7); } res.push(byte as u8); }
                        j
                    }
                    _ => {
                        let mut j = 0;
                        if len > plen { j = (len + 1) / 2 - (plen + 1) / 2; len = plen; }
                        let mut byte: u32 = 0;
                        for i in 1..=len {
                            let ch = src[i - 1];
                            let nib = if ch.is_ascii_alphabetic() { ((ch & 15) + 9) & 15 } else { ch & 15 } as u32;
                            if ty == b'h' { byte |= nib << 4; if i & 1 != 0 { byte >>= 4; } else { res.push(byte as u8); byte = 0; } }
                            else { byte |= nib; if i & 1 != 0 { byte <<= 4; } else { res.push(byte as u8); byte = 0; } }
                        }
                        if len & 1 != 0 { res.push(byte as u8); }
                        j
                    }
                };
                res.resize(res.len() + grow, 0);
            }
            b'f' | b'F' | b'e' | b'g' => for _ in 0..len {
                let v = nextfrom!(); let f = to_float32(c, v)?;
                res.extend_from_slice(&if ty == b'g' { f.to_be_bytes() } else { f.to_le_bytes() });
            },
            b'd' | b'D' | b'E' | b'G' => for _ in 0..len {
                let v = nextfrom!(); let f = to_float(c, v)?;
                res.extend_from_slice(&if ty == b'G' { f.to_be_bytes() } else { f.to_le_bytes() });
            },
            b'x' => { reserve_more(c, &mut res, len)?; res.resize(res.len() + len, 0) }
            b'X' => { if res.len() < len { return arg_err(c, "X outside of string"); } res.truncate(res.len() - len); }
            b'@' => { if len >= res.len() { let add = len - res.len(); reserve_more(c, &mut res, add)?; res.resize(len, 0); } else { res.truncate(len); } }
            b'%' => return arg_err(c, "% is not supported"),
            b'U' => for _ in 0..len {
                let v = nextfrom!(); let v = to_integer(c, v)?;
                let l = num_to_long(c, v)?;
                if l < 0 { return err(c, c.vm().c_range_error.get(), "pack(U): value out of range"); }
                uv_to_utf8(c, l as u64, &mut res)?;
            },
            b'u' | b'm' => {
                let v = nextfrom!(); let s = expect_str(c, v)?; let src = c.str_bytes(s).to_vec();
                if len == 0 && ty == b'm' { encodes(&mut res, &src, ty, false); continue; }
                let len = if len <= 2 { 45 } else if len > 63 && ty == b'u' { 63 } else { len / 3 * 3 };
                for chunk in src.chunks(len) { encodes(&mut res, chunk, ty, true); }
            }
            b'M' => {
                let v = nextfrom!(); let s = crate::rt::srb_tostr(c, v)?; let src = c.str_bytes(s).to_vec();
                qpencode(&mut res, &src, if len <= 1 { 72 } else { len });
            }
            b'w' => for _ in 0..len {
                let v = nextfrom!(); let v = to_integer(c, v)?;
                let mut x = c.to_big(v);
                if x.cmp(&BigInt::zero()) == std::cmp::Ordering::Less { return arg_err(c, "can't compress negative numbers"); }
                let m = BigInt::from_u64(0x7f);
                let mut tmp = vec![];
                loop {
                    tmp.push(x.bitop(&m, b'&').to_i64().unwrap_or(0) as u8);
                    x = x.shr(7);
                    if x.is_zero() { break; }
                }
                for (k, b) in tmp.iter().enumerate().rev() { res.push(if k > 0 { b | 0x80 } else { *b }); }
            },
            b'P' | b'p' => {
                if ty == b'P' {
                    if idx >= c.ary_len(ary) { return arg_err(c, TOO_FEW); }
                    let from = c.ary_get(ary, idx);
                    if from != NIL {
                        let s = expect_str(c, from)?;
                        let n = c.str_len(s);
                        if n < len { return arg_err(c, format!("too short buffer for P({} for {})", n, len)); }
                    }
                    len = 1;
                }
                for _ in 0..len {
                    let from = nextfrom!();
                    let t = if from == NIL { 0 } else { expect_str(c, from)?.raw() as u64 };
                    if assoc == NIL { assoc = c.ary_new(); }
                    c.ary_push(assoc, from);
                    res.extend_from_slice(&t.to_le_bytes());
                }
            }
            _ => return unknown_directive(c, "pack", ty, &fmt),
        }
    }
    let out = match buffer {
        Some(b) => {
            let r = c.str_modify(b)?;
            r.b = crate::sbuf::SBuf::from_vec(res);
            match enc_info { 1 => r.enc = crate::object::ENC_ASCII, 2 => r.enc = crate::object::ENC_UTF8, _ => {} }
            b
        }
        None => c.str_from_vec_enc(res, match enc_info { 1 => crate::object::ENC_ASCII, 2 => crate::object::ENC_UTF8, _ => crate::object::ENC_BINARY }),
    };
    if assoc != NIL { ivar_set_raw(c, out, assoc_sym(), assoc); }
    Ok(out)
}

fn utf8_to_uv(c: &mut Ctx, s: &[u8]) -> Result<(u64, usize), Esc> {
    let x = s[0] as u64;
    if x & 0x80 == 0 { return Ok((x, 1)); }
    if x & 0x40 == 0 { return arg_err(c, "malformed UTF-8 character"); }
    let (n, mut uv) = if x & 0x20 == 0 { (2, x & 0x1f) } else if x & 0x10 == 0 { (3, x & 0x0f) } else if x & 0x08 == 0 { (4, x & 0x07) }
        else if x & 0x04 == 0 { (5, x & 0x03) } else if x & 0x02 == 0 { (6, x & 0x01) } else { return arg_err(c, "malformed UTF-8 character"); };
    if n > s.len() { return arg_err(c, format!("malformed UTF-8 character (expected {} bytes, given {} bytes)", n, s.len())); }
    for k in 1..n {
        if s[k] & 0xc0 != 0x80 { return arg_err(c, "malformed UTF-8 character"); }
        uv = uv << 6 | (s[k] & 0x3f) as u64;
    }
    const LIMITS: [u64; 7] = [0, 0x80, 0x800, 0x10000, 0x200000, 0x4000000, 0x80000000];
    if uv < LIMITS[n - 1] { return arg_err(c, "redundant UTF-8 sequence"); }
    Ok((uv, n))
}

#[derive(PartialEq, Clone, Copy)]
enum Mode { Array, Block, One }

/// String#unpack (with or without a block) and String#unpack1.
pub fn unpack(c: &mut Ctx, str: Value, a: &[Value], blk: Blk, one: bool) -> RV {
    let (n, off) = one_kw(c, a, "offset")?;
    if n != 1 { return argc_err(c, n, 1, 1); }
    let fv = expect_str(c, a[0])?;
    let fmt = c.str_bytes(fv).to_vec();
    let offset = match off { Some(v) => num_to_long(c, v)?, None => 0 };
    let b = c.str_bytes(str).to_vec();
    if offset < 0 { return arg_err(c, "offset can't be negative"); }
    if offset as usize > b.len() { return arg_err(c, "offset outside of string"); }
    let mode = if one { Mode::One } else if !blk.is_null() { Mode::Block } else { Mode::Array };
    let ary = if mode == Mode::Array { c.ary_new() } else { NIL };
    let send = b.len();
    let mut s = offset as usize;
    let mut p = 0;
    macro_rules! push { ($v:expr) => {{
        let v = $v;
        match mode { Mode::Block => { yield1(c, blk, v)?; } Mode::Array => c.ary_push(ary, v), Mode::One => return Ok(v) }
    }}; }
    while let Some(d) = next_dir(c, &fmt, &mut p)? {
        let ty = d.ty; let star = d.star;
        let mut len = if d.at_end { 1 } else if star { send - s } else { d.len.unwrap_or(if ty == b'@' { 0 } else { 1 }) };
        let fixed = |sz: usize, len: &mut usize, s: usize| -> usize {
            let avail = (send - s) / sz;
            let mut extra = 0;
            if *len > avail { if !star { extra = *len - avail; } *len = avail; }
            extra
        };
        if let Some((sz, signed, be)) = int_kind(&d) {
            let extra = fixed(sz, &mut len, s);
            for _ in 0..len {
                let mut x: u64 = 0;
                for k in 0..sz { let byte = if be { b[s + k] } else { b[s + sz - 1 - k] }; x = x << 8 | byte as u64; }
                s += sz;
                let v = if signed { let sh = 64 - sz * 8; c.int_from_i64(((x << sh) as i64) >> sh) } else { c.int_from_i128(x as i128) };
                push!(v);
            }
            if mode == Mode::Array && extra > 0 { nil_fill(c, ary, len, extra)?; }
            continue;
        }
        match ty {
            b'%' => return arg_err(c, "% is not supported"),
            b'A' => {
                if len > send - s { len = send - s; }
                let mut e = len;
                while e > 0 && (b[s + e - 1] == b' ' || b[s + e - 1] == 0) { e -= 1; }
                let v = c.str_from_vec_enc(b[s..s + e].to_vec(), crate::object::ENC_BINARY); push!(v);
                s += len;
            }
            b'Z' => {
                if len > send - s { len = send - s; }
                let mut t = s;
                while t < s + len && b[t] != 0 { t += 1; }
                let v = c.str_from_vec_enc(b[s..t].to_vec(), crate::object::ENC_BINARY); push!(v);
                if t < send { t += 1; }
                s = if star { t } else { s + len };
            }
            b'a' => {
                if len > send - s { len = send - s; }
                let v = c.str_from_vec_enc(b[s..s + len].to_vec(), crate::object::ENC_BINARY); push!(v);
                s += len;
            }
            b'b' | b'B' | b'h' | b'H' => {
                let per = if ty == b'b' || ty == b'B' { 8 } else { 2 };
                if star || len > (send - s) * per { len = (send - s) * per; }
                let mut out = Vec::with_capacity(len);
                let mut bits: u32 = 0;
                for i in 0..len {
                    match ty {
                        b'b' => { if i & 7 != 0 { bits >>= 1; } else { bits = b[s] as u32; s += 1; } out.push(if bits & 1 != 0 { b'1' } else { b'0' }); }
                        b'B' => { if i & 7 != 0 { bits <<= 1; } else { bits = b[s] as u32; s += 1; } out.push(if bits & 128 != 0 { b'1' } else { b'0' }); }
                        b'h' => { if i & 1 != 0 { bits >>= 4; } else { bits = b[s] as u32; s += 1; } out.push(b"0123456789abcdef"[(bits & 15) as usize]); }
                        _ => { if i & 1 != 0 { bits <<= 4; } else { bits = b[s] as u32; s += 1; } out.push(b"0123456789abcdef"[((bits >> 4) & 15) as usize]); }
                    }
                }
                let v = c.str_from_vec_enc(out, crate::object::ENC_ASCII); push!(v);
            }
            b'f' | b'F' | b'e' | b'g' | b'd' | b'D' | b'E' | b'G' => {
                let sz = if b"fFeg".contains(&ty) { 4 } else { 8 };
                let extra = fixed(sz, &mut len, s);
                for _ in 0..len {
                    let sl = &b[s..s + sz]; s += sz;
                    let f = match ty {
                        b'g' => f32::from_be_bytes(sl.try_into().unwrap()) as f64,
                        b'G' => f64::from_be_bytes(sl.try_into().unwrap()),
                        b'f' | b'F' | b'e' => f32::from_le_bytes(sl.try_into().unwrap()) as f64,
                        _ => f64::from_le_bytes(sl.try_into().unwrap()),
                    };
                    let v = c.float_new(f); push!(v);
                }
                if mode == Mode::Array && extra > 0 { nil_fill(c, ary, len, extra)?; }
            }
            b'U' => {
                if len > send - s { len = send - s; }
                while len > 0 && s < send {
                    let (uv, l) = utf8_to_uv(c, &b[s..])?;
                    s += l; len -= 1;
                    let v = c.int_from_i128(uv as i128); push!(v);
                }
            }
            b'u' => {
                let cap = (send - s) * 3 / 4;
                let mut out: Vec<u8> = Vec::with_capacity(cap);
                let mut total = 0usize;
                let ok = |x: u8| (b' '..b'a').contains(&x);
                while s < send && b[s] > b' ' && b[s] < b'a' {
                    let mut l = ((b[s] - b' ') & 0o77) as usize; s += 1;
                    total += l;
                    if total > cap { l -= total - cap; total = cap; }
                    while l > 0 {
                        let mlen = l.min(3);
                        let mut q = [0u8; 4];
                        for qq in q.iter_mut() { if s < send && ok(b[s]) { *qq = (b[s] - b' ') & 0o77; s += 1; } }
                        let hunk = [q[0] << 2 | q[1] >> 4, q[1] << 4 | q[2] >> 2, q[2] << 6 | q[3]];
                        out.extend_from_slice(&hunk[..mlen]);
                        l -= mlen;
                    }
                    if s < send && b[s] != b'\r' && b[s] != b'\n' { s += 1; }
                    if s < send && b[s] == b'\r' { s += 1; }
                    if s < send && b[s] == b'\n' { s += 1; }
                }
                out.truncate(total);
                let v = c.str_from_vec_enc(out, crate::object::ENC_BINARY); push!(v);
            }
            b'm' => {
                let mut xt = [-1i32; 256];
                for (i, &ch) in B64.iter().enumerate() { xt[ch as usize] = i as i32; }
                let at = |s: usize| -> u8 { if s < send { b[s] } else { 0 } };
                let mut out: Vec<u8> = Vec::with_capacity((send - s + 3) * 3 / 4);
                let (mut a1, mut b1, mut c1, mut d1): (i32, i32, i32, i32) = (-1, -1, 0, 0);
                if len == 0 {
                    let bad = |c: &mut Ctx| -> RV { arg_err(c, "invalid base64") };
                    while s < send {
                        a1 = xt[b[s] as usize]; s += 1;
                        if s >= send || a1 == -1 { return bad(c); }
                        b1 = xt[b[s] as usize]; s += 1;
                        if s >= send || b1 == -1 { return bad(c); }
                        c1 = -1; d1 = -1;
                        if b[s] == b'=' { if s + 2 == send && b[s + 1] == b'=' { break; } return bad(c); }
                        c1 = xt[b[s] as usize]; s += 1;
                        if s >= send || c1 == -1 { return bad(c); }
                        if s + 1 == send && b[s] == b'=' { break; }
                        d1 = xt[b[s] as usize]; s += 1;
                        if d1 == -1 { return bad(c); }
                        out.extend_from_slice(&[(a1 << 2 | b1 >> 4) as u8, (b1 << 4 | c1 >> 2) as u8, (c1 << 6 | d1) as u8]);
                    }
                    if c1 == -1 { out.push((a1 << 2 | b1 >> 4) as u8); if b1 & 0xf != 0 { return bad(c); } }
                    else if d1 == -1 { out.extend_from_slice(&[(a1 << 2 | b1 >> 4) as u8, (b1 << 4 | c1 >> 2) as u8]); if c1 & 3 != 0 { return bad(c); } }
                } else {
                    while s < send {
                        a1 = -1; b1 = -1; c1 = -1; d1 = -1;
                        while s < send { a1 = xt[at(s) as usize]; if a1 != -1 { break; } s += 1; }
                        if s >= send { break; }
                        s += 1;
                        while s < send { b1 = xt[at(s) as usize]; if b1 != -1 { break; } s += 1; }
                        if s >= send { b1 = -1; break; }
                        s += 1;
                        while s < send { c1 = xt[at(s) as usize]; if c1 != -1 || at(s) == b'=' { break; } s += 1; }
                        if s >= send || at(s) == b'=' { if s >= send || c1 == -1 { c1 = -1; } break; }
                        s += 1;
                        while s < send { d1 = xt[at(s) as usize]; if d1 != -1 || at(s) == b'=' { break; } s += 1; }
                        if s >= send || at(s) == b'=' { break; }
                        s += 1;
                        out.extend_from_slice(&[(a1 << 2 | b1 >> 4) as u8, (b1 << 4 | c1 >> 2) as u8, (c1 << 6 | d1) as u8]);
                        a1 = -1;
                    }
                    if a1 != -1 && b1 != -1 {
                        out.push((a1 << 2 | b1 >> 4) as u8);
                        if c1 != -1 { out.push((b1 << 4 | c1 >> 2) as u8); }
                    }
                }
                let v = c.str_from_vec_enc(out, crate::object::ENC_BINARY); push!(v);
            }
            b'M' => {
                let hex = |x: u8| (x as char).to_digit(16).map(|d| d as u8);
                let mut out: Vec<u8> = Vec::with_capacity(send - s);
                let mut ss = s;
                while s < send {
                    if b[s] == b'=' {
                        s += 1; if s == send { break; }
                        if s + 1 < send && b[s] == b'\r' && b[s + 1] == b'\n' { s += 1; }
                        if b[s] != b'\n' {
                            let Some(h1) = hex(b[s]) else { break; };
                            s += 1; if s == send { break; }
                            let Some(h2) = hex(b[s]) else { break; };
                            out.push(h1 << 4 | h2);
                        }
                    } else { out.push(b[s]); }
                    s += 1; ss = s;
                }
                out.extend_from_slice(&b[ss..send]);
                let v = c.str_from_vec_enc(out, crate::object::ENC_BINARY); push!(v);
            }
            b'@' => { if len > send { return arg_err(c, "@ outside of string"); } s = len; }
            b'X' => { if len > s { return arg_err(c, "X outside of string"); } s -= len; }
            b'x' => { if len > send - s { return arg_err(c, "x outside of string"); } s += len; }
            b'w' => {
                let mut s0 = s;
                while len > 0 && s < send {
                    s += 1;
                    if b[s - 1] & 0x80 == 0 {
                        let mut x = BigInt::zero();
                        for &byte in &b[s0..s] { x = x.shl(7).add(&BigInt::from_u64((byte & 0x7f) as u64)); }
                        let v = c.big_to_value(x); push!(v);
                        len -= 1; s0 = s;
                    }
                }
            }
            b'P' => if send - s >= 8 {
                let t = u64::from_le_bytes(b[s..s + 8].try_into().unwrap()); s += 8;
                let v = if t == 0 { NIL } else {
                    let assoc = ivar_get(c, str, assoc_sym());
                    let o = associated_pointer(c, assoc, t)?;
                    if len < c.str_len(o) { let n = c.str_from_vec_enc(c.str_bytes(o)[..len].to_vec(), crate::object::ENC_BINARY); ivar_set_raw(c, n, assoc_sym(), assoc); n } else { o }
                };
                push!(v);
            },
            b'p' => {
                if len > (send - s) / 8 { len = (send - s) / 8; }
                for _ in 0..len {
                    let t = u64::from_le_bytes(b[s..s + 8].try_into().unwrap()); s += 8;
                    let v = if t == 0 { NIL } else { let assoc = ivar_get(c, str, assoc_sym()); associated_pointer(c, assoc, t)? };
                    push!(v);
                }
            }
            _ => return unknown_directive(c, "unpack", ty, &fmt),
        }
    }
    Ok(ary)
}
