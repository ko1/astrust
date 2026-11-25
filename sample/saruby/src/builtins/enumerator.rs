// Enumerator / Enumerator::Yielder / ArithmeticSequence; Enumerator::Lazy's own methods are in lazy.rs.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::array::*;
use crate::builtins::kernel::*;
use crate::symbol::{intern, s, sym_str};
use crate::value::*;
use crate::vm::*;

#[derive(Clone, Copy)]
pub struct REnum {
    pub recv: Value, pub mid: Sym, pub args: Value, pub size: Value, pub gen: Value,
    pub buf: Value, pub pos: usize, pub fiber: Value,
    pub arith: Option<(Value, Value, Value, bool)>,
    pub lz: u8,       // LZ_* (lazy.rs)
    pub kw: bool,     // the last of args was given as keywords
    pub procs: Value, // a chained Lazy: its stage entries (lazy.rs); recv is the root Lazy
    pub look: Value,  // #peek's lookahead (UNDEF: none)
    pub feed: Value,  // #feed's value (UNDEF: none)
    pub stop: Value,  // the StopIteration of a finished #next iteration (nil: none)
    pub init: bool,   // initialized (allocate leaves it false)
}
impl RData for REnum {
    fn visit(&mut self, m: &mut crate::gc::Visitor) {
        m.visit(&mut self.recv); m.visit(&mut self.args); m.visit(&mut self.size); m.visit(&mut self.gen); m.visit(&mut self.buf); m.visit(&mut self.fiber); m.visit(&mut self.procs);
        m.visit(&mut self.look); m.visit(&mut self.feed); m.visit(&mut self.stop);
        if let Some((a, b, c, _)) = &mut self.arith { m.visit(a); m.visit(b); m.visit(c); }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
pub struct RYielder { pub blk: Value }
impl RData for RYielder {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.blk); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// The state of Enumerator `v`, copied out (every field is Copy): nothing is borrowed across the calls that follow.
pub(crate) fn renum(c: &Ctx, v: Value) -> REnum { *c.data::<REnum>(v).expect("an Enumerator") }
pub(crate) fn renum_mut(c: &mut Ctx, v: Value) -> &mut REnum { c.data_mut::<REnum>(v).expect("an Enumerator") }

pub(crate) fn new_enum(c: &mut Ctx, klass: Value, recv: Value, mid: Sym, args: &[Value], size: Value) -> Value {
    let kw = kw_arg(c, args).is_some();
    let a = c.ary_new_from(args);
    c.alloc(klass, Body::Data(Box::new(REnum { recv, mid, args: a, size, gen: NIL, buf: NIL, pos: 0, fiber: NIL, arith: None, lz: 0, kw, procs: NIL, look: UNDEF, feed: UNDEF, stop: NIL, init: true })))
}

/// rb_enumeratorize_with_size_kw: an enumerator of a Lazy is a Lazy.
fn enum_class_for(c: &Ctx, recv: Value) -> Value {
    if crate::builtins::lazy::is_lazy(c, recv) { c.vm().c_lazy.get() } else { c.vm().c_enumerator.get() }
}
pub fn enum_for(c: &mut Ctx, recv: Value, mid: Sym, args: &[Value], size_fn: Value) -> RV {
    let k = enum_class_for(c, recv);
    Ok(new_enum(c, k, recv, mid, args, size_fn))
}
/// An enumerator whose size is the receiver's size.
pub fn enum_for_sized(c: &mut Ctx, recv: Value, mid: Sym, args: &[Value]) -> RV {
    let k = enum_class_for(c, recv);
    Ok(new_enum(c, k, recv, mid, args, TRUE))
}
pub fn enum_from_array(c: &mut Ctx, recv: Value, mid: Sym, args: &[Value], arr: Value) -> RV {
    let k = c.vm().c_enumerator.get();
    let e = new_enum(c, k, recv, mid, args, NIL);
    let none = c.ary_new();
    let en = renum_mut(c, e);
    en.buf = arr; en.recv = arr; en.mid = s::each; en.args = none;
    Ok(e)
}

pub fn arith_seq(c: &mut Ctx, b: Value, e: Value, st: Value, excl: bool, mid: Sym) -> RV {
    let k = c.vm().c_arith_seq.get();
    let v = new_enum(c, k, b, mid, &[], NIL);
    renum_mut(c, v).arith = Some((b, e, st, excl));
    Ok(v)
}
/// ArithmeticSequence made by Range#step / Range#%: inspects as "((range).mid(args))".
pub fn arith_seq_from(c: &mut Ctx, range: Value, b: Value, e: Value, st: Value, excl: bool, mid: Sym, args: &[Value]) -> RV {
    let k = c.vm().c_arith_seq.get();
    let v = new_enum(c, k, range, mid, args, NIL);
    renum_mut(c, v).arith = Some((b, e, st, excl));
    Ok(v)
}
/// Enumerable#to_a of an ArithmeticSequence whose #each is the builtin: collected without the generic each protocol.
pub fn arith_to_a_direct(c: &mut Ctx, slf: Value) -> Option<RV> {
    if is_arith_seq(c, slf) && !crate::builtins::enum_native::each_overridden(c, slf, e_each) { Some(collect_all(c, slf)) } else { None }
}
pub fn is_arith_seq(c: &Ctx, v: Value) -> bool { c.data::<REnum>(v).map(|e| e.arith.is_some()).unwrap_or(false) }
pub fn arith_seq_slice(c: &mut Ctx, seq: Value, ary: Value) -> RV {
    use crate::builtins::numeric_methods::num_to_long;
    let (mut b, mut e, st, mut excl) = renum(c, seq).arith.unwrap();
    let alen = c.ary_len(ary) as i64;
    let step = if st == NIL { 1 } else { num_to_long(c, st)? };
    if step < 0 {
        if excl && e != NIL { let e1 = num_to_long(c, e)?.saturating_add(1); e = c.int_from_i64(e1); excl = false; }
        std::mem::swap(&mut b, &mut e);
    }
    let oor = |c: &mut Ctx| -> RV { let s2 = inspect_str(c, seq)?; err(c, c.vm().c_range_error.get(), format!("{} out of range", s2)) };
    // rb_range_component_beg_len
    let mut beg = if b == NIL { 0 } else { num_to_long(c, b)? };
    let mut end = if e == NIL { -1 } else { num_to_long(c, e)? };
    let ex = if e == NIL { false } else { excl };
    let strict = step < -1 || step > 1;
    if beg < 0 { beg += alen; if beg < 0 { return if strict { oor(c) } else { Ok(NIL) }; } }
    if end < 0 { end += alen; }
    if !ex { end = end.saturating_add(1); }
    if !strict { if beg > alen { return Ok(NIL); } if end > alen { end = alen; } }
    let mut len = (end - beg).max(0);
    if strict && (beg > alen || len > alen) { return oor(c); }
    // rb_ary_subseq_step
    if beg > alen || beg < 0 || len < 0 { return Ok(NIL); }
    if alen < len || alen < beg + len { len = alen - beg; }
    let out = c.ary_new();
    if len == 0 { return Ok(out); }
    if step == 0 { return arg_err(c, "slice step cannot be zero"); }
    if step == 1 { for i in beg..beg + len { let x = c.ary_get(ary, i as usize); c.ary_push(out, x); } return Ok(out); }
    let orig_len = len;
    let mut step = step;
    if step > 0 && step >= len { let x = c.ary_get(ary, beg as usize); c.ary_push(out, x); return Ok(out); }
    if step < 0 && step < -len { step = -len; }
    let ustep = step.abs();
    let n = (len + ustep - 1) / ustep;
    let mut j = beg + if step > 0 { 0 } else { orig_len - 1 };
    for _ in 0..n { let x = c.ary_get(ary, j as usize); c.ary_push(out, x); j += step; }
    Ok(out)
}

/// Run the enumerator, feeding each yielded value to blk.
pub fn enum_each_with(c: &mut Ctx, e: Value, blk: Blk) -> RV {
    let en = renum(c, e);
    if en.procs != NIL || en.lz & (crate::builtins::lazy::LZ_GEN | crate::builtins::lazy::LZ_UNINIT) != 0 {
        return crate::builtins::lazy::each_with(c, e, en, blk);
    }
    if let Some((b, en_, st, excl)) = en.arith {
        if b.is_fixnum() && en_.is_fixnum() && st.is_fixnum() && crate::rt::redef(crate::rt::R_INT_EQ | crate::rt::R_INT_LT | crate::rt::R_INT_GE | crate::rt::R_INT_COERCE) {
            arith_each_calls(c, b, en_, st, excl, blk)?;
            return Ok(e);
        }
        let r = c.range_new(b, en_, excl);
        let bp = block_to_proc(c, blk)?;
        let fwd = Block::cfunc(c, |c, d, args, _b, _o| { let bb = c.proc_data(d).blk; invoke_block_full(c, &bb, args, std::ptr::null(), bb.self_, false, std::ptr::null(), false) }, bp);
        if en.mid == s::step && !en.recv.is_heap() || (en.mid == s::step && crate::numeric::is_numeric(en.recv)) {
            return funcall(c, b, s::step, &[en_, st], &fwd);
        }
        return funcall(c, r, s::step, &[st], &fwd);
    }
    if en.gen != NIL {
        let p = block_to_proc(c, blk)?;
        let k = c.vm().c_yielder.get();
        let y = c.alloc(k, Body::Data(Box::new(RYielder { blk: p })));
        return call_proc(c, en.gen, &[y], std::ptr::null());
    }
    if en.recv == NIL && en.mid == s::each && c.ary_len(en.args) == 0 {
        let user = matches!(c.method_entry(class_of(e), s::each), Some(me) if !matches!(me.kind, MKind::Cfunc(..)));
        if user { return funcall(c, e, s::each, &[], blk); }
    }
    // `en.args` stays on this frame and roots the copies while the call runs
    let args = c.ary(en.args).to_vec();
    // as rb_block_call: private methods too, and a keyword hash stays keywords
    let kw = CF_FCALL | if en.kw { CF_KW } else { 0 };
    let r = funcall_flags(c, en.recv, en.mid, &args, blk, kw);
    std::hint::black_box(en.args);
    r
}

/// enumerator.c arith_seq_each on Integers once ==, <, >= or coerce is redefined: the calls it makes.
fn arith_each_calls(c: &mut Ctx, b: Value, e: Value, st: Value, x: bool, blk: Blk) -> RV {
    let eq0 = |c: &mut Ctx, v: Value| -> Result<bool, Esc> { Ok(v == Value::fix(0) || (crate::rt::redef(crate::rt::R_INT_EQ) && funcall(c, v, s::eq, &[Value::fix(0)], std::ptr::null())?.truthy())) };
    if eq0(c, st)? { loop { yield1(c, blk, b)?; } }
    let (bi, ei, si) = (b.as_fix() as i128, e.as_fix() as i128, st.as_fix() as i128);
    let d = ei - bi;
    let len_1 = d / si - if d % si != 0 && ((d < 0) != (si < 0)) { 1 } else { 0 };
    let mut last = c.int_from_i64((bi + si * len_1) as i64);
    if x && (last == e || (crate::rt::redef(crate::rt::R_INT_EQ) && funcall(c, last, s::eq, &[e], std::ptr::null())?.truthy())) {
        last = c.int_from_i64((bi + si * (len_1 - 1)) as i64);
    }
    let neg = if crate::rt::redef(crate::rt::R_INT_LT) { funcall(c, st, s::lt, &[Value::fix(0)], std::ptr::null())?.truthy() } else { si < 0 };
    let mut cur = b;
    loop {
        let (p, q) = if neg { (cur, last) } else { (last, cur) };
        // NUM_GE: rb_num_coerce_relop asks q.coerce(p), then [0] >= [1]
        let ary = funcall(c, q, s::coerce, &[p], std::ptr::null())?;
        if !is_array(ary) || c.ary_len(ary) != 2 { return type_err(c, "coerce must return [x, y]"); }
        let (p2, q2) = (c.ary_get(ary, 0), c.ary_get(ary, 1));
        if !funcall(c, p2, s::ge, &[q2], std::ptr::null())?.truthy() { break; }
        yield1(c, blk, cur)?;
        cur = if cur.is_fixnum() { c.int_from_i64(cur.as_fix() + si as i64) } else { funcall(c, cur, s::plus, &[st], std::ptr::null())? };
    }
    Ok(NIL)
}

fn e_each(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if !a.is_empty() && is_arith_seq(c, slf) { return argc_err(c, a.len(), 0, 0); }   // arith_seq_each takes none
    if !a.is_empty() && crate::builtins::lazy::is_lazy(c, slf) {
        // enumerator_each: a dup with the arguments appended, then that is iterated
        let d = crate::builtins::lazy::dup_with_args(c, slf, a)?;
        return if blk.is_null() { Ok(d) } else { enum_each_with(c, d, blk) };
    }
    if blk.is_null() {
        if a.is_empty() { return Ok(slf); }
        let en = renum(c, slf);
        let mut args = c.ary(en.args).to_vec(); args.extend_from_slice(a);
        let k = c.vm().c_enumerator.get();
        return Ok(new_enum(c, k, en.recv, en.mid, &args, NIL));
    }
    if !a.is_empty() {
        let en = renum(c, slf);
        let mut args = c.ary(en.args).to_vec(); args.extend_from_slice(a);
        return funcall_public(c, en.recv, en.mid, &args, blk);
    }
    let r = enum_each_with(c, slf, blk)?;
    Ok(if renum(c, slf).arith.is_some() { slf } else { r })
}

fn e_with_index(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    if blk.is_null() { return enum_for_sized(c, slf, intern(b"with_index"), a); }
    let offv = if a.is_empty() || a[0] == NIL { Value::fix(0) } else { crate::builtins::numeric_methods::to_integer(c, a[0])? };
    let bp = block_to_proc(c, blk)?;
    let data = c.ary_new_from(&[offv, bp]);
    let b = Block::cfunc(c, |c, d, args, _b, _o| {
        let i = c.ary_get(d, 0); let p = c.ary_get(d, 1);
        let next = if i.is_fixnum() && Value::fixable(i.as_fix() + 1) { Value::fix(i.as_fix() + 1) } else { funcall(c, i, s::plus, &[Value::fix(1)], std::ptr::null())? };
        c.ary_mut(d)[0] = next;
        let v = match args.len() { 0 => NIL, 1 => args[0], _ => c.ary_new_from(args) };
        call_proc(c, p, &[v, i], std::ptr::null())
    }, data);
    enum_each_with(c, slf, Blk::of(&b))
}
fn e_each_with_index(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return enum_for_sized(c, slf, intern(b"each_with_index"), &[]); }
    e_with_index(c, slf, &[], blk)
}
fn e_with_object(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { let mid = this_mid(c, intern(b"with_object")); return enum_for_sized(c, slf, mid, a); }
    let memo = a[0];
    let bp = block_to_proc(c, blk)?;
    let data = c.ary_new_from(&[memo, bp]);
    let b = Block::cfunc(c, |c, d, args, _b, _o| {
        let v = match args.len() { 0 => NIL, 1 => args[0], _ => c.ary_new_from(args) };
        let (p, memo) = (c.ary_get(d, 1), c.ary_get(d, 0));
        call_proc(c, p, &[v, memo], std::ptr::null())
    }, data);
    enum_each_with(c, slf, Blk::of(&b))?;
    Ok(memo)
}
fn collect_all(c: &mut Ctx, slf: Value) -> RV {
    let out = c.ary_new();
    let b = Block::cfunc(c, |c, d, args, _b, _o| { let v = match args.len() { 0 => NIL, 1 => args[0], _ => c.ary_new_from(args) }; c.ary_push(d, v); Ok(NIL) }, out);
    enum_each_with(c, slf, Blk::of(&b))?;
    Ok(out)
}
/// ary2sv: the values #next / #peek return for the packed Array vs.
fn ary2sv(c: &mut Ctx, vs: Value, dup: bool) -> Value {
    if !is_array(vs) { return vs; }
    match c.ary_len(vs) { 0 => NIL, 1 => c.ary_get(vs, 0), _ => if dup { let v = c.ary(vs).to_vec(); c.ary_from_vec(v) } else { vs } }
}
fn e_next_values(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if obj_flags(slf) & FL_FROZEN != 0 { return Err(raise_frozen(c, slf)); }
    let l = renum(c, slf).look;
    if l != UNDEF { renum_mut(c, slf).look = UNDEF; return Ok(l); }
    crate::fiber::enum_next_values(c, slf)
}
fn e_next(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { let vs = e_next_values(c, slf, a, b)?; Ok(ary2sv(c, vs, false)) }
fn peek_values(c: &mut Ctx, slf: Value) -> RV {
    if obj_flags(slf) & FL_FROZEN != 0 { return Err(raise_frozen(c, slf)); }
    if renum(c, slf).look == UNDEF { let vs = crate::fiber::enum_next_values(c, slf)?; renum_mut(c, slf).look = vs; }
    Ok(renum(c, slf).look)
}
fn e_peek_values(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let vs = peek_values(c, slf)?; let v = c.ary(vs).to_vec(); Ok(c.ary_from_vec(v)) }
fn e_peek(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let vs = peek_values(c, slf)?; Ok(ary2sv(c, vs, true)) }
fn e_feed(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if obj_flags(slf) & FL_FROZEN != 0 { return Err(raise_frozen(c, slf)); }
    if renum(c, slf).feed != UNDEF { return type_err(c, "feed value already set"); }
    renum_mut(c, slf).feed = a[0];
    Ok(NIL)
}
fn e_rewind(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if obj_flags(slf) & FL_FROZEN != 0 { return Err(raise_frozen(c, slf)); }
    let r = renum(c, slf).recv;
    if crate::rt::check_respond(c, r, intern(b"rewind"))? { funcall(c, r, intern(b"rewind"), &[], std::ptr::null())?; }
    let en = renum_mut(c, slf);
    en.pos = 0; en.buf = NIL; en.fiber = NIL; en.look = UNDEF; en.feed = UNDEF; en.stop = NIL;
    Ok(slf)
}
/// rb_frame_this_func: the name the running builtin was called by (an alias's own).
pub(crate) fn this_mid(c: &Ctx, dflt: Sym) -> Sym {
    match c.cur_frame() { Some(f) if c.me_ref(f.me).is_some() => crate::builtins::object::callee_name(c, f.me), _ => dflt }
}
/// Length of an all-Integer arithmetic sequence (bignum ends too), Float::INFINITY for step 0.
fn arith_int_len(c: &mut Ctx, b: Value, e: Value, st: Value, x: bool) -> RV {
    let z = Value::fix(0);
    let call = |c: &mut Ctx, r: Value, op: &[u8], a: Value| funcall(c, r, intern(op), &[a], std::ptr::null());
    let sgn = call(c, st, b"<=>", z)?.as_fix();
    if sgn == 0 { return Ok(c.float_new(f64::INFINITY)); }
    let (d, k) = if sgn > 0 { (call(c, e, b"-", b)?, st) } else { (call(c, b, b"-", e)?, call(c, z, b"-", st)?) };
    if call(c, d, b"<", z)?.truthy() { return Ok(z); }
    let q = call(c, d, b"/", k)?;
    let n = call(c, q, b"+", Value::fix(1))?;
    if x && call(c, d, b"%", k)? == z { return call(c, n, b"-", Value::fix(1)); }
    Ok(n)
}
fn e_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let en = renum(c, slf);
    if en.procs != NIL || en.lz & crate::builtins::lazy::LZ_UNINIT != 0 { return crate::builtins::lazy::size(c, slf, en); }
    if let Some((b, e, st, x)) = en.arith {
        if b.is_fixnum() && e.is_fixnum() && st.is_fixnum() {
            let (b, e, s2) = (b.as_fix() as i128, e.as_fix() as i128, st.as_fix() as i128);
            if s2 == 0 { return Ok(c.float_new(f64::INFINITY)); }
            let (d, k) = if s2 > 0 { (e - b, s2) } else { (b - e, -s2) };
            let n = if d < 0 { 0 } else { d / k + 1 - if x && d % k == 0 { 1 } else { 0 } };
            return Ok(c.int_from_i128(n.max(0)));
        }
        if e != NIL && is_integer(b) && is_integer(e) && is_integer(st) { return arith_int_len(c, b, e, st, x); }
        if e == NIL { return Ok(c.float_new(f64::INFINITY)); }
        if is_float(b) || is_float(e) || is_float(st) {
            if let (Some(bf), Some(ef), Some(sf)) = (c.num_to_f64(b), c.num_to_f64(e), c.num_to_f64(st)) {
                let n = float_step_size(bf, ef, sf, x);
                if n.is_infinite() { return Ok(c.float_new(n)); }
                return Ok(crate::builtins::numeric_methods::f64_to_int(c, n));
            }
        }
        let a = collect_all(c, slf)?; return Ok(Value::fix(c.ary_len(a) as i64));
    }
    if en.size == TRUE {
        let r = en.recv;
        if respond_to(c, r, s::size, true) { return funcall(c, r, s::size, &[], std::ptr::null()); }
        return Ok(NIL);
    }
    // rb_check_funcall_kw(e->size, :call, *args)
    if is_proc(en.size) { let p = en.size; let args = c.ary(en.args).to_vec(); return call_proc(c, p, &args, std::ptr::null()); }
    if en.size.is_sym() {
        if let Some(r) = crate::builtins::array_methods::ary_gen_size(c, en.size.as_sym(), en.recv, en.args) { return r; }
        if en.size.as_sym() == s::ary_cycle_size { return crate::builtins::array_methods::ary_cycle_size(c, en.recv, en.args); }
    }
    if is_integer(en.size) || is_float(en.size) { return Ok(en.size); }
    if en.size != NIL && !en.size.is_sym() {
        if respond_to(c, en.size, s::call, true) { let args = c.ary(en.args).to_vec(); return funcall(c, en.size, s::call, &args, std::ptr::null()); }
        return Ok(en.size);
    }
    if en.lz & crate::builtins::lazy::SIZE_EXACT != 0 { return Ok(NIL); }
    if en.mid == s::loop_ { return Ok(c.float_new(f64::INFINITY)); }
    let (recv, mid) = (en.recv, en.mid);
    if (mid == s::times) && recv.is_fixnum() { return Ok(Value::fix(recv.as_fix().max(0))); }
    if crate::object::is_string(recv) {
        match crate::symbol::sym_bytes(mid) {
            b"each_char" | b"each_codepoint" => return Ok(Value::fix(c.str_char_len(recv) as i64)),
            b"each_byte" => return Ok(Value::fix(c.str_len(recv) as i64)),
            b"each_grapheme_cluster" => { let g = funcall(c, recv, intern(b"grapheme_clusters"), &[], std::ptr::null())?; return Ok(Value::fix(c.ary_len(g) as i64)); }
            _ => {}
        }
    }
    if mid == intern(b"each_slice") || mid == intern(b"each_cons") {
        if respond_to(c, recv, s::size, true) {
            let sz = funcall(c, recv, s::size, &[], std::ptr::null())?;
            let k = c.ary_get(en.args, 0).as_fix();
            if sz.is_fixnum() && k > 0 { let n = sz.as_fix(); return Ok(Value::fix(if mid == intern(b"each_slice") { (n + k - 1) / k } else { (n - k + 1).max(0) })); }
            return Ok(sz);
        }
    }
    if mid == s::upto || mid == s::downto {
        let lim = c.ary_get(en.args, 0);
        if recv.is_fixnum() && lim.is_fixnum() { let n = if mid == s::upto { lim.as_fix() - recv.as_fix() + 1 } else { recv.as_fix() - lim.as_fix() + 1 }; return Ok(Value::fix(n.max(0))); }
    }
    Ok(NIL)
}
/// CRuby's ruby_float_step_size.
pub fn float_step_size(beg: f64, end: f64, unit: f64, excl: bool) -> f64 {
    if unit == 0.0 { return f64::INFINITY; }
    if unit.is_infinite() { return if unit > 0.0 { (beg <= end) as i32 as f64 } else { (beg >= end) as i32 as f64 }; }
    let mut n = (end - beg) / unit;
    let mut err = (beg.abs() + end.abs() + (end - beg).abs()) / unit.abs() * f64::EPSILON;
    if err > 0.5 { err = 0.5; }
    if excl {
        if n <= 0.0 { return 0.0; }
        if n < 1.0 { n = 0.0; } else { n = (n - err).floor(); }
        let d = (n + 1.0) * unit + beg;
        if beg < end { if d < end { n += 1.0; } } else if beg > end && d > end { n += 1.0; }
        return n + 1.0;
    }
    if n < 0.0 { return 0.0; }
    (n + err).floor() + 1.0
}
fn e_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let en = renum(c, slf);
    if en.procs != NIL || en.lz & crate::builtins::lazy::LZ_ANY != 0 { return crate::builtins::lazy::inspect(c, slf, en); }
    if let Some((b, e, st, x)) = en.arith {
        let r = if en.mid == s::step && crate::numeric::is_numeric(b) && !is_range(en.recv) && false { String::new() } else { String::new() };
        let _ = r;
        if is_range(en.recv) {
            let rs = inspect_str(c, en.recv)?;
            let args = c.ary(en.args).to_vec();
            let mut a = String::new();
            if !args.is_empty() { a.push('('); for (i, x) in args.iter().enumerate() { if i > 0 { a.push_str(", "); } a.push_str(&inspect_str(c, *x)?); } a.push(')'); }
            return Ok(c.str_new(format!("(({}).{}{})", rs, sym_str(en.mid), a).as_bytes()));
        }
        let bs = inspect_str(c, b)?; let es = if e == NIL { String::new() } else { inspect_str(c, e)? }; let ss = inspect_str(c, st)?;
        if en.mid == s::step && !en.recv.is_heap() && en.recv == b && crate::numeric::is_numeric(b) && is_range_origin(slf) == false {
            // Numeric#step's own arguments, keywords as `k: v`
            let args = c.ary(en.args).to_vec();
            let kw = if en.kw { args.last().copied().filter(|&h| crate::object::is_hash(h)) } else { None };
            let mut parts = vec![];
            for &v in &args[..args.len() - kw.is_some() as usize] { parts.push(inspect_str(c, v)?); }
            if let Some(h) = kw { for (k, v) in c.hash_pairs(h) { let vs = inspect_str(c, v)?; parts.push(format!("{}: {}", sym_str(k.as_sym()), vs)); } }
            let a = if parts.is_empty() { String::new() } else { format!("({})", parts.join(", ")) };
            return Ok(c.str_new(format!("({}.step{})", bs, a).as_bytes()));
        }
        return Ok(c.str_new(format!("(({}{}{}).step({}))", bs, if x { "..." } else { ".." }, es, ss).as_bytes()));
    }
    if en.gen != NIL { return Ok(c.str_new(format!("#<Enumerator: #<Enumerator::Generator:{:#018x}>:each>", en.gen.raw()).as_bytes())); }
    let cls = class_path(c, real_class_of(c, slf));
    if !en.init { return Ok(c.str_new(format!("#<{}: uninitialized>", cls).as_bytes())); }
    // inspect_enumerator runs under rb_exec_recursive
    if !rec_enter(c, slf, s::inspect)? { return Ok(c.str_new(format!("#<{}: ...>", cls).as_bytes())); }
    let r = (|| -> RV {
        let r = inspect_value(c, en.recv)?;
        let mut out = format!("#<{}: ", cls).into_bytes();
        out.extend_from_slice(c.str_bytes(r));
        out.push(b':');
        out.extend_from_slice(sym_str(en.mid).as_bytes());
        append_args(c, &mut out, en.args)?;
        out.push(b'>');
        Ok(c.str_new(&out))
    })();
    rec_leave(c, slf, s::inspect);
    r
}
fn is_range_origin(_v: Value) -> bool { false }
/// append_method's argument list: "(a, b, k: v)"; a last non-empty Hash of Symbol keys prints as keywords.
pub(crate) fn append_args(c: &mut Ctx, out: &mut Vec<u8>, args: Value) -> Result<(), Esc> {
    let mut argv = c.ary(args).to_vec();
    if argv.is_empty() { return Ok(()); }
    let mut kw = NIL;
    if let Some(&h) = argv.last() {
        if is_hash(h) && c.hash_len(h) > 0 && c.hash_pairs(h).iter().all(|(k, _)| k.is_sym()) { kw = h; argv.pop(); }
    }
    out.push(b'(');
    for x in argv { let r = inspect_value(c, x)?; out.extend_from_slice(c.str_bytes(r)); out.extend_from_slice(b", "); }
    if kw != NIL {
        for (k, v) in c.hash_pairs(kw) {
            out.extend_from_slice(sym_str(k.as_sym()).as_bytes());
            out.extend_from_slice(b": ");
            let r = to_s_value(c, v)?;
            out.extend_from_slice(c.str_bytes(r));
            out.extend_from_slice(b", ");
        }
    }
    out.truncate(out.len() - 2);
    out.push(b')');
    Ok(())
}
fn e_initialize(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    if blk.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
    let g = block_to_proc(c, blk)?;
    let mut size = a.first().copied().unwrap_or(NIL);
    // rb_to_int unless nil / callable / Infinity
    let keep = size == NIL || is_integer(size) || is_proc(size) || (is_float(size) && c.num_to_f64(size) == Some(f64::INFINITY)) || respond_to(c, size, s::call, true);
    if !keep { let n = crate::builtins::numeric_methods::num_to_long(c, size)?; size = c.int_from_i64(n); }
    if obj_flags(slf) & FL_FROZEN != 0 { return Err(raise_frozen(c, slf)); }
    let Some(en) = c.data_mut::<REnum>(slf) else { return type_err(c, "wrong argument type (expected Enumerator)") };
    en.gen = g; en.size = size; en.recv = NIL; en.mid = s::each; en.buf = NIL; en.pos = 0; en.lz = 0; en.procs = NIL; en.init = true;
    Ok(slf)
}
fn y_s_new(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return err(c, c.vm().c_local_jump_error.get(), "no block given"); }
    let p = block_to_proc(c, blk)?;
    Ok(c.alloc(slf, Body::Data(Box::new(RYielder { blk: p }))))
}
fn y_yield(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let kw = kw_given(c); y_yield_kw(c, slf, a, kw) }
fn y_yield_kw(c: &mut Ctx, slf: Value, a: &[Value], kw: bool) -> RV {
    let p = c.data::<RYielder>(slf).expect("a Yielder").blk;
    let bb = c.proc_data(p).blk;
    invoke_block_full(c, &bb, a, std::ptr::null(), bb.self_, false, std::ptr::null(), kw)
}
fn y_ltlt(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { y_yield_kw(c, slf, a, false)?; Ok(slf) }
fn y_to_proc(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let b = Block::cfunc(c, |c, d, args, _b, _o| y_yield_kw(c, d, args, false), slf);
    let k = c.vm().c_proc.get();
    let p = c.alloc(k, Body::Proc(Box::new(ProcData { blk: b, is_lambda: true, from_method: NIL })));
    c.proc_data_mut(p).blk.proc_val = p;
    Ok(p)
}
fn e_take(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "attempt to take negative size"); }
    if n >= 1 << 60 { return arg_err(c, "array size too big"); }
    let out = c.ary_new();
    if n == 0 { return Ok(out); }
    let fib = crate::fiber::current(c);
    let data = c.ary_new_from(&[out, Value::fix(n), fib]);
    let b = Block::cfunc(c, |c, d, args, _b, _o| {
        let out = c.ary_get(d, 0);
        let v = match args.len() { 0 => NIL, 1 => args[0], _ => c.ary_new_from(args) };
        c.ary_push(out, v);
        // CRuby's `--n == 0` then rb_iter_break: from another thread / fiber the break is a LocalJumpError there
        let left = c.ary_get(d, 1).as_fix() - 1;
        c.ary_mut(d)[1] = Value::fix(left);
        if left == 0 {
            if crate::fiber::current(c) != c.ary_get(d, 2) { return Err(crate::rt::raise_local_jump(c, "break from proc-closure", "break", NIL)); }
            c.esc = EscKind::Throw; c.esc_tag_v = d; c.esc_val = out; return Err(Esc);
        }
        Ok(NIL)
    }, data);
    match enum_each_with(c, slf, Blk::of(&b)) {
        Err(e) => { if c.esc == EscKind::Throw && c.esc_tag_v == data { c.esc = EscKind::None; return Ok(out); } Err(e) }
        Ok(_) => Ok(out),
    }
}
fn e_first(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if !a.is_empty() { return e_take(c, slf, a, b); }
    // first_i: keep the value and break, every time
    let fib = crate::fiber::current(c);
    let data = c.ary_new_from(&[NIL, fib]);
    let b = Block::cfunc(c, |c, d, args, _b, _o| {
        let v = match args.len() { 0 => NIL, 1 => args[0], _ => c.ary_new_from(args) };
        c.ary_mut(d)[0] = v;
        let f = c.ary_get(d, 1);
        // nil: #first has returned (a kept block called late)
        if f == NIL { return Err(crate::rt::raise_local_jump(c, "unexpected break", "break", NIL)); }
        if crate::fiber::current(c) != f { return Err(crate::rt::raise_local_jump(c, "break from proc-closure", "break", NIL)); }
        c.esc = EscKind::Throw; c.esc_tag_v = d; c.esc_val = NIL; Err(Esc)
    }, data);
    let r = enum_each_with(c, slf, Blk::of(&b));
    c.ary_mut(data)[1] = NIL;
    match r {
        Err(e) => { if c.esc == EscKind::Throw && c.esc_tag_v == data { c.esc = EscKind::None; return Ok(c.ary_get(data, 0)); } Err(e) }
        Ok(_) => Ok(c.ary_get(data, 0)),
    }
}
fn arith_begin(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(renum(c, slf).arith.unwrap().0) }
fn arith_end(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(renum(c, slf).arith.unwrap().1) }
fn arith_step(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(renum(c, slf).arith.unwrap().2) }
fn arith_excl(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(renum(c, slf).arith.unwrap().3)) }
fn arith_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if !is_arith_seq(c, a[0]) { return Ok(FALSE); }
    let (b1, e1, s1, x1) = renum(c, slf).arith.unwrap();
    let (b2, e2, s2, x2) = renum(c, a[0]).arith.unwrap();
    use crate::builtins::array_methods::rb_eq;
    Ok(Value::bool(x1 == x2 && rb_eq(c, b1, b2)? && rb_eq(c, e1, e2)? && rb_eq(c, s1, s2)?))
}
fn arith_hash(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let (b, e, st, x) = renum(c, slf).arith.unwrap();
    let a = c.ary_new_from(&[Value::bool(x), b, e, st]);
    funcall(c, a, intern(b"hash"), &[], std::ptr::null())
}
fn arith_last(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { check_argc(c, a, 0, 1)?;
    // all-Integer: counted back from the end, not enumerated
    if let Some((bg, e, st, x)) = renum(c, slf).arith {
        if e != NIL && is_integer(bg) && is_integer(e) && is_integer(st) && a.len() <= 1 {
            let len = arith_int_len(c, bg, e, st, x)?;
            if is_integer(len) {
                let (z, one) = (Value::fix(0), Value::fix(1));
                let call = |c: &mut Ctx, r: Value, op: &[u8], a: Value| funcall(c, r, intern(op), &[a], std::ptr::null());
                let pos = call(c, len, b">", z)?.truthy();
                let lastv = |c: &mut Ctx, idx: Value| -> RV { let m = call(c, idx, b"*", st)?; call(c, bg, b"+", m) };
                if a.is_empty() {
                    if !pos { return Ok(NIL); }
                    let i = call(c, len, b"-", one)?;
                    return lastv(c, i);
                }
                let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
                if n < 0 { return arg_err(c, "negative array size"); }
                let nv = c.int_from_i64(n);
                let cntv = if call(c, len, b">", nv)?.truthy() { nv } else { len };
                if !cntv.is_fixnum() || cntv.as_fix() >= 1 << 60 { return arg_err(c, "array size too big"); }
                let cnt = cntv.as_fix();
                if cnt > 1 << 31 { return err(c, c.vm().c_no_memory_error.get(), "failed to allocate memory"); }
                let out = c.ary_new();
                let mut i = call(c, len, b"-", cntv)?;
                for _ in 0..cnt { let v = lastv(c, i)?; c.ary_push(out, v); i = call(c, i, b"+", one)?; }
                return Ok(out);
            }
        }
    }
    let all = collect_all(c, slf)?;
    funcall(c, all, intern(b"last"), a, b)
}
fn arith_first(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { check_argc(c, a, 0, 1)?; e_first(c, slf, a, b) }

pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if let Some(&e) = c.data::<REnum>(v) {
        // enumerator_init_copy
        if crate::fiber::enum_fiber_live(c, v) { return type_err(c, "can't copy execution context"); }
        let lz = if e.procs != NIL { e.lz | crate::builtins::lazy::LZ_DUP } else { e.lz };
        let n = c.alloc(obj_klass(v), Body::Data(Box::new(REnum { buf: NIL, pos: 0, fiber: NIL, lz, look: UNDEF, feed: UNDEF, stop: NIL, ..e })));
        return Ok(Some(n));
    }
    Ok(None)
}
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if crate::builtins::inherits(c, klass, c.vm().c_enumerator.get()) {
        if crate::builtins::inherits(c, klass, c.vm().c_arith_seq.get()) { return type_err(c, format!("allocator undefined for {}", class_path(c, klass))); }
        let e = new_enum(c, klass, NIL, s::each, &[], NIL);
        renum_mut(c, e).init = false;
        if crate::builtins::inherits(c, klass, c.vm().c_lazy.get()) { renum_mut(c, e).lz = crate::builtins::lazy::LZ_UNINIT; }
        return Ok(Some(e));
    }
    Ok(None)
}

/// The classes `init` defines, for the Vm.
pub struct Classes { pub enumerator: Value, pub yielder: Value, pub generator: Value, pub lazy: Value, pub chain: Value, pub arith_seq: Value }
/// `e` is Enumerator, already marked as having no allocator (its subclasses inherit that).
pub fn init(c: &mut Ctx, e: Value, object: Value, enumerable: Value) -> Classes {
    include_module(c, e, enumerable);
    let k = Classes { enumerator: e, yielder: define_class_under(c, e, "Yielder", object), generator: define_class_under(c, e, "Generator", object),
                      lazy: define_class_under(c, e, "Lazy", e), chain: define_class_under(c, e, "Chain", e), arith_seq: define_class_under(c, e, "ArithmeticSequence", e) };
    define_private_cfunc(c, e, "initialize", e_initialize, -1);
    for k in [e, k.generator, k.chain] { define_private_cfunc(c, k, "initialize_copy", crate::builtins::object::k_init_copy, 1); }
    define_cfunc(c, e, "each", e_each, -1);
    define_cfunc(c, e, "with_index", e_with_index, -1);
    define_cfunc(c, e, "each_with_index", e_each_with_index, 0);
    define_cfunc(c, e, "with_object", e_with_object, 1);
    define_cfunc(c, e, "each_with_object", e_with_object, 1);
    define_cfunc(c, e, "next", e_next, 0);
    define_cfunc(c, e, "peek", e_peek, 0);
    define_cfunc(c, e, "next_values", e_next_values, 0);
    define_cfunc(c, e, "peek_values", e_peek_values, 0);
    define_cfunc(c, e, "feed", e_feed, 1);
    define_cfunc(c, e, "rewind", e_rewind, 0);
    define_cfunc(c, e, "size", e_size, 0);
    define_cfunc(c, e, "inspect", e_inspect, 0);
    let y = k.yielder;
    define_singleton_cfunc(c, y, "new", y_s_new, 0);
    define_cfunc(c, y, "yield", y_yield, -1);
    define_cfunc(c, y, "<<", y_ltlt, 1);
    define_cfunc(c, y, "to_proc", y_to_proc, 0);
    let a = k.arith_seq;
    define_cfunc(c, a, "begin", arith_begin, 0);
    define_cfunc(c, a, "end", arith_end, 0);
    define_cfunc(c, a, "step", arith_step, 0);
    define_cfunc(c, a, "exclude_end?", arith_excl, 0);
    define_cfunc(c, a, "==", arith_eq, 1);
    define_cfunc(c, a, "===", arith_eq, 1);
    define_cfunc(c, a, "eql?", arith_eq, 1);
    define_cfunc(c, a, "hash", arith_hash, 0);
    define_cfunc(c, a, "last", arith_last, -1);
    define_cfunc(c, a, "first", arith_first, -1);
    // CRuby: each / inspect / size are ArithmeticSequence's own; to_a / entries are Enumerable's
    define_cfunc(c, a, "each", e_each, -1);
    define_cfunc(c, a, "inspect", e_inspect, 0);
    define_cfunc(c, a, "size", e_size, 0);
    let sa = singleton_class(c, a);
    crate::builtins::object::undef_method(c, sa, intern(b"new")).expect("ArithmeticSequence.new");
    k
}
