// String: byte buffer + encoding tag (UTF-8 / ASCII-8BIT / US-ASCII).
// Core API on the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
// Methods are in builtins/string_methods.rs.
#![forbid(unsafe_code)]

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
    // always inlined: handing the 3-word buffer to an out-of-line copy costs a store-forwarding stall
    #[inline(always)]
    pub fn str_from_sbuf(&mut self, b: SBuf, enc: u8) -> Value {
        self.str_account(b.len());
        let k = self.vm().c_string.get();
        self.alloc_with(k, move || Body::String(RString { b, enc }))
    }
    #[inline]
    pub fn str_new(&mut self, b: &[u8]) -> Value { self.str_new_enc(b, ENC_UTF8) }
    // the length comes from the slice: reading it back from the buffer just built would wait on its bytes
    #[inline]
    pub fn str_new_enc(&mut self, b: &[u8], enc: u8) -> Value {
        self.str_account(b.len());
        let k = self.vm().c_string.get();
        self.alloc_with(k, move || Body::String(RString { b: SBuf::from_slice(b), enc }))
    }
    /// A chilled string literal (FL_CHILLED / FL_CHILLED_SYM).
    #[inline]
    pub fn str_new_chilled(&mut self, b: &[u8], enc: u8, fl: u32) -> Value {
        self.str_account(b.len());
        let k = self.vm().c_string.get();
        self.alloc_with_flags(k, fl, move || Body::String(RString { b: SBuf::from_slice(b), enc }))
    }
    #[inline]
    pub fn str_with_capa(&mut self, n: usize, enc: u8) -> Value {
        self.str_account(n);
        let k = self.vm().c_string.get();
        self.alloc_with(k, move || Body::String(RString { b: SBuf::with_capacity(n), enc }))
    }
    #[inline]
    pub fn str_from_vec(&mut self, b: Vec<u8>) -> Value { self.str_from_sbuf(SBuf::from_vec(b), ENC_UTF8) }
    #[inline]
    pub fn str_from_vec_enc(&mut self, b: Vec<u8>, enc: u8) -> Value { self.str_from_sbuf(SBuf::from_vec(b), enc) }
    pub fn str_new_frozen(&mut self, b: &[u8]) -> Value {
        let v = self.str_new(b);
        obj_set_flags(v, FL_FROZEN);
        v
    }
    pub fn str_from(&mut self, s: &str) -> Value { self.str_new(s.as_bytes()) }
    /// Encoding of `like`, new contents (class String).
    #[inline]
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

    #[inline(always)]
    pub fn str_enc(&self, v: Value) -> u8 { self.rstring(v).enc }
    #[inline(always)]
    pub fn str_len(&self, v: Value) -> usize { self.rstring(v).b.len() }
    #[inline(always)]
    pub fn str_eq(&self, a: Value, b: Value) -> bool {
        // the encodings first: nothing stays live across the byte compare
        let (x, y) = (self.rstring(a), self.rstring(b));
        if x.enc == y.enc { x.b == y.b } else { str_eq_enc(&x.b, &y.b, x.enc, y.enc) }
    }
    #[inline(always)]
    pub fn str_hash(&self, v: Value) -> u64 {
        let r = self.rstring(v);
        let h = crate::astrust::hash_bytes(&r.b);
        if r.enc == ENC_UTF8 { h } else { str_hash_enc(h, &r.b, r.enc) }
    }
    /// String#to_sym: symbol::BAD_SYM for a broken String of an ASCII-compatible encoding other than UTF-8
    /// (sym_check_asciionly)
    #[inline(always)]
    pub fn str_intern(&self, v: Value) -> Sym {
        let r = self.rstring(v);
        if r.enc != ENC_UTF8 { return str_intern_enc(&r.b, r.enc); }
        intern(&r.b)
    }
    #[inline(always)]
    pub fn str_to_sym(&self, v: Value) -> Value {
        let r = self.rstring(v);
        if r.enc != ENC_UTF8 && r.enc != ENC_ASCII { return str_to_sym_enc(&r.b, r.enc); }
        Value::sym(intern(&r.b))
    }
    #[inline(always)]
    pub fn str_cmp(&self, a: Value, b: Value) -> std::cmp::Ordering {
        let (x, y) = (self.rstring(a), self.rstring(b));
        let o = x.b.cmp(&y.b);
        if o != std::cmp::Ordering::Equal || x.enc == y.enc { o } else { str_cmp_tie(&x.b, x.enc, y.enc) }
    }

    /// String `v` for writing; FrozenError when frozen.
    #[inline]
    pub fn str_modify(&mut self, v: Value) -> Result<&mut RString, Esc> {
        if obj_flags(v) & FL_MODIFY_CHECK != 0 { modify_check_slow(self, v)?; }
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
    #[inline]
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

/// A copy of a String's bytes that outlives the borrow of the context (on the stack when short).
pub struct ByteSnap { buf: [u8; ByteSnap::INLINE], heap: Vec<u8>, n: usize }
impl ByteSnap {
    const INLINE: usize = 128;
    #[inline]
    pub fn new(b: &[u8]) -> ByteSnap {
        let mut s = ByteSnap { buf: [0; ByteSnap::INLINE], heap: Vec::new(), n: b.len() };
        if b.len() <= ByteSnap::INLINE { s.buf[..b.len()].copy_from_slice(b); } else { s.heap = b.to_vec(); }
        s
    }
}
impl std::ops::Deref for ByteSnap {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &[u8] { if self.n <= ByteSnap::INLINE { &self.buf[..self.n] } else { &self.heap } }
}

#[cold]
#[inline(never)]
fn str_eq_enc(a: &[u8], b: &[u8], ea: u8, eb: u8) -> bool { a == b && str_comparable(a, ea, eb) }
/// rb_str_hash's encoding part: a non-ASCII String hashes apart from the same bytes in another encoding.
#[cold]
fn str_hash_enc(h: u64, b: &[u8], enc: u8) -> u64 {
    if b.is_empty() || (crate::builtins::encoding::ascii_compat(enc) && is_ascii(b)) { h } else { crate::astrust::hash_merge(h, enc as u64 + 1) }
}
/// CRuby's encoding index (BINARY 0, UTF-8 1: saruby swaps them).
pub fn cruby_enc_index(e: u8) -> u8 { match e { ENC_UTF8 => 1, ENC_BINARY => 0, e => e } }
/// rb_str_cmp for equal bytes in different encodings: incomparable ones order by encoding index.
#[cold]
fn str_cmp_tie(b: &[u8], ea: u8, eb: u8) -> std::cmp::Ordering {
    if str_comparable(b, ea, eb) { std::cmp::Ordering::Equal } else { cruby_enc_index(ea).cmp(&cruby_enc_index(eb)) }
}
/// rb_str_comparable for two Strings holding the same bytes `b` in encodings `ea` and `eb`.
#[cold]
pub fn str_comparable(b: &[u8], ea: u8, eb: u8) -> bool {
    use crate::builtins::encoding::ascii_compat;
    if b.is_empty() { return true; }
    let seven = is_ascii(b);
    let (a7, b7) = (seven && ascii_compat(ea), seven && ascii_compat(eb));
    (a7 && (b7 || ascii_compat(eb))) || (b7 && ascii_compat(ea))
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

/// A String in another encoding as a symbol: ASCII-only in an ASCII-compatible encoding is the plain
/// symbol (US-ASCII), anything else keeps its encoding (rb_str_intern).
#[cold]
#[inline(never)]
fn str_intern_enc(b: &[u8], enc: u8) -> Sym {
    use crate::builtins::encoding as en;
    // US-ASCII with high bytes is a symbol of its own, not an error
    if enc == ENC_ASCII { return if b.is_ascii() { intern(b) } else { crate::symbol::intern_enc(b, ENC_ASCII) }; }
    if en::ascii_compat(enc) && !b.is_ascii() && !en::is_valid(enc, b) { return crate::symbol::BAD_SYM; }
    str_to_sym_enc(b, enc).as_sym()
}
/// The symbol of bytes `b` of a String in `enc` (as String#to_sym without its check).
pub fn sym_of(b: &[u8], enc: u8) -> crate::value::Sym {
    if enc == ENC_UTF8 || enc == ENC_ASCII { intern(b) } else { str_to_sym_enc(b, enc).as_sym() }
}
#[cold]
#[inline(never)]
fn str_to_sym_enc(b: &[u8], enc: u8) -> Value {
    if crate::builtins::encoding::ascii_compat(enc) && is_ascii(b) { return Value::sym(intern(b)); }
    Value::sym(crate::symbol::intern_enc(b, enc))
}

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
    if enc == ENC_UTF8 { return is_ascii(b); }   // the common case, without the encoding table
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
    // valid UTF-8: one character per non-continuation byte
    if enc == ENC_UTF8 && std::str::from_utf8(b).is_ok() { return b.iter().filter(|&&x| (x as i8) >= -0x40).count(); }
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

/// String#inspect body, as a Rust String (lossy if a character was kept raw in a non-UTF-8 result encoding).
pub fn inspect_bytes(b: &[u8], enc: u8) -> String {
    match String::from_utf8(inspect_vec(b, enc)) { Ok(s) => s, Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned() }
}
/// rb_str_inspect: the body in the result encoding (inspect_resenc).
pub fn inspect_vec(b: &[u8], enc: u8) -> Vec<u8> {
    use crate::builtins::encoding as en;
    let mut out = Vec::with_capacity(b.len() + 2);
    out.push(b'"');
    let uni = en::is_unicode(enc) && !en::is_dummy(enc);
    let ac = en::ascii_compat(enc);
    // non-ASCII characters print raw only in the result encoding (default_internal, else default_external)
    let same = enc == inspect_resenc();
    let same_u8 = same && en::kind(enc) == en::EK::Utf8;
    let code_at = |i: usize| -> Option<u32> { en::precise_len(enc, b, i).map(|l| en::char_code(enc, &b[i..i + l])) };
    let mut run = (1u32, 0u32); // last Print range hit: runs of one script skip the search
    let mut i = 0;
    while i < b.len() {
        let Some(l) = en::precise_len(enc, b, i) else {
            let n = en::mbclen(enc, b, i);
            for k in 0..n { push_fmt(&mut out, format_args!("\\x{:02X}", b[i + k])); }
            i += n; continue;
        };
        let cp = en::char_code(enc, &b[i..i + l]);
        let is_asc = (ac && l == 1 && cp < 0x80) || (uni && cp < 0x80);
        // rb_str_inspect escapes these controls whatever the encoding
        let ctl = match cp { 0x0a => "\\n", 0x0d => "\\r", 0x09 => "\\t", 0x0c => "\\f", 0x0b => "\\v", 0x08 => "\\b", 0x07 => "\\a", 0x1b => "\\e", _ => "" };
        if !is_asc && !ctl.is_empty() { out.extend_from_slice(ctl.as_bytes()); i += l; continue; }
        if is_asc {
            match cp as u8 {
                b'"' => out.extend_from_slice(b"\\\""),
                b'\\' => out.extend_from_slice(b"\\\\"),
                b'\n' => out.extend_from_slice(b"\\n"),
                b'\t' => out.extend_from_slice(b"\\t"),
                b'\r' => out.extend_from_slice(b"\\r"),
                0x0c => out.extend_from_slice(b"\\f"),
                0x0b => out.extend_from_slice(b"\\v"),
                0x08 => out.extend_from_slice(b"\\b"),
                0x07 => out.extend_from_slice(b"\\a"),
                0x1b => out.extend_from_slice(b"\\e"),
                b'#' => {
                    let nx = if i + l < b.len() { code_at(i + l) } else { None };
                    if matches!(nx, Some(0x7b) | Some(0x24) | Some(0x40)) { out.extend_from_slice(b"\\#"); } else { out.push(b'#'); }
                }
                x @ 0x20..=0x7e => out.push(x),
                x => if uni { push_fmt(&mut out, format_args!("\\u{:04X}", x)) } else { push_fmt(&mut out, format_args!("\\x{:02X}", x)) },
            }
        } else if uni {
            if same_u8 && (cp >= run.0 && cp <= run.1 || astrogre::re::uni_print_range(cp).is_some_and(|r| { run = r; true })) { out.extend_from_slice(&b[i..i + l]); }
            else if cp > 0xffff { push_fmt(&mut out, format_args!("\\u{{{:X}}}", cp)); }
            else { push_fmt(&mut out, format_args!("\\u{:04X}", cp)); }
        } else if same && (l > 1 || en::hi_byte_print(enc, b[i])) {
            // a multibyte character of a legacy encoding is always printable (Onigmo's mb2 / mb4 ctype)
            out.extend_from_slice(&b[i..i + l]);
        } else if l == 1 {
            push_fmt(&mut out, format_args!("\\x{:02X}", b[i]));
        } else {
            push_fmt(&mut out, format_args!("\\x{{{:X}}}", cp));
        }
        i += l;
    }
    out.push(b'"');
    out
}
fn push_fmt(out: &mut Vec<u8>, a: std::fmt::Arguments) { use std::io::Write; let _ = out.write_fmt(a); }

/// Onigmo's print class (Unicode 17 tables shared with astrogre).
pub fn uni_printable(cp: u32) -> bool { astrogre::re::uni_is_print(cp) }

/// rb_str_inspect's resenc.
pub fn inspect_resenc() -> u8 { crate::builtins::encoding::default_internal().unwrap_or_else(crate::builtins::encoding::default_external) }
/// Symbol#inspect of a name in `enc`: the bytes and the result's encoding.
pub fn sym_inspect(name: &[u8], enc: u8) -> (Vec<u8>, u8) {
    use crate::builtins::encoding as en;
    // rb_str_symname_p: a non-ASCII name is quoted unless the result encoding is the symbol's
    let ascii = is_ascii(name) && en::ascii_compat(enc);
    let simple = if enc == ENC_UTF8 || ascii { sym_is_simple(name) } else {
        en::is_valid(enc, name) && crate::builtins::econv::convert_string(en::enc_name(enc), "UTF-8", name).is_some_and(|u| sym_is_simple(&u))
    };
    if simple && (ascii || inspect_resenc() == enc) {
        let mut v = b":".to_vec(); v.extend_from_slice(name);
        return (v, if ascii { crate::object::ENC_ASCII } else { enc });
    }
    let mut v = b":".to_vec(); v.extend_from_slice(&inspect_vec(name, if ascii { crate::object::ENC_ASCII } else { enc }));
    let r = inspect_resenc();
    (v, if en::ascii_compat(r) { r } else { crate::object::ENC_ASCII })
}

pub fn sym_is_simple(n: &[u8]) -> bool {
    if n.is_empty() { return false; }
    let ops: &[&[u8]] = &[b"+", b"-", b"*", b"/", b"%", b"**", b"==", b"===", b"!=", b"<=>", b"<", b"<=", b">", b">=", b"<<", b">>",
        b"!", b"~", b"+@", b"-@", b"[]", b"[]=", b"=~", b"!~", b"&", b"|", b"^", b"`"];
    if ops.contains(&n) { return true; }
    let s = match std::str::from_utf8(n) { Ok(s) => s, Err(_) => return false };
    // rb_str_symname_p's sym_printable (U+0085 passes rb_enc_isprint)
    if !s.is_ascii() && !s.chars().all(|c| (c as u32) < 0x80 || c as u32 == 0x85 || uni_printable(c as u32)) { return false; }
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

// set once a method_missing / respond_to? is defined after boot (check_array's fast path until then)
static MM_REDEF: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[inline] pub fn mm_redef() -> bool { MM_REDEF.load(std::sync::atomic::Ordering::Relaxed) }
pub fn note_mm_redef() { MM_REDEF.store(true, std::sync::atomic::Ordering::Relaxed) }
/// Implicit conversion to String (to_str).
/// rb_check_funcall without arguments: None when `v` does not respond to `mid`.  A respond_to? of its
/// own is asked first; a method_missing of its own is tried when there is no method (NoMethodError: None).
pub fn check_funcall(c: &mut Ctx, v: Value, mid: Sym) -> Result<Option<Value>, Esc> { check_funcall_args(c, v, mid, &[]) }
/// vm_respond_to: asks a user respond_to? (mid, true); a one-parameter one is deprecated (warned) and gets only mid.
pub fn ask_respond_to(c: &mut Ctx, v: Value, rt: *const MethodEntry, mid: Sym) -> Result<bool, Esc> {
    let ar = crate::builtins::proc::method_arity(c, rt);
    if ar > 2 { return arg_err(c, format!("respond_to? must accept 1 or 2 arguments (requires {})", ar)); }
    if ar == 1 {
        respond_to_arity1_warn(c, v, rt, mid)?;
        return Ok(funcall(c, v, s::respond_to_p, &[Value::sym(mid)], std::ptr::null())?.truthy());
    }
    Ok(funcall(c, v, s::respond_to_p, &[Value::sym(mid), TRUE], std::ptr::null())?.truthy())
}
#[cold]
fn respond_to_arity1_warn(c: &mut Ctx, v: Value, rt: *const MethodEntry, mid: Sym) -> Result<(), Esc> {
    if c.vm().verbose.get() == NIL { return Ok(()); }
    if crate::builtins::misc::warning_category_enabled(0) {
        let k = class_of(v);
        let (recv, sep) = if c.class(k).is_singleton { (to_rstring(c, v)?, '.') } else { (class_path(c, k), '#') };
        let q = crate::builtins::kernel::inspect_str(c, Value::sym(mid))?;
        crate::builtins::kernel::warn_deprecated(c, &format!("{}{}respond_to?({}) uses the deprecated method signature, which takes one parameter", recv, sep, q))?;
    }
    let loc = match c.me_ref(rt).map(|m| m.kind) { Some(MKind::Iseq(is)) => c.iseq_ref(is).map(|is| (is.info.file.to_string(), is.info.line)), _ => None };
    if let Some((f, l)) = loc { crate::builtins::kernel::warn_deprecated_at(c, &f, l as u32, "respond_to? is defined here")?; }
    Ok(())
}
/// rb_check_funcall with arguments.
pub fn check_funcall_args(c: &mut Ctx, v: Value, mid: Sym, args: &[Value]) -> Result<Option<Value>, Esc> {
    let k = class_of(v);
    let rt = lookup_method(c, k, s::respond_to_p);
    // a NoMethodError from method_missing is raised when respond_to? / respond_to_missing? said yes
    let mut said_yes = false;
    if c.me_ref(rt).is_some_and(|m| m.owner != c.vm().m_kernel.get()) {
        if !ask_respond_to(c, v, rt, mid)? { return Ok(None); }
        said_yes = true;
    }
    if respond_to(c, v, mid, true) { return funcall(c, v, mid, args, std::ptr::null()).map(Some); }
    // check_funcall_missing: a respond_to_missing? of its own is asked first
    let rm = lookup_method(c, k, s::respond_to_missing_p);
    if c.me_ref(rm).is_some_and(|m| m.owner != c.vm().c_object.get() && m.owner != c.vm().m_kernel.get() && m.owner != c.vm().c_basic_object.get()) {
        if !funcall(c, v, s::respond_to_missing_p, &[Value::sym(mid), TRUE], std::ptr::null())?.truthy() { return Ok(None); }
        said_yes = true;
    }
    let mm = lookup_method(c, k, s::method_missing);
    if !c.me_ref(mm).is_some_and(|m| m.owner != c.vm().c_basic_object.get()) { return Ok(None); }
    let mut ma = vec![Value::sym(mid)]; ma.extend_from_slice(args);
    match funcall(c, v, s::method_missing, &ma, std::ptr::null()) {
        Ok(r) => Ok(Some(r)),
        Err(e) => {
            if !said_yes && c.esc == EscKind::Raise && is_kind_of(c, c.esc_val, c.vm().c_no_method_error.get()) { c.esc = EscKind::None; return Ok(None); }
            Err(e)
        }
    }
}
pub fn expect_str(c: &mut Ctx, v: Value) -> RV {
    if is_string(v) { return Ok(v); }
    if let Some(r) = if crate::rt::conv_target(v) { check_funcall(c, v, s::to_str)? } else { None } {
        if is_string(r) { return Ok(r); }
        let cls = class_path(c, real_class_of(c, v));
        return type_err(c, format!("can't convert {} to String ({}#to_str gives {})", cls, cls, class_path(c, real_class_of(c, r))));
    }
    type_err(c, format!("no implicit conversion of {} into String", crate::builtins::kernel::class_name_for_err(c, v)))
}

// ---- case helpers -----------------------------------------------------------------------

pub fn map_case(b: &[u8], enc: u8, f: impl Fn(char) -> String, ascii: impl Fn(u8) -> u8) -> Vec<u8> {
    if enc != ENC_UTF8 || is_ascii(b) { return b.iter().map(|&x| ascii(x)).collect(); }
    match std::str::from_utf8(b) {
        Ok(s) => { let mut out = String::new(); for ch in s.chars() { out.push_str(&f(ch)); } out.into_bytes() }
        Err(_) => b.iter().map(|&x| ascii(x)).collect(),
    }
}
fn case_lookup(t: &'static [(u32, &'static str)], ch: char) -> Option<&'static str> {
    t.binary_search_by_key(&(ch as u32), |e| e.0).ok().map(|i| t[i].1)
}
/// Per-character mappings matching CRuby (`turkic`: dotted/dotless i).
pub fn up_char(ch: char, turkic: bool, out: &mut String) { if turkic && ch == 'i' { out.push('İ') } else { out.extend(ch.to_uppercase()) } }
pub fn down_char(ch: char, turkic: bool, out: &mut String) {
    if turkic && ch == 'I' { out.push('ı') } else if turkic && ch == 'İ' { out.push('i') } else { out.extend(ch.to_lowercase()) }
}
pub fn title_char(ch: char, turkic: bool, out: &mut String) {
    match case_lookup(&crate::builtins::case_tables::TITLE, ch) { Some(t) => out.push_str(t), None => up_char(ch, turkic, out) }
}
pub fn swap_char(ch: char, turkic: bool, out: &mut String) {
    if let Some(t) = case_lookup(&crate::builtins::case_tables::SWAP, ch) { out.push_str(t) }
    else if ch.is_uppercase() { down_char(ch, turkic, out) } else { up_char(ch, turkic, out) }
}
pub fn fold_char(ch: char, out: &mut String) {
    match case_lookup(&crate::builtins::case_tables::FOLD, ch) { Some(t) => out.push_str(t), None => out.extend(ch.to_lowercase()) }
}
/// UTF-8 case mapping: `first` for the first character, `rest` for the others; None if not valid UTF-8.
pub fn map_utf8(b: &[u8], first: impl Fn(char, &mut String), rest: impl Fn(char, &mut String)) -> Option<Vec<u8>> {
    let s = std::str::from_utf8(b).ok()?;
    let mut out = String::with_capacity(b.len());
    for (i, ch) in s.chars().enumerate() { if i == 0 { first(ch, &mut out) } else { rest(ch, &mut out) } }
    Some(out.into_bytes())
}
pub fn upcase(b: &[u8], enc: u8) -> Vec<u8> { map_case(b, enc, |c| c.to_uppercase().collect(), |x| x.to_ascii_uppercase()) }
pub fn downcase(b: &[u8], enc: u8) -> Vec<u8> { map_case(b, enc, |c| c.to_lowercase().collect(), |x| x.to_ascii_lowercase()) }
pub fn swapcase(b: &[u8], enc: u8) -> Vec<u8> {
    map_case(b, enc, |c| { let mut o = String::new(); swap_char(c, false, &mut o); o },
             |x| if x.is_ascii_uppercase() { x.to_ascii_lowercase() } else { x.to_ascii_uppercase() })
}
/// Full Unicode case folding (downcase(:fold), casecmp?).
pub fn fold_case(b: &[u8], enc: u8) -> Vec<u8> { map_case(b, enc, |c| { let mut o = String::new(); fold_char(c, &mut o); o }, |x| x.to_ascii_lowercase()) }
pub fn capitalize(b: &[u8], enc: u8) -> Vec<u8> {
    if enc == ENC_UTF8 && !is_ascii(b) { if let Some(v) = map_utf8(b, |c, o| title_char(c, false, o), |c, o| down_char(c, false, o)) { return v; } }
    let mut v: Vec<u8> = b.iter().map(|x| x.to_ascii_lowercase()).collect();
    if let Some(f) = v.first_mut() { *f = f.to_ascii_uppercase(); }
    v
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

/// The Integer `b` spells as String#to_i / Integer() read it (None: `strict` and not one).
pub fn str_to_inum(c: &mut Ctx, b: &[u8], base: u32, strict: bool) -> Option<Value> { str_to_big(b, base, strict).map(|n| c.big_to_value(n)) }
pub fn str_to_big(b: &[u8], base: u32, strict: bool) -> Option<crate::bignum::BigInt> {
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
    if digits.is_empty() { return if strict { None } else { Some(crate::bignum::BigInt::zero()) }; }
    let b = crate::bignum::BigInt::parse(&digits, base)?;
    Some(if neg { b.neg() } else { b })
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
