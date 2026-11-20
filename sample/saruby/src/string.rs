// String: byte buffer + encoding tag (UTF-8 / ASCII-8BIT / US-ASCII).
// Core API on the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
// Methods are in builtins/string_methods.rs.
#![forbid(unsafe_code)]
// the context-free functions, for the modules not converted yet
pub use crate::str_legacy::*;
pub use imp::*;

// a module of its own: the bans must not meet the re-export above
mod imp {
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{obj_flags, obj_klass, obj_set_flags, obj_set_klass};
use crate::object::*;
use crate::sbuf::SBuf;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

impl Ctx {
    // bytes kept inside the slot are not malloc'ed: only longer strings count toward the GC trigger
    #[inline(always)]
    fn str_account(&mut self, n: usize) { if n > crate::sbuf::INLINE { self.malloc_increase(n); } }
    /// The buffer becomes the String's payload (class String).
    #[inline]
    pub fn str_from_sbuf(&mut self, b: SBuf, enc: u8) -> Value {
        self.str_account(b.len());
        let k = self.vm().c_string;
        self.alloc(k, Body::String(RString { b, enc }))
    }
    #[inline]
    pub fn str_new(&mut self, b: &[u8]) -> Value { self.str_from_sbuf(SBuf::from_slice(b), ENC_UTF8) }
    #[inline]
    pub fn str_new_enc(&mut self, b: &[u8], enc: u8) -> Value { self.str_from_sbuf(SBuf::from_slice(b), enc) }
    #[inline]
    pub fn str_with_capa(&mut self, n: usize, enc: u8) -> Value {
        self.str_account(n);
        let k = self.vm().c_string;
        self.alloc(k, Body::String(RString { b: SBuf::with_capacity(n), enc }))
    }
    pub fn str_from_vec(&mut self, b: Vec<u8>) -> Value { self.str_from_sbuf(SBuf::from_vec(b), ENC_UTF8) }
    pub fn str_from_vec_enc(&mut self, b: Vec<u8>, enc: u8) -> Value { self.str_from_sbuf(SBuf::from_vec(b), enc) }
    pub fn str_new_frozen(&mut self, b: &[u8]) -> Value {
        let v = self.str_new(b);
        obj_set_flags(v, FL_FROZEN);
        v
    }
    pub fn str_from(&mut self, s: &str) -> Value { self.str_new(s.as_bytes()) }
    /// Encoding of `like`, new contents (class String).
    pub fn str_new_like(&mut self, like: Value, b: Vec<u8>) -> Value {
        let enc = self.str_enc(like);
        self.str_from_vec_enc(b, enc)
    }
    /// A String with the bytes and the encoding of `v` (class String; short ones stay inline).
    #[inline]
    pub fn str_copy(&mut self, v: Value) -> Value {
        let r = self.rstring(v);
        let (b, enc) = (SBuf::from_slice(&r.b), r.enc);
        self.str_from_sbuf(b, enc)
    }
    /// A String with the bytes of `v` and encoding `enc`.
    #[inline]
    pub fn str_copy_enc(&mut self, v: Value, enc: u8) -> Value { self.str_from_sbuf(SBuf::from_slice(self.str_bytes(v)), enc) }
    /// Bytes `r` of `v` as a String of v's encoding.
    #[inline]
    pub fn str_sub(&mut self, v: Value, r: std::ops::Range<usize>) -> Value {
        let x = self.rstring(v);
        let (b, enc) = (SBuf::from_slice(&x.b[r]), x.enc);
        self.str_from_sbuf(b, enc)
    }

    #[inline]
    pub fn str_enc(&self, v: Value) -> u8 { self.rstring(v).enc }
    #[inline]
    pub fn str_len(&self, v: Value) -> usize { self.rstring(v).b.len() }
    pub fn str_eq(&self, a: Value, b: Value) -> bool { self.rstring(a).b == self.rstring(b).b }
    pub fn str_hash(&self, v: Value) -> u64 { crate::astrust::hash_bytes(&self.rstring(v).b) }
    pub fn str_to_sym(&self, v: Value) -> Value { Value::sym(intern(&self.rstring(v).b)) }
    pub fn str_cmp(&self, a: Value, b: Value) -> std::cmp::Ordering { self.rstring(a).b.cmp(&self.rstring(b).b) }

    /// String `v` for writing; FrozenError when frozen.
    #[inline]
    pub fn str_modify(&mut self, v: Value) -> Result<&mut RString, Esc> {
        if obj_flags(v) & FL_FROZEN != 0 { return Err(raise_frozen(self, v)); }
        Ok(self.rstring_mut(v))
    }
    /// acc << s with the result encoding already decided; they may be the same String.
    #[inline]
    pub fn str_append_enc(&mut self, acc: Value, s: Value, enc: u8) {
        let (a, others) = self.str_mut_and(acc);
        let n = match others.str_bytes(s) {
            Some(b) => { a.enc = enc; a.b.extend_from_slice(b); b.len() }
            None => { let b = a.b.to_vec(); a.b.extend_from_slice(&b); 0 }
        };
        if n != 0 { self.malloc_increase(n); }
    }
    /// acc << s for a mutable acc of the same encoding (nothing to check).
    #[inline]
    pub fn str_append_same_enc(&mut self, acc: Value, s: Value) { let enc = self.str_enc(acc); self.str_append_enc(acc, s, enc); }

    /// Char boundaries (byte offsets) for a string, honoring its encoding.
    pub fn char_offsets(&self, v: Value) -> Vec<usize> { let r = self.rstring(v); char_offsets_of(&r.b, r.enc) }
    pub fn str_char_len(&self, v: Value) -> usize { let r = self.rstring(v); enc_strlen(&r.b, r.enc) }
    /// Code of the character at b[s..e] of string v (Unicode scalar for Unicode encodings).
    pub fn str_char_code(&self, v: Value, s: usize, e: usize) -> u32 { let r = self.rstring(v); crate::builtins::encoding::char_code(r.enc, &r.b[s..e]) }
    pub fn chars_of(&self, v: Value) -> Vec<Vec<u8>> {
        let r = self.rstring(v);
        let off = char_offsets_of(&r.b, r.enc);
        (0..off.len() - 1).map(|i| r.b[off[i]..off[i + 1]].to_vec()).collect()
    }
    /// A copy with the class of `v`.
    pub fn str_dup(&mut self, v: Value) -> Value {
        let r = self.rstring(v);
        let (b, enc) = (r.b.to_vec(), r.enc);
        let n = self.str_from_vec_enc(b, enc);
        obj_set_klass(n, obj_klass(v));
        n
    }
    pub fn str_concat_values(&mut self, parts: &[Value]) -> Value {
        let mut out = vec![];
        for p in parts { out.extend_from_slice(self.str_bytes(*p)); }
        self.str_from_vec(out)
    }
    pub fn ary_of_strs(&mut self, items: Vec<Vec<u8>>, enc: u8) -> Value {
        let a = self.ary_new_capa(items.len());
        for it in items { let s = self.str_from_vec_enc(it, enc); self.ary_push(a, s); }
        a
    }
}

pub fn str_append_str(c: &mut Ctx, acc: Value, s: Value) -> Result<(), Esc> {
    check_frozen(c, acc)?;
    let ea = c.str_enc(acc);
    let enc = if acc == s || ea == c.str_enc(s) { ea } else { crate::builtins::encoding::check_compatible(c, acc, s)? };
    c.str_append_enc(acc, s, enc);
    Ok(())
}

#[inline]
pub fn is_ascii(b: &[u8]) -> bool { b.is_ascii() }

pub fn utf8_len(b: &[u8]) -> usize {
    if is_ascii(b) { return b.len(); }
    let mut n = 0; let mut i = 0;
    while i < b.len() { i += char_len_at(b, i); n += 1; }
    n
}
/// UTF-8 character length at b[i] (1 for an invalid byte).
#[inline]
pub fn char_len_at(b: &[u8], i: usize) -> usize {
    if b[i] < 0x80 { return 1; }
    crate::builtins::encoding::utf8_precise(&b[i..]).unwrap_or(1)
}
/// Character length at b[i] in encoding `enc`.
#[inline]
pub fn enc_char_len(b: &[u8], i: usize, enc: u8) -> usize { crate::builtins::encoding::mbclen(enc, b, i) }
/// Every byte is one character (single-byte encoding, or ASCII-only in an ASCII-compatible one).
#[inline]
pub fn enc_single(b: &[u8], enc: u8) -> bool {
    crate::builtins::encoding::is_single_byte(enc) || (crate::builtins::encoding::ascii_compat(enc) && is_ascii(b))
}
pub fn char_offsets_of(b: &[u8], enc: u8) -> Vec<usize> {
    if enc_single(b, enc) { return (0..=b.len()).collect(); }
    let mut out = Vec::with_capacity(b.len() + 1);
    let mut i = 0;
    while i < b.len() { out.push(i); i += enc_char_len(b, i, enc); }
    out.push(b.len());
    out
}
pub fn enc_strlen(b: &[u8], enc: u8) -> usize {
    if enc_single(b, enc) { return b.len(); }
    let mut n = 0; let mut i = 0;
    while i < b.len() { i += enc_char_len(b, i, enc); n += 1; }
    n
}

pub fn decode_utf8(b: &[u8], i: usize) -> (u32, usize) {
    let l = char_len_at(b, i);
    let x = b[i] as u32;
    let cp = match l {
        1 => x,
        2 => ((x & 0x1f) << 6) | (b[i + 1] as u32 & 0x3f),
        3 => ((x & 0x0f) << 12) | ((b[i + 1] as u32 & 0x3f) << 6) | (b[i + 2] as u32 & 0x3f),
        _ => ((x & 0x07) << 18) | ((b[i + 1] as u32 & 0x3f) << 12) | ((b[i + 2] as u32 & 0x3f) << 6) | (b[i + 3] as u32 & 0x3f),
    };
    (cp, l)
}
pub fn encode_utf8(cp: u32, out: &mut Vec<u8>) {
    if let Some(ch) = char::from_u32(cp) { let mut buf = [0u8; 4]; out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes()); }
    else {
        // surrogates etc: raw encoding
        out.push((0xe0 | (cp >> 12)) as u8); out.push((0x80 | ((cp >> 6) & 0x3f)) as u8); out.push((0x80 | (cp & 0x3f)) as u8);
    }
}

pub fn valid_utf8(b: &[u8]) -> bool { std::str::from_utf8(b).is_ok() }

/// String#inspect body.
pub fn inspect_bytes(b: &[u8], enc: u8) -> String {
    use crate::builtins::encoding as en;
    let mut out = String::with_capacity(b.len() + 2);
    out.push('"');
    let uni = en::is_unicode(enc) && !en::is_dummy(enc);
    let ac = en::ascii_compat(enc);
    // result encoding is UTF-8: only UTF-8 strings may print non-ASCII characters raw
    let same = en::kind(enc) == en::EK::Utf8;
    let code_at = |i: usize| -> Option<u32> { en::precise_len(enc, b, i).map(|l| en::char_code(enc, &b[i..i + l])) };
    let mut i = 0;
    while i < b.len() {
        let Some(l) = en::precise_len(enc, b, i) else {
            let n = en::mbclen(enc, b, i);
            for k in 0..n { out.push_str(&format!("\\x{:02X}", b[i + k])); }
            i += n; continue;
        };
        let cp = en::char_code(enc, &b[i..i + l]);
        let is_asc = (ac && l == 1 && cp < 0x80) || (uni && cp < 0x80);
        if is_asc {
            match cp as u8 {
                b'"' => out.push_str("\\\""),
                b'\\' => out.push_str("\\\\"),
                b'\n' => out.push_str("\\n"),
                b'\t' => out.push_str("\\t"),
                b'\r' => out.push_str("\\r"),
                0x0c => out.push_str("\\f"),
                0x0b => out.push_str("\\v"),
                0x08 => out.push_str("\\b"),
                0x07 => out.push_str("\\a"),
                0x1b => out.push_str("\\e"),
                b'#' => {
                    let nx = if i + l < b.len() { code_at(i + l) } else { None };
                    if matches!(nx, Some(0x7b) | Some(0x24) | Some(0x40)) { out.push_str("\\#"); } else { out.push('#'); }
                }
                x @ 0x20..=0x7e => out.push(x as char),
                x => if uni { out.push_str(&format!("\\u{:04X}", x)) } else { out.push_str(&format!("\\x{:02X}", x)) },
            }
        } else if uni {
            if same && uni_printable(cp) { out.push(char::from_u32(cp).unwrap_or('?')); }
            else if cp > 0xffff { out.push_str(&format!("\\u{{{:X}}}", cp)); }
            else { out.push_str(&format!("\\u{:04X}", cp)); }
        } else if l == 1 {
            out.push_str(&format!("\\x{:02X}", b[i]));
        } else {
            out.push_str(&format!("\\x{{{:X}}}", cp));
        }
        i += l;
    }
    out.push('"');
    out
}

/// Is Unicode scalar `cp` printable (Onigmo print class; approximates "not Cc/Cn/Zl/Zp/Cs")?
pub fn uni_printable(cp: u32) -> bool {
    match cp {
        0..=0x1f | 0x7f..=0x9f | 0x2028 | 0x2029 | 0xd800..=0xdfff | 0xfdd0..=0xfdef => false,
        _ if cp & 0xfffe == 0xfffe => false,
        0x0378 | 0x0379 | 0x0380..=0x0383 | 0x038b | 0x038d | 0x03a2 => false,
        0x323b0..=0xdffff => false,
        0xe0000 | 0xe0002..=0xe001f | 0xe0080..=0xe00ff | 0xe01f0..=0xeffff => false,
        _ => cp <= 0x10ffff,
    }
}

pub fn sym_inspect(name: &[u8]) -> String {
    if sym_is_simple(name) { format!(":{}", String::from_utf8_lossy(name)) } else { format!(":{}", inspect_bytes(name, ENC_UTF8)) }
}

pub fn sym_is_simple(n: &[u8]) -> bool {
    if n.is_empty() { return false; }
    let ops: &[&[u8]] = &[b"+", b"-", b"*", b"/", b"%", b"**", b"==", b"===", b"!=", b"<=>", b"<", b"<=", b">", b">=", b"<<", b">>",
        b"!", b"~", b"+@", b"-@", b"[]", b"[]=", b"=~", b"!~", b"&", b"|", b"^", b"`"];
    if ops.contains(&n) { return true; }
    let s = match std::str::from_utf8(n) { Ok(s) => s, Err(_) => return false };
    let body = if let Some(r) = s.strip_prefix("@@") { r } else if let Some(r) = s.strip_prefix('@') { r } else if let Some(r) = s.strip_prefix('$') {
        if r.len() == 1 && "~*$?!@/\\;,.=:<>\"&`'+0123456789_".contains(r) { return true; }
        if r.len() == 2 && r.starts_with('-') && r[1..].chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') { return true; }
        if !r.is_empty() && r.bytes().all(|x| x.is_ascii_digit()) && !r.starts_with('0') { return true; }
        r
    } else { s };
    if body.is_empty() { return false; }
    let mut chars = body.chars().peekable();
    let f = *chars.peek().unwrap();
    if !(f.is_alphabetic() || f == '_' || (f as u32) >= 0x80) { return false; }
    let b: Vec<char> = body.chars().collect();
    for (i, &ch) in b.iter().enumerate() {
        if ch.is_alphanumeric() || ch == '_' || (ch as u32) >= 0x80 { continue; }
        if i == b.len() - 1 && (ch == '?' || ch == '!' || ch == '=') && s.len() == body.len() { continue; }
        return false;
    }
    true
}

pub fn to_rstring(c: &mut Ctx, v: Value) -> Result<String, Esc> {
    let s = if is_string(v) { v } else { crate::rt::srb_tostr(c, v)? };
    Ok(String::from_utf8_lossy(c.str_bytes(s)).into_owned())
}

/// Implicit conversion to String (to_str).
pub fn expect_str(c: &mut Ctx, v: Value) -> RV {
    if is_string(v) { return Ok(v); }
    if v.is_heap() && respond_to(v, s::to_str, true) {
        let r = funcall(c, v, s::to_str, &[], std::ptr::null())?;
        if is_string(r) { return Ok(r); }
        let cls = class_path(real_class_of(v));
        return type_err(c, format!("can't convert {} to String ({}#to_str gives {})", cls, cls, class_path(real_class_of(r))));
    }
    type_err(c, format!("no implicit conversion of {} into String", crate::builtins::kernel::class_name_for_err(v)))
}

// ---- case helpers -----------------------------------------------------------------------

pub fn map_case(b: &[u8], enc: u8, f: impl Fn(char) -> String, ascii: impl Fn(u8) -> u8) -> Vec<u8> {
    if enc != ENC_UTF8 || is_ascii(b) { return b.iter().map(|&x| ascii(x)).collect(); }
    match std::str::from_utf8(b) {
        Ok(s) => { let mut out = String::new(); for ch in s.chars() { out.push_str(&f(ch)); } out.into_bytes() }
        Err(_) => b.iter().map(|&x| ascii(x)).collect(),
    }
}
pub fn upcase(b: &[u8], enc: u8) -> Vec<u8> { map_case(b, enc, |c| c.to_uppercase().collect(), |x| x.to_ascii_uppercase()) }
pub fn downcase(b: &[u8], enc: u8) -> Vec<u8> { map_case(b, enc, |c| c.to_lowercase().collect(), |x| x.to_ascii_lowercase()) }
pub fn swapcase(b: &[u8], enc: u8) -> Vec<u8> {
    map_case(b, enc, |c| if c.is_uppercase() { c.to_lowercase().collect() } else { c.to_uppercase().collect() },
             |x| if x.is_ascii_uppercase() { x.to_ascii_lowercase() } else { x.to_ascii_uppercase() })
}
/// Unicode simple+full case folding approximation (for casecmp?).
pub fn fold_case(b: &[u8], enc: u8) -> Vec<u8> {
    if enc != ENC_UTF8 || is_ascii(b) { return b.iter().map(|x| x.to_ascii_lowercase()).collect(); }
    match std::str::from_utf8(b) {
        Ok(s) => {
            let mut out = String::new();
            for ch in s.chars() {
                match ch { 'ß' | 'ẞ' => out.push_str("ss"), 'ς' => out.push('σ'), _ => out.extend(ch.to_lowercase()) }
            }
            out.into_bytes()
        }
        Err(_) => b.iter().map(|x| x.to_ascii_lowercase()).collect(),
    }
}
pub fn capitalize(b: &[u8], enc: u8) -> Vec<u8> {
    if b.is_empty() { return vec![]; }
    let l = if enc == ENC_UTF8 { char_len_at(b, 0) } else { 1 };
    let mut out = upcase(&b[..l], enc);
    out.extend(downcase(&b[l..], enc));
    out
}

#[derive(PartialEq, Clone, Copy)]
enum Nb { NotChar, Found, Wrapped }

fn precise_full(enc: u8, p: &[u8]) -> Option<usize> { crate::builtins::encoding::precise_len(enc, p, 0) }

/// enc_succ_char: next character of the same byte length (CRuby string.c).
fn enc_succ_char(p: &mut [u8], enc: u8) -> Nb {
    use crate::builtins::encoding as en;
    let len = p.len();
    if en::min_len(enc) > 1 {
        if precise_full(enc, p) != Some(len) { return Nb::NotChar; }
        let c = en::char_code(enc, p) + 1;
        let Some(bs) = en::code_to_bytes(enc, c) else { return Nb::NotChar };
        if bs.len() != len { return Nb::Wrapped; }
        p.copy_from_slice(&bs);
        return Nb::Found;
    }
    loop {
        let mut i = len as isize - 1;
        while i >= 0 && p[i as usize] == 0xff { p[i as usize] = 0; i -= 1; }
        if i < 0 { return Nb::Wrapped; }
        p[i as usize] += 1;
        let l = precise_full(enc, p);
        if let Some(l) = l { if l == len { return Nb::Found; } for x in &mut p[l..] { *x = 0xff; } }
        if l.is_none() && !en::needmore(enc, p) && (i as usize) < len - 1 {
            let mut len2 = len - 1;
            while len2 > 0 { if precise_full(enc, &p[..len2]).is_some() || en::needmore(enc, &p[..len2]) { break; } len2 -= 1; }
            for x in &mut p[len2 + 1..] { *x = 0xff; }
        }
    }
}
fn enc_pred_char(p: &mut [u8], enc: u8) -> Nb {
    use crate::builtins::encoding as en;
    let len = p.len();
    if en::min_len(enc) > 1 {
        if precise_full(enc, p) != Some(len) { return Nb::NotChar; }
        let c = en::char_code(enc, p);
        if c == 0 { return Nb::NotChar; }
        let Some(bs) = en::code_to_bytes(enc, c - 1) else { return Nb::NotChar };
        if bs.len() != len { return Nb::Wrapped; }
        p.copy_from_slice(&bs);
        return Nb::Found;
    }
    loop {
        let mut i = len as isize - 1;
        while i >= 0 && p[i as usize] == 0 { p[i as usize] = 0xff; i -= 1; }
        if i < 0 { return Nb::Wrapped; }
        p[i as usize] -= 1;
        let l = precise_full(enc, p);
        if let Some(l) = l { if l == len { return Nb::Found; } for x in &mut p[l..] { *x = 0; } }
        if l.is_none() && !en::needmore(enc, p) && (i as usize) < len - 1 {
            let mut len2 = len - 1;
            while len2 > 0 { if precise_full(enc, &p[..len2]).is_some() || en::needmore(enc, &p[..len2]) { break; } len2 -= 1; }
            for x in &mut p[len2 + 1..] { *x = 0; }
        }
    }
}
/// ctype of a character for succ: 1 = digit, 2 = alpha, 0 = neither.
fn succ_ctype(enc: u8, p: &[u8]) -> u8 {
    use crate::builtins::encoding as en;
    let u = if en::is_unicode(enc) { Some(en::char_code(enc, p)) } else if p.len() == 1 && p[0] < 0x80 { Some(p[0] as u32) } else if en::is_single_byte(enc) { en::to_unicode(enc, p) } else { None };
    let Some(u) = u else { return 0 };
    let Some(ch) = char::from_u32(u) else { return 0 };
    let digit = ch.is_ascii_digit() || matches!(u, 0x660..=0x669 | 0x6f0..=0x6f9 | 0x966..=0x96f | 0xff10..=0xff19);
    if digit { 1 } else if ch.is_alphabetic() { 2 } else { 0 }
}
fn enc_succ_alnum_char(p: &mut [u8], enc: u8, carry: &mut Vec<u8>) -> Nb {
    let ct = succ_ctype(enc, p);
    if ct == 0 { return Nb::NotChar; }
    let save = p.to_vec();
    for _ in 0..=1 {
        if enc_succ_char(p, enc) == Nb::Found && succ_ctype(enc, p) == ct { return Nb::Found; }
    }
    p.copy_from_slice(&save);
    let mut range = 1;
    loop {
        let sv = p.to_vec();
        if enc_pred_char(p, enc) == Nb::Found {
            if succ_ctype(enc, p) != ct { p.copy_from_slice(&sv); break; }
        } else { p.copy_from_slice(&sv); break; }
        range += 1;
    }
    if range == 1 { return Nb::NotChar; }
    *carry = p.to_vec();
    if ct == 1 { enc_succ_char(carry, enc); }
    Nb::Wrapped
}

/// String#succ honouring the string's encoding (CRuby str_succ).
pub fn succ_enc(src: &[u8], enc: u8) -> Vec<u8> {
    use crate::builtins::encoding as en;
    if src.is_empty() { return vec![]; }
    if en::ascii_compat(enc) && is_ascii(src) { return succ_bytes(src); }
    let mut s = src.to_vec();
    let off = char_offsets_of(src, enc);
    let mut carry: Vec<u8> = vec![1];
    let mut carry_pos = 0usize;
    let mut found_alnum = false;
    let mut neighbor = Nb::Found;
    let mut last_alnum: Option<usize> = None;
    for k in (0..off.len() - 1).rev() {
        let (st, e) = (off[k], off[k + 1]);
        if neighbor == Nb::NotChar {
            if let Some(la) = last_alnum {
                let (a, x) = (s[la], s[st]);
                if if a.is_ascii_alphabetic() { x.is_ascii_digit() } else if a.is_ascii_digit() { x.is_ascii_alphabetic() } else { false } { break; }
            }
        }
        if precise_full(enc, &s[st..]).map(|l| l == e - st) != Some(true) { continue; }
        neighbor = enc_succ_alnum_char(&mut s[st..e], enc, &mut carry);
        match neighbor {
            Nb::NotChar => continue,
            Nb::Found => return s,
            Nb::Wrapped => last_alnum = Some(st),
        }
        found_alnum = true;
        carry_pos = st;
    }
    if !found_alnum {
        for k in (0..off.len() - 1).rev() {
            let (st, e) = (off[k], off[k + 1]);
            if precise_full(enc, &s[st..]).map(|l| l == e - st) != Some(true) { continue; }
            let mut tmp = s[st..e].to_vec();
            match enc_succ_char(&mut tmp, enc) {
                Nb::Found => { s[st..e].copy_from_slice(&tmp); return s; }
                Nb::Wrapped => { s[st..e].copy_from_slice(&tmp); }
                Nb::NotChar => {}
            }
            if precise_full(enc, &s[st..e]) != Some(e - st) { enc_succ_char(&mut s[st..e], enc); }
            if !en::ascii_compat(enc) { carry = s[st..e].to_vec(); }
            carry_pos = st;
        }
    }
    s.splice(carry_pos..carry_pos, carry);
    s
}

/// Ruby's String#succ.
pub fn succ_bytes(b: &[u8]) -> Vec<u8> {
    if b.is_empty() { return vec![]; }
    let mut s = b.to_vec();
    let alnum = s.iter().any(|x| x.is_ascii_alphanumeric());
    let mut i = s.len() as i64 - 1;
    if !alnum {
        // increment last byte with carry
        loop {
            if i < 0 { s.insert(0, 1); break; }
            let x = s[i as usize];
            if x == 0xff { s[i as usize] = 0; i -= 1; } else { s[i as usize] = x + 1; break; }
        }
        return s;
    }
    // skip trailing non-alnum
    while i >= 0 && !s[i as usize].is_ascii_alphanumeric() { i -= 1; }
    loop {
        let x = s[i as usize];
        let (nx, carry, ins) = match x {
            b'z' => (b'a', true, b'a'),
            b'Z' => (b'A', true, b'A'),
            b'9' => (b'0', true, b'1'),
            _ => (x + 1, false, 0),
        };
        s[i as usize] = nx;
        if !carry { break; }
        // find next alnum to the left
        let mut j = i - 1;
        while j >= 0 && !s[j as usize].is_ascii_alphanumeric() { j -= 1; }
        if j < 0 { s.insert(i as usize, ins); break; }
        i = j;
    }
    s
}

pub fn str_to_inum(b: &[u8], base: u32, strict: bool) -> Option<Value> {
    let s = String::from_utf8_lossy(b);
    let t = if strict { s.trim().to_string() } else { s.trim_start().to_string() };
    let (neg, mut body) = if let Some(r) = t.strip_prefix('-') { (true, r.to_string()) } else if let Some(r) = t.strip_prefix('+') { (false, r.to_string()) } else { (false, t.clone()) };
    let mut base = base;
    let lower = body.to_ascii_lowercase();
    if (base == 16 || base == 0) && lower.starts_with("0x") { body = body[2..].to_string(); base = 16; }
    else if (base == 2 || base == 0) && lower.starts_with("0b") { body = body[2..].to_string(); base = 2; }
    else if (base == 8 || base == 0) && lower.starts_with("0o") { body = body[2..].to_string(); base = 8; }
    else if (base == 10 || base == 0) && lower.starts_with("0d") { body = body[2..].to_string(); base = 10; }
    else if base == 0 && lower.starts_with('0') && lower.len() > 1 { body = body[1..].to_string(); base = 8; }
    if base == 0 { base = 10; }
    // take the valid prefix
    let mut digits = String::new();
    let mut last_us = true;
    for ch in body.chars() {
        if ch == '_' { if last_us { if strict { return None; } break; } last_us = true; continue; }
        if ch.is_digit(base) { digits.push(ch); last_us = false; } else { if strict { return None; } break; }
    }
    if strict && last_us && !digits.is_empty() { return None; }
    if digits.is_empty() { return if strict { None } else { Some(Value::fix(0)) }; }
    let b = crate::bignum::BigInt::parse(&digits, base)?;
    let b = if neg { b.neg() } else { b };
    Some(crate::bignum::big_to_value(b))
}

pub fn str_to_f(b: &[u8]) -> f64 {
    let s = String::from_utf8_lossy(b);
    let s = s.trim_start();
    let mut end = 0;
    let bytes = s.as_bytes();
    let mut seen_digit = false; let mut seen_dot = false; let mut seen_e = false;
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') { i += 1; }
    while i < bytes.len() {
        let ch = bytes[i];
        if ch.is_ascii_digit() { seen_digit = true; end = i + 1; }
        else if ch == b'_' && seen_digit && i > 0 && bytes[i - 1].is_ascii_digit() && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() { }
        else if ch == b'.' && !seen_dot && !seen_e && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() { seen_dot = true; }
        else if ch == b'.' && !seen_dot && !seen_e && seen_digit && i + 2 < bytes.len() && (bytes[i + 1] | 0x20) == b'e'
            && (bytes[i + 2].is_ascii_digit() || (i + 3 < bytes.len() && (bytes[i + 2] == b'+' || bytes[i + 2] == b'-') && bytes[i + 3].is_ascii_digit())) { seen_dot = true; }
        else if (ch == b'e' || ch == b'E') && seen_digit && !seen_e {
            seen_e = true;
            if i + 1 < bytes.len() && (bytes[i + 1] == b'+' || bytes[i + 1] == b'-') { i += 1; }
        } else { break; }
        i += 1;
    }
    let t: String = s[..end].chars().filter(|&c| c != '_').collect();
    t.parse::<f64>().unwrap_or(0.0)
}
}
