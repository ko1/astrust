// Enumerator / Enumerator::Yielder / ArithmeticSequence / Enumerator::Lazy (eager subset).
use crate::array::*;
use crate::builtins::kernel::*;
use crate::string::*;
use crate::symbol::{intern, s, sym_str};
use crate::value::*;
use crate::vm::*;

pub struct REnum {
    pub recv: Value, pub mid: Sym, pub args: Value, pub size: Value, pub gen: Value,
    pub buf: Value, pub pos: usize, pub fiber: Value,
    pub arith: Option<(Value, Value, Value, bool)>,
    pub lazy: bool,
}
impl RData for REnum {
    fn mark(&self, m: &mut crate::gc::Marker) {
        m.mark(self.recv); m.mark(self.args); m.mark(self.size); m.mark(self.gen); m.mark(self.buf); m.mark(self.fiber);
        if let Some((a, b, c, _)) = self.arith { m.mark(a); m.mark(b); m.mark(c); }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
pub struct RYielder { pub blk: Value }
impl RData for RYielder {
    fn mark(&self, m: &mut crate::gc::Marker) { m.mark(self.blk); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

pub fn renum(v: Value) -> Option<&'static mut REnum> { rdata::<REnum>(v) }

fn new_enum(klass: Value, recv: Value, mid: Sym, args: &[Value], size: Value) -> Value {
    let a = ary_new_from(args);
    crate::gc::alloc(klass, Body::Data(Box::new(REnum { recv, mid, args: a, size, gen: NIL, buf: NIL, pos: 0, fiber: NIL, arith: None, lazy: false })))
}

pub fn enum_for(_c: &mut Ctx, recv: Value, mid: Sym, args: &[Value], size_fn: Value) -> RV {
    Ok(new_enum(vm().c_enumerator, recv, mid, args, size_fn))
}
/// An enumerator whose size is the receiver's size.
pub fn enum_for_sized(_c: &mut Ctx, recv: Value, mid: Sym, args: &[Value]) -> RV {
    Ok(new_enum(vm().c_enumerator, recv, mid, args, TRUE))
}
pub fn enum_from_array(_c: &mut Ctx, recv: Value, mid: Sym, args: &[Value], arr: Value) -> RV {
    let e = new_enum(vm().c_enumerator, recv, mid, args, NIL);
    renum(e).unwrap().buf = arr;
    let src = arr;
    let en = renum(e).unwrap();
    en.recv = src; en.mid = s::each; en.args = ary_new();
    Ok(e)
}

pub fn arith_seq(_c: &mut Ctx, b: Value, e: Value, st: Value, excl: bool, mid: Sym) -> RV {
    let v = new_enum(vm().c_arith_seq, b, mid, &[], NIL);
    renum(v).unwrap().arith = Some((b, e, st, excl));
    Ok(v)
}
/// ArithmeticSequence made by Range#step / Range#%: inspects as "((range).mid(args))".
pub fn arith_seq_from(_c: &mut Ctx, range: Value, b: Value, e: Value, st: Value, excl: bool, mid: Sym, args: &[Value]) -> RV {
    let v = new_enum(vm().c_arith_seq, range, mid, args, NIL);
    renum(v).unwrap().arith = Some((b, e, st, excl));
    Ok(v)
}
pub fn is_arith_seq(v: Value) -> bool { renum(v).map(|e| e.arith.is_some()).unwrap_or(false) }
pub fn arith_seq_slice(c: &mut Ctx, seq: Value, ary: Value) -> RV {
    let (b, e, st, excl) = renum(seq).unwrap().arith.unwrap();
    let n = ary_len(ary) as i64;
    let step = crate::builtins::numeric_methods::num_to_long(c, st)?;
    let r = crate::range::range_new(b, e, excl);
    let Some((start, len)) = crate::range::range_beg_len(c, r, n, 0)? else { return Ok(NIL) };
    let out = ary_new();
    if step > 0 { let mut i = start; while i < start + len { ary_push(out, ary_get(ary, i as usize)); i += step; } }
    else if step < 0 { let mut i = start + len - 1; while i >= start { ary_push(out, ary_get(ary, i as usize)); i += step; } }
    else { return arg_err(c, "step can't be 0"); }
    Ok(out)
}

/// Run the enumerator, feeding each yielded value to blk.
pub fn enum_each_with(c: &mut Ctx, e: Value, blk: *const Block) -> RV {
    let en = renum(e).unwrap();
    if let Some((b, en_, st, excl)) = en.arith {
        if en.mid == s::step && !crate::object::is_range(en.recv) {
            if is_range(crate::range::range_new(b, en_, excl)) && false { }
        }
        let r = crate::range::range_new(b, en_, excl);
        let bp = block_to_proc(c, blk)?;
        let fwd = Block::cfunc(c, |c, d, args, _b| { let p = rproc(d); let bb = p.blk; invoke_block_full(c, &bb, args, std::ptr::null(), bb.self_, false, std::ptr::null(), false) }, bp);
        if en.mid == s::step && !en.recv.is_heap() || (en.mid == s::step && crate::numeric::is_numeric(en.recv)) {
            return funcall(c, b, s::step, &[en_, st], &fwd);
        }
        return funcall(c, r, s::step, &[st], &fwd);
    }
    if en.gen != NIL {
        let y = crate::gc::alloc(vm().c_yielder, Body::Data(Box::new(RYielder { blk: block_to_proc(c, blk)? })));
        let g = en.gen;
        return call_proc(c, g, &[y], std::ptr::null());
    }
    if en.recv == NIL && en.mid == s::each && ary_len(en.args) == 0 {
        let me = lookup_method(class_of(e), s::each);
        if !me.is_null() && !matches!(unsafe { (*me).kind }, MKind::Cfunc(..)) { return funcall(c, e, s::each, &[], blk); }
    }
    let (recv, mid, args) = (en.recv, en.mid, ary_to_vec(en.args));
    let keep = en.args;
    let r = funcall_public(c, recv, mid, &args, blk);
    std::hint::black_box(keep);
    r
}

fn e_each(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() {
        if a.is_empty() { return Ok(slf); }
        let en = renum(slf).unwrap();
        let mut args = ary_to_vec(en.args); args.extend_from_slice(a);
        return Ok(new_enum(vm().c_enumerator, en.recv, en.mid, &args, NIL));
    }
    if !a.is_empty() {
        let en = renum(slf).unwrap();
        let mut args = ary_to_vec(en.args); args.extend_from_slice(a);
        return funcall_public(c, en.recv, en.mid, &args, blk);
    }
    enum_each_with(c, slf, blk)
}

fn e_with_index(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    let off = if a.is_empty() || a[0] == NIL { 0 } else { crate::builtins::numeric_methods::num_to_long(c, a[0])? };
    if blk.is_null() { return enum_for(c, slf, intern(b"with_index"), a, NIL); }
    let data = ary_new_from(&[Value::fix(off), block_to_proc(c, blk)?]);
    let b = Block::cfunc(c, |c, d, args, _b| {
        let i = ary_get(d, 0); let p = ary_get(d, 1);
        rary(d)[0] = Value::fix(i.as_fix() + 1);
        let v = if args.len() == 1 { args[0] } else { ary_new_from(args) };
        call_proc(c, p, &[v, i], std::ptr::null())
    }, data);
    enum_each_with(c, slf, &b)
}
fn e_each_with_index(c: &mut Ctx, slf: Value, _a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return enum_for(c, slf, intern(b"each_with_index"), &[], NIL); }
    e_with_index(c, slf, &[], blk)?;
    Ok(slf)
}
fn e_with_object(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return enum_for(c, slf, intern(b"with_object"), a, NIL); }
    let memo = a[0];
    let data = ary_new_from(&[memo, block_to_proc(c, blk)?]);
    let b = Block::cfunc(c, |c, d, args, _b| {
        let v = if args.len() == 1 { args[0] } else { ary_new_from(args) };
        call_proc(c, ary_get(d, 1), &[v, ary_get(d, 0)], std::ptr::null())
    }, data);
    enum_each_with(c, slf, &b)?;
    Ok(memo)
}
fn collect_all(c: &mut Ctx, slf: Value) -> RV {
    let out = ary_new();
    let b = Block::cfunc(c, |_c, d, args, _b| { let v = if args.len() == 1 { args[0] } else { ary_new_from(args) }; ary_push(d, v); Ok(NIL) }, out);
    enum_each_with(c, slf, &b)?;
    Ok(out)
}
fn e_next(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if renum(slf).unwrap().buf == NIL {
        if let Some(r) = crate::fiber::enum_next(c, slf)? { return Ok(r); }
        let all = collect_all(c, slf)?;
        renum(slf).unwrap().buf = all;
    }
    let en = renum(slf).unwrap();
    if en.pos >= ary_len(en.buf) {
        let e = make_exception(vm().c_stop_iteration, "iteration reached an end");
        return Err(raise_exc(c, e));
    }
    let v = ary_get(en.buf, en.pos);
    en.pos += 1;
    Ok(v)
}
fn e_peek(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if crate::fiber::enum_has_fiber(slf) { return crate::fiber::enum_peek(c, slf); }
    let v = e_next(c, slf, a, b)?;
    renum(slf).unwrap().pos -= 1;
    Ok(v)
}
fn e_rewind(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let en = renum(slf).unwrap();
    en.pos = 0; en.buf = NIL; en.fiber = NIL;
    Ok(slf)
}
fn e_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let en = renum(slf).unwrap();
    if let Some((b, e, st, x)) = en.arith {
        if b.is_fixnum() && e.is_fixnum() && st.is_fixnum() {
            let (b, e, s2) = (b.as_fix(), e.as_fix(), st.as_fix());
            if s2 == 0 { return Ok(crate::numeric::float_new(f64::INFINITY)); }
            let n = if s2 > 0 { if e < b { 0 } else { (e - b) / s2 + 1 - if x && (e - b) % s2 == 0 { 1 } else { 0 } } } else { if e > b { 0 } else { (b - e) / (-s2) + 1 - if x && (b - e) % (-s2) == 0 { 1 } else { 0 } } };
            return Ok(Value::fix(n.max(0)));
        }
        if e == NIL { return Ok(crate::numeric::float_new(f64::INFINITY)); }
        let a = collect_all(c, slf)?; return Ok(Value::fix(ary_len(a) as i64));
    }
    if en.size == TRUE {
        let r = en.recv;
        if respond_to(r, s::size, true) { return funcall(c, r, s::size, &[], std::ptr::null()); }
        return Ok(NIL);
    }
    if is_proc(en.size) { let p = en.size; return call_proc(c, p, &[], std::ptr::null()); }
    if en.size.is_fixnum() || is_float(en.size) { return Ok(en.size); }
    if en.mid == s::loop_ { return Ok(crate::numeric::float_new(f64::INFINITY)); }
    let (recv, mid) = (en.recv, en.mid);
    if (mid == s::times) && recv.is_fixnum() { return Ok(Value::fix(recv.as_fix().max(0))); }
    if mid == intern(b"each_slice") || mid == intern(b"each_cons") {
        if respond_to(recv, s::size, true) {
            let sz = funcall(c, recv, s::size, &[], std::ptr::null())?;
            let k = ary_get(en.args, 0).as_fix();
            if sz.is_fixnum() && k > 0 { let n = sz.as_fix(); return Ok(Value::fix(if mid == intern(b"each_slice") { (n + k - 1) / k } else { (n - k + 1).max(0) })); }
        }
    }
    if mid == s::upto || mid == s::downto {
        let lim = ary_get(en.args, 0);
        if recv.is_fixnum() && lim.is_fixnum() { let n = if mid == s::upto { lim.as_fix() - recv.as_fix() + 1 } else { recv.as_fix() - lim.as_fix() + 1 }; return Ok(Value::fix(n.max(0))); }
    }
    Ok(NIL)
}
fn e_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let en = renum(slf).unwrap();
    if let Some((b, e, st, x)) = en.arith {
        let r = if en.mid == s::step && crate::numeric::is_numeric(b) && !is_range(en.recv) && false { String::new() } else { String::new() };
        let _ = r;
        if is_range(en.recv) {
            let rs = inspect_str(c, en.recv)?;
            let args = ary_to_vec(en.args);
            let mut a = String::new();
            if !args.is_empty() { a.push('('); for (i, x) in args.iter().enumerate() { if i > 0 { a.push_str(", "); } a.push_str(&inspect_str(c, *x)?); } a.push(')'); }
            return Ok(str_new(format!("(({}).{}{})", rs, sym_str(en.mid), a).as_bytes()));
        }
        let bs = inspect_str(c, b)?; let es = if e == NIL { String::new() } else { inspect_str(c, e)? }; let ss = inspect_str(c, st)?;
        if en.mid == s::step && !en.recv.is_heap() && en.recv == b && crate::numeric::is_numeric(b) && is_range_origin(slf) == false {
            let args = if e == NIL { format!("(by: {})", ss) } else { format!("({}, {})", es, ss) };
            return Ok(str_new(format!("({}.step{})", bs, args).as_bytes()));
        }
        return Ok(str_new(format!("(({}{}{}).step({}))", bs, if x { "..." } else { ".." }, es, ss).as_bytes()));
    }
    if en.gen != NIL { return Ok(str_new(format!("#<Enumerator: #<Enumerator::Generator:{:#018x}>:each>", en.gen.raw()).as_bytes())); }
    let r = inspect_str(c, en.recv)?;
    let args = ary_to_vec(en.args);
    let mut a = String::new();
    if !args.is_empty() { a.push('('); for (i, x) in args.iter().enumerate() { if i > 0 { a.push_str(", "); } a.push_str(&inspect_str(c, *x)?); } a.push(')'); }
    let cls = class_path(real_class_of(slf));
    Ok(str_new(format!("#<{}: {}:{}{}>", cls, r, sym_str(en.mid), a).as_bytes()))
}
fn is_range_origin(_v: Value) -> bool { false }
fn e_initialize(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    if blk.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
    let g = block_to_proc(c, blk)?;
    let size = a.first().copied().unwrap_or(NIL);
    if size != NIL && !size.is_fixnum() && !is_float(size) && !is_bignum(size) && !is_proc(size) && !respond_to(size, s::call, true) {
        crate::builtins::numeric_methods::num_to_long(c, size)?;
    }
    let Some(en) = renum(slf) else { return type_err(c, "wrong argument type (expected Enumerator)") };
    en.gen = g; en.size = size; en.recv = NIL; en.mid = s::each; en.buf = NIL; en.pos = 0;
    Ok(slf)
}
fn y_s_new(c: &mut Ctx, slf: Value, _a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return err(c, vm().c_local_jump_error, "no block given"); }
    let p = block_to_proc(c, blk)?;
    Ok(crate::gc::alloc(slf, Body::Data(Box::new(RYielder { blk: p }))))
}
fn y_yield(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let p = rdata::<RYielder>(slf).unwrap().blk;
    let v = match a.len() { 0 => NIL, 1 => a[0], _ => ary_new_from(a) };
    let pd = rproc(p);
    let bb = pd.blk;
    if let BlockKind::Cfunc(..) = bb.kind { return invoke_block_full(c, &bb, a, std::ptr::null(), bb.self_, false, std::ptr::null(), false); }
    let _ = v;
    invoke_block_full(c, &bb, a, std::ptr::null(), bb.self_, false, std::ptr::null(), false)
}
fn y_ltlt(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { y_yield(c, slf, a, b)?; Ok(slf) }
fn y_to_proc(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let b = Block::cfunc(c, |c, d, args, _b| y_yield(c, d, args, std::ptr::null()), slf);
    let p = crate::gc::alloc(vm().c_proc, Body::Proc(Box::new(ProcData { blk: b, is_lambda: true, from_method: NIL })));
    rproc(p).blk.proc_val = p;
    Ok(p)
}
fn e_take(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
    if n < 0 { return arg_err(c, "attempt to take negative size"); }
    let out = ary_new();
    if n == 0 { return Ok(out); }
    let data = ary_new_from(&[out, Value::fix(n)]);
    let b = Block::cfunc(c, |c, d, args, _b| {
        let out = ary_get(d, 0);
        let v = if args.len() == 1 { args[0] } else { ary_new_from(args) };
        ary_push(out, v);
        if ary_len(out) as i64 >= ary_get(d, 1).as_fix() { c.esc = EscKind::Throw; c.esc_tag_v = d; c.esc_val = out; return Err(Esc); }
        Ok(NIL)
    }, data);
    match enum_each_with(c, slf, &b) {
        Err(e) => { if c.esc == EscKind::Throw && c.esc_tag_v == data { c.esc = EscKind::None; return Ok(out); } Err(e) }
        Ok(_) => Ok(out),
    }
}
fn e_first(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if a.is_empty() { let r = e_take(c, slf, &[Value::fix(1)], b)?; return Ok(ary_get(r, 0)); }
    e_take(c, slf, a, b)
}
fn arith_begin(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(renum(slf).unwrap().arith.unwrap().0) }
fn arith_end(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(renum(slf).unwrap().arith.unwrap().1) }
fn arith_step(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(renum(slf).unwrap().arith.unwrap().2) }
fn arith_excl(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(renum(slf).unwrap().arith.unwrap().3)) }
fn arith_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if !is_arith_seq(a[0]) { return Ok(FALSE); }
    let (b1, e1, s1, x1) = renum(slf).unwrap().arith.unwrap();
    let (b2, e2, s2, x2) = renum(a[0]).unwrap().arith.unwrap();
    use crate::builtins::array_methods::rb_eq;
    Ok(Value::bool(x1 == x2 && rb_eq(c, b1, b2)? && rb_eq(c, e1, e2)? && rb_eq(c, s1, s2)?))
}
fn arith_last(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let all = collect_all(c, slf)?;
    funcall(c, all, intern(b"last"), a, b)
}
fn arith_first(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { e_first(c, slf, a, b) }
fn arith_to_a(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { collect_all(c, slf) }

pub fn dup_data(_c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if let Some(e) = renum(v) {
        let n = crate::gc::alloc(obj(v).klass, Body::Data(Box::new(REnum { recv: e.recv, mid: e.mid, args: e.args, size: e.size, gen: e.gen, buf: NIL, pos: 0, fiber: NIL, arith: e.arith, lazy: e.lazy })));
        return Ok(Some(n));
    }
    Ok(None)
}
pub fn alloc_data(_c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if crate::builtins::inherits(klass, vm().c_enumerator) { return Ok(Some(new_enum(klass, NIL, s::each, &[], NIL))); }
    Ok(None)
}

pub fn init() {
    let vm = vm();
    let e = define_class("Enumerator", vm.c_object);
    rclass(e).alloc = AllocKind::None;
    include_module(e, vm.m_enumerable);
    vm.c_enumerator = e;
    vm.c_yielder = define_class_under(e, "Yielder", vm.c_object);
    vm.c_generator = define_class_under(e, "Generator", vm.c_object);
    vm.c_lazy = define_class_under(e, "Lazy", e);
    vm.c_chain = define_class_under(e, "Chain", e);
    vm.c_arith_seq = define_class_under(e, "ArithmeticSequence", e);
    define_singleton_cfunc(e, "new", crate::builtins::object::c_new, -1);
    define_private_cfunc(e, "initialize", e_initialize, -1);
    define_cfunc(e, "each", e_each, -1);
    define_cfunc(e, "with_index", e_with_index, -1);
    define_cfunc(e, "each_with_index", e_each_with_index, 0);
    define_cfunc(e, "with_object", e_with_object, 1);
    define_cfunc(e, "each_with_object", e_with_object, 1);
    define_cfunc(e, "next", e_next, 0);
    define_cfunc(e, "peek", e_peek, 0);
    define_cfunc(e, "rewind", e_rewind, 0);
    define_cfunc(e, "size", e_size, 0);
    define_cfunc(e, "inspect", e_inspect, 0);
    define_cfunc(e, "to_s", e_inspect, 0);
    define_cfunc(e, "take", e_take, 1);
    define_cfunc(e, "first", e_first, -1);
    let y = vm.c_yielder;
    define_singleton_cfunc(y, "new", y_s_new, 0);
    define_cfunc(y, "yield", y_yield, -1);
    define_cfunc(y, "call", y_yield, -1);
    define_cfunc(y, "<<", y_ltlt, -1);
    define_cfunc(y, "to_proc", y_to_proc, 0);
    let a = vm.c_arith_seq;
    define_cfunc(a, "begin", arith_begin, 0);
    define_cfunc(a, "end", arith_end, 0);
    define_cfunc(a, "step", arith_step, 0);
    define_cfunc(a, "exclude_end?", arith_excl, 0);
    define_cfunc(a, "==", arith_eq, 1);
    define_cfunc(a, "===", arith_eq, 1);
    define_cfunc(a, "eql?", arith_eq, 1);
    define_cfunc(a, "last", arith_last, -1);
    define_cfunc(a, "first", arith_first, -1);
    define_cfunc(a, "to_a", arith_to_a, 0);
    define_cfunc(a, "entries", arith_to_a, 0);
    let _ = to_rstring;
}
