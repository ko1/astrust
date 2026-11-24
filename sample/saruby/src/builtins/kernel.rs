// Kernel: output, conversion functions, raise/catch/loop, require, gvars, misc.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{obj_flags, obj_set_flags};
use crate::array::*;
use crate::hash::*;
use crate::string::*;
use crate::symbol::{intern, intern_str, s, sym_bytes, sym_str};
use crate::value::*;
use crate::vm::*;
use std::io::Write;

// ---- output ---------------------------------------------------------------------------

// one interpreter thread (fibers and Threads run on it), so a thread-local is the process's buffer
thread_local! { static STDOUT_BUF: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) }; }
pub fn out_write(b: &[u8]) {
    let full = STDOUT_BUF.with_borrow_mut(|buf| { buf.extend_from_slice(b); buf.len() > 1 << 16 });
    if full || crate::builtins::io::STDOUT_SYNC.load(std::sync::atomic::Ordering::Relaxed) { out_flush(); }
}
// errno of a failed flush of STDOUT_BUF, raised by out_check where a context is at hand
thread_local! { static STDOUT_ERR: std::cell::Cell<i32> = const { std::cell::Cell::new(0) }; }
pub fn out_flush() {
    let _ = STDOUT_BUF.try_with(|cell| {
        let mut buf = cell.borrow_mut();
        if !buf.is_empty() {
            let mut so = std::io::stdout().lock();
            let r = so.write_all(&buf).and_then(|_| so.flush());
            buf.clear();
            if let Err(e) = r { STDOUT_ERR.with(|x| x.set(e.raw_os_error().unwrap_or(5))); }
        }
    });
}
/// Raise what the last stdout flush hit (rb_sys_fail_on_write); an EPIPE carries SIGPIPE,
/// which kills the process if nothing rescues it (raise_on_write).
pub fn out_check(c: &mut Ctx) -> Result<(), Esc> {
    let e = STDOUT_ERR.with(|x| x.replace(0));
    if e == 0 { return Ok(()); }
    let r = crate::builtins::io::raise_errno_at(c, e as i64, "rb_sys_fail_on_write - ", b"<STDOUT>");
    if e == 32 { ivar_set_raw(c, c.esc_val, intern(b"signo"), Value::fix(13)); }
    Err(r)
}
/// err_write for IO#write and friends on fd 2: the error is the caller's (a closed STDERR raises, as in CRuby).
pub fn err_write_io(b: &[u8]) -> Option<i64> {
    out_flush();
    let mut se = std::io::stderr().lock();
    let r = se.write_all(b).and_then(|_| se.flush());
    r.err().map(|e| e.raw_os_error().unwrap_or(5) as i64)
}
pub fn err_write(b: &[u8]) {
    out_flush();
    let mut se = std::io::stderr().lock();
    let _ = se.write_all(b);
    let _ = se.flush();
}

/// Write to $stdout (dispatching when it was reassigned).
pub fn stdout_write(c: &mut Ctx, b: &[u8]) -> Result<(), Esc> {
    let so = *c.vm().globals.borrow().get(&intern(b"$stdout")).unwrap_or(&c.vm().stdout.get());
    if so == c.vm().stdout.get() && !stdout_slow() { out_write(b); return out_check(c); }
    if so == c.vm().stderr.get() && !crate::builtins::io::out_redef() { err_write(b); return Ok(()); }
    let st = c.str_new(b);
    funcall(c, so, intern(b"write"), &[st], std::ptr::null())?;
    Ok(())
}
/// Write to $stderr (dispatching when it was reassigned).
pub fn stderr_write(c: &mut Ctx, b: &[u8]) -> Result<(), Esc> {
    let se = *c.vm().globals.borrow().get(&intern(b"$stderr")).unwrap_or(&c.vm().stderr.get());
    if se == c.vm().stderr.get() { err_write(b); return Ok(()); }
    let st = c.str_new(b);
    funcall(c, se, intern(b"write"), &[st], std::ptr::null())?;
    Ok(())
}
/// A warning goes through Warning.warn, as rb_warn does (so overrides and $stderr see it).
pub fn warning_write(c: &mut Ctx, b: &[u8]) -> Result<(), Esc> {
    let st = c.str_new(b);
    let w = c.vm().m_warning.get();
    funcall(c, w, intern(b"warn"), &[st], std::ptr::null())?;
    Ok(())
}
/// rb_warn_category: Warning.warn(str, category: cat) unless Warning.warn takes exactly one argument.
fn warning_write_cat(c: &mut Ctx, b: &[u8], cat: Value) -> Result<(), Esc> {
    let w = c.vm().m_warning.get();
    let sw = singleton_class(c, w);
    let me = search_method(c, sw, intern(b"warn"));
    if crate::builtins::proc::method_arity(c, me) == 1 { return warning_write(c, b); }
    let st = c.str_new(b);
    let h = c.hash_new();
    hash_set(c, h, sym_value("category"), cat)?;
    funcall_kw(c, w, intern(b"warn"), &[st, h], std::ptr::null(), true)?;
    Ok(())
}

// ---- string conversion helpers ------------------------------------------------------------

pub fn any_to_s_str(c: &Ctx, v: Value) -> String {
    let k = real_class_of(c, v);
    format!("#<{}:{:#018x}>", class_path(c, k), v.raw())
}

pub fn inspect_str(c: &mut Ctx, v: Value) -> Result<String, Esc> {
    let r = inspect_value(c, v)?;
    Ok(String::from_utf8_lossy(c.str_bytes(r)).into_owned())
}

/// rb_inspect: v.inspect, as a String by rb_obj_as_string.
pub fn inspect_value(c: &mut Ctx, v: Value) -> RV {
    if (v == NIL || v.is_fixnum()) && !crate::rt::redef(crate::rt::R_IMM_INSPECT) {
        return Ok(if v == NIL { c.str_new_enc(b"nil", crate::object::ENC_ASCII) } else { c.str_new_enc(v.as_fix().to_string().as_bytes(), crate::object::ENC_ASCII) });
    }
    let r = funcall(c, v, s::inspect, &[], std::ptr::null())?;
    let r = if is_string(r) { r } else {
        let t = funcall(c, r, s::to_s, &[], std::ptr::null())?;
        if is_string(t) { t } else { c.str_new(any_to_s_str(c, r).as_bytes()) }
    };
    // rb_inspect: a result not in the default internal / external encoding is escaped
    use crate::builtins::encoding as en;
    let e = en::default_internal().unwrap_or_else(en::default_external);
    let re = c.str_enc(r);
    if (!en::ascii_compat(e) || re != e) && !(en::ascii_compat(re) && c.str_bytes(r).is_ascii()) { return Ok(str_escape(c, r)); }
    Ok(r)
}
/// rb_str_escape: a US-ASCII copy with non-printable / non-ASCII characters escaped.
pub fn str_escape(c: &mut Ctx, v: Value) -> Value {
    use crate::builtins::encoding as en;
    let enc = c.str_enc(v);
    let b = c.str_bytes(v).to_vec();
    let (uni, ac) = (en::is_unicode(enc), en::ascii_compat(enc));
    let mut out = Vec::with_capacity(b.len());
    let mut p = 0;
    while p < b.len() {
        let Some(n) = en::precise_len(enc, &b, p) else {
            let n = en::min_len(enc).min(b.len() - p);
            for &x in &b[p..p + n] { out.extend_from_slice(format!("\\x{:02X}", x).as_bytes()); }
            p += n;
            continue;
        };
        let ch = &b[p..p + n];
        let cp = en::char_code(enc, ch);
        p += n;
        let esc: Option<&str> = match cp { 0 => Some("\\0"), 0x0a => Some("\\n"), 0x0d => Some("\\r"), 0x09 => Some("\\t"), 0x0c => Some("\\f"), 0x0b => Some("\\v"), 0x08 => Some("\\b"), 0x07 => Some("\\a"), 0x1b => Some("\\e"), 0x7f => Some("\\c?"), _ => None };
        if let Some(e) = esc { out.extend_from_slice(e.as_bytes()); }
        else if ac && cp < 0x80 && (0x20..0x7f).contains(&cp) { out.extend_from_slice(ch); }
        else if uni {
            if cp < 0x7f && (0x20..0x7f).contains(&cp) { out.push(cp as u8); }
            else if cp < 0x10000 { out.extend_from_slice(format!("\\u{:04X}", cp).as_bytes()); }
            else { out.extend_from_slice(format!("\\u{{{:X}}}", cp).as_bytes()); }
        } else if cp < 0x100 { out.extend_from_slice(format!("\\x{:02X}", cp).as_bytes()); }
        else { out.extend_from_slice(format!("\\x{{{:X}}}", cp).as_bytes()); }
    }
    c.str_from_vec_enc(out, crate::object::ENC_ASCII)
}

pub fn to_s_value(c: &mut Ctx, v: Value) -> RV { crate::rt::srb_tostr(c, v) }

pub fn nomethod_desc(c: &mut Ctx, v: Value) -> Result<String, Esc> {
    let _ = c;
    if v == NIL { return Ok("nil".into()); }
    if v == TRUE { return Ok("true".into()); }
    if v == FALSE { return Ok("false".into()); }
    if v == c.vm().main_obj.get() { return Ok("main".into()); }
    // name_err_mesg_receiver_name: a class is named by its #name (then #inspect)
    if is_class_or_module(v) {
        let kind = if c.class(v).is_module { "module" } else { "class" };
        return Ok(format!("{} {}", kind, crate::builtins::object::mesg_receiver_name(c, v)));
    }
    // name_err_mesg_to_str: an object with a singleton class is shown by rb_any_to_s.  saruby makes one where
    // CRuby does not (instance_eval), so only one with methods or modules counts
    if v.is_heap() { let k = c.class(class_of(v)); if k.is_singleton && (!k.mtbl.is_empty() || !k.includes.is_empty() || !k.prepends.is_empty()) { return Ok(any_to_s_str(c, v)); } }
    let k = real_class_of(c, v);
    Ok(format!("an instance of {}", crate::builtins::object::mesg_receiver_name(c, k)))
}

pub fn class_name_for_err(c: &Ctx, v: Value) -> String {
    if v == NIL { return "nil".into(); }
    if v == TRUE { return "true".into(); }
    if v == FALSE { return "false".into(); }
    class_path(c, real_class_of(c, v))
}

/// rb_respond_to: a redefined respond_to? is asked; Kernel#respond_to? answers from the
/// method table (public methods), then respond_to_missing?.
pub fn rb_respond_to(c: &mut Ctx, v: Value, mid: Sym) -> Result<bool, Esc> {
    let basic = search_method(c, c.vm().m_kernel.get(), s::respond_to_p);
    if lookup_method(c, class_of(v), s::respond_to_p) != basic {
        return Ok(funcall(c, v, s::respond_to_p, &[Value::sym(mid)], std::ptr::null())?.truthy());
    }
    if respond_to(c, v, mid, false) { return Ok(true); }
    if lookup_method(c, class_of(v), s::respond_to_missing_p) != search_method(c, c.vm().m_kernel.get(), s::respond_to_missing_p) {
        return Ok(funcall(c, v, s::respond_to_missing_p, &[Value::sym(mid), FALSE], std::ptr::null())?.truthy());
    }
    Ok(false)
}

// ---- recursion guard (inspect / hash / == of recursive structures) -------------------------

// context-free (legacy.rs): converted modules of earlier waves call them without a context
pub use crate::legacy::{recursion_enter, recursion_leave};

// CRuby's exec_recursive keeps {tag => {obj => true}} in a per-fiber Hash, readable as
// Thread.current[:__recursive_key__]. saruby keeps that Hash too once it is observable: after lib/pp.rb
// is loaded (it shares the :inspect list) or Hash#default is redefined (each new tag misses: rb_hash_aref).
pub static REC_PP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn rec_hash_on(tag: Sym) -> bool {
    crate::rt::redef(crate::rt::R_HASH_DEFAULT) || (tag == s::inspect && REC_PP.load(std::sync::atomic::Ordering::Relaxed))
}
fn ident_hash(c: &mut Ctx) -> Value { let h = c.hash_new(); c.hash_mut(h).identity = true; h }
/// The fiber's recursion Hash; a new one holds the tags the reference ruby's boot (rubygems) has used.
pub fn rec_outer(c: &mut Ctx) -> RV {
    let loc = crate::builtins::thread::current_locals(c)?;
    let key = Value::sym(intern(b"__recursive_key__"));
    if let Some(h) = hash_get_opt(c, loc, key)? { if is_hash(h) { return Ok(h); } }
    let h = ident_hash(c);
    for t in [&b"nonzero?"[..], b"hash", b"==", b"join", b"inspect"] { let l = ident_hash(c); hash_set(c, h, Value::sym(intern(t)), l)?; }
    hash_set(c, loc, key, h)?;
    Ok(h)
}
/// recursive_list_access
fn rec_list(c: &mut Ctx, tag: Sym) -> RV {
    let h = rec_outer(c)?;
    let l = hash_aref(c, h, Value::sym(tag))?;
    if is_hash(l) { return Ok(l); }
    let l = ident_hash(c);
    hash_aset(c, h, Value::sym(tag), l)?;
    Ok(l)
}
/// recursion_enter that also keeps the Hash above when it is observable.
pub fn rec_enter(c: &mut Ctx, v: Value, tag: Sym) -> Result<bool, Esc> {
    if !rec_hash_on(tag) { return Ok(recursion_enter(v, tag)); }
    let l = rec_list(c, tag)?;
    if hash_lookup(c, l, v)?.is_some() || !recursion_enter(v, tag) { return Ok(false); }
    hash_set(c, l, v, TRUE)?;
    Ok(true)
}
pub fn rec_leave(c: &mut Ctx, v: Value, tag: Sym) {
    recursion_leave(v, tag);
    if !rec_hash_on(tag) { return; }
    // identity Hashes: no Ruby code runs here
    let Ok(loc) = crate::builtins::thread::current_locals(c) else { return };
    let key = Value::sym(intern(b"__recursive_key__"));
    if let Ok(Some(h)) = hash_get_opt(c, loc, key) {
        if is_hash(h) { if let Ok(Some(l)) = hash_get_opt(c, h, Value::sym(tag)) { if is_hash(l) { let _ = hash_delete(c, l, v); } } }
    }
}

// ---- puts / print / p -------------------------------------------------------------------

/// rb_str_end_with_asciichar(str, '\n')
#[inline]
pub fn ends_with_newline(b: &[u8], enc: u8) -> bool {
    if enc == ENC_UTF8 || enc == ENC_ASCII || enc == ENC_BINARY { return b.last() == Some(&b'\n'); }
    ends_with_newline_enc(b, enc)
}
fn ends_with_newline_enc(b: &[u8], enc: u8) -> bool {
    use crate::builtins::encoding as en;
    let m = en::min_len(enc);
    if b.len() < m { return false; }
    if m == 1 { return b.last() == Some(&b'\n'); }
    let e = &b[b.len() - m..];
    let mut i = 0; while i < b.len() - m { i += en::mbclen(enc, b, i); }
    i == b.len() - m && en::char_code(enc, e) == 10
}
fn puts_one(c: &mut Ctx, v: Value, out: &mut Vec<u8>) -> Result<(), Esc> {
    if is_string(v) {
        let r = c.rstring(v);
        out.extend_from_slice(&r.b);
        if !ends_with_newline(&r.b, r.enc) { out.push(b'\n'); }
        return Ok(());
    }
    if is_array(v) {
        if !rec_enter(c, v, s::puts)? { out.extend_from_slice(b"[...]\n"); return Ok(()); }
        let n = c.ary_len(v);
        for i in 0..n { let x = c.ary_get(v, i); let r = puts_one(c, x, out); if r.is_err() { rec_leave(c, v, s::puts); return r; } }
        rec_leave(c, v, s::puts);
        return Ok(());
    }
    if let Some(a) = crate::array::check_array(c, v)? { return puts_one(c, a, out); }
    let s = to_s_value(c, v)?;
    out.extend_from_slice(&c.str_bytes(s));
    if !c.str_bytes(s).ends_with(b"\n") { out.push(b'\n'); }
    Ok(())
}

pub fn puts_bytes(c: &mut Ctx, args: &[Value]) -> Result<Vec<u8>, Esc> {
    let mut out = vec![];
    if args.is_empty() { out.push(b'\n'); }
    for &a in args { puts_one(c, a, &mut out)?; }
    Ok(out)
}

/// STDOUT converts what is written to it (set_encoding) or an output method may be overridden:
/// go through IO#write / #puts.
#[inline(always)]
fn stdout_slow() -> bool { crate::builtins::io::STDOUT_CONV.load(std::sync::atomic::Ordering::Relaxed) != 0 }
fn k_puts(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    crate::builtins::thread::blocking_point(c)?;
    let so = *c.vm().globals.borrow().get(&intern(b"$stdout")).unwrap_or(&c.vm().stdout.get());
    if so != c.vm().stdout.get() && (so != c.vm().stderr.get() || crate::builtins::io::out_redef()) || so == c.vm().stdout.get() && stdout_slow() {
        return funcall(c, so, s::puts, args, std::ptr::null());
    }
    let out = puts_bytes(c, args)?;
    if so == c.vm().stderr.get() { err_write(&out); } else { out_write(&out); out_check(c)?; }
    Ok(NIL)
}

fn k_print(c: &mut Ctx, _s: Value, args: &[Value], b: Blk) -> RV {
    use crate::builtins::io;
    crate::builtins::thread::blocking_point(c)?;
    let so = *c.vm().globals.borrow().get(&intern(b"$stdout")).unwrap_or(&c.vm().stdout.get());
    // rb_io_print to $stdout: $_, $, and $\, and a #write of $stdout's own
    if so != c.vm().stdout.get() || stdout_slow() || args.is_empty() || io::OUTPUT_SEPS.load(std::sync::atomic::Ordering::Relaxed) {
        if c.data::<io::RIo>(so).is_some() { return io::io_print(c, so, args, b); }
        io::print_dyn(c, so, args)?; return Ok(NIL);
    }
    let mut out = vec![];
    for &a in args { let s = to_s_value(c, a)?; out.extend_from_slice(&c.str_bytes(s)); }
    out_write(&out); out_check(c)?;
    Ok(NIL)
}

fn k_p(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    let so = *c.vm().globals.borrow().get(&intern(b"$stdout")).unwrap_or(&c.vm().stdout.get());
    if so != c.vm().stdout.get() || stdout_slow() { return p_dyn(c, args); }
    let mut out = vec![];
    for &a in args { let s = inspect_value(c, a)?; out.extend_from_slice(&c.str_bytes(s)); out.push(b'\n'); }
    out_write(&out); out_check(c)?;
    Ok(match args.len() { 0 => NIL, 1 => args[0], _ => c.ary_new_from(args) })
}

/// rb_p_write for each object, then rb_p_result's flush of a File $stdout.
#[cold]
fn p_dyn(c: &mut Ctx, args: &[Value]) -> RV {
    use crate::builtins::io;
    let so = *c.vm().globals.borrow().get(&intern(b"$stdout")).unwrap_or(&c.vm().stdout.get());
    let is_io = c.data::<io::RIo>(so).is_some();
    for &a in args {
        let s = inspect_value(c, a)?;
        let rs = c.str_new_enc(b"\n", ENC_ASCII);
        if is_io && !io::dyn_write(c, so) { funcall(c, so, intern(b"write"), &[s, rs], std::ptr::null())?; } else { io::writev_dyn(c, so, &[s, rs])?; }
    }
    if is_io { funcall(c, so, intern(b"flush"), &[], std::ptr::null())?; }
    Ok(match args.len() { 0 => NIL, 1 => args[0], _ => c.ary_new_from(args) })
}

fn k_putc(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    check_argc(c, args, 1, 1)?;
    let v = args[0];
    let so = *c.vm().globals.borrow().get(&intern(b"$stdout")).unwrap_or(&c.vm().stdout.get());
    // rb_f_putc: $stdout.putc unless writing to STDOUT as is; a String gives its first character
    if so != c.vm().stdout.get() || stdout_slow() || is_string(v) && c.str_bytes(v).first().is_some_and(|&b| b >= 0x80) {
        return funcall(c, so, intern(b"putc"), args, std::ptr::null());
    }
    let b = if is_string(v) { vec![*c.str_bytes(v).first().unwrap_or(&0)] } else { vec![(crate::builtins::numeric_methods::num_to_long(c, v)? & 0xff) as u8] };
    stdout_write(c, &b)?;
    Ok(v)
}

fn k_printf(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    if args.is_empty() { return Ok(NIL); }
    // printf(io, fmt, ...): a non-String first argument is the destination (io.write)
    if !is_string(args[0]) {
        if args.len() < 2 { return arg_err(c, "too few arguments"); }
        let s = crate::builtins::sprintf::format(c, args[1], &args[2..])?;
        funcall(c, args[0], intern(b"write"), &[s], std::ptr::null())?;
        return Ok(NIL);
    }
    let s = crate::builtins::sprintf::format(c, args[0], &args[1..])?;
    let b = c.str_bytes(s).to_vec();
    stdout_write(c, &b)?;
    Ok(NIL)
}

fn k_sprintf(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    if args.is_empty() { return argc_err(c, 0, 1, -1); }
    crate::builtins::sprintf::format(c, args[0], &args[1..])
}

/// Kernel#warn (warning.rb + rb_warn_m).
fn k_warn(c: &mut Ctx, slf: Value, args: &[Value], _b: Blk) -> RV {
    let (args, kw) = match kw_arg(c, args) { Some(h) => (&args[..args.len() - 1], Some(h)), None => (args, None) };
    let (mut uplevel, mut cat) = (NIL, NIL);
    if let Some(h) = kw {
        for (k, v) in c.hash_pairs(h) {
            if k == sym_value("uplevel") { uplevel = v; }
            else if k == sym_value("category") { cat = v; }
            else { let ks = inspect_str(c, k)?; return arg_err(c, format!("unknown keyword: {}", ks)); }
        }
    }
    if cat != NIL {
        cat = if cat.is_sym() { cat } else if is_string(cat) { c.str_to_sym(cat) } else if cat.is_heap() && respond_to(c, cat, intern(b"to_sym"), true) {
            let r = funcall(c, cat, intern(b"to_sym"), &[], std::ptr::null())?;
            if !r.is_sym() { let k = class_name_for_err(c, cat); return type_err(c, format!("can't convert {} to Symbol ({}#to_sym gives {})", k, k, class_name_for_err(c, r))); }
            r
        } else { return type_err(c, format!("no implicit conversion of {} into Symbol", class_name_for_err(c, cat))); };
        let i = crate::builtins::misc::warning_category(c, cat)?;
        if !crate::builtins::misc::warning_category_enabled(i) { return Ok(NIL); }
    }
    if c.vm().verbose.get() == NIL || args.is_empty() { return Ok(NIL); }
    let mut loc = None;
    if uplevel != NIL {
        let lev = crate::builtins::numeric_methods::num_to_long(c, uplevel)?;
        if lev < 0 { return arg_err(c, format!("negative level ({})", lev)); }
        loc = Some(backtrace_locs_n(c, 1 + lev as usize, Some(1)).and_then(|v| v.into_iter().next()));
    }
    let single = args.len() == 1 && uplevel == NIL && is_string(args[0]) && c.str_bytes(args[0]).last() == Some(&b'\n');
    let out = if single { c.str_bytes(args[0]).to_vec() } else {
        let mut pre = match &loc { None => vec![], Some(None) => b"warning: ".to_vec(), Some(Some(l)) => format!("{}:{}: warning: ", l.file, l.line).into_bytes() };
        pre.extend(puts_bytes(c, args)?);
        pre
    };
    if slf == c.vm().m_warning.get() { return stderr_write(c, &out).map(|_| NIL); }
    warning_write_cat(c, &out, cat)?;
    Ok(NIL)
}

// ---- raise ------------------------------------------------------------------------------

/// rb_make_exception: the arguments are positional (a keyword Hash goes to #exception as is).
pub fn make_exception_from_args(c: &mut Ctx, args: &[Value]) -> RV {
    if args.len() > 3 { return arg_err(c, format!("wrong number of arguments (given {}, expected 0..3)", args.len())); }
    let e = match args.len() {
        0 => {
            if c.errinfo != NIL { c.errinfo }
            else { make_exception(c, c.vm().c_runtime_error.get(), "unhandled exception") }
        }
        1 if is_string(args[0]) => {
            let e = new_object(c, c.vm().c_runtime_error.get());
            ivar_set_raw(c, e, s::mesg, args[0]);
            e
        }
        _ => {
            let k = args[0];
            if !respond_to(c, k, s::exception, true) { return type_err(c, "exception class/object expected"); }
            let e = funcall(c, k, s::exception, &args[1..args.len().min(2)], std::ptr::null())?;
            if !is_exception(c, e) { return type_err(c, "exception object expected"); }
            // rb_check_backtrace: nil leaves the backtrace to raise
            if args.len() >= 3 && args[2] != NIL {
                funcall(c, e, intern(b"set_backtrace"), &args[2..3], std::ptr::null())?;
            }
            e
        }
    };
    Ok(e)
}

/// extract_raise_opts: `cause:` taken out of a trailing keyword Hash (UNDEF: not given).
fn take_cause(c: &mut Ctx, args: &[Value]) -> Result<(Vec<Value>, Value), Esc> {
    let mut a = args.to_vec();
    let mut cause = UNDEF;
    if let Some(h) = kw_arg(c, args) {
        let k = crate::vm::sym_value("cause");
        if let Some(v) = hash_get_opt(c, h, k)? {
            cause = v;
            a.pop();
            if c.hash_len(h) > 1 { let o = c.hash_dup(h); hash_delete(c, o, k)?; a.push(o); }
        }
    }
    Ok((a, cause))
}
/// setup_exception's cause: an explicit one (nil keeps what the exception has), else $! unless it has one;
/// a cause that leads back to the exception is an error.
fn setup_cause(c: &mut Ctx, e: Value, cause: Value) -> Result<(), Esc> {
    let cause = if cause == UNDEF {
        if ivar_defined(c, e, s::cause) { return Ok(()); }
        c.errinfo
    } else {
        if cause != NIL && !is_exception(c, cause) { return type_err(c, "exception object expected"); }
        if cause == NIL { if !ivar_defined(c, e, s::cause) { ivar_set_raw(c, e, s::cause, NIL); } return Ok(()); }
        cause
    };
    if cause == NIL || cause == e { return Ok(()); }
    let mut x = cause;
    while x != NIL && is_exception(c, x) {
        if x == e { return arg_err(c, "circular causes"); }
        x = ivar_get(c, x, s::cause);
    }
    ivar_set_raw(c, e, s::cause, cause);
    if !ivar_defined(c, cause, s::cause) { ivar_set_raw(c, cause, s::cause, NIL); }
    Ok(())
}

/// rb_exception_setup: the exception of a cross-context raise (Thread#raise, Fiber#raise); `cause:`
/// is taken out (other keywords stay as a Hash argument) and the cause is the caller's `$!` unless given.
pub fn exception_setup(c: &mut Ctx, args: &[Value]) -> RV {
    let mut a = args.to_vec();
    let mut cause = UNDEF;
    // the copied options Hash is held here (a Value only in `a` is no root)
    let mut opts = NIL;
    if let Some(h) = kw_arg(c, args) {
        a.pop();
        let k = crate::vm::sym_value("cause");
        if let Some(v) = hash_get_opt(c, h, k)? {
            cause = v;
            if c.hash_len(h) > 1 { opts = c.hash_dup(h); hash_delete(c, opts, k)?; a.push(opts); }
        } else if c.hash_len(h) > 0 { opts = c.hash_dup(h); a.push(opts); }
    }
    if a.is_empty() && cause != UNDEF { return arg_err(c, "only cause is given with no arguments"); }
    let mut e = if a.is_empty() { let k = c.vm().c_runtime_error.get(); make_exception(c, k, "") } else { make_exception_from_args(c, &a)? };
    let resolved = if cause == UNDEF {
        if c.errinfo == e { NIL } else { c.errinfo }
    } else if cause == NIL { NIL } else {
        if !is_exception(c, cause) { return type_err(c, "exception object expected"); }
        if cause == e { NIL } else {
            let mut x = cause;
            while x != NIL && is_exception(c, x) {
                if x == e { return arg_err(c, "circular causes"); }
                x = ivar_get(c, x, s::cause);
            }
            cause
        }
    };
    if crate::heap::is_frozen(e) { e = crate::builtins::object::obj_dup(c, e, false, None)?; }
    ivar_set_raw(c, e, s::cause, resolved);
    std::hint::black_box(opts);
    Ok(e)
}

fn k_raise(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    let (mut a, cause) = take_cause(c, args)?;
    if a.is_empty() {
        if cause != UNDEF { return arg_err(c, "only cause is given with no arguments"); }
        if c.errinfo != NIL { a.push(c.errinfo); }
    }
    let e = if a.is_empty() { let k = c.vm().c_runtime_error.get(); make_exception(c, k, "") } else { make_exception_from_args(c, &a)? };
    setup_cause(c, e, cause)?;
    // the backtrace starts at the caller of raise
    if ivar_get(c, e, s::bt) == NIL {
        let bt = make_backtrace_lazy(c, 1);
        ivar_set_raw(c, e, s::bt, bt);
    }
    Err(raise_exc(c, e))
}

// ---- control ------------------------------------------------------------------------------

fn k_loop(c: &mut Ctx, slf: Value, _args: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, s::loop_, &[], NIL); }
    loop {
        // a check point per step, as the back edge of CRuby's `while true; yield; end`
        if crate::vm::ints_pending() { crate::builtins::thread::poll(c)?; }
        match yield_block(c, blk, &[]) {
            Ok(_) => {}
            Err(e) => {
                if c.esc == EscKind::Raise && is_kind_of(c, c.esc_val, c.vm().c_stop_iteration.get()) {
                    let ex = c.esc_val;
                    c.esc = EscKind::None;
                    return Ok(ivar_get(c, ex, intern(b"result")));
                }
                return Err(e);
            }
        }
    }
}

// (Ruby thread, tag): a throw only reaches a catch of its own thread
thread_local! { static CATCH_TAGS: std::cell::RefCell<Vec<(Value, Value)>> = const { std::cell::RefCell::new(vec![]) }; }

fn k_catch(c: &mut Ctx, _s: Value, args: &[Value], blk: Blk) -> RV {
    let tag = if args.is_empty() { new_object(c, c.vm().c_object.get()) } else { args[0] };
    let th = crate::builtins::thread::current_thread(c);
    CATCH_TAGS.with(|t| t.borrow_mut().push((th, tag)));
    let r = yield_block(c, blk, &[tag]);
    CATCH_TAGS.with(|t| { let mut t = t.borrow_mut(); if let Some(i) = t.iter().rposition(|&x| x == (th, tag)) { t.remove(i); } });
    match r {
        Err(e) => {
            if c.esc == EscKind::Throw && (c.esc_tag_v == tag || (is_string(tag) && is_string(c.esc_tag_v) && false)) {
                c.esc = EscKind::None;
                return Ok(c.esc_val);
            }
            Err(e)
        }
        ok => ok,
    }
}

fn k_throw(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    check_argc(c, args, 1, 2)?;
    let tag = args[0];
    let val = if args.len() > 1 { args[1] } else { NIL };
    let th = crate::builtins::thread::current_thread(c);
    let found = CATCH_TAGS.with(|t| t.borrow().iter().any(|&x| x == (th, tag)));
    if !found {
        let ins = inspect_str(c, tag)?;
        let e = make_exception(c, c.vm().c_uncaught_throw_error.get(), &format!("uncaught throw {}", ins));
        ivar_set_raw(c, e, intern(b"tag"), tag);
        ivar_set_raw(c, e, intern(b"value"), val);
        return Err(raise_exc(c, e));
    }
    c.esc = EscKind::Throw;
    c.esc_val = val;
    c.esc_tag_v = tag;
    Err(Esc)
}

fn k_block_given(c: &mut Ctx, _s: Value, _args: &[Value], _b: Blk) -> RV {
    // the cfunc frame's caller
    Ok(Value::bool(c.caller_frame().is_some_and(|p| crate::rt::frame_block_given(c, p))))
}

/// Kernel#lambda called as a method (send, a block-pass): a literal block becomes a lambda; a Proc must be one.
fn k_lambda(c: &mut Ctx, _s: Value, _args: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
    let literal = c.block_copy(blk).is_some_and(|b| b.proc_val == UNDEF && matches!(b.kind, BlockKind::Iseq(_)));
    let p = block_to_proc(c, blk)?;
    if literal { c.proc_data_mut(p).is_lambda = true; }
    else if !c.proc_data(p).is_lambda { return arg_err(c, "the lambda method requires a literal block"); }
    Ok(p)
}

fn k_proc(c: &mut Ctx, _s: Value, _args: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
    block_to_proc(c, blk)
}

/// The method of the nearest Ruby frame below the running builtin (`send :__method__` skips send).
fn ruby_caller_me(c: &Ctx) -> *const MethodEntry {
    let mut f = c.caller_frame();
    while let Some(x) = f { if x.kind != FK_CFUNC { return x.me; } f = c.prev_frame(x); }
    std::ptr::null()
}

fn k_method_name(c: &mut Ctx, _s: Value, _args: &[Value], _b: Blk) -> RV {
    let me = ruby_caller_me(c);
    Ok(c.me_ref(me).map_or(NIL, |m| Value::sym(m.name)))
}

fn k_callee_name(c: &mut Ctx, _s: Value, _args: &[Value], _b: Blk) -> RV {
    let me = ruby_caller_me(c);
    if me.is_null() { return Ok(NIL); }
    Ok(Value::sym(crate::builtins::object::callee_name(c, me)))
}

fn k_caller(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    let (start, len, drop) = crate::builtins::exception::caller_range(c, args)?;
    let Some(lines) = backtrace_lines_n(c, 1 + start, len) else { return Ok(NIL) };
    let lines = crate::builtins::exception::drop_last(lines, drop);
    Ok(c.ary_of_strs(lines.into_iter().map(|l| l.into_bytes()).collect(), ENC_UTF8))
}

pub fn k_exit(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    let st = match args.first() { None => 0, Some(&v) if v == TRUE => 0, Some(&v) if v == FALSE => 1, Some(&v) => crate::builtins::numeric_methods::num_to_long(c, v)? };
    let e = make_exception(c, c.vm().c_system_exit.get(), "exit");
    ivar_set_raw(c, e, s::status, Value::fix(st));
    Err(raise_exc(c, e))
}

fn k_exit_bang(_c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    let st = match args.first() { None => 0, Some(&v) if v == TRUE => 0, Some(&v) if v == FALSE => 1, Some(&v) => v.as_fix() };
    out_flush();
    std::process::exit(st as i32);
}

pub fn k_abort(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    let msg = if let Some(&m) = args.first() {
        let m = expect_str(c, m)?;
        // rb_f_abort: $stderr.puts(msg)
        let se = *c.vm().globals.borrow().get(&intern(b"$stderr")).unwrap_or(&c.vm().stderr.get());
        if se == c.vm().stderr.get() { let mut b = c.str_bytes(m).to_vec(); b.push(b'\n'); err_write(&b); } else { funcall(c, se, intern(b"puts"), &[m], std::ptr::null())?; }
        m
    } else { c.str_new(b"exit") };
    let e = new_object(c, c.vm().c_system_exit.get());
    ivar_set_raw(c, e, s::mesg, msg);
    ivar_set_raw(c, e, s::status, Value::fix(1));
    let _ = c;
    Err(raise_exc(c, e))
}

fn k_at_exit(c: &mut Ctx, _s: Value, _args: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return arg_err(c, "called without a block"); }
    // END { }: registered the first time only
    if let Some(is) = c.block_copy(blk).and_then(|b| match b.kind { BlockKind::Iseq(is) => c.iseq_ref(is), _ => None }) {
        match is.end_once.get() { 2 => return Ok(NIL), 1 => is.end_once.set(2), _ => {} }
    }
    let p = block_to_proc(c, blk)?;
    c.ary_push(c.vm().at_exit.get(), p);
    Ok(p)
}

/// ARGF (a lazily loaded prelude library).
pub fn argf(c: &mut Ctx) -> RV { const_lookup(c, c.vm().top_cref.get(), intern(b"ARGF")) }
fn argf_plain(c: &Ctx) -> bool { c.ary_len(c.vm().argv.get()) == 0 && const_get_at(c, c.vm().c_object.get(), intern(b"ARGF")).is_none() }
fn k_gets(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    out_flush();
    // no file arguments: plain stdin without loading ARGF
    let src = if argf_plain(c) { *c.vm().globals.borrow().get(&intern(b"$stdin")).unwrap_or(&c.vm().stdin.get()) } else { argf(c)? };
    let v = funcall_kw(c, src, intern(b"gets"), args, std::ptr::null(), kw_given(c))?;
    crate::builtins::svar::set_last_line(c, v);
    Ok(v)
}
/// rb_f_readline: ARGF.readline (or $stdin's, without file arguments)
fn k_readline(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    out_flush();
    let src = if argf_plain(c) { *c.vm().globals.borrow().get(&intern(b"$stdin")).unwrap_or(&c.vm().stdin.get()) } else { argf(c)? };
    let v = funcall_kw(c, src, intern(b"readline"), args, std::ptr::null(), kw_given(c))?;
    crate::builtins::svar::set_last_line(c, v);
    Ok(v)
}
fn k_readlines(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    out_flush();
    let src = if argf_plain(c) { *c.vm().globals.borrow().get(&intern(b"$stdin")).unwrap_or(&c.vm().stdin.get()) } else { argf(c)? };
    funcall_kw(c, src, intern(b"readlines"), args, std::ptr::null(), kw_given(c))
}

/// Print a warning line unless $VERBOSE is nil.
pub fn warn_str(c: &mut Ctx, msg: &str) { if c.vm().verbose.get() != NIL { err_write(msg.as_bytes()); } }

fn k_rand(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    crate::builtins::random::rand_value(c, args.first().copied().unwrap_or(NIL))
}
fn k_srand(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    let old = c.vm().rand_state.get();
    let seed = match args.first() { Some(&v) => crate::builtins::numeric_methods::num_to_long(c, v)? as u64, None => 0x2545F4914F6CDD1D };
    crate::builtins::random::seed(c, seed);
    Ok(c.int_from_i128(old as i128))
}

// ---- conversion functions ----------------------------------------------------------------

use crate::builtins::rational::kw_exception;

fn k_integer(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    let (n, exc) = kw_exception(c, args)?;
    let args = &args[..n];
    check_argc(c, args, 1, 2)?;
    let v = args[0];
    let base0 = if args.len() > 1 && args[1] != NIL { crate::builtins::numeric_methods::num_to_long(c, args[1])? } else { 0 };
    let r: RV = (|| {
        if is_string(v) {
            let b = c.str_bytes(v).to_vec();
            if b.contains(&0) { return arg_err(c, "string contains null byte"); }
            // rb_int_parse_cstr: base <= 0 detects a prefix ("0..." included); below -1 the default is -base
            let base = if base0 < -1 {
                let t = b.trim_ascii_start();
                let t = t.strip_prefix(b"+").or_else(|| t.strip_prefix(b"-")).unwrap_or(t);
                if t.len() > 1 && t[0] == b'0' { 0 } else { -base0 }
            } else if base0 <= 0 { 0 } else { base0 };
            if base == 1 || base > 36 { return arg_err(c, format!("invalid radix {}", base)); }
            let base = base as u32;
            match str_to_inum(c, &b, base, true) {
                Some(x) => Ok(x),
                None => { let ins = inspect_str(c, v)?; arg_err(c, format!("invalid value for Integer(): {}", ins)) }
            }
        } else if args.len() > 1 && args[1] != NIL {
            arg_err(c, "base specified for non string value")
        } else if v.is_fixnum() || is_bignum(v) { Ok(v) }
        else if is_float(v) {
            let f = float_val(v);
            if f.is_nan() || f.is_infinite() { return err(c, c.vm().c_float_domain_error.get(), crate::numeric::float_to_s(f)); }
            Ok(crate::builtins::numeric_methods::f64_to_int(c, f))
        } else if v == NIL { type_err(c, "can't convert nil into Integer") }
        else {
            // rb_convert_to_integer: to_int (its errors ignored), then to_str, then to_i
            if respond_to(c, v, s::to_int, true) {
                match funcall(c, v, s::to_int, &[], std::ptr::null()) {
                    Ok(r) if is_integer(r) => return Ok(r),
                    Err(_) if c.esc == EscKind::Raise => { c.esc = EscKind::None; c.esc_val = NIL; }
                    Err(e) => return Err(e),
                    _ => {}
                }
            }
            if v.is_heap() && respond_to(c, v, s::to_str, true) { let sv = expect_str(c, v)?; return k_integer(c, _s, &[sv], _b); }
            let cn = class_path(c, real_class_of(c, v));
            if !respond_to(c, v, s::to_i, true) { return type_err(c, format!("can't convert {} into Integer", cn)); }
            let r = funcall(c, v, s::to_i, &[], std::ptr::null())?;
            if is_integer(r) { return Ok(r); }
            let rn = class_path(c, real_class_of(c, r));
            type_err(c, format!("can't convert {} to Integer ({}#to_i gives {})", cn, cn, rn))
        }
    })();
    match r { Err(e) if !exc && c.esc == EscKind::Raise => { let _ = e; c.esc = EscKind::None; Ok(NIL) } r => r }
}

/// Digits of `radix` with single underscores between them, from b[i]; (end, digits seen).
fn scan_digits(b: &[u8], mut i: usize, hex: bool) -> (usize, usize) {
    let is_d = |x: u8| if hex { x.is_ascii_hexdigit() } else { x.is_ascii_digit() };
    let mut n = 0;
    while i < b.len() {
        if is_d(b[i]) { n += 1; i += 1; }
        else if b[i] == b'_' && n > 0 && is_d(b[i - 1]) && i + 1 < b.len() && is_d(b[i + 1]) { i += 1; }
        else { break; }
    }
    (i, n)
}
/// Kernel#Float's strict string form (rb_cstr_to_dbl with badcheck, 4.0): decimal with
/// optional "." / exponent ("1.", ".5" and "1.e5" included) or hexadecimal with "p" exponent.
pub fn parse_float_strict(_c: &mut Ctx, b: &[u8]) -> Option<f64> {
    let ws = |x: &u8| x.is_ascii_whitespace();
    let s = b.iter().position(|x| !ws(x))?;
    let e = b.iter().rposition(|x| !ws(x))? + 1;
    let t = &b[s..e];
    let mut i = 0;
    let neg = t[0] == b'-';
    if t[0] == b'+' || t[0] == b'-' { i += 1; }
    let hex = t.len() > i + 1 && t[i] == b'0' && (t[i + 1] == b'x' || t[i + 1] == b'X');
    if hex { i += 2; }
    let (j, int_n) = scan_digits(t, i, hex);
    let int_part = &t[i..j];
    i = j;
    let mut frac_part: &[u8] = b"";
    if i < t.len() && t[i] == b'.' {
        let (j, _) = scan_digits(t, i + 1, hex);
        frac_part = &t[i + 1..j];
        i = j;
    }
    if int_n == 0 && frac_part.is_empty() { return None; }
    let mut exp: i64 = 0;
    let exp_ch = if hex { [b'p', b'P'] } else { [b'e', b'E'] };
    let mut exp_txt: &[u8] = b"";
    if i < t.len() && exp_ch.contains(&t[i]) {
        let es = i + 1;
        let mut k = es;
        if k < t.len() && (t[k] == b'+' || t[k] == b'-') { k += 1; }
        let (j, n) = scan_digits(t, k, false);
        if n == 0 { return None; }
        exp_txt = &t[es..j];
        let digits: String = t[k..j].iter().filter(|&&x| x != b'_').map(|&x| x as char).collect();
        exp = digits.parse::<i64>().unwrap_or(i64::MAX / 4).min(1 << 40);
        if t[es] == b'-' { exp = -exp; }
        i = j;
    }
    if i != t.len() { return None; }
    let clean = |p: &[u8]| -> String { p.iter().filter(|&&x| x != b'_').map(|&x| x as char).collect() };
    let v = if hex {
        let mut m = 0.0f64;
        for ch in clean(int_part).chars() { m = m * 16.0 + ch.to_digit(16).unwrap() as f64; }
        let mut scale = 1.0 / 16.0;
        for ch in clean(frac_part).chars() { m += ch.to_digit(16).unwrap() as f64 * scale; scale /= 16.0; }
        if m == 0.0 { 0.0 } else { m * 2f64.powi(exp.clamp(-2200, 2200) as i32) }
    } else {
        let ip = clean(int_part);
        let fp = clean(frac_part);
        let et = clean(exp_txt);
        let txt = format!("{}.{}{}{}", if ip.is_empty() { "0" } else { &ip }, if fp.is_empty() { "0" } else { &fp }, if et.is_empty() { "" } else { "e" }, et);
        txt.parse::<f64>().ok()?
    };
    Some(if neg { -v } else { v })
}

fn k_float(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    let (n, exc) = kw_exception(c, args)?;
    let args = &args[..n];
    check_argc(c, args, 1, 1)?;
    let v = args[0];
    let r: RV = (|| {
        if is_float(v) { return Ok(v); }
        if v.is_fixnum() { return Ok(c.float_new(v.as_fix() as f64)); }
        if is_bignum(v) { return Ok(c.float_new(c.big(v).to_f64())); }
        if is_string(v) {
            let b = c.str_bytes(v).to_vec();
            if b.contains(&0) { return arg_err(c, "string for Float contains null byte"); }
            match parse_float_strict(c, &b) { Some(f) => return Ok(c.float_new(f)), None => { let ins = inspect_str(c, v)?; return arg_err(c, format!("invalid value for Float(): {}", ins)); } }
        }
        if v == NIL { return type_err(c, "can't convert nil into Float"); }
        if respond_to(c, v, s::to_f, true) {
            let r = funcall(c, v, s::to_f, &[], std::ptr::null())?;
            if is_float(r) { return Ok(r); }
            let cn = class_name_for_err(c, v);
            return type_err(c, format!("can't convert {} to Float ({}#to_f gives {})", cn, cn, class_name_for_err(c, r)));
        }
        type_err(c, format!("can't convert {} into Float", class_name_for_err(c, v)))
    })();
    match r { Err(e) if !exc && c.esc == EscKind::Raise => { let _ = e; c.esc = EscKind::None; Ok(NIL) } r => r }
}

fn k_string(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    check_argc(c, args, 1, 1)?;
    let v = args[0];
    if is_string(v) { return Ok(v); }
    if crate::rt::conv_target(v) && respond_to(c, v, s::to_str, true) { return funcall(c, v, s::to_str, &[], std::ptr::null()); }
    // convert_type_with_id: rb_check_funcall (a respond_to? / method_missing of its own count)
    if let Some(r) = crate::string::check_funcall(c, v, s::to_s)? {
        if !is_string(r) { return type_err(c, format!("can't convert {} to String ({}#to_s gives {})", class_name_for_err(c, v), class_name_for_err(c, v), class_name_for_err(c, r))); }
        return Ok(r);
    }
    type_err(c, format!("can't convert {} into String", class_name_for_err(c, v)))
}

pub fn k_array(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    check_argc(c, args, 1, 1)?;
    let v = args[0];
    if is_array(v) { return Ok(v); }
    if v == NIL && !crate::rt::redef(crate::rt::R_IMM_CONV) { return Ok(c.ary_new()); }
    if let Some(a) = check_array(c, v)? { return Ok(a); }
    if respond_to(c, v, s::to_a, true) {
        let r = funcall(c, v, s::to_a, &[], std::ptr::null())?;
        if is_array(r) { return Ok(r); }
        if r != NIL { return type_err(c, format!("can't convert {} to Array ({}#to_a gives {})", class_name_for_err(c, v), class_name_for_err(c, v), class_name_for_err(c, r))); }
    }
    Ok(c.ary_new_from(&[v]))
}

fn k_hash_conv(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    check_argc(c, args, 1, 1)?;
    let v = args[0];
    if is_hash(v) { return Ok(v); }
    if v == NIL || (is_array(v) && c.ary_len(v) == 0) { return Ok(c.hash_new()); }
    if let Some(h) = check_hash(c, v)? { return Ok(h); }
    type_err(c, format!("can't convert {} into Hash", class_name_for_err(c, v)))
}

// ---- object identity / hash ----------------------------------------------------------------

pub fn obj_hash(_c: &mut Ctx, slf: Value, _args: &[Value], _b: Blk) -> RV {
    let h = crate::astrust::hash_u64(slf.raw());
    Ok(Value::fix((h >> 2) as i64))
}

// ---- global variables ---------------------------------------------------------------------

fn resolve_gvar(c: &Ctx, name: Sym) -> Sym { *c.vm().global_aliases.borrow().get(&name).unwrap_or(&name) }

/// Built-in synonyms ($> is $stdout, ...): one variable under several names, as in CRuby.
pub const GVAR_SYNONYMS: &[(&str, &str)] = &[("$>", "$stdout"), ("$-0", "$/"), ("$-F", "$;"), ("$-d", "$DEBUG"),
    ("$-v", "$VERBOSE"), ("$-w", "$VERBOSE"), ("$PROGRAM_NAME", "$0"), ("$LOAD_PATH", "$:"), ("$-I", "$:"), ("$LOADED_FEATURES", "$\"")];

/// `alias $new $old`: an alias of an alias names the same variable.
pub fn gvar_alias(c: &mut Ctx, new: Sym, old: Sym) {
    let old = resolve_gvar(c, old);
    c.vm().global_aliases.borrow_mut().insert(new, old);
}

/// "file:line: warning: msg" through Warning.warn, unless $VERBOSE is nil (or the category is off).
pub fn gvar_warn(c: &mut Ctx, msg: &str, deprecated: bool) -> Result<(), Esc> {
    if c.vm().verbose.get() == NIL || (deprecated && !crate::builtins::misc::warning_category_enabled(0)) { return Ok(()); }
    let loc = c.cur_frame().map(|f| format!("{}:{}: ", frame_file(f), f.line())).unwrap_or_default();
    let m = format!("{}warning: {}\n", loc, msg);
    if deprecated { warning_write_cat(c, m.as_bytes(), sym_value("deprecated")) } else { warning_write(c, m.as_bytes()) }
}

pub fn warn_deprecated(c: &mut Ctx, msg: &str) -> Result<(), Esc> { gvar_warn(c, msg, true) }
/// rb_category_warn for Warning[`name`] (bit `cat` of misc::WARN_CATS).
pub fn category_warn(c: &mut Ctx, msg: &str, cat: usize, name: &str) -> Result<(), Esc> {
    if c.vm().verbose.get() == NIL || !crate::builtins::misc::warning_category_enabled(cat) { return Ok(()); }
    let loc = c.cur_frame().map(|f| format!("{}:{}: ", frame_file(f), f.line())).unwrap_or_default();
    let m = format!("{}warning: {}\n", loc, msg);
    warning_write_cat(c, m.as_bytes(), sym_value(name))
}

pub fn gvar_get(c: &mut Ctx, name: Sym) -> RV {
    let name = resolve_gvar(c, name);
    let b = sym_bytes(name);
    match b {
        b"$!" => return Ok(c.errinfo),
        b"$@" => { return Ok(if c.errinfo == NIL { NIL } else { exc_bt(c, c.errinfo) }); }
        b"$~" => return Ok(crate::builtins::svar::last_match(c)),
        b"$_" => return Ok(crate::builtins::svar::last_line(c)),
        b"$&" => return Ok(crate::regexp::nth_match(c, crate::builtins::svar::last_match(c), 0)),
        b"$`" => { let m = crate::builtins::svar::last_match(c); return Ok(if m == NIL { NIL } else { crate::regexp::match_pre(c, m) }); }
        b"$'" => { let m = crate::builtins::svar::last_match(c); return Ok(if m == NIL { NIL } else { crate::regexp::match_post(c, m) }); }
        b"$+" => { let m = crate::builtins::svar::last_match(c); return Ok(if m == NIL { NIL } else { crate::regexp::match_last_group(c, m) }); }
        b"$0" => return Ok(c.vm().dollar_zero.get()),
        b"$:" => return Ok(c.vm().load_path.get()),
        b"$\"" => return Ok(c.vm().loaded_features.get()),
        b"$$" => return Ok(Value::fix(std::process::id() as i64)),
        b"$*" => return Ok(c.vm().argv.get()),
        b"$<" => return argf(c),
        b"$FILENAME" => { let a = argf(c)?; return funcall(c, a, intern(b"filename"), &[], std::ptr::null()); }
        b"$VERBOSE" => return Ok(c.vm().verbose.get()),
        b"$DEBUG" => return Ok(*c.vm().globals.borrow().get(&name).unwrap_or(&Value::bool(c.vm().debug_flag.get()))),
        b"$?" => return Ok(crate::builtins::process::last_status(c)),
        b"$." => return Ok(*c.vm().globals.borrow().get(&name).unwrap_or(&Value::fix(0))),
        b"$=" => { gvar_warn(c, "variable $= is no longer effective", true)?; return Ok(FALSE); }
        _ => {}
    }
    let v = c.vm().globals.borrow().get(&name).copied();
    match v {
        Some(v) => Ok(v),
        None => {
            if c.vm().verbose.get() == TRUE && !gvar_defined(c, name) {
                gvar_warn(c, &format!("global variable '{}' not initialized", String::from_utf8_lossy(b)), false)?;
            }
            Ok(NIL)
        }
    }
}

/// $/ holds a plain frozen String: a subclass instance, an ivar holder or a mutable one is copied.
fn gvar_frozen_str(c: &mut Ctx, v: Value) -> Value {
    if is_frozen(v) && real_class_of(c, v) == c.vm().c_string.get() && ivar_list(c, v).is_empty() { return v; }
    let n = c.str_copy(v);
    obj_set_flags(n, crate::object::FL_FROZEN);
    n
}

pub fn gvar_set(c: &mut Ctx, shown: Sym, v: Value) -> RV {
    let name = resolve_gvar(c, shown);
    let nb = sym_bytes(name);
    let read_only = matches!(nb, b"$!" | b"$&" | b"$`" | b"$'" | b"$+" | b"$:" | b"$\"" | b"$$" | b"$*" | b"$<" | b"$FILENAME"
        | b"$?" | b"$-a" | b"$-l" | b"$-p" | b"$-W") || (nb.len() > 1 && (b'1'..=b'9').contains(&nb[1]));
    if read_only {
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("{} is a read-only variable", sym_str(shown)), Value::sym(shown), NIL)?;
        return Err(raise_exc(c, e));
    }
    let mut v = v;
    match nb {
        b"$~" => {
            if v != NIL && !(v.is_heap() && is_kind_of(c, v, c.vm().c_match_data.get())) {
                return type_err(c, format!("wrong argument type {} (expected MatchData)", class_name_for_err(c, v)));
            }
            crate::builtins::svar::set_last_match(c, v); return Ok(v);
        }
        b"$_" => { crate::builtins::svar::set_last_line(c, v); return Ok(v); }
        b"$@" => {
            if c.errinfo == NIL { return arg_err(c, "$! not set"); }
            let e = c.errinfo;
            funcall(c, e, intern(b"set_backtrace"), &[v], std::ptr::null())?;
            return Ok(v);
        }
        b"$VERBOSE" => { c.vm().verbose.set(if v == NIL { NIL } else { Value::bool(v.truthy()) }); return Ok(v); }
        b"$DEBUG" => { c.vm().debug_flag.set(v.truthy()); crate::vm::update_raise_hooks(); }
        b"$0" => { crate::builtins::process::set_dollar_zero(c, v)?; return Ok(v); }
        b"$stdout" | b"$stderr" => {
            if !respond_to(c, v, intern(b"write"), true) {
                return type_err(c, format!("{} must have write method, {} given", sym_str(shown), class_path(c, real_class_of(c, v))));
            }
        }
        b"$/" | b"$\\" | b"$," | b"$;" => {
            if nb == b"$/" { RS_SET.store(true, std::sync::atomic::Ordering::Relaxed); }
            if v != NIL {
                let re_ok = nb == b"$;" && crate::regexp::is_regexp(c, v);
                if !is_string(v) && !re_ok {
                    return type_err(c, format!("value of {} must be String{}", sym_str(shown), if nb == b"$;" { " or Regexp" } else { "" }));
                }
                gvar_warn(c, &format!("non-nil '{}' is deprecated", sym_str(shown)), true)?;
                if nb == b"$/" { v = gvar_frozen_str(c, v); }
                if nb == b"$\\" || nb == b"$," { crate::builtins::io::OUTPUT_SEPS.store(true, std::sync::atomic::Ordering::Relaxed); }
            }
        }
        b"$." => { let n = crate::builtins::numeric_methods::num_to_long(c, v)?; v = Value::fix(n as i32 as i64); }
        b"$-i" => { if v != NIL { v = expect_str(c, v)?; } }
        b"$=" => { gvar_warn(c, "variable $= is no longer effective; ignored", true)?; return Ok(v); }
        _ => {}
    }
    c.vm().globals.borrow_mut().insert(name, v);
    if TRACING.load(std::sync::atomic::Ordering::Relaxed) { run_traces(c, name, v)?; }
    Ok(v)
}

// trace_var: { $name => [command, ...] } (oldest first; the newest runs first)
static TRACES: crate::heap::Global = crate::heap::Global::new();
static TRACING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn traces_of(c: &mut Ctx, name: Sym) -> Result<Option<Value>, Esc> {
    let t = TRACES.get(c);
    if t == NIL { return Ok(None); }
    let l = hash_get_opt(c, t, Value::sym(name))?;
    Ok(l.filter(|&l| c.ary_len(l) > 0))
}
#[cold]
fn run_traces(c: &mut Ctx, name: Sym, v: Value) -> Result<(), Esc> {
    // a variable's traces do not run again for an assignment made by one of them (CRuby's block_trace)
    thread_local! { static RUNNING: std::cell::RefCell<Vec<Sym>> = const { std::cell::RefCell::new(vec![]) }; }
    if RUNNING.with(|r| r.borrow().contains(&name)) { return Ok(()); }
    let Some(l) = traces_of(c, name)? else { return Ok(()) };
    let cmds: Vec<Value> = c.ary(l).iter().rev().copied().collect();
    let keep = c.ary_new_from(&cmds);
    RUNNING.with(|r| r.borrow_mut().push(name));
    let mut r = Ok(());
    for cmd in cmds {
        r = if is_string(cmd) { let m = c.vm().main_obj.get(); funcall(c, m, intern(b"eval"), &[cmd], std::ptr::null()).map(drop) }
            else { funcall(c, cmd, intern(b"call"), &[v], std::ptr::null()).map(drop) };
        if r.is_err() { break; }
    }
    RUNNING.with(|r| { let mut r = r.borrow_mut(); if let Some(i) = r.iter().rposition(|&x| x == name) { r.remove(i); } });
    std::hint::black_box(keep);
    r
}
/// Kernel#trace_var(name, cmd = block): cmd (a Proc or a String to eval) runs at each assignment.
fn k_trace_var(c: &mut Ctx, _s: Value, a: &[Value], b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let name = crate::builtins::object::to_sym_arg(c, a[0])?;
    let name = resolve_gvar(c, name);
    let cmd = if a.len() == 2 { a[1] } else if !b.is_null() { block_to_proc(c, b)? } else { return arg_err(c, "tried to create Proc object without a block"); };
    if cmd == NIL { return untrace(c, name, None); }
    if TRACES.get(c) == NIL { let h = c.hash_new(); TRACES.set(c, h); }
    let t = TRACES.get(c);
    let l = match hash_get_opt(c, t, Value::sym(name))? { Some(l) => l, None => { let l = c.ary_new(); hash_set(c, t, Value::sym(name), l)?; l } };
    c.ary_push(l, cmd);
    TRACING.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(NIL)
}
/// Kernel#untrace_var(name, cmd = nil): the commands removed (all of them without cmd).
fn k_untrace_var(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let name = crate::builtins::object::to_sym_arg(c, a[0])?;
    let name = resolve_gvar(c, name);
    if !gvar_defined(c, name) {
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("undefined global variable {}", sym_str(name)), Value::sym(name), NIL)?;
        return Err(raise_exc(c, e));
    }
    untrace(c, name, a.get(1).copied())
}
fn untrace(c: &mut Ctx, name: Sym, cmd: Option<Value>) -> RV {
    let Some(l) = traces_of(c, name)? else { return Ok(if cmd.is_some() { NIL } else { c.ary_new() }) };
    match cmd {
        Some(x) => {
            let Some(i) = c.ary(l).iter().position(|&y| y == x) else { return Ok(NIL) };
            c.ary_mut(l).remove(i);
            Ok(c.ary_new_from(&[x]))
        }
        None => {
            let all: Vec<Value> = c.ary(l).iter().rev().copied().collect();
            c.ary_mut(l).clear();
            Ok(c.ary_from_vec(all))
        }
    }
}

pub fn gvar_defined(c: &Ctx, name: Sym) -> bool {
    let name = resolve_gvar(c, name);
    match sym_bytes(name) {
        // the built-in (virtual) variables are always defined
        b"$!" | b"$@" | b"$~" | b"$_" | b"$." | b"$?" | b"$-a" | b"$-i" | b"$-l" | b"$-p" | b"$-W" | b"$\\"
        | b"$0" | b"$:" | b"$\"" | b"$$" | b"$*" | b"$<" | b"$FILENAME" | b"$stdout" | b"$stderr" | b"$stdin" | b"$VERBOSE" | b"$DEBUG" | b"$," | b"$/" | b"$;" | b"$=" => true,
        _ => c.vm().globals.borrow().contains_key(&name),
    }
}

// ---- require / load ------------------------------------------------------------------------

pub const BUILTIN_FEATURES: &[&str] = &["set", "enumerator", "rational", "complex", "thread", "fiber", "continuation",
    "pp", "prettyprint", "stringio", "rbconfig", "etc", "io/console", "io/wait", "monitor", "timeout", "securerandom",
    "coverage", "ripper", "tempfile", "tmpdir", "english", "English", "bigdecimal", "date", "json", "time",
    "benchmark", "fileutils", "shellwords", "strscan", "digest", "digest/md5", "digest/sha1", "digest/sha2", "zlib", "base64",
    "optparse", "ostruct", "forwardable", "singleton", "observer", "delegate", "socket", "open3", "logger", "uri", "net/http",
    "erb", "cgi", "abbrev", "matrix", "prime", "tsort", "weakref", "find", "pathname", "io/nonblock", "fcntl",
    "nkf", "racc", "racc/parser", "win32ole", "openssl", "mkmf", "rubygems",
    "did_you_mean", "error_highlight", "syntax_suggest", "-test-/memory_status", "random/formatter",
    "date_core", "socket.so", "digest/sha2", "digest/sha1", "digest/md5", "digest/rmd160", "digest/bubblebabble", "net/protocol", "net/https",
    "open-uri", "resolv", "ipaddr", "pty", "syslog", "expect", "io/console", "bigdecimal/util", "date/format",
];

/// File.expand_path without a Ruby call: `~` is $HOME, a relative name is under `base` (or the cwd),
/// and `.` / `..` are folded lexically (symlinks stay).
pub fn expand_path_str(name: &str, base: Option<&str>) -> String {
    let joined = if let Some(rest) = name.strip_prefix('~').filter(|r| r.is_empty() || r.starts_with('/')) {
        format!("{}{}", std::env::var("HOME").unwrap_or_default(), rest)
    } else if name.starts_with('/') { name.to_string() } else {
        let b = match base { Some(b) if b.starts_with('/') => b.to_string(), Some(b) => expand_path_str(b, None),
            None => std::env::current_dir().map(|d| d.to_string_lossy().into_owned()).unwrap_or_else(|_| "/".into()) };
        format!("{}/{}", b, name)
    };
    let mut out: Vec<&str> = vec![];
    for seg in joined.split('/') { match seg { "" | "." => {} ".." => { out.pop(); } s => out.push(s) } }
    format!("/{}", out.join("/"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ext { None, Rb, So }
const SO_EXTS: &[&str] = &[".so", ".o", ".bundle", ".dll", ".dylib"];
/// The feature's extension (rb_feature_p's ext): only .rb and the shared-library ones count.
fn split_ext(f: &str) -> (&str, Ext) {
    let tail = f.rsplit('/').next().unwrap_or(f);
    if let Some(dot) = tail.rfind('.') {
        let (b, e) = f.split_at(f.len() - (tail.len() - dot));
        if e == ".rb" { return (b, Ext::Rb); }
        if SO_EXTS.contains(&e) { return (b, Ext::So); }
    }
    (f, Ext::None)
}
fn is_explicit(name: &str) -> bool { name.starts_with('/') || name.starts_with("./") || name.starts_with("../") || name.starts_with('~') }

/// $LOAD_PATH as CRuby's expanded load path: each entry (to_path) expanded and with symlinks resolved.
fn expanded_load_path(c: &mut Ctx, call: bool) -> Result<std::sync::Arc<Vec<String>>, Esc> {
    let lp = c.vm().load_path.get();
    // rb_get_expanded_load_path's snapshot: the same entries (and cwd, for relative ones) expand the same
    let n = c.ary_len(lp);
    let all_str = (0..n).all(|i| is_string(c.ary_get(lp, i)));
    let rel = all_str && (0..n).any(|i| !c.str_bytes(c.ary_get(lp, i)).starts_with(b"/"));
    let cwd = if rel { std::env::current_dir().ok() } else { None };
    if all_str {
        if let Some((r, w, e)) = LP_CACHE.lock().unwrap().as_ref() {
            if r.len() == n && *w == cwd && (0..n).all(|i| c.str_bytes(c.ary_get(lp, i)) == &r[i][..]) { return Ok(e.clone()); }
        }
    }
    let out = std::sync::Arc::new(expand_load_path(c, call)?);
    if all_str {
        let raw = (0..n).map(|i| c.str_bytes(c.ary_get(lp, i)).to_vec()).collect();
        *LP_CACHE.lock().unwrap() = Some((raw, cwd, out.clone()));
    }
    Ok(out)
}
type LpCache = (Vec<Vec<u8>>, Option<std::path::PathBuf>, std::sync::Arc<Vec<String>>);
static LP_CACHE: std::sync::Mutex<Option<LpCache>> = std::sync::Mutex::new(None);
fn expand_load_path(c: &mut Ctx, call: bool) -> Result<Vec<String>, Esc> {
    let lp = c.vm().load_path.get();
    let mut out = vec![];
    for i in 0..c.ary_len(lp) {
        let d = c.ary_get(lp, i);
        let s = if is_string(d) { String::from_utf8_lossy(c.str_bytes(d)).into_owned() } else if call { feature_name(c, d)? } else { continue };
        let e = expand_path_str(&s, None);
        out.push(std::fs::canonicalize(&e).map(|p| p.to_string_lossy().into_owned()).unwrap_or(e));
    }
    Ok(out)
}

/// rb_feature_p: a feature in $LOADED_FEATURES (or being loaded) that `name` names.  For a name without
/// an extension only a .rb feature is returned; feature_present says whether any other matched.
fn feature_provided(name: &str, lp: &[String], list: &[String]) -> Option<usize> {
    let (_, ne) = split_ext(name);
    let i = feature_match(name, lp, list, |fe| ne != Ext::None || fe == Ext::Rb);
    i
}
fn feature_present(name: &str, lp: &[String], list: &[String]) -> bool { feature_match(name, lp, list, |_| true).is_some() }
fn feature_match(name: &str, lp: &[String], list: &[String], take: impl Fn(Ext) -> bool) -> Option<usize> {
    let (nb, ne) = split_ext(name);
    let expanded = name.starts_with('/');
    list.iter().position(|f| {
        if !take(split_ext(f).1) { return false; }
        let (fb, fe) = split_ext(f);
        if fe == Ext::None { if ne != Ext::None || fb != nb { return false; } return true; }
        if ne != Ext::None && fe != ne { return false; }
        if fb == nb { return true; }
        if expanded || fb.len() <= nb.len() + 1 || !fb.ends_with(nb) { return false; }
        let pre = &fb[..fb.len() - nb.len()];
        pre.ends_with('/') && lp.iter().any(|d| d.as_str() == &pre[..pre.len() - 1])
    })
}
fn loaded_list(c: &Ctx) -> Vec<String> {
    let lf = c.vm().loaded_features.get();
    c.ary(lf).iter().filter(|&&x| is_string(x)).map(|&x| String::from_utf8_lossy(c.str_bytes(x)).into_owned()).collect()
}

/// The file `name` resolves to (rb_find_file): explicit paths as given, others under each load path entry.
fn find_file(name: &str, lp: &[String]) -> Option<String> {
    let (_, ext) = split_ext(name);
    let cands = |base: String| -> Option<String> {
        match ext {
            Ext::Rb => std::path::Path::new(&base).is_file().then_some(base),
            Ext::None => { let r = format!("{}.rb", base); std::path::Path::new(&r).is_file().then_some(r) }
            Ext::So => None,
        }
    };
    if is_explicit(name) { return cands(expand_path_str(name, None)); }
    // the reference ruby's rubylibdir / archdir: its files go through stdlib_file (saruby's own features first)
    let std = crate::builtins::prelude::stdlib_lp_dirs();
    lp.iter().filter(|d| !std.contains(d)).find_map(|d| cands(expand_path_str(name, Some(d))))
}

enum Feature { File(String), Native(String), Missing, SoOnly }

fn locate_feature(name: &str, lp: &[String], relative: bool) -> Feature {
    if let Some(f) = find_file(name, lp) { return Feature::File(f); }
    let (base, ext) = split_ext(name);
    let is_so = name.ends_with(".so");
    // "foo.so" may name a native part of its own, below a pure-Ruby "foo" (psych.so)
    let so_native = is_so && crate::builtins::prelude::has_feature(name);
    let native = ext != Ext::Rb && (crate::builtins::prelude::has_feature(base) || so_native);
    // the reference ruby's pure-Ruby stdlib, unless saruby implements the feature itself
    let std_file = if native || is_so || relative || is_explicit(name) { None } else { crate::builtins::prelude::stdlib_file(name) };
    match std_file {
        Some(f) => Feature::File(std::fs::canonicalize(&f).unwrap_or(f).to_string_lossy().into_owned()),
        None if native || BUILTIN_FEATURES.contains(&base) => Feature::Native(if so_native { name.to_string() } else { base.to_string() }),
        // an explicit path with only a native library beside it: dlopen would fail (LoadError without a path)
        None if ext == Ext::None && is_explicit(name) && SO_EXTS.iter().any(|e| std::path::Path::new(&format!("{}{}", expand_path_str(name, None), e)).is_file()) => Feature::SoOnly,
        None => Feature::Missing,
    }
}

/// rb_get_path for require/load: a String, else to_path (if any) and then to_str.
fn feature_name(c: &mut Ctx, v: Value) -> Result<String, Esc> {
    let s = if is_string(v) { v } else {
        let p = if v.is_heap() && respond_to(c, v, intern(b"to_path"), true) { funcall(c, v, intern(b"to_path"), &[], std::ptr::null())? } else { v };
        expect_str(c, p)?
    };
    Ok(String::from_utf8_lossy(c.str_bytes(s)).into_owned())
}

pub fn require_feature(c: &mut Ctx, path: Value, relative: bool) -> RV {
    let name = feature_name(c, path)?;
    let caller_file = if relative { c.caller_frame().map(|f| frame_file(f).to_string()) } else { None };
    let caller_file = caller_file.map(|f| if f == "-" || f == "-e" { "./x".to_string() } else { f });
    // require_relative in a feature of saruby's own (<internal:a/b>): the feature a/<name>
    if let Some(inner) = caller_file.as_deref().and_then(|f| f.strip_prefix("<internal:")).and_then(|f| f.strip_suffix('>')) {
        let mut rel: Vec<&str> = inner.split('/').collect();
        rel.pop();
        for seg in name.split('/') { match seg { ".." => { rel.pop(); } "." => {} s => rel.push(s) } }
        let rel = rel.join("/");
        if crate::builtins::prelude::has_feature(&rel) { let r = c.str_new(rel.as_bytes()); return require_feature(c, r, false); }
    }
    // require_relative: the path from the caller's directory (File.absolute_path(name, dir))
    let name = match &caller_file {
        Some(cf) => {
            // rb_current_realfilepath: the caller's file with symlinks resolved
            let real = crate::builtins::exception::realpath(cf).unwrap_or_else(|| cf.clone());
            let dir = match real.rfind('/') { Some(0) => "/".to_string(), Some(i) => real[..i].to_string(), None => ".".to_string() };
            expand_path_str(&name, Some(&dir))
        }
        None => name,
    };
    let lp = expanded_load_path(c, true)?;
    let me = crate::builtins::thread::current_thread(c);
    // already provided under this name, or being loaded
    loop {
        if feature_provided(&name, &lp, &loaded_list(c)).is_some() { return Ok(FALSE); }
        let loading: Vec<String> = LOADING.with_borrow(|l| l.iter().map(|e| e.0.clone()).collect());
        let Some(i) = feature_provided(&name, &lp, &loading) else { break };
        if !wait_loading(c, &loading[i], me)? { return Ok(FALSE); }
    }
    // a native feature of saruby's, recorded under its bare name, is provided (rb_feature_p finds the .so before any file)
    let native = crate::builtins::prelude::has_feature(&name) || BUILTIN_FEATURES.contains(&name.as_str());
    if native && !crate::builtins::prelude::has_lib(&name) && split_ext(&name).1 == Ext::None && loaded_list(c).iter().any(|f| *f == name) { return Ok(FALSE); }
    let file = match locate_feature(&name, &lp, caller_file.is_some()) {
        Feature::File(f) => f,
        Feature::Native(base) => {
            if base == "did_you_mean" { if let Some(r) = crate::builtins::did_you_mean::require(c) { return r; } }
            // a .so of that name is there and no .rb exists (pathname.so but pathname.rb loads)
            if !crate::builtins::prelude::has_lib(&base) && feature_present(&name, &lp, &loaded_list(c)) { return Ok(FALSE); }
            // saruby's own parts ("saruby/...") are not features of CRuby's: kept out of $LOADED_FEATURES
            if base.starts_with("saruby/") {
                if INTERNAL_LOADED.with_borrow(|l| l.contains(&base)) { return Ok(FALSE); }
                INTERNAL_LOADED.with_borrow_mut(|l| l.push(base.clone()));
                crate::builtins::prelude::load_feature(c, &base)?;
                return Ok(TRUE);
            }
            let lf = c.vm().loaded_features.get();
            // one with Ruby code is recorded as its .rb (as CRuby's lib/<name>.rb)
            let feat = if crate::builtins::prelude::has_lib(&base) { format!("{}.rb", base) } else { base.clone() };
            if c.ary(lf).iter().any(|&x| is_string(x) && (c.str_bytes(x) == base.as_bytes() || c.str_bytes(x) == feat.as_bytes())) { return Ok(FALSE); }
            let key = c.str_new(feat.as_bytes());
            c.ary_push(lf, key);
            crate::builtins::prelude::load_feature(c, &base)?;
            return Ok(TRUE);
        }
        Feature::SoOnly => {
            let e = make_exception(c, c.vm().c_load_error.get(), &format!("{}.so: cannot load a native extension", expand_path_str(&name, None)));
            return Err(raise_exc(c, e));
        }
        // a feature of that name without .rb (a .so, or no extension) is already there
        Feature::Missing if feature_present(&name, &lp, &loaded_list(c)) => return Ok(FALSE),
        Feature::Missing => {
            let e = make_exception(c, c.vm().c_load_error.get(), &format!("cannot load such file -- {}", name));
            { let p = c.str_new(name.as_bytes()); ivar_set_raw(c, e, intern(b"path"), p); }
            return Err(raise_exc(c, e));
        }
    };
    let key = file;
    let real = std::fs::canonicalize(&key).map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| key.clone());
    loop {
        let list = loaded_list(c);
        if list.iter().any(|f| *f == key) { return Ok(FALSE); }
        // the same file under another path (CRuby's loaded_features_realpaths)
        if REALPATHS.with_borrow(|r| r.iter().any(|(f, rp)| *rp == real && list.contains(f))) { return Ok(FALSE); }
        if loading_thread(&key).is_none() { break; }
        if !wait_loading(c, &key, me)? { return Ok(FALSE); }
    }
    LOADING.with_borrow_mut(|l| l.push((key.clone(), me)));
    let r = load_file(c, &key);
    LOADING.with_borrow_mut(|l| { if let Some(i) = l.iter().position(|(k, t)| *k == key && *t == me) { l.remove(i); } });
    crate::builtins::blop::notify(c, loading_key(&key));
    r?;
    let ks = c.str_new(key.as_bytes());
    let lf = c.vm().loaded_features.get();
    c.ary_push(lf, ks);
    REALPATHS.with_borrow_mut(|r| r.push((key, real)));
    Ok(TRUE)
}

/// A feature another thread is loading: wait for it (Ok(true): look again); this thread's own: circular.
fn wait_loading(c: &mut Ctx, key: &str, me: Value) -> Result<bool, Esc> {
    match loading_thread(key) {
        None => Ok(true),
        Some(t) if t == me => { warn_circular(c, key)?; Ok(false) }
        Some(_) => {
            let k = loading_key(key);
            crate::builtins::thread::block_on(c, |_| Ok(loading_thread(key).is_none()), Some(k), None, true)?;
            Ok(true)
        }
    }
}

// saruby's internal features already loaded
thread_local! { static INTERNAL_LOADED: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) }; }
// features being required, and by which thread (CRuby's loading table)
thread_local! { static LOADING: std::cell::RefCell<Vec<(String, Value)>> = const { std::cell::RefCell::new(Vec::new()) }; }
// (feature, its realpath) of the files require loaded
thread_local! { static REALPATHS: std::cell::RefCell<Vec<(String, String)>> = const { std::cell::RefCell::new(Vec::new()) }; }
fn loading_thread(key: &str) -> Option<Value> { LOADING.with_borrow(|l| l.iter().find(|(k, _)| k == key).map(|e| e.1)) }
fn loading_key(key: &str) -> u64 { key.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3)) | 1 }

fn warn_circular(c: &mut Ctx, key: &str) -> Result<(), Esc> {
    if !c.vm().verbose.get().truthy() { return Ok(()); }
    let at = c.caller_frame().map(|f| format!("{}:{}", frame_file(f), f.line())).unwrap_or_default();
    let mut msg = format!("{}: warning: {}: warning: loading in progress, circular require considered harmful - {}\n", at, at, key);
    for l in backtrace_locs_n(c, 0, None).unwrap_or_default().iter().rev() { msg.push_str(&format!("\tfrom {}:{}:in  '{}'\n", l.file, l.line, l.label)); }
    msg.push('\n');
    warning_write(c, msg.as_bytes())
}

pub enum FeatureState { LoadingByMe, LoadingByOther, Provided, None }
/// Whether the feature an autoload names is loaded or being loaded (check_autoload_required).
pub fn feature_state(c: &Ctx, path: Value) -> FeatureState {
    let name = String::from_utf8_lossy(c.str_bytes(path)).into_owned();
    let lp = {
        let lpv = c.vm().load_path.get();
        (0..c.ary_len(lpv)).map(|i| c.ary_get(lpv, i)).filter(|&d| is_string(d)).map(|d| {
            let e = expand_path_str(&String::from_utf8_lossy(c.str_bytes(d)), None);
            std::fs::canonicalize(&e).map(|p| p.to_string_lossy().into_owned()).unwrap_or(e)
        }).collect::<Vec<_>>()
    };
    let loading: Vec<(String, Value)> = LOADING.with_borrow(|l| l.clone());
    let keys: Vec<String> = loading.iter().map(|e| e.0.clone()).collect();
    if let Some(i) = feature_provided(&name, &lp, &keys).or_else(|| match locate_feature(&name, &lp, false) {
        Feature::File(f) | Feature::Native(f) => keys.iter().position(|k| *k == f), _ => None }) {
        return if loading[i].1 == crate::builtins::thread::current_thread(c) { FeatureState::LoadingByMe } else { FeatureState::LoadingByOther };
    }
    let list = loaded_list(c);
    if feature_provided(&name, &lp, &list).is_some() { return FeatureState::Provided; }
    match locate_feature(&name, &lp, false) {
        Feature::File(f) | Feature::Native(f) if list.contains(&f) => FeatureState::Provided,
        _ => FeatureState::None,
    }
}

/// $LOAD_PATH.resolve_feature_path: [:rb, path], [:so, "name.so"] for a feature saruby provides, or nil.
fn lp_resolve_feature_path(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let sv = expect_str(c, a[0])?;
    let name = String::from_utf8_lossy(c.str_bytes(sv)).into_owned();
    let lp = expanded_load_path(c, true)?;
    let (kind, path) = match locate_feature(&name, &lp, false) {
        Feature::File(f) => ("rb", f),
        Feature::Native(base) => ("so", format!("{}.so", base)),
        Feature::Missing | Feature::SoOnly => return Ok(NIL),
    };
    let ps = c.str_new(path.as_bytes());
    Ok(c.ary_new_from(&[Value::sym(intern(kind.as_bytes())), ps]))
}
pub fn init_load_path(c: &mut Ctx, lp: Value) { define_singleton_cfunc(c, lp, "resolve_feature_path", lp_resolve_feature_path, 1); }

pub fn load_file(c: &mut Ctx, path: &str) -> RV {
    let src = match std::fs::read(path) {
        Ok(s) => s,
        Err(_) => { let e = make_exception(c, c.vm().c_load_error.get(), &format!("cannot load such file -- {}", path)); return Err(raise_exc(c, e)); }
    };
    let file: &'static str = Box::leak(path.to_string().into_boxed_str());
    crate::builtins::exception::note_realpath(file);
    run_source(c, &src, file, 1)
}

/// Compile and run a source at top level (self = main).
pub fn run_source(c: &mut Ctx, src: &[u8], file: &'static str, line: i32) -> RV {
    let is = crate::compile::compile_source(src, file, line, IseqKind::Top, vec![]);
    crate::compile::flush_parse_warnings(c)?;
    let is = match is {
        Ok(is) => is,
        Err(msg) => { return syntax_err(c, msg, file); }
    };
    crate::run_top_iseq(c, is, c.vm().main_obj.get())
}

fn k_require(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV { check_argc(c, args, 1, 1)?; require_feature(c, args[0], false) }
fn k_require_relative(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV { check_argc(c, args, 1, 1)?; require_feature(c, args[0], true) }
fn k_load(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    check_argc(c, args, 1, 2)?;
    let name = feature_name(c, args[0])?;
    // rb_find_file: the name as given (no ".rb" added), in $LOAD_PATH unless explicit, else from the cwd
    let ex = if name.starts_with('~') { let nv = c.str_new(name.as_bytes()); String::from_utf8_lossy(&crate::builtins::file::expand(c, &[nv], true)?).into_owned() } else { name.clone() };
    let explicit = ex.starts_with('/') || ex.starts_with("./") || ex.starts_with("../");
    let mut found = None;
    if !explicit {
        let lp = c.vm().load_path.get();
        for i in 0..c.ary_len(lp) {
            let d = c.ary_get(lp, i);
            let d = if is_string(d) { d } else { let dn = feature_name(c, d)?; c.str_new(dn.as_bytes()) };
            let f = std::path::Path::new(&*String::from_utf8_lossy(c.str_bytes(d))).join(&ex);
            if f.is_file() { found = Some(f.to_string_lossy().into_owned()); break; }
        }
    }
    // rb_find_file expands what it found in $LOAD_PATH and an explicitly relative name
    let abs = |c: &mut Ctx, f: &str| -> Result<String, Esc> { let v = c.str_new(f.as_bytes()); Ok(String::from_utf8_lossy(&crate::builtins::file::expand(c, &[v], true)?).into_owned()) };
    let p = match found {
        Some(f) => abs(c, &f)?,
        None if std::path::Path::new(&ex).is_file() => if ex.starts_with("./") || ex.starts_with("../") { abs(c, &ex)? } else { ex },
        None => {
            let e = make_exception(c, c.vm().c_load_error.get(), &format!("cannot load such file -- {}", name));
            { let p = c.str_new(name.as_bytes()); ivar_set_raw(c, e, intern(b"path"), p); }
            return Err(raise_exc(c, e));
        }
    };
    let wrap = args.get(1).copied().unwrap_or(FALSE);
    if wrap.truthy() {
        // load(file, true): run under an anonymous module, self = a clone of main extended with it
        let m = if is_class_or_module(wrap) { wrap } else { new_module(c) };
        let main = c.vm().main_obj.get();
        let slf = crate::builtins::object::obj_dup(c, main, true, None)?;
        let sk = singleton_class(c, slf);
        include_module_c(c, sk, m);
        let src = match std::fs::read(&p) {
            Ok(s) => s,
            Err(_) => { let e = make_exception(c, c.vm().c_load_error.get(), &format!("cannot load such file -- {}", p)); return Err(raise_exc(c, e)); }
        };
        let file: &'static str = Box::leak(p.into_boxed_str());
        crate::builtins::exception::note_realpath(file);
        let is = crate::compile::compile_source(&src, file, 1, IseqKind::Top, vec![]);
        crate::compile::flush_parse_warnings(c)?;
        let is = match is { Ok(is) => is, Err(msg) => return syntax_err(c, msg, file) };
        let cref = new_cref(m, c.vm().top_cref.get());
        crate::run_top_iseq_cref(c, is, slf, cref)?;
        return Ok(TRUE);
    }
    load_file(c, &p)?;
    Ok(TRUE)
}

/// $/ was assigned (String#chomp reads it only then).
pub static RS_SET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// -n / -p: Kernel#chomp and #chop on $_ (module functions, as CRuby's rb_define_global_function).
pub fn define_dash_n(c: &mut Ctx) {
    fn on_line(c: &mut Ctx, a: &[Value], m: &[u8]) -> RV {
        let l = crate::builtins::svar::last_line(c);
        let s = expect_str(c, l)?;
        let r = funcall(c, s, intern(m), a, std::ptr::null())?;
        crate::builtins::svar::set_last_line(c, r);
        Ok(r)
    }
    let k = c.vm().m_kernel.get();
    define_module_function(c, k, "chomp", |c, _s, a, _b| on_line(c, a, b"chomp"), -1);
    define_module_function(c, k, "chop", |c, _s, a, _b| { check_argc(c, a, 0, 0)?; on_line(c, a, b"chop") }, -1);
}
/// rb_class_real(rb_vm_cbase()): where Kernel#autoload / #autoload? work.
fn autoload_cbase(c: &Ctx) -> Value {
    let Some(f) = c.caller_frame() else { return c.vm().c_object.get() };
    // the class a `def` there would define on (rt::srb_def): instance_eval's singleton included
    let mut k = c.cref_ref(f.cref()).map_or(c.vm().c_object.get(), |r| r.klass);
    // a method defined in Class.new / class_exec: the cref's def target is its class
    if let Some(t) = c.cref_ref(f.cref()).map(|r| r.def_target).filter(|&t| is_class_or_module(t)) { k = t; }
    if f.defined_class().is_heap() && f.kind != FK_CLASS { k = f.defined_class(); }
    while c.class(k).is_singleton { k = c.class(k).superclass; }
    k
}
fn k_autoload(c: &mut Ctx, _slf: Value, args: &[Value], b: Blk) -> RV { let k = autoload_cbase(c); m_autoload(c, k, args, b) }
/// rb_mod_autoload / rb_autoload_str.
fn m_autoload(c: &mut Ctx, k: Value, args: &[Value], _b: Blk) -> RV {
    check_argc(c, args, 2, 2)?;
    let nm = crate::builtins::object::to_sym_arg(c, args[0])?;
    let path = if is_string(args[1]) { args[1] } else { let f = feature_name(c, args[1])?; c.str_new(f.as_bytes()) };
    if !sym_bytes(nm).first().is_some_and(|b| b.is_ascii_uppercase()) {
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("autoload must be constant name: {}", sym_str(nm)), Value::sym(nm), k)?;
        return Err(raise_exc(c, e));
    }
    check_frozen(c, k)?;
    if c.str_len(path) == 0 { return arg_err(c, "empty feature name"); }
    if const_get_at(c, k, nm).is_some() { return Ok(NIL); }
    c.class_mut(k).autoload.insert(nm, path);
    if let Some(l) = source_location(c) { c.class_mut(k).cx_mut().const_loc.insert(nm, l); }
    call_const_added(c, k, nm)?;
    Ok(NIL)
}
/// Run an autoload's require: `main.require(path)` as CRuby (so an overridden require sees it);
/// saruby's own lazy features load directly.
pub fn autoload_require(c: &mut Ctx, path: Value) -> Result<(), Esc> {
    if c.str_bytes(path).starts_with(b"saruby/") { require_feature(c, path, false)?; return Ok(()); }
    let main = c.vm().main_obj.get();
    funcall(c, main, s::require, &[path], std::ptr::null())?;
    Ok(())
}

fn m_ruby2_keywords(c: &mut Ctx, slf: Value, args: &[Value], _b: Blk) -> RV {
    let k = if is_class_or_module(slf) { slf } else { c.vm().c_object.get() };
    for &a in args {
        let nm = crate::builtins::object::to_sym_arg(c, a)?;
        let me = search_method(c, k, nm);
        if me.is_null() {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("undefined method '{}' for {} '{}'", sym_str(nm), if c.class(k).is_module { "module" } else { "class" }, class_path(c, k)), Value::sym(nm), k)?;
            return Err(raise_exc(c, e));
        }
        let wloc = match c.caller_frame() { Some(pf) => format!("{}:{}: warning: Skipping set of ruby2_keywords flag for {} ", frame_file(pf), pf.line(), sym_str(nm)), None => String::new() };
        let Some(me) = c.me_ref(me) else { continue };
        if me.owner != k {
            warning_write(c, format!("{}(can only set in method defining module)\n", wloc).as_bytes())?;
            continue;
        }
        // a define_method body is flagged on its block's iseq, as in CRuby
        let target = match me.kind {
            MKind::Iseq(is) => Some(is),
            MKind::Bmethod(pv) => match c.proc_data(pv).blk.kind { BlockKind::Iseq(is) => Some(is), _ => None },
            _ => None,
        };
        let ok = match me.kind {
            MKind::Iseq(_) | MKind::Bmethod(_) if target.is_some() => {
                let Some(is) = c.iseq_ref(target.unwrap()) else { continue };
                let p = &is.params;
                let ok = p.rest.is_some() && p.post.is_empty() && p.kw.is_empty() && p.kwrest.is_none() && !p.no_kw;
                if ok { is.ruby2_keywords.set(true); }
                ok
            }
            MKind::Cfunc(..) => { warning_write(c, format!("{}(method not defined in Ruby)\n", wloc).as_bytes())?; continue; }
            _ => false,
        };
        if !ok { warning_write(c, format!("{}(method accepts keywords or post arguments or method does not accept argument splat)\n", wloc).as_bytes())?; }
    }
    Ok(NIL)
}
fn h_s_r2k_hash_p(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    if !is_hash(args[0]) { return type_err(c, format!("wrong argument type {} (expected Hash)", class_name_for_err(c, args[0]))); }
    Ok(Value::bool(obj_flags(args[0]) & FL_KWHASH != 0))
}
fn h_s_r2k_hash(c: &mut Ctx, _s: Value, args: &[Value], _b: Blk) -> RV {
    if !is_hash(args[0]) { return type_err(c, format!("wrong argument type {} (expected Hash)", class_name_for_err(c, args[0]))); }
    // rb_hash_dup: class, ivars, default / default_proc, compare_by_identity kept
    let h = args[0];
    let d = c.hash_dup(h);
    obj_set_klass(d, real_class_of(c, h));
    if obj_flags(h) & crate::object::FL_EXIVAR != 0 { for (k, v) in ivar_list(c, h) { ivar_set_raw(c, d, k, v); } }
    obj_set_flags(d, FL_KWHASH);
    Ok(d)
}
fn m_s_nesting(c: &mut Ctx, _s: Value, _args: &[Value], _b: Blk) -> RV {
    let out = c.ary_new();
    let mut cr = c.caller_frame().map_or(std::ptr::null(), |f| f.cref());
    while let Some(r) = c.cref_ref(cr) {
        if r.prev.is_null() { break; }
        let (k, prev) = (cref_klass(c, r), r.prev);
        c.ary_push(out, k);
        cr = prev;
    }
    Ok(out)
}
fn k_autoload_p(c: &mut Ctx, _slf: Value, args: &[Value], b: Blk) -> RV { let k = autoload_cbase(c); m_autoload_p(c, k, args, b) }
fn m_autoload_p(c: &mut Ctx, k: Value, args: &[Value], _b: Blk) -> RV {
    check_argc(c, args, 1, 2)?;
    let nm = crate::builtins::object::to_sym_arg(c, args[0])?;
    let inherit = args.get(1).map(|v| v.truthy()).unwrap_or(true);
    let list: Vec<Value> = if inherit { ancestors(c, k).to_vec() } else { vec![k] };
    for kk in list {
        if const_get_at(c, kk, nm).is_some() { return Ok(NIL); }
        if let Autoload::Ready(p) = autoload_state(c, kk, nm) { return Ok(if c.str_bytes(p).starts_with(b"saruby/") { NIL } else { p }); }
        if c.class(kk).autoload.contains_key(&nm) { return Ok(NIL); }
    }
    Ok(NIL)
}

fn k_dir(c: &mut Ctx, _s: Value, _args: &[Value], _b: Blk) -> RV {
    let f = c.caller_frame().map_or("-", frame_file).to_string();
    // code from eval: no real path; a given file name's dirname as is
    if c.caller_frame().is_some_and(|x| x.kind == FK_EVAL) {
        if f.starts_with("(eval") { return Ok(NIL); }
        let d = match f.rfind('/') { Some(0) => "/".to_string(), Some(i) => f[..i].to_string(), None => ".".to_string() };
        return Ok(c.str_new(d.as_bytes()));
    }
    let abs = std::path::PathBuf::from(crate::builtins::exception::realpath(&f).unwrap_or(f));
    Ok(c.str_new(abs.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or(".".into()).as_bytes()))
}

// ---- eval / binding --------------------------------------------------------------------

fn k_eval(c: &mut Ctx, slf: Value, args: &[Value], _b: Blk) -> RV {
    check_argc(c, args, 1, 4)?;
    let src = expect_str(c, args[0])?;
    let bind = if args.len() > 1 { args[1] } else { NIL };
    if bind != NIL && !crate::builtins::binding::is_binding(c, bind) {
        return type_err(c, format!("wrong argument type {} (expected binding)", class_name_for_err(c, bind)));
    }
    let (file, line) = crate::builtins::binding::eval_file_line(c, args.get(2).copied(), args.get(3).copied())?;
    let (srcb, enc) = (c.str_bytes(src).to_vec(), c.str_enc(src));
    if bind != NIL { return crate::builtins::binding::eval_in_binding(c, bind, &srcb, enc, file, line); }
    let b = crate::builtins::binding::binding_of_caller(c)?;
    // eval_string_with_cref(self, ...): the receiver (Kernel.eval) is self, the locals are the caller's
    crate::builtins::binding::set_receiver(c, b, slf);
    crate::builtins::binding::eval_in_binding(c, b, &srcb, enc, file, line)
}

fn k_binding(c: &mut Ctx, _s: Value, _args: &[Value], _b: Blk) -> RV { crate::builtins::binding::binding_of_caller(c) }

fn k_local_variables(c: &mut Ctx, _s: Value, _args: &[Value], _b: Blk) -> RV {
    let b = crate::builtins::binding::binding_of_caller(c)?;
    crate::builtins::binding::local_variables(c, b)
}

pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let k = vmr.m_kernel.get();
    let mf = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_module_function(c, k, n, f, a);
    mf(c, "puts", k_puts, -1);
    mf(c, "print", k_print, -1);
    mf(c, "p", k_p, -1);
    mf(c, "putc", k_putc, 1);
    mf(c, "printf", k_printf, -1);
    mf(c, "sprintf", k_sprintf, -1);
    mf(c, "format", k_sprintf, -1);
    mf(c, "warn", k_warn, -1);
    mf(c, "trace_var", k_trace_var, -1);
    mf(c, "untrace_var", k_untrace_var, -1);
    mf(c, "raise", k_raise, -1);
    mf(c, "fail", k_raise, -1);
    mf(c, "loop", k_loop, 0);
    mf(c, "catch", k_catch, -1);
    mf(c, "throw", k_throw, -1);
    mf(c, "block_given?", k_block_given, 0);
    mf(c, "lambda", k_lambda, 0);
    mf(c, "proc", k_proc, 0);
    mf(c, "__method__", k_method_name, 0);
    mf(c, "__callee__", k_callee_name, 0);
    mf(c, "caller", k_caller, -1);
    mf(c, "exit", k_exit, -1);
    mf(c, "exit!", k_exit_bang, -1);
    mf(c, "abort", k_abort, -1);
    mf(c, "at_exit", k_at_exit, 0);
    mf(c, "gets", k_gets, -1);
    mf(c, "readline", k_readline, -1);
    mf(c, "readlines", k_readlines, -1);
    mf(c, "rand", k_rand, -1);
    mf(c, "srand", k_srand, -1);
    mf(c, "Integer", k_integer, -1);
    mf(c, "Float", k_float, -1);
    mf(c, "String", k_string, -1);
    mf(c, "Array", k_array, -1);
    mf(c, "Hash", k_hash_conv, -1);
    mf(c, "require", k_require, -1);
    mf(c, "require_relative", k_require_relative, -1);
    mf(c, "load", k_load, -1);
    mf(c, "autoload", k_autoload, -1);
    mf(c, "autoload?", k_autoload_p, -1);
    define_cfunc(c, vmr.c_module.get(), "autoload?", m_autoload_p, -1);
    define_private_cfunc(c, vmr.c_module.get(), "ruby2_keywords", m_ruby2_keywords, -1);
    let sm = singleton_class(c, vmr.main_obj.get());
    define_private_cfunc(c, sm, "ruby2_keywords", m_ruby2_keywords, -1);
    define_singleton_cfunc(c, vmr.c_module.get(), "nesting", m_s_nesting, 0);
    define_singleton_cfunc(c, vmr.c_hash.get(), "ruby2_keywords_hash?", h_s_r2k_hash_p, 1);
    define_singleton_cfunc(c, vmr.c_hash.get(), "ruby2_keywords_hash", h_s_r2k_hash, 1);
    mf(c, "__dir__", k_dir, 0);
    mf(c, "eval", k_eval, -1);
    mf(c, "binding", k_binding, 0);
    mf(c, "local_variables", k_local_variables, 0);
    define_cfunc(c, vmr.c_module.get(), "autoload", m_autoload, -1);
    let _ = intern_str;
}

pub fn parse_int_for_format(c: &mut Ctx, v: Value) -> RV {
    let b = c.str_bytes(v).to_vec();   // str_to_inum may allocate a bignum
    match str_to_inum(c, &b, 0, true) {
        Some(x) => Ok(x),
        None => { let ins = inspect_str(c, v)?; arg_err(c, format!("invalid value for Integer(): {}", ins)) }
    }
}
