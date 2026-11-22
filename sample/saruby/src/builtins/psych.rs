// psych.so (ext/psych): Psych::Parser / Psych::Emitter over the libyaml port (yaml_parser.rs,
// yaml_emitter.rs) and the small helpers of ToRuby / ClassLoader; CRuby's lib/psych*.rb runs on top.
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::array::*;
use crate::builtins::encoding::{default_internal, ENC_UTF16BE, ENC_UTF16LE};
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::builtins::yaml_emitter::Emitter;
use crate::builtins::yaml_parser::{self as yp, EventData, Mark, Parser, Source};
use crate::heap::Global;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

static M_PSYCH: Global = Global::new();
static C_PARSER: Global = Global::new();
static C_EMITTER: Global = Global::new();

pub struct PParser { mark: Mark }
impl RData for PParser {
    fn visit(&mut self, _m: &mut crate::gc::Visitor) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
pub struct PEmitter { em: Emitter, io: Value }
impl RData for PEmitter {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.io); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let (p, e) = (C_PARSER.get(c), C_EMITTER.get(c));
    if p != NIL && crate::builtins::inherits(c, klass, p) { return Ok(Some(c.alloc(klass, Body::Data(Box::new(PParser { mark: Mark::default() }))))); }
    if e != NIL && crate::builtins::inherits(c, klass, e) {
        let mut em = Emitter::new();
        em.set_indent(2);
        return Ok(Some(c.alloc(klass, Body::Data(Box::new(PEmitter { em, io: NIL })))));
    }
    Ok(None)
}

fn check_type<T>(c: &mut Ctx, v: Value, expected: &str) -> Result<T, Esc> { type_err(c, format!("wrong argument type {} (expected {})", class_name_for_err(c, v), expected)) }
/// rb_str_export_to_enc(v, UTF-8): converted when possible, else the bytes as they are.
fn export_utf8(c: &mut Ctx, v: Value) -> Vec<u8> {
    let enc = c.str_enc(v);
    let b = c.str_bytes(v).to_vec();
    if enc == ENC_UTF8 || enc == ENC_BINARY || (crate::builtins::encoding::ascii_compat(enc) && b.is_ascii()) { return b; }
    if !crate::builtins::encoding::can_convert(enc, ENC_UTF8) { return b; }
    match crate::builtins::encoding::transcode(c, &b, enc, ENC_UTF8, &Default::default()) { Ok(o) => o, Err(_) => { c.esc = EscKind::None; b } }
}
/// A string argument as UTF-8 C string bytes (StringValueCStr after the export).
fn cstr_arg(c: &mut Ctx, v: Value) -> Result<Vec<u8>, Esc> {
    let b = export_utf8(c, v);
    if b.contains(&0) { return arg_err(c, "string contains null byte"); }
    Ok(b)
}
/// An optional (nil or String) anchor / tag.
fn opt_str(c: &mut Ctx, v: Value) -> Result<Option<Vec<u8>>, Esc> {
    if v == NIL { return Ok(None); }
    if !is_string(v) { return check_type(c, v, "String"); }
    cstr_arg(c, v).map(Some)
}
/// PSYCH_TRANSCODE: a UTF-8 String, exported to Encoding.default_internal when set.
fn ystr(c: &mut Ctx, b: &[u8]) -> Value {
    let s = c.str_new(b);
    match default_internal() {
        Some(e) if e != ENC_UTF8 && crate::builtins::encoding::can_convert(ENC_UTF8, e) && !(crate::builtins::encoding::ascii_compat(e) && b.is_ascii()) => {
            match crate::builtins::encoding::transcode(c, b, ENC_UTF8, e, &Default::default()) { Ok(o) => c.str_new_enc(&o, e), Err(_) => { c.esc = EscKind::None; s } }
        }
        Some(e) if e != ENC_UTF8 && crate::builtins::encoding::ascii_compat(e) && b.is_ascii() => c.str_new_enc(b, e),
        _ => s,
    }
}
/// rb_str_new2: up to the first NUL.
fn ycstr(c: &mut Ctx, b: &[u8]) -> Value { let n = b.iter().position(|&x| x == 0).unwrap_or(b.len()); ystr(c, &b[..n]) }
fn yopt(c: &mut Ctx, b: &Option<Vec<u8>>) -> Value { match b { Some(b) => ycstr(c, b), None => NIL } }

// ---- parser -------------------------------------------------------------------------------------

struct Src<'a> { c: &'a mut Ctx, io: Value, s: Vec<u8>, pos: usize, err: Option<Esc> }
impl Source for Src<'_> {
    fn read(&mut self, max: usize, out: &mut Vec<u8>) -> bool {
        if self.io == NIL {
            let n = max.min(self.s.len() - self.pos);
            out.extend_from_slice(&self.s[self.pos..self.pos + n]);
            self.pos += n;
            return true;
        }
        let r = funcall(self.c, self.io, intern(b"read"), &[Value::fix(max as i64)], std::ptr::null());
        let r = match r { Ok(v) if v == NIL => return true, Ok(v) => expect_str(self.c, v), Err(e) => Err(e) };
        match r { Ok(s) => { out.extend_from_slice(self.c.str_bytes(s)); true } Err(e) => { self.err = Some(e); false } }
    }
}

fn syntax_error(c: &mut Ctx, e: &yp::YError, path: Value) -> Esc {
    let m = M_PSYCH.get(c);
    let k = match const_get_inherit(c, m, intern(b"SyntaxError")) { Ok(Some(k)) => k, Ok(None) => return raise_msg(c, c.vm().c_runtime_error.get(), e.problem), Err(x) => return x };
    let problem = c.str_new_enc(e.problem.as_bytes(), ENC_ASCII);
    let context = match e.context { Some(s) => c.str_new_enc(s.as_bytes(), ENC_ASCII), None => NIL };
    let args = [path, Value::fix(e.context_mark.line as i64 + 1), Value::fix(e.context_mark.column as i64 + 1), Value::fix(e.problem_offset as i64), problem, context];
    match funcall(c, k, intern(b"new"), &args, std::ptr::null()) { Ok(x) => raise_exc(c, x), Err(x) => x }
}

struct Ids { loc: Sym, ss: Sym, es: Sym, sd: Sym, ed: Sym, al: Sym, sc: Sym, sq: Sym, eq: Sym, sm: Sym, em: Sym }

fn p_native_parse(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (handler, yaml, path) = (a[0], a[1], a[2]);
    let (io, bytes, enc) = if respond_to(c, yaml, intern(b"read"), false) {
        let ee = funcall(c, yaml, intern(b"external_encoding"), &[], std::ptr::null())?;
        let ei = if ee == NIL { ENC_BINARY } else { crate::builtins::encoding::to_enc(c, ee)? };
        let pe = if ei == ENC_ASCII || ei == ENC_UTF8 { yp::UTF8_ENCODING } else if ei == ENC_UTF16LE { yp::UTF16LE_ENCODING } else if ei == ENC_UTF16BE { yp::UTF16BE_ENCODING } else { yp::ANY_ENCODING };
        (yaml, Vec::new(), pe)
    } else {
        let s = expect_str(c, yaml)?;
        let e = c.str_enc(s);
        if e == ENC_UTF8 { (NIL, c.str_bytes(s).to_vec(), yp::UTF8_ENCODING) }
        else if e == ENC_UTF16LE { (NIL, c.str_bytes(s).to_vec(), yp::UTF16LE_ENCODING) }
        else if e == ENC_UTF16BE { (NIL, c.str_bytes(s).to_vec(), yp::UTF16BE_ENCODING) }
        else { (NIL, export_utf8(c, s), yp::UTF8_ENCODING) }
    };
    let ids = Ids { loc: intern(b"event_location"), ss: intern(b"start_stream"), es: intern(b"end_stream"), sd: intern(b"start_document"), ed: intern(b"end_document"),
        al: intern(b"alias"), sc: intern(b"scalar"), sq: intern(b"start_sequence"), eq: intern(b"end_sequence"), sm: intern(b"start_mapping"), em: intern(b"end_mapping") };
    let mut p = Parser::new(Src { c, io, s: bytes, pos: 0, err: None }, enc);
    loop {
        let r = p.parse();
        let mark = p.mark;
        let c = &mut *p.src.c;
        if let Some(d) = c.data_mut::<PParser>(slf) { d.mark = mark; }
        let ev = match r {
            Ok(Some(ev)) => ev,
            Ok(None) => { let e = p.error.clone(); match e { Some(e) => return Err(syntax_error(&mut *p.src.c, &e, path)), None => return Ok(slf) } }
            Err(()) => {
                if let Some(x) = p.src.err.take() { return Err(x); }
                let e = p.error.clone().expect("a parser error");
                return Err(syntax_error(&mut *p.src.c, &e, path));
            }
        };
        let fx = |n: usize| Value::fix(n as i64);
        let loc = [fx(ev.start.line), fx(ev.start.column), fx(ev.end.line), fx(ev.end.column)];
        // like the rb_protect chain of psych_parser.c, a raise in event_location is dropped
        if funcall(c, handler, ids.loc, &loc, std::ptr::null()).is_err() { c.esc = EscKind::None; }
        let nb = std::ptr::null();
        let mut done = false;
        match &ev.data {
            EventData::StreamStart { encoding } => { funcall(c, handler, ids.ss, &[Value::fix(*encoding as i64)], nb)?; }
            EventData::DocumentStart { version, tags, implicit } => {
                let tl = c.ary_new();
                let ver = match version { Some((ma, mi)) => c.ary_new_from(&[Value::fix(*ma as i64), Value::fix(*mi as i64)]), None => c.ary_new() };
                for (h, pr) in tags {
                    let hv = ycstr(c, h);
                    let pv = ycstr(c, pr);
                    let t = c.ary_new_from(&[hv, pv]);
                    c.ary_push(tl, t);
                }
                funcall(c, handler, ids.sd, &[ver, tl, Value::bool(*implicit)], nb)?;
            }
            EventData::DocumentEnd { implicit } => { funcall(c, handler, ids.ed, &[Value::bool(*implicit)], nb)?; }
            EventData::Alias { anchor } => { let a = ycstr(c, anchor); funcall(c, handler, ids.al, &[a], nb)?; }
            EventData::Scalar { anchor, tag, value, plain_implicit, quoted_implicit, style } => {
                let v = ystr(c, value);
                let an = yopt(c, anchor);
                let tg = yopt(c, tag);
                funcall(c, handler, ids.sc, &[v, an, tg, Value::bool(*plain_implicit), Value::bool(*quoted_implicit), Value::fix(*style as i64)], nb)?;
            }
            EventData::SequenceStart { anchor, tag, implicit, style } => {
                let an = yopt(c, anchor);
                let tg = yopt(c, tag);
                funcall(c, handler, ids.sq, &[an, tg, Value::bool(*implicit), Value::fix(*style as i64)], nb)?;
            }
            EventData::SequenceEnd => { funcall(c, handler, ids.eq, &[], nb)?; }
            EventData::MappingStart { anchor, tag, implicit, style } => {
                let an = yopt(c, anchor);
                let tg = yopt(c, tag);
                funcall(c, handler, ids.sm, &[an, tg, Value::bool(*implicit), Value::fix(*style as i64)], nb)?;
            }
            EventData::MappingEnd => { funcall(c, handler, ids.em, &[], nb)?; }
            EventData::StreamEnd => { funcall(c, handler, ids.es, &[], nb)?; done = true; }
        }
        if done { return Ok(slf); }
    }
}
/// Parser#parse(yaml, path = ...) of psych 4's psych.so (psych 5 defines it in parser.rb).
fn p_parse(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let path = if a.len() == 2 { a[1] } else if respond_to(c, a[0], intern(b"path"), false) { funcall(c, a[0], intern(b"path"), &[], std::ptr::null())? } else { c.str_new(b"<unknown>") };
    let h = ivar_get(c, slf, intern(b"@handler"));
    p_native_parse(c, slf, &[h, a[0], path], b)
}
fn p_mark(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let m = c.data::<PParser>(slf).map(|d| d.mark).unwrap_or_default();
    let k = match const_get_at(c, C_PARSER.get(c), intern(b"Mark")) { Some(k) => k, None => return err(c, c.vm().c_name_error.get(), "uninitialized constant Psych::Parser::Mark") };
    funcall(c, k, intern(b"new"), &[Value::fix(m.index as i64), Value::fix(m.line as i64), Value::fix(m.column as i64)], std::ptr::null())
}

// ---- emitter ------------------------------------------------------------------------------------

fn em_mut(c: &mut Ctx, v: Value) -> Result<&mut PEmitter, Esc> {
    if c.data::<PEmitter>(v).is_none() { return type_err(c, "wrong argument type (expected Psych/emitter)"); }
    Ok(c.data_mut::<PEmitter>(v).unwrap())
}
/// Hand one event to the emitter, then write what it flushed to the IO.
fn emit(c: &mut Ctx, slf: Value, ev: EventData) -> RV {
    if !Emitter::event_ok(&ev) { return Ok(slf); }
    let pe = em_mut(c, slf)?;
    let r = pe.em.emit(ev);
    let out = std::mem::take(&mut pe.em.out);
    let (io, problem) = (pe.io, pe.em.problem);
    for chunk in out {
        let s = c.str_new(&chunk);
        let w = funcall(c, io, intern(b"write"), &[s], std::ptr::null())?;
        if num_to_long(c, w)? as i32 == 0 { return err(c, c.vm().c_runtime_error.get(), "write error"); }
    }
    if r.is_err() { return err(c, c.vm().c_runtime_error.get(), problem.unwrap_or("")); }
    Ok(slf)
}
/// C truthiness of a VALUE: only false is 0.
fn ctrue(v: Value) -> bool { v != FALSE }
fn num2int(c: &mut Ctx, v: Value) -> Result<i64, Esc> {
    let n = num_to_long(c, v)?;
    if n > i32::MAX as i64 || n < i32::MIN as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too {} to convert to 'int'", n, if n < 0 { "small" } else { "big" })); }
    Ok(n)
}
fn e_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    if a.len() == 2 {
        let o = a[1];
        let lw = funcall(c, o, intern(b"line_width"), &[], std::ptr::null())?;
        let ind = funcall(c, o, intern(b"indentation"), &[], std::ptr::null())?;
        let can = funcall(c, o, intern(b"canonical"), &[], std::ptr::null())?;
        let lw = num2int(c, lw)?;
        let ind = num2int(c, ind)?;
        let pe = em_mut(c, slf)?;
        pe.em.set_width(lw);
        pe.em.set_indent(ind);
        pe.em.canonical = can == TRUE;
    }
    em_mut(c, slf)?.io = a[0];
    Ok(slf)
}
fn e_start_stream(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if !a[0].is_fixnum() { return check_type(c, a[0], "Integer"); }
    let e = num2int(c, a[0])?;
    emit(c, slf, EventData::StreamStart { encoding: e as i32 })
}
fn e_end_stream(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { emit(c, slf, EventData::StreamEnd) }
fn e_start_document(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (version, tags, imp) = (a[0], a[1], a[2]);
    if !crate::array::is_array(version) { return check_type(c, version, "Array"); }
    let vl = c.ary(version).len();
    let ver = if vl > 0 {
        let (ma, mi) = (c.ary(version).first().copied().unwrap_or(NIL), c.ary(version).get(1).copied().unwrap_or(NIL));
        let ma = num2int(c, ma)?;
        let mi = num2int(c, mi)?;
        Some((ma as i32, mi as i32))
    } else { None };
    let mut tl = Vec::new();
    if tags.truthy() {
        if !crate::array::is_array(tags) { return check_type(c, tags, "Array"); }
        let n = c.ary(tags).len();
        for i in 0..n {
            let Some(t) = c.ary(tags).get(i).copied() else { break };
            if !crate::array::is_array(t) { return check_type(c, t, "Array"); }
            if c.ary(t).len() < 2 { return err(c, c.vm().c_runtime_error.get(), "tag tuple must be of length 2"); }
            let (h, p) = (c.ary(t)[0], c.ary(t)[1]);
            let h = expect_str(c, h)?;
            let p = expect_str(c, p)?;
            let h = cstr_arg(c, h)?;
            let p = cstr_arg(c, p)?;
            tl.push((h, p));
        }
    }
    emit(c, slf, EventData::DocumentStart { version: ver, tags: tl, implicit: ctrue(imp) })
}
fn e_end_document(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { emit(c, slf, EventData::DocumentEnd { implicit: ctrue(a[0]) }) }
fn e_scalar(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if !is_string(a[0]) { return check_type(c, a[0], "String"); }
    let value = export_utf8(c, a[0]);
    let anchor = opt_str(c, a[1])?;
    let tag = opt_str(c, a[2])?;
    let style = num2int(c, a[5])?;
    emit(c, slf, EventData::Scalar { anchor, tag, value, plain_implicit: ctrue(a[3]), quoted_implicit: ctrue(a[4]), style: style as i32 })
}
fn e_start_sequence(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let anchor = opt_str(c, a[0])?;
    let tag = opt_str(c, a[1])?;
    let style = num2int(c, a[3])?;
    emit(c, slf, EventData::SequenceStart { anchor, tag, implicit: ctrue(a[2]), style: style as i32 })
}
fn e_end_sequence(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { emit(c, slf, EventData::SequenceEnd) }
fn e_start_mapping(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let anchor = opt_str(c, a[0])?;
    let tag = opt_str(c, a[1])?;
    let style = num2int(c, a[3])?;
    emit(c, slf, EventData::MappingStart { anchor, tag, implicit: ctrue(a[2]), style: style as i32 })
}
fn e_end_mapping(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { emit(c, slf, EventData::MappingEnd) }
fn e_alias(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let anchor = opt_str(c, a[0])?.unwrap_or_default();
    emit(c, slf, EventData::Alias { anchor })
}
fn e_canonical(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(em_mut(c, slf)?.em.canonical)) }
fn e_set_canonical(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { em_mut(c, slf)?.em.canonical = a[0] == TRUE; Ok(a[0]) }
fn e_indentation(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(em_mut(c, slf)?.em.best_indent as i64)) }
fn e_set_indentation(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let n = num2int(c, a[0])?; em_mut(c, slf)?.em.set_indent(n); Ok(a[0]) }
fn e_line_width(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(em_mut(c, slf)?.em.best_width as i64)) }
fn e_set_line_width(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let n = num2int(c, a[0])?; em_mut(c, slf)?.em.set_width(n); Ok(a[0]) }

// ---- to_ruby / class_loader ---------------------------------------------------------------------

fn tr_build_exception(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let e = funcall(c, a[0], intern(b"allocate"), &[], std::ptr::null())?;
    ivar_set_raw(c, e, intern(b"mesg"), a[1]);
    Ok(e)
}
/// rb_path_to_class.
fn cl_path2class(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let path = expect_str(c, a[0])?;
    let enc = c.str_enc(path);
    if !crate::builtins::encoding::ascii_compat(enc) { return arg_err(c, "invalid class path encoding (non ASCII)"); }
    let b = c.str_bytes(path).to_vec();
    if b.is_empty() || b[0] == b'#' {
        // QUOTE(): rb_str_quote_unprintable
        let printable = std::str::from_utf8(&b).map(|s| !s.chars().any(|ch| ch.is_control())).unwrap_or(false);
        let q = if printable { String::from_utf8_lossy(&b).into_owned() } else { inspect_str(c, path)? };
        return arg_err(c, format!("can't retrieve anonymous class {}", q));
    }
    let undefined = |c: &mut Ctx, upto: usize| -> RV { let s = String::from_utf8_lossy(&b[..upto]).into_owned(); arg_err(c, format!("undefined class/module {}", s)) };
    let mut k = c.vm().c_object.get();
    let (mut p, mut pbeg) = (0, 0);
    let mut first = true;
    while p < b.len() {
        while p < b.len() && b[p] != b':' { p += 1; }
        let seg = b[pbeg..p].to_vec();
        if p < b.len() && b[p] == b':' {
            if b.len() - p < 2 || b[p + 1] != b':' { return undefined(c, p); }
            p += 2;
            pbeg = p;
        }
        let valid = !seg.is_empty() && (seg[0].is_ascii_uppercase() || seg[0] >= 0x80) && seg.iter().all(|x| x.is_ascii_alphanumeric() || *x == b'_' || *x >= 0x80);
        if !valid { return undefined(c, p); }
        let id = intern(&seg);
        let found = match const_get_at(c, k, id) {
            Some(v) => Some(v),
            None if first => const_get_inherit(c, k, id)?,
            None => {
                let anc = ancestors(c, k);
                if anc.iter().skip(1).any(|&x| const_get_at(c, x, id).is_some()) { None } else { const_get_inherit(c, k, id)? }
            }
        };
        let Some(v) = found else { return undefined(c, p) };
        if !is_class_or_module(v) { let s = String::from_utf8_lossy(&b).into_owned(); return type_err(c, format!("{} does not refer to class/module", s)); }
        k = v;
        first = false;
    }
    Ok(k)
}

/// rb_define_module_under / rb_define_class_under: an existing constant is reopened.
fn module_under(c: &mut Ctx, outer: Value, name: &str) -> Value {
    match const_get_at(c, outer, intern(name.as_bytes())) { Some(m) if is_class_or_module(m) => m, _ => define_module_under(c, outer, name) }
}
fn class_under(c: &mut Ctx, outer: Value, name: &str, sup: Value) -> Value {
    match const_get_at(c, outer, intern(name.as_bytes())) { Some(k) if is_class_or_module(k) => k, _ => define_class_under(c, outer, name, sup) }
}

fn libyaml_version(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(c.ary_new_from(&[Value::fix(0), Value::fix(2), Value::fix(5)])) }

/// Run by the first `require "psych.so"`.
pub fn init(c: &mut Ctx) -> RV {
    if C_PARSER.get(c) != NIL { return Ok(NIL); }
    let o = c.vm().c_object.get();
    let m = module_under(c, o, "Psych");
    M_PSYCH.set(c, m);
    define_singleton_cfunc(c, m, "libyaml_version", libyaml_version, 0);

    let p = class_under(c, m, "Parser", o);
    C_PARSER.set(c, p);
    class_set_alloc(c, p, AllocKind::None);
    for (n, v) in [("ANY", yp::ANY_ENCODING), ("UTF8", yp::UTF8_ENCODING), ("UTF16LE", yp::UTF16LE_ENCODING), ("UTF16BE", yp::UTF16BE_ENCODING)] { const_set(c, p, intern(n.as_bytes()), Value::fix(v as i64)); }
    let f = c.str_new(b"psych/syntax_error");
    require_feature(c, f, false)?;
    define_private_cfunc(c, p, "_native_parse", p_native_parse, 3);
    define_cfunc(c, p, "parse", p_parse, -1);
    define_cfunc(c, p, "mark", p_mark, 0);

    let h = class_under(c, m, "Handler", o);
    let e = class_under(c, m, "Emitter", h);
    C_EMITTER.set(c, e);
    class_set_alloc(c, e, AllocKind::None);
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, e, n, f, a);
    d(c, "initialize", e_initialize, -1); d(c, "start_stream", e_start_stream, 1); d(c, "end_stream", e_end_stream, 0);
    d(c, "start_document", e_start_document, 3); d(c, "end_document", e_end_document, 1); d(c, "scalar", e_scalar, 6);
    d(c, "start_sequence", e_start_sequence, 4); d(c, "end_sequence", e_end_sequence, 0); d(c, "start_mapping", e_start_mapping, 4);
    d(c, "end_mapping", e_end_mapping, 0); d(c, "alias", e_alias, 1); d(c, "canonical", e_canonical, 0); d(c, "canonical=", e_set_canonical, 1);
    d(c, "indentation", e_indentation, 0); d(c, "indentation=", e_set_indentation, 1); d(c, "line_width", e_line_width, 0); d(c, "line_width=", e_set_line_width, 1);

    let cl = class_under(c, m, "ClassLoader", o);
    let vis = module_under(c, m, "Visitors");
    let visitor = class_under(c, vis, "Visitor", o);
    let tr = class_under(c, vis, "ToRuby", visitor);
    define_private_cfunc(c, tr, "build_exception", tr_build_exception, 2);
    define_private_cfunc(c, cl, "path2class", cl_path2class, 1);
    class_under(c, vis, "YAMLTree", visitor);
    Ok(NIL)
}
