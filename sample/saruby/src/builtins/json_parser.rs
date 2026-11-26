// json/ext/parser: the json gem's ext/json/ext/parser/parser.c, natively (JSON::Ext::ParserConfig,
// JSON::Ext::Parser.parse, JSON::ResumableParser). One state machine serves the gem's 3.x parser
// (iterative, resumable) and, behind `v2`, the 2.x one's messages, positions and string rules.
// The values under construction live in a Ruby Array (rooted); frames hold no Values.
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::bignum::BigInt;
use crate::builtins::kernel::*;
use crate::builtins::marshal::each_pair;
use crate::hash::*;
use crate::heap::{obj_set_flags, Global};
use crate::string::*;
use crate::symbol::{intern, s};
use crate::array::*;
use crate::value::*;
use crate::vm::*;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

static M_JSON: Global = Global::new();
static E_PARSER: Global = Global::new();
static E_NESTING: Global = Global::new();
static C_CONFIG: Global = Global::new();
static C_RESUMABLE: Global = Global::new();
static V_NAN: Global = Global::new();
static V_INF: Global = Global::new();
static V_NINF: Global = Global::new();
/// JSON::VERSION as major*10000 + minor*100 + patch (0: not read yet).
static VER: AtomicU32 = AtomicU32::new(0);

/// The loaded json gem's version (from JSON::VERSION); 3.x rules when unknown.
pub fn json_version(c: &Ctx) -> u32 {
    let v = VER.load(Relaxed);
    if v != 0 { return v; }
    let m = match const_get_at(c, c.vm().c_object.get(), intern(b"JSON")) { Some(m) => m, None => return 30000 };
    let n = match const_get_at(c, m, intern(b"VERSION")) {
        Some(s) if is_string(s) => {
            let t = String::from_utf8_lossy(c.str_bytes(s)).into_owned();
            let p: Vec<u32> = t.split('.').map(|x| x.parse().unwrap_or(0)).collect();
            p.first().copied().unwrap_or(3) * 10000 + p.get(1).copied().unwrap_or(0) * 100 + p.get(2).copied().unwrap_or(0)
        }
        _ => 30000,
    };
    VER.store(n.max(1), Relaxed);
    n.max(1)
}
fn v2(c: &Ctx) -> bool { json_version(c) < 30000 }

// ---- configuration ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Dup { Deprecated, Ignore, Raise }

#[derive(Clone, Copy)]
struct Config {
    on_load: Value, decimal_class: Value, decimal_mid: Sym, max_nesting: i64,
    allow_comments: bool, dup: Dup, allow_nan: bool, allow_trailing_comma: bool, allow_control_characters: bool,
    allow_invalid_escape: bool, symbolize_names: bool, freeze: bool,
}
impl Config {
    fn new(c: &Ctx) -> Config {
        Config { on_load: NIL, decimal_class: NIL, decimal_mid: intern(b"new"), max_nesting: 100, allow_comments: false,
                 dup: if v2(c) { Dup::Deprecated } else { Dup::Raise }, allow_nan: false, allow_trailing_comma: false,
                 allow_control_characters: false, allow_invalid_escape: false, symbolize_names: false, freeze: false }
    }
}

pub struct PConfig { cfg: Config, set: bool }
impl RData for PConfig {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.cfg.on_load); m.visit(&mut self.cfg.decimal_class); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

fn sym_is(k: Value, name: &[u8]) -> bool { k.is_sym() && crate::symbol::sym_bytes(k.as_sym()) == name }

/// parser_config_init: `opts` is nil or a Hash.
fn config_init(c: &mut Ctx, opts: Value) -> Result<Config, Esc> {
    let mut cfg = Config::new(c);
    if opts == NIL { return Ok(cfg); }
    if !is_hash(opts) { return type_err(c, format!("wrong argument type {} (expected Hash)", class_name_for_err(c, opts))); }
    if c.hash_len(opts) == 0 { return Ok(cfg); }
    let three = !v2(c);
    let mut unknown: Vec<Value> = vec![];
    each_pair(c, opts, |c, k, v| {
        if sym_is(k, b"max_nesting") { cfg.max_nesting = if v.truthy() { if v.is_fixnum() { v.as_fix() as i32 as i64 } else { crate::builtins::numeric_methods::num_to_long(c, v)? as i32 as i64 } } else { 0 }; }
        else if sym_is(k, b"allow_nan") { cfg.allow_nan = v.truthy(); }
        else if sym_is(k, b"allow_trailing_comma") { cfg.allow_trailing_comma = v.truthy(); }
        else if three && sym_is(k, b"allow_comments") { cfg.allow_comments = v.truthy(); }
        else if three && sym_is(k, b"allow_control_characters") { cfg.allow_control_characters = v.truthy(); }
        else if three && sym_is(k, b"allow_invalid_escape") { cfg.allow_invalid_escape = v.truthy(); }
        else if sym_is(k, b"symbolize_names") { cfg.symbolize_names = v.truthy(); }
        else if sym_is(k, b"freeze") { cfg.freeze = v.truthy(); }
        else if sym_is(k, b"on_load") {
            let mut p = v;
            if three && v.truthy() && !is_proc(v) {
                p = if respond_to(c, v, intern(b"to_proc"), true) { funcall(c, v, intern(b"to_proc"), &[], std::ptr::null())? } else { UNDEF };
                if !is_proc(p) { return type_err(c, "on_load must be a Proc"); }
            }
            cfg.on_load = if p.truthy() { p } else { NIL };
        }
        else if sym_is(k, b"allow_duplicate_key") { cfg.dup = if v.truthy() { Dup::Ignore } else { Dup::Raise }; }
        else if sym_is(k, b"decimal_class") {
            if v.truthy() {
                if respond_to(c, v, intern(b"try_convert"), false) { cfg.decimal_class = v; cfg.decimal_mid = intern(b"try_convert"); }
                else if respond_to(c, v, intern(b"new"), false) { cfg.decimal_class = v; cfg.decimal_mid = intern(b"new"); }
                else if is_class_or_module(v) && !c.class(v).is_module {
                    let name = class_path(c, v);
                    match name.rfind(':') {
                        Some(i) => {
                            let path = c.str_new(name[..i - 1].as_bytes());
                            cfg.decimal_class = funcall(c, c.vm().c_object.get(), intern(b"const_get"), &[path], std::ptr::null())?;
                            cfg.decimal_mid = intern(name[i + 1..].as_bytes());
                        }
                        None => { cfg.decimal_class = c.vm().m_kernel.get(); cfg.decimal_mid = intern(name.as_bytes()); }
                    }
                }
            }
        }
        else if three { unknown.push(k); }
        Ok(())
    })?;
    unknown_keywords(c, &unknown)?;
    Ok(cfg)
}

/// raise_argument_error_on_unknown_keywords (json.h).
pub fn unknown_keywords(c: &mut Ctx, unknown: &[Value]) -> Result<(), Esc> {
    if unknown.is_empty() { return Ok(()); }
    let mut names = vec![];
    for &k in unknown { let s = crate::rt::srb_tostr(c, k)?; names.push(String::from_utf8_lossy(c.str_bytes(s)).into_owned()); }
    if names.len() == 1 { arg_err(c, format!("unknown keyword: {}", names[0])) } else { arg_err(c, format!("unknown keywords: {}", names.join(", "))) }
}

// ---- the machine ------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
enum FT { Root, Array, Object }
#[derive(Clone, Copy, PartialEq, Debug)]
enum Ph { Done, ArrayComma, ObjectComma, Value, ObjectKey, ObjectColon }
#[derive(Clone, Copy, Debug)]
struct Frame { t: FT, ph: Ph, vhead: usize, start: usize }
fn completed(f: &mut Frame) -> Ph { f.ph = match f.t { FT::Root => Ph::Done, FT::Array => Ph::ArrayComma, FT::Object => Ph::ObjectComma }; f.ph }

/// A parse failure: Eos only suspends a resumable parse.
enum PE { Eos, Rb(Esc) }
impl From<Esc> for PE { fn from(e: Esc) -> PE { PE::Rb(e) } }
type PR<T> = Result<T, PE>;

const CACHE_CAPA: usize = 63;
const CACHE_MAX_LEN: usize = 55;

/// Parser state that outlives one call (ResumableParser) or one parse.
struct St {
    cur: usize,
    stack: Value,          // Array of the values under construction
    frames: Vec<Frame>,
    nesting: i64,
    in_array: i64,
    cache: Vec<Value>,     // sorted frozen key Strings, rooted by `cache_root`
    cache_root: Value,
    resumable: bool,
    v2: bool,
}

struct M<'a> { src: &'a [u8], st: &'a mut St, cfg: &'a Config }

static SCAN: [bool; 256] = {
    let mut t = [false; 256];
    let mut i = 0;
    while i < 0x20 { t[i] = true; i += 1; }
    t[b'"' as usize] = true;
    t[b'\\' as usize] = true;
    t
};

fn hexval(b: u8) -> i32 { match b { b'0'..=b'9' => (b - b'0') as i32, b'a'..=b'f' => (b - b'a' + 10) as i32, b'A'..=b'F' => (b - b'A' + 10) as i32, _ => -1 } }
fn utf32_to_utf8(ch: u32, out: &mut Vec<u8>) {
    if ch <= 0x7f { out.push(ch as u8); }
    else if ch <= 0x7ff { out.push(((ch >> 6) | 0xc0) as u8); out.push(((ch & 0x3f) | 0x80) as u8); }
    else if ch <= 0xffff { out.push(((ch >> 12) | 0xe0) as u8); out.push((((ch >> 6) & 0x3f) | 0x80) as u8); out.push(((ch & 0x3f) | 0x80) as u8); }
    else if ch <= 0x1fffff { out.push(((ch >> 18) | 0xf0) as u8); out.push((((ch >> 12) & 0x3f) | 0x80) as u8); out.push((((ch >> 6) & 0x3f) | 0x80) as u8); out.push(((ch & 0x3f) | 0x80) as u8); }
    else { out.push(b'?'); }
}

/// The text after `at` quoted for a message: build_parse_error_message's fragment ("EOF" at the end).
fn fragment(src: &[u8], at: usize, v2: bool) -> Vec<u8> {
    if at >= src.len() { return b"EOF".to_vec(); }
    let byte = |i: usize| src.get(i).copied().unwrap_or(0);
    let mut len = 0;
    while len < 32 { let ch = byte(at + len); if ch == 0 || ch == b'\n' || ch == b' ' || ch == b'\t' || ch == b'\r' { break; } len += 1; }
    if len == 0 {
        // the C string from the cursor: up to the first NUL
        let e = src[at..].iter().position(|&b| b == 0).map(|p| at + p).unwrap_or(src.len());
        return src[at..e].to_vec();
    }
    if v2 {
        // buffer = '\'' + bytes; trims the last (multibyte) character as the C code does
        let mut buf = vec![b'\''];
        buf.extend_from_slice(&src[at..at + len]);
        while buf[len] >= 0x80 && buf[len] < 0xc0 { len -= 1; }
        if buf[len] >= 0xc0 { len -= 1; }
        buf.truncate(len + 1);
        buf.push(b'\'');
        return buf;
    }
    if len == 32 { while len > 0 && byte(at + len) >= 0x80 && byte(at + len) < 0xc0 { len -= 1; } }
    let mut buf = vec![b'\''];
    buf.extend_from_slice(&src[at..at + len]);
    buf.push(b'\'');
    buf
}
fn fmt_msg(fmt: &str, frag: &[u8]) -> Vec<u8> {
    match fmt.find("%s") {
        Some(i) => { let mut m = fmt.as_bytes()[..i].to_vec(); m.extend_from_slice(frag); m.extend_from_slice(&fmt.as_bytes()[i + 2..]); m }
        None => fmt.as_bytes().to_vec(),
    }
}
fn position(src: &[u8], at: usize, v2: bool) -> (i64, i64) {
    let at = at.min(src.len());
    if v2 {
        // 2.x's loops: the newline that ends the scan for the column is not counted as a line
        let byte = |i: i64| if (i as usize) < src.len() { src[i as usize] } else { 0 };
        let (mut cur, mut col, mut line) = (at as i64, 0i64, 1i64);
        while cur >= 0 { let b = byte(cur); cur -= 1; if b == b'\n' { break; } col += 1; }
        while cur >= 0 { let b = byte(cur); cur -= 1; if b == b'\n' { line += 1; } }
        return (line, col);
    }
    let ls = src[..at].iter().rposition(|&b| b == b'\n').map(|p| p + 1).unwrap_or(0);
    let line = 1 + src[..ls].iter().filter(|&&b| b == b'\n').count() as i64;
    (line, enc_strlen(&src[ls..at], ENC_UTF8) as i64 + 1)
}

fn new_exc(c: &mut Ctx, klass: Value, msg: &[u8]) -> Value {
    let e = new_object(c, klass);
    let m = c.str_new(msg);
    ivar_set_raw(c, e, s::mesg, m);
    e
}

impl<'a> M<'a> {
    #[inline(always)] fn eos(&self) -> bool { self.st.cur >= self.src.len() }
    #[inline(always)] fn peek(&self) -> u8 { if self.st.cur < self.src.len() { self.src[self.st.cur] } else { 0 } }
    #[inline(always)] fn byte(&self, i: usize) -> u8 { self.src.get(i).copied().unwrap_or(0) }

    /// json_path_new: the path of the innermost value at the time of the error.
    fn json_path(&self, c: &mut Ctx, dup_key: Value) -> Value {
        let path = c.ary_new();
        let head = self.st.frames.len();
        let vlen = c.ary_len(self.st.stack);
        for d in 1..head {
            let f = self.st.frames[d];
            let inner = d == head - 1;
            let child = if inner { vlen } else { self.st.frames[d + 1].vhead };
            let count = child as i64 - f.vhead as i64;
            if f.t == FT::Array {
                let n = if f.ph == Ph::ArrayComma { count - 1 } else { count };
                c.ary_push(path, Value::fix(n));
            } else if inner && dup_key != UNDEF {
                c.ary_push(path, dup_key);
            } else if count & 1 == 1 {
                let k = c.ary_get(self.st.stack, child - 1);
                c.ary_push(path, k);
            } else if f.ph == Ph::ObjectComma && count >= 2 {
                let k = c.ary_get(self.st.stack, child - 2);
                c.ary_push(path, k);
            } else { break; }
        }
        path
    }
    /// A ParserError for `msg` (already formatted) at `at`.
    fn error_msg(&mut self, c: &mut Ctx, mut msg: Vec<u8>, at: usize, dup_key: Value) -> PE {
        let k = E_PARSER.get(c);
        if self.st.resumable {
            let e = new_exc(c, k, &msg);
            ivar_set_raw(c, e, intern(b"@line"), Value::fix(0));
            ivar_set_raw(c, e, intern(b"@column"), Value::fix(0));
            let p = self.json_path(c, dup_key);
            ivar_set_raw(c, e, intern(b"@json_path"), p);
            return PE::Rb(raise_exc(c, e));
        }
        let (line, col) = position(self.src, at, self.st.v2);
        msg.extend_from_slice(format!(" at line {} column {}", line, col).as_bytes());
        let e = new_exc(c, k, &msg);
        ivar_set_raw(c, e, intern(b"@line"), Value::fix(line));
        ivar_set_raw(c, e, intern(b"@column"), Value::fix(col));
        if !self.st.v2 { let p = self.json_path(c, dup_key); ivar_set_raw(c, e, intern(b"@json_path"), p); }
        PE::Rb(raise_exc(c, e))
    }
    /// raise_parse_error_at(fmt, at, eos)
    fn fail(&mut self, c: &mut Ctx, fmt: &str, at: usize, eos: bool) -> PE {
        self.st.cur = at;
        if self.st.resumable && eos { return PE::Eos; }
        let frag = fragment(self.src, at, self.st.v2);
        let msg = fmt_msg(fmt, &frag);
        self.error_msg(c, msg, at, UNDEF)
    }
    fn syntax(&mut self, c: &mut Ctx, fmt: &str) -> PE { let at = self.st.cur; self.fail(c, fmt, at, false) }

    fn push(&mut self, c: &mut Ctx, v: Value) -> PR<()> {
        let v = if self.cfg.on_load != NIL { call_proc(c, self.cfg.on_load, &[v], std::ptr::null())? } else { v };
        c.ary_push(self.st.stack, v);
        Ok(())
    }

    // -- whitespace and comments
    fn eat_comments(&mut self, c: &mut Ctx, resume: Option<usize>) -> PR<()> {
        if self.st.v2 {
            let cur = self.st.cur;
            if cur + 1 < self.src.len() {
                match self.src[cur + 1] {
                    b'/' => { self.st.cur = match self.src[cur..].iter().position(|&b| b == b'\n') { Some(p) => cur + p + 1, None => self.src.len() }; }
                    b'*' => {
                        self.st.cur += 2;
                        loop {
                            let from = self.st.cur;
                            match self.src[from..].iter().position(|&b| b == b'*') {
                                None => { let e = self.src.len(); return Err(self.fail(c, "unexpected end of input, expected closing '*/'", e, false)); }
                                Some(p) => {
                                    self.st.cur = from + p + 1;
                                    if self.st.cur < self.src.len() && self.src[self.st.cur] == b'/' { self.st.cur += 1; break; }
                                }
                            }
                        }
                    }
                    _ => return Err(self.syntax(c, "unexpected token %s")),
                }
                return Ok(());
            }
            return Err(self.syntax(c, "unexpected token %s"));
        }
        if !self.cfg.allow_comments { return Err(self.syntax(c, "unexpected token %s")); }
        let start = self.st.cur;
        let rewind = match resume { Some(r) if self.st.resumable => r, _ => start };
        self.st.cur += 1;
        match self.peek() {
            b'/' => {
                let from = self.st.cur;
                match self.src[from..].iter().position(|&b| b == b'\n') {
                    None => {
                        if self.st.resumable { return Err(self.fail(c, "unterminated comment, expected end of line", rewind, true)); }
                        self.st.cur = self.src.len();
                    }
                    Some(p) => self.st.cur = from + p + 1,
                }
            }
            b'*' => {
                self.st.cur += 1;
                loop {
                    let from = self.st.cur.min(self.src.len());
                    match self.src[from..].iter().position(|&b| b == b'*') {
                        None => return Err(self.fail(c, "unterminated comment, expected closing '*/'", rewind, true)),
                        Some(p) => {
                            self.st.cur = from + p + 1;
                            if self.peek() == b'/' { self.st.cur += 1; break; }
                        }
                    }
                }
            }
            _ => { let e = self.eos(); return Err(self.fail(c, "unexpected token %s", if e { rewind } else { start }, e)); }
        }
        Ok(())
    }
    #[inline(always)]
    fn ws(&mut self, c: &mut Ctx) -> PR<()> { self.ws_at(c, true, None) }
    #[inline(always)]
    fn ws_at(&mut self, c: &mut Ctx, comments: bool, resume: Option<usize>) -> PR<()> {
        loop {
            match self.peek() {
                b' ' | b'\n' | b'\t' | b'\r' => self.st.cur += 1,
                b'/' if comments => self.eat_comments(c, resume)?,
                _ => return Ok(()),
            }
        }
    }

    // -- strings
    #[inline(always)]
    fn scan(&mut self) -> bool {
        let n = self.src.len();
        let mut i = self.st.cur;
        while i < n { if SCAN[self.src[i] as usize] { self.st.cur = i; return true; } i += 1; }
        self.st.cur = i.min(n);
        false
    }
    fn build_string(&mut self, c: &mut Ctx, b: &[u8], intern_it: bool, symbolize: bool) -> Value {
        if symbolize { return Value::sym(intern(b)); }
        let v = c.str_new(b);
        if intern_it { obj_set_flags(v, FL_FROZEN); }
        v
    }
    fn fastpath(&mut self, c: &mut Ctx, a: usize, b: usize, is_name: bool) -> Value {
        let intern_it = is_name || self.cfg.freeze;
        let symbolize = is_name && self.cfg.symbolize_names;
        let len = b - a;
        if is_name && self.st.in_array > 0 && !symbolize && len > 0 && len <= CACHE_MAX_LEN && self.src[a].is_ascii_alphabetic() {
            let key = &self.src[a..b];
            let (mut lo, mut hi) = (0usize, self.st.cache.len());
            while lo < hi {
                let mid = (lo + hi) / 2;
                let e = c.str_bytes(self.st.cache[mid]);
                match e.len().cmp(&key.len()).then_with(|| e.cmp(key)) {
                    std::cmp::Ordering::Equal => return self.st.cache[mid],
                    std::cmp::Ordering::Less => lo = mid + 1,
                    std::cmp::Ordering::Greater => hi = mid,
                }
            }
            let src = self.src;
            let v = self.build_string(c, &src[a..b], true, false);
            if self.st.cache.len() < CACHE_CAPA {
                if self.st.cache_root == NIL { self.st.cache_root = c.ary_new(); }
                c.ary_push(self.st.cache_root, v);
                self.st.cache.insert(lo, v);
            }
            return v;
        }
        let src = self.src;
        self.build_string(c, &src[a..b], intern_it, symbolize)
    }
    fn unicode(&mut self, c: &mut Ctx, sp: usize, spe: usize) -> PR<u32> {
        if sp + 4 > spe { return Err(self.fail(c, "incomplete unicode character escape sequence at %s", sp - 2, false)); }
        let mut r = 0u32;
        for i in 0..4 {
            let d = hexval(self.src[sp + i]);
            if d < 0 { return Err(self.fail(c, "incomplete unicode character escape sequence at %s", sp - 2, false)); }
            r = (r << 4) | d as u32;
        }
        Ok(r)
    }
    fn finish_string(&mut self, c: &mut Ctx, buf: Vec<u8>, is_name: bool) -> Value {
        let symbolize = is_name && self.cfg.symbolize_names;
        if symbolize { return Value::sym(intern(&buf)); }
        let v = c.str_from_vec(buf);
        if is_name || self.cfg.freeze { obj_set_flags(v, FL_FROZEN); }
        v
    }
    /// json_string_unescape (3.x).
    fn unescape(&mut self, c: &mut Ctx, a: usize, e: usize, is_name: bool) -> PR<Value> {
        let src = self.src;
        let mut buf = Vec::with_capacity(e - a);
        let (mut p, mut pe) = (a, a);
        while pe < e {
            match src[pe..e].iter().position(|&b| b == b'\\') { Some(i) => pe += i, None => break }
            if pe > p { buf.extend_from_slice(&src[p..pe]); }
            pe += 1;
            let ch = src[pe];
            match ch {
                b'"' | b'/' => p = pe,
                b'\\' | b'n' | b'r' | b't' | b'b' | b'f' => {
                    buf.push(match ch { b'n' => b'\n', b'r' => b'\r', b't' => b'\t', b'b' => 8, b'f' => 12, x => x });
                    pe += 1; p = pe;
                }
                b'u' => {
                    pe += 1;
                    let mut cp = self.unicode(c, pe, e)?;
                    pe += 3;
                    if cp & 0xfc00 == 0xd800 {
                        pe += 1;
                        if pe + 6 <= e && &src[pe..pe + 2] == b"\\u" {
                            let sur = self.unicode(c, pe + 2, e)?;
                            if sur & 0xfc00 != 0xdc00 { return Err(self.fail(c, "invalid surrogate pair at %s", p, false)); }
                            cp = ((cp & 0x3f) << 10) | ((((cp >> 6) & 0xf) + 1) << 16) | (sur & 0x3ff);
                            pe += 5;
                        } else { return Err(self.fail(c, "incomplete surrogate pair at %s", p, false)); }
                    } else if cp & 0xfc00 == 0xdc00 { return Err(self.fail(c, "unpaired trailing surrogate at %s", p, false)); }
                    utf32_to_utf8(cp, &mut buf);
                    pe += 1; p = pe;
                }
                _ => {
                    if ch < 0x20 && !self.cfg.allow_control_characters {
                        let m = if ch == b'\n' { "Invalid unescaped newline character (\\n) in string: %s" } else if ch == 0 { "Invalid unescaped NULL character (\\0) in string: %s" } else { "invalid ASCII control character in string: %s" };
                        return Err(self.fail(c, m, pe - 1, false));
                    }
                    if self.cfg.allow_invalid_escape { buf.push(ch); pe += 1; p = pe; }
                    else { return Err(self.fail(c, "invalid escape character in string: %s", pe - 1, false)); }
                }
            }
        }
        if e > p { buf.extend_from_slice(&src[p..e]); }
        Ok(self.finish_string(c, buf, is_name))
    }
    /// json_string_unescape (2.x): unknown escapes keep the character, a lone high surrogate is '?'.
    fn unescape_v2(&mut self, c: &mut Ctx, a: usize, e: usize, is_name: bool) -> PR<Value> {
        let src = self.src;
        let mut buf = Vec::with_capacity(e - a);
        let (mut p, mut pe) = (a, a);
        while pe < e {
            match src[pe..e].iter().position(|&b| b == b'\\') { Some(i) => pe += i, None => break }
            if pe > p { buf.extend_from_slice(&src[p..pe]); }
            pe += 1;
            let ch = src[pe];
            match ch {
                b'n' | b'r' | b't' | b'"' | b'\\' | b'b' | b'f' => buf.push(match ch { b'n' => b'\n', b'r' => b'\r', b't' => b'\t', b'b' => 8, b'f' => 12, x => x }),
                b'u' => {
                    if pe + 5 > e { return Err(self.fail(c, "incomplete unicode character escape sequence at %s", p, false)); }
                    pe += 1;
                    let mut cp = self.unicode(c, pe, usize::MAX)?;
                    pe += 3;
                    let mut lone = false;
                    if cp & 0xfc00 == 0xd800 {
                        pe += 1;
                        if pe + 6 > e { return Err(self.fail(c, "incomplete surrogate pair at %s", p, false)); }
                        if src[pe] == b'\\' && src[pe + 1] == b'u' {
                            let sur = self.unicode(c, pe + 2, usize::MAX)?;
                            cp = ((cp & 0x3f) << 10) | ((((cp >> 6) & 0xf) + 1) << 16) | (sur & 0x3ff);
                            pe += 5;
                        } else { lone = true; }
                    }
                    if lone { buf.push(b'?'); } else { utf32_to_utf8(cp, &mut buf); }
                }
                _ => { p = pe; continue; }
            }
            pe += 1; p = pe;
        }
        if e > p { buf.extend_from_slice(&src[p..e]); }
        Ok(self.finish_string(c, buf, is_name))
    }
    /// A string at the cursor (on '"'); None when unterminated (3.x).
    fn string(&mut self, c: &mut Ctx, is_name: bool) -> PR<Option<Value>> {
        if self.st.v2 { return self.string_v2(c, is_name).map(Some); }
        self.st.cur += 1;
        let start = self.st.cur;
        if !self.scan() { return Ok(None); }
        if self.src[self.st.cur] == b'"' {
            let e = self.st.cur;
            let v = self.fastpath(c, start, e, is_name);
            self.st.cur += 1;
            return Ok(Some(v));
        }
        loop {
            match self.src[self.st.cur] {
                b'"' => {
                    let e = self.st.cur;
                    let v = self.unescape(c, start, e, is_name)?;
                    self.st.cur = e + 1;
                    return Ok(Some(v));
                }
                b'\\' => self.st.cur += 1,
                _ => if !self.cfg.allow_control_characters { return Err(self.syntax(c, "invalid ASCII control character in string: %s")); },
            }
            self.st.cur += 1;
            if !self.scan() { return Ok(None); }
        }
    }
    fn string_v2(&mut self, c: &mut Ctx, is_name: bool) -> PR<Value> {
        self.st.cur += 1;
        let start = self.st.cur;
        let mut escaped = false;
        while self.scan() {
            match self.src[self.st.cur] {
                b'"' => {
                    let e = self.st.cur;
                    let v = if escaped { self.unescape_v2(c, start, e, is_name)? } else { self.fastpath(c, start, e, is_name) };
                    self.st.cur = e + 1;
                    return Ok(v);
                }
                b'\\' => {
                    self.st.cur += 1;
                    escaped = true;
                    if self.byte(self.st.cur) < 0x20 { return Err(self.syntax(c, "invalid ASCII control character in string: %s")); }
                }
                _ => return Err(self.syntax(c, "invalid ASCII control character in string: %s")),
            }
            self.st.cur += 1;
        }
        Err(self.syntax(c, "unexpected end of input, expected closing \""))
    }

    // -- numbers
    fn digits(&mut self) -> usize {
        let s = self.st.cur;
        while self.peek().is_ascii_digit() { self.st.cur += 1; }
        self.st.cur - s
    }
    /// The Float of src[a..b]; warns as rb_cstr_to_dbl does when `cstr` and out of range.
    fn float_of(&mut self, c: &mut Ctx, a: usize, b: usize, cstr: bool) -> PR<Value> {
        let t = std::str::from_utf8(&self.src[a..b]).unwrap_or("0");
        let f: f64 = t.parse().unwrap_or(0.0);
        if cstr && c.vm().verbose.get() == TRUE {
            let mant_nonzero = self.src[a..b].iter().take_while(|&&x| x != b'e' && x != b'E').any(|&x| (b'1'..=b'9').contains(&x));
            if f.is_infinite() || (f == 0.0 && mant_nonzero) {
                let (w, ell) = if b - a > 20 { (20, "...") } else { (b - a, "") };
                let msg = format!("Float {}{} out of range", &t[..w], ell);
                let loc = backtrace_lines(c, 0).first().map(|l| l.split(":in ").next().unwrap_or("").to_string()).unwrap_or_default();
                warning_write(c, format!("{}: warning: {}\n", loc, msg).as_bytes())?;
            }
        }
        Ok(c.float_new(f))
    }
    fn integer_of(&mut self, c: &mut Ctx, a: usize, b: usize) -> Value {
        let t = std::str::from_utf8(&self.src[a..b]).unwrap_or("0");
        if b - a <= 18 { if let Ok(n) = t.parse::<i64>() { return c.int_from_i64(n); } }
        match BigInt::parse(t, 10) { Some(n) => c.big_to_value(n), None => Value::fix(0) }
    }
    fn decimal(&mut self, c: &mut Ctx, a: usize, b: usize) -> PR<Value> {
        let src = self.src;
        let text = c.str_new_enc(&src[a..b], ENC_BINARY);
        Ok(funcall(c, self.cfg.decimal_class, self.cfg.decimal_mid, &[text], std::ptr::null())?)
    }
    /// json_parse_number (3.x); None for an invalid number. The cursor is past the sign.
    fn number(&mut self, c: &mut Ctx, negative: bool, start: usize) -> PR<Option<Value>> {
        let first = self.byte(self.st.cur);
        let mut integer = true;
        let mut ndig = self.digits() as i64;
        if (first == b'0' && ndig > 1) || (negative && ndig == 0) { return Ok(None); }
        let mut exponent: i64 = 0;
        let mut point: i64 = -1;
        if self.peek() == b'.' {
            integer = false;
            point = ndig;
            self.st.cur += 1;
            let f = self.digits() as i64;
            ndig += f;
            if f == 0 { return Ok(None); }
        }
        if self.peek().to_ascii_lowercase() == b'e' {
            integer = false;
            self.st.cur += 1;
            let nc = self.peek();
            let mut neg_e = false;
            if nc == b'-' || nc == b'+' { neg_e = nc == b'-'; self.st.cur += 1; }
            let es = self.st.cur;
            let ed = self.digits();
            if ed == 0 { return Ok(None); }
            let t = std::str::from_utf8(&self.src[es..es + ed]).unwrap_or("0");
            exponent = match t.parse::<u64>() {
                Ok(n) if ed < 20 && n <= i64::MAX as u64 => if neg_e { -(n as i64) } else { n as i64 },
                _ => if neg_e { i32::MIN as i64 } else { i32::MAX as i64 },
            };
        }
        if self.st.resumable && self.eos() { return Ok(None); }
        let end = self.st.cur;
        if integer { return Ok(Some(self.integer_of(c, start, end))); }
        if point >= 0 { exponent -= ndig - point; }
        if self.cfg.decimal_class != NIL { return self.decimal(c, start, end).map(Some); }
        if exponent >= i32::MAX as i64 { return Ok(Some(if negative { V_NINF.get(c) } else { V_INF.get(c) })); }
        if exponent <= i32::MIN as i64 { return Ok(Some(c.float_new(if negative { -0.0 } else { 0.0 }))); }
        if ndig > 18 || ndig + exponent < -307 {
            if ndig + exponent < -324 { return Ok(Some(c.float_new(if negative { -0.0 } else { 0.0 }))); }
            return self.float_of(c, start, end, true).map(Some);
        }
        self.float_of(c, start, end, false).map(Some)
    }
    /// The 2.x number scanner, from the first character ('-' or a digit).
    fn number_v2(&mut self, c: &mut Ctx) -> PR<Value> {
        let start = self.st.cur;
        let mut integer = true;
        self.st.cur += 1;
        while self.peek().is_ascii_digit() { self.st.cur += 1; }
        let il = self.st.cur - start;
        let s0 = self.src[start];
        if (s0 == b'0' && il > 1) || (il > 2 && s0 == b'-' && self.src[start + 1] == b'0') || (il == 1 && s0 == b'-') {
            return Err(self.fail(c, "invalid number: %s", start, false));
        }
        if self.peek() == b'.' {
            integer = false;
            self.st.cur += 1;
            if !self.peek().is_ascii_digit() { return Err(self.syntax(c, "invalid number: %s")); }
            while self.peek().is_ascii_digit() { self.st.cur += 1; }
        }
        if self.peek() == b'e' || self.peek() == b'E' {
            integer = false;
            self.st.cur += 1;
            if self.peek() == b'+' || self.peek() == b'-' { self.st.cur += 1; }
            if !self.peek().is_ascii_digit() { return Err(self.syntax(c, "invalid number: %s")); }
            while self.peek().is_ascii_digit() { self.st.cur += 1; }
        }
        let end = self.st.cur;
        if integer { return Ok(self.integer_of(c, start, end)); }
        if self.cfg.decimal_class != NIL { return self.decimal(c, start, end); }
        self.float_of(c, start, end, true)
    }
    fn keyword(&mut self, c: &mut Ctx, kw: &[u8]) -> PR<()> {
        let rest = &self.src[self.st.cur.min(self.src.len())..];
        if rest.starts_with(kw) { self.st.cur += kw.len(); return Ok(()); }
        let eos = rest.len() < kw.len() && kw.starts_with(rest);
        let at = self.st.cur;
        Err(self.fail(c, "unexpected token %s", at, eos))
    }

    // -- containers
    fn decode_array(&mut self, c: &mut Ctx, count: usize) -> Value {
        let stack = self.st.stack;
        let n = c.ary_len(stack);
        let a = c.ary_new_capa(count);
        { let (d, o) = c.ary_mut_and(a); d.extend_from_slice(&o.ary(stack).unwrap()[n - count..]); }
        c.ary_mut(stack).truncate(n - count);
        if self.cfg.freeze { obj_set_flags(a, FL_FROZEN); }
        a
    }
    fn decode_object(&mut self, c: &mut Ctx, count: usize, at: usize) -> PR<Value> {
        let stack = self.st.stack;
        let n = c.ary_len(stack);
        let base = n - count;
        let h = c.hash_new_capa(count / 2);
        let mut i = 0;
        while i < count {
            let (k, v) = (c.ary_get(stack, base + i), c.ary_get(stack, base + i + 1));
            hash_set(c, h, k, v)?;
            i += 2;
        }
        if c.hash_len(h) < count / 2 && self.cfg.dup != Dup::Ignore {
            let seen = c.hash_new();
            let mut dup = NIL;
            let mut i = 0;
            while i < count {
                let k = c.ary_get(stack, base + i);
                let before = c.hash_len(seen);
                hash_set(c, seen, k, TRUE)?;
                if c.hash_len(seen) == before { dup = if k.is_sym() { c.str_new(crate::symbol::sym_bytes(k.as_sym())) } else { k }; break; }
                i += 2;
            }
            let ins = inspect_value(c, dup)?;
            let insb = c.str_bytes(ins).to_vec();
            if self.cfg.dup == Dup::Deprecated {
                let mut m = b"detected duplicate key ".to_vec();
                m.extend_from_slice(&insb);
                m.extend_from_slice(b" in JSON object. This will raise an error in json 3.0 unless enabled via `allow_duplicate_key: true`");
                let (line, col) = position(self.src, at, true);
                m.extend_from_slice(format!(" at line {} column {}", line, col).as_bytes());
                let w = c.str_new(&m);
                funcall(c, M_JSON.get(c), intern(b"deprecation_warning"), &[w], std::ptr::null())?;
            } else {
                let mut m = b"duplicate key ".to_vec();
                m.extend_from_slice(&insb);
                if self.st.v2 {
                    // the 2.x message is used as a format: a "%s" in it takes the fragment
                    let f = fragment(self.src, at, true);
                    let s = String::from_utf8_lossy(&m).into_owned();
                    m = fmt_msg(&s.replace("%%", "\u{0}"), &f).iter().map(|&b| if b == 0 { b'%' } else { b }).collect();
                }
                self.st.cur = at;
                return Err(self.error_msg(c, m, at, if self.st.v2 { UNDEF } else { dup }));
            }
        }
        c.ary_mut(stack).truncate(base);
        if self.cfg.freeze { obj_set_flags(h, FL_FROZEN); }
        Ok(h)
    }
    fn nest(&mut self, c: &mut Ctx) -> PR<()> {
        self.st.nesting += 1;
        if self.cfg.max_nesting != 0 && self.cfg.max_nesting < self.st.nesting {
            let k = E_NESTING.get(c);
            return Err(PE::Rb(err::<()>(c, k, format!("nesting of {} is too deep", self.st.nesting)).unwrap_err()));
        }
        Ok(())
    }
    fn vlen(&self, c: &Ctx) -> usize { c.ary_len(self.st.stack) }

    /// json_parse_any: run from the top frame's phase; true once the document is complete.
    fn run(&mut self, c: &mut Ctx) -> PR<bool> {
        let resumable = self.st.resumable;
        let mut ph = self.st.frames.last().unwrap().ph;
        loop {
            match ph {
                Ph::Done => return Ok(true),
                Ph::Value => {
                    self.ws(c)?;
                    let fi = self.st.frames.len() - 1;
                    if self.cfg.allow_trailing_comma && self.st.frames[fi].t == FT::Array && self.peek() == b']' && !self.st.v2 { ph = Ph::ArrayComma; continue; }
                    let vstart = self.st.cur;
                    let value = match self.peek() {
                        b'n' => { self.keyword(c, b"null")?; NIL }
                        b't' => { self.keyword(c, b"true")?; TRUE }
                        b'f' => { self.keyword(c, b"false")?; FALSE }
                        b'N' => { if !self.cfg.allow_nan { return Err(self.syntax(c, "unexpected token %s")); } self.keyword(c, b"NaN")?; V_NAN.get(c) }
                        b'I' => { if !self.cfg.allow_nan { return Err(self.syntax(c, "unexpected token %s")); } self.keyword(c, b"Infinity")?; V_INF.get(c) }
                        b'-' if self.st.v2 => {
                            if self.src[vstart..].starts_with(b"-Infinity") {
                                if !self.cfg.allow_nan { return Err(self.syntax(c, "unexpected token %s")); }
                                self.st.cur += 9;
                                V_NINF.get(c)
                            } else { self.number_v2(c)? }
                        }
                        b'0'..=b'9' if self.st.v2 => self.number_v2(c)?,
                        b'-' | b'0'..=b'9' => {
                            let neg = self.peek() == b'-';
                            if neg { self.st.cur += 1; }
                            let v = self.number(c, neg, vstart)?;
                            match v {
                                None if neg && self.cfg.allow_nan && self.peek() == b'I' => { self.st.cur = vstart; self.keyword(c, b"-Infinity")?; V_NINF.get(c) }
                                _ if resumable && self.eos() => { self.st.cur = vstart; return Ok(false); }
                                None => return Err(self.fail(c, "invalid number: %s", vstart, false)),
                                Some(v) => v,
                            }
                        }
                        b'"' => match self.string(c, false)? {
                            Some(v) => v,
                            None => {
                                let e = self.eos();
                                if resumable && e { self.st.cur = vstart; return Ok(false); }
                                let at = self.st.cur;
                                return Err(self.fail(c, "unexpected end of input, expected closing \"", at, e));
                            }
                        },
                        b'[' => {
                            self.st.cur += 1;
                            if self.st.v2 { self.ws(c)?; } else { self.ws_at(c, true, Some(vstart))?; }
                            if self.peek() == b']' { self.st.cur += 1; self.decode_array(c, 0) }
                            else if resumable && self.eos() { self.st.cur = vstart; return Ok(false); }
                            else {
                                self.nest(c)?;
                                self.st.in_array += 1;
                                let vh = self.vlen(c);
                                self.st.frames.push(Frame { t: FT::Array, ph: Ph::Value, vhead: vh, start: 0 });
                                ph = Ph::Value;
                                continue;
                            }
                        }
                        b'{' => {
                            self.st.cur += 1;
                            if self.st.v2 { self.ws(c)?; } else { self.ws_at(c, true, Some(vstart))?; }
                            if self.peek() == b'}' { self.st.cur += 1; self.decode_object(c, 0, vstart)? }
                            else if resumable && self.eos() { self.st.cur = vstart; return Ok(false); }
                            else {
                                self.nest(c)?;
                                let vh = self.vlen(c);
                                self.st.frames.push(Frame { t: FT::Object, ph: Ph::ObjectKey, vhead: vh, start: vstart });
                                if self.st.v2 {
                                    // 2.x reads the first key without skipping anything more
                                    if self.peek() != b'"' { return Err(self.syntax(c, "expected object key, got %s")); }
                                }
                                ph = Ph::ObjectKey;
                                continue;
                            }
                        }
                        0 if self.eos() => {
                            if self.st.v2 || !resumable { let at = self.st.cur; return Err(self.fail(c, "unexpected end of input", at, true)); }
                            return Ok(false);
                        }
                        0 if !self.st.v2 => return Err(self.syntax(c, "unexpected NULL byte: %s")),
                        _ => return Err(self.syntax(c, "unexpected character: %s")),
                    };
                    self.push(c, value)?;
                    ph = completed(self.st.frames.last_mut().unwrap());
                }
                Ph::ObjectKey => {
                    self.ws(c)?;
                    if self.cfg.allow_trailing_comma && self.peek() == b'}' { ph = Ph::ObjectComma; continue; }
                    if self.peek() == b'"' {
                        let start = self.st.cur;
                        match self.string(c, true)? {
                            Some(k) => self.push(c, k)?,
                            None => {
                                if resumable && self.eos() { self.st.cur = start; return Ok(false); }
                                return Err(self.syntax(c, "unexpected end of input, expected closing \""));
                            }
                        }
                        self.st.frames.last_mut().unwrap().ph = Ph::ObjectColon;
                        ph = Ph::ObjectColon;
                        continue;
                    }
                    if resumable && self.eos() { return Ok(false); }
                    let f = *self.st.frames.last().unwrap();
                    if self.vlen(c) == f.vhead { return Err(self.syntax(c, "expected object key, got %s")); }
                    return Err(self.syntax(c, "expected object key, got: %s"));
                }
                Ph::ObjectColon => {
                    self.ws(c)?;
                    if self.peek() == b':' {
                        self.st.cur += 1;
                        self.st.frames.last_mut().unwrap().ph = Ph::Value;
                        ph = Ph::Value;
                        continue;
                    }
                    if resumable && self.eos() { return Ok(false); }
                    let f = *self.st.frames.last().unwrap();
                    if self.vlen(c) - f.vhead == 1 { return Err(self.syntax(c, "expected ':' after object key")); }
                    return Err(self.syntax(c, "expected ':' after object key, got: %s"));
                }
                Ph::ArrayComma => {
                    self.ws(c)?;
                    match self.peek() {
                        b',' => {
                            self.st.cur += 1;
                            self.st.frames.last_mut().unwrap().ph = Ph::Value;
                            if self.st.v2 && self.cfg.allow_trailing_comma {
                                self.ws(c)?;
                                if self.peek() == b']' { self.st.frames.last_mut().unwrap().ph = Ph::ArrayComma; ph = Ph::ArrayComma; continue; }
                            }
                            ph = Ph::Value;
                        }
                        b']' => {
                            self.st.cur += 1;
                            let f = *self.st.frames.last().unwrap();
                            let count = self.vlen(c) - f.vhead;
                            self.st.nesting -= 1;
                            self.st.in_array -= 1;
                            let a = self.decode_array(c, count);
                            self.st.frames.pop();
                            self.push(c, a)?;
                            ph = completed(self.st.frames.last_mut().unwrap());
                        }
                        _ => {
                            if resumable && self.eos() { return Ok(false); }
                            return Err(self.syntax(c, "expected ',' or ']' after array value"));
                        }
                    }
                }
                Ph::ObjectComma => {
                    self.ws(c)?;
                    match self.peek() {
                        b',' => {
                            self.st.cur += 1;
                            self.st.frames.last_mut().unwrap().ph = Ph::ObjectKey;
                            if self.st.v2 {
                                self.ws(c)?;
                                if self.cfg.allow_trailing_comma && self.peek() == b'}' { self.st.frames.last_mut().unwrap().ph = Ph::ObjectComma; ph = Ph::ObjectComma; continue; }
                                if self.peek() != b'"' { return Err(self.syntax(c, "expected object key, got: %s")); }
                            }
                            ph = Ph::ObjectKey;
                        }
                        b'}' => {
                            self.st.cur += 1;
                            self.st.nesting -= 1;
                            let f = *self.st.frames.last().unwrap();
                            let count = self.vlen(c) - f.vhead;
                            let fin = self.st.cur;
                            self.st.cur = f.start;
                            let o = self.decode_object(c, count, f.start)?;
                            self.st.cur = fin;
                            self.st.frames.pop();
                            self.push(c, o)?;
                            ph = completed(self.st.frames.last_mut().unwrap());
                        }
                        _ => {
                            if resumable && self.eos() { return Ok(false); }
                            return Err(self.syntax(c, "expected ',' or '}' after object value, got: %s"));
                        }
                    }
                }
            }
        }
    }
}

fn new_state(c: &mut Ctx, resumable: bool) -> St {
    let stack = c.ary_new();
    St { cur: 0, stack, frames: Vec::new(), nesting: 0, in_array: if resumable { 1 } else { 0 }, cache: Vec::new(), cache_root: NIL, resumable, v2: v2(c) }
}

/// convert_encoding: UTF-8 as is, binary read as UTF-8, anything else encoded.
fn convert_encoding(c: &mut Ctx, src: Value) -> RV {
    let s = expect_str(c, src)?;
    let enc = c.str_enc(s);
    if enc == ENC_UTF8 { return Ok(s); }
    if enc == ENC_BINARY { return Ok(c.str_copy_enc(s, ENC_UTF8)); }
    let u = crate::builtins::misc::encoding_value(c, ENC_UTF8)?;
    let r = funcall(c, s, intern(b"encode"), &[u], std::ptr::null())?;
    expect_str(c, r)
}

/// cParser_parse: one document from `src`.
fn parse(c: &mut Ctx, cfg: &Config, src: Value) -> RV {
    let s = convert_encoding(c, src)?;
    let bytes = c.str_bytes(s).to_vec();
    let mut st = new_state(c, false);
    st.frames.push(Frame { t: FT::Root, ph: Ph::Value, vhead: 0, start: 0 });
    let mut m = M { src: &bytes, st: &mut st, cfg };
    let r = match m.run(c) {
        Ok(true) => {
            let v = c.ary_get(m.st.stack, 0);
            match m.ws(c) {
                Ok(()) if !m.eos() => Err(m.syntax(c, "unexpected token at end of stream %s")),
                Ok(()) => Ok(v),
                Err(e) => Err(e),
            }
        }
        Ok(false) => { let at = m.st.cur; Err(m.fail(c, "unexpected end of input", at, true)) }
        Err(e) => Err(e),
    };
    match r { Ok(v) => Ok(v), Err(PE::Rb(e)) => Err(e), Err(PE::Eos) => Ok(NIL) }
}

fn config_of(c: &Ctx, v: Value) -> Option<Config> { c.data::<PConfig>(v).map(|p| p.cfg) }

fn pc_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 1)?;
    if !v2(c) { check_frozen(c, slf)?; }
    let cfg = config_init(c, a[0])?;
    let p = c.data_mut::<PConfig>(slf).expect("a ParserConfig");
    p.cfg = cfg; p.set = true;
    Ok(slf)
}
fn pc_parse(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let cfg = config_of(c, slf).expect("a ParserConfig");
    parse(c, &cfg, a[0])
}
fn parser_s_parse(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    if v2(c) { convert_encoding(c, a[0])?; }
    let cfg = config_init(c, a[1])?;
    parse(c, &cfg, a[0])
}

// ---- ResumableParser --------------------------------------------------------------------------

pub struct Resumable {
    cfg: Config,
    st: Option<St>,
    buf: Vec<u8>,
    has_buf: bool,
    parsed: usize,
    incomplete: usize,
    complete: bool,
    in_use: bool,
}
impl RData for Resumable {
    fn visit(&mut self, m: &mut crate::gc::Visitor) {
        m.visit(&mut self.cfg.on_load); m.visit(&mut self.cfg.decimal_class);
        if let Some(st) = self.st.as_mut() { m.visit(&mut st.stack); m.visit(&mut st.cache_root); for v in st.cache.iter_mut() { m.visit(v); } }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn rp(c: &mut Ctx, v: Value) -> &mut Resumable { c.data_mut::<Resumable>(v).expect("a ResumableParser") }
fn rp_acquire(c: &mut Ctx, v: Value, lock: bool) -> Result<(), Esc> {
    if rp(c, v).in_use { return arg_err(c, "ResumableParser can't be used recursively"); }
    if lock { rp(c, v).in_use = true; }
    Ok(())
}
fn rp_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_frozen(c, slf)?;
    if a.len() > 1 || (a.len() == 1 && !is_hash(a[0])) { return Err(argc_error(c, a.len(), 0, 0)); }
    let cfg = config_init(c, a.first().copied().unwrap_or(NIL))?;
    rp(c, slf).cfg = cfg;
    Ok(slf)
}
fn argc_error(c: &mut Ctx, given: usize, min: usize, max: usize) -> Esc {
    let m = if min == max { format!("wrong number of arguments (given {}, expected {})", given, min) } else { format!("wrong number of arguments (given {}, expected {}..{})", given, min, max) };
    arg_err::<()>(c, m).unwrap_err()
}
fn rp_feed(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_frozen(c, slf)?;
    rp_acquire(c, slf, false)?;
    let s = convert_encoding(c, a[0])?;
    if c.str_len(s) == 0 { return Ok(slf); }
    let b = c.str_bytes(s).to_vec();
    let r = rp(c, slf);
    let cur = r.st.as_ref().map(|s| s.cur).unwrap_or(0);
    // append only: open frames keep offsets into the buffer
    if !r.has_buf || cur >= r.buf.len() { r.buf = b; if let Some(st) = r.st.as_mut() { st.cur = 0; } }
    else { r.buf.extend_from_slice(&b); }
    r.has_buf = true;
    Ok(slf)
}
fn rp_parse(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    rp_acquire(c, slf, true)?;
    {
        let r = rp(c, slf);
        if r.complete { r.parsed = 0; r.incomplete = 0; r.complete = false; }
        if !r.has_buf { r.in_use = false; return Ok(FALSE); }
    }
    if rp(c, slf).st.is_none() { let st = new_state(c, true); rp(c, slf).st = Some(st); }
    let (mut st, buf, cfg) = { let r = rp(c, slf); (r.st.take().unwrap(), std::mem::take(&mut r.buf), r.cfg) };
    let stack = st.stack;
    if st.frames.is_empty() { st.frames.push(Frame { t: FT::Root, ph: Ph::Value, vhead: 0, start: 0 }); }
    if st.frames.last().unwrap().ph == Ph::Done {
        st.frames.last_mut().unwrap().ph = Ph::Value;
        let n = c.ary_len(stack);
        c.ary_mut(stack).truncate(n.saturating_sub(1));
    }
    let initial = st.cur;
    let r = { let mut m = M { src: &buf, st: &mut st, cfg: &cfg }; m.run(c) };
    let (complete, err) = match r { Ok(b) => (b, None), Err(PE::Eos) => (false, None), Err(PE::Rb(e)) => (true, Some(e)) };
    let parsed = st.cur.saturating_sub(initial);
    let incomplete = if complete { 0 } else { buf.len() - st.cur.min(buf.len()) };
    // eat plain whitespace; drop the buffer once consumed
    while st.cur < buf.len() && matches!(buf[st.cur], b' ' | b'\n' | b'\t' | b'\r') { st.cur += 1; }
    let eos = st.cur >= buf.len();
    let r = rp(c, slf);
    r.complete = complete;
    r.parsed += parsed;
    r.incomplete = incomplete;
    if eos { r.has_buf = false; r.buf = Vec::new(); st.cur = 0; } else { r.buf = buf; }
    r.st = Some(st);
    r.in_use = false;
    let _ = stack;
    match err { Some(e) => Err(e), None => Ok(Value::bool(complete)) }
}
fn rp_value_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    rp_acquire(c, slf, false)?;
    let r = rp(c, slf);
    let Some(st) = r.st.as_ref() else { return Ok(FALSE) };
    let (stack, done) = (st.stack, st.frames.last().map(|f| f.ph == Ph::Done).unwrap_or(false));
    Ok(Value::bool(c.ary_len(stack) > 0 && done))
}
fn rp_value(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    rp_acquire(c, slf, false)?;
    let r = rp(c, slf);
    if let Some(st) = r.st.as_mut() {
        if st.frames.last().map(|f| f.ph == Ph::Done).unwrap_or(false) {
            st.frames.pop();
            let stack = st.stack;
            let n = c.ary_len(stack);
            let v = c.ary_get(stack, n - 1);
            c.ary_mut(stack).truncate(n - 1);
            return Ok(v);
        }
    }
    arg_err(c, "no ready value")
}
fn rp_clear(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    rp_acquire(c, slf, false)?;
    let r = rp(c, slf);
    r.has_buf = false; r.buf = Vec::new(); r.complete = true; r.parsed = 0; r.incomplete = 0; r.st = None;
    Ok(slf)
}
fn rp_partial_value(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    rp_acquire(c, slf, true)?;
    let r = partial_value(c, slf);
    rp(c, slf).in_use = false;
    r
}
fn partial_value(c: &mut Ctx, slf: Value) -> RV {
    let (cfg, frames, stack0, buf) = {
        let r = rp(c, slf);
        let Some(st) = r.st.as_ref() else { return Ok(NIL) };
        (r.cfg, st.frames.clone(), st.stack, r.buf.clone())
    };
    if c.ary_len(stack0) == 0 { return Ok(NIL); }
    let copy = c.ary_new_from(&[]);
    c.ary_concat(copy, stack0);
    let mut st = St { cur: 0, stack: copy, frames, nesting: 0, in_array: 1, cache: Vec::new(), cache_root: NIL, resumable: true, v2: false };
    let top = *st.frames.last().unwrap();
    if top.t == FT::Object && (top.ph == Ph::Value || top.ph == Ph::ObjectColon) { c.ary_push(copy, NIL); }
    let mut m = M { src: &buf, st: &mut st, cfg: &cfg };
    loop {
        let f = *m.st.frames.last().unwrap();
        let count = c.ary_len(copy) - f.vhead;
        match f.t {
            FT::Root => return Ok(c.ary_get(copy, c.ary_len(copy) - 1)),
            FT::Array => { let a = m.decode_array(c, count); m.st.frames.pop(); if let Err(PE::Rb(e)) = m.push(c, a) { return Err(e); } }
            FT::Object => {
                let o = match m.decode_object(c, count, 0) { Ok(o) => o, Err(PE::Rb(e)) => return Err(e), Err(PE::Eos) => NIL };
                m.st.frames.pop();
                if let Err(PE::Rb(e)) = m.push(c, o) { return Err(e); }
            }
        }
    }
}
fn rp_rest(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let r = rp(c, slf);
    if !r.has_buf { return Ok(c.str_new(b"")); }
    let cur = r.st.as_ref().map(|s| s.cur).unwrap_or(0);
    let b = r.buf[cur.min(r.buf.len())..].to_vec();
    Ok(c.str_from_vec(b))
}
fn rp_eos_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let r = rp(c, slf);
    let cur = r.st.as_ref().map(|s| s.cur).unwrap_or(0);
    Ok(Value::bool(!r.has_buf || cur >= r.buf.len()))
}
fn rp_partial_value_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let r = rp(c, slf);
    let Some(st) = r.st.as_ref() else { return Ok(FALSE) };
    let (stack, done) = (st.stack, st.frames.last().map(|f| f.ph == Ph::Done).unwrap_or(true));
    Ok(Value::bool(c.ary_len(stack) > 0 && !done))
}
fn rp_parsed_bytes(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let r = rp(c, slf);
    let n = (r.parsed + r.incomplete) as i64;
    Ok(c.int_from_i64(n))
}

// ---- allocation and init ----------------------------------------------------------------------

pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let (pc, rpk) = (C_CONFIG.get(c), C_RESUMABLE.get(c));
    if pc != NIL && crate::builtins::inherits(c, klass, pc) { let cfg = Config::new(c); return Ok(Some(c.alloc(klass, Body::Data(Box::new(PConfig { cfg, set: false }))))); }
    if rpk != NIL && crate::builtins::inherits(c, klass, rpk) {
        let cfg = Config::new(c);
        return Ok(Some(c.alloc(klass, Body::Data(Box::new(Resumable { cfg, st: None, buf: Vec::new(), has_buf: false, parsed: 0, incomplete: 0, complete: true, in_use: false })))));
    }
    Ok(None)
}

fn path_class(c: &mut Ctx, m: Value, name: &str) -> Result<Value, Esc> {
    match const_get_at(c, m, intern(name.as_bytes())) { Some(k) => Ok(k), None => err(c, c.vm().c_name_error.get(), format!("uninitialized constant JSON::{}", name)) }
}
fn module_under(c: &mut Ctx, outer: Value, name: &str) -> Value {
    match const_get_at(c, outer, intern(name.as_bytes())) { Some(m) if is_class_or_module(m) => m, _ => define_module_under(c, outer, name) }
}
pub fn class_under(c: &mut Ctx, outer: Value, name: &str, sup: Value) -> Value {
    match const_get_at(c, outer, intern(name.as_bytes())) { Some(k) if is_class_or_module(k) => k, _ => define_class_under(c, outer, name, sup) }
}

/// Init_parser, run by `require "json/ext/parser"`.
pub fn init(c: &mut Ctx) -> RV {
    let f = c.str_new(b"json/common");
    require_feature(c, f, false)?;
    let o = c.vm().c_object.get();
    let m = module_under(c, o, "JSON");
    M_JSON.set(c, m);
    json_version(c);
    let ext = module_under(c, m, "Ext");
    let pc = class_under(c, ext, "ParserConfig", o);
    class_set_alloc(c, pc, AllocKind::None);
    C_CONFIG.set(c, pc);
    let pe = path_class(c, m, "ParserError")?; E_PARSER.set(c, pe);
    let ne = path_class(c, m, "NestingError")?; E_NESTING.set(c, ne);
    if v2(c) { define_cfunc(c, pc, "initialize", pc_initialize, 1); } else { define_private_cfunc(c, pc, "initialize", pc_initialize, 1); }
    define_cfunc(c, pc, "parse", pc_parse, 1);
    let p = class_under(c, ext, "Parser", o);
    define_singleton_cfunc(c, p, "parse", parser_s_parse, 2);
    if !v2(c) {
        let r = class_under(c, m, "ResumableParser", o);
        class_set_alloc(c, r, AllocKind::None);
        C_RESUMABLE.set(c, r);
        define_private_cfunc(c, r, "initialize", rp_initialize, -1);
        let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, r, n, f, a);
        d(c, "<<", rp_feed, 1); d(c, "parse", rp_parse, 0); d(c, "value", rp_value, 0); d(c, "value?", rp_value_p, 0);
        d(c, "partial_value", rp_partial_value, 0); d(c, "partial_value?", rp_partial_value_p, 0); d(c, "clear", rp_clear, 0);
        d(c, "rest", rp_rest, 0); d(c, "eos?", rp_eos_p, 0); d(c, "parsed_bytes", rp_parsed_bytes, 0);
    }
    for (g, n) in [(&V_NAN, "NaN"), (&V_INF, "Infinity"), (&V_NINF, "MinusInfinity")] { let v = path_class(c, m, n)?; g.set(c, v); }
    Ok(NIL)
}
