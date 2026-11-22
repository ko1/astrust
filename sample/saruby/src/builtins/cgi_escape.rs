// cgi/escape.so (ext/cgi/escape/escape.c): CGI::EscapeExt, prepended to CGI::Escape (cgi/escape.rb).
#![forbid(unsafe_code)]
use crate::array::*;
use crate::builtins::encoding as en;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

fn nb() -> *const Block { std::ptr::null() }
fn ext_mod(c: &Ctx) -> Value {
    let cgi = const_get_at(c, c.vm().c_object.get(), intern(b"CGI")).unwrap_or(NIL);
    if cgi == NIL { NIL } else { const_get_at(c, cgi, intern(b"EscapeExt")).unwrap_or(NIL) }
}
/// rb_call_super: the pure-Ruby CGI::Escape method this one is prepended over.
fn call_super(c: &mut Ctx, slf: Value, name: &str, a: &[Value]) -> RV {
    let um = ivar_get(c, ext_mod(c), intern(format!("__super_{}", name).as_bytes()));
    let mut args = vec![slf];
    args.extend_from_slice(a);
    funcall(c, um, intern(b"bind_call"), &args, nb())
}
/// A new String of `like`'s class and encoding (rb_str_dup / preserve_original_state).
fn like(c: &mut Ctx, orig: Value, b: Vec<u8>, keep_class: bool) -> Value {
    let e = c.str_enc(orig);
    let n = c.str_from_vec_enc(b, e);
    if keep_class { obj_set_klass(n, real_class_of(c, orig)); }
    n
}

fn escape_html(c: &mut Ctx, s: Value) -> Value {
    let b = c.str_bytes(s);
    let mut out = Vec::with_capacity(b.len());
    for &x in b {
        match x { b'\'' => out.extend_from_slice(b"&#39;"), b'&' => out.extend_from_slice(b"&amp;"), b'"' => out.extend_from_slice(b"&quot;"),
            b'<' => out.extend_from_slice(b"&lt;"), b'>' => out.extend_from_slice(b"&gt;"), _ => out.push(x) }
    }
    let changed = out.len() > b.len();
    like(c, s, out, !changed)
}
/// ruby_scan_digits for base 10 / 16: (value, digits read, overflow)
fn scan_digits(b: &[u8], base: u32) -> (u64, usize, bool) {
    let (mut v, mut n, mut ov) = (0u64, 0usize, false);
    while n < b.len() {
        let Some(d) = (b[n] as char).to_digit(36) else { break };
        if d >= base { break; }
        match v.checked_mul(base as u64).and_then(|x| x.checked_add(d as u64)) { Some(x) => v = x, None => { ov = true; v = v.wrapping_mul(base as u64).wrapping_add(d as u64); } }
        n += 1;
    }
    (v, n, ov)
}
fn utf8_put(cp: u32, out: &mut Vec<u8>) {
    if cp < 0x80 { out.push(cp as u8); }
    else if cp < 0x800 { out.extend_from_slice(&[0xc0 | (cp >> 6) as u8, 0x80 | (cp & 0x3f) as u8]); }
    else if cp < 0x10000 { out.extend_from_slice(&[0xe0 | (cp >> 12) as u8, 0x80 | ((cp >> 6) & 0x3f) as u8, 0x80 | (cp & 0x3f) as u8]); }
    else { out.extend_from_slice(&[0xf0 | (cp >> 18) as u8, 0x80 | ((cp >> 12) & 0x3f) as u8, 0x80 | ((cp >> 6) & 0x3f) as u8, 0x80 | (cp & 0x3f) as u8]); }
}
/// optimized_unescape_html
fn unescape_html(c: &mut Ctx, s: Value) -> Value {
    let enc = c.str_enc(s);
    let name = en::enc_name(enc);
    let limit: u64 = if name.eq_ignore_ascii_case("UTF-8") { 0x10ffff } else if name.eq_ignore_ascii_case("ISO-8859-1") { 256 } else { 128 };
    let b = c.str_bytes(s).to_vec();
    let len = b.len();
    let at = |i: usize| if i < len { b[i] } else { 0 };
    let mut dest: Option<Vec<u8>> = None;
    let (mut i, mut beg) = (0usize, 0usize);
    while i < len {
        if b[i] != b'&' { i += 1; continue; }
        let plen = i - beg;
        i += 1;
        if i >= len { break; }
        let j = i;
        let m = |i: usize, lit: &[u8]| len - i >= lit.len() && &b[i..i + lit.len()] == lit;
        let ch: u8;
        match b[i] {
            b'a' if m(i + 1, b"pos;") => { ch = b'\''; i += 4; }
            b'a' if m(i + 1, b"mp;") => { ch = b'&'; i += 3; }
            b'q' if m(i + 1, b"uot;") => { ch = b'"'; i += 4; }
            b'g' if m(i + 1, b"t;") => { ch = b'>'; i += 2; }
            b'l' if m(i + 1, b"t;") => { ch = b'<'; i += 2; }
            b'a' | b'q' | b'g' | b'l' => { i = j; continue; }
            b'#' => {
                i += 1;
                let (cc, clen, ov);
                if len - i >= 2 && at(i).is_ascii_digit() { (cc, clen, ov) = scan_digits(&b[i..], 10); }
                else if (at(i) == b'x' || at(i) == b'X') && { i += 1; len - i >= 2 } && at(i).is_ascii_hexdigit() { (cc, clen, ov) = scan_digits(&b[i..], 16); }
                else { i = j; continue; }
                i += clen;
                if ov || cc >= limit || at(i) != b';' { i = j; continue; }
                let d = dest.get_or_insert_with(|| Vec::with_capacity(len));
                d.extend_from_slice(&b[beg..beg + plen]);
                if limit > 256 { utf8_put(cc as u32, d); } else { d.push(cc as u8); }
                beg = i + 1;
                i += 1;
                continue;
            }
            _ => { continue; }
        }
        let d = dest.get_or_insert_with(|| Vec::with_capacity(len));
        d.extend_from_slice(&b[beg..beg + plen]);
        d.push(ch);
        beg = i + 1;
        i += 1;
    }
    match dest {
        Some(mut d) => { d.extend_from_slice(&b[beg..]); like(c, s, d, false) }
        None => like(c, s, b, true),
    }
}
fn unreserved(x: u8) -> bool { x.is_ascii_alphanumeric() || matches!(x, b'-' | b'.' | b'_' | b'~') }
/// optimized_escape
fn escape(c: &mut Ctx, s: Value, plus: bool) -> Value {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let b = c.str_bytes(s);
    let mut out = Vec::with_capacity(b.len());
    let mut changed = false;
    for &x in b {
        if unreserved(x) { out.push(x); continue; }
        changed = true;
        if plus && x == b' ' { out.push(b'+'); } else { out.extend_from_slice(&[b'%', HEX[(x >> 4) as usize], HEX[(x & 15) as usize]]); }
    }
    like(c, s, out, !changed)
}
/// optimized_unescape
fn unescape(c: &mut Ctx, s: Value, encv: Value, plus: bool) -> RV {
    let enc = en::to_enc(c, encv)?;
    let b = c.str_bytes(s).to_vec();
    let len = b.len();
    let hexv = |x: u8| (x as char).to_digit(16).unwrap_or(0) as u8;
    let mut dest: Option<Vec<u8>> = None;
    let (mut i, mut beg) = (0usize, 0usize);
    while i < len {
        let ch = b[i];
        let (byte, clen) = if ch == b'%' {
            if i + 3 > len { break; }
            if !b[i + 1].is_ascii_hexdigit() || !b[i + 2].is_ascii_hexdigit() { i += 1; continue; }
            ((hexv(b[i + 1]) << 4) | hexv(b[i + 2]), 2)
        } else if plus && ch == b'+' { (b' ', 0) } else { i += 1; continue; };
        let d = dest.get_or_insert_with(|| Vec::with_capacity(len));
        d.extend_from_slice(&b[beg..i]);
        i += clen;
        beg = i + 1;
        d.push(byte);
        i += 1;
    }
    let orig = c.str_enc(s);
    let r = match dest { Some(mut d) => { d.extend_from_slice(&b[beg..]); like(c, s, d, false) } None => like(c, s, b, true) };
    if orig != enc {
        let valid = en::is_valid(enc, c.str_bytes(r));
        c.rstring_mut(r).enc = if valid { enc } else { orig };
    }
    Ok(r)
}
fn accept_charset(c: &mut Ctx, slf: Value, a: &[Value]) -> RV {
    if let Some(&e) = a.get(1) { return Ok(e); }
    let k = class_of(slf);
    match crate::rt::cvar_lookup(c, k, intern(b"@@accept_charset")) {
        Some((_, v)) => Ok(v),
        None => err(c, c.vm().c_name_error.get(), format!("uninitialized class variable @@accept_charset in {}", class_path(c, real_class_of(c, slf)))),
    }
}
fn ascii(c: &Ctx, s: Value) -> bool { en::ascii_compat(c.str_enc(s)) }
fn e_escape_html(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let s = expect_str(c, a[0])?;
    if ascii(c, s) { Ok(escape_html(c, s)) } else { call_super(c, slf, "escapeHTML", &[s]) }
}
fn e_unescape_html(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let s = expect_str(c, a[0])?;
    if ascii(c, s) { Ok(unescape_html(c, s)) } else { call_super(c, slf, "unescapeHTML", &[s]) }
}
fn e_escape(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let s = expect_str(c, a[0])?;
    if ascii(c, s) { Ok(escape(c, s, true)) } else { call_super(c, slf, "escape", &[s]) }
}
fn e_escape_uri(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let s = expect_str(c, a[0])?;
    if ascii(c, s) { Ok(escape(c, s, false)) } else { call_super(c, slf, "escapeURIComponent", &[s]) }
}
fn e_unescape(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let s = expect_str(c, a[0])?;
    if !ascii(c, s) { return call_super(c, slf, "unescape", a); }
    let e = accept_charset(c, slf, a)?;
    unescape(c, s, e, true)
}
fn e_unescape_uri(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let s = expect_str(c, a[0])?;
    if !ascii(c, s) { return call_super(c, slf, "unescapeURIComponent", a); }
    let e = accept_charset(c, slf, a)?;
    unescape(c, s, e, false)
}
/// InitVM_escape (after cgi/escape.rb has defined the Ruby versions).
pub fn init(c: &mut Ctx) -> Result<(), Esc> {
    let ob = c.vm().c_object.get();
    let cgi = match const_get_at(c, ob, intern(b"CGI")) { Some(v) => v, None => define_class(c, "CGI", ob) };
    let ext = match const_get_at(c, cgi, intern(b"EscapeExt")) { Some(v) => v, None => define_module_under(c, cgi, "EscapeExt") };
    let esc = match const_get_at(c, cgi, intern(b"Escape")) { Some(v) => v, None => define_module_under(c, cgi, "Escape") };
    for n in ["escapeHTML", "unescapeHTML", "escapeURIComponent", "unescapeURIComponent", "escape", "unescape"] {
        if respond_to_im(c, esc, n)? {
            let s = Value::sym(intern(n.as_bytes()));
            let um = funcall(c, esc, intern(b"instance_method"), &[s], nb())?;
            ivar_set_raw(c, ext, intern(format!("__super_{}", n).as_bytes()), um);
        }
    }
    let d = |c: &mut Ctx, n: &str, f: CFunc, ar: i32| define_cfunc(c, ext, n, f, ar);
    d(c, "escapeHTML", e_escape_html, 1); d(c, "unescapeHTML", e_unescape_html, 1);
    d(c, "escapeURIComponent", e_escape_uri, 1); d(c, "escape_uri_component", e_escape_uri, 1);
    d(c, "unescapeURIComponent", e_unescape_uri, -1); d(c, "unescape_uri_component", e_unescape_uri, -1);
    d(c, "escape", e_escape, 1); d(c, "unescape", e_unescape, -1);
    funcall(c, esc, intern(b"prepend"), &[ext], nb())?;
    funcall(c, cgi, intern(b"extend"), &[ext], nb())?;
    Ok(())
}
fn respond_to_im(c: &mut Ctx, m: Value, n: &str) -> Result<bool, Esc> {
    let s = Value::sym(intern(n.as_bytes()));
    Ok(funcall(c, m, intern(b"method_defined?"), &[s], nb())?.truthy())
}
