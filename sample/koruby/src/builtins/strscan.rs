// StringScanner (strscan), native.
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
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
static mut C_SCAN: Value = NIL;
static mut C_ERR: Value = NIL;
fn sc(c: &mut Ctx, v: Value) -> Result<&'static mut RScan, Esc> {
    match rdata::<RScan>(v) { Some(s) if s.str != UNDEF => Ok(s), _ => arg_err(c, "uninitialized StringScanner object") }
}
pub fn alloc_data(_c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let k = unsafe { C_SCAN };
    if k != NIL && crate::builtins::inherits(klass, k) { return Ok(Some(crate::gc::alloc(klass, Body::Data(Box::new(RScan { str: UNDEF, pos: 0, prev: 0, groups: None, re: NIL, fixed: false, base: 0 }))))); }
    Ok(None)
}
fn ss_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, opts) = match a.last() { Some(&l) if is_hash(l) => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    check_argc(c, pos, 1, 2)?;
    let sv = expect_str(c, pos[0])?;
    let fixed = match opts { Some(h) => crate::hash::hash_get_opt(c, h, sym_value("fixed_anchor"))?.map(|v| v.truthy()).unwrap_or(false), None => false };
    let r = rdata::<RScan>(s).unwrap();
    *r = RScan { str: sv, pos: 0, prev: 0, groups: None, re: NIL, fixed, base: 0 };
    Ok(NIL)
}
fn ss_init_copy(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = sc(c, a[0])?;
    let r = rdata::<RScan>(s).unwrap();
    *r = RScan { str: o.str, pos: o.pos, prev: o.prev, groups: o.groups.clone(), re: o.re, fixed: o.fixed, base: o.base };
    Ok(s)
}
fn sub(s: &RScan, a: usize, b: usize) -> Value { let src = rstr(s.str); let (a, b) = (a.min(src.b.len()), b.min(src.b.len())); str_new_enc(&src.b[a..b.max(a)], src.enc) }

/// Core matcher: `head` anchors at the scan pointer, otherwise search forward.
fn do_scan(c: &mut Ctx, slf: Value, pat: Value, advance: bool, want_str: bool, head: bool) -> RV {
    let pat = if crate::regexp::is_regexp(pat) { pat } else { expect_str(c, pat)? };
    let s = sc(c, slf)?;
    let bytes = str_bytes(s.str);
    if s.pos > bytes.len() { s.pos = bytes.len(); }
    if crate::regexp::is_regexp(pat) && !is_ascii(bytes) && !crate::builtins::encoding::is_valid(str_enc(s.str), bytes) {
        return arg_err(c, format!("invalid byte sequence in {}", crate::builtins::encoding::enc_name(str_enc(s.str))));
    }
    let found: Option<Vec<Option<(usize, usize)>>> = if let Some(r) = crate::regexp::rregexp(pat) {
        let re = r.re.as_ref().unwrap();
        let (hay, off) = if s.fixed { (bytes, 0) } else { (&bytes[s.pos..], s.pos) };
        let at = s.pos - off;
        let g = if head { re.match_at(hay, at) } else { re.search(hay, at) };
        s.base = off;
        g.map(|g| g.into_iter().map(|x| x.map(|(a, b)| (a + off, b + off))).collect())
    } else {
        let p = expect_str(c, pat)?;
        let pb = str_bytes(p);
        let s = sc(c, slf)?;
        s.base = s.pos;
        let rest = &bytes[s.pos..];
        if head { if rest.starts_with(pb) { Some(vec![Some((s.pos, s.pos + pb.len()))]) } else { None } }
        else if pb.is_empty() { Some(vec![Some((s.pos, s.pos))]) }
        else { rest.windows(pb.len()).position(|w| w == pb).map(|i| vec![Some((s.pos + i, s.pos + i + pb.len()))]) }
    };
    let s = sc(c, slf)?;
    s.re = if crate::regexp::is_regexp(pat) { pat } else { NIL };
    let Some(g) = found else { s.groups = None; return Ok(NIL) };
    let (_, e) = g[0].unwrap();
    let start = s.pos;
    s.prev = start;
    s.groups = Some(g);
    if advance { s.pos = e; }
    if want_str { Ok(sub(s, start, e)) } else { Ok(Value::fix((e - start) as i64)) }
}
fn check_pat(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    if crate::regexp::is_regexp(v) || is_string(v) || respond_to(v, intern(b"to_str"), true) { return Ok(()); }
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
    let src = rstr(s.str);
    if s.pos >= src.b.len() { s.groups = None; return Ok(NIL); }
    let l = enc_char_len(&src.b, s.pos, src.enc).clamp(1, src.b.len() - s.pos);
    s.prev = s.pos; s.base = s.pos; s.groups = Some(vec![Some((s.pos, s.pos + l))]); s.re = NIL;
    s.pos += l;
    Ok(sub(s, s.prev, s.pos))
}
fn ss_get_byte(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    if s.pos >= str_bytes(s.str).len() { s.groups = None; return Ok(NIL); }
    s.prev = s.pos; s.base = s.pos; s.groups = Some(vec![Some((s.pos, s.pos + 1))]); s.re = NIL;
    s.pos += 1;
    Ok(sub(s, s.prev, s.pos))
}
fn ss_scan_byte(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    let b = str_bytes(s.str);
    if s.pos >= b.len() { s.groups = None; return Ok(NIL); }
    s.prev = s.pos; s.base = s.pos; s.groups = Some(vec![Some((s.pos, s.pos + 1))]); s.re = NIL;
    s.pos += 1;
    Ok(Value::fix(b[s.pos - 1] as i64))
}
fn ss_peek_byte(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    Ok(str_bytes(s.str).get(s.pos).map(|&b| Value::fix(b as i64)).unwrap_or(NIL))
}
fn ss_peek(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative string size (or size too big)"); }
    let s = sc(c, slf)?;
    Ok(sub(s, s.pos, s.pos.saturating_add(n as usize)))
}
fn ss_eos(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; Ok(Value::bool(s.pos >= str_bytes(s.str).len())) }
fn ss_rest_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; Ok(Value::bool(s.pos < str_bytes(s.str).len())) }
fn ss_rest(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; Ok(sub(s, s.pos, usize::MAX)) }
fn ss_rest_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; Ok(Value::fix(str_bytes(s.str).len().saturating_sub(s.pos) as i64)) }
fn ss_string(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(sc(c, slf)?.str) }
fn ss_set_string(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let sv = expect_str(c, a[0])?;
    let s = sc(c, slf)?;
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
    let src = rstr(s.str);
    Ok(Value::fix(enc_strlen(&src.b[..s.pos.min(src.b.len())], src.enc) as i64))
}
fn ss_set_pos(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = num_to_long(c, a[0])?;
    let s = sc(c, slf)?;
    let len = str_bytes(s.str).len() as i64;
    let p = if n < 0 { n + len } else { n };
    if p < 0 || p > len { return err(c, vm().c_range_error, "index out of range"); }
    s.pos = p as usize;
    Ok(a[0])
}
fn ss_reset(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; s.pos = 0; s.groups = None; Ok(slf) }
fn ss_terminate(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; s.pos = str_bytes(s.str).len(); s.groups = None; Ok(slf) }
fn ss_unscan(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    if s.groups.is_none() { return err(c, unsafe { C_ERR }, "unscan failed: previous match record not exist"); }
    s.pos = s.prev; s.groups = None;
    Ok(slf)
}
fn ss_bol(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    let b = str_bytes(s.str);
    if s.pos > b.len() { return Ok(NIL); }
    Ok(Value::bool(s.pos == 0 || b[s.pos - 1] == b'\n'))
}
fn group(s: &RScan, i: i64) -> Value {
    let Some(g) = &s.groups else { return NIL };
    let i = if i < 0 { i + g.len() as i64 } else { i };
    if i < 0 || i as usize >= g.len() { return NIL; }
    match g[i as usize] { Some((a, b)) => sub(s, a, b), None => NIL }
}
fn ss_matched(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; Ok(group(s, 0)) }
fn ss_matched_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(sc(c, slf)?.groups.is_some())) }
fn ss_matched_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    Ok(match &s.groups { Some(g) => { let (a, b) = g[0].unwrap(); Value::fix((b - a) as i64) } None => NIL })
}
fn ss_pre_match(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    Ok(match &s.groups { Some(g) => { let (a, _) = g[0].unwrap(); sub(s, if s.fixed { 0 } else { 0 }, a) } None => NIL })
}
fn ss_post_match(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    Ok(match &s.groups { Some(g) => { let (_, b) = g[0].unwrap(); sub(s, b, usize::MAX) } None => NIL })
}
fn name_index(c: &mut Ctx, s: &RScan, name: &str) -> Result<i64, Esc> {
    if let Some(r) = crate::regexp::rregexp(s.re) { if let Some(&i) = r.re.as_ref().unwrap().group_index(name).last() { return Ok(i as i64); } }
    err(c, vm().c_index_error, format!("undefined group name reference: {}", name))
}
fn ss_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    if s.groups.is_none() { return Ok(NIL); }
    let i = if a[0].is_sym() || is_string(a[0]) {
        let n = if a[0].is_sym() { crate::symbol::sym_str(a[0].as_sym()) } else { String::from_utf8_lossy(str_bytes(a[0])).into_owned() };
        name_index(c, s, &n)?
    } else { num_to_long(c, a[0])? };
    Ok(group(s, i))
}
fn ss_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s = sc(c, slf)?; Ok(match &s.groups { Some(g) => Value::fix(g.len() as i64), None => NIL }) }
fn ss_captures(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    let Some(n) = s.groups.as_ref().map(|g| g.len()) else { return Ok(NIL) };
    let out = ary_new();
    for i in 1..n { let v = group(s, i as i64); ary_push(out, v); }
    Ok(out)
}
fn ss_values_at(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    if s.groups.is_none() { return Ok(NIL); }
    let out = ary_new();
    for &x in a { let i = num_to_long(c, x)?; let v = group(s, i); ary_push(out, v); }
    Ok(out)
}
fn ss_named_captures(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = sc(c, slf)?;
    let h = crate::hash::hash_new();
    let Some(r) = crate::regexp::rregexp(s.re) else { return Ok(h) };
    for (name, idx) in r.re.as_ref().unwrap().names() {
        let mut v = NIL;
        for &i in &idx { let g = group(s, i as i64); if g != NIL { v = g; } }
        let k = str_new(name.as_bytes());
        crate::hash::hash_aset(c, h, k, v)?;
    }
    Ok(h)
}
fn ss_scan_integer(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let base = match a.last() { Some(&h) if is_hash(h) => match crate::hash::hash_get_opt(c, h, sym_value("base"))? { Some(b) => num_to_long(c, b)?, None => 10 }, _ => 10 };
    if base != 10 && base != 16 { return arg_err(c, format!("Unsupported integer base: {}, expected 10 or 16", base)); }
    let s = sc(c, slf)?;
    let b = str_bytes(s.str);
    let mut i = s.pos;
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') { i += 1; }
    if base == 16 && i + 1 < b.len() && b[i] == b'0' && (b[i + 1] | 0x20) == b'x' { i += 2; }
    let ds = i;
    while i < b.len() && (if base == 10 { b[i].is_ascii_digit() } else { b[i].is_ascii_hexdigit() }) { i += 1; }
    if i == ds { s.groups = None; return Ok(NIL); }
    let text: Vec<u8> = b[s.pos..i].to_vec();
    s.prev = s.pos; s.base = s.pos; s.groups = Some(vec![Some((s.pos, i))]); s.re = NIL; s.pos = i;
    Ok(str_to_inum(&text, base as u32, false).unwrap_or(Value::fix(0)))
}
fn ss_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let Some(s) = rdata::<RScan>(slf) else { return Ok(NIL) };
    if s.str == UNDEF { return Ok(str_new(b"#<StringScanner (uninitialized)>")); }
    let b = str_bytes(s.str);
    if s.pos >= b.len() { return Ok(str_new(b"#<StringScanner fin>")); }
    let after = { let e = (s.pos + 5).min(b.len()); let mut t = inspect_bytes(&b[s.pos..e], ENC_BINARY); if e < b.len() { t.insert_str(t.len() - 1, "..."); } t };
    let _ = c;
    if s.pos == 0 { return Ok(str_new(format!("#<StringScanner {}/{} @ {}>", s.pos, b.len(), after).as_bytes())); }
    let before = { let st = s.pos.saturating_sub(5); let mut t = inspect_bytes(&b[st..s.pos], ENC_BINARY); if st > 0 { t.insert_str(1, "..."); } t };
    Ok(str_new(format!("#<StringScanner {}/{} {} @ {}>", s.pos, b.len(), before, after).as_bytes()))
}
fn ss_fixed_anchor_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(sc(c, slf)?.fixed)) }

pub fn init() {
    let vm = vm();
    let k = define_class("StringScanner", vm.c_object);
    rclass(k).alloc = AllocKind::None;
    unsafe { C_SCAN = k; C_ERR = define_class_under(k, "Error", vm.c_standard_error); }
    const_set(k, intern(b"Version"), str_new_frozen(b"3.1.6"));
    const_set(k, intern(b"Id"), str_new_frozen(b"$Id$"));
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
