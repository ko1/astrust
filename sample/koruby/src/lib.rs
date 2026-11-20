// koruby — a Ruby implementation in Rust on ASTrust (AST interpreter + partial
// evaluation to Rust, compiled by rustc and loaded with dlopen).
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code, static_mut_refs)]
#![allow(clippy::all)]

#[path = "../../../runtime/astrust.rs"]
pub mod astrust;
#[path = "../../../runtime/gc/mod.rs"]
pub mod astrust_gc;

pub mod prism;
pub mod prism_ast;
pub mod fxhash;
pub mod value;
pub mod symbol;
pub mod sbuf;
pub mod ivbuf;
pub mod object;
pub mod gc;
pub mod kalloc;
pub mod slots;
#[cfg(feature = "safe_stack")]
pub mod stack;
#[cfg(not(feature = "safe_stack"))]
#[path = "stack_raw.rs"]
pub mod stack;
pub mod vm;
pub mod compile;
pub mod rt;
pub mod opt;
pub mod bignum;
pub mod string;
pub mod array;
pub mod hash;
pub mod range;
pub mod numeric;
pub mod regexp;
pub mod regex_engine;
pub mod fiber;
pub mod aot;
pub mod builtins;

#[allow(unused_imports)]
pub mod node {
    use crate::astrust::*;
    use crate::compile::{IseqRef, CiRef, Str, LitCache};
    use crate::value::*;
    use crate::vm::*;
    include!("node_head.rs");
    include!("node_dispatch.rs");
    include!("node_hash.rs");
    include!("node_specialize.rs");
    include!("node_dump.rs");
    include!("node_replace.rs");
    #[inline(always)]
    pub fn node_allocated(_n: NodePtr) {}
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
    run_top_iseq_cref(c, is, slf, vm().top_cref)
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
    fr.line = 1;
    base[0].set(slf);
    base[1].set(NIL);
    nil_fill(base, 2, is.nlocals as usize);
    c.set_sp(base, is.nlocals as usize);
    if is.env_size > 0 {
        let env = gc::alloc(vm().c_env, object::Body::Env(vec![NIL; is.env_size as usize], NIL));
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
    e: Vec<String>,
    script: Option<String>,
    args: Vec<String>,
    plain: bool,
    aot: bool,
    dump_ast: bool,
    quiet: bool,
    verbose_astro: bool,
    warn: bool,
    store: String,
    compiled_only: bool,
    requires: Vec<String>,
    incdirs: Vec<String>,
    check_syntax: bool,
}

fn usage() -> ! {
    eprintln!("usage: koruby [options] [--] [script.rb] [args...]\n\
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

fn parse_args() -> Opts {
    let argv: Vec<String> = std::env::args().collect();
    let mut o = Opts { e: vec![], script: None, args: vec![], plain: false, aot: false, dump_ast: false, quiet: false,
        verbose_astro: false, warn: false, store: "code_store".into(), compiled_only: false, requires: vec![], incdirs: vec![], check_syntax: false };
    let mut i = 1;
    while i < argv.len() {
        let a = &argv[i];
        if o.script.is_some() || (!o.e.is_empty() && !a.starts_with('-')) { o.args.push(a.clone()); i += 1; continue; }
        match a.as_str() {
            "-e" => { i += 1; o.e.push(argv.get(i).cloned().unwrap_or_else(|| usage())); }
            "--plain" => o.plain = true,
            "--aot-compile" | "--pg-compile" => o.aot = true,
            "--compiled-only" => o.compiled_only = true,
            "--dump-ast" => o.dump_ast = true,
            "-q" | "--quiet" => o.quiet = true,
            "--astro-verbose" => o.verbose_astro = true,
            "-w" | "-W" | "-W2" | "-v" | "--verbose" => { o.warn = true; if a == "-v" { println!("{}", version()); } }
            "-W0" => {}
            "-c" => o.check_syntax = true,
            "--code-store" => { i += 1; o.store = argv.get(i).cloned().unwrap_or_else(|| usage()); }
            "--version" => { println!("{}", version()); std::process::exit(0); }
            "-h" | "--help" => usage(),
            "--" => { i += 1; if i < argv.len() && o.e.is_empty() { o.script = Some(argv[i].clone()); } else if i < argv.len() { o.args.push(argv[i].clone()); } }
            "--disable-gems" | "--disable=gems" | "--enable-frozen-string-literal" | "--yjit" | "--disable-all" => {}
            s if s.starts_with("-I") => { if s.len() > 2 { o.incdirs.push(s[2..].into()); } else { i += 1; o.incdirs.push(argv.get(i).cloned().unwrap_or_else(|| usage())); } }
            s if s.starts_with("-r") => { if s.len() > 2 { o.requires.push(s[2..].into()); } else { i += 1; o.requires.push(argv.get(i).cloned().unwrap_or_else(|| usage())); } }
            s if s.starts_with("--disable") || s.starts_with("--enable") => {}
            s if s.starts_with('-') && s.len() > 1 => { eprintln!("koruby: invalid option {}  (-h will show valid options)", s); std::process::exit(2); }
            _ => { if o.e.is_empty() { o.script = Some(a.clone()); } else { o.args.push(a.clone()); } }
        }
        i += 1;
    }
    o
}

pub fn version() -> String { format!("koruby 0.1.0 (ASTrust; Ruby 3.4 compatible subset) [{}]", env!("KORUBY_TARGET")) }

/// Print an uncaught exception CRuby-style.
pub fn report_exception(c: &mut Ctx, e: Value) {
    builtins::kernel::out_flush();
    let msg = builtins::exception::full_message(c, e, true).unwrap_or_else(|_| "exception\n".into());
    builtins::kernel::err_write(msg.as_bytes());
}

fn run_main(o: Opts) -> i32 {
    let mut c = Ctx {
        sp: NO_SP, stk: NO_STACK, frame: std::ptr::null_mut(),
        esc: EscKind::None, esc_val: NIL, esc_tag: 0, esc_tag_v: NIL, serial: 0, errinfo: NIL, last_match: NIL,
        native_limit: 0, in_ensure: 0, call_me: std::ptr::null(), call_blk: std::ptr::null(),
    };
    c.attach_stack(stack_new(8 << 20));
    let probe = 0u8;
    c.native_limit = (&probe as *const u8 as usize).saturating_sub(STACK_BYTES - (4 << 20));
    unsafe { MAIN_CTX = &mut c; }
    let mut top = Frame::empty(c.bottom());
    c.frame = &mut top.h;

    let vm = vm();
    vm.verbose = if o.warn { TRUE } else { FALSE };
    for d in &o.incdirs { let s = string::str_new(d.as_bytes()); array::ary_push(vm.load_path, s); }
    builtins::prelude::setup_load_path();
    for a in &o.args { let s = string::str_new(a.as_bytes()); array::ary_push(vm.argv, s); }
    aot::init(if o.plain { None } else { Some(&o.store) }, o.aot);
    vm.aot_collect = o.aot;

    let (src, file): (Vec<u8>, String) = if !o.e.is_empty() {
        (o.e.join("\n").into_bytes(), "-e".into())
    } else if let Some(s) = o.script.as_ref().filter(|s| s.as_str() != "-") {
        match std::fs::read(s) {
            Ok(b) => (b, s.clone()),
            Err(_) => { eprintln!("koruby: No such file or directory -- {} (LoadError)", s); return 1; }
        }
    } else {
        let mut b = vec![];
        use std::io::Read;
        let _ = std::io::stdin().read_to_end(&mut b);
        (b, "-".into())
    };
    vm.main_file = file.clone();
    vm.dollar_zero = string::str_new(file.as_bytes());
    let file: &'static str = Box::leak(file.into_boxed_str());

    // prelude (Ruby-level built-ins)
    if let Err(_) = builtins::prelude::load_prelude(&mut c) {
        let e = c.esc_val;
        report_exception(&mut c, e);
        return 1;
    }
    unsafe { rt::BOOTED = true; }
    gc::booted();

    // DATA / __END__
    let is = match compile::compile_source(&src, file, 1, IseqKind::Main, vec![]) {
        Ok(is) => is,
        Err(msg) => {
            builtins::kernel::err_write(format!("{}\n", msg).as_bytes());
            builtins::kernel::err_write(format!("{}: compile error (SyntaxError)\n", file).as_bytes());
            return 1;
        }
    };
    if o.check_syntax { println!("Syntax OK"); return 0; }
    builtins::io::setup_data(&src, file);
    if o.dump_ast {
        let mut out = String::new();
        node::dump_node(&mut out, is.body);
        println!("{}", out);
        return 0;
    }
    let mut status = 0;
    for r in &o.requires {
        let s = string::str_new(r.as_bytes());
        if builtins::kernel::require_feature(&mut c, s, false).is_err() { let e = c.esc_val; report_exception(&mut c, e); return 1; }
    }
    let r = run_top_iseq(&mut c, is, vm.main_obj);
    if r.is_err() { status = handle_toplevel_error(&mut c); }
    status = run_exit_handlers(&mut c, status);
    if o.aot {
        match aot::bake(&o.store) {
            Ok(n) => if o.verbose_astro || !o.quiet { eprintln!("koruby: baked {} new items into {}/all.so", n, o.store); },
            Err(e) => { eprintln!("koruby: AOT bake failed:\n{}", e); }
        }
    }
    if o.verbose_astro {
        let h = gc::heap();
        let st = h.stats();
        eprintln!("koruby: sd_hits={} gc={} gc_count={} gc_time={:?} heap_slots={} allocated={}", vm.sd_hits, h.backend_name(), h.ctl.collections(), h.ctl.time(), st.slots, st.allocated);
    }
    status
}

/// at_exit procs (LIFO), the EXIT trap, then flush every stream.
pub fn run_exit_handlers(c: &mut Ctx, mut status: i32) -> i32 {
    let vm = vm();
    loop {
        let ae = vm.at_exit;
        let Some(p) = array::rary(ae).pop() else { break };
        c.esc = EscKind::None;
        if call_proc(c, p, &[], std::ptr::null()).is_err() { status = handle_toplevel_error(c); }
    }
    if builtins::process::run_exit_trap(c).is_err() { status = handle_toplevel_error(c); }
    builtins::kernel::out_flush();
    builtins::io::flush_all();
    builtins::process::die_if_signaled();
    status
}
/// End of a forked child's block: report an error, run the exit handlers.
pub fn finish(c: &mut Ctx, errored: bool) -> i32 {
    let st = if errored { handle_toplevel_error(c) } else { 0 };
    run_exit_handlers(c, st)
}

fn handle_toplevel_error(c: &mut Ctx) -> i32 {
    match c.esc {
        EscKind::Raise => {
            let e = c.esc_val;
            c.esc = EscKind::None;
            if is_kind_of(e, vm().c_system_exit) {
                let st = ivar_get(e, symbol::s::status);
                return if st.is_fixnum() { st.as_fix() as i32 } else { 0 };
            }
            if is_kind_of(e, vm().c_signal_exception) {
                let no = ivar_get(e, symbol::intern(b"signo"));
                if no.is_fixnum() {
                    builtins::process::die_by_signal_at_exit(no.as_fix() as i32);
                    if !is_kind_of(e, vm().c_interrupt) { return 128 + no.as_fix() as i32; }
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

const STACK_BYTES: usize = 1 << 30;

pub fn main_entry() {
    let o = parse_args();
    symbol::init();
    let child = std::thread::Builder::new().stack_size(STACK_BYTES).spawn(move || {
        kalloc::adopt();
        let base = 0usize;
        gc::init(&base as *const usize as usize + 8);
        builtins::boot();
        let st = run_main(o);
        builtins::kernel::out_flush();
        std::process::exit(st);
    }).unwrap();
    builtins::process::block_signals_in_this_thread();
    let _ = child.join();
}
