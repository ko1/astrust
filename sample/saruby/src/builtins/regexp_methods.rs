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

fn byte_to_char(c: &Ctx, sv: Value, off: usize) -> i64 {
    use crate::builtins::encoding as en;
    let (b, enc) = (c.str_bytes(sv), c.str_enc(sv));
    if enc == ENC_UTF8 { return if is_ascii(b) { off as i64 } else { utf8_len(&b[..off]) as i64 }; }
    if en::is_single_byte(enc) { return off as i64; }
    let (mut i, mut n) = (0, 0);
    while i < off { i += en::mbclen(enc, b, i); n += 1; }
    n
}
fn rx(c: &Ctx, v: Value) -> &RRegexp { rregexp(c, v).expect("a Regexp") }
fn md(c: &Ctx, v: Value) -> &RMatch { rmatch(c, v).expect("a MatchData") }
/// rb_reg_check: TypeError for Regexp.allocate's uninitialized object.
fn rxc(c: &mut Ctx, v: Value) -> Result<&RRegexp, Esc> {
    if rx(c, v).re.is_none() { return type_err(c, "uninitialized Regexp"); }
    Ok(rx(c, v))
}

/// The pattern argument of Regexp.new / #initialize: (src, encoding, options, timeout).
fn reg_init_args(c: &mut Ctx, a: &[Value]) -> Result<(Vec<u8>, u8, u32, Option<f64>), Esc> {
    let (a, kw) = match kw_arg(c, a) { Some(h) => (&a[..a.len() - 1], Some(h)), None => (a, None) };
    if a.is_empty() || a.len() > 2 { return argc_err(c, a.len(), 1, 2); }
    let mut timeout = None;
    if let Some(h) = kw {
        if let Some(t) = crate::hash::hash_get_opt(c, h, sym_value("timeout"))? { if t != NIL { timeout = Some(crate::builtins::numeric_methods::to_f(c, t)?); } }
    }
    let opt = a.get(1).copied().unwrap_or(NIL);
    if is_regexp(c, a[0]) {
        if opt != NIL { reg_warn(c, "flags ignored")?; }
        let r = rxc(c, a[0])?;
        return Ok((r.src.clone(), r.enc, r.opts & RE_OPT_MASK, timeout.or(r.timeout)));
    }
    let mut opts = 0;
    if opt.is_fixnum() { opts = opt.as_fix() as u32 & RE_OPT_MASK; }
    else if is_string(opt) {
        for &ch in c.str_bytes(opt) { match ch { b'i' => opts |= 1, b'x' => opts |= 2, b'm' => opts |= 4, _ => { opts = u32::MAX; break; } } }
        if opts == u32::MAX { let t = String::from_utf8_lossy(c.str_bytes(opt)).into_owned(); return arg_err(c, format!("unknown regexp option: {}", t)); }
    } else if opt != NIL && opt != FALSE {
        if opt != TRUE && c.vm().verbose.get().truthy() {
            let t = inspect_str(c, opt)?;
            reg_warn(c, &format!("expected true or false as ignorecase: {}", t))?;
        }
        opts = 1;
    }
    let src = expect_str(c, a[0])?;
    Ok((c.str_bytes(src).to_vec(), c.str_enc(src), opts, timeout))
}

fn re_s_compile(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { crate::builtins::object::c_new(c, slf, a, b) }
fn re_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_frozen(c, slf)?;
    if rregexp(c, slf).map_or(false, |r| r.re.is_some()) { return type_err(c, "already initialized regexp"); }
    let (src, enc, opts, timeout) = reg_init_args(c, a)?;
    let (src, opts, enc, re) = match regexp_build(&src, enc, opts) { Ok(x) => x, Err(e) => return Err(crate::regexp::regexp_error(c, &e, enc)) };
    crate::regexp::emit_parse_warnings(c)?;
    let Some(me) = rregexp_mut(c, slf) else { return type_err(c, "wrong argument type (expected Regexp)") };
    me.src = src; me.opts = opts; me.re = Some(re); me.enc = enc; me.timeout = timeout;
    Ok(slf)
}
fn re_s_escape(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
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
    let out = escape(c.str_bytes(v));
    let e = if is_ascii(&out) { ENC_ASCII } else { enc };
    Ok(c.str_from_vec_enc(out, e))
}
/// rb_check_regexp_type: a Regexp, the result of #to_regexp, or nil.
fn check_regexp_type(c: &mut Ctx, v: Value) -> RV {
    if is_regexp(c, v) { return Ok(v); }
    let m = intern(b"to_regexp");
    if !respond_to(c, v, m, true) { return Ok(NIL); }
    let r = funcall(c, v, m, &[], std::ptr::null())?;
    if r != NIL && !is_regexp(c, r) {
        let cls = class_path(c, real_class_of(c, v));
        return type_err(c, format!("can't convert {} into Regexp ({}#to_regexp gives {})", cls, cls, class_path(c, real_class_of(c, r))));
    }
    Ok(r)
}
fn re_s_try_convert(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { check_regexp_type(c, a[0]) }
fn re_s_union(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    use crate::builtins::encoding as en;
    let list: Vec<Value> = if a.len() == 1 && is_array(a[0]) { c.ary(a[0]).to_vec() } else { a.to_vec() };
    let k = c.vm().c_regexp.get();
    if list.is_empty() { let s = c.str_new(b"(?!)"); return crate::builtins::object::c_new(c, k, &[s], Blk::NONE); }
    let keep = c.ary_new_from(&list);   // to_str may run Ruby code; list is not scanned
    if list.len() == 1 {
        let r = check_regexp_type(c, list[0])?;
        if r != NIL { return Ok(r); }
        let q = re_s_escape(c, NIL, &[list[0]], Blk::NONE)?;
        return crate::builtins::object::c_new(c, k, &[q], Blk::NONE);
    }
    let (mut asciionly, mut fixed, mut incompat): (bool, Option<u8>, Option<u8>) = (false, None, None);
    let mut src: Vec<u8> = vec![];
    let mismatch = |c: &mut Ctx, x: u8, y: u8| -> Esc { raise_msg(c, c.vm().c_argument_error.get(), &format!("incompatible encodings: {} and {}", en::enc_name(x), en::enc_name(y))) };
    for (i, &x) in list.iter().enumerate() {
        let r = check_regexp_type(c, x)?;
        let (part, e, kind) = if r != NIL {
            let rr = rxc(c, r)?;
            let e = rr.enc;
            let kind = if !en::ascii_compat(e) { 2 } else if rr.fixed() { 1 } else { 0 };
            (regexp_to_s(rr), e, kind)
        } else {
            let sv = expect_str(c, x)?;
            let e = c.str_enc(sv);
            let kind = if !en::ascii_compat(e) { 2 } else if is_ascii(c.str_bytes(sv)) { 0 } else { 1 };
            let q = re_s_escape(c, NIL, &[sv], Blk::NONE)?;
            (c.str_bytes(q).to_vec(), e, kind)
        };
        match kind {
            2 => match incompat { None => incompat = Some(e), Some(p) if p != e => return Err(mismatch(c, p, e)), _ => {} },
            1 => match fixed { None => fixed = Some(e), Some(p) if p != e => return Err(mismatch(c, p, e)), _ => {} },
            _ => asciionly = true,
        }
        if let Some(w) = incompat {
            if asciionly { return arg_err(c, format!("ASCII incompatible encoding: {}", en::enc_name(w))); }
            if let Some(f) = fixed { return Err(mismatch(c, w, f)); }
        }
        if i > 0 { src.push(b'|'); }
        // joined in UTF-8 when the result is ASCII-incompatible (CRuby appends "|" in its encoding)
        if incompat.is_some() { src.extend_from_slice(&to_utf8_form(&part, e)); } else { src.extend_from_slice(&part); }
    }
    std::hint::black_box(keep);
    let enc = incompat.or(fixed).unwrap_or(ENC_BINARY);
    if incompat.is_some() { src = utf8_to_wide(&src, enc); }
    let s = c.str_from_vec_enc(src, enc);
    crate::builtins::object::c_new(c, k, &[s], Blk::NONE)
}
/// The String a match method searches: a Symbol's name, or the to_str of anything else.
fn subject(c: &mut Ctx, v: Value) -> RV {
    if v.is_sym() { let b = crate::symbol::sym_bytes(v.as_sym()); return Ok(c.str_new_enc(b, if is_ascii(b) { ENC_ASCII } else { ENC_UTF8 })); }
    expect_str(c, v)
}
fn re_match(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if a.is_empty() || a.len() > 2 { return argc_err(c, a.len(), 1, 2); }
    if a[0] == NIL { crate::builtins::svar::set_last_match(c, NIL); return Ok(NIL); }
    let sv = subject(c, a[0])?;
    let pos = if a.len() > 1 { let p = crate::builtins::numeric_methods::num_to_long(c, a[1])?; let n = c.str_char_len(sv) as i64; let p = if p < 0 { p + n } else { p }; if p < 0 || p > n { crate::builtins::svar::set_last_match(c, NIL); return Ok(NIL); } c.char_offsets(sv)[p as usize] } else { 0 };
    let m = search(c, slf, sv, pos, true)?;
    if m != NIL && !blk.is_null() { return yield1(c, blk, m); }
    Ok(m)
}
fn re_match_p(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.is_empty() || a.len() > 2 { return argc_err(c, a.len(), 1, 2); }
    if a[0] == NIL { return Ok(FALSE); }
    let sv = subject(c, a[0])?;
    let pos = if a.len() > 1 { let p = crate::builtins::numeric_methods::num_to_long(c, a[1])?; let n = c.str_char_len(sv) as i64; let p = if p < 0 { p + n } else { p }; if p < 0 || p > n { prepare(c, slf, sv)?; return Ok(FALSE); } c.char_offsets(sv)[p as usize] } else { 0 };
    Ok(Value::bool(search_timed(c, slf, sv, pos)?.is_some()))
}
fn re_match_op(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if a[0] == NIL { crate::builtins::svar::set_last_match(c, NIL); return Ok(NIL); }
    let sv = subject(c, a[0])?;
    let m = search(c, slf, sv, 0, true)?;
    if m == NIL { return Ok(NIL); }
    let off = md(c, m).groups[0].map_or(0, |g| g.0);
    Ok(Value::fix(byte_to_char(c, sv, off)))
}
/// Regexp#~: match against $_.
fn re_tilde(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let line = gvar_get(c, intern(b"$_"))?;
    if !is_string(line) { crate::builtins::svar::set_last_match(c, NIL); return Ok(NIL); }
    re_match_op(c, slf, &[line], Blk::NONE)
}
fn re_eqq(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let v = a[0];
    let sv = if is_string(v) { v } else if v.is_sym() { subject(c, v)? } else if v.is_heap() && respond_to(c, v, s::to_str, true) { expect_str(c, v)? } else { return Ok(FALSE) };
    let m = search(c, slf, sv, 0, true)?;
    Ok(Value::bool(m != NIL))
}
fn re_source(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let r = rxc(c, slf)?;
    let (src, e) = (r.src.clone(), r.enc);
    let s = c.str_new_enc(&src, e); obj_set_flags(s, FL_FROZEN); Ok(s)
}
fn re_fixed_encoding(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(rx(c, slf).fixed())) }
fn re_encoding(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let e = rx(c, slf).enc; Ok(crate::builtins::encoding::encoding_value(c, e)) }
fn re_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if rx(c, slf).re.is_none() { let t = any_to_s_str(c, slf); return Ok(c.str_new(t.as_bytes())); }
    let r = rx(c, slf);
    let (t, e) = (regexp_inspect(r), if crate::builtins::encoding::ascii_compat(r.enc) { r.enc } else { ENC_ASCII });
    Ok(c.str_from_vec_enc(t, e))
}
fn re_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let r = rxc(c, slf)?; let (t, e) = (regexp_to_s(r), r.enc); Ok(c.str_from_vec_enc(t, e)) }
fn re_options(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix((rxc(c, slf)?.opts & RE_OPT_MASK) as i64)) }
fn re_casefold(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(rxc(c, slf)?.opts & 1 != 0)) }
/// rb_reg_equal
pub fn reg_equal(c: &mut Ctx, a: Value, b: Value) -> Result<bool, Esc> {
    if a == b { return Ok(true); }
    if !is_regexp(c, b) { return Ok(false); }
    rxc(c, a)?; rxc(c, b)?;
    let (r, o) = (rx(c, a), rx(c, b));
    Ok(r.fixed() == o.fixed() && (r.opts & 7) == (o.opts & 7) && r.enc == o.enc && r.src == o.src)
}
fn re_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { Ok(Value::bool(reg_equal(c, slf, a[0])?)) }
fn re_hash(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let r = rxc(c, slf)?;
    let h = crate::astrust::hash_bytes(&r.src) ^ (((r.opts & 7) as u64) << 8);
    Ok(Value::fix((h >> 2) as i64))
}
fn re_names(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let names = rxc(c, slf)?.re.as_ref().map(|r| r.names()).unwrap_or_default();
    let out = c.ary_new_capa(names.len());
    for (n, _) in names { let s = c.str_new(n.as_bytes()); c.ary_push(out, s); }
    Ok(out)
}
fn re_named_captures(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let names = rxc(c, slf)?.re.as_ref().map(|r| r.names()).unwrap_or_default();
    let h = c.hash_new();
    for (n, idx) in names {
        let a = c.ary_from_vec(idx.iter().map(|&i| Value::fix(i as i64)).collect());
        let k = c.str_new(n.as_bytes());
        crate::hash::hash_set(c, h, k, a)?;
    }
    Ok(h)
}
fn re_s_last_match(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    if let Some(&i) = a.first() {
        let lm = crate::builtins::svar::last_match(c);
        if lm == NIL { return Ok(NIL); }
        if i.is_sym() || is_string(i) { return md_aref(c, lm, &[i], Blk::NONE); }
        let i = crate::builtins::numeric_methods::num_to_long(c, i)?;
        return Ok(nth_match(c, lm, i));
    }
    Ok(crate::builtins::svar::last_match(c))
}
fn re_timeout(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let t = rx(c, slf).timeout; Ok(t.map_or(NIL, |f| c.float_new(f))) }
fn re_s_timeout(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let t = crate::regexp::TIMEOUT.load(std::sync::atomic::Ordering::Relaxed);
    Ok(if t == crate::regexp::NO_TIMEOUT { NIL } else { c.float_new(f64::from_bits(t)) })
}
fn re_s_set_timeout(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let t = if a[0] == NIL { None } else {
        let f = crate::builtins::numeric_methods::to_f(c, a[0])?;
        if !(f > 0.0) || f.is_nan() { let t = inspect_str(c, a[0])?; return arg_err(c, format!("invalid timeout: {}", t)); }
        Some(f)
    };
    crate::regexp::TIMEOUT.store(t.map_or(crate::regexp::NO_TIMEOUT, f64::to_bits), std::sync::atomic::Ordering::Relaxed);
    Ok(a[0])
}
/// Regexp.linear_time?: Onigmo's memoization applies unless the pattern has backrefs,
/// subexpression calls, the absent operator or captures inside lookarounds.
fn re_s_linear_time(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (args, _kw) = match kw_arg(c, a) { Some(_) => (&a[..a.len() - 1], true), None => (a, false) };
    if args.is_empty() || args.len() > 2 { return argc_err(c, args.len(), 1, 2); }
    let r = if is_regexp(c, args[0]) {
        if args.len() > 1 { reg_warn(c, "flags ignored")?; }
        args[0]
    } else {
        let k = c.vm().c_regexp.get();
        crate::builtins::object::c_new(c, k, a, Blk::NONE)?
    };
    Ok(Value::bool(rxc(c, r)?.re.as_ref().map_or(false, |re| re.linear_time())))
}

// MatchData
/// NUM2INT with the fixnum-in-int case inline (MatchData#[] is hot)
#[inline(always)]
fn md_int(c: &mut Ctx, k: Value) -> Result<i64, Esc> {
    if k.is_fixnum() { let v = k.as_fix(); if v as i32 as i64 == v { return Ok(v); } }
    crate::builtins::numeric_methods::num_to_int32(c, k)
}
pub(crate) fn group_index(c: &mut Ctx, m: Value, k: Value) -> Result<Option<usize>, Esc> {
    if k.is_sym() || is_string(k) {
        let nm = if k.is_sym() { crate::symbol::sym_str(k.as_sym()) } else { String::from_utf8_lossy(c.str_bytes(k)).into_owned() };
        let r = rregexp(c, md(c, m).regexp);
        let idx = r.and_then(|r| r.re.as_ref()).map(|r| r.group_index(&nm)).unwrap_or_default();
        if idx.is_empty() { return err(c, c.vm().c_index_error.get(), format!("undefined group name reference: {}", nm)); }
        for &i in idx.iter().rev() { if md(c, m).groups.get(i).map(|g| g.is_some()).unwrap_or(false) { return Ok(Some(i)); } }
        return Ok(Some(*idx.last().unwrap()));
    }
    let i = md_int(c, k)?;
    let n = md(c, m).groups.len() as i64;
    // like rb_reg_nth_match, a negative index never reaches group 0
    let i = if i < 0 { if i + n <= 0 { return Ok(None); } i + n } else { i };
    if i >= n { return Ok(None); }
    Ok(Some(i as usize))
}
/// match_backref_number + bounds check (begin/end/offset/byte*): no negative indices.
fn group_index_strict(c: &mut Ctx, m: Value, k: Value) -> Result<usize, Esc> {
    if k.is_sym() || is_string(k) { return Ok(group_index(c, m, k)?.unwrap_or(0)); }
    let i = md_int(c, k)?;
    if i < 0 || i >= md(c, m).groups.len() as i64 { return err(c, c.vm().c_index_error.get(), format!("index {} out of matches", i)); }
    Ok(i as usize)
}
fn md_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { check_argc(c, a, 1, 2)?;
    if a.len() == 2 || is_range(a[0]) { let all = md_to_a(c, slf, &[], Blk::NONE)?; return crate::builtins::array_methods::ary_aref(c, all, a); }
    match group_index(c, slf, a[0])? { Some(i) => Ok(nth_match(c, slf, i as i64)), None => Ok(NIL) }
}
fn md_match(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let i = group_index_strict(c, slf, a[0])?; Ok(nth_match(c, slf, i as i64)) }
fn md_match_length(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let i = group_index_strict(c, slf, a[0])?;
    let v = nth_match(c, slf, i as i64);
    Ok(if v == NIL { NIL } else { Value::fix(c.str_char_len(v) as i64) })
}
fn md_to_a(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let n = md(c, slf).groups.len();
    let out = c.ary_new_capa(n);
    for i in 0..n { let x = nth_match(c, slf, i as i64); c.ary_push(out, x); }
    Ok(out)
}
fn md_captures(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(match_groups_array(c, slf)) }
fn md_named_captures(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { { let n = a.len() - kw_arg(c, a).is_some() as usize; if n > 0 { return argc_err(c, n, 0, 0); } }
    let h = c.hash_new();
    let symk = match kw_arg(c, a) { Some(hh) => crate::hash::hash_get_opt(c, hh, sym_value("symbolize_names"))?.map(|v| v.truthy()).unwrap_or(false), None => false };
    let names = rregexp(c, md(c, slf).regexp).and_then(|r| r.re.as_ref()).map(|r| r.names()).unwrap_or_default();
    for (n, idx) in names {
        let mut v = NIL;
        for &i in idx.iter() { let x = nth_match(c, slf, i as i64); if x != NIL { v = x; } }
        let k = if symk { Value::sym(intern(n.as_bytes())) } else { c.str_new(n.as_bytes()) };
        crate::hash::hash_set(c, h, k, v)?;
    }
    Ok(h)
}
fn md_names(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { let r = md(c, slf).regexp; if r == NIL { return Ok(c.ary_new()); } re_names(c, r, a, b) }
/// Character offset of an end (0: begin, 1: end) of group a[0].
fn md_edge(c: &mut Ctx, slf: Value, a: &[Value], end: bool) -> RV {
    let i = group_index_strict(c, slf, a[0])?;
    let m = md(c, slf);
    Ok(match m.groups[i] { Some((s2, e)) => Value::fix(byte_to_char(c, m.str, if end { e } else { s2 })), None => NIL })
}
fn md_begin(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { md_edge(c, slf, a, false) }
fn md_end(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { md_edge(c, slf, a, true) }
fn md_offset(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { let x = md_begin(c, slf, a, b)?; let y = md_end(c, slf, a, b)?; Ok(c.ary_new_from(&[x, y])) }
fn md_byteoffset(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let i = group_index_strict(c, slf, a[0])?;
    let pair = match md(c, slf).groups.get(i).copied().flatten() { Some((s2, e)) => [Value::fix(s2 as i64), Value::fix(e as i64)], None => [NIL, NIL] };
    Ok(c.ary_new_from(&pair))
}
/// MatchData#bytebegin / #byteend (3.4)
fn md_bytebegin(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let i = group_index_strict(c, slf, a[0])?; Ok(md(c, slf).groups.get(i).copied().flatten().map_or(NIL, |(s2, _)| Value::fix(s2 as i64))) }
fn md_byteend(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let i = group_index_strict(c, slf, a[0])?; Ok(md(c, slf).groups.get(i).copied().flatten().map_or(NIL, |(_, e)| Value::fix(e as i64))) }
fn md_pre(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(match_pre(c, slf)) }
fn md_post(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(match_post(c, slf)) }
fn md_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(nth_match(c, slf, 0)) }
fn md_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(md(c, slf).groups.len() as i64)) }
// the snapshot taken at match time is already frozen
fn md_string(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(md(c, slf).str) }
fn md_regexp(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(md(c, slf).regexp) }
fn md_values_at(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let all = md_to_a(c, slf, &[], b)?;
    let out = c.ary_new();
    for &k in a {
        if k.is_sym() || is_string(k) {
            let i = group_index(c, slf, k)?.unwrap_or(0);
            let v = nth_match(c, slf, i as i64);
            c.ary_push(out, v);
        } else if is_range(k) {
            let part = crate::builtins::array_methods::a_values_at(c, all, &[k], b)?;
            let items = c.ary(part).to_vec();
            for v in items { c.ary_push(out, v); }
        } else {
            let v = match group_index(c, slf, k)? { Some(i) => nth_match(c, slf, i as i64), None => NIL };
            c.ary_push(out, v);
        }
    }
    Ok(out)
}
fn md_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
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
fn md_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if slf == a[0] { return Ok(TRUE); }
    let Some(o) = rmatch(c, a[0]) else { return Ok(FALSE) };
    let m = md(c, slf);
    let (r1, r2) = (m.regexp, o.regexp);
    if m.groups != o.groups || !c.str_eq(m.str, o.str) || c.str_enc(m.str) != c.str_enc(o.str) { return Ok(FALSE); }
    Ok(Value::bool(reg_equal(c, r1, r2)?))
}
/// match_hash: the regexp's, the string's and the offsets
fn md_hash(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let (r, st, g) = { let m = md(c, slf); (m.regexp, m.str, m.groups.clone()) };
    let mut h = crate::astrust::hash_merge(re_hash(c, r, &[], Blk::NONE)?.as_fix() as u64, c.str_hash(st));
    for x in g { h = crate::astrust::hash_merge(h, x.map_or(u64::MAX, |(b, e)| (b as u64) << 32 ^ e as u64)); }
    Ok(Value::fix((h >> 2) as i64))
}
fn md_deconstruct_keys(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let keys = a[0];
    if keys == NIL {
        let h = md_named_captures(c, slf, &[], b)?;
        let out = c.hash_new();
        for (k, v) in c.hash_pairs(h) { let ks = c.str_to_sym(k); crate::hash::hash_set(c, out, ks, v)?; }
        return Ok(out);
    }
    if !is_array(keys) { let t = class_path(c, real_class_of(c, keys)); return type_err(c, format!("wrong argument type {} (expected Array)", t)); }
    let out = c.hash_new();
    let ks = c.ary(keys).to_vec();
    let names = rregexp(c, md(c, slf).regexp).and_then(|r| r.re.as_ref()).map(|r| r.names()).unwrap_or_default();
    if ks.len() > names.len() { return Ok(out); }
    for k in ks {
        if !k.is_sym() { let t = class_path(c, real_class_of(c, k)); return type_err(c, format!("wrong argument type {} (expected Symbol)", t)); }
        let nm = crate::symbol::sym_str(k.as_sym());
        if !names.iter().any(|(n, _)| *n == nm) { break; }
        let i = group_index(c, slf, k)?.unwrap_or(0);
        let v = nth_match(c, slf, i as i64);
        crate::hash::hash_set(c, out, k, v)?;
    }
    Ok(out)
}

pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if let Some(r) = rregexp(c, v) {
        let (src, opts, enc, timeout, init) = (r.src.clone(), r.opts, r.enc, r.timeout, r.re.is_some());
        let re = if init { regexp_build(&src, enc, opts).ok().map(|x| x.3) } else { None };
        return Ok(Some(c.alloc(obj_klass(v), Body::Data(Box::new(RRegexp { src, opts, re, enc, timeout })))));
    }
    if let Some(m) = rmatch(c, v) { let (re, s, g) = (m.regexp, m.str, m.groups.clone()); return Ok(Some(match_new(c, re, s, g))); }
    Ok(None)
}
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if crate::builtins::inherits(c, klass, c.vm().c_regexp.get()) {
        return Ok(Some(c.alloc(klass, Body::Data(Box::new(RRegexp { src: vec![], opts: 0, re: None, enc: ENC_BINARY, timeout: None })))));
    }
    Ok(None)
}

/// Boot: Regexp and MatchData; returns (Regexp, MatchData).
pub fn init(c: &mut Ctx) -> (Value, Value) {
    let vmr = c.vm();
    let r = define_class(c, "Regexp", vmr.c_object.get());
    define_private_cfunc(c, r, "initialize_copy", crate::builtins::object::k_init_copy, 1);
    class_set_alloc(c, r, AllocKind::None);
    const_set(c, r, intern(b"IGNORECASE"), Value::fix(1));
    const_set(c, r, intern(b"EXTENDED"), Value::fix(2));
    const_set(c, r, intern(b"MULTILINE"), Value::fix(4));
    const_set(c, r, intern(b"FIXEDENCODING"), Value::fix(RE_OPT_FIXED as i64));
    const_set(c, r, intern(b"NOENCODING"), Value::fix(RE_OPT_ENC_NONE as i64));
    define_singleton_cfunc(c, r, "escape", re_s_escape, 1);
    define_singleton_cfunc(c, r, "quote", re_s_escape, 1);
    define_singleton_cfunc(c, r, "union", re_s_union, -1);
    define_singleton_cfunc(c, r, "last_match", re_s_last_match, -1);
    define_singleton_cfunc(c, r, "compile", re_s_compile, -1);
    define_singleton_cfunc(c, r, "try_convert", re_s_try_convert, 1);
    define_singleton_cfunc(c, r, "linear_time?", re_s_linear_time, -1);
    define_singleton_cfunc(c, r, "timeout", re_s_timeout, 0);
    define_singleton_cfunc(c, r, "timeout=", re_s_set_timeout, 1);
    define_private_cfunc(c, r, "initialize", re_initialize, -1);
    define_class_under(c, r, "TimeoutError", vmr.c_regexp_error.get());
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, r, n, f, a);
    d(c, "match", re_match, -1); d(c, "match?", re_match_p, -1); d(c, "=~", re_match_op, 1); d(c, "===", re_eqq, 1); d(c, "~", re_tilde, 0); d(c, "source", re_source, 0); d(c, "encoding", re_encoding, 0); d(c, "fixed_encoding?", re_fixed_encoding, 0);
    d(c, "inspect", re_inspect, 0); d(c, "to_s", re_to_s, 0); d(c, "options", re_options, 0); d(c, "casefold?", re_casefold, 0); d(c, "==", re_eq, 1);
    d(c, "eql?", re_eq, 1); d(c, "hash", re_hash, 0); d(c, "names", re_names, 0); d(c, "named_captures", re_named_captures, 0); d(c, "timeout", re_timeout, 0);
    let m = define_class(c, "MatchData", vmr.c_object.get());
    define_private_cfunc(c, m, "initialize_copy", crate::builtins::object::k_init_copy, 1);
    class_set_alloc(c, m, AllocKind::None);
    let ms = singleton_class(c, m);
    define_method_me(c, ms, intern(b"allocate"), new_me(intern(b"allocate"), ms, VISI_PUBLIC, MKind::Undef));
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, m, n, f, a);
    d(c, "[]", md_aref, -1); d(c, "to_a", md_to_a, 0); d(c, "captures", md_captures, 0); crate::builtins::object::alias_builtin(c, m, "deconstruct", "captures"); d(c, "named_captures", md_named_captures, -1);
    d(c, "names", md_names, 0); d(c, "begin", md_begin, 1); d(c, "end", md_end, 1); d(c, "offset", md_offset, 1); d(c, "byteoffset", md_byteoffset, 1); d(c, "bytebegin", md_bytebegin, 1); d(c, "byteend", md_byteend, 1);
    d(c, "pre_match", md_pre, 0); d(c, "post_match", md_post, 0); d(c, "to_s", md_to_s, 0); d(c, "size", md_size, 0); d(c, "length", md_size, 0);
    d(c, "string", md_string, 0); d(c, "regexp", md_regexp, 0); d(c, "values_at", md_values_at, -1); d(c, "inspect", md_inspect, 0); d(c, "==", md_eq, 1);
    d(c, "eql?", md_eq, 1); d(c, "hash", md_hash, 0); d(c, "deconstruct_keys", md_deconstruct_keys, 1); d(c, "match", md_match, 1); d(c, "match_length", md_match_length, 1);
    (r, m)
}
