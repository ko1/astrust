// Regexp / MatchData methods.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{obj_klass, obj_set_flags};
use crate::object::*;
use crate::builtins::kernel::*;
use crate::regexp::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

fn byte_to_char(c: &Ctx, sv: Value, off: usize) -> i64 { let b = c.str_bytes(sv); if c.str_enc(sv) != ENC_UTF8 || is_ascii(b) { off as i64 } else { utf8_len(&b[..off]) as i64 } }
fn rx(c: &Ctx, v: Value) -> &RRegexp { rregexp(c, v).expect("a Regexp") }
fn md(c: &Ctx, v: Value) -> &RMatch { rmatch(c, v).expect("a MatchData") }

fn re_s_new(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 3)?;
    if is_regexp(c, a[0]) { let r = rx(c, a[0]); let (src, opts) = (r.src.clone(), r.opts); return regexp_new(c, &src, opts); }
    let src = expect_str(c, a[0])?;
    let mut opts = 0;
    if a.len() > 1 {
        let o = a[1];
        if o.is_fixnum() { opts = o.as_fix() as u32 & 7; }
        else if is_string(o) {
            for &ch in c.str_bytes(o) { match ch { b'i' => opts |= 1, b'x' => opts |= 2, b'm' => opts |= 4, _ => { opts = u32::MAX; break; } } }
            if opts == u32::MAX { let t = String::from_utf8_lossy(c.str_bytes(o)).into_owned(); return arg_err(c, format!("unknown regexp option: {}", t)); }
        }
        else if o.truthy() { opts = 1; }
    }
    let b = c.str_bytes(src).to_vec();
    regexp_new(c, &b, opts)
}
fn re_s_escape(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    use crate::builtins::encoding as en;
    let v = if a[0].is_sym() { let b = crate::symbol::sym_bytes(a[0].as_sym()); c.str_new_enc(b, if crate::string::is_ascii(b) { ENC_ASCII } else { ENC_UTF8 }) } else { expect_str(c, a[0])? };
    let enc = c.str_enc(v);
    if !en::ascii_compat(enc) {
        // escape the UTF-8 form, then convert back to the wide encoding
        let src = c.str_bytes(v).to_vec();
        let u = en::transcode(c, &src, enc, ENC_UTF8, &en::TcOpts::default())?;
        let back = en::transcode(c, &escape(&u), ENC_UTF8, enc, &en::TcOpts::default())?;
        return Ok(c.str_from_vec_enc(back, enc));
    }
    Ok(c.str_from_vec_enc(escape(c.str_bytes(v)), enc))
}
fn re_s_union(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let list: Vec<Value> = if a.len() == 1 && is_array(a[0]) { c.ary(a[0]).to_vec() } else { a.to_vec() };
    if list.is_empty() { return regexp_new(c, b"(?!)", 0); }
    let keep = c.ary_new_from(&list);   // to_str may run Ruby code; list is not scanned
    let mut src = vec![];
    for (i, &x) in list.iter().enumerate() {
        if i > 0 { src.push(b'|'); }
        if is_regexp(c, x) { src.extend_from_slice(regexp_to_s(rx(c, x)).as_bytes()); } else { let sv = expect_str(c, x)?; src.extend(escape(c.str_bytes(sv))); }
    }
    std::hint::black_box(keep);
    regexp_new(c, &src, 0)
}
fn re_match(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if a[0] == NIL { c.last_match = NIL; return Ok(NIL); }
    let sv = if a[0].is_sym() { c.str_new(crate::symbol::sym_bytes(a[0].as_sym())) } else { expect_str(c, a[0])? };
    let pos = if a.len() > 1 { let p = crate::builtins::numeric_methods::num_to_long(c, a[1])?; let n = c.str_char_len(sv) as i64; let p = if p < 0 { p + n } else { p }; if p < 0 || p > n { c.last_match = NIL; return Ok(NIL); } c.char_offsets(sv)[p as usize] } else { 0 };
    let m = search(c, slf, sv, pos, true)?;
    if m != NIL && !blk.is_null() { return yield1(c, blk, m); }
    Ok(m)
}
fn re_match_p(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0] == NIL { return Ok(FALSE); }
    let sv = if a[0].is_sym() { c.str_new(crate::symbol::sym_bytes(a[0].as_sym())) } else { expect_str(c, a[0])? };
    let pos = if a.len() > 1 { let p = crate::builtins::numeric_methods::num_to_long(c, a[1])?; c.char_offsets(sv).get(p.max(0) as usize).copied().unwrap_or(usize::MAX) } else { 0 };
    Ok(Value::bool(search_raw(c, slf, c.str_bytes(sv), pos).is_some()))
}
fn re_match_op(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0] == NIL { c.last_match = NIL; return Ok(NIL); }
    let sv = if a[0].is_sym() { c.str_new(crate::symbol::sym_bytes(a[0].as_sym())) } else { expect_str(c, a[0])? };
    let m = search(c, slf, sv, 0, true)?;
    if m == NIL { return Ok(NIL); }
    let off = md(c, m).groups[0].map_or(0, |g| g.0);
    Ok(Value::fix(byte_to_char(c, sv, off)))
}
fn re_eqq(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let v = a[0];
    let sv = if is_string(v) { v } else if v.is_sym() { c.str_new(crate::symbol::sym_bytes(v.as_sym())) } else { return Ok(FALSE) };
    let m = search(c, slf, sv, 0, true)?;
    Ok(Value::bool(m != NIL))
}
fn re_enc(r: &RRegexp) -> u8 {
    if crate::string::is_ascii(&r.src) { if r.enc_fixed { ENC_UTF8 } else { ENC_ASCII } } else if r.opts & crate::regexp::RE_OPT_ENC_NONE != 0 { ENC_BINARY } else { ENC_UTF8 }
}
fn re_source(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let r = rx(c, slf);
    let e = if crate::string::is_ascii(&r.src) { ENC_ASCII } else { re_enc(r) };
    let src = r.src.clone();
    let s = c.str_new_enc(&src, e); obj_set_flags(s, FL_FROZEN); Ok(s)
}
fn re_fixed_encoding(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let r = rx(c, slf); Ok(Value::bool(r.enc_fixed || (!crate::string::is_ascii(&r.src) && r.opts & crate::regexp::RE_OPT_ENC_NONE == 0))) }
fn re_encoding(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let e = re_enc(rx(c, slf)); Ok(crate::builtins::encoding::encoding_value(c, e)) }
fn re_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let t = regexp_source_inspect(rx(c, slf)); Ok(c.str_new(t.as_bytes())) }
fn re_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let t = regexp_to_s(rx(c, slf)); Ok(c.str_new(t.as_bytes())) }
fn re_options(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix((rx(c, slf).opts & 0x27) as i64)) }
fn re_casefold(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(rx(c, slf).opts & 1 != 0)) }
fn re_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let Some(o) = rregexp(c, a[0]) else { return Ok(FALSE) };
    let r = rx(c, slf);
    Ok(Value::bool(r.src == o.src && (r.opts & 7) == (o.opts & 7)))
}
fn re_hash(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix((crate::astrust::hash_bytes(&rx(c, slf).src) >> 2) as i64)) }
fn re_names(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let names = rx(c, slf).re.as_ref().map(|r| r.names()).unwrap_or_default();
    let out = c.ary_new_capa(names.len());
    for (n, _) in names { let s = c.str_new(n.as_bytes()); c.ary_push(out, s); }
    Ok(out)
}
fn re_named_captures(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let names = rx(c, slf).re.as_ref().map(|r| r.names()).unwrap_or_default();
    let h = c.hash_new();
    for (n, idx) in names {
        let a = c.ary_from_vec(idx.iter().map(|&i| Value::fix(i as i64)).collect());
        let k = c.str_new(n.as_bytes());
        crate::hash::hash_set(c, h, k, a)?;
    }
    Ok(h)
}
fn re_s_last_match(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if let Some(&i) = a.first() {
        let lm = c.last_match;
        if lm == NIL { return Ok(NIL); }
        if i.is_sym() || is_string(i) { return md_aref(c, lm, &[i], std::ptr::null()); }
        let i = crate::builtins::numeric_methods::num_to_long(c, i)?;
        return Ok(nth_match(c, lm, i));
    }
    Ok(c.last_match)
}
fn re_freeze(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { obj_set_flags(slf, FL_FROZEN); Ok(slf) }

// MatchData
fn group_index(c: &mut Ctx, m: Value, k: Value) -> Result<Option<usize>, Esc> {
    if k.is_sym() || is_string(k) {
        let nm = if k.is_sym() { crate::symbol::sym_str(k.as_sym()) } else { String::from_utf8_lossy(c.str_bytes(k)).into_owned() };
        let r = rregexp(c, md(c, m).regexp);
        let idx = r.and_then(|r| r.re.as_ref()).map(|r| r.group_index(&nm)).unwrap_or_default();
        if idx.is_empty() { return err(c, c.vm().c_index_error, format!("undefined group name reference: {}", nm)); }
        for &i in idx.iter().rev() { if md(c, m).groups.get(i).map(|g| g.is_some()).unwrap_or(false) { return Ok(Some(i)); } }
        return Ok(Some(*idx.last().unwrap()));
    }
    let i = crate::builtins::numeric_methods::num_to_long(c, k)?;
    let n = md(c, m).groups.len() as i64;
    let i = if i < 0 { i + n } else { i };
    if i < 0 || i >= n { return Ok(None); }
    Ok(Some(i as usize))
}
/// match_backref_number + bounds check (begin/end/offset/byte*): no negative indices.
fn group_index_strict(c: &mut Ctx, m: Value, k: Value) -> Result<usize, Esc> {
    if k.is_sym() || is_string(k) { return Ok(group_index(c, m, k)?.unwrap_or(0)); }
    let i = crate::builtins::numeric_methods::num_to_long(c, k)?;
    if i < 0 || i >= md(c, m).groups.len() as i64 { return err(c, c.vm().c_index_error, format!("index {} out of matches", i)); }
    Ok(i as usize)
}
fn md_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.len() == 2 || is_range(a[0]) { let all = md_to_a(c, slf, &[], std::ptr::null())?; return crate::builtins::array_methods::ary_aref(c, all, a); }
    match group_index(c, slf, a[0])? { Some(i) => Ok(nth_match(c, slf, i as i64)), None => Ok(NIL) }
}
fn md_to_a(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let n = md(c, slf).groups.len();
    let out = c.ary_new_capa(n);
    for i in 0..n { let x = nth_match(c, slf, i as i64); c.ary_push(out, x); }
    Ok(out)
}
fn md_captures(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(match_groups_array(c, slf)) }
fn md_named_captures(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let h = c.hash_new();
    let symk = match kw_arg(a) { Some(hh) => crate::hash::hash_get_opt(c, hh, sym_value("symbolize_names"))?.map(|v| v.truthy()).unwrap_or(false), None => false };
    let names = rregexp(c, md(c, slf).regexp).and_then(|r| r.re.as_ref()).map(|r| r.names()).unwrap_or_default();
    for (n, idx) in names {
        let mut v = NIL;
        for &i in idx.iter() { let x = nth_match(c, slf, i as i64); if x != NIL { v = x; } }
        let k = if symk { Value::sym(intern(n.as_bytes())) } else { c.str_new(n.as_bytes()) };
        crate::hash::hash_set(c, h, k, v)?;
    }
    Ok(h)
}
fn md_names(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { let r = md(c, slf).regexp; if r == NIL { return Ok(c.ary_new()); } re_names(c, r, a, b) }
/// Character offset of an end (0: begin, 1: end) of group a[0].
fn md_edge(c: &mut Ctx, slf: Value, a: &[Value], end: bool) -> RV {
    let i = group_index_strict(c, slf, a[0])?;
    let m = md(c, slf);
    Ok(match m.groups[i] { Some((s2, e)) => Value::fix(byte_to_char(c, m.str, if end { e } else { s2 })), None => NIL })
}
fn md_begin(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { md_edge(c, slf, a, false) }
fn md_end(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { md_edge(c, slf, a, true) }
fn md_offset(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { let x = md_begin(c, slf, a, b)?; let y = md_end(c, slf, a, b)?; Ok(c.ary_new_from(&[x, y])) }
fn md_byteoffset(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let i = group_index_strict(c, slf, a[0])?;
    let pair = match md(c, slf).groups.get(i).copied().flatten() { Some((s2, e)) => [Value::fix(s2 as i64), Value::fix(e as i64)], None => [NIL, NIL] };
    Ok(c.ary_new_from(&pair))
}
/// MatchData#bytebegin / #byteend (3.4)
fn md_bytebegin(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let i = group_index_strict(c, slf, a[0])?; Ok(md(c, slf).groups.get(i).copied().flatten().map_or(NIL, |(s2, _)| Value::fix(s2 as i64))) }
fn md_byteend(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let i = group_index_strict(c, slf, a[0])?; Ok(md(c, slf).groups.get(i).copied().flatten().map_or(NIL, |(_, e)| Value::fix(e as i64))) }
fn md_pre(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(match_pre(c, slf)) }
fn md_post(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(match_post(c, slf)) }
fn md_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(nth_match(c, slf, 0)) }
fn md_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(md(c, slf).groups.len() as i64)) }
fn md_string(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s2 = md(c, slf).str; let v = c.str_copy(s2); obj_set_flags(v, FL_FROZEN); Ok(v) }
fn md_regexp(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(md(c, slf).regexp) }
fn md_values_at(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { let all = md_to_a(c, slf, &[], b)?; crate::builtins::array_methods::a_values_at(c, all, a, b) }
fn md_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let mut out = String::from("#<MatchData ");
    let m0 = nth_match(c, slf, 0);
    out.push_str(&inspect_str(c, m0)?);
    let names: Vec<(String, Vec<usize>)> = rregexp(c, md(c, slf).regexp).and_then(|r| r.re.as_ref()).map(|r| r.names()).unwrap_or_default();
    let n = md(c, slf).groups.len();
    for i in 1..n {
        let label = names.iter().find(|(_, idx)| idx.contains(&i)).map(|(n, _)| n.clone()).unwrap_or(i.to_string());
        out.push(' '); out.push_str(&label); out.push(':');
        let g = nth_match(c, slf, i as i64);
        out.push_str(&inspect_str(c, g)?);
    }
    out.push('>');
    Ok(c.str_new(out.as_bytes()))
}
fn md_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let Some(o) = rmatch(c, a[0]) else { return Ok(FALSE) };
    let m = md(c, slf);
    Ok(Value::bool(m.groups == o.groups && c.str_bytes(m.str) == c.str_bytes(o.str) && m.regexp == o.regexp))
}
fn md_deconstruct_keys(c: &mut Ctx, slf: Value, _a: &[Value], b: *const Block) -> RV {
    let h = md_named_captures(c, slf, &[], b)?;
    let out = c.hash_new();
    for (k, v) in c.hash_pairs(h) { let ks = c.str_to_sym(k); crate::hash::hash_set(c, out, ks, v)?; }
    Ok(out)
}

pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if let Some(r) = rregexp(c, v) {
        let (src, opts, enc_fixed) = (r.src.clone(), r.opts, r.enc_fixed);
        let re = crate::regex_engine::Regex::new(&src, opts & 7).ok();
        return Ok(Some(c.alloc(obj_klass(v), Body::Data(Box::new(RRegexp { src, opts, re, enc_fixed })))));
    }
    if let Some(m) = rmatch(c, v) { let (re, s, g) = (m.regexp, m.str, m.groups.clone()); return Ok(Some(match_new(c, re, s, g))); }
    Ok(None)
}
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if crate::builtins::inherits(klass, c.vm().c_regexp) {
        let re = crate::regex_engine::Regex::new(b"", 0).ok();
        return Ok(Some(c.alloc(klass, Body::Data(Box::new(RRegexp { src: vec![], opts: 0, re, enc_fixed: false })))));
    }
    Ok(None)
}
fn re_initialize(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let r = re_s_new(c, NIL, a, b)?;
    let src = rx(c, r);
    let (s2, o) = (src.src.clone(), src.opts);
    let re = crate::regex_engine::Regex::new(&s2, o & 7).ok();
    let Some(me) = rregexp_mut(c, slf) else { return type_err(c, "wrong argument type (expected Regexp)") };
    me.src = s2; me.opts = o; me.re = re;
    Ok(NIL)
}

/// Boot: Regexp and MatchData; returns (Regexp, MatchData).
pub fn init(vmr: &Vm) -> (Value, Value) {
    let r = define_class("Regexp", vmr.c_object);
    class_set_alloc(r, AllocKind::None);
    const_set(r, intern(b"IGNORECASE"), Value::fix(1));
    const_set(r, intern(b"EXTENDED"), Value::fix(2));
    const_set(r, intern(b"MULTILINE"), Value::fix(4));
    define_singleton_cfunc(r, "escape", re_s_escape, 1);
    define_singleton_cfunc(r, "quote", re_s_escape, 1);
    define_singleton_cfunc(r, "union", re_s_union, -1);
    define_singleton_cfunc(r, "last_match", re_s_last_match, -1);
    define_singleton_cfunc(r, "compile", re_s_new, -1);
    define_private_cfunc(r, "initialize", re_initialize, -1);
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(r, n, f, a);
    d("match", re_match, -1); d("match?", re_match_p, -1); d("=~", re_match_op, 1); d("===", re_eqq, 1); d("source", re_source, 0); d("encoding", re_encoding, 0); d("fixed_encoding?", re_fixed_encoding, 0);
    d("inspect", re_inspect, 0); d("to_s", re_to_s, 0); d("options", re_options, 0); d("casefold?", re_casefold, 0); d("==", re_eq, 1);
    d("eql?", re_eq, 1); d("hash", re_hash, 0); d("names", re_names, 0); d("named_captures", re_named_captures, 0); d("freeze", re_freeze, 0);
    let m = define_class("MatchData", vmr.c_object);
    class_set_alloc(m, AllocKind::None);
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(m, n, f, a);
    d("[]", md_aref, -1); d("to_a", md_to_a, 0); d("captures", md_captures, 0); d("deconstruct", md_captures, 0); d("named_captures", md_named_captures, -1);
    d("names", md_names, 0); d("begin", md_begin, 1); d("end", md_end, 1); d("offset", md_offset, 1); d("byteoffset", md_byteoffset, 1); d("bytebegin", md_bytebegin, 1); d("byteend", md_byteend, 1);
    d("pre_match", md_pre, 0); d("post_match", md_post, 0); d("to_s", md_to_s, 0); d("size", md_size, 0); d("length", md_size, 0);
    d("string", md_string, 0); d("regexp", md_regexp, 0); d("values_at", md_values_at, -1); d("inspect", md_inspect, 0); d("==", md_eq, 1);
    d("eql?", md_eq, 1); d("deconstruct_keys", md_deconstruct_keys, 1);
    let _ = s::to_s;
    (r, m)
}
