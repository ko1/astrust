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
    if e == 32 { ivar_set_raw(c.esc_val, intern(b"signo"), Value::fix(13)); }
    Err(r)
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
    if so == c.vm().stdout.get() { out_write(b); return out_check(c); }
    if so == c.vm().stderr.get() { err_write(b); return Ok(()); }
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
    let me = search_method(singleton_class(w), intern(b"warn"));
    if crate::builtins::proc::method_arity(c, me) == 1 { return warning_write(c, b); }
    let st = c.str_new(b);
    let h = c.hash_new();
    hash_set(c, h, sym_value("category"), cat)?;
    funcall_kw(c, w, intern(b"warn"), &[st, h], std::ptr::null(), true)?;
    Ok(())
}

// ---- string conversion helpers ------------------------------------------------------------

pub fn any_to_s_str(v: Value) -> String {
    let k = real_class_of(v);
    format!("#<{}:{:#018x}>", class_path(k), v.raw())
}

pub fn inspect_str(c: &mut Ctx, v: Value) -> Result<String, Esc> {
    let r = inspect_value(c, v)?;
    Ok(String::from_utf8_lossy(c.str_bytes(r)).into_owned())
}

pub fn inspect_value(c: &mut Ctx, v: Value) -> RV {
    if v == NIL { return Ok(c.str_new(b"nil")); }
    if v.is_fixnum() && !crate::rt::redef(0) { return Ok(c.str_new(v.as_fix().to_string().as_bytes())); }
    let r = funcall(c, v, s::inspect, &[], std::ptr::null())?;
    if is_string(r) { Ok(r) } else { Ok(c.str_new(any_to_s_str(v).as_bytes())) }
}

pub fn to_s_value(c: &mut Ctx, v: Value) -> RV { crate::rt::srb_tostr(c, v) }

pub fn nomethod_desc(c: &mut Ctx, v: Value) -> Result<String, Esc> {
    let _ = c;
    if v == NIL { return Ok("nil".into()); }
    if v == TRUE { return Ok("true".into()); }
    if v == FALSE { return Ok("false".into()); }
    if v == c.vm().main_obj.get() { return Ok("main".into()); }
    if is_class_or_module(v) {
        let cd = c.class(v);
        return Ok(format!("{} {}", if cd.is_module { "module" } else { "class" }, class_path(v)));
    }
    Ok(format!("an instance of {}", class_path(real_class_of(v))))
}

pub fn class_name_for_err(v: Value) -> String {
    if v == NIL { return "nil".into(); }
    if v == TRUE { return "true".into(); }
    if v == FALSE { return "false".into(); }
    class_path(real_class_of(v))
}

/// rb_respond_to: a redefined respond_to? is asked; Kernel#respond_to? answers from the
/// method table (public methods), then respond_to_missing?.
pub fn rb_respond_to(c: &mut Ctx, v: Value, mid: Sym) -> Result<bool, Esc> {
    let basic = search_method(c.vm().m_kernel.get(), s::respond_to_p);
    if lookup_method(class_of(v), s::respond_to_p) != basic {
        return Ok(funcall(c, v, s::respond_to_p, &[Value::sym(mid)], std::ptr::null())?.truthy());
    }
    if respond_to(v, mid, false) { return Ok(true); }
    if lookup_method(class_of(v), s::respond_to_missing_p) != search_method(c.vm().m_kernel.get(), s::respond_to_missing_p) {
        return Ok(funcall(c, v, s::respond_to_missing_p, &[Value::sym(mid), FALSE], std::ptr::null())?.truthy());
    }
    Ok(false)
}

// ---- recursion guard (inspect / hash / == of recursive structures) -------------------------

// context-free (legacy.rs): converted modules of earlier waves call them without a context
pub use crate::legacy::{recursion_enter, recursion_leave};

// ---- puts / print / p -------------------------------------------------------------------

fn puts_one(c: &mut Ctx, v: Value, out: &mut Vec<u8>) -> Result<(), Esc> {
    if is_string(v) {
        out.extend_from_slice(&c.str_bytes(v));
        if !c.str_bytes(v).ends_with(b"\n") { out.push(b'\n'); }
        return Ok(());
    }
    if is_array(v) {
        if !recursion_enter(v, s::puts) { out.extend_from_slice(b"[...]\n"); return Ok(()); }
        let n = c.ary_len(v);
        for i in 0..n { let x = c.ary_get(v, i); let r = puts_one(c, x, out); if r.is_err() { recursion_leave(v, s::puts); return r; } }
        recursion_leave(v, s::puts);
        return Ok(());
    }
    if v.is_heap() && !is_string(v) {
        if let Some(a) = crate::array::check_array(c, v)? { return puts_one(c, a, out); }
    }
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

fn k_puts(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let so = *c.vm().globals.borrow().get(&intern(b"$stdout")).unwrap_or(&c.vm().stdout.get());
    if so != c.vm().stdout.get() && so != c.vm().stderr.get() {
        return funcall(c, so, s::puts, args, std::ptr::null());
    }
    let out = puts_bytes(c, args)?;
    if so == c.vm().stderr.get() { err_write(&out); } else { out_write(&out); out_check(c)?; }
    Ok(NIL)
}

fn k_print(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let mut out = vec![];
    for &a in args { let s = to_s_value(c, a)?; out.extend_from_slice(&c.str_bytes(s)); }
    stdout_write(c, &out)?;
    Ok(NIL)
}

fn k_p(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let mut out = vec![];
    for &a in args { let s = inspect_value(c, a)?; out.extend_from_slice(&c.str_bytes(s)); out.push(b'\n'); }
    stdout_write(c, &out)?;
    Ok(match args.len() { 0 => NIL, 1 => args[0], _ => c.ary_new_from(args) })
}

fn k_pp(c: &mut Ctx, s: Value, args: &[Value], b: *const Block) -> RV {
    if args.len() == 1 {
        let txt = crate::builtins::pp::pretty_inspect(c, args[0], 80)?;
        stdout_write(c, txt.as_bytes())?;
        return Ok(args[0]);
    }
    k_p(c, s, args, b)
}

fn k_putc(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    check_argc(c, args, 1, 1)?;
    let v = args[0];
    let b = if is_string(v) { vec![*c.str_bytes(v).first().unwrap_or(&0)] } else { vec![(crate::builtins::numeric_methods::num_to_long(c, v)? & 0xff) as u8] };
    stdout_write(c, &b)?;
    Ok(v)
}

fn k_printf(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
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

fn k_sprintf(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    if args.is_empty() { return argc_err(c, 0, 1, -1); }
    crate::builtins::sprintf::format(c, args[0], &args[1..])
}

fn k_warn(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let (args, kw) = match kw_arg(args) { Some(h) => (&args[..args.len() - 1], Some(h)), None => (args, None) };
    let mut uplevel = None;
    let mut cat = NIL;
    if let Some(h) = kw {
        if let Some(u) = hash_get_opt(c, h, crate::vm::sym_value("uplevel"))? { if u != NIL { uplevel = Some(u.as_fix()); } }
        if let Some(k) = hash_get_opt(c, h, crate::vm::sym_value("category"))? { cat = k; }
    }
    // warning.rb: a category is a Symbol (or String) and the warning needs Warning[category]
    if cat != NIL {
        if is_string(cat) { cat = c.str_to_sym(cat); }
        if !cat.is_sym() { return type_err(c, format!("no implicit conversion of {} into Symbol", class_name_for_err(cat))); }
        let i = crate::builtins::misc::warning_category(c, cat)?;
        if !crate::builtins::misc::warning_category_enabled(i) { return Ok(NIL); }
    }
    if c.vm().verbose.get() == NIL { return Ok(NIL); }
    let mut out = puts_bytes(c, if args.is_empty() { return Ok(NIL) } else { args })?;
    if let Some(u) = uplevel {
        let lines = backtrace_lines(c, 1 + u as usize);
        if let Some(l) = lines.first() {
            let loc = l.split(":in ").next().unwrap_or("").to_string();
            let mut pre = format!("{}: warning: ", loc).into_bytes();
            pre.extend(out); out = pre;
        }
    }
    warning_write_cat(c, &out, cat)?;
    Ok(NIL)
}

// ---- raise ------------------------------------------------------------------------------

pub fn make_exception_from_args(c: &mut Ctx, args: &[Value]) -> RV {
    let (args, kw) = match kw_arg(args) { Some(h) => (&args[..args.len() - 1], Some(h)), None => (args, None) };
    let e = match args.len() {
        0 => {
            if c.errinfo != NIL { c.errinfo }
            else { make_exception(c.vm().c_runtime_error.get(), "unhandled exception") }
        }
        1 if is_string(args[0]) => {
            let e = new_object(c.vm().c_runtime_error.get());
            ivar_set_raw(e, s::mesg, args[0]);
            e
        }
        _ => {
            let k = args[0];
            if !respond_to(k, s::exception, true) { return type_err(c, "exception class/object expected"); }
            let e = funcall(c, k, s::exception, &args[1..args.len().min(2)], std::ptr::null())?;
            if !is_exception(e) { return type_err(c, "exception object expected"); }
            if args.len() >= 3 {
                let bt = if is_string(args[2]) { c.ary_new_from(&[args[2]]) } else { args[2] };
                ivar_set_raw(e, s::bt, bt);
            }
            e
        }
    };
    if let Some(h) = kw {
        if let Some(cause) = hash_get_opt(c, h, crate::vm::sym_value("cause"))? {
            if cause != NIL && !is_exception(cause) { return type_err(c, "exception object expected"); }
            ivar_set_raw(e, s::cause, cause);
        } else if args.is_empty() {
            return arg_err(c, "only cause is given with no arguments");
        }
    }
    Ok(e)
}

fn k_raise(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let e = make_exception_from_args(c, args)?;
    // the backtrace starts at the caller of raise
    if ivar_get(e, s::bt) == NIL {
        let bt = make_backtrace_lazy(c, 1);
        ivar_set_raw(e, s::bt, bt);
    }
    Err(raise_exc(c, e))
}

// ---- control ------------------------------------------------------------------------------

fn k_loop(c: &mut Ctx, slf: Value, _args: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, s::loop_, &[], NIL); }
    loop {
        match yield_block(c, blk, &[]) {
            Ok(_) => {}
            Err(e) => {
                if c.esc == EscKind::Raise && is_kind_of(c.esc_val, c.vm().c_stop_iteration.get()) {
                    let ex = c.esc_val;
                    c.esc = EscKind::None;
                    return Ok(ivar_get(ex, intern(b"result")));
                }
                return Err(e);
            }
        }
    }
}

thread_local! { static CATCH_TAGS: std::cell::RefCell<Vec<Value>> = const { std::cell::RefCell::new(vec![]) }; }

fn k_catch(c: &mut Ctx, _s: Value, args: &[Value], blk: *const Block) -> RV {
    let tag = if args.is_empty() { new_object(c.vm().c_object.get()) } else { args[0] };
    CATCH_TAGS.with(|t| t.borrow_mut().push(tag));
    let r = yield_block(c, blk, &[tag]);
    CATCH_TAGS.with(|t| t.borrow_mut().pop());
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

fn k_throw(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    check_argc(c, args, 1, 2)?;
    let tag = args[0];
    let val = if args.len() > 1 { args[1] } else { NIL };
    let found = CATCH_TAGS.with(|t| t.borrow().iter().any(|&x| x == tag));
    if !found {
        let ins = inspect_str(c, tag)?;
        let e = make_exception(c.vm().c_uncaught_throw_error.get(), &format!("uncaught throw {}", ins));
        ivar_set_raw(e, intern(b"tag"), tag);
        ivar_set_raw(e, intern(b"value"), val);
        return Err(raise_exc(c, e));
    }
    c.esc = EscKind::Throw;
    c.esc_val = val;
    c.esc_tag_v = tag;
    Err(Esc)
}

fn k_block_given(c: &mut Ctx, _s: Value, _args: &[Value], _b: *const Block) -> RV {
    // the cfunc frame's caller
    Ok(Value::bool(c.caller_frame().is_some_and(|p| !p.blk.is_null())))
}

fn k_lambda(c: &mut Ctx, _s: Value, _args: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
    let p = block_to_proc(c, blk)?;
    Ok(p)
}

fn k_proc(c: &mut Ctx, _s: Value, _args: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
    block_to_proc(c, blk)
}

fn k_method_name(c: &mut Ctx, _s: Value, _args: &[Value], _b: *const Block) -> RV {
    let me = c.caller_frame().map_or(std::ptr::null(), |p| p.me);
    Ok(c.me_ref(me).map_or(NIL, |m| Value::sym(m.name)))
}

fn k_callee_name(c: &mut Ctx, _s: Value, _args: &[Value], _b: *const Block) -> RV {
    let me = c.caller_frame().map_or(std::ptr::null(), |p| p.me);
    if me.is_null() { return Ok(NIL); }
    Ok(Value::sym(crate::builtins::object::callee_name(c, me)))
}

fn k_caller(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let (start, len) = crate::builtins::exception::caller_range(c, args)?;
    let Some(lines) = backtrace_lines_n(c, 1 + start, len) else { return Ok(NIL) };
    Ok(c.ary_of_strs(lines.into_iter().map(|l| l.into_bytes()).collect(), ENC_UTF8))
}

pub fn k_exit(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let st = match args.first() { None => 0, Some(&v) if v == TRUE => 0, Some(&v) if v == FALSE => 1, Some(&v) => crate::builtins::numeric_methods::num_to_long(c, v)? };
    let e = make_exception(c.vm().c_system_exit.get(), "exit");
    ivar_set_raw(e, s::status, Value::fix(st));
    Err(raise_exc(c, e))
}

fn k_exit_bang(_c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let st = match args.first() { None => 0, Some(&v) if v == TRUE => 0, Some(&v) if v == FALSE => 1, Some(&v) => v.as_fix() };
    out_flush();
    std::process::exit(st as i32);
}

pub fn k_abort(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let msg = if let Some(&m) = args.first() { let m = expect_str(c, m)?; let mut b = c.str_bytes(m).to_vec(); b.push(b'\n'); err_write(&b); m } else { c.str_new(b"exit") };
    let e = new_object(c.vm().c_system_exit.get());
    ivar_set_raw(e, s::mesg, msg);
    ivar_set_raw(e, s::status, Value::fix(1));
    let _ = c;
    Err(raise_exc(c, e))
}

fn k_at_exit(c: &mut Ctx, _s: Value, _args: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return arg_err(c, "called without a block"); }
    let p = block_to_proc(c, blk)?;
    c.ary_push(c.vm().at_exit.get(), p);
    Ok(p)
}

/// ARGF (a lazily loaded prelude library).
pub fn argf(c: &mut Ctx) -> RV { const_lookup(c, c.vm().top_cref.get(), intern(b"ARGF")) }
fn argf_plain(c: &Ctx) -> bool { c.ary_len(c.vm().argv.get()) == 0 && const_get_at(c.vm().c_object.get(), intern(b"ARGF")).is_none() }
fn k_gets(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    out_flush();
    // no file arguments: plain stdin without loading ARGF
    let src = if argf_plain(c) { *c.vm().globals.borrow().get(&intern(b"$stdin")).unwrap_or(&c.vm().stdin.get()) } else { argf(c)? };
    let v = funcall(c, src, intern(b"gets"), args, std::ptr::null())?;
    c.vm().globals.borrow_mut().insert(intern(b"$_"), v);
    Ok(v)
}
fn k_readline(c: &mut Ctx, s: Value, args: &[Value], b: *const Block) -> RV {
    let v = k_gets(c, s, args, b)?;
    if v == NIL { return err(c, c.vm().c_eof_error.get(), "end of file reached"); }
    Ok(v)
}
fn k_readlines(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    out_flush();
    let src = if argf_plain(c) { *c.vm().globals.borrow().get(&intern(b"$stdin")).unwrap_or(&c.vm().stdin.get()) } else { argf(c)? };
    funcall(c, src, intern(b"readlines"), args, std::ptr::null())
}

/// Print a warning line unless $VERBOSE is nil.
pub fn warn_str(c: &mut Ctx, msg: &str) { if c.vm().verbose.get() != NIL { err_write(msg.as_bytes()); } }

fn k_rand(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    crate::builtins::random::rand_value(c, args.first().copied().unwrap_or(NIL))
}
fn k_srand(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let old = c.vm().rand_state.get();
    let seed = match args.first() { Some(&v) => crate::builtins::numeric_methods::num_to_long(c, v)? as u64, None => 0x2545F4914F6CDD1D };
    crate::builtins::random::seed(c, seed);
    Ok(c.int_from_i128(old as i128))
}

// ---- conversion functions ----------------------------------------------------------------

use crate::builtins::rational::kw_exception;

fn k_integer(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
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
            match str_to_inum(&b, base, true) {
                Some(x) => Ok(x),
                None => { let ins = inspect_str(c, v)?; arg_err(c, format!("invalid value for Integer(): {}", ins)) }
            }
        } else if args.len() > 1 && args[1] != NIL {
            arg_err(c, "base specified for non string value")
        } else if v.is_fixnum() || is_bignum(v) { Ok(v) }
        else if is_float(v) {
            let f = float_val(v);
            if f.is_nan() || f.is_infinite() { return err(c, c.vm().c_float_domain_error.get(), crate::numeric::float_to_s(f)); }
            Ok(crate::legacy::f64_to_int(f))
        } else if v == NIL { type_err(c, "can't convert nil into Integer") }
        else {
            if respond_to(v, s::to_int, true) { let r = funcall(c, v, s::to_int, &[], std::ptr::null())?; if is_integer(r) { return Ok(r); } }
            if respond_to(v, s::to_i, true) { let r = funcall(c, v, s::to_i, &[], std::ptr::null())?; if is_integer(r) { return Ok(r); } }
            type_err(c, format!("can't convert {} into Integer", class_name_for_err(v)))
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

fn k_float(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
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
        if respond_to(v, s::to_f, true) {
            let r = funcall(c, v, s::to_f, &[], std::ptr::null())?;
            if is_float(r) { return Ok(r); }
            let cn = class_name_for_err(v);
            return type_err(c, format!("can't convert {} to Float ({}#to_f gives {})", cn, cn, class_name_for_err(r)));
        }
        type_err(c, format!("can't convert {} into Float", class_name_for_err(v)))
    })();
    match r { Err(e) if !exc && c.esc == EscKind::Raise => { let _ = e; c.esc = EscKind::None; Ok(NIL) } r => r }
}

fn k_string(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    check_argc(c, args, 1, 1)?;
    let v = args[0];
    if is_string(v) { return Ok(v); }
    if v.is_heap() && respond_to(v, s::to_str, true) { return funcall(c, v, s::to_str, &[], std::ptr::null()); }
    if respond_to(v, s::to_s, true) {
        let r = funcall(c, v, s::to_s, &[], std::ptr::null())?;
        if !is_string(r) { return type_err(c, format!("can't convert {} to String ({}#to_s gives {})", class_name_for_err(v), class_name_for_err(v), class_name_for_err(r))); }
        return Ok(r);
    }
    type_err(c, format!("can't convert {} into String", class_name_for_err(v)))
}

fn k_array(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    check_argc(c, args, 1, 1)?;
    let v = args[0];
    if is_array(v) { return Ok(v); }
    if v == NIL { return Ok(c.ary_new()); }
    if let Some(a) = check_array(c, v)? { return Ok(a); }
    if respond_to(v, s::to_a, true) {
        let r = funcall(c, v, s::to_a, &[], std::ptr::null())?;
        if is_array(r) { return Ok(r); }
        if r != NIL { return type_err(c, format!("can't convert {} to Array ({}#to_a gives {})", class_name_for_err(v), class_name_for_err(v), class_name_for_err(r))); }
    }
    Ok(c.ary_new_from(&[v]))
}

fn k_hash_conv(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    check_argc(c, args, 1, 1)?;
    let v = args[0];
    if is_hash(v) { return Ok(v); }
    if v == NIL || (is_array(v) && c.ary_len(v) == 0) { return Ok(c.hash_new()); }
    if let Some(h) = check_hash(c, v)? { return Ok(h); }
    type_err(c, format!("can't convert {} into Hash", class_name_for_err(v)))
}

// ---- object identity / hash ----------------------------------------------------------------

pub fn obj_hash(_c: &mut Ctx, slf: Value, _args: &[Value], _b: *const Block) -> RV {
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
fn gvar_warn(c: &mut Ctx, msg: &str, deprecated: bool) -> Result<(), Esc> {
    if c.vm().verbose.get() == NIL || (deprecated && !crate::builtins::misc::warning_category_enabled(0)) { return Ok(()); }
    let loc = c.cur_frame().map(|f| format!("{}:{}: ", frame_file(f), f.line())).unwrap_or_default();
    let m = format!("{}warning: {}\n", loc, msg);
    if deprecated { warning_write_cat(c, m.as_bytes(), sym_value("deprecated")) } else { warning_write(c, m.as_bytes()) }
}

pub fn gvar_get(c: &mut Ctx, name: Sym) -> RV {
    let name = resolve_gvar(c, name);
    let b = sym_bytes(name);
    match b {
        b"$!" => return Ok(c.errinfo),
        b"$@" => { return Ok(if c.errinfo == NIL { NIL } else { exc_bt(c, c.errinfo) }); }
        b"$~" => return Ok(c.last_match),
        b"$&" => return Ok(crate::regexp::nth_match(c, c.last_match, 0)),
        b"$`" => { let m = c.last_match; return Ok(if m == NIL { NIL } else { crate::regexp::match_pre(c, m) }); }
        b"$'" => { let m = c.last_match; return Ok(if m == NIL { NIL } else { crate::regexp::match_post(c, m) }); }
        b"$+" => { let m = c.last_match; return Ok(if m == NIL { NIL } else { crate::regexp::match_last_group(c, m) }); }
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
    Ok(*c.vm().globals.borrow().get(&name).unwrap_or(&NIL))
}

/// $/ holds a plain frozen String: a subclass instance, an ivar holder or a mutable one is copied.
fn gvar_frozen_str(c: &mut Ctx, v: Value) -> Value {
    if is_frozen(v) && real_class_of(v) == c.vm().c_string.get() && ivar_list(v).is_empty() { return v; }
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
            if v != NIL && !(v.is_heap() && is_kind_of(v, c.vm().c_match_data.get())) {
                return type_err(c, format!("wrong argument type {} (expected MatchData)", class_name_for_err(v)));
            }
            c.last_match = v; return Ok(v);
        }
        b"$@" => {
            if c.errinfo == NIL { return arg_err(c, "$! not set"); }
            let e = c.errinfo;
            funcall(c, e, intern(b"set_backtrace"), &[v], std::ptr::null())?;
            return Ok(v);
        }
        b"$VERBOSE" => { c.vm().verbose.set(if v == NIL { NIL } else { Value::bool(v.truthy()) }); return Ok(v); }
        b"$DEBUG" => { c.vm().debug_flag.set(v.truthy()); crate::vm::update_raise_hooks(); }
        b"$0" => { let s = expect_str(c, v)?; c.vm().dollar_zero.set(s); return Ok(v); }
        b"$stdout" | b"$stderr" => {
            if !respond_to(v, intern(b"write"), true) {
                return type_err(c, format!("{} must have write method, {} given", sym_str(shown), class_path(real_class_of(v))));
            }
        }
        b"$/" | b"$\\" | b"$," | b"$;" => {
            if v != NIL {
                let re_ok = nb == b"$;" && crate::regexp::is_regexp(c, v);
                if !is_string(v) && !re_ok {
                    return type_err(c, format!("value of {} must be String{}", sym_str(shown), if nb == b"$;" { " or Regexp" } else { "" }));
                }
                gvar_warn(c, &format!("non-nil '{}' is deprecated", sym_str(shown)), true)?;
                if nb == b"$/" { v = gvar_frozen_str(c, v); }
            }
        }
        b"$." => { let n = crate::builtins::numeric_methods::num_to_long(c, v)?; v = Value::fix(n as i32 as i64); }
        b"$-i" => { if v != NIL { v = expect_str(c, v)?; } }
        b"$=" => { gvar_warn(c, "variable $= is no longer effective; ignored", true)?; return Ok(v); }
        _ => {}
    }
    c.vm().globals.borrow_mut().insert(name, v);
    Ok(v)
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
    "objspace", "coverage", "ripper", "tempfile", "tmpdir", "english", "English", "bigdecimal", "date", "json", "time",
    "benchmark", "fileutils", "shellwords", "strscan", "digest", "digest/md5", "digest/sha1", "digest/sha2", "zlib", "base64",
    "optparse", "ostruct", "forwardable", "singleton", "observer", "delegate", "socket", "open3", "logger", "uri", "net/http",
    "erb", "cgi", "abbrev", "matrix", "prime", "tsort", "weakref", "find", "pathname", "io/nonblock", "fcntl",
    "nkf", "racc", "racc/parser", "readline", "reline", "win32ole", "openssl", "mkmf", "rubygems",
    "objspace", "did_you_mean", "error_highlight", "syntax_suggest", "-test-/memory_status", "random/formatter",
    "date_core", "socket.so", "io/console/size", "digest/sha2", "digest/sha1", "digest/md5", "digest/rmd160", "digest/bubblebabble", "net/protocol", "net/https",
    "open-uri", "resolv", "ipaddr", "pty", "syslog", "expect", "io/console", "bigdecimal/util", "date/format",
];

fn resolve_feature(c: &Ctx, name: &str, relative_to: Option<&str>) -> Option<std::path::PathBuf> {
    let with_rb = |p: std::path::PathBuf| -> Option<std::path::PathBuf> {
        if p.extension().map(|e| e == "rb").unwrap_or(false) && p.is_file() { return Some(p); }
        let mut q = p.clone().into_os_string(); q.push(".rb");
        let q = std::path::PathBuf::from(q);
        if q.is_file() { return Some(q); }
        if p.is_file() && p.extension().is_none() { return Some(p); }
        None
    };
    if let Some(base) = relative_to {
        let dir = std::path::Path::new(base).parent().unwrap_or(std::path::Path::new("."));
        return with_rb(dir.join(name));
    }
    let p = std::path::Path::new(name);
    if p.is_absolute() || name.starts_with("./") || name.starts_with("../") { return with_rb(p.to_path_buf()); }
    let lp = c.vm().load_path.get();
    for i in 0..c.ary_len(lp) {
        let d = c.ary_get(lp, i);
        if !is_string(d) { continue; }
        let dir = String::from_utf8_lossy(c.str_bytes(d)).into_owned();
        if let Some(f) = with_rb(std::path::Path::new(&dir).join(name)) { return Some(f); }
    }
    None
}

enum Feature { File(std::path::PathBuf), Native(String), Missing }

fn locate_feature(c: &Ctx, name: &str, caller_file: Option<&str>) -> Feature {
    let relative = caller_file.is_some();
    if let Some(f) = resolve_feature(c, name, caller_file) { return Feature::File(f); }
    let is_rb = name.ends_with(".rb");
    let is_so = name.ends_with(".so");
    let base = name.trim_end_matches(".rb").trim_end_matches(".so");
    // "foo.so" may name a native part of its own, below a pure-Ruby "foo" (psych.so)
    let so_native = is_so && crate::builtins::prelude::has_feature(name);
    let native = !is_rb && (crate::builtins::prelude::has_feature(base) || so_native);
    // the reference ruby's pure-Ruby stdlib, unless saruby implements the feature itself
    let std_file = if native || is_so || relative || name.starts_with('/') || name.starts_with('.') { None } else { crate::builtins::prelude::stdlib_file(name) };
    match std_file {
        Some(f) => Feature::File(f),
        None if native || BUILTIN_FEATURES.contains(&base) => Feature::Native(if so_native { name.to_string() } else { base.to_string() }),
        None => Feature::Missing,
    }
}

/// rb_get_path for require/load: a String, else to_path (if any) and then to_str.
fn feature_name(c: &mut Ctx, v: Value) -> Result<String, Esc> {
    let s = if is_string(v) { v } else {
        let p = if v.is_heap() && respond_to(v, intern(b"to_path"), true) { funcall(c, v, intern(b"to_path"), &[], std::ptr::null())? } else { v };
        expect_str(c, p)?
    };
    Ok(String::from_utf8_lossy(c.str_bytes(s)).into_owned())
}

pub fn require_feature(c: &mut Ctx, path: Value, relative: bool) -> RV {
    let name = feature_name(c, path)?;
    let caller_file = if relative { c.caller_frame().map(|f| frame_file(f).to_string()) } else { None };
    let caller_file = caller_file.map(|f| if f == "-" || f == "-e" { "./x".to_string() } else { f });
    let file = match locate_feature(c, &name, caller_file.as_deref()) {
        Feature::File(f) => f,
        Feature::Native(base) => {
            let lf = c.vm().loaded_features.get();
            if c.ary(lf).iter().any(|&x| is_string(x) && c.str_bytes(x) == base.as_bytes()) { return Ok(FALSE); }
            let key = c.str_new(base.as_bytes());
            c.ary_push(lf, key);
            crate::builtins::prelude::load_feature(c, &base)?;
            return Ok(TRUE);
        }
        Feature::Missing => {
            // require_relative reports the absolute path it tried (File.absolute_path(name, dir))
            let shown = match &caller_file {
                Some(cf) => {
                    let dir = std::path::Path::new(cf).parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_else(|| ".".into());
                    let (nv, dv) = (c.str_new(name.as_bytes()), c.str_new(dir.as_bytes()));
                    String::from_utf8_lossy(&crate::builtins::file::expand(c, &[nv, dv], false)?).into_owned()
                }
                None => name.clone(),
            };
            let e = make_exception(c.vm().c_load_error.get(), &format!("cannot load such file -- {}", shown));
            ivar_set_raw(e, intern(b"path"), c.str_new(shown.as_bytes()));
            return Err(raise_exc(c, e));
        }
    };
    let abs = std::fs::canonicalize(&file).unwrap_or(file.clone());
    let key = abs.to_string_lossy().into_owned();
    let lf = c.vm().loaded_features.get();
    if c.ary(lf).iter().any(|&x| is_string(x) && c.str_bytes(x) == key.as_bytes()) { return Ok(FALSE); }
    let ks = c.str_new(key.as_bytes()); c.ary_push(lf, ks);
    load_file(c, &key)?;
    Ok(TRUE)
}

/// $LOAD_PATH.resolve_feature_path: [:rb, path], [:so, "name.so"] for a feature saruby provides, or nil.
fn lp_resolve_feature_path(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let sv = expect_str(c, a[0])?;
    let name = String::from_utf8_lossy(c.str_bytes(sv)).into_owned();
    let (kind, path) = match locate_feature(c, &name, None) {
        Feature::File(f) => ("rb", std::path::absolute(&f).unwrap_or(f).to_string_lossy().into_owned()),
        Feature::Native(base) => ("so", format!("{}.so", base)),
        Feature::Missing => return Ok(NIL),
    };
    let ps = c.str_new(path.as_bytes());
    Ok(c.ary_new_from(&[Value::sym(intern(kind.as_bytes())), ps]))
}
pub fn init_load_path(lp: Value) { define_singleton_cfunc(lp, "resolve_feature_path", lp_resolve_feature_path, 1); }

pub fn load_file(c: &mut Ctx, path: &str) -> RV {
    let src = match std::fs::read(path) {
        Ok(s) => s,
        Err(_) => { let e = make_exception(c.vm().c_load_error.get(), &format!("cannot load such file -- {}", path)); return Err(raise_exc(c, e)); }
    };
    let file: &'static str = Box::leak(path.to_string().into_boxed_str());
    run_source(c, &src, file, 1)
}

/// Compile and run a source at top level (self = main).
pub fn run_source(c: &mut Ctx, src: &[u8], file: &'static str, line: i32) -> RV {
    let is = match crate::compile::compile_source(src, file, line, IseqKind::Top, vec![]) {
        Ok(is) => is,
        Err(msg) => { return err(c, c.vm().c_syntax_error.get(), msg); }
    };
    crate::run_top_iseq(c, is, c.vm().main_obj.get())
}

fn k_require(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV { check_argc(c, args, 1, 1)?; require_feature(c, args[0], false) }
fn k_require_relative(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV { check_argc(c, args, 1, 1)?; require_feature(c, args[0], true) }
fn k_load(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
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
    let p = match found {
        Some(f) => f,
        None if std::path::Path::new(&ex).is_file() => ex,
        None => {
            let e = make_exception(c.vm().c_load_error.get(), &format!("cannot load such file -- {}", name));
            ivar_set_raw(e, intern(b"path"), c.str_new(name.as_bytes()));
            return Err(raise_exc(c, e));
        }
    };
    let wrap = args.get(1).copied().unwrap_or(FALSE);
    if wrap.truthy() {
        // load(file, true): run under an anonymous module, self = a fresh main extended with it
        let m = if is_class_or_module(wrap) { wrap } else { new_module() };
        let slf = new_object(c.vm().c_object.get());
        let sk = singleton_class(slf);
        include_module_c(c, sk, m);
        let src = match std::fs::read(&p) {
            Ok(s) => s,
            Err(_) => { let e = make_exception(c.vm().c_load_error.get(), &format!("cannot load such file -- {}", p)); return Err(raise_exc(c, e)); }
        };
        let file: &'static str = Box::leak(p.into_boxed_str());
        let is = match crate::compile::compile_source(&src, file, 1, IseqKind::Top, vec![]) { Ok(is) => is, Err(msg) => return err(c, c.vm().c_syntax_error.get(), msg) };
        let cref = new_cref(m, c.vm().top_cref.get());
        crate::run_top_iseq_cref(c, is, slf, cref)?;
        return Ok(TRUE);
    }
    load_file(c, &p)?;
    Ok(TRUE)
}

fn k_autoload(c: &mut Ctx, slf: Value, args: &[Value], _b: *const Block) -> RV {
    check_argc(c, args, 2, 2)?;
    let k = if is_class_or_module(slf) { slf } else { c.vm().c_object.get() };
    let nm = crate::builtins::object::to_sym_arg(c, args[0])?;
    let path = expect_str(c, args[1])?;
    if c.str_len(path) == 0 { return arg_err(c, "empty feature name"); }
    c.class_mut(k).autoload.insert(nm, path);
    Ok(NIL)
}

fn m_ruby2_keywords(c: &mut Ctx, slf: Value, args: &[Value], _b: *const Block) -> RV {
    let k = if is_class_or_module(slf) { slf } else { c.vm().c_object.get() };
    for &a in args {
        let nm = crate::builtins::object::to_sym_arg(c, a)?;
        let me = search_method(k, nm);
        if me.is_null() {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("undefined method '{}' for {} '{}'", sym_str(nm), if c.class(k).is_module { "module" } else { "class" }, class_path(k)), Value::sym(nm), k)?;
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
fn h_s_r2k_hash_p(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    if !is_hash(args[0]) { return type_err(c, format!("wrong argument type {} (expected Hash)", class_name_for_err(args[0]))); }
    Ok(Value::bool(obj_flags(args[0]) & FL_KWHASH != 0))
}
fn h_s_r2k_hash(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    if !is_hash(args[0]) { return type_err(c, format!("wrong argument type {} (expected Hash)", class_name_for_err(args[0]))); }
    let d = c.hash_new();
    let pairs = c.hash_pairs(args[0]);
    let keep = c.ary_new_from(&pairs.iter().flat_map(|&(k, x)| [k, x]).collect::<Vec<_>>());   // pairs is not scanned
    for (k, x) in pairs { crate::hash::hash_aset(c, d, k, x)?; }
    std::hint::black_box(keep);
    obj_set_flags(d, FL_KWHASH);
    Ok(d)
}
fn m_s_nesting(c: &mut Ctx, _s: Value, _args: &[Value], _b: *const Block) -> RV {
    let out = c.ary_new();
    let mut cr = c.caller_frame().map_or(std::ptr::null(), |f| f.cref());
    while let Some(r) = c.cref_ref(cr) {
        if r.prev.is_null() { break; }
        let (k, prev) = (r.klass, r.prev);
        c.ary_push(out, k);
        cr = prev;
    }
    Ok(out)
}
fn m_autoload_p(c: &mut Ctx, slf: Value, args: &[Value], _b: *const Block) -> RV {
    check_argc(c, args, 1, 2)?;
    let k = if is_class_or_module(slf) { slf } else { c.vm().c_object.get() };
    let nm = crate::builtins::object::to_sym_arg(c, args[0])?;
    let inherit = args.get(1).map(|v| v.truthy()).unwrap_or(true);
    let list: Vec<Value> = if inherit { ancestors(k).clone() } else { vec![k] };
    for kk in list {
        if const_get_at(kk, nm).is_some() { return Ok(NIL); }
        if let Some(&p) = c.class(kk).autoload.get(&nm) { return Ok(if c.str_bytes(p).starts_with(b"saruby/") { NIL } else { p }); }
    }
    Ok(NIL)
}

fn k_dir(c: &mut Ctx, _s: Value, _args: &[Value], _b: *const Block) -> RV {
    let f = c.caller_frame().map_or("-", frame_file).to_string();
    let p = std::path::Path::new(&f);
    let abs = std::fs::canonicalize(p).unwrap_or(p.to_path_buf());
    Ok(c.str_new(abs.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or(".".into()).as_bytes()))
}

// ---- eval / binding --------------------------------------------------------------------

fn k_eval(c: &mut Ctx, slf: Value, args: &[Value], _b: *const Block) -> RV {
    check_argc(c, args, 1, 4)?;
    let src = expect_str(c, args[0])?;
    let bind = if args.len() > 1 { args[1] } else { NIL };
    if bind != NIL && !crate::builtins::binding::is_binding(c, bind) {
        return type_err(c, format!("wrong argument type {} (expected binding)", class_name_for_err(bind)));
    }
    let file: &'static str = if args.len() > 2 && args[2] != NIL { let f = expect_str(c, args[2])?; Box::leak(String::from_utf8_lossy(c.str_bytes(f)).into_owned().into_boxed_str()) } else { crate::builtins::binding::eval_file(c) };
    let line = if args.len() > 3 { crate::builtins::numeric_methods::num_to_long(c, args[3])? as i32 } else { 1 };
    let srcb = c.str_bytes(src).to_vec();
    if bind != NIL { return crate::builtins::binding::eval_in_binding(c, bind, &srcb, file, line); }
    let b = crate::builtins::binding::binding_of_caller(c)?;
    let _ = slf;
    crate::builtins::binding::eval_in_binding(c, b, &srcb, file, line)
}

fn k_binding(c: &mut Ctx, _s: Value, _args: &[Value], _b: *const Block) -> RV { crate::builtins::binding::binding_of_caller(c) }

fn k_local_variables(c: &mut Ctx, _s: Value, _args: &[Value], _b: *const Block) -> RV {
    let b = crate::builtins::binding::binding_of_caller(c)?;
    crate::builtins::binding::local_variables(c, b)
}

pub fn init(vmr: &Vm) {
    let k = vmr.m_kernel.get();
    let mf = |n: &str, f: CFunc, a: i32| define_module_function(k, n, f, a);
    mf("puts", k_puts, -1);
    mf("print", k_print, -1);
    mf("p", k_p, -1);
    mf("pp", k_pp, -1);
    mf("putc", k_putc, 1);
    mf("printf", k_printf, -1);
    mf("sprintf", k_sprintf, -1);
    mf("format", k_sprintf, -1);
    mf("warn", k_warn, -1);
    mf("raise", k_raise, -1);
    mf("fail", k_raise, -1);
    mf("loop", k_loop, 0);
    mf("catch", k_catch, -1);
    mf("throw", k_throw, -1);
    mf("block_given?", k_block_given, 0);
    mf("lambda", k_lambda, 0);
    mf("proc", k_proc, 0);
    mf("__method__", k_method_name, 0);
    mf("__callee__", k_callee_name, 0);
    mf("caller", k_caller, -1);
    mf("exit", k_exit, -1);
    mf("exit!", k_exit_bang, -1);
    mf("abort", k_abort, -1);
    mf("at_exit", k_at_exit, 0);
    mf("gets", k_gets, -1);
    mf("readline", k_readline, -1);
    mf("readlines", k_readlines, -1);
    mf("rand", k_rand, -1);
    mf("srand", k_srand, -1);
    mf("Integer", k_integer, -1);
    mf("Float", k_float, -1);
    mf("String", k_string, -1);
    mf("Array", k_array, -1);
    mf("Hash", k_hash_conv, -1);
    mf("require", k_require, -1);
    mf("require_relative", k_require_relative, -1);
    mf("load", k_load, -1);
    mf("autoload", k_autoload, -1);
    mf("autoload?", m_autoload_p, -1);
    define_cfunc(vmr.c_module.get(), "autoload?", m_autoload_p, -1);
    define_private_cfunc(vmr.c_module.get(), "ruby2_keywords", m_ruby2_keywords, -1);
    define_private_cfunc(singleton_class(vmr.main_obj.get()), "ruby2_keywords", m_ruby2_keywords, -1);
    define_singleton_cfunc(vmr.c_module.get(), "nesting", m_s_nesting, 0);
    define_singleton_cfunc(vmr.c_hash.get(), "ruby2_keywords_hash?", h_s_r2k_hash_p, 1);
    define_singleton_cfunc(vmr.c_hash.get(), "ruby2_keywords_hash", h_s_r2k_hash, 1);
    mf("__dir__", k_dir, 0);
    mf("eval", k_eval, -1);
    mf("binding", k_binding, 0);
    mf("local_variables", k_local_variables, 0);
    define_cfunc(vmr.c_module.get(), "autoload", k_autoload, -1);
    let _ = intern_str;
}

pub fn parse_int_for_format(c: &mut Ctx, v: Value) -> RV {
    let b = c.str_bytes(v).to_vec();   // str_to_inum may allocate a bignum
    match str_to_inum(&b, 0, true) {
        Some(x) => Ok(x),
        None => { let ins = inspect_str(c, v)?; arg_err(c, format!("invalid value for Integer(): {}", ins)) }
    }
}
