// StringIO (the stringio extension), with IO::generic_readable / IO::generic_writable.
use crate::array::*;
use crate::builtins::encoding as en;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

pub struct StrIO {
    pub string: Value,
    pub pos: usize,
    pub lineno: i64,
    pub readable: bool,
    pub writable: bool,
    pub append: bool,
    pub enc: Option<u8>,
}
impl RData for StrIO {
    fn mark(&self, m: &mut crate::gc::Marker) { m.mark(self.string); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

fn errno(name: &str) -> Value {
    let e = const_get_at(vm().c_object, intern(b"Errno")).unwrap_or(NIL);
    if e == NIL { return vm().c_io_error; }
    const_get_at(e, intern(name.as_bytes())).unwrap_or(vm().c_io_error)
}
fn sio(c: &mut Ctx, v: Value) -> Result<&'static mut StrIO, Esc> {
    match rdata::<StrIO>(v) { Some(d) => Ok(d), None => type_err(c, "uninitialized stream") }
}
fn readable(c: &mut Ctx, v: Value) -> Result<&'static mut StrIO, Esc> {
    let d = sio(c, v)?;
    if !d.readable { return err(c, vm().c_io_error, "not opened for reading"); }
    Ok(d)
}
fn writable(c: &mut Ctx, v: Value) -> Result<&'static mut StrIO, Esc> {
    let d = sio(c, v)?;
    if !d.writable { return err(c, vm().c_io_error, "not opened for writing"); }
    Ok(d)
}
fn bytes(d: &StrIO) -> &'static [u8] { &rstr(d.string).b }
fn read_enc(d: &StrIO) -> u8 { d.enc.unwrap_or(rstr(d.string).enc) }

fn sio_s_new(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    let o = crate::gc::alloc(slf, Body::Data(Box::new(StrIO { string: NIL, pos: 0, lineno: 0, readable: true, writable: true, append: false, enc: None })));
    funcall_kw(c, o, s::initialize, a, blk, kw_arg(a).is_some())?;
    Ok(o)
}
fn sio_s_open(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    let o = sio_s_new(c, slf, a, std::ptr::null())?;
    if blk.is_null() { return Ok(o); }
    let r = yield1(c, blk, o)?;
    if let Some(d) = rdata::<StrIO>(o) { d.readable = false; d.writable = false; }
    Ok(r)
}

#[derive(Default)]
struct Mode { r: bool, w: bool, append: bool, trunc: bool, bin: bool, enc: Option<u8>, bom: bool }
/// "bom|utf-8" style encoding spec.
fn enc_spec(c: &mut Ctx, m: &mut Mode, e: Value) -> Result<(), Esc> {
    let es = if is_string(e) { String::from_utf8_lossy(&rstr(e).b).into_owned() } else { String::new() };
    if es.len() > 4 && es[..4].eq_ignore_ascii_case("bom|") { m.bom = true; m.enc = en::find_enc(es[4..].as_bytes()); }
    else { m.enc = Some(en::to_enc(c, e)?); }
    Ok(())
}
fn parse_mode(c: &mut Ctx, ms: &str) -> Result<Mode, Esc> {
    let (flags, encs) = match ms.split_once(':') { Some((f, e)) => (f.to_string(), Some(e.to_string())), None => (ms.to_string(), None) };
    let mut it = flags.chars();
    let mut m = Mode::default();
    match it.next() {
        Some('r') => m.r = true, Some('w') => { m.w = true; m.trunc = true; } Some('a') => { m.w = true; m.append = true; }
        _ => return arg_err(c, format!("invalid access mode {}", ms)),
    }
    for ch in it {
        match ch { '+' => { m.r = true; m.w = true; } 'b' => m.bin = true, 't' => {} _ => return arg_err(c, format!("invalid access mode {}", ms)) }
    }
    if let Some(e) = encs { let e1 = e.split(':').next().unwrap_or("").to_string(); let v = str_new(e1.as_bytes()); enc_spec(c, &mut m, v)?; }
    Ok(m)
}
fn sio_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let kw = kw_arg(a);
    let pos: &[Value] = if kw.is_some() { &a[..a.len() - 1] } else { a };
    check_argc(c, pos, 0, 2)?;
    let string = match pos.first() { Some(&v) => expect_str(c, v)?, None => str_new_enc(b"", en::default_external()) };
    let frozen = is_frozen(string);
    let mut m = Mode { r: true, w: !frozen, ..Default::default() };
    if let Some(&mv) = pos.get(1) {
        if mv.is_fixnum() {
            let f = mv.as_fix();
            let acc = f & 3;
            m.r = acc == 0 || acc == 2; m.w = acc == 1 || acc == 2; m.append = f & 0x400 != 0; m.trunc = f & 0x200 != 0;
        } else if mv != NIL {
            let ms = expect_str(c, mv)?;
            let s0 = String::from_utf8_lossy(&rstr(ms).b).into_owned();
            m = parse_mode(c, &s0)?;
        }
    }
    if let Some(h) = kw {
        if let Some(mv) = crate::hash::hash_get_opt(c, h, sym_value("mode"))? { if mv != NIL { let ms = expect_str(c, mv)?; let s0 = String::from_utf8_lossy(&rstr(ms).b).into_owned(); m = parse_mode(c, &s0)?; } }
        if let Some(b) = crate::hash::hash_get_opt(c, h, sym_value("binmode"))? { if b.truthy() { m.bin = true; } }
        if let Some(e) = crate::hash::hash_get_opt(c, h, sym_value("encoding"))? { if e != NIL { enc_spec(c, &mut m, e)?; } }
        if let Some(e) = crate::hash::hash_get_opt(c, h, sym_value("external_encoding"))? { if e != NIL { enc_spec(c, &mut m, e)?; } }
    }
    if let Some(e) = m.enc { if !en::ascii_compat(e) && !m.bin && !m.trunc && !m.append { return arg_err(c, "ASCII incompatible encoding needs binmode"); } }
    if m.w && frozen { let e = make_exception(errno("EACCES"), "Permission denied"); return Err(raise_exc(c, e)); }
    if m.bin && m.enc.is_none() { m.enc = Some(ENC_BINARY); }
    if m.trunc { check_frozen(c, string)?; rstr(string).b.clear(); }
    if m.bin { if let Some(e) = m.enc { if !frozen { rstr(string).enc = e; } } }
    let mut start = 0;
    if m.bom {
        let b0 = &rstr(string).b;
        let (bom_enc, l) = if b0.starts_with(&[0xef, 0xbb, 0xbf]) { (ENC_UTF8, 3) } else if b0.starts_with(&[0xff, 0xfe]) { (en::ENC_UTF16LE, 2) } else if b0.starts_with(&[0xfe, 0xff]) { (en::ENC_UTF16BE, 2) } else { (0, 0) };
        if l > 0 { start = l; m.enc = Some(bom_enc); }
    }
    let d = sio(c, slf)?;
    d.string = string; d.pos = start; d.lineno = 0; d.readable = m.r; d.writable = m.w; d.append = m.append; d.enc = m.enc;
    Ok(NIL)
}
fn sio_string(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(sio(c, slf)?.string) }
fn sio_set_string(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let s2 = expect_str(c, a[0])?;
    let d = sio(c, slf)?;
    d.string = s2; d.pos = 0; d.lineno = 0; d.readable = true; d.writable = !is_frozen(s2);
    Ok(s2)
}

// ---- reading ----------------------------------------------------------------------------------

fn sio_read(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 2)?;
    let d = readable(c, slf)?;
    let buf = match a.get(1) { Some(&v) if v != NIL => Some(expect_str(c, v)?), _ => None };
    let all = bytes(d);
    let rest = if d.pos < all.len() { &all[d.pos..] } else { &[][..] };
    let (data, enc) = match a.first() {
        Some(&l) if l != NIL => {
            let n = num_to_long(c, l)?;
            if n < 0 { return arg_err(c, format!("negative length {} given", n)); }
            if n > 0 && rest.is_empty() { if let Some(bf) = buf { check_frozen(c, bf)?; rstr(bf).b.clear(); } return Ok(NIL); }
            (rest[..(n as usize).min(rest.len())].to_vec(), ENC_BINARY)
        }
        _ => (rest.to_vec(), read_enc(d)),
    };
    d.pos += data.len();
    if let Some(bf) = buf { check_frozen(c, bf)?; let r = rstr(bf); r.b = data; if a.first().map(|v| *v != NIL).unwrap_or(false) { r.enc = ENC_BINARY; } return Ok(bf); }
    Ok(str_from_vec_enc(data, enc))
}
fn sio_readpartial(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let r = sio_read(c, slf, a, b)?;
    if r == NIL || (a.first().map(|v| v.is_fixnum() && v.as_fix() > 0).unwrap_or(false) && rstr(r).b.is_empty()) { return err(c, vm().c_eof_error, "end of file reached"); }
    Ok(r)
}
/// gets core: returns the line bytes (None at EOF).
fn gets_impl(c: &mut Ctx, slf: Value, a: &[Value]) -> Result<Option<Value>, Esc> {
    let kw = kw_arg(a);
    let pos: &[Value] = if kw.is_some() { &a[..a.len() - 1] } else { a };
    check_argc(c, pos, 0, 2)?;
    let mut chomp = false;
    if let Some(h) = kw { if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("chomp"))? { chomp = v.truthy(); } }
    let rs_default = gvar_get(c, intern(b"$/"))?;
    let (mut sep, mut limit): (Option<Vec<u8>>, Option<i64>) = (if rs_default == NIL { None } else { Some(rstr(rs_default).b.clone()) }, None);
    match pos.len() {
        1 => { let x = pos[0]; if x == NIL { sep = None; } else if is_string(x) || !(x.is_fixnum() || is_float(x)) { let s2 = expect_str(c, x)?; sep = Some(rstr(s2).b.clone()); } else { limit = Some(num_to_long(c, x)?); } }
        2 => { let x = pos[0]; sep = if x == NIL { None } else { let s2 = expect_str(c, x)?; Some(rstr(s2).b.clone()) }; if pos[1] != NIL { limit = Some(num_to_long(c, pos[1])?); } }
        _ => {}
    }
    let d = readable(c, slf)?;
    let all = bytes(d);
    let enc = read_enc(d);
    if limit == Some(0) { return Ok(Some(str_new_enc(b"", enc))); }
    if d.pos >= all.len() { return Ok(None); }
    let mut st = d.pos;
    let rest = &all[st..];
    let paragraph = sep.as_deref() == Some(b"");
    if paragraph { while st < all.len() && all[st] == b'\n' { st += 1; } }
    let rest2 = &all[st..];
    let (mut end, mut sep_len) = match &sep {
        None => (rest2.len(), 0),
        Some(sp) if sp.is_empty() => match crate::builtins::string_methods::find_bytes(rest2, b"\n\n", 0) { Some(p) => (p + 2, 2), None => (rest2.len(), 0) },
        Some(sp) => match crate::builtins::string_methods::find_bytes(rest2, sp, 0) { Some(p) => (p + sp.len(), sp.len()), None => (rest2.len(), 0) },
    };
    let _ = rest;
    if let Some(l) = limit {
        if l > 0 && (l as usize) < end {
            // do not split a character
            let mut e = l as usize;
            let off = char_offsets_of(rest2, enc);
            if let Some(&b) = off.iter().find(|&&o| o >= e) { e = b; }
            if e < end { end = e; sep_len = 0; }
        }
    }
    let mut line = rest2[..end].to_vec();
    let mut adv = st + end;
    if paragraph && sep_len == 2 { while adv < all.len() && all[adv] == b'\n' { adv += 1; } }
    if chomp && sep_len > 0 {
        line.truncate(line.len() - sep_len);
        if sep.as_deref() == Some(b"\n") && line.ends_with(b"\r") { line.pop(); }
    }
    d.pos = adv;
    d.lineno += 1;
    Ok(Some(str_from_vec_enc(line, enc)))
}
fn sio_gets(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let r = gets_impl(c, slf, a)?.unwrap_or(NIL);
    gvar_set_last_line(c, r);
    Ok(r)
}
fn gvar_set_last_line(c: &mut Ctx, v: Value) { let _ = crate::builtins::kernel::gvar_set(c, intern(b"$_"), v); }
fn sio_readline(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    match gets_impl(c, slf, a)? { Some(v) => Ok(v), None => err(c, vm().c_eof_error, "end of file reached") }
}
fn sio_readlines(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let out = ary_new();
    while let Some(l) = gets_impl(c, slf, a)? { ary_push(out, l); }
    Ok(out)
}
fn sio_each_line(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_line"), a, NIL); }
    while let Some(l) = gets_impl(c, slf, a)? { yield1(c, blk, l)?; }
    Ok(slf)
}
fn sio_getc(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let d = readable(c, slf)?;
    let all = bytes(d);
    if d.pos >= all.len() { return Ok(NIL); }
    let enc = read_enc(d);
    let l = enc_char_len(all, d.pos, enc);
    let r = str_new_enc(&all[d.pos..d.pos + l], enc);
    d.pos += l;
    Ok(r)
}
fn sio_readchar(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let r = sio_getc(c, slf, a, b)?;
    if r == NIL { return err(c, vm().c_eof_error, "end of file reached"); }
    Ok(r)
}
fn sio_getbyte(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let d = readable(c, slf)?;
    let all = bytes(d);
    if d.pos >= all.len() { return Ok(NIL); }
    d.pos += 1;
    Ok(Value::fix(all[d.pos - 1] as i64))
}
fn sio_readbyte(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let r = sio_getbyte(c, slf, a, b)?;
    if r == NIL { return err(c, vm().c_eof_error, "end of file reached"); }
    Ok(r)
}
fn sio_each_char(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_char"), a, NIL); }
    loop { let ch = sio_getc(c, slf, &[], std::ptr::null())?; if ch == NIL { break; } yield1(c, blk, ch)?; }
    Ok(slf)
}
fn sio_each_byte(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_byte"), a, NIL); }
    loop { let ch = sio_getbyte(c, slf, &[], std::ptr::null())?; if ch == NIL { break; } yield1(c, blk, ch)?; }
    Ok(slf)
}
fn sio_each_codepoint(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_codepoint"), a, NIL); }
    loop {
        let ch = sio_getc(c, slf, &[], std::ptr::null())?;
        if ch == NIL { break; }
        let cp = en::char_code(rstr(ch).enc, &rstr(ch).b);
        yield1(c, blk, Value::fix(cp as i64))?;
    }
    Ok(slf)
}
/// Insert bytes before the current position (ungetc/ungetbyte).
fn unget(c: &mut Ctx, slf: Value, bs: &[u8]) -> RV {
    let d = readable(c, slf)?;
    check_frozen(c, d.string)?;
    let r = rstr(d.string);
    if d.pos > r.b.len() { r.b.resize(d.pos, 0); }
    if d.pos >= bs.len() && &r.b[d.pos - bs.len()..d.pos] == bs { d.pos -= bs.len(); return Ok(NIL); }
    if d.pos >= bs.len() { let p = d.pos - bs.len(); r.b[p..d.pos].copy_from_slice(bs); d.pos = p; }
    else { let p = d.pos; r.b.splice(0..p, bs.iter().copied()); d.pos = 0; }
    Ok(NIL)
}
fn sio_ungetc(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let x = a[0];
    if x == NIL { return Ok(NIL); }
    let bs = if x.is_fixnum() { let enc = read_enc(sio(c, slf)?); en::code_to_bytes(enc, x.as_fix() as u32).unwrap_or(vec![x.as_fix() as u8]) } else { let s2 = expect_str(c, x)?; rstr(s2).b.clone() };
    unget(c, slf, &bs)
}
fn sio_ungetbyte(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let x = a[0];
    if x == NIL { return Ok(NIL); }
    let bs = if x.is_fixnum() { vec![x.as_fix() as u8] } else { let s2 = expect_str(c, x)?; rstr(s2).b.clone() };
    unget(c, slf, &bs)
}

// ---- writing ----------------------------------------------------------------------------------

fn write_str(c: &mut Ctx, slf: Value, s2: Value) -> Result<usize, Esc> {
    let d = writable(c, slf)?;
    let target = d.string;
    check_frozen(c, target)?;
    let mut data = rstr(s2).b.clone();
    if data.is_empty() { return Ok(0); }
    let (te, se) = (rstr(target).enc, rstr(s2).enc);
    if te != se && te != ENC_BINARY && !is_ascii(&data) {
        if rstr(target).b.is_empty() { rstr(target).enc = se; }
        else if te != ENC_ASCII && en::ascii_compat(te) && en::ascii_compat(se) { data = en::transcode(c, &data, se, te, &en::TcOpts::default())?; }
    }
    let r = rstr(target);
    if d.append { d.pos = r.b.len(); }
    if d.pos > r.b.len() { r.b.resize(d.pos, 0); }
    let end = (d.pos + data.len()).min(r.b.len());
    r.b.splice(d.pos..end, data.iter().copied());
    d.pos += data.len();
    Ok(data.len())
}
fn sio_write(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mut n = 0;
    for &x in a { let s2 = crate::rt::rt_tostr(c, x)?; n += write_str(c, slf, s2)?; }
    Ok(Value::fix(n as i64))
}
fn sio_ltlt(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { sio_write(c, slf, a, b)?; Ok(slf) }
fn sio_print(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if a.is_empty() { let l = gvar_get(c, intern(b"$_"))?; if l != NIL { sio_write(c, slf, &[l], b)?; } return Ok(NIL); }
    let ofs = gvar_get(c, intern(b"$,"))?;
    for (i, &x) in a.iter().enumerate() { if i > 0 && ofs != NIL { sio_write(c, slf, &[ofs], b)?; } sio_write(c, slf, &[x], b)?; }
    let ors = gvar_get(c, intern(b"$\\"))?;
    if ors != NIL { sio_write(c, slf, &[ors], b)?; }
    Ok(NIL)
}
fn sio_puts(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let out = puts_bytes(c, a)?;
    let s2 = str_from_vec(out);
    write_str(c, slf, s2)?;
    Ok(NIL)
}
fn sio_printf(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return Ok(NIL); }
    let s2 = crate::builtins::sprintf::format(c, a[0], &a[1..])?;
    write_str(c, slf, s2)?;
    Ok(NIL)
}
fn sio_putc(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let x = a[0];
    let s2 = if is_string(x) { let f = chars_of(x).into_iter().next().unwrap_or_default(); str_new_enc(&f, rstr(x).enc) } else { str_new(&[num_to_long(c, x)? as u8]) };
    write_str(c, slf, s2)?;
    Ok(x)
}
fn sio_truncate(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = num_to_long(c, a[0])?;
    if n < 0 { let e = make_exception(errno("EINVAL"), "Invalid argument - negative length"); return Err(raise_exc(c, e)); }
    let d = writable(c, slf)?;
    check_frozen(c, d.string)?;
    rstr(d.string).b.resize(n as usize, 0);
    Ok(Value::fix(0))
}

// ---- positioning / state ---------------------------------------------------------------------

fn sio_pos(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(sio(c, slf)?.pos as i64)) }
fn sio_set_pos(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = num_to_long(c, a[0])?;
    if p < 0 { let e = make_exception(errno("EINVAL"), "Invalid argument"); return Err(raise_exc(c, e)); }
    sio(c, slf)?.pos = p as usize;
    Ok(a[0])
}
fn sio_seek(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let off = num_to_long(c, a[0])?;
    let whence = match a.get(1) { None => 0, Some(&w) if w.is_sym() => match crate::symbol::sym_bytes(w.as_sym()) { b"SET" => 0, b"CUR" => 1, b"END" => 2, _ => return arg_err(c, "unknown whence") }, Some(&w) => num_to_long(c, w)? };
    let d = sio(c, slf)?;
    let base = match whence { 0 => 0, 1 => d.pos as i64, 2 => bytes(d).len() as i64, _ => { let e = make_exception(errno("EINVAL"), &format!("Invalid argument - invalid whence")); return Err(raise_exc(c, e)); } };
    let np = base + off;
    if np < 0 { let e = make_exception(errno("EINVAL"), "Invalid argument"); return Err(raise_exc(c, e)); }
    d.pos = np as usize;
    Ok(Value::fix(0))
}
fn sio_rewind(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let d = sio(c, slf)?; d.pos = 0; d.lineno = 0; Ok(Value::fix(0)) }
fn sio_eof(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let d = readable(c, slf)?; Ok(Value::bool(d.pos >= bytes(d).len())) }
fn sio_lineno(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(sio(c, slf)?.lineno)) }
fn sio_set_lineno(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let n = num_to_long(c, a[0])?; sio(c, slf)?.lineno = n; Ok(a[0]) }
fn sio_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let d = sio(c, slf)?; Ok(Value::fix(bytes(d).len() as i64)) }
fn sio_close(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let d = sio(c, slf)?; d.readable = false; d.writable = false; Ok(NIL) }
fn sio_close_read(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let d = sio(c, slf)?; if !d.readable { return err(c, vm().c_io_error, "closing non-duplex IO for reading"); } d.readable = false; Ok(NIL)
}
fn sio_close_write(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let d = sio(c, slf)?; if !d.writable { return err(c, vm().c_io_error, "closing non-duplex IO for writing"); } d.writable = false; Ok(NIL)
}
fn sio_closed(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let d = sio(c, slf)?; Ok(Value::bool(!d.readable && !d.writable)) }
fn sio_closed_read(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(!sio(c, slf)?.readable)) }
fn sio_closed_write(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(!sio(c, slf)?.writable)) }
fn sio_self(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(slf) }
fn sio_nil(_c: &mut Ctx, _slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(NIL) }
fn sio_zero(_c: &mut Ctx, _slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(0)) }
fn sio_true(_c: &mut Ctx, _slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(TRUE) }
fn sio_false(_c: &mut Ctx, _slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(FALSE) }
fn sio_binmode(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let d = sio(c, slf)?; d.enc = Some(ENC_BINARY);
    if !is_frozen(d.string) { rstr(d.string).enc = ENC_BINARY; }
    Ok(slf)
}
fn sio_external_encoding(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let d = sio(c, slf)?; Ok(en::encoding_value(read_enc(d))) }
fn sio_set_encoding(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let e = match a.first() { Some(&v) if v != NIL => Some(en::to_enc(c, v)?), _ => None };
    let d = sio(c, slf)?;
    let e = e.unwrap_or(en::default_external());
    d.enc = Some(e);
    if !is_frozen(d.string) && d.writable { rstr(d.string).enc = e; }
    Ok(slf)
}
fn sio_reopen(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { sio_initialize(c, slf, a, b)?; Ok(slf) }

pub fn init() {
    let vm = vm();
    let k = define_class("StringIO", vm.c_object);
    rclass(k).alloc = AllocKind::None;
    let io = vm.c_io;
    for (n, v) in [("SEEK_SET", 0), ("SEEK_CUR", 1), ("SEEK_END", 2)] {
        if const_get_at(io, intern(n.as_bytes())).is_none() { const_set(io, intern(n.as_bytes()), Value::fix(v)); }
    }
    let gr = define_module_under(io, "generic_readable");
    let gw = define_module_under(io, "generic_writable");
    include_module(k, vm.m_enumerable);
    include_module(k, gr);
    include_module(k, gw);
    const_set(k, intern(b"VERSION"), str_new(b"3.1.2"));
    define_singleton_cfunc(k, "new", sio_s_new, -1);
    define_singleton_cfunc(k, "open", sio_s_open, -1);
    let d = |n: &str, f: CFunc, ar: i32| define_cfunc(k, n, f, ar);
    define_private_cfunc(k, "initialize", sio_initialize, -1);
    d("string", sio_string, 0); d("string=", sio_set_string, 1); d("read", sio_read, -1); d("gets", sio_gets, -1);
    d("readlines", sio_readlines, -1); d("each_line", sio_each_line, -1); d("each", sio_each_line, -1); d("getc", sio_getc, 0); d("getbyte", sio_getbyte, 0);
    d("each_char", sio_each_char, 0); d("each_byte", sio_each_byte, 0); d("each_codepoint", sio_each_codepoint, 0); d("ungetc", sio_ungetc, 1); d("ungetbyte", sio_ungetbyte, 1);
    d("write", sio_write, -1); d("putc", sio_putc, 1); d("truncate", sio_truncate, 1);
    d("pos", sio_pos, 0); d("tell", sio_pos, 0); d("pos=", sio_set_pos, 1); d("seek", sio_seek, -1); d("rewind", sio_rewind, 0);
    d("eof?", sio_eof, 0); d("eof", sio_eof, 0); d("lineno", sio_lineno, 0); d("lineno=", sio_set_lineno, 1); d("size", sio_size, 0); d("length", sio_size, 0);
    d("close", sio_close, 0); d("close_read", sio_close_read, 0); d("close_write", sio_close_write, 0); d("closed?", sio_closed, 0);
    d("closed_read?", sio_closed_read, 0); d("closed_write?", sio_closed_write, 0); d("sync", sio_true, 0); d("sync=", sio_self, 1);
    d("flush", sio_self, 0); d("fsync", sio_zero, 0); d("fileno", sio_nil, 0); d("isatty", sio_false, 0); d("tty?", sio_false, 0); d("pid", sio_nil, 0);
    d("fcntl", sio_nil, -1); d("binmode", sio_binmode, 0); d("external_encoding", sio_external_encoding, 0); d("internal_encoding", sio_nil, 0);
    d("set_encoding", sio_set_encoding, -1); d("set_encoding_by_bom", sio_nil, 0); d("reopen", sio_reopen, -1);
    let r = |n: &str, f: CFunc, ar: i32| define_cfunc(gr, n, f, ar);
    r("readchar", sio_readchar, 0); r("readbyte", sio_readbyte, 0); r("readline", sio_readline, -1); r("sysread", sio_readpartial, -1);
    r("readpartial", sio_readpartial, -1); r("read_nonblock", sio_readpartial, -1);
    let w = |n: &str, f: CFunc, ar: i32| define_cfunc(gw, n, f, ar);
    w("<<", sio_ltlt, 1); w("print", sio_print, -1); w("puts", sio_puts, -1); w("printf", sio_printf, -1); w("syswrite", sio_write, -1); w("write_nonblock", sio_write, -1);
}
