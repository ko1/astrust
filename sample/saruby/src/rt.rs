// Out-of-line runtime support for the nodes in node.def (slow paths).
use crate::heap::{obj_flags, obj_klass, obj_set_flags};
use crate::compile::*;
use crate::hash::*;
use crate::node::*;
use crate::object::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_body::*;   // no object accessor (docs/safe_heap.md)
use crate::string::*;
use crate::symbol::{intern, s, sym_str};
use crate::value::*;
use crate::vm::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

// ---- redefinition flags for operator fast paths -----------------------------------------

pub const R_INT_PLUS: u64 = 1 << 0;
pub const R_INT_MINUS: u64 = 1 << 1;
pub const R_INT_MUL: u64 = 1 << 2;
pub const R_INT_DIV: u64 = 1 << 3;
pub const R_INT_MOD: u64 = 1 << 4;
pub const R_INT_LT: u64 = 1 << 5;
pub const R_INT_LE: u64 = 1 << 6;
pub const R_INT_GT: u64 = 1 << 7;
pub const R_INT_GE: u64 = 1 << 8;
pub const R_INT_EQ: u64 = 1 << 9;
pub const R_INT_AND: u64 = 1 << 10;
pub const R_INT_OR: u64 = 1 << 11;
pub const R_FLO_PLUS: u64 = 1 << 12;
pub const R_FLO_MINUS: u64 = 1 << 13;
pub const R_FLO_MUL: u64 = 1 << 14;
pub const R_FLO_DIV: u64 = 1 << 15;
pub const R_FLO_LT: u64 = 1 << 16;
pub const R_FLO_LE: u64 = 1 << 17;
pub const R_FLO_GT: u64 = 1 << 18;
pub const R_FLO_GE: u64 = 1 << 19;
pub const R_FLO_EQ: u64 = 1 << 20;
pub const R_ARY_LTLT: u64 = 1 << 21;
pub const R_ARY_AREF: u64 = 1 << 22;
pub const R_ARY_ASET: u64 = 1 << 23;
pub const R_HASH_AREF: u64 = 1 << 24;
pub const R_HASH_ASET: u64 = 1 << 25;
pub const R_INT_LSHIFT: u64 = 1 << 26;
pub const R_INT_RSHIFT: u64 = 1 << 27;
pub const R_INT_XOR: u64 = 1 << 28;
pub const R_STR_LTLT: u64 = 1 << 29;
pub const R_FLO_POW: u64 = 1 << 30;
// == of nil / true / false / Symbol
pub const R_IMM_EQ: u64 = 1 << 31;
// === of the patterns `when` decides without a call (BOP_EQQ)
pub const R_INT_EQQ: u64 = 1 << 32;
pub const R_SYM_EQQ: u64 = 1 << 33;
pub const R_NIL_EQQ: u64 = 1 << 34;
pub const R_TRUE_EQQ: u64 = 1 << 35;
pub const R_FALSE_EQQ: u64 = 1 << 36;
pub const R_STR_EQQ: u64 = 1 << 37;
pub const R_STR_EQ: u64 = 1 << 40;
// ! of nil / true / false / Integer / Float / Symbol (node_not decides it for them without a lookup)
pub const R_IMM_NOT: u64 = 1 << 41;
// to_s of Integer / Symbol / nil in an interpolation (srb_tostr, srb_dstr_append)
pub const R_IMM_TO_S: u64 = 1 << 42;
// <=> of Integer / Float / String: OPTIMIZED_CMP's BOP_CMP (sort, min, max, sort_by, ...)
pub const R_INT_CMP: u64 = 1 << 43;
pub const R_FLO_CMP: u64 = 1 << 44;
pub const R_STR_CMP: u64 = 1 << 45;
// inspect of Integer / nil (inspect_value answers them without a call)
pub const R_IMM_INSPECT: u64 = 1 << 46;
// Hash#default (rb_hash_default_value)
pub const R_HASH_DEFAULT: u64 = 1 << 47;
// String#freeze / String#-@ ("lit".freeze, -"lit": opt_str_freeze / opt_str_uminus)
pub const R_STR_FREEZE: u64 = 1 << 48;
pub const R_STR_UMINUS: u64 = 1 << 49;
// Symbol#to_proc (a block argument `&:sym`)
pub const R_SYM_TO_PROC: u64 = 1 << 50;
// to_str / to_ary on Integer / Float / Symbol / nil / true / false, nil.to_a (the conversions skip immediates)
pub const R_IMM_CONV: u64 = 1 << 51;
// Array#each / Range#each (the Enumerable methods Array / Range answer natively)
pub const R_ARY_EACH: u64 = 1 << 52;
pub const R_RANGE_EACH: u64 = 1 << 53;
// Array#max / Array#min (`[a, b].max`: opt_newarray_send)
pub const R_ARY_MAX: u64 = 1 << 54;
pub const R_ARY_MIN: u64 = 1 << 55;
// Integer#succ / #coerce: numeric.rb's times loop, ArithmeticSequence#each's rb_num_coerce_relop
pub const R_INT_SUCC: u64 = 1 << 56;
pub const R_INT_COERCE: u64 = 1 << 57;
// Array#size: each_slice / each_cons ask for it (limit_by_enum_size)
pub const R_ARY_SIZE: u64 = 1 << 58;

pub static REDEF: AtomicU64 = AtomicU64::new(0);
pub static BOOTED: AtomicBool = AtomicBool::new(false);
#[inline(always)] pub fn booted() -> bool { BOOTED.load(Relaxed) }

// Integer#+ - * / % < <= > >= == & | << >> ^: the fixnum paths of operator nodes test all of them at once
// (c.int_redef()), so that one test covers every operator of a straight run of baked code
pub const R_INT_ALL: u64 = R_INT_PLUS | R_INT_MINUS | R_INT_MUL | R_INT_DIV | R_INT_MOD | R_INT_LT | R_INT_LE | R_INT_GT
    | R_INT_GE | R_INT_EQ | R_INT_AND | R_INT_OR | R_INT_LSHIFT | R_INT_RSHIFT | R_INT_XOR;
// (R_INT_ALL stays in bits 0..30: one `test` with an imm32 against the 64-bit word)
// === of the immediate patterns (node_when_lit, eqq_imm): one test, as CRuby's opt_case_dispatch
pub const R_EQQ_IMM: u64 = R_INT_EQQ | R_SYM_EQQ | R_NIL_EQQ | R_TRUE_EQQ | R_FALSE_EQQ;

#[inline(always)]
pub fn redef(f: u64) -> bool { REDEF.load(Relaxed) & f != 0 }
pub fn redef_word() -> u64 { REDEF.load(Relaxed) }
/// Might v answer to_str / to_ary?  Immediates only once one of those is defined for them.
#[inline(always)]
pub fn conv_target(v: Value) -> bool { v.is_heap() || redef(R_IMM_CONV) }

/// Called when a method is (re)defined after boot: disable affected fast paths.
pub fn note_redefinition(c: &mut Ctx, klass: Value, mid: Sym) {
    redef_note(c, klass, mid);
    c.redef = redef_word();
}

/// REDEF for (klass, mid); true if it changed.  The running context must then copy it (note_redefinition).
pub fn redef_note(c: &Ctx, klass: Value, mid: Sym) -> bool {
    if !booted() { return false; }
    if mid == s::puts || matches!(crate::symbol::sym_bytes(mid), b"write" | b"putc") { crate::builtins::io::note_out_method(c, klass); }
    if mid == s::lambda { LAMBDA_REDEF.store(true, Relaxed); }
    let before = redef_word();
    // a refinement of a basic operation turns its inline fast path off everywhere
    let klass = if c.class(klass).refined { c.class(klass).attached } else { klass };
    let vm = vm();
    let (int, flo, ary, hash, str) = (vm.c_integer.get(), vm.c_float.get(), vm.c_array.get(), vm.c_hash.get(), vm.c_string.get());
    let (nil, tru, fal, sym) = (vm.c_nil.get(), vm.c_true.get(), vm.c_false.get(), vm.c_symbol.get());
    let table: &[(Value, Sym, u64)] = &[
        (int, s::plus, R_INT_PLUS), (int, s::minus, R_INT_MINUS), (int, s::mul, R_INT_MUL), (int, s::div, R_INT_DIV),
        (int, s::modulo, R_INT_MOD), (int, s::lt, R_INT_LT), (int, s::le, R_INT_LE), (int, s::gt, R_INT_GT),
        (int, s::ge, R_INT_GE), (int, s::eq, R_INT_EQ), (int, s::and, R_INT_AND), (int, s::or, R_INT_OR),
        (int, s::ltlt, R_INT_LSHIFT), (int, s::gtgt, R_INT_RSHIFT), (int, s::xor, R_INT_XOR), (int, s::eqq, R_INT_EQQ),
        // != takes the == flag: the != shortcuts test that one (a bit of its own in R_INT_ALL would not fit an imm32)
        (int, s::neq, R_INT_EQ),
        (flo, s::plus, R_FLO_PLUS), (flo, s::minus, R_FLO_MINUS), (flo, s::mul, R_FLO_MUL), (flo, s::div, R_FLO_DIV),
        (flo, s::lt, R_FLO_LT), (flo, s::le, R_FLO_LE), (flo, s::gt, R_FLO_GT), (flo, s::ge, R_FLO_GE),
        (flo, s::eq, R_FLO_EQ), (flo, s::neq, R_FLO_EQ), (flo, s::pow, R_FLO_POW),
        (ary, s::ltlt, R_ARY_LTLT), (ary, s::aref, R_ARY_AREF), (ary, s::aset, R_ARY_ASET),
        (hash, s::aref, R_HASH_AREF), (hash, s::aset, R_HASH_ASET),
        (str, s::ltlt, R_STR_LTLT), (str, s::eqq, R_STR_EQQ), (str, s::eq, R_STR_EQ),
        (nil, s::eq, R_IMM_EQ), (tru, s::eq, R_IMM_EQ), (fal, s::eq, R_IMM_EQ), (sym, s::eq, R_IMM_EQ),
        (nil, s::eqq, R_NIL_EQQ), (tru, s::eqq, R_TRUE_EQQ), (fal, s::eqq, R_FALSE_EQQ), (sym, s::eqq, R_SYM_EQQ),
        (nil, s::not, R_IMM_NOT), (tru, s::not, R_IMM_NOT), (fal, s::not, R_IMM_NOT), (int, s::not, R_IMM_NOT),
        (flo, s::not, R_IMM_NOT), (sym, s::not, R_IMM_NOT),
        (int, s::to_s, R_IMM_TO_S), (sym, s::to_s, R_IMM_TO_S), (nil, s::to_s, R_IMM_TO_S),
        (int, s::cmp, R_INT_CMP), (flo, s::cmp, R_FLO_CMP), (str, s::cmp, R_STR_CMP),
        (int, s::inspect, R_IMM_INSPECT), (nil, s::inspect, R_IMM_INSPECT),
        (hash, s::default, R_HASH_DEFAULT), (str, s::freeze, R_STR_FREEZE), (str, s::uminus, R_STR_UMINUS),
        (sym, s::to_proc, R_SYM_TO_PROC), (ary, s::each, R_ARY_EACH), (ary, s::max, R_ARY_MAX), (ary, s::min, R_ARY_MIN), (vm.c_range.get(), s::each, R_RANGE_EACH),
        (int, s::to_str, R_IMM_CONV), (flo, s::to_str, R_IMM_CONV), (sym, s::to_str, R_IMM_CONV), (nil, s::to_str, R_IMM_CONV),
        (tru, s::to_str, R_IMM_CONV), (fal, s::to_str, R_IMM_CONV), (int, s::to_ary, R_IMM_CONV), (flo, s::to_ary, R_IMM_CONV),
        (sym, s::to_ary, R_IMM_CONV), (nil, s::to_ary, R_IMM_CONV), (tru, s::to_ary, R_IMM_CONV), (fal, s::to_ary, R_IMM_CONV),
        (nil, s::to_a, R_IMM_CONV), (int, s::succ, R_INT_SUCC), (int, s::coerce, R_INT_COERCE), (ary, s::size, R_ARY_SIZE),
    ];
    for &(k, m, f) in table {
        if m == mid && (klass == k || ancestors(c, k).contains(&klass)) { REDEF.fetch_or(f, Relaxed); }
    }
    if redef_word() == before { return false; }
    if redef(R_ARY_AREF) { FAST_ARY_AREF.store(0, Relaxed); }
    if redef(R_ARY_ASET) { FAST_ARY_ASET.store(0, Relaxed); }
    true
}

// The Array class while Array#[] / Array#[]= are the builtins, else 0 (no object's class): an index
// site tests the receiver's class and the redefinition in one compare.  Set on the first miss after boot.
pub static FAST_ARY_AREF: AtomicU64 = AtomicU64::new(0);
pub static FAST_ARY_ASET: AtomicU64 = AtomicU64::new(0);
#[inline(always)]
pub fn fast_ary_aref() -> u64 { FAST_ARY_AREF.load(Relaxed) }
#[inline(always)]
pub fn fast_ary_aset() -> u64 { FAST_ARY_ASET.load(Relaxed) }

/// `a[b]` when the site's inline path did not apply: everything, starting with the Array and Hash cases.
#[inline(never)]
#[cold]
pub fn srb_aref_out(c: &mut Ctx, a: Value, b: Value, line: Pos, cc: &OpCc) -> RV {
    if booted() && !redef(R_ARY_AREF) { FAST_ARY_AREF.store(vm().c_array.get().raw(), Relaxed); }
    use crate::builtins::{array_methods as am, hash_methods as hm};
    if b.is_fixnum() && is_array(a) && obj_klass(a) == vm().c_array.get() && !redef(R_ARY_AREF) { return Ok(am::ary_entry(c, a, b.as_fix())); }
    if is_hash(a) && obj_klass(a) == vm().c_hash.get() && !redef(R_HASH_AREF) {
        c.frame_mut().pos = line;
        return hm::h_aref(c, a, &[b], Blk::NONE);
    }
    // Integer#[] (optcarrot's bit tests): its body when the site's cache holds it, without the call
    use crate::inline_builtin::*;
    if cc.peek().is_some_and(|cc| bi_hit(cc, a, BI_INT_AREF)) { if let Some(v) = bi_body(c, BI_INT_AREF, a, b) { return Ok(v); } }
    srb_binop(c, a, b, s::aref, line, cc)
}

/// `a << b` when the site's inline path did not apply: Array#<< of a frozen Array raises from this line, as CRuby.
#[inline(never)]
#[cold]
pub fn srb_ltlt_out(c: &mut Ctx, a: Value, b: Value, line: Pos, cc: &OpCc) -> RV {
    if is_array(a) && obj_klass(a) == vm().c_array.get() && !redef(R_ARY_LTLT) {
        c.frame_mut().pos = line;
        return crate::builtins::array_methods::a_ltlt(c, a, &[b], Blk::NONE);
    }
    srb_binop(c, a, b, s::ltlt, line, cc)
}

/// `a[b] = v`, as srb_aref_out.
#[inline(never)]
#[cold]
pub fn srb_aset_out(c: &mut Ctx, a: Value, b: Value, v: Value, line: Pos, cc: &OpCc) -> RV {
    if booted() && !redef(R_ARY_ASET) { FAST_ARY_ASET.store(vm().c_array.get().raw(), Relaxed); }
    use crate::builtins::{array_methods as am, hash_methods as hm};
    // Array#[]= / Hash#[]= themselves: errors (frozen, index) with this line and no frame of their own, as CRuby
    if b.is_fixnum() && is_array(a) && obj_klass(a) == vm().c_array.get() && !redef(R_ARY_ASET) {
        c.frame_mut().pos = line;
        return am::ary_store(c, a, b.as_fix(), v);
    }
    if is_hash(a) && obj_klass(a) == vm().c_hash.get() && !redef(R_HASH_ASET) {
        c.frame_mut().pos = line;
        return hm::h_aset(c, a, &[b, v], Blk::NONE);
    }
    srb_aset(c, a, b, v, line, cc)
}

// ---- literals ----------------------------------------------------------------------------

/// node_fstr_recv off its cached path: a fresh copy once the method (freeze: um 0, -@: um 1) is redefined.
#[inline(never)]
pub fn srb_fstr_recv(c: &mut Ctx, s: Str, enc: u32, um: u32, lc: &LitCache) -> Value {
    if c.redef_any(if um == 0 { R_STR_FREEZE } else { R_STR_UMINUS }) { return c.str_new_enc(s, enc as u8); }
    match lc.v.get() { Some(v) => v, None => srb_fstr(c, s, enc, lc) }
}

pub fn srb_fstr(c: &mut Ctx, s: Str, enc: u32, lc: &LitCache) -> Value {
    // frozen literals share the -"str" pool, like CRuby's fstring table
    let found = vm().frozen_lits.borrow().get(s).copied();   // not borrowed across the allocation below
    let v = match found {
        Some(v) if c.str_enc(v) == enc as u8 => v,
        other => {
            let pooled = other.is_some();
            let v = c.str_new_enc(s, enc as u8);
            obj_set_flags(v, FL_FROZEN);
            if pooled { vm().node_values.borrow_mut().push(v); } else { vm().frozen_lits.borrow_mut().insert(s.to_vec(), v); }
            v
        }
    };
    lc.v.set(Some(v));
    v
}

/// Append an interpolated value to the string being built (acc is fresh: mutable, unshared).
pub fn srb_dstr_append(c: &mut Ctx, acc: Value, v: Value) -> Result<(), Esc> {
    if is_string(v) {
        let (a, others) = c.str_mut_and(acc);
        let n = match others.rstring(v) {
            Some(b) if a.enc == b.enc => { a.b.extend_from_slice(&b.b); b.b.len() }
            _ => return str_append_str(c, acc, v),
        };
        c.malloc_increase(n);
        return Ok(());
    }
    if v.is_fixnum() && !redef(R_IMM_TO_S) {
        // an empty acc would take the Integer string's US-ASCII encoding: left to the slow path
        let a = c.rstring_mut(acc);
        if !a.b.is_empty() && a.enc <= ENC_ASCII {
            let mut buf = [0u8; 20];
            let i = v.as_fix();
            let mut u = i.unsigned_abs();
            let mut k = buf.len();
            loop { k -= 1; buf[k] = b'0' + (u % 10) as u8; u /= 10; if u == 0 { break; } }
            if i < 0 { k -= 1; buf[k] = b'-'; }
            a.b.extend_from_slice(&buf[k..]);
            return Ok(());
        }
    }
    let s = srb_tostr(c, v)?;
    str_append_str(c, acc, s)
}

pub fn srb_tostr(c: &mut Ctx, v: Value) -> RV {
    if is_string(v) { return Ok(v); }
    if crate::vm::refinements_on() {
        if let Some(r) = funcall_refined(c, std::ptr::null(), v, s::to_s, &[], std::ptr::null(), CF_FCALL) { let r = r?; if is_string(r) { return Ok(r); } }
    }
    if !redef(R_IMM_TO_S) {
        if v.is_fixnum() { return Ok(c.str_new(v.as_fix().to_string().as_bytes())); }
        if v.is_sym() { return Ok(c.str_new(crate::symbol::sym_bytes(v.as_sym()))); }
        if v == NIL { return Ok(c.str_new(b"")); }
    }
    let r = funcall(c, v, s::to_s, &[], std::ptr::null())?;
    if is_string(r) { return Ok(r); }
    let t = crate::builtins::kernel::any_to_s_str(c, v);
    Ok(c.str_new(t.as_bytes()))
}

pub fn srb_regex_lit(c: &mut Ctx, src: Str, opts: u32, lc: &LitCache) -> RV {
    let r = crate::regexp::regexp_new_lit(c, src, opts)?;
    obj_set_flags(r, FL_FROZEN);
    vm().node_values.borrow_mut().push(r);
    lc.v.set(Some(r));
    Ok(r)
}

pub fn srb_dregex(c: &mut Ctx, s: Value, opts: u32, once: bool, lc: &LitCache) -> RV {
    let r = crate::regexp::regexp_new_dyn(c, s, opts)?;
    if once { vm().node_values.borrow_mut().push(r); lc.v.set(Some(r)); }
    Ok(r)
}

pub fn srb_biglit(c: &mut Ctx, digits: Str, lc: &LitCache) -> Value {
    let b = crate::bignum::BigInt::parse(std::str::from_utf8(digits).unwrap(), 10).unwrap();
    let v = c.big_to_value(b);
    vm().node_values.borrow_mut().push(v);
    lc.v.set(Some(v));
    v
}

pub fn srb_splat_into(c: &mut Ctx, a: Value, v: Value) -> Result<(), Esc> {
    let arr = splat_to_array(c, v)?;
    let n = c.ary(arr).len();
    for i in 0..n { let x = c.ary(arr)[i]; c.ary_push(a, x); }
    Ok(())
}

/// *v: Array as is, nil → [], to_a if defined, else [v].
pub fn splat_to_array(c: &mut Ctx, v: Value) -> RV {
    if is_array(v) { return Ok(v); }
    if v == NIL { return Ok(c.ary_new()); }
    if is_hash(v) { return crate::hash::hash_to_a(c, v); }
    if respond_to(c, v, s::to_a, true) {
        let r = funcall(c, v, s::to_a, &[], std::ptr::null())?;
        if is_array(r) { return Ok(r); }
        if r == NIL { return Ok(c.ary_new_from(&[v])); }
        return type_err(c, format!("can't convert {} to Array ({}#to_a gives {})", class_path(c, real_class_of(c, v)), class_path(c, real_class_of(c, v)), class_path(c, real_class_of(c, r))));
    }
    Ok(c.ary_new_from(&[v]))
}

pub fn srb_hash_merge_kw(c: &mut Ctx, h: Value, v: Value) -> Result<(), Esc> {
    if v == NIL { return Ok(()); }
    let src = if is_hash(v) { v } else {
        if !respond_to(c, v, s::to_hash, true) {
            return type_err(c, format!("no implicit conversion of {} into Hash", crate::builtins::kernel::class_name_for_err(c, v)));
        }
        let r = funcall(c, v, s::to_hash, &[], std::ptr::null())?;
        if !is_hash(r) {
            let cls = class_path(c, real_class_of(c, v));
            return type_err(c, format!("can't convert {} to Hash ({}#to_hash gives {})", cls, cls, class_path(c, real_class_of(c, r))));
        }
        r
    };
    crate::hash::hash_copy_entries(c, h, src)
}

pub fn srb_range_new(c: &mut Ctx, b: Value, e: Value, excl: bool) -> RV {
    crate::range::range_new_checked(c, b, e, excl)
}

pub fn srb_range_lit(c: &mut Ctx, b: Value, e: Value, excl: bool, lc: &LitCache) -> Value {
    let r = c.range_new(b, e, excl);
    obj_set_flags(r, FL_FROZEN);
    vm().node_values.borrow_mut().push(r);
    lc.v.set(Some(r));
    r
}

// ---- variables ----------------------------------------------------------------------------

pub fn srb_ivar_get(c: &Ctx, slf: Value, name: Sym, ic: &IvarCache) -> Value {
    if is_object(slf) {
        let k = real_class_of(c, slf);
        if let Some(&i) = c.class(k).ivar_index.get(&name) {
            if obj_klass(slf) == k { ic.klass.set(k.raw()); ic.idx.set(i); }
        }
    }
    let v = ivar_get(c, slf, name);
    // a class's ivar in a non-main Ractor: checked at the next check point (this path cannot raise)
    if crate::builtins::ractor::multi() { crate::builtins::ractor::class_ivar_read(c, slf, name, v); }
    v
}

/// A site's way out of its inline path: the cache without the baked slot first (a stale slot stays cheap), then the miss.
#[cold]
#[inline(never)]
pub fn srb_ivar_get_out(slf: Value, name: Sym, ic: &IvarCache, c: &Ctx) -> Value {
    if slf.is_heap() && obj_klass(slf).raw() == ic.klass.get() {
        if let Some(ivs) = c.obj_ivars(slf) {
            return match ivs.get(ic.idx.get() as usize) { Some(v) if v != UNDEF => v, _ => NIL };
        }
    }
    srb_ivar_get(c, slf, name, ic)
}

#[cold]
#[inline(never)]
pub fn srb_ivar_set_out(c: &mut Ctx, slf: Value, name: Sym, v: Value, ic: &IvarCache) -> RV {
    if slf.is_heap() && obj_klass(slf).raw() == ic.klass.get() && obj_flags(slf) & FL_FROZEN == 0 {
        if let Some(ivs) = c.obj_ivars_mut(slf) {
            if ivs.set(ic.idx.get() as usize, v) { return Ok(v); }
        }
    }
    srb_ivar_set(c, slf, name, v, ic)
}

/// `@x += v` / `@x -= v` outside node_ivar_arith's inline path: read, operate, write.
#[inline(never)]
#[cold]
pub fn srb_ivar_arith_out(c: &mut Ctx, slf: Value, name: Sym, sub: bool, v: Value, line: Pos, ic: &IvarCache, cc: &OpCc) -> RV {
    let cur = srb_ivar_get_out(slf, name, ic, c);
    let r = crate::opt::srb_arith_out(c, cur, v, if sub { s::minus } else { s::plus }, line, cc)?;
    srb_ivar_set_out(c, slf, name, r, ic)
}

pub fn srb_ivar_set(c: &mut Ctx, slf: Value, name: Sym, v: Value, ic: &IvarCache) -> RV {
    if crate::builtins::ractor::multi() { crate::builtins::ractor::class_ivar_set_check(c, slf, name)?; }
    ivar_set(c, slf, name, v)?;
    if is_object(slf) {
        let k = real_class_of(c, slf);
        if obj_klass(slf) == k {
            ic.klass.set(k.raw()); ic.idx.set(ivar_index_for(c, k, name));
        }
    }
    Ok(v)

}

pub fn srb_gvar_get(c: &mut Ctx, name: Sym) -> RV {
    if crate::builtins::ractor::multi() { crate::builtins::ractor::gvar_check(c, name)?; }
    crate::builtins::kernel::gvar_get(c, name)
}
pub fn srb_gvar_set(c: &mut Ctx, name: Sym, v: Value) -> RV {
    if crate::builtins::ractor::multi() { crate::builtins::ractor::gvar_check(c, name)?; }
    crate::builtins::kernel::gvar_set(c, name, v)
}

/// The class whose class variables the current code sees (lexical cref, skipping singletons).
fn cvar_base(c: &Ctx) -> Value {
    let mut cr = unsafe { (*c.frame).cref() };
    unsafe {
        while !cr.is_null() {
            let k = (*cr).klass;
            if !c.class(k).is_singleton || (*cr).prev.is_null() {
                if c.class(k).is_singleton { let a = c.class(k).attached; if is_class_or_module(a) { return a; } }
                return k;
            }
            cr = (*cr).prev;
        }
    }
    vm().c_object.get()
}

pub fn cvar_lookup(c: &Ctx, k: Value, name: Sym) -> Option<(Value, Value)> {
    for &a in ancestors(c, k).iter() {
        if let Some(&v) = c.class(a).cvars.get(&name) { return Some((a, v)); }
    }
    None
}

/// @@x where the lexical scope is the top level (whatever self is) is an error.
fn cvar_toplevel_check(c: &mut Ctx) -> Result<(), Esc> {
    let cr = unsafe { (*c.frame).cref() };
    if cr.is_null() || unsafe { (*cr).prev.is_null() } { let r: RV = err(c, vm().c_runtime_error.get(), "class variable access from toplevel"); return r.map(drop); }
    Ok(())
}

pub fn srb_cvar_get(c: &mut Ctx, name: Sym) -> RV {
    cvar_toplevel_check(c)?;
    let k = cvar_base(c);
    if crate::builtins::ractor::multi() { crate::builtins::ractor::cvar_check(c, k, name)?; }
    match cvar_lookup(c, k, name) {
        Some((_, v)) => Ok(v),
        None => {
            let msg = format!("uninitialized class variable {} in {}", sym_str(name), class_path(c, k));
            let e = make_name_error(c, vm().c_name_error.get(), &msg, Value::sym(name), k)?;
            Err(raise_exc(c, e))
        }
    }
}

pub fn srb_cvar_set(c: &mut Ctx, name: Sym, v: Value) -> RV {
    cvar_toplevel_check(c)?;
    let k = cvar_base(c);
    if crate::builtins::ractor::multi() { crate::builtins::ractor::cvar_check(c, k, name)?; }
    match cvar_lookup(c, k, name) {
        Some((owner, _)) => { c.class_mut(owner).cvars.insert(name, v); }
        None => { c.class_mut(k).cvars.insert(name, v); }
    }
    Ok(v)
}

#[cold]
pub fn srb_const_get(c: &mut Ctx, cref: *const Cref, name: Sym, cc: &ConstCache) -> RV {
    let v = const_lookup(c, cref, name)?;
    // while other Ractors live only shareable values are cached: every Ractor may read those
    if crate::builtins::ractor::multi() && !crate::builtins::ractor::shareable(c, v) {
        if crate::builtins::ractor::isolated(c) {
            let (k, lexical) = crate::builtins::ractor::const_found_in(c, cref, name);
            crate::builtins::ractor::const_check(c, k, lexical, name, v)?;
        }
        return Ok(v);
    }
    cc.serial.set(vm().const_serial.get());
    cc.cref.set(cref as usize);
    cc.val.set(v.raw());
    Ok(v)
}

pub fn srb_const_path(c: &mut Ctx, parent: Value, name: Sym) -> RV {
    let parent = if parent == UNDEF { vm().c_object.get() } else { parent };
    if !is_class_or_module(parent) {
        let s = crate::builtins::kernel::inspect_str(c, parent)?;
        return type_err(c, format!("{} is not a class/module", s));
    }
    if let Some(v) = const_get_inherit(c, parent, name)? {
        if let Some(m) = const_access_check(c, parent, name, true)? { return Ok(m); }
        if crate::builtins::ractor::multi() { crate::builtins::ractor::const_check(c, parent, false, name, v)?; }
        return Ok(v);
    }
    const_missing(c, parent, name)
}

/// `parent::Name = v` (parent UNDEF: `::Name = v`)
pub fn srb_cdecl_path(c: &mut Ctx, parent: Value, name: Sym, v: Value) -> RV {
    let k = if parent == UNDEF { vm().c_object.get() } else { parent };
    if !is_class_or_module(k) {
        let s = crate::builtins::kernel::inspect_str(c, k)?;
        return type_err(c, format!("{} is not a class/module", s));
    }
    srb_cdecl(c, k, name, v)
}

pub fn srb_cdecl(c: &mut Ctx, parent: Value, name: Sym, v: Value) -> RV {
    let k = if parent == NIL {
        let cr = unsafe { (*c.frame).cref() };
        if cr.is_null() { vm().c_object.get() } else { unsafe { (*cr).klass } }
    } else if parent == UNDEF { vm().c_object.get() } else { parent };
    if !is_class_or_module(k) {
        let s = crate::builtins::kernel::inspect_str(c, k)?;
        return type_err(c, format!("{} is not a class/module", s));
    }
    if crate::builtins::ractor::multi() { crate::builtins::ractor::cdecl_check(c, v)?; }
    const_set_here(c, k, name, v)?;
    Ok(v)
}

pub fn srb_back_ref(c: &mut Ctx, kind: u32) -> RV {
    let m = crate::builtins::svar::last_match(c);
    if m == NIL { return Ok(NIL); }
    Ok(match kind {
        0 => crate::regexp::nth_match(c, m, 0),
        1 => crate::regexp::match_pre(c, m),
        2 => crate::regexp::match_post(c, m),
        _ => crate::regexp::match_last_group(c, m),
    })
}

// ---- control ------------------------------------------------------------------------------

pub fn srb_has_custom_not(c: &Ctx, v: Value) -> bool {
    // a refined ! is not in the class's table: the call finds it
    if crate::vm::refinements_on() { return true; }
    let me = lookup_method(c, class_of(v), s::not);
    !me.is_null() && unsafe { (*me).owner } != vm().c_basic_object.get()
}

#[cold]
/// vm_opt_newarray_max / _min: OPTIMIZED_CMP(v, best) for each later element v.
#[inline(never)]
pub fn srb_newarray_minmax(c: &mut Ctx, a: Value, mx: bool, cc: &OpCc) -> RV {
    if redef(if mx { R_ARY_MAX } else { R_ARY_MIN }) { return srb_call0(c, a, if mx { s::max } else { s::min }, cc); }
    let mut best = match c.ary(a).first() { Some(&v) => v, None => return Ok(NIL) };
    for i in 1..c.ary_len(a) {
        let v = c.ary_get(a, i);
        let o = crate::builtins::array_methods::rb_cmp(c, v, best)?;
        if if mx { o.is_gt() } else { o.is_lt() } { best = v; }
    }
    Ok(best)
}

pub fn srb_call0(c: &mut Ctx, v: Value, mid: Sym, cc: &OpCc) -> RV {
    let base = stage(c, 2)?;
    base[0].set(v);
    call_cached(c, cc.get(), mid, base, 0, std::ptr::null(), 0)
}

/// `redo` in a block entered through its entry node: run the body again until it ends otherwise.
/// A loop's back edge saw the interrupt bit (vm::ints_pending).
#[cold]
#[inline(never)]
pub fn srb_poll(c: &mut Ctx) -> Result<(), Esc> { crate::builtins::thread::poll(c) }

#[cold]
#[inline(never)]
pub fn srb_block_redo(c: &mut Ctx, body: NodePtr, fp: Fp) -> RV {
    loop {
        c.esc = EscKind::None;
        let r = unsafe { ((*body).head.dispatcher)(c, body, fp) };
        if !(r.is_err() && c.esc == EscKind::Redo) { return r; }
    }
}

pub fn srb_break(c: &mut Ctx, v: Value) -> RV {
    let f = c.frame();
    if f.self_blk().is_null() {
        return err(c, vm().c_local_jump_error.get(), "break from proc-closure");
    }
    let tag = unsafe { (*f.self_blk()).tag };
    // a proc's break needs the call that took the literal block to be still running
    if f.kind == FK_BLOCK && !break_target_alive(c, tag) {
        return Err(raise_local_jump(c, "break from proc-closure", "break", v));
    }
    c.esc = EscKind::Break;
    c.esc_val = v;
    c.esc_tag = tag;
    Err(Esc)
}

#[cold]
fn break_target_alive(c: &Ctx, tag: u64) -> bool {
    let mut g = c.frame;
    while !g.is_null() {
        let b = unsafe { (*g).blk };
        if !b.is_null() {
            let bb = unsafe { &*b };
            if bb.tag == tag && (bb.proc_val == UNDEF || !is_proc(bb.proc_val) || b != proc_block(c, bb.proc_val)) { return true; }
        }
        g = unsafe { (*g).prev };
    }
    false
}

/// `return` inside a proc: its home method must still be running.
#[cold]
pub fn srb_block_return(c: &mut Ctx, v: Value) -> RV {
    let id = unsafe { (*c.frame).id };
    let mut g = c.frame;
    let mut alive = false;
    while !g.is_null() {
        let k = unsafe { (*g).kind };
        if unsafe { (*g).id } == id && k != FK_BLOCK && k != FK_CFUNC { alive = true; break; }
        g = unsafe { (*g).prev };
    }
    if !alive || home_is_class_body(c, id) { return Err(raise_local_jump(c, "unexpected return", "return", v)); }
    c.esc = EscKind::Return;
    c.esc_val = v;
    c.esc_tag = id;
    Err(Esc)
}

/// A block's `return` cannot leave a class/module body (it can leave `class << x`, see srb_sclass).
fn home_is_class_body(c: &Ctx, id: u64) -> bool {
    let mut g = c.cur_frame();
    while let Some(f) = g {
        if f.id == id && f.kind != FK_BLOCK && f.kind != FK_CFUNC {
            return f.kind == FK_CLASS && c.iseq_ref(f.iseq()).is_some_and(|is| is.info.label != "singleton class");
        }
        g = c.prev_frame(f);
    }
    false
}

#[cold]
pub fn raise_local_jump(c: &mut Ctx, msg: &str, reason: &str, v: Value) -> Esc {
    let e = make_exception(c, vm().c_local_jump_error.get(), msg);
    ivar_set_raw(c, e, crate::symbol::intern(b"__reason"), sym_value(reason));
    ivar_set_raw(c, e, crate::symbol::intern(b"__exit_value"), v);
    raise_exc(c, e)
}

pub fn srb_rescuable(c: &Ctx, exc: Value) -> bool { is_exception(c, exc) }

pub fn srb_is_standard_error(c: &Ctx, exc: Value) -> bool { is_kind_of(c, exc, vm().c_standard_error.get()) }

pub fn srb_rescue_match(c: &mut Ctx, k: Value, exc: Value, splat: bool) -> Result<bool, Esc> {
    if splat {
        let a = splat_to_array(c, k)?;
        let n = c.ary(a).len();
        for i in 0..n { let x = c.ary(a)[i]; if srb_rescue_match(c, x, exc, false)? { return Ok(true); } }
        return Ok(false);
    }
    if is_class_or_module(k) {
        let me = lookup_method(c, class_of(k), s::eqq);
        if !me.is_null() && matches!(unsafe { (*me).kind }, MKind::Cfunc(..)) { return Ok(is_kind_of(c, exc, k)); }
        let r = funcall(c, k, s::eqq, &[exc], std::ptr::null())?;
        return Ok(r.truthy());
    }
    if !respond_to(c, k, s::eqq, true) || !k.is_heap() {
        return type_err(c, "class or module required for rescue clause");
    }
    let r = funcall(c, k, s::eqq, &[exc], std::ptr::null())?;
    Ok(r.truthy())
}

pub fn srb_when_splat(c: &mut Ctx, p: Value, subj: Value) -> Result<bool, Esc> {
    let a = splat_to_array(c, p)?;
    let n = c.ary(a).len();
    let cc = CallCache::default();
    for i in 0..n { let x = c.ary(a)[i]; if srb_eqq(c, x, subj, &cc)? { return Ok(true); } }
    Ok(false)
}

#[cold]
pub fn srb_eqq(c: &mut Ctx, p: Value, subj: Value, cc: &CallCache) -> Result<bool, Esc> {
    if !redef(R_EQQ_IMM) {
        if p.is_fixnum() && subj.is_fixnum() { return Ok(p == subj); }
        if p.is_sym() { return Ok(p == subj); }
        if p == NIL || p == TRUE || p == FALSE { return Ok(p == subj); }
    }
    if is_string(p) && is_string(subj) && obj_klass(p) == vm().c_string.get() && !redef(R_STR_EQQ) { return Ok(c.str_eq(p, subj)); }
    let base = stage(c, 3)?;
    base[0].set(p); base[2].set(subj);
    let r = call_cached(c, cc, s::eqq, base, 1, std::ptr::null(), 0)?;
    Ok(r.truthy())
}

/// srb_eqq for a `when` site; an answer of the builtin Module#=== is kept in the site's memo.
#[inline(never)]
#[cold]
pub fn srb_eqq_class(c: &mut Ctx, p: Value, subj: Value, cc: &CallCache, km: &KindMemo) -> Result<bool, Esc> {
    let r = srb_eqq(c, p, subj, cc)?;
    // cc.me is what the call just ran when the cache is p's (a polymorphic site keeps others elsewhere)
    if p.is_heap() && cc.klass.get() == obj_klass(p).raw() && cc.me.get() == vm().ids.m_eqq.get() as usize {
        let (k, ser) = (class_of(subj).raw(), kind_serial());
        if km.pat.get() != p.raw() || km.ser.get() != ser { km.pat.set(p.raw()); km.ser.set(ser); for m in &km.memo { m.set(0); } }
        km.memo[kind_way(k)].set(k | r as u64);
    }
    Ok(r)
}

/// defined? answers are pooled frozen US-ASCII strings.
fn defined_str(c: &mut Ctx, s: &[u8]) -> Value {
    let mut key = vec![0xff, b'd'];
    key.extend_from_slice(s);
    if let Some(&v) = vm().frozen_lits.borrow().get(&key) { return v; }
    let v = c.str_new_enc(s, ENC_ASCII);
    obj_set_flags(v, FL_FROZEN);
    vm().frozen_lits.borrow_mut().insert(key, v);
    v
}

pub fn srb_defined<D: Disp>(c: &mut Ctx, _n: NodePtr, fp: Fp, kind: u32, name: Sym, recv: NodePtr) -> RV {
    let yes = |c: &mut Ctx, s: &[u8]| -> RV { Ok(defined_str(c, s)) };
    match kind {
        DEF_LOCALVAR => yes(c, b"local-variable"),
        DEF_SELF => yes(c, b"self"),
        DEF_NIL => yes(c, b"nil"),
        DEF_TRUE => yes(c, b"true"),
        DEF_FALSE => yes(c, b"false"),
        DEF_ASGN => yes(c, b"assignment"),
        DEF_BACKREF => yes(c, b"global-variable"),
        DEF_IVAR => if ivar_defined(c, fp[0].get(), name) { yes(c, b"instance-variable") } else { Ok(NIL) },
        DEF_GVAR => if crate::builtins::kernel::gvar_defined(c, name) { yes(c, b"global-variable") } else { Ok(NIL) },
        DEF_CVAR => {
            let k = cvar_base(c);
            if cvar_lookup(c, k, name).is_some() { yes(c, b"class variable") } else { Ok(NIL) }
        }
        DEF_CONST => {
            let cref = unsafe { (*c.frame).cref() };
            let saved = (c.esc, c.esc_val);
            match const_defined_lex(c, cref, name) { true => yes(c, b"constant"), false => { c.esc = saved.0; c.esc_val = saved.1; Ok(NIL) } }
        }
        DEF_CPATH => {
            let p = match D::call(c, recv, fp) { Ok(v) => v, Err(_) => { c.esc = EscKind::None; return Ok(NIL); } };
            let p = if p == UNDEF { vm().c_object.get() } else { p };
            if !is_class_or_module(p) { return Ok(NIL); }
            if ancestors(c, p).iter().any(|&k| has_const(c, k, name))
                || (c.class(p).is_module && c.class(vm().c_object.get()).consts.contains_key(&name)) { yes(c, b"constant") } else { Ok(NIL) }
        }
        DEF_FCALL => {
            let slf = fp[0].get();
            if respond_to(c, slf, name, true) { return yes(c, b"method"); }
            let rm = lookup_method(c, class_of(slf), s::respond_to_missing_p);
            if !rm.is_null() && unsafe { (*rm).owner } != vm().c_object.get() && unsafe { (*rm).owner } != vm().m_kernel.get() {
                let t = funcall(c, slf, s::respond_to_missing_p, &[Value::sym(name), TRUE], std::ptr::null())?;
                if t.truthy() { return yes(c, b"method"); }
            }
            Ok(NIL)
        }
        DEF_METHOD => {
            let r = match D::call(c, recv, fp) { Ok(v) => v, Err(_) => { c.esc = EscKind::None; return Ok(NIL); } };
            if r == UNDEF { return Ok(NIL); }
            let me = lookup_method(c, class_of(r), name);
            if me.is_null() {
                // respond_to_missing?
                let rm = lookup_method(c, class_of(r), s::respond_to_missing_p);
                if !rm.is_null() && unsafe { (*rm).owner } != vm().c_object.get() && unsafe { (*rm).owner } != vm().m_kernel.get() {
                    let t = funcall(c, r, s::respond_to_missing_p, &[Value::sym(name), TRUE], std::ptr::null())?;
                    if t.truthy() { return yes(c, b"method"); }
                }
                return Ok(NIL);
            }
            let visi = unsafe { (*me).visi };
            if visi == VISI_PUBLIC { return yes(c, b"method"); }
            if visi == VISI_PROTECTED && is_kind_of(c, fp[0].get(), unsafe { (*me).owner }) { return yes(c, b"method"); }
            Ok(NIL)
        }
        DEF_YIELD => if unsafe { !(*c.frame).blk.is_null() } { yes(c, b"yield") } else { Ok(NIL) },
        DEF_SUPER => {
            let f = c.frame();
            if f.me.is_null() { return Ok(NIL); }
            let me = unsafe { &*f.me };
            let sm = super_method(c, class_of(fp[0].get()), me.owner, me.name);
            if sm.is_null() { Ok(NIL) } else { yes(c, b"super") }
        }
        _ => yes(c, b"expression"),
    }
}

fn const_defined_lex(c: &mut Ctx, cref: *const Cref, name: Sym) -> bool {
    let mut cr = cref;
    unsafe {
        while !cr.is_null() && !(*cr).prev.is_null() {
            let k = (*cr).klass;
            if has_const(c, k, name) { return true; }
            cr = (*cr).prev;
        }
    }
    let start = if cref.is_null() { vm().c_object.get() } else { unsafe { (*cref).klass } };
    let _ = c;
    ancestors(c, start).iter().any(|&k| has_const(c, k, name)) || has_const(c, vm().c_object.get(), name)
}

// ---- calls ---------------------------------------------------------------------------------

#[inline(always)]
pub fn make_block(c: &mut Ctx, is: &'static Iseq, fp: Fp) -> Block {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("make_block");
    let f = unsafe { &*c.frame };
    let tag = c.next_serial();
    Block {
        kind: BlockKind::Iseq(is), self_: fp[0].get(), env: fp[1].get(), me: f.me, outer_blk: f.blk,
        cref: f.cref(), home_id: f.id, tag, proc_val: UNDEF, defined_class: f.defined_class(),
    }
}

pub fn srb_callx_arg(c: &mut Ctx, ci: CiRef, i: usize, v: Value, tmp: Value, kw: &mut Value, blkv: &mut Value) -> Result<(), Esc> {
    match ci.kinds[i] {
        AK_ARG => { c.ary_push(tmp, v); }
        AK_SPLAT => {
            let a = splat_to_array(c, v)?;
            let n = c.ary(a).len();
            for j in 0..n { let x = c.ary(a)[j]; c.ary_push(tmp, x); }
            // a ruby2_keywords hash at the end of a splat is passed as keywords
            if n > 0 {
                let last = c.ary(a)[n - 1];
                if is_hash(last) && obj_flags(last) & FL_KWHASH != 0 && *kw == NIL {
                    c.ary_mut(tmp).pop();
                    let h = c.hash_new();
                    crate::hash::hash_copy_entries(c, h, last)?;
                    c.ary_push(tmp, h);
                    *kw = h;
                    c.ary_mut(tmp).pop();
                }
            }
        }
        AK_KW => {
            if *kw == NIL { *kw = c.hash_new(); c.ary_push(tmp, *kw); c.ary_mut(tmp).pop(); }
            srb_hash_merge_kw(c, *kw, v)?;
        }
        AK_KWSPLAT => {
            if *kw == NIL { *kw = c.hash_new(); }
            if v != NIL { srb_hash_merge_kw(c, *kw, v)?; }
            // keep kw reachable through tmp's spare slot
        }
        _ => { *blkv = v; }
    }
    // keep kw and blkv rooted (they may only be in this frame's registers)
    if *kw != NIL || *blkv != NIL {
        let base = tmp;
        let _ = base;
    }
    Ok(())
}

fn block_from_value(c: &mut Ctx, blkv: Value, holder: &mut Block) -> Result<*const Block, Esc> {
    if blkv == NIL { return Ok(std::ptr::null()); }
    if is_proc(blkv) { return Ok(proc_block(c, blkv)); }
    if blkv.is_sym() && !redef(R_SYM_TO_PROC) {
        // &:sym under refinements remembers the scope it was written in
        let cref = unsafe { (*c.frame).cref() };
        if crate::vm::refinements_on() && cref_refinements(cref) != NIL {
            *holder = Block::cfunc(c, |_c, _d, _a, _b, _o| Ok(NIL), NIL);
            holder.kind = BlockKind::Sym(blkv.as_sym());
            holder.cref = cref;
            return Ok(holder as *const Block);
        }
        let p = crate::builtins::proc::sym_to_proc(c, blkv)?;
        return Ok(proc_block(c, p));
    }
    if is_method_obj(blkv) {
        *holder = Block::cfunc(c, |_c, _d, _a, _b, _o| Ok(NIL), NIL);
        holder.kind = BlockKind::Method(blkv);
        return Ok(holder as *const Block);
    }
    if respond_to(c, blkv, s::to_proc, true) {
        let p = funcall(c, blkv, s::to_proc, &[], std::ptr::null())?;
        if is_proc(p) { return Ok(proc_block(c, p)); }
        let cls = class_path(c, real_class_of(c, blkv));
        return type_err(c, format!("can't convert {} to Proc ({}#to_proc gives {})", cls, cls, class_path(c, real_class_of(c, p))));
    }
    type_err(c, format!("wrong argument type {} (expected Proc)", class_path(c, real_class_of(c, blkv))))
}

/// Stage tmp (+ kw) as arguments at base and call.
pub fn srb_call_x(c: &mut Ctx, cc: &CallCache, mid: Sym, base: Stk, tmp: Value, kw: Value, blkv: Value, ci: CiRef, flags: u32, fp: Fp) -> RV {
    let n = c.ary(tmp).len();
    let has_kw = kw != NIL && (c.hash(kw).live > 0 || ci.kinds.contains(&AK_KW));
    let total = n + if has_kw { 1 } else { 0 };
    stack_check(c, total + 4)?;
    base[1].set(tmp);
    for i in 0..n { base[2 + i].set(c.ary(tmp)[i]); }
    if has_kw { base[2 + n].set(kw); }
    c.set_sp(base, 2 + total);
    let mut holder = Block::cfunc(c, |_c, _d, _a, _b, _o| Ok(NIL), NIL);
    let mut lit;
    let blk: *const Block = if let Some(is) = ci.blk {
        lit = make_block(c, is, fp);
        &lit
    } else { block_from_value(c, blkv, &mut holder)? };
    let fl = flags & !CF_ATTR | if has_kw { CF_KW | if !ci.kinds.contains(&AK_KW) { CF_KW_SPLAT } else { 0 } } else { 0 };
    let attr_val = if flags & CF_ATTR != 0 && total > 0 { base[1 + total].get() } else { UNDEF };
    let r = call_cached(c, cc, mid, base, total as u32, blk, fl);
    std::hint::black_box((&tmp, &kw, &blkv));
    let r = match r {
        Err(e) if ci.blk.is_some() && c.esc == EscKind::Break && c.esc_tag == unsafe { (*blk).tag } => { let _ = e; c.esc = EscKind::None; Ok(c.esc_val) }
        r => r,
    };
    lit = holder; let _ = lit;
    if attr_val != UNDEF { r.map(|_| attr_val) } else { r }
}

/// A call whose only argument is a splat (no block, keywords or attribute assignment).
#[inline(always)]
pub fn srb_splat1(ci: CiRef, flags: u32) -> bool {
    ci.kinds.len() == 1 && ci.kinds[0] == AK_SPLAT && ci.blk.is_none() && flags & CF_ATTR == 0
}

/// `f(*a)` (srb_splat1): an Array's elements staged directly; anything else as srb_call_x.
#[inline(never)]
pub fn srb_call_splat1(c: &mut Ctx, cc: &CallCache, mid: Sym, base: Stk, v: Value, ci: CiRef, flags: u32, fp: Fp) -> RV {
    let n = if is_array(v) { c.ary_len(v) } else { usize::MAX };
    // a ruby2_keywords hash at the end is passed as keywords: the general path
    let kwh = n != usize::MAX && n > 0 && { let l = c.ary(v)[n - 1]; is_hash(l) && crate::heap::obj_flags(l) & FL_KWHASH != 0 };
    if n == usize::MAX || kwh {
        let tmp = c.ary_new();
        base[1].set(tmp);
        let (mut kw, mut blkv) = (NIL, NIL);
        srb_callx_arg(c, ci, 0, v, tmp, &mut kw, &mut blkv)?;
        return srb_call_x(c, cc, mid, base, tmp, kw, blkv, ci, flags, fp);
    }
    stack_check(c, n + 4)?;
    base[1].set(v);
    for i in 0..n { base[2 + i].set(c.ary(v)[i]); }
    c.set_sp(base, 2 + n);
    call_cached(c, cc, mid, base, n as u32, std::ptr::null(), flags)
}

pub fn srb_super(c: &mut Ctx, base: Stk, tmp: Value, kw: Value, blkv: Value, ci: CiRef, fp: Fp, _zsuper: bool) -> RV {
    let f = c.frame();
    if f.me.is_null() { return err(c, vm().c_runtime_error.get(), "super called outside of method"); }
    let me = unsafe { &*f.me };
    let slf = fp[0].get();
    let klass = class_of(slf);
    // a define_method body searches from its own owner, not the class its block was made under
    let owner = if f.defined_class() != NIL && !matches!(me.kind, MKind::Bmethod(_)) { f.defined_class() } else { me.owner };
    let sm = super_method(c, klass, owner, me.name);
    let fblk = f.blk;
    let n = c.ary(tmp).len();
    let has_kw = kw != NIL && (c.hash(kw).live > 0 || ci.kinds.contains(&AK_KW));
    let total = n + if has_kw { 1 } else { 0 };
    stack_check(c, total + 4)?;
    base[0].set(slf);
    base[1].set(tmp);
    for i in 0..n { base[2 + i].set(c.ary(tmp)[i]); }
    if has_kw { base[2 + n].set(kw); }
    c.set_sp(base, 2 + total);
    let mut holder = Block::cfunc(c, |_c, _d, _a, _b, _o| Ok(NIL), NIL);
    let lit;
    let blk: *const Block = if let Some(is) = ci.blk {
        lit = make_block(c, is, fp);
        &lit
    } else if ci.kinds.contains(&AK_BLOCK) {
        block_from_value(c, blkv, &mut holder)?
    } else { fblk };
    let fl = CF_FCALL | CF_SUPER | if has_kw { CF_KW | if !ci.kinds.contains(&AK_KW) { CF_KW_SPLAT } else { 0 } } else { 0 };
    let r = if sm.is_null() {
        let mm = lookup_method(c, klass, s::method_missing);
        if !mm.is_null() && unsafe { (*mm).owner } != vm().c_basic_object.get() {
            method_missing(c, slf, me.name, base, total as u32, blk, fl)
        } else {
            no_method_error(c, slf, me.name, CF_SUPER)
        }
    } else {
        invoke(c, sm, slf, base, total as u32, blk, fl)
    };
    c.set_sp(base, 0);
    std::hint::black_box((&tmp, &kw, &blkv));
    match r {
        Err(e) if ci.blk.is_some() && c.esc == EscKind::Break && c.esc_tag == unsafe { (*blk).tag } => { let _ = e; c.esc = EscKind::None; Ok(c.esc_val) }
        r => r,
    }
}

pub fn srb_no_block(c: &mut Ctx) -> RV { err(c, vm().c_local_jump_error.get(), "no block given (yield)") }

/// A yield site's feedback: which block iseq it ran.
#[inline(always)]
pub fn srb_yield_note(yc: &YieldCache, blk: *const Block) {
    if let BlockKind::Iseq(is) = unsafe { (*blk).kind } { yc.note(is as usize); } else { yc.seen.set(1); }
}

/// A baked yield site whose compiled-in block (entry hash h) did not match: arm the site's
/// guard if this block is that one, then yield out of line.
#[inline(never)]
pub fn srb_yield_miss(c: &mut Ctx, yc: &YieldCache, h: u64, blk: *const Block, base: Stk, k: u32) -> RV {
    srb_yield_note(yc, blk);
    if let BlockKind::Iseq(is) = unsafe { (*blk).kind } {
        let bentry = unsafe { (*is).bentry };
        if !bentry.is_null() && crate::node::hash_node(bentry) == h { yc.fast.set(is as usize); }
    }
    srb_yield_staged(c, blk, base, k)
}

#[inline(always)]
pub fn srb_yield_staged(c: &mut Ctx, blk: *const Block, base: Stk, k: u32) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("yield: node_yield");
    let b = unsafe { &*blk };
    if let Some(is) = block_fast(c, b, k as usize) {
        stack_check(c, is.nlocals as usize + 16)?;
        c.set_sp(base, 0);
        let r = run_block_fast(c, b, is, base, k);
        c.set_sp(base, 0);
        return r;
    }
    if let BlockKind::Iseq(is) = b.kind {
        let is = unsafe { &*is };
        base[0].set(b.self_);
        let lam = is_lambda_block(c, blk);
        let r = run_block_iseq(c, b, is, base, k, std::ptr::null(), lam, std::ptr::null(), false);
        c.set_sp(base, 0);
        return r;
    }
    let args = to_vec(&base[2..2 + k as usize]);
    let r = invoke_block_full(c, b, &args, std::ptr::null(), b.self_, false, std::ptr::null(), false);
    c.set_sp(base, 0);
    r
}

pub fn srb_yield_x(c: &mut Ctx, tmp: Value, kw: Value) -> RV {
    let blk = unsafe { (*c.frame).blk };
    if blk.is_null() { return srb_no_block(c); }
    let mut args = c.ary(tmp).to_vec();
    // an empty **h still counts as "keywords given" for an iseq block (no auto-splat for non-simple blocks)
    let has_kw = kw != NIL && (c.hash(kw).live > 0 || matches!(unsafe { (*blk).kind }, BlockKind::Iseq(_)) && !is_lambda_block(c, blk));
    if has_kw { args.push(kw); }
    let b = unsafe { &*blk };
    invoke_block_full(c, b, &args, std::ptr::null(), b.self_, false, std::ptr::null(), has_kw)
}

/// `lambda` was defined somewhere after boot: a literal `lambda { }` asks which method it calls.
pub static LAMBDA_REDEF: AtomicBool = AtomicBool::new(false);
pub fn srb_lambda(c: &mut Ctx, is: &'static Iseq, fp: Fp) -> RV {
    let b = make_block(c, is, fp);
    if LAMBDA_REDEF.load(Relaxed) {
        let slf = fp[0].get();
        let me = lookup_method(c, class_of(slf), s::lambda);
        if c.me_ref(me).is_some_and(|m| m.owner != vm().m_kernel.get()) { return funcall(c, slf, s::lambda, &[], &b); }
    }
    let p = block_to_proc(c, &b)?;
    c.proc_data_mut(p).is_lambda = true;

    Ok(p)
}

#[cold]
pub fn srb_binop(c: &mut Ctx, a: Value, b: Value, mid: Sym, line: Pos, cc: &OpCc) -> RV {
    let base = stage(c, 3)?;
    base[0].set(a); base[2].set(b);
    c.frame_mut().pos = line;
    call_cached(c, cc.get(), mid, base, 1, std::ptr::null(), 0)
}

#[cold]
pub fn srb_eq(c: &mut Ctx, a: Value, b: Value, line: Pos, cc: &OpCc) -> RV {
    if is_string(a) && is_string(b) && obj_klass(a) == vm().c_string.get() && !redef(R_STR_EQ) { return Ok(Value::bool(c.str_eq(a, b))); }

    srb_binop(c, a, b, s::eq, line, cc)
}

pub fn srb_aset(c: &mut Ctx, a: Value, b: Value, v: Value, line: Pos, cc: &OpCc) -> RV {
    let base = stage(c, 4)?;
    base[0].set(a); base[2].set(b); base[3].set(v);
    c.frame_mut().pos = line;
    call_cached(c, cc.get(), s::aset, base, 2, std::ptr::null(), 0)?;
    Ok(v)
}

// ---- definitions --------------------------------------------------------------------------

fn cref_class(c: &Ctx) -> Value {
    let cr = unsafe { (*c.frame).cref() };
    if cr.is_null() { vm().c_object.get() } else { unsafe { (*cr).klass } }
}

pub fn define_iseq_method(c: &mut Ctx, klass: Value, mid: Sym, is: &'static Iseq, visi: u8) {
    let cref = unsafe { (*c.frame).cref() };
    let me = new_me_cref(mid, klass, visi, MKind::Iseq(is), cref);
    note_redefinition(c, klass, mid);
    define_method_me(c, klass, mid, me);
}

pub fn srb_def(c: &mut Ctx, mid: Sym, is: &'static Iseq, _fp: Fp) -> RV {
    let f = c.frame();
    let mut klass = cref_class(c);
    // `def` inside a method body defines on the cref class; inside instance_eval, on the singleton
    if f.defined_class() != NIL && f.kind != FK_CLASS { klass = f.defined_class(); }
    let mut visi = if f.kind == FK_CLASS || f.kind == FK_TOP || f.kind == FK_EVAL || (f.kind == FK_BLOCK && f.defined_class() == klass) { f.visi } else { VISI_PUBLIC };
    // eval at top level defines private methods; a def in a method body is public
    if klass == vm().c_object.get() && f.kind == FK_EVAL && unsafe { (*f.cref()).prev.is_null() } { visi = VISI_PRIVATE; }
    if visi != VISI_MODFUNC && (mid == s::initialize || mid == s::initialize_copy || mid == s::respond_to_missing_p
        || mid == s::mid_initialize_dup || mid == s::mid_initialize_clone) { visi = VISI_PRIVATE; }
    if visi == VISI_MODFUNC {
        define_iseq_method(c, klass, mid, is, VISI_PRIVATE);
        let sk = singleton_class(c, klass);
        define_iseq_method(c, sk, mid, is, VISI_PUBLIC);
    } else {
        define_iseq_method(c, klass, mid, is, visi);
    }
    crate::builtins::coverage::on_def(c, is as *const Iseq as usize, klass, mid);
    crate::builtins::object::call_method_added(c, klass, mid)?;
    Ok(Value::sym(mid))
}

pub fn srb_sdef(c: &mut Ctx, recv: Value, mid: Sym, is: &'static Iseq) -> RV {
    // nil / true / false: their class, as CRuby's singleton_class
    if (!recv.is_heap() || recv.is_fixnum()) && recv != NIL && recv != TRUE && recv != FALSE {
        return type_err(c, format!("can't define singleton"));
    }
    if recv.is_heap() && obj_flags(recv) & (FL_FROZEN | FL_CHILLED_ANY) != 0 { crate::vm::modify_check_slow(c, recv)?; }
    let k = singleton_class(c, recv);
    define_iseq_method(c, k, mid, is, VISI_PUBLIC);
    crate::builtins::coverage::on_def(c, is as *const Iseq as usize, k, mid);
    if crate::vm::respond_to(c, recv, s::singleton_method_added, true) {
        funcall(c, recv, s::singleton_method_added, &[Value::sym(mid)], std::ptr::null())?;
    }
    Ok(Value::sym(mid))
}

/// Run a class/module body: self = klass, cref pushed.
pub fn run_class_body(c: &mut Ctx, klass: Value, is: &'static Iseq) -> RV {
    let base = c.top();
    stack_check(c, is.nlocals as usize + 8)?;
    let outer_cref = unsafe { (*c.frame).cref() };
    let cref = new_cref(klass, outer_cref);
    let mut fr = Frame::empty(base);
    fr.prev = c.frame; fr.iseq = is; fr.kind = FK_CLASS; fr.visi = VISI_PUBLIC;
    fr.id = c.next_serial(); fr.cref = cref; fr.pos = Pos::info(&is.info.line);
    base[0].set(klass); base[1].set(NIL);
    nil_fill(base, 2, is.nlocals as usize);
    c.set_sp(base, is.nlocals as usize);
    if is.env_size > 0 {
        let env = c.alloc(vm().c_env.get(), Body::Env(crate::envvals::EnvVals::new(is.env_size as usize), NIL));
        base[1].set(env);
    }
    c.frame = &mut fr.h;
    let r = run_iseq_body(c, is, base, EV_CLASS, EV_END);
    c.frame = fr.prev;
    c.set_sp(base, 0);
    r
}

pub fn srb_class(c: &mut Ctx, outer: Value, name: Sym, sup: Value, is: &'static Iseq, _fp: Fp) -> RV {
    let outer = if outer == UNDEF {
        let cr = unsafe { (*c.frame).cref() };
        if outer == UNDEF && false { vm().c_object.get() } else if cr.is_null() { vm().c_object.get() } else { unsafe { (*cr).klass } }
    } else { outer };
    let outer = if outer == NIL { vm().c_object.get() } else { outer };
    if !is_class_or_module(outer) {
        let s = crate::builtins::kernel::inspect_str(c, outer)?;
        return type_err(c, format!("{} is not a class/module", s));
    }
    if sup != UNDEF && !(is_class_or_module(sup) && !c.class(sup).is_module) {
        return type_err(c, format!("superclass must be an instance of Class (given an instance of {})", class_path(c, real_class_of(c, sup))));
    }
    let existing = const_get_at(c, outer, name).or_else(|| if outer == vm().c_object.get() { None } else { None });
    let existing = match existing {
        Some(v) => Some(v),
        None => {
            if c.class(outer).autoload.contains_key(&name) { do_autoload(c, outer, name)? } else { None }
        }
    };
    let klass = match existing {
        Some(k) => {
            if !is_class_or_module(k) || c.class(k).is_module {
                return type_err(c, format!("{} is not a class", sym_str(name)));
            }
            if sup != UNDEF && c.class(k).superclass != sup {
                let mut real_sup = c.class(k).superclass;
                while real_sup != NIL && c.class(real_sup).is_singleton { real_sup = c.class(real_sup).superclass; }
                if real_sup != sup { return type_err(c, format!("superclass mismatch for class {}", sym_str(name))); }
            }
            k
        }
        None => {
            let s = if sup == UNDEF { vm().c_object.get() } else { sup };
            if s == vm().c_class.get() { return type_err(c, "can't make subclass of Class"); }
            if c.class(s).is_singleton { return type_err(c, "can't make subclass of singleton class"); }
            let k = new_class(c, s);
            name_class(c, k, outer, name);
            const_set_here(c, outer, name, k)?;
            crate::builtins::object::call_inherited(c, s, k)?;
            k
        }
    };
    run_class_body(c, klass, is)
}

pub fn srb_module(c: &mut Ctx, outer: Value, name: Sym, is: &'static Iseq, _fp: Fp) -> RV {
    let outer = if outer == UNDEF {
        let cr = unsafe { (*c.frame).cref() };
        if cr.is_null() { vm().c_object.get() } else { unsafe { (*cr).klass } }
    } else { outer };
    if !is_class_or_module(outer) {
        let s = crate::builtins::kernel::inspect_str(c, outer)?;
        return type_err(c, format!("{} is not a class/module", s));
    }
    // reopening an autoload constant loads it first (as srb_class does)
    let existing = match const_get_at(c, outer, name) {
        None if c.class(outer).autoload.contains_key(&name) => do_autoload(c, outer, name)?,
        x => x,
    };
    let m = match existing {
        Some(m) => {
            if !is_class_or_module(m) || !c.class(m).is_module { return type_err(c, format!("{} is not a module", sym_str(name))); }
            m
        }
        None => {
            let m = new_module(c);
            name_class(c, m, outer, name);
            const_set_here(c, outer, name, m)?;
            m
        }
    };
    run_class_body(c, m, is)
}

pub fn srb_sclass(c: &mut Ctx, recv: Value, is: &'static Iseq, _fp: Fp) -> RV {
    if recv.is_fixnum() || recv.is_sym() || recv.is_flonum() {
        return type_err(c, "can't define singleton");
    }
    let k = singleton_class(c, recv);
    let body_id = c.serial + 1;   // the id run_class_body gives its frame
    let r = run_class_body(c, k, is);
    // `return` in `class << x` returns from the enclosing method
    if r.is_err() && c.esc == EscKind::Return && c.esc_tag == body_id {
        let v = c.esc_val;
        if c.frame().kind == FK_BLOCK { return srb_block_return(c, v); }
        c.esc_tag = c.frame().id;
    }
    r
}

pub fn srb_alias(c: &mut Ctx, newname: Value, oldname: Value) -> RV {
    let klass = {
        let f = c.frame();
        if f.defined_class() != NIL && f.kind != FK_CLASS { f.defined_class() } else { cref_class(c) }
    };
    crate::builtins::object::alias_method(c, klass, newname.as_sym(), oldname.as_sym())?;
    Ok(NIL)
}

pub fn srb_undef(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    let klass = {
        let f = c.frame();
        if f.defined_class() != NIL && f.kind != FK_CLASS { f.defined_class() } else { cref_class(c) }
    };
    let mid = if v.is_sym() { v.as_sym() } else { c.str_to_sym(v).as_sym() };
    crate::builtins::object::undef_method(c, klass, mid)
}

// ---- multiple assignment ------------------------------------------------------------------

pub fn srb_masgn_array(c: &mut Ctx, rhs: Value, _splat: bool) -> RV {
    if is_array(rhs) { return Ok(rhs); }
    if conv_target(rhs) && !is_hash(rhs) && !is_string(rhs) && respond_to(c, rhs, s::to_ary, true) {
        let r = funcall(c, rhs, s::to_ary, &[], std::ptr::null())?;
        if is_array(r) { return Ok(r); }
        if r != NIL { return type_err(c, format!("can't convert {} to Array", class_path(c, real_class_of(c, rhs)))); }
    }
    Ok(c.ary_new_from(&[rhs]))
}

pub fn srb_masgn_elem(c: &mut Ctx, a: Value, idx: u32, kind: u32, post: u32) -> Value {
    let n = c.ary_len(a);
    match kind {
        0 => c.ary_get(a, idx as usize),
        1 => {
            let s = idx as usize;
            let e = n.saturating_sub(post as usize);
            if s < e { c.ary_from_vec(c.ary(a)[s..e].to_vec()) } else { c.ary_new() }
        }
        _ => {
            // post element i: from the back, but not overlapping the front part
            let pre = n.saturating_sub(post as usize);
            let i = pre + idx as usize;
            if (n as i64) < (post as i64) {
                // fewer elements than needed: posts take what's after the pre part
                c.ary_get(a, idx as usize + n.saturating_sub(post as usize))
            } else { c.ary_get(a, i) }
        }
    }
}

pub fn srb_unsupported(c: &mut Ctx, what: Str) -> RV {
    err(c, vm().c_not_impl_error.get(), String::from_utf8_lossy(what))
}

/// Pattern matching asks `respond_to?` (so respond_to_missing? counts).
fn pm_responds(c: &mut Ctx, v: Value, name: &[u8]) -> Result<bool, Esc> {
    if respond_to(c, v, intern(name), false) { return Ok(true); }
    Ok(funcall(c, v, intern(b"respond_to?"), &[Value::sym(intern(name))], std::ptr::null())?.truthy())
}

pub fn srb_special<D: Disp>(c: &mut Ctx, _n: NodePtr, fp: Fp, kind: u32, arg: NodePtr) -> RV {
    let v = D::call(c, arg, fp)?;
    match kind {
        SP_IS_NIL => Ok(Value::bool(v == NIL)),
        SP_NOMATCH => {
            let s = crate::builtins::kernel::inspect_str(c, v)?;
            err(c, vm().c_no_matching_pattern_error.get(), s)
        }
        SP_NOMATCH_MSG => {
            let (subj, why) = (c.ary(v)[0], c.ary(v)[1]);
            let s = crate::builtins::kernel::inspect_str(c, subj)?;
            if is_array(why) {
                // [message, matchee, key]: a missing Hash key
                let msg = format!("{}: {}", s, String::from_utf8_lossy(c.str_bytes(c.ary(why)[0])));
                let e = make_exception(c, vm().c_no_matching_pattern_key_error.get(), &msg);
                ivar_set_raw(c, e, s::receiver, c.ary(why)[1]);
                ivar_set_raw(c, e, s::key, c.ary(why)[2]);
                return Err(raise_exc(c, e));
            }
            let msg = if why == NIL { s } else { format!("{}: {}", s, String::from_utf8_lossy(c.str_bytes(why))) };
            err(c, vm().c_no_matching_pattern_error.get(), msg)
        }
        SP_PM_MSG => {
            // elements stay in `v` (GC-visible) while inspect allocates
            let av = c.ary(v).to_vec();
            let a = |i: usize| av[i];
            let insp = crate::builtins::kernel::inspect_str;
            let msg = match a(0).as_fix() {
                PM_EQQ => format!("{} === {} does not return true", insp(c, a(1))?, insp(c, a(2))?),
                PM_GUARD => "guard clause does not return true".to_string(),
                PM_ARY if a(2) == NIL => format!("{} does not respond to #deconstruct", insp(c, a(1))?),
                PM_ARY => format!("{} length mismatch (given {}, expected {}{})", insp(c, a(2))?, c.ary(a(2)).len(), a(3).as_fix(), if a(4) == TRUE { "" } else { "+" }),
                PM_DECON => format!("{} does not respond to #deconstruct", insp(c, a(1))?),
                PM_FIND => format!("{} does not match to find pattern", insp(c, a(1))?),
                PM_DECONK => format!("{} does not respond to #deconstruct_keys", insp(c, a(1))?),
                PM_KEY => {
                    let t = format!("key not found: {}", insp(c, a(2))?);
                    let m = c.str_new(t.as_bytes());
                    return Ok(c.ary_new_from(&[m, a(1), a(2)]));
                }
                PM_REST => format!("rest of {} is not empty", insp(c, a(1))?),
                _ => format!("{} is not empty", insp(c, a(1))?),
            };
            Ok(c.str_new(msg.as_bytes()))
        }
        SP_NAMED_CAPTURE => {
            let m = crate::builtins::svar::last_match(c);
            if m == NIL { return Ok(NIL); }
            Ok(crate::regexp::named_match(c, m, v.as_sym()))
        }
        SP_DECONSTRUCT => {
            if is_array(v) { return Ok(v); }
            if pm_responds(c, v, b"deconstruct")? {
                let r = funcall(c, v, intern(b"deconstruct"), &[], std::ptr::null())?;
                if !is_array(r) { return type_err(c, "deconstruct must return Array"); }
                return Ok(r);
            }
            Ok(NIL)
        }
        SP_LEN_EQ | SP_LEN_GE => {
            let a = c.ary(v)[0]; let k = c.ary(v)[1].as_fix() as usize;
            if !is_array(a) { return Ok(FALSE); }
            let n = c.ary(a).len();
            Ok(Value::bool(if kind == SP_LEN_EQ { n == k } else { n >= k }))
        }
        SP_DECONSTRUCT_KEYS => {
            let subj = c.ary(v)[0]; let keys = c.ary(v)[1];
            if is_hash(subj) { return Ok(subj); }
            if pm_responds(c, subj, b"deconstruct_keys")? {
                let r = funcall(c, subj, intern(b"deconstruct_keys"), &[keys], std::ptr::null())?;
                if !is_hash(r) { return type_err(c, "deconstruct_keys must return Hash"); }
                return Ok(r);
            }
            Ok(NIL)
        }
        SP_HASH_HAS => {
            let h = c.ary(v)[0]; let k = c.ary(v)[1];
            if !is_hash(h) { return Ok(FALSE); }
            Ok(Value::bool(hash_get_opt(c, h, k)?.is_some()))
        }
        SP_HASH_EXCEPT => {
            let h = c.ary(v)[0]; let used = c.ary(v)[1];
            let out = c.hash_new();
            if is_hash(h) {
                let mut i = 0;
                while let Some((k, x)) = c.hash_entry_at(h, i) {
                    i += 1;
                    if k == UNDEF || c.ary(used).contains(&k) { continue; }
                    hash_aset(c, out, k, x)?;
                }
            }
            Ok(out)
        }
        SP_EMPTY_P => Ok(Value::bool(is_hash(v) && c.hash(v).live == 0)),

        SP_FOR_UNPACK => {
            if is_array(v) && c.ary(v).len() == 1 { Ok(c.ary(v)[0]) } else { Ok(v) }
        }
        SP_FIND_ELEM => {
            let a = c.ary(v)[0]; let i = c.ary(v)[1].as_fix() as usize; let j = c.ary(v)[2].as_fix() as usize;
            Ok(c.ary(a).get(i + j).copied().unwrap_or(NIL))
        }
        SP_LEN => Ok(Value::fix(if is_array(v) { c.ary(v).len() as i64 } else { -1 })),
        SP_FIND_PRE => { let a = c.ary(v)[0]; let i = c.ary(v)[1].as_fix() as usize; Ok(c.ary_from_vec(c.ary(a)[..i].to_vec())) }
        SP_FIND_POST => {
            let a = c.ary(v)[0]; let i = c.ary(v)[1].as_fix() as usize; let k = c.ary(v)[2].as_fix() as usize;
            let s = (i + k).min(c.ary(a).len());
            Ok(c.ary_from_vec(c.ary(a)[s..].to_vec()))
        }
        _ => Ok(NIL),
    }
}

/// Call with literal keywords staged as values: an Iseq callee taking only required +
/// keyword params gets them directly in its slots; otherwise a keyword Hash is built.
pub fn srb_call_kw(c: &mut Ctx, cc: &CallCache, mid: Sym, base: Stk, k: usize, ci: CiRef, flags: u32, fp: Fp) -> RV {
    let nkw = ci.kw_names.len();
    let npos = k - nkw;
    let recv = base[0].get();
    let lit;
    let blk: *const Block = match ci.blk { Some(is) => { lit = make_block(c, is, fp); &lit } None => std::ptr::null() };
    let me = lookup_cc(c, cc, recv, mid, flags);
    // every keyword in declared order to a dense method: its kentry with the staged values as params
    if cc.fast_argc.get() != KW_DENSE | k as u32 && !me.is_null() { kw_dense_fill(cc, me, npos, &ci.kw_names, flags); }
    let r = if cc.fast_argc.get() == KW_DENSE | k as u32 {
        c.call_me = me; c.call_blk = blk;
        native_check(c)?;
        let r = run_body(c, cc.fast.get() as NodePtr, base);
        c.set_sp(base, 0);
        r
    } else if !me.is_null() && kw_direct_ok(me, npos, &ci.kw_names, flags) {
        let is = match unsafe { (*me).kind } { MKind::Iseq(is) => unsafe { &*is }, _ => unreachable!() };
        let r = invoke_iseq_kw(c, me, is, recv, base, npos, &ci.kw_names, blk);
        c.set_sp(base, 0);
        r
    } else {
        let h = c.hash_new_capa(nkw);
        base[1].set(h);
        for (j, &nm) in ci.kw_names.iter().enumerate() {
            let v = base[2 + npos + j].get();
            hash_set(c, h, Value::sym(nm), v)?;
        }
        base[2 + npos].set(h); c.set_sp(base, 3 + npos);
        call_cached(c, cc, mid, base, npos as u32 + 1, blk, flags | CF_KW)
    };
    match r {
        Err(_) if ci.blk.is_some() && c.esc == EscKind::Break && c.esc_tag == unsafe { (*blk).tag } => { c.esc = EscKind::None; Ok(c.esc_val) }
        r => r,
    }
}

/// A keyword site whose compiled-in callee (entry hash h) was not hit: srb_call_kw, then spec_klass.
#[inline(never)]
pub fn srb_call_kw_spec(c: &mut Ctx, cc: &CallCache, mid: Sym, base: Stk, k: usize, ci: CiRef, flags: u32, fp: Fp, h: u64) -> RV {
    let r = srb_call_kw(c, cc, mid, base, k, ci, flags, fp);
    cc.spec_klass.set(if cc.fast_hash.get() == h && cc.fast_argc.get() == KW_DENSE | k as u32 { cc.klass.get() } else { 0 });
    r
}
