// Regexp / MatchData objects and $~.  The matching engine is regex_engine.rs.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::obj_set_flags;
use crate::object::*;
use crate::regex_engine::{Regex, OPT_I, OPT_X, OPT_M};
use crate::string::*;
use crate::value::*;
use crate::vm::*;

pub struct RRegexp { pub src: Vec<u8>, pub opts: u32, pub re: Option<Regex>, pub enc_fixed: bool }
impl RData for RRegexp {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

pub struct RMatch { pub regexp: Value, pub str: Value, pub groups: Vec<Option<(usize, usize)>> }
impl RData for RMatch {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.regexp); m.visit(&mut self.str); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

pub const RE_OPT_ENC_NONE: u32 = 32;

pub fn regexp_new(c: &mut Ctx, src: &[u8], opts: u32) -> RV {
    let ropts = opts & (OPT_I | OPT_X | OPT_M);
    match Regex::new(src, ropts) {
        Ok(re) => { let src = src.to_vec(); Ok(c.alloc_with(c.vm().c_regexp.get(), move || Body::Data(Box::new(RRegexp { src, opts, re: Some(re), enc_fixed: false })))) }
        Err(e) => err(c, c.vm().c_regexp_error.get(), format!("{}: /{}/", e, String::from_utf8_lossy(src))),
    }
}

pub fn rregexp(c: &Ctx, v: Value) -> Option<&RRegexp> { c.data::<RRegexp>(v) }
pub fn rregexp_mut(c: &mut Ctx, v: Value) -> Option<&mut RRegexp> { c.data_mut::<RRegexp>(v) }
pub fn rmatch(c: &Ctx, v: Value) -> Option<&RMatch> { c.data::<RMatch>(v) }
pub fn rmatch_mut(c: &mut Ctx, v: Value) -> Option<&mut RMatch> { c.data_mut::<RMatch>(v) }
pub fn is_regexp(c: &Ctx, v: Value) -> bool { rregexp(c, v).is_some() }

/// Search re in str from byte `start`; returns a MatchData or NIL, and sets $~.
/// ArgumentError for a String a regexp cannot search (broken encoding).
pub fn check_searchable(c: &mut Ctx, s: Value) -> Result<(), Esc> {
    let bytes = c.str_bytes(s);
    let enc = c.str_enc(s);
    if !crate::string::is_ascii(bytes) && !crate::builtins::encoding::is_valid(enc, bytes) {
        return arg_err(c, format!("invalid byte sequence in {}", crate::builtins::encoding::enc_name(enc)));
    }
    Ok(())
}
pub fn search(c: &mut Ctx, re: Value, s: Value, start: usize, set_last: bool) -> RV {
    check_searchable(c, s)?;
    let bytes = c.str_bytes(s);
    let groups = if start > bytes.len() { None } else { search_raw(c, re, bytes, start) };
    let Some(groups) = groups else { if set_last { c.last_match = NIL; } return Ok(NIL) };
    let m = match_new(c, re, s, groups);
    if set_last { c.last_match = m; }
    Ok(m)
}

/// Raw search without allocating: group spans.
pub fn search_raw(c: &Ctx, re: Value, bytes: &[u8], start: usize) -> Option<Vec<Option<(usize, usize)>>> {
    let r = rregexp(c, re)?;
    if start > bytes.len() { return None; }
    r.re.as_ref()?.search(bytes, start)
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

pub fn regexp_source_inspect(r: &RRegexp) -> String {
    let mut s = String::from("/");
    let src = String::from_utf8_lossy(&r.src);
    let mut prev_bs = false;
    for ch in src.chars() {
        if ch == '/' && !prev_bs { s.push_str("\\/"); } else if ch == '\n' { s.push_str("\\n"); } else { s.push(ch); }
        prev_bs = ch == '\\' && !prev_bs;
    }
    s.push('/');
    if r.opts & 4 != 0 { s.push('m'); }
    if r.opts & 1 != 0 { s.push('i'); }
    if r.opts & 2 != 0 { s.push('x'); }
    if r.opts & RE_OPT_ENC_NONE != 0 { s.push('n'); }
    s
}

pub fn regexp_to_s(r: &RRegexp) -> String {
    let mut on = String::new(); let mut off = String::from("-");
    if r.opts & 4 != 0 { on.push('m') } else { off.push('m') }
    if r.opts & 1 != 0 { on.push('i') } else { off.push('i') }
    if r.opts & 2 != 0 { on.push('x') } else { off.push('x') }
    let off = if off == "-" { String::new() } else { off };
    format!("(?{}{}:{})", on, off, String::from_utf8_lossy(&r.src))
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

/// Pattern argument for String methods: Regexp as is, String → literal regexp.
pub fn to_regexp(c: &mut Ctx, pat: Value) -> RV {
    if is_regexp(c, pat) { return Ok(pat); }
    let s = expect_str(c, pat)?;
    let b = escape(c.str_bytes(s));
    regexp_new(c, &b, 0)
}

pub fn match_groups_array(c: &mut Ctx, m: Value) -> Value {
    let n = rmatch(c, m).map_or(0, |md| md.groups.len());
    let a = c.ary_new();
    for i in 1..n { let x = nth_match(c, m, i as i64); c.ary_push(a, x); }
    a
}
