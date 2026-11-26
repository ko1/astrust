// Zlib (ext/zlib/zlib.c of CRuby 4.0), native, over the Rust ports of zlib 1.3 in
// zlib_check.rs / zlib_deflate.rs / zlib_inflate.rs.
// A stream's state (`ZStream`, no Values) is taken out of its object while it runs, so Ruby code
// called meanwhile (a block, io.read) finds the object busy instead of a borrowed payload.
#![forbid(unsafe_code)]
use crate::object::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::builtins::zlib_check::*;
use crate::builtins::zlib_deflate::Deflate;
use crate::builtins::zlib_inflate::Inflate;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

const FLAG_READY: u32 = 1;
const FLAG_IN_STREAM: u32 = 2;
const FLAG_FINISHED: u32 = 4;
const FLAG_CLOSING: u32 = 8;
const FLAG_GZFILE: u32 = 16;
const FLAG_REUSE_BUFFER: u32 = 32;
const FLAG_IN_PROGRESS: u32 = 64;
const GZ_SYNC: u32 = 128;
const GZ_HEADER_FINISHED: u32 = 256;
const GZ_FOOTER_FINISHED: u32 = 512;
const GZ_MTIME_IS_SET: u32 = 1024;
const INITIAL_BUFSIZE: usize = 1024;
const STEP_MAX: usize = 16384;
const STEP_MIN: usize = 2048;
const GZFILE_READ_SIZE: i64 = 2048;

#[derive(Clone, Copy)]
struct Zk {
    m: Value, error: Value, stream_end: Value, need_dict: Value, data_error: Value, stream_error: Value, mem_error: Value,
    buf_error: Value, version_error: Value, in_progress: Value, deflate: Value, inflate: Value, gz_error: Value,
    no_footer: Value, crc_error: Value, length_error: Value, gzwriter: Value, gzreader: Value,
}
thread_local! { static ZK: std::cell::Cell<Option<Zk>> = const { std::cell::Cell::new(None) }; }
fn zk() -> Zk { ZK.with(|z| z.get()).expect("zlib loaded") }

enum Eng { None, Def(Box<Deflate>), Inf(Box<Inflate>) }
impl Clone for Eng {
    fn clone(&self) -> Eng { match self { Eng::None => Eng::None, Eng::Def(d) => Eng::Def(Box::new((**d).clone())), Eng::Inf(i) => Eng::Inf(i.clone()) } }
}

/// struct zstream; `buf`/`input` nil is None, `capa` stands for rb_str_capacity(buf).
#[derive(Clone)]
struct ZStream { flags: u32, buf: Option<Vec<u8>>, capa: usize, avail_out: usize, input: Option<Vec<u8>>, avail_in: usize, inflate: bool, eng: Eng }

impl ZStream {
    fn new(inflate: bool) -> ZStream { ZStream { flags: 0, buf: None, capa: 0, avail_out: 0, input: None, avail_in: 0, inflate, eng: Eng::None } }
    fn ready(&self) -> bool { self.flags & FLAG_READY != 0 }
    fn finished(&self) -> bool { self.flags & FLAG_FINISHED != 0 }
    fn gzfile(&self) -> bool { self.flags & FLAG_GZFILE != 0 }
    fn filled(&self) -> usize { self.buf.as_ref().map(|b| b.len()).unwrap_or(0) }
    fn total_in(&self) -> u64 { match &self.eng { Eng::Def(d) => d.total_in, Eng::Inf(i) => i.total_in, Eng::None => 0 } }
    fn total_out(&self) -> u64 { match &self.eng { Eng::Def(d) => d.total_out, Eng::Inf(i) => i.total_out, Eng::None => 0 } }
    fn set_total_in(&mut self, n: u64) { match &mut self.eng { Eng::Def(d) => d.total_in = n, Eng::Inf(i) => i.total_in = n, Eng::None => {} } }
    fn adler(&self) -> u32 { match &self.eng { Eng::Def(d) => d.adler, Eng::Inf(i) => i.adler, Eng::None => 0 } }
    fn data_type(&self) -> i32 { match &self.eng { Eng::Def(d) => d.data_type, Eng::Inf(i) => i.data_type, Eng::None => 0 } }
    fn msg(&self) -> Option<&'static str> { match &self.eng { Eng::Def(d) => d.msg, Eng::Inf(i) => i.msg, Eng::None => None } }

    fn expand_buffer_into(&mut self, size: usize) {
        match &self.buf {
            None => { self.buf = Some(Vec::with_capacity(size)); self.capa = size; self.avail_out = size; }
            Some(b) => if self.avail_out != size { self.capa = b.len() + size; self.avail_out = size; },
        }
    }
    fn expand_non_stream(&mut self) {
        let len = self.filled();
        if self.capa.saturating_sub(len) >= STEP_MAX { self.avail_out = STEP_MAX; }
        else {
            let inc = (len / 2).max(STEP_MIN);
            self.capa = len + inc;
            self.avail_out = inc.min(STEP_MAX);
        }
    }
    fn append_buffer(&mut self, src: &[u8]) {
        match &mut self.buf {
            None => { self.buf = Some(src.to_vec()); self.capa = src.len(); self.avail_out = 0; }
            Some(b) => {
                if self.capa < b.len() + src.len() { self.capa = b.len() + src.len(); self.avail_out = 0; }
                else if self.avail_out >= src.len() { self.avail_out -= src.len(); } else { self.avail_out = 0; }
                b.extend_from_slice(src);
            }
        }
    }
    /// zstream_shift_buffer's buffer part: the first `len` bytes (or all).
    fn shift(&mut self, len: usize) -> Vec<u8> {
        let b = self.buf.get_or_insert_with(Vec::new);
        let out: Vec<u8> = if b.len() <= len { std::mem::take(b) } else { let rest = b.split_off(len); std::mem::replace(b, rest) };
        let free = self.capa.saturating_sub(self.filled());
        self.avail_out = free.min(STEP_MAX);
        out
    }
    fn ungets(&mut self, s: &[u8]) {
        self.expand_buffer_into(s.len());
        let b = self.buf.as_mut().unwrap();
        b.splice(0..0, s.iter().copied());
        if self.avail_out > 0 { let n = s.len().min(self.avail_out); self.avail_out -= n; }
    }
    fn append_input(&mut self, src: &[u8]) {
        if src.is_empty() { return; }
        match &mut self.input { None => self.input = Some(src.to_vec()), Some(i) => i.extend_from_slice(src) }
    }
    fn discard_input(&mut self, len: usize) {
        if let Some(i) = &mut self.input {
            if i.len() <= len { self.input = None; } else { i.drain(..len); }
        }
    }
    fn reset_input(&mut self) { self.input = None; }
    fn passthrough_input(&mut self) { if let Some(i) = self.input.take() { self.append_buffer(&i); } }
    fn detach_input(&mut self) -> Vec<u8> { self.input.take().unwrap_or_default() }
    fn input_len(&self) -> usize { self.input.as_ref().map(|i| i.len()).unwrap_or(0) }
    fn eng_reset(&mut self) { match &mut self.eng { Eng::Def(d) => d.reset(), Eng::Inf(i) => i.reset(), Eng::None => {} } }
    fn eng_end(&mut self) -> i32 {
        let r = match &self.eng { Eng::Def(d) => d.end_status(), _ => Z_OK };
        self.eng = Eng::None;
        r
    }
    fn reset(&mut self) {
        self.eng_reset();
        self.flags = FLAG_READY;
        self.buf = None;
        self.avail_out = 0;
        self.reset_input();
    }
}

fn raise_zlib_error<T>(c: &mut Ctx, code: i32, msg: Option<&str>) -> Result<T, Esc> {
    let k = zk();
    let m = msg.unwrap_or_else(|| err_msg(code)).to_string();
    let cls = match code {
        Z_STREAM_END => k.stream_end, Z_NEED_DICT => k.need_dict, Z_STREAM_ERROR => k.stream_error, Z_DATA_ERROR => k.data_error,
        Z_BUF_ERROR => k.buf_error, Z_VERSION_ERROR => k.version_error, Z_MEM_ERROR => k.mem_error,
        _ => return err(c, k.error, format!("unknown zlib error {}: {}", code, m)),
    };
    err(c, cls, m)
}

/// rb_warning: only under $VERBOSE = true, with the caller's location.
fn rb_warning(c: &mut Ctx, msg: &str) -> Result<(), Esc> {
    if c.vm().verbose.get() != TRUE { return Ok(()); }
    let loc = backtrace_lines(c, 0).first().map(|l| l.split(":in ").next().unwrap_or("").to_string()).unwrap_or_default();
    warning_write(c, format!("{}: warning: {}\n", loc, msg).as_bytes())
}
fn bin(c: &mut Ctx, b: Vec<u8>) -> Value { c.str_from_vec_enc(b, ENC_BINARY) }
fn uint(c: &mut Ctx, v: u64) -> Value { c.int_from_i64(v as i64) }

/// NUM2ULONG.
fn to_ulong(c: &mut Ctx, v: Value) -> Result<u64, Esc> {
    if is_bignum(v) {
        let b = c.big(v);
        if !b.neg && b.mag.len() <= 2 { return Ok(b.mag.iter().rev().fold(0u64, |a, &w| (a << 32) | w as u64)); }
        if !b.neg { return err(c, c.vm().c_range_error.get(), "bignum out of range of unsigned long"); }
    }
    Ok(num_to_long(c, v)? as u64)
}
/// FIX2INT-style argument with a default for nil.
fn int_arg(c: &mut Ctx, v: Option<&Value>, dflt: i32) -> Result<i32, Esc> {
    match v { None => Ok(dflt), Some(&v) if v == NIL => Ok(dflt), Some(&v) => Ok(num_to_long(c, v)? as i32) }
}

// ---- RData payloads ----------------------------------------------------------------------

/// Zlib::Deflate / Zlib::Inflate: `z` is None while the stream runs.
struct RZ { z: Option<Box<ZStream>>, reuse: Value }
impl RData for RZ {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.reuse); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// The non-Value part of struct gzfile.
#[derive(Clone)]
struct GzCore { z: ZStream, level: i32, os_code: u8, mtime: u32, crc: u32, lineno: i32, ungetc: i64, enc: u8, enc2: Option<u8>, writer: bool }
impl GzCore {
    fn new(inflate: bool) -> GzCore {
        let mut z = ZStream::new(inflate);
        z.flags |= FLAG_GZFILE;
        GzCore { z, level: 0, os_code: OS_CODE, mtime: 0, crc: 0, lineno: 0, ungetc: 0, enc: crate::builtins::encoding::default_external(), enc2: None, writer: !inflate }
    }
    fn finished(&self) -> bool { self.z.finished() && self.z.filled() == 0 }
}
struct RGz { g: Option<Box<GzCore>>, io: Value, orig_name: Value, comment: Value, ecopts: Value, path: Value }
impl RData for RGz {
    fn visit(&mut self, m: &mut crate::gc::Visitor) {
        m.visit(&mut self.io); m.visit(&mut self.orig_name); m.visit(&mut self.comment); m.visit(&mut self.ecopts); m.visit(&mut self.path);
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let Some(k) = ZK.with(|z| z.get()) else { return Ok(None) };
    if crate::builtins::inherits(c, klass, k.deflate) || crate::builtins::inherits(c, klass, k.inflate) {
        let inf = crate::builtins::inherits(c, klass, k.inflate);
        let o = c.alloc(klass, Body::Data(Box::new(RZ { z: Some(Box::new(ZStream::new(inf))), reuse: NIL })));
        if inf { let h = c.hash_new(); ivar_set_raw(c, o, intern(b"@dictionaries"), h); }
        return Ok(Some(o));
    }
    if crate::builtins::inherits(c, klass, k.gzwriter) || crate::builtins::inherits(c, klass, k.gzreader) {
        let inf = crate::builtins::inherits(c, klass, k.gzreader);
        return Ok(Some(c.alloc(klass, Body::Data(Box::new(RGz { g: Some(Box::new(GzCore::new(inf))), io: NIL, orig_name: NIL, comment: NIL, ecopts: NIL, path: NIL })))));
    }
    Ok(None)
}
/// dup/clone: a fresh object of the class; initialize_copy fills it.
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if c.data::<RZ>(v).is_some() || c.data::<RGz>(v).is_some() { return alloc_data(c, real_class_of(c, v)); }
    Ok(None)
}

fn in_progress<T>(c: &mut Ctx) -> Result<T, Esc> { err(c, zk().in_progress, "zlib stream is in progress") }
/// TypedData_Get_Struct: the stream, taken out.
fn take_z(c: &mut Ctx, ov: Value) -> Result<Box<ZStream>, Esc> {
    match c.data_mut::<RZ>(ov) {
        Some(r) => match r.z.take() { Some(z) => Ok(z), None => in_progress(c) },
        None => type_err(c, format!("wrong argument type {} (expected zstream)", class_name_for_err(c, ov))),
    }
}
fn put_z(c: &mut Ctx, ov: Value, z: Box<ZStream>) { if let Some(r) = c.data_mut::<RZ>(ov) { r.z = Some(z); } }
/// Run `f` on the stream of `ov`; `ready` is get_zstream's check.
fn with_z(c: &mut Ctx, ov: Value, ready: bool, f: impl FnOnce(&mut Ctx, &mut ZStream) -> RV) -> RV {
    let mut z = take_z(c, ov)?;
    if ready && !z.ready() { put_z(c, ov, z); return err(c, zk().error, "stream is not ready"); }
    let r = f(c, &mut z);
    put_z(c, ov, z);
    r
}

// ---- zstream run ------------------------------------------------------------------------

/// Call deflate()/inflate() once on avail_out bytes after the buffer.
fn run_once(z: &mut ZStream, input: &[u8], ip: &mut usize, flush: i32) -> i32 {
    let n = z.avail_out;
    let buf = z.buf.get_or_insert_with(Vec::new);
    let filled = buf.len();
    buf.resize(filled + n, 0);
    let mut io = ZIo { input, ip: *ip, out: &mut buf[filled..], op: 0 };
    let err = match &mut z.eng {
        Eng::Def(d) => d.deflate(&mut io, flush),
        Eng::Inf(i) => i.inflate(&mut io, flush),
        Eng::None => Z_STREAM_ERROR,
    };
    *ip = io.ip;
    let w = io.op;
    buf.truncate(filled + w);
    if z.capa < buf.len() { z.capa = buf.len(); }
    z.avail_out = n - w;
    err
}

fn yield_buf(c: &mut Ctx, z: &mut ZStream, blk: Blk, reuse: Value) -> Result<(), Esc> {
    let data = z.buf.take().unwrap_or_default();
    let v = detach_value(c, z, data, reuse)?;
    if z.flags & FLAG_REUSE_BUFFER != 0 { z.buf = Some(Vec::new()); }
    let r = yield1(c, blk, v);
    r.map(|_| ())
}
/// zstream_expand_buffer (may yield a full buffer to the block).
fn expand_buffer(c: &mut Ctx, z: &mut ZStream, blk: Blk, reuse: Value) -> Result<(), Esc> {
    if z.buf.is_none() { z.expand_buffer_into(INITIAL_BUFSIZE); return Ok(()); }
    if !z.gzfile() && !blk.is_null() {
        let filled = z.filled();
        if filled >= STEP_MAX {
            let r = yield_buf(c, z, blk, reuse);
            z.expand_buffer_into(STEP_MAX);
            return r;
        }
        z.expand_buffer_into(STEP_MAX - filled);
    } else {
        z.expand_non_stream();
    }
    Ok(())
}
/// The Ruby String for a detached buffer: a new binary String, or the reused one.
fn detach_value(c: &mut Ctx, z: &ZStream, data: Vec<u8>, reuse: Value) -> RV {
    if z.flags & FLAG_REUSE_BUFFER != 0 && reuse != NIL {
        let s = c.str_modify(reuse)?;
        s.b = crate::sbuf::SBuf::from_vec(data);
        return Ok(reuse);
    }
    Ok(bin(c, data))
}

/// Who is running: the object (for @dictionaries), its block, and a reused output String.
#[derive(Clone, Copy)]
struct RunCx<'a> { obj: Value, blk: Blk<'a>, reuse: Value }
const NOCX: RunCx = RunCx { obj: NIL, blk: Blk::NONE, reuse: NIL };

fn zstream_run(c: &mut Ctx, z: &mut ZStream, src: &[u8], flush: i32, cx: RunCx) -> Result<(), Esc> {
    if z.flags & FLAG_IN_PROGRESS != 0 { return in_progress(c); }
    z.flags |= FLAG_IN_PROGRESS;
    let r = zstream_run_try(c, z, src, flush, cx);
    z.flags &= !FLAG_IN_PROGRESS;
    r
}
fn zstream_run_try(c: &mut Ctx, z: &mut ZStream, src: &[u8], flush: i32, cx: RunCx) -> Result<(), Esc> {
    let stream_output = !z.gzfile() && !cx.blk.is_null();
    let input: Vec<u8> = if z.input.is_none() && src.is_empty() { Vec::new() } else { z.append_input(src); z.detach_input() };
    let mut ip = 0usize;
    if z.avail_out == 0 { expand_buffer(c, z, cx.blk, cx.reuse)?; }
    let mut jump: Option<Esc> = None;
    loop {
        let mut err;
        loop {
            err = run_once(z, &input, &mut ip, flush);
            if err == Z_STREAM_END {
                z.flags &= !FLAG_IN_STREAM;
                z.flags |= FLAG_FINISHED;
                break;
            }
            if err != Z_OK && err != Z_BUF_ERROR { break; }
            if z.avail_out > 0 { z.flags |= FLAG_IN_STREAM; break; }
            if input.len() == ip && z.inflate { z.flags |= FLAG_IN_STREAM; break; }
            if stream_output {
                if let Err(e) = expand_buffer(c, z, cx.blk, cx.reuse) { err = Z_OK; jump = Some(e); break; }
            } else { z.expand_non_stream(); }
        }
        if flush != Z_FINISH && err == Z_BUF_ERROR && z.avail_out > 0 { z.flags |= FLAG_IN_STREAM; }
        z.reset_input();
        z.avail_in = input.len() - ip;
        if err != Z_OK && err != Z_STREAM_END {
            if z.avail_in > 0 { z.append_input(&input[ip..]); }
            if err == Z_NEED_DICT && cx.obj != NIL {
                let dicts = ivar_get(c, cx.obj, intern(b"@dictionaries"));
                if is_hash(dicts) {
                    let key = uint(c, z.adler() as u64);
                    let dict = crate::hash::hash_aref(c, dicts, key)?;
                    if dict != NIL {
                        inflate_set_dictionary(c, z, dict)?;
                        continue;
                    }
                }
            }
            let m = z.msg();
            return raise_zlib_error(c, err, m);
        }
        if z.avail_in > 0 { z.append_input(&input[ip..]); }
        break;
    }
    match jump { Some(e) => Err(e), None => Ok(()) }
}

/// zstream_detach_buffer: the output so far (nil when it goes to the block later).
fn detach_buffer(c: &mut Ctx, z: &mut ZStream, cx: RunCx) -> RV {
    if !z.finished() && !z.gzfile() && !cx.blk.is_null() { return Ok(NIL); }
    let data = z.buf.take().unwrap_or_default();
    let dst = detach_value(c, z, data, cx.reuse)?;
    z.avail_out = 0;
    if !z.gzfile() && !cx.blk.is_null() { yield1(c, cx.blk, dst)?; return Ok(NIL); }
    Ok(dst)
}

fn zstream_end(c: &mut Ctx, z: &mut ZStream) -> Result<(), Esc> {
    if !z.ready() { rb_warning(c, "attempt to close uninitialized zstream; ignored.")?; return Ok(()); }
    if z.flags & FLAG_IN_STREAM != 0 {
        rb_warning(c, "attempt to close unfinished zstream; reset forced.")?;
        z.reset();
    }
    z.reset_input();
    let err = z.eng_end();
    if err != Z_OK { return raise_zlib_error(c, err, None); }
    z.flags = 0;
    Ok(())
}

fn inflate_set_dictionary(c: &mut Ctx, z: &mut ZStream, dic: Value) -> Result<(), Esc> {
    let src = expect_str(c, dic)?;
    let d = c.str_bytes(src).to_vec();
    let err = match &mut z.eng { Eng::Inf(i) => i.set_dictionary(&d), _ => Z_STREAM_ERROR };
    if err != Z_OK { let m = z.msg(); return raise_zlib_error(c, err, m); }
    Ok(())
}

// ---- Zlib module functions ----------------------------------------------------------------

fn do_checksum(c: &mut Ctx, a: &[Value], f: fn(u32, &[u8]) -> u32, init: u32) -> RV {
    check_argc(c, a, 0, 2)?;
    let str = a.first().copied().unwrap_or(NIL);
    let vsum = a.get(1).copied().unwrap_or(NIL);
    let mut sum: u32 = if vsum != NIL { to_ulong(c, vsum)? as u32 } else if str == NIL { 0 } else { init };
    if str == NIL {
        sum = init;   // crc32/adler32 with a NULL buffer return their initial value
    } else if is_kind_of(c, str, c.vm().c_io.get()) {
        loop {
            let b = funcall(c, str, intern(b"read"), &[Value::fix(8192)], std::ptr::null())?;
            if b == NIL { break; }
            let b = expect_str(c, b)?;
            sum = f(sum, c.str_bytes(b));
        }
    } else {
        let s = expect_str(c, str)?;
        sum = f(sum, c.str_bytes(s));
    }
    Ok(uint(c, sum as u64))
}
fn z_adler32(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { do_checksum(c, a, adler32, 1) }
fn z_crc32(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { do_checksum(c, a, crc32, 0) }
fn z_adler32_combine(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (x, y, l) = (to_ulong(c, a[0])?, to_ulong(c, a[1])?, num_to_long(c, a[2])?);
    Ok(uint(c, adler32_combine(x as u32, y as u32, l) as u64))
}
fn z_crc32_combine(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (x, y, l) = (to_ulong(c, a[0])?, to_ulong(c, a[1])?, num_to_long(c, a[2])?);
    Ok(uint(c, crc32_combine(x as u32, y as u32, l) as u64))
}
fn z_crc_table(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let v: Vec<Value> = crc_table().iter().map(|&x| Value::fix(x as i64)).collect();
    Ok(c.ary_new_from(&v))
}
fn z_zlib_version(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.str_new_enc(b"1.3.1", ENC_BINARY)) }   // zlibVersion() of the system zlib CRuby runs with

// ---- ZStream ------------------------------------------------------------------------------

fn zs_end(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_z(c, s, true, |c, z| { zstream_end(c, z)?; Ok(NIL) }) }
fn zs_reset(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    with_z(c, s, true, |_c, z| { z.reset(); Ok(NIL) })?;
    if let Some(r) = c.data_mut::<RZ>(s) { r.reuse = NIL; }
    Ok(NIL)
}
fn cx_of<'a>(c: &Ctx, s: Value, blk: Blk<'a>) -> RunCx<'a> { RunCx { obj: s, blk, reuse: c.data::<RZ>(s).map(|r| r.reuse).unwrap_or(NIL) } }
fn zs_finish(c: &mut Ctx, s: Value, _a: &[Value], blk: Blk) -> RV {
    let cx = cx_of(c, s, blk);
    with_z(c, s, true, |c, z| { zstream_run(c, z, b"", Z_FINISH, cx)?; detach_buffer(c, z, cx) })
}
fn zs_flush_next_in(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_z(c, s, false, |c, z| { let v = z.detach_input(); Ok(bin(c, v)) }) }
fn zs_flush_next_out(c: &mut Ctx, s: Value, _a: &[Value], blk: Blk) -> RV {
    let cx = cx_of(c, s, blk);
    with_z(c, s, false, |c, z| detach_buffer(c, z, cx))
}
fn zs_avail_out(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_z(c, s, false, |c, z| Ok(uint(c, z.avail_out as u64))) }
fn zs_set_avail_out(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let n = num_to_long(c, a[0])?;
    with_z(c, s, true, |_c, z| { z.expand_buffer_into(n as u32 as usize); Ok(NIL) })?;
    Ok(a[0])
}
fn zs_avail_in(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_z(c, s, false, |_c, z| Ok(Value::fix(z.input_len() as i32 as i64))) }
fn zs_total_in(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_z(c, s, true, |c, z| Ok(uint(c, z.total_in()))) }
fn zs_total_out(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_z(c, s, true, |c, z| Ok(uint(c, z.total_out()))) }
fn zs_data_type(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_z(c, s, true, |_c, z| Ok(Value::fix(z.data_type() as i64))) }
fn zs_adler(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_z(c, s, true, |c, z| Ok(uint(c, z.adler() as u64))) }
fn zs_finished_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_z(c, s, true, |_c, z| Ok(Value::bool(z.finished()))) }
fn zs_closed_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_z(c, s, false, |_c, z| Ok(Value::bool(!z.ready()))) }

// ---- Deflate ------------------------------------------------------------------------------

fn deflate_init(c: &mut Ctx, level: i32, wbits: i32, memlevel: i32, strategy: i32) -> Result<Box<Deflate>, Esc> {
    match Deflate::new(level, Z_DEFLATED, wbits, memlevel, strategy) { Ok(d) => Ok(d), Err(e) => raise_zlib_error(c, e, None) }
}
fn def_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 4)?;
    let level = int_arg(c, a.first(), -1)?;
    let wbits = int_arg(c, a.get(1), MAX_WBITS)?;
    let memlevel = int_arg(c, a.get(2), DEF_MEM_LEVEL)?;
    let strategy = int_arg(c, a.get(3), Z_DEFAULT_STRATEGY)?;
    let d = deflate_init(c, level, wbits, memlevel, strategy)?;
    with_z(c, s, false, |_c, z| { z.eng = Eng::Def(d); z.flags |= FLAG_READY; Ok(NIL) })?;
    Ok(s)
}
fn def_init_copy(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    if s == a[0] { return Ok(s); }
    let z2 = take_z(c, a[0])?;
    if !z2.ready() { put_z(c, a[0], z2); return err(c, zk().error, "stream is not ready"); }
    let copy = (*z2).clone();
    put_z(c, a[0], z2);
    with_z(c, s, false, |_c, z| { *z = copy; Ok(NIL) })?;
    Ok(s)
}

fn level_of(c: &mut Ctx, v: Option<&Value>) -> Result<i32, Esc> { int_arg(c, v, -1) }
fn def_s_deflate(c: &mut Ctx, _s: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let lev = level_of(c, a.get(1))?;
    let src = expect_str(c, a[0])?;
    let d = deflate_init(c, lev, MAX_WBITS, DEF_MEM_LEVEL, Z_DEFAULT_STRATEGY)?;
    let mut z = ZStream::new(false);
    z.eng = Eng::Def(d);
    z.flags |= FLAG_READY;
    let bytes = c.str_bytes(src).to_vec();
    let cx = RunCx { obj: NIL, blk, reuse: NIL };
    let r = zstream_run(c, &mut z, &bytes, Z_FINISH, cx).and_then(|_| detach_buffer(c, &mut z, cx));
    let (ek, ev) = (c.esc, c.esc_val);
    let e = zstream_end(c, &mut z);
    if r.is_err() { c.esc = ek; c.esc_val = ev; return r; }
    e?;
    r
}
fn do_deflate(c: &mut Ctx, z: &mut ZStream, src: Value, flush: i32, cx: RunCx) -> Result<(), Esc> {
    if src == NIL { return zstream_run(c, z, b"", Z_FINISH, cx); }
    let s = expect_str(c, src)?;
    if flush != Z_NO_FLUSH || c.str_len(s) > 0 {
        let b = c.str_bytes(s).to_vec();
        zstream_run(c, z, &b, flush, cx)?;
    }
    Ok(())
}
fn def_deflate(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let flush = int_arg(c, a.get(1), Z_NO_FLUSH)?;
    let cx = cx_of(c, s, blk);
    with_z(c, s, true, |c, z| { do_deflate(c, z, a[0], flush, cx)?; detach_buffer(c, z, cx) })
}
fn def_addstr(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    let cx = cx_of(c, s, blk);
    with_z(c, s, true, |c, z| { do_deflate(c, z, a[0], Z_NO_FLUSH, cx)?; Ok(NIL) })?;
    Ok(s)
}
fn def_flush(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let flush = int_arg(c, a.first(), Z_SYNC_FLUSH)?;
    let cx = cx_of(c, s, blk);
    with_z(c, s, true, |c, z| {
        if flush != Z_NO_FLUSH { zstream_run(c, z, b"", flush, cx)?; }
        detach_buffer(c, z, cx)
    })
}
fn def_params(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    let level = int_arg(c, a.first(), -1)?;
    let strategy = int_arg(c, a.get(1), Z_DEFAULT_STRATEGY)?;
    let cx = cx_of(c, s, blk);
    with_z(c, s, true, |c, z| {
        let mut err = params_once(z, level, strategy);
        while err == Z_BUF_ERROR {
            rb_warning(c, "deflateParams() returned Z_BUF_ERROR")?;
            expand_buffer(c, z, cx.blk, cx.reuse)?;
            err = params_once(z, level, strategy);
        }
        if err != Z_OK { let m = z.msg(); return raise_zlib_error(c, err, m); }
        Ok(NIL)
    })
}
/// deflateParams on avail_out bytes after the buffer (it may flush a block).
fn params_once(z: &mut ZStream, level: i32, strategy: i32) -> i32 {
    let null = z.buf.is_none();
    if let Eng::Def(d) = &mut z.eng { d.next_out_null = null; }
    let n = z.avail_out;
    let mut tmp = Vec::new();
    let buf = z.buf.as_mut().unwrap_or(&mut tmp);
    let filled = buf.len();
    buf.resize(filled + n, 0);
    let mut io = ZIo { input: &[], ip: 0, out: &mut buf[filled..], op: 0 };
    let err = match &mut z.eng { Eng::Def(d) => { let e = d.params(&mut io, level, strategy); d.next_out_null = false; e } _ => Z_STREAM_ERROR };
    let w = io.op;
    buf.truncate(filled + w);
    if z.capa < buf.len() { z.capa = buf.len(); }
    z.avail_out = n - w;
    err
}
fn def_set_dictionary(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let src = expect_str(c, a[0])?;
    let d = c.str_bytes(src).to_vec();
    with_z(c, s, true, |c, z| {
        let err = match &mut z.eng { Eng::Def(df) => df.set_dictionary(&d), _ => Z_STREAM_ERROR };
        if err != Z_OK { let m = z.msg(); return raise_zlib_error(c, err, m); }
        Ok(NIL)
    })?;
    Ok(a[0])
}

// ---- Inflate ------------------------------------------------------------------------------

fn inflate_init(c: &mut Ctx, wbits: i32) -> Result<Box<Inflate>, Esc> {
    match Inflate::new(wbits) { Ok(i) => Ok(i), Err(e) => raise_zlib_error(c, e, None) }
}
fn inf_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let wbits = int_arg(c, a.first(), MAX_WBITS)?;
    let i = inflate_init(c, wbits)?;
    with_z(c, s, false, |_c, z| { z.eng = Eng::Inf(i); z.flags |= FLAG_READY; Ok(NIL) })?;
    Ok(s)
}
fn inf_s_inflate(c: &mut Ctx, _s: Value, a: &[Value], blk: Blk) -> RV {
    let src = expect_str(c, a[0])?;
    let i = inflate_init(c, MAX_WBITS)?;
    let mut z = ZStream::new(true);
    z.eng = Eng::Inf(i);
    z.flags |= FLAG_READY;
    let bytes = c.str_bytes(src).to_vec();
    let cx = RunCx { obj: NIL, blk, reuse: NIL };
    let r = zstream_run(c, &mut z, &bytes, Z_SYNC_FLUSH, cx).and_then(|_| zstream_run(c, &mut z, b"", Z_FINISH, cx)).and_then(|_| detach_buffer(c, &mut z, cx));
    let (ek, ev) = (c.esc, c.esc_val);
    let e = zstream_end(c, &mut z);
    if r.is_err() { c.esc = ek; c.esc_val = ev; return r; }
    e?;
    r
}
fn do_inflate(c: &mut Ctx, z: &mut ZStream, src: Value, cx: RunCx) -> Result<(), Esc> {
    if src == NIL { return zstream_run(c, z, b"", Z_FINISH, cx); }
    let s = expect_str(c, src)?;
    if c.str_len(s) > 0 || z.avail_in > 0 {
        let b = c.str_bytes(s).to_vec();
        zstream_run(c, z, &b, Z_SYNC_FLUSH, cx)?;
    }
    Ok(())
}
fn inf_add_dictionary(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let dicts = ivar_get(c, s, intern(b"@dictionaries"));
    let sum = do_checksum(c, &a[..1], adler32, 1)?;
    crate::hash::hash_aset(c, dicts, sum, a[0])?;
    Ok(s)
}
fn inf_inflate(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    let (pos, opts) = match a.last() { Some(&h) if is_hash(h) && !a.is_empty() => (&a[..a.len() - 1], Some(h)), _ => (a, None) };
    let mut buffer = NIL;
    if let Some(h) = opts {
        if let Some(b) = crate::hash::hash_get_opt(c, h, sym_value("buffer"))? { if b != NIL { buffer = expect_str(c, b)?; } }
    }
    check_argc(c, pos, 1, 1)?;
    let src = pos[0];
    // ZSTREAM_REUSE_BUFFER: the output goes to `buffer`
    let (old_reuse, inited) = match c.data::<RZ>(s) { Some(r) => (r.reuse, r.z.is_some()), None => (NIL, false) };
    if !inited { return in_progress(c); }
    if buffer != NIL {
        let reusing = with_z(c, s, true, |_c, z| Ok(Value::bool(z.flags & FLAG_REUSE_BUFFER != 0)))?.truthy() && old_reuse == buffer;
        if !reusing {
            c.str_modify(buffer)?;
            let len = c.str_len(buffer);
            let capa = if len >= STEP_MAX { len } else { STEP_MAX };
            c.rstring_mut(buffer).b.truncate(0);
            with_z(c, s, true, |_c, z| { z.flags |= FLAG_REUSE_BUFFER; z.buf = Some(Vec::new()); z.capa = capa; Ok(NIL) })?;
            if let Some(r) = c.data_mut::<RZ>(s) { r.reuse = buffer; }
        }
    } else {
        let was = with_z(c, s, true, |_c, z| {
            let w = z.flags & FLAG_REUSE_BUFFER != 0;
            if w { z.flags &= !FLAG_REUSE_BUFFER; z.buf = None; }
            Ok(Value::bool(w))
        })?;
        if was.truthy() { if let Some(r) = c.data_mut::<RZ>(s) { r.reuse = NIL; } }
    }
    let cx = cx_of(c, s, blk);
    with_z(c, s, true, |c, z| {
        if z.finished() {
            if src == NIL { return detach_buffer(c, z, cx); }
            let sv = expect_str(c, src)?;
            let b = c.str_bytes(sv).to_vec();
            z.append_buffer(&b);
            if z.flags & FLAG_REUSE_BUFFER != 0 { c.str_modify(buffer)?.b.truncate(0); return Ok(buffer); }
            return Ok(c.str_new_enc(b"", ENC_BINARY));
        }
        do_inflate(c, z, src, cx)?;
        let dst = detach_buffer(c, z, cx)?;
        if z.finished() { z.passthrough_input(); }
        Ok(dst)
    })
}
fn inf_addstr(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    let cx = cx_of(c, s, blk);
    with_z(c, s, true, |c, z| {
        if z.finished() {
            if a[0] != NIL { let sv = expect_str(c, a[0])?; let b = c.str_bytes(sv).to_vec(); z.append_buffer(&b); }
        } else {
            do_inflate(c, z, a[0], cx)?;
            if z.finished() { z.passthrough_input(); }
        }
        Ok(NIL)
    })?;
    Ok(s)
}
fn sync_engine(z: &mut ZStream, data: &[u8]) -> (i32, usize) {
    let mut empty: [u8; 0] = [];
    let mut io = ZIo { input: data, ip: 0, out: &mut empty, op: 0 };
    let err = match &mut z.eng { Eng::Inf(i) => i.sync(&mut io), _ => Z_STREAM_ERROR };
    (err, data.len() - io.ip)
}
fn inf_sync(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let src = expect_str(c, a[0])?;
    let bytes = c.str_bytes(src).to_vec();
    with_z(c, s, true, |c, z| {
        if let Some(inp) = z.input.clone() {
            let (err, rest) = sync_engine(z, &inp);
            z.avail_in = rest;
            if err == Z_OK {
                z.discard_input(inp.len() - rest);
                z.append_input(&bytes);
                return Ok(TRUE);
            }
            z.reset_input();
            if err != Z_DATA_ERROR { let m = z.msg(); return raise_zlib_error(c, err, m); }
        }
        if bytes.is_empty() { return Ok(FALSE); }
        let (err, rest) = sync_engine(z, &bytes);
        z.avail_in = rest;
        if err == Z_OK { z.append_input(&bytes[bytes.len() - rest..]); return Ok(TRUE); }
        if err != Z_DATA_ERROR { let m = z.msg(); return raise_zlib_error(c, err, m); }
        Ok(FALSE)
    })
}
fn inf_sync_point_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    with_z(c, s, true, |_c, z| Ok(Value::bool(matches!(&z.eng, Eng::Inf(i) if i.sync_point()))))
}
fn inf_set_dictionary(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    with_z(c, s, true, |c, z| { inflate_set_dictionary(c, z, a[0])?; Ok(NIL) })?;
    Ok(a[0])
}

// ---- GzipFile -----------------------------------------------------------------------------

fn gz_vals(c: &Ctx, ov: Value) -> (Value, Value, Value) {
    match c.data::<RGz>(ov) { Some(r) => (r.io, r.orig_name, r.comment), None => (NIL, NIL, NIL) }
}
fn gz_io(c: &Ctx, ov: Value) -> Value { gz_vals(c, ov).0 }
fn take_gz(c: &mut Ctx, ov: Value) -> Result<Box<GzCore>, Esc> {
    match c.data_mut::<RGz>(ov) {
        Some(r) => match r.g.take() { Some(g) => Ok(g), None => in_progress(c) },
        None => type_err(c, format!("wrong argument type {} (expected gzfile)", class_name_for_err(c, ov))),
    }
}
fn put_gz(c: &mut Ctx, ov: Value, g: Box<GzCore>) { if let Some(r) = c.data_mut::<RGz>(ov) { r.g = Some(g); } }
/// Run `f` on the gzfile; `ready` is get_gzfile's check ("closed gzip stream").
fn with_gz(c: &mut Ctx, ov: Value, ready: bool, f: impl FnOnce(&mut Ctx, &mut GzCore) -> RV) -> RV {
    let mut g = take_gz(c, ov)?;
    if ready && !g.z.ready() { put_gz(c, ov, g); return err(c, zk().gz_error, "closed gzip stream"); }
    let r = f(c, &mut g);
    put_gz(c, ov, g);
    r
}

fn gz_raise<T>(c: &mut Ctx, g: &GzCore, klass: Value, msg: &str) -> Result<T, Esc> {
    let e = make_exception(c, klass, msg);
    if let Some(i) = &g.z.input { let s = c.str_new_enc(i, ENC_BINARY); ivar_set_raw(c, e, intern(b"@input"), s); }
    Err(raise_exc(c, e))
}

fn gzfile_reset(g: &mut GzCore) {
    g.z.reset();
    g.z.flags |= FLAG_GZFILE;
    g.crc = 0;
    g.lineno = 0;
    g.ungetc = 0;
}

/// gzfile_write_raw.
fn write_raw(c: &mut Ctx, g: &mut GzCore, io: Value) -> Result<(), Esc> {
    if g.z.filled() > 0 {
        let data = g.z.buf.take().unwrap_or_default();
        g.z.avail_out = 0;
        let s = bin(c, data);
        funcall(c, io, intern(b"write"), &[s], std::ptr::null())?;
        if g.z.flags & GZ_SYNC != 0 && respond_to(c, io, intern(b"flush"), false) { funcall(c, io, intern(b"flush"), &[], std::ptr::null())?; }
    }
    Ok(())
}
/// gzfile_read_raw: io.readpartial(2048[, outbuf]), falling back to io.read; nil at EOF.
fn read_raw(c: &mut Ctx, io: Value, outbuf: Value) -> RV {
    let mut args = vec![Value::fix(GZFILE_READ_SIZE)];
    if outbuf != NIL { args.push(outbuf); }
    match funcall(c, io, intern(b"readpartial"), &args, std::ptr::null()) {
        Ok(s) => { if !is_string(s) { return type_err(c, format!("wrong argument type {} (expected String)", class_name_for_err(c, s))); } Ok(s) }
        Err(e) => {
            if c.esc != EscKind::Raise { return Err(e); }
            let x = c.esc_val;
            if is_kind_of(c, x, c.vm().c_eof_error.get()) { c.esc = EscKind::None; return Ok(NIL); }
            if is_kind_of(c, x, c.vm().c_no_method_error.get()) {
                c.esc = EscKind::None;
                let s = funcall(c, io, intern(b"read"), &args, std::ptr::null())?;
                if s != NIL && !is_string(s) { return type_err(c, format!("wrong argument type {} (expected String)", class_name_for_err(c, s))); }
                return Ok(s);
            }
            Err(e)
        }
    }
}
/// gzfile_read_raw_ensure: at least `size` bytes of input, false at EOF.
fn read_raw_ensure(c: &mut Ctx, g: &mut GzCore, io: Value, size: usize, outbuf: Value) -> Result<bool, Esc> {
    if io == UNDEF && g.z.input_len() < size { return err(c, zk().gz_error, "unexpected end of string"); }
    while g.z.input.is_none() || g.z.input_len() < size {
        let s = read_raw(c, io, outbuf)?;
        if s == NIL { return Ok(false); }
        let b = c.str_bytes(s).to_vec();
        g.z.append_input(&b);
    }
    Ok(true)
}
fn read_raw_until_zero(c: &mut Ctx, g: &mut GzCore, io: Value, mut offset: usize) -> Result<usize, Esc> {
    loop {
        let inp = g.z.input.as_deref().unwrap_or(&[]);
        if let Some(p) = inp[offset.min(inp.len())..].iter().position(|&b| b == 0) { return Ok(offset + p); }
        // Zlib.gunzip has no io (Qundef); CRuby calls readpartial/read on it anyway
        if io == UNDEF { return err(c, c.vm().c_no_method_error.get(), "undefined method 'read' for an instance of Float"); }
        let s = read_raw(c, io, NIL)?;
        if s == NIL { return err(c, zk().gz_error, "unexpected end of file"); }
        offset = g.z.input_len();
        let b = c.str_bytes(s).to_vec();
        g.z.append_input(&b);
    }
}

fn make_header(c: &mut Ctx, g: &mut GzCore, orig_name: Value, comment: Value) {
    let mut flags = 0u8;
    let mut extraflags = 0u8;
    if orig_name != NIL { flags |= 0x8; }
    if comment != NIL { flags |= 0x10; }
    if g.z.flags & GZ_MTIME_IS_SET == 0 {
        g.mtime = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as u32;
    }
    if g.level == 1 { extraflags |= 0x4; } else if g.level == 9 { extraflags |= 0x2; }
    let m = g.mtime.to_le_bytes();
    let buf = [0x1f, 0x8b, 8, flags, m[0], m[1], m[2], m[3], extraflags, g.os_code];
    g.z.append_buffer(&buf);
    if orig_name != NIL { let b = c.str_bytes(orig_name).to_vec(); g.z.append_buffer(&b); g.z.append_buffer(b"\0"); }
    if comment != NIL { let b = c.str_bytes(comment).to_vec(); g.z.append_buffer(&b); g.z.append_buffer(b"\0"); }
    g.z.flags |= GZ_HEADER_FINISHED;
}
fn make_footer(g: &mut GzCore) {
    let mut buf = [0u8; 8];
    buf[..4].copy_from_slice(&g.crc.to_le_bytes());
    buf[4..].copy_from_slice(&(g.z.total_in() as u32).to_le_bytes());
    g.z.append_buffer(&buf);
    g.z.flags |= GZ_FOOTER_FINISHED;
}
/// gzfile_read_header; returns (orig_name, comment) as found.
fn read_header(c: &mut Ctx, g: &mut GzCore, io: Value, outbuf: Value) -> Result<(Value, Value), Esc> {
    let k = zk();
    if !read_raw_ensure(c, g, io, 10, outbuf)? { return gz_raise(c, g, k.gz_error, "not in gzip format"); }
    let head: Vec<u8> = g.z.input.as_ref().unwrap()[..10].to_vec();
    if head[0] != 0x1f || head[1] != 0x8b { return gz_raise(c, g, k.gz_error, "not in gzip format"); }
    if head[2] != 8 { return err(c, k.gz_error, format!("unsupported compression method {}", head[2])); }
    let flags = head[3];
    if flags & 0x2 != 0 { return err(c, k.gz_error, "multi-part gzip file is not supported"); }
    if flags & 0x20 != 0 { return err(c, k.gz_error, "encrypted gzip file is not supported"); }
    if flags & 0xc0 != 0 { return err(c, k.gz_error, format!("unknown flags 0x{:02x}", flags as i8 as i32 as u32)); }
    g.level = if head[8] & 0x4 != 0 { 1 } else if head[8] & 0x2 != 0 { 9 } else { -1 };
    g.mtime = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
    g.os_code = head[9];
    g.z.discard_input(10);
    let (mut name, mut comment) = (NIL, NIL);
    if flags & 0x4 != 0 {
        if !read_raw_ensure(c, g, io, 2, outbuf)? { return err(c, k.gz_error, "unexpected end of file"); }
        let i = g.z.input.as_ref().unwrap();
        let len = i[0] as usize | (i[1] as usize) << 8;
        if !read_raw_ensure(c, g, io, 2 + len, outbuf)? { return err(c, k.gz_error, "unexpected end of file"); }
        g.z.discard_input(2 + len);
    }
    for (bit, slot) in [(0x8u8, &mut name), (0x10u8, &mut comment)] {
        if flags & bit != 0 {
            if !read_raw_ensure(c, g, io, 1, outbuf)? { return err(c, k.gz_error, "unexpected end of file"); }
            let p = read_raw_until_zero(c, g, io, 0)?;
            let s = g.z.input.as_ref().unwrap()[..p].to_vec();
            *slot = c.str_new_enc(&s, ENC_BINARY);
            g.z.discard_input(p + 1);
        }
    }
    if g.z.input_len() > 0 { zstream_run(c, &mut g.z, b"", Z_SYNC_FLUSH, NOCX)?; }
    Ok((name, comment))
}
fn check_footer(c: &mut Ctx, g: &mut GzCore, io: Value, outbuf: Value) -> Result<(), Esc> {
    let k = zk();
    g.z.flags |= GZ_FOOTER_FINISHED;
    if !read_raw_ensure(c, g, io, 8, outbuf)? { return gz_raise(c, g, k.no_footer, "footer is not found"); }
    let i = g.z.input.as_ref().unwrap();
    let crc = u32::from_le_bytes([i[0], i[1], i[2], i[3]]);
    let length = u32::from_le_bytes([i[4], i[5], i[6], i[7]]);
    let ti = g.z.total_in() + 8;
    g.z.set_total_in(ti);
    g.z.discard_input(8);
    if g.crc != crc { return err(c, k.crc_error, "invalid compressed data -- crc error"); }
    if g.z.total_out() as u32 != length { return err(c, k.length_error, "invalid compressed data -- length error"); }
    Ok(())
}
fn gz_write(c: &mut Ctx, g: &mut GzCore, io: Value, orig_name: Value, comment: Value, s: &[u8]) -> Result<(), Esc> {
    if g.z.flags & GZ_HEADER_FINISHED == 0 { make_header(c, g, orig_name, comment); }
    if !s.is_empty() || g.z.flags & GZ_SYNC != 0 {
        g.crc = crc32(g.crc, s);
        let fl = if g.z.flags & GZ_SYNC != 0 { Z_SYNC_FLUSH } else { Z_NO_FLUSH };
        zstream_run(c, &mut g.z, s, fl, NOCX)?;
    }
    write_raw(c, g, io)
}
fn read_more(c: &mut Ctx, g: &mut GzCore, io: Value, outbuf: Value) -> Result<usize, Esc> {
    while !g.z.finished() {
        let s = if io == UNDEF { NIL } else { read_raw(c, io, outbuf)? };
        if s == NIL {
            if !g.z.finished() { return err(c, zk().gz_error, "unexpected end of file"); }
            break;
        }
        if c.str_len(s) > 0 {
            let b = c.str_bytes(s).to_vec();
            zstream_run(c, &mut g.z, &b, Z_SYNC_FLUSH, NOCX)?;
        }
        if g.z.filled() > 0 { break; }
    }
    Ok(g.z.filled())
}
fn calc_crc(g: &mut GzCore, s: &[u8]) {
    if (s.len() as i64) <= g.ungetc { g.ungetc -= s.len() as i64; }
    else {
        g.crc = crc32(g.crc, &s[g.ungetc as usize..]);
        g.ungetc = 0;
    }
}
/// gzfile_newstr: the external encoding, converted when an internal one is set.
fn newstr(c: &mut Ctx, g: &GzCore, b: Vec<u8>, ecopts: Value) -> RV {
    let Some(e2) = g.enc2 else { return Ok(c.str_from_vec_enc(b, g.enc)) };
    let o = if is_hash(ecopts) { crate::builtins::encoding::parse_tc_opts(c, ecopts)? } else { crate::builtins::encoding::parse_tc_opts(c, NIL)? };
    if !crate::builtins::encoding::can_convert(e2, g.enc) { return Ok(c.str_from_vec_enc(b, e2)); }
    let out = crate::builtins::encoding::transcode(c, &b, e2, g.enc, &o)?;
    Ok(c.str_from_vec_enc(out, g.enc))
}
fn fill(c: &mut Ctx, g: &mut GzCore, io: Value, len: i64, outbuf: Value) -> Result<i64, Esc> {
    if len < 0 { return arg_err(c, format!("negative length {} given", len)); }
    if len == 0 { return Ok(0); }
    while !g.z.finished() && (g.z.filled() as i64) < len { read_more(c, g, io, outbuf)?; }
    if g.finished() {
        if g.z.flags & GZ_FOOTER_FINISHED == 0 { check_footer(c, g, io, outbuf)?; }
        return Ok(-1);
    }
    Ok(len.min(g.z.filled() as i64))
}
/// Set outbuf's bytes (rb_str_resize + memcpy).
fn set_outbuf(c: &mut Ctx, outbuf: Value, b: &[u8]) -> Result<(), Esc> {
    let s = c.str_modify(outbuf)?;
    s.b = crate::sbuf::SBuf::from_slice(b);
    Ok(())
}
fn shift_to(c: &mut Ctx, g: &mut GzCore, len: usize, outbuf: Value) -> RV {
    let data = g.z.shift(len);
    calc_crc(g, &data);
    if outbuf != NIL { set_outbuf(c, outbuf, &data)?; return Ok(outbuf); }
    Ok(bin(c, data))
}
fn gz_read(c: &mut Ctx, g: &mut GzCore, io: Value, len: i64, outbuf: Value) -> RV {
    let len = fill(c, g, io, len, outbuf)?;
    if len < 0 { if outbuf != NIL { set_outbuf(c, outbuf, b"")?; } return Ok(NIL); }
    if len == 0 {
        if outbuf == NIL { return Ok(c.str_new_enc(b"", ENC_BINARY)); }
        set_outbuf(c, outbuf, b"")?;
        return Ok(outbuf);
    }
    shift_to(c, g, len as usize, outbuf)
}
fn gz_read_all(c: &mut Ctx, g: &mut GzCore, io: Value, dst: Value, ecopts: Value) -> RV {
    while !g.z.finished() { read_more(c, g, io, dst)?; }
    if g.finished() {
        if g.z.flags & GZ_FOOTER_FINISHED == 0 { check_footer(c, g, io, dst)?; }
        if dst != NIL { set_outbuf(c, dst, b"")?; return Ok(dst); }
        return Ok(c.str_new_enc(b"", ENC_BINARY));
    }
    let data = g.z.buf.take().unwrap_or_default();
    g.z.avail_out = 0;
    calc_crc(g, &data);
    // CRuby returns the detached buffer (dst itself is not filled)
    newstr(c, g, data, ecopts)
}
fn gz_getc(c: &mut Ctx, g: &mut GzCore, io: Value, ecopts: Value) -> RV {
    let len = crate::builtins::encoding::max_len(g.enc);
    while !g.z.finished() && g.z.filled() < len { read_more(c, g, io, NIL)?; }
    if g.finished() {
        if g.z.flags & GZ_FOOTER_FINISHED == 0 { check_footer(c, g, io, NIL)?; }
        return Ok(NIL);
    }
    let b = g.z.buf.as_deref().unwrap_or(&[]);
    let n = enc_char_len(b, 0, g.enc).clamp(1, b.len());
    let dst = gz_read(c, g, io, n as i64, NIL)?;
    if dst == NIL { return Ok(NIL); }
    let bytes = c.str_bytes(dst).to_vec();
    newstr(c, g, bytes, ecopts)
}

fn gz_end(c: &mut Ctx, g: &mut GzCore, io: Value, orig_name: Value, comment: Value) -> Result<(), Esc> {
    if g.z.flags & FLAG_CLOSING != 0 { return Ok(()); }
    g.z.flags |= FLAG_CLOSING;
    let r = if g.writer {
        (|| -> Result<(), Esc> {
            if g.z.flags & GZ_HEADER_FINISHED == 0 { make_header(c, g, orig_name, comment); }
            zstream_run(c, &mut g.z, b"", Z_FINISH, NOCX)?;
            make_footer(g);
            write_raw(c, g, io)
        })()
    } else if g.finished() && g.z.flags & GZ_FOOTER_FINISHED == 0 {
        check_footer(c, g, io, NIL)
    } else { Ok(()) };
    let (ek, ev) = (c.esc, c.esc_val);
    let e = zstream_end(c, &mut g.z);
    if r.is_err() { c.esc = ek; c.esc_val = ev; return r; }
    e
}
/// gzfile_close: end, forget io, and io.close when asked.
fn gz_close(c: &mut Ctx, ov: Value, g: &mut GzCore, closeflag: bool) -> RV {
    let (io, on, cm) = gz_vals(c, ov);
    let r = gz_end(c, g, io, on, cm);
    if let Some(d) = c.data_mut::<RGz>(ov) { d.io = NIL; d.orig_name = NIL; d.comment = NIL; }
    r?;
    if closeflag && respond_to(c, io, intern(b"close"), false) { funcall(c, io, intern(b"close"), &[], std::ptr::null())?; }
    Ok(io)
}

fn gzf_to_io(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_gz(c, s, true, |c, _g| Ok(gz_io(c, s))) }
fn gzf_crc(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_gz(c, s, true, |c, g| Ok(uint(c, g.crc as u64))) }
fn gzf_mtime(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let t = with_gz(c, s, true, |_c, g| Ok(Value::fix(g.mtime as i64)))?;
    let tk = c.vm().c_time.get();
    funcall(c, tk, intern(b"at"), &[t], std::ptr::null())
}
fn gzf_level(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_gz(c, s, true, |_c, g| Ok(Value::fix(g.level as i64))) }
fn gzf_os_code(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_gz(c, s, true, |_c, g| Ok(Value::fix(g.os_code as i64))) }
fn gzf_orig_name(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    with_gz(c, s, true, |_c, _g| Ok(NIL))?;
    let v = gz_vals(c, s).1;
    Ok(if v == NIL { NIL } else { c.str_dup(v) })
}
fn gzf_comment(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    with_gz(c, s, true, |_c, _g| Ok(NIL))?;
    let v = gz_vals(c, s).2;
    Ok(if v == NIL { NIL } else { c.str_dup(v) })
}
fn gzf_lineno(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_gz(c, s, true, |_c, g| Ok(Value::fix(g.lineno as i64))) }
fn gzf_set_lineno(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let n = num_to_long(c, a[0])?;
    if n > i32::MAX as i64 || n < i32::MIN as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too {} to convert to 'int'", n, if n < 0 { "small" } else { "big" })); }
    with_gz(c, s, true, |_c, g| { g.lineno = n as i32; Ok(NIL) })?;
    Ok(a[0])
}
fn header_check(c: &mut Ctx, s: Value) -> Result<(), Esc> {
    let done = with_gz(c, s, true, |_c, g| Ok(Value::bool(g.z.flags & GZ_HEADER_FINISHED != 0)))?;
    if done.truthy() { return err(c, zk().gz_error, "header is already written"); }
    Ok(())
}
fn gzw_set_mtime(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    header_check(c, s)?;
    let k = c.vm().main_obj.get();
    let v = funcall(c, k, intern(b"Integer"), &[a[0]], std::ptr::null())?;
    let n = to_ulong(c, v)?;
    if n > u32::MAX as u64 && (n as i64) < i32::MIN as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too big to convert to 'unsigned int'", n)); }
    with_gz(c, s, true, |_c, g| { g.mtime = n as u32; g.z.flags |= GZ_MTIME_IS_SET; Ok(NIL) })?;
    Ok(a[0])
}
/// orig_name= / comment=: a copy cut at the first NUL.
fn header_str(c: &mut Ctx, s: Value, v: Value) -> RV {
    header_check(c, s)?;
    let sv = expect_str(c, v)?;
    let d = c.str_dup(sv);
    if let Some(p) = c.str_bytes(d).iter().position(|&b| b == 0) { c.rstring_mut(d).b.truncate(p); }
    Ok(d)
}
fn gzw_set_orig_name(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let d = header_str(c, s, a[0])?;
    if let Some(r) = c.data_mut::<RGz>(s) { r.orig_name = d; }
    Ok(a[0])
}
fn gzw_set_comment(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let d = header_str(c, s, a[0])?;
    if let Some(r) = c.data_mut::<RGz>(s) { r.comment = d; }
    Ok(a[0])
}
fn gzf_close(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let ready = with_gz(c, s, false, |_c, g| Ok(Value::bool(g.z.ready())))?;
    if !ready.truthy() { return Ok(NIL); }
    with_gz(c, s, false, |c, g| gz_close(c, s, g, true))
}
fn gzf_finish(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_gz(c, s, true, |c, g| gz_close(c, s, g, false)) }
fn gzf_closed_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(gz_io(c, s) == NIL)) }
fn gzf_eof_p(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let io = gz_io(c, s);
    with_gz(c, s, true, |c, g| {
        while !g.z.finished() && g.z.filled() == 0 { read_more(c, g, io, NIL)?; }
        Ok(Value::bool(g.finished()))
    })
}
fn gzf_sync(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_gz(c, s, true, |_c, g| Ok(Value::bool(g.z.flags & GZ_SYNC != 0))) }
fn gzf_set_sync(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let on = a[0].truthy();
    with_gz(c, s, true, |_c, g| { if on { g.z.flags |= GZ_SYNC; } else { g.z.flags &= !GZ_SYNC; } Ok(NIL) })?;
    Ok(a[0])
}
fn gzf_total_in(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { with_gz(c, s, true, |c, g| Ok(uint(c, g.z.total_in()))) }
fn gzf_total_out(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    with_gz(c, s, true, |c, g| {
        let (t, f) = (g.z.total_out(), g.z.filled() as u64);
        Ok(if t >= f { uint(c, t - f) } else { Value::fix(-((f - t) as i64)) })
    })
}
fn gzf_path(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.data::<RGz>(s).map(|r| r.path).unwrap_or(NIL)) }

/// io.path when io responds to it (IOError ignored), and the singleton GzipFile#path.
fn init_path(c: &mut Ctx, ov: Value, io: Value) -> Result<(), Esc> {
    if !respond_to(c, io, intern(b"path"), false) { return Ok(()); }
    match funcall(c, io, intern(b"path"), &[], std::ptr::null()) {
        Ok(p) => {
            if let Some(r) = c.data_mut::<RGz>(ov) { r.path = p; }
            let sk = singleton_class(c, ov);
            define_cfunc(c, sk, "path", gzf_path, 0);
            Ok(())
        }
        Err(e) => {
            if c.esc == EscKind::Raise && is_kind_of(c, c.esc_val, c.vm().c_io_error.get()) { c.esc = EscKind::None; return Ok(()); }
            Err(e)
        }
    }
}
/// rb_io_extract_encoding_option + rb_gzfile_ecopts.
fn ecopts(c: &mut Ctx, ov: Value, g: &mut GzCore, opts: Value) -> Result<(), Esc> {
    use crate::builtins::encoding as en;
    if opts == NIL { return Ok(()); }
    let mut ext: Option<u8> = None;
    let mut int: Option<u8> = None;
    let mut given = false;
    if let Some(e) = crate::hash::hash_get_opt(c, opts, sym_value("encoding"))? {
        if e != NIL {
            given = true;
            if is_string(e) {
                let s = String::from_utf8_lossy(c.str_bytes(e)).into_owned();
                let mut it = s.splitn(2, ':');
                let a = it.next().unwrap_or("");
                let a = a.strip_prefix("BOM|").or_else(|| a.strip_prefix("bom|")).unwrap_or(a);
                ext = en::find_enc(a.as_bytes());
                if let Some(b) = it.next() { if b != "-" { int = en::find_enc(b.as_bytes()); } }
            } else { ext = Some(en::to_enc(c, e)?); }
        }
    }
    if let Some(e) = crate::hash::hash_get_opt(c, opts, sym_value("external_encoding"))? { if e != NIL { given = true; ext = Some(en::to_enc(c, e)?); } }
    if let Some(e) = crate::hash::hash_get_opt(c, opts, sym_value("internal_encoding"))? { if e != NIL { given = true; int = Some(en::to_enc(c, e)?); } }
    if !given { return Ok(()); }
    let ext = ext.unwrap_or(g.enc);
    let int = int.or_else(|| if ext != ENC_BINARY { en::default_internal() } else { None });
    match int {
        Some(i) if i != ext => { g.enc = i; g.enc2 = Some(ext); }
        _ => { g.enc = ext; g.enc2 = None; }
    }
    if g.enc2.is_some() { if let Some(r) = c.data_mut::<RGz>(ov) { r.ecopts = opts; } }
    Ok(())
}

// ---- GzipWriter ---------------------------------------------------------------------------

fn gzw_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, opt) = match a.last() { Some(&h) if a.len() > 1 && is_hash(h) => (&a[..a.len() - 1], h), _ => (a, NIL) };
    check_argc(c, pos, 1, 3)?;
    let io = pos[0];
    let level = level_of(c, pos.get(1))?;
    let strategy = int_arg(c, pos.get(2), Z_DEFAULT_STRATEGY)?;
    let d = deflate_init(c, level, -MAX_WBITS, DEF_MEM_LEVEL, strategy)?;
    with_gz(c, s, false, |c, g| {
        g.level = level;
        g.z.eng = Eng::Def(d);
        g.z.flags |= FLAG_READY;
        if let Some(r) = c.data_mut::<RGz>(s) { r.io = io; }
        ecopts(c, s, g, opt)?;
        Ok(NIL)
    })?;
    init_path(c, s, io)?;
    Ok(s)
}
fn gzw_flush(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let flush = int_arg(c, a.first(), Z_SYNC_FLUSH)?;
    let io = gz_io(c, s);
    with_gz(c, s, true, |c, g| {
        if flush != Z_NO_FLUSH { zstream_run(c, &mut g.z, b"", flush, NOCX)?; }
        write_raw(c, g, io)?;
        Ok(NIL)
    })?;
    if respond_to(c, io, intern(b"flush"), false) { funcall(c, io, intern(b"flush"), &[], std::ptr::null())?; }
    Ok(s)
}
fn gzw_write(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let mut total = 0usize;
    for &v in a {
        let mut str = if is_string(v) { v } else { to_s_value(c, v)? };
        let (enc2, io, on, cm) = { let (io, on, cm) = gz_vals(c, s); (with_gz(c, s, true, |_c, g| Ok(g.enc2.map(|e| Value::fix(e as i64)).unwrap_or(NIL)))?, io, on, cm) };
        if enc2 != NIL && enc2.as_fix() as u8 != ENC_BINARY {
            let to = enc2.as_fix() as u8;
            let from = c.str_enc(str);
            if from != to && crate::builtins::encoding::can_convert(from, to) {
                let o = crate::builtins::encoding::parse_tc_opts(c, NIL)?;
                let b = c.str_bytes(str).to_vec();
                let out = crate::builtins::encoding::transcode(c, &b, from, to, &o)?;
                str = c.str_from_vec_enc(out, to);
            }
        }
        let bytes = c.str_bytes(str).to_vec();
        with_gz(c, s, true, |c, g| { gz_write(c, g, io, on, cm, &bytes)?; Ok(NIL) })?;
        total += bytes.len();
    }
    Ok(Value::fix(total as i64))
}
fn gzw_putc(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let ch = if is_string(a[0]) { c.str_bytes(a[0]).first().copied().unwrap_or(0) } else { num_to_long(c, a[0])? as u8 };
    let (io, on, cm) = gz_vals(c, s);
    with_gz(c, s, true, |c, g| { gz_write(c, g, io, on, cm, &[ch])?; Ok(NIL) })?;
    Ok(a[0])
}

// ---- GzipReader ---------------------------------------------------------------------------

fn gzr_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, opt) = match a.last() { Some(&h) if a.len() > 1 && is_hash(h) => (&a[..a.len() - 1], h), _ => (a, NIL) };
    check_argc(c, pos, 1, 1)?;
    let io = pos[0];
    let i = inflate_init(c, -MAX_WBITS)?;
    with_gz(c, s, false, |c, g| {
        g.z.eng = Eng::Inf(i);
        g.z.flags |= FLAG_READY;
        if let Some(r) = c.data_mut::<RGz>(s) { r.io = io; }
        let (on, cm) = read_header(c, g, io, NIL)?;
        if let Some(r) = c.data_mut::<RGz>(s) { r.orig_name = on; r.comment = cm; }
        ecopts(c, s, g, opt)?;
        Ok(NIL)
    })?;
    init_path(c, s, io)?;
    Ok(s)
}
fn ecopts_of(c: &Ctx, s: Value) -> Value { c.data::<RGz>(s).map(|r| r.ecopts).unwrap_or(NIL) }
fn gzr_rewind(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let io = gz_io(c, s);
    with_gz(c, s, true, |c, g| {
        let n = g.z.total_in() as i64 + g.z.input_len() as i64;
        let off = c.int_from_i64(-n);
        funcall(c, io, intern(b"seek"), &[off, Value::fix(1)], std::ptr::null())?;
        gzfile_reset(g);
        Ok(Value::fix(0))
    })
}
fn gzr_unused(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let io = gz_io(c, s);
    with_gz(c, s, false, |c, g| {
        if !g.z.ready() || !g.finished() { return Ok(NIL); }
        if g.z.flags & GZ_FOOTER_FINISHED == 0 { check_footer(c, g, io, NIL)?; }
        match &g.z.input { None => Ok(NIL), Some(i) => { let i = i.clone(); Ok(bin(c, i)) } }
    })
}
fn gzr_read(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 2)?;
    let vlen = a.first().copied().unwrap_or(NIL);
    let outbuf = a.get(1).copied().unwrap_or(NIL);
    let io = gz_io(c, s);
    let eco = ecopts_of(c, s);
    if vlen == NIL { return with_gz(c, s, true, |c, g| gz_read_all(c, g, io, outbuf, eco)); }
    let len = num_to_long(c, vlen)?;
    with_gz(c, s, true, |_c, _g| Ok(NIL))?;
    if len < 0 { return arg_err(c, format!("negative length {} given", len)); }
    with_gz(c, s, true, |c, g| gz_read(c, g, io, len, outbuf))
}
fn gzr_readpartial(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let outbuf = a.get(1).copied().unwrap_or(NIL);
    let io = gz_io(c, s);
    with_gz(c, s, true, |_c, _g| Ok(NIL))?;
    let len = num_to_long(c, a[0])?;
    if len < 0 { return arg_err(c, format!("negative length {} given", len)); }
    if outbuf != NIL && !is_string(outbuf) { return type_err(c, format!("wrong argument type {} (expected String)", class_name_for_err(c, outbuf))); }
    with_gz(c, s, true, |c, g| {
        if len == 0 {
            if outbuf == NIL { return Ok(c.str_new_enc(b"", ENC_BINARY)); }
            set_outbuf(c, outbuf, b"")?;
            return Ok(outbuf);
        }
        while !g.z.finished() && g.z.filled() == 0 { read_more(c, g, io, outbuf)?; }
        if g.finished() {
            if g.z.flags & GZ_FOOTER_FINISHED == 0 { check_footer(c, g, io, outbuf)?; }
            if outbuf != NIL { set_outbuf(c, outbuf, b"")?; }
            return err(c, c.vm().c_eof_error.get(), "end of file reached");
        }
        shift_to(c, g, len as usize, outbuf)
    })
}
fn gzr_getc(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let io = gz_io(c, s);
    let eco = ecopts_of(c, s);
    with_gz(c, s, true, |c, g| gz_getc(c, g, io, eco))
}
fn gzr_readchar(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    let d = gzr_getc(c, s, a, b)?;
    if d == NIL { return err(c, c.vm().c_eof_error.get(), "end of file reached"); }
    Ok(d)
}
fn gzr_getbyte(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let io = gz_io(c, s);
    let d = with_gz(c, s, true, |c, g| gz_read(c, g, io, 1, NIL))?;
    if d == NIL { return Ok(NIL); }
    Ok(Value::fix(c.str_bytes(d)[0] as i64))
}
fn gzr_readbyte(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    let d = gzr_getbyte(c, s, a, b)?;
    if d == NIL { return err(c, c.vm().c_eof_error.get(), "end of file reached"); }
    Ok(d)
}
fn gzr_each_char(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"each_char"), &[], NIL); }
    loop { let ch = gzr_getc(c, s, a, Blk::NONE)?; if ch == NIL { break; } yield1(c, blk, ch)?; }
    Ok(NIL)
}
fn gzr_each_byte(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"each_byte"), &[], NIL); }
    loop { let ch = gzr_getbyte(c, s, a, Blk::NONE)?; if ch == NIL { break; } yield1(c, blk, ch)?; }
    Ok(NIL)
}
fn gzr_ungetc(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    if a[0].is_fixnum() { return gzr_ungetbyte(c, s, a, b); }
    with_gz(c, s, true, |_c, _g| Ok(NIL))?;
    let mut sv = expect_str(c, a[0])?;
    let enc2 = with_gz(c, s, true, |_c, g| Ok(g.enc2.map(|e| Value::fix(e as i64)).unwrap_or(NIL)))?;
    if enc2 != NIL && enc2.as_fix() as u8 != ENC_BINARY {
        let to = enc2.as_fix() as u8;
        let from = c.str_enc(sv);
        if from != to && crate::builtins::encoding::can_convert(from, to) {
            let o = crate::builtins::encoding::parse_tc_opts(c, NIL)?;
            let bb = c.str_bytes(sv).to_vec();
            let out = crate::builtins::encoding::transcode(c, &bb, from, to, &o)?;
            sv = c.str_from_vec_enc(out, to);
        }
    }
    let bytes = c.str_bytes(sv).to_vec();
    with_gz(c, s, true, |_c, g| { g.z.ungets(&bytes); g.ungetc += bytes.len() as i64; Ok(NIL) })
}
fn gzr_ungetbyte(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    with_gz(c, s, true, |_c, _g| Ok(NIL))?;
    let ch = if is_string(a[0]) { c.str_bytes(a[0]).first().copied().unwrap_or(0) } else { num_to_long(c, a[0])? as u8 };
    with_gz(c, s, true, |_c, g| { g.z.ungets(&[ch]); g.ungetc += 1; Ok(NIL) })
}
fn skip_linebreaks(c: &mut Ctx, g: &mut GzCore, io: Value) -> Result<(), Esc> {
    while g.z.filled() == 0 {
        if g.finished() { return Ok(()); }
        read_more(c, g, io, NIL)?;
    }
    let mut n = 0usize;
    loop {
        let b = g.z.buf.as_deref().unwrap_or(&[]);
        n += 1;
        if b[n - 1] != b'\n' { break; }
        if n >= g.z.filled() {
            let d = g.z.buf.take().unwrap_or_default();
            g.z.avail_out = 0;
            calc_crc(g, &d);
            while g.z.filled() == 0 {
                if g.finished() { return Ok(()); }
                read_more(c, g, io, NIL)?;
            }
            n = 0;
        }
    }
    let d = g.z.shift(n - 1);
    calc_crc(g, &d);
    Ok(())
}
fn charboundary(c: &mut Ctx, g: &mut GzCore, io: Value, n: usize) -> Result<usize, Esc> {
    use crate::builtins::encoding as en;
    let b = g.z.buf.as_deref().unwrap_or(&[]).to_vec();
    let enc = g.enc;
    // rb_enc_left_char_head of s + n - 1
    let offs = char_offsets_of(&b, enc);
    let p = offs.iter().copied().filter(|&o| o <= n - 1).max().unwrap_or(0);
    if p < n {
        match en::precise_len(enc, &b, p) {
            Some(l) => return Ok(p + l),
            None => {
                if en::needmore(enc, &b[p..]) {
                    let need = en::max_len(enc).saturating_sub(b.len() - p).max(1);
                    let l = fill(c, g, io, (n + need) as i64, NIL)?;
                    if l > 0 { return Ok(l as usize); }
                }
            }
        }
    }
    Ok(n)
}
fn gzr_gets_impl(c: &mut Ctx, s: Value, a: &[Value]) -> RV {
    let io = gz_io(c, s);
    let eco = ecopts_of(c, s);
    with_gz(c, s, true, |_c, _g| Ok(NIL))?;
    let mut rs = global_get("$/");
    let mut limit: i64 = -1;
    if !a.is_empty() {
        check_argc(c, a, 1, 2)?;
        rs = a[0];
        let mut lim = a.get(1).copied().unwrap_or(NIL);
        if lim != NIL {
            if rs != NIL { rs = expect_str(c, rs)?; }
        } else if rs != NIL {
            if is_string(rs) {} else if respond_to(c, rs, intern(b"to_str"), true) { rs = expect_str(c, rs)?; }
            else { lim = rs; rs = global_get("$/"); }
        }
        if lim != NIL {
            limit = num_to_long(c, lim)?;
            if limit == 0 { return Ok(c.str_new_enc(b"", ENC_BINARY)); }
        }
    }
    let rsb: Option<Vec<u8>> = if rs == NIL { None } else { Some(c.str_bytes(rs).to_vec()) };
    with_gz(c, s, true, |c, g| {
        let maxlen = crate::builtins::encoding::max_len(g.enc);
        let Some(rsb) = rsb else {
            let dst;
            if limit < 0 {
                let d = gz_read_all(c, g, io, NIL, eco)?;
                if c.str_len(d) == 0 { return Ok(NIL); }
                dst = d;
            } else {
                let n = fill(c, g, io, limit, NIL)?;
                if n <= 0 { return Ok(NIL); }
                let n = if maxlen > 1 && n >= limit && !g.finished() { charboundary(c, g, io, n as usize)? } else { limit as usize };
                let d = g.z.shift(n);
                calc_crc(g, &d);
                dst = newstr(c, g, d, eco)?;
            }
            g.lineno += 1;
            return Ok(dst);
        };
        let (rsv, rspara): (Vec<u8>, bool) = if rsb.is_empty() { (b"\n\n".to_vec(), true) } else { (rsb, false) };
        let rslen = rsv.len();
        if rspara { skip_linebreaks(c, g, io)?; }
        while g.z.filled() < rslen {
            if g.z.finished() {
                if g.z.filled() > 0 { g.lineno += 1; }
                let r = gz_read(c, g, io, rslen as i64, NIL)?;
                return if r == NIL { Ok(NIL) } else { let b = c.str_bytes(r).to_vec(); newstr_raw(c, b) };
            }
            read_more(c, g, io, NIL)?;
        }
        let mut p = 0usize;
        let mut n = rslen;
        loop {
            if n > g.z.filled() {
                if g.z.finished() { break; }
                read_more(c, g, io, NIL)?;
                p = n - rslen;
            }
            let mut filled = g.z.filled();
            if limit > 0 && filled as i64 >= limit { filled = limit as usize; }
            let b = g.z.buf.as_deref().unwrap_or(&[]);
            let end = (p + (filled + 1).saturating_sub(n)).min(b.len());
            let res = b[p.min(end)..end].iter().position(|&x| x == rsv[0]).map(|i| p + i);
            match res {
                None => {
                    n = filled;
                    if limit > 0 && filled as i64 >= limit { break; }
                    n += 1;
                }
                Some(r) => {
                    n += r - p;
                    p = r;
                    if rslen == 1 || b.get(p..p + rslen) == Some(&rsv[..]) { break; }
                    p += 1;
                    n += 1;
                }
            }
        }
        if maxlen > 1 && n as i64 == limit && (g.z.filled() > n || !g.z.finished()) { n = charboundary(c, g, io, n)?; }
        g.lineno += 1;
        let dst = gz_read(c, g, io, n as i64, NIL)?;
        if dst == NIL { return Ok(NIL); }
        if rspara { skip_linebreaks(c, g, io)?; }
        let b = c.str_bytes(dst).to_vec();
        newstr(c, g, b, eco)
    })
}
/// The early return of gzreader_gets keeps gzfile_read's binary String.
fn newstr_raw(c: &mut Ctx, b: Vec<u8>) -> RV { Ok(bin(c, b)) }
fn gzr_gets(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let d = gzr_gets_impl(c, s, a)?;
    if d != NIL { gvar_set(c, intern(b"$_"), d)?; }
    Ok(d)
}
fn gzr_readline(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    let d = gzr_gets(c, s, a, b)?;
    if d == NIL { return err(c, c.vm().c_eof_error.get(), "end of file reached"); }
    Ok(d)
}
fn gzr_each(c: &mut Ctx, s: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { let m = c.cur_method_name().unwrap_or(intern(b"each")); return crate::builtins::enumerator::enum_for(c, s, m, &[], NIL); }   // RETURN_ENUMERATOR(obj, 0, 0)
    loop { let l = gzr_gets_impl(c, s, a)?; if l == NIL { break; } yield1(c, blk, l)?; }
    Ok(s)
}
fn gzr_readlines(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let dst = c.ary_new();
    loop { let l = gzr_gets_impl(c, s, a)?; if l == NIL { break; } c.ary_push(dst, l); }
    Ok(dst)
}
fn gzr_external_encoding(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let e = with_gz(c, s, true, |_c, g| Ok(Value::fix(g.enc as i64)))?;
    Ok(crate::builtins::encoding::encoding_value(c, e.as_fix() as u8))
}

// ---- Zlib.gzip / Zlib.gunzip --------------------------------------------------------------

fn z_gzip(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, opts) = match a.last() { Some(&h) if is_hash(h) => (&a[..a.len() - 1], Some(h)), _ => (a, None) };
    let (mut level, mut strategy) = (NIL, NIL);
    if let Some(h) = opts {
        crate::hash::hash_each(c, h, |c, k, _| {
            if k != sym_value("level") && k != sym_value("strategy") {
                let ks = inspect_str(c, k)?;
                return arg_err(c, format!("unknown keyword: {}", ks));
            }
            Ok(())
        })?;
        if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("level"))? { level = v; }
        if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("strategy"))? { strategy = v; }
    }
    check_argc(c, pos, 1, 1)?;
    let src = expect_str(c, pos[0])?;
    let mut g = GzCore::new(false);
    g.level = int_arg(c, Some(&level), -1)?;
    let st = int_arg(c, Some(&strategy), Z_DEFAULT_STRATEGY)?;
    let d = deflate_init(c, g.level, -MAX_WBITS, DEF_MEM_LEVEL, st)?;
    g.z.eng = Eng::Def(d);
    g.z.flags |= FLAG_READY;
    let bytes = c.str_bytes(src).to_vec();
    make_header(c, &mut g, NIL, NIL);
    let r = (|| -> RV {
        if !bytes.is_empty() {
            g.crc = crc32(g.crc, &bytes);
            zstream_run(c, &mut g.z, &bytes, Z_NO_FLUSH, NOCX)?;
        }
        g.z.flags |= FLAG_CLOSING;
        zstream_run(c, &mut g.z, b"", Z_FINISH, NOCX)?;
        make_footer(&mut g);
        zstream_end(c, &mut g.z)?;
        let data = g.z.buf.take().unwrap_or_default();
        Ok(bin(c, data))
    })();
    if g.z.ready() { let (ek, ev) = (c.esc, c.esc_val); let _ = zstream_end(c, &mut g.z); if r.is_err() { c.esc = ek; c.esc_val = ev; } }
    r
}
fn z_gunzip(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let src = expect_str(c, a[0])?;
    let mut g = GzCore::new(true);
    let i = inflate_init(c, -MAX_WBITS)?;
    g.z.eng = Eng::Inf(i);
    let b = c.str_bytes(src).to_vec();
    g.z.input = if b.is_empty() { None } else { Some(b) };
    g.z.flags |= FLAG_READY;
    let r = (|| -> RV {
        read_header(c, &mut g, UNDEF, NIL)?;
        let data = g.z.buf.take().unwrap_or_default();
        g.z.avail_out = 0;
        calc_crc(&mut g, &data);
        if !g.z.finished() { return err(c, zk().gz_error, "unexpected end of file"); }
        if g.z.input.is_none() { return err(c, zk().no_footer, "footer is not found"); }
        check_footer(c, &mut g, UNDEF, NIL)?;
        Ok(bin(c, data))
    })();
    g.z.flags |= FLAG_CLOSING;
    let (ek, ev) = (c.esc, c.esc_val);
    let e = zstream_end(c, &mut g.z);
    if r.is_err() { c.esc = ek; c.esc_val = ev; return r; }
    e?;
    r
}

const RUBY_PART: &str = r#"
module Zlib
  class GzipFile
    class Error
      attr_reader :input
      def inspect
        s = super
        return s if @input.nil?
        s[0...-1] + ", input=#{@input.inspect}>"
      end
    end
    def self.wrap(*args, **kw)
      ov = kw.empty? ? new(*args) : new(*args, **kw)
      return ov unless block_given?
      begin
        yield ov
      ensure
        ov.close unless ov.closed?
      end
    end
  end
  class GzipWriter
    def self.open(filename, *args, **kw, &blk)
      io = File.open(filename, "wb")
      begin
        ov = kw.empty? ? new(io, *args) : new(io, *args, **kw)
      rescue Exception
        io.close
        raise
      end
      return ov unless blk
      begin
        yield ov
      ensure
        ov.close unless ov.closed?
      end
    end
    def <<(str)
      write(str)
      self
    end
    def print(*args)
      args = [$_] if args.empty?
      args.each_with_index do |a, i|
        write($,) if i > 0 && $,
        write(a)
      end
      write($\) if $\
      nil
    end
    def printf(*args)
      write(format(*args))
      nil
    end
    def puts(*args)
      if args.empty?
        write("\n")
        return nil
      end
      __puts_ary(args, {})
      nil
    end
    private def __puts_ary(args, seen)
      args.each do |a|
        if a.is_a?(String)
          line = a
        elsif (ary = Array.try_convert(a))
          if seen[ary.__id__]
            line = "[...]"
          else
            seen[ary.__id__] = true
            __puts_ary(ary, seen)
            seen.delete(ary.__id__)
            next
          end
        else
          line = a.to_s
          line = a.inspect unless line.is_a?(String)
        end
        if line.end_with?("\n")
          write(line)
        else
          write(line, "\n")
        end
      end
    end
  end
  class GzipReader
    include Enumerable
    def self.open(filename, *args, **kw, &blk)
      io = File.open(filename, "rb")
      begin
        ov = kw.empty? ? new(io, *args) : new(io, *args, **kw)
      rescue Exception
        io.close
        raise
      end
      return ov unless blk
      begin
        yield ov
      ensure
        ov.close unless ov.closed?
      end
    end
    def self.zcat(io, options = nil, &block)
      buf = nil
      begin
        ov = options ? new(io, options) : new(io)
        if block
          ov.each(&block)
        else
          buf ||= "".b
          buf << ov.read
        end
        ov.read
        pos = io.pos
        unused = ov.unused
        ov.finish
        if unused
          pos -= unused.length
          io.pos = pos
        end
      end while pos < io.size
      return nil if block
      buf
    end
  end
end
"#;

/// Run by the first `require "zlib"`.
pub fn init(c: &mut Ctx) {
    let (std_err, object) = (c.vm().c_standard_error.get(), c.vm().c_object.get());
    let m = define_module(c, "Zlib");
    let error = define_class_under(c, m, "Error", std_err);
    let d = |c: &mut Ctx, n: &str| define_class_under(c, m, n, error);
    let mut k = Zk { m, error, stream_end: d(c, "StreamEnd"), need_dict: d(c, "NeedDict"), data_error: d(c, "DataError"), stream_error: d(c, "StreamError"),
        mem_error: d(c, "MemError"), buf_error: d(c, "BufError"), version_error: d(c, "VersionError"), in_progress: d(c, "InProgressError"),
        deflate: NIL, inflate: NIL, gz_error: NIL, no_footer: NIL, crc_error: NIL, length_error: NIL, gzwriter: NIL, gzreader: NIL };
    define_module_function(c, m, "zlib_version", z_zlib_version, 0);
    define_module_function(c, m, "adler32", z_adler32, -1);
    define_module_function(c, m, "adler32_combine", z_adler32_combine, 3);
    define_module_function(c, m, "crc32", z_crc32, -1);
    define_module_function(c, m, "crc32_combine", z_crc32_combine, 3);
    define_module_function(c, m, "crc_table", z_crc_table, 0);
    let cs = |c: &mut Ctx, n: &str, v: &[u8]| { let s = c.str_new_enc(v, ENC_BINARY); const_set(c, m, intern(n.as_bytes()), s); };
    cs(c, "VERSION", b"3.2.3");
    cs(c, "ZLIB_VERSION", b"1.3");
    let ci = |c: &mut Ctx, n: &str, v: i64| const_set(c, m, intern(n.as_bytes()), Value::fix(v));
    let zs = define_class_under(c, m, "ZStream", object);
    class_set_alloc(c, zs, AllocKind::None);
    let z = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, zs, n, f, a);
    z(c, "avail_out", zs_avail_out, 0); z(c, "avail_out=", zs_set_avail_out, 1); z(c, "avail_in", zs_avail_in, 0); z(c, "total_in", zs_total_in, 0);
    z(c, "total_out", zs_total_out, 0); z(c, "data_type", zs_data_type, 0); z(c, "adler", zs_adler, 0); z(c, "finished?", zs_finished_p, 0);
    z(c, "stream_end?", zs_finished_p, 0); z(c, "closed?", zs_closed_p, 0); z(c, "ended?", zs_closed_p, 0); z(c, "close", zs_end, 0); z(c, "end", zs_end, 0);
    z(c, "reset", zs_reset, 0); z(c, "finish", zs_finish, 0); z(c, "flush_next_in", zs_flush_next_in, 0); z(c, "flush_next_out", zs_flush_next_out, 0);
    ci(c, "BINARY", Z_BINARY as i64); ci(c, "ASCII", Z_TEXT as i64); ci(c, "TEXT", Z_TEXT as i64); ci(c, "UNKNOWN", Z_UNKNOWN as i64);
    let df = define_class_under(c, m, "Deflate", zs);
    class_set_alloc(c, df, AllocKind::None);
    define_singleton_cfunc(c, df, "deflate", def_s_deflate, -1);
    define_singleton_cfunc(c, m, "deflate", def_s_deflate, -1);
    let z = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, df, n, f, a);
    define_private_cfunc(c, df, "initialize", def_initialize, -1);
    define_private_cfunc(c, df, "initialize_copy", def_init_copy, 1);
    z(c, "deflate", def_deflate, -1); z(c, "<<", def_addstr, 1); z(c, "flush", def_flush, -1); z(c, "params", def_params, 2); z(c, "set_dictionary", def_set_dictionary, 1);
    let inf = define_class_under(c, m, "Inflate", zs);
    class_set_alloc(c, inf, AllocKind::None);
    define_singleton_cfunc(c, inf, "inflate", inf_s_inflate, 1);
    define_singleton_cfunc(c, m, "inflate", inf_s_inflate, 1);
    define_private_cfunc(c, inf, "initialize", inf_initialize, -1);
    let z = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, inf, n, f, a);
    z(c, "add_dictionary", inf_add_dictionary, 1); z(c, "inflate", inf_inflate, -1); z(c, "<<", inf_addstr, 1); z(c, "sync", inf_sync, 1);
    z(c, "sync_point?", inf_sync_point_p, 0); z(c, "set_dictionary", inf_set_dictionary, 1);
    ci(c, "NO_COMPRESSION", 0); ci(c, "BEST_SPEED", 1); ci(c, "BEST_COMPRESSION", 9); ci(c, "DEFAULT_COMPRESSION", -1);
    ci(c, "FILTERED", 1); ci(c, "HUFFMAN_ONLY", 2); ci(c, "RLE", 3); ci(c, "FIXED", 4); ci(c, "DEFAULT_STRATEGY", 0);
    ci(c, "MAX_WBITS", 15); ci(c, "DEF_MEM_LEVEL", 8); ci(c, "MAX_MEM_LEVEL", 9);
    ci(c, "NO_FLUSH", 0); ci(c, "SYNC_FLUSH", 2); ci(c, "FULL_FLUSH", 3); ci(c, "FINISH", 4);
    let gzf = define_class_under(c, m, "GzipFile", object);
    class_set_alloc(c, gzf, AllocKind::None);
    let gz_error = define_class_under(c, gzf, "Error", error);
    k.gz_error = gz_error;
    k.no_footer = define_class_under(c, gzf, "NoFooter", gz_error);
    k.crc_error = define_class_under(c, gzf, "CRCError", gz_error);
    k.length_error = define_class_under(c, gzf, "LengthError", gz_error);
    let gzw = define_class_under(c, m, "GzipWriter", gzf);
    let gzr = define_class_under(c, m, "GzipReader", gzf);
    class_set_alloc(c, gzw, AllocKind::None);
    class_set_alloc(c, gzr, AllocKind::None);
    k.deflate = df;
    k.inflate = inf;
    k.gzwriter = gzw;
    k.gzreader = gzr;
    ZK.with(|z| z.set(Some(k)));
    let z = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, gzf, n, f, a);
    z(c, "to_io", gzf_to_io, 0); z(c, "crc", gzf_crc, 0); z(c, "mtime", gzf_mtime, 0); z(c, "level", gzf_level, 0); z(c, "os_code", gzf_os_code, 0);
    z(c, "orig_name", gzf_orig_name, 0); z(c, "comment", gzf_comment, 0); z(c, "close", gzf_close, 0); z(c, "finish", gzf_finish, 0);
    z(c, "closed?", gzf_closed_p, 0); z(c, "sync", gzf_sync, 0); z(c, "sync=", gzf_set_sync, 1);
    let r = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, gzr, n, f, a);
    r(c, "lineno", gzf_lineno, 0); r(c, "lineno=", gzf_set_lineno, 1); r(c, "eof", gzf_eof_p, 0); r(c, "eof?", gzf_eof_p, 0);
    r(c, "pos", gzf_total_out, 0); r(c, "tell", gzf_total_out, 0);
    let w = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, gzw, n, f, a);
    w(c, "mtime=", gzw_set_mtime, 1); w(c, "orig_name=", gzw_set_orig_name, 1); w(c, "comment=", gzw_set_comment, 1);
    w(c, "pos", gzf_total_in, 0); w(c, "tell", gzf_total_in, 0);
    define_private_cfunc(c, gzw, "initialize", gzw_initialize, -1);
    w(c, "flush", gzw_flush, -1); w(c, "write", gzw_write, -1); w(c, "putc", gzw_putc, 1);
    define_private_cfunc(c, gzr, "initialize", gzr_initialize, -1);
    r(c, "rewind", gzr_rewind, 0); r(c, "unused", gzr_unused, 0); r(c, "read", gzr_read, -1); r(c, "readpartial", gzr_readpartial, -1);
    r(c, "getc", gzr_getc, 0); r(c, "getbyte", gzr_getbyte, 0); r(c, "readchar", gzr_readchar, 0); r(c, "readbyte", gzr_readbyte, 0);
    r(c, "each_byte", gzr_each_byte, 0); r(c, "each_char", gzr_each_char, 0); r(c, "ungetc", gzr_ungetc, 1); r(c, "ungetbyte", gzr_ungetbyte, 1);
    r(c, "gets", gzr_gets, -1); r(c, "readline", gzr_readline, -1); r(c, "each", gzr_each, -1); r(c, "each_line", gzr_each, -1);
    r(c, "readlines", gzr_readlines, -1); r(c, "external_encoding", gzr_external_encoding, 0);
    define_singleton_cfunc(c, m, "gzip", z_gzip, -1);
    define_singleton_cfunc(c, m, "gunzip", z_gunzip, 1);
    for (n, v) in [("OS_CODE", 3), ("OS_MSDOS", 0), ("OS_AMIGA", 1), ("OS_VMS", 2), ("OS_UNIX", 3), ("OS_ATARI", 5), ("OS_OS2", 6), ("OS_MACOS", 7),
        ("OS_TOPS20", 10), ("OS_WIN32", 11), ("OS_VMCMS", 4), ("OS_ZSYSTEM", 8), ("OS_CPM", 9), ("OS_QDOS", 12), ("OS_RISCOS", 13), ("OS_UNKNOWN", 255)] { ci(c, n, v); }
    let _ = crate::builtins::kernel::run_internal_source(c, RUBY_PART.as_bytes(), "<internal:zlib>");
}
