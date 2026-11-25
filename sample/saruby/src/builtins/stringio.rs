// StringIO 3.2.0: a port of ext/stringio/stringio.c, with IO::generic_readable / IO::generic_writable.
// As in CRuby, a StringIO holds its stream state in a shared struct (`SState`, here a hidden Data
// object) so that dup / initialize_copy share position and mode; the readable / writable bits that
// close_read / close_write clear are per object (`RStrIO`).
#![forbid(unsafe_code)]
use crate::array::*;
use crate::builtins::encoding as en;
use crate::builtins::kernel::inspect_str;
use crate::builtins::numeric_methods::num_to_long;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

const FM_R: u32 = 1;
const FM_W: u32 = 2;
const FM_RW: u32 = 3;
const FM_APPEND: u32 = 0x40;
const FM_TRUNC: u32 = 0x800;
const FM_BOM: u32 = 0x100000;

#[derive(Clone, Copy)]
pub struct SState { string: Value, enc: Option<u8>, pos: i64, lineno: i64, flags: u32 }
impl RData for SState {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.string); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
/// The StringIO object: its shared state (NIL until initialize) and its own open bits.
pub struct RStrIO { inner: Value, rd: bool, wr: bool }
impl RData for RStrIO {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.inner); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

fn nb() -> Blk<'static> { Blk::NONE }
fn hidden_mark() -> Sym { intern(b"__strio") }
fn errno_exc(c: &mut Ctx, name: &str, msg: Option<&str>) -> Esc {
    let no = if name == "EINVAL" { 22 } else { 13 };
    let k = crate::builtins::exception::errno_class(c, no);
    let base = crate::builtins::exception::errno_text(Value::fix(no));
    let m = match msg { Some(m) => format!("{} - {}", base, m), None => base };
    let e = make_exception(c, k, &m);
    ivar_set_raw(c, e, s::errno, Value::fix(no));
    raise_exc(c, e)
}
fn error_inval<T>(c: &mut Ctx, msg: Option<&str>) -> Result<T, Esc> { Err(errno_exc(c, "EINVAL", msg)) }
fn eof_error<T>(c: &mut Ctx) -> Result<T, Esc> { err(c, c.vm().c_eof_error.get(), "end of file reached") }

/// check_strio: the object's payload, or TypeError.
fn strio_parts(c: &mut Ctx, v: Value) -> Result<(Value, bool, bool), Esc> {
    match c.data::<RStrIO>(v) {
        Some(o) => Ok((o.inner, o.rd, o.wr)),
        None => type_err(c, format!("wrong argument type {} (expected strio)", class_path(c, real_class_of(c, v)))),
    }
}
fn obj_mut(c: &mut Ctx, v: Value) -> &mut RStrIO { c.data_mut::<RStrIO>(v).expect("a StringIO") }
/// get_strio (StringIO(obj)): frozen check, type check, initialized.
fn strio(c: &mut Ctx, v: Value) -> Result<SState, Esc> {
    check_frozen(c, v)?;
    let (inner, _, _) = strio_parts(c, v)?;
    if inner == NIL { return err(c, c.vm().c_io_error.get(), "uninitialized stream"); }
    Ok(*c.data::<SState>(inner).expect("strio state"))
}
fn st_mut(c: &mut Ctx, v: Value) -> &mut SState {
    let inner = c.data::<RStrIO>(v).expect("a StringIO").inner;
    c.data_mut::<SState>(inner).expect("strio state")
}
fn mode_set(c: &mut Ctx, v: Value, bit: u32) -> bool {
    let (inner, rd, wr) = match c.data::<RStrIO>(v) { Some(o) => (o.inner, o.rd, o.wr), None => return false };
    let objbit = if bit == FM_R { rd } else { wr };
    objbit && inner != NIL && c.data::<SState>(inner).is_some_and(|s| s.flags & bit != 0)
}
fn readable(c: &mut Ctx, v: Value) -> Result<SState, Esc> {
    let p = strio(c, v)?;
    if !mode_set(c, v, FM_R) { return err(c, c.vm().c_io_error.get(), "not opened for reading"); }
    Ok(p)
}
fn writable(c: &mut Ctx, v: Value) -> Result<SState, Esc> {
    let p = strio(c, v)?;
    if !mode_set(c, v, FM_W) { return err(c, c.vm().c_io_error.get(), "not opened for writing"); }
    Ok(p)
}
fn closed(c: &mut Ctx, v: Value) -> bool { !mode_set(c, v, FM_R) && !mode_set(c, v, FM_W) }
fn check_modifiable(c: &mut Ctx, p: &SState) -> Result<(), Esc> {
    if p.string != NIL && is_frozen(p.string) { return err(c, c.vm().c_io_error.get(), "not modifiable string"); }
    Ok(())
}
fn slen(c: &Ctx, p: &SState) -> i64 { if p.string == NIL { 0 } else { c.str_bytes(p.string).len() as i64 } }
fn outside(c: &Ctx, p: &SState, pos: i64) -> bool { p.string == NIL || pos >= slen(c, p) }
fn eos(c: &Ctx, p: &SState) -> bool { outside(c, p, p.pos) }
fn get_enc(c: &Ctx, p: &SState) -> Option<u8> { p.enc.or_else(|| if p.string != NIL { Some(c.str_enc(p.string)) } else { None }) }
fn enc_or_default(e: Option<u8>) -> u8 { e.unwrap_or_else(en::default_external) }
/// strio_substr
fn substr(c: &mut Ctx, p: &SState, pos: i64, len: i64, enc: u8) -> Value {
    let rlen = slen(c, p) - pos;
    let len = len.min(rlen).max(0);
    if len == 0 { return c.str_new_enc(b"", enc); }
    let b = c.str_bytes(p.string)[pos as usize..(pos + len) as usize].to_vec();
    c.str_from_vec_enc(b, enc)
}
fn is_7bit(c: &Ctx, s: Value) -> bool { en::ascii_compat(c.str_enc(s)) && is_ascii(c.str_bytes(s)) }
/// rb_str_conv_enc: `s` converted to `to`, or `s` itself when that fails.
fn conv_enc(c: &mut Ctx, s: Value, from: u8, to: u8) -> Value {
    if from == to { return s; }
    if is_7bit(c, s) && en::ascii_compat(to) { let b = c.str_bytes(s).to_vec(); return c.str_from_vec_enc(b, to); }
    match crate::builtins::econv::convert_string(en::enc_name(from), en::enc_name(to), c.str_bytes(s)) {
        Some(b) => c.str_from_vec_enc(b, to),
        None => s,
    }
}
/// rb_enc_cr_str_buf_cat of `data` (in `denc`) onto the stream's String.
fn cat_enc(c: &mut Ctx, target: Value, data: &[u8], denc: u8) -> Result<(), Esc> {
    let tenc = c.str_enc(target);
    let mut res = tenc;
    if tenc != denc {
        let t7 = is_7bit(c, target);
        let d7 = en::ascii_compat(denc) && is_ascii(data);
        if !t7 && !d7 { return Err(en::compat_error(c, tenc, denc)); }
        if t7 && !d7 { res = denc; }
    }
    let r = c.str_modify(target)?;
    r.b.extend_from_slice(data);
    r.enc = res;
    Ok(())
}

pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let mut p = klass;
    while p != NIL && !ivar_defined(c, p, hidden_mark()) {
        let mut s2 = c.class(p).superclass;
        while s2 != NIL && c.class(s2).is_module { s2 = c.class(s2).superclass; }
        p = s2;
    }
    if p == NIL { return Ok(None); }
    Ok(Some(c.alloc(klass, Body::Data(Box::new(RStrIO { inner: NIL, rd: false, wr: false })))))
}
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if c.data::<RStrIO>(v).is_none() { return Ok(None); }
    Ok(Some(c.alloc(real_class_of(c, v), Body::Data(Box::new(RStrIO { inner: NIL, rd: false, wr: false })))))
}
fn new_state(c: &mut Ctx) -> Value {
    let k = c.vm().c_object.get();
    c.alloc(k, Body::Data(Box::new(SState { string: NIL, enc: None, pos: 0, lineno: 0, flags: 0 })))
}

// ---- construction -----------------------------------------------------------------------------

fn sio_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (inner, _, _) = strio_parts(c, slf)?;
    if inner == NIL { let n = new_state(c); obj_mut(c, slf).inner = n; }
    strio_init(c, slf, a)
}
/// detect_bom: (encoding, BOM length)
fn detect_bom(b: &[u8]) -> Option<(u8, i64)> {
    let f = |n: &str| en::find_enc(n.as_bytes());
    match b {
        [0xef, 0xbb, 0xbf, ..] => Some((ENC_UTF8, 3)),
        [0xfe, 0xff, ..] => Some((f("UTF-16BE")?, 2)),
        [0xff, 0xfe, 0, 0, ..] => Some((f("UTF-32LE")?, 4)),
        [0xff, 0xfe, ..] => Some((f("UTF-16LE")?, 2)),
        [0, 0, 0xfe, 0xff, ..] => Some((f("UTF-32BE")?, 4)),
        _ => None,
    }
}
fn set_encoding_by_bom(c: &mut Ctx, slf: Value) -> Option<u8> {
    let p = *st_mut(c, slf);
    let found = if p.string == NIL { None } else { detect_bom(c.str_bytes(p.string)) };
    if let Some((e, l)) = found {
        st_mut(c, slf).pos = l;
        if p.flags & FM_W != 0 { c.rstring_mut(p.string).enc = e; }
    }
    st_mut(c, slf).enc = found.map(|x| x.0);
    found.map(|x| x.0)
}
fn strio_init(c: &mut Ctx, slf: Value, a: &[Value]) -> RV {
    let kw = kw_arg(c, a);
    let pos: &[Value] = if kw.is_some() { &a[..a.len() - 1] } else { a };
    check_argc(c, pos, 0, 2)?;
    let mut string = pos.first().copied().unwrap_or(NIL);
    let vmode = pos.get(1).copied().unwrap_or(NIL);
    let om = crate::builtins::io::parse_mode(c, &[vmode], kw)?;
    let mode_given = vmode != NIL || match kw { Some(h) => crate::hash::hash_get_opt(c, h, sym_value("mode"))?.is_some_and(|m| m != NIL), None => false };
    let acc = om.flags & 3;
    let mut flags = if acc == 0 { FM_R } else if acc == 1 { FM_W } else { FM_RW };
    if om.flags & 0o2000 != 0 { flags |= FM_APPEND; }
    if om.flags & 0o1000 != 0 { flags |= FM_TRUNC; }
    if om.bom { flags |= FM_BOM; }
    let conv_enc = match om.encs { Some((e, _)) => e, None if om.bin => Some(ENC_BINARY), None => en::default_internal().filter(|&i| i != en::default_external()) };
    if string != NIL { string = expect_str(c, string)?; }
    else if pos.is_empty() { string = c.str_new_enc(b"", en::default_external()); }
    if string != NIL && is_frozen(string) {
        if flags & FM_W != 0 { return Err(errno_exc(c, "EACCES", None)); }
    } else if !mode_given { flags |= FM_W; }
    if string != NIL && flags & FM_TRUNC != 0 { c.str_modify(string)?.b.clear(); }
    let enc = if pos.len() == 1 && string != NIL { Some(c.str_enc(string)) } else { conv_enc };
    *st_mut(c, slf) = SState { string, enc, pos: 0, lineno: 0, flags };
    if flags & FM_BOM != 0 { set_encoding_by_bom(c, slf); }
    let o = obj_mut(c, slf);
    o.rd = flags & FM_R != 0; o.wr = flags & FM_W != 0;
    Ok(slf)
}
fn class_new(c: &mut Ctx, klass: Value, a: &[Value]) -> RV {
    let o = crate::builtins::object::alloc_instance(c, klass)?;
    funcall_kw(c, o, s::initialize, a, nb(), kw_arg(c, a).is_some())?;
    Ok(o)
}
fn sio_s_new(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if !blk.is_null() {
        let n = class_path(c, slf);
        let loc = backtrace_lines(c, 0).first().map(|l| l.split(":in ").next().unwrap_or("").to_string()).unwrap_or_default();
        crate::builtins::kernel::warning_write(c, format!("{}: warning: {}::new() does not take block; use {}::open() instead\n", loc, n, n).as_bytes())?;
    }
    class_new(c, slf, a)
}
fn sio_s_open(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let o = class_new(c, slf, a)?;
    if blk.is_null() { return Ok(o); }
    let r = yield1(c, blk, o);
    // strio_finalize (rb_ensure)
    if let Ok(_) = strio(c, o) { let p = st_mut(c, o); p.string = NIL; p.flags &= !FM_RW; }
    r
}

// ---- trivial / state ----------------------------------------------------------------------------

fn sio_false(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { strio(c, slf)?; Ok(FALSE) }
fn sio_nil(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { strio(c, slf)?; Ok(NIL) }
fn sio_self(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { strio(c, slf)?; Ok(slf) }
fn sio_zero(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { strio(c, slf)?; Ok(Value::fix(0)) }
fn sio_first(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { strio(c, slf)?; Ok(a[0]) }
fn sio_true(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { strio(c, slf)?; Ok(TRUE) }
fn sio_fcntl(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    strio(c, slf)?;
    err(c, c.vm().c_not_impl_error.get(), "fcntl() function is unimplemented on this machine")
}
fn sio_string(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(strio(c, slf)?.string) }
fn sio_set_string(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    strio(c, slf)?;
    st_mut(c, slf).flags &= !FM_RW;
    let s2 = expect_str(c, a[0])?;
    let p = st_mut(c, slf);
    p.flags = if is_frozen(s2) { FM_R } else { FM_RW };
    p.pos = 0; p.lineno = 0; p.string = s2;
    Ok(s2)
}
fn sio_close(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { strio(c, slf)?; let o = obj_mut(c, slf); o.rd = false; o.wr = false; Ok(NIL) }
fn sio_close_read(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let p = strio(c, slf)?;
    if p.flags & FM_R == 0 { return err(c, c.vm().c_io_error.get(), "closing non-duplex IO for reading"); }
    obj_mut(c, slf).rd = false; Ok(NIL)
}
fn sio_close_write(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let p = strio(c, slf)?;
    if p.flags & FM_W == 0 { return err(c, c.vm().c_io_error.get(), "closing non-duplex IO for writing"); }
    obj_mut(c, slf).wr = false; Ok(NIL)
}
fn sio_closed(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { strio(c, slf)?; Ok(Value::bool(closed(c, slf))) }
fn sio_closed_read(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { strio(c, slf)?; Ok(Value::bool(!mode_set(c, slf, FM_R))) }
fn sio_closed_write(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { strio(c, slf)?; Ok(Value::bool(!mode_set(c, slf, FM_W))) }
/// strio_to_read: readable and not at the end.
fn to_read(c: &mut Ctx, slf: Value) -> Result<Option<SState>, Esc> { let p = readable(c, slf)?; Ok(if eos(c, &p) { None } else { Some(p) }) }
fn sio_eof(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(to_read(c, slf)?.is_none())) }
fn sio_copy(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut orig = a[0];
    if c.data::<RStrIO>(orig).is_none() {
        let r = if orig.is_heap() && respond_to(c, orig, intern(b"to_strio"), true) { funcall(c, orig, intern(b"to_strio"), &[], nb())? } else { NIL };
        if c.data::<RStrIO>(r).is_none() {
            let from = crate::builtins::kernel::class_name_for_err(c, orig);
            if r == NIL && !(orig.is_heap() && respond_to(c, orig, intern(b"to_strio"), true)) { return type_err(c, format!("no implicit conversion of {} into StringIO", from)); }
            return type_err(c, format!("can't convert {} to StringIO ({}#to_strio gives {})", class_path(c, real_class_of(c, orig)), class_path(c, real_class_of(c, orig)), class_path(c, real_class_of(c, r))));
        }
        orig = r;
    }
    if orig == slf { return Ok(slf); }
    strio(c, orig)?;
    strio_parts(c, slf)?;
    let (inner, rd, wr) = strio_parts(c, orig)?;
    let o = obj_mut(c, slf);
    o.inner = inner; o.rd = rd; o.wr = wr;
    Ok(slf)
}
fn sio_lineno(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(strio(c, slf)?.lineno)) }
fn sio_set_lineno(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { strio(c, slf)?; let n = num_to_long(c, a[0])?; st_mut(c, slf).lineno = n; Ok(a[0]) }
fn sio_binmode(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let p = strio(c, slf)?;
    st_mut(c, slf).enc = Some(ENC_BINARY);
    if mode_set(c, slf, FM_W) && p.string != NIL { c.rstring_mut(p.string).enc = ENC_BINARY; }
    Ok(slf)
}
fn sio_reopen(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    check_frozen(c, slf)?;
    if a.len() == 1 && !is_string(a[0]) { return sio_copy(c, slf, a, b); }
    strio(c, slf)?;
    strio_init(c, slf, a)
}
fn sio_pos(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(strio(c, slf)?.pos)) }
fn sio_set_pos(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    strio(c, slf)?;
    let p = num_to_long(c, a[0])?;
    if p < 0 { return error_inval(c, None); }
    st_mut(c, slf).pos = p;
    Ok(a[0])
}
fn sio_rewind(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { strio(c, slf)?; let p = st_mut(c, slf); p.pos = 0; p.lineno = 0; Ok(Value::fix(0)) }
fn sio_seek(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    strio(c, slf)?;
    check_argc(c, a, 1, 2)?;
    let amount = num_to_long(c, a[0])?;
    if closed(c, slf) { return err(c, c.vm().c_io_error.get(), "closed stream"); }
    let whence = match a.get(1) { Some(&w) if w != NIL => num_to_long(c, w)?, _ => 0 };
    let p = strio(c, slf)?;
    let offset = match whence { 0 => 0, 1 => p.pos, 2 => slen(c, &p), _ => return error_inval(c, Some("invalid whence")) };
    if amount > i64::MAX - offset || amount + offset < 0 { return error_inval(c, None); }
    st_mut(c, slf).pos = amount + offset;
    Ok(Value::fix(0))
}

// ---- reading --------------------------------------------------------------------------------

fn sio_each_byte(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_byte"), a, NIL); }
    while let Some(p) = to_read(c, slf)? {
        let x = c.str_bytes(p.string)[p.pos as usize];
        st_mut(c, slf).pos += 1;
        yield1(c, blk, Value::fix(x as i64))?;
    }
    Ok(slf)
}
fn sio_getc(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let p = readable(c, slf)?;
    if eos(c, &p) { return Ok(NIL); }
    let enc = enc_or_default(get_enc(c, &p));
    let b = c.str_bytes(p.string);
    let l = en::mbclen(enc, b, p.pos as usize).min(b.len() - p.pos as usize).max(1);
    let ch = b[p.pos as usize..p.pos as usize + l].to_vec();
    st_mut(c, slf).pos += l as i64;
    Ok(c.str_from_vec_enc(ch, enc))
}
fn sio_getbyte(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let p = readable(c, slf)?;
    if eos(c, &p) { return Ok(NIL); }
    let x = c.str_bytes(p.string)[p.pos as usize];
    st_mut(c, slf).pos += 1;
    Ok(Value::fix(x as i64))
}
/// strio_unget_bytes; `cp` None: the stream's own String (copied after it is resized).
fn unget_bytes(c: &mut Ctx, slf: Value, cp: Option<&[u8]>, cl: usize) -> RV {
    let p = *st_mut(c, slf);
    let pos = p.pos as usize;
    room(c, pos.saturating_add(cl))?;
    let r = c.str_modify(p.string)?;
    let len = r.b.len();
    let npos;
    if cl > pos {
        let mut nb2 = r.b[..cl.min(len)].to_vec();
        nb2.resize(cl, 0);
        if pos < len { nb2.extend_from_slice(&r.b[pos..]); }
        r.b = nb2.into();
        npos = 0;
    } else {
        if pos > len { r.b.resize(pos, 0); }
        npos = pos - cl;
    }
    let src: Vec<u8> = match cp { Some(b) => b.to_vec(), None => r.b[..cl].to_vec() };
    r.b[npos..npos + cl].copy_from_slice(&src);
    st_mut(c, slf).pos = npos as i64;
    Ok(NIL)
}
fn sio_ungetc(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = readable(c, slf)?;
    check_modifiable(c, &p)?;
    if p.string == NIL || a[0] == NIL { return Ok(NIL); }
    let x = a[0];
    if x.is_fixnum() || (x.is_heap() && matches!(c.body_of(x), Body::Bignum(_))) {
        let cc = num_to_long(c, x)?;
        if cc < i32::MIN as i64 || cc > i32::MAX as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too {} to convert to 'int'", cc, if cc < 0 { "small" } else { "big" })); }
        let enc = c.str_enc(p.string);
        let cp = cc as u32;
        // rb_enc_codelen + rb_enc_mbcput (UTF-8's 0xfe / 0xff are one byte), rb_enc_uint_chr's error otherwise
        let wide = !en::is_single_byte(enc) && en::kind(enc) != en::EK::Utf8;
        let bs = if en::kind(enc) == en::EK::Utf8 && cp >= 0xffff_fffe { vec![cp as u8] } else if let Some(bs) = if wide { en::onig_mbcput(enc, cp) } else { en::code_to_bytes(enc, cp).filter(|_| !(en::is_single_byte(enc) && cp > 0xff)) } { bs } else {
            let m = match crate::builtins::numeric_methods::enc_chr(enc, cp as u64) {
                Err(true) => format!("{} out of char range", cp),
                _ => format!("invalid codepoint 0x{:X} in {}", cp, en::enc_name(enc)),
            };
            return err(c, c.vm().c_range_error.get(), m);
        };
        return unget_bytes(c, slf, Some(&bs), bs.len());
    }
    let mut s2 = expect_str(c, x)?;
    if c.str_bytes(s2).is_empty() { return Ok(NIL); }
    let (enc, enc2) = (c.str_enc(p.string), c.str_enc(s2));
    if enc != enc2 && enc != ENC_BINARY { s2 = conv_enc(c, s2, enc2, enc); }
    unget_string(c, slf, s2)?;
    Ok(NIL)
}
fn unget_string(c: &mut Ctx, slf: Value, s2: Value) -> RV {
    let cl = c.str_bytes(s2).len();
    if cl == 0 { return Ok(NIL); }
    if s2 == st_mut(c, slf).string { return unget_bytes(c, slf, None, cl); }
    let b = c.str_bytes(s2).to_vec();
    unget_bytes(c, slf, Some(&b), cl)
}
fn sio_ungetbyte(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = readable(c, slf)?;
    check_modifiable(c, &p)?;
    if p.string == NIL || a[0] == NIL { return Ok(NIL); }
    let x = a[0];
    if x.is_fixnum() || (x.is_heap() && matches!(c.body_of(x), Body::Bignum(_))) {
        let v = funcall(c, x, intern(b"&"), &[Value::fix(0xff)], nb())?;
        let b = (num_to_long(c, v)? & 0xff) as u8;
        return unget_bytes(c, slf, Some(&[b]), 1);
    }
    let s2 = expect_str(c, x)?;
    unget_string(c, slf, s2)
}
fn sio_readchar(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let r = funcall(c, slf, intern(b"getc"), &[], nb())?;
    if r == NIL { return eof_error(c); }
    Ok(r)
}
fn sio_readbyte(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let r = funcall(c, slf, intern(b"getbyte"), &[], nb())?;
    if r == NIL { return eof_error(c); }
    Ok(r)
}
fn sio_each_char(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_char"), a, NIL); }
    loop { let ch = sio_getc(c, slf, &[], nb())?; if ch == NIL { break; } yield1(c, blk, ch)?; }
    Ok(slf)
}
fn sio_each_codepoint(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_codepoint"), a, NIL); }
    let p = readable(c, slf)?;
    let enc = enc_or_default(get_enc(c, &p));
    while let Some(p) = to_read(c, slf)? {
        let b = c.str_bytes(p.string);
        let i = p.pos as usize;
        let Some(n) = en::precise_len(enc, b, i) else { return arg_err(c, format!("invalid byte sequence in {}", en::enc_name(enc))) };
        let cp = en::char_code(enc, &b[i..i + n]);
        st_mut(c, slf).pos += n as i64;
        yield1(c, blk, Value::fix(cp as i64))?;
    }
    Ok(slf)
}

struct GetlineArg { rs: Value, limit: i64, chomp: bool }
fn check_string_type(c: &mut Ctx, v: Value) -> RV {
    if is_string(v) { return Ok(v); }
    if v.is_heap() && respond_to(c, v, s::to_str, true) { return expect_str(c, v); }
    Ok(NIL)
}
fn prepare_getline_args(c: &mut Ctx, p: &SState, a: &[Value]) -> Result<GetlineArg, Esc> {
    let kw = kw_arg(c, a);
    let pos: &[Value] = if kw.is_some() { &a[..a.len() - 1] } else { a };
    check_argc(c, pos, 0, 2)?;
    let rs_g = gvar_get_rs(c)?;
    let mut limit = -1;
    let respect_chomp = pos.is_empty() || pos[0] != NIL;
    let mut rs = rs_g;
    match pos.len() {
        1 => {
            rs = pos[0];
            if rs != NIL && !is_string(rs) {
                let t = check_string_type(c, rs)?;
                if t == NIL { limit = num_to_long(c, rs)?; rs = rs_g; } else { rs = t; }
            }
        }
        2 => {
            rs = pos[0];
            if rs != NIL { rs = expect_str(c, rs)?; }
            if pos[1] != NIL { limit = num_to_long(c, pos[1])?; }
        }
        _ => {}
    }
    if p.string != NIL && rs != NIL {
        let enc_rs = c.str_enc(rs);
        let enc_io = enc_or_default(get_enc(c, p));
        if enc_rs != enc_io && (!is_7bit(c, rs) || (!c.str_bytes(rs).is_empty() && !en::ascii_compat(enc_io))) {
            if rs == rs_g {
                rs = match en::code_to_bytes(enc_io, b'\n' as u32) { Some(b) => c.str_from_vec_enc(b, enc_io), None => c.str_new_enc(b"\n", enc_io) };
            } else {
                return arg_err(c, format!("encoding mismatch: {} IO with {} RS", en::enc_name(enc_io), en::enc_name(enc_rs)));
            }
        }
    }
    let mut chomp = false;
    if let Some(h) = kw {
        let keys: Vec<Value> = c.hash_pairs(h).into_iter().map(|p| p.0).collect();
        for k in keys { if k != sym_value("chomp") { let ks = inspect_str(c, k)?; return arg_err(c, format!("unknown keyword: {}", ks)); } }
        if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("chomp"))? { if respect_chomp { chomp = v.truthy(); } }
    }
    Ok(GetlineArg { rs, limit, chomp })
}
fn gvar_get_rs(c: &mut Ctx) -> RV { crate::builtins::kernel::gvar_get(c, intern(b"$/")) }
fn chomp_newline_width(b: &[u8], s: usize, e: usize) -> usize {
    if e > s && b[e - 1] == b'\n' { if e - 1 > s && b[e - 2] == b'\r' { return 2; } return 1; }
    0
}
/// strio_getline
fn getline(c: &mut Ctx, slf: Value, arg: &GetlineArg) -> RV {
    let p = *st_mut(c, slf);
    if p.string == NIL || p.pos >= slen(c, &p) { return Ok(NIL); }
    let enc = enc_or_default(get_enc(c, &p));
    let b = c.str_bytes(p.string).to_vec();
    let mut s = p.pos as usize;
    let mut e = b.len();
    if arg.limit > 0 && (arg.limit as usize) < e - s {
        let lim = s + arg.limit as usize;
        let offs = char_offsets_of(&b[s..], enc);
        e = offs.iter().map(|&o| o + s).find(|&o| o >= lim).unwrap_or(e);
    }
    let mut w = 0usize;
    let str;
    if arg.rs == NIL {
        if arg.chomp { w = chomp_newline_width(&b, s, e); }
        str = substr(c, &p, p.pos, (e - s - w) as i64, enc);
    } else {
        let rsb = c.str_bytes(arg.rs).to_vec();
        let n = rsb.len();
        if n == 0 {
            let mut q = s;
            while b[q] == b'\n' { q += 1; if q == e { return Ok(NIL); } }
            s = q;
            let mut paragraph_end = None;
            while let Some(off) = b[q..e].iter().position(|&x| x == b'\n') {
                q += off + 1;
                if !((q < e && b[q] == b'\n') || (q + 1 < e && b[q] == b'\r' && b[q + 1] == b'\n')) { continue; }
                paragraph_end = Some(q - if q >= 2 && b[q - 2] == b'\r' { 2 } else { 1 });
                while (q < e && b[q] == b'\n') || (q + 1 < e && b[q] == b'\r' && b[q + 1] == b'\n') { q += if b[q] == b'\r' { 2 } else { 1 }; }
                e = q;
                break;
            }
            if arg.chomp { if let Some(pe) = paragraph_end { w = e - pe; } }
            str = substr(c, &p, s as i64, (e - s - w) as i64, enc);
        } else if n == 1 {
            if let Some(off) = b[s..e].iter().position(|&x| x == rsb[0]) {
                let q = s + off;
                e = q + 1;
                w = if arg.chomp { (q > s && b[q - 1] == b'\r') as usize + 1 } else { 0 };
            }
            str = substr(c, &p, p.pos, (e - s - w) as i64, enc);
        } else {
            if n < e - s + arg.chomp as usize {
                if e - s < 1024 || n == e - s {
                    let mut q = s;
                    while q + n <= e {
                        if b[q..q + n] == rsb[..] { e = q + n; w = if arg.chomp { n } else { 0 }; break; }
                        q += 1;
                    }
                } else if let Some(off) = crate::builtins::string_methods::find_bytes(&b[s..e], &rsb, 0) {
                    e = s + off + if arg.chomp { 0 } else { n };
                }
            }
            str = substr(c, &p, p.pos, (e - s - w) as i64, enc);
        }
    }
    let m = st_mut(c, slf);
    m.pos = e as i64;
    m.lineno += 1;
    Ok(str)
}
fn set_last_line(c: &mut Ctx, v: Value) -> Result<(), Esc> { crate::builtins::kernel::gvar_set(c, intern(b"$_"), v)?; Ok(()) }
fn sio_gets(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = readable(c, slf)?;
    let arg = prepare_getline_args(c, &p, a)?;
    if arg.limit == 0 {
        if p.string == NIL { return Ok(NIL); }
        let e = enc_or_default(get_enc(c, &p));
        return Ok(c.str_new_enc(b"", e));
    }
    let r = getline(c, slf, &arg)?;
    set_last_line(c, r)?;
    Ok(r)
}
fn sio_readline(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let r = funcall_kw(c, slf, intern(b"gets"), a, nb(), kw_arg(c, a).is_some())?;
    if r == NIL { return eof_error(c); }
    Ok(r)
}
fn sio_each(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let p = readable(c, slf)?;
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_line"), a, NIL); }
    let arg = prepare_getline_args(c, &p, a)?;
    if arg.limit == 0 { return arg_err(c, "invalid limit: 0 for each_line"); }
    loop { let l = getline(c, slf, &arg)?; if l == NIL { break; } yield1(c, blk, l)?; }
    Ok(slf)
}
fn sio_readlines(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = readable(c, slf)?;
    let arg = prepare_getline_args(c, &p, a)?;
    if arg.limit == 0 { return arg_err(c, "invalid limit: 0 for readlines"); }
    let out = c.ary_new();
    loop { let l = getline(c, slf, &arg)?; if l == NIL { break; } c.ary_push(out, l); }
    Ok(out)
}
/// strio_read
fn read_impl(c: &mut Ctx, slf: Value, a: &[Value]) -> RV {
    let p = readable(c, slf)?;
    let mut buf = NIL;
    let mut binary = false;
    let len: i64;
    if a.len() > 2 { return argc_err(c, a.len(), 0, 2); }
    if a.len() == 2 && a[1] != NIL { buf = expect_str(c, a[1])?; check_frozen(c, buf)?; }
    if !a.is_empty() && a[0] != NIL {
        let l = num_to_long(c, a[0])?;
        if l < 0 { return arg_err(c, format!("negative length {} given", l)); }
        let p = strio(c, slf)?;
        if eos(c, &p) {
            if buf != NIL { c.rstring_mut(buf).b.clear(); }
            return Ok(if l > 0 { NIL } else { c.str_new_enc(b"", ENC_BINARY) });
        }
        len = l; binary = true;
    } else {
        if p.string == NIL { return Ok(NIL); }
        let total = slen(c, &p);
        if total <= p.pos {
            let enc = get_enc(c, &p);
            let r = if buf == NIL { c.str_new_enc(b"", ENC_BINARY) } else { c.rstring_mut(buf).b.clear(); buf };
            if let Some(e) = enc { c.rstring_mut(r).enc = e; }
            return Ok(r);
        }
        len = total - p.pos;
    }
    let p = strio(c, slf)?;
    let r = if buf == NIL {
        let enc = if binary { ENC_BINARY } else { enc_or_default(get_enc(c, &p)) };
        substr(c, &p, p.pos, len, enc)
    } else {
        let rest = slen(c, &p) - p.pos;
        let l = len.min(rest).max(0) as usize;
        let data = c.str_bytes(p.string)[p.pos as usize..p.pos as usize + l].to_vec();
        let se = c.str_enc(p.string);
        let r = c.rstring_mut(buf);
        r.b = data.into();
        if !binary { r.enc = se; }
        buf
    };
    let n = c.str_bytes(r).len() as i64;
    st_mut(c, slf).pos += n;
    Ok(r)
}
fn sio_read(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { read_impl(c, slf, a) }
fn sio_pread(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 2, 3)?;
    let len = num_to_long(c, a[0])?;
    let off = num_to_long(c, a[1])?;
    let buf = a.get(2).copied().unwrap_or(NIL);
    if len < 0 { let s2 = inspect_str(c, a[0])?; return arg_err(c, format!("negative string size (or size too big): {}", s2)); }
    if len == 0 { return Ok(if buf == NIL { c.str_new_enc(b"", ENC_BINARY) } else { buf }); }
    if off < 0 { let s2 = inspect_str(c, a[1])?; return error_inval(c, Some(&format!("pread: Invalid offset argument: {}", s2))); }
    let p = readable(c, slf)?;
    if outside(c, &p, off) { return eof_error(c); }
    if buf == NIL { return Ok(substr(c, &p, off, len, ENC_BINARY)); }
    let buf = expect_str(c, buf)?;
    let l = len.min(slen(c, &p) - off) as usize;
    let data = c.str_bytes(p.string)[off as usize..off as usize + l].to_vec();
    let r = c.str_modify(buf)?;
    r.b = data.into(); r.enc = ENC_BINARY;
    Ok(buf)
}
fn sio_sysread(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let r = funcall_kw(c, slf, intern(b"read"), a, nb(), kw_arg(c, a).is_some())?;
    if r == NIL { return eof_error(c); }
    Ok(r)
}
fn sio_read_nonblock(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let kw = kw_arg(c, a);
    let pos: &[Value] = if kw.is_some() { &a[..a.len() - 1] } else { a };
    check_argc(c, pos, 1, 2)?;
    let r = read_impl(c, slf, pos)?;
    if r == NIL {
        if let Some(h) = kw { if crate::hash::hash_get_opt(c, h, sym_value("exception"))? == Some(FALSE) { return Ok(NIL); } }
        return eof_error(c);
    }
    Ok(r)
}

// ---- writing --------------------------------------------------------------------------------

/// strio_write
fn write_one(c: &mut Ctx, slf: Value, x: Value) -> Result<i64, Esc> {
    let p = writable(c, slf)?;
    let mut s2 = if is_string(x) { x } else { crate::rt::srb_tostr(c, x)? };
    let Some(enc) = get_enc(c, &p) else { return Ok(0) };
    let enc2 = c.str_enc(s2);
    if enc != enc2 && enc != ENC_BINARY && enc != ENC_ASCII {
        let conv = conv_enc(c, s2, enc2, enc);
        if conv == s2 && enc2 != ENC_BINARY && enc2 != ENC_ASCII && !(is_7bit(c, s2) && en::ascii_compat(enc)) {
            return Err(en::compat_error(c, enc, enc2));
        }
        s2 = conv;
    }
    let data = c.str_bytes(s2).to_vec();
    let len = data.len();
    if len == 0 { return Ok(0); }
    check_modifiable(c, &p)?;
    let olen = slen(c, &p);
    if p.flags & FM_APPEND != 0 { st_mut(c, slf).pos = olen; }
    let pos = st_mut(c, slf).pos;
    if pos == olen {
        let de = if enc == ENC_BINARY || enc2 == ENC_BINARY { enc } else { c.str_enc(s2) };
        cat_enc(c, p.string, &data, de)?;
    } else {
        room(c, (pos as usize).saturating_add(len))?;
        let r = c.str_modify(p.string)?;
        let pu = pos as usize;
        if pu + len > r.b.len() { r.b.resize(pu + len, 0); }
        r.b[pu..pu + len].copy_from_slice(&data);
    }
    st_mut(c, slf).pos += len as i64;
    Ok(len as i64)
}
fn sio_write(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut n = 0;
    for &x in a { n += write_one(c, slf, x)?; }
    Ok(Value::fix(n))
}
fn sio_putc(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = writable(c, slf)?;
    check_modifiable(c, &p)?;
    let ch = a[0];
    let s2 = if is_string(ch) {
        if p.string == NIL { return Ok(ch); }
        let f = c.chars_of(ch).into_iter().next().unwrap_or_default();
        let e = c.str_enc(ch);
        c.str_new_enc(&f, e)
    } else {
        let n = num_to_long(c, ch)? as u8;
        if p.string == NIL { return Ok(ch); }
        c.str_new_enc(&[n], ENC_BINARY)
    };
    write_one(c, slf, s2)?;
    Ok(ch)
}
/// rb_io_write: `io.write(str)`.
fn io_write(c: &mut Ctx, io: Value, s2: Value) -> RV { funcall(c, io, intern(b"write"), &[s2], nb()) }
/// rb_io_writev for a non-IO: one write call, or one per piece when #write takes one argument.
fn io_writev(c: &mut Ctx, io: Value, pieces: &[Value]) -> Result<(), Esc> {
    if pieces.len() > 1 {
        let me = lookup_method(c, class_of(io), intern(b"write"));
        if !me.is_null() && crate::builtins::proc::method_arity(c, me) == 1 {
            for &x in pieces { io_write(c, io, x)?; }
            return Ok(());
        }
    }
    funcall(c, io, intern(b"write"), pieces, nb())?;
    Ok(())
}
fn default_rs(c: &mut Ctx) -> Value { c.str_new_enc(b"\n", ENC_ASCII) }
fn ends_with_newline(c: &Ctx, s2: Value) -> bool {
    let (b, e) = (c.str_bytes(s2), c.str_enc(s2));
    match en::code_to_bytes(e, b'\n' as u32) { Some(nl) if !en::ascii_compat(e) => b.ends_with(&nl), _ => b.last() == Some(&b'\n') }
}
fn puts_one(c: &mut Ctx, io: Value, x: Value) -> Result<(), Esc> {
    let mut line = check_string_type(c, x)?;
    if line == NIL {
        if let Some(ary) = crate::array::check_array(c, x)? {
            if !crate::builtins::kernel::rec_enter(c, ary, s::puts)? { let r = c.str_new(b"[...]"); return puts_one(c, io, r); }
            let n = c.ary_len(ary);
            let mut res = Ok(());
            for i in 0..n { let e = c.ary_get(ary, i); res = puts_one(c, io, e); if res.is_err() { break; } }
            crate::builtins::kernel::rec_leave(c, ary, s::puts);
            return res;
        }
        line = crate::rt::srb_tostr(c, x)?;
    }
    if c.str_bytes(line).is_empty() || !ends_with_newline(c, line) { let rs = default_rs(c); io_writev(c, io, &[line, rs]) } else { io_writev(c, io, &[line]) }
}
fn sio_puts(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.is_empty() { let rs = default_rs(c); io_write(c, slf, rs)?; return Ok(NIL); }
    for &x in a { puts_one(c, slf, x)?; }
    Ok(NIL)
}
fn sio_print(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let line;
    let args: &[Value] = if a.is_empty() { line = [crate::builtins::kernel::gvar_get(c, intern(b"$_"))?]; &line } else { a };
    let fs = crate::builtins::kernel::gvar_get(c, intern(b"$,"))?;
    for (i, &x) in args.iter().enumerate() {
        if i > 0 && fs != NIL { io_write(c, slf, fs)?; }
        io_write(c, slf, x)?;
    }
    let ors = crate::builtins::kernel::gvar_get(c, intern(b"$\\"))?;
    if ors != NIL { io_write(c, slf, ors)?; }
    Ok(NIL)
}
fn sio_printf(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.is_empty() { return arg_err(c, "too few arguments"); }
    let s2 = crate::builtins::sprintf::format(c, a[0], &a[1..])?;
    io_write(c, slf, s2)?;
    Ok(NIL)
}
fn sio_addstr(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { io_write(c, slf, a[0])?; Ok(slf) }
fn sio_syswrite(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { io_write(c, slf, a[0]) }
fn sio_write_nonblock(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let pos: &[Value] = if kw_arg(c, a).is_some() { &a[..a.len() - 1] } else { a };
    check_argc(c, pos, 1, 1)?;
    io_write(c, slf, pos[0])
}
fn sio_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let p = strio(c, slf)?; Ok(Value::fix(slen(c, &p))) }
/// NoMemoryError for a buffer CRuby's malloc would refuse (> 1 TiB).
fn room(c: &mut Ctx, n: usize) -> Result<(), Esc> {
    if n >= i64::MAX as usize { return arg_err(c, "string size too big"); }
    if n > 1usize << 40 { return err(c, c.vm().c_no_memory_error.get(), "failed to allocate memory"); }
    Ok(())
}
fn sio_truncate(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = writable(c, slf)?;
    let l = num_to_long(c, a[0])?;
    if l < 0 { return error_inval(c, Some("negative length")); }
    if p.string == NIL { return Ok(FALSE); }
    room(c, l as usize)?;
    c.str_modify(p.string)?.b.resize(l as usize, 0);
    Ok(Value::fix(0))
}
fn sio_external_encoding(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let p = strio(c, slf)?;
    Ok(match get_enc(c, &p) { Some(e) => en::encoding_value(c, e), None => NIL })
}
fn sio_internal_encoding(_c: &mut Ctx, _slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(NIL) }
fn sio_set_encoding(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let p = strio(c, slf)?;
    let pos: &[Value] = if kw_arg(c, a).is_some() { &a[..a.len() - 1] } else { a };
    check_argc(c, pos, 1, 2)?;
    let ext = pos[0];
    let enc = if ext == NIL { Some(en::default_external()) } else {
        match en::to_enc_opt(c, ext)? {
            Ok(e) => Some(e),
            Err(_) => {
                let es = expect_str(c, ext)?;
                let mut m = b"r:".to_vec(); m.extend_from_slice(c.str_bytes(es));
                let ms = c.str_new(&m);
                let om = crate::builtins::io::parse_mode(c, &[ms], None)?;
                om.encs.and_then(|e| e.1)
            }
        }
    };
    st_mut(c, slf).enc = enc;
    if let Some(e) = enc { if p.string != NIL && mode_set(c, slf, FM_W) && !is_frozen(p.string) { c.rstring_mut(p.string).enc = e; } }
    Ok(slf)
}
fn sio_set_encoding_by_bom(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    strio(c, slf)?;
    Ok(match set_encoding_by_bom(c, slf) { Some(e) => en::encoding_value(c, e), None => NIL })
}

// ---- io/console's generic_readable methods ------------------------------------------------------

fn gr_getch(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { funcall(c, slf, intern(b"getc"), a, nb()) }
fn gr_getpass(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    if let Some(&pr) = a.first() { if pr != NIL { let s2 = expect_str(c, pr)?; io_write(c, slf, s2)?; } }
    if respond_to(c, slf, intern(b"flush"), false) { funcall(c, slf, intern(b"flush"), &[], nb())?; }
    let r = funcall(c, slf, intern(b"gets"), &[], nb());
    let rs = default_rs(c);
    io_write(c, slf, rs)?;
    let r = r?;
    if r != NIL { funcall(c, r, intern(b"chomp!"), &[rs], nb())?; }
    Ok(r)
}
/// The part of io/console that is not about terminals.
pub fn init_console(c: &mut Ctx) {
    let io = c.vm().c_io.get();
    let gr = const_get_at(c, io, intern(b"generic_readable")).unwrap_or_else(|| define_module_under(c, io, "generic_readable"));
    define_cfunc(c, gr, "getch", gr_getch, -1);
    define_cfunc(c, gr, "getpass", gr_getpass, -1);
}

/// StringIO is built by `require "stringio"` (CRuby: not preloaded).
pub fn init_once(c: &mut Ctx) { if !DONE.swap(true, std::sync::atomic::Ordering::Relaxed) { init(c); } }
static DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let k = define_class(c, "StringIO", vmr.c_object.get());
    class_set_alloc(c, k, AllocKind::None);
    ivar_set_generic(k, hidden_mark(), TRUE);
    let io = vmr.c_io.get();
    for (n, v) in [("SEEK_SET", 0), ("SEEK_CUR", 1), ("SEEK_END", 2)] {
        if const_get_at(c, io, intern(n.as_bytes())).is_none() { const_set(c, io, intern(n.as_bytes()), Value::fix(v)); }
    }
    include_module(c, k, vmr.m_enumerable.get());
    let v = c.alloc(vmr.c_string.get(), Body::String(RString { b: crate::sbuf::SBuf::from_slice(b"3.2.0"), enc: ENC_UTF8 }));
    const_set(c, k, intern(b"VERSION"), v);
    const_set(c, k, intern(b"MAX_LENGTH"), Value::fix(i64::MAX >> 1));
    define_singleton_cfunc(c, k, "new", sio_s_new, -1);
    define_singleton_cfunc(c, k, "open", sio_s_open, -1);
    define_private_cfunc(c, k, "initialize", sio_initialize, -1);
    define_private_cfunc(c, k, "initialize_copy", sio_copy, 1);
    let d = |c: &mut Ctx, n: &str, f: CFunc, ar: i32| define_cfunc(c, k, n, f, ar);
    d(c, "reopen", sio_reopen, -1); d(c, "string", sio_string, 0); d(c, "string=", sio_set_string, 1); d(c, "lineno", sio_lineno, 0); d(c, "lineno=", sio_set_lineno, 1);
    d(c, "binmode", sio_binmode, 0); d(c, "close", sio_close, 0); d(c, "close_read", sio_close_read, 0); d(c, "close_write", sio_close_write, 0);
    d(c, "closed?", sio_closed, 0); d(c, "closed_read?", sio_closed_read, 0); d(c, "closed_write?", sio_closed_write, 0); d(c, "eof", sio_eof, 0); d(c, "eof?", sio_eof, 0);
    d(c, "fcntl", sio_fcntl, -1); d(c, "flush", sio_self, 0); d(c, "fsync", sio_zero, 0); d(c, "pos", sio_pos, 0); d(c, "pos=", sio_set_pos, 1);
    d(c, "rewind", sio_rewind, 0); d(c, "seek", sio_seek, -1); d(c, "sync", sio_true, 0); d(c, "sync=", sio_first, 1); d(c, "tell", sio_pos, 0);
    d(c, "each", sio_each, -1); d(c, "each_line", sio_each, -1); d(c, "each_byte", sio_each_byte, 0); d(c, "each_char", sio_each_char, 0); d(c, "each_codepoint", sio_each_codepoint, 0);
    d(c, "getc", sio_getc, 0); d(c, "ungetc", sio_ungetc, 1); d(c, "ungetbyte", sio_ungetbyte, 1); d(c, "getbyte", sio_getbyte, 0); d(c, "gets", sio_gets, -1);
    d(c, "readlines", sio_readlines, -1); d(c, "read", sio_read, -1); d(c, "pread", sio_pread, -1); d(c, "write", sio_write, -1); d(c, "putc", sio_putc, 1);
    d(c, "isatty", sio_false, 0); d(c, "tty?", sio_false, 0); d(c, "pid", sio_nil, 0); d(c, "fileno", sio_nil, 0); d(c, "size", sio_size, 0); d(c, "length", sio_size, 0);
    d(c, "truncate", sio_truncate, 1); d(c, "external_encoding", sio_external_encoding, 0); d(c, "internal_encoding", sio_internal_encoding, 0);
    d(c, "set_encoding", sio_set_encoding, -1); d(c, "set_encoding_by_bom", sio_set_encoding_by_bom, 0);
    // io/console may have made it (and added getch / getpass) first
    let gr = const_get_at(c, io, intern(b"generic_readable")).unwrap_or_else(|| define_module_under(c, io, "generic_readable"));
    let r = |c: &mut Ctx, n: &str, f: CFunc, ar: i32| define_cfunc(c, gr, n, f, ar);
    r(c, "readchar", sio_readchar, 0); r(c, "readbyte", sio_readbyte, 0); r(c, "readline", sio_readline, -1); r(c, "sysread", sio_sysread, -1);
    r(c, "readpartial", sio_sysread, -1); r(c, "read_nonblock", sio_read_nonblock, -1);
    include_module(c, k, gr);
    let gw = define_module_under(c, io, "generic_writable");
    let w = |c: &mut Ctx, n: &str, f: CFunc, ar: i32| define_cfunc(c, gw, n, f, ar);
    w(c, "<<", sio_addstr, 1); w(c, "print", sio_print, -1); w(c, "printf", sio_printf, -1); w(c, "puts", sio_puts, -1); w(c, "syswrite", sio_syswrite, 1); w(c, "write_nonblock", sio_write_nonblock, -1);
    include_module(c, k, gw);
}
