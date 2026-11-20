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
pub fn out_flush() {
    let _ = STDOUT_BUF.try_with(|cell| {
        let mut buf = cell.borrow_mut();
        if !buf.is_empty() {
            let mut so = std::io::stdout().lock();
            let _ = so.write_all(&buf);
            let _ = so.flush();
            buf.clear();
        }
    });
}
pub fn err_write(b: &[u8]) {
    out_flush();
    let mut se = std::io::stderr().lock();
    let _ = se.write_all(b);
    let _ = se.flush();
}

/// Write to $stdout (dispatching when it was reassigned).
pub fn stdout_write(c: &mut Ctx, b: &[u8]) -> Result<(), Esc> {
    let so = *c.vm().globals.get(&intern(b"$stdout")).unwrap_or(&c.vm().stdout);
    if so == c.vm().stdout { out_write(b); return Ok(()); }
    if so == c.vm().stderr { err_write(b); return Ok(()); }
    let st = c.str_new(b);
    funcall(c, so, intern(b"write"), &[st], std::ptr::null())?;
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
    if v == c.vm().main_obj { return Ok("main".into()); }
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
    let so = *c.vm().globals.get(&intern(b"$stdout")).unwrap_or(&c.vm().stdout);
    if so != c.vm().stdout && so != c.vm().stderr {
        return funcall(c, so, s::puts, args, std::ptr::null());
    }
    let out = puts_bytes(c, args)?;
    if so == c.vm().stderr { err_write(&out); } else { out_write(&out); }
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
    if c.vm().verbose == NIL { return Ok(NIL); }
    let mut uplevel = None;
    if let Some(h) = kw {
        if let Some(u) = hash_get_opt(c, h, crate::vm::sym_value("uplevel"))? { if u != NIL { uplevel = Some(u.as_fix()); } }
    }
    let mut out = puts_bytes(c, if args.is_empty() { return Ok(NIL) } else { args })?;
    if let Some(u) = uplevel {
        let lines = backtrace_lines(c, 1 + u as usize);
        if let Some(l) = lines.first() {
            let loc = l.split(":in ").next().unwrap_or("").to_string();
            let mut pre = format!("{}: warning: ", loc).into_bytes();
            pre.extend(out); out = pre;
        }
    }
    let se = *c.vm().globals.get(&intern(b"$stderr")).unwrap_or(&c.vm().stderr);
    if se != c.vm().stderr {
        let st = c.str_from_vec(out);
        funcall(c, se, intern(b"write"), &[st], std::ptr::null())?;
    } else { err_write(&out); }
    Ok(NIL)
}

// ---- raise ------------------------------------------------------------------------------

pub fn make_exception_from_args(c: &mut Ctx, args: &[Value]) -> RV {
    let (args, kw) = match kw_arg(args) { Some(h) => (&args[..args.len() - 1], Some(h)), None => (args, None) };
    let e = match args.len() {
        0 => {
            if c.errinfo != NIL { c.errinfo }
            else { make_exception(c.vm().c_runtime_error, "unhandled exception") }
        }
        1 if is_string(args[0]) => {
            let e = new_object(c.vm().c_runtime_error);
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
                if c.esc == EscKind::Raise && is_kind_of(c.esc_val, c.vm().c_stop_iteration) {
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
    let tag = if args.is_empty() { new_object(c.vm().c_object) } else { args[0] };
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
        let e = make_exception(c.vm().c_uncaught_throw_error, &format!("uncaught throw {}", ins));
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
    Ok(Value::sym(crate::builtins::object::callee_name(me)))
}

fn k_caller(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let (start, len) = crate::builtins::exception::caller_range(c, args)?;
    let lines = backtrace_lines(c, 1 + start);
    let lines: Vec<String> = match len { Some(n) => lines.into_iter().take(n).collect(), None => lines };
    Ok(c.ary_of_strs(lines.into_iter().map(|l| l.into_bytes()).collect(), ENC_UTF8))
}

pub fn k_exit(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let st = match args.first() { None => 0, Some(&v) if v == TRUE => 0, Some(&v) if v == FALSE => 1, Some(&v) => crate::builtins::numeric_methods::num_to_long(c, v)? };
    let e = make_exception(c.vm().c_system_exit, "exit");
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
    let e = new_object(c.vm().c_system_exit);
    ivar_set_raw(e, s::mesg, msg);
    ivar_set_raw(e, s::status, Value::fix(1));
    let _ = c;
    Err(raise_exc(c, e))
}

fn k_at_exit(c: &mut Ctx, _s: Value, _args: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return arg_err(c, "called without a block"); }
    let p = block_to_proc(c, blk)?;
    c.ary_push(c.vm().at_exit, p);
    Ok(p)
}

/// ARGF (a lazily loaded prelude library).
pub fn argf(c: &mut Ctx) -> RV { const_lookup(c, c.vm().top_cref, intern(b"ARGF")) }
fn argf_plain(c: &Ctx) -> bool { c.ary_len(c.vm().argv) == 0 && const_get_at(c.vm().c_object, intern(b"ARGF")).is_none() }
fn k_gets(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    out_flush();
    // no file arguments: plain stdin without loading ARGF
    let src = if argf_plain(c) { *c.vm().globals.get(&intern(b"$stdin")).unwrap_or(&c.vm().stdin) } else { argf(c)? };
    let v = funcall(c, src, intern(b"gets"), args, std::ptr::null())?;
    c.vm_mut().globals.insert(intern(b"$_"), v);
    Ok(v)
}
fn k_readline(c: &mut Ctx, s: Value, args: &[Value], b: *const Block) -> RV {
    let v = k_gets(c, s, args, b)?;
    if v == NIL { return err(c, c.vm().c_eof_error, "end of file reached"); }
    Ok(v)
}
fn k_readlines(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    out_flush();
    let src = if argf_plain(c) { *c.vm().globals.get(&intern(b"$stdin")).unwrap_or(&c.vm().stdin) } else { argf(c)? };
    funcall(c, src, intern(b"readlines"), args, std::ptr::null())
}

/// Print a warning line unless $VERBOSE is nil.
pub fn warn_str(c: &mut Ctx, msg: &str) { if c.vm().verbose != NIL { err_write(msg.as_bytes()); } }

fn k_rand(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    crate::builtins::random::rand_value(c, args.first().copied().unwrap_or(NIL))
}
fn k_srand(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let old = c.vm().rand_state;
    let seed = match args.first() { Some(&v) => crate::builtins::numeric_methods::num_to_long(c, v)? as u64, None => 0x2545F4914F6CDD1D };
    crate::builtins::random::seed(c, seed);
    Ok(c.int_from_i128(old as i128))
}

// ---- conversion functions ----------------------------------------------------------------

fn kw_exception(c: &mut Ctx, args: &[Value]) -> Result<(usize, bool), Esc> {
    if let Some(h) = kw_arg(args) {
        let ex = hash_get_opt(c, h, crate::vm::sym_value("exception"))?.map(|v| v.truthy()).unwrap_or(true);
        return Ok((args.len() - 1, ex));
    }
    Ok((args.len(), true))
}

fn k_integer(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let (n, exc) = kw_exception(c, args)?;
    let args = &args[..n];
    check_argc(c, args, 1, 2)?;
    let v = args[0];
    let base = if args.len() > 1 && args[1] != NIL { crate::builtins::numeric_methods::num_to_long(c, args[1])? as u32 } else { 0 };
    let r: RV = (|| {
        if is_string(v) {
            let b = c.str_bytes(v).to_vec();
            if b.contains(&0) { return arg_err(c, "string contains null byte"); }
            match str_to_inum(&b, base, true) {
                Some(x) => Ok(x),
                None => { let ins = inspect_str(c, v)?; arg_err(c, format!("invalid value for Integer(): {}", ins)) }
            }
        } else if args.len() > 1 && args[1] != NIL {
            arg_err(c, "base specified for non string value")
        } else if v.is_fixnum() || is_bignum(v) { Ok(v) }
        else if is_float(v) {
            let f = float_val(v);
            if f.is_nan() || f.is_infinite() { return err(c, c.vm().c_float_domain_error, crate::numeric::float_to_s(f)); }
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

pub fn parse_float_strict(c: &mut Ctx, b: &[u8]) -> Option<f64> {
    let s = String::from_utf8_lossy(b);
    let t = s.trim();
    if t.is_empty() { return None; }
    let tl = t.to_ascii_lowercase();
    if tl.starts_with("0x") || tl.starts_with("-0x") || tl.starts_with("+0x") {
        return str_to_inum(t.as_bytes(), 16, true).map(|v| c.num_to_f64(v).unwrap());
    }
    // validate: digits with single underscores, optional fraction / exponent
    let bytes = t.as_bytes();
    let mut i = 0;
    if bytes[i] == b'+' || bytes[i] == b'-' { i += 1; }
    let start_digits = i;
    let mut prev_digit = false;
    while i < bytes.len() && (bytes[i].is_ascii_digit() || (bytes[i] == b'_' && prev_digit && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit())) { prev_digit = bytes[i] != b'_'; i += 1; }
    if i == start_digits { return None; }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let fs = i;
        while i < bytes.len() && (bytes[i].is_ascii_digit() || (bytes[i] == b'_' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit())) { i += 1; }
        if i == fs { return None; }
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') { i += 1; }
        let es = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
        if i == es { return None; }
    }
    if i != bytes.len() { return None; }
    let clean: String = t.chars().filter(|&c| c != '_').collect();
    clean.parse().ok()
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
            match parse_float_strict(c, &b) { Some(f) => return Ok(c.float_new(f)), None => { let ins = inspect_str(c, v)?; return arg_err(c, format!("invalid value for Float(): {}", ins)); } }
        }
        if v == NIL { return type_err(c, "can't convert nil into Float"); }
        if respond_to(v, s::to_f, true) { return funcall(c, v, s::to_f, &[], std::ptr::null()); }
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

fn resolve_gvar(c: &Ctx, name: Sym) -> Sym { *c.vm().global_aliases.get(&name).unwrap_or(&name) }

pub fn gvar_get(c: &mut Ctx, name: Sym) -> RV {
    let name = resolve_gvar(c, name);
    let b = sym_bytes(name);
    match b {
        b"$!" => return Ok(c.errinfo),
        b"$@" => { return Ok(if c.errinfo == NIL { NIL } else { exc_bt(c.errinfo) }); }
        b"$~" => return Ok(c.last_match),
        b"$&" => return Ok(crate::regexp::nth_match(c.last_match, 0)),
        b"$0" | b"$PROGRAM_NAME" => return Ok(c.vm().dollar_zero),
        b"$:" | b"$LOAD_PATH" | b"$-I" => return Ok(c.vm().load_path),
        b"$\"" | b"$LOADED_FEATURES" => return Ok(c.vm().loaded_features),
        b"$$" => return Ok(Value::fix(std::process::id() as i64)),
        b"$*" => return Ok(c.vm().argv),
        b"$<" => return argf(c),
        b"$FILENAME" => { let a = argf(c)?; return funcall(c, a, intern(b"filename"), &[], std::ptr::null()); }
        b"$VERBOSE" | b"$-v" | b"$-w" => return Ok(c.vm().verbose),
        b"$DEBUG" | b"$-d" => return Ok(Value::bool(c.vm().debug_flag)),
        b"$?" => return Ok(crate::builtins::process::last_status()),
        _ => {}
    }
    Ok(*c.vm().globals.get(&name).unwrap_or(&NIL))
}

pub fn gvar_set(c: &mut Ctx, name: Sym, v: Value) -> RV {
    let name = resolve_gvar(c, name);
    match sym_bytes(name) {
        b"$!" => { c.errinfo = v; return Ok(v); }
        b"$~" => { c.last_match = v; return Ok(v); }
        b"$VERBOSE" | b"$-v" | b"$-w" => { c.vm_mut().verbose = if v == NIL { NIL } else { Value::bool(v.truthy()) }; return Ok(v); }
        b"$DEBUG" => { c.vm_mut().debug_flag = v.truthy(); return Ok(v); }
        b"$0" | b"$PROGRAM_NAME" => { c.vm_mut().dollar_zero = v; return Ok(v); }
        b"$stdout" | b"$stderr" => {
            if !respond_to(v, intern(b"write"), true) {
                return type_err(c, format!("{} must have write method, {} given", sym_str(name), class_name_for_err(v)));
            }
        }
        b"$:" | b"$LOAD_PATH" | b"$\"" | b"$LOADED_FEATURES" | b"$$" => {
            let e = make_name_error(c, c.vm().c_name_error, &format!("{} is a read-only variable", sym_str(name)), Value::sym(name), NIL)?;
            return Err(raise_exc(c, e));
        }
        _ => {}
    }
    c.vm_mut().globals.insert(name, v);
    Ok(v)
}

pub fn gvar_defined(c: &Ctx, name: Sym) -> bool {
    let name = resolve_gvar(c, name);
    match sym_bytes(name) {
        b"$!" => c.errinfo != NIL,
        b"$~" => c.last_match != NIL,
        b"$0" | b"$PROGRAM_NAME" | b"$:" | b"$LOAD_PATH" | b"$\"" | b"$LOADED_FEATURES" | b"$$" | b"$*" | b"$<" | b"$FILENAME" | b"$stdout" | b"$stderr" | b"$stdin" | b"$VERBOSE" | b"$DEBUG" | b"$," | b"$/" | b"$;" => true,
        _ => c.vm().globals.contains_key(&name),
    }
}

// ---- require / load ------------------------------------------------------------------------

pub const BUILTIN_FEATURES: &[&str] = &["set", "enumerator", "rational", "complex", "thread", "fiber", "continuation",
    "pp", "prettyprint", "stringio", "rbconfig", "etc", "io/console", "io/wait", "monitor", "timeout", "securerandom",
    "objspace", "coverage", "ripper", "tempfile", "tmpdir", "english", "English", "bigdecimal", "date", "json", "time",
    "benchmark", "fileutils", "shellwords", "strscan", "digest", "digest/md5", "digest/sha1", "digest/sha2", "zlib", "base64",
    "optparse", "ostruct", "forwardable", "singleton", "observer", "delegate", "socket", "open3", "logger", "uri", "net/http",
    "erb", "cgi", "yaml", "psych", "abbrev", "matrix", "prime", "tsort", "weakref", "find", "pathname", "io/nonblock", "fcntl",
    "nkf", "racc", "racc/parser", "readline", "reline", "win32ole", "openssl", "mkmf", "rubygems", "bundler", "bundler/setup",
    "objspace", "did_you_mean", "error_highlight", "syntax_suggest", "-test-/memory_status", "random/formatter",
    "date_core", "socket.so", "io/console/size", "digest/sha2", "digest/sha1", "digest/md5", "digest/rmd160", "digest/bubblebabble", "net/protocol", "net/https",
    "open-uri", "resolv", "ipaddr", "pty", "fiddle", "syslog", "expect", "io/console", "yaml/store", "bigdecimal/util", "date/format", "psych/y",
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
    let lp = c.vm().load_path;
    for i in 0..c.ary_len(lp) {
        let d = c.ary_get(lp, i);
        if !is_string(d) { continue; }
        let dir = String::from_utf8_lossy(c.str_bytes(d)).into_owned();
        if let Some(f) = with_rb(std::path::Path::new(&dir).join(name)) { return Some(f); }
    }
    None
}

pub fn require_feature(c: &mut Ctx, path: Value, relative: bool) -> RV {
    let name = to_rstring(c, path)?;
    let caller_file = if relative { c.caller_frame().map(|f| frame_file(f).to_string()) } else { None };
    let caller_file = caller_file.map(|f| if f == "-" || f == "-e" { "./x".to_string() } else { f });
    let resolved = resolve_feature(c, &name, caller_file.as_deref());
    let file = match resolved {
        Some(f) => f,
        None => {
            let is_rb = name.ends_with(".rb");
            let is_so = name.ends_with(".so");
            let base = name.trim_end_matches(".rb").trim_end_matches(".so");
            let lf = c.vm().loaded_features;
            let native = !is_rb && crate::builtins::prelude::has_feature(base);
            // the reference ruby's pure-Ruby stdlib, unless saruby implements the feature itself
            let std_file = if native || is_so || relative || name.starts_with('/') || name.starts_with('.') { None } else { crate::builtins::prelude::stdlib_file(&name) };
            match std_file {
                Some(f) => f,
                None => {
                    if native || BUILTIN_FEATURES.contains(&base) {
                        if c.ary(lf).iter().any(|&x| is_string(x) && c.str_bytes(x) == base.as_bytes()) { return Ok(FALSE); }
                        let key = c.str_new(base.as_bytes());
                        c.ary_push(lf, key);
                        crate::builtins::prelude::load_feature(c, base)?;
                        return Ok(TRUE);
                    }
                    let e = make_exception(c.vm().c_load_error, &format!("cannot load such file -- {}", name));
                    ivar_set_raw(e, intern(b"path"), c.str_new(name.as_bytes()));
                    return Err(raise_exc(c, e));
                }
            }
        }
    };
    let abs = std::fs::canonicalize(&file).unwrap_or(file.clone());
    let key = abs.to_string_lossy().into_owned();
    let lf = c.vm().loaded_features;
    if c.ary(lf).iter().any(|&x| is_string(x) && c.str_bytes(x) == key.as_bytes()) { return Ok(FALSE); }
    let ks = c.str_new(key.as_bytes()); c.ary_push(lf, ks);
    load_file(c, &key)?;
    Ok(TRUE)
}

pub fn load_file(c: &mut Ctx, path: &str) -> RV {
    let src = match std::fs::read(path) {
        Ok(s) => s,
        Err(_) => { let e = make_exception(c.vm().c_load_error, &format!("cannot load such file -- {}", path)); return Err(raise_exc(c, e)); }
    };
    let file: &'static str = Box::leak(path.to_string().into_boxed_str());
    run_source(c, &src, file, 1)
}

/// Compile and run a source at top level (self = main).
pub fn run_source(c: &mut Ctx, src: &[u8], file: &'static str, line: i32) -> RV {
    let is = match crate::compile::compile_source(src, file, line, IseqKind::Top, vec![]) {
        Ok(is) => is,
        Err(msg) => { return err(c, c.vm().c_syntax_error, msg); }
    };
    crate::run_top_iseq(c, is, c.vm().main_obj)
}

fn k_require(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV { check_argc(c, args, 1, 1)?; require_feature(c, args[0], false) }
fn k_require_relative(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV { check_argc(c, args, 1, 1)?; require_feature(c, args[0], true) }
fn k_load(c: &mut Ctx, _s: Value, args: &[Value], _b: *const Block) -> RV {
    let name = to_rstring(c, args[0])?;
    let p = resolve_feature(c, &name, None).map(|p| p.to_string_lossy().into_owned()).unwrap_or(name);
    let wrap = args.get(1).copied().unwrap_or(FALSE);
    if wrap.truthy() {
        // load(file, true): run under an anonymous module, self = a fresh main extended with it
        let m = if is_class_or_module(wrap) { wrap } else { new_module() };
        let slf = new_object(c.vm().c_object);
        let sk = singleton_class(slf);
        include_module(sk, m);
        let src = match std::fs::read(&p) {
            Ok(s) => s,
            Err(_) => { let e = make_exception(c.vm().c_load_error, &format!("cannot load such file -- {}", p)); return Err(raise_exc(c, e)); }
        };
        let file: &'static str = Box::leak(p.into_boxed_str());
        let is = match crate::compile::compile_source(&src, file, 1, IseqKind::Top, vec![]) { Ok(is) => is, Err(msg) => return err(c, c.vm().c_syntax_error, msg) };
        let cref = new_cref(m, c.vm().top_cref);
        crate::run_top_iseq_cref(c, is, slf, cref)?;
        return Ok(TRUE);
    }
    load_file(c, &p)?;
    Ok(TRUE)
}

fn k_autoload(c: &mut Ctx, slf: Value, args: &[Value], _b: *const Block) -> RV {
    check_argc(c, args, 2, 2)?;
    let k = if is_class_or_module(slf) { slf } else { c.vm().c_object };
    let nm = crate::builtins::object::to_sym_arg(c, args[0])?;
    let path = expect_str(c, args[1])?;
    if c.str_len(path) == 0 { return arg_err(c, "empty feature name"); }
    c.class_mut(k).autoload.insert(nm, path);
    Ok(NIL)
}

fn m_ruby2_keywords(c: &mut Ctx, slf: Value, args: &[Value], _b: *const Block) -> RV {
    let k = if is_class_or_module(slf) { slf } else { c.vm().c_object };
    for &a in args {
        let nm = crate::builtins::object::to_sym_arg(c, a)?;
        let me = search_method(k, nm);
        if me.is_null() {
            let e = make_name_error(c, c.vm().c_name_error, &format!("undefined method '{}' for {} '{}'", sym_str(nm), if c.class(k).is_module { "module" } else { "class" }, class_path(k)), Value::sym(nm), k)?;
            return Err(raise_exc(c, e));
        }
        let wloc = match c.caller_frame() { Some(pf) => format!("{}:{}: warning: Skipping set of ruby2_keywords flag for {} ", frame_file(pf), pf.line, sym_str(nm)), None => String::new() };
        let Some(me) = c.me_ref(me) else { continue };
        if me.owner != k {
            err_write(format!("{}(can only set in method defining module)\n", wloc).as_bytes());
            continue;
        }
        let ok = match me.kind {
            MKind::Iseq(is) => {
                let Some(is) = c.iseq_ref(is) else { continue };
                let p = &is.params;
                let ok = p.rest.is_some() && p.post.is_empty() && p.kw.is_empty() && p.kwrest.is_none() && !p.no_kw;
                if ok { is.ruby2_keywords.set(true); }
                ok
            }
            MKind::Cfunc(..) => { err_write(format!("{}(method not defined in Ruby)\n", wloc).as_bytes()); continue; }
            _ => false,
        };
        if !ok { err_write(format!("{}(method accepts keywords or post arguments or method does not accept argument splat)\n", wloc).as_bytes()); }
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
    let k = if is_class_or_module(slf) { slf } else { c.vm().c_object };
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
    let file: &'static str = if args.len() > 2 { Box::leak(to_rstring(c, args[2])?.into_boxed_str()) } else { "(eval)" };
    let line = if args.len() > 3 { args[3].as_fix() as i32 } else { 1 };
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
    let k = vmr.m_kernel;
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
    define_cfunc(vmr.c_module, "autoload?", m_autoload_p, -1);
    define_private_cfunc(vmr.c_module, "ruby2_keywords", m_ruby2_keywords, -1);
    define_private_cfunc(singleton_class(vmr.main_obj), "ruby2_keywords", m_ruby2_keywords, -1);
    define_singleton_cfunc(vmr.c_module, "nesting", m_s_nesting, 0);
    define_singleton_cfunc(vmr.c_hash, "ruby2_keywords_hash?", h_s_r2k_hash_p, 1);
    define_singleton_cfunc(vmr.c_hash, "ruby2_keywords_hash", h_s_r2k_hash, 1);
    mf("__dir__", k_dir, 0);
    mf("eval", k_eval, -1);
    mf("binding", k_binding, 0);
    mf("local_variables", k_local_variables, 0);
    define_cfunc(vmr.c_module, "autoload", k_autoload, -1);
    let _ = intern_str;
}

pub fn parse_int_for_format(c: &mut Ctx, v: Value) -> RV {
    let b = c.str_bytes(v).to_vec();   // str_to_inum may allocate a bignum
    match str_to_inum(&b, 0, true) {
        Some(x) => Ok(x),
        None => { let ins = inspect_str(c, v)?; arg_err(c, format!("invalid value for Integer(): {}", ins)) }
    }
}
