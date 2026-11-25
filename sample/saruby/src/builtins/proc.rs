// Proc / Method / UnboundMethod.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{obj_klass, obj_set_klass};
use crate::builtins::kernel::*;
use crate::object::*;
use crate::symbol::{intern, s, sym_str};
use crate::value::*;
use crate::vm::*;

fn sym_proc_call(c: &mut Ctx, data: Value, args: &[Value], blk: Blk, _o: Blk) -> RV {
    if args.is_empty() { return arg_err(c, "no receiver given"); }
    let mid = data.as_sym();
    funcall_flags(c, args[0], mid, &args[1..], blk, CF_PUBLIC_SEND)
}

/// A Proc of class `klass` running block `b`; `b` stays on the stack (rooted) until the slot holds it.
fn new_proc(c: &mut Ctx, klass: Value, b: Block, is_lambda: bool, from_method: Value) -> Value {
    let p = c.alloc_with(klass, move || Body::Proc(Box::new(ProcData { blk: b, is_lambda, from_method })));
    c.proc_data_mut(p).blk.proc_val = p;
    p
}
fn new_method_obj(c: &mut Ctx, klass: Value, m: MethodObj) -> Value {
    let MethodObj { recv, me, name, unbound, owner } = m;
    c.alloc_with(klass, move || Body::Method(Box::new(MethodObj { recv, me, name, unbound, owner })))
}
/// (recv, me, name, unbound, owner) of a Method / UnboundMethod, by value.
fn mo(c: &Ctx, v: Value) -> MethodObj {
    let m = c.method_obj(v);
    MethodObj { recv: m.recv, me: m.me, name: m.name, unbound: m.unbound, owner: m.owner }
}
fn me_owner(c: &Ctx, me: *const MethodEntry) -> Value { c.me_ref(me).map_or(NIL, |m| m.owner) }

pub fn sym_to_proc(c: &mut Ctx, sym: Value) -> RV {
    if let Some(&p) = c.vm().symproc_cache.borrow().get(&sym.as_sym()) { return Ok(p); }
    let mut b = Block::cfunc(c, sym_proc_call, sym);
    b.kind = BlockKind::Sym(sym.as_sym());
    let p = new_proc(c, c.vm().c_proc.get(), b, true, NIL);
    c.vm().symproc_cache.borrow_mut().insert(sym.as_sym(), p);
    Ok(p)
}

pub fn proc_arity(c: &Ctx, p: Value) -> i64 {
    let pd = c.proc_data(p);
    match pd.blk.kind {
        BlockKind::Iseq(is) => c.iseq_ref(is).map_or(-1, |is| iseq_arity(is, pd.is_lambda)),
        BlockKind::Sym(_) => -2,
        BlockKind::Method(m) => method_arity(c, c.method_obj(m).me),
        BlockKind::Cfunc(f, _) if f as *const () as usize == crate::builtins::enum_native::map_blk as *const () as usize && is_proc(pd.from_method) => outer_arity(c, pd.from_method),
        BlockKind::Cfunc(f, _) => if f as *const () as usize == crate::builtins::hash_methods::hash_proc_call as *const () as usize { 1 } else { -1 },
        BlockKind::Proc(q) => proc_arity(c, q),
    }
}

/// The arity of a block as an rb_lambda_call ifunc reports it: min, or -min-1 with a rest.
fn outer_arity(c: &Ctx, p: Value) -> i64 {
    match c.proc_data(p).blk.kind {
        BlockKind::Iseq(is) => c.iseq_ref(is).map_or(-1, |is| iseq_arity(is, false)),
        BlockKind::Proc(q) => outer_arity(c, q),
        _ => proc_arity(c, p),
    }
}

pub fn iseq_arity(is: &Iseq, lambda: bool) -> i64 {
    // rb_iseq_min_max_arity + rb_proc_arity / method_def_arity
    let p = &is.params;
    let req_kw = p.kw.iter().any(|k| k.required);
    let min = (p.lead.len() + p.post.len()) as i64 + if req_kw { 1 } else { 0 };
    if p.rest.is_some() { return -min - 1; }
    if !lambda { return min; }
    let max = (p.lead.len() + p.opt.len() + p.post.len()) as i64 + if !p.kw.is_empty() || p.kwrest.is_some() { 1 } else { 0 };
    if min == max { min } else { -min - 1 }
}

/// rb_iseq_min_max_arity: (min, max; None: unlimited).
fn iseq_min_max(is: &Iseq) -> (i64, Option<i64>) {
    let p = &is.params;
    let min = (p.lead.len() + p.post.len()) as i64 + if p.kw.iter().any(|k| k.required) { 1 } else { 0 };
    if p.rest.is_some() { return (min, None); }
    (min, Some((p.lead.len() + p.opt.len() + p.post.len()) as i64 + if !p.kw.is_empty() || p.kwrest.is_some() { 1 } else { 0 }))
}
/// rb_proc_min_max_arity of a lambda (a Method's proc: its method).
fn proc_min_max(c: &Ctx, p: Value) -> (i64, Option<i64>) {
    let is = match c.proc_data(p).blk.kind {
        BlockKind::Iseq(is) => Some(is),
        BlockKind::Method(m) => match c.me_ref(c.method_obj(m).me).map(|e| e.kind) {
            Some(MKind::Iseq(is)) => Some(is),
            Some(MKind::Bmethod(q)) => return proc_min_max(c, q),
            _ => None,
        },
        _ => None,
    };
    if let Some(i) = is.and_then(|is| c.iseq_ref(is)) { return iseq_min_max(i); }
    let a = proc_arity(c, p);
    if a >= 0 { (a, Some(a)) } else { (-a - 1, None) }
}

pub fn method_arity(c: &Ctx, me: *const MethodEntry) -> i64 {
    let Some(me) = c.me_ref(me) else { return -1 };
    match me.kind {
        MKind::Iseq(is) => c.iseq_ref(is).map_or(-1, |is| iseq_arity(is, true)),
        MKind::Cfunc(_, a) => a as i64,
        MKind::AttrReader(_) => 0,
        MKind::AttrWriter(_) => 1,
        MKind::Bmethod(p) => match c.proc_data(p).blk.kind { BlockKind::Iseq(is) => c.iseq_ref(is).map_or(-1, |is| iseq_arity(is, true)), _ => proc_arity(c, p) },
        _ => -1,
    }
}

/// Proc#parameters / Method#parameters of an iseq: built as symbols under the iseq borrow, then allocated.
pub fn params_list(c: &mut Ctx, is: *const Iseq, lambda: bool) -> Value {
    let mut rows: Vec<Vec<Value>> = vec![];
    if let Some(is) = c.iseq_ref(is) {
        let p = &is.params;
        let name_of = |slot: u32| -> Option<Sym> {
            if let Some(&(_, n)) = p.slot_names.iter().find(|x| x.0 == slot) { return Some(n); }
            for (n, sl, _) in &is.local_names { if *sl == slot as i32 { return Some(*n); } }
            // captured params: env copy recorded in env_params
            for (sl, ei) in &is.env_params { if *sl == slot { for (n, _, e) in &is.local_names { if *e == *ei as i32 { return Some(*n); } } } }
            None
        };
        let row = |kind: &str, slot: Option<u32>| -> Vec<Value> {
            let mut items = vec![sym_value(kind)];
            let anon = match kind { "rest" => Some("*"), "keyrest" => Some("**"), "block" => Some("&"), _ => None };
            let named = slot.and_then(name_of).filter(|n| !crate::symbol::sym_bytes(*n).starts_with(b"__"));
            if let Some(n) = named { items.push(Value::sym(n)); }
            else if let Some(a) = anon { if slot.is_none() || slot.and_then(name_of).is_some_and(|n| crate::symbol::sym_bytes(n).starts_with(b"__fwd")) { items.push(sym_value(a)); } }
            items
        };
        let req = if lambda { "req" } else { "opt" };
        for &l in &p.lead { rows.push(row(req, Some(l))); }
        for (l, _) in &p.opt { rows.push(row("opt", Some(*l))); }
        if let Some(r) = p.rest { rows.push(row("rest", if p.anon_rest { None } else { Some(r) })); }
        for &l in &p.post { rows.push(row(req, Some(l))); }
        for k in &p.kw { rows.push(vec![sym_value(if k.required { "keyreq" } else { "key" }), Value::sym(k.name)]); }
        if let Some(r) = p.kwrest { rows.push(row("keyrest", if p.anon_kwrest { None } else { Some(r) })); }
        if p.no_kw { rows.push(vec![sym_value("nokey")]); }
        if let Some(b) = p.block { rows.push(row("block", if p.anon_block { None } else { Some(b) })); }
    }
    rows_to_ary(c, rows)
}
/// Symbols only (immediates): no rooting needed while the rows are a Rust Vec.
fn rows_to_ary(c: &mut Ctx, rows: Vec<Vec<Value>>) -> Value {
    let out = c.ary_new_capa(rows.len());
    for r in rows { let e = c.ary_from_vec(r); c.ary_push(out, e); }
    out
}

fn proc_call(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let kw = kw_arg(c, a).is_some();
    call_proc_kw(c, slf, a, b, kw)
}
fn proc_arity_m(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(proc_arity(c, slf))) }
fn proc_lambda_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(c.proc_data(slf).is_lambda)) }
fn proc_to_proc(_c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(slf) }
fn proc_parameters(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (mut lambda, kind) = { let pd = c.proc_data(slf); (pd.is_lambda, pd.blk.kind) };
    if let Some(h) = kw_arg(c, a) { if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("lambda"))? { if v != NIL { lambda = v.truthy(); } } }
    match kind {
        BlockKind::Iseq(is) => Ok(params_list(c, is, lambda)),
        BlockKind::Sym(_) => Ok(rows_to_ary(c, vec![vec![sym_value("req")], vec![sym_value("rest")]])),
        BlockKind::Method(m) => method_parameters(c, m, &[], Blk::NONE),
        _ => Ok(rows_to_ary(c, vec![vec![sym_value("rest")]])),
    }
}
fn proc_curry(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let arity = if !a.is_empty() { a[0].as_fix() } else { let ar = proc_arity(c, slf); if ar < 0 { -ar - 1 } else { ar } };
    if c.proc_data(slf).is_lambda && !a.is_empty() {
        // rb_check_arity(sarity, min, max)
        let (min, max) = proc_min_max(c, slf);
        if arity < min || max.is_some_and(|m| arity > m) {
            let exp = match max { Some(m) if m == min => min.to_string(), Some(m) => format!("{}..{}", min, m), None => format!("{}+", min) };
            return arg_err(c, format!("wrong number of arguments (given {}, expected {})", arity, exp));
        }
    }
    let got = c.ary_new();
    make_curry(c, slf, arity, got)
}
fn curry_call(c: &mut Ctx, data: Value, args: &[Value], _blk: Blk, _o: Blk) -> RV {
    let p = c.ary_get(data, 0); let arity = c.ary_get(data, 1).as_fix(); let got = c.ary_get(data, 2);
    let mut v = c.ary(got).to_vec();
    v.extend_from_slice(args);
    let all = c.ary_from_vec(v);
    if (c.ary_len(all) as i64) >= arity { let v = c.ary(all).to_vec(); return call_proc(c, p, &v, std::ptr::null()); }
    make_curry(c, p, arity, all)
}
fn make_curry(c: &mut Ctx, p: Value, arity: i64, got: Value) -> RV {
    let data = c.ary_new_from(&[p, Value::fix(arity), got]);
    let b = Block::cfunc(c, curry_call, data);
    let lambda = c.proc_data(p).is_lambda;
    Ok(new_proc(c, c.vm().c_proc.get(), b, lambda, NIL))
}
fn proc_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let (kind, lambda) = { let pd = c.proc_data(slf); (pd.blk.kind, pd.is_lambda) };
    let loc = match kind { BlockKind::Iseq(is) => c.iseq_ref(is).map_or(String::new(), |is| format!(" {}:{}", is.info.file, is.info.line)), BlockKind::Sym(s) => { let i = crate::builtins::kernel::inspect_str(c, Value::sym(s))?; format!("(&{})", i) } _ => String::new() };
    let cls = class_path(c, real_class_of(c, slf));
    let t = format!("#<{}:{:#018x}{}{}>", cls, slf.raw(), loc, if lambda { " (lambda)" } else { "" });
    // rb_sprintf gives ASCII-8BIT; a non-ASCII name or path brings its own encoding in
    let enc = if t.is_ascii() { crate::object::ENC_BINARY } else { crate::object::ENC_UTF8 };
    Ok(c.str_new_enc(t.as_bytes(), enc))
}
fn callable_check(c: &mut Ctx, g: Value) -> Result<(), Esc> {
    if !is_proc(g) && !is_method_obj(g) && !respond_to(c, g, s::call, true) { return type_err(c, "callable object is expected"); }
    Ok(())
}
fn proc_compose_right(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    // (f >> g).call(x) = g.(f.(x))
    let g = a[0];
    callable_check(c, g)?;
    let data = c.ary_new_from(&[slf, g, FALSE]);
    let b = Block::cfunc(c, compose_call, data);
    let lambda = !is_proc(slf) || c.proc_data(slf).is_lambda;
    Ok(new_proc(c, c.vm().c_proc.get(), b, lambda, NIL))
}
fn proc_compose_left(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let g = a[0];
    callable_check(c, g)?;
    let data = c.ary_new_from(&[g, slf, TRUE]);
    let b = Block::cfunc(c, compose_call, data);
    // lambda-ness of the proc called first (rb_proc_compose_to_left)
    let lambda = !is_proc(g) || c.proc_data(g).is_lambda;
    Ok(new_proc(c, c.vm().c_proc.get(), b, lambda, NIL))
}
fn compose_call(c: &mut Ctx, data: Value, args: &[Value], blk: Blk, _o: Blk) -> RV {
    let kw = block_kw(c);
    let f = c.ary_get(data, 0); let g = c.ary_get(data, 1);
    let x = funcall_kw(c, f, s::call, args, blk, kw)?;
    funcall(c, g, s::call, &[x], std::ptr::null())
}
/// rb_proc_s_new: proc_new (a Proc of another class is copied into `slf`), then #initialize with the arguments.
fn proc_s_new(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if b.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
    let p = block_to_proc(c, b)?;
    if slf == c.vm().c_proc.get() && a.is_empty() { return Ok(p); }
    let np = if real_class_of(c, p) == slf { p } else {
        let np = crate::builtins::object::obj_dup(c, p, false, None)?;
        obj_set_klass(np, slf);
        np
    };
    let kw = kw_arg(c, a).is_some();
    funcall_kw(c, np, s::initialize, a, std::ptr::null(), kw)?;
    Ok(np)
}
fn proc_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if slf == a[0] { return Ok(TRUE); }
    if !is_proc(a[0]) { return Ok(FALSE); }
    let (x, y) = (c.proc_data(slf), c.proc_data(a[0]));
    let same = match (x.blk.kind, y.blk.kind) { (BlockKind::Iseq(p), BlockKind::Iseq(q)) => p == q && x.blk.env == y.blk.env && x.blk.self_ == y.blk.self_ && x.blk.home_id == y.blk.home_id, _ => false };
    Ok(Value::bool(same && x.is_lambda == y.is_lambda))
}
fn proc_hash(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let x = c.proc_data(slf);
    let h = match x.blk.kind { BlockKind::Iseq(p) => p as u64 ^ x.blk.env.raw(), _ => slf.raw() };
    Ok(Value::fix((crate::astrust::hash_u64(h) >> 2) as i64))
}
/// [file, line] of an iseq, or nil.
fn iseq_location(c: &mut Ctx, is: *const Iseq) -> RV {
    let Some((file, line)) = c.iseq_ref(is).map(|is| (is.info.file, is.info.line)) else { return Ok(NIL) };
    let f = c.str_new(file.as_bytes());
    Ok(c.ary_new_from(&[f, Value::fix(line as i64)]))
}
/// rb_proc_location of a block proc: (file, line), or None.
pub fn proc_location(c: &Ctx, p: Value) -> Option<(&'static str, u32)> {
    let BlockKind::Iseq(is) = c.proc_data(p).blk.kind else { return None };
    c.iseq_ref(is).map(|is| (is.info.file, is.info.line))
}
fn proc_source_location(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    match c.proc_data(slf).blk.kind {
        BlockKind::Iseq(is) => iseq_location(c, is),
        BlockKind::Method(m) => meth_source_location(c, m, &[], Blk::NONE),
        _ => Ok(NIL),
    }
}
fn proc_binding(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if !matches!(c.proc_data(slf).blk.kind, BlockKind::Iseq(_) | BlockKind::Method(_)) { return arg_err(c, "Can't create Binding from C level Proc"); }
    crate::builtins::binding::binding_of_proc(c, slf)
}
/// Proc#ruby2_keywords: flags the block's iseq (shared by dups), as rb_proc_ruby2_keywords.
fn proc_ruby2kw(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    check_frozen(c, slf)?;
    let BlockKind::Iseq(is) = c.proc_data(slf).blk.kind else {
        gvar_warn(c, "Skipping set of ruby2_keywords flag for proc (proc not defined in Ruby)", false)?;
        return Ok(slf);
    };
    let Some(is) = c.iseq_ref(is) else { return Ok(slf) };
    let p = &is.params;
    if p.rest.is_some() && p.post.is_empty() && p.kw.is_empty() && p.kwrest.is_none() && !p.no_kw { is.ruby2_keywords.set(true); }
    else { gvar_warn(c, "Skipping set of ruby2_keywords flag for proc (proc accepts keywords or post arguments or proc does not accept argument splat)", false)?; }
    Ok(slf)
}

// ---- Method / UnboundMethod -----------------------------------------------------------------

pub fn method_object(c: &mut Ctx, recv: Value, mid: Sym, public_only: bool) -> RV {
    let me = if c.refinements_on() { lookup_method_refined(c, c.cur_frame().map_or(std::ptr::null(), |f| f.cref()), class_of(recv), mid) } else { lookup_method(c, class_of(recv), mid) };
    if c.me_ref(me).is_none_or(|m| public_only && m.visi != VISI_PUBLIC) {
        // respond_to_missing? → a method that goes through method_missing
        if respond_to(c, recv, s::respond_to_missing_p, true) {
            let r = funcall(c, recv, s::respond_to_missing_p, &[Value::sym(mid), Value::bool(!public_only)], std::ptr::null())?;
            if r.truthy() {
                let mm = new_me(mid, class_of(recv), VISI_PUBLIC, MKind::Cfunc(mm_forward, -1));
                return Ok(new_method_obj(c, c.vm().c_method.get(), MethodObj { recv, me: mm, name: mid, unbound: false, owner: class_of(recv) }));
            }
        }
        // rb_method_name_error: the class of the receiver (its singleton class when it has one)
        let k = class_of(recv);
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("undefined method '{}' for class '{}'", sym_str(mid), class_path(c, k)), Value::sym(mid), recv)?;
        return Err(raise_exc(c, e));
    }
    let owner = me_owner(c, me);
    Ok(new_method_obj(c, c.vm().c_method.get(), MethodObj { recv, me, name: mid, unbound: false, owner }))
}

fn mm_forward(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let me = c.cur_frame().map_or(std::ptr::null(), |f| f.me);
    let Some(mid) = c.me_ref(me).map(|m| m.name) else { return arg_err(c, "no method name") };
    let mut args = vec![Value::sym(mid)];
    args.extend_from_slice(a);
    funcall(c, slf, s::method_missing, &args, b)
}

pub fn unbound_method_object(c: &mut Ctx, klass: Value, mid: Sym, public_only: bool) -> RV {
    // refinements active where instance_method is called
    let me = if c.refinements_on() { lookup_method_refined(c, c.caller_frame().map_or(std::ptr::null(), |f| f.cref()), klass, mid) } else { std::ptr::null() };
    let me = if me.is_null() { search_method(c, klass, mid) } else { me };
    if c.me_ref(me).is_none_or(|m| matches!(m.kind, MKind::Undef) || (public_only && m.visi != VISI_PUBLIC)) {
        let kind = if c.class(klass).is_module { "module" } else { "class" };
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("undefined method '{}' for {} '{}'", sym_str(mid), kind, class_path(c, klass)), Value::sym(mid), klass)?;
        return Err(raise_exc(c, e));
    }
    let owner = me_owner(c, me);
    Ok(new_method_obj(c, c.vm().c_unbound_method.get(), MethodObj { recv: klass, me, name: mid, unbound: true, owner }))
}

fn meth_call(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let m = mo(c, slf);
    let (recv, me) = (m.recv, m.me);
    let base = c.top();
    stack_check(c, a.len() + 4)?;
    base[0].set(recv); base[1].set(NIL);
    for (i, x) in a.iter().enumerate() { base[2 + i].set(*x); }
    c.set_sp(base, 2 + a.len());
    let kw = kw_arg(c, a).is_some();
    let r = invoke(c, me, recv, base, a.len() as u32, b, CF_FCALL | if kw { CF_KW } else { 0 });
    c.set_sp(base, 0);
    r
}
fn meth_to_proc(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let mut b = Block::cfunc(c, |_c, _d, _a, _b, _o| Ok(NIL), NIL);
    b.kind = BlockKind::Method(slf);
    b.self_ = c.method_obj(slf).recv;
    Ok(new_proc(c, c.vm().c_proc.get(), b, true, slf))
}
fn meth_arity(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(method_arity(c, c.method_obj(slf).me))) }
fn meth_name(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::sym(c.method_obj(slf).name)) }
fn meth_owner(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let me = c.method_obj(slf).me; Ok(zsuper_owner(me).unwrap_or_else(|| me_owner(c, me))) }
fn meth_receiver(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.method_obj(slf).recv) }
fn meth_unbind(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let m = mo(c, slf);
    Ok(new_method_obj(c, c.vm().c_unbound_method.get(), MethodObj { recv: real_class_of(c, m.recv), me: m.me, name: m.name, unbound: true, owner: m.owner }))
}
fn meth_original_name(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    Ok(c.me_ref(c.method_obj(slf).me).map_or(NIL, |m| Value::sym(m.name)))
}
/// Arity of a cfunc-like entry, or the iseq the entry runs (Method#parameters, #inspect).
enum Sig { Iseq(*const Iseq), Arity(i32), Other }
fn me_sig(c: &Ctx, me: *const MethodEntry) -> Sig {
    match c.me_ref(me).map(|m| m.kind) {
        Some(MKind::Iseq(is)) => Sig::Iseq(is),
        Some(MKind::Bmethod(p)) => match c.proc_data(p).blk.kind { BlockKind::Iseq(is) => Sig::Iseq(is), _ => Sig::Arity(-1) },
        Some(MKind::Cfunc(_, a)) => Sig::Arity(a),
        Some(MKind::AttrReader(_)) => Sig::Arity(0),
        Some(MKind::AttrWriter(_)) => Sig::Arity(1),
        _ => Sig::Other,
    }
}
pub fn method_parameters(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let me = c.method_obj(slf).me;
    let bmethod_noiseq = matches!(c.me_ref(me).map(|m| m.kind), Some(MKind::Bmethod(_)));
    match me_sig(c, me) {
        Sig::Iseq(is) => Ok(params_list(c, is, true)),
        Sig::Arity(_) if bmethod_noiseq => Ok(c.ary_new()),
        Sig::Arity(1) if matches!(c.me_ref(me).map(|m| m.kind), Some(MKind::AttrWriter(_))) => Ok(rows_to_ary(c, vec![vec![sym_value("req")]])),
        Sig::Arity(a) if matches!(c.me_ref(me).map(|m| m.kind), Some(MKind::Cfunc(..))) => {
            let rows = if a < 0 { vec![vec![sym_value("rest")]] } else { (0..a).map(|_| vec![sym_value("req")]).collect() };
            Ok(rows_to_ary(c, rows))
        }
        _ => Ok(c.ary_new()),
    }
}
fn meth_source_location(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let me = c.method_obj(slf).me;
    match c.me_ref(me).map(|m| m.kind) {
        Some(MKind::Iseq(is)) => iseq_location(c, is),
        Some(MKind::Bmethod(p)) => proc_source_location(c, p, a, b),
        // natively-defined Kernel methods that CRuby 4.0 writes in kernel.rb
        Some(MKind::Cfunc(..)) if me_owner(c, c.method_obj(slf).me) == c.vm().m_kernel.get() => {
            let line = match sym_str(c.method_obj(slf).name).as_str() { "tap" => 89, "then" | "yield_self" => 121, _ => return Ok(NIL) };
            let f = c.str_new(b"<internal:kernel>");
            Ok(c.ary_new_from(&[f, Value::fix(line)]))
        }
        _ => Ok(NIL),
    }
}
/// "(a, b=..., *r, k:, &blk) file:line" part of Method#inspect
fn meth_sig(c: &mut Ctx, slf: Value) -> Result<String, Esc> {
    let me = c.method_obj(slf).me;
    let is = match me_sig(c, me) {
        Sig::Iseq(is) => is,
        Sig::Arity(a) => { let inner = if a < 0 { "*".to_string() } else { vec!["_"; a as usize].join(", ") }; return Ok(format!("({})", inner)); }
        Sig::Other => return Ok("(*)".into()),
    };
    let Some((file, line, internal_arity, forwarding)) = c.iseq_ref(is).map(|is| (is.info.file, is.info.line, iseq_arity(is, true), is.params.forwarding)) else { return Ok("(*)".into()) };
    // prelude methods look like C methods
    if file.starts_with("<internal:") {
        let a = internal_arity;
        let inner = if a < 0 { "*".to_string() } else { vec!["_"; a as usize].join(", ") };
        return Ok(format!("({})", inner));
    }
    if forwarding { return Ok(format!("(...) {}:{}", file, line)); }
    let ps = method_parameters(c, slf, &[], Blk::NONE)?;
    let mut parts = vec![];
    for e in c.ary(ps) {
        let e = c.ary(*e);
        let kind = sym_str(e[0].as_sym()).to_string();
        let name = if e.len() > 1 { sym_str(e[1].as_sym()).to_string() } else { String::new() };
        parts.push(match kind.as_str() {
            "req" => if name.is_empty() { "_".into() } else { name },
            "opt" => format!("{}=...", if name.is_empty() { "_" } else { &name }),
            "rest" => if name == "*" { name } else { format!("*{}", name) },
            "keyreq" => format!("{}:", name),
            "key" => format!("{}: ...", name),
            "keyrest" => if name == "**" { name } else { format!("**{}", name) },
            "nokey" => "**nil".into(),
            "block" => if name == "&" { name } else { format!("&{}", name) },
            _ => name,
        });
    }
    let inner = if parts == ["*", "**", "&"] { "...".to_string() } else { parts.join(", ") };
    Ok(format!("({}) {}:{}", inner, file, line))
}
fn meth_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let m = mo(c, slf);
    // an alias shows the class of the method it names (method_inspect's defined_class)
    let is_alias = c.vm().alias_names.borrow().contains_key(&(m.me as usize));
    let (owner, me_name) = match c.me_ref(m.me) { Some(me) => (if is_alias { me.orig_owner } else { me.owner }, me.name), None => (NIL, m.name) };
    let sig = meth_sig(c, slf)?;
    let nm = if me_name != m.name { format!("{}({})", sym_str(m.name), sym_str(me_name)) } else { sym_str(m.name).to_string() };
    let s = if m.unbound {
        format!("#<UnboundMethod: {}#{}{}>", class_path(c, owner), nm, sig)
    } else if is_class_or_module(owner) && c.class(owner).is_singleton {
        let r = inspect_str(c, m.recv)?;
        format!("#<Method: {}.{}{}>", r, nm, sig)
    } else {
        // method_inspect's mklass: a class / module receiver is shown by its singleton class
        let rc = if is_class_or_module(m.recv) { singleton_class(c, m.recv) } else { real_class_of(c, m.recv) };
        if rc == owner { format!("#<Method: {}#{}{}>", class_path(c, owner), nm, sig) }
        else { format!("#<Method: {}({})#{}{}>", class_path(c, rc), class_path(c, owner), nm, sig) }
    };
    Ok(str_new_rb(c, &s))
}
/// The method definition behind an entry (rb_method_definition_eq / rb_hash_method_definition): aliases and
/// visibility copies share it; 0 for none.
fn def_key(c: &Ctx, me: *const MethodEntry) -> u64 {
    match c.me_ref(me).map(|m| m.kind) {
        Some(MKind::Iseq(p)) => p as u64,
        // a respond_to_missing? method is its name
        Some(MKind::Cfunc(f, _)) if f as *const () as usize == mm_forward as *const () as usize => c.me_ref(me).map_or(0, |m| (m.name.0 as u64) << 3 | 4),
        Some(MKind::Cfunc(f, _)) => f as usize as u64,
        Some(MKind::Bmethod(p)) => p.raw(),
        Some(MKind::AttrReader(s)) => (s.0 as u64) << 3 | 1,
        Some(MKind::AttrWriter(s)) => (s.0 as u64) << 3 | 2,
        _ => me as u64,
    }
}
/// Method#== / UnboundMethod#==: the same kind of object, definition and (Method) receiver.
fn meth_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if !is_method_obj(a[0]) { return Ok(FALSE); }
    let (x, y) = (mo(c, slf), mo(c, a[0]));
    if real_class_of(c, slf) != real_class_of(c, a[0]) || x.unbound != y.unbound { return Ok(FALSE); }
    let same_body = x.me == y.me || def_key(c, x.me) == def_key(c, y.me);
    Ok(Value::bool(same_body && (x.unbound || x.recv == y.recv)))
}
fn meth_hash(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let m = mo(c, slf);
    let r = if m.unbound { 0 } else { m.recv.raw() };
    Ok(Value::fix((crate::astrust::hash_u64(r ^ def_key(c, m.me).rotate_left(17)) >> 2) as i64))
}
fn meth_super_method(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let m = mo(c, slf);
    let klass = if m.unbound { m.recv } else { class_of(m.recv) };
    // CRuby: an alias of a class's method is a copy defined in the aliasing class (unlike `super` itself)
    let (owner, orig) = c.me_ref(m.me).map_or((NIL, NIL), |e| (e.owner, e.orig_owner));
    let is_alias = c.vm().alias_names.borrow().contains_key(&(m.me as usize));
    let from = if is_alias && is_class_or_module(orig) && !c.class(orig).is_module { owner } else { orig };
    let mid = c.me_ref(m.me).map_or(m.name, |e| e.name);   // an alias searches for the original name
    let sm = super_method(c, klass, from, mid);
    if sm.is_null() { return Ok(NIL); }
    let owner = me_owner(c, sm);
    Ok(new_method_obj(c, obj_klass(slf), MethodObj { recv: m.recv, me: sm, name: mid, unbound: m.unbound, owner }))
}
fn meth_curry(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let p = meth_to_proc(c, slf, &[], b)?;
    proc_curry(c, p, a, b)
}
fn um_bind(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let m = mo(c, slf);
    let owner = me_owner(c, m.me);
    let (is_module, is_singleton) = if is_class_or_module(owner) { let k = c.class(owner); (k.is_module, k.is_singleton) } else { (false, false) };
    if !is_module && !is_kind_of(c, a[0], owner) {
        if is_singleton { return type_err(c, "singleton method called for a different object"); }
        return type_err(c, format!("bind argument must be an instance of {}", class_path(c, owner)));
    }
    Ok(new_method_obj(c, c.vm().c_method.get(), MethodObj { recv: a[0], me: m.me, name: m.name, unbound: false, owner: m.owner }))
}
fn um_bind_call(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let m = um_bind(c, slf, &a[..1], b)?;
    meth_call(c, m, &a[1..], b)
}

/// Boot: methods of Proc, Method and UnboundMethod.
pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let p = vmr.c_proc.get();
    define_cfunc(c, p, "call", proc_call, -1);
    define_cfunc(c, p, "[]", proc_call, -1);
    define_cfunc(c, p, "===", proc_call, -1);
    define_cfunc(c, p, "yield", proc_call, -1);
    define_cfunc(c, p, "arity", proc_arity_m, 0);
    define_cfunc(c, p, "lambda?", proc_lambda_p, 0);
    define_cfunc(c, p, "to_proc", proc_to_proc, 0);
    define_cfunc(c, p, "parameters", proc_parameters, -1);
    define_cfunc(c, p, "curry", proc_curry, -1);
    define_cfunc(c, p, "inspect", proc_inspect, 0);
    define_cfunc(c, p, "to_s", proc_inspect, 0);
    define_cfunc(c, p, ">>", proc_compose_right, 1);
    define_cfunc(c, p, "<<", proc_compose_left, 1);
    define_cfunc(c, p, "==", proc_eq, 1);
    define_cfunc(c, p, "eql?", proc_eq, 1);
    define_cfunc(c, p, "hash", proc_hash, 0);
    define_cfunc(c, p, "source_location", proc_source_location, 0);
    define_cfunc(c, p, "binding", proc_binding, 0);
    define_cfunc(c, p, "ruby2_keywords", proc_ruby2kw, 0);
    define_singleton_cfunc(c, p, "new", proc_s_new, -1);
    // CRuby defines these on the classes (same bodies as Kernel's)
    for k in [p, vmr.c_method.get(), vmr.c_unbound_method.get(), vmr.c_binding.get()] {
        define_cfunc(c, k, "clone", crate::builtins::object::k_clone, -1);
        define_cfunc(c, k, "dup", crate::builtins::object::k_dup, 0);
    }

    for k in [vmr.c_method.get(), vmr.c_unbound_method.get()] {
        define_cfunc(c, k, "arity", meth_arity, 0);
        define_cfunc(c, k, "name", meth_name, 0);
        define_cfunc(c, k, "owner", meth_owner, 0);
        define_cfunc(c, k, "original_name", meth_original_name, 0);
        define_cfunc(c, k, "parameters", method_parameters, 0);
        define_cfunc(c, k, "source_location", meth_source_location, 0);
        define_cfunc(c, k, "inspect", meth_inspect, 0);
        define_cfunc(c, k, "to_s", meth_inspect, 0);
        define_cfunc(c, k, "==", meth_eq, 1);
        define_cfunc(c, k, "eql?", meth_eq, 1);
        define_cfunc(c, k, "hash", meth_hash, 0);
        define_cfunc(c, k, "super_method", meth_super_method, 0);
    }
    let m = vmr.c_method.get();
    define_cfunc(c, m, "call", meth_call, -1);
    define_cfunc(c, m, ">>", proc_compose_right, 1);
    define_cfunc(c, m, "<<", proc_compose_left, 1);
    define_cfunc(c, m, "[]", meth_call, -1);
    define_cfunc(c, m, "===", meth_call, -1);
    define_cfunc(c, m, "to_proc", meth_to_proc, 0);
    define_cfunc(c, m, "receiver", meth_receiver, 0);
    define_cfunc(c, m, "unbind", meth_unbind, 0);
    define_cfunc(c, m, "curry", meth_curry, -1);
    let u = vmr.c_unbound_method.get();
    define_cfunc(c, u, "bind", um_bind, 1);
    define_cfunc(c, u, "bind_call", um_bind_call, -1);
    let _ = intern;
}
