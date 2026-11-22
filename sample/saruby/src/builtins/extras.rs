// Small reflection / system methods that complete the core classes.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::array::*;
use crate::builtins::exception::raise_errno;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::builtins::object::to_sym_arg;
use crate::builtins::sys;
use crate::hash::*;
use crate::string::*;
use crate::symbol::{intern, sym_bytes, sym_str};
use crate::value::*;
use crate::vm::*;

fn m_deprecate_constant(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    for &x in a {
        let n = to_sym_arg(c, x)?;
        if !c.class(s).consts.contains_key(&n) {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("constant {}::{} not defined", class_path(c, s), sym_str(n)), Value::sym(n), s)?;
            return Err(raise_exc(c, e));
        }
    }
    Ok(s)
}
fn m_remove_cvar(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = to_sym_arg(c, a[0])?;
    if !sym_bytes(n).starts_with(b"@@") {
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("'{}' is not allowed as a class variable name", sym_str(n)), Value::sym(n), s)?;
        return Err(raise_exc(c, e));
    }
    match c.class_mut(s).cvars.remove(&n) {
        Some(v) => Ok(v),
        None => { let e = make_name_error(c, c.vm().c_name_error.get(), &format!("class variable {} not defined for {}", sym_str(n), class_path(c, s)), Value::sym(n), s)?; Err(raise_exc(c, e)) }
    }
}
fn m_undefined_instance_methods(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let names: Vec<Value> = c.class(s).mtbl.iter().filter(|(_, me)| matches!(c.me_ref(**me).map(|m| &m.kind), Some(MKind::Undef))).map(|(n, _)| Value::sym(*n)).collect();
    Ok(c.ary_from_vec(names))
}
fn le_path(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(c, s, intern(b"path"))) }
fn nme_private_call(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(ivar_get(c, s, intern(b"private_call")).truthy())) }
fn sig_initialize(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, 2); }
    let (no, name) = if a[0].is_fixnum() {
        let n = a[0].as_fix();
        let nm = match a.get(1) { Some(&m) => to_rstring(c, m)?, None => match crate::builtins::process::signame(n as i32) { Some(x) => format!("SIG{}", x), None => return arg_err(c, format!("invalid signal number ({})", n)) } };
        (n, nm)
    } else {
        if a.len() > 1 { return argc_err(c, a.len(), 1, 1); }
        let n = crate::builtins::process::signo_arg(c, a[0])?;
        (n as i64, format!("SIG{}", crate::builtins::process::signame(n).unwrap_or("")))
    };
    ivar_set_raw(c, s, intern(b"signo"), Value::fix(no));
    let m = c.str_new(name.as_bytes());
    ivar_set_raw(c, s, crate::symbol::s::mesg, m);
    Ok(NIL)
}
fn sig_signo(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(c, s, intern(b"signo"))) }
fn sig_signm(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { funcall(c, s, intern(b"message"), &[], std::ptr::null()) }
fn m_expm1(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let f = crate::builtins::numeric_methods::rb_float(c, a[0])?; Ok(c.float_new(f.exp_m1())) }
fn m_log1p(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let f = crate::builtins::numeric_methods::rb_float(c, a[0])?;
    if f < -1.0 { return err(c, c.vm().c_math_domain_error.get(), "Numerical argument is out of domain - \"log1p\""); }
    Ok(c.float_new(f.ln_1p()))
}
fn m_gamma(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let f = crate::builtins::numeric_methods::rb_float(c, a[0])?;
    if f == f64::NEG_INFINITY || (f < 0.0 && f == f.floor()) { return err(c, c.vm().c_math_domain_error.get(), "Numerical argument is out of domain - \"gamma\""); }
    if f == f.floor() && (1.0..=23.0).contains(&f) { let mut r = 1.0f64; let mut k = 2.0; while k < f { r *= k; k += 1.0; } return Ok(c.float_new(r)); }
    Ok(c.float_new(sys::tgamma(f)))
}
fn m_lgamma(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let f = crate::builtins::numeric_methods::rb_float(c, a[0])?;
    if f == f64::NEG_INFINITY { return err(c, c.vm().c_math_domain_error.get(), "Numerical argument is out of domain - \"lgamma\""); }
    let (r, sign) = if f == 0.0 { (f64::INFINITY, if f.is_sign_negative() { -1 } else { 1 }) } else { let mut sign = 1i32; let r = sys::lgamma_r(f, &mut sign); (r, if sign < 0 { -1 } else { 1 }) };
    let rv = c.float_new(r);
    Ok(c.ary_new_from(&[rv, Value::fix(sign)]))
}
fn m_erf(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let f = crate::builtins::numeric_methods::rb_float(c, a[0])?; Ok(c.float_new(sys::erf(f))) }
fn m_erfc(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let f = crate::builtins::numeric_methods::rb_float(c, a[0])?; Ok(c.float_new(sys::erfc(f))) }
fn m_cbrt(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let f = crate::builtins::numeric_methods::rb_float(c, a[0])?; let mut r = sys::cbrt(f); if r.is_finite() && !(f == 0.0 && r == 0.0) { r = (2.0 * r + (f / r / r)) / 3.0; } Ok(c.float_new(r)) }
/// GC.stat_heap([heap [, key]]): saruby has one heap of fixed-size slots (heap 0).
fn gc_stat_heap(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let h = crate::gc::stats();
    let st = c.hash_new();
    for (k, v) in [("slot_size", std::mem::size_of::<RObj>() as i64), ("heap_live_slots", (h.slots - h.free) as i64), ("heap_free_slots", h.free as i64),
                   ("total_allocated_objects", h.allocated as i64), ("total_freed_objects", h.freed as i64)] {
        hash_set(c, st, sym_value(k), Value::fix(v))?;
    }
    match a.first() {
        None => { let all = c.hash_new(); hash_set(c, all, Value::fix(0), st)?; Ok(all) }
        Some(&i) if i == Value::fix(0) => match a.get(1) {
            Some(&k) if k.is_sym() => match crate::hash::hash_get_opt(c, st, k)? { Some(v) => Ok(v), None => arg_err(c, format!("unknown key: {}", sym_str(k.as_sym()))) },
            _ => Ok(st),
        },
        Some(_) => arg_err(c, "size pool index out of range"),
    }
}
fn gc_config(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let h = c.hash_new();
    hash_set(c, h, sym_value("rgengc_allow_full_mark"), TRUE)?;
    let d = c.str_new(b"default");
    hash_set(c, h, sym_value("implementation"), d)?;
    Ok(h)
}
fn k_global_variables(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let out = c.ary_new();
    // the built-in names first (CRuby's order), then the others by name: the table's own order is a hash order
    let mut user: Vec<Vec<u8>> = c.vm().globals.borrow().keys().map(|k| sym_bytes(*k).to_vec()).collect();
    user.sort();
    let mut names: Vec<Vec<u8>> = vec![];
    for n in ["$0", "$PROGRAM_NAME", "$:", "$LOAD_PATH", "$\"", "$LOADED_FEATURES", "$$", "$*", "$?", "$!", "$@", "$~", "$&", "$`", "$'", "$+", "$VERBOSE", "$DEBUG", "$FILENAME", "$<", "$,", "$;", "$.", "$_", "$-w", "$-v", "$-d", "$-I", "$-0", "$-a", "$-i", "$-l", "$-p", "$stdin", "$stdout", "$stderr", "$>", "$/", "$\\", "$="] {
        names.push(n.as_bytes().to_vec());
    }
    for u in user { if !names.contains(&u) { names.push(u); } }
    for n in names { c.ary_push(out, Value::sym(intern(&n))); }
    Ok(out)
}
fn k_test(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 2, 3)?;
    let cmd = if is_string(a[0]) { *c.str_bytes(a[0]).first().unwrap_or(&b'?') } else { num_to_long(c, a[0])? as u8 };
    let name = match cmd {
        b'e' => "exist?", b'f' => "file?", b'd' => "directory?", b'r' => "readable?", b'R' => "readable_real?", b'w' => "writable?", b'W' => "writable_real?",
        b'x' => "executable?", b'X' => "executable_real?", b's' => "size?", b'z' => "zero?", b'l' => "symlink?", b'p' => "pipe?", b'S' => "socket?",
        b'b' => "blockdev?", b'c' => "chardev?", b'u' => "setuid?", b'g' => "setgid?", b'k' => "sticky?", b'o' => "owned?", b'O' => "owned?", b'G' => "grpowned?",
        b'M' => "mtime", b'A' => "atime", b'C' => "ctime", b'-' => "identical?",
        b'=' | b'<' | b'>' => {
            let t1 = funcall(c, c.vm().c_file.get(), intern(b"mtime"), &a[1..2], std::ptr::null())?;
            let t2 = funcall(c, c.vm().c_file.get(), intern(b"mtime"), &a[2..3], std::ptr::null())?;
            return funcall(c, t1, intern(if cmd == b'=' { b"==" } else if cmd == b'<' { b"<" } else { b">" }), &[t2], std::ptr::null());
        }
        _ => return arg_err(c, format!("unknown command '{}'", cmd as char)),
    };
    funcall(c, c.vm().c_file.get(), intern(name.as_bytes()), &a[1..], std::ptr::null())
}
fn io_wait(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    // wait(events, timeout) or wait(timeout = nil, mode = :read)
    let (fd, buffered) = { let io = crate::builtins::io::get_io(c, s)?; (io.fd, io.rpos < io.rbuf.len()) };
    // io.c io_wait: two non-Symbol arguments are (events, timeout); otherwise modes and at most one timeout
    let int_form = a.len() == 2 && !a[0].is_sym() && !a[1].is_sym();
    let (mut events, mut timeout) = (0i64, NIL);
    if int_form {
        events = num_to_long(c, a[0])?;
        if events <= 0 { return arg_err(c, "Events must be positive integer!"); }
        timeout = a[1];
    } else {
        let mut given = false;
        for &x in a {
            if x.is_sym() { events |= match sym_str(x.as_sym()).as_str() { "r" | "read" | "readable" => 1, "w" | "write" | "writable" => 4, "rw" | "read_write" | "readable_writable" => 5, o => return arg_err(c, format!("unsupported mode: {}", o)) }; continue; }
            if given { return arg_err(c, "timeout given more than once"); }
            given = true;
            // rb_time_interval
            if !crate::numeric::is_numeric(x) { return type_err(c, format!("can't convert {} into time interval", class_path(c, real_class_of(c, x)))); }
            if crate::builtins::numeric_methods::to_f(c, x)? < 0.0 { return arg_err(c, "time interval must not be negative"); }
            timeout = x;
        }
        if events == 0 { events = 1; }
    }
    if events & 1 != 0 && buffered { return Ok(if int_form { Value::fix(1) } else { s }); }
    let dl = crate::builtins::io::wait_deadline(c, if timeout == NIL { None } else { Some(&timeout) })?;
    let ev = (if events & 1 != 0 { sys::POLLIN } else { 0 }) | (if events & 4 != 0 { sys::POLLOUT } else { 0 }) | (if events & 2 != 0 { 2 } else { 0 });
    let Some(rev) = crate::builtins::io::wait_one(c, fd, ev, dl)? else { return Ok(NIL) };
    crate::builtins::io::get_io(c, s)?;
    if !int_form { return Ok(s); }
    let mut r = 0;
    if rev & (sys::POLLIN | sys::POLLHUP) != 0 { r |= 1; }
    if rev & sys::POLLOUT != 0 { r |= 4; }
    Ok(Value::fix(r & events.max(1)))
}
fn io_each_codepoint(c: &mut Ctx, s: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s, intern(b"each_codepoint"), a, NIL); }
    loop {
        let ch = funcall(c, s, intern(b"getc"), &[], std::ptr::null())?;
        if ch == NIL { break; }
        let cp = funcall(c, ch, intern(b"ord"), &[], std::ptr::null())?;
        yield1(c, blk, cp)?;
    }
    Ok(s)
}
fn wm_h(c: &Ctx, s: Value) -> Value { ivar_get(c, s, intern(b"@h")) }
fn wm_size(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(c.hash_len(wm_h(c, s)) as i64)) }
fn wm_delete(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV {
    match hash_delete(c, wm_h(c, s), a[0])? { Some(v) => Ok(v), None => if b.is_null() { Ok(NIL) } else { yield1(c, b, a[0]) } }
}
fn wm_keys(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let v = c.hash_pairs(wm_h(c, s)).into_iter().map(|p| p.0).collect(); Ok(c.ary_from_vec(v)) }
fn wm_values(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let v = c.hash_pairs(wm_h(c, s)).into_iter().map(|p| p.1).collect(); Ok(c.ary_from_vec(v)) }
/// The pairs of the map's table, slot by slot (the block may change it).
fn wm_walk(c: &mut Ctx, s: Value, mut f: impl FnMut(&mut Ctx, Value, Value) -> Result<(), Esc>) -> Result<(), Esc> {
    let h = wm_h(c, s);
    let mut i = 0;
    while let Some((k, v)) = c.hash_entry_at(h, i) { if k != UNDEF { f(c, k, v)?; } i += 1; }
    Ok(())
}
fn wm_each(c: &mut Ctx, s: Value, _a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return err(c, c.vm().c_local_jump_error.get(), "no block given (yield)"); }
    let name = c.cur_method_name().unwrap_or(crate::symbol::s::each);
    wm_walk(c, s, |c, k, v| {
        match sym_bytes(name) { b"each_key" => { yield1(c, b, k)?; } b"each_value" => { yield1(c, b, v)?; } _ => { yield_block(c, b, &[k, v])?; } }
        Ok(())
    })?;
    Ok(s)
}
fn wm_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let mut parts = vec![];
    // as weakmap.c: immediates by #inspect, objects by rb_any_to_s
    let show = |c: &mut Ctx, v: Value| if v.is_imm() { inspect_str(c, v) } else { Ok(crate::builtins::kernel::any_to_s_str(c, v)) };
    wm_walk(c, s, |c, k, v| { parts.push(format!("{} => {}", show(c, k)?, show(c, v)?)); Ok(()) })?;
    Ok(c.str_new(format!("#<{}:0x{:016x}{}{}>", class_path(c, real_class_of(c, s)), s.raw(), if parts.is_empty() { "" } else { ": " }, parts.join(", ")).as_bytes()))
}
fn wkm_init(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let h = c.hash_new(); ivar_set_raw(c, s, intern(b"@h"), h); Ok(NIL) }
fn wkm_aref(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { Ok(hash_get_opt(c, wm_h(c, s), a[0])?.unwrap_or(NIL)) }
fn wkm_aset(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0].is_imm() || is_float(a[0]) || is_bignum(a[0]) { return arg_err(c, "WeakKeyMap keys must be garbage collectable"); }
    hash_set(c, wm_h(c, s), a[0], a[1])?;
    Ok(a[1])
}
fn wkm_getkey(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { Ok(match hash_lookup(c, wm_h(c, s), a[0])? { Some(i) => c.hash_entry_at(wm_h(c, s), i).map(|e| e.0).unwrap_or(NIL), None => NIL }) }
fn wkm_key_p(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(hash_lookup(c, wm_h(c, s), a[0])?.is_some())) }
fn wkm_clear(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { c.hash_clear(wm_h(c, s)); Ok(s) }
fn wkm_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let n = c.hash_len(wm_h(c, s)); Ok(c.str_new(format!("#<{}:0x{:016x} size={}>", class_path(c, real_class_of(c, s)), s.raw(), n).as_bytes())) }
fn p_setrlimit(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 2, 3)?;
    let res = num_to_long(c, a[0])? as i32;
    let lim = |c: &mut Ctx, v: Value| -> Result<u64, Esc> { if is_bignum(v) { Ok(u64::MAX) } else { Ok(num_to_long(c, v)? as u64) } };
    let cur = lim(c, a[1])?;
    let max = match a.get(2) { Some(&m) => lim(c, m)?, None => cur };
    if sys::setrlimit(res, &sys::Rlimit { cur, max }) < 0 { return Err(raise_errno(c, sys::errno(), "setrlimit")); }
    Ok(NIL)
}
fn st_s_wait(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let r = funcall(c, c.vm().m_process.get(), intern(b"wait2"), a, std::ptr::null())?;
    Ok(if is_array(r) { c.ary_get(r, 1) } else { NIL })
}

pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let md = vmr.c_module.get();
    define_cfunc(c, md, "deprecate_constant", m_deprecate_constant, -1);
    define_cfunc(c, md, "remove_class_variable", m_remove_cvar, 1);
    define_cfunc(c, md, "undefined_instance_methods", m_undefined_instance_methods, 0);
    define_cfunc(c, vmr.c_load_error.get(), "path", le_path, 0);
    define_cfunc(c, vmr.c_syntax_error.get(), "path", le_path, 0);
    define_cfunc(c, vmr.c_no_method_error.get(), "private_call?", nme_private_call, 0);
    let se = vmr.c_signal_exception.get();
    define_private_cfunc(c, se, "initialize", sig_initialize, -1);
    define_cfunc(c, se, "signo", sig_signo, 0); define_cfunc(c, se, "signm", sig_signm, 0);
    define_module_function(c, vmr.m_math.get(), "expm1", m_expm1, 1);
    define_module_function(c, vmr.m_math.get(), "log1p", m_log1p, 1);
    for (n, f) in [("erf", m_erf as CFunc), ("erfc", m_erfc), ("cbrt", m_cbrt), ("gamma", m_gamma), ("lgamma", m_lgamma)] { define_module_function(c, vmr.m_math.get(), n, f, 1); }
    let gc = vmr.m_gc.get();
    define_module_function(c, gc, "auto_compact", |_c, _s, _a, _b| Ok(FALSE), 0);
    define_module_function(c, gc, "auto_compact=", |_c, _s, a, _b| Ok(a[0]), 1);
    define_module_function(c, gc, "measure_total_time", |_c, _s, _a, _b| Ok(TRUE), 0);
    define_module_function(c, gc, "measure_total_time=", |_c, _s, a, _b| Ok(a[0]), 1);
    define_module_function(c, gc, "total_time", |_c, _s, _a, _b| Ok(Value::fix(crate::gc::total_time().as_nanos() as i64)), 0);
    define_module_function(c, gc, "config", gc_config, -1);
    define_module_function(c, gc, "stat_heap", gc_stat_heap, -1);
    define_module_function(c, gc, "latest_gc_info", |c, _s, _a, _b| Ok(c.hash_new()), -1);
    define_module_function(c, gc, "verify_internal_consistency", |_c, _s, _a, _b| Ok(NIL), 0);
    let k = vmr.m_kernel.get();
    define_module_function(c, k, "global_variables", k_global_variables, 0);
    define_module_function(c, k, "test", k_test, -1);
    define_module_function(c, k, "gem", |_c, _s, _a, _b| Ok(TRUE), -1);
    define_cfunc(c, vmr.c_io.get(), "wait", io_wait, -1);
    define_cfunc(c, vmr.c_io.get(), "each_codepoint", io_each_codepoint, 0);
    let wm = vmr.c_object_space_weak_map.get();
    include_module(c, wm, vmr.m_enumerable.get());
    for n in ["size", "length"] { define_cfunc(c, wm, n, wm_size, 0); }
    define_cfunc(c, wm, "delete", wm_delete, 1); define_cfunc(c, wm, "keys", wm_keys, 0); define_cfunc(c, wm, "values", wm_values, 0);
    for n in ["each", "each_pair", "each_key", "each_value"] { define_cfunc(c, wm, n, wm_each, 0); }
    define_cfunc(c, wm, "inspect", wm_inspect, 0);
    define_cfunc(c, wm, "member?", |c, s, a, _b| Ok(Value::bool(hash_lookup(c, wm_h(c, s), a[0])?.is_some())), 1);
    let wk = define_class_under(c, vmr.m_object_space.get(), "WeakKeyMap", vmr.c_object.get());
    define_private_cfunc(c, wk, "initialize", wkm_init, 0);
    define_cfunc(c, wk, "[]", wkm_aref, 1); define_cfunc(c, wk, "[]=", wkm_aset, 2); define_cfunc(c, wk, "getkey", wkm_getkey, 1); define_cfunc(c, wk, "key?", wkm_key_p, 1);
    define_cfunc(c, wk, "delete", wm_delete, 1); define_cfunc(c, wk, "clear", wkm_clear, 0); define_cfunc(c, wk, "inspect", wkm_inspect, 0);
    let p = vmr.m_process.get();
    define_module_function(c, p, "setrlimit", p_setrlimit, -1);
    define_module_function(c, p, "getsid", |_c, _s, _a, _b| Ok(Value::fix(sys::getsid(0) as i64)), -1);
    define_module_function(c, p, "getpriority", |_c, _s, _a, _b| Ok(Value::fix(0)), 2);
    define_module_function(c, p, "setpgrp", |_c, _s, _a, _b| { sys::setpgid(0, 0); Ok(Value::fix(0)) }, 0);
    define_module_function(c, p, "maxgroups", |_c, _s, _a, _b| Ok(Value::fix(65536)), 0);
    let st = const_get_at(c, p, intern(b"Status")).unwrap();
    define_singleton_cfunc(c, st, "wait", st_s_wait, -1);
    define_singleton_cfunc(c, vmr.c_thread.get(), "ignore_deadlock", |_c, _s, _a, _b| Ok(Value::bool(crate::builtins::thread::IGNORE_DEADLOCK.load(std::sync::atomic::Ordering::Relaxed))), 0);
    define_singleton_cfunc(c, vmr.c_thread.get(), "ignore_deadlock=", |_c, _s, a, _b| { crate::builtins::thread::IGNORE_DEADLOCK.store(a[0].truthy(), std::sync::atomic::Ordering::Relaxed); Ok(a[0]) }, 1);
}
