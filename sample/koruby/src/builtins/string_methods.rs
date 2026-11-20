// String methods.
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::regexp;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

macro_rules! cfn { ($name:ident, |$c:ident, $s:ident, $a:ident, $b:ident| $body:expr) => {
    #[allow(unused_variables)]
    pub fn $name($c: &mut Ctx, $s: Value, $a: &[Value], $b: *const Block) -> RV { $body }
}; }

fn b(v: Value) -> &'static [u8] { &rstr(v).b }
fn new_like(slf: Value, v: Vec<u8>) -> Value { str_new_like(slf, v) }

fn modify(c: &mut Ctx, v: Value) -> Result<&'static mut RString, Esc> {
    if obj(v).flags & FL_FROZEN != 0 { return Err(raise_frozen(c, v)); }
    Ok(rstr(v))
}

/// Character index → byte offset (None if out of range); `i` may equal the length.
fn char_to_byte(v: Value, i: usize) -> Option<usize> {
    let r = rstr(v);
    if r.enc != ENC_UTF8 || is_ascii(&r.b) { return if i <= r.b.len() { Some(i) } else { None }; }
    let mut pos = 0; let mut n = 0;
    while n < i { if pos >= r.b.len() { return None; } pos += char_len_at(&r.b, pos); n += 1; }
    Some(pos)
}
fn byte_to_char(v: Value, off: usize) -> usize {
    let r = rstr(v);
    if r.enc != ENC_UTF8 || is_ascii(&r.b) { return off; }
    utf8_len(&r.b[..off])
}

/// substring by char start/len
fn substr(v: Value, start: i64, len: i64) -> Option<Value> {
    let n = str_char_len(v) as i64;
    let mut st = start;
    if st < 0 { st += n; }
    if st < 0 || st > n || len < 0 { return None; }
    let e = (st + len).min(n);
    let bs = char_to_byte(v, st as usize)?; let be = char_to_byte(v, e as usize)?;
    Some(new_like(v, b(v)[bs..be].to_vec()))
}

pub fn str_aref(c: &mut Ctx, slf: Value, a: &[Value]) -> RV {
    check_argc(c, a, 1, 2)?;
    if a.len() == 2 {
        if regexp::is_regexp(a[0]) {
            let m = regexp::search(c, a[0], slf, 0, true)?;
            if m == NIL { return Ok(NIL); }
            if a[1].is_sym() || is_string(a[1]) { let nm = if a[1].is_sym() { a[1].as_sym() } else { intern(b(a[1])) }; return Ok(regexp::named_match(m, nm)); }
            let i = num_to_long(c, a[1])?;
            return Ok(regexp::nth_match(m, i));
        }
        let st = num_to_long(c, a[0])?; let ln = num_to_long(c, a[1])?;
        return Ok(substr(slf, st, ln).unwrap_or(NIL));
    }
    let x = a[0];
    if x.is_fixnum() {
        let i = x.as_fix();
        let n = str_char_len(slf) as i64;
        let i = if i < 0 { i + n } else { i };
        if i < 0 || i >= n { return Ok(NIL); }
        return Ok(substr(slf, i, 1).unwrap_or(NIL));
    }
    if is_range(x) {
        let n = str_char_len(slf) as i64;
        return match crate::range::range_beg_len(c, x, n, 0)? { Some((st, ln)) => Ok(substr(slf, st, ln).unwrap_or(NIL)), None => Ok(NIL) };
    }
    if is_string(x) {
        return Ok(if find_bytes(b(slf), b(x), 0).is_some() { new_like(x, b(x).to_vec()) } else { NIL });
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
fn rfind_bytes(h: &[u8], n: &[u8], upto: usize) -> Option<usize> {
    if n.len() > h.len() { return None; }
    let mut i = upto.min(h.len() - n.len()) as i64;
    while i >= 0 { if &h[i as usize..i as usize + n.len()] == n { return Some(i as usize); } i -= 1; }
    None
}

cfn!(s_initialize, |c, slf, a, _b| {
    let pos: Vec<Value> = a.iter().copied().filter(|v| !is_hash(*v) || kw_arg(a) != Some(*v)).collect();
    if let Some(&v) = pos.first() { let sv = expect_str(c, v)?; let nb = b(sv).to_vec(); let e = rstr(sv).enc; let r = modify(c, slf)?; r.b = nb; r.enc = e; }
    Ok(NIL)
});
cfn!(s_s_new, |c, slf, a, blk| crate::builtins::object::c_new(c, slf, a, blk));
cfn!(s_length, |c, slf, a, b_| Ok(Value::fix(str_char_len(slf) as i64)));
cfn!(s_bytesize, |c, slf, a, b_| Ok(Value::fix(b(slf).len() as i64)));
cfn!(s_plus, |c, slf, a, b_| {
    let o = expect_str(c, a[0])?;
    let mut v = b(slf).to_vec(); v.extend_from_slice(b(o));
    let enc = if rstr(slf).enc == ENC_BINARY || rstr(o).enc == ENC_BINARY && !is_ascii(b(o)) { ENC_BINARY } else { ENC_UTF8 };
    Ok(str_from_vec_enc(v, enc))
});
cfn!(s_mul, |c, slf, a, b_| {
    let n = num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative argument"); }
    if (b(slf).len() as u128) * (n as u128) > (1u128 << 31) * 64 { return arg_err(c, "argument too big"); }
    Ok(new_like(slf, b(slf).repeat(n as usize)))
});
cfn!(s_mod, |c, slf, a, b_| {
    let args: Vec<Value> = if is_array(a[0]) { rary(a[0]).clone() } else { vec![a[0]] };
    crate::builtins::sprintf::format(c, slf, &args)
});
cfn!(s_eq, |c, slf, a, b_| {
    let o = a[0];
    if is_string(o) { return Ok(Value::bool(b(slf) == b(o))); }
    if o.is_heap() && respond_to(o, s::to_str, true) { let r = funcall(c, o, s::eq, &[slf], std::ptr::null())?; return Ok(Value::bool(r.truthy())); }
    Ok(FALSE)
});
cfn!(s_eql, |c, slf, a, b_| Ok(Value::bool(is_string(a[0]) && b(slf) == b(a[0]))));
cfn!(s_hash, |c, slf, a, b_| Ok(Value::fix((str_hash(slf) >> 2) as i64)));
cfn!(s_cmp, |c, slf, a, b_| {
    if is_string(a[0]) { return Ok(Value::fix(b(slf).cmp(b(a[0])) as i64)); }
    if a[0].is_heap() && respond_to(a[0], s::to_str, true) && respond_to(a[0], s::cmp, true) {
        let r = funcall(c, a[0], s::cmp, &[slf], std::ptr::null())?;
        if r.is_fixnum() { return Ok(Value::fix(-r.as_fix())); }
    }
    Ok(NIL)
});
cfn!(s_match_op, |c, slf, a, b_| {
    if is_string(a[0]) { return type_err(c, "wrong argument type String (expected Regexp)"); }
    if regexp::is_regexp(a[0]) {
        let m = regexp::search(c, a[0], slf, 0, true)?;
        if m == NIL { return Ok(NIL); }
        let (st, _) = regexp::rmatch(m).unwrap().groups[0].unwrap();
        return Ok(Value::fix(byte_to_char(slf, st) as i64));
    }
    funcall(c, a[0], s::match_op, &[slf], std::ptr::null())
});
cfn!(s_match, |c, slf, a, blk| {
    if a.is_empty() { return argc_err(c, 0, 1, 2); }
    let re = regexp::to_regexp(c, a[0])?;
    let pos = if a.len() > 1 { let p = num_to_long(c, a[1])?; let n = str_char_len(slf) as i64; let p = if p < 0 { p + n } else { p }; if p < 0 || p > n { c.last_match = NIL; return Ok(NIL); } char_to_byte(slf, p as usize).unwrap_or(0) } else { 0 };
    let m = regexp::search(c, re, slf, pos, true)?;
    if m != NIL && !blk.is_null() { return yield1(c, blk, m); }
    Ok(m)
});
cfn!(s_match_p, |c, slf, a, b_| {
    let re = regexp::to_regexp(c, a[0])?;
    let pos = if a.len() > 1 { char_to_byte(slf, num_to_long(c, a[1])?.max(0) as usize).unwrap_or(usize::MAX) } else { 0 };
    Ok(Value::bool(regexp::search_raw(re, b(slf), pos).is_some()))
});
cfn!(s_aref, |c, slf, a, b_| str_aref(c, slf, a));
cfn!(s_aset, |c, slf, a, b_| {
    check_argc(c, a, 2, 3)?;
    let val = expect_str(c, *a.last().unwrap())?;
    let n = str_char_len(slf) as i64;
    let (bs, be) = if a.len() == 3 {
        if regexp::is_regexp(a[0]) {
            let m = regexp::search(c, a[0], slf, 0, true)?;
            if m == NIL { return err(c, vm().c_index_error, "regexp not matched"); }
            let g = num_to_long(c, a[1])?;
            match regexp::rmatch(m).unwrap().groups.get(g as usize).copied().flatten() { Some(x) => x, None => return err(c, vm().c_index_error, format!("index {} out of regexp", g)) }
        } else {
            let mut st = num_to_long(c, a[0])?; let ln = num_to_long(c, a[1])?;
            if st < 0 { st += n; }
            if st < 0 || st > n { let o = num_to_long(c, a[0])?; return err(c, vm().c_index_error, format!("index {} out of string", o)); }
            if ln < 0 { return err(c, vm().c_index_error, format!("negative length {}", ln)); }
            (char_to_byte(slf, st as usize).unwrap(), char_to_byte(slf, (st + ln).min(n) as usize).unwrap())
        }
    } else if a[0].is_fixnum() {
        let mut i = a[0].as_fix();
        if i < 0 { i += n; }
        if i < 0 || i >= n { return err(c, vm().c_index_error, format!("index {} out of string", a[0].as_fix())); }
        (char_to_byte(slf, i as usize).unwrap(), char_to_byte(slf, i as usize + 1).unwrap())
    } else if is_range(a[0]) {
        match crate::range::range_beg_len(c, a[0], n, 1)? { Some((st, ln)) => (char_to_byte(slf, st as usize).unwrap(), char_to_byte(slf, (st + ln).min(n) as usize).unwrap()), None => return Ok(val) }
    } else if is_string(a[0]) {
        match find_bytes(b(slf), b(a[0]), 0) { Some(p) => (p, p + b(a[0]).len()), None => return err(c, vm().c_index_error, "string not matched") }
    } else if regexp::is_regexp(a[0]) {
        let m = regexp::search(c, a[0], slf, 0, true)?;
        if m == NIL { return err(c, vm().c_index_error, "regexp not matched"); }
        regexp::rmatch(m).unwrap().groups[0].unwrap()
    } else { let i = num_to_long(c, a[0])?; return s_aset(c, slf, &[Value::fix(i), val], b_); };
    let vb = b(val).to_vec();
    let r = modify(c, slf)?;
    r.b.splice(bs..be, vb);
    Ok(val)
});
cfn!(s_concat, |c, slf, a, b_| {
    let mut add = vec![];
    for &x in a {
        if x.is_fixnum() || is_bignum(x) {
            let i = num_to_long(c, x)?;
            if rstr(slf).enc == ENC_BINARY || rstr(slf).enc == ENC_ASCII { if !(0..256).contains(&i) { return err(c, vm().c_range_error, format!("{} out of char range", i)); } add.push(i as u8); }
            else { if i < 0 || i > 0x10ffff { return err(c, vm().c_range_error, format!("{} out of char range", i)); } encode_utf8(i as u32, &mut add); }
        } else { let s2 = expect_str(c, x)?; add.extend_from_slice(b(s2)); }
    }
    let r = modify(c, slf)?;
    r.b.extend_from_slice(&add);
    Ok(slf)
});
cfn!(s_ltlt, |c, slf, a, b_| {
    let x = a[0];
    if is_string(x) { str_append_str(c, slf, x)?; return Ok(slf); }
    s_concat(c, slf, a, b_)
});
cfn!(s_to_s, |c, slf, a, b_| {
    if obj(slf).klass == vm().c_string { return Ok(slf); }
    Ok(str_new_enc(b(slf), rstr(slf).enc))
});
cfn!(s_to_sym, |c, slf, a, b_| Ok(str_to_sym(slf)));
cfn!(s_to_i, |c, slf, a, b_| {
    let base = if a.is_empty() { 10 } else { num_to_long(c, a[0])? };
    if base == 1 || base > 36 || base < 0 { return arg_err(c, format!("invalid radix {}", base)); }
    Ok(str_to_inum(b(slf), base as u32, false).unwrap_or(Value::fix(0)))
});
cfn!(s_to_f, |c, slf, a, b_| Ok(crate::numeric::float_new(str_to_f(b(slf)))));
cfn!(s_inspect, |c, slf, a, b_| Ok(str_new(inspect_bytes(b(slf), rstr(slf).enc).as_bytes())));
cfn!(s_dump, |c, slf, a, b_| {
    let mut out = String::from("\"");
    for &x in b(slf) {
        match x { b'"' => out.push_str("\\\""), b'\\' => out.push_str("\\\\"), b'\n' => out.push_str("\\n"), b'\t' => out.push_str("\\t"), b'\r' => out.push_str("\\r"),
            0x1b => out.push_str("\\e"), b'#' => out.push('#'), 0x20..=0x7e => out.push(x as char), _ => out.push_str(&format!("\\x{:02X}", x)) }
    }
    out.push('"');
    Ok(str_new(out.as_bytes()))
});
macro_rules! case_m { ($name:ident, $bang:ident, $f:expr) => {
    cfn!($name, |c, slf, a, b_| { let r = rstr(slf); Ok(new_like(slf, $f(&r.b, r.enc))) });
    cfn!($bang, |c, slf, a, b_| { let r = rstr(slf); let nv = $f(&r.b, r.enc); if nv == r.b { return Ok(NIL); } let r = modify(c, slf)?; r.b = nv; Ok(slf) });
}; }
case_m!(s_upcase, s_upcase_b, upcase);
case_m!(s_downcase, s_downcase_b, downcase);
case_m!(s_swapcase, s_swapcase_b, swapcase);
case_m!(s_capitalize, s_capitalize_b, capitalize);
fn reverse_bytes(bv: &[u8], enc: u8) -> Vec<u8> {
    if enc != ENC_UTF8 || is_ascii(bv) { let mut v = bv.to_vec(); v.reverse(); return v; }
    let mut parts = vec![]; let mut i = 0;
    while i < bv.len() { let l = char_len_at(bv, i); parts.push(&bv[i..i + l]); i += l; }
    parts.reverse();
    parts.concat()
}
case_m!(s_reverse, s_reverse_b, reverse_bytes);
cfn!(s_include, |c, slf, a, b_| { let o = expect_str(c, a[0])?; Ok(Value::bool(find_bytes(b(slf), b(o), 0).is_some())) });
cfn!(s_start_with, |c, slf, a, b_| {
    for &x in a {
        if regexp::is_regexp(x) { if let Some(g) = regexp::rregexp(x).unwrap().re.as_ref().unwrap().match_at(b(slf), 0) { let m = regexp::match_new(x, slf, g); c.last_match = m; return Ok(TRUE); } continue; }
        let o = expect_str(c, x)?; if b(slf).starts_with(b(o)) { return Ok(TRUE); }
    }
    Ok(FALSE)
});
cfn!(s_end_with, |c, slf, a, b_| { for &x in a { let o = expect_str(c, x)?; if b(slf).ends_with(b(o)) { return Ok(TRUE); } } Ok(FALSE) });
cfn!(s_index, |c, slf, a, b_| {
    let n = str_char_len(slf) as i64;
    let mut st = if a.len() > 1 { num_to_long(c, a[1])? } else { 0 };
    if st < 0 { st += n; if st < 0 { return Ok(NIL); } }
    if st > n { return Ok(NIL); }
    let bs = char_to_byte(slf, st as usize).unwrap();
    if regexp::is_regexp(a[0]) {
        let m = regexp::search(c, a[0], slf, bs, true)?;
        if m == NIL { return Ok(NIL); }
        return Ok(Value::fix(byte_to_char(slf, regexp::rmatch(m).unwrap().groups[0].unwrap().0) as i64));
    }
    let o = expect_str(c, a[0])?;
    Ok(match find_bytes(b(slf), b(o), bs) { Some(p) => Value::fix(byte_to_char(slf, p) as i64), None => NIL })
});
cfn!(s_rindex, |c, slf, a, b_| {
    let n = str_char_len(slf) as i64;
    let mut st = if a.len() > 1 { num_to_long(c, a[1])? } else { n };
    if st < 0 { st += n; if st < 0 { return Ok(NIL); } }
    if st > n { st = n; }
    let bs = char_to_byte(slf, st as usize).unwrap();
    if regexp::is_regexp(a[0]) {
        let mut p = bs as i64;
        while p >= 0 {
            if let Some(g) = regexp::rregexp(a[0]).unwrap().re.as_ref().unwrap().match_at(b(slf), p as usize) {
                let m = regexp::match_new(a[0], slf, g); c.last_match = m;
                return Ok(Value::fix(byte_to_char(slf, p as usize) as i64));
            }
            p -= 1;
        }
        c.last_match = NIL;
        return Ok(NIL);
    }
    let o = expect_str(c, a[0])?;
    Ok(match rfind_bytes(b(slf), b(o), bs) { Some(p) => Value::fix(byte_to_char(slf, p) as i64), None => NIL })
});
fn is_ws(x: u8) -> bool { x == b' ' || (9..=13).contains(&x) || x == 0 }
fn strip_impl(v: &[u8], l: bool, r: bool) -> Vec<u8> {
    let mut s = 0; let mut e = v.len();
    if l { while s < e && is_ws(v[s]) { s += 1; } }
    if r { while e > s && is_ws(v[e - 1]) { e -= 1; } }
    v[s..e].to_vec()
}
macro_rules! strip_m { ($name:ident, $bang:ident, $l:expr, $r:expr) => {
    cfn!($name, |c, slf, a, b_| Ok(new_like(slf, strip_impl(b(slf), $l, $r))));
    cfn!($bang, |c, slf, a, b_| { let nv = strip_impl(b(slf), $l, $r); if nv.len() == b(slf).len() { modify(c, slf)?; return Ok(NIL); } modify(c, slf)?.b = nv; Ok(slf) });
}; }
strip_m!(s_strip, s_strip_b, true, true);
strip_m!(s_lstrip, s_lstrip_b, true, false);
strip_m!(s_rstrip, s_rstrip_b, false, true);
fn chomp_impl(c: &mut Ctx, slf: Value, a: &[Value]) -> Result<Vec<u8>, Esc> {
    let v = b(slf);
    if a.is_empty() || (a[0] != NIL && is_string(a[0]) && b(a[0]) == b"\n") {
        if v.ends_with(b"\r\n") { return Ok(v[..v.len() - 2].to_vec()); }
        if v.ends_with(b"\n") || v.ends_with(b"\r") { return Ok(v[..v.len() - 1].to_vec()); }
        return Ok(v.to_vec());
    }
    if a[0] == NIL { return Ok(v.to_vec()); }
    let suf = expect_str(c, a[0])?;
    let sb = b(suf);
    if sb.is_empty() { let mut e = v.len(); while e > 0 && v[e - 1] == b'\n' { e -= 1; if e > 0 && v[e - 1] == b'\r' { e -= 1; } } return Ok(v[..e].to_vec()); }
    if v.ends_with(sb) { Ok(v[..v.len() - sb.len()].to_vec()) } else { Ok(v.to_vec()) }
}
cfn!(s_chomp, |c, slf, a, b_| { let v = chomp_impl(c, slf, a)?; Ok(new_like(slf, v)) });
cfn!(s_chomp_b, |c, slf, a, b_| { let v = chomp_impl(c, slf, a)?; if v.len() == b(slf).len() { modify(c, slf)?; return Ok(NIL); } modify(c, slf)?.b = v; Ok(slf) });
fn chop_impl(v: &[u8], enc: u8) -> Vec<u8> {
    if v.is_empty() { return vec![]; }
    if v.ends_with(b"\r\n") { return v[..v.len() - 2].to_vec(); }
    if enc == ENC_UTF8 { let mut i = v.len() - 1; while i > 0 && v[i] & 0xc0 == 0x80 { i -= 1; } return v[..i].to_vec(); }
    v[..v.len() - 1].to_vec()
}
cfn!(s_chop, |c, slf, a, b_| Ok(new_like(slf, chop_impl(b(slf), rstr(slf).enc))));
cfn!(s_chop_b, |c, slf, a, b_| { if b(slf).is_empty() { return Ok(NIL); } let v = chop_impl(b(slf), rstr(slf).enc); modify(c, slf)?.b = v; Ok(slf) });
cfn!(s_chr, |c, slf, a, b_| { let v = b(slf); if v.is_empty() { return Ok(new_like(slf, vec![])); } let l = if rstr(slf).enc == ENC_UTF8 { char_len_at(v, 0) } else { 1 }; Ok(new_like(slf, v[..l].to_vec())) });
cfn!(s_chars, |c, slf, a, blk| {
    let parts = chars_of(slf);
    if !blk.is_null() { for p in parts { let s2 = new_like(slf, p); yield1(c, blk, s2)?; } return Ok(slf); }
    Ok(ary_of_strs(parts, rstr(slf).enc))
});
cfn!(s_each_char, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_char"), &[], NIL); }
    s_chars(c, slf, a, blk)
});
cfn!(s_bytes, |c, slf, a, blk| {
    if !blk.is_null() { let n = b(slf).len(); for i in 0..n { let x = *b(slf).get(i).unwrap_or(&0); yield1(c, blk, Value::fix(x as i64))?; } return Ok(slf); }
    Ok(ary_from_vec(b(slf).iter().map(|&x| Value::fix(x as i64)).collect()))
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
fn lines_args(c: &mut Ctx, a: &[Value]) -> Result<(Vec<u8>, bool), Esc> {
    let mut sep = b"\n".to_vec(); let mut chomp = false;
    for &x in a {
        if is_hash(x) { if let Some(v) = crate::hash::hash_get_opt(c, x, sym_value("chomp"))? { chomp = v.truthy(); } }
        else if x == NIL { sep = vec![0xff, 0xfe, 0xfd]; } else { let s2 = expect_str(c, x)?; sep = b(s2).to_vec(); }
    }
    Ok((sep, chomp))
}
cfn!(s_lines, |c, slf, a, blk| {
    let (sep, chomp) = lines_args(c, a)?;
    let ls = if sep == [0xff, 0xfe, 0xfd] { vec![b(slf).to_vec()] } else { lines_of(b(slf), &sep, chomp) };
    if !blk.is_null() { for l in ls { let s2 = new_like(slf, l); yield1(c, blk, s2)?; } return Ok(slf); }
    Ok(ary_of_strs(ls, rstr(slf).enc))
});
cfn!(s_each_line, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_line"), a, NIL); }
    s_lines(c, slf, a, blk)
});
cfn!(s_split, |c, slf, a, blk| {
    let limit = if a.len() > 1 { num_to_long(c, a[1])? } else { 0 };
    let v = b(slf);
    let mut out: Vec<Vec<u8>> = vec![];
    let pat = if a.is_empty() || a[0] == NIL { let fs = gvar_get(c, intern(b"$;"))?; if fs == NIL { str_new(b" ") } else { fs } } else { a[0] };
    if is_string(pat) && b(pat) == b" " {
        // awk-style
        let mut i = 0;
        while i < v.len() && is_ws(v[i]) { i += 1; }
        while i < v.len() {
            if limit > 0 && out.len() as i64 == limit - 1 { out.push(v[i..].to_vec()); break; }
            let st = i;
            while i < v.len() && !is_ws(v[i]) { i += 1; }
            out.push(v[st..i].to_vec());
            while i < v.len() && is_ws(v[i]) { i += 1; }
        }
    } else if is_string(pat) {
        let p = b(pat);
        if p.is_empty() {
            for ch in chars_of(slf) { if limit > 0 && out.len() as i64 == limit - 1 { break; } out.push(ch); }
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
        let re = regexp::to_regexp(c, pat)?;
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
    let arr = ary_of_strs(out, rstr(slf).enc);
    if !blk.is_null() { let n = ary_len(arr); for i in 0..n { yield1(c, blk, ary_get(arr, i))?; } return Ok(slf); }
    Ok(arr)
});

/// Expand a replacement string with \1 \0 \& \k<name> \` \' \\
fn expand_repl(rep: &[u8], m_bytes: &[u8], groups: &[Option<(usize, usize)>], re: Value) -> Vec<u8> {
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
                        if let Some(r) = regexp::rregexp(re) { for &gi in r.re.as_ref().unwrap().group_index(&name).iter().rev() { if groups.get(gi).map(|g| g.is_some()).unwrap_or(false) { grp(gi, &mut out); break; } } }
                        i += 4 + e;
                    } else { out.push(x); i += 1; }
                }
                _ => { out.push(x); out.push(y); i += 2; }
            }
        } else { out.push(x); i += 1; }
    }
    out
}

/// sub/gsub core: returns (new bytes, changed)
fn subst(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block, global: bool) -> Result<Option<Vec<u8>>, Esc> {
    if a.is_empty() || a.len() > 2 { return argc_err(c, a.len(), 1, 2); }
    let pat = a[0];
    let hash_rep = if a.len() == 2 && is_hash(a[1]) { Some(a[1]) } else { None };
    let str_rep = if a.len() == 2 && hash_rep.is_none() { Some(expect_str(c, a[1])?) } else { None };
    if a.len() == 1 && blk.is_null() { return arg_err(c, "wrong number of arguments (given 1, expected 2)"); }
    let src = b(slf).to_vec();
    let lit = if is_string(pat) { Some(b(pat).to_vec()) } else { None };
    let re = if lit.is_none() { regexp::to_regexp(c, pat)? } else { NIL };
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
        let mval = if lit.is_none() { let srcv = str_new_like(slf, src.clone()); let m = regexp::match_new(re, srcv, g.clone()); c.last_match = m; m } else { NIL };
        if let Some(h) = hash_rep {
            let key = str_new_like(slf, src[ms..me].to_vec());
            let v = crate::hash::hash_aref(c, h, key)?;
            let sv = crate::rt::rt_tostr(c, v)?;
            out.extend_from_slice(b(sv));
        } else if let Some(r) = str_rep {
            out.extend(expand_repl(b(r), &src, &g, re));
        } else {
            let mstr = str_new_like(slf, src[ms..me].to_vec());
            if lit.is_some() { let m = regexp::match_new(NIL, str_new_like(slf, src.clone()), g.clone()); let _ = m; }
            let v = yield1(c, blk, mstr)?;
            if mval != NIL { c.last_match = mval; }
            let sv = crate::rt::rt_tostr(c, v)?;
            out.extend_from_slice(b(sv));
        }
        changed = true;
        last = me;
        if me == ms {
            if ms < src.len() { let l = char_len_at(&src, ms); out.extend_from_slice(&src[ms..ms + l]); last = ms + l; i = ms + l; } else { i = ms + 1; }
        } else { i = me; }
        if !global || i > src.len() { break; }
    }
    if !changed { if lit.is_none() { c.last_match = NIL; } return Ok(None); }
    if last < src.len() { out.extend_from_slice(&src[last..]); }
    Ok(Some(out))
}
cfn!(s_sub, |c, slf, a, blk| Ok(new_like(slf, subst(c, slf, a, blk, false)?.unwrap_or_else(|| b(slf).to_vec()))));
cfn!(s_gsub, |c, slf, a, blk| {
    if a.len() == 1 && blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"gsub"), a, NIL); }
    Ok(new_like(slf, subst(c, slf, a, blk, true)?.unwrap_or_else(|| b(slf).to_vec())))
});
cfn!(s_sub_b, |c, slf, a, blk| { check_frozen(c, slf)?; match subst(c, slf, a, blk, false)? { Some(v) => { modify(c, slf)?.b = v; Ok(slf) } None => Ok(NIL) } });
cfn!(s_gsub_b, |c, slf, a, blk| { check_frozen(c, slf)?; match subst(c, slf, a, blk, true)? { Some(v) => { modify(c, slf)?.b = v; Ok(slf) } None => Ok(NIL) } });
cfn!(s_scan, |c, slf, a, blk| {
    let re = regexp::to_regexp(c, a[0])?;
    let src = b(slf).to_vec();
    let out = ary_new();
    let mut i = 0;
    while i <= src.len() {
        let Some(g) = regexp::search_raw(re, &src, i) else { break };
        let (ms, me) = g[0].unwrap();
        let m = regexp::match_new(re, slf, g.clone());
        c.last_match = m;
        let item = if g.len() == 1 { str_new_like(slf, src[ms..me].to_vec()) } else {
            let arr = ary_new();
            for gi in g.iter().skip(1) { ary_push(arr, match gi { Some((x, y)) => str_new_like(slf, src[*x..*y].to_vec()), None => NIL }); }
            arr
        };
        if !blk.is_null() { yield1(c, blk, item)?; c.last_match = m; } else { ary_push(out, item); }
        i = if me == ms { me + if me < src.len() { char_len_at(&src, me) } else { 1 } } else { me };
    }
    if !blk.is_null() { return Ok(slf); }
    Ok(out)
});
fn tr_expand(set: &[u8]) -> (bool, Vec<u32>) {
    let chars: Vec<u32> = String::from_utf8_lossy(set).chars().map(|c| c as u32).collect();
    let (neg, chars) = if chars.len() > 1 && chars[0] == '^' as u32 { (true, chars[1..].to_vec()) } else { (false, chars) };
    let mut out = vec![];
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' as u32 && i + 1 < chars.len() { out.push(chars[i + 1]); i += 2; continue; }
        if i + 2 < chars.len() && chars[i + 1] == '-' as u32 {
            let (a, b2) = (chars[i], chars[i + 2]);
            if a <= b2 { for x in a..=b2 { out.push(x); } }
            i += 3;
        } else { out.push(chars[i]); i += 1; }
    }
    (neg, out)
}
fn tr_impl(src: &[u8], from: &[u8], to: &[u8], squeeze: bool) -> Vec<u8> {
    let (neg, f) = tr_expand(from);
    let (_, t) = tr_expand(to);
    let s = String::from_utf8_lossy(src);
    let mut out = String::new();
    let mut last: Option<char> = None;
    for ch in s.chars() {
        let cp = ch as u32;
        let pos = f.iter().position(|&x| x == cp);
        let mapped = if neg {
            if pos.is_none() { t.last().copied() } else { None }
        } else { pos.map(|p| if t.is_empty() { u32::MAX } else { *t.get(p).unwrap_or(t.last().unwrap()) }) };
        match mapped {
            Some(u32::MAX) => {}
            Some(m) => { let mc = char::from_u32(m).unwrap_or(ch); if squeeze && last == Some(mc) { continue; } out.push(mc); last = Some(mc); }
            None => { out.push(ch); last = None; }
        }
    }
    out.into_bytes()
}
cfn!(s_tr, |c, slf, a, b_| { let f = expect_str(c, a[0])?; let t = expect_str(c, a[1])?; Ok(new_like(slf, tr_impl(b(slf), b(f), b(t), false))) });
cfn!(s_tr_b, |c, slf, a, b_| { let f = expect_str(c, a[0])?; let t = expect_str(c, a[1])?; let v = tr_impl(b(slf), b(f), b(t), false); if v == b(slf) { return Ok(NIL); } modify(c, slf)?.b = v; Ok(slf) });
cfn!(s_tr_s, |c, slf, a, b_| { let f = expect_str(c, a[0])?; let t = expect_str(c, a[1])?; Ok(new_like(slf, tr_impl(b(slf), b(f), b(t), true))) });
fn in_sets(ch: u32, sets: &[(bool, Vec<u32>)]) -> bool { sets.iter().all(|(neg, s)| s.contains(&ch) != *neg) }
fn sets_of(c: &mut Ctx, a: &[Value]) -> Result<Vec<(bool, Vec<u32>)>, Esc> {
    let mut v = vec![];
    for &x in a { let s2 = expect_str(c, x)?; v.push(tr_expand(b(s2))); }
    Ok(v)
}
cfn!(s_delete, |c, slf, a, b_| {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let sets = sets_of(c, a)?;
    let s2: String = String::from_utf8_lossy(b(slf)).chars().filter(|&ch| !in_sets(ch as u32, &sets)).collect();
    Ok(new_like(slf, s2.into_bytes()))
});
cfn!(s_delete_b, |c, slf, a, b_| { let r = s_delete(c, slf, a, b_)?; if b(r) == b(slf) { return Ok(NIL); } let nb = b(r).to_vec(); modify(c, slf)?.b = nb; Ok(slf) });
cfn!(s_count, |c, slf, a, b_| {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let sets = sets_of(c, a)?;
    Ok(Value::fix(String::from_utf8_lossy(b(slf)).chars().filter(|&ch| in_sets(ch as u32, &sets)).count() as i64))
});
cfn!(s_squeeze, |c, slf, a, b_| {
    let sets = sets_of(c, a)?;
    let mut out = String::new(); let mut last = None;
    for ch in String::from_utf8_lossy(b(slf)).chars() {
        if Some(ch) == last && (sets.is_empty() || in_sets(ch as u32, &sets)) { continue; }
        out.push(ch); last = Some(ch);
    }
    Ok(new_like(slf, out.into_bytes()))
});
cfn!(s_squeeze_b, |c, slf, a, b_| { let r = s_squeeze(c, slf, a, b_)?; if b(r) == b(slf) { return Ok(NIL); } let nb = b(r).to_vec(); modify(c, slf)?.b = nb; Ok(slf) });
fn justify(c: &mut Ctx, slf: Value, a: &[Value], mode: u8) -> RV {
    let w = num_to_long(c, a[0])?;
    let pad = if a.len() > 1 { let p = expect_str(c, a[1])?; b(p).to_vec() } else { b" ".to_vec() };
    if pad.is_empty() { return arg_err(c, "zero width padding"); }
    let n = str_char_len(slf) as i64;
    if w <= n { return Ok(new_like(slf, b(slf).to_vec())); }
    let total = (w - n) as usize;
    let padchars: Vec<Vec<u8>> = { let ps = str_new(&pad); chars_of(ps) };
    let mk = |k: usize| -> Vec<u8> { let mut v = vec![]; for i in 0..k { v.extend_from_slice(&padchars[i % padchars.len()]); } v };
    let (l, r) = match mode { b'l' => (0, total), b'r' => (total, 0), _ => (total / 2, total - total / 2) };
    let mut out = mk(l); out.extend_from_slice(b(slf)); out.extend(mk(r));
    Ok(new_like(slf, out))
}
cfn!(s_ljust, |c, slf, a, b_| justify(c, slf, a, b'l'));
cfn!(s_rjust, |c, slf, a, b_| justify(c, slf, a, b'r'));
cfn!(s_center, |c, slf, a, b_| justify(c, slf, a, b'c'));
cfn!(s_empty, |c, slf, a, b_| Ok(Value::bool(b(slf).is_empty())));
cfn!(s_replace, |c, slf, a, b_| { let o = expect_str(c, a[0])?; let nb = b(o).to_vec(); let e = rstr(o).enc; let r = modify(c, slf)?; r.b = nb; r.enc = e; Ok(slf) });
cfn!(s_clear, |c, slf, a, b_| { modify(c, slf)?.b.clear(); Ok(slf) });
cfn!(s_insert, |c, slf, a, b_| {
    let idx = num_to_long(c, a[0])?; let o = expect_str(c, a[1])?;
    let n = str_char_len(slf) as i64;
    let pos = if idx < 0 { idx + n + 1 } else { idx };
    if pos < 0 || pos > n { return err(c, vm().c_index_error, format!("index {} out of string", idx)); }
    let bp = char_to_byte(slf, pos as usize).unwrap();
    let ob = b(o).to_vec();
    modify(c, slf)?.b.splice(bp..bp, ob);
    Ok(slf)
});
cfn!(s_prepend, |c, slf, a, b_| { let mut pre = vec![]; for &x in a { let o = expect_str(c, x)?; pre.extend_from_slice(b(o)); } modify(c, slf)?.b.splice(0..0, pre); Ok(slf) });
cfn!(s_ord, |c, slf, a, b_| {
    let v = b(slf);
    if v.is_empty() { return arg_err(c, "empty string"); }
    if rstr(slf).enc == ENC_UTF8 { let (cp, _) = decode_utf8(v, 0); return Ok(Value::fix(cp as i64)); }
    Ok(Value::fix(v[0] as i64))
});
cfn!(s_sum, |c, slf, a, b_| { let bits = if a.is_empty() { 16 } else { num_to_long(c, a[0])? }; let s2: u64 = b(slf).iter().map(|&x| x as u64).sum(); Ok(Value::fix(if bits > 0 && bits < 63 { (s2 & ((1 << bits) - 1)) as i64 } else { s2 as i64 })) });
cfn!(s_succ, |c, slf, a, b_| Ok(new_like(slf, succ_bytes(b(slf)))));
cfn!(s_succ_b, |c, slf, a, b_| { let v = succ_bytes(b(slf)); modify(c, slf)?.b = v; Ok(slf) });
cfn!(s_upto, |c, slf, a, blk| {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, s::upto, a, NIL); }
    let last = expect_str(c, a[0])?;
    let excl = a.len() > 1 && a[1].truthy();
    let lb = b(last).to_vec();
    // numeric strings iterate numerically
    let all_digits = |v: &[u8]| !v.is_empty() && v.iter().all(|x| x.is_ascii_digit());
    if all_digits(b(slf)) && all_digits(&lb) && b(slf).len() <= lb.len() {
        let (x, y): (i64, i64) = (String::from_utf8_lossy(b(slf)).parse().unwrap_or(0), String::from_utf8_lossy(&lb).parse().unwrap_or(0));
        let w = b(slf).len();
        let mut i = x;
        while if excl { i < y } else { i <= y } { let s2 = str_new(format!("{:0w$}", i, w = w).as_bytes()); yield1(c, blk, s2)?; i += 1; }
        return Ok(slf);
    }
    let mut cur = b(slf).to_vec();
    if cur.len() > lb.len() { return Ok(slf); }
    loop {
        if excl && cur == lb { break; }
        let s2 = new_like(slf, cur.clone());
        yield1(c, blk, s2)?;
        if cur == lb || cur.len() > lb.len() { break; }
        cur = succ_bytes(&cur);
        if cur.len() > lb.len() { break; }
    }
    Ok(slf)
});
cfn!(s_hex, |c, slf, a, b_| Ok(str_to_inum(b(slf), 16, false).unwrap_or(Value::fix(0))));
cfn!(s_oct, |c, slf, a, b_| {
    let v = String::from_utf8_lossy(b(slf)).trim().to_ascii_lowercase();
    let (neg, body) = if let Some(r) = v.strip_prefix('-') { (true, r.to_string()) } else { (false, v.clone()) };
    let base = if body.starts_with("0x") { 16 } else if body.starts_with("0b") { 2 } else if body.starts_with("0o") || body.starts_with('0') { 8 } else { 8 };
    let r = str_to_inum(body.as_bytes(), if body.starts_with("0x") || body.starts_with("0b") || body.starts_with("0o") { 0 } else { base }, false).unwrap_or(Value::fix(0));
    if neg { funcall(c, r, s::uminus, &[], std::ptr::null()) } else { Ok(r) }
});
cfn!(s_freeze, |c, slf, a, b_| { obj(slf).flags |= FL_FROZEN; Ok(slf) });
cfn!(s_uplus, |c, slf, a, b_| if is_frozen(slf) { Ok(str_new_enc(b(slf), rstr(slf).enc)) } else { Ok(slf) });
cfn!(s_uminus, |c, slf, a, b_| {
    if is_frozen(slf) && obj(slf).klass == vm().c_string { return Ok(slf); }
    let key = b(slf).to_vec();
    if let Some(&v) = vm().frozen_lits.get(&key) { return Ok(v); }
    let v = str_new_enc(&key, rstr(slf).enc); obj(v).flags |= FL_FROZEN;
    vm().frozen_lits.insert(key, v);
    Ok(v)
});
cfn!(s_encoding, |c, slf, a, b_| crate::builtins::misc::encoding_value(c, rstr(slf).enc));
cfn!(s_force_encoding, |c, slf, a, b_| { let e = crate::builtins::misc::encoding_id(c, a[0])?; check_frozen(c, slf)?; rstr(slf).enc = e; Ok(slf) });
cfn!(s_b, |c, slf, a, b_| Ok(str_new_enc(b(slf), ENC_BINARY)));
cfn!(s_valid_encoding, |c, slf, a, b_| Ok(Value::bool(rstr(slf).enc != ENC_UTF8 || valid_utf8(b(slf)))));
cfn!(s_ascii_only, |c, slf, a, b_| Ok(Value::bool(is_ascii(b(slf)))));
cfn!(s_unicode_normalize, |c, slf, a, b_| Ok(new_like(slf, b(slf).to_vec())));
cfn!(s_encode, |c, slf, a, b_| {
    let mut enc = ENC_UTF8;
    if let Some(&e) = a.first() { if !is_hash(e) { enc = crate::builtins::misc::encoding_id(c, e)?; } }
    Ok(str_new_enc(b(slf), enc))
});
cfn!(s_scrub, |c, slf, a, b_| {
    let rep = if !a.is_empty() && is_string(a[0]) { String::from_utf8_lossy(b(a[0])).into_owned() } else { "\u{FFFD}".into() };
    let v = b(slf);
    let mut out = String::new(); let mut i = 0;
    while i < v.len() { match std::str::from_utf8(&v[i..]) { Ok(s2) => { out.push_str(s2); break; } Err(e) => { out.push_str(std::str::from_utf8(&v[i..i + e.valid_up_to()]).unwrap()); out.push_str(&rep); i += e.valid_up_to() + e.error_len().unwrap_or(v.len() - i - e.valid_up_to()).max(1); } } }
    Ok(new_like(slf, out.into_bytes()))
});
cfn!(s_casecmp, |c, slf, a, b_| {
    if !is_string(a[0]) { if a[0].is_heap() && respond_to(a[0], s::to_str, true) { let o = expect_str(c, a[0])?; return s_casecmp(c, slf, &[o], b_); } return type_err(c, format!("no implicit conversion of {} into String", class_name_for_err(a[0]))); }
    let x: Vec<u8> = b(slf).iter().map(|c| c.to_ascii_lowercase()).collect();
    let y: Vec<u8> = b(a[0]).iter().map(|c| c.to_ascii_lowercase()).collect();
    Ok(Value::fix(x.cmp(&y) as i64))
});
cfn!(s_casecmp_p, |c, slf, a, b_| {
    if !is_string(a[0]) { if a[0].is_heap() && respond_to(a[0], s::to_str, true) { let o = expect_str(c, a[0])?; return s_casecmp_p(c, slf, &[o], b_); } return type_err(c, format!("no implicit conversion of {} into String", class_name_for_err(a[0]))); }
    Ok(Value::bool(downcase(b(slf), ENC_UTF8) == downcase(b(a[0]), ENC_UTF8)))
});
cfn!(s_partition, |c, slf, a, b_| {
    let v = b(slf).to_vec();
    let (ms, me) = if regexp::is_regexp(a[0]) {
        match regexp::search(c, a[0], slf, 0, true)? { m if m == NIL => (usize::MAX, 0), m => regexp::rmatch(m).unwrap().groups[0].unwrap() }
    } else { let p = expect_str(c, a[0])?; match find_bytes(&v, b(p), 0) { Some(i) => (i, i + b(p).len()), None => (usize::MAX, 0) } };
    if ms == usize::MAX { return Ok(ary_from_vec(vec![new_like(slf, v), str_new(b""), str_new(b"")])); }
    Ok(ary_from_vec(vec![new_like(slf, v[..ms].to_vec()), new_like(slf, v[ms..me].to_vec()), new_like(slf, v[me..].to_vec())]))
});
cfn!(s_rpartition, |c, slf, a, b_| {
    let v = b(slf).to_vec();
    let found = if regexp::is_regexp(a[0]) {
        let mut p = v.len() as i64; let mut r = None;
        while p >= 0 { if let Some(g) = regexp::rregexp(a[0]).unwrap().re.as_ref().unwrap().match_at(&v, p as usize) { r = Some(g[0].unwrap()); break; } p -= 1; }
        r
    } else { let p = expect_str(c, a[0])?; rfind_bytes(&v, b(p), v.len()).map(|i| (i, i + b(p).len())) };
    match found {
        None => Ok(ary_from_vec(vec![str_new(b""), str_new(b""), new_like(slf, v)])),
        Some((ms, me)) => Ok(ary_from_vec(vec![new_like(slf, v[..ms].to_vec()), new_like(slf, v[ms..me].to_vec()), new_like(slf, v[me..].to_vec())])),
    }
});
cfn!(s_delete_prefix, |c, slf, a, b_| { let p = expect_str(c, a[0])?; let v = b(slf); Ok(new_like(slf, if v.starts_with(b(p)) { v[b(p).len()..].to_vec() } else { v.to_vec() })) });
cfn!(s_delete_suffix, |c, slf, a, b_| { let p = expect_str(c, a[0])?; let v = b(slf); Ok(new_like(slf, if v.ends_with(b(p)) { v[..v.len() - b(p).len()].to_vec() } else { v.to_vec() })) });
cfn!(s_delete_prefix_b, |c, slf, a, b_| { let p = expect_str(c, a[0])?; check_frozen(c, slf)?; let n = b(p).len(); if b(slf).starts_with(b(p)) { rstr(slf).b.drain(..n); Ok(slf) } else { Ok(NIL) } });
cfn!(s_delete_suffix_b, |c, slf, a, b_| { let p = expect_str(c, a[0])?; check_frozen(c, slf)?; let n = b(p).len(); if b(slf).ends_with(b(p)) { let l = b(slf).len(); rstr(slf).b.truncate(l - n); Ok(slf) } else { Ok(NIL) } });
cfn!(s_slice_b, |c, slf, a, b_| {
    let r = str_aref(c, slf, a)?;
    if r == NIL { return Ok(NIL); }
    let mut args: Vec<Value> = a.to_vec(); args.push(str_new(b""));
    s_aset(c, slf, &args, b_)?;
    Ok(r)
});
cfn!(s_getbyte, |c, slf, a, b_| { let i = num_to_long(c, a[0])?; let n = b(slf).len() as i64; let i = if i < 0 { i + n } else { i }; Ok(if i >= 0 && i < n { Value::fix(b(slf)[i as usize] as i64) } else { NIL }) });
cfn!(s_setbyte, |c, slf, a, b_| { let i = num_to_long(c, a[0])?; let v = num_to_long(c, a[1])?; let n = b(slf).len() as i64; let i2 = if i < 0 { i + n } else { i }; if i2 < 0 || i2 >= n { return err(c, vm().c_index_error, format!("index {} out of string", i)); } modify(c, slf)?.b[i2 as usize] = v as u8; Ok(a[1]) });
cfn!(s_byteslice, |c, slf, a, b_| {
    let n = b(slf).len() as i64;
    let (st, ln) = if a.len() == 2 { (num_to_long(c, a[0])?, num_to_long(c, a[1])?) } else if is_range(a[0]) { match crate::range::range_beg_len(c, a[0], n, 0)? { Some(x) => x, None => return Ok(NIL) } } else { (num_to_long(c, a[0])?, 1) };
    let st = if st < 0 { st + n } else { st };
    if st < 0 || st > n || ln < 0 { return Ok(NIL); }
    let e = (st + ln).min(n);
    Ok(new_like(slf, b(slf)[st as usize..e as usize].to_vec()))
});
cfn!(s_unpack, |c, slf, a, b_| crate::builtins::misc::unpack(c, slf, a));
cfn!(s_unpack1, |c, slf, a, b_| { let r = crate::builtins::misc::unpack(c, slf, a)?; Ok(ary_get(r, 0)) });
cfn!(s_crypt, |c, slf, a, b_| Ok(new_like(slf, b(slf).to_vec())));
cfn!(s_each_grapheme, |c, slf, a, blk| s_each_char(c, slf, a, blk));
cfn!(s_codepoints, |c, slf, a, blk| {
    let v = b(slf); let out = ary_new(); let mut i = 0;
    while i < v.len() { let (cp, l) = if rstr(slf).enc == ENC_UTF8 { decode_utf8(v, i) } else { (v[i] as u32, 1) }; ary_push(out, Value::fix(cp as i64)); i += l; }
    if !blk.is_null() { let n = ary_len(out); for k in 0..n { yield1(c, blk, ary_get(out, k))?; } return Ok(slf); }
    Ok(out)
});
cfn!(s_squish, |c, slf, a, b_| Ok(slf));
cfn!(s_between, |c, slf, a, b_| Ok(Value::bool(b(slf) >= b(a[0]) && b(slf) <= b(a[1]))));
cfn!(s_intern_frozen_p, |c, slf, a, b_| Ok(Value::bool(is_frozen(slf))));
cfn!(s_dup, |c, slf, a, b_| crate::builtins::object::obj_dup(c, slf, false, None));
cfn!(s_s_try_convert, |c, slf, a, b_| { if is_string(a[0]) { return Ok(a[0]); } if a[0].is_heap() && respond_to(a[0], s::to_str, true) { return expect_str(c, a[0]); } Ok(NIL) });

pub fn init() {
    let k = vm().c_string;
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
    d("end_with?", s_end_with, -1); d("index", s_index, -1); d("rindex", s_rindex, -1); d("byteindex", s_index, -1);
    d("strip", s_strip, 0); d("strip!", s_strip_b, 0); d("lstrip", s_lstrip, 0); d("lstrip!", s_lstrip_b, 0); d("rstrip", s_rstrip, 0); d("rstrip!", s_rstrip_b, 0);
    d("chomp", s_chomp, -1); d("chomp!", s_chomp_b, -1); d("chop", s_chop, 0); d("chop!", s_chop_b, 0); d("chr", s_chr, 0);
    d("chars", s_chars, 0); d("each_char", s_each_char, 0); d("bytes", s_bytes, 0); d("each_byte", s_each_byte, 0);
    d("lines", s_lines, -1); d("each_line", s_each_line, -1); d("split", s_split, -1); d("sub", s_sub, -1); d("gsub", s_gsub, -1);
    d("sub!", s_sub_b, -1); d("gsub!", s_gsub_b, -1); d("scan", s_scan, 1); d("tr", s_tr, 2); d("tr!", s_tr_b, 2); d("tr_s", s_tr_s, 2);
    d("delete", s_delete, -1); d("delete!", s_delete_b, -1); d("count", s_count, -1); d("squeeze", s_squeeze, -1); d("squeeze!", s_squeeze_b, -1);
    d("ljust", s_ljust, -1); d("rjust", s_rjust, -1); d("center", s_center, -1); d("empty?", s_empty, 0); d("replace", s_replace, 1);
    d("initialize_copy", s_replace, 1); d("clear", s_clear, 0); d("insert", s_insert, 2); d("prepend", s_prepend, -1); d("ord", s_ord, 0); d("sum", s_sum, -1);
    d("succ", s_succ, 0); d("next", s_succ, 0); d("succ!", s_succ_b, 0); d("next!", s_succ_b, 0); d("upto", s_upto, -1); d("hex", s_hex, 0); d("oct", s_oct, 0);
    d("freeze", s_freeze, 0); d("+@", s_uplus, 0); d("-@", s_uminus, 0); d("dedup", s_uminus, 0); d("encoding", s_encoding, 0);
    d("force_encoding", s_force_encoding, 1); d("b", s_b, 0); d("valid_encoding?", s_valid_encoding, 0); d("ascii_only?", s_ascii_only, 0);
    d("unicode_normalize", s_unicode_normalize, -1); d("encode", s_encode, -1); d("scrub", s_scrub, -1); d("casecmp", s_casecmp, 1); d("casecmp?", s_casecmp_p, 1);
    d("partition", s_partition, 1); d("rpartition", s_rpartition, 1); d("delete_prefix", s_delete_prefix, 1); d("delete_suffix", s_delete_suffix, 1);
    d("delete_prefix!", s_delete_prefix_b, 1); d("delete_suffix!", s_delete_suffix_b, 1); d("slice!", s_slice_b, -1); d("getbyte", s_getbyte, 1); d("setbyte", s_setbyte, 2);
    d("byteslice", s_byteslice, -1); d("unpack", s_unpack, -1); d("unpack1", s_unpack1, -1); d("crypt", s_crypt, 1); d("each_grapheme_cluster", s_each_grapheme, 0);
    d("grapheme_clusters", s_chars, 0); d("codepoints", s_codepoints, 0); d("each_codepoint", s_codepoints, 0); d("between?", s_between, 2);
    define_singleton_cfunc(k, "try_convert", s_s_try_convert, 1);
    let _ = (s_squish, s_intern_frozen_p, s_dup, s_s_new);
}
