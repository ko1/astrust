// Assorted built-ins: pack/unpack, GC, ObjectSpace, Warning, RbConfig, constants.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{obj_klass, obj_set_flags, Global};
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::hash::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::sync::atomic::AtomicBool;

// ---- Encoding (the methods and objects are encoding.rs's) --------------------------------

pub struct REnc { pub id: u8 }
impl RData for REnc {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
pub fn encoding_value(c: &mut Ctx, id: u8) -> RV { Ok(crate::builtins::encoding::encoding_value(c, id)) }

// ---- base64 (pack itself is pack.rs) ----------------------------------------------------------------------

pub fn base64(b: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for ch in b.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char); out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if ch.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if ch.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}
pub fn unbase64(s2: &[u8]) -> Vec<u8> {
    let mut out = vec![]; let mut buf = 0u32; let mut bits = 0;
    for &x in s2 {
        let v = match x { b'A'..=b'Z' => x - b'A', b'a'..=b'z' => x - b'a' + 26, b'0'..=b'9' => x - b'0' + 52, b'+' | b'-' => 62, b'/' | b'_' => 63, _ => continue } as u32;
        buf = buf << 6 | v; bits += 6;
        if bits >= 8 { bits -= 8; out.push((buf >> bits) as u8); }
    }
    out
}
// ---- GC / ObjectSpace -----------------------------------------------------------------------

fn gc_start(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { crate::gc::collect(c); run_finalizer_queue(c, FinAt::Internal(44)); Ok(NIL) }
// GC.__start(full_mark), behind the prelude's GC.start (keywords as in CRuby's gc.rb, no Hash)
fn gc_start_full(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    if a[0].truthy() { crate::gc::collect(c) } else { crate::gc::collect_minor(c) }
    run_finalizer_queue(c, FinAt::Internal(44));
    Ok(NIL)
}
fn gc_count(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(crate::gc::collections() as i64)) }
fn gc_disable(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(crate::gc::set_disabled(true))) }
fn gc_enable(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(crate::gc::set_disabled(false))) }
fn gc_stat(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (n, m, h) = (crate::gc::collections() as i64, crate::gc::minor_collections() as i64, crate::gc::stats());
    let st = c.hash_new();
    for (k, v) in [("count", n), ("heap_live_slots", (h.slots - h.free) as i64), ("heap_free_slots", h.free as i64),
                   ("total_allocated_objects", h.allocated as i64), ("total_freed_objects", h.freed as i64), ("heap_available_slots", h.slots as i64), ("major_gc_count", n - m), ("minor_gc_count", m)] {
        hash_set(c, st, sym_value(k), Value::fix(v))?;
    }
    if let Some(&k) = a.first() { if k.is_sym() { return Ok(hash_get_opt(c, st, k)?.unwrap_or(Value::fix(0))); } }
    Ok(st)
}
fn os_each_object(c: &mut Ctx, s2: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s2, intern(b"each_object"), a, NIL); }
    let k = a.first().copied();
    // nothing allocates between the walk and the Array that roots the result
    let mut vs = vec![];
    crate::gc::each_object(|v| vs.push(v));
    vs.retain(|&v| {
        if matches!(c.body_of(v), Body::Env(..)) || obj_klass(v) == NIL { return false; }
        if is_class_or_module(v) && crate::builtins::objspace::singleton_internal(c, v) { return false; }
        match k { Some(k) => is_class_or_module(obj_klass(v)) && is_kind_of(c, v, k), None => true }
    });
    let n = vs.len();
    let list = c.ary_from_vec(vs);
    for i in 0..n { let x = c.ary_get(list, i); yield1(c, blk, x)?; }
    Ok(Value::fix(n as i64))
}
// ObjectSpace.garbage_collect(full_mark: true, immediate_mark: true, immediate_sweep: true), as gc.rb
fn os_garbage_collect(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let mut full = true;
    if let Some(h) = kw_arg(c, a) {
        let mut unknown = vec![];
        for (k, v) in c.hash_pairs(h) {
            if k == sym_value("full_mark") { full = v.truthy(); }
            else if k != sym_value("immediate_mark") && k != sym_value("immediate_sweep") { unknown.push(inspect_str(c, k)?); }
        }
        if !unknown.is_empty() { return arg_err(c, format!("unknown keyword{}: {}", if unknown.len() > 1 { "s" } else { "" }, unknown.join(", "))); }
    }
    if full { crate::gc::collect(c) } else { crate::gc::collect_minor(c) }
    run_finalizer_queue(c, FinAt::Internal(595));
    Ok(NIL)
}

// Warning[category] flags (bit = index in WARN_CAT_NAMES); experimental is on by default, -w turns on deprecated.
static WARN_CATS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0b0010);
const WARN_CAT_NAMES: [&str; 4] = ["deprecated", "experimental", "performance", "strict_unused_block"];
use std::sync::atomic::Ordering::Relaxed;
pub fn warning_enable_deprecated() { WARN_CATS.fetch_or(1, Relaxed); }
/// rb_warning_category_from_name: a Symbol naming a category.
pub fn warning_category(c: &mut Ctx, v: Value) -> Result<usize, Esc> {
    if !v.is_sym() { return type_err(c, format!("wrong argument type {} (expected Symbol)", class_name_for_err(c, v))); }
    let n = crate::symbol::sym_str(v.as_sym());
    match WARN_CAT_NAMES.iter().position(|&x| x == n) { Some(i) => Ok(i), None => arg_err(c, format!("unknown category: {}", n)) }
}
pub fn warning_category_enabled(i: usize) -> bool { WARN_CATS.load(Relaxed) & (1 << i) != 0 }
fn warning_warn(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    if !is_string(a[0]) { return type_err(c, format!("wrong argument type {} (expected String)", class_name_for_err(c, a[0]))); }
    let b = c.str_bytes(a[0]).to_vec(); stderr_write(c, &b)?; Ok(NIL)
}
// Finalizers (gc/default.c): FIN_TABLE maps an object to [object_id, callable...] by identity with weak
// keys; the collector moves the array of an object that died to FIN_QUEUE (gc.rs sweep_weak) and sets the
// interrupt bit, so the queue runs at the next check point (thread::poll), or at once after GC.start.
static FIN_TABLE: Global = Global::new();
static FIN_QUEUE: Global = Global::new();
static FIN_ANY: AtomicBool = AtomicBool::new(false);       // some finalizer was ever defined
static FIN_PENDING: AtomicBool = AtomicBool::new(false);   // FIN_QUEUE is not empty
static FIN_RUNNING: AtomicBool = AtomicBool::new(false);   // CRuby's `finalizing`: the queue is not re-entered
fn fin_table(c: &mut Ctx) -> Value {
    let t = FIN_TABLE.get(c);
    if t != NIL { return t; }
    let h = c.alloc(NIL, Body::Hash(Box::new(RHash::with_capacity(0))));
    c.hash_mut(h).identity = true;
    obj_set_flags(h, FL_WEAK_KEYS);
    FIN_TABLE.set(c, h);
    let q = c.ary_new();
    FIN_QUEUE.set(c, q);
    FIN_ANY.store(true, Relaxed);
    h
}
/// The finalizer table (gc.rs tells it from the other weak tables), nil before the first define.
pub fn fin_table_value(c: &Ctx) -> Value { FIN_TABLE.get(c) }
/// The collector found these tables' objects dead: queue them (no allocation, no Ruby code).
pub fn fin_enqueue(c: &mut Ctx, dead: Vec<Value>) {
    let q = FIN_QUEUE.get(c);
    c.ary_mut(q).extend(dead);
    FIN_PENDING.store(true, Relaxed);
    crate::vm::set_ints();
}
fn os_define_finalizer(c: &mut Ctx, _s: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let o = a[0];
    if !o.is_heap() || is_float(o) || is_bignum(o) { return arg_err(c, format!("cannot define finalizer for {}", class_path(c, real_class_of(c, o)))); }
    if is_frozen(o) { return Err(raise_frozen(c, o)); }
    let p = if a.len() > 1 { a[1] } else if !blk.is_null() { block_to_proc(c, blk)? } else { return arg_err(c, "tried to create Proc object without a block"); };
    if !respond_to(c, p, intern(b"call"), true) { let d = inspect_str(c, p)?; return arg_err(c, format!("wrong type argument {} (should be callable)", d)); }
    // the callable keeps the object alive: CRuby warns (it then runs only at exit)
    let recv = if is_proc(p) { c.proc_data(p).blk.self_ } else if is_method_obj(p) { c.method_obj(p).recv } else { NIL };
    if recv == o { gvar_warn(c, "finalizer references object to be finalized", false)?; }
    let t = fin_table(c);
    let p = match hash_get_opt(c, t, o)? {
        Some(fs) => {
            // an equal callable is defined once
            let mut found = None;
            let mut i = 1;
            while i < c.ary_len(fs) {
                let q = c.ary_get(fs, i);
                if funcall(c, q, intern(b"=="), &[p], std::ptr::null())?.truthy() { found = Some(q); break; }
                i += 1;
            }
            match found { Some(q) => q, None => { c.ary_push(fs, p); p } }
        }
        None => { let id = Value::fix(object_id(o) as i64); let fs = c.ary_new_from(&[id, p]); hash_set(c, t, o, fs)?; p }
    };
    let r = c.ary_new_from(&[Value::fix(0), p]);
    obj_set_flags(r, FL_FROZEN);
    Ok(r)
}
fn os_undefine_finalizer(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    if a[0].is_heap() && is_frozen(a[0]) { return Err(raise_frozen(c, a[0])); }
    let t = FIN_TABLE.get(c);
    if t != NIL { hash_delete(c, t, a[0])?; }
    Ok(a[0])
}
/// dup / clone: the copy gets the finalizers too, called with its own id (rb_gc_copy_finalizer).
pub fn copy_finalizer(c: &mut Ctx, from: Value, to: Value) -> Result<(), Esc> {
    if !FIN_ANY.load(Relaxed) { return Ok(()); }
    let t = FIN_TABLE.get(c);
    let Some(fs) = hash_get_opt(c, t, from)? else { return Ok(()) };
    let v = c.ary(fs).to_vec();
    let n = c.ary_from_vec(v);
    c.ary_mut(n)[0] = Value::fix(object_id(to) as i64);
    hash_set(c, t, to, n)
}
/// Where a finalizer's exception is reported: the current frame, CRuby's gc.rb line, or the script (exit).
#[derive(Clone, Copy)]
pub enum FinAt { Here, Internal(u32), Exit }
/// rb_gc_run_obj_finalizer: each callable of `fs` with the id; an exception is reported and the next one runs.
fn run_one(c: &mut Ctx, fs: Value, at: FinAt) {
    let id = c.ary_get(fs, 0);
    let saved = c.errinfo;
    let mut i = 1;
    while i < c.ary_len(fs) {
        let p = c.ary_get(fs, i);
        i += 1;
        if funcall(c, p, intern(b"call"), &[id], std::ptr::null()).is_ok() { continue; }
        let e = c.esc_val;
        c.esc = EscKind::None;
        if c.vm().verbose.get() != NIL {
            let pi = inspect_str(c, p).unwrap_or_default();
            c.esc = EscKind::None;
            let loc = match at {
                FinAt::Here => c.cur_frame().map(|f| format!("{}:{}", frame_file(f), f.line())).unwrap_or_default(),
                FinAt::Internal(l) => format!("<internal:gc>:{}", l),
                FinAt::Exit => { let z = c.vm().dollar_zero.get(); if is_string(z) { String::from_utf8_lossy(c.str_bytes(z)).into_owned() } else { String::new() } }
            };
            let msg = format!("{}: warning: Exception in finalizer {}\n", loc, pi);
            if warning_write(c, msg.as_bytes()).is_err() { c.esc = EscKind::None; }
            crate::report_exception(c, e);
        }
        c.esc = EscKind::None;
    }
    c.errinfo = saved;
}
/// Run the finalizers of objects collected so far (gc_finalize_deferred); a no-op while they run.
pub fn run_finalizer_queue(c: &mut Ctx, at: FinAt) {
    if !FIN_PENDING.load(Relaxed) || FIN_RUNNING.swap(true, Relaxed) { return; }
    drain_queue(c, at);
    FIN_RUNNING.store(false, Relaxed);
}
fn drain_queue(c: &mut Ctx, at: FinAt) {
    loop {
        let q = FIN_QUEUE.get(c);
        if c.ary_len(q) == 0 { break; }
        let fs = c.ary_mut(q).remove(0);
        run_one(c, fs, at);
    }
    FIN_PENDING.store(false, Relaxed);
}
/// At exit: the finalizers of every object still registered (in the order defined, including ones
/// defined meanwhile), then those of objects collected but not finalized yet.
pub fn run_finalizers(c: &mut Ctx) {
    if !FIN_ANY.load(Relaxed) { return; }
    FIN_RUNNING.store(true, Relaxed);
    let t = FIN_TABLE.get(c);
    loop {
        let first = (0..).map_while(|i| c.hash_entry_at(t, i)).find(|e| e.0 != UNDEF);
        let Some((o, fs)) = first else { break };
        let _ = hash_delete(c, t, o);
        run_one(c, fs, FinAt::Exit);
    }
    drain_queue(c, FinAt::Exit);
    FIN_RUNNING.store(false, Relaxed);
}
fn rbconfig_ruby(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "saruby".into());
    Ok(c.str_new(exe.as_bytes()))
}
pub fn rbconfig_init(c: &mut Ctx) {
    if const_get_at(c, c.vm().c_object.get(), intern(b"RbConfig")).is_some() { return; }
    let rbc = define_module(c, "RbConfig");
    let cfg = c.hash_new_capa(32);
    let exe = std::env::current_exe().unwrap_or_else(|_| "saruby".into());
    let bindir = exe.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| ".".into());
    let name = exe.file_name().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "saruby".into());
    let (lib, _gem) = crate::builtins::prelude::stdlib_dirs().unwrap_or_default();
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" };
    let archname = format!("{}-linux", arch);
    let archdir = format!("{}/{}", lib, archname);
    let libprefix = std::path::Path::new(&lib).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    // the Unicode version of the case / property tables (Rust's char)
    let (u0, u1, u2) = char::UNICODE_VERSION;
    let (uv, uev) = (format!("{}.{}.{}", u0, u1, u2), format!("{}.{}", u0, u1));
    let entries: &[(&str, &str)] = &[("ruby_install_name", &name), ("RUBY_INSTALL_NAME", &name), ("RUBY_BASE_NAME", "ruby"), ("bindir", &bindir), ("EXEEXT", ""),
        ("host_os", "linux"), ("target_os", "linux"), ("host_cpu", arch), ("target_cpu", arch), ("arch", &archname), ("host", &archname),
        ("MAJOR", "3"), ("MINOR", "4"), ("TEENY", "0"), ("PATCHLEVEL", "0"), ("ruby_version", "3.4.0"), ("RUBY_PROGRAM_VERSION", "3.4.0"),
        ("RUBY_API_VERSION", "3.4"), ("rubylibdir", &lib), ("rubylibprefix", &libprefix), ("archdir", &archdir), ("rubyarchdir", &archdir), ("DLEXT", "so"), ("SOEXT", "so"),
        ("LIBEXT", "a"), ("OBJEXT", "o"), ("PATH_SEPARATOR", ":"), ("SHELL", "/bin/sh"), ("CC", "gcc"), ("CPP", "gcc -E"), ("ENABLE_SHARED", "yes"),
        ("build_os", "linux"), ("build_cpu", arch), ("build", &archname), ("LIBRUBY_SO", "libsaruby.so"), ("LIBRUBY", "libsaruby.so"), ("libdirname", "libdir"), ("LIBPATHENV", "LD_LIBRARY_PATH"), ("AR", "ar"), ("STRIP", "strip"), ("UNICODE_VERSION", &uv), ("UNICODE_EMOJI_VERSION", &uev), ("prefix", &bindir), ("exec_prefix", &bindir),
        ("libdir", &bindir), ("sitedir", ""), ("sitelibdir", ""), ("sitearchdir", ""), ("vendordir", ""), ("vendorlibdir", ""), ("vendorarchdir", ""),
        ("datadir", ""), ("mandir", ""), ("sysconfdir", "/etc"), ("localstatedir", "/var"), ("configure_args", ""), ("CFLAGS", ""), ("LDFLAGS", "")];
    for (k, v) in entries { let ks = c.str_new_frozen(k.as_bytes()); let vs = c.str_new(v.as_bytes()); let _ = hash_set(c, cfg, ks, vs); }
    const_set(c, rbc, intern(b"CONFIG"), cfg);
    const_set(c, rbc, intern(b"MAKEFILE_CONFIG"), cfg);
    let top = c.str_new(bindir.as_bytes());
    const_set(c, rbc, intern(b"TOPDIR"), top);
    define_singleton_cfunc(c, rbc, "ruby", rbconfig_ruby, 0);
}

pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    Ok(if c.data::<REnc>(v).is_some() { Some(v) } else { None })
}
pub fn alloc_data(_c: &mut Ctx, _klass: Value) -> Result<Option<Value>, Esc> { Ok(None) }

/// A frozen constant String.
fn boot_str(c: &mut Ctx, b: &[u8]) -> Value {
    let k = c.vm().c_string.get();
    let s = c.alloc(k, Body::String(RString { b: crate::sbuf::SBuf::from_slice(b), enc: ENC_UTF8 }));
    obj_set_flags(s, FL_FROZEN);
    s
}

pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let ob = vmr.c_object.get();
    // GC / ObjectSpace
    let gc = define_module(c, "GC");
    vmr.m_gc.set(gc);
    define_module_function(c, gc, "start", gc_start, -1);
    define_singleton_cfunc(c, gc, "__start", gc_start_full, 1);
    define_module_function(c, gc, "count", gc_count, 0);
    define_module_function(c, gc, "disable", gc_disable, 0);
    define_module_function(c, gc, "enable", gc_enable, 0);
    define_module_function(c, gc, "stat", gc_stat, -1);
    define_module_function(c, gc, "compact", gc_start, 0);
    define_module_function(c, gc, "stress", |_c, _s, _a, _b| Ok(FALSE), 0);
    define_module_function(c, gc, "stress=", |_c, _s, a, _b| Ok(a[0]), 1);
    let os = define_module(c, "ObjectSpace");
    vmr.m_object_space.set(os);
    define_module_function(c, os, "each_object", os_each_object, -1);
    define_module_function(c, os, "count_objects", crate::builtins::objspace::count_objects, -1);
    define_module_function(c, os, "garbage_collect", os_garbage_collect, -1);
    define_module_function(c, os, "define_finalizer", os_define_finalizer, -1);
    define_module_function(c, os, "undefine_finalizer", os_undefine_finalizer, 1);
    crate::builtins::objspace::init_core(c);
    let wn = define_module(c, "Warning");
    vmr.m_warning.set(wn);
    // rb_define_method + rb_extend_object: a Warning#warn defined later is what Warning.warn calls
    define_cfunc(c, wn, "warn", warning_warn, -1);
    let wsc = singleton_class(c, wn);
    include_module(c, wsc, wn);
    define_singleton_cfunc(c, wn, "[]", |c, _s, a, _b| { let i = warning_category(c, a[0])?; Ok(Value::bool(WARN_CATS.load(Relaxed) & (1 << i) != 0)) }, 1);
    define_singleton_cfunc(c, wn, "[]=", |c, _s, a, _b| {
        let i = warning_category(c, a[0])?;
        if a[1].truthy() { WARN_CATS.fetch_or(1 << i, Relaxed); } else { WARN_CATS.fetch_and(!(1 << i), Relaxed); }
        Ok(a[1])
    }, 2);
    define_singleton_cfunc(c, wn, "categories", |c, _s, _a, _b| { let v: Vec<Value> = WARN_CAT_NAMES.iter().map(|n| sym_value(n)).collect(); Ok(c.ary_new_from(&v)) }, 0);
    // constants
    let set = |c: &mut Ctx, n: &str, v: Value| const_set(c, ob, intern(n.as_bytes()), v);
    for (n, v) in [("RUBY_VERSION", "3.4.0"), ("RUBY_RELEASE_DATE", "2026-11-20"), ("RUBY_PLATFORM", "x86_64-linux"), ("RUBY_ENGINE", "saruby"),
                   ("RUBY_ENGINE_VERSION", "0.1.0"), ("RUBY_COPYRIGHT", "saruby - Copyright (C) 2026 Koichi Sasada")] { let s = boot_str(c, v.as_bytes()); set(c, n, s); }
    let s = boot_str(c, crate::version().as_bytes());
    set(c, "RUBY_DESCRIPTION", s);
    set(c, "RUBY_PATCHLEVEL", Value::fix(0));
    set(c, "ARGV", vmr.argv.get());
    crate::builtins::lazy_const(c, ob, "TOPLEVEL_BINDING", "saruby/toplevel_binding");
    let _ = (intern, num_to_long);
    vmr.c_location.set(crate::builtins::exception::init_location(c));
}
