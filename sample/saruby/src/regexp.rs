// Regexp / MatchData objects and $~.  The matching engine is regex_engine.rs.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::obj_set_flags;
use crate::object::*;
use crate::regex_engine::{Enc, Regex};
use crate::string::*;
use crate::value::*;
use crate::vm::*;

pub struct RRegexp { pub src: Vec<u8>, pub opts: u32, pub re: Option<Regex>, pub enc: u8, pub timeout: Option<f64> }
impl RData for RRegexp {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
impl RRegexp {
    pub fn fixed(&self) -> bool { self.opts & RE_OPT_FIXED != 0 }
}

pub struct RMatch { pub regexp: Value, pub str: Value, pub groups: Vec<Option<(usize, usize)>> }
impl RData for RMatch {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.regexp); m.visit(&mut self.str); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

pub const RE_OPT_FIXED: u32 = 16;
pub const RE_OPT_ENC_NONE: u32 = 32;
/// Regexp#options bits; a literal's opts also carry (source encoding + 1) << RE_LIT_ENC_SHIFT.
pub const RE_OPT_MASK: u32 = 7 | RE_OPT_FIXED | RE_OPT_ENC_NONE;
pub const RE_LIT_ENC_SHIFT: u32 = 8;

/// Mode marker (not an engine mode) for a UTF-16/32 string of encoding (mode - WIDE): its
/// UTF-8 form is matched (the pattern was read as UTF-8 too) and offsets are mapped back.
const WIDE: u8 = 0x80;

type Groups = Vec<Option<(usize, usize)>>;

/// One search (or anchored match with `at`) in mode `e`.
#[inline(always)]
pub fn engine_run(rx: &Regex, bytes: &[u8], start: usize, e: Enc, dl: Option<std::time::Instant>, at: bool) -> Result<Option<Groups>, ()> {
    if e.0 >= WIDE { return wide_run(rx, bytes, start, e, dl, at); }
    if at { Ok(rx.match_at_enc(bytes, start, e)) } else { rx.search_timeout(bytes, start, e, dl) }
}

#[cold]
#[inline(never)]
fn wide_run(rx: &Regex, bytes: &[u8], start: usize, e: Enc, dl: Option<std::time::Instant>, at: bool) -> Result<Option<Groups>, ()> {
    use crate::builtins::encoding as en;
    let enc = e.0 - WIDE;
    let (mut u, mut orig) = (Vec::with_capacity(bytes.len()), Vec::with_capacity(bytes.len() + 1));
    let mut i = 0;
    let mut ustart = None;
    while i < bytes.len() {
        if i == start { ustart = Some(u.len()); }
        let l = en::precise_len(enc, bytes, i).unwrap_or(1);
        let ch = en::to_unicode(enc, &bytes[i..(i + l).min(bytes.len())]).and_then(char::from_u32).unwrap_or(char::REPLACEMENT_CHARACTER);
        let n = u.len();
        u.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
        orig.resize(orig.len() + (u.len() - n), i);
        i += l;
    }
    orig.push(bytes.len());
    let Some(us) = ustart.or(if start >= bytes.len() { Some(u.len()) } else { None }) else { return Ok(None) };
    let g = if at { rx.match_at_enc(&u, us, Enc::UTF8) } else { rx.search_timeout(&u, us, Enc::UTF8, dl)? };
    Ok(g.map(|g| g.into_iter().map(|x| x.map(|(a, b)| (orig[a], orig[b]))).collect()))
}

/// How the engine splits a string of `enc` into chars.
pub fn engine_enc(enc: u8) -> Enc {
    use crate::builtins::encoding::{kind, EK};
    match kind(enc) {
        EK::Utf8 => Enc::UTF8,
        EK::U16LE | EK::U16BE | EK::U32LE | EK::U32BE | EK::Cesu8 => Enc(WIDE + enc),
        EK::EucJp(_) => Enc::EUCJP,
        EK::Sjis(_) => Enc::SJIS,
        EK::Dbcs | EK::Big5 | EK::Gbk => Enc::DBCS,
        EK::EucTw => Enc::EUCTW,
        EK::Gb18030 => Enc::GB18030,
        _ => Enc::BYTE,
    }
}

/// A Regexp from a literal or internal source: `opts` may carry the literal's source encoding.
pub fn regexp_new(c: &mut Ctx, src: &[u8], opts: u32) -> RV {
    let e = (opts >> RE_LIT_ENC_SHIFT) & 0xff;
    let enc = if e == 0 { ENC_UTF8 } else { (e - 1) as u8 };
    regexp_new_enc(c, src, enc, opts & RE_OPT_MASK)
}

/// An interpolated literal: the built String's encoding, unless /e /s /u fixed one.
pub fn regexp_new_dyn(c: &mut Ctx, s: Value, opts: u32) -> RV {
    let src = c.str_bytes(s).to_vec();
    let r = if opts & RE_OPT_FIXED != 0 { regexp_new(c, &src, opts)? } else { let e = c.str_enc(s); regexp_new_enc(c, &src, e, opts & RE_OPT_MASK)? };
    obj_set_flags(r, FL_FROZEN);
    Ok(r)
}

/// Where each regexp literal's source is (by the address of its leaked bytes), for the error.
static LIT_LINES: std::sync::Mutex<Option<std::collections::HashMap<usize, (&'static str, u32)>>> = std::sync::Mutex::new(None);
pub fn note_literal(src: &'static [u8], file: &'static str, line: u32) {
    if let Ok(mut m) = LIT_LINES.lock() { m.get_or_insert_with(Default::default).insert(src.as_ptr() as usize, (file, line)); }
}

/// Compile-time check of a literal (CRuby builds it while compiling): parse only, nothing is kept.
/// Returns the error message and the parse-time warnings.
pub fn check_literal(src: &[u8], opts: u32) -> (Option<String>, Vec<String>) {
    let e = (opts >> RE_LIT_ENC_SHIFT) & 0xff;
    let enc = if e == 0 { ENC_UTF8 } else { (e - 1) as u8 };
    let r = regexp_check(src, enc, opts & RE_OPT_MASK);
    (r.err(), crate::regex_engine::take_warnings())
}

/// regexp_build without the matcher: the same errors if the engine's parse finds every one (see the trial README).
pub fn regexp_check(src: &[u8], enc: u8, opts: u32) -> Result<(), String> {
    let (unesc, opts, enc, mode) = regexp_prep(src, enc, opts)?;
    Regex::check(&unesc, opts & 7, mode).map_err(|e| format!("{}: {}", e, regexp_desc(src, enc, opts)))
}

/// A regexp literal: CRuby rejects an invalid one at parse time, so it is a SyntaxError here.
pub fn regexp_new_lit(c: &mut Ctx, src: &[u8], opts: u32) -> RV {
    let e = (opts >> RE_LIT_ENC_SHIFT) & 0xff;
    let enc = if e == 0 { ENC_UTF8 } else { (e - 1) as u8 };
    match regexp_build(src, enc, opts & RE_OPT_MASK) {
        Ok(x) => { let _ = crate::regex_engine::take_warnings(); Ok(regexp_alloc(c, x)) }
        Err(m) => {
            let at = LIT_LINES.lock().ok().and_then(|t| t.as_ref().and_then(|t| t.get(&(src.as_ptr() as usize)).copied()));
            let loc = match at { Some((f, l)) => format!("{}:{}", f, l), None => backtrace_lines(c, 0).first().map(|l| l.split(":in ").next().unwrap_or("").to_string()).unwrap_or_default() };
            err(c, c.vm().c_syntax_error.get(), format!("{}: {}", loc, m))
        }
    }
}

/// rb_reg_initialize_str: `src` is a String of encoding `enc`.
pub fn regexp_new_enc(c: &mut Ctx, src: &[u8], enc: u8, opts: u32) -> RV {
    match regexp_build(src, enc, opts) {
        Ok(x) => { emit_parse_warnings(c)?; Ok(regexp_alloc(c, x)) }
        Err(m) => Err(regexp_error(c, &m, enc)),
    }
}
/// RegexpError from rb_enc_reg_error_desc: the message is in the pattern's encoding (even UTF-16).
pub fn regexp_error(c: &mut Ctx, m: &str, enc: u8) -> Esc {
    let e = make_exception(c, c.vm().c_regexp_error.get(), "");
    let ms = c.str_new_enc(m.as_bytes(), enc);
    ivar_set_raw(c, e, crate::symbol::s::mesg, ms);
    raise_exc(c, e)
}

fn regexp_alloc(c: &mut Ctx, (src, opts, enc, re): (Vec<u8>, u32, u8, Regex)) -> Value {
    c.alloc_with(c.vm().c_regexp.get(), move || Body::Data(Box::new(RRegexp { src, opts, re: Some(re), enc, timeout: None })))
}

/// Source, options, encoding and compiled pattern, or the RegexpError message.
pub fn regexp_build(src: &[u8], enc: u8, opts: u32) -> Result<(Vec<u8>, u32, u8, Regex), String> {
    let (unesc, opts, enc, mode) = regexp_prep(src, enc, opts)?;
    let re = Regex::new_enc(&unesc, opts & 7, mode).map_err(|e| format!("{}: {}", e, regexp_desc(src, enc, opts)))?;
    Ok((src.to_vec(), opts, enc, re))
}


/// rb_reg_initialize up to the engine: (pattern for the engine, options, encoding, engine mode).
fn regexp_prep(src: &[u8], enc: u8, opts: u32) -> Result<(Vec<u8>, u32, u8, Enc), String> {
    use crate::builtins::encoding as en;
    let desc = |e: &str| format!("{}: {}", e, regexp_desc(src, enc, opts));
    let mut enc = enc;
    let mut opts = opts & RE_OPT_MASK;
    if opts & RE_OPT_ENC_NONE != 0 && enc != ENC_BINARY {
        if !(en::ascii_compat(enc) && is_ascii(src)) { return Err(desc("/.../n has a non escaped non ASCII character in non ASCII-8BIT script")); }
        enc = ENC_BINARY;
    }
    if en::is_dummy(enc) { return Err(desc("can't make regexp with dummy encoding")); }
    // an ASCII-incompatible pattern is read in UTF-8 (its chars are the same)
    // a UTF-16/32 pattern is not preprocessed (each character is multibyte): Onigmo reads its escapes
    let (unesc, fixed) = if en::ascii_compat(enc) { preprocess(src, enc, opts & 2 != 0) } else { wide_unescape(src, enc, opts & 2 != 0).map(|u| (u, Some(enc))) }.map_err(|e| desc(&e))?;
    let fixed = if !en::ascii_compat(enc) { Some(enc) } else { fixed };
    match fixed {
        Some(fe) => {
            if (fe != enc && opts & RE_OPT_FIXED != 0) || (fe != ENC_BINARY && opts & RE_OPT_ENC_NONE != 0) {
                return Err(desc("incompatible character encoding"));
            }
            if fe != ENC_BINARY { enc = fe; }
            opts |= RE_OPT_FIXED;
        }
        None => if opts & RE_OPT_FIXED == 0 { enc = ENC_ASCII; },
    }
    // an ASCII-only pattern is used on any ASCII-compatible string; most strings are UTF-8
    let mode = if opts & RE_OPT_FIXED != 0 && en::ascii_compat(enc) { engine_enc(enc) } else { Enc::UTF8 };
    // CESU-8 is matched in its UTF-8 form, as UTF-16/32
    let unesc = if mode.0 >= WIDE && en::ascii_compat(enc) { wide_to_utf8(&unesc, enc).ok_or_else(|| desc("invalid multibyte character"))? } else { unesc };
    Ok((unesc, opts, enc, mode))
}

fn wide_to_utf8(src: &[u8], enc: u8) -> Option<Vec<u8>> {
    use crate::builtins::encoding as en;
    let mut out = vec![];
    let mut i = 0;
    while i < src.len() {
        let l = en::precise_len(enc, src, i)?;
        let u = en::to_unicode(enc, &src[i..i + l])?;
        out.extend_from_slice(char::from_u32(u)?.encode_utf8(&mut [0; 4]).as_bytes());
        i += l;
    }
    Some(out)
}

/// A UTF-16/32 pattern in UTF-8 for the engine, with the escapes Onigmo reads itself in such a pattern
/// (rb_reg_preprocess leaves them): \x{H..} and \cX / \C-X / \M-X are code points, \xHH / \OOO are raw
/// bytes that must make whole characters, \u is a plain "u".  Comments are copied unread.
fn wide_unescape(src: &[u8], enc: u8, ext: bool) -> Result<Vec<u8>, String> {
    use crate::builtins::encoding as en;
    let mut cs: Vec<u32> = vec![];
    let mut i = 0;
    while i < src.len() {
        let l = en::precise_len(enc, src, i).ok_or("invalid multibyte character")?;
        cs.push(en::char_code(enc, &src[i..i + l]));
        i += l;
    }
    let ch = |k: usize| cs.get(k).copied();
    let is_x = |v: Option<u32>| v.is_some_and(|v| v < 0x80 && (v as u8).is_ascii_hexdigit());
    let mut out = Vec::with_capacity(cs.len());
    let put = |out: &mut Vec<u8>, v: u32| { if let Some(c) = char::from_u32(v) { out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes()); } };
    // a code point from an escape: ASCII as \xHH so it stays a literal
    let code = |out: &mut Vec<u8>, v: u32| { if v < 0x80 { out.extend_from_slice(format!("\\x{:02X}", v).as_bytes()); } else { put(out, v); } };
    let (mut ext, mut class, mut groups) = (ext, 0u32, 0u32);
    // one raw byte escape at cs[p] (after the backslash at p - 1): (byte, base, next index), or None if it is not one
    let raw_at = |p: usize, groups: u32| -> Result<Option<(u8, u8, usize)>, String> {
        if ch(p.wrapping_sub(1)) != Some(b'\\' as u32) { return Ok(None); }
        match ch(p) {
            Some(0x78) if ch(p + 1) != Some(b'{' as u32) => {
                let (mut v, mut q) = (0u32, p + 1);
                while q < p + 3 && is_x(ch(q)) { v = v * 16 + (ch(q).unwrap() as u8 as char).to_digit(16).unwrap(); q += 1; }
                Ok(Some((v as u8, 16, q)))
            }
            Some(d @ 0x30..=0x39) => {
                let mut q = p;
                let mut n: u64 = 0;
                while let Some(x @ 0x30..=0x39) = ch(q) { n = (n * 10 + (x - 0x30) as u64).min(1 << 40); q += 1; }
                if d != 0x30 && (n <= 9 || n <= groups as u64) { return Ok(None); }   // a back reference
                if d == 0x38 || d == 0x39 { return Ok(None); }
                let (mut v, mut q) = (0u32, if d == 0x30 { p + 1 } else { p });
                let max = if d == 0x30 { p + 3 } else { p + 3 };
                while q < max && matches!(ch(q), Some(0x30..=0x37)) { v = v * 8 + ch(q).unwrap() - 0x30; q += 1; }
                if v > 0xff { return Err("too big number".into()); }
                Ok(Some((v as u8, 8, q)))
            }
            _ => Ok(None),
        }
    };
    let mut p = 0;
    while p < cs.len() {
        let c = cs[p];
        if c != b'\\' as u32 {
            p += 1;
            match c {
                0x28 if class == 0 => {
                    // (?#...) is skipped by Onigmo as it is
                    if ch(p) == Some(0x3f) && ch(p + 1) == Some(0x23) {
                        out.extend_from_slice(b"(?#");
                        p += 2;
                        while p < cs.len() && cs[p] != 0x29 { if cs[p] == 0x5c && p + 1 < cs.len() { put(&mut out, cs[p]); p += 1; } put(&mut out, cs[p]); p += 1; }
                        if p < cs.len() { out.push(b')'); p += 1; }
                        continue;
                    }
                    if ch(p) != Some(0x3f) || (ch(p + 1) == Some(0x3c) && !matches!(ch(p + 2), Some(0x3d) | Some(0x21))) || ch(p + 1) == Some(0x27) { groups += 1; }
                    // (?x) / (?-x) for the rest of the pattern
                    let mut q = p + 1;
                    if ch(p) == Some(0x3f) {
                        let mut neg = false;
                        while let Some(x) = ch(q) { match x { 0x2d => neg = true, 0x78 => { if ch(q + 1).is_some_and(|y| y == 0x29 || y == 0x3a || y == 0x2d || (y as u8).is_ascii_alphabetic()) { ext = !neg; } } 0x69 | 0x6d | 0x61 | 0x64 | 0x75 => {} _ => break } q += 1; }
                    }
                    let _ = q;
                }
                0x5b => class += 1,
                0x5d => class = class.saturating_sub(1),
                0x23 if ext && class == 0 => {
                    while p < cs.len() && cs[p] != 0x0a { p += 1; }
                    continue;
                }
                _ => {}
            }
            put(&mut out, c);
            continue;
        }
        let Some(e) = ch(p + 1) else { return Err("end pattern at escape".into()) };
        if e >= 0x80 { out.push(b'\\'); put(&mut out, e); p += 2; continue; }
        // raw bytes: as many as make whole characters
        if let Some((b0, base, mut q)) = raw_at(p + 1, groups)? {
            let mut bytes = vec![b0];
            loop {
                let need = if bytes.len() >= en::min_len(enc) { match en::precise_len(enc, &bytes, 0) { Some(l) => l, None => if en::needmore(enc, &bytes) { bytes.len() + 1 } else { bytes.len() } } } else { en::min_len(enc) };
                if bytes.len() >= need { break; }
                match raw_at(q + 1, groups)? { Some((b, bb, q2)) if ch(q) == Some(0x5c) && (class == 0 || bb == base) => { bytes.push(b); q = q2; } _ => return Err("too short multibyte code string".into()) }
            }
            if en::precise_len(enc, &bytes, 0) != Some(bytes.len()) { return Err("invalid code point value".into()); }
            code(&mut out, en::char_code(enc, &bytes));
            p = q;
            continue;
        }
        match e as u8 {
            b'x' => {
                // \x{H..}: up to 8 digits, then a code point of the encoding
                let (mut v, mut q) = (0u64, p + 3);
                while q < p + 11 && is_x(ch(q)) { v = v * 16 + (ch(q).unwrap() as u8 as char).to_digit(16).unwrap() as u64; if v > i32::MAX as u64 { return Err("too big wide-char value".into()); } q += 1; }
                if is_x(ch(q)) { return Err("too long wide-char value".into()); }
                if q > p + 3 && ch(q) == Some(0x7d) {
                    let v = v as u32;
                    if v > 0x10ffff || (0xd800..0xe000).contains(&v) { return Err("invalid code point value".into()); }
                    code(&mut out, v);
                    p = q + 1;
                } else { out.push(b'x'); p += 2; }
            }
            b'u' => { out.push(b'u'); p += 2; }
            b'c' | b'C' | b'M' => {
                // fetch_escaped_value
                fn esc(cs: &[u32], p: &mut usize) -> Result<u32, String> {
                    let c = cs[*p]; *p += 1;
                    let at = |k: usize| cs.get(k).copied();
                    match c {
                        0x4d => {
                            if *p >= cs.len() { return Err("end pattern at meta".into()); }
                            if at(*p) != Some(0x2d) { return Err("invalid meta-code syntax".into()); }
                            *p += 1;
                            if *p >= cs.len() { return Err("end pattern at meta".into()); }
                            let mut v = cs[*p]; *p += 1;
                            if v == 0x5c { v = esc(cs, p)?; }
                            Ok((v & 0xff) | 0x80)
                        }
                        0x43 | 0x63 => {
                            if c == 0x43 {
                                if *p >= cs.len() { return Err("end pattern at control".into()); }
                                if at(*p) != Some(0x2d) { return Err("invalid control-code syntax".into()); }
                                *p += 1;
                            }
                            if *p >= cs.len() { return Err("end pattern at control".into()); }
                            let mut v = cs[*p]; *p += 1;
                            if v == 0x3f { return Ok(0x7f); }
                            if v == 0x5c { v = esc(cs, p)?; }
                            Ok(v & 0x9f)
                        }
                        0x6e => Ok(0x0a), 0x74 => Ok(0x09), 0x72 => Ok(0x0d), 0x66 => Ok(0x0c), 0x76 => Ok(0x0b), 0x61 => Ok(0x07), 0x62 => Ok(0x08), 0x65 => Ok(0x1b),
                        x => Ok(x),
                    }
                }
                let mut q = p + 1;
                let v = esc(&cs, &mut q)?;
                code(&mut out, v);
                p = q;
            }
            _ => { out.push(b'\\'); put(&mut out, e); p += 2; }
        }
    }
    Ok(out)
}

/// "/src/mix" as RegexpError messages show the pattern (rb_reg_error_desc).
fn regexp_desc(src: &[u8], enc: u8, opts: u32) -> String {
    use crate::builtins::encoding as en;
    let mut s = vec![b'/'];
    expr_str(&mut s, src, enc, Some(en::default_internal().unwrap_or_else(en::default_external)), b'/');
    s.push(b'/');
    if opts & 4 != 0 { s.push(b'm'); }
    if opts & 1 != 0 { s.push(b'i'); }
    if opts & 2 != 0 { s.push(b'x'); }
    String::from_utf8_lossy(&s).into_owned()
}

/// rb_reg_preprocess: non-ASCII escapes become raw chars of `enc` (or UTF-8 for \u), comments
/// become "(?#)", and the encoding the pattern is fixed to, if any, is found.
fn preprocess(src: &[u8], enc: u8, ext: bool) -> Result<(Vec<u8>, Option<u8>), String> {
    let mut st = Unesc { out: Vec::with_capacity(src.len()), fixed: None, has_prop: false };
    unescape0(src, &mut 0, enc, &mut st, ext, false)?;
    if st.has_prop && st.fixed.is_none() { st.fixed = Some(enc); }
    Ok((st.out, st.fixed))
}

struct Unesc { out: Vec<u8>, fixed: Option<u8>, has_prop: bool }

/// unescape_nonascii0; `recurse`: one (?x:..) group whose ')' ends the scan.
fn unescape0(src: &[u8], pp: &mut usize, enc: u8, st: &mut Unesc, ext: bool, recurse: bool) -> Result<(), String> {
    use crate::builtins::encoding as en;
    let mut out = std::mem::take(&mut st.out);
    let mut fixed = st.fixed;
    let mut has_prop = st.has_prop;
    let set = |fixed: &mut Option<u8>, e: u8, msg: &str| -> Result<(), String> {
        match *fixed { None => { *fixed = Some(e); Ok(()) } Some(f) if f == e => Ok(()), _ => Err(msg.to_string()) }
    };
    let mut p = *pp;
    let (mut ext, mut in_class, mut parens) = (ext, 0u32, 1u32);
    while p < src.len() {
        let b = src[p];
        if b >= 0x80 {
            let l = en::precise_len(enc, src, p).ok_or("invalid multibyte character")?;
            out.extend_from_slice(&src[p..p + l]);
            p += l;
            set(&mut fixed, enc, "non ASCII character in different encoding regexp")?;
            continue;
        }
        if b != b'\\' {
            p += 1;
            match b {
                b'#' if ext && in_class == 0 => {
                    while p < src.len() {
                        let c = src[p];
                        p += 1;
                        if c == b'\n' { break; }
                        if c >= 0x80 && fixed.is_none() && enc == ENC_UTF8 { fixed = Some(enc); }
                    }
                    out.extend_from_slice(b"(?#)");
                    continue;
                }
                b'[' => in_class += 1,
                b']' => in_class = in_class.saturating_sub(1),
                b')' if in_class == 0 && recurse => {
                    parens -= 1;
                    if parens == 0 { out.push(b); break; }
                }
                b'(' if in_class == 0 && p + 1 < src.len() && src[p] == b'?' => {
                    if src[p + 1] == b'#' {
                        // the content is skipped unchecked except for its characters
                        let mut q = p;
                        let mut closed = false;
                        while q < src.len() {
                            let c = src[q];
                            q += 1;
                            if c == b')' { closed = true; break; }
                            if c == b'\\' || c >= 0x80 {
                                if c >= 0x80 {
                                    if fixed.is_none() && enc == ENC_UTF8 { fixed = Some(enc); }
                                    q -= 1;
                                }
                                q += en::precise_len(enc, src, q).ok_or("invalid multibyte character")?;
                            }
                        }
                        if closed { out.extend_from_slice(b"(?#)"); p = q; } else { out.push(b); }
                        continue;
                    }
                    if recurse { parens += 1; }
                    // (?x: / (?-x: that flips extended mode: scanned with it up to the ')'
                    let (mut invert, mut local, mut sub) = (false, 0i8, false);
                    for &c in &src[p + 1..] {
                        match c {
                            b'x' => local = if invert { -1 } else { 1 },
                            b'-' => invert = true,
                            b'i' | b'm' | b'a' | b'd' | b'u' => {}
                            b':' | b')' => {
                                if local == 0 || (local == 1) == ext { break; }
                                if c == b')' { ext = local == 1; break; }
                                out.push(b);
                                let mut st = Unesc { out: std::mem::take(&mut out), fixed, has_prop };
                                unescape0(src, &mut p, enc, &mut st, local == 1, true)?;
                                (out, fixed, has_prop) = (st.out, st.fixed, st.has_prop);
                                sub = true;
                                break;
                            }
                            _ => break,
                        }
                    }
                    if sub { continue; }
                }
                b'(' if in_class == 0 && recurse => parens += 1,
                _ => {}
            }
            out.push(b);
            continue;
        }
        if p + 1 >= src.len() { return Err("too short escape sequence".into()); }
        let c = src[p + 1];
        if c >= 0x80 {
            let l = en::precise_len(enc, src, p + 1).ok_or("invalid multibyte character")?;
            out.extend_from_slice(&src[p..p + 1 + l]);
            p += 1 + l;
            set(&mut fixed, enc, "non ASCII character in different encoding regexp")?;
            continue;
        }
        let octal_high = matches!(c, b'1'..=b'7') && scan_oct(&src[p + 1..], 3).0 > 0o177;
        match c {
            b'0' | b'x' | b'c' | b'C' | b'M' => {}
            b'1'..=b'7' if octal_high => {}
            b'u' => {
                p += 2;
                if p >= src.len() { return Err("too short escape sequence".into()); }
                if src[p] == b'{' {
                    p += 1;
                    while p < src.len() && src[p].is_ascii_whitespace() { p += 1; }
                    let mut any = false;
                    loop {
                        let (v, n) = scan_hex(&src[p..], usize::MAX);
                        if n == 0 { break; }
                        if n > 6 { return Err("invalid Unicode range".into()); }
                        p += n;
                        append_utf8(v, &mut out, &mut fixed)?;
                        any = true;
                        while p < src.len() && src[p].is_ascii_whitespace() { p += 1; }
                    }
                    if !any || p >= src.len() || src[p] != b'}' { return Err("invalid Unicode list".into()); }
                    p += 1;
                } else {
                    let (v, n) = scan_hex(&src[p..], 4);
                    if n < 4 { return Err("invalid Unicode escape".into()); }
                    p += 4;
                    append_utf8(v, &mut out, &mut fixed)?;
                }
                continue;
            }
            b'p' | b'P' => { if fixed.is_none() { has_prop = true; } out.extend_from_slice(&src[p..p + 2]); p += 2; continue; }
            _ => { out.extend_from_slice(&src[p..p + 2]); p += 2; continue; }
        }
        // \x \0 \c \C \M and high octal: one char of `enc` from escaped bytes
        if enc == ENC_ASCII {
            let st = p;
            read_escaped_byte(src, &mut p)?;
            out.extend_from_slice(&src[st..p]);
            continue;
        }
        let mut ch = vec![read_escaped_byte(src, &mut p)?];
        while ch.len() < en::max_len(enc) && en::needmore(enc, &ch) { ch.push(read_escaped_byte(src, &mut p)?); }
        if en::precise_len(enc, &ch, 0) != Some(ch.len()) { return Err("invalid multibyte escape".into()); }
        if ch.len() > 1 || ch[0] >= 0x80 {
            out.extend_from_slice(&ch);
            set(&mut fixed, enc, "escaped non ASCII character in different encoding regexp")?;
        } else {
            out.extend_from_slice(format!("\\x{:02X}", ch[0]).as_bytes());
        }
    }
    *pp = p;
    *st = Unesc { out, fixed, has_prop };
    Ok(())
}

fn scan_oct(b: &[u8], max: usize) -> (u32, usize) {
    let mut v = 0;
    let mut n = 0;
    while n < max && n < b.len() && (b'0'..=b'7').contains(&b[n]) { v = v * 8 + (b[n] - b'0') as u32; n += 1; }
    (v, n)
}
fn scan_hex(b: &[u8], max: usize) -> (u32, usize) {
    let mut v: u32 = 0;
    let mut n = 0;
    while n < max && n < b.len() && b[n].is_ascii_hexdigit() { v = v.saturating_mul(16).saturating_add((b[n] as char).to_digit(16).unwrap()); n += 1; }
    (v, n)
}
fn append_utf8(v: u32, out: &mut Vec<u8>, fixed: &mut Option<u8>) -> Result<(), String> {
    if v > 0x10ffff || (0xd800..0xe000).contains(&v) { return Err("invalid Unicode range".into()); }
    if v < 0x80 { out.extend_from_slice(format!("\\x{:02X}", v).as_bytes()); return Ok(()); }
    out.extend_from_slice(char::from_u32(v).unwrap().encode_utf8(&mut [0; 4]).as_bytes());
    match *fixed { None => { *fixed = Some(ENC_UTF8); Ok(()) } Some(ENC_UTF8) => Ok(()), _ => Err("UTF-8 character in non UTF-8 regexp".into()) }
}

/// read_escaped_byte: one byte from \xHH, \OOO, \M-x, \C-x, \cx (and their nestings) at src[*p].
fn read_escaped_byte(src: &[u8], p: &mut usize) -> Result<u8, String> {
    if *p >= src.len() || src[*p] != b'\\' { return Err("too short escaped multibyte character".into()); }
    *p += 1;
    let (mut meta, mut ctrl) = (false, false);
    let code: u32 = loop {
        if *p >= src.len() { return Err("too short escape sequence".into()); }
        let c = src[*p];
        *p += 1;
        match c {
            b'\\' => break b'\\' as u32,
            b'n' => break 10,
            b't' => break 9,
            b'r' => break 13,
            b'f' => break 12,
            b'v' => break 11,
            b'a' => break 7,
            b'e' => break 0x1b,
            b'0'..=b'7' => { *p -= 1; let (v, n) = scan_oct(&src[*p..], 3); *p += n; break v; }
            b'x' => { let (v, n) = scan_hex(&src[*p..], 2); if n == 0 { return Err("invalid hex escape".into()); } *p += n; break v; }
            b'M' => {
                if meta { return Err("duplicate meta escape".into()); }
                meta = true;
                if *p + 1 < src.len() && src[*p] == b'-' && src[*p + 1] < 0x80 {
                    *p += 1;
                    if src[*p] == b'\\' { *p += 1; continue; }
                    *p += 1;
                    break src[*p - 1] as u32;
                }
                return Err("too short meta escape".into());
            }
            b'C' | b'c' => {
                if c == b'C' { if *p >= src.len() || src[*p] != b'-' { return Err("too short control escape".into()); } *p += 1; }
                if ctrl { return Err("duplicate control escape".into()); }
                ctrl = true;
                if *p < src.len() && src[*p] < 0x80 {
                    if src[*p] == b'\\' { *p += 1; continue; }
                    *p += 1;
                    break src[*p - 1] as u32;
                }
                return Err("too short control escape".into());
            }
            _ => return Err("unexpected escape sequence".into()),
        }
    };
    if code > 0xff { return Err("invalid escape code".into()); }
    let mut code = code as u8;
    if ctrl { code &= 0x1f; }
    if meta { code |= 0x80; }
    Ok(code)
}

pub fn rregexp(c: &Ctx, v: Value) -> Option<&RRegexp> { c.data::<RRegexp>(v) }
pub fn rregexp_mut(c: &mut Ctx, v: Value) -> Option<&mut RRegexp> { c.data_mut::<RRegexp>(v) }
pub fn rmatch(c: &Ctx, v: Value) -> Option<&RMatch> { c.data::<RMatch>(v) }
pub fn rmatch_mut(c: &mut Ctx, v: Value) -> Option<&mut RMatch> { c.data_mut::<RMatch>(v) }
pub fn is_regexp(c: &Ctx, v: Value) -> bool { rregexp(c, v).is_some() }

/// ArgumentError for a String a regexp cannot search (broken encoding).
pub fn check_searchable(c: &mut Ctx, s: Value) -> Result<(), Esc> {
    let bytes = c.str_bytes(s);
    let enc = c.str_enc(s);
    if !crate::string::is_ascii(bytes) && !crate::builtins::encoding::is_valid(enc, bytes) {
        return arg_err(c, format!("invalid byte sequence in {}", crate::builtins::encoding::enc_name(enc)));
    }
    Ok(())
}

enum Prep { Ok(Enc), Broken(u8), Uninit, Incompat(u8, u8), Warn(Enc, u8) }

/// rb_reg_prepare_enc without the raising: `r` against a string of `senc` / `bytes` (`valid`:
/// the caller has checked the string).  Without a coderange cache the string is scanned only
/// when the answer depends on it being 7-bit: an ASCII-only (non-fixed) pattern reads a 7-bit
/// string the same in its own encoding and in the string's.
#[inline(always)]
fn prep(r: &RRegexp, senc: u8, bytes: &[u8], valid: bool) -> Prep {
    use crate::builtins::encoding as en;
    if !valid && !is_ascii(bytes) && !en::is_valid(senc, bytes) { return Prep::Broken(senc); }
    // the common case: an ASCII-only pattern (not fixed, not /n), or the string's own encoding
    if r.opts & (RE_OPT_FIXED | RE_OPT_ENC_NONE) == 0 && r.re.is_some() && (r.enc == senc || en::ascii_compat(senc)) {
        return Prep::Ok(if senc == ENC_UTF8 { Enc::UTF8 } else { engine_enc(senc) });
    }
    prep_rest(r, senc, bytes)
}

#[inline(never)]
fn prep_rest(r: &RRegexp, senc: u8, bytes: &[u8]) -> Prep {
    use crate::builtins::encoding as en;
    if r.re.is_none() { return Prep::Uninit; }
    let renc = r.enc;
    if renc == senc { return Prep::Ok(engine_enc(senc)); }
    if !en::ascii_compat(senc) { return Prep::Incompat(renc, senc); }
    if r.fixed() {
        if !en::ascii_compat(renc) || !is_ascii(bytes) { return Prep::Incompat(renc, senc); }
        return Prep::Ok(engine_enc(renc));
    }
    if r.opts & RE_OPT_ENC_NONE != 0 && senc != ENC_BINARY && !is_ascii(bytes) { return Prep::Warn(engine_enc(senc), senc); }
    Prep::Ok(engine_enc(senc))
}

#[inline]
fn prep_raise(c: &mut Ctx, p: Prep) -> Result<Enc, Esc> {
    match p {
        Prep::Ok(e) => Ok(e),
        p => prep_slow(c, p),
    }
}

#[cold]
#[inline(never)]
fn prep_slow(c: &mut Ctx, p: Prep) -> Result<Enc, Esc> {
    use crate::builtins::encoding as en;
    match p {
        Prep::Ok(e) => Ok(e),
        Prep::Broken(senc) => arg_err(c, format!("invalid byte sequence in {}", en::enc_name(senc))),
        Prep::Uninit => type_err(c, "uninitialized Regexp"),
        Prep::Incompat(renc, senc) => Err(reg_enc_error(c, renc, senc)),
        Prep::Warn(e, senc) => { reg_warn(c, &format!("historical binary regexp match /.../n against {} string", en::enc_name(senc)))?; Ok(e) }
    }
}

/// rb_reg_prepare_enc: how to read `s` when matching `re` against it, or the error CRuby raises.
pub fn prepare(c: &mut Ctx, re: Value, s: Value) -> Result<Enc, Esc> {
    let p = match rregexp(c, re) { Some(r) => { let st = c.rstring(s); prep(r, st.enc, &st.b, false) } None => Prep::Ok(Enc::UTF8) };
    prep_raise(c, p)
}

/// prepare for a string the caller has already checked for a broken encoding.
pub fn prepare_valid(c: &mut Ctx, re: Value, s: Value) -> Result<Enc, Esc> {
    let p = match rregexp(c, re) { Some(r) => { let st = c.rstring(s); prep(r, st.enc, &st.b, true) } None => Prep::Ok(Enc::UTF8) };
    prep_raise(c, p)
}

/// prepare on a string the caller holds (`senc` / `bytes`); Err needs prep_fail.
pub struct PrepErr(Prep);
#[inline]
pub fn prep_r(r: &RRegexp, senc: u8, bytes: &[u8]) -> Result<Enc, PrepErr> {
    match prep(r, senc, bytes, false) { Prep::Ok(e) => Ok(e), p => Err(PrepErr(p)) }
}
pub fn prep_fail(c: &mut Ctx, e: PrepErr) -> Result<Enc, Esc> { prep_slow(c, e.0) }

fn reg_enc_error(c: &mut Ctx, renc: u8, senc: u8) -> Esc {
    use crate::builtins::encoding::enc_name_msg;
    raise_msg(c, c.vm().c_compat_error.get(), &format!("incompatible encoding regexp match ({} regexp with {} string)", enc_name_msg(renc), enc_name_msg(senc)))
}

/// rb_warn: "file:line: warning: msg" at the Ruby caller, unless $VERBOSE is nil.
pub fn reg_warn(c: &mut Ctx, msg: &str) -> Result<(), Esc> {
    if c.vm().verbose.get() == NIL { return Ok(()); }
    let loc = backtrace_lines(c, 0).first().map(|l| l.split(":in ").next().unwrap_or("").to_string()).unwrap_or_default();
    crate::builtins::kernel::warning_write(c, format!("{}: warning: {}\n", loc, msg).as_bytes())
}

/// The engine's parse-time warnings of the compile just done (nested repeat operators).
pub fn emit_parse_warnings(c: &mut Ctx) -> Result<(), Esc> {
    for w in crate::regex_engine::take_warnings() { reg_warn(c, &w)?; }
    Ok(())
}

/// Regexp.timeout (f64 bits; NO_TIMEOUT = unset).  Enforced by search / match? (not by the
/// raw searches of gsub, scan, split).
pub static TIMEOUT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(NO_TIMEOUT);
pub const NO_TIMEOUT: u64 = u64::MAX;

/// When a match of `r` must give up: its own timeout, else Regexp.timeout.
fn deadline(r: &RRegexp) -> Option<std::time::Instant> {
    let t = r.timeout.or_else(|| { let b = TIMEOUT.load(std::sync::atomic::Ordering::Relaxed); if b == NO_TIMEOUT { None } else { Some(f64::from_bits(b)) } })?;
    Some(std::time::Instant::now() + std::time::Duration::from_secs_f64(t.max(0.0)))
}

/// prepare + search under the regexp's timeout (Regexp::TimeoutError when it passes).
pub fn search_timed(c: &mut Ctx, re: Value, s: Value, start: usize) -> Result<Option<Vec<Option<(usize, usize)>>>, Esc> {
    let st = c.rstring(s);
    let (senc, bytes) = (st.enc, &st.b[..]);
    let Some(r) = rregexp(c, re) else { return Ok(None) };
    let r = match prep(r, senc, bytes, false) {
        Prep::Ok(e) => {
            let Some(rx) = r.re.as_ref() else { return Ok(None) };
            if start > bytes.len() { return Ok(None); }
            engine_run(rx, bytes, start, e, deadline(r), false)
        }
        p => {
            let e = prep_raise(c, p)?;
            let (r, bytes) = (rregexp(c, re).unwrap(), c.str_bytes(s));
            let Some(rx) = r.re.as_ref() else { return Ok(None) };
            if start > bytes.len() { return Ok(None); }
            engine_run(rx, bytes, start, e, deadline(r), false)
        }
    };
    match r {
        Ok(g) => Ok(g),
        Err(()) => {
            let k = const_get_at(c, c.vm().c_regexp.get(), crate::symbol::intern(b"TimeoutError")).unwrap_or(c.vm().c_regexp_error.get());
            err(c, k, "regexp match timeout")
        }
    }
}

/// Search re in str from byte `start`; returns a MatchData or NIL, and sets $~.
pub fn search(c: &mut Ctx, re: Value, s: Value, start: usize, set_last: bool) -> RV {
    let groups = search_timed(c, re, s, start)?;
    let Some(groups) = groups else { if set_last { crate::builtins::svar::set_last_match(c, NIL); } return Ok(NIL) };
    let m = match_new(c, re, s, groups);
    if set_last { crate::builtins::svar::set_last_match(c, m); }
    Ok(m)
}

/// Raw search without allocating: group spans (subject read as UTF-8 unless the pattern is fixed).
pub fn search_raw(c: &Ctx, re: Value, bytes: &[u8], start: usize) -> Option<Vec<Option<(usize, usize)>>> {
    let r = rregexp(c, re)?;
    if start > bytes.len() { return None; }
    r.re.as_ref()?.search(bytes, start)
}

/// search_raw with the subject read as `e` (from prepare).
pub fn search_raw_e(c: &Ctx, re: Value, bytes: &[u8], start: usize, e: Enc) -> Option<Vec<Option<(usize, usize)>>> {
    let r = rregexp(c, re)?;
    if start > bytes.len() { return None; }
    engine_run(r.re.as_ref()?, bytes, start, e, None, false).unwrap_or(None)
}

/// Anchored match at byte `pos`, the subject read as `e`.
pub fn match_at_e(c: &Ctx, re: Value, bytes: &[u8], pos: usize, e: Enc) -> Option<Vec<Option<(usize, usize)>>> {
    engine_run(rregexp(c, re)?.re.as_ref()?, bytes, pos, e, None, true).unwrap_or(None)
}

/// match_at_e with \G at `gpos` (backward search); wide encodings keep \G at `pos`.
pub fn match_at_gpos_e(c: &Ctx, re: Value, bytes: &[u8], pos: usize, gpos: usize, e: Enc) -> Option<Vec<Option<(usize, usize)>>> {
    if e.0 >= WIDE { return match_at_e(c, re, bytes, pos, e); }
    rregexp(c, re)?.re.as_ref()?.match_at_gpos(bytes, pos, gpos, e)
}

pub fn match_new(c: &mut Ctx, re: Value, s: Value, groups: Vec<Option<(usize, usize)>>) -> Value {
    // CRuby's MatchData keeps a frozen snapshot: a String changed after the match must not move its groups
    let s = if is_frozen(s) { s } else { let d = c.str_dup(s); obj_set_flags(d, FL_FROZEN); d };
    c.alloc_with(c.vm().c_match_data.get(), move || Body::Data(Box::new(RMatch { regexp: re, str: s, groups })))
}

/// The span of group `n` (negative from the end) of MatchData `m`, and its string.
fn group_span(c: &Ctx, m: Value, n: i64) -> Option<(Value, usize, usize)> {
    let md = rmatch(c, m)?;
    let n = if n < 0 { md.groups.len() as i64 + n } else { n };
    if n < 0 { return None; }
    md.groups.get(n as usize).copied().flatten().map(|(a, b)| (md.str, a, b))
}
/// rb_reg_nth_match: a negative n counts from the end but never reaches group 0.
pub fn nth_match(c: &mut Ctx, m: Value, n: i64) -> Value {
    if n < 0 && rmatch(c, m).map_or(true, |md| n + md.groups.len() as i64 <= 0) { return NIL; }
    match group_span(c, m, n) { Some((s, a, b)) => c.str_sub(s, a..b), None => NIL }
}

/// The capture named by a String / Symbol; IndexError for a name the regexp lacks.
pub fn named_match_strict(c: &mut Ctx, m: Value, name: Value) -> RV {
    let nm = if name.is_sym() { crate::symbol::sym_str(name.as_sym()) } else { String::from_utf8_lossy(c.str_bytes(name)).into_owned() };
    let known = rmatch(c, m).and_then(|md| rregexp(c, md.regexp)).and_then(|r| r.re.as_ref()).map_or(false, |r| !r.group_index(&nm).is_empty());
    if !known { return err(c, c.vm().c_index_error.get(), format!("undefined group name reference: {}", nm)); }
    Ok(named_match(c, m, crate::symbol::intern(nm.as_bytes())))
}
pub fn match_pre(c: &mut Ctx, m: Value) -> Value {
    match group_span(c, m, 0) { Some((s, a, _)) => c.str_sub(s, 0..a), None => NIL }
}
pub fn match_post(c: &mut Ctx, m: Value) -> Value {
    match group_span(c, m, 0) { Some((s, _, b)) => { let e = c.str_len(s); c.str_sub(s, b..e) } None => NIL }
}
pub fn match_last_group(c: &mut Ctx, m: Value) -> Value {
    let Some(md) = rmatch(c, m) else { return NIL };
    match (1..md.groups.len()).rev().find(|&i| md.groups[i].is_some()) { Some(i) => nth_match(c, m, i as i64), None => NIL }
}
pub fn named_match(c: &mut Ctx, m: Value, name: Sym) -> Value {
    let nm = crate::symbol::sym_str(name);
    let found = rmatch(c, m).and_then(|md| {
        let r = rregexp(c, md.regexp)?;
        let idxs = r.re.as_ref()?.group_index(&nm);
        idxs.iter().rev().copied().find(|&i| md.groups.get(i).map(|g| g.is_some()).unwrap_or(false))
    });
    match found { Some(i) => nth_match(c, m, i as i64), None => NIL }
}

/// rb_reg_expr_str: the source as inspect / to_s show it (`term` escaped, unprintables as \xHH;
/// with `resenc`, non-ASCII chars of another encoding as \x{..} / \u....).
fn expr_str(out: &mut Vec<u8>, src: &[u8], enc: u8, resenc: Option<u8>, term: u8) {
    use crate::builtins::encoding as en;
    let printable = |c: u8| (0x20..0x7f).contains(&c);
    if !en::ascii_compat(enc) {
        // ASCII chars keep their wide bytes (rb_enc_ascget), others are escaped
        let mut p = 0;
        while p < src.len() {
            let Some(l) = en::precise_len(enc, src, p) else { out.extend_from_slice(format!("\\x{:02X}", src[p]).as_bytes()); p += 1; continue };
            let cp = en::char_code(enc, &src[p..p + l]);
            if cp < 0x80 { out.extend_from_slice(&src[p..p + l]); } else { out.extend_from_slice(format!("\\u{:04X}", cp).as_bytes()); }
            p += l;
        }
        return;
    }
    let mut need_escape = !en::is_valid(enc, src);
    if !need_escape {
        let mut p = 0;
        while p < src.len() {
            let c = src[p];
            if c >= 0x80 {
                if Some(enc) == resenc { p += en::mbclen(enc, src, p); } else { need_escape = true; break; }
            } else if c != term && printable(c) { p += 1; } else { need_escape = true; break; }
        }
    }
    if !need_escape { out.extend_from_slice(src); return; }
    let unicode = en::is_unicode(enc);
    let mut p = 0;
    while p < src.len() {
        let c = src[p];
        if c == b'\\' && p + 1 < src.len() {
            let n = 1 + en::mbclen(enc, src, p + 1);
            out.extend_from_slice(&src[p..(p + n).min(src.len())]);
            p += n;
            continue;
        }
        if c >= 0x80 {
            match en::precise_len(enc, src, p) {
                None => { out.extend_from_slice(format!("\\x{:02X}", c).as_bytes()); p += 1; }
                Some(l) => {
                    if resenc.is_some() {
                        let cp = en::char_code(enc, &src[p..p + l]);
                        let t = if unicode { if cp < 0x10000 { format!("\\u{:04X}", cp) } else { format!("\\u{{{:X}}}", cp) } } else if cp < 0x100 { format!("\\x{:02X}", cp) } else { format!("\\x{{{:X}}}", cp) };
                        out.extend_from_slice(t.as_bytes());
                    } else {
                        out.extend_from_slice(&src[p..p + l]);
                    }
                    p += l;
                }
            }
            continue;
        }
        if c == term { out.push(b'\\'); out.push(c); }
        else if printable(c) || matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') { out.push(c); }
        else { out.extend_from_slice(format!("\\x{:02X}", c).as_bytes()); }
        p += 1;
    }
}

/// Regexp#inspect (rb_reg_desc).
pub fn regexp_inspect(r: &RRegexp) -> Vec<u8> {
    use crate::builtins::encoding as en;
    let resenc = en::default_internal().unwrap_or_else(en::default_external);
    let mut s = vec![b'/'];
    expr_str(&mut s, &r.src, r.enc, Some(resenc), b'/');
    s.push(b'/');
    if r.opts & 4 != 0 { s.push(b'm'); }
    if r.opts & 1 != 0 { s.push(b'i'); }
    if r.opts & 2 != 0 { s.push(b'x'); }
    if r.opts & RE_OPT_ENC_NONE != 0 { s.push(b'n'); }
    s
}

fn opt_of(c: u8) -> u32 { match c { b'i' => 1, b'x' => 2, b'm' => 4, _ => 0 } }

/// Regexp#to_s (rb_reg_str_with_term): "(?on-off:src)", absorbing a leading (?opts) / (?opts:...)
/// that spans the whole source.
pub fn regexp_to_s(r: &RRegexp) -> Vec<u8> {
    if !crate::builtins::encoding::ascii_compat(r.enc) {
        // built on the UTF-8 form, returned in the regexp's encoding
        let u = wide_to_utf8(&r.src, r.enc).unwrap_or_default();
        return utf8_to_wide(&to_s_of(&u, r.opts, ENC_UTF8), r.enc);
    }
    to_s_of(&r.src, r.opts, r.enc)
}

/// UTF-8 bytes in an ASCII-incompatible Unicode encoding.
pub fn utf8_to_wide(u: &[u8], enc: u8) -> Vec<u8> {
    let mut out = vec![];
    for ch in String::from_utf8_lossy(u).chars() {
        if let Some(b) = crate::builtins::encoding::from_unicode(enc, ch as u32) { out.extend_from_slice(&b); }
    }
    out
}

/// The UTF-8 form of `b` in `enc` (as is when ASCII-compatible).
pub fn to_utf8_form(b: &[u8], enc: u8) -> Vec<u8> {
    use crate::builtins::encoding as en;
    if en::ascii_compat(enc) && en::kind(enc) != en::EK::Cesu8 { b.to_vec() } else { wide_to_utf8(b, enc).unwrap_or_default() }
}

fn to_s_of(src: &[u8], opts: u32, enc: u8) -> Vec<u8> {
    let mut opts = opts & 7;
    let r_opts = opts;
    let mut s = 0;
    let mut e = src.len();
    loop {
        if e - s < 4 || src[s] != b'(' || src[s + 1] != b'?' { break; }
        let mut o = opts;
        let mut p = s + 2;
        while p < e && opt_of(src[p]) != 0 { o |= opt_of(src[p]); p += 1; }
        if p + 1 < e && src[p] == b'-' {
            p += 1;
            while p < e && opt_of(src[p]) != 0 { o &= !opt_of(src[p]); p += 1; }
        }
        if p < e && src[p] == b')' { opts = o; s = p + 1; continue; }
        if p < e && src[p] == b':' && src[e - 1] == b')' && p + 1 <= e - 1
            && Regex::new_enc(&src[p + 1..e - 1], o, engine_enc(enc)).is_ok() {
            opts = o;
            s = p + 1;
            e -= 1;
        } else {
            opts = r_opts;
            s = 0;
            e = src.len();
        }
        break;
    }
    let mut out = b"(?".to_vec();
    for (b, ch) in [(4, b'm'), (1, b'i'), (2, b'x')] { if opts & b != 0 { out.push(ch); } }
    if opts & 7 != 7 {
        out.push(b'-');
        for (b, ch) in [(4, b'm'), (1, b'i'), (2, b'x')] { if opts & b == 0 { out.push(ch); } }
    }
    out.push(b':');
    expr_str(&mut out, &src[s..e], enc, None, b'/');
    out.push(b')');
    out
}

/// Escape regexp metacharacters (Regexp.escape).
pub fn escape(b: &[u8]) -> Vec<u8> {
    let mut out = vec![];
    for &x in b {
        match x {
            b'.' | b'*' | b'?' | b'+' | b'^' | b'$' | b'|' | b'(' | b')' | b'[' | b']' | b'{' | b'}' | b'\\' | b'/' | b'-' => { out.push(b'\\'); out.push(x); }
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\t' => out.extend_from_slice(b"\\t"),
            b'\r' => out.extend_from_slice(b"\\r"),
            0x0c => out.extend_from_slice(b"\\f"),
            0x0b => out.extend_from_slice(b"\\v"),
            b' ' => out.extend_from_slice(b"\\ "),
            b'#' => out.extend_from_slice(b"\\#"),
            _ => out.push(x),
        }
    }
    out
}

/// rb_reg_regcomp: a String used as a pattern as it is (String#match, #match?).
pub fn regcomp(c: &mut Ctx, s: Value) -> RV {
    let b = c.str_bytes(s).to_vec();
    let e = c.str_enc(s);
    regexp_new_enc(c, &b, e, 0)
}

/// Pattern argument for String methods: Regexp as is, String → literal regexp.
pub fn to_regexp(c: &mut Ctx, pat: Value) -> RV {
    if is_regexp(c, pat) { return Ok(pat); }
    let s = expect_str(c, pat)?;
    let b = escape(c.str_bytes(s));
    let e = c.str_enc(s);
    regexp_new_enc(c, &b, e, 0)
}

pub fn match_groups_array(c: &mut Ctx, m: Value) -> Value {
    let n = rmatch(c, m).map_or(0, |md| md.groups.len());
    let a = c.ary_new();
    for i in 1..n { let x = nth_match(c, m, i as i64); c.ary_push(a, x); }
    a
}
