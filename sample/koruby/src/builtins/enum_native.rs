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


// ---- native Enumerable (hot methods): iterate with #each and a Rust block ----------------------
// The packed element is CRuby's ENUM_WANT_SVALUE: 0 args -> nil, 1 -> it, n -> [args].
use crate::array::*;
use crate::symbol::s;

#[inline]
fn pack(args: &[Value]) -> Value { match args.len() { 0 => NIL, 1 => args[0], _ => ary_new_from(args) } }
#[inline]
fn ublk(d: Value, i: usize) -> *const Block { ary_get(d, i).as_fix() as usize as *const Block }
#[inline]
fn bptr(b: *const Block) -> Value { Value::fix(b as usize as i64) }

/// Run recv.each with a Rust block; Ok(true) if it was stopped early by `enum_stop`.
fn run_each(c: &mut Ctx, recv: Value, f: CBlockFn, data: Value, args: &[Value]) -> Result<bool, Esc> {
    let r = if args.is_empty() && is_range(recv) && obj(recv).klass == vm().c_range && !range_each_redefined() {
        // Integer ranges: no #each dispatch, no Ruby block frames
        match crate::range::range_parts(recv) {
            (b, e, x) if b.is_fixnum() && e.is_fixnum() => {
                let hi = if x { e.as_fix() - 1 } else { e.as_fix() };
                let mut i = b.as_fix();
                let mut r = Ok(NIL);
                while i <= hi { if let Err(e) = f(c, data, &[Value::fix(i)], std::ptr::null()) { r = Err(e); break; } i += 1; }
                r
            }
            _ => { let b = Block::cfunc(c, f, data); funcall(c, recv, s::each, args, &b) }
        }
    } else {
        let b = Block::cfunc(c, f, data);
        funcall(c, recv, s::each, args, &b)
    };
    match r {
        Ok(_) => Ok(false),
        Err(e) => {
            if c.esc == EscKind::Throw && c.esc_tag_v == data { c.esc = EscKind::None; return Ok(true); }
            Err(e)
        }
    }
}
fn range_each_redefined() -> bool {
    let me = lookup_method(vm().c_range, s::each);
    me.is_null() || me != unsafe { crate::builtins::range_methods::R_EACH_ME }
}
fn enum_stop(c: &mut Ctx, data: Value) -> RV { c.esc = EscKind::Throw; c.esc_tag_v = data; c.esc_val = NIL; Err(Esc) }

fn no_blk(c: &mut Ctx, slf: Value, mid: &str, a: &[Value]) -> RV {
    crate::builtins::enumerator::enum_for_sized(c, slf, crate::symbol::intern(mid.as_bytes()), a)
}

fn en_map(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "map", a); }
    let d = ary_new_from(&[ary_new(), bptr(blk)]);
    run_each(c, slf, |c, d, args, _| { let r = yield_block(c, ublk(d, 1), args)?; ary_push(ary_get(d, 0), r); Ok(NIL) }, d, &[])?;
    Ok(ary_get(d, 0))
}
fn en_flat_map(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "flat_map", a); }
    let d = ary_new_from(&[ary_new(), bptr(blk)]);
    run_each(c, slf, |c, d, args, _| {
        let r = yield_block(c, ublk(d, 1), args)?;
        let out = ary_get(d, 0);
        match if r.is_heap() { check_array(c, r)? } else { None } { Some(ra) => { let v = rary(ra).clone(); rary(out).extend(v); } None => ary_push(out, r) }
        Ok(NIL)
    }, d, &[])?;
    Ok(ary_get(d, 0))
}
fn en_filter_map(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "filter_map", a); }
    let d = ary_new_from(&[ary_new(), bptr(blk)]);
    run_each(c, slf, |c, d, args, _| { let r = yield_block(c, ublk(d, 1), args)?; if r.truthy() { ary_push(ary_get(d, 0), r); } Ok(NIL) }, d, &[])?;
    Ok(ary_get(d, 0))
}
fn select_impl(c: &mut Ctx, slf: Value, blk: *const Block, want: bool) -> RV {
    let d = ary_new_from(&[ary_new(), bptr(blk), Value::bool(want)]);
    run_each(c, slf, |c, d, args, _| {
        let v = pack(args);
        if yield1(c, ublk(d, 1), v)?.truthy() == ary_get(d, 2).truthy() { ary_push(ary_get(d, 0), v); }
        Ok(NIL)
    }, d, &[])?;
    Ok(ary_get(d, 0))
}
fn en_select(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "select", a); }
    select_impl(c, slf, blk, true)
}
fn en_reject(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "reject", a); }
    select_impl(c, slf, blk, false)
}
fn en_to_a(c: &mut Ctx, slf: Value, a: &[Value], _blk: *const Block) -> RV {
    let d = ary_new_from(&[ary_new()]);
    run_each(c, slf, |_c, d, args, _| { let v = pack(args); ary_push(ary_get(d, 0), v); Ok(NIL) }, d, a)?;
    Ok(ary_get(d, 0))
}
fn en_each_with_index(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "each_with_index", a); }
    let d = ary_new_from(&[Value::fix(0), bptr(blk)]);
    run_each(c, slf, |c, d, args, _| {
        let i = ary_get(d, 0);
        rary(d)[0] = Value::fix(i.as_fix() + 1);
        let v = pack(args);
        yield_block(c, ublk(d, 1), &[v, i])?;
        Ok(NIL)
    }, d, a)?;
    Ok(slf)
}
fn en_each_with_object(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "each_with_object", a); }
    let d = ary_new_from(&[a[0], bptr(blk)]);
    run_each(c, slf, |c, d, args, _| { let v = pack(args); yield_block(c, ublk(d, 1), &[v, ary_get(d, 0)])?; Ok(NIL) }, d, &[])?;
    Ok(a[0])
}
fn en_include(c: &mut Ctx, slf: Value, a: &[Value], _blk: *const Block) -> RV {
    let d = ary_new_from(&[a[0]]);
    let hit = run_each(c, slf, |c, d, args, _| {
        let v = pack(args);
        if crate::builtins::array_methods::rb_eq(c, v, ary_get(d, 0))? { return enum_stop(c, d); }
        Ok(NIL)
    }, d, &[])?;
    Ok(Value::bool(hit))
}
fn en_find(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, crate::symbol::intern(b"find"), a, NIL); }
    let d = ary_new_from(&[NIL, bptr(blk)]);
    let hit = run_each(c, slf, |c, d, args, _| {
        let v = pack(args);
        if yield1(c, ublk(d, 1), v)?.truthy() { rary(d)[0] = v; return enum_stop(c, d); }
        Ok(NIL)
    }, d, &[])?;
    if hit { return Ok(ary_get(d, 0)); }
    match a.first() { Some(&ifnone) if ifnone != NIL => funcall(c, ifnone, s::call, &[], std::ptr::null()), _ => Ok(NIL) }
}
fn en_count(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    let mode = if !a.is_empty() { 1 } else if !blk.is_null() { 2 } else { 0 };
    let d = ary_new_from(&[Value::fix(0), bptr(blk), Value::fix(mode), a.first().copied().unwrap_or(NIL)]);
    run_each(c, slf, |c, d, args, _| {
        let hit = match ary_get(d, 2).as_fix() {
            0 => true,
            1 => { let v = pack(args); crate::builtins::array_methods::rb_eq(c, v, ary_get(d, 3))? }
            _ => yield_block(c, ublk(d, 1), args)?.truthy(),
        };
        if hit { rary(d)[0] = Value::fix(ary_get(d, 0).as_fix() + 1); }
        Ok(NIL)
    }, d, &[])?;
    Ok(ary_get(d, 0))
}
fn en_group_by(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return no_blk(c, slf, "group_by", a); }
    let d = ary_new_from(&[crate::hash::hash_new(), bptr(blk)]);
    run_each(c, slf, |c, d, args, _| {
        let v = pack(args);
        let k = yield1(c, ublk(d, 1), v)?;
        let h = ary_get(d, 0);
        match crate::hash::hash_get_opt(c, h, k)? {
            Some(arr) => ary_push(arr, v),
            None => { let arr = ary_new_from(&[v]); crate::hash::hash_aset(c, h, k, arr)?; }
        }
        Ok(NIL)
    }, d, &[])?;
    Ok(ary_get(d, 0))
}
fn en_inject(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if a.len() > 2 || (a.is_empty() && blk.is_null()) { return argc_err(c, a.len(), 1, 2); }
    // d = [acc (UNDEF = first element pending), blk, op (nil = block)]
    let (acc, op) = match (a.len(), blk.is_null()) {
        (2, _) => (a[0], a[1]),
        (1, true) => (UNDEF, a[0]),
        (1, false) => (a[0], NIL),
        _ => (UNDEF, NIL),
    };
    let op = if op == NIL || op.is_sym() { op } else if is_string(op) { Value::sym(crate::symbol::intern(&rstr(op).b)) } else {
        let ins = crate::builtins::kernel::inspect_str(c, op)?;
        return type_err(c, format!("{} is not a symbol nor a string", ins));
    };
    let d = ary_new_from(&[acc, bptr(blk), op]);
    run_each(c, slf, |c, d, args, _| {
        let v = pack(args);
        let acc = ary_get(d, 0);
        if acc == UNDEF { rary(d)[0] = v; return Ok(NIL); }
        let op = ary_get(d, 2);
        let r = if op == NIL { yield_block(c, ublk(d, 1), &[acc, v])? } else {
            let m = op.as_sym();
            if m == s::plus && acc.is_fixnum() && v.is_fixnum() {
                match acc.as_fix().checked_add(v.as_fix()) { Some(x) if Value::fixable(x) => Value::fix(x), _ => funcall_public(c, acc, m, &[v], std::ptr::null())? }
            } else { funcall_public(c, acc, m, &[v], std::ptr::null())? }
        };
        rary(d)[0] = r;
        Ok(NIL)
    }, d, &[])?;
    let r = ary_get(d, 0);
    Ok(if r == UNDEF { NIL } else { r })
}

fn slice_cons(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block, cons: bool) -> RV {
    let n = num_to_long(c, a[0])?;
    if n <= 0 { return arg_err(c, if cons { "invalid size" } else { "invalid slice size" }); }
    let mid = if cons { "each_cons" } else { "each_slice" };
    if blk.is_null() {
        let mut size = NIL;
        if respond_to(slf, s::size, true) {
            let sz = funcall(c, slf, s::size, &[], std::ptr::null())?;
            size = if sz.is_fixnum() { Value::fix(if cons { (sz.as_fix() - n + 1).max(0) } else { (sz.as_fix() + n - 1) / n }) } else { sz };
        }
        return crate::builtins::enumerator::enum_for(c, slf, crate::symbol::intern(mid.as_bytes()), &[Value::fix(n)], size);
    }
    let d = ary_new_from(&[ary_new(), bptr(blk), Value::fix(n), Value::bool(cons)]);
    run_each(c, slf, |c, d, args, _| {
        let v = pack(args);
        let buf = ary_get(d, 0);
        let n = ary_get(d, 2).as_fix() as usize;
        ary_push(buf, v);
        if ary_get(d, 3).truthy() {
            if ary_len(buf) > n { rary(buf).remove(0); }
            if ary_len(buf) == n { let cp = ary_new_from(rary(buf)); yield1(c, ublk(d, 1), cp)?; }
        } else if ary_len(buf) == n {
            rary(d)[0] = ary_new();
            yield1(c, ublk(d, 1), buf)?;
        }
        Ok(NIL)
    }, d, &[])?;
    let buf = ary_get(d, 0);
    if !cons && ary_len(buf) > 0 { yield1(c, blk, buf)?; }
    Ok(slf)
}
fn en_each_slice(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV { slice_cons(c, slf, a, blk, false) }
fn en_each_cons(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV { slice_cons(c, slf, a, blk, true) }

fn define_enumerable_natives() {
    let m0 = vm().m_enumerable;
    define_cfunc(m0, "each_slice", en_each_slice, 1); define_cfunc(m0, "each_cons", en_each_cons, 1);
    let m = vm().m_enumerable;
    define_cfunc(m, "map", en_map, 0); define_cfunc(m, "collect", en_map, 0);
    define_cfunc(m, "flat_map", en_flat_map, 0); define_cfunc(m, "collect_concat", en_flat_map, 0);
    define_cfunc(m, "filter_map", en_filter_map, 0);
    define_cfunc(m, "select", en_select, 0); define_cfunc(m, "filter", en_select, 0); define_cfunc(m, "find_all", en_select, 0);
    define_cfunc(m, "reject", en_reject, 0);
    define_cfunc(m, "to_a", en_to_a, -1); define_cfunc(m, "entries", en_to_a, -1);
    define_cfunc(m, "each_with_index", en_each_with_index, -1);
    define_cfunc(m, "each_with_object", en_each_with_object, 1);
    define_cfunc(m, "include?", en_include, 1); define_cfunc(m, "member?", en_include, 1);
    define_cfunc(m, "find", en_find, -1); define_cfunc(m, "detect", en_find, -1);
    define_cfunc(m, "count", en_count, -1);
    define_cfunc(m, "group_by", en_group_by, 0);
    define_cfunc(m, "inject", en_inject, -1); define_cfunc(m, "reduce", en_inject, -1);
}

pub fn init() {
    define_enumerable_natives();
    let m = vm().m_enumerable;
    define_singleton_cfunc(m, "__int", e_int, 1);
    define_singleton_cfunc(m, "__hash", e_hash, 1);
    define_singleton_cfunc(m, "__argc", e_argc, 3);
    define_singleton_cfunc(m, "__cmp", e_cmp, 2);
    define_singleton_cfunc(m, "__range_sum", |c, _s, a, b| crate::builtins::range_methods::r_sum(c, a[0], &a[1..], b), 2);
    define_singleton_cfunc(m, "__ivget", e_ivget, 2);
    define_singleton_cfunc(m, "__ivset", e_ivset, 3);
}
