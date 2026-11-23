// JSON (generator + parser), native; built on first `require "json"`.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{obj_klass, obj_set_flags, obj_set_klass, Global};
use crate::builtins::marshal::each_pair;
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::hash::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

static M_JSON: Global = Global::new();
static C_STATE: Global = Global::new();
fn jconst(c: &Ctx, name: &str) -> Value { const_get_at(c, M_JSON.get(c), intern(name.as_bytes())).unwrap() }
fn jerr<T>(c: &mut Ctx, klass: &str, msg: impl AsRef<str>) -> Result<T, Esc> { let k = jconst(c, klass); err(c, k, msg) }
/// dst[k] = v for each pair of src, slot by slot (hashing a key may run Ruby code).
fn merge_into(c: &mut Ctx, dst: Value, src: Value) -> Result<(), Esc> { each_pair(c, src, |c, k, v| hash_set(c, dst, k, v)) }
/// GeneratorError for a String that is not valid UTF-8.
fn bad_utf8(c: &mut Ctx, src: Value) -> Esc {
    let e = make_exception(c, jconst(c, "GeneratorError"), "source sequence is illegal/malformed utf-8");
    ivar_set_raw(c, e, intern(b"@invalid_object"), src);
    raise_exc(c, e)
}

// ---- generator --------------------------------------------------------------------------------

struct Gen { out: Vec<u8>, indent: Vec<u8>, space: Vec<u8>, space_before: Vec<u8>, object_nl: Vec<u8>, array_nl: Vec<u8>,
             allow_nan: bool, ascii_only: bool, script_safe: bool, max_nesting: usize, depth: usize, state: Value, opts: Value, coder: Value, strict: bool }

fn opt(c: &mut Ctx, h: Value, k: &str) -> Result<Option<Value>, Esc> { if h == NIL { return Ok(None); } hash_get_opt(c, h, sym_value(k)) }
fn opt_bytes(c: &mut Ctx, h: Value, k: &str) -> Result<Vec<u8>, Esc> { Ok(match opt(c, h, k)? { Some(v) if is_string(v) => c.str_bytes(v).to_vec(), _ => vec![] }) }
fn opt_bool(c: &mut Ctx, h: Value, k: &str) -> Result<bool, Esc> { Ok(opt(c, h, k)?.map(|v| v.truthy()).unwrap_or(false)) }
/// Generator settings from nil / Hash / JSON::State.
fn gen_from(c: &mut Ctx, v: Value) -> Result<Gen, Esc> {
    let (h, state, depth) = if v == NIL { (NIL, NIL, 0) } else if is_hash(v) { (v, NIL, 0) } else if v.is_heap() && real_class_of(c, v) == C_STATE.get(c) {
        let d = ivar_get(c, v, intern(b"@depth"));
        (ivar_get(c, v, intern(b"@opts")), v, if d.is_fixnum() { d.as_fix() as usize } else { 0 })
    } else if respond_to(c, v, intern(b"to_h"), true) { (funcall(c, v, intern(b"to_h"), &[], std::ptr::null())?, NIL, 0) } else { return type_err(c, "can't convert to Hash"); };
    let max_nesting = match opt(c, h, "max_nesting")? { Some(m) if m.is_fixnum() => m.as_fix().max(0) as usize, Some(_) => 0, None => 100 };
    Ok(Gen { out: vec![], indent: opt_bytes(c, h, "indent")?, space: opt_bytes(c, h, "space")?, space_before: opt_bytes(c, h, "space_before")?,
             object_nl: opt_bytes(c, h, "object_nl")?, array_nl: opt_bytes(c, h, "array_nl")?, allow_nan: opt_bool(c, h, "allow_nan")?,
             ascii_only: opt_bool(c, h, "ascii_only")?, script_safe: opt_bool(c, h, "script_safe")? || opt_bool(c, h, "escape_slash")?, max_nesting, depth, state, opts: h, coder: NIL, strict: opt_bool(c, h, "strict")? })
}
/// json 3's float text: shortest digits, plain within 7 trailing zeros, else scientific.
fn float_json(f: f64) -> String {
    if f == 0.0 { return if f.is_sign_negative() { "-0.0".into() } else { "0.0".into() }; }
    let e = format!("{:e}", f.abs());
    let (mant, exp) = e.split_once('e').unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let nd = digits.len() as i32;
    let k = exp.parse::<i32>().unwrap() - (nd - 1);
    let ex = (k + nd - 1).abs();
    let neg = f < 0.0;
    let body = if k >= 0 && ex < nd + if neg { 6 } else { 7 } { format!("{}{}.0", digits, "0".repeat(k as usize)) }
    else if k < 0 && (k > -7 || ex < 10) {
        let off = nd - k.abs();
        if off <= 0 { format!("0.{}{}", "0".repeat((-off) as usize), digits) } else { format!("{}.{}", &digits[..off as usize], &digits[off as usize..]) }
    } else { format!("{}{}{}e{}{}", &digits[..1], if nd > 1 { "." } else { "" }, &digits[1..], if k + nd - 1 < 0 { "-" } else { "+" }, ex) };
    if neg { format!("-{}", body) } else { body }
}
impl Gen {
    /// Write b as a JSON string; false (nothing written) if it is not UTF-8 (the caller raises).
    #[must_use]
    fn string(&mut self, b: &[u8]) -> bool {
        if std::str::from_utf8(b).is_err() { return false; }
        self.out.push(b'"');
        let mut i = 0;
        while i < b.len() {
            let ch = b[i];
            match ch {
                b'"' => self.out.extend_from_slice(b"\\\""), b'\\' => self.out.extend_from_slice(b"\\\\"),
                b'\n' => self.out.extend_from_slice(b"\\n"), b'\r' => self.out.extend_from_slice(b"\\r"), b'\t' => self.out.extend_from_slice(b"\\t"),
                8 => self.out.extend_from_slice(b"\\b"), 12 => self.out.extend_from_slice(b"\\f"),
                b'/' if self.script_safe => self.out.extend_from_slice(b"\\/"),
                0..=0x1f => self.out.extend_from_slice(format!("\\u{:04x}", ch).as_bytes()),
                0x80..=0xff => {
                    let (cp, l) = decode_utf8(b, i);
                    if self.ascii_only || (self.script_safe && (cp == 0x2028 || cp == 0x2029)) {
                        if cp >= 0x10000 { let v = cp - 0x10000; self.out.extend_from_slice(format!("\\u{:04x}\\u{:04x}", 0xd800 + (v >> 10), 0xdc00 + (v & 0x3ff)).as_bytes()); }
                        else { self.out.extend_from_slice(format!("\\u{:04x}", cp).as_bytes()); }
                    } else { self.out.extend_from_slice(&b[i..i + l]); }
                    i += l; continue;
                }
                _ => self.out.push(ch),
            }
            i += 1;
        }
        self.out.push(b'"');
        true
    }
    fn nl_indent(&mut self, nl: bool) { if nl { for _ in 0..self.depth { self.out.extend_from_slice(&self.indent); } } }
    fn enter(&mut self, c: &mut Ctx) -> Result<(), Esc> {
        self.depth += 1;
        if self.max_nesting != 0 && self.depth > self.max_nesting { return jerr(c, "NestingError", format!("nesting of {} is too deep. Did you try to serialize objects with circular references?", self.depth)); }
        Ok(())
    }
    fn value(&mut self, c: &mut Ctx, v: Value) -> Result<(), Esc> {
        if v == NIL { self.out.extend_from_slice(b"null"); }
        else if v == TRUE { self.out.extend_from_slice(b"true"); }
        else if v == FALSE { self.out.extend_from_slice(b"false"); }
        else if v.is_fixnum() { self.out.extend_from_slice(v.as_fix().to_string().as_bytes()); }
        else if is_float(v) {
            let f = float_val(v);
            if !f.is_finite() {
                if !self.allow_nan {
                    let e = make_exception(c, jconst(c, "GeneratorError"), &format!("{} not allowed in JSON", crate::numeric::float_to_s(f)));
                    ivar_set_raw(c, e, intern(b"@invalid_object"), v);
                    return Err(raise_exc(c, e));
                }
                self.out.extend_from_slice(crate::numeric::float_to_s(f).as_bytes());
            } else { self.out.extend_from_slice(float_json(f).as_bytes()); }
        }
        else if is_bignum(v) { self.out.extend_from_slice(c.big(v).to_string_radix(10).as_bytes()); }
        else if is_string(v) && real_class_of(c, v) == c.vm().c_string.get() {
            let enc = c.str_enc(v);
            let ok = if enc == ENC_UTF8 || enc == ENC_ASCII || enc == ENC_BINARY { self.string(c.str_bytes(v)) } else {
                let oh = c.hash_new();
                let o = crate::builtins::encoding::parse_tc_opts(c, oh)?;
                let src = c.str_bytes(v).to_vec();
                let b = crate::builtins::encoding::transcode(c, &src, enc, ENC_UTF8, &o)?;
                self.string(&b)
            };
            if !ok { return Err(bad_utf8(c, v)); }
        }
        else if v.is_sym() { if !self.string(crate::symbol::sym_bytes(v.as_sym())) { return Err(bad_utf8(c, v)); } }
        else if is_array(v) && real_class_of(c, v) == c.vm().c_array.get() {
            if c.ary_len(v) == 0 { self.out.extend_from_slice(b"[]"); return Ok(()); }
            self.enter(c)?;
            self.out.push(b'[');
            self.out.extend_from_slice(&self.array_nl);
            let mut i = 0;
            while let Some(&x) = c.ary(v).get(i) {
                if i > 0 { self.out.push(b','); self.out.extend_from_slice(&self.array_nl); }
                self.nl_indent(true);
                self.value(c, x)?;
                i += 1;
            }
            self.depth -= 1;
            self.out.extend_from_slice(&self.array_nl);
            self.nl_indent(true);
            self.out.push(b']');
        }
        else if is_hash(v) && real_class_of(c, v) == c.vm().c_hash.get() {
            if c.hash_len(v) == 0 { self.out.extend_from_slice(b"{}"); return Ok(()); }
            self.enter(c)?;
            self.out.push(b'{');
            self.out.extend_from_slice(&self.object_nl);
            let mut first = true;
            each_pair(c, v, |c, k, x| {
                if !first { self.out.push(b','); self.out.extend_from_slice(&self.object_nl); }
                first = false;
                self.nl_indent(true);
                let k = if self.coder != NIL && !is_string(k) && !k.is_sym() { call_proc(c, self.coder, &[k, TRUE], std::ptr::null())? } else { k };
                if k.is_sym() {
                    let b = crate::symbol::sym_bytes(k.as_sym());
                    if !self.string(b) { let s = c.str_new(b); return Err(bad_utf8(c, s)); }
                } else {
                    let ks = if is_string(k) { k } else { crate::rt::srb_tostr(c, k)? };
                    if !self.string(c.str_bytes(ks)) { return Err(bad_utf8(c, ks)); }
                }
                self.out.extend_from_slice(&self.space_before);
                self.out.push(b':');
                self.out.extend_from_slice(&self.space);
                self.value(c, x)
            })?;
            self.depth -= 1;
            self.out.extend_from_slice(&self.object_nl);
            self.nl_indent(true);
            self.out.push(b'}');
        }
        else {
            if self.coder != NIL || self.strict {
                // Coder / strict mode: the callback turns it into a JSON-native value, or it is an error
                let r = if self.coder != NIL { call_proc(c, self.coder, &[v, FALSE], std::ptr::null())? } else { v };
                let native = r.is_imm() || is_float(r) || is_bignum(r) || ((is_string(r) || is_array(r) || is_hash(r)) && [c.vm().c_string.get(), c.vm().c_array.get(), c.vm().c_hash.get()].contains(&real_class_of(c, r)));
                if !native || r == v {
                    let e = make_exception(c, jconst(c, "GeneratorError"), &format!("{} not allowed in JSON", class_path(c, real_class_of(c, r))));
                    ivar_set_raw(c, e, intern(b"@invalid_object"), r);
                    return Err(raise_exc(c, e));
                }
                return self.value(c, r);
            }
            // anything else answers for itself: obj.to_json(state)
            if self.state == NIL {
                let st = new_object(c, C_STATE.get(c));
                let o = if self.opts == NIL { c.hash_new() } else { self.opts };
                ivar_set_raw(c, st, intern(b"@opts"), o);
                self.state = st;
            }
            ivar_set_raw(c, self.state, intern(b"@depth"), Value::fix(self.depth as i64));
            let r = funcall(c, v, intern(b"to_json"), &[self.state], std::ptr::null())?;
            let r = expect_str(c, r)?;
            self.out.extend_from_slice(c.str_bytes(r));
        }
        Ok(())
    }
}
fn generate_with(c: &mut Ctx, x: Value, opts: Value) -> RV {
    let mut g = gen_from(c, opts)?;
    g.value(c, x)?;
    Ok(c.str_from_vec(std::mem::take(&mut g.out)))
}
fn j_generate(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { check_argc(c, a, 1, 2)?; generate_with(c, a[0], a.get(1).copied().unwrap_or(NIL)) }
fn pretty_opts(c: &mut Ctx, extra: Value) -> RV {
    let h = c.hash_new();
    for (k, v) in [("indent", "  "), ("space", " "), ("object_nl", "\n"), ("array_nl", "\n")] { let s = c.str_new(v.as_bytes()); hash_set(c, h, sym_value(k), s)?; }
    if extra != NIL { let e = if is_hash(extra) { extra } else { funcall(c, extra, intern(b"to_h"), &[], std::ptr::null())? }; merge_into(c, h, e)?; }
    Ok(h)
}
fn j_pretty_generate(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { check_argc(c, a, 1, 2)?; let o = pretty_opts(c, a.get(1).copied().unwrap_or(NIL))?; generate_with(c, a[0], o) }
fn j_dump(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, kw) = match a.last() { Some(&l) if is_hash(l) && a.len() > 1 => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    let h = c.hash_new();
    hash_set(c, h, sym_value("allow_nan"), TRUE)?; hash_set(c, h, sym_value("max_nesting"), FALSE)?;
    if let Some(k) = kw { merge_into(c, h, k)?; }
    let mut io = NIL;
    for &x in &pos[1..] { if x.is_fixnum() { hash_set(c, h, sym_value("max_nesting"), x)?; } else if x != NIL { io = x; } }
    let s = generate_with(c, pos[0], h)?;
    if io == NIL { return Ok(s); }
    funcall(c, io, intern(b"write"), &[s], std::ptr::null())?;
    Ok(io)
}
/// obj.to_json(state = nil) for the core classes.
fn o_to_json(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let st = a.first().copied().unwrap_or(NIL);
    let core = s.is_imm() || is_float(s) || is_bignum(s) || ((is_string(s) || is_array(s) || is_hash(s)) && [c.vm().c_string.get(), c.vm().c_array.get(), c.vm().c_hash.get()].contains(&real_class_of(c, s)));
    if core { return generate_with(c, s, st); }
    // generic objects (and subclasses of core classes) become their to_s
    let mut g = gen_from(c, st)?;
    if is_array(s) || is_hash(s) { let k = obj_klass(s); obj_set_klass(s, if is_array(s) { c.vm().c_array.get() } else { c.vm().c_hash.get() }); let r = g.value(c, s); obj_set_klass(s, k); r?; }
    else { let t = crate::rt::srb_tostr(c, s)?; if !g.string(c.str_bytes(t)) { return Err(bad_utf8(c, t)); } }
    Ok(c.str_from_vec(std::mem::take(&mut g.out)))
}
fn st_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let h = c.hash_new();
    if let Some(&o) = a.first() { if o != NIL { let o = if is_hash(o) { o } else { funcall(c, o, intern(b"to_h"), &[], std::ptr::null())? }; merge_into(c, h, o)?; } }
    ivar_set_raw(c, s, intern(b"@opts"), h); ivar_set_raw(c, s, intern(b"@depth"), Value::fix(0));
    Ok(NIL)
}
fn st_generate(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { generate_with(c, a[0], s) }
/// State readers / writers are keyed by the called method's name.
fn st_get(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let name = c.cur_method_name().map(crate::symbol::sym_str).unwrap_or_default();
    let key = name.trim_end_matches('?');
    if key == "depth" { return Ok(ivar_get(c, s, intern(b"@depth"))); }
    let v = opt(c, ivar_get(c, s, intern(b"@opts")), key)?;
    Ok(match (key, v) {
        (_, Some(v)) if name.ends_with('?') => Value::bool(v.truthy()),
        (_, Some(v)) => v,
        ("max_nesting", None) => Value::fix(100),
        ("indent" | "space" | "space_before" | "object_nl" | "array_nl", None) => c.str_new(b""),
        _ if name.ends_with('?') => FALSE,
        _ => NIL,
    })
}
fn st_set(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let name = c.cur_method_name().map(crate::symbol::sym_str).unwrap_or_default();
    let key = name.trim_end_matches('=');
    if key == "depth" { ivar_set_raw(c, s, intern(b"@depth"), a[0]); return Ok(a[0]); }
    hash_set(c, ivar_get(c, s, intern(b"@opts")), sym_value(key), a[0])?;
    Ok(a[0])
}
fn st_aref(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { let k = if is_string(a[0]) { c.str_to_sym(a[0]) } else { a[0] }; Ok(hash_get_opt(c, ivar_get(c, s, intern(b"@opts")), k)?.unwrap_or(NIL)) }
fn st_to_h(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.hash_dup(ivar_get(c, s, intern(b"@opts")))) }

// ---- parser -----------------------------------------------------------------------------------

// `src`: the input's bytes, copied once (object_class / array_class callbacks may change the String)
struct Parser { src: Vec<u8>, pos: usize, symbolize: bool, allow_nan: bool, freeze: bool, max_nesting: usize, depth: usize, trailing_comma: bool, dup_key: bool, comments: bool, obj_class: Value, ary_class: Value, on_load: Value, decimal: Option<(Value, Sym)> }
impl Parser {
    fn fail<T>(&self, c: &mut Ctx, msg: &str, at: usize) -> Result<T, Esc> {
        let b = &self.src;
        let (mut line, mut col) = (1, 1);
        for &ch in &b[..at.min(b.len())] { if ch == b'\n' { line += 1; col = 1; } else { col += 1; } }
        jerr(c, "ParserError", format!("{} at line {} column {}", msg, line, col))
    }
    /// The input from `at`, cut to 32 bytes on a character boundary.
    fn frag(&self, at: usize) -> String {
        let b = &self.src;
        let mut e = (at + 32).min(b.len());
        while e > at && e < b.len() && (b[e] & 0xc0) == 0x80 { e -= 1; }
        String::from_utf8_lossy(&b[at.min(b.len())..e]).into_owned()
    }
    fn ws(&mut self) {
        let b = &self.src;
        loop {
            while self.pos < b.len() && matches!(b[self.pos], b' ' | b'\t' | b'\n' | b'\r') { self.pos += 1; }
            if !self.comments || self.pos + 1 >= b.len() || b[self.pos] != b'/' { return; }
            if b[self.pos + 1] == b'/' { while self.pos < b.len() && b[self.pos] != b'\n' { self.pos += 1; } }
            else if b[self.pos + 1] == b'*' { match b[self.pos + 2..].windows(2).position(|w| w == b"*/") { Some(i) => self.pos += i + 4, None => return } }
            else { return; }
        }
    }
    /// A value, passed through the on_load proc (json 3: its result replaces the value).
    fn value(&mut self, c: &mut Ctx) -> RV {
        let v = self.value_raw(c)?;
        self.loaded(c, v)
    }
    fn loaded(&mut self, c: &mut Ctx, v: Value) -> RV {
        if self.on_load == NIL { return Ok(v); }
        funcall(c, self.on_load, intern(b"call"), &[v], std::ptr::null())
    }
    fn value_raw(&mut self, c: &mut Ctx) -> RV {
        self.ws();
        let b = &self.src;
        let Some(&ch) = b.get(self.pos) else { return self.fail(c, "unexpected end of input", self.pos) };
        match ch {
            b'{' => self.object(c),
            b'[' => self.array(c),
            b'"' => self.string(c),
            b'-' | b'0'..=b'9' => {
                if self.allow_nan && b[self.pos..].starts_with(b"-Infinity") { self.pos += 9; return Ok(c.float_new(f64::NEG_INFINITY)); }
                self.number(c)
            }
            _ => {
                for (lit, v) in [(&b"null"[..], NIL), (b"true", TRUE), (b"false", FALSE)] { if b[self.pos..].starts_with(lit) { self.pos += lit.len(); return Ok(v); } }
                if self.allow_nan {
                    if b[self.pos..].starts_with(b"NaN") { self.pos += 3; return Ok(c.float_new(f64::NAN)); }
                    if b[self.pos..].starts_with(b"Infinity") { self.pos += 8; return Ok(c.float_new(f64::INFINITY)); }
                }
                if matches!(ch, b'n' | b't' | b'f' | b'N' | b'I' | b'/') { return self.fail(c, &format!("unexpected token '{}'", self.frag(self.pos)), self.pos); }
                self.fail(c, &format!("unexpected character: '{}'", self.frag(self.pos)), self.pos)
            }
        }
    }
    fn number(&mut self, c: &mut Ctx) -> RV {
        let b = &self.src;
        let st = self.pos;
        let mut i = st;
        if b[i] == b'-' { i += 1; }
        let ds = i;
        while i < b.len() && b[i].is_ascii_digit() { i += 1; }
        let mut bad = i == ds || (b[ds] == b'0' && i - ds > 1);
        let mut is_f = false;
        if i < b.len() && b[i] == b'.' { is_f = true; i += 1; let fs = i; while i < b.len() && b[i].is_ascii_digit() { i += 1; } bad |= i == fs; }
        if i < b.len() && (b[i] | 0x20) == b'e' { is_f = true; i += 1; if i < b.len() && (b[i] == b'+' || b[i] == b'-') { i += 1; } let es = i; while i < b.len() && b[i].is_ascii_digit() { i += 1; } bad |= i == es; }
        if bad {
            let mut e = i; while e < b.len() && (b[e].is_ascii_alphanumeric() || matches!(b[e], b'.' | b'+' | b'-')) { e += 1; }
            return self.fail(c, &format!("invalid number: '{}'", String::from_utf8_lossy(&b[st..e])), st);
        }
        self.pos = i;
        if is_f {
            if let Some((recv, mid)) = self.decimal {
                let text = c.str_new_enc(&b[st..i], ENC_BINARY);
                return funcall(c, recv, mid, &[text], std::ptr::null());
            }
            return Ok(c.float_new(std::str::from_utf8(&b[st..i]).unwrap().parse::<f64>().unwrap_or(0.0)));
        }
        Ok(str_to_inum(c, &b[st..i], 10, false).unwrap_or(Value::fix(0)))
    }
    fn string(&mut self, c: &mut Ctx) -> RV {
        let b = &self.src;
        let st = self.pos;
        let mut i = st + 1;
        let mut out: Vec<u8> = vec![];
        loop {
            let Some(&ch) = b.get(i) else { return self.fail(c, "unexpected end of input, expected closing \"", i) };
            match ch {
                b'"' => { i += 1; break; }
                b'\\' => {
                    let Some(&e) = b.get(i + 1) else { return self.fail(c, "unexpected end of input, expected closing \"", i + 1) };
                    i += 2;
                    match e {
                        b'"' | b'\\' | b'/' => out.push(e), b'b' => out.push(8), b'f' => out.push(12), b'n' => out.push(b'\n'), b'r' => out.push(b'\r'), b't' => out.push(b'\t'),
                        b'u' => {
                            let hex4 = |p: usize| -> Option<u32> { if p + 4 > b.len() { return None; } u32::from_str_radix(std::str::from_utf8(&b[p..p + 4]).ok()?, 16).ok() };
                            let Some(mut cp) = hex4(i) else { return self.fail(c, &format!("incomplete unicode character escape sequence: '{}'", self.frag(i - 2)), i - 2) };
                            i += 4;
                            if (0xd800..0xdc00).contains(&cp) && b[i..].starts_with(b"\\u") { if let Some(lo) = hex4(i + 2) { if (0xdc00..0xe000).contains(&lo) { cp = 0x10000 + ((cp - 0xd800) << 10) + (lo - 0xdc00); i += 6; } } }
                            if (0xd800..0xe000).contains(&cp) { out.extend_from_slice(&[0xef, 0xbf, 0xbd]); } else { encode_utf8(cp, &mut out); }
                        }
                        _ => return self.fail(c, &format!("invalid escape character in string: '{}'", self.frag(i - 2)), i - 2),
                    }
                }
                _ => { out.push(ch); i += 1; }
            }
        }
        self.pos = i;
        let v = c.str_from_vec(out);
        if self.freeze { obj_set_flags(v, FL_FROZEN); }
        Ok(v)
    }
    fn enter(&mut self, c: &mut Ctx) -> Result<(), Esc> {
        self.depth += 1;
        if self.max_nesting != 0 && self.depth > self.max_nesting { return jerr(c, "NestingError", format!("nesting of {} is too deep", self.depth)); }
        stack_check(c, 16)
    }
    fn array(&mut self, c: &mut Ctx) -> RV {
        self.enter(c)?;
        self.pos += 1;
        let out = if self.ary_class == NIL { c.ary_new() } else { crate::builtins::object::c_new(c, self.ary_class, &[], Blk::NONE)? };
        self.ws();
        if self.src.get(self.pos) == Some(&b']') { self.pos += 1; self.depth -= 1; return Ok(out); }
        loop {
            let v = self.value(c)?;
            if is_array(out) { c.ary_push(out, v); } else { funcall(c, out, intern(b"<<"), &[v], std::ptr::null())?; }
            self.ws();
            match self.src.get(self.pos) {
                Some(b',') => { self.pos += 1; if self.trailing_comma { self.ws(); if self.src.get(self.pos) == Some(&b']') { self.pos += 1; break; } } }
                Some(b']') => { self.pos += 1; break; }
                None => return self.fail(c, "unexpected end of input", self.pos),
                _ => return self.fail(c, "expected ',' or ']' after array value", self.pos),
            }
        }
        self.depth -= 1;
        if self.freeze { obj_set_flags(out, FL_FROZEN); }
        Ok(out)
    }
    fn object(&mut self, c: &mut Ctx) -> RV {
        self.enter(c)?;
        let start = self.pos;
        self.pos += 1;
        let out = if self.obj_class == NIL { c.hash_new() } else { crate::builtins::object::c_new(c, self.obj_class, &[], Blk::NONE)? };
        self.ws();
        if self.src.get(self.pos) == Some(&b'}') { self.pos += 1; self.depth -= 1; return Ok(out); }
        let mut after_comma = false;
        loop {
            self.ws();
            match self.src.get(self.pos) {
                Some(b'"') => {}
                None => return self.fail(c, "expected object key, got EOF", self.pos),
                Some(b'}') if after_comma && self.trailing_comma => { self.pos += 1; break; }
                _ => return self.fail(c, &format!("expected object key, got{} '{}'", if after_comma { ":" } else { "" }, self.frag(self.pos)), self.pos),
            }
            let k = self.string(c)?;
            let k = self.loaded(c, k)?;
            self.ws();
            if self.src.get(self.pos) != Some(&b':') { return self.fail(c, "expected ':' after object key", self.pos); }
            self.pos += 1;
            let v = self.value(c)?;
            let k = if self.symbolize { c.str_to_sym(k) } else { k };
            if is_hash(out) && !self.dup_key && hash_lookup(c, out, k)?.is_some() {
                let d = if k.is_sym() { format!("\"{}\"", crate::symbol::sym_str(k.as_sym())) } else { inspect_str(c, k)? };
                return self.fail(c, &format!("duplicate key {}", d), start);
            }
            if is_hash(out) { hash_aset(c, out, k, v)?; } else { funcall(c, out, intern(b"[]="), &[k, v], std::ptr::null())?; }
            self.ws();
            match self.src.get(self.pos) {
                Some(b',') => { self.pos += 1; after_comma = true; }
                Some(b'}') => { self.pos += 1; break; }
                None => return self.fail(c, "unexpected end of input", self.pos),
                _ => return self.fail(c, "expected ',' or '}' after object value", self.pos),
            }
        }
        self.depth -= 1;
        if self.freeze { obj_set_flags(out, FL_FROZEN); }
        Ok(out)
    }
}
/// decimal_class: receiver and method for a float literal (json 3.1 parser.c: try_convert, new, or Kernel#<Name>).
fn decimal_class(c: &mut Ctx, k: Value) -> Result<Option<(Value, Sym)>, Esc> {
    if !k.truthy() { return Ok(None); }
    if respond_to(c, k, intern(b"try_convert"), false) { return Ok(Some((k, intern(b"try_convert")))); }
    if respond_to(c, k, intern(b"new"), false) { return Ok(Some((k, intern(b"new")))); }
    if !is_class_or_module(k) || c.class(k).is_module { return Ok(None); }
    let name = class_path(c, k);
    match name.rfind("::") {
        Some(i) => {
            let (o, path) = (c.vm().c_object.get(), c.str_new(name[..i].as_bytes()));
            let m = funcall(c, o, intern(b"const_get"), &[path], std::ptr::null())?;
            Ok(Some((m, intern(name[i + 2..].as_bytes()))))
        }
        None => Ok(Some((c.vm().m_kernel.get(), intern(name.as_bytes())))),
    }
}
fn parse_with(c: &mut Ctx, src: Value, opts: Value, bang: bool) -> RV {
    let src = expect_str(c, src)?;
    let mut p = Parser { src: c.str_bytes(src).to_vec(), pos: 0, symbolize: opt_bool(c, opts, "symbolize_names")?, allow_nan: bang || opt_bool(c, opts, "allow_nan")?, freeze: opt_bool(c, opts, "freeze")?,
        max_nesting: match opt(c, opts, "max_nesting")? { Some(m) if m.is_fixnum() => m.as_fix().max(0) as usize, _ => 0 }, depth: 0,
        trailing_comma: opt_bool(c, opts, "allow_trailing_comma")?, comments: opt_bool(c, opts, "allow_comments")?, dup_key: opt(c, opts, "allow_duplicate_key")?.map(|v| v.truthy()).unwrap_or(false), obj_class: NIL, ary_class: NIL, on_load: NIL, decimal: None };
    if let Some(pr) = opt(c, opts, "on_load")? { p.on_load = pr; }
    if let Some(k) = opt(c, opts, "decimal_class")? { p.decimal = decimal_class(c, k)?; }
    if let Some(k) = opt(c, opts, "object_class")? { if k != NIL && k != c.vm().c_hash.get() { p.obj_class = k; } }
    if let Some(k) = opt(c, opts, "array_class")? { if k != NIL && k != c.vm().c_array.get() { p.ary_class = k; } }
    let v = p.value(c)?;
    p.ws();
    if p.pos < p.src.len() { return p.fail(c, &format!("unexpected token at end of stream '{}'", p.frag(p.pos)), p.pos); }
    Ok(v)
}
fn j_parse(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { check_argc(c, a, 1, 2)?; parse_with(c, a[0], a.get(1).copied().unwrap_or(NIL), false) }
fn j_parse_bang(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { check_argc(c, a, 1, 2)?; parse_with(c, a[0], a.get(1).copied().unwrap_or(NIL), true) }
fn j_load(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, kw) = match a.last() { Some(&l) if is_hash(l) && a.len() > 1 => (&a[..a.len() - 1], l), _ => (a, NIL) };
    let mut src = pos[0];
    let blank_ok = if kw != NIL { hash_get_opt(c, kw, sym_value("allow_blank"))?.map_or(true, |v| v.truthy()) } else { true };
    if src == NIL && blank_ok { return Ok(NIL); }
    if !is_string(src) {
        if respond_to(c, src, intern(b"to_str"), true) { src = expect_str(c, src)?; }
        else if respond_to(c, src, intern(b"to_io"), true) { let io = funcall(c, src, intern(b"to_io"), &[], std::ptr::null())?; src = funcall(c, io, intern(b"read"), &[], std::ptr::null())?; }
        else { src = funcall(c, src, intern(b"read"), &[], std::ptr::null())?; }
    }
    // json 3: only an empty source is blank (whitespace is a parse error)
    if blank_ok && is_string(src) && c.str_len(src) == 0 { return Ok(NIL); }
    let h = c.hash_new();
    hash_set(c, h, sym_value("allow_nan"), TRUE)?;
    if let Some(&p) = pos.get(1) { if p != NIL { hash_set(c, h, sym_value("on_load"), p)?; } }
    if kw != NIL { merge_into(c, h, kw)?; }
    parse_with(c, src, h, false)
}
fn j_load_file(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let s = funcall(c, c.vm().c_file.get(), intern(b"read"), &a[..1], std::ptr::null())?;
    parse_with(c, s, a.get(1).copied().unwrap_or(NIL), false)
}
// JSON::Coder: options plus a callback for values JSON has no notation for
fn coder_initialize(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    let o = a.first().copied().unwrap_or(NIL);
    let o = if o == NIL { c.hash_new() } else { o };
    ivar_set_raw(c, s, intern(b"@opts"), o);
    let pr = if b.is_null() { NIL } else { block_to_proc(c, b)? };
    ivar_set_raw(c, s, intern(b"@proc"), pr);
    Ok(NIL)
}
fn coder_dump(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let mut g = gen_from(c, ivar_get(c, s, intern(b"@opts")))?;
    g.coder = ivar_get(c, s, intern(b"@proc")); g.strict = true;
    if g.opts == NIL || opt(c, g.opts, "max_nesting")?.is_none() { g.max_nesting = 100; }
    g.value(c, a[0])?;
    let out = c.str_from_vec(std::mem::take(&mut g.out));
    match a.get(1) { Some(&io) if io != NIL => { funcall(c, io, intern(b"write"), &[out], std::ptr::null())?; Ok(io) } _ => Ok(out) }
}
fn coder_load(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { parse_with(c, a[0], ivar_get(c, s, intern(b"@opts")), false) }
fn j_aref(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { if is_string(a[0]) { j_parse(c, s, a, b) } else { j_generate(c, s, a, b) } }
fn k_j(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    for &x in a { let s = generate_with(c, x, NIL)?; let mut b = c.str_bytes(s).to_vec(); b.push(b'\n'); stdout_write(c, &b)?; }
    Ok(NIL)
}
fn k_jj(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    for &x in a { let o = pretty_opts(c, NIL)?; let s = generate_with(c, x, o)?; let mut b = c.str_bytes(s).to_vec(); b.push(b'\n'); stdout_write(c, &b)?; }
    Ok(NIL)
}

pub fn init(c: &mut Ctx) {
    if M_JSON.get(c) != NIL { return; }
    let (c_standard_error, c_object, m_kernel) = (c.vm().c_standard_error.get(), c.vm().c_object.get(), c.vm().m_kernel.get());
    let m = define_module(c, "JSON");
    M_JSON.set(c, m);
    let base = define_class_under(c, m, "JSONError", c_standard_error);
    let pe = define_class_under(c, m, "ParserError", base);
    define_class_under(c, m, "NestingError", pe);
    let ge = define_class_under(c, m, "GeneratorError", base);
    define_method_me(c, ge, intern(b"invalid_object"), new_me(intern(b"invalid_object"), ge, VISI_PUBLIC, MKind::AttrReader(intern(b"@invalid_object"))));
    const_set(c, m, intern(b"UnparserError"), ge);
    let ver = c.str_new_frozen(b"3.1.0");
    const_set(c, m, intern(b"VERSION"), ver);
    const_set(c, m, intern(b"JSON_LOADED"), TRUE);
    let mf = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_module_function(c, m, n, f, a);
    mf(c, "generate", j_generate, -1); mf(c, "unparse", j_generate, -1); mf(c, "fast_generate", j_generate, -1); mf(c, "pretty_generate", j_pretty_generate, -1);
    mf(c, "pretty_unparse", j_pretty_generate, -1); mf(c, "dump", j_dump, -1); mf(c, "parse", j_parse, -1); mf(c, "parse!", j_parse_bang, -1); mf(c, "load", j_load, -1);
    mf(c, "unsafe_load", j_load, -1); mf(c, "restore", j_load, -1); mf(c, "load_file", j_load_file, -1); mf(c, "load_file!", j_load_file, -1);
    define_singleton_cfunc(c, m, "[]", j_aref, -1);
    let st = define_class_under(c, m, "State", c_object);
    C_STATE.set(c, st);
    define_private_cfunc(c, st, "initialize", st_initialize, -1);
    define_cfunc(c, st, "generate", st_generate, -1); define_cfunc(c, st, "[]", st_aref, 1); define_cfunc(c, st, "to_h", st_to_h, 0); define_cfunc(c, st, "to_hash", st_to_h, 0);
    for n in ["indent", "space", "space_before", "object_nl", "array_nl", "max_nesting", "depth", "allow_nan?", "ascii_only?", "script_safe?", "strict?"] { define_cfunc(c, st, n, st_get, 0); }
    for n in ["indent=", "space=", "space_before=", "object_nl=", "array_nl=", "max_nesting=", "depth=", "allow_nan=", "ascii_only=", "script_safe=", "strict="] { define_cfunc(c, st, n, st_set, 1); }
    let cd = define_class_under(c, m, "Coder", c_object);
    define_private_cfunc(c, cd, "initialize", coder_initialize, -1);
    define_cfunc(c, cd, "dump", coder_dump, -1); define_cfunc(c, cd, "generate", coder_dump, -1); define_cfunc(c, cd, "load", coder_load, 1); define_cfunc(c, cd, "parse", coder_load, 1);
    let ext = define_module_under(c, m, "Ext");
    let gen = define_module_under(c, ext, "Generator");
    const_set(c, gen, intern(b"State"), st);
    define_cfunc(c, c_object, "to_json", o_to_json, -1);
    define_module_function(c, m_kernel, "j", k_j, -1);
    define_module_function(c, m_kernel, "jj", k_jj, -1);
    define_module_function(c, m_kernel, "JSON", |c, s, a, b| j_aref(c, s, a, b), -1);
    let _ = num_to_long;
}
