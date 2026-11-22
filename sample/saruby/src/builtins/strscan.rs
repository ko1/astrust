// StringScanner (strscan), native.
// The scanner state is read as a `Copy` snapshot (`Sc`) and written through `sc_mut`, so no
// reference into it or into the scanned String lives across a call that takes the context.
#![forbid(unsafe_code)]
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

pub struct RScan { str: Value, pos: usize, prev: usize, groups: Option<Vec<Option<(usize, usize)>>>, re: Value, fixed: bool, base: usize }
impl RData for RScan {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.str); m.visit(&mut self.re); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
/// What most methods need of the scanner, by value.
#[derive(Clone, Copy)]
struct Sc { str: Value, pos: usize, prev: usize, re: Value, fixed: bool, ngroups: Option<usize>, m0: (usize, usize) }
/// Snapshot of the scanner; ArgumentError unless `v` is an initialized StringScanner.
fn sc(c: &mut Ctx, v: Value) -> Result<Sc, Esc> {
    match c.data::<RScan>(v) {
        Some(s) if s.str != UNDEF => {
            let m0 = s.groups.as_ref().and_then(|g| g[0]).unwrap_or((0, 0));
            Ok(Sc { str: s.str, pos: s.pos, prev: s.prev, re: s.re, fixed: s.fixed, ngroups: s.groups.as_ref().map(|g| g.len()), m0 })
        }
        _ => arg_err(c, "uninitialized StringScanner object"),
    }
}
/// The scanner for writing; `v` has passed `sc`.
fn sc_mut(c: &mut Ctx, v: Value) -> &mut RScan { c.data_mut::<RScan>(v).expect("a StringScanner") }
/// Record a match of bytes a..b found at the scan pointer `a`, and move past it.
fn set_match1(c: &mut Ctx, v: Value, a: usize, b: usize) {
    let s = sc_mut(c, v);
    s.prev = a; s.base = a; s.groups = Some(vec![Some((a, b))]); s.re = NIL; s.pos = b;
}
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if c.data::<RScan>(v).is_none() { return Ok(None); }
    let k = real_class_of(v);
    alloc_data(c, k)
}
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let k = c.vm().c_string_scanner.get();
    if k != NIL && crate::builtins::inherits(klass, k) { return Ok(Some(c.alloc(klass, Body::Data(Box::new(RScan { str: UNDEF, pos: 0, prev: 0, groups: None, re: NIL, fixed: false, base: 0 }))))); }
    Ok(None)
}
fn ss_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, opts) = match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    check_argc(c, pos, 1, 2)?;
    let sv = expect_str(c, pos[0])?;
    let fixed = match opts { Some(h) => crate::hash::hash_get_opt(c, h, sym_value("fixed_anchor"))?.map(|v| v.truthy()).unwrap_or(false), None => false };
    *sc_mut(c, s) = RScan { str: sv, pos: 0, prev: 0, groups: None, re: NIL, fixed, base: 0 };
    Ok(NIL)
}
fn ss_init_copy(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    sc(c, a[0])?;
    let o = c.data::<RScan>(a[0]).expect("a StringScanner");
    // strscan_init_copy copies the registers but not the regexp (so names are not copied)
    let n = RScan { str: o.str, pos: o.pos, prev: o.prev, groups: o.groups.clone(), re: NIL, fixed: o.fixed, base: o.base };
    let old_re = sc_mut(c, s).re;
    *sc_mut(c, s) = RScan { re: old_re, ..n };
    Ok(s)
}
/// Bytes a..b of `str` (clamped to its length) as a String of its encoding.
fn sub(c: &mut Ctx, str: Value, a: usize, b: usize) -> Value { let n = c.str_len(str); let (a, b) = (a.min(n), b.min(n)); c.str_sub(str, a..b.max(a)) }

/// Core matcher: `head` anchors at the scan pointer, otherwise search forward.
fn do_scan(c: &mut Ctx, slf: Value, pat: Value, advance: bool, want_str: bool, head: bool) -> RV {
    let pat = if crate::regexp::is_regexp(c, pat) { pat } else { expect_str(c, pat)? };
    let mut s = sc(c, slf)?;
    let len = c.str_len(s.str);
    if s.pos > len { s.pos = len; sc_mut(c, slf).pos = len; }
    let is_re = crate::regexp::is_regexp(c, pat);
    let md = if is_re { crate::regexp::prepare(c, pat, s.str)? } else { crate::regex_engine::Enc::UTF8 };
    let src = c.rstring(s.str);
    let bytes: &[u8] = &src.b;
    if is_re && !is_ascii(bytes) && !crate::builtins::encoding::is_valid(src.enc, bytes) {
        let enc = src.enc;
        return arg_err(c, format!("invalid byte sequence in {}", crate::builtins::encoding::enc_name(enc)));
    }
    // the string stays borrowed while matching: nothing below takes the context until `found` is known
    let (found, base): (Option<Vec<Option<(usize, usize)>>>, usize) = if let Some(r) = crate::regexp::rregexp(c, pat) {
        let re = r.re.as_ref().unwrap();
        let (hay, off) = if s.fixed { (bytes, 0) } else { (&bytes[s.pos..], s.pos) };
        let at = s.pos - off;
        let g = if head { re.match_at_enc(hay, at, md) } else { re.search_enc(hay, at, md) };
        (g.map(|g| g.into_iter().map(|x| x.map(|(a, b)| (a + off, b + off))).collect()), off)
    } else {
        let pb = c.str_bytes(pat);
        let rest = &bytes[s.pos..];
        let f = if head { if rest.starts_with(pb) { Some(vec![Some((s.pos, s.pos + pb.len()))]) } else { None } }
            else if pb.is_empty() { Some(vec![Some((s.pos, s.pos))]) }
            else { rest.windows(pb.len()).position(|w| w == pb).map(|i| vec![Some((s.pos + i, s.pos + i + pb.len()))]) };
        (f, s.pos)
    };
    let st = sc_mut(c, slf);
    st.base = base;
    st.re = if is_re { pat } else { NIL };
    let Some(g) = found else { st.groups = None; return Ok(NIL) };
    let (_, e) = g[0].unwrap();
    let start = s.pos;
    st.prev = start;
    st.groups = Some(g);
    if advance { st.pos = e; }
    if want_str { Ok(sub(c, s.str, start, e)) } else { Ok(Value::fix((e - start) as i64)) }
}
fn check_pat(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    if crate::regexp::is_regexp(c, v) || is_string(v) || respond_to(v, intern(b"to_str"), true) { return Ok(()); }
    type_err(c, format!("wrong argument type {} (expected Regexp)", class_name_for_err(v)))
}
macro_rules! scan_fn { ($name:ident, $adv:expr, $str:expr, $head:expr) => { fn $name(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { check_pat(c, a[0])?; do_scan(c, s, a[0], $adv, $str, $head) } }; }
scan_fn!(ss_scan, true, true, true);
scan_fn!(ss_match_p, false, false, true);
scan_fn!(ss_skip, true, false, true);
scan_fn!(ss_check, false, true, true);
scan_fn!(ss_scan_until, true, true, false);
scan_fn!(ss_skip_until, true, false, false);
scan_fn!(ss_exist_p, false, false, false);
scan_fn!(ss_check_until, false, true, false);
fn ss_scan_full(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { check_pat(c, a[0])?; do_scan(c, s, a[0], a[1].truthy(), a[2].truthy(), true) }
fn ss_search_full(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { check_pat(c, a[0])?; do_scan(c, s, a[0], a[1].truthy(), a[2].truthy(), false) }
fn ss_getch(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    let src = c.rstring(s.str);
    if s.pos >= src.b.len() { sc_mut(c, slf).groups = None; return Ok(NIL); }
    let l = enc_char_len(&src.b, s.pos, src.enc).clamp(1, src.b.len() - s.pos);
    set_match1(c, slf, s.pos, s.pos + l);
    Ok(sub(c, s.str, s.pos, s.pos + l))
}
fn ss_get_byte(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    if s.pos >= c.str_len(s.str) { sc_mut(c, slf).groups = None; return Ok(NIL); }
    set_match1(c, slf, s.pos, s.pos + 1);
    Ok(sub(c, s.str, s.pos, s.pos + 1))
}
fn ss_scan_byte(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    let Some(&x) = c.str_bytes(s.str).get(s.pos) else { sc_mut(c, slf).groups = None; return Ok(NIL) };
    set_match1(c, slf, s.pos, s.pos + 1);
    Ok(Value::fix(x as i64))
}
fn ss_peek_byte(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    Ok(c.str_bytes(s.str).get(s.pos).map(|&b| Value::fix(b as i64)).unwrap_or(NIL))
}
fn ss_peek(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative string size (or size too big)"); }
    let s = sc(c, slf)?;
    Ok(sub(c, s.str, s.pos, s.pos.saturating_add(n as usize)))
}
fn ss_eos(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; Ok(Value::bool(s.pos >= c.str_len(s.str))) }
fn ss_rest_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; Ok(Value::bool(s.pos < c.str_len(s.str))) }
fn ss_rest(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; Ok(sub(c, s.str, s.pos, usize::MAX)) }
fn ss_rest_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; Ok(Value::fix(c.str_len(s.str).saturating_sub(s.pos) as i64)) }
fn ss_string(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(sc(c, slf)?.str) }
fn ss_set_string(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let sv = expect_str(c, a[0])?;
    sc(c, slf)?;
    let s = sc_mut(c, slf);
    s.str = sv; s.pos = 0; s.groups = None;
    Ok(a[0])
}
fn ss_concat(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let sv = expect_str(c, a[0])?;
    let s = sc(c, slf)?;
    str_append_str(c, s.str, sv)?;
    Ok(slf)
}
fn ss_pos(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(sc(c, slf)?.pos as i64)) }
fn ss_charpos(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    let src = c.rstring(s.str);
    Ok(Value::fix(enc_strlen(&src.b[..s.pos.min(src.b.len())], src.enc) as i64))
}
fn ss_set_pos(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = num_to_long(c, a[0])?;
    let s = sc(c, slf)?;
    let len = c.str_len(s.str) as i64;
    let p = if n < 0 { n + len } else { n };
    if p < 0 || p > len { return err(c, c.vm().c_range_error.get(), "index out of range"); }
    sc_mut(c, slf).pos = p as usize;
    Ok(a[0])
}
fn ss_reset(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { sc(c, slf)?; let s = sc_mut(c, slf); s.pos = 0; s.groups = None; Ok(slf) }
fn ss_terminate(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; let n = c.str_len(s.str); let s = sc_mut(c, slf); s.pos = n; s.groups = None; Ok(slf) }
fn ss_unscan(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    if s.ngroups.is_none() { return err(c, c.vm().c_string_scanner_error.get(), "unscan failed: previous match record not exist"); }
    let st = sc_mut(c, slf);
    st.pos = s.prev; st.groups = None;
    Ok(slf)
}
fn ss_bol(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    let b = c.str_bytes(s.str);
    if s.pos > b.len() { return Ok(NIL); }
    Ok(Value::bool(s.pos == 0 || b[s.pos - 1] == b'\n'))
}
/// Group i of the last match of scanner `slf` (which has passed `sc`).
fn group(c: &mut Ctx, slf: Value, i: i64) -> Value {
    let s = c.data::<RScan>(slf).expect("a StringScanner");
    let Some(g) = &s.groups else { return NIL };
    let i = if i < 0 { i + g.len() as i64 } else { i };
    if i < 0 || i as usize >= g.len() { return NIL; }
    match g[i as usize] { Some((a, b)) => { let str = s.str; sub(c, str, a, b) } None => NIL }
}
fn ss_matched(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { sc(c, slf)?; Ok(group(c, slf, 0)) }
fn ss_matched_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(sc(c, slf)?.ngroups.is_some())) }
fn ss_matched_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    Ok(if s.ngroups.is_some() { Value::fix((s.m0.1 - s.m0.0) as i64) } else { NIL })
}
fn ss_pre_match(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    Ok(if s.ngroups.is_some() { sub(c, s.str, 0, s.m0.0) } else { NIL })
}
fn ss_post_match(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    Ok(if s.ngroups.is_some() { sub(c, s.str, s.m0.1, usize::MAX) } else { NIL })
}
fn name_index(c: &mut Ctx, re: Value, name: &str) -> Result<i64, Esc> {
    if let Some(r) = crate::regexp::rregexp(c, re) { if let Some(&i) = r.re.as_ref().unwrap().group_index(name).last() { return Ok(i as i64); } }
    err(c, c.vm().c_index_error.get(), format!("undefined group name reference: {}", name))
}
fn ss_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    if s.ngroups.is_none() { return Ok(NIL); }
    let i = if a[0].is_sym() || is_string(a[0]) {
        let n = if a[0].is_sym() { crate::symbol::sym_str(a[0].as_sym()) } else { String::from_utf8_lossy(c.str_bytes(a[0])).into_owned() };
        name_index(c, s.re, &n)?
    } else { num_to_long(c, a[0])? };
    Ok(group(c, slf, i))
}
fn ss_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; Ok(match s.ngroups { Some(n) => Value::fix(n as i64), None => NIL }) }
fn ss_captures(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    let Some(n) = s.ngroups else { return Ok(NIL) };
    let out = c.ary_new();
    for i in 1..n { let v = group(c, slf, i as i64); c.ary_push(out, v); }
    Ok(out)
}
fn ss_values_at(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let s = sc(c, slf)?;
    if s.ngroups.is_none() { return Ok(NIL); }
    let out = c.ary_new();
    for &x in a { let v = ss_aref(c, slf, &[x], b)?; c.ary_push(out, v); }
    Ok(out)
}
fn ss_named_captures(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    let h = c.hash_new();
    let names = match crate::regexp::rregexp(c, s.re) { Some(r) => r.re.as_ref().unwrap().names(), None => return Ok(h) };
    for (name, idx) in names {
        let mut v = NIL;
        for &i in &idx { let g = group(c, slf, i as i64); if g != NIL { v = g; } }
        let k = c.str_new(name.as_bytes());
        crate::hash::hash_aset(c, h, k, v)?;
    }
    Ok(h)
}
fn ss_scan_integer(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    // def scan_integer(base: 10) (strscan.rb)
    let kw = kw_arg(a);
    let npos = a.len() - kw.is_some() as usize;
    if npos > 0 { return argc_err(c, npos, 0, 0); }
    let base = match kw { Some(h) => {
        for (k, _) in c.hash_pairs(h) { if k != sym_value("base") { let ks = crate::builtins::kernel::inspect_str(c, k)?; return arg_err(c, format!("unknown keyword: {}", ks)); } }
        crate::hash::hash_get_opt(c, h, sym_value("base"))?.unwrap_or(Value::fix(10))
    } None => Value::fix(10) };
    let base = if base == Value::fix(10) { 10 } else if base == Value::fix(16) { 16 } else {
        let bs = crate::builtins::kernel::inspect_str(c, base)?;
        return arg_err(c, format!("Unsupported integer base: {}, expected 10 or 16", bs));
    };
    let s = sc(c, slf)?;
    sc_mut(c, slf).groups = None;
    let enc = c.str_enc(s.str);
    if !crate::builtins::encoding::ascii_compat(enc) {
        return err(c, c.vm().c_compat_error.get(), format!("ASCII incompatible encoding: {}", crate::builtins::encoding::enc_name(enc)));
    }
    // strscan_scan_base10_integer / strscan_scan_base16_integer
    let b = c.str_bytes(s.str);
    let rest = &b[s.pos.min(b.len())..];
    let n = rest.len();
    if n == 0 { return Ok(NIL); }
    let dig = |x: u8| if base == 10 { x.is_ascii_digit() } else { x.is_ascii_hexdigit() };
    let mut len = 0;
    if rest[0] == b'-' || rest[0] == b'+' { len += 1; }
    if base == 16 && n >= len + 3 && rest[len] == b'0' && rest[len + 1] == b'x' && rest[len + 2].is_ascii_hexdigit() { len += 2; }
    if len >= n || !dig(rest[len]) { return Ok(NIL); }
    while len < n && dig(rest[len]) { len += 1; }
    let text: Vec<u8> = rest[..len].to_vec();
    let (st, en) = (s.pos, s.pos + len);
    set_match1(c, slf, st, en);
    Ok(str_to_inum(&text, base as u32, false).unwrap_or(Value::fix(0)))
}
fn ss_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let Some(s) = c.data::<RScan>(slf) else { return Ok(NIL) };
    if s.str == UNDEF { return Ok(c.str_new(b"#<StringScanner (uninitialized)>")); }
    let (pos, b) = (s.pos, c.str_bytes(s.str));
    if pos >= b.len() { return Ok(c.str_new(b"#<StringScanner fin>")); }
    let after = { let e = (pos + 5).min(b.len()); let mut t = inspect_bytes(&b[pos..e], ENC_BINARY); if e < b.len() { t.insert_str(t.len() - 1, "..."); } t };
    let text = if pos == 0 { format!("#<StringScanner {}/{} @ {}>", pos, b.len(), after) } else {
        let before = { let st = pos.saturating_sub(5); let mut t = inspect_bytes(&b[st..pos], ENC_BINARY); if st > 0 { t.insert_str(1, "..."); } t };
        format!("#<StringScanner {}/{} {} @ {}>", pos, b.len(), before, after)
    };
    Ok(c.str_new(text.as_bytes()))
}
fn ss_fixed_anchor_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(sc(c, slf)?.fixed)) }

/// Run by the first `require "strscan"`.
pub fn init(c: &mut Ctx) {
    let k = define_class("StringScanner", c.vm().c_object.get());
    class_set_alloc(k, AllocKind::None);
    let e = define_class_under(k, "Error", c.vm().c_standard_error.get());
    c.vm().c_string_scanner.set(k);
    c.vm().c_string_scanner_error.set(e);
    let v = c.str_new_frozen(b"3.1.6");
    const_set(k, intern(b"Version"), v);
    let v = c.str_new_frozen(b"$Id$");
    const_set(k, intern(b"Id"), v);
    define_private_cfunc(k, "initialize", ss_initialize, -1);
    define_private_cfunc(k, "initialize_copy", ss_init_copy, 1);
    define_singleton_cfunc(k, "must_C_version", |_c, s, _a, _b| Ok(s), 0);
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(k, n, f, a);
    d("scan", ss_scan, 1); d("match?", ss_match_p, 1); d("skip", ss_skip, 1); d("check", ss_check, 1); d("scan_until", ss_scan_until, 1);
    d("skip_until", ss_skip_until, 1); d("exist?", ss_exist_p, 1); d("check_until", ss_check_until, 1); d("scan_full", ss_scan_full, 3);
    d("search_full", ss_search_full, 3); d("getch", ss_getch, 0); d("get_byte", ss_get_byte, 0); d("getbyte", ss_get_byte, 0); d("scan_byte", ss_scan_byte, 0);
    d("peek_byte", ss_peek_byte, 0); d("peek", ss_peek, 1); d("peep", ss_peek, 1); d("eos?", ss_eos, 0); d("empty?", ss_eos, 0); d("rest?", ss_rest_p, 0);
    d("rest", ss_rest, 0); d("rest_size", ss_rest_size, 0); d("restsize", ss_rest_size, 0); d("string", ss_string, 0); d("string=", ss_set_string, 1);
    d("concat", ss_concat, 1); d("<<", ss_concat, 1); d("pos", ss_pos, 0); d("pointer", ss_pos, 0); d("pos=", ss_set_pos, 1); d("pointer=", ss_set_pos, 1);
    d("charpos", ss_charpos, 0); d("reset", ss_reset, 0); d("terminate", ss_terminate, 0); d("clear", ss_terminate, 0); d("unscan", ss_unscan, 0);
    d("bol?", ss_bol, 0); d("beginning_of_line?", ss_bol, 0); d("matched", ss_matched, 0); d("matched?", ss_matched_p, 0); d("matched_size", ss_matched_size, 0);
    d("pre_match", ss_pre_match, 0); d("post_match", ss_post_match, 0); d("[]", ss_aref, 1); d("size", ss_size, 0); d("captures", ss_captures, 0);
    d("values_at", ss_values_at, -1); d("named_captures", ss_named_captures, 0); d("scan_integer", ss_scan_integer, -1); d("inspect", ss_inspect, 0);
    d("fixed_anchor?", ss_fixed_anchor_p, 0);
}
