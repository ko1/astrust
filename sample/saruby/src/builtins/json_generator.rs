// json/ext/generator: the json gem's ext/json/ext/generator/generator.c, natively
// (JSON::Ext::Generator::State and, for json 2.x, the GeneratorMethods modules).
// 2.x keeps the nesting depth in the State itself; 3.x in the generation and copies it to a
// temporary State only around `to_json` calls.
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::builtins::json_fpconv::dtoa;
use crate::builtins::json_parser::{class_under, json_version, unknown_keywords};
use crate::builtins::kernel::*;
use crate::builtins::marshal::each_pair;
use crate::hash::*;
use crate::heap::{is_heap_float, obj_klass, Global};
use crate::string::*;
use crate::symbol::{intern, s};
use crate::array::*;
use crate::value::*;
use crate::vm::*;

static M_JSON: Global = Global::new();
static C_STATE: Global = Global::new();
static C_FRAGMENT: Global = Global::new();
static E_GENERATOR: Global = Global::new();
static E_NESTING: Global = Global::new();
static M_STRING_EXTEND: Global = Global::new();
static P_SORT_KEYS: Global = Global::new();
static P_RFC_NUMBER: Global = Global::new();
static P_RFC_SORT: Global = Global::new();

fn v2(c: &Ctx) -> bool { json_version(c) < 30000 }
/// rb_warn: "file:line: warning: msg" at the Ruby caller, unless $VERBOSE is nil.
fn rb_warn(c: &mut Ctx, msg: &str) -> Result<(), Esc> {
    if c.vm().verbose == NIL { return Ok(()); }
    let loc = backtrace_lines(c, 0).first().map(|l| l.split(":in ").next().unwrap_or("").to_string()).unwrap_or_default();
    warning_write(c, format!("{}: warning: {}\n", loc, msg).as_bytes())
}

#[derive(Clone, Copy)]
struct Cfg {
    indent: Value, space: Value, space_before: Value, object_nl: Value, array_nl: Value, as_json: Value, sort_keys: Value,
    max_nesting: i64, depth: i64, buffer_initial_length: i64,
    allow_duplicate_key: Option<bool>, allow_nan: bool, ascii_only: bool, script_safe: bool, strict: bool, rfc8785: bool,
}
impl Cfg {
    fn new() -> Cfg {
        Cfg { indent: NIL, space: NIL, space_before: NIL, object_nl: NIL, array_nl: NIL, as_json: FALSE, sort_keys: FALSE,
              max_nesting: 100, depth: 0, buffer_initial_length: 1024, allow_duplicate_key: None, allow_nan: false,
              ascii_only: false, script_safe: false, strict: false, rfc8785: false }
    }
}

pub struct PState { s: Cfg }
impl RData for PState {
    fn visit(&mut self, m: &mut crate::gc::Visitor) {
        let s = &mut self.s;
        for v in [&mut s.indent, &mut s.space, &mut s.space_before, &mut s.object_nl, &mut s.array_nl, &mut s.as_json, &mut s.sort_keys] { m.visit(v); }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn cfg_of(c: &Ctx, v: Value) -> Cfg { c.data::<PState>(v).expect("a JSON::State").s }
fn cfg_mut(c: &mut Ctx, v: Value) -> &mut Cfg { &mut c.data_mut::<PState>(v).expect("a JSON::State").s }
fn is_state(c: &Ctx, v: Value) -> bool { c.data::<PState>(v).is_some() }
fn new_state(c: &mut Ctx, s: Cfg) -> Value { let k = C_STATE.get(c); c.alloc(k, Body::Data(Box::new(PState { s }))) }

fn gen_error(c: &mut Ctx, invalid: Value, msg: &[u8]) -> Esc {
    let k = E_GENERATOR.get(c);
    let m = c.str_new(msg);
    match funcall(c, k, intern(b"new"), &[m], std::ptr::null()) {
        Ok(e) => { ivar_set_raw(e, intern(b"@invalid_object"), invalid); raise_exc(c, e) }
        Err(e) => e,
    }
}
fn nesting_error(c: &mut Ctx, msg: String) -> Esc { let k = E_NESTING.get(c); err::<()>(c, k, msg).unwrap_err() }

const CR_7BIT: u8 = 0;
const CR_VALID: u8 = 1;
const CR_BROKEN: u8 = 2;
fn coderange(c: &Ctx, v: Value) -> u8 {
    let (b, enc) = (c.str_bytes(v), c.str_enc(v));
    if crate::builtins::encoding::ascii_compat(enc) && is_ascii(b) { CR_7BIT }
    else if crate::builtins::encoding::is_valid(enc, b) { CR_VALID } else { CR_BROKEN }
}
fn valid_json_string(c: &Ctx, v: Value) -> bool {
    match coderange(c, v) { CR_7BIT => true, CR_VALID => { let e = c.str_enc(v); e == ENC_UTF8 || e == ENC_ASCII } _ => false }
}
fn to_s(c: &mut Ctx, v: Value) -> RV { let t = funcall(c, v, intern(b"to_s"), &[], std::ptr::null())?; expect_str(c, t) }
fn check_string(c: &mut Ctx, v: Value) -> RV {
    if is_string(v) { Ok(v) } else { type_err(c, format!("wrong argument type {} (expected String)", class_name_for_err(v))) }
}
/// str.encode(UTF-8); a StandardError becomes a GeneratorError with its message.
fn encode_utf8(c: &mut Ctx, v: Value) -> RV {
    let u = crate::builtins::misc::encoding_value(c, ENC_UTF8)?;
    match funcall(c, v, intern(b"encode"), &[u], std::ptr::null()) {
        Ok(r) => Ok(r),
        Err(e) => {
            if c.esc != EscKind::Raise || !is_kind_of(c.esc_val, c.vm().c_standard_error) { return Err(e); }
            let x = c.esc_val;
            c.esc = EscKind::None; c.esc_val = NIL;
            let m = funcall(c, x, intern(b"message"), &[], std::ptr::null())?;
            let mb = c.str_bytes(m).to_vec();
            Err(gen_error(c, v, &mb))
        }
    }
}

// ---- the output buffer --------------------------------------------------------------------------

struct Out { b: Vec<u8>, io: Value, capa: usize, init: usize, v2: bool }
impl Out {
    fn new(io: Value, init: i64, v2: bool) -> Out {
        let init = if init > 0 { init as usize } else if io != NIL { 16383 } else { 1024 };
        Out { b: Vec::with_capacity(if io != NIL { 0 } else { init.min(1 << 20) }), io, capa: if io != NIL && v2 { 512 } else { 0 }, init, v2 }
    }
    /// fbuffer_inc_capa for an IO target: write out what is buffered when `n` more do not fit.
    fn reserve(&mut self, c: &mut Ctx, n: usize) -> Result<(), Esc> {
        if self.io == NIL || n <= self.capa.saturating_sub(self.b.len()) { return Ok(()); }
        if self.v2 {
            if self.capa < 16383 { self.capa = 16383; } else { self.flush(c)?; }
            if n < self.capa { return Ok(()); }
        } else if self.capa != 0 {
            self.flush(c)?;
            if n < self.capa { return Ok(()); }
        }
        let mut nc = if self.capa != 0 { self.capa } else { self.init };
        while nc < n + self.b.len() { nc *= 2; }
        self.capa = self.capa.max(nc);
        Ok(())
    }
    fn flush(&mut self, c: &mut Ctx) -> Result<(), Esc> {
        let s = c.str_from_vec(std::mem::take(&mut self.b));
        funcall(c, self.io, intern(b"write"), &[s], std::ptr::null())?;
        Ok(())
    }
    #[inline]
    fn put(&mut self, c: &mut Ctx, x: &[u8]) -> Result<(), Esc> {
        if x.is_empty() { return Ok(()); }
        if self.io != NIL { self.reserve(c, x.len())?; }
        self.b.extend_from_slice(x);
        Ok(())
    }
    #[inline]
    fn ch(&mut self, c: &mut Ctx, x: u8) -> Result<(), Esc> {
        if self.io != NIL { self.reserve(c, 1)?; }
        self.b.push(x);
        Ok(())
    }
    fn str(&mut self, c: &mut Ctx, v: Value) -> Result<(), Esc> {
        if self.io == NIL { self.b.extend_from_slice(c.str_bytes(v)); return Ok(()); }
        let x = c.str_bytes(v).to_vec();
        self.put(c, &x)
    }
    fn repeat(&mut self, c: &mut Ctx, v: Value, n: i64) -> Result<(), Esc> {
        if n <= 0 { return Ok(()); }
        let x = c.str_bytes(v).to_vec();
        if self.io != NIL { self.reserve(c, x.len() * n as usize)?; }
        for _ in 0..n { self.b.extend_from_slice(&x); }
        Ok(())
    }
}

// ---- string escaping ----------------------------------------------------------------------------

const HEX: &[u8; 16] = b"0123456789abcdef";
fn esc1(o: &mut Vec<u8>, ch: u8) {
    match ch {
        b'"' => o.extend_from_slice(b"\\\""), b'\\' => o.extend_from_slice(b"\\\\"), b'/' => o.extend_from_slice(b"\\/"),
        8 => o.extend_from_slice(b"\\b"), 12 => o.extend_from_slice(b"\\f"), b'\n' => o.extend_from_slice(b"\\n"),
        b'\r' => o.extend_from_slice(b"\\r"), b'\t' => o.extend_from_slice(b"\\t"),
        _ => o.extend_from_slice(&[b'\\', b'u', b'0', b'0', HEX[(ch >> 4) as usize], HEX[(ch & 0xf) as usize]]),
    }
}
fn u_esc(o: &mut Vec<u8>, w: u32) { o.extend_from_slice(&[b'\\', b'u', HEX[(w >> 12) as usize & 0xf], HEX[(w >> 8) as usize & 0xf], HEX[(w >> 4) as usize & 0xf], HEX[w as usize & 0xf]]); }
/// The escaped body of a string whose bytes are valid (convert_UTF8_to_*_JSON).
fn escape_into(o: &mut Vec<u8>, b: &[u8], ascii_only: bool, script_safe: bool) {
    let mut i = 0;
    let mut run = 0;
    while i < b.len() {
        let ch = b[i];
        if ch >= 0x20 && ch != b'"' && ch != b'\\' && ch < 0x80 && !(script_safe && ch == b'/') { i += 1; continue; }
        if ch < 0x80 { o.extend_from_slice(&b[run..i]); esc1(o, ch); i += 1; run = i; continue; }
        let len = match ch { 0xc0..=0xdf => 2, 0xe0..=0xef => 3, 0xf0..=0xf7 => 4, _ => 1 }.min(b.len() - i);
        if ascii_only {
            o.extend_from_slice(&b[run..i]);
            let mut w = match len { 2 => (ch & 0x1f) as u32, 3 => (ch & 0x0f) as u32, 4 => (ch & 0x07) as u32, _ => 0 };
            for k in 1..len { w = (w << 6) | (b[i + k] & 0x3f) as u32; }
            if w <= 0xffff { u_esc(o, w); } else { let w = w - 0x10000; u_esc(o, 0xd800 + (w >> 10)); u_esc(o, 0xdc00 + (w & 0x3ff)); }
            i += len; run = i; continue;
        }
        if script_safe && ch == 0xe2 && len == 3 && b[i + 1] == 0x80 && (b[i + 2] >> 1) == 0x54 {
            o.extend_from_slice(&b[run..i]);
            o.extend_from_slice(if b[i + 2] & 1 != 0 { b"\\u2029" } else { b"\\u2028" });
            i += 3; run = i; continue;
        }
        i += len;
    }
    o.extend_from_slice(&b[run..]);
}

/// The appends of convert_UTF8_to_*_JSON, for an IO target (where each growth may flush).
enum Op { Put(Vec<u8>), Reserve(usize), Add(Vec<u8>) }
fn needs_basic(ch: u8) -> bool { ch < 0x20 || ch == b'"' || ch == b'\\' }
fn escape_ops(b: &[u8], ascii_only: bool, script_safe: bool, v2: bool) -> Vec<Op> {
    let mut ops = vec![];
    let n = b.len();
    let esc = |i: usize, len: usize| { let mut o = vec![]; escape_into(&mut o, &b[i..i + len], ascii_only, script_safe); o };
    if ascii_only || script_safe {
        // the scalar searches: a run, then each escaped character
        let (mut i, mut run) = (0, 0);
        while i < n {
            let ch = b[i];
            let len = match ch { 0xc0..=0xdf => 2, 0xe0..=0xef => 3, 0xf0..=0xf7 => 4, _ => 1 }.min(n - i);
            let hit = if ch < 0x80 { needs_basic(ch) || (script_safe && ch == b'/') }
                else if ascii_only { true } else { len == 3 && ch == 0xe2 && b[i + 1] == 0x80 && (b[i + 2] >> 1) == 0x54 };
            if hit {
                if i > run { ops.push(Op::Put(b[run..i].to_vec())); }
                ops.push(Op::Put(esc(i, len)));
                i += len; run = i;
            } else { i += len; }
        }
        if n > run { ops.push(Op::Put(b[run..n].to_vec())); }
        return ops;
    }
    // search_escape_basic_sse2: 16-byte chunks, the tail copied ahead when 4 or more bytes remain
    let mask_of = |from: usize| -> u32 { let mut m = 0; for k in 0..16 { let ch = if from + k < n { b[from + k] } else { b' ' }; if needs_basic(ch) { m |= 1 << k; } } m };
    let (mut ptr, mut cursor, mut has, mut mask, mut base) = (0usize, 0usize, false, 0u32, 0usize);
    macro_rules! flush { () => { if ptr > cursor { ops.push(Op::Put(b[cursor..ptr].to_vec())); cursor = ptr; } } }
    loop {
        let found = 'search: {
            if has {
                if mask > 0 { ptr = base + mask.trailing_zeros() as usize; mask &= mask - 1; flush!(); break 'search true; }
                has = false;
                ptr = if base + 16 >= n { n } else { base + 16 };
            }
            while ptr + 16 <= n {
                let m = mask_of(ptr);
                if m != 0 { mask = m; has = true; base = ptr; ptr = base + mask.trailing_zeros() as usize; mask &= mask - 1; flush!(); break 'search true; }
                ptr += 16;
            }
            let rem = n - ptr;
            if rem >= if v2 { 6 } else { 4 } {
                flush!();
                ops.push(Op::Reserve(16));
                let m = mask_of(ptr);
                if m == 0 { ops.push(Op::Add(b[ptr..n].to_vec())); ptr = n; cursor = n; break 'search false; }
                has = true; mask = m; base = ptr;
                ptr = base + mask.trailing_zeros() as usize; mask &= mask - 1; flush!();
                break 'search true;
            }
            while ptr < n { if needs_basic(b[ptr]) { flush!(); break 'search true; } ptr += 1; }
            flush!();
            false
        };
        if !found { break; }
        ops.push(Op::Put(esc(ptr, 1)));
        ptr += 1; cursor = ptr;
    }
    let _ = cursor;
    ops
}

// ---- generation -------------------------------------------------------------------------------

struct G { out: Out, s: Cfg, vstate: Value, depth: i64, v2: bool }

impl G {
    fn call_as_json(&self, c: &mut Ctx, ob: Value, is_key: bool) -> RV {
        if self.v2 { call_proc(c, self.s.as_json, &[ob], std::ptr::null()) } else { call_proc(c, self.s.as_json, &[ob, Value::bool(is_key)], std::ptr::null()) }
    }
    fn increase_depth(&mut self, c: &mut Ctx) -> Result<(), Esc> {
        // the C generator recurses on the machine stack too; deep input is a SystemStackError
        stack_check(c, 16)?;
        if self.v2 {
            self.depth += 1;
            if self.s.max_nesting != 0 && self.depth > self.s.max_nesting { self.depth -= 1; return Err(nesting_error(c, format!("nesting of {} is too deep", self.depth))); }
            return Ok(());
        }
        if self.s.max_nesting != 0 && self.depth >= self.s.max_nesting {
            return Err(nesting_error(c, format!("nesting of {} is too deep. Did you try to serialize objects with circular references?", self.depth + 1)));
        }
        self.depth += 1;
        Ok(())
    }
    /// ob.to_json(state) with a State that carries the current depth.
    fn call_to_json(&mut self, c: &mut Ctx, ob: Value) -> RV {
        if self.vstate == NIL { let s = self.s; self.vstate = new_state(c, s); }
        let vs = self.vstate;
        cfg_mut(c, vs).depth = self.depth;
        let r = funcall(c, ob, intern(b"to_json"), &[vs], std::ptr::null());
        // the callee may reconfigure the State; generation goes on with it
        let d = self.depth;
        self.s = cfg_of(c, vs);
        if self.v2 { self.depth = self.s.depth; } else { self.depth = d; }
        r
    }
    fn fallback(&mut self, c: &mut Ctx, ob: Value) -> Result<(), Esc> {
        if respond_to(ob, intern(b"to_json"), false) {
            let t = self.call_to_json(c, ob)?;
            let t = check_string(c, t)?;
            return self.out.str(c, t);
        }
        let t = funcall(c, ob, intern(b"to_s"), &[], std::ptr::null())?;
        let t = if self.v2 { check_string(c, t)? } else { expect_str(c, t)? };
        self.string(c, t)
    }
    /// generate_json_string (2.x: binary as UTF-8, other encodings converted).
    fn string(&mut self, c: &mut Ctx, v: Value) -> Result<(), Esc> {
        let v = if self.v2 { self.ensure_valid_v2(c, v)? } else { self.ensure_valid(c, v, false, false)? };
        self.raw_string(c, v)
    }
    fn ensure_valid_v2(&mut self, c: &mut Ctx, v: Value) -> RV {
        let enc = c.str_enc(v);
        if enc == ENC_UTF8 || enc == ENC_ASCII { return Ok(v); }
        if enc == ENC_BINARY {
            let u = c.str_copy_enc(v, ENC_UTF8);
            match coderange(c, u) {
                CR_7BIT => return Ok(u),
                CR_VALID => {
                    rb_warn(c, "JSON.generate: UTF-8 string passed as BINARY, this will raise an encoding error in json 3.0")?;
                    return Ok(u);
                }
                _ => {}
            }
        }
        encode_utf8(c, v)
    }
    fn ensure_valid(&mut self, c: &mut Ctx, v: Value, as_json_called: bool, is_key: bool) -> RV {
        if valid_json_string(c, v) { return Ok(v); }
        if !as_json_called && self.s.strict && self.s.as_json.truthy() {
            let r = self.call_as_json(c, v, false)?;
            if r != v {
                if is_string(r) {
                    if !valid_json_string(c, r) { return Err(gen_error(c, v, b"source sequence is illegal/malformed utf-8")); }
                } else if is_key {
                    let m = format!("{} not allowed as object key in JSON", class_path(class_of(r)));
                    return Err(gen_error(c, r, m.as_bytes()));
                }
                return Ok(r);
            }
        }
        let r = encode_utf8(c, v)?;
        let r = check_string(c, r)?;
        if !valid_json_string(c, r) { return Err(gen_error(c, r, b"source sequence is illegal/malformed utf-8")); }
        Ok(r)
    }
    fn raw_string(&mut self, c: &mut Ctx, v: Value) -> Result<(), Esc> {
        if coderange(c, v) == CR_BROKEN { return Err(gen_error(c, v, b"source sequence is illegal/malformed utf-8")); }
        let (ao, ss) = (self.s.ascii_only, self.s.script_safe);
        if self.out.io == NIL {
            self.out.b.push(b'"');
            let b = c.str_bytes(v);
            escape_into(&mut self.out.b, b, ao, ss);
            self.out.b.push(b'"');
            return Ok(());
        }
        // an IO target sees the buffer flushed where the C escaper grows it
        let ops = escape_ops(c.str_bytes(v), ao, ss, self.v2);
        self.out.ch(c, b'"')?;
        for op in ops {
            match op {
                Op::Put(x) => self.out.put(c, &x)?,
                Op::Reserve(n) => self.out.reserve(c, n)?,
                Op::Add(x) => self.out.b.extend_from_slice(&x),
            }
        }
        self.out.ch(c, b'"')
    }
    fn float(&mut self, c: &mut Ctx, v: Value) -> Result<(), Esc> {
        let f = float_val(v);
        if !f.is_finite() {
            if !self.s.allow_nan {
                if self.s.strict && self.s.as_json.truthy() {
                    let r = self.call_as_json(c, v, false)?;
                    if r != v {
                        self.increase_depth(c)?;
                        self.value(c, r, true)?;
                        self.depth -= 1;
                        return Ok(());
                    }
                }
                let t = to_s(c, v)?;
                let mut m = c.str_bytes(t).to_vec();
                m.extend_from_slice(b" not allowed in JSON");
                return Err(gen_error(c, v, &m));
            }
            let t = to_s(c, v)?;
            return self.out.str(c, t);
        }
        if self.s.rfc8785 { return self.rfc_number(c, v); }
        let mut t = Vec::with_capacity(32);
        dtoa(f, &mut t, self.v2);
        if self.out.io != NIL { self.out.reserve(c, if self.v2 { 28 } else { 32 })?; }
        self.out.b.extend_from_slice(&t);
        Ok(())
    }
    fn rfc_number(&mut self, c: &mut Ctx, v: Value) -> Result<(), Esc> {
        let p = P_RFC_NUMBER.get(c);
        let t = call_proc(c, p, &[v], std::ptr::null())?;
        let t = check_string(c, t)?;
        self.out.str(c, t)
    }
    fn array(&mut self, c: &mut Ctx, v: Value) -> Result<(), Esc> {
        self.increase_depth(c)?;
        if c.ary_len(v) == 0 { self.out.put(c, b"[]")?; self.depth -= 1; return Ok(()); }
        self.out.ch(c, b'[')?;
        let s = self.s;
        if s.array_nl != NIL { self.out.str(c, s.array_nl)?; }
        let mut i = 0;
        while i < c.ary_len(v) {
            if i > 0 { self.out.ch(c, b',')?; if self.s.array_nl != NIL { let n = self.s.array_nl; self.out.str(c, n)?; } }
            if self.s.indent != NIL { let (ind, d) = (self.s.indent, self.depth); self.out.repeat(c, ind, d)?; }
            let x = c.ary_get(v, i);
            self.value(c, x, true)?;
            i += 1;
        }
        self.depth -= 1;
        if self.s.array_nl != NIL {
            let n = self.s.array_nl;
            self.out.str(c, n)?;
            if self.s.indent != NIL { let (ind, d) = (self.s.indent, self.depth); self.out.repeat(c, ind, d)?; }
        }
        self.out.ch(c, b']')
    }
    fn object(&mut self, c: &mut Ctx, v: Value) -> Result<(), Esc> {
        let mut h = v;
        if !self.v2 {
            if self.s.rfc8785 {
                let p = P_RFC_SORT.get(c);
                let aj = if self.s.strict { self.s.as_json } else { NIL };
                h = call_proc(c, p, &[h, aj], std::ptr::null())?;
                if !is_hash(h) { return type_err(c, format!("wrong argument type {} (expected Hash)", class_name_for_err(h))); }
            } else if self.s.sort_keys.truthy() {
                let p = self.s.sort_keys;
                h = call_proc(c, p, &[h], std::ptr::null())?;
                if !is_hash(h) { return type_err(c, format!("wrong argument type {} (expected Hash)", class_name_for_err(h))); }
            }
        }
        self.increase_depth(c)?;
        if c.hash_len(h) == 0 { self.out.put(c, b"{}")?; self.depth -= 1; return Ok(()); }
        self.out.ch(c, b'{')?;
        let mut first = true;
        let mut first_type = 0u8;
        let mut mixed = false;
        each_pair(c, h, |c, k, x| self.pair(c, h, k, x, &mut first, &mut first_type, &mut mixed))?;
        self.depth -= 1;
        if self.s.object_nl != NIL {
            let n = self.s.object_nl;
            self.out.str(c, n)?;
            if self.s.indent != NIL { let (ind, d) = (self.s.indent, self.depth); self.out.repeat(c, ind, d)?; }
        }
        self.out.ch(c, b'}')
    }
    #[allow(clippy::too_many_arguments)]
    fn pair(&mut self, c: &mut Ctx, h: Value, key: Value, x: Value, first: &mut bool, first_type: &mut u8, mixed: &mut bool) -> Result<(), Esc> {
        // key kinds as rb_type sees them: 1 String, 2 Symbol, 3 anything else
        let kind = |k: Value| if is_string(k) { 1u8 } else if k.is_sym() { 2 } else { 3 };
        let mut kt = kind(key);
        if *first { *first = false; *first_type = kt; } else { self.out.ch(c, b',')?; }
        if self.s.object_nl != NIL { let n = self.s.object_nl; self.out.str(c, n)?; }
        if self.s.indent != NIL { let (ind, d) = (self.s.indent, self.depth); self.out.repeat(c, ind, d)?; }
        let mut key = key;
        let mut as_json_called = false;
        let key_s = loop {
            match kt {
                1 => {
                    if !self.v2 && *first_type != 1 { self.mixed_keys(c, h, mixed)?; }
                    break if real_class_of(key) == c.vm().c_string { key } else { self.string_subclass_key(c, key)? };
                }
                2 => {
                    if !self.v2 && *first_type != 2 { self.mixed_keys(c, h, mixed)?; }
                    break c.str_new_enc(crate::symbol::sym_bytes(key.as_sym()), sym_enc(key));
                }
                _ => {
                    if !self.v2 && self.s.strict {
                        if self.s.as_json.truthy() && !as_json_called {
                            key = self.call_as_json(c, key, true)?;
                            kt = kind(key);
                            as_json_called = true;
                            continue;
                        }
                        let m = format!("{} not allowed as object key in JSON", class_path(class_of(key)));
                        return Err(gen_error(c, key, m.as_bytes()));
                    }
                    let t = funcall(c, key, intern(b"to_s"), &[], std::ptr::null())?;
                    if !is_string(t) { return type_err(c, format!("can't convert {} to String ({}#to_s gives {})", class_path(real_class_of(key)), class_path(real_class_of(key)), class_path(real_class_of(t)))); }
                    break t;
                }
            }
        };
        if self.v2 {
            if real_class_of(key_s) == c.vm().c_string { self.string(c, key_s)?; } else { self.value(c, key_s, true)?; }
        } else {
            let ks = self.ensure_valid(c, key_s, as_json_called, true)?;
            if is_string(ks) && real_class_of(ks) == c.vm().c_string { self.raw_string(c, ks)?; } else { self.value(c, ks, true)?; }
        }
        if self.s.space_before != NIL { let n = self.s.space_before; self.out.str(c, n)?; }
        self.out.ch(c, b':')?;
        if self.s.space != NIL { let n = self.s.space; self.out.str(c, n)?; }
        self.value(c, x, true)
    }
    fn string_subclass_key(&mut self, c: &mut Ctx, key: Value) -> RV {
        let t = funcall(c, key, intern(b"to_s"), &[], std::ptr::null())?;
        if !is_string(t) {
            let n = class_path(real_class_of(key));
            return type_err(c, format!("can't convert {} to String ({}#to_s gives {})", n, n, class_path(real_class_of(t))));
        }
        Ok(t)
    }
    fn mixed_keys(&mut self, c: &mut Ctx, h: Value, mixed: &mut bool) -> Result<(), Esc> {
        if *mixed { return Ok(()); }
        *mixed = true;
        if self.s.allow_duplicate_key != Some(true) {
            let m = M_JSON.get(c);
            funcall(c, m, intern(b"on_mixed_keys_hash"), &[h], std::ptr::null())?;
        }
        Ok(())
    }
    /// generate_json (fallback) / generate_json_no_fallback.
    fn value(&mut self, c: &mut Ctx, ob: Value, fallback: bool) -> Result<(), Esc> {
        let mut ob = ob;
        let mut as_json_called = false;
        loop {
            if ob == NIL { return self.out.put(c, b"null"); }
            if ob == FALSE { return self.out.put(c, b"false"); }
            if ob == TRUE { return self.out.put(c, b"true"); }
            if ob.is_fixnum() {
                if self.s.rfc8785 && !self.v2 { return self.rfc_number(c, ob); }
                let t = ob.as_fix().to_string();
                if self.out.io != NIL { self.out.reserve(c, 20)?; }
                self.out.b.extend_from_slice(t.as_bytes());
                return Ok(());
            }
            if ob.is_flonum() { return self.float(c, ob); }
            if ob.is_sym() {
                if self.s.strict {
                    let s = c.str_new_enc(crate::symbol::sym_bytes(ob.as_sym()), sym_enc(ob));
                    if self.v2 { return self.string(c, s); }
                    ob = s;
                    continue;
                }
                return self.fallback(c, ob);
            }
            if !ob.is_heap() { if self.general(c, &mut ob, &mut as_json_called)? { continue; } return Ok(()); }
            let klass = real_class_of(ob);
            let exact = |k: Value| !fallback || klass == k;
            let vmr = c.vm();
            let (ch, ca, cs, cf) = (vmr.c_hash, vmr.c_array, vmr.c_string, vmr.c_float);
            if is_bignum(ob) {
                if self.s.rfc8785 && !self.v2 { return self.rfc_number(c, ob); }
                let t = funcall(c, ob, intern(b"to_s"), &[], std::ptr::null())?;
                let t = if self.v2 { t } else { expect_str(c, t)? };
                return self.out.str(c, t);
            }
            if is_hash(ob) && exact(ch) { return self.object(c, ob); }
            if is_array(ob) && exact(ca) { return self.array(c, ob); }
            if is_string(ob) && exact(cs) {
                if self.v2 { return self.string(c, ob); }
                if valid_json_string(c, ob) { return self.raw_string(c, ob); }
                if as_json_called { return Err(gen_error(c, ob, b"source sequence is illegal/malformed utf-8")); }
                ob = self.ensure_valid(c, ob, false, false)?;
                as_json_called = true;
                continue;
            }
            if is_heap_float(ob) && exact(cf) { return self.float(c, ob); }
            if klass == C_FRAGMENT.get(c) && C_FRAGMENT.get(c) != NIL {
                let f = ivar_get(ob, intern(b"json"));
                let f = check_string(c, f)?;
                return self.out.str(c, f);
            }
            if self.general(c, &mut ob, &mut as_json_called)? { continue; }
            return Ok(());
        }
    }
    /// The `general:` label: strict mode's as_json (true: generate the result), else to_json / to_s.
    fn general(&mut self, c: &mut Ctx, ob: &mut Value, as_json_called: &mut bool) -> Result<bool, Esc> {
        if self.s.strict {
            if self.s.as_json.truthy() && !*as_json_called {
                *ob = self.call_as_json(c, *ob, false)?;
                *as_json_called = true;
                return Ok(true);
            }
            let m = format!("{} not allowed in JSON", class_path(class_of(*ob)));
            return Err(gen_error(c, *ob, m.as_bytes()));
        }
        self.fallback(c, *ob)?;
        Ok(false)
    }
}
fn sym_enc(_v: Value) -> u8 { ENC_UTF8 }

fn validate_rfc8785(c: &mut Ctx, s: &Cfg) -> Result<(), Esc> {
    if !s.rfc8785 { return Ok(()); }
    let opt = if s.indent != NIL { "indent" } else if s.space != NIL { "space" } else if s.space_before != NIL { "space_before" }
        else if s.object_nl != NIL { "object_nl" } else if s.array_nl != NIL { "array_nl" } else if s.ascii_only { "ascii_only" }
        else if s.script_safe { "script_safe" } else if s.allow_nan { "allow_nan" } else { return Ok(()) };
    arg_err(c, format!("{} cannot be used with rfc8785", opt))
}

/// What to generate at the top: a value as `generate` does, or one known kind (2.x to_json).
#[derive(Clone, Copy, PartialEq)]
enum Top { Value, NoFallback, Object, Array, Integer, Float, String }

fn run(c: &mut Ctx, g: &mut G, ob: Value, top: Top) -> Result<(), Esc> {
    if !g.v2 { let s = g.s; validate_rfc8785(c, &s)?; }
    match top {
        Top::Value => g.value(c, ob, true),
        Top::NoFallback => g.value(c, ob, false),
        Top::Object => g.object(c, ob),
        Top::Array => g.array(c, ob),
        Top::Integer => g.value(c, ob, false),
        Top::Float => g.float(c, ob),
        Top::String => g.string(c, ob),
    }
}
fn finish(c: &mut Ctx, g: &mut G) -> RV {
    if g.out.io != NIL {
        g.out.flush(c)?;
        let io = g.out.io;
        funcall(c, io, intern(b"flush"), &[], std::ptr::null())?;
        return Ok(io);
    }
    Ok(c.str_from_vec(std::mem::take(&mut g.out.b)))
}

/// cState_partial_generate: generate with State `st`.
fn partial_generate(c: &mut Ctx, st: Value, ob: Value, top: Top, io: Value) -> RV {
    let s = cfg_of(c, st);
    let two = v2(c);
    let mut g = G { out: Out::new(io, s.buffer_initial_length, two), s, vstate: if two { st } else { NIL }, depth: s.depth, v2: two };
    let r = run(c, &mut g, ob, top);
    if two { cfg_mut(c, g.vstate).depth = g.depth; }
    r?;
    finish(c, &mut g)
}
/// cState_m_generate: State.generate(ob, opts, io) with a configuration that is never a Ruby object.
fn m_generate(c: &mut Ctx, ob: Value, opts: Value, io: Value, top: Top) -> RV {
    let two = v2(c);
    let mut s = Cfg::new();
    configure(c, &mut s, opts)?;
    let mut g = G { out: Out::new(io, s.buffer_initial_length, two), s, vstate: NIL, depth: s.depth, v2: two };
    run(c, &mut g, ob, top)?;
    finish(c, &mut g)
}

// ---- configuration ----------------------------------------------------------------------------

fn sym_is(k: Value, name: &[u8]) -> bool { k.is_sym() && crate::symbol::sym_bytes(k.as_sym()) == name }
fn string_config(c: &mut Ctx, v: Value) -> RV {
    if !v.truthy() { return Ok(NIL); }
    if !is_string(v) { return type_err(c, format!("wrong argument type {} (expected String)", class_name_for_err(v))); }
    if c.str_len(v) == 0 { return Ok(NIL); }
    if is_frozen(v) { return Ok(v); }
    let n = c.str_copy(v);
    crate::heap::obj_set_flags(n, FL_FROZEN);
    Ok(n)
}
fn long_config(c: &mut Ctx, v: Value) -> Result<i64, Esc> {
    if !v.truthy() { return Ok(0); }
    if v.is_fixnum() { return Ok(v.as_fix()); }
    if v2(c) { return Ok(0); }
    crate::builtins::numeric_methods::num_to_long(c, v)
}
fn depth_config(c: &mut Ctx, v: Value) -> Result<i64, Esc> {
    if v2(c) { return long_config(c, v); }
    if !v.truthy() { return Ok(0); }
    let d = crate::builtins::numeric_methods::num_to_long(c, v)?;
    if d < 0 { return arg_err(c, format!("depth must be >= 0 (got {})", d)); }
    if d > i32::MAX as i64 { return arg_err(c, format!("depth is too large (got {})", d)); }
    Ok(d)
}
fn buffer_initial_length_set(c: &mut Ctx, s: &mut Cfg, v: Value) -> Result<(), Esc> {
    if !v.is_fixnum() { return type_err(c, format!("wrong argument type {} (expected Integer)", class_name_for_err(v))); }
    if v.as_fix() > 0 { s.buffer_initial_length = v.as_fix(); }
    Ok(())
}
fn as_json_config(c: &mut Ctx, v: Value) -> RV {
    if v2(c) {
        if is_proc(v) { return Ok(v); }
        if !respond_to(v, intern(b"to_proc"), true) { return type_err(c, format!("no implicit conversion of {} into Proc", class_name_for_err(v))); }
        let p = funcall(c, v, intern(b"to_proc"), &[], std::ptr::null())?;
        if !is_proc(p) { let n = class_path(real_class_of(v)); return type_err(c, format!("can't convert {} to Proc ({}#to_proc gives {})", n, n, class_path(real_class_of(p)))); }
        return Ok(p);
    }
    if is_proc(v) { return Ok(v); }
    let p = if respond_to(v, intern(b"to_proc"), true) { funcall(c, v, intern(b"to_proc"), &[], std::ptr::null())? } else { UNDEF };
    if !is_proc(p) { return type_err(c, "as_json must be a Proc"); }
    Ok(p)
}
fn normalize_sort_keys(c: &mut Ctx, v: Value) -> RV {
    if is_proc(v) { return Ok(v); }
    if v == TRUE { return Ok(P_SORT_KEYS.get(c)); }
    if v.truthy() { return type_err(c, "The `sort_keys` argument must be a boolean or a Proc"); }
    Ok(FALSE)
}
/// configure_state: Hash keys over the fields.
fn configure(c: &mut Ctx, s: &mut Cfg, opts: Value) -> Result<(), Esc> {
    if !opts.truthy() { return Ok(()); }
    if !is_hash(opts) { return type_err(c, format!("wrong argument type {} (expected Hash)", class_name_for_err(opts))); }
    if c.hash_len(opts) == 0 { return Ok(()); }
    let two = v2(c);
    let mut unknown: Vec<Value> = vec![];
    each_pair(c, opts, |c, k, v| {
        if sym_is(k, b"indent") { s.indent = string_config(c, v)?; }
        else if sym_is(k, b"space") { s.space = string_config(c, v)?; }
        else if sym_is(k, b"space_before") { s.space_before = string_config(c, v)?; }
        else if sym_is(k, b"object_nl") { s.object_nl = string_config(c, v)?; }
        else if sym_is(k, b"array_nl") { s.array_nl = string_config(c, v)?; }
        else if sym_is(k, b"max_nesting") { s.max_nesting = long_config(c, v)?; }
        else if sym_is(k, b"allow_nan") { s.allow_nan = v.truthy(); }
        else if sym_is(k, b"ascii_only") { s.ascii_only = v.truthy(); }
        else if sym_is(k, b"depth") { s.depth = depth_config(c, v)?; }
        else if sym_is(k, b"buffer_initial_length") { buffer_initial_length_set(c, s, v)?; }
        else if sym_is(k, b"script_safe") { s.script_safe = v.truthy(); }
        else if two && sym_is(k, b"escape_slash") { s.script_safe = v.truthy(); }
        else if sym_is(k, b"strict") { s.strict = v.truthy(); }
        else if !two && sym_is(k, b"allow_duplicate_key") { s.allow_duplicate_key = Some(v.truthy()); }
        else if !two && sym_is(k, b"rfc8785") { s.rfc8785 = v.truthy(); }
        else if sym_is(k, b"as_json") { s.as_json = if v.truthy() { as_json_config(c, v)? } else { FALSE }; }
        else if !two && sym_is(k, b"sort_keys") { s.sort_keys = normalize_sort_keys(c, v)?; }
        else if !two { unknown.push(k); }
        Ok(())
    })?;
    if !two { unknown_keywords(c, &unknown)?; validate_rfc8785(c, s)?; }
    Ok(())
}

// ---- State methods ----------------------------------------------------------------------------

fn frozen_check(c: &mut Ctx, slf: Value) -> Result<(), Esc> { if !v2(c) { check_frozen(c, slf)?; } Ok(()) }
fn empty_frozen(c: &mut Ctx) -> Value { let v = c.str_new(b""); crate::heap::obj_set_flags(v, FL_FROZEN); v }

macro_rules! str_attr {
    ($get:ident, $set:ident, $f:ident) => {
        fn $get(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let v = cfg_of(c, slf).$f; Ok(if v == NIL { empty_frozen(c) } else { v }) }
        fn $set(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { frozen_check(c, slf)?; let v = string_config(c, a[0])?; cfg_mut(c, slf).$f = v; Ok(NIL) }
    };
}
str_attr!(st_indent, st_set_indent, indent);
str_attr!(st_space, st_set_space, space);
str_attr!(st_space_before, st_set_space_before, space_before);
str_attr!(st_object_nl, st_set_object_nl, object_nl);
str_attr!(st_array_nl, st_set_array_nl, array_nl);
macro_rules! bool_attr {
    ($get:ident, $set:ident, $f:ident) => {
        fn $get(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(cfg_of(c, slf).$f)) }
        fn $set(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { frozen_check(c, slf)?; cfg_mut(c, slf).$f = a[0].truthy(); Ok(NIL) }
    };
}
bool_attr!(st_script_safe, st_set_script_safe, script_safe);
bool_attr!(st_strict, st_set_strict, strict);
bool_attr!(st_allow_nan, st_set_allow_nan, allow_nan);
bool_attr!(st_ascii_only, st_set_ascii_only, ascii_only);
fn st_as_json(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(cfg_of(c, slf).as_json) }
fn st_set_as_json(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { frozen_check(c, slf)?; let p = as_json_config(c, a[0])?; cfg_mut(c, slf).as_json = p; Ok(NIL) }
fn st_check_circular(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(cfg_of(c, slf).max_nesting != 0)) }
fn st_max_nesting(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(cfg_of(c, slf).max_nesting)) }
fn st_set_max_nesting(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { frozen_check(c, slf)?; let n = long_config(c, a[0])?; cfg_mut(c, slf).max_nesting = n; Ok(NIL) }
fn st_depth(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(cfg_of(c, slf).depth)) }
fn st_set_depth(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { frozen_check(c, slf)?; let n = depth_config(c, a[0])?; cfg_mut(c, slf).depth = n; Ok(NIL) }
fn st_buffer_initial_length(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(cfg_of(c, slf).buffer_initial_length)) }
fn st_set_buffer_initial_length(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    frozen_check(c, slf)?;
    let mut s = cfg_of(c, slf);
    buffer_initial_length_set(c, &mut s, a[0])?;
    cfg_mut(c, slf).buffer_initial_length = s.buffer_initial_length;
    Ok(NIL)
}
fn st_sort_keys(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(cfg_of(c, slf).sort_keys) }
fn st_set_sort_keys(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { frozen_check(c, slf)?; let p = normalize_sort_keys(c, a[0])?; cfg_mut(c, slf).sort_keys = p; Ok(p) }
fn st_rfc8785(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(cfg_of(c, slf).rfc8785)) }
fn st_set_rfc8785(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { frozen_check(c, slf)?; cfg_mut(c, slf).rfc8785 = a[0].truthy(); Ok(Value::bool(a[0].truthy())) }
fn st_allow_duplicate_key(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(match cfg_of(c, slf).allow_duplicate_key { Some(b) => Value::bool(b), None => FALSE }) }
fn st_initialize(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    rb_warn(c, "The json gem extension was loaded with the stdlib ruby code. You should upgrade rubygems with `gem update --system`")?;
    Ok(slf)
}
fn st_configure(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    frozen_check(c, slf)?;
    let mut s = cfg_of(c, slf);
    configure(c, &mut s, a[0])?;
    *cfg_mut(c, slf) = s;
    Ok(slf)
}
fn st_init_copy(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if slf == a[0] { return Ok(slf); }
    if !is_state(c, a[0]) { return type_err(c, format!("wrong argument type {} (expected JSON/Generator/State)", class_name_for_err(a[0]))); }
    let s = cfg_of(c, a[0]);
    *cfg_mut(c, slf) = s;
    Ok(slf)
}
fn st_s_from_state(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if is_kind_of(o, slf) { return Ok(o); }
    if is_hash(o) { return funcall(c, slf, intern(b"new"), &[o], std::ptr::null()); }
    let k = C_STATE.get(c);
    funcall(c, k, intern(b"new"), &[], std::ptr::null())
}
fn from_state(c: &mut Ctx, o: Value) -> RV { let k = C_STATE.get(c); st_s_from_state(c, k, &[o], std::ptr::null()) }
fn set_proc_once(c: &mut Ctx, g: &Global, p: Value, name: &str) -> RV {
    if !is_proc(p) { return type_err(c, format!("{} must be a Proc", name)); }
    if g.get(c) != NIL { return arg_err(c, format!("{} can only be set once", name)); }
    g.set(c, p);
    Ok(p)
}
fn st_s_default_sort_keys(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { set_proc_once(c, &P_SORT_KEYS, a[0], "sort_key_proc") }
fn st_s_rfc_number(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { set_proc_once(c, &P_RFC_NUMBER, a[0], "rfc8785_number_formatter_proc") }
fn st_s_rfc_sort(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { set_proc_once(c, &P_RFC_SORT, a[0], "rfc8785_sort_keys_proc") }
fn st_generate(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let io = a.get(1).copied().filter(|v| v.truthy()).unwrap_or(NIL);
    partial_generate(c, slf, a[0], Top::Value, io)
}
fn st_generate_no_fallback(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let io = a.get(1).copied().filter(|v| v.truthy()).unwrap_or(NIL);
    partial_generate(c, slf, a[0], Top::NoFallback, io)
}
fn st_s_generate(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let io = if a[2].truthy() { a[2] } else { NIL }; m_generate(c, a[0], a[1], io, Top::Value) }
fn st_s_generate_no_fallback(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let io = if a[2].truthy() { a[2] } else { NIL }; m_generate(c, a[0], a[1], io, Top::NoFallback) }

// ---- json 2.x GeneratorMethods ------------------------------------------------------------------

fn gm_state(c: &mut Ctx, a: &[Value]) -> RV { check_argc(c, a, 0, 1)?; from_state(c, a.first().copied().unwrap_or(NIL)) }
fn gm_hash(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let st = gm_state(c, a)?; partial_generate(c, st, slf, Top::Object, NIL) }
fn gm_array(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let st = gm_state(c, a)?; partial_generate(c, st, slf, Top::Array, NIL) }
fn gm_integer(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let st = gm_state(c, a)?; partial_generate(c, st, slf, Top::Integer, NIL) }
fn gm_float(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let st = gm_state(c, a)?; partial_generate(c, st, slf, Top::Float, NIL) }
fn gm_string(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let st = gm_state(c, a)?; partial_generate(c, st, slf, Top::String, NIL) }
fn gm_object(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let t = funcall(c, slf, intern(b"to_s"), &[], std::ptr::null())?;
    if a.len() > 1 { check_argc(c, a, 0, 1)?; }
    let t = check_string(c, t)?;
    let st = from_state(c, a.first().copied().unwrap_or(NIL))?;
    partial_generate(c, st, t, Top::String, NIL)
}
fn gm_lit(c: &mut Ctx, a: &[Value], s: &[u8]) -> RV { check_argc(c, a, 0, 1)?; Ok(c.str_new(s)) }
fn gm_true(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { gm_lit(c, a, b"true") }
fn gm_false(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { gm_lit(c, a, b"false") }
fn gm_nil(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { gm_lit(c, a, b"null") }
fn gm_raw_object(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let h = c.hash_new();
    let m = M_JSON.get(c);
    let id = funcall(c, m, intern(b"create_id"), &[], std::ptr::null())?;
    let n = c.str_new_enc(class_path(real_class_of(slf)).as_bytes(), ENC_ASCII);
    hash_aset(c, h, id, n)?;
    let f = c.str_new(b"C*");
    let ary = funcall(c, slf, intern(b"unpack"), &[f], std::ptr::null())?;
    let k = c.str_new(b"raw");
    hash_aset(c, h, k, ary)?;
    Ok(h)
}
fn gm_raw(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { let h = gm_raw_object(c, slf, &[], b)?; gm_hash(c, h, a, b) }
fn gm_string_included(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let ext = M_STRING_EXTEND.get(c);
    let r = funcall(c, a[0], intern(b"extend"), &[ext], std::ptr::null())?;
    let _ = slf;
    Ok(r)
}
fn gm_json_create(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if !is_hash(a[0]) { return type_err(c, format!("wrong argument type {} (expected Hash)", class_name_for_err(a[0]))); }
    let k = c.str_new(b"raw");
    let ary = hash_aref(c, a[0], k)?;
    let f = c.str_new(b"C*");
    funcall(c, ary, intern(b"pack"), &[f], std::ptr::null())
}

// ---- allocation and init ----------------------------------------------------------------------

pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let k = C_STATE.get(c);
    if k != NIL && crate::builtins::inherits(klass, k) { return Ok(Some(c.alloc(klass, Body::Data(Box::new(PState { s: Cfg::new() }))))); }
    Ok(None)
}
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if !is_state(c, v) { return Ok(None); }
    let k = obj_klass(v);
    let k = if c.class(k).is_singleton { real_class_of(v) } else { k };
    Ok(Some(c.alloc(k, Body::Data(Box::new(PState { s: Cfg::new() })))))
}

/// Init_generator, run by `require "json/ext/generator"`.
pub fn init(c: &mut Ctx) -> RV {
    let f = c.str_new(b"json/common");
    require_feature(c, f, false)?;
    let o = c.vm().c_object;
    let m = match const_get_at(o, intern(b"JSON")) { Some(m) => m, None => define_module("JSON") };
    M_JSON.set(c, m);
    json_version(c);
    let two = v2(c);
    if let Some(fr) = const_get_at(m, intern(b"Fragment")) { C_FRAGMENT.set(c, fr); }
    let ext = match const_get_at(m, intern(b"Ext")) { Some(e) => e, None => define_module_under(m, "Ext") };
    let gen = match const_get_at(ext, intern(b"Generator")) { Some(g) => g, None => define_module_under(ext, "Generator") };
    for (g, n) in [(&E_GENERATOR, "GeneratorError"), (&E_NESTING, "NestingError")] {
        match const_get_at(m, intern(n.as_bytes())) { Some(k) => g.set(c, k), None => return err(c, c.vm().c_name_error, format!("uninitialized constant JSON::{}", n)) }
    }
    let st = class_under(gen, "State", o);
    class_set_alloc(st, AllocKind::None);
    C_STATE.set(c, st);
    define_singleton_cfunc(st, "from_state", st_s_from_state, 1);
    if !two {
        define_singleton_cfunc(st, "default_sort_keys_proc=", st_s_default_sort_keys, 1);
        define_singleton_cfunc(st, "rfc8785_number_formatter_proc=", st_s_rfc_number, 1);
        define_singleton_cfunc(st, "rfc8785_sort_keys_proc=", st_s_rfc_sort, 1);
    }
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(st, n, f, a);
    d("initialize", st_initialize, -1);
    define_private_cfunc(st, "_configure", st_configure, 1);
    d("initialize_copy", st_init_copy, 1);
    d("indent", st_indent, 0); d("indent=", st_set_indent, 1);
    d("space", st_space, 0); d("space=", st_set_space, 1);
    d("space_before", st_space_before, 0); d("space_before=", st_set_space_before, 1);
    d("object_nl", st_object_nl, 0); d("object_nl=", st_set_object_nl, 1);
    d("array_nl", st_array_nl, 0); d("array_nl=", st_set_array_nl, 1);
    d("as_json", st_as_json, 0); d("as_json=", st_set_as_json, 1);
    d("max_nesting", st_max_nesting, 0); d("max_nesting=", st_set_max_nesting, 1);
    d("script_safe", st_script_safe, 0); d("script_safe?", st_script_safe, 0); d("script_safe=", st_set_script_safe, 1);
    if two { d("escape_slash", st_script_safe, 0); d("escape_slash?", st_script_safe, 0); d("escape_slash=", st_set_script_safe, 1); }
    d("strict", st_strict, 0); d("strict?", st_strict, 0); d("strict=", st_set_strict, 1);
    d("check_circular?", st_check_circular, 0);
    d("allow_nan?", st_allow_nan, 0); d("allow_nan=", st_set_allow_nan, 1);
    d("ascii_only?", st_ascii_only, 0); d("ascii_only=", st_set_ascii_only, 1);
    d("depth", st_depth, 0); d("depth=", st_set_depth, 1);
    d("buffer_initial_length", st_buffer_initial_length, 0); d("buffer_initial_length=", st_set_buffer_initial_length, 1);
    d("generate", st_generate, -1);
    if two {
        d("generate_new", st_generate, -1);
        define_singleton_cfunc(st, "generate", st_s_generate, 3);
    } else {
        d("_generate_no_fallback", st_generate_no_fallback, -1);
        d("sort_keys", st_sort_keys, 0); d("sort_keys=", st_set_sort_keys, 1);
        d("rfc8785?", st_rfc8785, 0); d("rfc8785=", st_set_rfc8785, 1);
        define_private_cfunc(st, "allow_duplicate_key?", st_allow_duplicate_key, 0);
        define_singleton_cfunc(st, "generate", st_s_generate, 3);
        define_singleton_cfunc(st, "_generate_no_fallback", st_s_generate_no_fallback, 3);
    }
    if two {
        let gm = define_module_under(gen, "GeneratorMethods");
        let mm = |n: &str, f: CFunc| { let x = define_module_under(gm, n); define_cfunc(x, "to_json", f, -1); x };
        mm("Object", gm_object); mm("Hash", gm_hash); mm("Array", gm_array); mm("Integer", gm_integer); mm("Float", gm_float);
        let ms = mm("String", gm_string);
        define_singleton_cfunc(ms, "included", gm_string_included, 1);
        define_cfunc(ms, "to_json_raw", gm_raw, -1);
        define_cfunc(ms, "to_json_raw_object", gm_raw_object, 0);
        let se = define_module_under(ms, "Extend");
        define_cfunc(se, "json_create", gm_json_create, 1);
        M_STRING_EXTEND.set(c, se);
        mm("TrueClass", gm_true); mm("FalseClass", gm_false); mm("NilClass", gm_nil);
    }
    let f = c.str_new(b"json/ext/generator/state");
    require_feature(c, f, false)?;
    let _ = s::mesg;
    Ok(NIL)
}
