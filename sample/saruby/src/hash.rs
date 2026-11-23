// Hash: insertion-ordered entries + open-addressing index (built once the hash grows).
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).  Methods are in builtins/hash_methods.rs.
#![forbid(unsafe_code)]

#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::array::*;
use crate::symbol::s;
use crate::value::*;
use crate::vm::*;

const SMALL: usize = 8;
const EMPTY: u32 = u32::MAX;

// keys whose eql? is identity (and whose hash is mix(raw)): small tables are scanned by key alone
#[inline(always)]
fn imm_key(k: Value) -> bool { k.is_fixnum() || (k.is_special() && k != UNDEF) }

fn mix(h: u64) -> u64 { crate::astrust::hash_u64(h) }
fn identity_hash(k: Value) -> u64 { mix(k.raw()) }

/// What one scan of the table under a shared borrow ends with.
enum Probe { Found(usize), Miss, Ask(usize, Value) }

impl RHash {
    #[inline]
    pub fn with_capacity(n: usize) -> RHash {
        RHash { entries: Vec::with_capacity(n), index: Vec::new(), live: 0, default: NIL, default_proc: NIL, identity: false, iter_lev: 0, first: 0 }
    }
    pub fn pairs(&self) -> Vec<(Value, Value)> { self.entries.iter().filter(|e| e.key != UNDEF).map(|e| (e.key, e.val)).collect() }
    pub fn clear(&mut self) { self.entries.clear(); self.index.clear(); self.live = 0; self.first = 0; }
    /// A compacted copy: same entries, defaults and identity flag, not being iterated.
    pub fn dup(&self) -> RHash {
        let mut d = RHash { entries: self.entries.iter().filter(|e| e.key != UNDEF).copied().collect(), index: vec![], live: self.live,
                            default: self.default, default_proc: self.default_proc, identity: self.identity, iter_lev: 0, first: 0 };
        d.rebuild_index();
        d
    }
    pub fn rebuild_index(&mut self) {
        // compact deleted entries first when many; never under an iteration, which walks the slots by number
        if self.entries.len() > self.live * 2 + 8 && self.iter_lev == 0 {
            self.entries.retain(|e| e.key != UNDEF);
            self.first = 0;
        }
        if self.entries.len() <= SMALL { self.index.clear(); return; }
        let mut cap = 16;
        while cap < self.entries.len() * 2 { cap <<= 1; }
        self.index.clear();
        self.index.resize(cap, EMPTY);
        let mask = cap - 1;
        for (i, e) in self.entries.iter().enumerate() {
            if e.key == UNDEF { continue; }
            let mut p = (e.hash as usize) & mask;
            while self.index[p] != EMPTY { p = (p + 1) & mask; }
            self.index[p] = i as u32;
        }
    }
    /// Scan on from step `*st` for hash code `hc`: the slot holding `k` itself, or the next one whose key needs `eql?`.
    #[inline]
    fn probe(&self, k: Value, hc: u64, st: &mut usize) -> Probe {
        if self.index.is_empty() {
            while let Some(e) = self.entries.get(*st) {
                let i = *st;
                *st += 1;
                if e.key == UNDEF || e.hash != hc { continue; }
                if e.key == k { return Probe::Found(i); }
                if !self.identity { return Probe::Ask(i, e.key); }
            }
            return Probe::Miss;
        }
        let mask = self.index.len() - 1;
        loop {
            let ix = self.index[(hc as usize).wrapping_add(*st) & mask];
            if ix == EMPTY { return Probe::Miss; }
            *st += 1;
            let e = &self.entries[ix as usize];
            if e.key == UNDEF || e.hash != hc { continue; }
            if e.key == k { return Probe::Found(ix as usize); }
            if !self.identity { return Probe::Ask(ix as usize, e.key); }
        }
    }
    /// Slot of an immediate key: it is its own hash and `eql?` is identity, so no Ruby code runs.
    #[inline]
    fn find_imm(&self, k: Value) -> Option<usize> {
        if self.index.is_empty() { return self.entries.iter().position(|e| e.key == k); }
        let mask = self.index.len() - 1;
        let mut p = (mix(k.raw()) as usize) & mask;
        loop {
            let ix = self.index[p];
            if ix == EMPTY { return None; }
            if self.entries[ix as usize].key == k { return Some(ix as usize); }
            p = (p + 1) & mask;
        }
    }
    /// Value of an immediate key.
    #[inline]
    fn get_imm(&self, k: Value) -> Option<Value> {
        if self.index.is_empty() { return self.entries.iter().find(|e| e.key == k).map(|e| e.val); }
        self.find_imm(k).map(|i| self.entries[i].val)
    }
    /// Append a key known to be absent; the new number of slots.
    fn insert_new(&mut self, k: Value, v: Value, hc: u64) -> usize {
        self.entries.push(HEntry { key: k, val: v, hash: hc });
        self.live += 1;
        if self.entries.len() > SMALL {
            if self.index.is_empty() || self.entries.len() * 2 > self.index.len() { self.rebuild_index(); }
            else {
                let mask = self.index.len() - 1;
                let mut p = (hc as usize) & mask;
                while self.index[p] != EMPTY { p = (p + 1) & mask; }
                self.index[p] = (self.entries.len() - 1) as u32;
            }
        }
        self.entries.len()
    }
    /// Delete the live entries `keep` refuses, in place: no Ruby code, no allocation (gc::sweep_weak).
    pub fn prune(&mut self, mut keep: impl FnMut(&mut HEntry) -> bool) {
        let mut i = self.first;
        while i < self.entries.len() {
            if self.entries[i].key != UNDEF && !keep(&mut self.entries[i]) { self.remove_slot(i); }
            i += 1;
        }
    }
    /// Empty slot i (a live one).
    fn remove_slot(&mut self, i: usize) -> Value {
        let v = self.entries[i].val;
        self.entries[i].key = UNDEF;
        self.entries[i].val = NIL;
        self.live -= 1;
        if self.live == 0 { self.clear(); }
        else if i == self.first { while self.entries[self.first].key == UNDEF { self.first += 1; } }
        v
    }
}

impl Ctx {
    pub fn hash_new(&mut self) -> Value { self.hash_new_capa(0) }
    pub fn hash_new_capa(&mut self, n: usize) -> Value {
        let k = self.vm().c_hash.get();
        self.alloc(k, Body::Hash(Box::new(RHash::with_capacity(n))))
    }
    pub fn hash_len(&self, h: Value) -> usize { self.hash(h).live }
    /// Number of slots, deleted ones included: the bound of an iteration by slot number.
    pub fn hash_slots(&self, h: Value) -> usize { self.hash(h).entries.len() }
    /// The first slot that may be live: iterations start here.
    pub fn hash_first_slot(&self, h: Value) -> usize { self.hash(h).first }
    /// Slot i (key UNDEF when deleted), for index-based iteration robust to mutation.
    pub fn hash_entry_at(&self, h: Value, i: usize) -> Option<(Value, Value)> { self.hash(h).entries.get(i).map(|e| (e.key, e.val)) }
    pub fn hash_pairs(&self, h: Value) -> Vec<(Value, Value)> { self.hash(h).pairs() }
    pub fn hash_clear(&mut self, h: Value) { self.hash_mut(h).clear(); }
    pub fn hash_dup(&mut self, h: Value) -> Value {
        let d = self.hash(h).dup();
        self.alloc(obj_klass(h), Body::Hash(Box::new(d)))
    }
    /// Delete the entry in slot i (found by iteration, so stale-hash keys go too).
    pub fn hash_delete_slot(&mut self, h: Value, i: usize) {
        let r = self.hash_mut(h);
        if i < r.entries.len() && r.entries[i].key != UNDEF { r.remove_slot(i); }
    }
    // CRuby's iter_lev: while a hash is iterated, adding a key or rehashing raises
    pub fn hash_iter_enter(&mut self, h: Value) { self.hash_mut(h).iter_lev += 1; }
    pub fn hash_iter_leave(&mut self, h: Value) { let r = self.hash_mut(h); r.iter_lev = r.iter_lev.saturating_sub(1); }
    #[inline]
    pub fn hash_iterating(&self, h: Value) -> bool { self.hash(h).iter_lev != 0 }
}

fn big_hash(c: &Ctx, v: Value) -> u64 { crate::astrust::hash_str(&c.big(v).to_string_radix(16)) }

/// Ruby `hash` of a key.
pub fn key_hash(c: &mut Ctx, k: Value) -> Result<u64, Esc> {
    match hash_of(c, k)? { Hv::Raw(h) => Ok(h), Hv::Ruby(r) => Ok(mix(r)) }
}
/// rb_hash: the value `k.hash` answers, as the bits an Integer#hash / Array#hash result has.
pub fn ruby_hash(c: &mut Ctx, k: Value) -> Result<u64, Esc> {
    match hash_of(c, k)? { Hv::Raw(h) => Ok(h >> 2), Hv::Ruby(r) => Ok(r) }
}
/// A table hash computed here (whose #hash is it >> 2), or what a Ruby #hash returned.
enum Hv { Raw(u64), Ruby(u64) }
fn hash_of(c: &mut Ctx, k: Value) -> Result<Hv, Esc> {
    if k.is_fixnum() || k.is_sym() || !k.is_heap() {
        if k.is_flonum() { let f = k.flonum_val(); return Ok(Hv::Raw(mix(if f == 0.0 { 0 } else { f.to_bits() }))); }
        return Ok(Hv::Raw(mix(k.raw())));
    }
    if is_string(k) { return Ok(Hv::Raw(crate::astrust::hash_bytes(c.str_bytes(k)))); }
    if is_float(k) { let f = float_val(k); return Ok(Hv::Raw(mix(if f == 0.0 { 0 } else { f.to_bits() }))); }
    if is_bignum(k) { return Ok(Hv::Raw(big_hash(c, k))); }
    let (kernel, array) = match c.method_entry(class_of(k), s::hash) {
        Some(me) => (std::ptr::eq(me, c.vm().ids.kernel_hash.get()),
            matches!(me.kind, MKind::Cfunc(f, _) if f as *const () as usize == crate::builtins::array_methods::a_hash as *const () as usize)),
        None => (false, false),
    };
    if kernel { return Ok(Hv::Raw(mix(k.raw()))); }
    if is_array(k) && array { return Ok(Hv::Raw(ary_hash(c, k)?)); }
    let r = funcall(c, k, s::hash, &[], std::ptr::null())?;
    if r.is_fixnum() { return Ok(Hv::Ruby(r.as_fix() as u64)); }
    if is_bignum(r) { return Ok(Hv::Raw(big_hash(c, r))); }
    if r.is_heap() && !is_float(r) && respond_to(c, r, s::to_int, true) {
        let i = funcall(c, r, s::to_int, &[], std::ptr::null())?;
        if i.is_fixnum() { return Ok(Hv::Ruby(i.as_fix() as u64)); }
        if is_bignum(i) { return Ok(Hv::Raw(big_hash(c, i))); }
    }
    type_err(c, format!("no implicit conversion of {} into Integer", crate::builtins::kernel::class_name_for_err(c, r)))
}
/// Array#hash over the elements' #hash values; a recursive array hashes by its length alone.
pub fn ary_hash(c: &mut Ctx, k: Value) -> Result<u64, Esc> {
    let n = c.ary_len(k);
    let start: u64 = 0x345678 ^ n as u64;
    hash_outer(c, k, crate::astrust::hash_merge(start, 0x41727261), |c| {
        let mut h = start;
        let mut i = 0;
        while i < c.ary_len(k) { let x = c.ary_get(k, i); h = crate::astrust::hash_merge(h, ruby_hash(c, x)?); i += 1; }
        Ok(h)
    })
}
thread_local! {
    static HASH_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static HASH_RECURSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// rb_exec_recursive_outer for #hash of `v`: a recursion met anywhere inside makes the outermost
/// call answer `rec`, so equal recursive structures hash alike.
pub fn hash_outer(c: &mut Ctx, v: Value, rec: u64, f: impl FnOnce(&mut Ctx) -> Result<u64, Esc>) -> Result<u64, Esc> {
    if !crate::builtins::kernel::rec_enter(c, v, s::hash)? { HASH_RECURSED.with(|r| r.set(true)); return Ok(rec); }
    let d = HASH_DEPTH.with(|x| { let d = x.get(); x.set(d + 1); d });
    let r = f(c);
    HASH_DEPTH.with(|x| x.set(d));
    crate::builtins::kernel::rec_leave(c, v, s::hash);
    if d == 0 && HASH_RECURSED.with(|x| x.replace(false)) { return r.map(|_| rec); }
    r
}

/// Ruby `eql?`.
pub fn key_eql(c: &mut Ctx, a: Value, b: Value) -> Result<bool, Esc> {
    if a == b { return Ok(true); }
    if !a.is_heap() {
        if a.is_flonum() && is_float(b) { return Ok(a.flonum_val() == float_val(b)); }
        return Ok(false);
    }
    if is_string(a) { return Ok(is_string(b) && c.rstring(a).b == c.rstring(b).b); }
    if is_float(a) { return Ok(is_float(b) && float_val(a) == float_val(b)); }
    if is_bignum(a) && b.is_heap() { return Ok(is_bignum(b) && c.big(a) == c.big(b)); }
    let r = funcall(c, a, s::eql_p, &[b], std::ptr::null())?;
    Ok(r.truthy())
}

/// Entry index of key k.
pub fn hash_lookup(c: &mut Ctx, hv: Value, k: Value) -> Result<Option<usize>, Esc> {
    if imm_key(k) { return Ok(c.hash(hv).find_imm(k)); }
    let hc = if c.hash(hv).identity { identity_hash(k) } else { key_hash(c, k)? };
    lookup_with_hash(c, hv, k, hc)
}

fn lookup_with_hash(c: &mut Ctx, hv: Value, k: Value, hc: u64) -> Result<Option<usize>, Esc> {
    let mut st = 0;
    loop {
        // one borrow up to the next key that has to be asked; `eql?` is Ruby code and may change the table
        let h = c.hash(hv);
        let (i, key) = match h.probe(k, hc, &mut st) { Probe::Found(i) => return Ok(Some(i)), Probe::Miss => return Ok(None), Probe::Ask(i, key) => (i, key) };
        let shape = (h.entries.len(), h.index.len());
        let eq = key_eql(c, k, key)?;
        let h = c.hash(hv);
        let moved = (h.entries.len(), h.index.len()) != shape || h.entries[i].key != key;
        if moved { st = 0; } else if eq { return Ok(Some(i)); }
    }
}

pub fn hash_get_opt(c: &mut Ctx, h: Value, k: Value) -> Result<Option<Value>, Esc> {
    Ok(hash_lookup(c, h, k)?.map(|i| c.hash(h).entries[i].val))
}

/// rb_hash_default_value: a redefined #default is called, the builtin one is read here.
pub fn hash_default_value(c: &mut Ctx, h: Value, k: Value) -> RV {
    if (obj_klass(h) != c.vm().c_hash.get() || crate::rt::redef(crate::rt::R_HASH_DEFAULT)) && default_redefined(c, h) {
        return funcall(c, h, s::default, &[k], std::ptr::null());
    }
    let r = c.hash(h);
    let (p, d) = (r.default_proc, r.default);
    if p != NIL { return call_proc(c, p, &[h, k], std::ptr::null()); }
    Ok(d)
}

#[cold]
fn default_redefined(c: &Ctx, h: Value) -> bool {
    !matches!(c.method_entry(class_of(h), s::default), Some(me) if matches!(me.kind, MKind::Cfunc(f, _) if f as *const () as usize == crate::builtins::hash_methods::h_default as *const () as usize))
}

pub fn hash_aref(c: &mut Ctx, h: Value, k: Value) -> RV {
    let r = c.hash(h);
    let found = if imm_key(k) {
        match r.get_imm(k) { Some(v) => return Ok(v), None => None }
    } else { hash_lookup(c, h, k)? };
    match found {
        Some(i) => Ok(c.hash(h).entries[i].val),
        None => hash_default_value(c, h, k),
    }
}

#[inline]
pub fn hash_aset(c: &mut Ctx, h: Value, k: Value, v: Value) -> Result<(), Esc> {
    if obj_flags(h) & FL_FROZEN != 0 { return Err(raise_frozen(c, h)); }
    let k = if is_string(k) { str_key(c, h, k) } else { k };
    hash_set(c, h, k, v)
}
/// A String key is stored as a frozen copy (unless frozen already, or compared by identity).
#[inline(never)]
fn str_key(c: &mut Ctx, h: Value, k: Value) -> Value {
    if obj_flags(k) & FL_FROZEN != 0 || c.hash(h).identity { return k; }
    let r = c.rstring(k);
    let (b, enc) = (r.b.to_vec(), r.enc);
    let d = c.str_from_vec_enc(b, enc);
    obj_set_flags(d, FL_FROZEN);
    d
}

/// Insert without key dup / frozen check.
pub fn hash_set(c: &mut Ctx, h: Value, k: Value, v: Value) -> Result<(), Esc> {
    let hc = if imm_key(k) {
        // the whole update under one borrow
        let r = c.hash_mut(h);
        if let Some(i) = r.find_imm(k) { r.entries[i].val = v; return Ok(()); }
        mix(k.raw())
    } else {
        let hc = if c.hash(h).identity { identity_hash(k) } else { key_hash(c, k)? };
        if let Some(i) = lookup_with_hash(c, h, k, hc)? { c.hash_mut(h).entries[i].val = v; return Ok(()); }
        hc
    };
    let r = c.hash_mut(h);
    if r.iter_lev != 0 { return err(c, c.vm().c_runtime_error.get(), "can't add a new key into hash during iteration"); }
    if r.insert_new(k, v, hc) % 64 == 0 { c.malloc_increase(64 * 24); }
    Ok(())
}

/// rb_hash_add_new_element: insert k unless present (k hashed once); true if inserted.
pub fn hash_add_new(c: &mut Ctx, h: Value, k: Value, v: Value) -> Result<bool, Esc> {
    let hc = if imm_key(k) {
        if c.hash(h).find_imm(k).is_some() { return Ok(false); }
        mix(k.raw())
    } else {
        let hc = if c.hash(h).identity { identity_hash(k) } else { key_hash(c, k)? };
        if lookup_with_hash(c, h, k, hc)?.is_some() { return Ok(false); }
        hc
    };
    let r = c.hash_mut(h);
    if r.iter_lev != 0 { return err(c, c.vm().c_runtime_error.get(), "can't add a new key into hash during iteration"); }
    if r.insert_new(k, v, hc) % 64 == 0 { c.malloc_increase(64 * 24); }
    Ok(true)
}

/// rb_enum_tally_up: k hashed once, its count + 1 (no key copy); an existing count must be an Integer.
pub fn hash_tally_up(c: &mut Ctx, h: Value, k: Value) -> Result<(), Esc> {
    let (found, hc) = if imm_key(k) { (c.hash(h).find_imm(k), mix(k.raw())) } else {
        let hc = if c.hash(h).identity { identity_hash(k) } else { key_hash(c, k)? };
        (lookup_with_hash(c, h, k, hc)?, hc)
    };
    if let Some(i) = found {
        let n = c.hash(h).entries[i].val;
        // tally_up: FIXNUM_MAX itself fails its Check_Type(T_BIGNUM), as in CRuby
        let at_max = crate::heap::is_bignum(n) && c.big(n).to_i64() == Some(FIXNUM_MAX);
        let n1 = if n.is_fixnum() && n.as_fix() < FIXNUM_MAX { Value::fix(n.as_fix() + 1) } else if crate::heap::is_bignum(n) && !at_max {
            let b = c.big(n).add(&crate::bignum::BigInt::from_i64(1));
            c.big_to_value(b)
        } else {
            return type_err(c, format!("wrong argument type {} (expected Integer)", crate::builtins::kernel::class_name_for_err(c, n)));
        };
        // no Ruby code ran since the lookup: slot i is still k's
        c.hash_mut(h).entries[i].val = n1;
        return Ok(());
    }
    let r = c.hash_mut(h);
    if r.iter_lev != 0 { return err(c, c.vm().c_runtime_error.get(), "can't add a new key into hash during iteration"); }
    if r.insert_new(k, Value::fix(1), hc) % 64 == 0 { c.malloc_increase(64 * 24); }
    Ok(())
}

/// `dst[k] = v` for each entry of `src`, slot by slot: hashing a key runs Ruby code, which may
/// change `src`, so its pairs are never copied out to an unrooted Vec first.
pub fn hash_copy_entries(c: &mut Ctx, dst: Value, src: Value) -> Result<(), Esc> {
    let mut i = 0;
    while let Some((k, v)) = c.hash_entry_at(src, i) {
        if k != UNDEF { hash_aset(c, dst, k, v)?; }
        i += 1;
    }
    Ok(())
}


pub fn hash_delete(c: &mut Ctx, h: Value, k: Value) -> Result<Option<Value>, Esc> {
    match hash_lookup(c, h, k)? {
        Some(i) => {
            let r = c.hash_mut(h);
            let v = r.remove_slot(i);
            if r.live != 0 && r.entries.len() > r.live * 2 + 16 { r.rebuild_index(); }
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

pub fn hash_rehash(c: &mut Ctx, h: Value) -> Result<(), Esc> {
    if c.hash_iterating(h) { return err(c, c.vm().c_runtime_error.get(), "rehash during iteration"); }
    // the pairs move to a Ruby Array: `hash` of a key is Ruby code, and the emptied table roots nothing
    let flat: Vec<Value> = c.hash(h).entries.iter().filter(|e| e.key != UNDEF).flat_map(|e| [e.key, e.val]).collect();
    let keep = c.ary_from_vec(flat);
    c.hash_clear(h);
    let mut i = 0;
    while i + 1 < c.ary_len(keep) {
        let (k, v) = (c.ary_get(keep, i), c.ary_get(keep, i + 1));
        hash_set(c, h, k, v)?;
        i += 2;
    }
    Ok(())
}

pub fn hash_to_a(c: &mut Ctx, h: Value) -> RV {
    let a = c.ary_new_capa(c.hash_len(h));
    let mut i = 0;
    while let Some((k, v)) = c.hash_entry_at(h, i) {
        if k != UNDEF { let p = c.ary_new_from(&[k, v]); c.ary_push(a, p); }
        i += 1;
    }
    Ok(a)
}

/// Implicit conversion to Hash (to_hash).
pub fn check_hash(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if is_hash(v) { return Ok(Some(v)); }
    if v.is_heap() && respond_to(c, v, s::to_hash, true) {
        let r = funcall(c, v, s::to_hash, &[], std::ptr::null())?;
        if is_hash(r) { return Ok(Some(r)); }
        return type_err(c, "can't convert to Hash");
    }
    Ok(None)
}
