// Out-of-line runtime support for the nodes in node.def (slow paths).
use crate::compile::*;
use crate::hash::*;
use crate::node::*;
use crate::object::*;
use crate::string::*;
use crate::symbol::{intern, s, sym_str};
use crate::value::*;
use crate::vm::*;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};

// ---- redefinition flags for operator fast paths -----------------------------------------

pub const R_INT_PLUS: u32 = 1 << 0;
pub const R_INT_MINUS: u32 = 1 << 1;
pub const R_INT_MUL: u32 = 1 << 2;
pub const R_INT_DIV: u32 = 1 << 3;
pub const R_INT_MOD: u32 = 1 << 4;
pub const R_INT_LT: u32 = 1 << 5;
pub const R_INT_LE: u32 = 1 << 6;
pub const R_INT_GT: u32 = 1 << 7;
pub const R_INT_GE: u32 = 1 << 8;
pub const R_INT_EQ: u32 = 1 << 9;
pub const R_INT_AND: u32 = 1 << 10;
pub const R_INT_OR: u32 = 1 << 11;
pub const R_FLO_PLUS: u32 = 1 << 12;
pub const R_FLO_MINUS: u32 = 1 << 13;
pub const R_FLO_MUL: u32 = 1 << 14;
pub const R_FLO_DIV: u32 = 1 << 15;
pub const R_FLO_LT: u32 = 1 << 16;
pub const R_FLO_LE: u32 = 1 << 17;
pub const R_FLO_GT: u32 = 1 << 18;
pub const R_FLO_GE: u32 = 1 << 19;
pub const R_FLO_EQ: u32 = 1 << 20;
pub const R_ARY_LTLT: u32 = 1 << 21;
pub const R_ARY_AREF: u32 = 1 << 22;
pub const R_ARY_ASET: u32 = 1 << 23;
pub const R_HASH_AREF: u32 = 1 << 24;
pub const R_HASH_ASET: u32 = 1 << 25;
pub const R_INT_LSHIFT: u32 = 1 << 26;
pub const R_INT_RSHIFT: u32 = 1 << 27;
pub const R_INT_XOR: u32 = 1 << 28;
pub const R_STR_LTLT: u32 = 1 << 29;
pub const R_FLO_POW: u32 = 1 << 30;
pub const R_IMM_EQ: u32 = 1 << 31;

pub static REDEF: AtomicU32 = AtomicU32::new(0);
pub static mut BOOTED: bool = false;

// Integer#+ - * / % < <= > >= == & | << >> ^: the fixnum paths of operator nodes test all of them at once
// (c.int_redef()), so that one test covers every operator of a straight run of baked code
pub const R_INT_ALL: u32 = R_INT_PLUS | R_INT_MINUS | R_INT_MUL | R_INT_DIV | R_INT_MOD | R_INT_LT | R_INT_LE | R_INT_GT
    | R_INT_GE | R_INT_EQ | R_INT_AND | R_INT_OR | R_INT_LSHIFT | R_INT_RSHIFT | R_INT_XOR;

#[inline(always)]
pub fn redef(f: u32) -> bool { REDEF.load(Relaxed) & f != 0 }
pub fn redef_word() -> u32 { REDEF.load(Relaxed) }

/// Called when a method is (re)defined after boot: disable affected fast paths.
pub fn note_redefinition(c: &mut Ctx, klass: Value, mid: Sym) {
    redef_note(klass, mid);
    c.redef = redef_word();
}

/// REDEF for (klass, mid); true if it changed.  The running context must then copy it (note_redefinition).
pub fn redef_note(klass: Value, mid: Sym) -> bool {
    if unsafe { !BOOTED } { return false; }
    let before = redef_word();
    // a refinement of a basic operation turns its inline fast path off everywhere
    let klass = if rclass(klass).refined { rclass(klass).attached } else { klass };
    let vm = vm();
    let table: &[(Value, Sym, u32)] = &[
        (vm.c_integer, s::plus, R_INT_PLUS), (vm.c_integer, s::minus, R_INT_MINUS), (vm.c_integer, s::mul, R_INT_MUL),
        (vm.c_integer, s::div, R_INT_DIV), (vm.c_integer, s::modulo, R_INT_MOD), (vm.c_integer, s::lt, R_INT_LT),
        (vm.c_integer, s::le, R_INT_LE), (vm.c_integer, s::gt, R_INT_GT), (vm.c_integer, s::ge, R_INT_GE),
        (vm.c_integer, s::eq, R_INT_EQ), (vm.c_integer, s::and, R_INT_AND), (vm.c_integer, s::or, R_INT_OR),
        (vm.c_float, s::plus, R_FLO_PLUS), (vm.c_float, s::minus, R_FLO_MINUS), (vm.c_float, s::mul, R_FLO_MUL),
        (vm.c_float, s::div, R_FLO_DIV), (vm.c_float, s::lt, R_FLO_LT), (vm.c_float, s::le, R_FLO_LE),
        (vm.c_float, s::gt, R_FLO_GT), (vm.c_float, s::ge, R_FLO_GE), (vm.c_float, s::eq, R_FLO_EQ),
        (vm.c_array, s::ltlt, R_ARY_LTLT), (vm.c_array, s::aref, R_ARY_AREF), (vm.c_array, s::aset, R_ARY_ASET),
        (vm.c_integer, s::ltlt, R_INT_LSHIFT), (vm.c_integer, s::gtgt, R_INT_RSHIFT), (vm.c_integer, s::xor, R_INT_XOR),
        (vm.c_hash, s::aref, R_HASH_AREF), (vm.c_hash, s::aset, R_HASH_ASET), (vm.c_string, s::ltlt, R_STR_LTLT), (vm.c_float, s::pow, R_FLO_POW),
        (vm.c_nil, s::eq, R_IMM_EQ), (vm.c_true, s::eq, R_IMM_EQ), (vm.c_false, s::eq, R_IMM_EQ), (vm.c_symbol, s::eq, R_IMM_EQ),
    ];
    for &(k, m, f) in table {
        if m == mid && (klass == k || ancestors(k).contains(&klass)) { REDEF.fetch_or(f, Relaxed); }
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
pub fn srb_aref_out(c: &mut Ctx, a: Value, b: Value, line: Pos, cc: &mut OpCc) -> RV {
    if unsafe { BOOTED } && !redef(R_ARY_AREF) { FAST_ARY_AREF.store(vm().c_array.raw(), Relaxed); }
    if b.is_fixnum() && is_array(a) && obj(a).klass == vm().c_array && !redef(R_ARY_AREF) {
        let v = c.ary(a);
        let i = b.as_fix();
        if let Some(&x) = v.get(i as usize) { return Ok(x); }
        return Ok(if i < 0 { v.len().checked_sub(i.unsigned_abs() as usize).and_then(|j| v.get(j).copied()).unwrap_or(NIL) } else { NIL });
    }
    if is_hash(a) && obj(a).klass == vm().c_hash && !redef(R_HASH_AREF) { return crate::hash::hash_aref(c, a, b); }
    // Integer#[] (optcarrot's bit tests): its body when the site's cache holds it, without the call
    use crate::inline_builtin::*;
    if cc.peek().is_some_and(|cc| bi_hit(cc, a, BI_INT_AREF)) { if let Some(v) = bi_body(c, BI_INT_AREF, a, b) { return Ok(v); } }
    srb_binop(c, a, b, s::aref, line, cc)
}

/// `a[b] = v`, as srb_aref_out.
#[inline(never)]
#[cold]
pub fn srb_aset_out(c: &mut Ctx, a: Value, b: Value, v: Value, line: Pos, cc: &mut OpCc) -> RV {
    if unsafe { BOOTED } && !redef(R_ARY_ASET) { FAST_ARY_ASET.store(vm().c_array.raw(), Relaxed); }
    if b.is_fixnum() && is_array(a) && obj(a).klass == vm().c_array && obj(a).flags & FL_FROZEN == 0 && !redef(R_ARY_ASET) {
        let ary = c.ary_mut(a);
        let i = b.as_fix();
        if i >= 0 && (i as usize) < ary.len() { ary[i as usize] = v; return Ok(v); }
    }
    if is_hash(a) && obj(a).klass == vm().c_hash && !redef(R_HASH_ASET) { crate::hash::hash_aset(c, a, b, v)?; return Ok(v); }
    srb_aset(c, a, b, v, line, cc)
}

// ---- literals ----------------------------------------------------------------------------

pub fn srb_fstr(s: Str, enc: u32, lc: &mut LitCache) -> Value {
    // frozen literals share the -"str" pool, like CRuby's fstring table
    let v = match vm().frozen_lits.get(s) {
        Some(&v) if rstr(v).enc == enc as u8 => v,
        other => {
            let pooled = other.is_some();
            let v = crate::string::str_new_enc(s, enc as u8);
            obj(v).flags |= FL_FROZEN;
            if pooled { vm().node_values.push(v); } else { vm().frozen_lits.insert(s.to_vec(), v); }
            v
        }
    };
    lc.v = v.raw();
    v
}

/// Append an interpolated value to the string being built (acc is fresh: mutable, unshared).
pub fn srb_dstr_append(c: &mut Ctx, acc: Value, v: Value) -> Result<(), Esc> {
    if is_string(v) {
        let (a, b) = (rstr(acc), rstr(v));
        if a.enc == b.enc { crate::gc::malloc_increase(b.b.len()); a.b.extend_from_slice(&b.b); return Ok(()); }
        return str_append_str(c, acc, v);
    }
    if v.is_fixnum() {
        // an empty acc would take the Integer string's US-ASCII encoding: left to the slow path
        let a = rstr(acc);
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
    if unsafe { REFINEMENTS_ON } {
        if let Some(r) = funcall_refined(c, std::ptr::null(), v, s::to_s, &[], std::ptr::null(), CF_FCALL) { let r = r?; if is_string(r) { return Ok(r); } }
    }
    if v.is_fixnum() { return Ok(str_new(v.as_fix().to_string().as_bytes())); }
    if v.is_sym() { return Ok(str_new(crate::symbol::sym_bytes(v.as_sym()))); }
    if v == NIL { return Ok(str_new(b"")); }
    let r = funcall(c, v, s::to_s, &[], std::ptr::null())?;
    if is_string(r) { Ok(r) } else { Ok(str_new(crate::builtins::kernel::any_to_s_str(v).as_bytes())) }
}

pub fn srb_regex_lit(c: &mut Ctx, src: Str, opts: u32, lc: &mut LitCache) -> RV {
    let r = crate::regexp::regexp_new(c, src, opts)?;
    obj(r).flags |= FL_FROZEN;
    vm().node_values.push(r);
    lc.v = r.raw();
    Ok(r)
}

pub fn srb_dregex(c: &mut Ctx, s: Value, opts: u32, once: bool, lc: &mut LitCache) -> RV {
    let b = str_bytes(s).to_vec();
    let r = crate::regexp::regexp_new(c, &b, opts)?;
    if once { vm().node_values.push(r); lc.v = r.raw(); }
    Ok(r)
}

pub fn srb_biglit(digits: Str, lc: &mut LitCache) -> Value {
    let b = crate::bignum::BigInt::parse(std::str::from_utf8(digits).unwrap(), 10).unwrap();
    let v = crate::bignum::big_to_value(b);
    vm().node_values.push(v);
    lc.v = v.raw();
    v
}

pub fn srb_splat_into(c: &mut Ctx, a: Value, v: Value) -> Result<(), Esc> {
    let arr = splat_to_array(c, v)?;
    let n = rary(arr).len();
    for i in 0..n { let x = rary(arr)[i]; ary_push(a, x); }
    Ok(())
}

/// *v: Array as is, nil → [], to_a if defined, else [v].
pub fn splat_to_array(c: &mut Ctx, v: Value) -> RV {
    if is_array(v) { return Ok(v); }
    if v == NIL { return Ok(ary_new()); }
    if is_hash(v) { return crate::hash::hash_to_a(c, v); }
    if respond_to(v, s::to_a, true) {
        let r = funcall(c, v, s::to_a, &[], std::ptr::null())?;
        if is_array(r) { return Ok(r); }
        if r == NIL { return Ok(ary_new_from(&[v])); }
        return type_err(c, format!("can't convert {} to Array ({}#to_a gives {})", class_path(real_class_of(v)), class_path(real_class_of(v)), class_path(real_class_of(r))));
    }
    Ok(ary_new_from(&[v]))
}

pub fn srb_hash_merge_kw(c: &mut Ctx, h: Value, v: Value) -> Result<(), Esc> {
    if v == NIL { return Ok(()); }
    let src = if is_hash(v) { v } else {
        if !respond_to(v, s::to_hash, true) {
            return type_err(c, format!("no implicit conversion of {} into Hash", crate::builtins::kernel::class_name_for_err(v)));
        }
        let r = funcall(c, v, s::to_hash, &[], std::ptr::null())?;
        if !is_hash(r) {
            let cls = class_path(real_class_of(v));
            return type_err(c, format!("can't convert {} to Hash ({}#to_hash gives {})", cls, cls, class_path(real_class_of(r))));
        }
        r
    };
    for (k, x) in hash_pairs(src) { hash_aset(c, h, k, x)?; }
    Ok(())
}

pub fn srb_range_new(c: &mut Ctx, b: Value, e: Value, excl: bool) -> RV {
    crate::range::range_new_checked(c, b, e, excl)
}

pub fn srb_range_lit(b: Value, e: Value, excl: bool, lc: &mut LitCache) -> Value {
    let r = crate::range::range_new(b, e, excl);
    obj(r).flags |= FL_FROZEN;
    vm().node_values.push(r);
    lc.v = r.raw();
    r
}

// ---- variables ----------------------------------------------------------------------------

#[inline(always)]
pub fn env_at(mut env: Value, hops: u32) -> Value {
    for _ in 0..hops {
        env = match &obj(env).body { Body::Env(_, p) => *p, _ => unsafe { std::hint::unreachable_unchecked() } };
    }
    env
}
#[inline(always)]
pub fn env_get(env: Value, idx: u32) -> Value {
    match &obj(env).body { Body::Env(v, _) => v.get(idx as usize), _ => unsafe { std::hint::unreachable_unchecked() } }
}
#[inline(always)]
pub fn env_set(env: Value, idx: u32, x: Value) {
    match &mut obj(env).body { Body::Env(v, _) => v.set(idx as usize, x), _ => unsafe { std::hint::unreachable_unchecked() } }
}

pub fn srb_ivar_get(slf: Value, name: Sym, ic: &mut IvarCache) -> Value {
    if slf.is_heap() {
        if let Body::Object(_) = &obj(slf).body {
            let k = real_class_of(slf);
            if let Some(&i) = rclass(k).ivar_index.get(&name) {
                if obj(slf).klass == k { ic.klass = k.raw(); ic.idx = i; }
            }
        }
    }
    ivar_get(slf, name)
}

/// A site's way out of its inline path: the cache without the baked slot first (a stale slot stays cheap), then the miss.
#[cold]
#[inline(never)]
pub fn srb_ivar_get_out(slf: Value, name: Sym, ic: &mut IvarCache) -> Value {
    if slf.is_heap() {
        let o = obj(slf);
        if o.klass.raw() == ic.klass {
            if let Body::Object(ivs) = &o.body {
                return match ivs.get(ic.idx as usize) { Some(v) if v != UNDEF => v, _ => NIL };
            }
        }
    }
    srb_ivar_get(slf, name, ic)
}

#[cold]
#[inline(never)]
pub fn srb_ivar_set_out(c: &mut Ctx, slf: Value, name: Sym, v: Value, ic: &mut IvarCache) -> RV {
    if slf.is_heap() {
        let o = obj(slf);
        if o.klass.raw() == ic.klass && o.flags & FL_FROZEN == 0 {
            if let Body::Object(ivs) = &mut o.body {
                if ivs.set(ic.idx as usize, v) { return Ok(v); }
            }
        }
    }
    srb_ivar_set(c, slf, name, v, ic)
}

/// `@x += v` / `@x -= v` outside node_ivar_arith's inline path: read, operate, write.
#[inline(never)]
#[cold]
pub fn srb_ivar_arith_out(c: &mut Ctx, slf: Value, name: Sym, sub: bool, v: Value, line: Pos, ic: &mut IvarCache, cc: &mut OpCc) -> RV {
    let cur = srb_ivar_get_out(slf, name, ic);
    let r = crate::opt::srb_arith_out(c, cur, v, if sub { s::minus } else { s::plus }, line, cc)?;
    srb_ivar_set_out(c, slf, name, r, ic)
}

pub fn srb_ivar_set(c: &mut Ctx, slf: Value, name: Sym, v: Value, ic: &mut IvarCache) -> RV {
    ivar_set(c, slf, name, v)?;
    if slf.is_heap() {
        if let Body::Object(_) = &obj(slf).body {
            let k = real_class_of(slf);
            if obj(slf).klass == k {
                ic.klass = k.raw(); ic.idx = ivar_index_for(k, name);
            }
        }
    }
    Ok(v)
}

pub fn srb_gvar_get(c: &mut Ctx, name: Sym) -> RV { crate::builtins::kernel::gvar_get(c, name) }
pub fn srb_gvar_set(c: &mut Ctx, name: Sym, v: Value) -> RV { crate::builtins::kernel::gvar_set(c, name, v) }

/// The class whose class variables the current code sees (lexical cref, skipping singletons).
fn cvar_base(c: &Ctx) -> Value {
    let mut cr = unsafe { (*c.frame).cref() };
    unsafe {
        while !cr.is_null() {
            let k = (*cr).klass;
            if !rclass(k).is_singleton || (*cr).prev.is_null() {
                if rclass(k).is_singleton { let a = rclass(k).attached; if is_class_or_module(a) { return a; } }
                return k;
            }
            cr = (*cr).prev;
        }
    }
    vm().c_object
}

pub fn cvar_lookup(k: Value, name: Sym) -> Option<(Value, Value)> {
    for &a in ancestors(k).iter() {
        if let Some(&v) = rclass(a).cvars.get(&name) { return Some((a, v)); }
    }
    None
}

/// @@x where the lexical scope is the top level (whatever self is) is an error.
fn cvar_toplevel_check(c: &mut Ctx) -> Result<(), Esc> {
    let cr = unsafe { (*c.frame).cref() };
    if cr.is_null() || unsafe { (*cr).prev.is_null() } { let r: RV = err(c, vm().c_runtime_error, "class variable access from toplevel"); return r.map(drop); }
    Ok(())
}

pub fn srb_cvar_get(c: &mut Ctx, name: Sym) -> RV {
    cvar_toplevel_check(c)?;
    let k = cvar_base(c);
    match cvar_lookup(k, name) {
        Some((_, v)) => Ok(v),
        None => {
            let msg = format!("uninitialized class variable {} in {}", sym_str(name), class_path(k));
            let e = make_name_error(c, vm().c_name_error, &msg, Value::sym(name), k)?;
            Err(raise_exc(c, e))
        }
    }
}

pub fn srb_cvar_set(c: &mut Ctx, name: Sym, v: Value) -> RV {
    cvar_toplevel_check(c)?;
    let k = cvar_base(c);
    match cvar_lookup(k, name) {
        Some((owner, _)) => { rclass(owner).cvars.insert(name, v); }
        None => { rclass(k).cvars.insert(name, v); }
    }
    Ok(v)
}

#[cold]
pub fn srb_const_get(c: &mut Ctx, cref: *const Cref, name: Sym, cc: &mut ConstCache) -> RV {
    let v = const_lookup(c, cref, name)?;
    cc.serial = vm().const_serial;
    cc.cref = cref as usize;
    cc.val = v.raw();
    Ok(v)
}

pub fn srb_const_path(c: &mut Ctx, parent: Value, name: Sym) -> RV {
    let parent = if parent == UNDEF { vm().c_object } else { parent };
    if !is_class_or_module(parent) {
        let s = crate::builtins::kernel::inspect_str(c, parent)?;
        return type_err(c, format!("{} is not a class/module", s));
    }
    if let Some(v) = const_get_inherit(c, parent, name)? {
        if parent != vm().c_object || true { return Ok(v); }
    }
    const_missing(c, parent, name)
}

/// `parent::Name = v` (parent UNDEF: `::Name = v`)
pub fn srb_cdecl_path(c: &mut Ctx, parent: Value, name: Sym, v: Value) -> RV {
    let k = if parent == UNDEF { vm().c_object } else { parent };
    if !is_class_or_module(k) {
        let s = crate::builtins::kernel::inspect_str(c, k)?;
        return type_err(c, format!("{} is not a class/module", s));
    }
    srb_cdecl(c, k, name, v)
}

pub fn srb_cdecl(c: &mut Ctx, parent: Value, name: Sym, v: Value) -> RV {
    let k = if parent == NIL {
        let cr = unsafe { (*c.frame).cref() };
        if cr.is_null() { vm().c_object } else { unsafe { (*cr).klass } }
    } else if parent == UNDEF { vm().c_object } else { parent };
    if !is_class_or_module(k) {
        let s = crate::builtins::kernel::inspect_str(c, k)?;
        return type_err(c, format!("{} is not a class/module", s));
    }
    const_set(k, name, v);
    Ok(v)
}

pub fn srb_back_ref(c: &mut Ctx, kind: u32) -> RV {
    let m = c.last_match;
    if m == NIL { return Ok(NIL); }
    Ok(match kind {
        0 => crate::regexp::nth_match(c, m, 0),
        1 => crate::regexp::match_pre(c, m),
        2 => crate::regexp::match_post(c, m),
        _ => crate::regexp::match_last_group(c, m),
    })
}

// ---- control ------------------------------------------------------------------------------

pub fn srb_has_custom_not(v: Value) -> bool {
    let me = lookup_method(class_of(v), s::not);
    !me.is_null() && unsafe { (*me).owner } != vm().c_basic_object
}

#[cold]
pub fn srb_call0(c: &mut Ctx, v: Value, mid: Sym, cc: &mut OpCc) -> RV {
    let base = stage(c, 2)?;
    base[0].set(v);
    call_cached(c, cc.get(), mid, base, 0, std::ptr::null(), 0)
}

/// `redo` in a block entered through its entry node: run the body again until it ends otherwise.
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
        return err(c, vm().c_local_jump_error, "break from proc-closure");
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
            if bb.tag == tag && (bb.proc_val == UNDEF || !is_proc(bb.proc_val) || b != proc_block(bb.proc_val)) { return true; }
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
    if !alive { return Err(raise_local_jump(c, "unexpected return", "return", v)); }
    c.esc = EscKind::Return;
    c.esc_val = v;
    c.esc_tag = id;
    Err(Esc)
}

#[cold]
pub fn raise_local_jump(c: &mut Ctx, msg: &str, reason: &str, v: Value) -> Esc {
    let e = make_exception(vm().c_local_jump_error, msg);
    ivar_set_raw(e, crate::symbol::intern(b"__reason"), sym_value(reason));
    ivar_set_raw(e, crate::symbol::intern(b"__exit_value"), v);
    raise_exc(c, e)
}

pub fn srb_rescuable(exc: Value) -> bool { is_exception(exc) }

pub fn srb_is_standard_error(exc: Value) -> bool { is_kind_of(exc, vm().c_standard_error) }

pub fn srb_rescue_match(c: &mut Ctx, k: Value, exc: Value, splat: bool) -> Result<bool, Esc> {
    if splat {
        let a = splat_to_array(c, k)?;
        let n = rary(a).len();
        for i in 0..n { let x = rary(a)[i]; if srb_rescue_match(c, x, exc, false)? { return Ok(true); } }
        return Ok(false);
    }
    if is_class_or_module(k) {
        let me = lookup_method(class_of(k), s::eqq);
        if !me.is_null() && matches!(unsafe { (*me).kind }, MKind::Cfunc(..)) { return Ok(is_kind_of(exc, k)); }
        let r = funcall(c, k, s::eqq, &[exc], std::ptr::null())?;
        return Ok(r.truthy());
    }
    if !respond_to(k, s::eqq, true) || !k.is_heap() {
        return type_err(c, "class or module required for rescue clause");
    }
    let r = funcall(c, k, s::eqq, &[exc], std::ptr::null())?;
    Ok(r.truthy())
}

pub fn srb_when_splat(c: &mut Ctx, p: Value, subj: Value) -> Result<bool, Esc> {
    let a = splat_to_array(c, p)?;
    let n = rary(a).len();
    let mut cc = CallCache::default();
    for i in 0..n { let x = rary(a)[i]; if srb_eqq(c, x, subj, &mut cc)? { return Ok(true); } }
    Ok(false)
}

#[cold]
pub fn srb_eqq(c: &mut Ctx, p: Value, subj: Value, cc: &mut CallCache) -> Result<bool, Esc> {
    if p.is_fixnum() && subj.is_fixnum() { return Ok(p == subj); }
    if p.is_sym() { return Ok(p == subj); }
    if p == NIL || p == TRUE || p == FALSE { return Ok(p == subj); }
    if is_string(p) && is_string(subj) && obj(p).klass == vm().c_string { return Ok(str_bytes(p) == str_bytes(subj)); }
    let base = stage(c, 3)?;
    base[0].set(p); base[2].set(subj);
    let r = call_cached(c, cc, s::eqq, base, 1, std::ptr::null(), 0)?;
    Ok(r.truthy())
}

/// srb_eqq for a `when` site; an answer of the builtin Module#=== is kept in the site's memo.
#[inline(never)]
#[cold]
pub fn srb_eqq_class(c: &mut Ctx, p: Value, subj: Value, cc: &mut CallCache, km: &mut KindMemo) -> Result<bool, Esc> {
    let r = srb_eqq(c, p, subj, cc)?;
    // cc.me is what the call just ran when the cache is p's (a polymorphic site keeps others elsewhere)
    if p.is_heap() && cc.klass == obj(p).klass.raw() && cc.me == vm().ids.m_eqq as usize {
        let (k, ser) = (class_of(subj).raw(), kind_serial());
        if km.pat != p.raw() || km.ser != ser { *km = KindMemo { pat: p.raw(), ser, memo: [0; KIND_WAYS] }; }
        km.memo[kind_way(k)] = k | r as u64;
    }
    Ok(r)
}

/// defined? answers are pooled frozen US-ASCII strings.
fn defined_str(s: &[u8]) -> Value {
    let mut key = vec![0xff, b'd'];
    key.extend_from_slice(s);
    if let Some(&v) = vm().frozen_lits.get(&key) { return v; }
    let v = crate::string::str_new_enc(s, ENC_ASCII);
    obj(v).flags |= FL_FROZEN;
    vm().frozen_lits.insert(key, v);
    v
}

pub fn srb_defined<D: Disp>(c: &mut Ctx, _n: NodePtr, fp: Fp, kind: u32, name: Sym, recv: NodePtr) -> RV {
    let yes = |s: &[u8]| -> RV { Ok(defined_str(s)) };
    match kind {
        DEF_LOCALVAR => yes(b"local-variable"),
        DEF_SELF => yes(b"self"),
        DEF_NIL => yes(b"nil"),
        DEF_TRUE => yes(b"true"),
        DEF_FALSE => yes(b"false"),
        DEF_ASGN => yes(b"assignment"),
        DEF_BACKREF => yes(b"global-variable"),
        DEF_IVAR => if ivar_defined(fp[0].get(), name) { yes(b"instance-variable") } else { Ok(NIL) },
        DEF_GVAR => if crate::builtins::kernel::gvar_defined(c, name) { yes(b"global-variable") } else { Ok(NIL) },
        DEF_CVAR => {
            let k = cvar_base(c);
            if cvar_lookup(k, name).is_some() { yes(b"class variable") } else { Ok(NIL) }
        }
        DEF_CONST => {
            let cref = unsafe { (*c.frame).cref() };
            let saved = (c.esc, c.esc_val);
            match const_defined_lex(c, cref, name) { true => yes(b"constant"), false => { c.esc = saved.0; c.esc_val = saved.1; Ok(NIL) } }
        }
        DEF_CPATH => {
            let p = match D::call(c, recv, fp) { Ok(v) => v, Err(_) => { c.esc = EscKind::None; return Ok(NIL); } };
            let p = if p == UNDEF { vm().c_object } else { p };
            if !is_class_or_module(p) { return Ok(NIL); }
            if ancestors(p).iter().any(|&k| rclass(k).consts.contains_key(&name) || rclass(k).autoload.contains_key(&name))
                || (rclass(p).is_module && rclass(vm().c_object).consts.contains_key(&name)) { yes(b"constant") } else { Ok(NIL) }
        }
        DEF_FCALL => {
            let slf = fp[0].get();
            if respond_to(slf, name, true) { return yes(b"method"); }
            let rm = lookup_method(class_of(slf), s::respond_to_missing_p);
            if !rm.is_null() && unsafe { (*rm).owner } != vm().c_object && unsafe { (*rm).owner } != vm().m_kernel {
                let t = funcall(c, slf, s::respond_to_missing_p, &[Value::sym(name), TRUE], std::ptr::null())?;
                if t.truthy() { return yes(b"method"); }
            }
            Ok(NIL)
        }
        DEF_METHOD => {
            let r = match D::call(c, recv, fp) { Ok(v) => v, Err(_) => { c.esc = EscKind::None; return Ok(NIL); } };
            if r == UNDEF { return Ok(NIL); }
            let me = lookup_method(class_of(r), name);
            if me.is_null() {
                // respond_to_missing?
                let rm = lookup_method(class_of(r), s::respond_to_missing_p);
                if !rm.is_null() && unsafe { (*rm).owner } != vm().c_object && unsafe { (*rm).owner } != vm().m_kernel {
                    let t = funcall(c, r, s::respond_to_missing_p, &[Value::sym(name), TRUE], std::ptr::null())?;
                    if t.truthy() { return yes(b"method"); }
                }
                return Ok(NIL);
            }
            let visi = unsafe { (*me).visi };
            if visi == VISI_PUBLIC { return yes(b"method"); }
            if visi == VISI_PROTECTED && is_kind_of(fp[0].get(), unsafe { (*me).owner }) { return yes(b"method"); }
            Ok(NIL)
        }
        DEF_YIELD => if unsafe { !(*c.frame).blk.is_null() } { yes(b"yield") } else { Ok(NIL) },
        DEF_SUPER => {
            let f = c.frame();
            if f.me.is_null() { return Ok(NIL); }
            let me = unsafe { &*f.me };
            let sm = super_method(class_of(fp[0].get()), me.owner, me.name);
            if sm.is_null() { Ok(NIL) } else { yes(b"super") }
        }
        _ => yes(b"expression"),
    }
}

fn const_defined_lex(c: &mut Ctx, cref: *const Cref, name: Sym) -> bool {
    let mut cr = cref;
    unsafe {
        while !cr.is_null() && !(*cr).prev.is_null() {
            let k = (*cr).klass;
            if rclass(k).consts.contains_key(&name) || rclass(k).autoload.contains_key(&name) { return true; }
            cr = (*cr).prev;
        }
    }
    let start = if cref.is_null() { vm().c_object } else { unsafe { (*cref).klass } };
    let _ = c;
    ancestors(start).iter().any(|&k| rclass(k).consts.contains_key(&name) || rclass(k).autoload.contains_key(&name))
        || rclass(vm().c_object).consts.contains_key(&name) || rclass(vm().c_object).autoload.contains_key(&name)
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
        AK_ARG => { ary_push(tmp, v); }
        AK_SPLAT => {
            let a = splat_to_array(c, v)?;
            let n = rary(a).len();
            for j in 0..n { let x = rary(a)[j]; ary_push(tmp, x); }
            // a ruby2_keywords hash at the end of a splat is passed as keywords
            if n > 0 {
                let last = rary(a)[n - 1];
                if is_hash(last) && obj(last).flags & FL_KWHASH != 0 && *kw == NIL {
                    rary(tmp).pop();
                    let h = hash_new();
                    for (k, x) in hash_pairs(last) { hash_aset(c, h, k, x)?; }
                    ary_push(tmp, h);
                    *kw = h;
                    rary(tmp).pop();
                }
            }
        }
        AK_KW => {
            if *kw == NIL { *kw = hash_new(); ary_push(tmp, *kw); rary(tmp).pop(); }
            srb_hash_merge_kw(c, *kw, v)?;
        }
        AK_KWSPLAT => {
            if *kw == NIL { *kw = hash_new(); }
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
    if is_proc(blkv) { return Ok(proc_block(blkv)); }
    if blkv.is_sym() {
        // &:sym under refinements remembers the scope it was written in
        let cref = unsafe { (*c.frame).cref() };
        if unsafe { REFINEMENTS_ON } && cref_refinements(cref) != NIL {
            *holder = Block::cfunc(c, |_c, _d, _a, _b| Ok(NIL), NIL);
            holder.kind = BlockKind::Sym(blkv.as_sym());
            holder.cref = cref;
            return Ok(holder as *const Block);
        }
        let p = crate::builtins::proc::sym_to_proc(c, blkv)?;
        return Ok(proc_block(p));
    }
    if is_method_obj(blkv) {
        *holder = Block::cfunc(c, |_c, _d, _a, _b| Ok(NIL), NIL);
        holder.kind = BlockKind::Method(blkv);
        return Ok(holder as *const Block);
    }
    if respond_to(blkv, s::to_proc, true) {
        let p = funcall(c, blkv, s::to_proc, &[], std::ptr::null())?;
        if is_proc(p) { return Ok(proc_block(p)); }
        let cls = class_path(real_class_of(blkv));
        return type_err(c, format!("can't convert {} to Proc ({}#to_proc gives {})", cls, cls, class_path(real_class_of(p))));
    }
    type_err(c, format!("wrong argument type {} (expected Proc)", class_path(real_class_of(blkv))))
}

/// Stage tmp (+ kw) as arguments at base and call.
pub fn srb_call_x(c: &mut Ctx, cc: &mut CallCache, mid: Sym, base: Stk, tmp: Value, kw: Value, blkv: Value, ci: CiRef, flags: u32, fp: Fp) -> RV {
    let n = rary(tmp).len();
    let has_kw = kw != NIL && (rhash(kw).live > 0 || ci.kinds.contains(&AK_KW));
    let total = n + if has_kw { 1 } else { 0 };
    stack_check(c, total + 4)?;
    base[1].set(tmp);
    for i in 0..n { base[2 + i].set(rary(tmp)[i]); }
    if has_kw { base[2 + n].set(kw); }
    c.set_sp(base, 2 + total);
    let mut holder = Block::cfunc(c, |_c, _d, _a, _b| Ok(NIL), NIL);
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
pub fn srb_call_splat1(c: &mut Ctx, cc: &mut CallCache, mid: Sym, base: Stk, v: Value, ci: CiRef, flags: u32, fp: Fp) -> RV {
    let n = if is_array(v) { c.ary_len(v) } else { usize::MAX };
    // a ruby2_keywords hash at the end is passed as keywords: the general path
    let kwh = n != usize::MAX && n > 0 && { let l = c.ary(v)[n - 1]; is_hash(l) && crate::heap::obj_flags(l) & FL_KWHASH != 0 };
    if n == usize::MAX || kwh {
        let tmp = crate::array::ary_new();
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
    if f.me.is_null() { return err(c, vm().c_runtime_error, "super called outside of method"); }
    let me = unsafe { &*f.me };
    let slf = fp[0].get();
    let klass = class_of(slf);
    let owner = if f.defined_class() != NIL { f.defined_class() } else { me.owner };
    let sm = super_method(klass, owner, me.name);
    let n = rary(tmp).len();
    let has_kw = kw != NIL && (rhash(kw).live > 0 || ci.kinds.contains(&AK_KW));
    let total = n + if has_kw { 1 } else { 0 };
    stack_check(c, total + 4)?;
    base[0].set(slf);
    base[1].set(tmp);
    for i in 0..n { base[2 + i].set(rary(tmp)[i]); }
    if has_kw { base[2 + n].set(kw); }
    c.set_sp(base, 2 + total);
    let mut holder = Block::cfunc(c, |_c, _d, _a, _b| Ok(NIL), NIL);
    let lit;
    let blk: *const Block = if let Some(is) = ci.blk {
        lit = make_block(c, is, fp);
        &lit
    } else if ci.kinds.contains(&AK_BLOCK) {
        block_from_value(c, blkv, &mut holder)?
    } else { f.blk };
    let fl = CF_FCALL | CF_SUPER | if has_kw { CF_KW | if !ci.kinds.contains(&AK_KW) { CF_KW_SPLAT } else { 0 } } else { 0 };
    let r = if sm.is_null() {
        let mm = lookup_method(klass, s::method_missing);
        if !mm.is_null() && unsafe { (*mm).owner } != vm().c_basic_object {
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

pub fn srb_no_block(c: &mut Ctx) -> RV { err(c, vm().c_local_jump_error, "no block given (yield)") }

/// A yield site's feedback: which block iseq it ran.
#[inline(always)]
pub fn srb_yield_note(yc: &mut YieldCache, blk: *const Block) {
    if let BlockKind::Iseq(is) = unsafe { (*blk).kind } { yc.note(is as usize); } else { yc.seen = 1; }
}

/// A baked yield site whose compiled-in block (entry hash h) did not match: arm the site's
/// guard if this block is that one, then yield out of line.
#[inline(never)]
pub fn srb_yield_miss(c: &mut Ctx, yc: &mut YieldCache, h: u64, blk: *const Block, base: Stk, k: u32) -> RV {
    srb_yield_note(yc, blk);
    if let BlockKind::Iseq(is) = unsafe { (*blk).kind } {
        let bentry = unsafe { (*is).bentry };
        if !bentry.is_null() && crate::node::hash_node(bentry) == h { yc.fast = is as usize; }
    }
    srb_yield_staged(c, blk, base, k)
}

#[inline(always)]
pub fn srb_yield_staged(c: &mut Ctx, blk: *const Block, base: Stk, k: u32) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("yield: node_yield");
    let b = unsafe { &*blk };
    if let Some(is) = block_fast(b, k as usize) {
        stack_check(c, is.nlocals as usize + 16)?;
        c.set_sp(base, 0);
        let r = run_block_fast(c, b, is, base, k);
        c.set_sp(base, 0);
        return r;
    }
    if let BlockKind::Iseq(is) = b.kind {
        let is = unsafe { &*is };
        base[0].set(b.self_);
        let r = run_block_iseq(c, b, is, base, k, std::ptr::null(), is_lambda_block(blk), std::ptr::null(), false);
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
    let mut args = rary(tmp).clone();
    // an empty **h still counts as "keywords given" for an iseq block (no auto-splat for non-simple blocks)
    let has_kw = kw != NIL && (rhash(kw).live > 0 || matches!(unsafe { (*blk).kind }, BlockKind::Iseq(_)) && !is_lambda_block(blk));
    if has_kw { args.push(kw); }
    let b = unsafe { &*blk };
    invoke_block_full(c, b, &args, std::ptr::null(), b.self_, false, std::ptr::null(), has_kw)
}

pub fn srb_lambda(c: &mut Ctx, is: &'static Iseq, fp: Fp) -> RV {
    let b = make_block(c, is, fp);
    let p = block_to_proc(c, &b)?;
    rproc(p).is_lambda = true;
    Ok(p)
}

#[cold]
pub fn srb_binop(c: &mut Ctx, a: Value, b: Value, mid: Sym, line: Pos, cc: &mut OpCc) -> RV {
    let base = stage(c, 3)?;
    base[0].set(a); base[2].set(b);
    c.frame().pos = line;
    call_cached(c, cc.get(), mid, base, 1, std::ptr::null(), 0)
}

#[cold]
pub fn srb_eq(c: &mut Ctx, a: Value, b: Value, line: Pos, cc: &mut OpCc) -> RV {
    if is_string(a) && is_string(b) && obj(a).klass == vm().c_string { return Ok(Value::bool(crate::string::str_eq(a, b))); }
    srb_binop(c, a, b, s::eq, line, cc)
}

pub fn srb_aset(c: &mut Ctx, a: Value, b: Value, v: Value, line: Pos, cc: &mut OpCc) -> RV {
    let base = stage(c, 4)?;
    base[0].set(a); base[2].set(b); base[3].set(v);
    c.frame().pos = line;
    call_cached(c, cc.get(), s::aset, base, 2, std::ptr::null(), 0)?;
    Ok(v)
}

// ---- definitions --------------------------------------------------------------------------

fn cref_class(c: &Ctx) -> Value {
    let cr = unsafe { (*c.frame).cref() };
    if cr.is_null() { vm().c_object } else { unsafe { (*cr).klass } }
}

pub fn define_iseq_method(c: &mut Ctx, klass: Value, mid: Sym, is: &'static Iseq, visi: u8) {
    let cref = unsafe { (*c.frame).cref() };
    let me = new_me_cref(mid, klass, visi, MKind::Iseq(is), cref);
    note_redefinition(c, klass, mid);
    define_method_me(klass, mid, me);
}

pub fn srb_def(c: &mut Ctx, mid: Sym, is: &'static Iseq, _fp: Fp) -> RV {
    let f = c.frame();
    let mut klass = cref_class(c);
    // `def` inside a method body defines on the cref class; inside instance_eval, on the singleton
    if f.defined_class() != NIL && f.kind != FK_CLASS { klass = f.defined_class(); }
    let mut visi = if f.kind == FK_CLASS || f.kind == FK_TOP || f.kind == FK_EVAL || (f.kind == FK_BLOCK && f.defined_class() == klass) { f.visi } else { VISI_PUBLIC };
    // eval at top level defines private methods; a def in a method body is public
    if klass == vm().c_object && f.kind == FK_EVAL && unsafe { (*f.cref()).prev.is_null() } { visi = VISI_PRIVATE; }
    if mid == s::initialize || mid == s::initialize_copy || mid == s::respond_to_missing_p
        || mid == s::mid_initialize_dup || mid == s::mid_initialize_clone { visi = VISI_PRIVATE; }
    if visi == VISI_MODFUNC {
        define_iseq_method(c, klass, mid, is, VISI_PRIVATE);
        let sk = singleton_class(klass);
        define_iseq_method(c, sk, mid, is, VISI_PUBLIC);
    } else {
        define_iseq_method(c, klass, mid, is, visi);
    }
    crate::builtins::object::call_method_added(c, klass, mid)?;
    Ok(Value::sym(mid))
}

pub fn srb_sdef(c: &mut Ctx, recv: Value, mid: Sym, is: &'static Iseq) -> RV {
    if !recv.is_heap() || recv.is_fixnum() {
        return type_err(c, format!("can't define singleton"));
    }
    if is_frozen(recv) { return Err(raise_frozen(c, recv)); }
    let k = singleton_class(recv);
    define_iseq_method(c, k, mid, is, VISI_PUBLIC);
    if crate::vm::respond_to(recv, s::singleton_method_added, true) {
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
    fr.id = c.next_serial(); fr.cref = cref; fr.pos = Pos::at(&is.info.line);
    base[0].set(klass); base[1].set(NIL);
    nil_fill(base, 2, is.nlocals as usize);
    c.set_sp(base, is.nlocals as usize);
    if is.env_size > 0 {
        let env = crate::gc::alloc(vm().c_env, Body::Env(crate::envvals::EnvVals::new(is.env_size as usize), NIL));
        base[1].set(env);
    }
    c.frame = &mut fr.h;
    let r = run_body(c, is.body, base);
    c.frame = fr.prev;
    c.set_sp(base, 0);
    r
}

pub fn srb_class(c: &mut Ctx, outer: Value, name: Sym, sup: Value, is: &'static Iseq, _fp: Fp) -> RV {
    let outer = if outer == UNDEF {
        let cr = unsafe { (*c.frame).cref() };
        if outer == UNDEF && false { vm().c_object } else if cr.is_null() { vm().c_object } else { unsafe { (*cr).klass } }
    } else { outer };
    let outer = if outer == NIL { vm().c_object } else { outer };
    if !is_class_or_module(outer) {
        let s = crate::builtins::kernel::inspect_str(c, outer)?;
        return type_err(c, format!("{} is not a class/module", s));
    }
    if sup != UNDEF && !(is_class_or_module(sup) && !rclass(sup).is_module) {
        return type_err(c, format!("superclass must be an instance of Class (given an instance of {})", class_path(real_class_of(sup))));
    }
    let existing = const_get_at(outer, name).or_else(|| if outer == vm().c_object { None } else { None });
    let existing = match existing {
        Some(v) => Some(v),
        None => {
            if let Some(&path) = rclass(outer).autoload.get(&name) {
                rclass(outer).autoload.remove(&name);
                crate::builtins::kernel::require_feature(c, path, false)?;
                const_get_at(outer, name)
            } else { None }
        }
    };
    let klass = match existing {
        Some(k) => {
            if !is_class_or_module(k) || rclass(k).is_module {
                return type_err(c, format!("{} is not a class", sym_str(name)));
            }
            if sup != UNDEF && rclass(k).superclass != sup {
                let mut real_sup = rclass(k).superclass;
                while real_sup != NIL && rclass(real_sup).is_singleton { real_sup = rclass(real_sup).superclass; }
                if real_sup != sup { return type_err(c, format!("superclass mismatch for class {}", sym_str(name))); }
            }
            k
        }
        None => {
            let s = if sup == UNDEF { vm().c_object } else { sup };
            if s == vm().c_class { return type_err(c, "can't make subclass of Class"); }
            if rclass(s).is_singleton { return type_err(c, "can't make subclass of singleton class"); }
            let k = new_class(s);
            name_class(k, outer, name);
            const_set(outer, name, k);
            crate::builtins::object::call_inherited(c, s, k)?;
            k
        }
    };
    run_class_body(c, klass, is)
}

pub fn srb_module(c: &mut Ctx, outer: Value, name: Sym, is: &'static Iseq, _fp: Fp) -> RV {
    let outer = if outer == UNDEF {
        let cr = unsafe { (*c.frame).cref() };
        if cr.is_null() { vm().c_object } else { unsafe { (*cr).klass } }
    } else { outer };
    if !is_class_or_module(outer) {
        let s = crate::builtins::kernel::inspect_str(c, outer)?;
        return type_err(c, format!("{} is not a class/module", s));
    }
    // reopening an autoload constant loads it first (as srb_class does)
    if const_get_at(outer, name).is_none() {
        if let Some(path) = rclass(outer).autoload.remove(&name) { crate::builtins::kernel::require_feature(c, path, false)?; }
    }
    let m = match const_get_at(outer, name) {
        Some(m) => {
            if !is_class_or_module(m) || !rclass(m).is_module { return type_err(c, format!("{} is not a module", sym_str(name))); }
            m
        }
        None => {
            let m = new_module();
            name_class(m, outer, name);
            const_set(outer, name, m);
            m
        }
    };
    run_class_body(c, m, is)
}

pub fn srb_sclass(c: &mut Ctx, recv: Value, is: &'static Iseq, _fp: Fp) -> RV {
    if recv.is_fixnum() || recv.is_sym() || recv.is_flonum() {
        return type_err(c, "can't define singleton");
    }
    let k = singleton_class(recv);
    run_class_body(c, k, is)
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
    let mid = if v.is_sym() { v.as_sym() } else { crate::string::str_to_sym(v).as_sym() };
    crate::builtins::object::undef_method(c, klass, mid)
}

// ---- multiple assignment ------------------------------------------------------------------

pub fn srb_masgn_array(c: &mut Ctx, rhs: Value, _splat: bool) -> RV {
    if is_array(rhs) { return Ok(rhs); }
    if rhs.is_heap() && !is_hash(rhs) && !is_string(rhs) && respond_to(rhs, s::to_ary, true) {
        let r = funcall(c, rhs, s::to_ary, &[], std::ptr::null())?;
        if is_array(r) { return Ok(r); }
        if r != NIL { return type_err(c, format!("can't convert {} to Array", class_path(real_class_of(rhs)))); }
    }
    Ok(ary_new_from(&[rhs]))
}

pub fn srb_masgn_elem(a: Value, idx: u32, kind: u32, post: u32) -> Value {
    let v = rary(a);
    let n = v.len();
    match kind {
        0 => v.get(idx as usize).copied().unwrap_or(NIL),
        1 => {
            let s = idx as usize;
            let e = n.saturating_sub(post as usize);
            if s < e { ary_new_from(&v[s..e]) } else { ary_new() }
        }
        _ => {
            // post element i: from the back, but not overlapping the front part
            let pre = n.saturating_sub(post as usize);
            let i = pre + idx as usize;
            if (n as i64) < (post as i64) {
                // fewer elements than needed: posts take what's after the pre part
                v.get(idx as usize + n.saturating_sub(post as usize)).copied().unwrap_or(NIL)
            } else { v.get(i).copied().unwrap_or(NIL) }
        }
    }
}

pub fn srb_unsupported(c: &mut Ctx, what: Str) -> RV {
    err(c, vm().c_not_impl_error, String::from_utf8_lossy(what))
}

/// Pattern matching asks `respond_to?` (so respond_to_missing? counts).
fn pm_responds(c: &mut Ctx, v: Value, name: &[u8]) -> Result<bool, Esc> {
    if respond_to(v, intern(name), false) { return Ok(true); }
    Ok(funcall(c, v, intern(b"respond_to?"), &[Value::sym(intern(name))], std::ptr::null())?.truthy())
}

pub fn srb_special<D: Disp>(c: &mut Ctx, _n: NodePtr, fp: Fp, kind: u32, arg: NodePtr) -> RV {
    let v = D::call(c, arg, fp)?;
    match kind {
        SP_IS_NIL => Ok(Value::bool(v == NIL)),
        SP_NOMATCH => {
            let s = crate::builtins::kernel::inspect_str(c, v)?;
            err(c, vm().c_no_matching_pattern_error, s)
        }
        SP_NOMATCH_MSG => {
            let (subj, why) = (rary(v)[0], rary(v)[1]);
            let s = crate::builtins::kernel::inspect_str(c, subj)?;
            if is_array(why) {
                // [message, matchee, key]: a missing Hash key
                let msg = format!("{}: {}", s, String::from_utf8_lossy(str_bytes(rary(why)[0])));
                let e = make_exception(vm().c_no_matching_pattern_key_error, &msg);
                ivar_set_raw(e, s::receiver, rary(why)[1]);
                ivar_set_raw(e, s::key, rary(why)[2]);
                return Err(raise_exc(c, e));
            }
            let msg = if why == NIL { s } else { format!("{}: {}", s, String::from_utf8_lossy(str_bytes(why))) };
            err(c, vm().c_no_matching_pattern_error, msg)
        }
        SP_PM_MSG => {
            // elements stay in `v` (GC-visible) while inspect allocates
            let a = |i: usize| rary(v)[i];
            let insp = crate::builtins::kernel::inspect_str;
            let msg = match a(0).as_fix() {
                PM_EQQ => format!("{} === {} does not return true", insp(c, a(1))?, insp(c, a(2))?),
                PM_GUARD => "guard clause does not return true".to_string(),
                PM_ARY if a(2) == NIL => format!("{} does not respond to #deconstruct", insp(c, a(1))?),
                PM_ARY => format!("{} length mismatch (given {}, expected {}{})", insp(c, a(2))?, rary(a(2)).len(), a(3).as_fix(), if a(4) == TRUE { "" } else { "+" }),
                PM_DECON => format!("{} does not respond to #deconstruct", insp(c, a(1))?),
                PM_FIND => format!("{} does not match to find pattern", insp(c, a(1))?),
                PM_DECONK => format!("{} does not respond to #deconstruct_keys", insp(c, a(1))?),
                PM_KEY => {
                    let m = str_new(format!("key not found: {}", insp(c, a(2))?).as_bytes());
                    return Ok(ary_new_from(&[m, a(1), a(2)]));
                }
                PM_REST => format!("rest of {} is not empty", insp(c, a(1))?),
                _ => format!("{} is not empty", insp(c, a(1))?),
            };
            Ok(str_new(msg.as_bytes()))
        }
        SP_NAMED_CAPTURE => {
            let m = c.last_match;
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
            let a = rary(v)[0]; let k = rary(v)[1].as_fix() as usize;
            if !is_array(a) { return Ok(FALSE); }
            let n = rary(a).len();
            Ok(Value::bool(if kind == SP_LEN_EQ { n == k } else { n >= k }))
        }
        SP_DECONSTRUCT_KEYS => {
            let subj = rary(v)[0]; let keys = rary(v)[1];
            if is_hash(subj) { return Ok(subj); }
            if pm_responds(c, subj, b"deconstruct_keys")? {
                let r = funcall(c, subj, intern(b"deconstruct_keys"), &[keys], std::ptr::null())?;
                if !is_hash(r) { return type_err(c, "deconstruct_keys must return Hash"); }
                return Ok(r);
            }
            Ok(NIL)
        }
        SP_HASH_HAS => {
            let h = rary(v)[0]; let k = rary(v)[1];
            if !is_hash(h) { return Ok(FALSE); }
            Ok(Value::bool(hash_get_opt(c, h, k)?.is_some()))
        }
        SP_HASH_EXCEPT => {
            let h = rary(v)[0]; let used = rary(v)[1];
            let out = hash_new();
            if is_hash(h) {
                for (k, x) in hash_pairs(h) {
                    if rary(used).contains(&k) { continue; }
                    hash_aset(c, out, k, x)?;
                }
            }
            Ok(out)
        }
        SP_EMPTY_P => Ok(Value::bool(is_hash(v) && rhash(v).live == 0)),
        SP_FOR_UNPACK => {
            if is_array(v) && rary(v).len() == 1 { Ok(rary(v)[0]) } else { Ok(v) }
        }
        SP_FIND_ELEM => {
            let a = rary(v)[0]; let i = rary(v)[1].as_fix() as usize; let j = rary(v)[2].as_fix() as usize;
            Ok(rary(a).get(i + j).copied().unwrap_or(NIL))
        }
        SP_LEN => Ok(Value::fix(if is_array(v) { rary(v).len() as i64 } else { -1 })),
        SP_FIND_PRE => { let a = rary(v)[0]; let i = rary(v)[1].as_fix() as usize; Ok(ary_new_from(&rary(a)[..i])) }
        SP_FIND_POST => {
            let a = rary(v)[0]; let i = rary(v)[1].as_fix() as usize; let k = rary(v)[2].as_fix() as usize;
            let s = (i + k).min(rary(a).len());
            Ok(ary_new_from(&rary(a)[s..]))
        }
        _ => Ok(NIL),
    }
}

/// Call with literal keywords staged as values: an Iseq callee taking only required +
/// keyword params gets them directly in its slots; otherwise a keyword Hash is built.
pub fn srb_call_kw(c: &mut Ctx, cc: &mut CallCache, mid: Sym, base: Stk, k: usize, ci: CiRef, flags: u32, fp: Fp) -> RV {
    let nkw = ci.kw_names.len();
    let npos = k - nkw;
    let recv = base[0].get();
    let lit;
    let blk: *const Block = match ci.blk { Some(is) => { lit = make_block(c, is, fp); &lit } None => std::ptr::null() };
    let me = lookup_cc(c, cc, recv, mid, flags);
    // every keyword in declared order to a dense method: its kentry with the staged values as params
    if cc.fast_argc != KW_DENSE | k as u32 && !me.is_null() { kw_dense_fill(cc, me, npos, &ci.kw_names, flags); }
    let r = if cc.fast_argc == KW_DENSE | k as u32 {
        c.call_me = me; c.call_blk = blk;
        native_check(c)?;
        let r = run_body(c, cc.fast as NodePtr, base);
        c.set_sp(base, 0);
        r
    } else if !me.is_null() && kw_direct_ok(me, npos, &ci.kw_names, flags) {
        let is = match unsafe { (*me).kind } { MKind::Iseq(is) => unsafe { &*is }, _ => unreachable!() };
        let r = invoke_iseq_kw(c, me, is, recv, base, npos, &ci.kw_names, blk);
        c.set_sp(base, 0);
        r
    } else {
        let h = hash_new_capa(nkw);
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
pub fn srb_call_kw_spec(c: &mut Ctx, cc: &mut CallCache, mid: Sym, base: Stk, k: usize, ci: CiRef, flags: u32, fp: Fp, h: u64) -> RV {
    let r = srb_call_kw(c, cc, mid, base, k, ci, flags, fp);
    cc.spec_klass = if cc.fast_hash == h && cc.fast_argc == KW_DENSE | k as u32 { cc.klass } else { 0 };
    r
}
