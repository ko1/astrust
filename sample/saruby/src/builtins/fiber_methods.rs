// Fiber.new(blocking:, storage:), fiber storage (Fiber[] / []= / #storage), blocking fibers and
// Fiber#inspect; the switching itself is fiber.rs.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::builtins::kernel::*;
use crate::builtins::thread::key_sym;
use crate::fiber::{current, storage, set_storage};
use crate::hash::*;
use crate::heap::is_frozen;
use crate::object::*;
use crate::value::*;
use crate::vm::*;
use crate::symbol::intern;

fn dup(c: &mut Ctx, v: Value) -> RV { if v == NIL { Ok(NIL) } else { crate::builtins::object::obj_dup(c, v, false, None) } }

fn validate(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    if v == NIL { return Ok(()); }
    if !is_hash(v) { return type_err(c, "storage must be a hash"); }
    if is_frozen(v) { let k = c.vm().c_frozen_error.get(); return err(c, k, "storage must not be frozen"); }
    for (k, _) in c.hash_pairs(v) {
        if !k.is_sym() { return type_err(c, format!("wrong argument type {} (expected Symbol)", class_name_for_err(c, k))); }
    }
    Ok(())
}

/// A copy of the running fiber's storage, for a new fiber or thread.
pub fn inherited_storage(c: &mut Ctx) -> RV { let f = current(c); let s = storage(c, f); dup(c, s) }

fn f_s_new(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let mut blocking = false;
    let mut st = UNDEF;
    if let Some(h) = kw_arg(a) {
        let (kb, kp, ks) = (sym_value("blocking"), sym_value("pool"), sym_value("storage"));
        let mut unknown = vec![];
        for (k, v) in c.hash_pairs(h) {
            if k == kb { blocking = v.truthy(); } else if k == ks { st = v; } else if k != kp { unknown.push(inspect_str(c, k)?); }
        }
        if !unknown.is_empty() { return arg_err(c, format!("unknown keyword{}: {}", if unknown.len() > 1 { "s" } else { "" }, unknown.join(", "))); }
    }
    if b.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
    let p = block_to_proc(c, b)?;
    let st = if st == UNDEF || st == TRUE { inherited_storage(c)? } else { validate(c, st)?; dup(c, st)? };
    Ok(crate::fiber::new_user_fiber(c, slf, p, blocking, st))
}

fn f_s_aref(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let k = key_sym(c, a[0])?;
    let h = storage(c, current(c));
    if h == NIL { return Ok(NIL); }
    Ok(hash_get_opt(c, h, k)?.unwrap_or(NIL))
}

fn f_s_aset(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (k, v) = (key_sym(c, a[0])?, a[1]);
    let f = current(c);
    let mut h = storage(c, f);
    if h == NIL {
        if v == NIL { return Ok(NIL); }
        h = c.hash_new();
        set_storage(c, f, h);
    }
    if v == NIL { return Ok(hash_delete(c, h, k)?.unwrap_or(NIL)); }
    hash_aset(c, h, k, v)?;
    Ok(v)
}

fn same_fiber(c: &mut Ctx, f: Value) -> Result<(), Esc> {
    if f != current(c) { return arg_err(c, "Fiber storage can only be accessed from the Fiber it belongs to"); }
    Ok(())
}

fn f_storage(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    same_fiber(c, slf)?;
    let h = storage(c, slf);
    dup(c, h)
}

fn f_storage_set(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if let (Some(f), true) = (c.caller_frame(), crate::builtins::misc::warning_category_enabled(1)) {
        let msg = format!("{}:{}: warning: Fiber#storage= is experimental and may be removed in the future!\n", frame_file(f), f.line());
        warn_str(c, &msg);
    }
    same_fiber(c, slf)?;
    validate(c, a[0])?;
    let h = dup(c, a[0])?;
    set_storage(c, slf, h);
    Ok(a[0])
}

fn f_blocking_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(crate::fiber::is_blocking(c, slf))) }

/// The thread's blocking count, which is 1 while a blocking fiber runs (CRuby th->blocking).
fn f_s_blocking_p(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    Ok(if crate::fiber::is_blocking(c, current(c)) { Value::fix(1) } else { FALSE })
}

fn f_s_blocking(c: &mut Ctx, _s: Value, _a: &[Value], b: *const Block) -> RV {
    let f = current(c);
    if crate::fiber::is_blocking(c, f) { return yield1(c, b, f); }
    crate::fiber::set_blocking(c, f, true);
    let r = yield1(c, b, f);
    crate::fiber::set_blocking(c, f, false);
    r
}

/// Fiber.set_scheduler(s): the old one is closed first.
fn f_s_set_scheduler(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { crate::builtins::thread::set_scheduler(c, a[0]) }
fn f_s_scheduler(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(crate::builtins::thread::thread_scheduler(c)) }
fn f_s_current_scheduler(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(crate::builtins::thread::scheduler_current(c)) }
/// Fiber.schedule(*args, &b): the scheduler's #fiber makes and starts the fiber (rb_fiber_s_schedule).
fn f_s_schedule(c: &mut Ctx, _s: Value, a: &[Value], b: *const Block) -> RV {
    let s = crate::builtins::thread::thread_scheduler(c);
    if s == NIL { return err(c, c.vm().c_runtime_error.get(), "No scheduler is available!"); }
    match kw_arg(a) {
        Some(_) => funcall_kw(c, s, intern(b"fiber"), a, b, true),
        None => funcall(c, s, intern(b"fiber"), a, b),
    }
}

/// CRuby fiber_to_s: "#<Fiber:0x... file:line (status)>", no location for a thread's root fiber.
fn f_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let (st, resuming) = crate::fiber::status(c, slf);
    let p = crate::fiber::first_proc(c, slf);
    let loc = if p != NIL && is_proc(p) {
        match c.proc_data(p).blk.kind { BlockKind::Iseq(is) => c.iseq_ref(is).map_or(String::new(), |is| format!(" {}:{}", is.info.file, is.info.line)), _ => String::new() }
    } else { String::new() };
    let t = format!("#<{}:{:#018x}{} ({}{})>", class_path(c, real_class_of(c, slf)), slf.raw(), loc, st, if resuming { " by resuming" } else { "" });
    Ok(c.str_new(t.as_bytes()))
}

pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let k = vmr.c_fiber.get();
    define_singleton_cfunc(c, k, "new", f_s_new, -1);
    define_singleton_cfunc(c, k, "[]", f_s_aref, 1);
    define_singleton_cfunc(c, k, "[]=", f_s_aset, 2);
    define_singleton_cfunc(c, k, "blocking?", f_s_blocking_p, 0);
    define_singleton_cfunc(c, k, "blocking", f_s_blocking, 0);
    define_singleton_cfunc(c, k, "set_scheduler", f_s_set_scheduler, 1);
    define_singleton_cfunc(c, k, "scheduler", f_s_scheduler, 0);
    define_singleton_cfunc(c, k, "current_scheduler", f_s_current_scheduler, 0);
    define_singleton_cfunc(c, k, "schedule", f_s_schedule, -1);
    define_cfunc(c, k, "storage", f_storage, 0);
    define_cfunc(c, k, "storage=", f_storage_set, 1);
    define_cfunc(c, k, "blocking?", f_blocking_p, 0);
    define_cfunc(c, k, "inspect", f_inspect, 0);
    define_cfunc(c, k, "to_s", f_inspect, 0);
}
