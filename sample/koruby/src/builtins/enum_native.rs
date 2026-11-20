// Native helpers for the Ruby-level Enumerable / Set / Data in prelude/core.rb.
use crate::builtins::numeric_methods::num_to_long;
use crate::value::*;
use crate::vm::*;

// Enumerable.__int(n): implicit Integer conversion (to_int) with CRuby's errors.
fn e_int(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    Ok(Value::fix(num_to_long(c, a[0])?))
}
// Enumerable.__hash(h): implicit Hash conversion (to_hash) or TypeError.
fn e_hash(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    match crate::hash::check_hash(c, a[0])? {
        Some(h) => Ok(h),
        None => type_err(c, format!("no implicit conversion of {} into Hash", crate::builtins::kernel::class_name_for_err(a[0]))),
    }
}
// Enumerable.__argc(given, min, max): ArgumentError with CRuby's wording (max -1 = unlimited).
fn e_argc(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    argc_err(c, a[0].as_fix() as usize, a[1].as_fix() as i32, a[2].as_fix() as i32)
}

// Enumerable.__cmp(a, b): a <=> b as -1/0/1, ArgumentError "comparison of X with Y failed" on nil.
fn e_cmp(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = crate::builtins::array_methods::rb_cmp(c, a[0], a[1])?;
    Ok(Value::fix(o as i64))
}

// Enumerable.__ivget / __ivset: hidden (non-@) instance slots for Data members; set ignores frozen.
fn e_ivget(_c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(a[0], a[1].as_sym())) }
fn e_ivset(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if !a[0].is_heap() || !a[1].is_sym() { return type_err(c, "bad hidden ivar"); }
    ivar_set_raw(a[0], a[1].as_sym(), a[2]);
    Ok(a[2])
}

/// Does recv's #each differ from `native` (a subclass or singleton overrode it)?
pub fn each_overridden(recv: Value, native: CFunc) -> bool {
    let me = lookup_method(class_of(recv), crate::symbol::s::each);
    if me.is_null() { return true; }
    match unsafe { (*me).kind } { MKind::Cfunc(f, _) => f as *const () as usize != native as *const () as usize, _ => true }
}

/// Call Enumerable's own implementation of mid on recv (for natives whose #each was overridden).
pub fn call_enumerable(c: &mut Ctx, recv: Value, mid: &str, args: &[Value], blk: *const Block) -> RV {
    let mid = crate::symbol::intern(mid.as_bytes());
    let me = lookup_method(vm().m_enumerable, mid);
    if me.is_null() { return funcall(c, recv, mid, args, blk); }
    let base = c.sp;
    if unsafe { base.add(args.len() + 2) } >= c.stack_end { return funcall(c, recv, mid, args, blk); }
    unsafe {
        *base = recv;
        *base.add(1) = NIL;
        for (i, a) in args.iter().enumerate() { *base.add(2 + i) = *a; }
        c.sp = base.add(2 + args.len());
    }
    let r = invoke(c, me, recv, base, args.len() as u32, blk, CF_FCALL);
    c.sp = base;
    r
}

pub fn init() {
    let m = vm().m_enumerable;
    define_singleton_cfunc(m, "__int", e_int, 1);
    define_singleton_cfunc(m, "__hash", e_hash, 1);
    define_singleton_cfunc(m, "__argc", e_argc, 3);
    define_singleton_cfunc(m, "__cmp", e_cmp, 2);
    define_singleton_cfunc(m, "__ivget", e_ivget, 2);
    define_singleton_cfunc(m, "__ivset", e_ivset, 3);
}
