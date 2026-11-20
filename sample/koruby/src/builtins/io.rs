// IO / File / Dir (a practical subset).
use crate::array::*;
use crate::builtins::kernel::*;
use crate::object::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;
use std::io::{Read, Write};

pub static mut STDOUT_SYNC: bool = false;

pub enum IoKind { Stdout, Stderr, Stdin, File(Option<std::fs::File>), Str }
pub struct RIo {
    pub kind: IoKind, pub path: String, pub rbuf: Vec<u8>, pub rpos: usize, pub eof: bool, pub wbuf: Vec<u8>,
    pub closed: bool, pub lineno: i64, pub writable: bool, pub readable: bool, pub binmode: bool, pub sync: bool,
}
impl RData for RIo {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
pub fn rio(v: Value) -> Option<&'static mut RIo> { rdata::<RIo>(v) }

fn new_io(klass: Value, kind: IoKind, path: &str, r: bool, w: bool) -> Value {
    crate::gc::alloc(klass, Body::Data(Box::new(RIo { kind, path: path.into(), rbuf: vec![], rpos: 0, eof: false, wbuf: vec![], closed: false, lineno: 0, writable: w, readable: r, binmode: false, sync: false })))
}

fn io_flush_one(io: &mut RIo) {
    if io.wbuf.is_empty() { return; }
    match &mut io.kind {
        IoKind::File(Some(f)) => { let _ = f.write_all(&io.wbuf); }
        _ => {}
    }
    io.wbuf.clear();
}

pub fn flush_all() {
    out_flush();
    crate::gc::each_object(|v| { if let Some(io) = rio(v) { io_flush_one(io); } });
}

fn io_write_bytes(c: &mut Ctx, io_v: Value, b: &[u8]) -> Result<(), Esc> {
    let Some(io) = rio(io_v) else { return type_err(c, "not an IO"); };
    if io.closed { return err(c, vm().c_io_error, "closed stream"); }
    match &mut io.kind {
        IoKind::Stdout => out_write(b),
        IoKind::Stderr => err_write(b),
        IoKind::File(Some(_)) => {
            if !io.writable { return err(c, vm().c_io_error, "not opened for writing"); }
            io.wbuf.extend_from_slice(b);
            if io.wbuf.len() > 1 << 16 || io.sync { io_flush_one(io); }
        }
        _ => return err(c, vm().c_io_error, "not opened for writing"),
    }
    Ok(())
}

fn fill(io: &mut RIo) -> bool {
    if io.rpos < io.rbuf.len() { return true; }
    if io.eof { return false; }
    io.rbuf.clear(); io.rpos = 0;
    let mut buf = vec![0u8; 65536];
    let n = match &mut io.kind {
        IoKind::Stdin => { out_flush(); std::io::stdin().read(&mut buf).unwrap_or(0) }
        IoKind::File(Some(f)) => f.read(&mut buf).unwrap_or(0),
        _ => 0,
    };
    if n == 0 { io.eof = true; return false; }
    io.rbuf.extend_from_slice(&buf[..n]);
    true
}
fn read_line(io: &mut RIo, sep: Option<&[u8]>, limit: Option<usize>) -> Option<Vec<u8>> {
    let mut line = vec![];
    loop {
        if !fill(io) { break; }
        let avail = &io.rbuf[io.rpos..];
        match sep {
            Some(sp) if !sp.is_empty() => {
                // search for separator across the line buffer
                let mut found = None;
                for (k, &b) in avail.iter().enumerate() {
                    line.push(b);
                    if line.ends_with(sp) { found = Some(k + 1); break; }
                    if let Some(l) = limit { if line.len() >= l { found = Some(k + 1); break; } }
                }
                match found { Some(k) => { io.rpos += k; return Some(line); } None => { io.rpos = io.rbuf.len(); } }
            }
            _ => { line.extend_from_slice(avail); io.rpos = io.rbuf.len(); }
        }
    }
    if line.is_empty() { None } else { Some(line) }
}
fn read_all(io: &mut RIo) -> Vec<u8> {
    let mut v = vec![];
    while fill(io) { v.extend_from_slice(&io.rbuf[io.rpos..]); io.rpos = io.rbuf.len(); }
    v
}

fn io_write(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mut n = 0;
    for &x in a { let sv = crate::rt::rt_tostr(c, x)?; let b = str_bytes(sv).to_vec(); n += b.len(); io_write_bytes(c, slf, &b)?; }
    Ok(Value::fix(n as i64))
}
fn io_puts(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let out = puts_bytes(c, a)?; io_write_bytes(c, slf, &out)?; Ok(NIL) }
fn io_print(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mut out = vec![];
    for &x in a { let sv = crate::rt::rt_tostr(c, x)?; out.extend_from_slice(str_bytes(sv)); }
    io_write_bytes(c, slf, &out)?; Ok(NIL)
}
fn io_ltlt(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { io_write(c, slf, a, b)?; Ok(slf) }
fn io_printf(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return Ok(NIL); }
    let s2 = crate::builtins::sprintf::format(c, a[0], &a[1..])?;
    let b = str_bytes(s2).to_vec();
    io_write_bytes(c, slf, &b)?; Ok(NIL)
}
fn io_p(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mut out = vec![];
    for &x in a { let s2 = inspect_value(c, x)?; out.extend_from_slice(str_bytes(s2)); out.push(b'\n'); }
    io_write_bytes(c, slf, &out)?; Ok(NIL)
}
fn io_putc(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let b = if is_string(a[0]) { vec![str_bytes(a[0])[0]] } else { vec![crate::builtins::numeric_methods::num_to_long(c, a[0])? as u8] };
    io_write_bytes(c, slf, &b)?; Ok(a[0])
}
fn io_flush(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if let Some(io) = rio(slf) { match io.kind { IoKind::Stdout => out_flush(), _ => io_flush_one(io) } }
    Ok(slf)
}
fn io_sync(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(rio(slf).map(|i| i.sync || matches!(i.kind, IoKind::Stderr)).unwrap_or(false))) }
fn io_set_sync(_c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if let Some(io) = rio(slf) { io.sync = a[0].truthy(); if matches!(io.kind, IoKind::Stdout) { unsafe { STDOUT_SYNC = io.sync; } } }
    Ok(a[0])
}
fn io_gets(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let Some(io) = rio(slf) else { return Ok(NIL) };
    let mut sep: Option<Vec<u8>> = Some(b"\n".to_vec()); let mut chomp = false; let mut limit = None;
    for &x in a {
        if is_hash(x) { if let Some(v) = crate::hash::hash_get_opt(c, x, sym_value("chomp"))? { chomp = v.truthy(); } }
        else if x == NIL { sep = None; } else if is_string(x) { sep = Some(str_bytes(x).to_vec()); if sep.as_ref().unwrap().is_empty() { sep = Some(b"\n\n".to_vec()); } }
        else if x.is_fixnum() { limit = Some(x.as_fix() as usize); }
    }
    let line = read_line(io, sep.as_deref(), limit);
    let v = match line { Some(mut l) => { io.lineno += 1; if chomp { if let Some(sp) = &sep { if l.ends_with(sp) { l.truncate(l.len() - sp.len()); } } } str_from_vec(l) } None => NIL };
    vm().globals.insert(intern(b"$_"), v);
    Ok(v)
}
fn io_readline(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let v = io_gets(c, slf, a, b)?;
    if v == NIL { return err(c, vm().c_eof_error, "end of file reached"); }
    Ok(v)
}
fn io_each_line(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_line"), a, NIL); }
    loop { let l = io_gets(c, slf, a, std::ptr::null())?; if l == NIL { break; } yield1(c, blk, l)?; }
    Ok(slf)
}
fn io_readlines(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let out = ary_new();
    loop { let l = io_gets(c, slf, a, std::ptr::null())?; if l == NIL { break; } ary_push(out, l); }
    Ok(out)
}
fn io_read(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let Some(io) = rio(slf) else { return Ok(NIL) };
    if io.closed { return err(c, vm().c_io_error, "closed stream"); }
    if let Some(&n) = a.first() {
        if n != NIL {
            let n = n.as_fix() as usize;
            let mut v = vec![];
            while v.len() < n && fill(io) { let take = (n - v.len()).min(io.rbuf.len() - io.rpos); v.extend_from_slice(&io.rbuf[io.rpos..io.rpos + take]); io.rpos += take; }
            if v.is_empty() && n > 0 { return Ok(NIL); }
            return Ok(str_from_vec_enc(v, ENC_BINARY));
        }
    }
    Ok(str_from_vec(read_all(io)))
}
fn io_getc(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let Some(io) = rio(slf) else { return Ok(NIL) };
    if !fill(io) { return Ok(NIL); }
    let l = char_len_at(&io.rbuf, io.rpos).min(io.rbuf.len() - io.rpos);
    let v = io.rbuf[io.rpos..io.rpos + l].to_vec(); io.rpos += l;
    Ok(str_from_vec(v))
}
fn io_eof(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(rio(slf).map(|io| !fill(io)).unwrap_or(true))) }
fn io_close(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if let Some(io) = rio(slf) { io_flush_one(io); io.closed = true; if let IoKind::File(f) = &mut io.kind { *f = None; } }
    Ok(NIL)
}
fn io_closed(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(rio(slf).map(|i| i.closed).unwrap_or(false))) }
fn io_fileno(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    Ok(Value::fix(match rio(slf).map(|i| &i.kind) { Some(IoKind::Stdin) => 0, Some(IoKind::Stdout) => 1, Some(IoKind::Stderr) => 2, _ => 3 }))
}
fn io_tty(_c: &mut Ctx, _slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(FALSE) }
fn io_path(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(str_new(rio(slf).map(|i| i.path.clone()).unwrap_or_default().as_bytes())) }
fn io_binmode(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { if let Some(i) = rio(slf) { i.binmode = true; } Ok(slf) }
fn io_inspect(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let p = rio(slf).map(|i| i.path.clone()).unwrap_or_default();
    Ok(str_new(format!("#<{}:{}>", class_path(real_class_of(slf)), p).as_bytes()))
}
fn io_lineno(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(rio(slf).map(|i| i.lineno).unwrap_or(0))) }
fn io_rewind(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if let Some(io) = rio(slf) { if let IoKind::File(Some(f)) = &mut io.kind { use std::io::Seek; let _ = f.seek(std::io::SeekFrom::Start(0)); } io.rbuf.clear(); io.rpos = 0; io.eof = false; io.lineno = 0; }
    Ok(Value::fix(0))
}

fn open_file(c: &mut Ctx, path: &str, mode: &str) -> RV {
    let mut o = std::fs::OpenOptions::new();
    let m = mode.replace('b', "").replace('t', "");
    let m = m.split(':').next().unwrap_or("r").to_string();
    match m.as_str() {
        "r" => { o.read(true); }
        "w" => { o.write(true).create(true).truncate(true); }
        "a" => { o.append(true).create(true); }
        "r+" => { o.read(true).write(true); }
        "w+" => { o.read(true).write(true).create(true).truncate(true); }
        "a+" => { o.read(true).append(true).create(true); }
        _ => { return arg_err(c, format!("invalid access mode {}", mode)); }
    }
    match o.open(path) {
        Ok(f) => Ok(new_io(vm().c_file, IoKind::File(Some(f)), path, m.contains('r') || m.contains('+'), m != "r")),
        Err(e) => Err(crate::builtins::exception::raise_errno(c, crate::builtins::exception::io_err_to_errno(&e), &format!("rb_sysopen - {}", path))),
    }
}
fn mode_arg(c: &mut Ctx, a: &[Value]) -> Result<String, Esc> {
    for &x in a.iter().skip(1) {
        if is_string(x) { return Ok(String::from_utf8_lossy(str_bytes(x)).into_owned()); }
        if is_hash(x) { if let Some(m) = crate::hash::hash_get_opt(c, x, sym_value("mode"))? { return to_rstring(c, m); } }
    }
    Ok("r".into())
}
fn path_arg(c: &mut Ctx, v: Value) -> Result<String, Esc> {
    if is_string(v) { return Ok(String::from_utf8_lossy(str_bytes(v)).into_owned()); }
    if respond_to(v, intern(b"to_path"), true) { let p = funcall(c, v, intern(b"to_path"), &[], std::ptr::null())?; return to_rstring(c, p); }
    let s2 = expect_str(c, v)?;
    Ok(String::from_utf8_lossy(str_bytes(s2)).into_owned())
}
fn f_s_open(c: &mut Ctx, _slf: Value, a: &[Value], blk: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, 3); }
    let p = path_arg(c, a[0])?;
    let m = mode_arg(c, a)?;
    let f = open_file(c, &p, &m)?;
    if blk.is_null() { return Ok(f); }
    let r = yield1(c, blk, f);
    io_close(c, f, &[], std::ptr::null())?;
    r
}
fn f_s_read(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    match std::fs::read(&p) { Ok(b) => Ok(str_from_vec(b)), Err(e) => Err(crate::builtins::exception::raise_errno(c, crate::builtins::exception::io_err_to_errno(&e), &format!("rb_sysopen - {}", p))) }
}
fn f_s_binread(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { let r = f_s_read(c, s, a, b)?; rstr(r).enc = ENC_BINARY; Ok(r) }
fn f_s_readlines(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV {
    let all = f_s_read(c, s, &a[..1], b)?;
    funcall(c, all, intern(b"lines"), &a[1..], std::ptr::null())
}
fn f_s_foreach(c: &mut Ctx, s: Value, a: &[Value], blk: *const Block) -> RV {
    let lines = f_s_readlines(c, s, a, std::ptr::null())?;
    if blk.is_null() { return funcall(c, lines, s::each, &[], std::ptr::null()); }
    for i in 0..ary_len(lines) { yield1(c, blk, ary_get(lines, i))?; }
    Ok(NIL)
}
fn f_s_write(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    let sv = crate::rt::rt_tostr(c, a[1])?;
    let append = a.len() > 2 && is_hash(a[2]) && crate::hash::hash_get_opt(c, a[2], sym_value("mode"))?.map(|m| is_string(m) && str_bytes(m).starts_with(b"a")).unwrap_or(false);
    let r = if append { std::fs::OpenOptions::new().append(true).create(true).open(&p).and_then(|mut f| f.write_all(str_bytes(sv))) } else { std::fs::write(&p, str_bytes(sv)) };
    match r { Ok(_) => Ok(Value::fix(str_bytes(sv).len() as i64)), Err(e) => Err(crate::builtins::exception::raise_errno(c, crate::builtins::exception::io_err_to_errno(&e), &format!("rb_sysopen - {}", p))) }
}
fn f_s_exist(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { let p = path_arg(c, a[0])?; Ok(Value::bool(std::path::Path::new(&p).exists())) }
fn f_s_file_p(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { let p = path_arg(c, a[0])?; Ok(Value::bool(std::path::Path::new(&p).is_file())) }
fn f_s_dir_p(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { let p = path_arg(c, a[0])?; Ok(Value::bool(std::path::Path::new(&p).is_dir())) }
fn f_s_size(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    match std::fs::metadata(&p) { Ok(m) => Ok(Value::fix(m.len() as i64)), Err(e) => Err(crate::builtins::exception::raise_errno(c, crate::builtins::exception::io_err_to_errno(&e), &format!("rb_file_s_size - {}", p))) }
}
fn f_s_delete(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    for &x in a { let p = path_arg(c, x)?; if let Err(e) = std::fs::remove_file(&p) { return Err(crate::builtins::exception::raise_errno(c, crate::builtins::exception::io_err_to_errno(&e), &format!("apply2files - {}", p))); } }
    Ok(Value::fix(a.len() as i64))
}
fn f_s_basename(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    let t = p.trim_end_matches('/');
    let mut base = if t.is_empty() && p.starts_with('/') { "/".to_string() } else { t.rsplit('/').next().unwrap_or("").to_string() };
    if a.len() > 1 {
        let suf = to_rstring(c, a[1])?;
        if suf == ".*" { if let Some(i) = base.rfind('.') { if i > 0 { base.truncate(i); } } }
        else if base.ends_with(&suf) && base != suf { base.truncate(base.len() - suf.len()); }
    }
    Ok(str_new(base.as_bytes()))
}
fn f_s_dirname(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    let t = p.trim_end_matches('/');
    let d = match t.rfind('/') { Some(0) => "/".to_string(), Some(i) => t[..i].trim_end_matches('/').to_string(), None => if p.starts_with('/') { "/".into() } else { ".".into() } };
    Ok(str_new(if d.is_empty() { "/" } else { &d }.as_bytes()))
}
fn f_s_extname(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    let base = p.rsplit('/').next().unwrap_or("");
    let e = match base.rfind('.') { Some(i) if i > 0 && i + 1 < base.len() => base[i..].to_string(), _ => String::new() };
    Ok(str_new(e.as_bytes()))
}
fn f_s_join(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mut parts = vec![];
    fn flat(c: &mut Ctx, v: Value, out: &mut Vec<String>) -> Result<(), Esc> { if is_array(v) { for i in 0..ary_len(v) { flat(c, ary_get(v, i), out)?; } Ok(()) } else { out.push(path_arg(c, v)?); Ok(()) } }
    for &x in a { flat(c, x, &mut parts)?; }
    let mut out = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i == 0 { out.push_str(p); continue; }
        match (out.ends_with('/'), p.starts_with('/')) { (true, true) => out.push_str(&p[1..]), (false, false) => { out.push('/'); out.push_str(p); } _ => out.push_str(p) }
    }
    Ok(str_new(out.as_bytes()))
}
fn f_s_expand_path(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    let base = if a.len() > 1 && a[1] != NIL { path_arg(c, a[1])? } else { std::env::current_dir().map(|d| d.to_string_lossy().into_owned()).unwrap_or("/".into()) };
    let p = if let Some(r) = p.strip_prefix('~') { format!("{}{}", std::env::var("HOME").unwrap_or_default(), r) } else { p };
    let full = if p.starts_with('/') { p } else { format!("{}/{}", base, p) };
    let mut comps: Vec<&str> = vec![];
    for x in full.split('/') { match x { "" | "." => {} ".." => { comps.pop(); } y => comps.push(y) } }
    Ok(str_new(format!("/{}", comps.join("/")).as_bytes()))
}
fn f_s_absolute_path(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { f_s_expand_path(c, s, a, b) }
fn f_s_mtime(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    let m = std::fs::metadata(&p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    crate::builtins::time::time_at(c, m)
}
fn d_s_pwd(_c: &mut Ctx, _slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(str_new(std::env::current_dir().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default().as_bytes())) }
fn d_s_glob(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV {
    let pat = path_arg(c, a[0])?;
    let out = ary_new();
    // simple glob: directory part + wildcard in the last component
    let (dir, file) = match pat.rfind('/') { Some(i) => (pat[..i].to_string(), pat[i + 1..].to_string()), None => (".".into(), pat.clone()) };
    if let Ok(rd) = std::fs::read_dir(&dir) {
        let mut names: Vec<String> = rd.filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned())).collect();
        names.sort();
        for n in names {
            if glob_match(file.as_bytes(), n.as_bytes()) && !n.starts_with('.') {
                let full = if pat.contains('/') { format!("{}/{}", dir, n) } else { n };
                ary_push(out, str_new(full.as_bytes()));
            }
        }
    }
    Ok(out)
}
fn glob_match(p: &[u8], s2: &[u8]) -> bool {
    if p.is_empty() { return s2.is_empty(); }
    match p[0] {
        b'*' => (0..=s2.len()).any(|i| glob_match(&p[1..], &s2[i..])),
        b'?' => !s2.is_empty() && glob_match(&p[1..], &s2[1..]),
        x => !s2.is_empty() && s2[0] == x && glob_match(&p[1..], &s2[1..]),
    }
}
fn d_s_exist(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { f_s_dir_p(c, s, a, b) }
fn d_s_mkdir(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let p = path_arg(c, a[0])?; match std::fs::create_dir(&p) { Ok(_) => Ok(Value::fix(0)), Err(e) => Err(crate::builtins::exception::raise_errno(c, crate::builtins::exception::io_err_to_errno(&e), &p)) } }
fn d_s_entries(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let p = path_arg(c, a[0])?;
    let out = ary_new();
    ary_push(out, str_new(b".")); ary_push(out, str_new(b".."));
    if let Ok(rd) = std::fs::read_dir(&p) { for e in rd.flatten() { ary_push(out, str_new(e.file_name().to_string_lossy().as_bytes())); } }
    Ok(out)
}

pub fn setup_data(src: &[u8], _file: &str) {
    if let Some(pos) = find_end_marker(src) {
        let data = src[pos..].to_vec();
        let io = new_io(vm().c_file, IoKind::File(None), "DATA", true, false);
        if let Some(r) = rio(io) { r.rbuf = data; r.eof = true; }
        const_set(vm().c_object, intern(b"DATA"), io);
    }
}
fn find_end_marker(src: &[u8]) -> Option<usize> {
    let mut i = 0;
    for line in src.split(|&b| b == b'\n') {
        if line == b"__END__" || line == b"__END__\r" { let p = i + line.len() + 1; return Some(p.min(src.len())); }
        i += line.len() + 1;
    }
    None
}

pub fn init() {
    let vm = vm();
    let io = define_class("IO", vm.c_object);
    rclass(io).alloc = AllocKind::None;
    include_module(io, vm.m_enumerable);
    vm.c_io = io;
    let f = define_class("File", io);
    vm.c_file = f;
    let d = |n: &str, fun: CFunc, a: i32| define_cfunc(io, n, fun, a);
    d("write", io_write, -1); d("puts", io_puts, -1); d("print", io_print, -1); d("<<", io_ltlt, 1); d("printf", io_printf, -1); d("p", io_p, -1); d("putc", io_putc, 1);
    d("flush", io_flush, 0); d("fsync", io_flush, 0); d("sync", io_sync, 0); d("sync=", io_set_sync, 1); d("gets", io_gets, -1); d("readline", io_readline, -1);
    d("each_line", io_each_line, -1); d("each", io_each_line, -1); d("readlines", io_readlines, -1); d("read", io_read, -1); d("getc", io_getc, 0);
    d("eof?", io_eof, 0); d("eof", io_eof, 0); d("close", io_close, 0); d("closed?", io_closed, 0); d("fileno", io_fileno, 0); d("to_i", io_fileno, 0);
    d("tty?", io_tty, 0); d("isatty", io_tty, 0); d("path", io_path, 0); d("to_path", io_path, 0); d("binmode", io_binmode, 0); d("inspect", io_inspect, 0);
    d("lineno", io_lineno, 0); d("rewind", io_rewind, 0);
    let df = |n: &str, fun: CFunc, a: i32| define_singleton_cfunc(f, n, fun, a);
    df("open", f_s_open, -1); df("new", f_s_open, -1); df("read", f_s_read, -1); df("binread", f_s_binread, -1); df("readlines", f_s_readlines, -1);
    df("foreach", f_s_foreach, -1); df("write", f_s_write, -1); df("binwrite", f_s_write, -1); df("exist?", f_s_exist, 1); df("file?", f_s_file_p, 1);
    df("directory?", f_s_dir_p, 1); df("size", f_s_size, 1); df("delete", f_s_delete, -1); df("unlink", f_s_delete, -1); df("basename", f_s_basename, -1);
    df("dirname", f_s_dirname, -1); df("extname", f_s_extname, 1); df("join", f_s_join, -1); df("expand_path", f_s_expand_path, -1);
    df("absolute_path", f_s_absolute_path, -1); df("mtime", f_s_mtime, 1); df("readable?", f_s_exist, 1);
    define_singleton_cfunc(io, "read", f_s_read, -1);
    define_singleton_cfunc(io, "readlines", f_s_readlines, -1);
    define_singleton_cfunc(io, "write", f_s_write, -1);
    define_singleton_cfunc(io, "foreach", f_s_foreach, -1);
    const_set(f, intern(b"SEPARATOR"), str_new_frozen(b"/"));
    const_set(f, intern(b"ALT_SEPARATOR"), NIL);
    const_set(f, intern(b"PATH_SEPARATOR"), str_new_frozen(b":"));
    let dir = define_class("Dir", vm.c_object);
    vm.c_dir = dir;
    define_singleton_cfunc(dir, "pwd", d_s_pwd, 0);
    define_singleton_cfunc(dir, "getwd", d_s_pwd, 0);
    define_singleton_cfunc(dir, "glob", d_s_glob, -1);
    define_singleton_cfunc(dir, "[]", d_s_glob, -1);
    define_singleton_cfunc(dir, "exist?", d_s_exist, 1);
    define_singleton_cfunc(dir, "mkdir", d_s_mkdir, -1);
    define_singleton_cfunc(dir, "entries", d_s_entries, 1);
    let so = new_io(io, IoKind::Stdout, "<STDOUT>", false, true);
    let se = new_io(io, IoKind::Stderr, "<STDERR>", false, true);
    let si = new_io(io, IoKind::Stdin, "<STDIN>", true, false);
    vm.stdout = so; vm.stderr = se; vm.stdin = si;
    const_set(vm.c_object, intern(b"STDOUT"), so);
    const_set(vm.c_object, intern(b"STDERR"), se);
    const_set(vm.c_object, intern(b"STDIN"), si);
    vm.globals.insert(intern(b"$stdout"), so);
    vm.globals.insert(intern(b"$stderr"), se);
    vm.globals.insert(intern(b"$stdin"), si);
    vm.globals.insert(intern(b"$>"), so);
    vm.globals.insert(intern(b"$/"), str_new_frozen(b"\n"));
    let _ = s::puts;
}
