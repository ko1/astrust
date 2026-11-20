// String methods.
// Heap access goes through the context (heap.rs, docs/safe_heap.md): bytes borrowed from a
// String cannot be held across a call that takes `c`; what is needed across one is copied.
#![forbid(unsafe_code)]
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::regexp;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

macro_rules! cfn { ($name:ident, |$c:ident, $s:ident, $a:ident, $b:ident| $body:expr) => {
    #[allow(unused_variables)]
    pub fn $name($c: &mut Ctx, $s: Value, $a: &[Value], $b: *const Block) -> RV { $body }
}; }

/// Character index → byte offset (None if out of range); `i` may equal the length.
fn char_to_byte(c: &Ctx, v: Value, i: usize) -> Option<usize> {
    let r = c.rstring(v);
    if enc_single(&r.b, r.enc) { return if i <= r.b.len() { Some(i) } else { None }; }
    let mut pos = 0; let mut n = 0;
    while n < i { if pos >= r.b.len() { return None; } pos += enc_char_len(&r.b, pos, r.enc); n += 1; }
    Some(pos)
}
fn byte_to_char(c: &Ctx, v: Value, off: usize) -> usize {
    let r = c.rstring(v);
    if enc_single(&r.b, r.enc) { return off; }
    enc_strlen(&r.b[..off], r.enc)
}
fn need_ascii_compat(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    let e = c.str_enc(v);
    if !crate::builtins::encoding::ascii_compat(e) { return err(c, c.vm().c_compat_error, format!("ASCII incompatible encoding: {}", crate::builtins::encoding::enc_name(e))); }
    Ok(())
}
/// Encoding of `a + b` or CompatibilityError.
fn compat(c: &mut Ctx, a: Value, x: Value) -> Result<u8, Esc> { crate::builtins::encoding::check_compatible(c, a, x) }
/// Decode a string into (code, byte range) per character.
fn codes_of(c: &Ctx, v: Value) -> Vec<(u32, usize, usize)> {
    let r = c.rstring(v);
    let off = char_offsets_of(&r.b, r.enc);
    (0..off.len() - 1).map(|k| (crate::builtins::encoding::char_code(r.enc, &r.b[off[k]..off[k + 1]]), off[k], off[k + 1])).collect()
}
/// Raise ArgumentError for a broken string (tr/delete/count/squeeze etc.).
fn check_valid(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    let r = c.rstring(v);
    let enc = r.enc;
    if !crate::builtins::encoding::is_valid(enc, &r.b) { return arg_err(c, format!("invalid byte sequence in {}", crate::builtins::encoding::enc_name(enc))); }
    Ok(())
}

/// substring by char start/len
fn substr(c: &mut Ctx, v: Value, start: i64, len: i64) -> Option<Value> {
    let n = c.str_char_len(v) as i64;
    let mut st = start;
    if st < 0 { st += n; }
    if st < 0 || st > n || len < 0 { return None; }
    let e = (st + len).min(n);
    let bs = char_to_byte(c, v, st as usize)?; let be = char_to_byte(c, v, e as usize)?;
    Some(c.str_from_vec_enc(c.str_bytes(v)[bs..be].to_vec(), c.str_enc(v)))
}

pub fn str_aref(c: &mut Ctx, slf: Value, a: &[Value]) -> RV {
    check_argc(c, a, 1, 2)?;
    if a.len() == 2 {
        if regexp::is_regexp(a[0]) {
            let m = regexp::search(c, a[0], slf, 0, true)?;
            if m == NIL { return Ok(NIL); }
            if a[1].is_sym() || is_string(a[1]) { let nm = if a[1].is_sym() { a[1].as_sym() } else { intern(c.str_bytes(a[1])) }; return Ok(regexp::named_match(m, nm)); }
            let i = num_to_long(c, a[1])?;
            return Ok(regexp::nth_match(m, i));
        }
        let st = num_to_long(c, a[0])?; let ln = num_to_long(c, a[1])?;
        return Ok(substr(c, slf, st, ln).unwrap_or(NIL));
    }
    let x = a[0];
    if x.is_fixnum() {
        let i = x.as_fix();
        let n = c.str_char_len(slf) as i64;
        let i = if i < 0 { i + n } else { i };
        if i < 0 || i >= n { return Ok(NIL); }
        return Ok(substr(c, slf, i, 1).unwrap_or(NIL));
    }
    if is_range(x) {
        let n = c.str_char_len(slf) as i64;
        return match crate::range::range_beg_len(c, x, n, 0)? { Some((st, ln)) => Ok(substr(c, slf, st, ln).unwrap_or(NIL)), None => Ok(NIL) };
    }
    if is_string(x) {
        return Ok(if find_bytes(c.str_bytes(slf), c.str_bytes(x), 0).is_some() { c.str_new_like(x, c.str_bytes(x).to_vec()) } else { NIL });
    }
    if regexp::is_regexp(x) {
        let m = regexp::search(c, x, slf, 0, true)?;
        return Ok(if m == NIL { NIL } else { regexp::nth_match(m, 0) });
    }
    let i = num_to_long(c, x)?;
    str_aref(c, slf, &[Value::fix(i)])
}

pub fn find_bytes(h: &[u8], n: &[u8], from: usize) -> Option<usize> {
    if n.is_empty() { return if from <= h.len() { Some(from) } else { None }; }
    if from >= h.len() || n.len() > h.len() - from { return None; }
    let first = n[0];
    let mut i = from;
    while i + n.len() <= h.len() {
        match h[i..h.len() - n.len() + 1].iter().position(|&x| x == first) {
            None => return None,
            Some(p) => { i += p; if &h[i..i + n.len()] == n { return Some(i); } i += 1; }
        }
    }
    None
}
/// find_bytes, but only hits that start on a character boundary of `slf`.
fn find_on_boundary(c: &Ctx, slf: Value, n: &[u8], from: usize) -> Option<usize> {
    let r = c.rstring(slf);
    if enc_single(&r.b, r.enc) || crate::builtins::encoding::kind(r.enc) == crate::builtins::encoding::EK::Utf8 && crate::builtins::encoding::is_valid(r.enc, n) {
        return find_bytes(&r.b, n, from);
    }
    let off = char_offsets_of(&r.b, r.enc);
    let mut i = from;
    loop {
        let p = find_bytes(&r.b, n, i)?;
        if off.binary_search(&p).is_ok() { return Some(p); }
        i = p + 1;
    }
}
fn rfind_bytes(h: &[u8], n: &[u8], upto: usize) -> Option<usize> {
    if n.len() > h.len() { return None; }
    let mut i = upto.min(h.len() - n.len()) as i64;
    while i >= 0 { if &h[i as usize..i as usize + n.len()] == n { return Some(i as usize); } i -= 1; }
    None
}

cfn!(s_initialize, |c, slf, a, _b| {
    let kw = kw_arg(a);
    let pos: &[Value] = if kw.is_some() { &a[..a.len() - 1] } else { a };
    check_argc(c, pos, 0, 1)?;
    if let Some(&v) = pos.first() {
        let sv = expect_str(c, v)?;
        if sv != slf { let nb = c.str_bytes(sv).to_vec(); let e = c.str_enc(sv); let r = c.str_modify(slf)?; r.b = nb.into(); r.enc = e; }
    }
    if let Some(h) = kw {
        if let Some(e) = crate::hash::hash_get_opt(c, h, sym_value("encoding"))? { let e = crate::builtins::encoding::to_enc(c, e)?; c.str_modify(slf)?.enc = e; }
        if let Some(cap) = crate::hash::hash_get_opt(c, h, sym_value("capacity"))? { num_to_long(c, cap)?; }
    }
    Ok(NIL)
});
cfn!(s_s_new, |c, slf, a, blk| {
    let o = crate::builtins::object::alloc_instance(c, slf)?;
    if is_string(o) { c.rstring_mut(o).enc = ENC_BINARY; }
    funcall_kw(c, o, s::initialize, a, blk, kw_arg(a).is_some())?;
    Ok(o)
});
cfn!(s_length, |c, slf, a, b_| Ok(Value::fix(c.str_char_len(slf) as i64)));
cfn!(s_bytesize, |c, slf, a, b_| Ok(Value::fix(c.str_bytes(slf).len() as i64)));
cfn!(s_plus, |c, slf, a, b_| {
    let o = expect_str(c, a[0])?;
    let enc = compat(c, slf, o)?;
    let mut v = c.str_bytes(slf).to_vec(); v.extend_from_slice(c.str_bytes(o));
    Ok(c.str_from_vec_enc(v, enc))
});
cfn!(s_mul, |c, slf, a, b_| {
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative argument"); }
    if (c.str_bytes(slf).len() as u128) * (n as u128) > (1u128 << 31) * 64 { return arg_err(c, "argument too big"); }
    Ok(c.str_new_like(slf, c.str_bytes(slf).repeat(n as usize)))
});
cfn!(s_mod, |c, slf, a, b_| {
    let x = a[0];
    let arr = if is_array(x) { x } else if x.is_heap() && !is_string(x) && respond_to(x, intern(b"to_ary"), true) {
        let r = funcall(c, x, intern(b"to_ary"), &[], std::ptr::null())?;
        if is_array(r) || r == NIL { r } else { return type_err(c, format!("can't convert {} to Array ({}#to_ary gives {})", class_name_for_err(x), class_name_for_err(x), class_name_for_err(r))); }
    } else { NIL };
    // the elements stay reachable through `arr`, kept alive past the call
    let args: Vec<Value> = if arr != NIL { c.ary(arr).to_vec() } else { vec![x] };
    let r = crate::builtins::sprintf::format(c, slf, &args);
    std::hint::black_box(arr);
    r
});
cfn!(s_eq, |c, slf, a, b_| {
    let o = a[0];
    if is_string(o) { return Ok(Value::bool(c.str_bytes(slf) == c.str_bytes(o))); }
    if !o.is_heap() { return Ok(FALSE); }
    // rb_respond_to honours a user-defined respond_to?
    let rt = intern(b"respond_to?");
    let ok = if respond_to(o, rt, true) { funcall(c, o, rt, &[Value::sym(s::to_str)], std::ptr::null())?.truthy() } else { respond_to(o, s::to_str, true) };
    if ok { let r = funcall(c, o, s::eq, &[slf], std::ptr::null())?; return Ok(Value::bool(r.truthy())); }
    Ok(FALSE)
});
cfn!(s_eql, |c, slf, a, b_| Ok(Value::bool(is_string(a[0]) && c.str_bytes(slf) == c.str_bytes(a[0]))));
cfn!(s_hash, |c, slf, a, b_| Ok(Value::fix((c.str_hash(slf) >> 2) as i64)));
cfn!(s_cmp, |c, slf, a, b_| {
    if is_string(a[0]) { return Ok(Value::fix(c.str_bytes(slf).cmp(c.str_bytes(a[0])) as i64)); }
    if a[0].is_heap() && respond_to(a[0], s::to_str, true) {
        let t = funcall(c, a[0], s::to_str, &[], std::ptr::null())?;
        if is_string(t) { return Ok(Value::fix(c.str_bytes(slf).cmp(c.str_bytes(t)) as i64)); }
    }
    if a[0].is_heap() && respond_to(a[0], s::cmp, true) {
        let r = funcall(c, a[0], s::cmp, &[slf], std::ptr::null())?;
        if r == NIL { return Ok(NIL); }
        let pos = if r.is_fixnum() { r.as_fix() > 0 } else if is_bignum(r) { !c.big(r).neg } else { funcall(c, r, intern(b">"), &[Value::fix(0)], std::ptr::null())?.truthy() };
        let neg = !pos && if r.is_fixnum() { r.as_fix() < 0 } else if is_bignum(r) { c.big(r).neg } else { funcall(c, r, intern(b"<"), &[Value::fix(0)], std::ptr::null())?.truthy() };
        return Ok(Value::fix(if pos { -1 } else if neg { 1 } else { 0 }));
    }
    Ok(NIL)
});
cfn!(s_match_op, |c, slf, a, b_| {
    if is_string(a[0]) { return type_err(c, "wrong argument type String (expected Regexp)"); }
    if regexp::is_regexp(a[0]) {
        let m = regexp::search(c, a[0], slf, 0, true)?;
        if m == NIL { return Ok(NIL); }
        let (st, _) = regexp::rmatch(m).unwrap().groups[0].unwrap();
        return Ok(Value::fix(byte_to_char(c, slf, st) as i64));
    }
    funcall(c, a[0], s::match_op, &[slf], std::ptr::null())
});
cfn!(s_match, |c, slf, a, blk| {
    if a.is_empty() { return argc_err(c, 0, 1, 2); }
    let re = regexp::to_regexp(c, a[0])?;
    let pos = if a.len() > 1 { let p = num_to_long(c, a[1])?; let n = c.str_char_len(slf) as i64; let p = if p < 0 { p + n } else { p }; if p < 0 || p > n { c.last_match = NIL; return Ok(NIL); } char_to_byte(c, slf, p as usize).unwrap_or(0) } else { 0 };
    let m = regexp::search(c, re, slf, pos, true)?;
    if m != NIL && !blk.is_null() { return yield1(c, blk, m); }
    Ok(m)
});
cfn!(s_match_p, |c, slf, a, b_| {
    let re = regexp::to_regexp(c, a[0])?;
    let pos = if a.len() > 1 { let p = num_to_long(c, a[1])?.max(0) as usize; char_to_byte(c, slf, p).unwrap_or(usize::MAX) } else { 0 };
    Ok(Value::bool(regexp::search_raw(re, c.str_bytes(slf), pos).is_some()))
});
cfn!(s_aref, |c, slf, a, b_| str_aref(c, slf, a));
cfn!(s_aset, |c, slf, a, b_| {
    check_argc(c, a, 2, 3)?;
    let val = expect_str(c, *a.last().unwrap())?;
    let n = c.str_char_len(slf) as i64;
    let (bs, be) = if a.len() == 3 {
        if regexp::is_regexp(a[0]) {
            let m = regexp::search(c, a[0], slf, 0, true)?;
            if m == NIL { return err(c, c.vm().c_index_error, "regexp not matched"); }
            let g = num_to_long(c, a[1])?;
            match regexp::rmatch(m).unwrap().groups.get(g as usize).copied().flatten() { Some(x) => x, None => return err(c, c.vm().c_index_error, format!("index {} out of regexp", g)) }
        } else {
            let mut st = num_to_long(c, a[0])?; let ln = num_to_long(c, a[1])?;
            if st < 0 { st += n; }
            if st < 0 || st > n { let o = num_to_long(c, a[0])?; return err(c, c.vm().c_index_error, format!("index {} out of string", o)); }
            if ln < 0 { return err(c, c.vm().c_index_error, format!("negative length {}", ln)); }
            (char_to_byte(c, slf, st as usize).unwrap(), char_to_byte(c, slf, (st + ln).min(n) as usize).unwrap())
        }
    } else if a[0].is_fixnum() {
        let mut i = a[0].as_fix();
        if i < 0 { i += n; }
        if i < 0 || i > n { return err(c, c.vm().c_index_error, format!("index {} out of string", a[0].as_fix())); }
        let bs = char_to_byte(c, slf, i as usize).unwrap(); (bs, char_to_byte(c, slf, i as usize + 1).unwrap_or(bs))
    } else if is_range(a[0]) {
        match crate::range::range_beg_len(c, a[0], n, 1)? { Some((st, ln)) => (char_to_byte(c, slf, st as usize).unwrap(), char_to_byte(c, slf, (st + ln).min(n) as usize).unwrap()), None => return Ok(val) }
    } else if is_string(a[0]) {
        match find_bytes(c.str_bytes(slf), c.str_bytes(a[0]), 0) { Some(p) => (p, p + c.str_bytes(a[0]).len()), None => return err(c, c.vm().c_index_error, "string not matched") }
    } else if regexp::is_regexp(a[0]) {
        let m = regexp::search(c, a[0], slf, 0, true)?;
        if m == NIL { return err(c, c.vm().c_index_error, "regexp not matched"); }
        regexp::rmatch(m).unwrap().groups[0].unwrap()
    } else { let i = num_to_long(c, a[0])?; return s_aset(c, slf, &[Value::fix(i), val], b_); };
    let vb = c.str_bytes(val).to_vec();
    let r = c.str_modify(slf)?;
    r.b.splice(bs..be, vb);
    Ok(val)
});
cfn!(s_concat, |c, slf, a, b_| {
    check_frozen(c, slf)?;
    // convert everything first: concat(self, self) appends the original contents
    let parts = c.ary_new();
    for &x in a {
        if x.is_fixnum() || is_bignum(x) { let s2 = int_to_char_str(c, slf, x)?; c.ary_push(parts, s2); }
        else { let s2 = expect_str(c, x)?; let d = c.str_copy(s2); c.ary_push(parts, d); }
    }
    for i in 0..c.ary_len(parts) { let p = c.ary_get(parts, i); str_append_str(c, slf, p)?; }
    Ok(slf)
});
/// Integer argument of << / concat: the character with that code in self's encoding.
fn int_to_char_str(c: &mut Ctx, slf: Value, x: Value) -> RV {
    use crate::builtins::encoding as en;
    let enc = c.str_enc(slf);
    if x.is_fixnum() && x.as_fix() < 0 || is_bignum(x) {
        let s = crate::builtins::kernel::inspect_str(c, x)?;
        return err(c, c.vm().c_range_error, format!("{} out of char range", s));
    }
    let i = x.as_fix();
    if enc == ENC_BINARY || enc == ENC_ASCII {
        if i > 255 { return err(c, c.vm().c_range_error, format!("{} out of char range", i)); }
        if enc == ENC_ASCII && i > 127 { c.rstring_mut(slf).enc = ENC_BINARY; }
        return Ok(c.str_new_enc(&[i as u8], c.str_enc(slf)));
    }
    if i > u32::MAX as i64 || (en::is_unicode(enc) && i > 0x10ffff) || (en::is_single_byte(enc) && i > 0xff) { return err(c, c.vm().c_range_error, format!("{} out of char range", i)); }
    match en::code_to_bytes(enc, i as u32) {
        Some(v) => Ok(c.str_new_enc(&v, enc)),
        None => err(c, c.vm().c_range_error, format!("invalid codepoint 0x{:X} in {}", i, en::enc_name(enc))),
    }
}
cfn!(s_ltlt, |c, slf, a, b_| {
    let x = a[0];
    if is_string(x) { str_append_str(c, slf, x)?; return Ok(slf); }
    if x.is_fixnum() || is_bignum(x) { check_frozen(c, slf)?; let s2 = int_to_char_str(c, slf, x)?; str_append_str(c, slf, s2)?; return Ok(slf); }
    let s2 = expect_str(c, x)?;
    str_append_str(c, slf, s2)?;
    Ok(slf)
});
cfn!(s_to_s, |c, slf, a, b_| {
    if obj_klass(slf) == c.vm().c_string { return Ok(slf); }
    Ok(c.str_copy(slf))
});
cfn!(s_to_sym, |c, slf, a, b_| Ok(c.str_to_sym(slf)));
cfn!(s_to_i, |c, slf, a, b_| {
    need_ascii_compat(c, slf)?;
    let base = if a.is_empty() { 10 } else { num_to_long(c, a[0])? };
    if base == 1 || base > 36 || base < 0 { return arg_err(c, format!("invalid radix {}", base)); }
    Ok(str_to_inum(c.str_bytes(slf), base as u32, false).unwrap_or(Value::fix(0)))
});
cfn!(s_to_f, |c, slf, a, b_| Ok(c.float_new(str_to_f(c.str_bytes(slf)))));
cfn!(s_inspect, |c, slf, a, b_| Ok(c.str_new(inspect_bytes(c.str_bytes(slf), c.str_enc(slf)).as_bytes())));
cfn!(s_dump, |c, slf, a, b_| {
    use crate::builtins::encoding as en;
    let v = c.str_bytes(slf); let enc = c.str_enc(slf);
    let u8m = en::kind(enc) == en::EK::Utf8;
    let mut out = String::from("\"");
    let mut i = 0;
    while i < v.len() {
        let x = v[i];
        match x {
            b'"' => out.push_str("\\\""), b'\\' => out.push_str("\\\\"), b'\n' => out.push_str("\\n"), b'\t' => out.push_str("\\t"), b'\r' => out.push_str("\\r"),
            0x0c => out.push_str("\\f"), 0x0b => out.push_str("\\v"), 0x08 => out.push_str("\\b"), 0x07 => out.push_str("\\a"), 0x1b => out.push_str("\\e"),
            b'#' => { if i + 1 < v.len() && matches!(v[i + 1], b'$' | b'@' | b'{') { out.push_str("\\#"); } else { out.push('#'); } }
            0x20..=0x7e => out.push(x as char),
            _ if u8m && x >= 0x80 => match en::utf8_precise(&v[i..]) {
                Some(l) => { let cp = decode_utf8(v, i).0; if cp > 0xffff { out.push_str(&format!("\\u{{{:X}}}", cp)); } else { out.push_str(&format!("\\u{:04X}", cp)); } i += l; continue; }
                None => out.push_str(&format!("\\x{:02X}", x)),
            },
            _ => out.push_str(&format!("\\x{:02X}", x)),
        }
        i += 1;
    }
    out.push('"');
    if !en::ascii_compat(enc) { out.push_str(&format!(".dup.force_encoding(\"{}\")", en::enc_name(enc))); return Ok(c.str_new(out.as_bytes())); }
    Ok(c.str_new_enc(out.as_bytes(), enc))
});
/// Case-mapping options (:ascii / :turkic / :lithuanian / :fold): returns (ascii, turkic, fold).
fn case_opts(c: &mut Ctx, a: &[Value], fold_ok: bool) -> Result<(bool, bool, bool), Esc> {
    if a.len() > 2 { return arg_err(c, "too many options"); }
    let name = |v: Value| if v.is_sym() { String::from_utf8_lossy(crate::symbol::sym_bytes(v.as_sym())).into_owned() } else { String::new() };
    let mut r = (false, false, false);
    for (i, &v) in a.iter().enumerate() {
        if !v.is_sym() { return type_err(c, format!("no implicit conversion of {} into Symbol", class_name_for_err(v))); }
        match name(v).as_str() {
            "ascii" if a.len() == 1 => r.0 = true,
            "turkic" | "lithuanian" if a.len() == 1 || i == 0 || name(a[0]) == "turkic" || name(a[0]) == "lithuanian" => { if name(v) == "turkic" { r.1 = true; } }
            "fold" if fold_ok && a.len() == 1 => r.2 = true,
            "ascii" | "fold" if a.len() > 1 => return arg_err(c, "too many options"),
            "fold" => return arg_err(c, "option :fold only allowed for downcasing"),
            _ => { let s = crate::builtins::kernel::inspect_str(c, v)?; return arg_err(c, format!("invalid option: {}", s)); }
        }
    }
    Ok(r)
}
fn case_apply(b: &[u8], enc: u8, which: u8, o: (bool, bool, bool)) -> Vec<u8> {
    if o.0 || enc != ENC_UTF8 {
        return match which {
            b'u' => b.iter().map(|x| x.to_ascii_uppercase()).collect(),
            b'd' => b.iter().map(|x| x.to_ascii_lowercase()).collect(),
            b's' => b.iter().map(|&x| if x.is_ascii_uppercase() { x.to_ascii_lowercase() } else { x.to_ascii_uppercase() }).collect(),
            _ => { let mut v: Vec<u8> = b.iter().map(|x| x.to_ascii_lowercase()).collect(); if let Some(f) = v.first_mut() { *f = f.to_ascii_uppercase(); } v }
        };
    }
    if o.2 { return fold_case(b, enc); }
    if o.1 {
        // Turkic dotted/dotless i
        let s = String::from_utf8_lossy(b);
        let m: String = match which {
            b'u' => s.chars().map(|ch| if ch == 'i' { "İ".to_string() } else { ch.to_uppercase().collect() }).collect(),
            b'd' => s.chars().map(|ch| if ch == 'I' { "ı".to_string() } else if ch == 'İ' { "i".to_string() } else { ch.to_lowercase().collect() }).collect(),
            _ => return case_apply(b, enc, which, (false, false, false)),
        };
        return m.into_bytes();
    }
    match which { b'u' => upcase(b, enc), b'd' => downcase(b, enc), b's' => swapcase(b, enc), _ => capitalize(b, enc) }
}
macro_rules! case_m { ($name:ident, $bang:ident, $w:expr) => {
    cfn!($name, |c, slf, a, b_| { let o = case_opts(c, a, $w == b'd')?; let r = c.rstring(slf); let nv = case_apply(&r.b, r.enc, $w, o); Ok(c.str_new_like(slf, nv)) });
    cfn!($bang, |c, slf, a, b_| { let o = case_opts(c, a, $w == b'd')?; check_frozen(c, slf)?; let r = c.rstring(slf); let nv = case_apply(&r.b, r.enc, $w, o); if nv == r.b { return Ok(NIL); } c.str_modify(slf)?.b = nv.into(); Ok(slf) });
}; }
case_m!(s_upcase, s_upcase_b, b'u');
case_m!(s_downcase, s_downcase_b, b'd');
case_m!(s_swapcase, s_swapcase_b, b's');
case_m!(s_capitalize, s_capitalize_b, b'c');
fn reverse_bytes(bv: &[u8], enc: u8) -> Vec<u8> {
    if enc_single(bv, enc) { let mut v = bv.to_vec(); v.reverse(); return v; }
    let off = char_offsets_of(bv, enc);
    let mut out = Vec::with_capacity(bv.len());
    for k in (0..off.len() - 1).rev() { out.extend_from_slice(&bv[off[k]..off[k + 1]]); }
    out
}
cfn!(s_reverse, |c, slf, a, b_| { let r = c.rstring(slf); let nv = reverse_bytes(&r.b, r.enc); Ok(c.str_new_like(slf, nv)) });
cfn!(s_reverse_b, |c, slf, a, b_| { let r = c.str_modify(slf)?; let nv = reverse_bytes(&r.b, r.enc); r.b = nv.into(); Ok(slf) });
cfn!(s_include, |c, slf, a, b_| { let o = expect_str(c, a[0])?; Ok(Value::bool(find_bytes(c.str_bytes(slf), c.str_bytes(o), 0).is_some())) });
cfn!(s_start_with, |c, slf, a, b_| {
    for &x in a {
        if regexp::is_regexp(x) { if let Some(g) = regexp::rregexp(x).unwrap().re.as_ref().unwrap().match_at(c.str_bytes(slf), 0) { let m = regexp::match_new(x, slf, g); c.last_match = m; return Ok(TRUE); } c.last_match = NIL; continue; }
        let o = expect_str(c, x)?; compat(c, slf, o)?; if c.str_bytes(slf).starts_with(c.str_bytes(o)) && c.char_offsets(slf).binary_search(&c.str_bytes(o).len()).is_ok() { return Ok(TRUE); }
    }
    Ok(FALSE)
});
cfn!(s_end_with, |c, slf, a, b_| { for &x in a { let o = expect_str(c, x)?; compat(c, slf, o)?; if c.str_bytes(slf).ends_with(c.str_bytes(o)) && c.char_offsets(slf).binary_search(&(c.str_bytes(slf).len() - c.str_bytes(o).len())).is_ok() { return Ok(TRUE); } } Ok(FALSE) });
cfn!(s_index, |c, slf, a, b_| {
    let n = c.str_char_len(slf) as i64;
    let mut st = if a.len() > 1 { num_to_long(c, a[1])? } else { 0 };
    if st < 0 { st += n; if st < 0 { return Ok(NIL); } }
    if st > n { return Ok(NIL); }
    let bs = char_to_byte(c, slf, st as usize).unwrap();
    if regexp::is_regexp(a[0]) {
        let m = regexp::search(c, a[0], slf, bs, true)?;
        if m == NIL { return Ok(NIL); }
        return Ok(Value::fix(byte_to_char(c, slf, regexp::rmatch(m).unwrap().groups[0].unwrap().0) as i64));
    }
    let o = expect_str(c, a[0])?;
    compat(c, slf, o)?;
    Ok(match find_on_boundary(c, slf, c.str_bytes(o), bs) { Some(p) => Value::fix(byte_to_char(c, slf, p) as i64), None => NIL })
});
cfn!(s_rindex, |c, slf, a, b_| {
    let n = c.str_char_len(slf) as i64;
    let mut st = if a.len() > 1 { num_to_long(c, a[1])? } else { n };
    if st < 0 { st += n; if st < 0 { return Ok(NIL); } }
    if st > n { st = n; }
    let bs = char_to_byte(c, slf, st as usize).unwrap();
    if regexp::is_regexp(a[0]) {
        let mut p = bs as i64;
        while p >= 0 {
            if let Some(g) = regexp::rregexp(a[0]).unwrap().re.as_ref().unwrap().match_at(c.str_bytes(slf), p as usize) {
                let m = regexp::match_new(a[0], slf, g); c.last_match = m;
                return Ok(Value::fix(byte_to_char(c, slf, p as usize) as i64));
            }
            p -= 1;
        }
        c.last_match = NIL;
        return Ok(NIL);
    }
    let o = expect_str(c, a[0])?;
    Ok(match rfind_bytes(c.str_bytes(slf), c.str_bytes(o), bs) { Some(p) => Value::fix(byte_to_char(c, slf, p) as i64), None => NIL })
});
fn is_ws(x: u8) -> bool { x == b' ' || (9..=13).contains(&x) || x == 0 }
/// strip core; `sel` = tr-style selector sets (empty: whitespace and NUL).
fn strip_range(c: &Ctx, v: Value, sel: &[(bool, Vec<u32>)], l: bool, r: bool) -> (usize, usize) {
    let x = c.rstring(v);
    let bv: &[u8] = &x.b;
    let (mut s, mut e) = (0, bv.len());
    if sel.is_empty() {
        if l { while s < e && is_ws(bv[s]) { s += 1; } }
        if r { while e > s && is_ws(bv[e - 1]) { e -= 1; } }
        return (s, e);
    }
    let off = char_offsets_of(bv, x.enc);
    let cp = |k: usize| crate::builtins::encoding::char_code(x.enc, &bv[off[k]..off[k + 1]]);
    let (mut i, mut j) = (0, off.len() - 1);
    if l { while i < j && in_sets(cp(i), sel) { i += 1; } }
    if r { while j > i && in_sets(cp(j - 1), sel) { j -= 1; } }
    s = off[i]; e = off[j];
    (s, e)
}
macro_rules! strip_m { ($name:ident, $bang:ident, $l:expr, $r:expr) => {
    cfn!($name, |c, slf, a, b_| { let sel = sets_of(c, slf, a)?; let (s, e) = strip_range(c, slf, &sel, $l, $r); Ok(c.str_sub(slf, s..e)) });
    cfn!($bang, |c, slf, a, b_| {
        let sel = sets_of(c, slf, a)?;
        let (s, e) = strip_range(c, slf, &sel, $l, $r);
        let r = c.str_modify(slf)?;
        if s == 0 && e == r.b.len() { return Ok(NIL); }
        r.b.truncate(e); r.b.remove_front(s);
        Ok(slf)
    });
}; }
strip_m!(s_strip, s_strip_b, true, true);
strip_m!(s_lstrip, s_lstrip_b, true, false);
strip_m!(s_rstrip, s_rstrip_b, false, true);
fn chomp_impl(c: &mut Ctx, slf: Value, a: &[Value]) -> Result<Vec<u8>, Esc> {
    check_argc(c, a, 0, 1)?;
    if c.str_bytes(slf).is_empty() { return Ok(vec![]); }
    if a.is_empty() || (a[0] != NIL && is_string(a[0]) && c.str_bytes(a[0]) == b"\n") {
        let v = c.str_bytes(slf);
        if v.ends_with(b"\r\n") { return Ok(v[..v.len() - 2].to_vec()); }
        if v.ends_with(b"\n") || v.ends_with(b"\r") { return Ok(v[..v.len() - 1].to_vec()); }
        return Ok(v.to_vec());
    }
    if a[0] == NIL { return Ok(c.str_bytes(slf).to_vec()); }
    let suf = expect_str(c, a[0])?;
    // read self after the conversion: to_str may have changed it
    let (v, sb) = (c.str_bytes(slf), c.str_bytes(suf));
    if sb.is_empty() { let mut e = v.len(); while e > 0 && v[e - 1] == b'\n' { e -= 1; if e > 0 && v[e - 1] == b'\r' { e -= 1; } } return Ok(v[..e].to_vec()); }
    let cut = v.ends_with(sb) && crate::builtins::encoding::is_valid(c.str_enc(suf), sb) && c.char_offsets(slf).binary_search(&(v.len() - sb.len())).is_ok();
    let keep = if cut { v.len() - sb.len() } else { v.len() };
    if cut { compat(c, slf, suf)?; }
    Ok(c.str_bytes(slf)[..keep].to_vec())
}
cfn!(s_chomp, |c, slf, a, b_| { let v = chomp_impl(c, slf, a)?; Ok(c.str_new_like(slf, v)) });
cfn!(s_chomp_b, |c, slf, a, b_| { check_frozen(c, slf)?; let v = chomp_impl(c, slf, a)?; if v.len() == c.str_bytes(slf).len() { return Ok(NIL); } c.str_modify(slf)?.b = v.into(); Ok(slf) });
fn chop_impl(v: &[u8], enc: u8) -> Vec<u8> {
    if v.is_empty() { return vec![]; }
    if v.ends_with(b"\r\n") { return v[..v.len() - 2].to_vec(); }
    let off = char_offsets_of(v, enc);
    v[..off[off.len() - 2]].to_vec()
}
cfn!(s_chop, |c, slf, a, b_| Ok(c.str_new_like(slf, chop_impl(c.str_bytes(slf), c.str_enc(slf)))));
cfn!(s_chop_b, |c, slf, a, b_| { if c.str_bytes(slf).is_empty() { return Ok(NIL); } let v = chop_impl(c.str_bytes(slf), c.str_enc(slf)); c.str_modify(slf)?.b = v.into(); Ok(slf) });
cfn!(s_chr, |c, slf, a, b_| { let v = c.str_bytes(slf); if v.is_empty() { return Ok(c.str_new_like(slf, vec![])); } let l = enc_char_len(v, 0, c.str_enc(slf)); Ok(c.str_new_like(slf, v[..l].to_vec())) });
cfn!(s_chars, |c, slf, a, blk| {
    let parts = c.chars_of(slf);
    if !blk.is_null() { for p in parts { let s2 = c.str_new_like(slf, p); yield1(c, blk, s2)?; } return Ok(slf); }
    Ok(c.ary_of_strs(parts, c.str_enc(slf)))
});
cfn!(s_each_char, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_char"), &[], NIL); }
    s_chars(c, slf, a, blk)
});
cfn!(s_bytes, |c, slf, a, blk| {
    if !blk.is_null() { let n = c.str_bytes(slf).len(); for i in 0..n { let x = *c.str_bytes(slf).get(i).unwrap_or(&0); yield1(c, blk, Value::fix(x as i64))?; } return Ok(slf); }
    Ok(c.ary_from_vec(c.str_bytes(slf).iter().map(|&x| Value::fix(x as i64)).collect()))
});
cfn!(s_each_byte, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_byte"), &[], NIL); }
    s_bytes(c, slf, a, blk)
});
fn lines_of(v: &[u8], sep: &[u8], chomp: bool) -> Vec<Vec<u8>> {
    let mut out = vec![];
    if v.is_empty() { return out; }
    if sep.is_empty() {
        // paragraph mode
        let mut i = 0;
        while i < v.len() {
            let j = i;
            loop {
                match find_bytes(v, b"\n\n", j) {
                    Some(p) => { let mut e = p + 2; while e < v.len() && v[e] == b'\n' { e += 1; } out.push(if chomp { v[i..p].to_vec() } else { v[i..p + 2].to_vec() }); i = e; break; }
                    None => { out.push(v[i..].to_vec()); i = v.len(); break; }
                }
                #[allow(unreachable_code)] { j += 1; }
            }
        }
        return out;
    }
    let mut i = 0;
    while i < v.len() {
        match find_bytes(v, sep, i) {
            Some(p) => {
                let e = p + sep.len();
                let mut line = v[i..e].to_vec();
                if chomp { line.truncate(line.len() - sep.len()); if sep == b"\n" && line.ends_with(b"\r") { line.pop(); } }
                out.push(line); i = e;
            }
            None => { out.push(v[i..].to_vec()); break; }
        }
    }
    out
}
fn lines_args(c: &mut Ctx, slf: Value, a: &[Value]) -> Result<(Vec<u8>, bool), Esc> {
    let mut sep = b"\n".to_vec(); let mut chomp = false;
    let kw = kw_arg(a).or_else(|| a.last().copied().filter(|v| is_hash(*v)));
    let pos: &[Value] = if kw.is_some() { &a[..a.len() - 1] } else { a };
    check_argc(c, pos, 0, 1)?;
    if let Some(h) = kw { if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("chomp"))? { chomp = v.truthy(); } }
    if pos.first() == Some(&NIL) { return Ok((vec![0xff, 0xfe, 0xfd], chomp)); }
    if c.str_bytes(slf).is_empty() { return Ok((sep, chomp)); }
    if let Some(&x) = pos.first() {
        if x == NIL { sep = vec![0xff, 0xfe, 0xfd]; } else { let s2 = expect_str(c, x)?; compat(c, slf, s2)?; sep = c.str_bytes(s2).to_vec(); }
    }
    Ok((sep, chomp))
}
cfn!(s_lines, |c, slf, a, blk| {
    let (sep, chomp) = lines_args(c, slf, a)?;
    let ls = if sep == [0xff, 0xfe, 0xfd] { vec![c.str_bytes(slf).to_vec()] } else { lines_of(c.str_bytes(slf), &sep, chomp) };
    if !blk.is_null() { for l in ls { let s2 = c.str_new_like(slf, l); yield1(c, blk, s2)?; } return Ok(slf); }
    Ok(c.ary_of_strs(ls, c.str_enc(slf)))
});
cfn!(s_each_line, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_line"), a, NIL); }
    s_lines(c, slf, a, blk)
});
cfn!(s_split, |c, slf, a, blk| {
    check_argc(c, a, 0, 2)?;
    let limit = if a.len() > 1 { num_to_long(c, a[1])? } else { 0 };
    if limit == 1 {
        let arr = if c.str_bytes(slf).is_empty() { c.ary_new() } else { let d = c.str_dup(slf); c.ary_from_vec(vec![d]) };
        if !blk.is_null() { if c.ary_len(arr) > 0 { yield1(c, blk, c.ary_get(arr, 0))?; } return Ok(slf); }
        return Ok(arr);
    }
    if c.str_bytes(slf).is_empty() { return Ok(if blk.is_null() { c.ary_new() } else { slf }); }
    let mut out: Vec<Vec<u8>> = vec![];
    let pat = if a.is_empty() || a[0] == NIL { let fs = gvar_get(c, intern(b"$;"))?; if fs == NIL { c.str_new(b" ") } else { fs } } else { a[0] };
    let pat = get_pat(c, pat)?;
    if is_string(pat) { check_valid(c, pat)?; }
    check_valid(c, slf)?;
    let default_pat = a.is_empty() || a[0] == NIL;
    if is_string(pat) && !(default_pat && c.str_bytes(pat) == b" ") { compat(c, slf, pat)?; }
    // " " in an ASCII-compatible encoding is the one-space pattern without decoding it
    let awk = is_string(pat) && (c.str_bytes(pat) == b" " && (default_pat || crate::builtins::encoding::ascii_compat(c.str_enc(pat))) || { let cs = codes_of(c, pat); cs.len() == 1 && cs[0].0 == 32 });
    let re = if is_string(pat) { NIL } else { regexp::to_regexp(c, pat)? };
    let sp = |x: u8| matches!(x, 9..=13 | 32);
    if awk && c.str_enc(slf) <= ENC_ASCII && limit == 0 && blk.is_null() {
        // the common call: build the result directly (short pieces need no malloc)
        // the pieces are allocated while scanning, so scan a copy (on the stack when short)
        let src = ByteSnap::new(c.str_bytes(slf));
        let (v, enc): (&[u8], u8) = (&src, c.str_enc(slf));
        let n = v.len();
        let arr = c.ary_new();
        let mut k = 0;
        while k < n && sp(v[k]) { k += 1; }
        while k < n {
            let st = k;
            while k < n && !sp(v[k]) { k += 1; }
            let piece = c.str_new_enc(&v[st..k], enc);
            c.ary_push(arr, piece);
            while k < n && sp(v[k]) { k += 1; }
        }
        return Ok(arr);
    }
    // from here to the end of the scan nothing takes the context
    let v = c.str_bytes(slf);
    if awk && c.str_enc(slf) <= ENC_ASCII {
        // UTF-8 / binary / US-ASCII: whitespace bytes are whitespace characters
        let n = v.len();
        let mut k = 0;
        while k < n && sp(v[k]) { k += 1; }
        while k < n {
            if limit > 0 && out.len() as i64 == limit - 1 { out.push(v[k..].to_vec()); break; }
            let st = k;
            while k < n && !sp(v[k]) { k += 1; }
            let en = k;
            out.push(v[st..en].to_vec());
            while k < n && sp(v[k]) { k += 1; }
            if k >= n && en < n { out.push(vec![]); }
        }
    } else if awk {
        // awk mode: split on ASCII whitespace characters (code units for wide encodings)
        let cs = codes_of(c, slf);
        let sp = |x: u32| matches!(x, 9..=13 | 32);
        let mut k = 0;
        while k < cs.len() && sp(cs[k].0) { k += 1; }
        while k < cs.len() {
            if limit > 0 && out.len() as i64 == limit - 1 { out.push(v[cs[k].1..].to_vec()); break; }
            let st = cs[k].1;
            while k < cs.len() && !sp(cs[k].0) { k += 1; }
            let en = if k < cs.len() { cs[k].1 } else { v.len() };
            out.push(v[st..en].to_vec());
            while k < cs.len() && sp(cs[k].0) { k += 1; }
            if k >= cs.len() && en < v.len() { out.push(vec![]); }
        }
    } else if is_string(pat) {
        let p = c.str_bytes(pat);
        if p.is_empty() {
            for ch in c.chars_of(slf) { if limit > 0 && out.len() as i64 == limit - 1 { break; } out.push(ch); }
            let used: usize = out.iter().map(|x| x.len()).sum();
            if used < v.len() && limit > 0 { out.push(v[used..].to_vec()); }
        } else {
            let mut i = 0;
            loop {
                if limit > 0 && out.len() as i64 == limit - 1 { out.push(v[i..].to_vec()); break; }
                match find_bytes(v, p, i) { Some(q) => { out.push(v[i..q].to_vec()); i = q + p.len(); } None => { out.push(v[i..].to_vec()); break; } }
            }
        }
    } else {
        let mut i = 0; let mut start = 0;
        loop {
            if limit > 0 && out.len() as i64 == limit - 1 { out.push(v[start..].to_vec()); break; }
            let Some(g) = regexp::search_raw(re, v, i) else { out.push(v[start..].to_vec()); break; };
            let (ms, me) = g[0].unwrap();
            if me == ms {
                if ms >= v.len() { out.push(v[start..].to_vec()); break; }
                if ms == start { i = ms + char_len_at(v, ms); if ms == 0 { continue; } out.push(v[start..i].to_vec()); start = i; for gi in g.iter().skip(1) { if let Some((x, y)) = gi { out.push(v[*x..*y].to_vec()); } } continue; }
                out.push(v[start..ms].to_vec());
            } else { out.push(v[start..ms].to_vec()); }
            for gi in g.iter().skip(1) { if let Some((x, y)) = gi { out.push(v[*x..*y].to_vec()); } }
            start = me; i = if me == ms { me + char_len_at(v, me) } else { me };
            if i > v.len() { break; }
        }
    }
    if limit == 0 { while out.last().map(|x| x.is_empty()).unwrap_or(false) { out.pop(); } }
    let arr = c.ary_of_strs(out, c.str_enc(slf));
    if !blk.is_null() { let n = c.ary_len(arr); for i in 0..n { yield1(c, blk, c.ary_get(arr, i))?; } return Ok(slf); }
    Ok(arr)
});

/// Pattern argument of sub/gsub/split: a Regexp or a String (via to_str).
fn get_pat(c: &mut Ctx, p: Value) -> RV {
    if regexp::is_regexp(p) || is_string(p) { return Ok(p); }
    if p.is_heap() && respond_to(p, s::to_str, true) { return expect_str(c, p); }
    type_err(c, format!("wrong argument type {} (expected Regexp)", class_name_for_err(p)))
}
/// What expand_repl can raise; it takes no context so that it can read the replacement in place.
enum ReplErr { Undefined(String), BadFormat }
impl ReplErr {
    fn raise(self, c: &mut Ctx) -> Esc {
        match self {
            ReplErr::Undefined(name) => raise_msg(c, c.vm().c_index_error, &format!("undefined group name reference: {}", name)),
            ReplErr::BadFormat => raise_msg(c, c.vm().c_runtime_error, "invalid group name reference format"),
        }
    }
}
/// Expand a replacement string with \1 \0 \& \k<name> \` \' \\
fn expand_repl(rep: &[u8], m_bytes: &[u8], groups: &[Option<(usize, usize)>], re: Value) -> Result<Vec<u8>, ReplErr> {
    let mut out = vec![];
    let mut i = 0;
    while i < rep.len() {
        let x = rep[i];
        if x == b'\\' && i + 1 < rep.len() {
            let y = rep[i + 1];
            let grp = |n: usize, out: &mut Vec<u8>| { if let Some(Some((a, b))) = groups.get(n) { out.extend_from_slice(&m_bytes[*a..*b]); } };
            match y {
                b'0'..=b'9' => { grp((y - b'0') as usize, &mut out); i += 2; }
                b'&' => { grp(0, &mut out); i += 2; }
                b'`' => { if let Some((a, _)) = groups[0] { out.extend_from_slice(&m_bytes[..a]); } i += 2; }
                b'\'' => { if let Some((_, b)) = groups[0] { out.extend_from_slice(&m_bytes[b..]); } i += 2; }
                b'\\' => { out.push(b'\\'); i += 2; }
                b'k' if i + 2 < rep.len() && rep[i + 2] == b'<' => {
                    if let Some(e) = rep[i + 3..].iter().position(|&z| z == b'>') {
                        let name = String::from_utf8_lossy(&rep[i + 3..i + 3 + e]).into_owned();
                        let idx = regexp::rregexp(re).map(|r| r.re.as_ref().unwrap().group_index(&name)).unwrap_or_default();
                        if idx.is_empty() { return Err(ReplErr::Undefined(name)); }
                        for &gi in idx.iter().rev() { if groups.get(gi).map(|g| g.is_some()).unwrap_or(false) { grp(gi, &mut out); break; } }
                        i += 4 + e;
                    } else { return Err(ReplErr::BadFormat); }
                }
                _ => { out.push(x); out.push(y); i += 2; }
            }
        } else { out.push(x); i += 1; }
    }
    Ok(out)
}

/// sub/gsub core: returns (new bytes, changed)
fn subst(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block, global: bool, bang: bool) -> Result<Option<(Vec<u8>, u8)>, Esc> {
    if a.is_empty() || a.len() > 2 { return argc_err(c, a.len(), 1, 2); }
    let pat = a[0];
    let hash_rep = if a.len() == 2 && is_hash(a[1]) { Some(a[1]) } else { None };
    let str_rep = if a.len() == 2 && hash_rep.is_none() { Some(expect_str(c, a[1])?) } else { None };
    if a.len() == 1 && blk.is_null() { return arg_err(c, "wrong number of arguments (given 1, expected 2)"); }
    let src = c.str_bytes(slf).to_vec();
    let pat = get_pat(c, pat)?;
    let lit = if is_string(pat) { Some(c.str_bytes(pat).to_vec()) } else { None };
    let mut re = if lit.is_none() { regexp::to_regexp(c, pat)? } else { NIL };
    if lit.is_some() { compat(c, slf, pat)?; } else { check_valid(c, slf)?; }
    let enc = c.str_enc(slf);
    let mut renc = enc;
    let src_ascii = is_ascii(c.str_bytes(slf));
    let mut srcv = NIL; // frozen copy of the source shared by every MatchData
    let mut out = vec![];
    let mut i = 0; let mut last = 0; let mut changed = false;
    loop {
        let g: Vec<Option<(usize, usize)>> = if let Some(l) = &lit {
            match find_bytes(&src, l, i) { Some(p) => vec![Some((p, p + l.len()))], None => break }
        } else {
            match regexp::search_raw(re, &src, i) { Some(g) => g, None => break }
        };
        let (ms, me) = g[0].unwrap();
        out.extend_from_slice(&src[last..ms]);
        if re == NIL { re = regexp::to_regexp(c, pat)?; }
        if srcv == NIL { srcv = c.str_new_like(slf, src.clone()); obj_set_flags(srcv, FL_FROZEN); }
        let mval = regexp::match_new(re, srcv, g.clone());
        c.last_match = mval;
        if let Some(h) = hash_rep {
            let key = c.str_new_like(slf, src[ms..me].to_vec());
            let v = crate::hash::hash_aref(c, h, key)?;
            let sv = crate::rt::srb_tostr(c, v)?;
            renc = join_enc(c, renc, src_ascii, sv)?; out.extend_from_slice(c.str_bytes(sv));
        } else if let Some(r) = str_rep {
            renc = join_enc(c, renc, src_ascii, r)?; match expand_repl(c.str_bytes(r), &src, &g, re) { Ok(ex) => out.extend(ex), Err(e) => return Err(e.raise(c)) }
        } else {
            let mstr = c.str_new_like(slf, src[ms..me].to_vec());
            let v = yield1(c, blk, mstr)?;
            if bang && c.str_bytes(slf) != &src[..] { return err(c, c.vm().c_runtime_error, "string modified"); }
            c.last_match = mval;
            let sv = crate::rt::srb_tostr(c, v)?;
            renc = join_enc(c, renc, src_ascii, sv)?; out.extend_from_slice(c.str_bytes(sv));
        }
        changed = true;
        last = me;
        if me == ms {
            if ms < src.len() { let l = enc_char_len(&src, ms, enc); out.extend_from_slice(&src[ms..ms + l]); last = ms + l; i = ms + l; } else { i = ms + 1; }
        } else { i = me; }
        if !global || i > src.len() { break; }
    }
    if !changed { c.last_match = NIL; return Ok(None); }
    if last < src.len() { out.extend_from_slice(&src[last..]); }
    Ok(Some((out, renc)))
}
/// Result encoding after splicing replacement `rv` into a subject (rb_enc_check rules).
fn join_enc(c: &mut Ctx, renc: u8, src_ascii: bool, rv: Value) -> Result<u8, Esc> {
    use crate::builtins::encoding as en;
    let re = c.str_enc(rv);
    if re == renc || (is_ascii(c.str_bytes(rv)) && en::ascii_compat(renc) && en::ascii_compat(re)) { return Ok(renc); }
    if src_ascii && en::ascii_compat(renc) && en::ascii_compat(re) { return Ok(re); }
    Err(en::compat_error(c, renc, re))
}
fn sub_result(c: &mut Ctx, slf: Value, r: Option<(Vec<u8>, u8)>) -> Value {
    match r { Some((v, e)) => c.str_from_vec_enc(v, e), None => c.str_copy(slf) }
}
cfn!(s_sub, |c, slf, a, blk| { let r = subst(c, slf, a, blk, false, false)?; Ok(sub_result(c, slf, r)) });
cfn!(s_gsub, |c, slf, a, blk| {
    if a.len() == 1 && blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"gsub"), a, NIL); }
    let r = subst(c, slf, a, blk, true, false)?; Ok(sub_result(c, slf, r))
});
cfn!(s_sub_b, |c, slf, a, blk| { check_frozen(c, slf)?; if a.len() == 1 && blk.is_null() { return argc_err(c, 1, 2, 2); } match subst(c, slf, a, blk, false, true)? { Some((v, e)) => { let r = c.str_modify(slf)?; r.b = v.into(); r.enc = e; Ok(slf) } None => Ok(NIL) } });
cfn!(s_gsub_b, |c, slf, a, blk| { if a.len() == 1 && blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"gsub!"), a, NIL); } check_frozen(c, slf)?; match subst(c, slf, a, blk, true, true)? { Some((v, e)) => { let r = c.str_modify(slf)?; r.b = v.into(); r.enc = e; Ok(slf) } None => Ok(NIL) } });
cfn!(s_scan, |c, slf, a, blk| {
    let p = a[0];
    if !regexp::is_regexp(p) && !is_string(p) && !(p.is_heap() && respond_to(p, s::to_str, true)) {
        return type_err(c, format!("wrong argument type {} (expected Regexp)", class_name_for_err(p)));
    }
    let p = if regexp::is_regexp(p) { p } else { expect_str(c, p)? };
    if is_string(p) { compat(c, slf, p)?; }
    check_valid(c, slf)?;
    let re = regexp::to_regexp(c, p)?;
    let src = c.str_bytes(slf).to_vec();
    let out = c.ary_new();
    let mut i = 0;
    while i <= src.len() {
        let Some(g) = regexp::search_raw(re, &src, i) else { break };
        let (ms, me) = g[0].unwrap();
        let m = regexp::match_new(re, slf, g.clone());
        c.last_match = m;
        let item = if g.len() == 1 { c.str_new_like(slf, src[ms..me].to_vec()) } else {
            let arr = c.ary_new();
            for gi in g.iter().skip(1) { let it = match gi { Some((x, y)) => c.str_new_like(slf, src[*x..*y].to_vec()), None => NIL }; c.ary_push(arr, it); }
            arr
        };
        if !blk.is_null() { yield1(c, blk, item)?; c.last_match = m; } else { c.ary_push(out, item); }
        i = if me == ms { me + if me < src.len() { char_len_at(&src, me) } else { 1 } } else { me };
    }
    if !blk.is_null() { return Ok(slf); }
    Ok(out)
});
/// tr-style character set: (negated, codes).
type TrSet = (bool, Vec<u32>);
fn tr_set(c: &mut Ctx, v: Value, allow_neg: bool) -> Result<TrSet, Esc> {
    check_valid(c, v)?;
    let chars: Vec<u32> = codes_of(c, v).into_iter().map(|x| x.0).collect();
    let (neg, chars) = if allow_neg && chars.len() > 1 && chars[0] == '^' as u32 { (true, &chars[1..]) } else { (false, &chars[..]) };
    let mut out = vec![];
    let mut i = 0;
    while i < chars.len() {
        let (a0, adv) = if chars[i] == '\\' as u32 && i + 1 < chars.len() { (chars[i + 1], 2) } else { (chars[i], 1) };
        if i + adv + 1 < chars.len() && chars[i + adv] == '-' as u32 {
            let j = i + adv + 1;
            let (b0, adv2) = if chars[j] == '\\' as u32 && j + 1 < chars.len() { (chars[j + 1], 2) } else { (chars[j], 1) };
            if a0 > b0 {
                let enc = c.str_enc(v);
                let mut rs = code_to_bytes_lossy(enc, a0); rs.push(b'-'); rs.extend(code_to_bytes_lossy(enc, b0));
                let shown = inspect_bytes(&rs, enc);
                return arg_err(c, format!("invalid range {} in string transliteration", shown));
            }
            for x in a0..=b0 { out.push(x); }
            i = j + adv2;
        } else { out.push(a0); i += adv; }
    }
    Ok((neg, out))
}
fn code_to_bytes_lossy(enc: u8, cp: u32) -> Vec<u8> { crate::builtins::encoding::code_to_bytes(enc, cp).unwrap_or_default() }
fn tr_impl(c: &mut Ctx, slf: Value, from: Value, to: Value, squeeze: bool) -> Result<Vec<u8>, Esc> {
    let from = expect_str(c, from)?; let to = expect_str(c, to)?;
    compat(c, slf, from)?; compat(c, slf, to)?;
    let (neg, f) = tr_set(c, from, true)?;
    let (_, t) = tr_set(c, to, false)?;
    if c.str_bytes(slf).is_empty() { return Ok(vec![]); }
    check_valid(c, slf)?;
    let enc = c.str_enc(slf);
    let mut out = Vec::with_capacity(c.str_bytes(slf).len());
    let mut last: Option<u32> = None;
    for (cp, s, e) in codes_of(c, slf) {
        let pos = f.iter().position(|&x| x == cp);
        let mapped = if neg { if pos.is_none() { Some(t.last().copied()) } else { None } }
                     else { pos.map(|p| if t.is_empty() { None } else { Some(*t.get(p).unwrap_or(t.last().unwrap())) }) };
        match mapped {
            Some(None) => { last = None; }
            Some(Some(m)) => {
                if squeeze && last == Some(m) { continue; }
                match crate::builtins::encoding::code_to_bytes(enc, m) { Some(bs) => out.extend(bs), None => out.extend_from_slice(&c.str_bytes(slf)[s..e]) }
                last = Some(m);
            }
            None => { out.extend_from_slice(&c.str_bytes(slf)[s..e]); last = None; }
        }
    }
    Ok(out)
}
cfn!(s_tr, |c, slf, a, b_| { let v = tr_impl(c, slf, a[0], a[1], false)?; Ok(c.str_new_like(slf, v)) });
cfn!(s_tr_b, |c, slf, a, b_| { let v = tr_impl(c, slf, a[0], a[1], false)?; check_frozen(c, slf)?; if v == c.str_bytes(slf) { return Ok(NIL); } c.str_modify(slf)?.b = v.into(); Ok(slf) });
cfn!(s_tr_s, |c, slf, a, b_| { let v = tr_impl(c, slf, a[0], a[1], true)?; Ok(c.str_new_like(slf, v)) });
cfn!(s_tr_s_b, |c, slf, a, b_| { let v = tr_impl(c, slf, a[0], a[1], true)?; check_frozen(c, slf)?; if v == c.str_bytes(slf) { return Ok(NIL); } c.str_modify(slf)?.b = v.into(); Ok(slf) });
fn in_sets(ch: u32, sets: &[TrSet]) -> bool { sets.iter().all(|(neg, s)| s.contains(&ch) != *neg) }
fn sets_of(c: &mut Ctx, slf: Value, a: &[Value]) -> Result<Vec<TrSet>, Esc> {
    let mut v = vec![];
    for &x in a { let s2 = expect_str(c, x)?; compat(c, slf, s2)?; v.push(tr_set(c, s2, true)?); }
    Ok(v)
}
/// Bytes of slf without (keep=false) / only (keep=true) the characters in the sets.
fn filter_chars(c: &mut Ctx, slf: Value, sets: &[TrSet]) -> Result<Vec<u8>, Esc> {
    check_valid(c, slf)?;
    let mut out = vec![];
    for (cp, s, e) in codes_of(c, slf) { if !in_sets(cp, sets) { out.extend_from_slice(&c.str_bytes(slf)[s..e]); } }
    Ok(out)
}
cfn!(s_delete, |c, slf, a, b_| {
    if c.str_bytes(slf).is_empty() { return Ok(c.str_new_like(slf, vec![])); }
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let sets = sets_of(c, slf, a)?;
    let v = filter_chars(c, slf, &sets)?;
    Ok(c.str_new_like(slf, v))
});
cfn!(s_delete_b, |c, slf, a, b_| {
    if c.str_bytes(slf).is_empty() { return Ok(NIL); }
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let sets = sets_of(c, slf, a)?;
    let v = filter_chars(c, slf, &sets)?;
    if v.len() == c.str_bytes(slf).len() { return Ok(NIL); }
    c.str_modify(slf)?.b = v.into(); Ok(slf)
});
cfn!(s_count, |c, slf, a, b_| {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let sets = sets_of(c, slf, a)?;
    if c.str_bytes(slf).is_empty() { return Ok(Value::fix(0)); }
    check_valid(c, slf)?;
    Ok(Value::fix(codes_of(c, slf).into_iter().filter(|x| in_sets(x.0, &sets)).count() as i64))
});
fn squeeze_impl(c: &mut Ctx, slf: Value, a: &[Value]) -> Result<Vec<u8>, Esc> {
    let sets = sets_of(c, slf, a)?;
    if c.str_bytes(slf).is_empty() { return Ok(vec![]); }
    check_valid(c, slf)?;
    let mut out = vec![]; let mut last = None;
    for (cp, s, e) in codes_of(c, slf) {
        if Some(cp) == last && (sets.is_empty() || in_sets(cp, &sets)) { continue; }
        out.extend_from_slice(&c.str_bytes(slf)[s..e]); last = Some(cp);
    }
    Ok(out)
}
cfn!(s_squeeze, |c, slf, a, b_| { let v = squeeze_impl(c, slf, a)?; Ok(c.str_new_like(slf, v)) });
cfn!(s_squeeze_b, |c, slf, a, b_| { let v = squeeze_impl(c, slf, a)?; check_frozen(c, slf)?; if v.len() == c.str_bytes(slf).len() { return Ok(NIL); } c.str_modify(slf)?.b = v.into(); Ok(slf) });
fn justify(c: &mut Ctx, slf: Value, a: &[Value], mode: u8) -> RV {
    let w = num_to_long(c, a[0])?;
    let pad = if a.len() > 1 { let p = expect_str(c, a[1])?; c.str_bytes(p).to_vec() } else { b" ".to_vec() };
    if pad.is_empty() { return arg_err(c, "zero width padding"); }
    let n = c.str_char_len(slf) as i64;
    if w <= n { return Ok(c.str_new_like(slf, c.str_bytes(slf).to_vec())); }
    let total = (w - n) as usize;
    let padchars: Vec<Vec<u8>> = { let ps = c.str_new(&pad); c.chars_of(ps) };
    let mk = |k: usize| -> Vec<u8> { let mut v = vec![]; for i in 0..k { v.extend_from_slice(&padchars[i % padchars.len()]); } v };
    let (l, r) = match mode { b'l' => (0, total), b'r' => (total, 0), _ => (total / 2, total - total / 2) };
    let mut out = mk(l); out.extend_from_slice(c.str_bytes(slf)); out.extend(mk(r));
    Ok(c.str_new_like(slf, out))
}
cfn!(s_ljust, |c, slf, a, b_| justify(c, slf, a, b'l'));
cfn!(s_rjust, |c, slf, a, b_| justify(c, slf, a, b'r'));
cfn!(s_center, |c, slf, a, b_| justify(c, slf, a, b'c'));
cfn!(s_empty, |c, slf, a, b_| Ok(Value::bool(c.str_bytes(slf).is_empty())));
cfn!(s_replace, |c, slf, a, b_| { let o = expect_str(c, a[0])?; let nb = c.str_bytes(o).to_vec(); let e = c.str_enc(o); let r = c.str_modify(slf)?; r.b = nb.into(); r.enc = e; Ok(slf) });
cfn!(s_clear, |c, slf, a, b_| { c.str_modify(slf)?.b.clear(); Ok(slf) });
cfn!(s_insert, |c, slf, a, b_| {
    let idx = num_to_long(c, a[0])?; let o = expect_str(c, a[1])?;
    let n = c.str_char_len(slf) as i64;
    let pos = if idx < 0 { idx + n + 1 } else { idx };
    if pos < 0 || pos > n { return err(c, c.vm().c_index_error, format!("index {} out of string", idx)); }
    let bp = char_to_byte(c, slf, pos as usize).unwrap();
    let ob = c.str_bytes(o).to_vec();
    c.str_modify(slf)?.b.splice(bp..bp, ob);
    Ok(slf)
});
cfn!(s_prepend, |c, slf, a, b_| { let mut pre = vec![]; for &x in a { let o = expect_str(c, x)?; pre.extend_from_slice(c.str_bytes(o)); } c.str_modify(slf)?.b.splice(0..0, pre); Ok(slf) });
cfn!(s_ord, |c, slf, a, b_| {
    let v = c.str_bytes(slf);
    if v.is_empty() { return arg_err(c, "empty string"); }
    let enc = c.str_enc(slf);
    match crate::builtins::encoding::precise_len(enc, v, 0) {
        Some(l) => Ok(Value::fix(crate::builtins::encoding::char_code(enc, &v[..l]) as i64)),
        None => arg_err(c, format!("invalid byte sequence in {}", crate::builtins::encoding::enc_name(enc))),
    }
});
cfn!(s_sum, |c, slf, a, b_| { let bits = if a.is_empty() { 16 } else { num_to_long(c, a[0])? }; let s2: u64 = c.str_bytes(slf).iter().map(|&x| x as u64).sum(); Ok(Value::fix(if bits > 0 && bits < 63 { (s2 & ((1 << bits) - 1)) as i64 } else { s2 as i64 })) });
cfn!(s_succ, |c, slf, a, b_| Ok(c.str_new_like(slf, succ_enc(c.str_bytes(slf), c.str_enc(slf)))));
cfn!(s_succ_b, |c, slf, a, b_| { let v = succ_enc(c.str_bytes(slf), c.str_enc(slf)); c.str_modify(slf)?.b = v.into(); Ok(slf) });
cfn!(s_upto, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, s::upto, a, NIL); }
    let last = expect_str(c, a[0])?;
    compat(c, slf, last)?;
    let excl = a.len() > 1 && a[1].truthy();
    let lb = c.str_bytes(last).to_vec();
    // numeric strings iterate numerically
    let all_digits = |v: &[u8]| !v.is_empty() && v.iter().all(|x| x.is_ascii_digit());
    if all_digits(c.str_bytes(slf)) && all_digits(&lb) && c.str_bytes(slf).len() <= lb.len() {
        let (x, y): (i64, i64) = (String::from_utf8_lossy(c.str_bytes(slf)).parse().unwrap_or(0), String::from_utf8_lossy(&lb).parse().unwrap_or(0));
        let w = c.str_bytes(slf).len();
        let mut i = x;
        while if excl { i < y } else { i <= y } { let s2 = c.str_new(format!("{:0w$}", i, w = w).as_bytes()); yield1(c, blk, s2)?; i += 1; }
        return Ok(slf);
    }
    if c.str_bytes(slf).len() == 1 && lb.len() == 1 && c.str_bytes(slf)[0] < 0x80 && lb[0] < 0x80 {
        let (x, y) = (c.str_bytes(slf)[0], lb[0]);
        if x > y || (excl && x == y) { return Ok(slf); }
        let mut ch = x;
        loop { if excl && ch == y { break; } let s2 = c.str_new_like(slf, vec![ch]); yield1(c, blk, s2)?; if ch == y { break; } ch += 1; }
        return Ok(slf);
    }
    let mut cur = c.str_bytes(slf).to_vec();
    if cur.len() > lb.len() { return Ok(slf); }
    loop {
        if excl && cur == lb { break; }
        let s2 = c.str_new_like(slf, cur.clone());
        yield1(c, blk, s2)?;
        if cur == lb || cur.len() > lb.len() { break; }
        cur = succ_enc(&cur, c.str_enc(slf));
        if cur.len() > lb.len() { break; }
    }
    Ok(slf)
});
cfn!(s_hex, |c, slf, a, b_| { need_ascii_compat(c, slf)?; Ok(str_to_inum(c.str_bytes(slf), 16, false).unwrap_or(Value::fix(0))) });
cfn!(s_oct, |c, slf, a, b_| {
    need_ascii_compat(c, slf)?;
    let v = String::from_utf8_lossy(c.str_bytes(slf)).trim().to_ascii_lowercase();
    let (neg, body) = if let Some(r) = v.strip_prefix('-') { (true, r.to_string()) } else { (false, v.clone()) };
    let base = if body.starts_with("0x") { 16 } else if body.starts_with("0b") { 2 } else if body.starts_with("0o") || body.starts_with('0') { 8 } else { 8 };
    let r = str_to_inum(body.as_bytes(), if body.starts_with("0x") || body.starts_with("0b") || body.starts_with("0o") { 0 } else { base }, false).unwrap_or(Value::fix(0));
    if neg { funcall(c, r, s::uminus, &[], std::ptr::null()) } else { Ok(r) }
});
cfn!(s_freeze, |c, slf, a, b_| { obj_set_flags(slf, FL_FROZEN); Ok(slf) });
cfn!(s_uplus, |c, slf, a, b_| if is_frozen(slf) { Ok(c.str_copy(slf)) } else { Ok(slf) });
cfn!(s_uminus, |c, slf, a, b_| {
    let has_iv = c.vm().gen_ivars.get(&(slf.raw() as usize)).map(|v| !v.is_empty()).unwrap_or(false);
    if obj_klass(slf) != c.vm().c_string || has_iv {
        if is_frozen(slf) && !has_iv { return Ok(slf); }
        let v = c.str_copy(slf); obj_set_klass(v, obj_klass(slf)); obj_set_flags(v, FL_FROZEN); return Ok(v);
    }
    let key = c.str_bytes(slf).to_vec();
    if let Some(&v) = c.vm().frozen_lits.get(&key) { if c.str_enc(v) == c.str_enc(slf) { return Ok(v); } }
    // a bare frozen receiver becomes the pool entry itself
    let v = if is_frozen(slf) { slf } else { let v = c.str_new_enc(&key, c.str_enc(slf)); obj_set_flags(v, FL_FROZEN); v };
    c.vm_mut().frozen_lits.insert(key, v);
    Ok(v)
});
cfn!(s_encoding, |c, slf, a, b_| crate::builtins::misc::encoding_value(c, c.str_enc(slf)));
cfn!(s_force_encoding, |c, slf, a, b_| { let e = crate::builtins::encoding::to_enc(c, a[0])?; check_frozen(c, slf)?; c.rstring_mut(slf).enc = e; Ok(slf) });
cfn!(s_b, |c, slf, a, b_| Ok(c.str_copy_enc(slf, ENC_BINARY)));
cfn!(s_valid_encoding, |c, slf, a, b_| Ok(Value::bool(crate::builtins::encoding::is_valid(c.str_enc(slf), c.str_bytes(slf)))));
cfn!(s_ascii_only, |c, slf, a, b_| Ok(Value::bool(crate::builtins::encoding::ascii_compat(c.str_enc(slf)) && is_ascii(c.str_bytes(slf)))));
cfn!(s_unicode_normalize, |c, slf, a, b_| { unicode_check(c, slf)?; Ok(c.str_new_like(slf, c.str_bytes(slf).to_vec())) });
cfn!(s_unicode_normalize_b, |c, slf, a, b_| { unicode_check(c, slf)?; check_frozen(c, slf)?; Ok(slf) });
cfn!(s_unicode_normalized_p, |c, slf, a, b_| { unicode_check(c, slf)?; Ok(TRUE) });
fn unicode_check(c: &mut Ctx, slf: Value) -> Result<(), Esc> {
    let e = c.str_enc(slf);
    if !crate::builtins::encoding::is_unicode(e) && !(e == ENC_ASCII || is_ascii(c.str_bytes(slf)) && crate::builtins::encoding::ascii_compat(e)) {
        return err(c, c.vm().c_compat_error, format!("Unicode Normalization not appropriate for {}", crate::builtins::encoding::enc_name_msg(e)));
    }
    Ok(())
}
/// String#encode core: returns (bytes, encoding).
fn encode_impl(c: &mut Ctx, slf: Value, a: &[Value]) -> Result<(Vec<u8>, u8), Esc> {
    use crate::builtins::encoding as en;
    let kw = kw_arg(a);
    let pos: &[Value] = if kw.is_some() { &a[..a.len() - 1] } else { a };
    let pos: &[Value] = if kw.is_none() && !pos.is_empty() && is_hash(*pos.last().unwrap()) { &pos[..pos.len() - 1] } else { pos };
    let opth = if kw.is_some() { kw } else if !a.is_empty() && is_hash(*a.last().unwrap()) { Some(*a.last().unwrap()) } else { None };
    if pos.len() > 2 { return argc_err(c, pos.len(), 0, 2); }
    let o = match opth { Some(h) => en::parse_tc_opts(c, h)?, None => en::TcOpts::default() };
    let src_enc = c.str_enc(slf);
    let to = match pos.first() {
        None => match en::default_internal() { Some(e) => Ok(e), None => Ok(src_enc) },
        Some(&v) => en::to_enc_opt(c, v)?,
    };
    let from = match pos.get(1) { None => Ok(src_enc), Some(&v) => en::to_enc_opt(c, v)? };
    let (to, from) = match (to, from) {
        (Ok(t), Ok(f)) => (t, f),
        (t, f) => {
            let tn = match &t { Ok(e) => en::enc_name(*e).to_string(), Err(n) => n.clone() };
            let fnm = match &f { Ok(e) => en::enc_name(*e).to_string(), Err(n) => n.clone() };
            if tn.eq_ignore_ascii_case(&fnm) { return Ok((c.str_bytes(slf).to_vec(), src_enc)); }
            return Err(en::converter_not_found(c, &fnm, &tn));
        }
    };
    let plain = o.xml == 0 && o.newline == 0;
    if from == to && plain && !o.invalid_replace { return Ok((c.str_bytes(slf).to_vec(), to)); }
    if from == to && plain && o.invalid_replace && en::is_valid(from, c.str_bytes(slf)) { return Ok((c.str_bytes(slf).to_vec(), to)); }
    if plain && from != to && en::ascii_compat(from) && en::ascii_compat(to) && is_ascii(c.str_bytes(slf)) && !en::is_dummy(to) && to != ENC_BINARY
        || plain && to == ENC_BINARY && en::ascii_compat(from) && is_ascii(c.str_bytes(slf)) { return Ok((c.str_bytes(slf).to_vec(), to)); }
    let src = c.str_bytes(slf).to_vec();
    let out = en::transcode(c, &src, from, to, &o)?;
    Ok((out, to))
}
cfn!(s_encode, |c, slf, a, b_| { let (v, e) = encode_impl(c, slf, a)?; Ok(c.str_from_vec_enc(v, e)) });
cfn!(s_encode_b, |c, slf, a, b_| { check_frozen(c, slf)?; let (v, e) = encode_impl(c, slf, a)?; let r = c.rstring_mut(slf); r.b = v.into(); r.enc = e; Ok(slf) });
/// scrub core: None if the string is valid.
fn scrub_impl(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> Result<Option<Vec<u8>>, Esc> {
    use crate::builtins::encoding as en;
    let enc = c.str_enc(slf);
    check_argc(c, a, 0, 1)?;
    if en::is_valid(enc, c.str_bytes(slf)) { return Ok(None); }
    let rep = match a.first() { Some(&r) if r != NIL => { let r = expect_str(c, r)?; Some(r) } _ => None };
    if let Some(r) = rep {
        let re = c.str_enc(r);
        if !en::is_valid(re, c.str_bytes(r)) { return arg_err(c, format!("replacement must be valid byte sequence '{}'", inspect_bytes(c.str_bytes(r), c.str_enc(r)))); }
        if re != enc && !(is_ascii(c.str_bytes(r)) && en::ascii_compat(enc)) { return Err(en::compat_error(c, enc, re)); }
    }
    let default_rep = if en::is_unicode(enc) { en::code_to_bytes(enc, 0xfffd).unwrap() } else { en::from_unicode(enc, '?' as u32).unwrap_or(vec![b'?']) };
    let src = c.str_bytes(slf).to_vec();
    let mut out = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        if let Some(l) = en::precise_len(enc, &src, i) { out.extend_from_slice(&src[i..i + l]); i += l; continue; }
        let n = en::invalid_len(enc, &src, i);
        if let Some(r) = rep { out.extend_from_slice(c.str_bytes(r)); }
        else if !blk.is_null() {
            let bad = c.str_new_enc(&src[i..i + n], enc);
            let v = yield1(c, blk, bad)?;
            let s2 = expect_str(c, v)?;
            if !en::is_valid(c.str_enc(s2), c.str_bytes(s2)) { return arg_err(c, format!("replacement must be valid byte sequence '{}'", inspect_bytes(c.str_bytes(s2), c.str_enc(s2)))); }
            out.extend_from_slice(c.str_bytes(s2));
        } else { out.extend_from_slice(&default_rep); }
        i += n;
    }
    Ok(Some(out))
}
cfn!(s_scrub, |c, slf, a, blk| { let v = scrub_impl(c, slf, a, blk)?; Ok(c.str_new_like(slf, v.unwrap_or_else(|| c.str_bytes(slf).to_vec()))) });
cfn!(s_scrub_b, |c, slf, a, blk| { check_frozen(c, slf)?; if let Some(v) = scrub_impl(c, slf, a, blk)? { c.str_modify(slf)?.b = v.into(); } Ok(slf) });
/// to_str-convertible other string for casecmp, or None.
fn casecmp_other(c: &mut Ctx, slf: Value, o: Value) -> Result<Option<Value>, Esc> {
    let o = if is_string(o) { o } else if o.is_heap() && respond_to(o, s::to_str, true) { expect_str(c, o)? } else { return type_err(c, format!("no implicit conversion of {} into String", class_name_for_err(o))) };
    if crate::builtins::encoding::compatible(c, slf, o).is_none() { return Ok(None); }
    Ok(Some(o))
}
/// ASCII case folding for casecmp.
fn casefold_ascii(c: &Ctx, v: Value) -> Vec<u8> {
    let r = c.rstring(v);
    if crate::builtins::encoding::ascii_compat(r.enc) { return r.b.iter().map(|c| c.to_ascii_lowercase()).collect(); }
    let mut out = vec![];
    for (cp, s0, e0) in codes_of(c, v) {
        if cp < 128 { out.extend(crate::builtins::encoding::code_to_bytes(r.enc, (cp as u8).to_ascii_lowercase() as u32).unwrap_or_default()); }
        else { out.extend_from_slice(&r.b[s0..e0]); }
    }
    out
}
cfn!(s_casecmp, |c, slf, a, b_| {
    if !is_string(a[0]) && !(a[0].is_heap() && respond_to(a[0], s::to_str, true)) { return Ok(NIL); }
    let Some(o) = casecmp_other(c, slf, a[0])? else { return Ok(NIL) };
    let (x, y) = (casefold_ascii(c, slf), casefold_ascii(c, o));
    Ok(Value::fix(x.cmp(&y) as i64))
});
cfn!(s_casecmp_p, |c, slf, a, b_| {
    if !is_string(a[0]) && !(a[0].is_heap() && respond_to(a[0], s::to_str, true)) { return Ok(NIL); }
    let Some(o) = casecmp_other(c, slf, a[0])? else { return Ok(NIL) };
    Ok(Value::bool(fold_case(c.str_bytes(slf), c.str_enc(slf)) == fold_case(c.str_bytes(o), c.str_enc(o))))
});
/// [v[..ms], v[ms..me], v[me..]] as Strings of slf's encoding.
fn split3(c: &mut Ctx, slf: Value, v: &[u8], ms: usize, me: usize) -> Value {
    let (x, y, z) = (c.str_new_like(slf, v[..ms].to_vec()), c.str_new_like(slf, v[ms..me].to_vec()), c.str_new_like(slf, v[me..].to_vec()));
    c.ary_from_vec(vec![x, y, z])
}
cfn!(s_partition, |c, slf, a, b_| {
    let v = c.str_bytes(slf).to_vec();
    let (ms, me) = if regexp::is_regexp(a[0]) {
        match regexp::search(c, a[0], slf, 0, true)? { m if m == NIL => (usize::MAX, 0), m => regexp::rmatch(m).unwrap().groups[0].unwrap() }
    } else { let p = expect_str(c, a[0])?; match find_bytes(&v, c.str_bytes(p), 0) { Some(i) => (i, i + c.str_bytes(p).len()), None => (usize::MAX, 0) } };
    if ms == usize::MAX { let (x, y, z) = (c.str_new_like(slf, v), c.str_new(b""), c.str_new(b"")); return Ok(c.ary_from_vec(vec![x, y, z])); }
    Ok(split3(c, slf, &v, ms, me))
});
cfn!(s_rpartition, |c, slf, a, b_| {
    let v = c.str_bytes(slf).to_vec();
    let found = if regexp::is_regexp(a[0]) {
        let mut p = v.len() as i64; let mut r = None;
        while p >= 0 { if let Some(g) = regexp::rregexp(a[0]).unwrap().re.as_ref().unwrap().match_at(&v, p as usize) { r = Some(g[0].unwrap()); break; } p -= 1; }
        r
    } else { let p = expect_str(c, a[0])?; rfind_bytes(&v, c.str_bytes(p), v.len()).map(|i| (i, i + c.str_bytes(p).len())) };
    match found {
        None => { let (x, y, z) = (c.str_new(b""), c.str_new(b""), c.str_new_like(slf, v)); Ok(c.ary_from_vec(vec![x, y, z])) }
        Some((ms, me)) => Ok(split3(c, slf, &v, ms, me)),
    }
});
cfn!(s_delete_prefix, |c, slf, a, b_| { let p = expect_str(c, a[0])?; let v = c.str_bytes(slf); Ok(c.str_new_like(slf, if v.starts_with(c.str_bytes(p)) { v[c.str_bytes(p).len()..].to_vec() } else { v.to_vec() })) });
cfn!(s_delete_suffix, |c, slf, a, b_| { let p = expect_str(c, a[0])?; let v = c.str_bytes(slf); Ok(c.str_new_like(slf, if v.ends_with(c.str_bytes(p)) { v[..v.len() - c.str_bytes(p).len()].to_vec() } else { v.to_vec() })) });
cfn!(s_delete_prefix_b, |c, slf, a, b_| { let p = expect_str(c, a[0])?; check_frozen(c, slf)?; let n = c.str_bytes(p).len(); if c.str_bytes(slf).starts_with(c.str_bytes(p)) { c.rstring_mut(slf).b.remove_front(n); Ok(slf) } else { Ok(NIL) } });
cfn!(s_delete_suffix_b, |c, slf, a, b_| { let p = expect_str(c, a[0])?; check_frozen(c, slf)?; let n = c.str_bytes(p).len(); if c.str_bytes(slf).ends_with(c.str_bytes(p)) { let l = c.str_bytes(slf).len(); c.rstring_mut(slf).b.truncate(l - n); Ok(slf) } else { Ok(NIL) } });
cfn!(s_slice_b, |c, slf, a, b_| {
    let r = str_aref(c, slf, a)?;
    if r == NIL { return Ok(NIL); }
    let lm = c.last_match;
    if regexp::is_regexp(a[0]) {
        // $~ keeps the pre-mutation text
        if regexp::rmatch(lm).map(|md| md.str == slf).unwrap_or(false) { let cp = c.str_copy(slf); obj_set_flags(cp, FL_FROZEN); if let Some(md) = regexp::rmatch(lm) { md.str = cp; } }
    }
    let mut args: Vec<Value> = a.to_vec(); args.push(c.str_new(b""));
    s_aset(c, slf, &args, b_)?;
    if regexp::is_regexp(a[0]) { c.last_match = lm; }
    Ok(r)
});
cfn!(s_getbyte, |c, slf, a, b_| { let i = num_to_long(c, a[0])?; let n = c.str_bytes(slf).len() as i64; let i = if i < 0 { i + n } else { i }; Ok(if i >= 0 && i < n { Value::fix(c.str_bytes(slf)[i as usize] as i64) } else { NIL }) });
cfn!(s_setbyte, |c, slf, a, b_| { let i = num_to_long(c, a[0])?; let v = num_to_long(c, a[1])?; let n = c.str_bytes(slf).len() as i64; let i2 = if i < 0 { i + n } else { i }; if i2 < 0 || i2 >= n { return err(c, c.vm().c_index_error, format!("index {} out of string", i)); } c.str_modify(slf)?.b[i2 as usize] = v as u8; Ok(a[1]) });
cfn!(s_byteslice, |c, slf, a, b_| {
    let n = c.str_bytes(slf).len() as i64;
    check_argc(c, a, 1, 2)?;
    let single = a.len() == 1 && !is_range(a[0]);
    let (st, ln) = if a.len() == 2 { (num_to_long(c, a[0])?, num_to_long(c, a[1])?) } else if is_range(a[0]) { match crate::range::range_beg_len(c, a[0], n, 0)? { Some(x) => x, None => return Ok(NIL) } } else { (num_to_long(c, a[0])?, 1) };
    let st = if st < 0 { st + n } else { st };
    if st < 0 || st > n || ln < 0 || (single && st == n) { return Ok(NIL); }
    let e = (st + ln).min(n);
    Ok(c.str_new_like(slf, c.str_bytes(slf)[st as usize..e as usize].to_vec()))
});
cfn!(s_unpack, |c, slf, a, b_| { if a.is_empty() || (a.len() == 1 && kw_arg(a).is_some()) { return argc_err(c, 0, 1, 1); } crate::builtins::misc::unpack(c, slf, a) });
cfn!(s_unpack1, |c, slf, a, b_| { if a.is_empty() || (a.len() == 1 && kw_arg(a).is_some()) { return argc_err(c, 0, 1, 1); } let r = crate::builtins::misc::unpack(c, slf, a)?; Ok(c.ary_get(r, 0)) });
cfn!(s_crypt, |c, slf, a, b_| Ok(c.str_new_like(slf, c.str_bytes(slf).to_vec())));
/// Extended grapheme clusters (approximation of UAX #29: CRLF, combining marks, ZWJ
/// sequences, variation selectors, emoji modifiers, regional-indicator pairs, Hangul).
fn graphemes(c: &Ctx, v: Value) -> Vec<(usize, usize)> {
    let r = c.rstring(v);
    let off = char_offsets_of(&r.b, r.enc);
    let n = off.len() - 1;
    let uni = crate::builtins::encoding::is_unicode(r.enc);
    let cp = |k: usize| if uni && crate::builtins::encoding::precise_len(r.enc, &r.b, off[k]) == Some(off[k + 1] - off[k]) { Some(crate::builtins::encoding::char_code(r.enc, &r.b[off[k]..off[k + 1]])) } else { None };
    let extend = |u: u32| matches!(u, 0x300..=0x36f | 0x483..=0x489 | 0x591..=0x5bd | 0x610..=0x61a | 0x64b..=0x65f | 0x900..=0x903 | 0x93a..=0x94f
        | 0x1ab0..=0x1aff | 0x1dc0..=0x1dff | 0x200c | 0x20d0..=0x20ff | 0x302a..=0x302f | 0x3099 | 0x309a | 0xfe00..=0xfe0f | 0xfe20..=0xfe2f
        | 0x1f3fb..=0x1f3ff | 0xe0020..=0xe007f | 0xe0100..=0xe01ef);
    let hangul_l = |u: u32| (0x1100..=0x115f).contains(&u);
    let hangul_vt = |u: u32| (0x1160..=0x11ff).contains(&u);
    let mut out = vec![];
    let mut k = 0;
    while k < n {
        let st = k;
        let c0 = cp(k);
        k += 1;
        if c0 == Some(0x0d) && k < n && cp(k) == Some(0x0a) { k += 1; out.push((off[st], off[k])); continue; }
        if matches!(c0, Some(0x0a) | Some(0x0d)) || c0.map(|u| u < 0x20 || u == 0x7f).unwrap_or(true) { out.push((off[st], off[k])); continue; }
        if let Some(u) = c0 {
            if (0x1f1e6..=0x1f1ff).contains(&u) && k < n && cp(k).map(|w| (0x1f1e6..=0x1f1ff).contains(&w)).unwrap_or(false) { k += 1; }
            if hangul_l(u) { while k < n && cp(k).map(|w| hangul_l(w) || hangul_vt(w) || (0xac00..=0xd7a3).contains(&w)).unwrap_or(false) { k += 1; } }
        }
        loop {
            if k >= n { break; }
            match cp(k) {
                Some(w) if extend(w) => k += 1,
                Some(0x200d) => { k += 1; if k < n && cp(k).map(|w| w >= 0x2000).unwrap_or(false) { k += 1; } }
                Some(w) if hangul_vt(w) => k += 1,
                _ => break,
            }
        }
        out.push((off[st], off[k]));
    }
    out
}
/// Grapheme clusters of a UTF-8 string via the regex engine's \X.
fn graphemes_re(c: &mut Ctx, v: Value) -> Result<Vec<(usize, usize)>, Esc> {
    let re = regexp::regexp_new(c, b"\\X", 0)?;
    let src = c.str_bytes(v).to_vec();
    let mut out = vec![];
    let mut i = 0;
    while i < src.len() {
        match regexp::search_raw(re, &src, i) {
            Some(g) => { let (s0, e0) = g[0].unwrap(); if e0 <= i { break; } out.push((s0, e0)); i = e0; }
            None => break,
        }
    }
    Ok(out)
}
cfn!(s_grapheme_clusters, |c, slf, a, blk| {
    let gs = if c.str_enc(slf) == ENC_UTF8 && crate::builtins::encoding::is_valid(ENC_UTF8, c.str_bytes(slf)) { graphemes_re(c, slf)? } else { graphemes(c, slf) };
    let arr = c.ary_new();
    for (s0, e0) in gs { let p = c.str_sub(slf, s0..e0); c.ary_push(arr, p); }
    if !blk.is_null() { for i in 0..c.ary_len(arr) { yield1(c, blk, c.ary_get(arr, i))?; } return Ok(slf); }
    Ok(arr)
});
cfn!(s_each_grapheme, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_grapheme_cluster"), &[], NIL); }
    s_grapheme_clusters(c, slf, a, blk)
});
cfn!(s_each_codepoint, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_codepoint"), &[], NIL); }
    s_codepoints(c, slf, a, blk)
});
cfn!(s_codepoints, |c, slf, a, blk| {
    check_valid(c, slf)?;
    let out = c.ary_new();
    for (cp, _, _) in codes_of(c, slf) { c.ary_push(out, Value::fix(cp as i64)); }
    if !blk.is_null() { let n = c.ary_len(out); for k in 0..n { yield1(c, blk, c.ary_get(out, k))?; } return Ok(slf); }
    Ok(out)
});
cfn!(s_squish, |c, slf, a, b_| Ok(slf));
/// Digits with single underscores between them; returns (digits, end).
fn scan_digits(v: &[u8], mut i: usize) -> (String, usize) {
    let mut d = String::new();
    while i < v.len() {
        if v[i].is_ascii_digit() { d.push(v[i] as char); i += 1; }
        else if v[i] == b'_' && !d.is_empty() && i + 1 < v.len() && v[i + 1].is_ascii_digit() { i += 1; }
        else { break; }
    }
    (d, i)
}
/// Unsigned decimal number [digits][.digits][e[+-]digits] as exact (num, den); None if no digits.
fn scan_decimal(v: &[u8], i: usize) -> Option<(crate::bignum::BigInt, crate::bignum::BigInt, usize, bool)> {
    use crate::bignum::BigInt;
    let (ip, mut j) = scan_digits(v, i);
    let mut frac = String::new();
    let mut is_float = false;
    if j < v.len() && v[j] == b'.' && j + 1 < v.len() && v[j + 1].is_ascii_digit() { let (f, k) = scan_digits(v, j + 1); frac = f; j = k; is_float = true; }
    if ip.is_empty() && frac.is_empty() { return None; }
    let mut exp: i64 = 0;
    if j < v.len() && (v[j] == b'e' || v[j] == b'E') {
        let mut k = j + 1; let mut neg = false;
        if k < v.len() && (v[k] == b'+' || v[k] == b'-') { neg = v[k] == b'-'; k += 1; }
        let (e, k2) = scan_digits(v, k);
        if !e.is_empty() { exp = e.parse::<i64>().unwrap_or(i64::MAX / 4).min(100000); if neg { exp = -exp; } j = k2; is_float = true; }
    }
    let digits = format!("{}{}", ip, frac);
    let mut num = BigInt::parse(if digits.is_empty() { "0" } else { &digits }, 10)?;
    let mut den = BigInt::from_i64(10).pow(frac.len() as u64);
    if exp > 0 { num = num.mul(&BigInt::from_i64(10).pow(exp as u64)); } else if exp < 0 { den = den.mul(&BigInt::from_i64(10).pow((-exp) as u64)); }
    Some((num, den, j, is_float))
}
fn kernel_call(c: &mut Ctx, name: &str, args: &[Value]) -> RV { let m = c.vm().main_obj; funcall(c, m, intern(name.as_bytes()), args, std::ptr::null()) }
/// [sign] decimal [/ decimal] as a Rational; (value, end, kind 0=int 1=float 2=rational).
fn scan_rational(c: &mut Ctx, v: &[u8], i: usize) -> Result<Option<(Value, usize, u8)>, Esc> {
    use crate::bignum::big_to_value;
    let mut j = i; let mut neg = false;
    if j < v.len() && (v[j] == b'+' || v[j] == b'-') { neg = v[j] == b'-'; j += 1; }
    let Some((mut n, mut d, k, fl)) = scan_decimal(v, j) else { return Ok(None) };
    j = k;
    let mut kind = if fl { 1 } else { 0 };
    if j < v.len() && v[j] == b'/' {
        if let Some((n2, d2, k2, _)) = scan_decimal(v, j + 1) { n = n.mul(&d2); d = d.mul(&n2); j = k2; kind = 2; }
    }
    if neg { n = n.neg(); }
    let r = kernel_call(c, "Rational", &[big_to_value(n), big_to_value(d)])?;
    Ok(Some((r, j, kind)))
}
cfn!(s_to_r, |c, slf, a, b_| {
    let v = c.str_bytes(slf).to_vec();
    let mut i = 0; while i < v.len() && is_ws(v[i]) { i += 1; }
    match scan_rational(c, &v, i)? { Some((r, _, _)) => Ok(r), None => kernel_call(c, "Rational", &[Value::fix(0), Value::fix(1)]) }
});
/// A to_c component in its natural class (Integer, Float or Rational).
fn natural(c: &mut Ctx, r: Value, kind: u8) -> RV {
    match kind { 0 => funcall(c, r, intern(b"to_i"), &[], std::ptr::null()), 1 => funcall(c, r, intern(b"to_f"), &[], std::ptr::null()), _ => Ok(r) }
}
cfn!(s_to_c, |c, slf, a, b_| {
    let v = c.str_bytes(slf).to_vec();
    let mut i = 0; while i < v.len() && is_ws(v[i]) { i += 1; }
    let zero = Value::fix(0);
    let is_i = |k: usize| k < v.len() && (v[k] == b'i' || v[k] == b'j' || v[k] == b'I' || v[k] == b'J');
    // bare "i" / "+i" / "-i"
    let sign_only = |k: usize| -> Option<(i64, usize)> {
        let mut k = k; let mut s = 1;
        if k < v.len() && (v[k] == b'+' || v[k] == b'-') { if v[k] == b'-' { s = -1; } k += 1; }
        if is_i(k) { Some((s, k + 1)) } else { None }
    };
    if let Some((s, _)) = sign_only(i) { return kernel_call(c, "Complex", &[zero, Value::fix(s)]); }
    let Some((r1, j, k1)) = scan_rational(c, &v, i)? else { return kernel_call(c, "Complex", &[zero, zero]) };
    let x = natural(c, r1, k1)?;
    if is_i(j) { return kernel_call(c, "Complex", &[zero, x]); }
    if j < v.len() && v[j] == b'@' {
        if let Some((r2, _, k2)) = scan_rational(c, &v, j + 1)? { let y = natural(c, r2, k2)?; let cx = const_get_at(c.vm().c_object, intern(b"Complex")).unwrap_or(NIL); return funcall(c, cx, intern(b"polar"), &[x, y], std::ptr::null()); }
        return kernel_call(c, "Complex", &[x, zero]);
    }
    if j < v.len() && (v[j] == b'+' || v[j] == b'-') {
        if let Some((s, _)) = sign_only(j) { return kernel_call(c, "Complex", &[x, Value::fix(s)]); }
        if let Some((r2, k, k2)) = scan_rational(c, &v, j)? {
            if is_i(k) { let y = natural(c, r2, k2)?; return kernel_call(c, "Complex", &[x, y]); }
        }
    }
    kernel_call(c, "Complex", &[x, zero])
});
// String#undump.
cfn!(s_undump, |c, slf, a, b_| {
    let v = c.str_bytes(slf).to_vec();
    let bad = |c: &mut Ctx| -> RV { err(c, c.vm().c_runtime_error, "invalid dumped string; not wrapped with '\"' nor '\"...\".force_encoding(\"...\")' form") };
    if v.len() < 2 || v[0] != b'"' { return bad(c); }
    let mut out = vec![]; let mut i = 1; let mut closed = false; let mut uni = false;
    while i < v.len() {
        let x = v[i];
        if x == b'"' { closed = true; i += 1; break; }
        if x != b'\\' { out.push(x); i += 1; continue; }
        i += 1;
        if i >= v.len() { return err(c, c.vm().c_runtime_error, "invalid escape"); }
        let y = v[i]; i += 1;
        match y {
            b'n' => out.push(b'\n'), b't' => out.push(b'\t'), b'r' => out.push(b'\r'), b'a' => out.push(7), b'b' => out.push(8),
            b'v' => out.push(11), b'f' => out.push(12), b'e' => out.push(0x1b), b's' => out.push(b' '),
            b'x' => {
                let h: String = v[i..].iter().take(2).take_while(|z| z.is_ascii_hexdigit()).map(|&z| z as char).collect();
                if h.is_empty() { return err(c, c.vm().c_runtime_error, "invalid hex escape"); }
                out.push(u8::from_str_radix(&h, 16).unwrap()); i += h.len();
            }
            b'u' => {
                uni = true;
                if i < v.len() && v[i] == b'{' {
                    let e = match v[i..].iter().position(|&z| z == b'}') { Some(e) => i + e, None => return err(c, c.vm().c_runtime_error, "unterminated Unicode escape") };
                    for part in String::from_utf8_lossy(&v[i + 1..e]).split_whitespace() { let cp = u32::from_str_radix(part, 16).unwrap_or(0xfffd); encode_utf8(cp, &mut out); }
                    i = e + 1;
                } else {
                    let h: String = v[i..].iter().take(4).map(|&z| z as char).collect();
                    let Ok(cp) = u32::from_str_radix(&h, 16) else { return err(c, c.vm().c_runtime_error, "invalid Unicode escape") };
                    encode_utf8(cp, &mut out); i += 4;
                }
            }
            _ => out.push(y),
        }
    }
    if !closed { return err(c, c.vm().c_runtime_error, "unterminated dumped string"); }
    let mut enc = c.str_enc(slf);
    let rest = &v[i..];
    if !rest.is_empty() {
        let rs = String::from_utf8_lossy(rest).into_owned();
        let t = rs.strip_prefix(".dup").unwrap_or(&rs);
        let Some(t) = t.strip_prefix(".force_encoding(\"").and_then(|t| t.strip_suffix("\")")) else { return bad(c) };
        if uni { return err(c, c.vm().c_runtime_error, "dumped string contained Unicode escape but used force_encoding"); }
        let name = c.str_new(t.as_bytes()); enc = crate::builtins::encoding::to_enc(c, name)?;
    }
    let r = c.str_from_vec_enc(out, enc);
    Ok(r)
});
cfn!(s_append_as_bytes, |c, slf, a, b_| {
    check_frozen(c, slf)?;
    let mut add = vec![];
    for &x in a {
        if x.is_fixnum() { add.push(x.as_fix() as u8); }
        else if is_bignum(x) { let bg = c.big(x); let m = bg.mag.first().copied().unwrap_or(0) as u8; add.push(if bg.neg { m.wrapping_neg() } else { m }); }
        else if is_string(x) { add.extend_from_slice(c.str_bytes(x)); }
        else { return type_err(c, format!("wrong argument type {} (expected String or Integer)", class_name_for_err(x))); }
    }
    c.rstring_mut(slf).b.extend_from_slice(&add);
    Ok(slf)
});
/// Byte offset argument check for byteindex/byterindex/bytesplice.
fn check_char_boundary(c: &mut Ctx, slf: Value, off: usize, what: &str) -> Result<(), Esc> {
    let r = c.rstring(slf);
    if off >= r.b.len() || enc_single(&r.b, r.enc) { return Ok(()); }
    if char_offsets_of(&r.b, r.enc).binary_search(&off).is_err() { return err(c, c.vm().c_index_error, format!("{} {} does not land on character boundary", what, off)); }
    Ok(())
}
cfn!(s_byteindex, |c, slf, a, b_| {
    check_argc(c, a, 1, 2)?;
    let n = c.str_bytes(slf).len() as i64;
    let mut st = if a.len() > 1 { num_to_long(c, a[1])? } else { 0 };
    if st < 0 { st += n; if st < 0 { if regexp::is_regexp(a[0]) { c.last_match = NIL; } return Ok(NIL); } }
    if st > n { if regexp::is_regexp(a[0]) { c.last_match = NIL; } return Ok(NIL); }
    check_char_boundary(c, slf, st as usize, "offset")?;
    if regexp::is_regexp(a[0]) {
        let m = regexp::search(c, a[0], slf, st as usize, true)?;
        if m == NIL { return Ok(NIL); }
        return Ok(Value::fix(regexp::rmatch(m).unwrap().groups[0].unwrap().0 as i64));
    }
    let o = expect_str(c, a[0])?;
    compat(c, slf, o)?;
    Ok(match find_on_boundary(c, slf, c.str_bytes(o), st as usize) { Some(p) => Value::fix(p as i64), None => NIL })
});
cfn!(s_byterindex, |c, slf, a, b_| {
    check_argc(c, a, 1, 2)?;
    let n = c.str_bytes(slf).len() as i64;
    let mut st = if a.len() > 1 { num_to_long(c, a[1])? } else { n };
    if st < 0 { st += n; if st < 0 { if regexp::is_regexp(a[0]) { c.last_match = NIL; } return Ok(NIL); } }
    if st > n { st = n; }
    check_char_boundary(c, slf, st as usize, "offset")?;
    if regexp::is_regexp(a[0]) {
        let mut p = st;
        while p >= 0 {
            if let Some(g) = regexp::rregexp(a[0]).unwrap().re.as_ref().unwrap().match_at(c.str_bytes(slf), p as usize) {
                let m = regexp::match_new(a[0], slf, g); c.last_match = m;
                return Ok(Value::fix(p));
            }
            p -= 1;
        }
        c.last_match = NIL;
        return Ok(NIL);
    }
    let o = expect_str(c, a[0])?;
    compat(c, slf, o)?;
    Ok(match rfind_bytes(c.str_bytes(slf), c.str_bytes(o), st as usize) { Some(p) => Value::fix(p as i64), None => NIL })
});
/// (start, len) in bytes from either a Range or an (index, length) pair.
fn byte_range(c: &mut Ctx, slf: Value, a: &[Value], n: i64) -> Result<(usize, usize), Esc> {
    let (st, ln) = if a.len() == 1 {
        if !is_range(a[0]) { return type_err(c, format!("wrong argument type {} (expected Range)", class_name_for_err(a[0]))); }
        crate::range::range_beg_len(c, a[0], n, 1)?.unwrap()
    } else {
        let (mut st, ln) = (num_to_long(c, a[0])?, num_to_long(c, a[1])?);
        if ln < 0 { return err(c, c.vm().c_index_error, format!("negative length {}", ln)); }
        if st < 0 { st += n; }
        if st < 0 || st > n { let o = num_to_long(c, a[0])?; return err(c, c.vm().c_index_error, format!("index {} out of string", o)); }
        (st, ln)
    };
    let e = (st + ln).min(n);
    check_char_boundary(c, slf, st as usize, "offset")?;
    check_char_boundary(c, slf, e as usize, "offset")?;
    Ok((st as usize, e as usize - st as usize))
}
cfn!(s_bytesplice, |c, slf, a, b_| {
    let n = c.str_bytes(slf).len() as i64;
    let (dst, src, srange): (&[Value], Value, &[Value]) = match a.len() {
        2 => (&a[..1], a[1], &[]),
        3 if is_range(a[0]) => (&a[..1], a[1], &a[2..]),
        3 => (&a[..2], a[2], &[]),
        5 => (&a[..2], a[2], &a[3..]),
        _ => return argc_err(c, a.len(), 2, 5),
    };
    if !srange.is_empty() && (srange.len() == 1) != (dst.len() == 1) {
        return type_err(c, format!("wrong argument type {} (expected Range)", class_name_for_err(if dst.len() == 1 { srange[0] } else { dst[0] })));
    }
    let s2 = expect_str(c, src)?;
    let (ds, dl) = byte_range(c, slf, dst, n)?;
    let (ss, sl) = if srange.is_empty() { (0, c.str_bytes(s2).len()) } else { let m = c.str_bytes(s2).len() as i64; byte_range(c, s2, srange, m)? };
    let enc = compat(c, slf, s2)?;
    check_frozen(c, slf)?;
    let piece = c.str_bytes(s2)[ss..ss + sl].to_vec();
    let r = c.rstring_mut(slf);
    r.b.splice(ds..ds + dl, piece);
    r.enc = enc;
    Ok(slf)
});
cfn!(s_between, |c, slf, a, b_| Ok(Value::bool(c.str_bytes(slf) >= c.str_bytes(a[0]) && c.str_bytes(slf) <= c.str_bytes(a[1]))));
cfn!(s_intern_frozen_p, |c, slf, a, b_| Ok(Value::bool(is_frozen(slf))));
cfn!(s_dup, |c, slf, a, b_| crate::builtins::object::obj_dup(c, slf, false, None));
cfn!(s_s_try_convert, |c, slf, a, b_| { if is_string(a[0]) { return Ok(a[0]); } if a[0].is_heap() && respond_to(a[0], s::to_str, true) { return expect_str(c, a[0]); } Ok(NIL) });

pub fn init(k: Value) {
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(k, n, f, a);
    define_private_cfunc(k, "initialize", s_initialize, -1);
    d("length", s_length, 0); d("size", s_length, 0); d("bytesize", s_bytesize, 0); d("+", s_plus, 1); d("*", s_mul, 1); d("%", s_mod, 1);
    d("==", s_eq, 1); d("===", s_eq, 1); d("eql?", s_eql, 1); d("hash", s_hash, 0); d("<=>", s_cmp, 1); d("=~", s_match_op, 1);
    d("match", s_match, -1); d("match?", s_match_p, -1); d("[]", s_aref, -1); d("slice", s_aref, -1); d("[]=", s_aset, -1);
    d("concat", s_concat, -1); d("<<", s_ltlt, 1); d("to_s", s_to_s, 0); d("to_str", s_to_s, 0); d("to_sym", s_to_sym, 0); d("intern", s_to_sym, 0);
    d("to_i", s_to_i, -1); d("to_f", s_to_f, 0); d("inspect", s_inspect, 0); d("dump", s_dump, 0);
    d("upcase", s_upcase, -1); d("upcase!", s_upcase_b, -1); d("downcase", s_downcase, -1); d("downcase!", s_downcase_b, -1);
    d("swapcase", s_swapcase, -1); d("swapcase!", s_swapcase_b, -1); d("capitalize", s_capitalize, -1); d("capitalize!", s_capitalize_b, -1);
    d("reverse", s_reverse, 0); d("reverse!", s_reverse_b, 0); d("include?", s_include, 1); d("start_with?", s_start_with, -1);
    d("end_with?", s_end_with, -1); d("index", s_index, -1); d("rindex", s_rindex, -1); d("byteindex", s_byteindex, -1); d("byterindex", s_byterindex, -1); d("bytesplice", s_bytesplice, -1); d("append_as_bytes", s_append_as_bytes, -1); d("tr_s!", s_tr_s_b, 2); d("encode!", s_encode_b, -1); d("scrub!", s_scrub_b, -1); d("unicode_normalize!", s_unicode_normalize_b, -1); d("unicode_normalized?", s_unicode_normalized_p, -1);
    d("strip", s_strip, -1); d("strip!", s_strip_b, -1); d("lstrip", s_lstrip, -1); d("lstrip!", s_lstrip_b, -1); d("rstrip", s_rstrip, -1); d("rstrip!", s_rstrip_b, -1);
    d("chomp", s_chomp, -1); d("chomp!", s_chomp_b, -1); d("chop", s_chop, 0); d("chop!", s_chop_b, 0); d("chr", s_chr, 0);
    d("chars", s_chars, 0); d("each_char", s_each_char, 0); d("bytes", s_bytes, 0); d("each_byte", s_each_byte, 0);
    d("lines", s_lines, -1); d("each_line", s_each_line, -1); d("split", s_split, -1); d("sub", s_sub, -1); d("gsub", s_gsub, -1);
    d("sub!", s_sub_b, -1); d("gsub!", s_gsub_b, -1); d("scan", s_scan, 1); d("tr", s_tr, 2); d("tr!", s_tr_b, 2); d("tr_s", s_tr_s, 2);
    d("delete", s_delete, -1); d("delete!", s_delete_b, -1); d("count", s_count, -1); d("squeeze", s_squeeze, -1); d("squeeze!", s_squeeze_b, -1);
    d("ljust", s_ljust, -1); d("rjust", s_rjust, -1); d("center", s_center, -1); d("empty?", s_empty, 0); d("replace", s_replace, 1);
    define_private_cfunc(k, "initialize_copy", s_replace, 1); d("clear", s_clear, 0); d("insert", s_insert, 2); d("prepend", s_prepend, -1); d("ord", s_ord, 0); d("sum", s_sum, -1);
    d("succ", s_succ, 0); d("next", s_succ, 0); d("succ!", s_succ_b, 0); d("next!", s_succ_b, 0); d("upto", s_upto, -1); d("hex", s_hex, 0); d("oct", s_oct, 0);
    d("freeze", s_freeze, 0); d("+@", s_uplus, 0); d("-@", s_uminus, 0); d("dedup", s_uminus, 0); d("encoding", s_encoding, 0);
    d("force_encoding", s_force_encoding, 1); d("b", s_b, 0); d("valid_encoding?", s_valid_encoding, 0); d("ascii_only?", s_ascii_only, 0);
    d("unicode_normalize", s_unicode_normalize, -1); d("encode", s_encode, -1); d("scrub", s_scrub, -1); d("casecmp", s_casecmp, 1); d("casecmp?", s_casecmp_p, 1);
    d("partition", s_partition, 1); d("rpartition", s_rpartition, 1); d("delete_prefix", s_delete_prefix, 1); d("delete_suffix", s_delete_suffix, 1);
    d("delete_prefix!", s_delete_prefix_b, 1); d("delete_suffix!", s_delete_suffix_b, 1); d("slice!", s_slice_b, -1); d("getbyte", s_getbyte, 1); d("setbyte", s_setbyte, 2);
    d("byteslice", s_byteslice, -1); d("unpack", s_unpack, -1); d("unpack1", s_unpack1, -1); d("crypt", s_crypt, 1); d("each_grapheme_cluster", s_each_grapheme, 0);
    d("grapheme_clusters", s_grapheme_clusters, 0); d("codepoints", s_codepoints, 0); d("each_codepoint", s_each_codepoint, 0); d("dup", s_dup, 0); d("undump", s_undump, 0); d("to_r", s_to_r, 0); d("to_c", s_to_c, 0);
    define_singleton_cfunc(k, "try_convert", s_s_try_convert, 1);
    let _ = (s_squish, s_intern_frozen_p, s_between);
    define_singleton_cfunc(k, "new", s_s_new, -1);
}
