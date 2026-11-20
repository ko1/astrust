// Regexp / MatchData objects and $~.  The matching engine is regex_engine.rs.
use crate::array::*;
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
        Ok(re) => Ok(crate::gc::alloc(vm().c_regexp, Body::Data(Box::new(RRegexp { src: src.to_vec(), opts, re: Some(re), enc_fixed: false })))),
        Err(e) => err(c, vm().c_regexp_error, format!("{}: /{}/", e, String::from_utf8_lossy(src))),
    }
}

pub fn rregexp(v: Value) -> Option<&'static mut RRegexp> { rdata::<RRegexp>(v) }
pub fn rmatch(v: Value) -> Option<&'static mut RMatch> { rdata::<RMatch>(v) }
pub fn is_regexp(v: Value) -> bool { rregexp(v).is_some() }

/// Search re in str from byte `start`; returns a MatchData or NIL, and sets $~.
pub fn search(c: &mut Ctx, re: Value, s: Value, start: usize, set_last: bool) -> RV {
    let r = rregexp(re).unwrap();
    let bytes = &rstr(s).b;
    let enc = rstr(s).enc;
    if !crate::string::is_ascii(bytes) && !crate::builtins::encoding::is_valid(enc, bytes) {
        return arg_err(c, format!("invalid byte sequence in {}", crate::builtins::encoding::enc_name(enc)));
    }
    if start > bytes.len() { if set_last { c.last_match = NIL; } return Ok(NIL); }
    let groups = match r.re.as_ref().unwrap().search(bytes, start) {
        Some(g) => g,
        None => { if set_last { c.last_match = NIL; } return Ok(NIL); }
    };
    let m = match_new(re, s, groups);
    if set_last { c.last_match = m; }
    Ok(m)
}

/// Raw search without allocating: group spans.
pub fn search_raw(re: Value, bytes: &[u8], start: usize) -> Option<Vec<Option<(usize, usize)>>> {
    let r = rregexp(re)?;
    if start > bytes.len() { return None; }
    r.re.as_ref().unwrap().search(bytes, start)
}

pub fn match_new(re: Value, s: Value, groups: Vec<Option<(usize, usize)>>) -> Value {
    // CRuby's MatchData keeps a frozen snapshot: a String changed after the match must not move its groups
    let s = if is_frozen(s) { s } else { let d = crate::string::str_dup(s); obj(d).flags |= FL_FROZEN; d };
    crate::gc::alloc(vm().c_match_data, Body::Data(Box::new(RMatch { regexp: re, str: s, groups })))
}

pub fn nth_match(m: Value, n: i64) -> Value {
    let Some(md) = rmatch(m) else { return NIL; };
    let n = if n < 0 { md.groups.len() as i64 + n } else { n };
    if n < 0 { return NIL; }
    match md.groups.get(n as usize) {
        Some(Some((a, b))) => { let s = &rstr(md.str).b[*a..*b]; str_new_enc(s, rstr(md.str).enc) }
        _ => NIL,
    }
}
pub fn match_pre(m: Value) -> Value {
    let md = rmatch(m).unwrap();
    let (a, _) = md.groups[0].unwrap();
    str_new_enc(&rstr(md.str).b[..a], rstr(md.str).enc)
}
pub fn match_post(m: Value) -> Value {
    let md = rmatch(m).unwrap();
    let (_, b) = md.groups[0].unwrap();
    str_new_enc(&rstr(md.str).b[b..], rstr(md.str).enc)
}
pub fn match_last_group(m: Value) -> Value {
    let md = rmatch(m).unwrap();
    for i in (1..md.groups.len()).rev() { if md.groups[i].is_some() { return nth_match(m, i as i64); } }
    NIL
}
pub fn named_match(m: Value, name: Sym) -> Value {
    let md = rmatch(m).unwrap();
    let r = rregexp(md.regexp).unwrap();
    let nm = crate::symbol::sym_str(name);
    let idxs = r.re.as_ref().unwrap().group_index(&nm);
    for &i in idxs.iter().rev() { if md.groups.get(i).map(|g| g.is_some()).unwrap_or(false) { return nth_match(m, i as i64); } }
    NIL
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
    if is_regexp(pat) { return Ok(pat); }
    let s = expect_str(c, pat)?;
    let b = escape(&rstr(s).b);
    regexp_new(c, &b, 0)
}

pub fn match_groups_array(m: Value) -> Value {
    let md = rmatch(m).unwrap();
    let a = ary_new();
    for i in 1..md.groups.len() { let x = nth_match(m, i as i64); ary_push(a, x); }
    a
}
