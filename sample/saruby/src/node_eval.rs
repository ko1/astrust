// This file is auto-generated from node.def by astrustgen. Do not edit.
// saruby — node.def (ASTrust).  Bodies are Rust.
//
// Every dispatcher is   fn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV
// fp is the current frame on the VM value stack, a slice of slots (src/stack.rs): fp[0] = self,
// fp[1] = env (heap Env for variables captured by blocks, or the outer env), then
// parameters and locals.  Calls stage [recv, -, args...] at sp (`stage`); the callee's
// frame starts there, so arguments land directly in the callee's parameter slots.
//
// Fast paths live here (they get inlined into specialized code); slow paths are
// out-of-line functions in rt.rs.

use crate::vm::*;
use crate::value::*;
use crate::object::*;
use crate::rt::*;
use crate::opt::*;
use crate::inline_builtin::*;
use crate::inline_iter::*;
use crate::node::*;
use crate::compile::{IseqRef, CiRef, Str, LitCache, CallInfo};
use crate::symbol::s;

// ---- literals ---------------------------------------------------------------
















// items with kinds: 0 = element, 1 = *splat

// pairs: k0 v0 k1 v1 ...; kinds per pair: 0 = pair, 1 = **splat (single node)



// ---- variables ----------------------------------------------------------------





// slot (baked): 1 + the index the site's cache held, 0 none.  Baked code tests it against the cache,
// so which slot is known there; a stale one only sends the site out of line.  The slot is part of
// the SD's name (profile_node): a program with another ivar order gets its own SD.


// `@x += v` / `@x -= v` with a v that cannot change @x (compile.rs): one self / class / slot test for the
// read and the write, fixnums inline.













// ---- control flow -----------------------------------------------------------
















// begin/rescue: clauses are node_resbody, which answer UNDEF when they do not match

// matches: list of class expressions (empty = StandardError); splats flagged in kinds


// case/when: one `when` clause's test against the subject in a hidden local

// `when LIT` for one immediate literal (Integer / Symbol / nil / true / false)


// ---- calls ----------------------------------------------------------------------



// f(args..., k: v, ...) with literal symbol keywords: values staged after the positionals

// general call: splats, keywords, block-pass (described by ci)







// ---- specialized operators (fixnum / float fast paths) ---------------------------------

// ty (baked): 1 + what the site's operands were besides fixnums (TyFb, recorded by the interpreter), 0 in the
// interpreter.  Baked code keeps the Float paths inline only where Floats were seen; the rest is srb_arith_out.




// Float ** Float with a non-negative base (a negative one may give a Complex)













// An Array indexed by an Integer in range is inline (fast_ary_*: the class and the redefinition in one
// compare).  A Hash stays inline in the interpreter and at baked sites that saw one (TF_HASH).


// ---- definitions -------------------------------------------------------------------

// Entry of a required-params-only method: frame setup with its sizes baked in.  me / block come in ca:
// from a call that compiled the entry in, or from c.call_* (CallArgs).  flags: ME_* (vm.rs), decided by the compiler.

// Entry of a block with only required params run as a plain block (run_block_fast; a `->` lambda: call_proc_kw): frame setup with
// its sizes baked in.  The Block comes in ca.blk (CallArgs), the arguments are at fp[2..], sp is at the base.









// ---- multiple assignment -------------------------------------------------------------------

// rhs → Array stored in tmp slot; targets read it through node_masgn_elem

// kind: 0 = element idx (from front), 1 = splat [idx, len - post), 2 = element from back (post)


// the value of a method-body-level `__method__`, etc. are cfuncs; this node evaluates an
// arbitrary closure-free runtime helper by id (rarely used constructs)

#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_nil(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    Ok(NIL)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_true(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    Ok(TRUE)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_false(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    Ok(FALSE)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_self(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    Ok(fp[0].get())
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_lit(c: &mut Ctx, n: NodePtr, fp: Fp, v: Value) -> RV {
    Ok(v)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_float(c: &mut Ctx, n: NodePtr, fp: Fp, v: f64) -> RV {
    Ok(crate::numeric::float_new(v))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_str(c: &mut Ctx, n: NodePtr, fp: Fp, s: Str, enc: u32) -> RV {
    Ok(crate::string::str_new_enc(s, enc as u8))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_fstr(c: &mut Ctx, n: NodePtr, fp: Fp, s: Str, enc: u32, lc: &mut LitCache) -> RV {
    if lc.v != 0 { return Ok(unsafe { Value::from_raw(lc.v) }); }
    Ok(srb_fstr(s, enc, lc))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_dstr<D_parts: DispList>(c: &mut Ctx, n: NodePtr, fp: Fp, parts: NodeList, enc: u32, cap: u32) -> RV {
    let acc = crate::string::str_with_capa(cap as usize, enc as u8);
    for i in 0..<D_parts as DispList>::len(parts) {
        let v = <D_parts as DispList>::nth(c, parts, i, fp)?;
        srb_dstr_append(c, acc, v)?;
    }
    Ok(acc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_tostr(c: &mut Ctx, n: NodePtr, fp: Fp, v: Value) -> RV {
    if is_string(v) { return Ok(v); }
    srb_tostr(c, v)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_dsym(c: &mut Ctx, n: NodePtr, fp: Fp, s: Value) -> RV {
    Ok(crate::string::str_to_sym(s))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_regex(c: &mut Ctx, n: NodePtr, fp: Fp, src: Str, opts: u32, lc: &mut LitCache) -> RV {
    if lc.v != 0 { return Ok(unsafe { Value::from_raw(lc.v) }); }
    srb_regex_lit(c, src, opts, lc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_dregex(c: &mut Ctx, n: NodePtr, fp: Fp, s: Value, opts: u32, once: bool, lc: &mut LitCache) -> RV {
    if once && lc.v != 0 { return Ok(unsafe { Value::from_raw(lc.v) }); }
    srb_dregex(c, s, opts, once, lc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_biglit(c: &mut Ctx, n: NodePtr, fp: Fp, digits: Str, lc: &mut LitCache) -> RV {
    if lc.v != 0 { return Ok(unsafe { Value::from_raw(lc.v) }); }
    Ok(srb_biglit(digits, lc))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_ary<D_elems: DispList>(c: &mut Ctx, n: NodePtr, fp: Fp, elems: NodeList) -> RV {
    let k = <D_elems as DispList>::len(elems);
    // in locals, stored from registers: staged slots are read back with a wide load that stalls on their stores
    if k <= crate::avec::INLINE {
        let mut x = [NIL; crate::avec::INLINE];
        <D_elems as DispList>::each(c, elems, fp,  |c, i, v| { x[i] = v; Ok(()) })?;
        return Ok(c.ary_new_small(k, x));
    }
    let base = stage(c, k)?;
    <D_elems as DispList>::each(c, elems, fp,  |c, i, v| { base[i].set(v); Ok(()) })?;
    let a = c.ary_from_slots(&base[0..k]);
    c.set_sp(base, 0);
    Ok(a)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_ary_build<D_elems: DispList>(c: &mut Ctx, n: NodePtr, fp: Fp, elems: NodeList, kinds: Str) -> RV {
    let a = c.ary_new();
    let base = stage(c, 1)?;
    base[0].set(a);
    for i in 0..<D_elems as DispList>::len(elems) {
        let v = <D_elems as DispList>::nth(c, elems, i, fp)?;
        if kinds[i] == 1 { srb_splat_into(c, a, v)?; } else { c.ary_push(a, v); }
    }
    c.set_sp(base, 0);
    Ok(a)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_hash<D_items: DispList>(c: &mut Ctx, n: NodePtr, fp: Fp, items: NodeList, kinds: Str) -> RV {
    let h = crate::hash::hash_new_capa(kinds.len());
    let base = stage(c, 2)?;
    base[0].set(h);
    let mut i = 0;
    let mut k = 0;
    while i < <D_items as DispList>::len(items) {
        if kinds[k] == 1 {
            let v = <D_items as DispList>::nth(c, items, i, fp)?;
            srb_hash_merge_kw(c, h, v)?;
            i += 1;
        } else {
            let key = <D_items as DispList>::nth(c, items, i, fp)?;
            base[1].set(key);
            let val = <D_items as DispList>::nth(c, items, i + 1, fp)?;
            crate::hash::hash_aset(c, h, key, val)?;
            i += 2;
        }
        k += 1;
    }
    c.set_sp(base, 0);
    Ok(h)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_range(c: &mut Ctx, n: NodePtr, fp: Fp, b: Value, e: Value, excl: bool) -> RV {
    srb_range_new(c, b, e, excl)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_range_lit(c: &mut Ctx, n: NodePtr, fp: Fp, b: Value, e: Value, excl: bool, lc: &mut LitCache) -> RV {
    if lc.v != 0 { return Ok(unsafe { Value::from_raw(lc.v) }); }
    Ok(srb_range_lit(b, e, excl, lc))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_lvar(c: &mut Ctx, n: NodePtr, fp: Fp, idx: u32) -> RV {
    Ok(slot(fp, idx).get())
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_lasgn(c: &mut Ctx, n: NodePtr, fp: Fp, idx: u32, v: Value) -> RV {
    slot(fp, idx).set(v);
    Ok(v)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_dvar(c: &mut Ctx, n: NodePtr, fp: Fp, hops: u32, idx: u32) -> RV {
    let env = env_at(fp[1].get(), hops);
    Ok(env_get(env, idx))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_dasgn(c: &mut Ctx, n: NodePtr, fp: Fp, hops: u32, idx: u32, v: Value) -> RV {
    let env = env_at(fp[1].get(), hops);
    env_set(env, idx, v);
    Ok(v)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_ivar(c: &mut Ctx, n: NodePtr, fp: Fp, name: Sym, ic: &mut IvarCache, slot: u32) -> RV {
    let slf = fp[0].get();
    if slf.is_heap() {
        let o = obj(slf);
        if o.klass.raw() == ic.klass {
            if let Body::Object(ivs) = &o.body {
                if slot == 0 {
                    if let Some(v) = ivs.get(ic.idx as usize) { return Ok(if v == UNDEF { NIL } else { v }); }
                    return Ok(NIL);
                } else if ic.idx == slot - 1 {
                    // an unset slot leaves through the miss path (a branch to code the site already has, not a select)
                    if let Some(v) = ivs.get((slot - 1) as usize) { if v != UNDEF { return Ok(v); } }
                }
            }
        }
    }
    Ok(srb_ivar_get_out(slf, name, ic))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_iasgn(c: &mut Ctx, n: NodePtr, fp: Fp, name: Sym, v: Value, ic: &mut IvarCache, slot: u32) -> RV {
    let slf = fp[0].get();
    if slf.is_heap() {
        let o = obj(slf);
        if o.klass.raw() == ic.klass && o.flags & FL_FROZEN == 0 {
            if let Body::Object(ivs) = &mut o.body {
                if slot == 0 { if ivs.set(ic.idx as usize, v) { return Ok(v); } }
                else if ic.idx == slot - 1 && ivs.set((slot - 1) as usize, v) { return Ok(v); }
            }
        }
    }
    srb_ivar_set_out(c, slf, name, v, ic)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_ivar_arith<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, name: Sym, sub: bool, v: Value, line: H_line, ic: &mut IvarCache, cc: &mut OpCc, slot: u32) -> RV {
    let slf = fp[0].get();
    if slf.is_heap() && v.is_fixnum() && !c.int_redef() {
        let o = obj(slf);
        if o.klass.raw() == ic.klass && o.flags & FL_FROZEN == 0 && (slot == 0 || ic.idx == slot - 1) {
            if let Body::Object(ivs) = &mut o.body {
                let i = (if slot == 0 { ic.idx } else { slot - 1 }) as usize;
                if let Some(cur) = ivs.get(i) {
                    if cur.is_fixnum() {
                        if let Some(r) = if sub { cur.fix_sub(v) } else { cur.fix_add(v) } { ivs.set(i, r); return Ok(r); }
                    }
                }
            }
        }
    }
    srb_ivar_arith_out(c, slf, name, sub, v, line.pos(), ic, cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_gvar(c: &mut Ctx, n: NodePtr, fp: Fp, name: Sym) -> RV {
    srb_gvar_get(c, name)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_gasgn(c: &mut Ctx, n: NodePtr, fp: Fp, name: Sym, v: Value) -> RV {
    srb_gvar_set(c, name, v)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_cvar(c: &mut Ctx, n: NodePtr, fp: Fp, name: Sym) -> RV {
    srb_cvar_get(c, name)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_cvasgn(c: &mut Ctx, n: NodePtr, fp: Fp, name: Sym, v: Value) -> RV {
    srb_cvar_set(c, name, v)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_const<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, name: Sym, line: H_line, cc: &mut ConstCache) -> RV {
    let cref = unsafe { (*c.frame).cref() };
    if cc.serial == vm().const_serial && cc.cref == cref as usize { return Ok(unsafe { Value::from_raw(cc.val) }); }
    line.store(c.frame());
    srb_const_get(c, cref, name, cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_const_path<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, parent: Value, name: Sym, line: H_line) -> RV {
    line.store(c.frame());
    srb_const_path(c, parent, name)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_top_const(c: &mut Ctx, n: NodePtr, fp: Fp, name: Sym) -> RV {
    srb_const_path(c, vm().c_object, name)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_cdecl(c: &mut Ctx, n: NodePtr, fp: Fp, name: Sym, v: Value) -> RV {
    srb_cdecl(c, NIL, name, v)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_cdecl_path(c: &mut Ctx, n: NodePtr, fp: Fp, parent: Value, name: Sym, v: Value) -> RV {
    srb_cdecl_path(c, parent, name, v)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_errinfo(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    Ok(c.errinfo)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_nth_ref(c: &mut Ctx, n: NodePtr, fp: Fp, nth: u32) -> RV {
    Ok(crate::regexp::nth_match(c, c.last_match, nth as i64))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_back_ref(c: &mut Ctx, n: NodePtr, fp: Fp, kind: u32) -> RV {
    srb_back_ref(c, kind)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_block<D_stmts: DispList>(c: &mut Ctx, n: NodePtr, fp: Fp, stmts: NodeList) -> RV {
    let mut last = NIL;
    <D_stmts as DispList>::each(c, stmts, fp,  |c, i, v| { last = v; Ok(()) })?;
    Ok(last)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_line<H_line: crate::vm::LineHole, D_body: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, line: H_line, body: NodePtr) -> RV {
    line.store(c.frame());
    <D_body as Disp>::call(c, body, fp)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_if<D_cond: Disp, D_then_b: Disp, D_else_b: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, cond: NodePtr, then_b: NodePtr, else_b: NodePtr) -> RV {
    if <D_cond as Disp>::test(c, cond, fp)? { <D_then_b as Disp>::call(c, then_b, fp) } else { <D_else_b as Disp>::call(c, else_b, fp) }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_and<D_a: Disp, D_b: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, a: NodePtr, b: NodePtr) -> RV {
    let v = <D_a as Disp>::call(c, a, fp)?;
    if v.falsy() { Ok(v) } else { <D_b as Disp>::call(c, b, fp) }
}
#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_TEST_node_and<D_a: Disp, D_b: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, a: NodePtr, b: NodePtr) -> Result<bool, Esc> {
    if <D_a as Disp>::test(c, a, fp)? { <D_b as Disp>::test(c, b, fp) } else { Ok(false) }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_or<D_a: Disp, D_b: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, a: NodePtr, b: NodePtr) -> RV {
    let v = <D_a as Disp>::call(c, a, fp)?;
    if v.truthy() { Ok(v) } else { <D_b as Disp>::call(c, b, fp) }
}
#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_TEST_node_or<D_a: Disp, D_b: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, a: NodePtr, b: NodePtr) -> Result<bool, Esc> {
    if <D_a as Disp>::test(c, a, fp)? { Ok(true) } else { <D_b as Disp>::test(c, b, fp) }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_not(c: &mut Ctx, n: NodePtr, fp: Fp, v: Value, cc: &mut OpCc) -> RV {
    if v.is_heap() && srb_has_custom_not(v) { return srb_call0(c, v, s::not, cc); }
    Ok(Value::bool(v.falsy()))
}
#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_TEST_node_not(c: &mut Ctx, n: NodePtr, fp: Fp, v: Value, cc: &mut OpCc) -> Result<bool, Esc> {
    if v.is_heap() && srb_has_custom_not(v) { return Ok(srb_call0(c, v, s::not, cc)?.truthy()); }
    Ok(v.falsy())
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_while<D_cond: Disp, D_body: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, cond: NodePtr, body: NodePtr, until: bool) -> RV {
    loop {
        let t = <D_cond as Disp>::test(c, cond, fp)?;
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
pub fn EVAL_node_dowhile<D_cond: Disp, D_body: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, cond: NodePtr, body: NodePtr, until: bool) -> RV {
    loop {
        match <D_body as Disp>::call(c, body, fp) {
            Ok(_) => {}
            Err(e) => match c.esc {
                EscKind::LoopNext => { c.esc = EscKind::None; }
                EscKind::LoopBreak => { c.esc = EscKind::None; return Ok(c.esc_val); }
                _ => return Err(e),
            }
        }
        let t = <D_cond as Disp>::test(c, cond, fp)?;
        if t == until { break; }
    }
    Ok(NIL)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_loop_break(c: &mut Ctx, n: NodePtr, fp: Fp, v: Value) -> RV {
    c.esc = EscKind::LoopBreak;
    c.esc_val = v;
    Err(Esc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_loop_next(c: &mut Ctx, n: NodePtr, fp: Fp, v: Value) -> RV {
    c.esc = EscKind::LoopNext;
    c.esc_val = v;
    Err(Esc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_break(c: &mut Ctx, n: NodePtr, fp: Fp, v: Value) -> RV {
    srb_break(c, v)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_next(c: &mut Ctx, n: NodePtr, fp: Fp, v: Value) -> RV {
    c.esc = EscKind::Next;
    c.esc_val = v;
    Err(Esc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_return(c: &mut Ctx, n: NodePtr, fp: Fp, v: Value) -> RV {
    if unsafe { (*c.frame).kind } == FK_BLOCK { return srb_block_return(c, v); }
    c.esc = EscKind::Return;
    c.esc_val = v;
    c.esc_tag = unsafe { (*c.frame).id };
    Err(Esc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_retry(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    c.esc = EscKind::Retry;
    c.esc_val = NIL;
    Err(Esc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_redo(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    c.esc = EscKind::Redo;
    c.esc_val = NIL;
    Err(Esc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_rescue<D_body: Disp, D_clauses: DispList, D_else_b: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, body: NodePtr, clauses: NodeList, else_b: NodePtr, has_else: bool) -> RV {
    let saved_sp = c.sp_mark();
    let saved_err = c.errinfo;
    loop {
        match <D_body as Disp>::call(c, body, fp) {
            Ok(v) => { return if has_else { <D_else_b as Disp>::call(c, else_b, fp) } else { Ok(v) }; }
            Err(e) => {
                if c.esc != EscKind::Raise { return Err(e); }
                c.sp_reset(saved_sp);
                let exc = c.esc_val;
                if !srb_rescuable(exc) { return Err(e); }
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
pub fn EVAL_node_resbody<D_classes: DispList, D_assign: Disp, D_body: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, classes: NodeList, kinds: Str, assign: NodePtr, body: NodePtr) -> RV {
    let exc = c.errinfo;
    let mut ok = <D_classes as DispList>::len(classes) == 0 && srb_is_standard_error(exc);
    let mut i = 0;
    while !ok && i < <D_classes as DispList>::len(classes) {
        let k = <D_classes as DispList>::nth(c, classes, i, fp)?;
        ok = srb_rescue_match(c, k, exc, kinds[i] == 1)?;
        i += 1;
    }
    if !ok { return Ok(UNDEF); }
    <D_assign as Disp>::call(c, assign, fp)?;
    <D_body as Disp>::call(c, body, fp)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_ensure<D_body: Disp, D_ens: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, body: NodePtr, ens: NodePtr) -> RV {
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
pub fn EVAL_node_when<D_pats: DispList>(c: &mut Ctx, n: NodePtr, fp: Fp, subj_slot: u32, pats: NodeList, kinds: Str, cc: &mut CallCache, km: &mut KindMemo) -> RV {
    let subj = slot(fp, subj_slot).get();
    for i in 0..<D_pats as DispList>::len(pats) {
        let p = <D_pats as DispList>::nth(c, pats, i, fp)?;
        if kinds[i] == 1 {
            if srb_when_splat(c, p, subj)? { return Ok(TRUE); }
        } else {
            let hit = match eqq_imm(p, subj) {
                Some(b) => b,
                None => {
                    // Module#=== asked of the same subject class again: d is 0 (no) or 1 (yes)
                    let k = class_of(subj).raw();
                    let d = km.memo[kind_way(k)] ^ k;
                    // is_class: the slot of a dead pattern class may be another object by now (a new class would move the serial)
                    if p.raw() == km.pat && d <= 1 && km.ser == kind_serial() && is_class_or_module(p) { d == 1 } else { srb_eqq_class(c, p, subj, cc, km)? }
                }
            };
            if hit { return Ok(TRUE); }
        }
    }
    Ok(FALSE)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_when_lit(c: &mut Ctx, n: NodePtr, fp: Fp, subj_slot: u32, v: Value, cc: &mut CallCache) -> RV {
    let subj = slot(fp, subj_slot).get();
    if subj == v { return Ok(TRUE); }
    if !v.is_fixnum() || subj.is_fixnum() { return Ok(FALSE); }
    Ok(Value::bool(srb_eqq(c, v, subj, cc)?))
}
#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_TEST_node_when_lit(c: &mut Ctx, n: NodePtr, fp: Fp, subj_slot: u32, v: Value, cc: &mut CallCache) -> Result<bool, Esc> {
    let subj = slot(fp, subj_slot).get();
    if subj == v { return Ok(true); }
    if !v.is_fixnum() || subj.is_fixnum() { return Ok(false); }
    srb_eqq(c, v, subj, cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_defined<D_recv: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, kind: u32, name: Sym, recv: NodePtr) -> RV {
    srb_defined::<D_recv>(c, n, fp, kind, name, recv)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_call<D_recv: Disp, D_args: DispList, H_line: crate::vm::LineHole, D_callee: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: H_line, cc: &mut CallCache, callee_hash: u64, kind: u32, shared: u32, bi: u32) -> RV {
    // baked self call of a callee without parameters, locals, env or block literal: it runs in this frame
    if shared != 0 && callee_hash != 0 {
        let r0 = fp[0].get();
        if spec_hit(cc, r0, kind) {
            if shared == 1 { line.store(c.frame()); }
            return <D_callee as Disp>::spec(c, cc.fast as NodePtr, fp, CallArgs { me: cc.me as *const MethodEntry, blk: std::ptr::null(), shared: true, inl: true });
        }
    }
    let k = <D_args as DispList>::len(args);
    let (base, r0) = if bi != 0 {
        // baked C-method site named as builtins of inline_builtin.rs (bi: their bodies): the one the cache
        // holds, staging only for the call
        let r0 = <D_recv as Disp>::call(c, recv, fp)?;
        let a0 = if k == 1 { <D_args as DispList>::nth(c, args, 0, fp)? } else { NIL };
        if bi.is_power_of_two() {
            let b = bi.trailing_zeros();
            if bi_hit(cc, r0, b) { if let Some(v) = bi_body(c, b, r0, a0) { return Ok(v); } }
        } else if let Some(v) = bi_run(c, cc, bi, r0, a0) { return Ok(v); }
        std::hint::cold_path();
        let base = stage(c, 2 + k)?;
        base[0].set(r0);
        if k == 1 { base[2].set(a0); }
        (base, r0)
    } else {
        let base = stage(c, 2 + k)?;
        let r0 = <D_recv as Disp>::call(c, recv, fp)?;
        base[0].set(r0);
        <D_args as DispList>::each(c, args, fp,  |c, i, v| { base[2 + i].set(v); Ok(()) })?;
        (base, r0)
    };
    line.store(c.frame());
    if bi != 0 { return call_cached_out(c, cc, mid, base, k as u32, std::ptr::null(), flags); }
    // with arguments in between, reading the slot back is cheaper than keeping the receiver live
    let r0 = if k == 0 { r0 } else { base[0].get() };
    if kind == SITE_INTERP { return call_cached(c, cc, mid, base, k as u32, std::ptr::null(), flags); }
    // baked code: one cache test, then the callee compiled into this site or the site's one fast path
    if callee_hash != 0 {
        // spec_klass is set only while the cache holds the compiled-in callee (a shared site tested it above)
        if shared == 0 && spec_hit(cc, r0, kind) {
            // sp is where stage() left it: what the entry expects
            let callee_fp = frame(c, base)?;
            let r = <D_callee as Disp>::spec(c, cc.fast as NodePtr, callee_fp, CallArgs::inline(cc.me as *const MethodEntry, std::ptr::null()));
            c.set_sp(base, 0);
            return r;
        }
        std::hint::cold_path();
        return call_spec_out(c, cc, mid, base, k as u32, std::ptr::null(), flags, callee_hash);
    }
    if site_hit(cc, r0, kind) {
        if let Some(r) = site_fast(c, cc, r0, base, k as u32, std::ptr::null(), kind) { return r; }
    }
    call_cached_out(c, cc, mid, base, k as u32, std::ptr::null(), flags)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_call_blk<D_recv: Disp, D_args: DispList, H_line: crate::vm::LineHole, D_callee: Disp, D_body: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: H_line, blk: IseqRef, cc: &mut CallCache, callee_hash: u64, kind: u32, body_hash: u64, it: u32) -> RV {
    let k = <D_args as DispList>::len(args);
    let base = stage(c, 2 + k)?;
    let r0 = <D_recv as Disp>::call(c, recv, fp)?;
    base[0].set(r0);
    for i in 0..k { let v = <D_args as DispList>::nth(c, args, i, fp)?; base[2 + i].set(v); }
    line.store(c.frame());
    // with arguments in between, reading the slot back is cheaper than keeping the receiver live
    let r0 = if k == 0 { r0 } else { base[0].get() };
    let b = make_block(c, blk, fp);
    let r = if it != 0 && body_hash != 0 {
        // baked builtin iterator (inline_iter.rs): the builtin's own loop under its cfunc frame, the block's entry compiled in
        let bp: *const Block = &b;
        // fast_hash: iter_refill checked this entry; the tree hash implies it now (iseq_hash), kept as one cheap compare
        let hit = (bi_hit(cc, r0, it & 0xff) && cc.fast_hash == body_hash) || iter_refill(c, cc, r0, mid, flags, it, blk, body_hash);
        let bb = c.top();
        match if hit { frame(c, bb).ok() } else { None } {
            Some(bfp) => {
                let pf = c.frame;
                let mut fr = Frame::cfunc(pf, cc.me as *const MethodEntry, bp);
                let frp: *mut Frame = &mut fr;
                // IT_SHARE: the block's frame made once, every step runs in it (the loops of iter_loop
                // neither raise nor stage between steps, so they may run in that frame too)
                let share = it & IT_SHARE != 0;
                let np = (it >> 8 & 0xff) as usize;
                let mut bfr = block_frame(&b, bfp, frp, blk);
                c.frame = if share { &mut bfr.h } else { frp };
                let a = [if k > 0 { base[2].get() } else { NIL }, if k > 1 { base[3].get() } else { NIL }];
                let r = iter_loop(c, it, r0, &a[..k.min(2)], &mut |c: &mut Ctx, v: &[Value]| {
                    put_args(c, bb, bfp, np, v);
                    // a step starts at the block's line
                    if share { c.frame().pos = Pos::at(&blk.info.line); }
                    <D_body as Disp>::spec(c, blk.bentry, bfp, CallArgs { me: std::ptr::null(), blk: bp, shared: share, inl: true })
                });
                c.frame = pf;
                match r {
                    Some(r) => { c.set_sp(base, 0); r }
                    None => { std::hint::cold_path(); c.set_sp(base, 2 + k); call_cached_out(c, cc, mid, base, k as u32, &b, flags) }
                }
            }
            None => { std::hint::cold_path(); call_cached_out(c, cc, mid, base, k as u32, &b, flags) }
        }
    }
    else if kind == SITE_INTERP { call_cached(c, cc, mid, base, k as u32, &b, flags) }
    else if callee_hash != 0 {
        if spec_hit(cc, r0, kind) {
            let callee_fp = frame(c, base)?;
            let r = <D_callee as Disp>::spec(c, cc.fast as NodePtr, callee_fp, CallArgs::new(cc.me as *const MethodEntry, &b));
            c.set_sp(base, 0);
            r
        } else { call_spec_out(c, cc, mid, base, k as u32, &b, flags, callee_hash) }
    } else if site_hit(cc, r0, kind) && callee_hash == 0 {
        match site_fast(c, cc, r0, base, k as u32, &b, kind) { Some(r) => r, None => call_cached_out(c, cc, mid, base, k as u32, &b, flags) }
    } else { call_cached_out(c, cc, mid, base, k as u32, &b, flags) };
    match r {
        Err(e) if c.esc == EscKind::Break && c.esc_tag == b.tag => { c.esc = EscKind::None; Ok(c.esc_val) }
        r => r,
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_call_kw<D_recv: Disp, D_args: DispList, H_line: crate::vm::LineHole, D_callee: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: H_line, ci: CiRef, cc: &mut CallCache, callee_hash: u64, kind: u32) -> RV {
    let k = <D_args as DispList>::len(args);
    let base = stage(c, 2 + k)?;
    base[0].set(<D_recv as Disp>::call(c, recv, fp)?);
    for i in 0..k { let v = <D_args as DispList>::nth(c, args, i, fp)?; base[2 + i].set(v); }
    line.store(c.frame());
    // baked: the callee's kentry compiled in (every keyword passed in declared order, no block literal)
    if callee_hash != 0 {
        let r0 = base[0].get();
        if spec_hit(cc, r0, kind) {
            let callee_fp = frame(c, base)?;
            let r = <D_callee as Disp>::spec(c, cc.fast as NodePtr, callee_fp, CallArgs::inline(cc.me as *const MethodEntry, std::ptr::null()));
            c.set_sp(base, 0);
            return r;
        }
        return srb_call_kw_spec(c, cc, mid, base, k, ci, flags, fp, callee_hash);
    }
    srb_call_kw(c, cc, mid, base, k, ci, flags, fp)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_call_x<D_recv: Disp, D_args: DispList, H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: H_line, ci: CiRef, cc: &mut CallCache) -> RV {
    let base = stage(c, 2)?;
    let r0 = <D_recv as Disp>::call(c, recv, fp)?;
    base[0].set(r0);
    // f(*a) alone: a's elements are staged as the arguments, no argument Array (tmp is a itself)
    let splat1 = srb_splat1(ci, flags);
    let mut blkv = NIL;
    let mut kw = NIL;
    let tmp = if splat1 { <D_args as DispList>::nth(c, args, 0, fp)? } else {
        let tmp = crate::array::ary_new();
        base[1].set(tmp);
        for i in 0..<D_args as DispList>::len(args) {
            let v = <D_args as DispList>::nth(c, args, i, fp)?;
            srb_callx_arg(c, ci, i, v, tmp, &mut kw, &mut blkv)?;
        }
        tmp
    };
    line.store(c.frame());
    if splat1 { return srb_call_splat1(c, cc, mid, base, tmp, ci, flags, fp); }
    srb_call_x(c, cc, mid, base, tmp, kw, blkv, ci, flags, fp)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_attrasgn<D_recv: Disp, D_args: DispList, H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: H_line, cc: &mut CallCache, kind: u32) -> RV {
    let k = <D_args as DispList>::len(args);
    let base = stage(c, 2 + k)?;
    base[0].set(<D_recv as Disp>::call(c, recv, fp)?);
    for i in 0..k { let v = <D_args as DispList>::nth(c, args, i, fp)?; base[2 + i].set(v); }
    line.store(c.frame());
    let val = base[1 + k].get();
    if kind == SITE_INTERP { call_cached(c, cc, mid, base, k as u32, std::ptr::null(), flags)?; return Ok(val); }
    // baked code: only the site's one fast path inline, as node_call
    let r0 = base[0].get();
    if site_hit(cc, r0, kind) {
        if let Some(r) = site_fast(c, cc, r0, base, k as u32, std::ptr::null(), kind) { r?; return Ok(val); }
    }
    call_cached_out(c, cc, mid, base, k as u32, std::ptr::null(), flags)?;
    Ok(val)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_super<D_args: DispList, H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, args: NodeList, line: H_line, ci: CiRef, zsuper: bool) -> RV {
    let base = stage(c, 2)?;
    let tmp = crate::array::ary_new();
    base[0].set(fp[0].get()); base[1].set(tmp);
    let mut blkv = NIL;
    let mut kw = NIL;
    for i in 0..<D_args as DispList>::len(args) {
        let v = <D_args as DispList>::nth(c, args, i, fp)?;
        srb_callx_arg(c, ci, i, v, tmp, &mut kw, &mut blkv)?;
    }
    line.store(c.frame());
    srb_super(c, base, tmp, kw, blkv, ci, fp, zsuper)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_yield<D_args: DispList, D_callee: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, args: NodeList, yc: &mut YieldCache, callee_hash: u64, np: u32) -> RV {
    let k = <D_args as DispList>::len(args);
    let blk = unsafe { (*c.frame).blk };
    if blk.is_null() { return srb_no_block(c); }
    let base = stage(c, 2 + k)?;
    for i in 0..k { let v = <D_args as DispList>::nth(c, args, i, fp)?; base[2 + i].set(v); }
    if callee_hash == 0 { srb_yield_note(yc, blk); return srb_yield_staged(c, blk, base, k as u32); }
    // baked, one block seen: its entry compiled in, guarded by the iseq (yc.fast is set once its hash matched)
    let b = unsafe { &*blk };
    if let BlockKind::Iseq(is) = b.kind {
        if is as usize == yc.fast && (b.proc_val == UNDEF || !rproc(b.proc_val).is_lambda) {
            // np = 1 + the block's lead parameter count: missing arguments are nil
            if (k as u32) + 1 < np { nil_fill(base, 2 + k, 1 + np as usize); }
            c.set_sp(base, 0);
            let bfp = frame(c, base)?;
            let r = <D_callee as Disp>::spec(c, unsafe { (*is).bentry }, bfp, CallArgs::new(std::ptr::null(), blk));
            c.set_sp(base, 0);
            return r;
        }
    }
    srb_yield_miss(c, yc, callee_hash, blk, base, k as u32)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_yield_x<D_args: DispList>(c: &mut Ctx, n: NodePtr, fp: Fp, args: NodeList, ci: CiRef) -> RV {
    let tmp = crate::array::ary_new();
    let base = stage(c, 1)?;
    base[0].set(tmp);
    let mut blkv = NIL;
    let mut kw = NIL;
    for i in 0..<D_args as DispList>::len(args) {
        let v = <D_args as DispList>::nth(c, args, i, fp)?;
        srb_callx_arg(c, ci, i, v, tmp, &mut kw, &mut blkv)?;
    }
    let r = srb_yield_x(c, tmp, kw);
    c.set_sp(base, 0);
    r
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_block_given(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    Ok(Value::bool(unsafe { !(*c.frame).blk.is_null() }))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_lambda(c: &mut Ctx, n: NodePtr, fp: Fp, blk: IseqRef) -> RV {
    srb_lambda(c, blk, fp)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_plus<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !c.int_redef() { if let Some(r) = a.fix_add(b) { return Ok(r); } }
    if tf_flo(ty) {
        if let Some((x, y)) = flo2(a, b, R_FLO_PLUS, R_INT_PLUS) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(crate::numeric::float_new(x + y)); }
        return srb_binop(c, a, b, s::plus, line.pos(), cc);
    }
    srb_arith_out(c, a, b, s::plus, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_minus<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !c.int_redef() { if let Some(r) = a.fix_sub(b) { return Ok(r); } }
    if tf_flo(ty) {
        if let Some((x, y)) = flo2(a, b, R_FLO_MINUS, R_INT_MINUS) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(crate::numeric::float_new(x - y)); }
        return srb_binop(c, a, b, s::minus, line.pos(), cc);
    }
    srb_arith_out(c, a, b, s::minus, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_mul<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    if a.both_fix(b) && !c.int_redef() { if let Some(r) = a.fix_mul(b) { return Ok(r); } }
    if tf_flo(ty) {
        if let Some((x, y)) = flo2(a, b, R_FLO_MUL, R_INT_MUL) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(crate::numeric::float_new(x * y)); }
        return srb_binop(c, a, b, s::mul, line.pos(), cc);
    }
    srb_arith_out(c, a, b, s::mul, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_div<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    if a.is_fixnum() && b.is_fixnum() && b.as_fix() != 0 && !c.int_redef() {
        let (x, y) = (a.as_fix(), b.as_fix());
        let q = if (x % y != 0) && ((x < 0) != (y < 0)) { x / y - 1 } else { x / y };
        if Value::fixable(q) { return Ok(Value::fix(q)); }
    }
    if tf_flo(ty) {
        if let Some((x, y)) = flo2(a, b, R_FLO_DIV, R_INT_DIV) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(crate::numeric::float_new(x / y)); }
        return srb_binop(c, a, b, s::div, line.pos(), cc);
    }
    srb_arith_out(c, a, b, s::div, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_pow<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc) -> RV {
    if a.is_flonum() && b.is_flonum() && !redef(R_FLO_POW) {
        let (x, y) = (a.flonum_val(), b.flonum_val());
        if x >= 0.0 { return Ok(crate::numeric::float_new(crate::numeric::libm_pow(x, y))); }
    }
    srb_binop(c, a, b, s::pow, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_mod<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc) -> RV {
    if a.is_fixnum() && b.is_fixnum() && b.as_fix() != 0 && !c.int_redef() {
        let (x, y) = (a.as_fix(), b.as_fix());
        let m = x % y;
        let m = if m != 0 && ((m < 0) != (y < 0)) { m + y } else { m };
        return Ok(Value::fix(m));
    }
    srb_binop(c, a, b, s::modulo, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_lt<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    if a.both_fix(b) && !c.int_redef() { return Ok(Value::bool(a.fix_ord() < b.fix_ord())); }
    if tf_flo(ty) {
        if let Some((x, y)) = flo2_cmp(a, b, R_FLO_LT, R_INT_LT) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(Value::bool(x < y)); }
        return srb_binop(c, a, b, s::lt, line.pos(), cc);
    }
    srb_arith_out(c, a, b, s::lt, line.pos(), cc)
}
#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_TEST_node_opt_lt<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> Result<bool, Esc> {
    if a.both_fix(b) && !c.int_redef() { return Ok(a.fix_ord() < b.fix_ord()); }
    if tf_flo(ty) {
        if a.is_flonum() && b.is_flonum() && !redef(R_FLO_LT) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(a.flonum_val() < b.flonum_val()); }
        if ty == 0 { return Ok(srb_binop(c, a, b, s::lt, line.pos(), cc)?.truthy()); }
    }
    Ok(srb_arith_out(c, a, b, s::lt, line.pos(), cc)?.truthy())
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_le<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    if a.both_fix(b) && !c.int_redef() { return Ok(Value::bool(a.fix_ord() <= b.fix_ord())); }
    if tf_flo(ty) {
        if let Some((x, y)) = flo2_cmp(a, b, R_FLO_LE, R_INT_LE) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(Value::bool(x <= y)); }
        return srb_binop(c, a, b, s::le, line.pos(), cc);
    }
    srb_arith_out(c, a, b, s::le, line.pos(), cc)
}
#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_TEST_node_opt_le<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> Result<bool, Esc> {
    if a.both_fix(b) && !c.int_redef() { return Ok(a.fix_ord() <= b.fix_ord()); }
    if tf_flo(ty) {
        if a.is_flonum() && b.is_flonum() && !redef(R_FLO_LE) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(a.flonum_val() <= b.flonum_val()); }
        if ty == 0 { return Ok(srb_binop(c, a, b, s::le, line.pos(), cc)?.truthy()); }
    }
    Ok(srb_arith_out(c, a, b, s::le, line.pos(), cc)?.truthy())
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_gt<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    if a.both_fix(b) && !c.int_redef() { return Ok(Value::bool(a.fix_ord() > b.fix_ord())); }
    if tf_flo(ty) {
        if let Some((x, y)) = flo2_cmp(a, b, R_FLO_GT, R_INT_GT) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(Value::bool(x > y)); }
        return srb_binop(c, a, b, s::gt, line.pos(), cc);
    }
    srb_arith_out(c, a, b, s::gt, line.pos(), cc)
}
#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_TEST_node_opt_gt<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> Result<bool, Esc> {
    if a.both_fix(b) && !c.int_redef() { return Ok(a.fix_ord() > b.fix_ord()); }
    if tf_flo(ty) {
        if a.is_flonum() && b.is_flonum() && !redef(R_FLO_GT) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(a.flonum_val() > b.flonum_val()); }
        if ty == 0 { return Ok(srb_binop(c, a, b, s::gt, line.pos(), cc)?.truthy()); }
    }
    Ok(srb_arith_out(c, a, b, s::gt, line.pos(), cc)?.truthy())
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_ge<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    if a.both_fix(b) && !c.int_redef() { return Ok(Value::bool(a.fix_ord() >= b.fix_ord())); }
    if tf_flo(ty) {
        if let Some((x, y)) = flo2_cmp(a, b, R_FLO_GE, R_INT_GE) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(Value::bool(x >= y)); }
        return srb_binop(c, a, b, s::ge, line.pos(), cc);
    }
    srb_arith_out(c, a, b, s::ge, line.pos(), cc)
}
#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_TEST_node_opt_ge<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> Result<bool, Esc> {
    if a.both_fix(b) && !c.int_redef() { return Ok(a.fix_ord() >= b.fix_ord()); }
    if tf_flo(ty) {
        if a.is_flonum() && b.is_flonum() && !redef(R_FLO_GE) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(a.flonum_val() >= b.flonum_val()); }
        if ty == 0 { return Ok(srb_binop(c, a, b, s::ge, line.pos(), cc)?.truthy()); }
    }
    Ok(srb_arith_out(c, a, b, s::ge, line.pos(), cc)?.truthy())
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_eq<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    if a.both_fix(b) && !c.int_redef() { return Ok(Value::bool(a == b)); }
    if tf_flo(ty) { if let Some((x, y)) = flo2_cmp(a, b, R_FLO_EQ, R_INT_EQ) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(Value::bool(x == y)); } }
    // nil / true / false / Symbol receivers: == is identity
    if a.is_special() && !redef(R_IMM_EQ) { return Ok(Value::bool(a == b)); }
    if ty == 0 { return srb_eq(c, a, b, line.pos(), cc); }
    srb_arith_out(c, a, b, s::eq, line.pos(), cc)
}
#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_TEST_node_opt_eq<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> Result<bool, Esc> {
    if a.both_fix(b) && !c.int_redef() { return Ok(a == b); }
    if tf_flo(ty) { if a.is_flonum() && b.is_flonum() && !redef(R_FLO_EQ) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(a.flonum_val() == b.flonum_val()); } }
    if a.is_sym() && b.is_sym() { return Ok(a == b); }
    if ty == 0 { return Ok(srb_eq(c, a, b, line.pos(), cc)?.truthy()); }
    Ok(srb_arith_out(c, a, b, s::eq, line.pos(), cc)?.truthy())
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_neq<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    if a.both_fix(b) && !c.int_redef() { return Ok(Value::bool(a != b)); }
    if tf_flo(ty) {
        if let Some((x, y)) = flo2_cmp(a, b, R_FLO_EQ, R_INT_EQ) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(Value::bool(x != y)); }
        return srb_binop(c, a, b, s::neq, line.pos(), cc);
    }
    srb_arith_out(c, a, b, s::neq, line.pos(), cc)
}
#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_TEST_node_opt_neq<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> Result<bool, Esc> {
    if a.both_fix(b) && !c.int_redef() { return Ok(a != b); }
    if tf_flo(ty) {
        if a.is_flonum() && b.is_flonum() && !redef(R_FLO_EQ) { if ty == 0 { tf.0 |= TF_FLO; } return Ok(a.flonum_val() != b.flonum_val()); }
        if ty == 0 { return Ok(srb_binop(c, a, b, s::neq, line.pos(), cc)?.truthy()); }
    }
    Ok(srb_arith_out(c, a, b, s::neq, line.pos(), cc)?.truthy())
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_ltlt<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !c.int_redef() {
        if let Some(r) = a.fix_shl(b.as_fix()) { return Ok(r); }
    }
    if is_array(a) && !redef(R_ARY_LTLT) && obj(a).flags & FL_FROZEN == 0 && obj(a).klass == vm().c_array {
        c.ary_push(a, b);
        return Ok(a);
    }
    if is_string(a) && is_string(b) && obj(a).klass == vm().c_string && !redef(R_STR_LTLT) {
        if obj(a).flags & FL_FROZEN == 0 && rstr(a).enc == rstr(b).enc {
            crate::string::str_append_same_enc(a, b);
            return Ok(a);
        }
        line.store(c.frame());
        crate::string::str_append_str(c, a, b)?;
        return Ok(a);
    }
    srb_binop(c, a, b, s::ltlt, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_gtgt<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !c.int_redef() {
        let k = b.as_fix();
        if k >= 0 { return Ok(Value::fix(if k >= 63 { if a.as_fix() < 0 { -1 } else { 0 } } else { a.as_fix() >> k })); }
    }
    srb_binop(c, a, b, s::gtgt, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_xor<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !c.int_redef() { if let Some(r) = Value::odd(a.raw() ^ b.raw()) { return Ok(r); } }
    srb_binop(c, a, b, s::xor, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_and<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !c.int_redef() { if let Some(r) = Value::odd(a.raw() & b.raw()) { return Ok(r); } }
    srb_binop(c, a, b, s::and, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_or<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc) -> RV {
    if a.is_fixnum() && b.is_fixnum() && !c.int_redef() { if let Some(r) = Value::odd(a.raw() | b.raw()) { return Ok(r); } }
    srb_binop(c, a, b, s::or, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_aref<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    // the class load sits between the two tag tests, or they are merged into setcc / or
    if a.is_heap() && obj(a).klass.raw() == fast_ary_aref() && b.is_fixnum() {
        if let Some(&x) = c.ary(a).get(b.as_fix() as usize) { return Ok(x); }
    } else if tf_hash(ty) && is_hash(a) && obj(a).klass == vm().c_hash && !redef(R_HASH_AREF) {
        if ty == 0 { tf.0 |= TF_HASH; }
        return crate::hash::hash_aref(c, a, b);
    }
    srb_aref_out(c, a, b, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_opt_aset<H_line: crate::vm::LineHole>(c: &mut Ctx, n: NodePtr, fp: Fp, a: Value, b: Value, v: Value, line: H_line, cc: &mut OpCc, tf: &mut TyFb, ty: u32) -> RV {
    if a.is_heap() && obj(a).klass.raw() == fast_ary_aset() && b.is_fixnum() && obj(a).flags & FL_FROZEN == 0 {
        if let Some(x) = c.ary_mut(a).get_mut(b.as_fix() as usize) { *x = v; return Ok(v); }
    } else if tf_hash(ty) && is_hash(a) && obj(a).klass == vm().c_hash && !redef(R_HASH_ASET) {
        if ty == 0 { tf.0 |= TF_HASH; }
        crate::hash::hash_aset(c, a, b, v)?;
        return Ok(v);
    }
    srb_aset_out(c, a, b, v, line.pos(), cc)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_method_entry<H_line: crate::vm::LineHole, D_body: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, nlocals: u32, nparams: u32, flags: u32, line: H_line, body: NodePtr, ca: CallArgs) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit_body("entry", n, flags & ME_ENV != 0, nparams as usize, !ca.blk.is_null());
    // nothing in a leaf body can call, raise or look at a frame
    if flags & ME_LEAF != 0 { return <D_body as Disp>::call(c, body, fp); }
    let me = ca.me;
    let blk = ca.blk;
    // id 0: no block can return to this frame, and its own `return` is caught by the innermost entry
    let id = if flags & ME_ID != 0 { c.next_serial() } else { 0 };
    let prev = c.frame;
    let mut fr = Frame { prev, fp: head(fp), me, blk, id, pos: line.pos(), kind: FK_METHOD, visi: VISI_PUBLIC, lite: true, pad: 0 };
    // no native stack check here: whoever calls an entry out of line makes it (run_entry, site_fast)
    // a shared frame (the caller's, CallArgs) has no slots of its own and keeps the caller's env
    if !(shareable(nlocals, nparams, flags) && ca.shared) {
        enter(c, fp, nlocals, nparams)?;
        fp[1].set(NIL);
    }
    let mut i = 2 + nparams;
    while i < nlocals { slot(fp, i).set(NIL); i += 1; }
    if flags & ME_ENV != 0 { make_env(unsafe { &*fr.iseq() }, fp, NIL); }
    c.frame = &mut fr;
    let r = <D_body as Disp>::call(c, body, fp);
    // compiled in and no call in the body: `prev` stays in a register and the reload of fr.prev (a store-load
    // round trip) goes; out of line it would cost a saved register
    c.frame = if flags & ME_NOCALL != 0 && ca.inl { prev } else { fr.prev };
    match r {
        Ok(v) => Ok(v),
        Err(e) => if c.esc == EscKind::Return && c.esc_tag == id { c.esc = EscKind::None; Ok(c.esc_val) } else { Err(e) },
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_block_entry<H_line: crate::vm::LineHole, D_body: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, nlocals: u32, nparams: u32, flags: u32, line: H_line, body: NodePtr, ca: CallArgs) -> RV {
    let b = unsafe { &*ca.blk };
    // a lambda (ME_LAMBDA, called with its exact arity) is its own return target
    let lambda = flags & ME_LAMBDA != 0;
    // shared: an inline iterator (node_call_blk, IT_SHARE) made this frame, self and env once for its loop
    let shared = ca.shared && flags & (ME_LAMBDA | ME_ENV) == 0;
    let id = if lambda { c.next_serial() } else { b.home_id };
    // iseq null: Frame::iseq() takes it from self_blk when asked
    let mut fr = FrameFull {
        h: Frame { prev: c.frame, fp: head(fp), me: b.me, blk: b.outer_blk, id, pos: line.pos(), kind: if lambda { FK_LAMBDA } else { FK_BLOCK }, visi: VISI_PUBLIC, lite: false, pad: 0 },
        cref: b.cref, iseq: std::ptr::null(), self_blk: b, defined_class: b.defined_class,
    };
    if !shared {
        fp[0].set(b.self_);
        fp[1].set(b.env);
    }
    let mut i = 2 + nparams;
    while i < nlocals { slot(fp, i).set(NIL); i += 1; }
    enter_base(c, fp, nlocals)?;
    if flags & ME_ENV != 0 { make_env(unsafe { &*fr.h.iseq() }, fp, b.env); }
    if !shared { c.frame = &mut fr.h; }
    let mut r = <D_body as Disp>::call(c, body, fp);
    // redo runs the body again out of line (a second inline copy would double the block's code)
    if r.is_err() && c.esc == EscKind::Redo { r = srb_block_redo(c, body, fp); }
    if !shared { c.frame = fr.prev; }
    match r {
        Ok(v) => Ok(v),
        Err(e) => if c.esc == EscKind::Next || (lambda && ((c.esc == EscKind::Return && c.esc_tag == id) || (c.esc == EscKind::Break && c.esc_tag == b.tag))) {
            c.esc = EscKind::None; Ok(c.esc_val)
        } else { Err(e) },
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_def(c: &mut Ctx, n: NodePtr, fp: Fp, mid: Sym, body: IseqRef) -> RV {
    srb_def(c, mid, body, fp)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_sdef(c: &mut Ctx, n: NodePtr, fp: Fp, recv: Value, mid: Sym, body: IseqRef) -> RV {
    srb_sdef(c, recv, mid, body)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_class<D_cpath: Disp, D_superclass: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, cpath: NodePtr, name: Sym, superclass: NodePtr, body: IseqRef, has_cpath: bool, has_super: bool) -> RV {
    let outer = if has_cpath { <D_cpath as Disp>::call(c, cpath, fp)? } else { UNDEF };
    let sup = if has_super { <D_superclass as Disp>::call(c, superclass, fp)? } else { UNDEF };
    srb_class(c, outer, name, sup, body, fp)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_module<D_cpath: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, cpath: NodePtr, name: Sym, body: IseqRef, has_cpath: bool) -> RV {
    let outer = if has_cpath { <D_cpath as Disp>::call(c, cpath, fp)? } else { UNDEF };
    srb_module(c, outer, name, body, fp)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_sclass(c: &mut Ctx, n: NodePtr, fp: Fp, recv: Value, body: IseqRef) -> RV {
    srb_sclass(c, recv, body, fp)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_alias(c: &mut Ctx, n: NodePtr, fp: Fp, newname: Value, oldname: Value) -> RV {
    srb_alias(c, newname, oldname)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_valias(c: &mut Ctx, n: NodePtr, fp: Fp, newname: Sym, oldname: Sym) -> RV {
    vm().global_aliases.insert(newname, oldname);
    Ok(NIL)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_undef<D_names: DispList>(c: &mut Ctx, n: NodePtr, fp: Fp, names: NodeList) -> RV {
    for i in 0..<D_names as DispList>::len(names) {
        let v = <D_names as DispList>::nth(c, names, i, fp)?;
        srb_undef(c, v)?;
    }
    Ok(NIL)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_masgn<D_targets: DispList>(c: &mut Ctx, n: NodePtr, fp: Fp, rhs: Value, tmp: u32, targets: NodeList, splat_rhs: bool) -> RV {
    let a = srb_masgn_array(c, rhs, splat_rhs)?;
    slot(fp, tmp).set(a);
    for i in 0..<D_targets as DispList>::len(targets) { <D_targets as DispList>::nth(c, targets, i, fp)?; }
    Ok(rhs)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_masgn_elem(c: &mut Ctx, n: NodePtr, fp: Fp, tmp: u32, idx: u32, kind: u32, post: u32) -> RV {
    let a = slot(fp, tmp).get();
    Ok(srb_masgn_elem(a, idx, kind, post))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_unsupported(c: &mut Ctx, n: NodePtr, fp: Fp, what: Str) -> RV {
    srb_unsupported(c, what)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_special<D_arg: Disp>(c: &mut Ctx, n: NodePtr, fp: Fp, kind: u32, arg: NodePtr) -> RV {
    srb_special::<D_arg>(c, n, fp, kind, arg)
}

