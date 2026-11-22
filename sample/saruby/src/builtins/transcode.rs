// String#encode / #encode! and Encoding::Converter on the engine in econv.rs (CRuby's
// transcode.c from str_transcode0 / transcode_loop down to econv_*), and the attributes of
// Encoding::InvalidByteSequenceError / UndefinedConversionError.
#![forbid(unsafe_code)]
use crate::array::*;
use crate::builtins::econv::{self as ev, Econv, Res};
use crate::builtins::encoding as en;
use crate::builtins::misc::REnc;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

/// Encoding id of a transcoder-side name (case-insensitive, aliases too).
pub fn enc_id(name: &str) -> Option<u8> { if name.is_empty() { None } else { en::find_enc(name.as_bytes()) } }
fn encobj(c: &Ctx, name: &str) -> Value { match enc_id(name) { Some(e) => en::encoding_value(c, e), None => NIL } }

/// rb_str_dump of a binary string.
pub fn dump_bin(v: &[u8]) -> String {
    let mut out = String::from("\"");
    for (i, &x) in v.iter().enumerate() {
        match x {
            b'"' => out.push_str("\\\""), b'\\' => out.push_str("\\\\"), b'\n' => out.push_str("\\n"), b'\t' => out.push_str("\\t"), b'\r' => out.push_str("\\r"),
            0x0c => out.push_str("\\f"), 0x0b => out.push_str("\\v"), 0x08 => out.push_str("\\b"), 0x07 => out.push_str("\\a"), 0x1b => out.push_str("\\e"),
            b'#' if matches!(v.get(i + 1), Some(b'$' | b'@' | b'{')) => out.push_str("\\#"),
            0x20..=0x7e => out.push(x as char),
            _ => out.push_str(&format!("\\x{:02X}", x)),
        }
    }
    out.push('"');
    out
}

// ---- exceptions ---------------------------------------------------------------------------------

fn set(e: Value, k: &[u8], v: Value) { ivar_set_raw(e, intern(k), v); }
fn attr(v: Value, k: &[u8]) -> Value { let x = ivar_get(v, intern(k)); if x == UNDEF { NIL } else { x } }

/// make_econv_exception: the exception for the last error, None if there was none.
pub fn econv_exception(c: &mut Ctx, ec: &Econv) -> Option<Value> {
    let le = &ec.last_error;
    let e = match le.result {
        Some(r @ (Res::InvalidByteSequence | Res::IncompleteInput)) => {
            let d = dump_bin(&le.error_bytes);
            let msg = if r == Res::IncompleteInput { format!("incomplete {} on {}", d, le.src) }
                      else if !le.readagain.is_empty() { format!("{} followed by {} on {}", d, dump_bin(&le.readagain), le.src) }
                      else { format!("{} on {}", d, le.src) };
            let e = make_exception(c.vm().c_invalid_byte_seq_error, &msg);
            let b = c.str_new_enc(&le.error_bytes, ENC_BINARY);
            set(e, b"__tc_bytes", b);
            let ra = if r == Res::IncompleteInput || le.readagain.is_empty() { NIL } else { c.str_new_enc(&le.readagain, ENC_BINARY) };
            set(e, b"__tc_again", ra);
            set(e, b"__tc_incomplete", Value::bool(r == Res::IncompleteInput));
            e
        }
        Some(Res::UndefinedConversion) => {
            let b = &le.error_bytes;
            let mut dumped = None;
            if le.src == "UTF-8" {
                if let Some(n) = en::utf8_precise(b) { if n == b.len() { dumped = Some(format!("U+{:04X}", decode_utf8(b, 0).0)); } }
            }
            let dumped = dumped.unwrap_or_else(|| dump_bin(b));
            let msg = if le.src == ec.sname && le.dst == ec.dname { format!("{} from {} to {}", dumped, le.src, le.dst) } else {
                let mut m = format!("{} to {} in conversion from {}", dumped, le.dst, ec.sname);
                for el in &ec.elems { if !ev::is_decorator(el.tc.tr) { m.push_str(" to "); m.push_str(el.tc.tr.dst); } }
                m
            };
            let e = make_exception(c.vm().c_undef_conv_error, &msg);
            let ch = c.str_new_enc(b, enc_id(le.src).unwrap_or(ENC_BINARY));
            set(e, b"__tc_char", ch);
            e
        }
        _ => return None,
    };
    let (sn, dn) = (c.str_new_enc(le.src.as_bytes(), ENC_BINARY), c.str_new_enc(le.dst.as_bytes(), ENC_BINARY));
    set(e, b"__tc_src_name", sn);
    set(e, b"__tc_dst_name", dn);
    let (se, de) = (encobj(c, le.src), encobj(c, le.dst));
    if se != NIL { set(e, b"__tc_src", se); }
    if de != NIL { set(e, b"__tc_dst", de); }
    Some(e)
}
pub fn exc_source_encoding(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(attr(slf, b"__tc_src")) }
pub fn exc_destination_encoding(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(attr(slf, b"__tc_dst")) }
pub fn exc_source_encoding_name(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(attr(slf, b"__tc_src_name")) }
pub fn exc_destination_encoding_name(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(attr(slf, b"__tc_dst_name")) }
pub fn exc_error_bytes(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(attr(slf, b"__tc_bytes")) }
pub fn exc_readagain_bytes(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(attr(slf, b"__tc_again")) }
pub fn exc_incomplete_input(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(attr(slf, b"__tc_incomplete")) }
pub fn exc_error_char(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(attr(slf, b"__tc_char")) }

/// econv_description
fn description(sname: &str, dname: &str, ecflags: u32, mut m: String) -> String {
    let mut has = false;
    if !sname.is_empty() || !dname.is_empty() {
        if sname.is_empty() { m.push_str(dname); } else if dname.is_empty() { m.push_str(sname); } else { m.push_str(&format!("{} to {}", sname, dname)); }
        has = true;
    }
    if ecflags & (ev::NEWLINE_DECORATOR_MASK | ev::XML_TEXT_DECORATOR | ev::XML_ATTR_CONTENT_DECORATOR | ev::XML_ATTR_QUOTE_DECORATOR) != 0 {
        if has { m.push_str(" with "); }
        let mut pre = "";
        for (f, n) in [(ev::UNIVERSAL_NEWLINE_DECORATOR, "universal_newline"), (ev::CRLF_NEWLINE_DECORATOR, "crlf_newline"), (ev::CR_NEWLINE_DECORATOR, "cr_newline"),
                       (ev::LF_NEWLINE_DECORATOR, "lf_newline"), (ev::XML_TEXT_DECORATOR, "xml_text"), (ev::XML_ATTR_CONTENT_DECORATOR, "xml_attr_content"), (ev::XML_ATTR_QUOTE_DECORATOR, "xml_attr_quote")] {
            if ecflags & f != 0 { m.push_str(pre); m.push_str(n); pre = ","; }
        }
        has = true;
    }
    if !has { m.push_str("no-conversion"); }
    m
}
/// rb_econv_open_exc
pub fn open_exc(c: &mut Ctx, sname: &str, dname: &str, ecflags: u32) -> Esc {
    let m = description(sname, dname, ecflags, "code converter not found (".into()) + ")";
    raise_msg(c, c.vm().c_converter_not_found_error, &m)
}

// ---- options ------------------------------------------------------------------------------------

/// The parts of an option hash the converter keeps (rb_econv_prepare_options' new hash).
#[derive(Clone, Copy, Default)]
pub struct Opts { pub replace: Option<Value>, pub fallback: Option<Value> }

fn sym_name(v: Value) -> String { String::from_utf8_lossy(crate::symbol::sym_bytes(v.as_sym())).into_owned() }
fn get(c: &mut Ctx, h: Value, k: &str) -> RV { crate::hash::hash_aref(c, h, sym_value(k)) }

/// econv_opts
pub fn econv_opts(c: &mut Ctx, h: Value, mut ecflags: u32) -> Result<u32, Esc> {
    let rep = sym_value("replace");
    let v = get(c, h, "invalid")?;
    if v == rep { ecflags |= ev::INVALID_REPLACE; } else if v != NIL { return arg_err(c, "unknown value for invalid character option"); }
    let v = get(c, h, "undef")?;
    if v == rep { ecflags |= ev::UNDEF_REPLACE; } else if v != NIL { return arg_err(c, "unknown value for undefined character option"); }
    if get(c, h, "replace")? != NIL && ecflags & ev::INVALID_REPLACE == 0 { ecflags |= ev::UNDEF_REPLACE; }
    let v = get(c, h, "xml")?;
    if v != NIL {
        if v == sym_value("text") { ecflags |= ev::XML_TEXT_DECORATOR | ev::UNDEF_HEX_CHARREF; }
        else if v == sym_value("attr") { ecflags |= ev::XML_ATTR_CONTENT_DECORATOR | ev::XML_ATTR_QUOTE_DECORATOR | ev::UNDEF_HEX_CHARREF; }
        else if v.is_sym() { return arg_err(c, format!("unexpected value for xml option: {}", sym_name(v))); }
        else { return arg_err(c, "unexpected value for xml option"); }
    }
    let mut nlflag = 0;
    let v = get(c, h, "newline")?;
    if v != NIL {
        nlflag = 2;
        ecflags &= !ev::NEWLINE_DECORATOR_MASK;
        if v == sym_value("universal") { ecflags |= ev::UNIVERSAL_NEWLINE_DECORATOR; }
        else if v == sym_value("crlf") { ecflags |= ev::CRLF_NEWLINE_DECORATOR; }
        else if v == sym_value("cr") { ecflags |= ev::CR_NEWLINE_DECORATOR; }
        else if v == sym_value("lf") { ecflags |= ev::LF_NEWLINE_DECORATOR; }
        else if v.is_sym() { return arg_err(c, format!("unexpected value for newline option: {}", sym_name(v))); }
        else { return arg_err(c, "unexpected value for newline option"); }
    }
    let mut setflags = 0;
    for (k, f) in [("universal_newline", ev::UNIVERSAL_NEWLINE_DECORATOR), ("crlf_newline", ev::CRLF_NEWLINE_DECORATOR), ("cr_newline", ev::CR_NEWLINE_DECORATOR), ("lf_newline", ev::LF_NEWLINE_DECORATOR)] {
        let v = get(c, h, k)?;
        if v.truthy() { setflags |= f; }
        if v != NIL { nlflag |= 1; }
    }
    match nlflag {
        1 => { ecflags &= !ev::NEWLINE_DECORATOR_MASK; ecflags |= setflags; }
        3 => if c.vm().verbose == TRUE { crate::builtins::kernel::warn_str(c, "warning: :newline option precedes other newline options\n"); },
        _ => {}
    }
    Ok(ecflags)
}

/// rb_econv_prepare_options
pub fn prepare_options(c: &mut Ctx, h: Value, ecflags: u32) -> Result<(u32, Opts), Esc> {
    let ecflags = econv_opts(c, h, ecflags)?;
    let mut o = Opts::default();
    let v = get(c, h, "replace")?;
    if v != NIL {
        let s = expect_str(c, v)?;
        if !en::is_valid(c.str_enc(s), c.str_bytes(s)) {
            let d = funcall(c, s, intern(b"dump"), &[], std::ptr::null())?;
            let d = String::from_utf8_lossy(c.str_bytes(d)).into_owned();
            return arg_err(c, format!("replacement string is broken: {} as {}", d, en::enc_name(c.str_enc(s))));
        }
        let f = c.str_copy(s);
        obj_set_flags(f, FL_FROZEN);
        o.replace = Some(f);
    }
    let v = get(c, h, "fallback")?;
    if v != NIL {
        match crate::hash::check_hash(c, v)? {
            Some(hh) => o.fallback = Some(hh),
            None => if is_proc(v) || is_method_obj(v) || respond_to(v, intern(b"[]"), false) { o.fallback = Some(v); },
        }
    }
    Ok((ecflags, o))
}

/// rb_econv_open_opts
pub fn open_opts(c: &Ctx, sname: &str, dname: &str, ecflags: u32, o: &Opts) -> Option<Econv> {
    let mut ec = Econv::open(sname, dname, ecflags)?;
    if let Some(r) = o.replace {
        let (b, e) = (c.str_bytes(r).to_vec(), en::enc_name(c.str_enc(r)));
        if !ec.set_replacement(&b, e) { return None; }
    }
    Some(ec)
}

// ---- transcode_loop / String#encode ---------------------------------------------------------------

/// transcode_loop: all of `src` converted; raises the conversion errors.
pub fn transcode_loop(c: &mut Ctx, src: &[u8], sname: &str, dname: &str, ecflags: u32, o: &Opts) -> Result<Vec<u8>, Esc> {
    let Some(mut ec) = open_opts(c, sname, dname, ecflags, o) else { return Err(open_exc(c, sname, dname, ecflags)) };
    let max_output = ec.last_tc.map(|i| ec.elems[i].tc.tr.max_out as usize).unwrap_or(1);
    let mut out = Vec::with_capacity(src.len() + 30);
    let mut cap = src.len() + 30;
    let mut ip = 0;
    loop {
        let ret = ec.convert(src, &mut ip, &mut out, cap, 0);
        if let (Some(fb), Res::UndefinedConversion) = (o.fallback, ret) {
            let le = &ec.last_error;
            let rep = c.str_new_enc(&le.error_bytes.clone(), enc_id(le.src).unwrap_or(ENC_BINARY));
            let r = if is_hash(fb) { crate::hash::hash_aref(c, fb, rep)? }
                    else if is_proc(fb) || is_method_obj(fb) { funcall(c, fb, intern(b"call"), &[rep], std::ptr::null())? }
                    else { funcall_public(c, fb, intern(b"[]"), &[rep], std::ptr::null())? };
            let r = if r != NIL && r != UNDEF { Some(expect_str(c, r)?) } else { None };
            if let Some(r) = r {
                let (b, e) = (c.str_bytes(r).to_vec(), en::enc_name(c.str_enc(r)));
                if !ec.insert_output(&b, e) { return arg_err(c, "too big fallback string"); }
                continue;
            }
        }
        match ret {
            Res::InvalidByteSequence | Res::IncompleteInput | Res::UndefinedConversion => {
                let e = econv_exception(c, &ec).unwrap();
                return Err(raise_exc(c, e));
            }
            Res::DestinationBufferFull => { cap = (out.len() + max_output) * 2; }
            _ => return Ok(out),
        }
    }
}

/// enc_arg: (name, encoding) of an Encoding or a name.
fn enc_arg(c: &mut Ctx, v: Value) -> Result<(String, Option<u8>), Esc> {
    if let Some(e) = to_encoding_index(c, v) { return Ok((en::enc_name(e).to_string(), Some(e))); }
    let s = expect_str(c, v)?;
    let b = c.str_bytes(s);
    if b.contains(&0) { return arg_err(c, "string contains null byte"); }
    Ok((String::from_utf8_lossy(b).into_owned(), None))
}
/// rb_to_encoding_index
fn to_encoding_index(c: &mut Ctx, v: Value) -> Option<u8> {
    if let Some(e) = c.data::<REnc>(v) { return Some(e.id); }
    if !is_string(v) { return None; }
    if !en::ascii_compat(c.str_enc(v)) { return None; }
    en::find_enc(c.str_bytes(v))
}

/// str_transcode0: Ok(None) = unchanged (dup), Ok(Some((bytes or None = same bytes, enc))).
fn str_transcode0(c: &mut Ctx, slf: Value, pos: &[Value], mut ecflags: u32, o: &Opts) -> Result<Option<(Option<Vec<u8>>, u8)>, Esc> {
    if pos.len() > 2 { return argc_err(c, pos.len(), 0, 2); }
    let mut explicit_invalid = true;
    let arg1 = if pos.is_empty() {
        let a = match en::default_internal() { Some(e) => en::encoding_value(c, e), None => {
            if ecflags == 0 { return Ok(None); }
            en::encoding_value(c, c.str_enc(slf))
        } };
        if ecflags & ev::INVALID_MASK == 0 { explicit_invalid = false; }
        ecflags |= ev::INVALID_REPLACE | ev::UNDEF_REPLACE;
        a
    } else { pos[0] };
    let arg2 = pos.get(1).copied();
    let (dname, denc) = enc_arg(c, arg1)?;
    let (mut sname, mut senc) = match arg2 { None => { let e = c.str_enc(slf); (en::enc_name(e).to_string(), Some(e)) } Some(v) => enc_arg(c, v)? };
    let dencidx = denc;
    let mut src = c.str_bytes(slf).to_vec();
    let (mut sn, mut dn) = (sname.clone(), dname.clone());
    if ecflags & (ev::NEWLINE_DECORATOR_MASK | ev::XML_TEXT_DECORATOR | ev::XML_ATTR_CONTENT_DECORATOR | ev::XML_ATTR_QUOTE_DECORATOR) == 0 {
        if senc.is_some() && senc == denc {
            if ecflags & ev::INVALID_MASK != 0 && explicit_invalid {
                let rep = o.replace.unwrap_or(NIL);
                let s2 = c.str_copy_enc(slf, senc.unwrap());
                let r = crate::builtins::string_methods::scrub_bytes(c, s2, rep)?;
                return Ok(Some((r, senc.unwrap())));
            }
            return Ok(if arg2.is_none() { None } else { Some((None, senc.unwrap())) });
        }
        if let (Some(s), Some(d)) = (senc, denc) {
            if en::ascii_compat(s) && en::ascii_compat(d) && is_ascii(&src) { return Ok(Some((None, d))); }
        }
        if ev::enc_eq(&sname, &dname) { return Ok(if arg2.is_none() { None } else { denc.map(|d| (None, d)) }); }
    } else {
        if let (Some(s), Some(d)) = (senc, denc) {
            if !en::ascii_compat(s) && !en::ascii_compat(d) {
                src = transcode_loop(c, &src, &sname, "UTF-8", 0, &Opts::default()).unwrap_or(src);
                senc = Some(ENC_UTF8);
                sname = "UTF-8".into();
                sn = sname.clone();
            }
        }
        if ev::enc_eq(&sname, &dname) { sn = String::new(); dn = String::new(); }
    }
    let out = transcode_loop(c, &src, &sn, &dn, ecflags, o)?;
    let _ = senc;
    Ok(Some((Some(out), dencidx.unwrap_or(ENC_BINARY))))
}

/// (positional args, ecflags, opts) of an encode call.
fn encode_args<'a>(c: &mut Ctx, a: &'a [Value]) -> Result<(&'a [Value], u32, Opts), Esc> {
    match kw_arg(a) {
        Some(h) => { let (f, o) = prepare_options(c, h, 0)?; Ok((&a[..a.len() - 1], f, o)) }
        None => Ok((a, 0, Opts::default())),
    }
}
pub fn s_encode(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, f, o) = encode_args(c, a)?;
    let r = str_transcode0(c, slf, pos, f, &o)?;
    let v = match r {
        None => c.str_copy(slf),
        Some((None, e)) => c.str_copy_enc(slf, e),
        Some((Some(b), e)) => c.str_from_vec_enc(b, e),
    };
    let k = real_class_of(slf);
    if k != c.vm().c_string { obj_set_klass(v, k); }
    Ok(v)
}
pub fn s_encode_bang(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_frozen(c, slf)?;
    let (pos, f, o) = encode_args(c, a)?;
    match str_transcode0(c, slf, pos, f, &o)? {
        None => {}
        Some((None, e)) => c.rstring_mut(slf).enc = e,
        Some((Some(b), e)) => { let r = c.rstring_mut(slf); r.b = b.into(); r.enc = e; }
    }
    Ok(slf)
}

/// Convert bytes between two encodings with flags/options (for IO, JSON, ...): CRuby's
/// rb_str_encode path without the String-level shortcuts.
pub fn convert_bytes(c: &mut Ctx, src: &[u8], from: u8, to: u8, ecflags: u32, o: &Opts) -> Result<Vec<u8>, Esc> {
    let (mut sn, mut dn) = (en::enc_name(from).to_string(), en::enc_name(to).to_string());
    let deco = ecflags & ev::DECORATOR_MASK != 0;
    if from == to {
        if !deco {
            if ecflags & ev::INVALID_MASK != 0 {
                let s = c.str_new_enc(src, from);
                return Ok(crate::builtins::string_methods::scrub_bytes(c, s, o.replace.unwrap_or(NIL))?.unwrap_or_else(|| src.to_vec()));
            }
            return Ok(src.to_vec());
        }
        sn.clear(); dn.clear();
    }
    transcode_loop(c, src, &sn, &dn, ecflags, o)
}

// ---- Encoding::Converter ------------------------------------------------------------------------

pub struct REconv { ec: Option<Econv>, senc: Option<u8>, denc: Option<u8> }
impl RData for REconv {
    fn visit(&mut self, _m: &mut crate::gc::Visitor) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let k = c.vm().c_econv;
    if k != NIL && crate::builtins::inherits(klass, k) { return Ok(Some(c.alloc(klass, Body::Data(Box::new(REconv { ec: None, senc: None, denc: None }))))); }
    Ok(None)
}
/// check_econv
fn ecm(c: &mut Ctx, v: Value) -> Result<&mut REconv, Esc> {
    if c.data::<REconv>(v).is_none_or(|r| r.ec.is_none()) { return type_err(c, "uninitialized encoding converter"); }
    Ok(c.data_mut::<REconv>(v).unwrap())
}
fn ec_of(c: &mut Ctx, v: Value) -> Result<&mut Econv, Esc> { Ok(ecm(c, v)?.ec.as_mut().unwrap()) }

fn to_i32(c: &mut Ctx, v: Value) -> Result<i64, Esc> { crate::builtins::numeric_methods::num_to_long(c, v) }

/// econv_args: (sname, dname, senc, denc, ecflags, opts)
#[allow(clippy::type_complexity)]
fn econv_args(c: &mut Ctx, a: &[Value]) -> Result<(String, String, Option<u8>, Option<u8>, u32, Opts), Esc> {
    let kw = kw_arg(a);
    let pos = if kw.is_some() { &a[..a.len() - 1] } else { a };
    if pos.len() < 2 || pos.len() > 3 { return argc_err(c, pos.len(), 2, 3); }
    let (ecflags, o) = if pos.len() == 3 && pos[2] != NIL {
        if kw.is_some() { return argc_err(c, pos.len() + 1, 2, 3); }
        let f = to_int_val(c, pos[2])?;
        (f as u32, Opts::default())
    } else if let Some(h) = kw { prepare_options(c, h, 0)? } else { (0, Opts::default()) };
    let one = |c: &mut Ctx, v: Value| -> Result<(String, Option<u8>), Esc> {
        match to_encoding_index(c, v) { Some(e) => Ok((en::enc_name(e).to_string(), Some(e))), None => { let s = expect_str(c, v)?; Ok((String::from_utf8_lossy(c.str_bytes(s)).into_owned(), None)) } }
    };
    let (sn, se) = one(c, pos[0])?;
    let (dn, de) = one(c, pos[1])?;
    Ok((sn, dn, se, de, ecflags, o))
}
fn to_int_val(c: &mut Ctx, v: Value) -> Result<i64, Esc> {
    let i = if v.is_fixnum() { v } else { funcall(c, v, intern(b"to_int"), &[], std::ptr::null())? };
    if !i.is_fixnum() { let s = crate::builtins::kernel::inspect_str(c, v)?; return type_err(c, format!("can't convert {} to Integer", s)); }
    Ok(i.as_fix())
}

fn ec_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if c.data::<REconv>(slf).is_some_and(|r| r.ec.is_some()) { return type_err(c, "already initialized"); }
    let convpath = if a.len() == 1 { crate::array::check_array(c, a[0])? } else { None };
    let (ec, senc, denc) = if let Some(cp) = convpath {
        let mut ec = Econv::empty();
        let (mut senc, mut denc) = (None, None);
        let mut sname = String::new();
        let mut dname = String::new();
        let mut first = true;
        let n = c.ary(cp).len();
        for i in 0..n {
            let elt = c.ary(cp).get(i).copied().unwrap_or(NIL);
            let (sn, dn, se, de) = if let Some(pair) = crate::array::check_array(c, elt)? {
                if c.ary(pair).len() != 2 { return arg_err(c, "not a 2-element array in convpath"); }
                let (p0, p1) = (c.ary(pair)[0], c.ary(pair)[1]);
                let (sn, se) = enc_arg(c, p0)?;
                let (dn, de) = enc_arg(c, p1)?;
                (sn, dn, se, de)
            } else {
                let s = expect_str(c, elt)?;
                (String::new(), String::from_utf8_lossy(c.str_bytes(s)).into_owned(), None, None)
            };
            if sn.is_empty() {
                let n = ec.elems.len();
                if !ec.add_converter("", &dn, n) { return arg_err(c, format!("decoration failed: {}", dn)); }
            } else {
                let j = ec.elems.len();
                let path = ev::search_path(&sn, &dn);
                let ok = match path { Some(p) => p.iter().all(|tr| { let n = ec.elems.len(); ec.add_converter(tr.src, tr.dst, n) }), None => false };
                if !ok { return arg_err(c, format!("adding conversion failed: {} to {}", sn, dn)); }
                if first { first = false; senc = se; sname = ec.elems[j].tc.tr.src.to_string(); }
                denc = de;
                dname = ec.elems[ec.elems.len() - 1].tc.tr.dst.to_string();
            }
        }
        if first { senc = None; denc = None; }
        ec.sname = sname; ec.dname = dname;
        (ec, senc, denc)
    } else {
        let (sn, dn, se, de, f, o) = econv_args(c, a)?;
        match open_opts(c, &sn, &dn, f, &o) { Some(ec) => (ec, se, de), None => return Err(open_exc(c, &sn, &dn, f)) }
    };
    let (senc, denc) = if ec.sname.is_empty() && ec.dname.is_empty() { (senc, denc) } else { (senc.or(enc_id(&ec.sname)), denc.or(enc_id(&ec.dname))) };
    *c.data_mut::<REconv>(slf).unwrap() = REconv { ec: Some(ec), senc, denc };
    Ok(slf)
}

fn ec_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let cname = class_path(real_class_of(slf));
    let s = match c.data::<REconv>(slf).and_then(|r| r.ec.as_ref()) {
        None => format!("#<{}: uninitialized>", cname),
        Some(ec) => description(&ec.sname, &ec.dname, ec.flags, format!("#<{}: ", cname)) + ">",
    };
    Ok(c.str_new_enc(s.as_bytes(), ENC_BINARY))
}
fn ec_convpath(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let items: Vec<(&'static str, &'static str)> = ec_of(c, slf)?.elems.iter().map(|e| (e.tc.tr.src, e.tc.tr.dst)).collect();
    let out = c.ary_new();
    for (s, d) in items {
        let v = if s.is_empty() { c.str_new_enc(d.as_bytes(), ENC_BINARY) } else { let (a, b) = (encobj(c, s), encobj(c, d)); c.ary_from_vec(vec![a, b]) };
        c.ary_push(out, v);
    }
    Ok(out)
}
fn ec_source_encoding(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let e = ecm(c, slf)?.senc; Ok(e.map(|e| en::encoding_value(c, e)).unwrap_or(NIL)) }
fn ec_destination_encoding(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let e = ecm(c, slf)?.denc; Ok(e.map(|e| en::encoding_value(c, e)).unwrap_or(NIL)) }

fn res_sym(r: Res) -> Value {
    sym_value(match r {
        Res::InvalidByteSequence => "invalid_byte_sequence", Res::IncompleteInput => "incomplete_input", Res::UndefinedConversion => "undefined_conversion",
        Res::DestinationBufferFull => "destination_buffer_full", Res::SourceBufferEmpty => "source_buffer_empty", Res::Finished => "finished", Res::AfterOutput => "after_output",
    })
}

/// econv_primitive_convert
fn ec_primitive_convert(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let kw = kw_arg(a);
    let pos = if kw.is_some() { &a[..a.len() - 1] } else { a };
    if pos.len() < 2 || pos.len() > 5 { return argc_err(c, a.len(), 2, 5); }
    let (input, output) = (pos[0], pos[1]);
    let off_v = pos.get(2).copied().unwrap_or(NIL);
    let size_v = pos.get(3).copied().unwrap_or(NIL);
    let flags_v = pos.get(4).copied().unwrap_or(NIL);
    ecm(c, slf)?;
    let mut off: i64 = if off_v == NIL { 0 } else { to_i32(c, off_v)? };
    let mut size: i64 = if size_v == NIL { 0 } else { to_i32(c, size_v)? };
    let flags = if flags_v != NIL {
        if kw.is_some() { return argc_err(c, pos.len() + 1, 2, 5); }
        to_int_val(c, flags_v)? as u32
    } else if let Some(h) = kw {
        let mut f = 0;
        if get(c, h, "partial_input")?.truthy() { f |= ev::PARTIAL_INPUT; }
        if get(c, h, "after_output")?.truthy() { f |= ev::AFTER_OUTPUT; }
        f
    } else { 0 };
    let output = expect_str(c, output)?;
    let input = if input != NIL { Some(expect_str(c, input)?) } else { None };
    c.str_modify(output)?;
    if size_v == NIL {
        size = (c.str_len(output).max(23)) as i64;
        if let Some(i) = input { size = size.max(c.str_len(i) as i64); }
    }
    let mut off_nil = off_v == NIL;
    let res = loop {
        if off_nil { off = c.str_len(output) as i64; }
        if off < 0 { return arg_err(c, "negative output_byteoffset"); }
        if (c.str_len(output) as i64) < off { return arg_err(c, "output_byteoffset too big"); }
        if size < 0 { return arg_err(c, "negative output_bytesize"); }
        if off.checked_add(size).is_none() { return arg_err(c, "output_byteoffset+output_bytesize too big"); }
        let src: Vec<u8> = match input { Some(i) => c.str_bytes(i).to_vec(), None => vec![] };
        let mut out: Vec<u8> = c.str_bytes(output)[..off as usize].to_vec();
        let stop = off as usize + size as usize;
        let mut ip = 0;
        let res = ec_of(c, slf)?.convert(&src, &mut ip, &mut out, stop, flags);
        c.rstring_mut(output).b = out.into();
        if let Some(i) = input { if ip > 0 { c.str_modify(i)?.b.remove_front(ip); } }
        if size_v == NIL && res == Res::DestinationBufferFull { size = size.saturating_mul(2); off_nil = true; continue; }
        break res;
    };
    if let Some(d) = ecm(c, slf)?.denc { c.rstring_mut(output).enc = d; }
    Ok(res_sym(res))
}

fn ec_convert(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let s = expect_str(c, a[0])?;
    ecm(c, slf)?;
    let src = c.str_copy(s);
    let dst = c.str_new_enc(b"", ENC_BINARY);
    let r = ec_primitive_convert(c, slf, &[src, dst, NIL, NIL, Value::fix(ev::PARTIAL_INPUT as i64)], std::ptr::null())?;
    if r == res_sym(Res::InvalidByteSequence) || r == res_sym(Res::UndefinedConversion) || r == res_sym(Res::IncompleteInput) {
        let ec = ec_of(c, slf)?.clone();
        let e = econv_exception(c, &ec).unwrap();
        return Err(raise_exc(c, e));
    }
    if r == res_sym(Res::Finished) { return arg_err(c, "converter already finished"); }
    Ok(dst)
}
fn ec_finish(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    ecm(c, slf)?;
    let dst = c.str_new_enc(b"", ENC_BINARY);
    let r = ec_primitive_convert(c, slf, &[NIL, dst, NIL, NIL, Value::fix(0)], std::ptr::null())?;
    if r == res_sym(Res::InvalidByteSequence) || r == res_sym(Res::UndefinedConversion) || r == res_sym(Res::IncompleteInput) {
        let ec = ec_of(c, slf)?.clone();
        let e = econv_exception(c, &ec).unwrap();
        return Err(raise_exc(c, e));
    }
    Ok(dst)
}
fn ec_primitive_errinfo(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let le = ec_of(c, slf)?.last_error.clone();
    let r = res_sym(le.result.unwrap_or(Res::SourceBufferEmpty));
    let s = if le.error_tc.is_none() { NIL } else { c.str_new_enc(le.src.as_bytes(), ENC_BINARY) };
    let d = if le.error_tc.is_none() { NIL } else { c.str_new_enc(le.dst.as_bytes(), ENC_BINARY) };
    let (eb, ra) = if le.error_tc.is_some() { (c.str_new_enc(&le.error_bytes, ENC_BINARY), c.str_new_enc(&le.readagain, ENC_BINARY)) } else { (NIL, NIL) };
    Ok(c.ary_from_vec(vec![r, s, d, eb, ra]))
}
fn ec_insert_output(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let s = expect_str(c, a[0])?;
    let ins = ec_of(c, slf)?.insert_encoding();
    let to = enc_id(ins).unwrap_or(ENC_BINARY);
    let from = c.str_enc(s);
    let b = c.str_bytes(s).to_vec();
    let conv = if from == to || ins.is_empty() || en::ascii_compat(from) && en::ascii_compat(to) && is_ascii(&b) { b }
               else { transcode_loop(c, &b, en::enc_name(from), ins, 0, &Opts::default())? };
    if !ec_of(c, slf)?.insert_output(&conv, ins) { return arg_err(c, "too big string"); }
    Ok(NIL)
}
fn ec_putback(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    let max = match a.first() { Some(&m) if m != NIL => Some(to_i32(c, m)?), _ => None };
    let ec = ec_of(c, slf)?;
    let pb = ec.putbackable();
    let n = match max { None => pb, Some(m) => (m.max(0) as usize).min(pb) };
    let v = ec.putback(n);
    let senc = ecm(c, slf)?.senc;
    Ok(c.str_from_vec_enc(v, senc.unwrap_or(ENC_BINARY)))
}
fn ec_last_error(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let ec = ec_of(c, slf)?.clone();
    Ok(econv_exception(c, &ec).unwrap_or(NIL))
}
fn ec_replacement(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let (b, e) = ec_of(c, slf)?.replacement();
    Ok(c.str_from_vec_enc(b, enc_id(&e).unwrap_or(ENC_BINARY)))
}
fn ec_set_replacement(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let s = expect_str(c, a[0])?;
    let (b, e) = (c.str_bytes(s).to_vec(), en::enc_name(c.str_enc(s)));
    if !ec_of(c, slf)?.set_replacement(&b, e) { return err(c, c.vm().c_undef_conv_error, "replacement character setup failed"); }
    Ok(a[0])
}
fn ec_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    ecm(c, slf)?;
    let Some(o) = c.data::<REconv>(a[0]) else { return Ok(NIL) };
    let Some(e2) = o.ec.as_ref() else { return Ok(FALSE) };
    let e1 = c.data::<REconv>(slf).unwrap().ec.as_ref().unwrap();
    let same = e1.sname == e2.sname && e1.dname == e2.dname && e1.flags == e2.flags && e1.replacement == e2.replacement
        && e1.elems.len() == e2.elems.len() && e1.elems.iter().zip(&e2.elems).all(|(x, y)| std::ptr::eq(x.tc.tr, y.tc.tr));
    Ok(Value::bool(same))
}
fn ec_s_asciicompat_encoding(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (n, _) = enc_arg(c, a[0])?;
    Ok(match ev::asciicompat_encoding(&n) { Some(r) => encobj(c, r), None => NIL })
}
fn ec_s_search_convpath(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (sn, dn, _, _, f, _) = econv_args(c, a)?;
    let Some(path) = ev::search_path(&sn, &dn) else { return Err(open_exc(c, &sn, &dn, f)) };
    let Some(decos) = ev::decorator_names(f) else { return Err(open_exc(c, &sn, &dn, f)) };
    let mut items: Vec<Value> = vec![];
    for tr in &path { let (x, y) = (encobj(c, tr.src), encobj(c, tr.dst)); let p = c.ary_from_vec(vec![x, y]); items.push(p); }
    // decorate_convpath: decorators go before a last ASCII-incompatible encoder
    let mut n = items.len();
    let last = if !decos.is_empty() && path.last().is_some_and(|t| t.asciicompat == 2) { n -= 1; items.pop() } else { None };
    let _ = n;
    for d in decos { let s = c.str_new_enc(d.as_bytes(), ENC_BINARY); items.push(s); }
    if let Some(l) = last { items.push(l); }
    Ok(c.ary_from_vec(items))
}

pub fn init(vmr: &mut Vm) {
    let k = define_class_under(vmr.c_encoding, "Converter", vmr.c_object);
    class_set_alloc(k, AllocKind::None);
    vmr.c_econv = k;
    define_singleton_cfunc(k, "asciicompat_encoding", ec_s_asciicompat_encoding, 1);
    define_singleton_cfunc(k, "search_convpath", ec_s_search_convpath, -1);
    define_cfunc(k, "initialize", ec_initialize, -1);
    define_cfunc(k, "inspect", ec_inspect, 0);
    define_cfunc(k, "convpath", ec_convpath, 0);
    define_cfunc(k, "source_encoding", ec_source_encoding, 0);
    define_cfunc(k, "destination_encoding", ec_destination_encoding, 0);
    define_cfunc(k, "primitive_convert", ec_primitive_convert, -1);
    define_cfunc(k, "convert", ec_convert, 1);
    define_cfunc(k, "finish", ec_finish, 0);
    define_cfunc(k, "primitive_errinfo", ec_primitive_errinfo, 0);
    define_cfunc(k, "insert_output", ec_insert_output, 1);
    define_cfunc(k, "putback", ec_putback, -1);
    define_cfunc(k, "last_error", ec_last_error, 0);
    define_cfunc(k, "replacement", ec_replacement, 0);
    define_cfunc(k, "replacement=", ec_set_replacement, 1);
    define_cfunc(k, "==", ec_eq, 1);
    for (n, v) in [("INVALID_MASK", ev::INVALID_MASK), ("INVALID_REPLACE", ev::INVALID_REPLACE), ("UNDEF_MASK", ev::UNDEF_MASK), ("UNDEF_REPLACE", ev::UNDEF_REPLACE),
                   ("UNDEF_HEX_CHARREF", ev::UNDEF_HEX_CHARREF), ("PARTIAL_INPUT", ev::PARTIAL_INPUT), ("AFTER_OUTPUT", ev::AFTER_OUTPUT),
                   ("UNIVERSAL_NEWLINE_DECORATOR", ev::UNIVERSAL_NEWLINE_DECORATOR), ("LF_NEWLINE_DECORATOR", ev::LF_NEWLINE_DECORATOR),
                   ("CRLF_NEWLINE_DECORATOR", ev::CRLF_NEWLINE_DECORATOR), ("CR_NEWLINE_DECORATOR", ev::CR_NEWLINE_DECORATOR),
                   ("XML_TEXT_DECORATOR", ev::XML_TEXT_DECORATOR), ("XML_ATTR_CONTENT_DECORATOR", ev::XML_ATTR_CONTENT_DECORATOR),
                   ("XML_ATTR_QUOTE_DECORATOR", ev::XML_ATTR_QUOTE_DECORATOR)] {
        const_set(k, intern(n.as_bytes()), Value::fix(v as i64));
    }
}
