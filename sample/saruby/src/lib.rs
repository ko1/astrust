// saruby — a Ruby implementation in Rust on ASTrust (AST interpreter + partial
// evaluation to Rust, compiled by rustc and loaded with dlopen).
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code, static_mut_refs)]
#![allow(clippy::all)]

#[path = "../../../runtime/astrust.rs"]
pub mod astrust;
#[path = "../../../runtime/gc/mod.rs"]
pub mod astrust_gc;

pub mod prism;
pub mod prism_ast;
pub mod prism_generic;
pub mod fxhash;
pub mod value;
pub mod symbol;
pub mod sbuf;
pub mod ivbuf;
pub mod avec;
pub mod envvals;
pub mod legacy;
pub mod object;
pub mod heap;
pub mod gc;
pub mod kalloc;
#[cfg(feature = "nodeprof")]
pub mod nodeprof;
pub mod slots;
#[cfg(not(feature = "raw_stack"))]
pub mod stack;
#[cfg(feature = "raw_stack")]
#[path = "stack_raw.rs"]
pub mod stack;
pub mod vm;
pub mod compile;
pub mod rt;
pub mod opt;
pub mod inline_builtin;
pub mod inline_iter;
pub mod bignum;
pub mod string;
pub mod array;
pub mod hash;
pub mod range;
pub mod numeric;
pub mod regexp;
pub mod regex_engine;
pub mod fiber;
pub mod fiber_os;
pub mod aot;
pub mod builtins;

#[allow(unused_imports)]
pub mod node {
    use crate::astrust::*;
    use crate::compile::{IseqRef, CiRef, Str, LitCache};
    use crate::value::*;
    use crate::vm::*;
    use crate::opt::TyFb;
    include!("node_head.rs");
    include!("node_dispatch.rs");
    include!("node_hash.rs");
    include!("node_specialize.rs");
    include!("node_dump.rs");
    include!("node_replace.rs");
    // the prism node id being compiled (compile.rs `c`): every node made meanwhile is tagged with it.  A static,
    // not a thread_local: in libsaruby.so a TLS access is a __tls_get_addr call per node; only the
    // interpreter thread compiles.
    static NODE_TAG: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    use std::sync::atomic::Ordering::Relaxed;
    /// Set the tag of nodes made from now on (a prism node id; 0: none) and return the previous one.
    #[inline(always)]
    pub fn node_tag_swap(id: u32) -> u32 {
        // load + store, not swap (a locked xchg): no other thread writes it
        let old = NODE_TAG.load(Relaxed);
        NODE_TAG.store(id, Relaxed);   // < 1 << (32 - NF_TAG_SHIFT): compile.rs checks the parse's largest
        old
    }
    pub fn node_tag_current() -> u32 { NODE_TAG.load(Relaxed) }
    /// The prism node id a node was compiled from (0: unknown).
    pub fn node_tag(n: NodePtr) -> u32 { node_flags(n) >> NF_TAG_SHIFT }
    #[inline(always)]
    pub fn node_allocated(n: NodePtr) {
        let t = NODE_TAG.load(Relaxed);
        if t != 0 { node_set_flags(n, node_flags(n) | t << NF_TAG_SHIFT); }
    }
    pub use crate::eval::*;
}
#[allow(unused_imports)]
pub mod eval {
    include!("node_eval.rs");
}

use value::*;
use vm::*;

/// Run a top-level iseq with the given self.
pub fn run_top_iseq(c: &mut Ctx, is: &'static Iseq, slf: Value) -> RV {
    run_top_iseq_cref(c, is, slf, vm().top_cref.get())
}

pub fn run_top_iseq_cref(c: &mut Ctx, is: &'static Iseq, slf: Value, cref: *const Cref) -> RV {
    let base = c.top();
    stack_check(c, is.nlocals as usize + 8)?;
    let mut fr = Frame::empty(base);
    fr.prev = c.frame;
    fr.iseq = is;
    fr.kind = FK_TOP;
    fr.visi = VISI_PRIVATE;
    fr.id = c.next_serial();
    fr.cref = cref;
    fr.pos = Pos::info(&LINE_1);
    base[0].set(slf);
    base[1].set(NIL);
    nil_fill(base, 2, is.nlocals as usize);
    c.set_sp(base, is.nlocals as usize);
    if is.env_size > 0 {
        let env = c.alloc(vm().c_env.get(), object::Body::Env(crate::envvals::EnvVals::new(is.env_size as usize), NIL));
        base[1].set(env);
    }
    c.frame = &mut fr.h;
    let r = run_body(c, is.body, base);
    c.frame = fr.prev;
    c.set_sp(base, 0);
    match r {
        Err(e) if c.esc == EscKind::Return && c.esc_tag == fr.id => { let _ = e; c.esc = EscKind::None; Ok(c.esc_val) }
        r => r,
    }
}

struct Opts {
    e: Vec<Vec<u8>>,
    script: Option<String>,
    args: Vec<Vec<u8>>,
    plain: bool,
    aot: bool,
    dump_ast: bool,
    quiet: bool,
    verbose_astro: bool,
    warn: bool,
    nowarn: bool,   // -W0: $VERBOSE = nil
    store: String,
    compiled_only: bool,
    requires: Vec<String>,
    incdirs: Vec<String>,
    check_syntax: bool,
    cmdline: u8,   // prism's -a -l -n -p bits
    inplace: Option<String>,   // -i[ext]: $-i
    enc_ext: Option<String>,   // -E ext[:int] / --encoding / --external-encoding
    enc_int: Option<String>,   // -E :int / --internal-encoding
    uflag: bool,   // -U: default_internal UTF-8
}

fn usage() -> ! {
    eprintln!("usage: saruby [options] [--] [script.rb] [args...]\n\
  -e 'code'        run code\n\
  -I dir           add to $LOAD_PATH\n\
  -r lib           require lib before the script\n\
  -w / -W          enable warnings\n\
  -c               check syntax only\n\
  --plain          pure interpreter (ignore the code store)\n\
  --aot-compile    run, then specialize every executed body into code_store/all.so (rustc)\n\
  --compiled-only  (accepted for harness compatibility; same as default)\n\
  --code-store DIR code store directory (default: ./code_store)\n\
  --dump-ast       print the ASTrust tree of the script and exit\n\
  --astro-verbose  print AOT / GC statistics\n\
  -q               quiet\n\
  --version        print version");
    std::process::exit(2);
}

/// The command line as the process started, raw bytes (-e code and ARGV need not be UTF-8). Use this, never
/// std::env::args: std reads the argv strings lazily, and `$0 =` / Process.setproctitle overwrite them.
pub fn program_args() -> &'static [Vec<u8>] {
    static ARGS: std::sync::OnceLock<Vec<Vec<u8>>> = std::sync::OnceLock::new();
    ARGS.get_or_init(|| std::env::args_os().map(std::os::unix::ffi::OsStringExt::into_vec).collect())
}

fn parse_args() -> Opts {
    let raw = program_args();
    let argv: Vec<String> = raw.iter().map(|a| String::from_utf8_lossy(a).into_owned()).collect();
    let mut o = Opts { e: vec![], script: None, args: vec![], plain: false, aot: false, dump_ast: false, quiet: false,
        verbose_astro: false, warn: false, nowarn: false, store: "code_store".into(), compiled_only: false, requires: vec![], incdirs: vec![], check_syntax: false, cmdline: 0, inplace: None, enc_ext: None, enc_int: None, uflag: false };
    let mut i = 1;
    while i < argv.len() {
        let a = &argv[i];
        if o.script.is_some() || (!o.e.is_empty() && !a.starts_with('-')) { o.args.push(raw[i].clone()); i += 1; continue; }
        match a.as_str() {
            "-e" => { i += 1; o.e.push(raw.get(i).cloned().unwrap_or_else(|| usage())); }
            "--plain" => o.plain = true,
            "--aot-compile" | "--pg-compile" => o.aot = true,
            "--compiled-only" => o.compiled_only = true,
            "--dump-ast" => o.dump_ast = true,
            "-q" | "--quiet" => o.quiet = true,
            "--astro-verbose" => o.verbose_astro = true,
            "-w" | "-W" | "-W2" | "-v" | "--verbose" => { o.warn = true; if a == "-v" { println!("{}", version()); } }
            "-W0" => o.nowarn = true,
            "-c" => o.check_syntax = true,
            // -n / -p / -l / -a, also combined ("-ne", "-lane")
            s if s.len() > 1 && s.starts_with('-') && !s.starts_with("--") && s[1..].bytes().all(|b| b"nple".contains(&b) || b == b'a') && s[1..].bytes().any(|b| b != b'e') => {
                for b in s[1..].bytes() {
                    match b { b'a' => o.cmdline |= 0x1, b'l' => o.cmdline |= 0x4, b'n' => o.cmdline |= 0x8, b'p' => o.cmdline |= 0x18,
                        _ => { i += 1; o.e.push(raw.get(i).cloned().unwrap_or_else(|| usage())); } }
                }
            }
            "--code-store" => { i += 1; o.store = argv.get(i).cloned().unwrap_or_else(|| usage()); }
            "--version" => { println!("{}", version()); std::process::exit(0); }
            "-h" | "--help" => usage(),
            "--" => { i += 1; if i < argv.len() && o.e.is_empty() { o.script = Some(argv[i].clone()); } else if i < argv.len() { o.args.push(raw[i].clone()); } }
            "--enable-frozen-string-literal" | "--yjit" => {}
            s if s.starts_with("--disable-") || s.starts_with("--disable=") => disable_features(&s[10..]),
            s if s.starts_with("-I") => { if s.len() > 2 { o.incdirs.push(s[2..].into()); } else { i += 1; o.incdirs.push(argv.get(i).cloned().unwrap_or_else(|| usage())); } }
            s if s.starts_with("-i") => o.inplace = Some(s[2..].into()),
            "-U" => o.uflag = true,
            "-E" | "--encoding" | "--external-encoding" | "--internal-encoding" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| cli_fail(&format!("missing argument for {}", a)));
                enc_opt(&mut o, a, &v);
            }
            s if s.starts_with("--encoding=") || s.starts_with("--external-encoding=") || s.starts_with("--internal-encoding=") => {
                let (k, v) = s.split_once('=').unwrap_or((s, ""));
                enc_opt(&mut o, k, v);
            }
            s if s.starts_with("-E") => enc_opt(&mut o, "-E", &s[2..]),
            s if s.starts_with("-r") => { if s.len() > 2 { o.requires.push(s[2..].into()); } else { i += 1; o.requires.push(argv.get(i).cloned().unwrap_or_else(|| usage())); } }
            s if s.starts_with("--disable") || s.starts_with("--enable") => {}
            s if s == "--backtrace-limit" || s.starts_with("--backtrace-limit=") => {
                let v = if s.len() > 17 { s[18..].to_string() } else { i += 1; argv.get(i).cloned().unwrap_or_default() };
                if v.is_empty() { cli_fail("missing argument for --backtrace-limit"); }
                match v.parse::<i64>() {
                    Ok(n) if n >= -1 => crate::builtins::exception::BT_LIMIT.store(n, std::sync::atomic::Ordering::Relaxed),
                    _ => cli_fail("wrong limit for backtrace length"),
                }
            }
            s if s.starts_with('-') && s.len() > 1 => { eprintln!("saruby: invalid option {}  (-h will show valid options)", s); std::process::exit(2); }
            _ => { if o.e.is_empty() { o.script = Some(a.clone()); } else { o.args.push(raw[i].clone()); } }
        }
        i += 1;
    }
    o
}

/// ruby.c's fatal option errors: "PROG: message (RuntimeError)", status 1.
fn cli_fail(m: &str) -> ! { eprintln!("saruby: {} (RuntimeError)", m); std::process::exit(1) }
/// -E ext[:int] / --encoding / --external-encoding / --internal-encoding (set_external_encoding_once).
fn enc_opt(o: &mut Opts, opt: &str, v: &str) {
    let (e, i) = match opt { "--internal-encoding" => (None, Some(v)), "--external-encoding" => (Some(v), None), _ => match v.split_once(':') { Some((e, i)) => (Some(e), Some(i)), None => (Some(v), None) } };
    if let Some(e) = e.filter(|e| !e.is_empty()) {
        if let Some(old) = &o.enc_ext { cli_fail(&format!("default_external already set to {}", old)); }
        o.enc_ext = Some(e.into());
    }
    if let Some(i) = i.filter(|i| !i.is_empty()) {
        if let Some(old) = &o.enc_int { cli_fail(&format!("default_internal already set to {}", old)); }
        o.enc_int = Some(i.into());
    }
}
/// --disable-X / --disable=X,Y: gems (gem_prelude, so error_highlight, did_you_mean and syntax_suggest too, as CRuby 4.0), all, or one.
fn disable_features(list: &str) {
    for f in list.split(',') {
        let (eh, dym, ss) = match f { "all" | "gems" => (true, true, true), "error_highlight" => (true, false, false), "did_you_mean" => (false, true, false), "syntax_suggest" => (false, false, true), _ => (false, false, false) };
        if eh { crate::builtins::error_highlight::disable(); }
        if dym { crate::builtins::did_you_mean::disable(); }
        if ss { crate::builtins::syntax_suggest::disable(); }
    }
}

/// The CRuby release saruby follows (RUBY_VERSION, RbConfig, -v); RUBY_ENGINE stays "saruby".
pub const RUBY_VERSION: &str = "4.0.10";
pub const RUBY_RELEASE_DATE: &str = "2026-11-19";
pub const RUBY_REVISION: &str = "a704b7cd95a42cc0cf5f307137bf3aac0285b915";
pub const ENGINE_VERSION: &str = "0.1.0";
pub const RUBY_PLATFORM: &str = if cfg!(target_arch = "aarch64") { "aarch64-linux" } else { "x86_64-linux" };
/// RUBY_DESCRIPTION, also what -v / --version print.
pub fn version() -> String {
    format!("saruby {} (ASTrust) like ruby {} ({} revision {}) +PRISM [{}]", ENGINE_VERSION, RUBY_VERSION, RUBY_RELEASE_DATE, &RUBY_REVISION[..10], RUBY_PLATFORM)
}

/// Print an uncaught exception CRuby-style.
pub fn report_exception(c: &mut Ctx, e: Value) {
    builtins::kernel::out_flush();
    // as CRuby's error_print (C), the message is made without trace events
    let msg = builtins::tracepoint::quiet(|| builtins::exception::full_message(c, e)).unwrap_or_else(|_| "exception\n".into());
    builtins::kernel::err_write(msg.as_bytes());
}

fn run_main(o: Opts) -> i32 {
    let mut c = Ctx {
        sp: NO_SP, stk: NO_STACK, frame: std::ptr::null_mut(),
        esc: EscKind::None, esc_val: NIL, esc_tag: 0, esc_tag_v: NIL, serial: 0, errinfo: NIL, last_match: NIL,
        native_limit: 0, in_ensure: 0, cblk_kw: false, redef: 0, call_me: std::ptr::null(), call_blk: std::ptr::null(),
        // SAFETY: the main context; fibers run instead of it, never beside it
        tok: unsafe { heap::Token::mint() },
    };
    // the value stack grows up from the 64-byte line above `c` (the hottest native-stack data), the native stack
    // down: their lines alias mod 4096 (store-to-load stalls) only when the two hot depths add up to a page
    c.attach_stack(stack_new_at(8 << 20, (&raw const c as usize + std::mem::size_of::<Ctx>()).next_multiple_of(64)));
    let probe = 0u8;
    c.native_limit = (&probe as *const u8 as usize).saturating_sub(STACK_BYTES - (4 << 20));
    vm().main_ctx.set(&mut c);
    let mut top = Frame::empty(c.bottom());
    c.frame = &mut top.h;
    builtins::boot(&mut c);

    let vm = vm();
    vm.verbose.set(if o.warn { TRUE } else if o.nowarn { NIL } else { FALSE });
    // -e: prism's command-line E bit (no unused-variable warnings at its top level)
    let cmdline = o.cmdline | if o.e.is_empty() { 0 } else { 0x2 };
    if cmdline != 0 {
        prism::COMMAND_LINE.store(cmdline, std::sync::atomic::Ordering::Relaxed);
        if o.cmdline & 0x18 != 0 { builtins::kernel::define_dash_n(&mut c); }
    }
    if o.warn { builtins::misc::warning_enable_deprecated(); }
    if o.enc_ext.is_some() || o.enc_int.is_some() || o.uflag {
        let i = if o.uflag && o.enc_int.is_none() { Some("UTF-8") } else { o.enc_int.as_deref() };
        if let Err(m) = builtins::encoding::set_cmdline_encodings(o.enc_ext.as_deref(), i) { cli_fail(&m); }
        // rb_stdio_set_default_encoding: the standard streams take the defaults as they now are
        for io in [vm.stdin.get(), vm.stdout.get(), vm.stderr.get()] { let _ = funcall(&mut c, io, symbol::intern(b"set_encoding"), &[NIL], std::ptr::null()); }
    }
    if let Some(x) = &o.inplace { let s = c.str_new(x.as_bytes()); vm.globals.borrow_mut().insert(crate::symbol::intern(b"$-i"), s); }
    for d in &o.incdirs { let s = c.str_new(d.as_bytes()); c.ary_push(vm.load_path.get(), s); }
    builtins::prelude::setup_load_path(&mut c);
    builtins::did_you_mean::snapshot_load_path(&mut c);
    for a in &o.args { let s = builtins::encoding::external_str(&mut c, a.clone(), builtins::encoding::default_external()); c.ary_push(vm.argv.get(), s); }
    aot::init(if o.plain { None } else { Some(&o.store) }, o.aot);
    vm.aot_collect.set(o.aot);

    let (src, file): (Vec<u8>, String) = if !o.e.is_empty() {
        // each -e line ends with a newline (ruby.c)
        (o.e.iter().flat_map(|l| l.iter().copied().chain([b'\n'])).collect(), "-e".into())
    } else if let Some(s) = o.script.as_ref().filter(|s| s.as_str() != "-") {
        match std::fs::read(s) {
            Ok(b) => { builtins::exception::note_realpath(s); (b, s.clone()) }
            Err(_) => { eprintln!("saruby: No such file or directory -- {} (LoadError)", s); return 1; }
        }
    } else {
        let mut b = vec![];
        use std::io::Read;
        let _ = std::io::stdin().read_to_end(&mut b);
        (b, "-".into())
    };
    *vm.main_file.borrow_mut() = file.clone();
    let z = c.str_new_enc(file.as_bytes(), builtins::encoding::locale_enc()); heap::obj_set_flags(z, object::FL_FROZEN); vm.dollar_zero.set(z);
    let file: &'static str = Box::leak(file.into_boxed_str());
    compile::set_file_enc(file, builtins::encoding::locale_enc());

    inline_builtin::init(&c);
    // prelude (Ruby-level built-ins)
    if let Err(_) = builtins::prelude::load_prelude(&mut c) {
        let e = c.esc_val;
        report_exception(&mut c, e);
        return 1;
    }
    rt::BOOTED.store(true, std::sync::atomic::Ordering::Relaxed);
    gc::booted();

    // DATA / __END__
    // -e is read in the locale encoding (a script file in UTF-8 unless its magic comment says otherwise)
    let src_enc = if o.e.is_empty() { object::ENC_UTF8 } else { builtins::encoding::locale_enc() };
    compile::CHECK_ONLY.store(o.check_syntax, std::sync::atomic::Ordering::Relaxed);
    let is = compile::compile_source_enc(&src, src_enc, file, 1, IseqKind::Main, vec![]);
    compile::CHECK_ONLY.store(false, std::sync::atomic::Ordering::Relaxed);
    let _ = compile::flush_parse_warnings(&mut c);
    let is = match is {
        Ok(is) => is,
        Err(msg) => {
            // -r libraries are loaded before the main script is parsed: their at_exit handlers still run
            for r in &o.requires {
                let s = c.str_new(r.as_bytes());
                if builtins::kernel::require_feature(&mut c, s, false).is_err() { let e = c.esc_val; report_exception(&mut c, e); return 1; }
            }
            let e = vm::compile_error_exc(&mut c, msg, file);
            // -c reports at the program's name (the error is raised before ruby_set_script_name)
            if o.check_syntax { *vm.main_file.borrow_mut() = program_args().first().map(|a| String::from_utf8_lossy(a).into_owned()).unwrap_or_default(); builtins::syntax_suggest::disable(); }
            report_exception(&mut c, e);
            if !o.requires.is_empty() { finish(&mut c, false); }
            return 1;
        }
    };
    if o.check_syntax { println!("Syntax OK"); return 0; }
    builtins::io::setup_data(&mut c, &src, file);
    if o.dump_ast {
        let mut out = String::new();
        node::dump_node(&mut out, is.body);
        println!("{}", out);
        return 0;
    }
    for r in &o.requires {
        let s = c.str_new(r.as_bytes());
        if builtins::kernel::require_feature(&mut c, s, false).is_err() { let e = c.esc_val; report_exception(&mut c, e); return 1; }
    }
    let r = run_top_iseq(&mut c, is, vm.main_obj.get());
    let status = finish(&mut c, r.is_err());
    #[cfg(feature = "nodeprof")] nodeprof::report();
    if o.aot {
        match aot::bake(&o.store) {
            Ok(n) => if o.verbose_astro || !o.quiet { eprintln!("saruby: baked {} new items into {}/all.so", n, o.store); },
            Err(e) => { eprintln!("saruby: AOT bake failed:\n{}", e); }
        }
    }
    if o.verbose_astro {
        let st = gc::stats();
        eprintln!("saruby: sd_hits={} gc={} gc_count={} gc_time={:?} heap_slots={} allocated={}", vm.sd_hits.get(), gc::backend_name(), gc::collections(), gc::total_time(), st.slots, st.allocated);
    }
    status
}

thread_local! {
    /// The error that ended the main script while the exit handlers run, and whether a handler error showed it as its cause.
    static ORIG_ERR: std::cell::Cell<(Value, bool)> = const { std::cell::Cell::new((NIL, false)) };
}
/// rb_ec_cleanup: the EXIT trap, at_exit procs (LIFO; each error reported as it happens), the trap's error,
/// then the main error (`errored`: c.esc holds it) last, unless a handler error showed it as its cause;
/// a later error's status wins.
pub fn finish(c: &mut Ctx, errored: bool) -> i32 {
    let vm = vm();
    let take = |c: &mut Ctx| { let s = (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v); c.esc = EscKind::None; s };
    let put = |c: &mut Ctx, s: (EscKind, Value, u64, Value)| { c.esc = s.0; c.esc_val = s.1; c.esc_tag = s.2; c.esc_tag_v = s.3; };
    let main_err = errored.then(|| take(c));
    if let Some((EscKind::Raise, e, _, _)) = main_err { c.errinfo = e; ORIG_ERR.set((e, false)); }
    let mut later: Option<i32> = None;
    // the main thread's scheduler runs to completion first (rb_ec_fiber_scheduler_finalize)
    if builtins::thread::scheduler_finalize(c).is_err() { later = Some(handle_toplevel_error(c)); }
    let trap_err = builtins::process::run_exit_trap(c).is_err().then(|| take(c));
    loop {
        let ae = vm.at_exit.get();
        let Some(p) = c.ary_mut(ae).pop() else { break };
        c.esc = EscKind::None;
        if call_proc(c, p, &[], std::ptr::null()).is_err() {
            // the next handlers see this error in $! (rb_ec_exec_end_proc)
            let raised = (c.esc == EscKind::Raise).then_some(c.esc_val);
            if let Some(e) = raised { let o = ORIG_ERR.get(); if o.0 != NIL && ivar_get(c, e, symbol::s::cause) == o.0 && !is_kind_of(c, e, c.vm().c_system_exit.get()) { ORIG_ERR.set((o.0, true)); } }
            later = Some(handle_toplevel_error(c));
            if let Some(e) = raised { c.errinfo = e; }
        }
    }
    let trapped = trap_err.is_some();
    if let Some(t) = trap_err { put(c, t); later = Some(handle_toplevel_error(c)); }
    builtins::thread::terminate_all(c);
    if !trapped { builtins::misc::run_finalizers(c); }
    let mut status = later.unwrap_or(0);
    if let Some(m) = main_err {
        put(c, m);
        let st = if ORIG_ERR.get().1 { 1 } else { handle_toplevel_error(c) };
        if later.is_none() { status = st; }
    }
    ORIG_ERR.set((NIL, false));
    builtins::kernel::out_flush();
    builtins::io::flush_all(c);
    builtins::process::die_if_signaled();
    status
}

fn handle_toplevel_error(c: &mut Ctx) -> i32 {
    match c.esc {
        EscKind::Raise => {
            let e = c.esc_val;
            c.esc = EscKind::None;
            if is_kind_of(c, e, vm().c_system_exit.get()) {
                let st = ivar_get(c, e, symbol::s::status);
                return if st.is_fixnum() { st.as_fix() as i32 } else { 0 };
            }
            if is_kind_of(c, e, vm().c_signal_exception.get()) || ivar_get(c, e, symbol::intern(b"signo")).is_fixnum() {
                let no = ivar_get(c, e, symbol::intern(b"signo"));
                if no.is_fixnum() {
                    builtins::process::die_by_signal_at_exit(no.as_fix() as i32);
                    // a signal whose default action does not end the process (CHLD, WINCH): exit 1, as CRuby
                    if !is_kind_of(c, e, vm().c_interrupt.get()) { return 1; }
                }
            }
            report_exception(c, e);
            1
        }
        EscKind::Throw => {
            let tag = c.esc_tag_v;
            let s = builtins::kernel::inspect_str(c, tag).unwrap_or_default();
            builtins::kernel::err_write(format!("uncaught throw {} (UncaughtThrowError)\n", s).as_bytes());
            1
        }
        EscKind::Break | EscKind::Next | EscKind::Return | EscKind::LoopBreak | EscKind::LoopNext => {
            c.esc = EscKind::None;
            0
        }
        _ => { c.esc = EscKind::None; 1 }
    }
}

/// The main thread's stack size (fiber.rs scans it while a fiber runs).
pub const STACK_BYTES: usize = 1 << 30;

/// ruby.c's fill_standard_fds: a closed fd 0 reads EOF and a closed 1 / 2 fails with EPIPE (a pipe with the other
/// end closed).  Rust's runtime has already put a read-write /dev/null there; that is what is replaced.
fn fill_standard_fds() {
    use builtins::sys;
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let filled = |fd: i32| sys::fcntl(fd, sys::F_GETFL, 0) & sys::O_ACCMODE == sys::O_RDWR
        && sys::fd_metadata(fd).is_ok_and(|m| m.file_type().is_char_device() && m.rdev() == 0x103);
    let (f0, f1, f2) = (filled(0), filled(1), filled(2));
    let mut p = [0i32; 2];
    if f0 && sys::pipe2(&mut p, 0) == 0 { sys::close(p[1]); sys::dup2(p[0], 0); sys::close(p[0]); }
    if (f1 || f2) && sys::pipe2(&mut p, 0) == 0 {
        sys::close(p[0]);
        if f1 { sys::dup2(p[1], 1); }
        if f2 { sys::dup2(p[1], 2); }
        sys::close(p[1]);
    }
}

pub fn main_entry() {
    kalloc::pad_system();
    builtins::sys::install_fatal_signals();
    let o = parse_args();
    fill_standard_fds();
    // CRuby's main: setlocale(LC_CTYPE, "") before any thread starts
    builtins::encoding::init_locale();
    symbol::init();
    fiber_os::nil_fd_init();
    let child = std::thread::Builder::new().stack_size(STACK_BYTES).spawn(move || {
        kalloc::adopt();
        let base = 0usize;
        gc::init(&base as *const usize as usize + 8);
        builtins::boot::install();
        let st = run_main(o);
        builtins::kernel::out_flush();
        std::process::exit(st);
    }).unwrap();
    builtins::process::block_signals_in_this_thread();
    // a panic of the interpreter thread is a crash, not exit status 0
    if child.join().is_err() { std::process::abort(); }
}
