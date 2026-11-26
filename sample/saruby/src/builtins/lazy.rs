// Enumerator::Lazy as enumerator.c's lazy_* functions. A chained Lazy is its root Lazy (the first one
// without stages) and a flat Array of stage entries (struct proc_entry); one Cfunc block given to
// root.each runs an element through the stages (lazy_init_yielder) and yields what passes.
// CRuby builds a Generator per chain and yields through `yielder << v`; here the block is called directly.
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::array::*;
use crate::builtins::enum_native::call_method_of;
use crate::builtins::enumerator::{append_args, new_enum, renum, renum_mut, this_mid, REnum, RYielder};
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::symbol::{intern, s, sym_bytes};
use crate::value::*;
use crate::vm::*;

pub const LZ_HIDE: u8 = 1;    // Enumerable#lazy: no ":each" in #inspect (@method false)
pub const LZ_GEN: u8 = 2;     // Lazy.new: `gen` runs per element of recv.each (lazy_init_block_i)
pub const LZ_UNINIT: u8 = 4;  // allocated, not initialized
pub const LZ_DUP: u8 = 8;     // a copy of a chain: CRuby's copy has the Generator and no procs, so it is a root
pub const LZ_ANY: u8 = LZ_HIDE | LZ_GEN | LZ_UNINIT | LZ_DUP;
pub const SIZE_EXACT: u8 = 16; // no size function (to_enum without a block): #size is the size value only

#[derive(Clone, Copy, PartialEq)]
enum Kind { Map, FlatMap, Select, FilterMap, Reject, Grep, GrepIter, GrepV, GrepVIter, Zip, ZipArrays, Take, TakeWhile, Drop, DropWhile, Uniq, UniqIter, Compact, WithIndex }

/// struct proc_entry: the stage's block, its argument (`memo`), and @method / @arguments for #inspect and #size.
#[derive(Clone, Copy)]
struct Entry { proc_: Value, memo: Value, kind: Kind, mid: Sym, args: Value }
impl RData for Entry {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.proc_); m.visit(&mut self.memo); m.visit(&mut self.args); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn entry(c: &Ctx, v: Value) -> Entry { *c.data::<Entry>(v).expect("a lazy stage") }

pub fn is_lazy(c: &Ctx, v: Value) -> bool {
    if !v.is_heap() || c.data::<REnum>(v).is_none() { return false; }
    let k = c.vm().c_lazy.get();
    obj_klass(v) == k || is_kind_of(c, v, k)
}

/// enumerable_lazy: a Lazy over obj.each, sized by obj.size, inspected without ":each".
pub fn enumerable_lazy(c: &mut Ctx, obj: Value) -> Value {
    let k = c.vm().c_lazy.get();
    let l = new_enum(c, k, obj, s::each, &[], TRUE);
    renum_mut(c, l).lz = LZ_HIDE;
    l
}

// ---- running a chain ----------------------------------------------------------------------------

/// The element on its way through the stages (struct MEMO): LAZY_MEMO_PACKED, LAZY_MEMO_BREAK.
struct Memo { value: Value, packed: bool, brk: bool }

// the run state given to root.each: [stages, per-run stage state (yielder's @memo), fiber, live]
const ST_PROCS: usize = 0;
const ST_MEMOS: usize = 1;
const ST_FIB: usize = 2;
const ST_LIVE: usize = 3;

#[inline]
fn pack(c: &mut Ctx, args: &[Value]) -> Value { match args.len() { 0 => NIL, 1 => args[0], _ => c.ary_new_from(args) } }

/// Lazy#each and #next: a chain, or Lazy.new's block per element of its receiver.
pub fn each_with(c: &mut Ctx, e: Value, en: REnum, blk: Blk) -> RV {
    if en.lz & LZ_UNINIT != 0 { return arg_err(c, "uninitialized enumerator"); }
    if blk.is_null() { return Ok(e); }
    if en.procs != NIL { return run_chain(c, en, blk); }
    // Generator#each of Lazy.new: the block gets a Yielder and each element of recv.each(*args)
    let p = block_to_proc(c, blk)?;
    let k = c.vm().c_yielder.get();
    let y = c.alloc(k, Body::Data(Box::new(RYielder { blk: p })));
    let d = c.ary_new_from(&[en.gen, y]);
    let b = Block::cfunc(c, lz_new_i, d);
    let args = c.ary(en.args).to_vec();
    funcall_flags(c, en.recv, s::each, &args, &b, CF_FCALL)?;
    std::hint::black_box(en.args);
    Ok(NIL)
}
fn lz_new_i(c: &mut Ctx, d: Value, args: &[Value], _b: Blk, _o: Blk) -> RV {
    let (g, y) = (c.ary_get(d, 0), c.ary_get(d, 1));
    let mut v = Vec::with_capacity(args.len() + 1);
    v.push(y);
    v.extend_from_slice(args);
    call_proc(c, g, &v, Blk::NONE)?;
    Ok(NIL)
}

/// lazy_precheck: a take(0) anywhere in the chain means nothing is iterated.
fn precheck(c: &Ctx, procs: Value) -> bool {
    (0..c.ary_len(procs)).all(|i| { let e = entry(c, c.ary_get(procs, i)); !(e.kind == Kind::Take && e.memo == Value::fix(0)) })
}
fn stateful(k: Kind) -> bool { matches!(k, Kind::Take | Kind::Drop | Kind::DropWhile | Kind::Uniq | Kind::UniqIter | Kind::Zip | Kind::ZipArrays | Kind::WithIndex) }

fn run_chain(c: &mut Ctx, en: REnum, blk: Blk) -> RV {
    let procs = en.procs;
    if !precheck(c, procs) { return Ok(NIL); }
    let n = c.ary_len(procs);
    let memos = if (0..n).any(|i| stateful(entry(c, c.ary_get(procs, i)).kind)) { c.ary_from_vec(vec![NIL; n]) } else { NIL };
    let fib = crate::fiber::current(c);
    let st = c.ary_new_from(&[procs, memos, fib, TRUE]);
    let b = Block::cfunc_for(c, lz_yielder, st, blk);
    // rb_block_call(root, :each): the root is a Lazy without stages
    let r = funcall(c, en.recv, s::each, &[], &b);
    c.ary_mut(st)[ST_LIVE] = FALSE;
    match r {
        Ok(_) => Ok(NIL),
        Err(x) => { if c.esc == EscKind::Throw && c.esc_tag_v == st { c.esc = EscKind::None; return Ok(NIL); } Err(x) }
    }
}

/// lazy_init_yielder: one element of root.each.
fn lz_yielder(c: &mut Ctx, st: Value, args: &[Value], _b: Blk, out: Blk) -> RV {
    let mut m = Memo { value: pack(c, args), packed: args.len() > 1, brk: false };
    yielder_result(c, st, st, 0, &mut m, out)
}
/// lazy_yielder_result: stages `from`.. on `m`, the survivor to the block (`yielder << v`); a stage's break
/// ends the innermost iteration: `tag` is root.each's run state or a flat_map's inner each.
fn yielder_result(c: &mut Ctx, st: Value, tag: Value, from: usize, m: &mut Memo, out: Blk) -> RV {
    if run_stages(c, st, tag, from, m, out)? { yield1(c, out, m.value)?; }
    if m.brk { return iter_break(c, st, tag); }
    Ok(m.value)
}
/// rb_iter_break; an inner each's tag records the break for its flat_map stage (LAZY_MEMO_BREAK of the shared MEMO).
fn iter_break(c: &mut Ctx, st: Value, tag: Value) -> RV {
    if c.ary_get(st, ST_LIVE) != TRUE { return Err(crate::rt::raise_local_jump(c, "unexpected break", "break", NIL)); }
    if crate::fiber::current(c) != c.ary_get(st, ST_FIB) { return Err(crate::rt::raise_local_jump(c, "break from proc-closure", "break", NIL)); }
    if tag != st { c.ary_mut(tag)[2] = TRUE; }
    c.esc = EscKind::Throw; c.esc_tag_v = tag; c.esc_val = NIL;
    Err(Esc)
}
fn run_stages(c: &mut Ctx, st: Value, tag: Value, from: usize, m: &mut Memo, out: Blk) -> Result<bool, Esc> {
    let procs = c.ary_get(st, ST_PROCS);
    let n = c.ary_len(procs);
    let mut i = from;
    while i < n {
        let e = entry(c, c.ary_get(procs, i));
        if !stage(c, st, tag, i, &e, m, out)? { return Ok(false); }
        i += 1;
    }
    Ok(true)
}

fn memo_raw(c: &Ctx, st: Value, i: usize) -> Value {
    let ms = c.ary_get(st, ST_MEMOS);
    if ms == NIL { NIL } else { c.ary_get(ms, i) }
}
/// rb_ary_entry(memos, i), or the entry's own memo before the first store.
fn memo_get(c: &Ctx, st: Value, i: usize, dflt: Value) -> Value { let v = memo_raw(c, st, i); if v == NIL { dflt } else { v } }
fn memo_set(c: &mut Ctx, st: Value, i: usize, v: Value) { let ms = c.ary_get(st, ST_MEMOS); c.ary_mut(ms)[i] = v; }

/// lazyenum_yield: the block gets the element as one value.
fn call1(c: &mut Ctx, p: Value, v: Value) -> RV { call_proc(c, p, &[v], Blk::NONE) }
/// lazyenum_yield_values: a packed element is spread.
fn yield_values(c: &mut Ctx, p: Value, m: &Memo) -> RV {
    if m.packed { let a = c.ary(m.value).to_vec(); call_proc(c, p, &a, Blk::NONE) } else { call1(c, p, m.value) }
}
fn eqq(c: &mut Ctx, pat: Value, v: Value) -> Result<bool, Esc> { Ok(funcall(c, pat, s::eqq, &[v], Blk::NONE)?.truthy()) }

/// The lazy_*_proc functions: false drops the element (the chain stops for it).
fn stage(c: &mut Ctx, st: Value, tag: Value, i: usize, e: &Entry, m: &mut Memo, out: Blk) -> Result<bool, Esc> {
    match e.kind {
        Kind::Map => { m.value = yield_values(c, e.proc_, m)?; m.packed = false; Ok(true) }
        Kind::Select => Ok(call1(c, e.proc_, m.value)?.truthy()),
        Kind::Reject => Ok(!call1(c, e.proc_, m.value)?.truthy()),
        Kind::FilterMap => {
            let v = yield_values(c, e.proc_, m)?;
            if !v.truthy() { return Ok(false); }
            m.value = v; m.packed = false;
            Ok(true)
        }
        Kind::Grep => eqq(c, e.memo, m.value),
        Kind::GrepV => Ok(!eqq(c, e.memo, m.value)?),
        Kind::GrepIter | Kind::GrepVIter => {
            if eqq(c, e.memo, m.value)? != (e.kind == Kind::GrepIter) { return Ok(false); }
            m.value = call1(c, e.proc_, m.value)?; m.packed = false;
            Ok(true)
        }
        Kind::Compact => Ok(m.value != NIL),
        Kind::TakeWhile => {
            if !yield_values(c, e.proc_, m)?.truthy() { m.brk = true; return Ok(false); }
            Ok(true)
        }
        Kind::DropWhile => {
            if !memo_get(c, st, i, e.memo).truthy() {
                if yield_values(c, e.proc_, m)?.truthy() { return Ok(false); }
                memo_set(c, st, i, TRUE);
            }
            Ok(true)
        }
        Kind::Take => {
            let memo = memo_get(c, st, i, e.memo);
            let remain = num_to_long(c, memo)?.wrapping_sub(1);
            if remain == 0 { m.brk = true; }
            let r = c.int_from_i64(remain);
            memo_set(c, st, i, r);
            Ok(true)
        }
        Kind::Drop => {
            let memo = memo_get(c, st, i, e.memo);
            let remain = num_to_long(c, memo)?;
            if remain > 0 { let r = c.int_from_i64(remain - 1); memo_set(c, st, i, r); return Ok(false); }
            Ok(true)
        }
        Kind::Uniq | Kind::UniqIter => {
            let key = if e.kind == Kind::Uniq { m.value } else { call1(c, e.proc_, m.value)? };
            let mut h = memo_raw(c, st, i);
            if h == NIL { h = c.hash_new(); memo_set(c, st, i, h); }
            crate::hash::hash_add_new(c, h, key, FALSE)
        }
        Kind::WithIndex => {
            let memo = memo_get(c, st, i, e.memo);
            if e.proc_ != NIL { call_proc(c, e.proc_, &[m.value, memo], Blk::NONE)?; m.packed = false; }
            else { m.value = c.ary_new_from(&[m.value, memo]); m.packed = true; }
            let n = num_to_long(c, memo)?;
            let nx = match n.checked_add(1) { Some(x) => c.int_from_i64(x), None => { let b = c.int_from_i64(n); funcall(c, b, s::plus, &[Value::fix(1)], Blk::NONE)? } };
            memo_set(c, st, i, nx);
            Ok(true)
        }
        // lazy_zip_arrays_func / lazy_zip_func keep LAZY_MEMO_PACKED as it was
        Kind::ZipArrays => {
            let memo = memo_raw(c, st, i);
            let count = if memo == NIL { 0 } else { num_to_long(c, memo)? };
            let arrays = e.memo;
            let n = c.ary_len(arrays);
            let row = c.ary_new_capa(n + 1);
            c.ary_push(row, m.value);
            for j in 0..n {
                let a = c.ary_get(arrays, j);
                let x = if (count as usize) < c.ary_len(a) { c.ary_get(a, count as usize) } else { NIL };
                c.ary_push(row, x);
            }
            m.value = row;
            memo_set(c, st, i, Value::fix(count + 1));
            Ok(true)
        }
        Kind::Zip => {
            let mut its = memo_raw(c, st, i);
            if its == NIL {
                let args = e.memo;
                its = c.ary_new();
                for j in 0..c.ary_len(args) { let a = c.ary_get(args, j); let it = funcall(c, a, intern(b"to_enum"), &[], Blk::NONE)?; c.ary_push(its, it); }
                memo_set(c, st, i, its);
            }
            let row = c.ary_new();
            c.ary_push(row, m.value);
            for j in 0..c.ary_len(its) { let it = c.ary_get(its, j); let v = next_or_nil(c, it)?; c.ary_push(row, v); }
            m.value = row;
            Ok(true)
        }
        Kind::FlatMap => flat_map(c, st, tag, i, e, m, out),
    }
}
/// call_next rescued by next_stopped: nil once the enumerator has ended.
fn next_or_nil(c: &mut Ctx, it: Value) -> RV {
    match funcall(c, it, intern(b"next"), &[], Blk::NONE) {
        Ok(v) => Ok(v),
        Err(x) => {
            if c.esc == EscKind::Raise && is_kind_of(c, c.esc_val, c.vm().c_stop_iteration.get()) { c.esc = EscKind::None; return Ok(NIL); }
            Err(x)
        }
    }
}
/// lazy_flat_map_proc: an Array (or to_ary) is spread, a lazy-like value (each and force) iterated; the
/// elements before the last go through the following stages here, the last one goes on as this element.
fn flat_map(c: &mut Ctx, st: Value, tag: Value, i: usize, e: &Entry, m: &mut Memo, out: Blk) -> Result<bool, Esc> {
    let v = yield_values(c, e.proc_, m)?;
    let from = i + 1;
    let brk = m.brk;
    let ary = if is_array(v) { Some(v) } else if respond_to(c, v, intern(b"force"), false) && respond_to(c, v, s::each, false) {
        m.brk = false;
        let d = c.ary_new_from(&[st, Value::fix(from as i64), FALSE]);
        let b = Block::cfunc_for(c, flat_map_i, d, out);
        if let Err(x) = funcall(c, v, s::each, &[], &b) {
            if !(c.esc == EscKind::Throw && c.esc_tag_v == d) { return Err(x); }
            c.esc = EscKind::None;
        }
        if brk || c.ary_get(d, 2) == TRUE { m.brk = true; }
        return Ok(false);
    } else { check_array(c, v)? };
    let Some(a) = ary else { m.value = v; m.packed = false; return Ok(true) };
    m.brk = false;
    let mut j = 0;
    while j + 1 < c.ary_len(a) {
        m.value = c.ary_get(a, j); m.packed = false;
        yielder_result(c, st, tag, from, m, out)?;
        j += 1;
    }
    if brk { m.brk = true; }
    if j >= c.ary_len(a) { return Ok(false); }
    m.value = c.ary_get(a, j); m.packed = false;
    Ok(true)
}
/// lazy_flat_map_i: an element of the inner each, through the stages after the flat_map ([st, from, broke]).
fn flat_map_i(c: &mut Ctx, d: Value, args: &[Value], _b: Blk, out: Blk) -> RV {
    let st = c.ary_get(d, 0);
    let from = c.ary_get(d, 1).as_fix() as usize;
    let mut m = Memo { value: pack(c, args), packed: args.len() > 1, brk: false };
    yielder_result(c, st, d, from, &mut m, out)
}

// ---- size, inspect, dup ---------------------------------------------------------------------------

/// enumerator_size of a chain: the root's size through each stage's size function, nil at one without.
pub fn size(c: &mut Ctx, _slf: Value, en: REnum) -> RV {
    if en.lz & LZ_UNINIT != 0 { return arg_err(c, "uninitialized enumerator"); }
    let root = en.recv;
    let mut r = if respond_to(c, root, s::size, true) { funcall(c, root, s::size, &[], Blk::NONE)? } else { NIL };
    let procs = en.procs;
    for i in 0..c.ary_len(procs) {
        let e = entry(c, c.ary_get(procs, i));
        match e.kind {
            Kind::Map | Kind::WithIndex | Kind::Zip | Kind::ZipArrays => {}
            Kind::Take => {
                let len = num_to_long(c, c.ary_get(e.args, 0))?;
                if !(r == NIL || (r.is_fixnum() && r.as_fix() < len)) { r = c.int_from_i64(len); }
            }
            Kind::Drop => {
                let len = num_to_long(c, c.ary_get(e.args, 0))?;
                if r == NIL {
                } else if r.is_fixnum() { r = Value::fix((r.as_fix() - len).max(0)); }
                else { let l = c.int_from_i64(len); r = funcall(c, r, s::minus, &[l], Blk::NONE)?; }
            }
            _ => return Ok(NIL),
        }
    }
    Ok(r)
}

/// inspect_enumerator for a Lazy: a chain prints its root, then one "#<Lazy: ...:stage(args)>" per stage.
pub fn inspect(c: &mut Ctx, slf: Value, en: REnum) -> RV {
    let cls = class_path(c, real_class_of(c, slf));
    if en.lz & LZ_UNINIT != 0 { return Ok(c.str_new(format!("#<{}: uninitialized>", cls).as_bytes())); }
    if !rec_enter(c, slf, s::inspect)? { return Ok(c.str_new(format!("#<{}: ...>", cls).as_bytes())); }
    let r = inspect_body(c, slf, en, &cls);
    rec_leave(c, slf, s::inspect);
    r
}
fn inspect_body(c: &mut Ctx, slf: Value, en: REnum, cls: &str) -> RV {
    let ri = inspect_value(c, en.recv)?;
    let mut out = Vec::new();
    if en.lz & LZ_DUP != 0 {
        // the copy's receiver is the chain's Generator
        out.extend_from_slice(format!("#<{}: #<Enumerator::Generator:{:#018x}>:each", cls, en.procs.raw()).as_bytes());
        append_args(c, &mut out, en.args)?;
        out.push(b'>');
        return Ok(c.str_new(&out));
    }
    if en.procs == NIL || real_class_of(c, en.recv) != real_class_of(c, slf) {
        out.extend_from_slice(format!("#<{}: ", cls).as_bytes());
        out.extend_from_slice(c.str_bytes(ri));
        if en.procs == NIL {
            if en.lz & LZ_HIDE == 0 { out.push(b':'); out.extend_from_slice(sym_bytes(en.mid)); }
            append_args(c, &mut out, en.args)?;
        }
        out.push(b'>');
    } else {
        out.extend_from_slice(c.str_bytes(ri));
    }
    let procs = en.procs;
    for i in 0..if procs == NIL { 0 } else { c.ary_len(procs) } {
        let e = entry(c, c.ary_get(procs, i));
        let mut o = format!("#<{}: ", cls).into_bytes();
        o.append(&mut out);
        o.push(b':');
        o.extend_from_slice(sym_bytes(e.mid));
        if e.args != FALSE { append_args(c, &mut o, e.args)?; }
        o.push(b'>');
        out = o;
    }
    Ok(c.str_new(&out))
}

/// enumerator_each with arguments: a copy (rb_obj_dup) with them appended and no size.
pub fn dup_with_args(c: &mut Ctx, slf: Value, a: &[Value]) -> RV {
    let Some(d) = crate::builtins::enumerator::dup_data(c, slf)? else { return type_err(c, "not an Enumerator") };
    let en = renum(c, d);
    let mut args = c.ary(en.args).to_vec();
    args.extend_from_slice(a);
    let na = c.ary_from_vec(args);
    let m = renum_mut(c, d);
    m.args = na; m.size = NIL; m.lz |= SIZE_EXACT;
    Ok(d)
}

// ---- Enumerator::Lazy's methods -----------------------------------------------------------------

/// lazy_add_method: a new Lazy of the same root with one more stage.
fn add(c: &mut Ctx, slf: Value, kind: Kind, blk: Blk, memo: Value, args: Value, name: &str) -> RV {
    let mid = this_mid(c, intern(name.as_bytes()));
    let proc_ = block_to_proc(c, blk)?;
    let en = renum(c, slf);
    if en.lz & LZ_UNINIT != 0 { return arg_err(c, "uninitialized enumerator"); }
    // enumerator_init_copy(enumerator_allocate(rb_cLazy), obj)
    let k = c.vm().c_lazy.get();
    if real_class_of(c, slf) != k { return type_err(c, "initialize_copy should take same class object"); }
    if crate::fiber::enum_fiber_live(c, slf) { return type_err(c, "can't copy execution context"); }
    let (root, procs) = if en.procs != NIL && en.lz & LZ_DUP == 0 { let p = c.ary(en.procs).to_vec(); (en.recv, c.ary_from_vec(p)) } else { (slf, c.ary_new()) };
    let ok = c.vm().c_object.get();
    let ent = c.alloc(ok, Body::Data(Box::new(Entry { proc_, memo, kind, mid, args })));
    c.ary_push(procs, ent);
    let l = new_enum(c, k, root, s::each, &[], NIL);
    renum_mut(c, l).procs = procs;
    Ok(l)
}
macro_rules! need_block {
    ($c:expr, $b:expr, $name:literal) => { if $b.is_null() { return arg_err($c, concat!("tried to call lazy ", $name, " without a block")); } };
}
fn lazy_map(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV { need_block!(c, blk, "map"); add(c, slf, Kind::Map, blk, NIL, FALSE, "map") }
fn lazy_flat_map(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV { need_block!(c, blk, "flat_map"); add(c, slf, Kind::FlatMap, blk, NIL, FALSE, "flat_map") }
fn lazy_select(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV { need_block!(c, blk, "select"); add(c, slf, Kind::Select, blk, NIL, FALSE, "select") }
fn lazy_filter_map(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV { need_block!(c, blk, "filter_map"); add(c, slf, Kind::FilterMap, blk, NIL, FALSE, "filter_map") }
fn lazy_reject(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV { need_block!(c, blk, "reject"); add(c, slf, Kind::Reject, blk, NIL, FALSE, "reject") }
fn lazy_take_while(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV { need_block!(c, blk, "take_while"); add(c, slf, Kind::TakeWhile, blk, NIL, FALSE, "take_while") }
fn lazy_drop_while(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV { need_block!(c, blk, "drop_while"); add(c, slf, Kind::DropWhile, blk, FALSE, FALSE, "drop_while") }
fn lazy_grep(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let args = c.ary_new_from(&a[..1]);
    add(c, slf, if blk.is_null() { Kind::Grep } else { Kind::GrepIter }, blk, a[0], args, "grep")
}
fn lazy_grep_v(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let args = c.ary_new_from(&a[..1]);
    add(c, slf, if blk.is_null() { Kind::GrepV } else { Kind::GrepVIter }, blk, a[0], args, "grep_v")
}
fn lazy_uniq(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV { add(c, slf, if blk.is_null() { Kind::Uniq } else { Kind::UniqIter }, blk, NIL, FALSE, "uniq") }
fn lazy_compact(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV { add(c, slf, Kind::Compact, blk, NIL, FALSE, "compact") }
fn lazy_take(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let len = num_to_long(c, a[0])?;
    if len < 0 { return arg_err(c, "attempt to take negative size"); }
    let n = c.int_from_i64(len);   // no more conversion
    let args = c.ary_new_from(&[n]);
    add(c, slf, Kind::Take, blk, n, args, "take")
}
fn lazy_drop(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let len = num_to_long(c, a[0])?;
    if len < 0 { return arg_err(c, "attempt to drop negative size"); }
    let args = c.ary_new_from(&a[..1]);
    add(c, slf, Kind::Drop, blk, a[0], args, "drop")
}
fn lazy_with_index(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let memo = match a.first() { Some(&m) if m != NIL => m, _ => Value::fix(0) };
    let args = c.ary_new_from(&[memo]);
    add(c, slf, Kind::WithIndex, blk, memo, args, "with_index")
}
/// lazy_each_with_index (4.0.10): with_index(0) without a block, the eager super with one.
fn lazy_each_with_index(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV {
    if !blk.is_null() { return lazy_super_call(c, slf, intern(b"each_with_index"), &[], blk); }
    let args = c.ary_new_from(&[Value::fix(0)]);
    add(c, slf, Kind::WithIndex, blk, Value::fix(0), args, "each_with_index")
}
fn lazy_zip(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if !blk.is_null() { return lazy_super_call(c, slf, intern(b"zip"), a, blk); }
    let ary = c.ary_new_capa(a.len());
    for (i, &x) in a.iter().enumerate() {
        match check_array(c, x)? {
            Some(v) => c.ary_push(ary, v),
            None => {
                for &y in &a[i..] {
                    if !respond_to(c, y, s::each, false) {
                        let cls = class_path(c, real_class_of(c, y));
                        return type_err(c, format!("wrong argument type {} (must respond to :each)", cls));
                    }
                }
                let all = c.ary_new_from(a);
                return add(c, slf, Kind::Zip, blk, all, all, "zip");
            }
        }
    }
    add(c, slf, Kind::ZipArrays, blk, ary, ary, "zip")
}
fn lazy_lazy(_c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(slf) }
fn lazy_eager(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let k = c.vm().c_enumerator.get();
    Ok(new_enum(c, k, slf, s::each, &[], TRUE))
}
/// lazy_use_super_method: to_enum(:map) and co. iterate with Enumerable's eager method.
const SUPER_NAMES: [&str; 19] = ["map", "collect", "flat_map", "collect_concat", "select", "find_all", "filter", "filter_map", "reject",
                                 "grep", "grep_v", "zip", "take", "take_while", "drop", "drop_while", "uniq", "with_index", "each_with_index"];
fn lazy_to_enum(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let mut meth = if a.is_empty() { s::each } else { crate::builtins::object::to_sym_arg(c, a[0])? };
    if SUPER_NAMES.iter().any(|n| n.as_bytes() == sym_bytes(meth)) { meth = intern(format!("_enumerable_{}", crate::symbol::sym_str(meth)).as_bytes()); }
    let size = block_to_proc(c, blk)?;
    let k = c.vm().c_lazy.get();
    let l = new_enum(c, k, slf, meth, if a.is_empty() { &[] } else { &a[1..] }, size);
    renum_mut(c, l).lz = SIZE_EXACT;
    Ok(l)
}
fn lazy_initialize(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    need_block!(c, blk, "new");
    let g = block_to_proc(c, blk)?;
    if obj_flags(slf) & FL_FROZEN != 0 { return Err(raise_frozen(c, slf)); }
    let size = a.get(1).copied().unwrap_or(NIL);
    let none = c.ary_new();
    let Some(en) = c.data_mut::<REnum>(slf) else { return type_err(c, "wrong argument type (expected Enumerator)") };
    en.recv = a[0]; en.mid = s::each; en.args = none; en.size = size; en.gen = g; en.lz = LZ_GEN; en.kw = false;
    en.procs = NIL; en.buf = NIL; en.pos = 0; en.fiber = NIL; en.arith = None;
    Ok(slf)
}
/// rb_call_super from a Lazy method: the method of Lazy's superclass chain (Enumerator, Enumerable).
fn lazy_super_call(c: &mut Ctx, slf: Value, mid: Sym, a: &[Value], blk: Blk) -> RV {
    let k = c.vm().c_enumerator.get();
    call_method_of(c, k, slf, mid, a, blk)
}
/// lazy_super: chunk, slice_*, chunk_while are Enumerable's, made lazy.
fn lazy_super(c: &mut Ctx, slf: Value, name: &str, a: &[Value], blk: Blk) -> RV {
    let r = lazy_super_call(c, slf, intern(name.as_bytes()), a, blk)?;
    Ok(enumerable_lazy(c, r))
}

fn en_lazy(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(enumerable_lazy(c, slf)) }
/// Enumerable.__chain(enums): new_enum_chain, lazy when one of them is.
fn en_chain(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let enums = c.ary(a[0]).to_vec();
    // decided before the call: the Vec copy is stale after it (docs/gc_moving.md 2.4)
    let lazy = enums.iter().any(|&e| is_lazy(c, e));
    let k = c.vm().c_chain.get();
    let ch = funcall(c, k, s::new, &enums, Blk::NONE)?;
    if lazy { return Ok(enumerable_lazy(c, ch)); }
    Ok(ch)
}

pub fn init(c: &mut Ctx, k: Value, enumerable: Value) {
    define_cfunc(c, enumerable, "lazy", en_lazy, 0);
    define_singleton_cfunc(c, c.vm().m_internal.get(), "__chain", en_chain, 1);
    macro_rules! enumerable_alias {
        ($($n:literal),*) => { $( define_private_cfunc(c, k, concat!("_enumerable_", $n), |c, s, a, b| { let m = c.vm().m_enumerable.get(); call_method_of(c, m, s, intern($n.as_bytes()), a, b) }, -1); )* };
    }
    enumerable_alias!("map", "collect", "flat_map", "collect_concat", "select", "find_all", "filter", "filter_map", "reject",
                      "grep", "grep_v", "zip", "take", "take_while", "drop", "drop_while", "uniq");
    define_private_cfunc(c, k, "_enumerable_with_index", |c, s, a, b| { let e = c.vm().c_enumerator.get(); call_method_of(c, e, s, intern(b"with_index"), a, b) }, -1);
    define_private_cfunc(c, k, "_enumerable_each_with_index", |c, s, a, b| { let e = c.vm().c_enumerator.get(); call_method_of(c, e, s, intern(b"each_with_index"), a, b) }, -1);
    define_private_cfunc(c, k, "initialize", lazy_initialize, -1);
    define_cfunc(c, k, "to_enum", lazy_to_enum, -1);
    define_cfunc(c, k, "enum_for", lazy_to_enum, -1);
    define_cfunc(c, k, "eager", lazy_eager, 0);
    define_cfunc(c, k, "map", lazy_map, 0);
    define_cfunc(c, k, "collect", lazy_map, 0);
    define_cfunc(c, k, "flat_map", lazy_flat_map, 0);
    define_cfunc(c, k, "collect_concat", lazy_flat_map, 0);
    define_cfunc(c, k, "select", lazy_select, 0);
    define_cfunc(c, k, "find_all", lazy_select, 0);
    define_cfunc(c, k, "filter", lazy_select, 0);
    define_cfunc(c, k, "filter_map", lazy_filter_map, 0);
    define_cfunc(c, k, "reject", lazy_reject, 0);
    define_cfunc(c, k, "grep", lazy_grep, 1);
    define_cfunc(c, k, "grep_v", lazy_grep_v, 1);
    define_cfunc(c, k, "zip", lazy_zip, -1);
    define_cfunc(c, k, "take", lazy_take, 1);
    define_cfunc(c, k, "take_while", lazy_take_while, 0);
    define_cfunc(c, k, "drop", lazy_drop, 1);
    define_cfunc(c, k, "drop_while", lazy_drop_while, 0);
    define_cfunc(c, k, "lazy", lazy_lazy, 0);
    define_cfunc(c, k, "chunk", |c, s, a, b| lazy_super(c, s, "chunk", a, b), -1);
    define_cfunc(c, k, "slice_before", |c, s, a, b| lazy_super(c, s, "slice_before", a, b), -1);
    define_cfunc(c, k, "slice_after", |c, s, a, b| lazy_super(c, s, "slice_after", a, b), -1);
    define_cfunc(c, k, "slice_when", |c, s, a, b| lazy_super(c, s, "slice_when", a, b), -1);
    define_cfunc(c, k, "chunk_while", |c, s, a, b| lazy_super(c, s, "chunk_while", a, b), -1);
    define_cfunc(c, k, "uniq", lazy_uniq, 0);
    define_cfunc(c, k, "compact", lazy_compact, 0);
    define_cfunc(c, k, "with_index", lazy_with_index, -1);
    define_cfunc(c, k, "each_with_index", lazy_each_with_index, 0);
    let _ = crate::builtins::object::alias_method(c, k, intern(b"force"), intern(b"to_a"));
}
