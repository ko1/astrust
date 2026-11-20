// Proc / Method / UnboundMethod.
use crate::array::*;
use crate::builtins::kernel::*;
use crate::string::*;
use crate::symbol::{intern, s, sym_str};
use crate::value::*;
use crate::vm::*;

fn sym_proc_call(c: &mut Ctx, data: Value, args: &[Value], blk: *const Block) -> RV {
    if args.is_empty() { return arg_err(c, "no receiver given"); }
    let mid = data.as_sym();
    funcall_public(c, args[0], mid, &args[1..], blk)
}

pub fn sym_to_proc(c: &mut Ctx, sym: Value) -> RV {
    if let Some(&p) = vm().symproc_cache.get(&sym.as_sym()) { return Ok(p); }
    let mut b = Block::cfunc(c, sym_proc_call, sym);
    b.kind = BlockKind::Sym(sym.as_sym());
    let p = crate::gc::alloc(vm().c_proc, Body::Proc(Box::new(ProcData { blk: b, is_lambda: true, from_method: NIL })));
    rproc(p).blk.proc_val = p;
    vm().symproc_cache.insert(sym.as_sym(), p);
    Ok(p)
}

pub fn proc_arity(p: Value) -> i64 {
    let pd = rproc(p);
    match pd.blk.kind {
        BlockKind::Iseq(is) => {
            let is = unsafe { &*is };
            iseq_arity(is, pd.is_lambda)
        }
        BlockKind::Sym(_) => -2,
        BlockKind::Method(m) => method_arity(rmethod(m).me),
        BlockKind::Cfunc(..) => -1,
        BlockKind::Proc(q) => proc_arity(q),
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

pub fn method_arity(me: *const MethodEntry) -> i64 {
    let me = unsafe { &*me };
    match me.kind {
        MKind::Iseq(is) => iseq_arity(unsafe { &*is }, true),
        MKind::Cfunc(_, a) => a as i64,
        MKind::AttrReader(_) => 0,
        MKind::AttrWriter(_) => 1,
        MKind::Bmethod(p) => match rproc(p).blk.kind { BlockKind::Iseq(is) => iseq_arity(unsafe { &*is }, true), _ => proc_arity(p) },
        _ => -1,
    }
}

fn params_list(is: &Iseq, lambda: bool) -> Value {
    let out = ary_new();
    let p = &is.params;
    let name_of = |slot: u32| -> Option<Sym> {
        if let Some(&(_, n)) = p.slot_names.iter().find(|x| x.0 == slot) { return Some(n); }
        for (n, sl, _) in &is.local_names { if *sl == slot as i32 { return Some(*n); } }
        // captured params: env copy recorded in env_params
        for (sl, ei) in &is.env_params { if *sl == slot { for (n, _, e) in &is.local_names { if *e == *ei as i32 { return Some(*n); } } } }
        None
    };
    let push = |kind: &str, slot: Option<u32>| {
        let mut items = vec![crate::vm::sym_value(kind)];
        let anon = match kind { "rest" => Some("*"), "keyrest" => Some("**"), "block" => Some("&"), _ => None };
        let named = slot.and_then(name_of).filter(|n| !crate::symbol::sym_bytes(*n).starts_with(b"__"));
        if let Some(n) = named { items.push(Value::sym(n)); }
        else if let Some(a) = anon { if slot.is_none() || slot.and_then(name_of).is_some_and(|n| crate::symbol::sym_bytes(n).starts_with(b"__fwd")) { items.push(crate::vm::sym_value(a)); } }
        let e = ary_new_from(&items);
        ary_push(out, e);
    };
    let req = if lambda { "req" } else { "opt" };
    for &l in &p.lead { push(req, Some(l)); }
    for (l, _) in &p.opt { push("opt", Some(*l)); }
    if let Some(r) = p.rest { push("rest", if p.anon_rest { None } else { Some(r) }); }
    for &l in &p.post { push(req, Some(l)); }
    for k in &p.kw { let e = ary_new_from(&[crate::vm::sym_value(if k.required { "keyreq" } else { "key" }), Value::sym(k.name)]); ary_push(out, e); }
    if let Some(r) = p.kwrest { push("keyrest", if p.anon_kwrest { None } else { Some(r) }); }
    if p.no_kw { let e = ary_new_from(&[crate::vm::sym_value("nokey")]); ary_push(out, e); }
    if let Some(b) = p.block { push("block", if p.anon_block { None } else { Some(b) }); }
    out
}

fn proc_call(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let kw = kw_arg(a).is_some();
    call_proc_kw(c, slf, a, b, kw)
}
fn proc_arity_m(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(proc_arity(slf))) }
fn proc_lambda_p(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(rproc(slf).is_lambda)) }
fn proc_to_proc(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(slf) }
fn proc_parameters(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let pd = rproc(slf);
    let mut lambda = pd.is_lambda;
    if let Some(h) = kw_arg(a) { if let Some(v) = crate::hash::hash_get_opt(c, h, sym_value("lambda"))? { if v != NIL { lambda = v.truthy(); } } }
    match pd.blk.kind {
        BlockKind::Iseq(is) => Ok(params_list(unsafe { &*is }, lambda)),
        BlockKind::Sym(_) => Ok(ary_from_vec(vec![ary_new_from(&[sym_value("req")]), ary_new_from(&[sym_value("rest")])])),
        BlockKind::Method(m) => method_parameters(c, m, &[], std::ptr::null()),
        _ => Ok(ary_from_vec(vec![ary_new_from(&[sym_value("rest")])])),
    }
}
fn proc_curry(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let arity = if !a.is_empty() { a[0].as_fix() } else { let ar = proc_arity(slf); if ar < 0 { -ar - 1 } else { ar } };
    if rproc(slf).is_lambda && !a.is_empty() {
        let ar = proc_arity(slf);
        if (ar >= 0 && arity != ar) || (ar < 0 && arity < -ar - 1) { return arg_err(c, format!("wrong number of arguments (given {}, expected {})", arity, if ar >= 0 { ar.to_string() } else { format!("{}+", -ar - 1) })); }
    }
    make_curry(c, slf, arity, ary_new())
}
fn curry_call(c: &mut Ctx, data: Value, args: &[Value], _blk: *const Block) -> RV {
    let p = ary_get(data, 0); let arity = ary_get(data, 1).as_fix(); let got = ary_get(data, 2);
    let all = ary_new_from(&rary(got).clone());
    for &x in args { ary_push(all, x); }
    if (ary_len(all) as i64) >= arity { let v = rary(all).clone(); return call_proc(c, p, &v, std::ptr::null()); }
    make_curry(c, p, arity, all)
}
fn make_curry(c: &mut Ctx, p: Value, arity: i64, got: Value) -> RV {
    let data = ary_new_from(&[p, Value::fix(arity), got]);
    let b = Block::cfunc(c, curry_call, data);
    let np = crate::gc::alloc(vm().c_proc, Body::Proc(Box::new(ProcData { blk: b, is_lambda: rproc(p).is_lambda, from_method: NIL })));
    rproc(np).blk.proc_val = np;
    Ok(np)
}
fn proc_inspect(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let pd = rproc(slf);
    let loc = match pd.blk.kind { BlockKind::Iseq(is) => { let is = unsafe { &*is }; format!(" {}:{}", is.info.file, is.info.line) } BlockKind::Sym(s) => format!("(&:{})", sym_str(s)), _ => String::new() };
    Ok(str_new(format!("#<Proc:{:#018x}{}{}>", slf.raw(), loc, if pd.is_lambda { " (lambda)" } else { "" }).as_bytes()))
}
fn proc_compose_right(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    // (f >> g).call(x) = g.(f.(x))
    let g = a[0];
    if !is_proc(g) && !is_method_obj(g) && !respond_to(g, s::call, true) { return type_err(c, "callable object is expected"); }
    let data = ary_new_from(&[slf, g, FALSE]);
    let b = Block::cfunc(c, compose_call, data);
    let np = crate::gc::alloc(vm().c_proc, Body::Proc(Box::new(ProcData { blk: b, is_lambda: !is_proc(slf) || rproc(slf).is_lambda, from_method: NIL })));
    rproc(np).blk.proc_val = np;
    Ok(np)
}
fn proc_compose_left(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let g = a[0];
    if !is_proc(g) && !is_method_obj(g) && !respond_to(g, s::call, true) { return type_err(c, "callable object is expected"); }
    let data = ary_new_from(&[g, slf, TRUE]);
    let b = Block::cfunc(c, compose_call, data);
    let np = crate::gc::alloc(vm().c_proc, Body::Proc(Box::new(ProcData { blk: b, is_lambda: true, from_method: NIL })));
    rproc(np).blk.proc_val = np;
    Ok(np)
}
fn compose_call(c: &mut Ctx, data: Value, args: &[Value], blk: *const Block) -> RV {
    let f = ary_get(data, 0); let g = ary_get(data, 1);
    let x = funcall(c, f, s::call, args, blk)?;
    funcall(c, g, s::call, &[x], std::ptr::null())
}
fn proc_s_new(c: &mut Ctx, slf: Value, _a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
    let p = block_to_proc(c, b)?;
    if slf != vm().c_proc {
        let np = crate::builtins::object::obj_dup(c, p, false, None)?;
        obj(np).klass = slf;
        funcall(c, np, s::initialize, &[], std::ptr::null())?;
        return Ok(np);
    }
    Ok(p)
}
fn proc_eq(_c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if slf == a[0] { return Ok(TRUE); }
    if !is_proc(a[0]) { return Ok(FALSE); }
    let (x, y) = (rproc(slf), rproc(a[0]));
    let same = match (x.blk.kind, y.blk.kind) { (BlockKind::Iseq(p), BlockKind::Iseq(q)) => p == q && x.blk.env == y.blk.env && x.blk.self_ == y.blk.self_, _ => false };
    Ok(Value::bool(same && x.is_lambda == y.is_lambda))
}
fn proc_hash(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let x = rproc(slf);
    let h = match x.blk.kind { BlockKind::Iseq(p) => p as u64 ^ x.blk.env.raw(), _ => slf.raw() };
    Ok(Value::fix((crate::astrust::hash_u64(h) >> 2) as i64))
}
fn proc_source_location(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    match rproc(slf).blk.kind {
        BlockKind::Iseq(is) => { let is = unsafe { &*is }; Ok(ary_new_from(&[str_new(is.info.file.as_bytes()), Value::fix(is.info.line as i64)])) }
        _ => Ok(NIL),
    }
}
fn proc_binding(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { crate::builtins::binding::binding_of_proc(c, slf) }
fn proc_ruby2kw(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(slf) }

// ---- Method / UnboundMethod -----------------------------------------------------------------

pub fn method_object(c: &mut Ctx, recv: Value, mid: Sym, public_only: bool) -> RV {
    let me = if unsafe { REFINEMENTS_ON } { lookup_method_refined(unsafe { (*c.frame).cref }, class_of(recv), mid) } else { lookup_method(class_of(recv), mid) };
    if me.is_null() || (public_only && unsafe { (*me).visi } != VISI_PUBLIC) {
        // respond_to_missing? → a method that goes through method_missing
        if respond_to(recv, s::respond_to_missing_p, true) {
            let r = funcall(c, recv, s::respond_to_missing_p, &[Value::sym(mid), TRUE], std::ptr::null())?;
            if r.truthy() {
                let mm = new_me(mid, class_of(recv), VISI_PUBLIC, MKind::Cfunc(mm_forward, -1));
                return Ok(crate::gc::alloc(vm().c_method, Body::Method(Box::new(MethodObj { recv, me: mm, name: mid, unbound: false, owner: class_of(recv) }))));
            }
        }
        let desc = nomethod_desc(c, recv)?;
        let e = make_name_error(c, vm().c_name_error, &format!("undefined method '{}' for {}", sym_str(mid), desc), Value::sym(mid), recv)?;
        return Err(raise_exc(c, e));
    }
    Ok(crate::gc::alloc(vm().c_method, Body::Method(Box::new(MethodObj { recv, me, name: mid, unbound: false, owner: unsafe { (*me).owner } }))))
}

fn mm_forward(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let f = c.frame();
    let mid = unsafe { (*f.me).name };
    let mut args = vec![Value::sym(mid)];
    args.extend_from_slice(a);
    funcall(c, slf, s::method_missing, &args, b)
}

pub fn unbound_method_object(c: &mut Ctx, klass: Value, mid: Sym, public_only: bool) -> RV {
    let me = search_method(klass, mid);
    if me.is_null() || matches!(unsafe { (*me).kind }, MKind::Undef) || (public_only && unsafe { (*me).visi } != VISI_PUBLIC) {
        let kind = if rclass(klass).is_module { "module" } else { "class" };
        let e = make_name_error(c, vm().c_name_error, &format!("undefined method '{}' for {} '{}'", sym_str(mid), kind, class_path(klass)), Value::sym(mid), klass)?;
        return Err(raise_exc(c, e));
    }
    Ok(crate::gc::alloc(vm().c_unbound_method, Body::Method(Box::new(MethodObj { recv: klass, me, name: mid, unbound: true, owner: unsafe { (*me).owner } }))))
}

fn meth_call(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let m = rmethod(slf);
    let (recv, me) = (m.recv, m.me);
    let base = c.sp;
    stack_check(c, a.len() + 4)?;
    unsafe {
        *base = recv; *base.add(1) = NIL;
        for (i, x) in a.iter().enumerate() { *base.add(2 + i) = *x; }
        c.sp = base.add(2 + a.len());
    }
    let kw = kw_arg(a).is_some();
    let r = invoke(c, me, recv, base, a.len() as u32, b, CF_FCALL | if kw { CF_KW } else { 0 });
    c.sp = base;
    r
}
fn meth_to_proc(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let mut b = Block::cfunc(c, |_c, _d, _a, _b| Ok(NIL), NIL);
    b.kind = BlockKind::Method(slf);
    b.self_ = rmethod(slf).recv;
    let p = crate::gc::alloc(vm().c_proc, Body::Proc(Box::new(ProcData { blk: b, is_lambda: true, from_method: slf })));
    rproc(p).blk.proc_val = p;
    Ok(p)
}
fn meth_arity(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(method_arity(rmethod(slf).me))) }
fn meth_name(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::sym(rmethod(slf).name)) }
fn meth_owner(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(unsafe { (*rmethod(slf).me).owner }) }
fn meth_receiver(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(rmethod(slf).recv) }
fn meth_unbind(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let m = rmethod(slf);
    Ok(crate::gc::alloc(vm().c_unbound_method, Body::Method(Box::new(MethodObj { recv: real_class_of(m.recv), me: m.me, name: m.name, unbound: true, owner: m.owner }))))
}
fn meth_original_name(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::sym(unsafe { (*rmethod(slf).me).name })) }
pub fn method_parameters(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let me = unsafe { &*rmethod(slf).me };
    match me.kind {
        MKind::Iseq(is) => Ok(params_list(unsafe { &*is }, true)),
        MKind::Bmethod(p) => match rproc(p).blk.kind { BlockKind::Iseq(is) => Ok(params_list(unsafe { &*is }, true)), _ => Ok(ary_new()) },
        MKind::AttrWriter(_) => Ok(ary_from_vec(vec![ary_new_from(&[sym_value("req")])])),
        MKind::Cfunc(_, a) => {
            let out = ary_new();
            if a < 0 { ary_push(out, ary_new_from(&[sym_value("rest")])); } else { for _ in 0..a { ary_push(out, ary_new_from(&[sym_value("req")])); } }
            Ok(out)
        }
        _ => Ok(ary_new()),
    }
}
fn meth_source_location(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let me = unsafe { &*rmethod(slf).me };
    match me.kind {
        MKind::Iseq(is) => { let is = unsafe { &*is }; Ok(ary_new_from(&[str_new(is.info.file.as_bytes()), Value::fix(is.info.line as i64)])) }
        MKind::Bmethod(p) => proc_source_location(_c, p, _a, _b),
        _ => Ok(NIL),
    }
}
/// "(a, b=..., *r, k:, &blk) file:line" part of Method#inspect
fn meth_sig(c: &mut Ctx, slf: Value) -> Result<String, Esc> {
    let me = unsafe { &*rmethod(slf).me };
    let (iseq, cfunc_arity): (Option<&Iseq>, Option<i32>) = match me.kind {
        MKind::Iseq(is) => (Some(unsafe { &*is }), None),
        MKind::Bmethod(p) => match rproc(p).blk.kind { BlockKind::Iseq(is) => (Some(unsafe { &*is }), None), _ => (None, Some(-1)) },
        MKind::Cfunc(_, a) => (None, Some(a)),
        MKind::AttrReader(_) => (None, Some(0)),
        MKind::AttrWriter(_) => (None, Some(1)),
        _ => (None, Some(-1)),
    };
    if let Some(a) = cfunc_arity {
        let inner = if a < 0 { "*".to_string() } else { vec!["_"; a as usize].join(", ") };
        return Ok(format!("({})", inner));
    }
    let is = iseq.unwrap();
    // prelude methods look like C methods
    if is.info.file.starts_with("<internal:") {
        let a = iseq_arity(is, true);
        let inner = if a < 0 { "*".to_string() } else { vec!["_"; a as usize].join(", ") };
        return Ok(format!("({})", inner));
    }
    if is.params.forwarding { return Ok(format!("(...) {}:{}", is.info.file, is.info.line)); }
    let ps = method_parameters(c, slf, &[], std::ptr::null())?;
    let mut parts = vec![];
    for i in 0..ary_len(ps) {
        let e = ary_get(ps, i);
        let kind = sym_str(ary_get(e, 0).as_sym()).to_string();
        let name = if ary_len(e) > 1 { sym_str(ary_get(e, 1).as_sym()).to_string() } else { String::new() };
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
    Ok(format!("({}) {}:{}", inner, is.info.file, is.info.line))
}
fn meth_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let m = rmethod(slf);
    let me = unsafe { &*m.me };
    let owner = me.owner;
    let sig = meth_sig(c, slf)?;
    let nm = if me.name != m.name { format!("{}({})", sym_str(m.name), sym_str(me.name)) } else { sym_str(m.name).to_string() };
    let s = if m.unbound {
        format!("#<UnboundMethod: {}#{}{}>", class_path(owner), nm, sig)
    } else if rclass(owner).is_singleton {
        let r = inspect_str(c, m.recv)?;
        format!("#<Method: {}.{}{}>", r, nm, sig)
    } else {
        let rc = real_class_of(m.recv);
        if rc == owner { format!("#<Method: {}#{}{}>", class_path(owner), nm, sig) }
        else { format!("#<Method: {}({})#{}{}>", class_path(rc), class_path(owner), nm, sig) }
    };
    Ok(str_new(s.as_bytes()))
}
fn meth_eq(_c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if !is_method_obj(a[0]) { return Ok(FALSE); }
    let (x, y) = (rmethod(slf), rmethod(a[0]));
    let same_body = x.me == y.me || unsafe { match ((*x.me).kind, (*y.me).kind) { (MKind::Iseq(p), MKind::Iseq(q)) => p == q, (MKind::Cfunc(f, _), MKind::Cfunc(g, _)) => f as usize == g as usize, _ => false } };
    Ok(Value::bool(x.recv == y.recv && same_body && obj(slf).klass == obj(a[0]).klass))
}
fn meth_hash(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let m = rmethod(slf);
    Ok(Value::fix((crate::astrust::hash_u64(m.recv.raw() ^ (m.me as u64)) >> 2) as i64))
}
fn meth_super_method(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let m = rmethod(slf);
    let klass = if m.unbound { m.recv } else { class_of(m.recv) };
    let sm = super_method(klass, unsafe { (*m.me).owner }, m.name);
    if sm.is_null() { return Ok(NIL); }
    let _ = c;
    Ok(crate::gc::alloc(obj(slf).klass, Body::Method(Box::new(MethodObj { recv: m.recv, me: sm, name: m.name, unbound: m.unbound, owner: unsafe { (*sm).owner } }))))
}
fn meth_curry(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let p = meth_to_proc(c, slf, &[], b)?;
    proc_curry(c, p, a, b)
}
fn um_bind(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let m = rmethod(slf);
    let owner = unsafe { (*m.me).owner };
    if !rclass(owner).is_module && !is_kind_of(a[0], owner) && !(rclass(owner).is_singleton) {
        return type_err(c, format!("bind argument must be an instance of {}", class_path(owner)));
    }
    Ok(crate::gc::alloc(vm().c_method, Body::Method(Box::new(MethodObj { recv: a[0], me: m.me, name: m.name, unbound: false, owner: m.owner }))))
}
fn um_bind_call(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let m = um_bind(c, slf, &a[..1], b)?;
    meth_call(c, m, &a[1..], b)
}

pub fn init() {
    let vm = vm();
    let p = vm.c_proc;
    define_cfunc(p, "call", proc_call, -1);
    define_cfunc(p, "()", proc_call, -1);
    define_cfunc(p, "[]", proc_call, -1);
    define_cfunc(p, "===", proc_call, -1);
    define_cfunc(p, "yield", proc_call, -1);
    define_cfunc(p, "arity", proc_arity_m, 0);
    define_cfunc(p, "lambda?", proc_lambda_p, 0);
    define_cfunc(p, "to_proc", proc_to_proc, 0);
    define_cfunc(p, "parameters", proc_parameters, -1);
    define_cfunc(p, "curry", proc_curry, -1);
    define_cfunc(p, "inspect", proc_inspect, 0);
    define_cfunc(p, "to_s", proc_inspect, 0);
    define_cfunc(p, ">>", proc_compose_right, 1);
    define_cfunc(p, "<<", proc_compose_left, 1);
    define_cfunc(p, "==", proc_eq, 1);
    define_cfunc(p, "eql?", proc_eq, 1);
    define_cfunc(p, "hash", proc_hash, 0);
    define_cfunc(p, "source_location", proc_source_location, 0);
    define_cfunc(p, "binding", proc_binding, 0);
    define_cfunc(p, "ruby2_keywords", proc_ruby2kw, 0);
    define_singleton_cfunc(p, "new", proc_s_new, -1);

    for k in [vm.c_method, vm.c_unbound_method] {
        define_cfunc(k, "arity", meth_arity, 0);
        define_cfunc(k, "name", meth_name, 0);
        define_cfunc(k, "owner", meth_owner, 0);
        define_cfunc(k, "original_name", meth_original_name, 0);
        define_cfunc(k, "parameters", method_parameters, 0);
        define_cfunc(k, "source_location", meth_source_location, 0);
        define_cfunc(k, "inspect", meth_inspect, 0);
        define_cfunc(k, "to_s", meth_inspect, 0);
        define_cfunc(k, "==", meth_eq, 1);
        define_cfunc(k, "eql?", meth_eq, 1);
        define_cfunc(k, "hash", meth_hash, 0);
        define_cfunc(k, "super_method", meth_super_method, 0);
    }
    let m = vm.c_method;
    define_cfunc(m, "call", meth_call, -1);
    define_cfunc(m, ">>", proc_compose_right, 1);
    define_cfunc(m, "<<", proc_compose_left, 1);
    define_cfunc(m, "()", meth_call, -1);
    define_cfunc(m, "[]", meth_call, -1);
    define_cfunc(m, "===", meth_call, -1);
    define_cfunc(m, "to_proc", meth_to_proc, 0);
    define_cfunc(m, "receiver", meth_receiver, 0);
    define_cfunc(m, "unbind", meth_unbind, 0);
    define_cfunc(m, "curry", meth_curry, -1);
    let u = vm.c_unbound_method;
    define_cfunc(u, "bind", um_bind, 1);
    define_cfunc(u, "bind_call", um_bind_call, -1);
    let _ = intern;
}
