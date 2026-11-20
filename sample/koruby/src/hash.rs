// Hash: insertion-ordered entries + open-addressing index (built once the hash grows).
use crate::array::*;
use crate::symbol::s;
use crate::value::*;
use crate::vm::*;

const SMALL: usize = 8;
const EMPTY: u32 = u32::MAX;

// Hashes being iterated (CRuby's iter_lev): adding a key or rehashing then raises.
thread_local!(static ITER: std::cell::RefCell<Vec<Value>> = const { std::cell::RefCell::new(Vec::new()) });
pub fn hash_iter_enter(h: Value) { ITER.with(|v| v.borrow_mut().push(h)); }
pub fn hash_iter_leave(h: Value) {
    ITER.with(|v| { let mut v = v.borrow_mut(); if let Some(i) = v.iter().rposition(|x| *x == h) { v.remove(i); } });
}
pub fn hash_iterating(h: Value) -> bool { ITER.with(|v| { let v = v.borrow(); !v.is_empty() && v.contains(&h) }) }

pub fn hash_new() -> Value {
    crate::gc::alloc(vm().c_hash, Body::Hash(Box::new(RHash { entries: Vec::new(), index: Vec::new(), live: 0, default: NIL, default_proc: NIL, identity: false })))
}
pub fn hash_new_capa(n: usize) -> Value {
    let h = hash_new();
    rhash(h).entries.reserve(n);
    h
}

fn mix(h: u64) -> u64 { crate::astrust::hash_u64(h) }

/// Ruby `hash` of a key.
pub fn key_hash(c: &mut Ctx, k: Value) -> Result<u64, Esc> {
    if k.is_fixnum() || k.is_sym() || !k.is_heap() {
        if k.is_flonum() { let f = k.flonum_val(); return Ok(mix(if f == 0.0 { 0 } else { f.to_bits() })); }
        return Ok(mix(k.raw()));
    }
    let o = obj(k);
    match &o.body {
        Body::String(s) => return Ok(crate::astrust::hash_bytes(&s.b)),
        Body::Float(f) => return Ok(mix(if *f == 0.0 { 0 } else { f.to_bits() })),
        Body::Bignum(b) => return Ok(crate::astrust::hash_str(&b.to_string_radix(16))),
        _ => {}
    }
    let me = lookup_method(class_of(k), s::hash);
    if !me.is_null() {
        if let MKind::Cfunc(f, _) = unsafe { (*me).kind } {
            let _ = f;
            if me == unsafe { crate::builtins::object::KERNEL_HASH_ME } { return Ok(mix(k.raw())); }
        }
    }
    if is_array(k) && unsafe { (*me).owner } == vm().c_array {
        let mut h: u64 = 0x345678 ^ rary(k).len() as u64;
        let n = rary(k).len();
        if crate::builtins::kernel::recursion_enter(k, s::hash) {
            for i in 0..n { let x = ary_get(k, i); let xh = key_hash(c, x)?; h = crate::astrust::hash_merge(h, xh); }
            crate::builtins::kernel::recursion_leave(k, s::hash);
        }
        return Ok(h);
    }
    let r = funcall(c, k, s::hash, &[], std::ptr::null())?;
    if r.is_fixnum() { return Ok(mix(r.as_fix() as u64)); }
    if is_bignum(r) { return Ok(crate::astrust::hash_str(&rbig(r).to_string_radix(16))); }
    if r.is_heap() && !is_float(r) && respond_to(r, s::to_int, true) {
        let i = funcall(c, r, s::to_int, &[], std::ptr::null())?;
        if i.is_fixnum() { return Ok(mix(i.as_fix() as u64)); }
        if is_bignum(i) { return Ok(crate::astrust::hash_str(&rbig(i).to_string_radix(16))); }
    }
    type_err(c, format!("no implicit conversion of {} into Integer", crate::builtins::kernel::class_name_for_err(r)))
}

/// Ruby `eql?`.
pub fn key_eql(c: &mut Ctx, a: Value, b: Value) -> Result<bool, Esc> {
    if a == b { return Ok(true); }
    if !a.is_heap() {
        if a.is_flonum() && is_float(b) { return Ok(a.flonum_val() == float_val(b)); }
        return Ok(false);
    }
    match (&obj(a).body, b.is_heap()) {
        (Body::String(x), true) => if let Body::String(y) = &obj(b).body { return Ok(x.b == y.b); } else { return Ok(false); },
        (Body::Float(x), _) => return Ok(is_float(b) && *x == float_val(b)),
        (Body::Bignum(x), true) => if let Body::Bignum(y) = &obj(b).body { return Ok(x == y); } else { return Ok(false); },
        _ => {}
    }
    let r = funcall(c, a, s::eql_p, &[b], std::ptr::null())?;
    Ok(r.truthy())
}

fn identity_hash(k: Value) -> u64 { mix(k.raw()) }

fn rebuild_index(h: &mut RHash) {
    // compact deleted entries first when many
    if h.entries.len() > h.live * 2 + 8 {
        h.entries.retain(|e| e.key != UNDEF);
    }
    if h.entries.len() <= SMALL { h.index.clear(); return; }
    let mut cap = 16;
    while cap < h.entries.len() * 2 { cap <<= 1; }
    h.index.clear();
    h.index.resize(cap, EMPTY);
    let mask = cap - 1;
    for (i, e) in h.entries.iter().enumerate() {
        if e.key == UNDEF { continue; }
        let mut p = (e.hash as usize) & mask;
        while h.index[p] != EMPTY { p = (p + 1) & mask; }
        h.index[p] = i as u32;
    }
}

/// Entry index of key k.
pub fn hash_lookup(c: &mut Ctx, hv: Value, k: Value) -> Result<Option<usize>, Esc> {
    let identity = rhash(hv).identity;
    let hc = if identity { identity_hash(k) } else { key_hash(c, k)? };
    lookup_with_hash(c, hv, k, hc)
}

fn lookup_with_hash(c: &mut Ctx, hv: Value, k: Value, hc: u64) -> Result<Option<usize>, Esc> {
    let identity = rhash(hv).identity;
    let h = rhash(hv);
    if h.index.is_empty() {
        let n = h.entries.len();
        for i in 0..n {
            let e = rhash(hv).entries[i];
            if e.key == UNDEF || e.hash != hc { continue; }
            if e.key == k || (!identity && key_eql(c, k, e.key)?) { return Ok(Some(i)); }
        }
        return Ok(None);
    }
    let mask = h.index.len() - 1;
    let mut p = (hc as usize) & mask;
    loop {
        let h = rhash(hv);
        let ix = h.index[p];
        if ix == EMPTY { return Ok(None); }
        let e = h.entries[ix as usize];
        if e.key != UNDEF && e.hash == hc && (e.key == k || (!identity && key_eql(c, k, e.key)?)) { return Ok(Some(ix as usize)); }
        p = (p + 1) & mask;
    }
}

pub fn hash_get_opt(c: &mut Ctx, h: Value, k: Value) -> Result<Option<Value>, Esc> {
    Ok(hash_lookup(c, h, k)?.map(|i| rhash(h).entries[i].val))
}

pub fn hash_default_value(c: &mut Ctx, h: Value, k: Value) -> RV {
    let r = rhash(h);
    if r.default_proc != NIL { let p = r.default_proc; return call_proc(c, p, &[h, k], std::ptr::null()); }
    Ok(r.default)
}

pub fn hash_aref(c: &mut Ctx, h: Value, k: Value) -> RV {
    match hash_lookup(c, h, k)? {
        Some(i) => Ok(rhash(h).entries[i].val),
        None => {
            if obj(h).klass != vm().c_hash {
                let me = lookup_method(class_of(h), s::default);
                if !me.is_null() && !matches!(unsafe { (*me).kind }, MKind::Cfunc(..)) {
                    return funcall(c, h, s::default, &[k], std::ptr::null());
                }
            }
            hash_default_value(c, h, k)
        }
    }
}

pub fn hash_aset(c: &mut Ctx, h: Value, k: Value, v: Value) -> Result<(), Esc> {
    if obj(h).flags & FL_FROZEN != 0 { return Err(raise_frozen(c, h)); }
    let identity = rhash(h).identity;
    let k = if !identity && is_string(k) && obj(k).flags & FL_FROZEN == 0 {
        let d = crate::string::str_new_enc(&rstr(k).b, rstr(k).enc);
        obj(d).flags |= FL_FROZEN;
        d
    } else { k };
    hash_set(c, h, k, v)
}

/// Insert without key dup / frozen check.
pub fn hash_set(c: &mut Ctx, h: Value, k: Value, v: Value) -> Result<(), Esc> {
    let identity = rhash(h).identity;
    let hc = if identity { identity_hash(k) } else { key_hash(c, k)? };
    if let Some(i) = lookup_with_hash(c, h, k, hc)? { rhash(h).entries[i].val = v; return Ok(()); }
    if hash_iterating(h) { return err(c, vm().c_runtime_error, "can't add a new key into hash during iteration"); }
    let r = rhash(h);
    r.entries.push(HEntry { key: k, val: v, hash: hc });
    r.live += 1;
    if r.entries.len() > SMALL {
        if r.index.is_empty() || r.entries.len() * 2 > r.index.len() { rebuild_index(r); }
        else {
            let mask = r.index.len() - 1;
            let mut p = (hc as usize) & mask;
            while r.index[p] != EMPTY { p = (p + 1) & mask; }
            r.index[p] = (r.entries.len() - 1) as u32;
        }
    }
    if r.entries.len() % 64 == 0 { crate::gc::malloc_increase(64 * 24); }
    Ok(())
}

pub fn hash_delete(c: &mut Ctx, h: Value, k: Value) -> Result<Option<Value>, Esc> {
    match hash_lookup(c, h, k)? {
        Some(i) => {
            let r = rhash(h);
            let v = r.entries[i].val;
            r.entries[i].key = UNDEF;
            r.entries[i].val = NIL;
            r.live -= 1;
            if r.live == 0 { r.entries.clear(); r.index.clear(); }
            else if r.entries.len() > r.live * 2 + 16 { rebuild_index(r); }
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

/// Delete the entry in slot i (found by iteration, so stale-hash keys go too).
pub fn hash_delete_slot(h: Value, i: usize) {
    let r = rhash(h);
    if i >= r.entries.len() || r.entries[i].key == UNDEF { return; }
    r.entries[i].key = UNDEF;
    r.entries[i].val = NIL;
    r.live -= 1;
    if r.live == 0 { r.entries.clear(); r.index.clear(); }
}

pub fn hash_clear(h: Value) { let r = rhash(h); r.entries.clear(); r.index.clear(); r.live = 0; }

pub fn hash_rehash(c: &mut Ctx, h: Value) -> Result<(), Esc> {
    if hash_iterating(h) { return err(c, vm().c_runtime_error, "rehash during iteration"); }
    let pairs = hash_pairs(h);
    let keep = ary_new();
    for (k, v) in &pairs { ary_push(keep, *k); ary_push(keep, *v); }
    hash_clear(h);
    for (k, v) in pairs { hash_set(c, h, k, v)?; }
    std::hint::black_box(keep);
    Ok(())
}

pub fn hash_pairs(h: Value) -> Vec<(Value, Value)> {
    rhash(h).entries.iter().filter(|e| e.key != UNDEF).map(|e| (e.key, e.val)).collect()
}
pub fn hash_len(h: Value) -> usize { rhash(h).live }

/// The i-th live entry (i counts all slots), for index-based iteration robust to mutation.
pub fn hash_entry_at(h: Value, i: usize) -> Option<(Value, Value)> {
    let r = rhash(h);
    r.entries.get(i).map(|e| (e.key, e.val))
}
pub fn hash_slots(h: Value) -> usize { rhash(h).entries.len() }

pub fn hash_to_a(_c: &mut Ctx, h: Value) -> RV {
    let a = ary_new_capa(hash_len(h));
    for (k, v) in hash_pairs(h) { let p = ary_new_from(&[k, v]); ary_push(a, p); }
    Ok(a)
}

pub fn hash_dup(h: Value) -> Value {
    let r = rhash(h);
    let n = crate::gc::alloc(obj(h).klass, Body::Hash(Box::new(RHash {
        entries: r.entries.iter().filter(|e| e.key != UNDEF).copied().collect(), index: vec![], live: r.live,
        default: r.default, default_proc: r.default_proc, identity: r.identity })));
    rebuild_index(rhash(n));
    n
}

/// Implicit conversion to Hash (to_hash).
pub fn check_hash(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if is_hash(v) { return Ok(Some(v)); }
    if v.is_heap() && respond_to(v, s::to_hash, true) {
        let r = funcall(c, v, s::to_hash, &[], std::ptr::null())?;
        if is_hash(r) { return Ok(Some(r)); }
        return type_err(c, "can't convert to Hash");
    }
    Ok(None)
}
