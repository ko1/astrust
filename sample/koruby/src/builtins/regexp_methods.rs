// Regexp / MatchData methods.
use crate::array::*;
use crate::builtins::kernel::*;
use crate::regexp::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

fn byte_to_char(sv: Value, off: usize) -> i64 { let b = str_bytes(sv); if rstr(sv).enc != ENC_UTF8 || is_ascii(b) { off as i64 } else { utf8_len(&b[..off]) as i64 } }

fn re_s_new(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 3)?;
    if is_regexp(a[0]) { let r = rregexp(a[0]).unwrap(); return regexp_new(c, &r.src.clone(), r.opts); }
    let src = expect_str(c, a[0])?;
    let mut opts = 0;
    if a.len() > 1 {
        let o = a[1];
        if o.is_fixnum() { opts = o.as_fix() as u32 & 7; }
        else if is_string(o) { for &ch in str_bytes(o) { match ch { b'i' => opts |= 1, b'x' => opts |= 2, b'm' => opts |= 4, _ => return arg_err(c, format!("unknown regexp option: {}", String::from_utf8_lossy(str_bytes(o)))) } } }
        else if o.truthy() { opts = 1; }
    }
    let b = rstr(src).b.clone();
    regexp_new(c, &b, opts)
}
fn re_s_escape(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let v = if a[0].is_sym() { str_new(crate::symbol::sym_bytes(a[0].as_sym())) } else { expect_str(c, a[0])? };
    Ok(str_from_vec(escape(str_bytes(v))))
}
fn re_s_union(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let list: Vec<Value> = if a.len() == 1 && is_array(a[0]) { rary(a[0]).clone() } else { a.to_vec() };
    if list.is_empty() { return regexp_new(c, b"(?!)", 0); }
    let mut src = vec![];
    for (i, &x) in list.iter().enumerate() {
        if i > 0 { src.push(b'|'); }
        if is_regexp(x) { src.extend_from_slice(regexp_to_s(rregexp(x).unwrap()).as_bytes()); } else { let sv = expect_str(c, x)?; src.extend(escape(str_bytes(sv))); }
    }
    regexp_new(c, &src, 0)
}
fn re_match(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if a[0] == NIL { c.last_match = NIL; return Ok(NIL); }
    let sv = if a[0].is_sym() { str_new(crate::symbol::sym_bytes(a[0].as_sym())) } else { expect_str(c, a[0])? };
    let pos = if a.len() > 1 { let p = crate::builtins::numeric_methods::num_to_long(c, a[1])?; let n = str_char_len(sv) as i64; let p = if p < 0 { p + n } else { p }; if p < 0 || p > n { c.last_match = NIL; return Ok(NIL); } char_offsets(sv)[p as usize] } else { 0 };
    let m = search(c, slf, sv, pos, true)?;
    if m != NIL && !blk.is_null() { return yield1(c, blk, m); }
    Ok(m)
}
fn re_match_p(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0] == NIL { return Ok(FALSE); }
    let sv = if a[0].is_sym() { str_new(crate::symbol::sym_bytes(a[0].as_sym())) } else { expect_str(c, a[0])? };
    let pos = if a.len() > 1 { let p = crate::builtins::numeric_methods::num_to_long(c, a[1])?; char_offsets(sv).get(p.max(0) as usize).copied().unwrap_or(usize::MAX) } else { 0 };
    Ok(Value::bool(search_raw(slf, str_bytes(sv), pos).is_some()))
}
fn re_match_op(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0] == NIL { c.last_match = NIL; return Ok(NIL); }
    let sv = if a[0].is_sym() { str_new(crate::symbol::sym_bytes(a[0].as_sym())) } else { expect_str(c, a[0])? };
    let m = search(c, slf, sv, 0, true)?;
    if m == NIL { return Ok(NIL); }
    Ok(Value::fix(byte_to_char(sv, rmatch(m).unwrap().groups[0].unwrap().0)))
}
fn re_eqq(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let v = a[0];
    let sv = if is_string(v) { v } else if v.is_sym() { str_new(crate::symbol::sym_bytes(v.as_sym())) } else { return Ok(FALSE) };
    let m = search(c, slf, sv, 0, true)?;
    Ok(Value::bool(m != NIL))
}
fn re_source(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(str_new_frozen(&rregexp(slf).unwrap().src)) }
fn re_inspect(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(str_new(regexp_source_inspect(rregexp(slf).unwrap()).as_bytes())) }
fn re_to_s(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(str_new(regexp_to_s(rregexp(slf).unwrap()).as_bytes())) }
fn re_options(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix((rregexp(slf).unwrap().opts & 0x27) as i64)) }
fn re_casefold(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(rregexp(slf).unwrap().opts & 1 != 0)) }
fn re_eq(_c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let Some(o) = rregexp(a[0]) else { return Ok(FALSE) };
    let r = rregexp(slf).unwrap();
    Ok(Value::bool(r.src == o.src && (r.opts & 7) == (o.opts & 7)))
}
fn re_hash(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix((crate::astrust::hash_bytes(&rregexp(slf).unwrap().src) >> 2) as i64)) }
fn re_names(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let r = rregexp(slf).unwrap();
    Ok(ary_from_vec(r.re.as_ref().unwrap().names().into_iter().map(|(n, _)| str_new(n.as_bytes())).collect()))
}
fn re_named_captures(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let r = rregexp(slf).unwrap();
    let h = crate::hash::hash_new();
    for (n, idx) in r.re.as_ref().unwrap().names() {
        let a = ary_from_vec(idx.iter().map(|&i| Value::fix(i as i64)).collect());
        crate::hash::hash_set(c, h, str_new(n.as_bytes()), a)?;
    }
    Ok(h)
}
fn re_s_last_match(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if let Some(&i) = a.first() { return Ok(nth_match(c.last_match, i.as_fix())); }
    Ok(c.last_match)
}
fn re_freeze(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { obj(slf).flags |= FL_FROZEN; Ok(slf) }

// MatchData
fn md(v: Value) -> &'static mut RMatch { rmatch(v).unwrap() }
fn group_index(c: &mut Ctx, m: Value, k: Value) -> Result<Option<usize>, Esc> {
    if k.is_sym() || is_string(k) {
        let nm = if k.is_sym() { crate::symbol::sym_str(k.as_sym()) } else { String::from_utf8_lossy(str_bytes(k)).into_owned() };
        let r = rregexp(md(m).regexp);
        let idx = r.map(|r| r.re.as_ref().unwrap().group_index(&nm)).unwrap_or_default();
        if idx.is_empty() { return err(c, vm().c_index_error, format!("undefined group name reference: {}", nm)); }
        for &i in idx.iter().rev() { if md(m).groups.get(i).map(|g| g.is_some()).unwrap_or(false) { return Ok(Some(i)); } }
        return Ok(Some(*idx.last().unwrap()));
    }
    let i = crate::builtins::numeric_methods::num_to_long(c, k)?;
    let n = md(m).groups.len() as i64;
    let i = if i < 0 { i + n } else { i };
    if i < 0 || i >= n { return Ok(None); }
    Ok(Some(i as usize))
}
fn md_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.len() == 2 || is_range(a[0]) { let all = md_to_a(c, slf, &[], std::ptr::null())?; return crate::builtins::array_methods::ary_aref(c, all, a); }
    match group_index(c, slf, a[0])? { Some(i) => Ok(nth_match(slf, i as i64)), None => Ok(NIL) }
}
fn md_to_a(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let n = md(slf).groups.len(); Ok(ary_from_vec((0..n).map(|i| nth_match(slf, i as i64)).collect())) }
fn md_captures(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(match_groups_array(slf)) }
fn md_named_captures(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let h = crate::hash::hash_new();
    let symk = match kw_arg(a) { Some(hh) => crate::hash::hash_get_opt(c, hh, sym_value("symbolize_names"))?.map(|v| v.truthy()).unwrap_or(false), None => false };
    if let Some(r) = rregexp(md(slf).regexp) {
        for (n, idx) in r.re.as_ref().unwrap().names() {
            let mut v = NIL;
            for &i in idx.iter() { let x = nth_match(slf, i as i64); if x != NIL { v = x; } }
            let k = if symk { Value::sym(intern(n.as_bytes())) } else { str_new(n.as_bytes()) };
            crate::hash::hash_set(c, h, k, v)?;
        }
    }
    Ok(h)
}
fn md_names(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { let r = md(slf).regexp; if r == NIL { return Ok(ary_new()); } re_names(c, r, a, b) }
fn md_begin(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let i = group_index(c, slf, a[0])?.ok_or(()).or_else(|_| err(c, vm().c_index_error, "index out of matches"))?;
    Ok(match md(slf).groups[i] { Some((s2, _)) => Value::fix(byte_to_char(md(slf).str, s2)), None => NIL })
}
fn md_end(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let i = group_index(c, slf, a[0])?.ok_or(()).or_else(|_| err(c, vm().c_index_error, "index out of matches"))?;
    Ok(match md(slf).groups[i] { Some((_, e)) => Value::fix(byte_to_char(md(slf).str, e)), None => NIL })
}
fn md_offset(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { let x = md_begin(c, slf, a, b)?; let y = md_end(c, slf, a, b)?; Ok(ary_new_from(&[x, y])) }
fn md_byteoffset(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let i = group_index(c, slf, a[0])?.unwrap_or(0);
    Ok(match md(slf).groups[i] { Some((s2, e)) => ary_new_from(&[Value::fix(s2 as i64), Value::fix(e as i64)]), None => ary_new_from(&[NIL, NIL]) })
}
fn md_pre(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(match_pre(slf)) }
fn md_post(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(match_post(slf)) }
fn md_to_s(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(nth_match(slf, 0)) }
fn md_size(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(md(slf).groups.len() as i64)) }
fn md_string(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let s2 = md(slf).str; let v = str_new_enc(str_bytes(s2), rstr(s2).enc); obj(v).flags |= FL_FROZEN; Ok(v) }
fn md_regexp(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(md(slf).regexp) }
fn md_values_at(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { let all = md_to_a(c, slf, &[], b)?; crate::builtins::array_methods::a_values_at(c, all, a, b) }
fn md_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let mut out = String::from("#<MatchData ");
    out.push_str(&inspect_str(c, nth_match(slf, 0))?);
    let names: Vec<(String, Vec<usize>)> = rregexp(md(slf).regexp).map(|r| r.re.as_ref().unwrap().names()).unwrap_or_default();
    for i in 1..md(slf).groups.len() {
        let label = names.iter().find(|(_, idx)| idx.contains(&i)).map(|(n, _)| n.clone()).unwrap_or(i.to_string());
        out.push(' '); out.push_str(&label); out.push(':');
        out.push_str(&inspect_str(c, nth_match(slf, i as i64))?);
    }
    out.push('>');
    Ok(str_new(out.as_bytes()))
}
fn md_eq(_c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let Some(o) = rmatch(a[0]) else { return Ok(FALSE) };
    let m = md(slf);
    Ok(Value::bool(m.groups == o.groups && str_bytes(m.str) == str_bytes(o.str) && m.regexp == o.regexp))
}
fn md_deconstruct_keys(c: &mut Ctx, slf: Value, _a: &[Value], b: *const Block) -> RV {
    let h = md_named_captures(c, slf, &[], b)?;
    let out = crate::hash::hash_new();
    for (k, v) in crate::hash::hash_pairs(h) { crate::hash::hash_set(c, out, str_to_sym(k), v)?; }
    Ok(out)
}

pub fn dup_data(_c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if let Some(r) = rregexp(v) { let re = crate::regex_engine::Regex::new(&r.src, r.opts & 7).ok(); return Ok(Some(crate::gc::alloc(obj(v).klass, Body::Data(Box::new(RRegexp { src: r.src.clone(), opts: r.opts, re, enc_fixed: r.enc_fixed }))))); }
    if let Some(m) = rmatch(v) { return Ok(Some(match_new(m.regexp, m.str, m.groups.clone()))); }
    Ok(None)
}
pub fn alloc_data(_c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if crate::builtins::inherits(klass, vm().c_regexp) {
        let re = crate::regex_engine::Regex::new(b"", 0).ok();
        return Ok(Some(crate::gc::alloc(klass, Body::Data(Box::new(RRegexp { src: vec![], opts: 0, re, enc_fixed: false })))));
    }
    Ok(None)
}
fn re_initialize(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let r = re_s_new(c, NIL, a, b)?;
    let src = rregexp(r).unwrap();
    let (s2, o) = (src.src.clone(), src.opts);
    let me = rregexp(slf).unwrap();
    me.src = s2; me.opts = o; me.re = crate::regex_engine::Regex::new(&me.src, o & 7).ok();
    Ok(NIL)
}

pub fn init() {
    let vm = vm();
    let r = define_class("Regexp", vm.c_object);
    rclass(r).alloc = AllocKind::None;
    vm.c_regexp = r;
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
    d("match", re_match, -1); d("match?", re_match_p, -1); d("=~", re_match_op, 1); d("===", re_eqq, 1); d("source", re_source, 0);
    d("inspect", re_inspect, 0); d("to_s", re_to_s, 0); d("options", re_options, 0); d("casefold?", re_casefold, 0); d("==", re_eq, 1);
    d("eql?", re_eq, 1); d("hash", re_hash, 0); d("names", re_names, 0); d("named_captures", re_named_captures, 0); d("freeze", re_freeze, 0);
    let m = define_class("MatchData", vm.c_object);
    rclass(m).alloc = AllocKind::None;
    vm.c_match_data = m;
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(m, n, f, a);
    d("[]", md_aref, -1); d("to_a", md_to_a, 0); d("captures", md_captures, 0); d("deconstruct", md_captures, 0); d("named_captures", md_named_captures, -1);
    d("names", md_names, 0); d("begin", md_begin, 1); d("end", md_end, 1); d("offset", md_offset, 1); d("byteoffset", md_byteoffset, 1);
    d("pre_match", md_pre, 0); d("post_match", md_post, 0); d("to_s", md_to_s, 0); d("size", md_size, 0); d("length", md_size, 0);
    d("string", md_string, 0); d("regexp", md_regexp, 0); d("values_at", md_values_at, -1); d("inspect", md_inspect, 0); d("==", md_eq, 1);
    d("eql?", md_eq, 1); d("deconstruct_keys", md_deconstruct_keys, 1);
    let _ = s::to_s;
}
