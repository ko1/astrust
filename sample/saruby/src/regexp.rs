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

/// How the engine splits a string of `enc` into chars.
pub fn engine_enc(enc: u8) -> Enc {
    use crate::builtins::encoding::{kind, EK};
    match kind(enc) {
        EK::Utf8 | EK::U16LE | EK::U16BE | EK::U32LE | EK::U32BE => Enc::UTF8,
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

/// rb_reg_initialize_str: `src` is a String of encoding `enc`.
pub fn regexp_new_enc(c: &mut Ctx, src: &[u8], enc: u8, opts: u32) -> RV {
    let (src, opts, enc, re) = match regexp_build(src, enc, opts) {
        Ok(x) => x,
        Err(e) => return err(c, c.vm().c_regexp_error.get(), e),
    };
    Ok(c.alloc_with(c.vm().c_regexp.get(), move || Body::Data(Box::new(RRegexp { src, opts, re: Some(re), enc, timeout: None }))))
}

/// Source, options, encoding and compiled pattern, or the RegexpError message.
pub fn regexp_build(src: &[u8], enc: u8, opts: u32) -> Result<(Vec<u8>, u32, u8, Regex), String> {
    let (unesc, opts, enc, mode) = regexp_prep(src, enc, opts)?;
    let re = Regex::new_enc(&unesc, opts & 7, mode).map_err(|e| format!("{}: {}", e, regexp_desc(src, enc, opts)))?;
    Ok((src.to_vec(), opts, enc, re))
}

/// A literal's RegexpError message, if any (compile time: CRuby reports it as a SyntaxError).
pub fn regexp_check(src: &[u8], opts: u32) -> Result<(), String> {
    let e = (opts >> RE_LIT_ENC_SHIFT) & 0xff;
    let enc = if e == 0 { ENC_UTF8 } else { (e - 1) as u8 };
    let (unesc, opts, enc, mode) = regexp_prep(src, enc, opts & RE_OPT_MASK)?;
    Regex::check(&unesc, opts & 7, mode).map_err(|e| format!("{}: {}", e, regexp_desc(src, enc, opts)))
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
    let conv;
    let (body, benc) = if en::ascii_compat(enc) { (src, enc) } else {
        conv = wide_to_utf8(src, enc).ok_or_else(|| desc("invalid multibyte character"))?;
        (&conv[..], ENC_UTF8)
    };
    let (unesc, fixed) = preprocess(body, benc).map_err(|e| desc(&e))?;
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
    let mode = if opts & RE_OPT_FIXED != 0 { engine_enc(enc) } else { Enc::UTF8 };
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

/// rb_reg_preprocess: non-ASCII escapes become raw chars of `enc` (or UTF-8 for \u), and the
/// encoding the pattern is fixed to, if any, is found.
fn preprocess(src: &[u8], enc: u8) -> Result<(Vec<u8>, Option<u8>), String> {
    use crate::builtins::encoding as en;
    let mut out = Vec::with_capacity(src.len());
    let mut fixed: Option<u8> = None;
    let mut has_prop = false;
    let set = |fixed: &mut Option<u8>, e: u8, msg: &str| -> Result<(), String> {
        match *fixed { None => { *fixed = Some(e); Ok(()) } Some(f) if f == e => Ok(()), _ => Err(msg.to_string()) }
    };
    let mut p = 0;
    while p < src.len() {
        let b = src[p];
        if b >= 0x80 {
            let l = en::precise_len(enc, src, p).ok_or("invalid multibyte character")?;
            out.extend_from_slice(&src[p..p + l]);
            p += l;
            set(&mut fixed, enc, "non ASCII character in different encoding regexp")?;
            continue;
        }
        if b != b'\\' { out.push(b); p += 1; continue; }
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
    if has_prop && fixed.is_none() { fixed = Some(enc); }
    Ok((out, fixed))
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

/// rb_reg_prepare_enc: how to read `s` when matching `re` against it, or the error CRuby raises.
pub fn prepare(c: &mut Ctx, re: Value, s: Value) -> Result<Enc, Esc> {
    use crate::builtins::encoding as en;
    let senc = c.str_enc(s);
    let bytes = c.str_bytes(s);
    let seven = en::ascii_compat(senc) && is_ascii(bytes);
    if !seven && !en::is_valid(senc, bytes) {
        return arg_err(c, format!("invalid byte sequence in {}", en::enc_name(senc)));
    }
    let Some(r) = rregexp(c, re) else { return Ok(Enc::UTF8) };
    if r.re.is_none() { return type_err(c, "uninitialized Regexp"); }
    let (renc, fixed, none) = (r.enc, r.fixed(), r.opts & RE_OPT_ENC_NONE != 0);
    if renc == senc { return Ok(engine_enc(senc)); }
    if seven && renc == ENC_ASCII { return Ok(engine_enc(renc)); }
    if !en::ascii_compat(senc) { return Err(reg_enc_error(c, renc, senc)); }
    if fixed {
        if !en::ascii_compat(renc) || !seven { return Err(reg_enc_error(c, renc, senc)); }
        return Ok(engine_enc(renc));
    }
    if none && senc != ENC_BINARY && !seven {
        reg_warn(c, &format!("historical binary regexp match /.../n against {} string", en::enc_name(senc)))?;
    }
    Ok(engine_enc(senc))
}

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

/// Regexp.timeout (f64 bits; NO_TIMEOUT = unset).  Enforced by search / match? (not by the
/// raw searches of gsub, scan, split).
pub static TIMEOUT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(NO_TIMEOUT);
pub const NO_TIMEOUT: u64 = u64::MAX;

/// When a match of `re` must give up: its own timeout, else Regexp.timeout.
fn deadline(c: &Ctx, re: Value) -> Option<std::time::Instant> {
    let own = rregexp(c, re).and_then(|r| r.timeout);
    let t = own.or_else(|| { let b = TIMEOUT.load(std::sync::atomic::Ordering::Relaxed); if b == NO_TIMEOUT { None } else { Some(f64::from_bits(b)) } })?;
    Some(std::time::Instant::now() + std::time::Duration::from_secs_f64(t.max(0.0)))
}

/// search_raw_e under the regexp's timeout: Regexp::TimeoutError when it passes.
pub fn search_timed(c: &mut Ctx, re: Value, s: Value, start: usize, e: Enc) -> Result<Option<Vec<Option<(usize, usize)>>>, Esc> {
    let dl = deadline(c, re);
    let r = {
        let bytes = c.str_bytes(s);
        let Some(rx) = rregexp(c, re).and_then(|r| r.re.as_ref()) else { return Ok(None) };
        if start > bytes.len() { return Ok(None); }
        rx.search_timeout(bytes, start, e, dl)
    };
    match r {
        Ok(g) => Ok(g),
        Err(()) => {
            let k = const_get_at(c.vm().c_regexp.get(), crate::symbol::intern(b"TimeoutError")).unwrap_or(c.vm().c_regexp_error.get());
            err(c, k, "regexp match timeout")
        }
    }
}

/// Search re in str from byte `start`; returns a MatchData or NIL, and sets $~.
pub fn search(c: &mut Ctx, re: Value, s: Value, start: usize, set_last: bool) -> RV {
    let e = prepare(c, re, s)?;
    let groups = search_timed(c, re, s, start, e)?;
    let Some(groups) = groups else { if set_last { c.last_match = NIL; } return Ok(NIL) };
    let m = match_new(c, re, s, groups);
    if set_last { c.last_match = m; }
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
    r.re.as_ref()?.search_enc(bytes, start, e)
}

/// Anchored match at byte `pos`, the subject read as `e`.
pub fn match_at_e(c: &Ctx, re: Value, bytes: &[u8], pos: usize, e: Enc) -> Option<Vec<Option<(usize, usize)>>> {
    rregexp(c, re)?.re.as_ref()?.match_at_enc(bytes, pos, e)
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
pub fn nth_match(c: &mut Ctx, m: Value, n: i64) -> Value {
    match group_span(c, m, n) { Some((s, a, b)) => c.str_sub(s, a..b), None => NIL }
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
    let mut opts = r.opts & 7;
    let src = &r.src[..];
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
            && Regex::new_enc(&src[p + 1..e - 1], o, engine_enc(r.enc)).is_ok() {
            opts = o;
            s = p + 1;
            e -= 1;
        } else {
            opts = r.opts & 7;
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
    expr_str(&mut out, &src[s..e], r.enc, None, b'/');
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
