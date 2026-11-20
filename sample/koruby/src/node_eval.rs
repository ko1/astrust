// This file is auto-generated from node.def by astrustgen. Do not edit.
// koruby — node.def (ASTrust).  Bodies are Rust.
//
// Every dispatcher is   fn(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV
// fp points at the current frame's locals on the VM value stack: fp[0] = self,
// fp[1] = env (heap Env for variables captured by blocks, or the outer env), then
// parameters and locals.  Calls stage [recv, -, args...] at c.sp; the callee's
// frame starts there, so arguments land directly in the callee's parameter slots.
//
// Fast paths live here (they get inlined into specialized code); slow paths are
// out-of-line functions in rt.rs.

use crate::vm::*;
use crate::value::*;
use crate::object::*;
use crate::rt::*;
use crate::node::*;
use crate::compile::{IseqRef, CiRef, Str, LitCache, CallInfo};
use crate::symbol::s;

// ---- literals ---------------------------------------------------------------
















// items with kinds: 0 = element, 1 = *splat

// pairs: k0 v0 k1 v1 ...; kinds per pair: 0 = pair, 1 = **splat (single node)



// ---- variables ----------------------------------------------------------------



















// ---- control flow -----------------------------------------------------------
















// begin/rescue: clauses are node_resbody, which answer UNDEF when they do not match

// matches: list of class expressions (empty = StandardError); splats flagged in kinds


// case/when: one `when` clause's test against the subject in a hidden local


// ---- calls ----------------------------------------------------------------------



// general call: splats, keywords, block-pass (described by ci)







// ---- specialized operators (fixnum / float fast paths) ---------------------------------

















// ---- definitions -------------------------------------------------------------------









// ---- multiple assignment -------------------------------------------------------------------

// rhs → Array stored in tmp slot; targets read it through node_masgn_elem

// kind: 0 = element idx (from front), 1 = splat [idx, len - post), 2 = element from back (post)


// the value of a method-body-level `__method__`, etc. are cfuncs; this node evaluates an
// arbitrary closure-free runtime helper by id (rarely used constructs)



#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_nil(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV {
    Ok(NIL)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_true(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV {
    Ok(TRUE)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_false(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV {
    Ok(FALSE)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_self(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV {
    Ok(unsafe { *fp })
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_lit(c: &mut Ctx, n: NodePtr, fp: *mut Value, v: Value) -> RV {
    Ok(v)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_float(c: &mut Ctx, n: NodePtr, fp: *mut Value, v: f64) -> RV {
    Ok(crate::numeric::float_new(v))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_str(c: &mut Ctx, n: NodePtr, fp: *mut Value, s: Str, enc: u32) -> RV {
    Ok(crate::string::str_new_enc(s, enc as u8))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_fstr(c: &mut Ctx, n: NodePtr, fp: *mut Value, s: Str, enc: u32, lc: &mut LitCache) -> RV {
    if lc.v != 0 { return Ok(unsafe { Value::from_raw(lc.v) }); }
    Ok(rt_fstr(s, enc, lc))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_dstr<D_parts: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, parts: NodeList, enc: u32) -> RV {
    let acc = crate::string::str_new_enc(b"", enc as u8);
    for i in 0..<D_parts as DispList>::len(parts) {
        let v = <D_parts as DispList>::nth(c, parts, i, fp)?;
        rt_dstr_append(c, acc, v)?;
    }
    Ok(acc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_tostr(c: &mut Ctx, n: NodePtr, fp: *mut Value, v: Value) -> RV {
    if is_string(v) { return Ok(v); }
    rt_tostr(c, v)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_dsym(c: &mut Ctx, n: NodePtr, fp: *mut Value, s: Value) -> RV {
    Ok(crate::string::str_to_sym(s))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_regex(c: &mut Ctx, n: NodePtr, fp: *mut Value, src: Str, opts: u32, lc: &mut LitCache) -> RV {
    if lc.v != 0 { return Ok(unsafe { Value::from_raw(lc.v) }); }
    rt_regex_lit(c, src, opts, lc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_dregex(c: &mut Ctx, n: NodePtr, fp: *mut Value, s: Value, opts: u32, once: bool, lc: &mut LitCache) -> RV {
    if once && lc.v != 0 { return Ok(unsafe { Value::from_raw(lc.v) }); }
    rt_dregex(c, s, opts, once, lc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_biglit(c: &mut Ctx, n: NodePtr, fp: *mut Value, digits: Str, lc: &mut LitCache) -> RV {
    if lc.v != 0 { return Ok(unsafe { Value::from_raw(lc.v) }); }
    Ok(rt_biglit(digits, lc))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_ary<D_elems: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, elems: NodeList) -> RV {
    let k = <D_elems as DispList>::len(elems);
    let base = c.sp;
    c.sp = unsafe { base.add(k) };
    for i in 0..k {
        let v = <D_elems as DispList>::nth(c, elems, i, fp)?;
        unsafe { *base.add(i) = v; }
    }
    let a = crate::array::ary_new_from(unsafe { std::slice::from_raw_parts(base, k) });
    c.sp = base;
    Ok(a)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_ary_build<D_elems: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, elems: NodeList, kinds: Str) -> RV {
    let a = crate::array::ary_new();
    let base = c.sp;
    unsafe { *base = a; c.sp = base.add(1); }
    for i in 0..<D_elems as DispList>::len(elems) {
        let v = <D_elems as DispList>::nth(c, elems, i, fp)?;
        if kinds[i] == 1 { rt_splat_into(c, a, v)?; } else { crate::array::ary_push(a, v); }
    }
    c.sp = base;
    Ok(a)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_hash<D_items: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, items: NodeList, kinds: Str) -> RV {
    let h = crate::hash::hash_new();
    let base = c.sp;
    unsafe { *base = h; c.sp = base.add(2); }
    let mut i = 0;
    let mut k = 0;
    while i < <D_items as DispList>::len(items) {
        if kinds[k] == 1 {
            let v = <D_items as DispList>::nth(c, items, i, fp)?;
            rt_hash_merge_kw(c, h, v)?;
            i += 1;
        } else {
            let key = <D_items as DispList>::nth(c, items, i, fp)?;
            unsafe { *base.add(1) = key; }
            let val = <D_items as DispList>::nth(c, items, i + 1, fp)?;
            crate::hash::hash_aset(c, h, key, val)?;
            i += 2;
        }
        k += 1;
    }
    c.sp = base;
    Ok(h)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_range(c: &mut Ctx, n: NodePtr, fp: *mut Value, b: Value, e: Value, excl: bool) -> RV {
    rt_range_new(c, b, e, excl)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_range_lit(c: &mut Ctx, n: NodePtr, fp: *mut Value, b: Value, e: Value, excl: bool, lc: &mut LitCache) -> RV {
    if lc.v != 0 { return Ok(unsafe { Value::from_raw(lc.v) }); }
    Ok(rt_range_lit(b, e, excl, lc))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_lvar(c: &mut Ctx, n: NodePtr, fp: *mut Value, idx: u32) -> RV {
    Ok(unsafe { *fp.add(idx as usize) })
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_lasgn(c: &mut Ctx, n: NodePtr, fp: *mut Value, idx: u32, v: Value) -> RV {
    unsafe { *fp.add(idx as usize) = v; }
    Ok(v)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_dvar(c: &mut Ctx, n: NodePtr, fp: *mut Value, hops: u32, idx: u32) -> RV {
    let env = env_at(unsafe { *fp.add(1) }, hops);
    Ok(env_get(env, idx))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_dasgn(c: &mut Ctx, n: NodePtr, fp: *mut Value, hops: u32, idx: u32, v: Value) -> RV {
    let env = env_at(unsafe { *fp.add(1) }, hops);
    env_set(env, idx, v);
    Ok(v)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_ivar(c: &mut Ctx, n: NodePtr, fp: *mut Value, name: Sym, ic: &mut IvarCache) -> RV {
    let slf = unsafe { *fp };
    if slf.is_heap() {
        let o = obj(slf);
        if o.klass.raw() == ic.klass {
            if let Body::Object(ivs) = &o.body {
                if let Some(&v) = ivs.get(ic.idx as usize) { return Ok(if v == UNDEF { NIL } else { v }); }
                return Ok(NIL);
            }
        }
    }
    Ok(rt_ivar_get(slf, name, ic))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_iasgn(c: &mut Ctx, n: NodePtr, fp: *mut Value, name: Sym, v: Value, ic: &mut IvarCache) -> RV {
    let slf = unsafe { *fp };
    if slf.is_heap() {
        let o = obj(slf);
        if o.klass.raw() == ic.klass && o.flags & FL_FROZEN == 0 {
            if let Body::Object(ivs) = &mut o.body {
                if let Some(x) = ivs.get_mut(ic.idx as usize) { *x = v; return Ok(v); }
            }
        }
    }
    rt_ivar_set(c, slf, name, v, ic)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_gvar(c: &mut Ctx, n: NodePtr, fp: *mut Value, name: Sym) -> RV {
    rt_gvar_get(c, name)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_gasgn(c: &mut Ctx, n: NodePtr, fp: *mut Value, name: Sym, v: Value) -> RV {
    rt_gvar_set(c, name, v)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_cvar(c: &mut Ctx, n: NodePtr, fp: *mut Value, name: Sym) -> RV {
    rt_cvar_get(c, name)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_cvasgn(c: &mut Ctx, n: NodePtr, fp: *mut Value, name: Sym, v: Value) -> RV {
    rt_cvar_set(c, name, v)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_const(c: &mut Ctx, n: NodePtr, fp: *mut Value, name: Sym, line: u32, cc: &mut ConstCache) -> RV {
    let cref = unsafe { (*c.frame).cref };
    if cc.serial == vm().const_serial && cc.cref == cref as usize { return Ok(unsafe { Value::from_raw(cc.val) }); }
    unsafe { (*c.frame).line = line; }
    rt_const_get(c, cref, name, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_const_path(c: &mut Ctx, n: NodePtr, fp: *mut Value, parent: Value, name: Sym, line: u32) -> RV {
    unsafe { (*c.frame).line = line; }
    rt_const_path(c, parent, name)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_top_const(c: &mut Ctx, n: NodePtr, fp: *mut Value, name: Sym) -> RV {
    rt_const_path(c, vm().c_object, name)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_cdecl(c: &mut Ctx, n: NodePtr, fp: *mut Value, name: Sym, v: Value) -> RV {
    rt_cdecl(c, NIL, name, v)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_cdecl_path(c: &mut Ctx, n: NodePtr, fp: *mut Value, parent: Value, name: Sym, v: Value) -> RV {
    rt_cdecl_path(c, parent, name, v)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_errinfo(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV {
    Ok(c.errinfo)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_nth_ref(c: &mut Ctx, n: NodePtr, fp: *mut Value, nth: u32) -> RV {
    Ok(crate::regexp::nth_match(c.last_match, nth as i64))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_back_ref(c: &mut Ctx, n: NodePtr, fp: *mut Value, kind: u32) -> RV {
    rt_back_ref(c, kind)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_block<D_stmts: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, stmts: NodeList) -> RV {
    let k = <D_stmts as DispList>::len(stmts);
    let mut i = 0;
    while i + 1 < k { <D_stmts as DispList>::nth(c, stmts, i, fp)?; i += 1; }
    <D_stmts as DispList>::nth(c, stmts, k - 1, fp)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_line<D_body: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, line: u32, body: NodePtr) -> RV {
    unsafe { (*c.frame).line = line; }
    <D_body as Disp>::call(c, body, fp)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_if<D_cond: Disp, D_then_b: Disp, D_else_b: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, cond: NodePtr, then_b: NodePtr, else_b: NodePtr) -> RV {
    if <D_cond as Disp>::call(c, cond, fp)?.truthy() { <D_then_b as Disp>::call(c, then_b, fp) } else { <D_else_b as Disp>::call(c, else_b, fp) }
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_and<D_a: Disp, D_b: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: NodePtr, b: NodePtr) -> RV {
    let v = <D_a as Disp>::call(c, a, fp)?;
    if v.falsy() { Ok(v) } else { <D_b as Disp>::call(c, b, fp) }
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_or<D_a: Disp, D_b: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: NodePtr, b: NodePtr) -> RV {
    let v = <D_a as Disp>::call(c, a, fp)?;
    if v.truthy() { Ok(v) } else { <D_b as Disp>::call(c, b, fp) }
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_not(c: &mut Ctx, n: NodePtr, fp: *mut Value, v: Value, cc: &mut CallCache) -> RV {
    if v.is_heap() && rt_has_custom_not(v) { return rt_call0(c, v, s::not, cc); }
    Ok(Value::bool(v.falsy()))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_while<D_cond: Disp, D_body: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, cond: NodePtr, body: NodePtr, until: bool) -> RV {
    loop {
        let t = <D_cond as Disp>::call(c, cond, fp)?.truthy();
        if t == until { break; }
        loop {
            match <D_body as Disp>::call(c, body, fp) {
                Ok(_) => break,
                Err(e) => match c.esc {
                    EscKind::LoopNext => { c.esc = EscKind::None; break; }
                    EscKind::LoopBreak => { c.esc = EscKind::None; return Ok(c.esc_val); }
                    EscKind::Redo => { c.esc = EscKind::None; }
                    _ => return Err(e),
                }
            }
        }
    }
    Ok(NIL)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_dowhile<D_cond: Disp, D_body: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, cond: NodePtr, body: NodePtr, until: bool) -> RV {
    loop {
        match <D_body as Disp>::call(c, body, fp) {
            Ok(_) => {}
            Err(e) => match c.esc {
                EscKind::LoopNext => { c.esc = EscKind::None; }
                EscKind::LoopBreak => { c.esc = EscKind::None; return Ok(c.esc_val); }
                _ => return Err(e),
            }
        }
        let t = <D_cond as Disp>::call(c, cond, fp)?.truthy();
        if t == until { break; }
    }
    Ok(NIL)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_loop_break(c: &mut Ctx, n: NodePtr, fp: *mut Value, v: Value) -> RV {
    c.esc = EscKind::LoopBreak;
    c.esc_val = v;
    Err(Esc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_loop_next(c: &mut Ctx, n: NodePtr, fp: *mut Value, v: Value) -> RV {
    c.esc = EscKind::LoopNext;
    c.esc_val = v;
    Err(Esc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_break(c: &mut Ctx, n: NodePtr, fp: *mut Value, v: Value) -> RV {
    rt_break(c, v)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_next(c: &mut Ctx, n: NodePtr, fp: *mut Value, v: Value) -> RV {
    c.esc = EscKind::Next;
    c.esc_val = v;
    Err(Esc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_return(c: &mut Ctx, n: NodePtr, fp: *mut Value, v: Value) -> RV {
    c.esc = EscKind::Return;
    c.esc_val = v;
    c.esc_tag = unsafe { (*c.frame).id };
    Err(Esc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_retry(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV {
    c.esc = EscKind::Retry;
    c.esc_val = NIL;
    Err(Esc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_redo(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV {
    c.esc = EscKind::Redo;
    c.esc_val = NIL;
    Err(Esc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_rescue<D_body: Disp, D_clauses: DispList, D_else_b: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, body: NodePtr, clauses: NodeList, else_b: NodePtr, has_else: bool) -> RV {
    let saved_sp = c.sp;
    let saved_err = c.errinfo;
    loop {
        match <D_body as Disp>::call(c, body, fp) {
            Ok(v) => { return if has_else { <D_else_b as Disp>::call(c, else_b, fp) } else { Ok(v) }; }
            Err(e) => {
                if c.esc != EscKind::Raise { return Err(e); }
                c.sp = saved_sp;
                let exc = c.esc_val;
                if !rt_rescuable(exc) { return Err(e); }
                c.esc = EscKind::None;
                c.errinfo = exc;
                let mut matched = false;
                let mut result = Ok(NIL);
                for i in 0..<D_clauses as DispList>::len(clauses) {
                    let r = <D_clauses as DispList>::nth(c, clauses, i, fp);
                    match r {
                        Ok(v) if v == UNDEF => continue,
                        other => { matched = true; result = other; break; }
                    }
                }
                if !matched {
                    c.errinfo = saved_err;
                    c.esc = EscKind::Raise;
                    c.esc_val = exc;
                    return Err(e);
                }
                if result.is_err() && c.esc == EscKind::Retry { c.esc = EscKind::None; c.errinfo = saved_err; continue; }
                if result.is_ok() { c.errinfo = saved_err; }
                return result;
            }
        }
    }
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_resbody<D_classes: DispList, D_assign: Disp, D_body: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, classes: NodeList, kinds: Str, assign: NodePtr, body: NodePtr) -> RV {
    let exc = c.errinfo;
    let mut ok = <D_classes as DispList>::len(classes) == 0 && rt_is_standard_error(exc);
    let mut i = 0;
    while !ok && i < <D_classes as DispList>::len(classes) {
        let k = <D_classes as DispList>::nth(c, classes, i, fp)?;
        ok = rt_rescue_match(c, k, exc, kinds[i] == 1)?;
        i += 1;
    }
    if !ok { return Ok(UNDEF); }
    <D_assign as Disp>::call(c, assign, fp)?;
    <D_body as Disp>::call(c, body, fp)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_ensure<D_body: Disp, D_ens: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, body: NodePtr, ens: NodePtr) -> RV {
    let r = <D_body as Disp>::call(c, body, fp);
    let (k, v, t, tv) = (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v);
    let saved_err = c.errinfo;
    if r.is_err() && k == EscKind::Raise { c.errinfo = v; }
    c.esc = EscKind::None;
    <D_ens as Disp>::call(c, ens, fp)?;
    c.errinfo = saved_err;
    c.esc = k; c.esc_val = v; c.esc_tag = t; c.esc_tag_v = tv;
    r
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_when<D_pats: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, subj_slot: u32, pats: NodeList, kinds: Str, cc: &mut CallCache) -> RV {
    let subj = unsafe { *fp.add(subj_slot as usize) };
    for i in 0..<D_pats as DispList>::len(pats) {
        let p = <D_pats as DispList>::nth(c, pats, i, fp)?;
        if kinds[i] == 1 {
            if rt_when_splat(c, p, subj)? { return Ok(TRUE); }
        } else if rt_eqq(c, p, subj, cc)? { return Ok(TRUE); }
    }
    Ok(FALSE)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_defined<D_recv: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, kind: u32, name: Sym, recv: NodePtr) -> RV {
    rt_defined::<D_recv>(c, n, fp, kind, name, recv)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_call<D_recv: Disp, D_args: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32, cc: &mut CallCache) -> RV {
    let k = <D_args as DispList>::len(args);
    let base = c.sp;
    unsafe {
        c.sp = base.add(2 + k);
        *base = <D_recv as Disp>::call(c, recv, fp)?;
        for i in 0..k { let v = <D_args as DispList>::nth(c, args, i, fp)?; *base.add(2 + i) = v; }
        (*c.frame).line = line;
    }
    call_cached(c, cc, mid, base, k as u32, std::ptr::null(), flags)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_call_blk<D_recv: Disp, D_args: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32, blk: IseqRef, cc: &mut CallCache) -> RV {
    let k = <D_args as DispList>::len(args);
    let base = c.sp;
    unsafe {
        c.sp = base.add(2 + k);
        *base = <D_recv as Disp>::call(c, recv, fp)?;
        for i in 0..k { let v = <D_args as DispList>::nth(c, args, i, fp)?; *base.add(2 + i) = v; }
        (*c.frame).line = line;
    }
    let b = make_block(c, blk, fp);
    let r = call_cached(c, cc, mid, base, k as u32, &b, flags);
    match r {
        Err(e) if c.esc == EscKind::Break && c.esc_tag == b.tag => { c.esc = EscKind::None; Ok(c.esc_val) }
        r => r,
    }
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_call_x<D_recv: Disp, D_args: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32, ci: CiRef, cc: &mut CallCache) -> RV {
    let base = c.sp;
    unsafe { c.sp = base.add(2); }
    let r0 = <D_recv as Disp>::call(c, recv, fp)?;
    unsafe { *base = r0; }
    let tmp = crate::array::ary_new();
    unsafe { *base.add(1) = tmp; }
    let mut blkv = NIL;
    let mut kw = NIL;
    for i in 0..<D_args as DispList>::len(args) {
        let v = <D_args as DispList>::nth(c, args, i, fp)?;
        rt_callx_arg(c, ci, i, v, tmp, &mut kw, &mut blkv)?;
    }
    unsafe { (*c.frame).line = line; }
    rt_call_x(c, cc, mid, base, tmp, kw, blkv, ci, flags, fp)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_attrasgn<D_recv: Disp, D_args: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32, cc: &mut CallCache) -> RV {
    let k = <D_args as DispList>::len(args);
    let base = c.sp;
    unsafe {
        c.sp = base.add(2 + k);
        *base = <D_recv as Disp>::call(c, recv, fp)?;
        for i in 0..k { let v = <D_args as DispList>::nth(c, args, i, fp)?; *base.add(2 + i) = v; }
        (*c.frame).line = line;
    }
    let val = unsafe { *base.add(1 + k) };
    call_cached(c, cc, mid, base, k as u32, std::ptr::null(), flags)?;
    Ok(val)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_super<D_args: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, args: NodeList, line: u32, ci: CiRef, zsuper: bool) -> RV {
    let base = c.sp;
    unsafe { c.sp = base.add(2); }
    let tmp = crate::array::ary_new();
    unsafe { *base = *fp; *base.add(1) = tmp; }
    let mut blkv = NIL;
    let mut kw = NIL;
    for i in 0..<D_args as DispList>::len(args) {
        let v = <D_args as DispList>::nth(c, args, i, fp)?;
        rt_callx_arg(c, ci, i, v, tmp, &mut kw, &mut blkv)?;
    }
    unsafe { (*c.frame).line = line; }
    rt_super(c, base, tmp, kw, blkv, ci, fp, zsuper)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_yield<D_args: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, args: NodeList) -> RV {
    let k = <D_args as DispList>::len(args);
    let blk = unsafe { (*c.frame).blk };
    if blk.is_null() { return rt_no_block(c); }
    let base = c.sp;
    unsafe {
        c.sp = base.add(2 + k);
        for i in 0..k { let v = <D_args as DispList>::nth(c, args, i, fp)?; *base.add(2 + i) = v; }
    }
    rt_yield_staged(c, blk, base, k as u32)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_yield_x<D_args: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, args: NodeList, ci: CiRef) -> RV {
    let base = c.sp;
    let tmp = crate::array::ary_new();
    unsafe { *base = tmp; c.sp = base.add(1); }
    let mut blkv = NIL;
    let mut kw = NIL;
    for i in 0..<D_args as DispList>::len(args) {
        let v = <D_args as DispList>::nth(c, args, i, fp)?;
        rt_callx_arg(c, ci, i, v, tmp, &mut kw, &mut blkv)?;
    }
    let r = rt_yield_x(c, tmp, kw);
    c.sp = base;
    r
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_block_given(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV {
    Ok(Value::bool(unsafe { !(*c.frame).blk.is_null() }))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_lambda(c: &mut Ctx, n: NodePtr, fp: *mut Value, blk: IseqRef) -> RV {
    rt_lambda(c, blk, fp)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_plus(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_PLUS) {
        if let Some(r) = a.as_fix().checked_add(b.as_fix()) { if Value::fixable(r) { return Ok(Value::fix(r)); } }
    }
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_PLUS) { return Ok(crate::numeric::float_new(a.flonum_val() + b.flonum_val())); }
    rt_binop(c, a, b, s::plus, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_minus(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_MINUS) {
        if let Some(r) = a.as_fix().checked_sub(b.as_fix()) { if Value::fixable(r) { return Ok(Value::fix(r)); } }
    }
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_MINUS) { return Ok(crate::numeric::float_new(a.flonum_val() - b.flonum_val())); }
    rt_binop(c, a, b, s::minus, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_mul(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_MUL) {
        if let Some(r) = a.as_fix().checked_mul(b.as_fix()) { if Value::fixable(r) { return Ok(Value::fix(r)); } }
    }
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_MUL) { return Ok(crate::numeric::float_new(a.flonum_val() * b.flonum_val())); }
    rt_binop(c, a, b, s::mul, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_div(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && b.as_fix() != 0 && !redef(R_INT_DIV) {
        let (x, y) = (a.as_fix(), b.as_fix());
        let q = if (x % y != 0) && ((x < 0) != (y < 0)) { x / y - 1 } else { x / y };
        if Value::fixable(q) { return Ok(Value::fix(q)); }
    }
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_DIV) { return Ok(crate::numeric::float_new(a.flonum_val() / b.flonum_val())); }
    rt_binop(c, a, b, s::div, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_mod(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && b.as_fix() != 0 && !redef(R_INT_MOD) {
        let (x, y) = (a.as_fix(), b.as_fix());
        let m = x % y;
        let m = if m != 0 && ((m < 0) != (y < 0)) { m + y } else { m };
        return Ok(Value::fix(m));
    }
    rt_binop(c, a, b, s::modulo, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_lt(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_LT) { return Ok(Value::bool(a.as_fix() < b.as_fix())); }
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_LT) { return Ok(Value::bool(a.flonum_val() < b.flonum_val())); }
    rt_binop(c, a, b, s::lt, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_le(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_LE) { return Ok(Value::bool(a.as_fix() <= b.as_fix())); }
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_LE) { return Ok(Value::bool(a.flonum_val() <= b.flonum_val())); }
    rt_binop(c, a, b, s::le, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_gt(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_GT) { return Ok(Value::bool(a.as_fix() > b.as_fix())); }
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_GT) { return Ok(Value::bool(a.flonum_val() > b.flonum_val())); }
    rt_binop(c, a, b, s::gt, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_ge(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_GE) { return Ok(Value::bool(a.as_fix() >= b.as_fix())); }
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_GE) { return Ok(Value::bool(a.flonum_val() >= b.flonum_val())); }
    rt_binop(c, a, b, s::ge, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_eq(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_EQ) { return Ok(Value::bool(a == b)); }
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_EQ) { return Ok(Value::bool(a.flonum_val() == b.flonum_val())); }
    if a.is_sym() && b.is_sym() { return Ok(Value::bool(a == b)); }
    rt_eq(c, a, b, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_neq(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_EQ) { return Ok(Value::bool(a != b)); }
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_EQ) { return Ok(Value::bool(a.flonum_val() != b.flonum_val())); }
    rt_binop(c, a, b, s::neq, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_ltlt(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if is_array(a) && !redef(R_ARY_LTLT) && obj(a).flags & FL_FROZEN == 0 && obj(a).klass == vm().c_array {
        crate::array::ary_push(a, b);
        return Ok(a);
    }
    rt_binop(c, a, b, s::ltlt, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_and(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_AND) { return Ok(Value::fix(a.as_fix() & b.as_fix())); }
    rt_binop(c, a, b, s::and, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_or(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !redef(R_INT_OR) { return Ok(Value::fix(a.as_fix() | b.as_fix())); }
    rt_binop(c, a, b, s::or, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_aref(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, line: u32, cc: &mut CallCache) -> RV {
    if b.is_fixnum() && is_array(a) && obj(a).klass == vm().c_array && !redef(R_ARY_AREF) {
        let v = rary(a);
        let mut i = b.as_fix();
        if i < 0 { i += v.len() as i64; }
        return Ok(if i >= 0 && (i as usize) < v.len() { v[i as usize] } else { NIL });
    }
    if is_hash(a) && obj(a).klass == vm().c_hash && !redef(R_HASH_AREF) { return crate::hash::hash_aref(c, a, b); }
    rt_binop(c, a, b, s::aref, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_aset(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: Value, b: Value, v: Value, line: u32, cc: &mut CallCache) -> RV {
    if b.is_fixnum() && is_array(a) && obj(a).klass == vm().c_array && obj(a).flags & FL_FROZEN == 0 && !redef(R_ARY_ASET) {
        let ary = rary(a);
        let i = b.as_fix();
        if i >= 0 && (i as usize) < ary.len() { ary[i as usize] = v; return Ok(v); }
    }
    if is_hash(a) && obj(a).klass == vm().c_hash && !redef(R_HASH_ASET) { crate::hash::hash_aset(c, a, b, v)?; return Ok(v); }
    rt_aset(c, a, b, v, line, cc)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_def(c: &mut Ctx, n: NodePtr, fp: *mut Value, mid: Sym, body: IseqRef) -> RV {
    rt_def(c, mid, body, fp)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_sdef(c: &mut Ctx, n: NodePtr, fp: *mut Value, recv: Value, mid: Sym, body: IseqRef) -> RV {
    rt_sdef(c, recv, mid, body)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_class<D_cpath: Disp, D_superclass: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, cpath: NodePtr, name: Sym, superclass: NodePtr, body: IseqRef, has_cpath: bool, has_super: bool) -> RV {
    let outer = if has_cpath { <D_cpath as Disp>::call(c, cpath, fp)? } else { UNDEF };
    let sup = if has_super { <D_superclass as Disp>::call(c, superclass, fp)? } else { UNDEF };
    rt_class(c, outer, name, sup, body, fp)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_module<D_cpath: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, cpath: NodePtr, name: Sym, body: IseqRef, has_cpath: bool) -> RV {
    let outer = if has_cpath { <D_cpath as Disp>::call(c, cpath, fp)? } else { UNDEF };
    rt_module(c, outer, name, body, fp)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_sclass(c: &mut Ctx, n: NodePtr, fp: *mut Value, recv: Value, body: IseqRef) -> RV {
    rt_sclass(c, recv, body, fp)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_alias(c: &mut Ctx, n: NodePtr, fp: *mut Value, newname: Value, oldname: Value) -> RV {
    rt_alias(c, newname, oldname)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_valias(c: &mut Ctx, n: NodePtr, fp: *mut Value, newname: Sym, oldname: Sym) -> RV {
    vm().global_aliases.insert(newname, oldname);
    Ok(NIL)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_undef<D_names: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, names: NodeList) -> RV {
    for i in 0..<D_names as DispList>::len(names) {
        let v = <D_names as DispList>::nth(c, names, i, fp)?;
        rt_undef(c, v)?;
    }
    Ok(NIL)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_masgn<D_targets: DispList>(c: &mut Ctx, n: NodePtr, fp: *mut Value, rhs: Value, tmp: u32, targets: NodeList, splat_rhs: bool) -> RV {
    let a = rt_masgn_array(c, rhs, splat_rhs)?;
    unsafe { *fp.add(tmp as usize) = a; }
    for i in 0..<D_targets as DispList>::len(targets) { <D_targets as DispList>::nth(c, targets, i, fp)?; }
    Ok(rhs)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_masgn_elem(c: &mut Ctx, n: NodePtr, fp: *mut Value, tmp: u32, idx: u32, kind: u32, post: u32) -> RV {
    let a = unsafe { *fp.add(tmp as usize) };
    Ok(rt_masgn_elem(a, idx, kind, post))
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_unsupported(c: &mut Ctx, n: NodePtr, fp: *mut Value, what: Str) -> RV {
    rt_unsupported(c, what)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_special<D_arg: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, kind: u32, arg: NodePtr) -> RV {
    rt_special::<D_arg>(c, n, fp, kind, arg)
}

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_flipflop<D_a: Disp, D_b: Disp>(c: &mut Ctx, n: NodePtr, fp: *mut Value, a: NodePtr, b: NodePtr, excl: bool, slot: u32) -> RV {
    let st = unsafe { *fp.add(slot as usize) };
    if st.falsy() {
        if <D_a as Disp>::call(c, a, fp)?.truthy() {
            if !excl && <D_b as Disp>::call(c, b, fp)?.truthy() { return Ok(TRUE); }
            unsafe { *fp.add(slot as usize) = TRUE; }
            return Ok(TRUE);
        }
        Ok(FALSE)
    } else {
        if <D_b as Disp>::call(c, b, fp)?.truthy() { unsafe { *fp.add(slot as usize) = FALSE; } }
        Ok(TRUE)
    }
}
