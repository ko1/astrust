// BasicObject / Object (Kernel instance methods) / Module / Class.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{is_heap_float, obj_clear_flags, obj_flags, obj_klass, obj_set_flags, obj_set_klass};
use crate::array::*;
use crate::builtins::kernel::*;
use crate::hash::*;
use crate::string::*;
use crate::symbol::{intern, s, sym_bytes, sym_str};
use crate::value::*;
use crate::vm::*;

pub fn to_sym_arg(c: &mut Ctx, v: Value) -> Result<Sym, Esc> {
    if v.is_sym() { return Ok(v.as_sym()); }
    if is_string(v) { return Ok(intern(c.str_bytes(v))); }
    if v.is_heap() && respond_to(c, v, s::to_str, true) { let s2 = expect_str(c, v)?; return Ok(intern(c.str_bytes(s2))); }
    let ins = inspect_str(c, v)?;
    type_err(c, format!("{} is not a symbol nor a string", ins))
}

// ---- BasicObject --------------------------------------------------------------------

fn bo_initialize(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(NIL) }
/// BasicObject#== entry (function pointers are not reliably unique, so identify it by entry).
pub fn bo_eq(_c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(s == a[0] || (s.is_flonum() && false))) }
fn bo_not(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(s.falsy())) }
fn bo_neq(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let r = funcall(c, s, crate::symbol::s::eq, &[a[0]], std::ptr::null())?;
    Ok(Value::bool(r.falsy()))
}
fn bo_id(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(c.int_from_i128(object_id(s) as i128)) }
fn bo_send(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if a.is_empty() { return arg_err(c, "no method name given"); }
    let mid = to_sym_arg(c, a[0])?;
    let kw = kw_arg(a).is_some();
    if c.refinements_on() {
        if let Some(r) = funcall_refined(c, std::ptr::null(), slf, mid, &a[1..], b, CF_FCALL | if kw { CF_KW } else { 0 }) { return r; }
    }
    funcall_kw(c, slf, mid, &a[1..], b, kw)
}
fn k_public_send(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if a.is_empty() { return arg_err(c, "no method name given"); }
    let mid = to_sym_arg(c, a[0])?;
    let kw = kw_arg(a).is_some();
    if c.refinements_on() {
        if let Some(r) = funcall_refined(c, std::ptr::null(), slf, mid, &a[1..], b, if kw { CF_KW } else { 0 }) { return r; }
    }
    funcall_flags(c, slf, mid, &a[1..], b, if kw { CF_KW } else { 0 })
}
fn bo_instance_eval(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if b.is_null() {
        if a.is_empty() { return argc_err(c, 0, 1, 3); }
        let src = expect_str(c, a[0])?;
        let bytes = c.str_bytes(src).to_vec();
        let file: &'static str = if a.len() > 1 { Box::leak(to_rstring(c, a[1])?.into_boxed_str()) } else { crate::builtins::binding::eval_file(c) };
        return crate::builtins::binding::eval_with_self(c, slf, &bytes, file, if a.len() > 2 { a[2].as_fix() as i32 } else { 1 });
    }
    let dc = if slf.is_heap() && !slf.is_flonum() { singleton_class(c, slf) } else { NIL };
    yield_with_self(c, b, &[slf], slf, dc)
}
fn bo_instance_exec(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return err(c, c.vm().c_local_jump_error.get(), "no block given (yield)"); }
    let dc = if slf.is_heap() { singleton_class(c, slf) } else { NIL };
    yield_with_self(c, b, a, slf, dc)
}
fn bo_method_missing(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return arg_err(c, "no method name given"); }
    if !a[0].is_sym() { return arg_err(c, format!("method name must be a Symbol but {} is given", class_name_for_err(c, a[0]))); }
    no_method_error(c, slf, a[0].as_sym(), 0)
}
fn bo_singleton_method_added(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(NIL) }

// ---- Kernel instance methods ---------------------------------------------------------------

fn k_class(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(real_class_of(c, slf)) }
fn k_singleton_class(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if slf.is_fixnum() || slf.is_sym() || slf.is_flonum() || is_bignum(slf) || is_heap_float(slf) { return type_err(c, "can't define singleton"); }
    Ok(singleton_class(c, slf))
}
fn k_frozen(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(is_frozen(slf))) }
fn k_freeze(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if slf.is_heap() { obj_set_flags(slf, FL_FROZEN); obj_clear_flags(slf, FL_CHILLED); }
    Ok(slf)
}
fn k_nil_p(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(slf == NIL)) }
fn k_eqq(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if slf == a[0] { return Ok(TRUE); }
    let r = funcall(c, slf, s::eq, &[a[0]], std::ptr::null())?;
    Ok(Value::bool(r.truthy()))
}
fn k_match(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(NIL) }
fn k_nmatch(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let r = funcall(c, slf, s::match_op, &[a[0]], std::ptr::null())?;
    Ok(Value::bool(r.falsy()))
}
fn k_cmp(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if slf == a[0] { return Ok(Value::fix(0)); }
    let r = funcall(c, slf, s::eq, &[a[0]], std::ptr::null())?;
    Ok(if r.truthy() { Value::fix(0) } else { NIL })
}
fn k_eql(_c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(slf == a[0])) }
fn k_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if slf == c.vm().main_obj.get() { return Ok(c.str_new(b"main")); }
    Ok(c.str_new(any_to_s_str(c, slf).as_bytes()))
}
pub fn k_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if slf == c.vm().main_obj.get() { return Ok(c.str_new(b"main")); }
    let ivs = ivar_list(c, slf);
    let ivs: Vec<(Sym, Value)> = ivs.into_iter().filter(|(n, _)| sym_bytes(*n).first() == Some(&b'@')).collect();
    let k = real_class_of(c, slf);
    if ivs.is_empty() { return Ok(c.str_new(format!("#<{}:{:#018x}>", class_path(c, k), slf.raw()).as_bytes())); }
    if !recursion_enter(slf, s::inspect) { return Ok(c.str_new(format!("#<{}:{:#018x} ...>", class_path(c, k), slf.raw()).as_bytes())); }
    let mut out = format!("#<{}:{:#018x}", class_path(c, k), slf.raw());
    let keep = c.ary_new();
    for (i, (n, v)) in ivs.iter().enumerate() {
        c.ary_push(keep, *v);
        let r = inspect_str(c, *v);
        let r = match r { Ok(r) => r, Err(e) => { recursion_leave(slf, s::inspect); return Err(e); } };
        out.push_str(if i == 0 { " " } else { ", " });
        out.push_str(&sym_str(*n)); out.push('='); out.push_str(&r);
    }
    recursion_leave(slf, s::inspect);
    out.push('>');
    Ok(c.str_new(out.as_bytes()))
}
fn k_is_a(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if !is_class_or_module(a[0]) { return type_err(c, "class or module required"); }
    Ok(Value::bool(is_kind_of(c, slf, a[0])))
}
fn k_instance_of(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if !is_class_or_module(a[0]) { return type_err(c, "class or module required"); }
    Ok(Value::bool(real_class_of(c, slf) == a[0]))
}
fn k_respond_to(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let mid = to_sym_arg(c, a[0])?;
    let priv_ = a.len() > 1 && a[1].truthy();
    if respond_to(c, slf, mid, priv_) { return Ok(TRUE); }
    if c.refinements_on() {
        let me = lookup_method_refined(c, cur_cref(c), class_of(slf), mid);
        if !me.is_null() && (priv_ || me_visi(c, me) == VISI_PUBLIC) { return Ok(TRUE); }
    }
    let rm = lookup_method(c, class_of(slf), s::respond_to_missing_p);
    if !rm.is_null() {
        let r = funcall(c, slf, s::respond_to_missing_p, &[Value::sym(mid), Value::bool(priv_)], std::ptr::null())?;
        return Ok(Value::bool(r.truthy()));
    }
    Ok(FALSE)
}
fn k_respond_to_missing(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(FALSE) }
fn k_ivar_get(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = to_sym_arg(c, a[0])?;
    check_ivar_name(c, n)?;
    Ok(ivar_get(c, slf, n))
}
fn check_ivar_name(c: &mut Ctx, n: Sym) -> Result<(), Esc> {
    let b = sym_bytes(n);
    if b.len() < 2 || b[0] != b'@' || b[1] == b'@' || b[1].is_ascii_digit() {
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("'{}' is not allowed as an instance variable name", sym_str(n)), Value::sym(n), NIL)?;
        return Err(raise_exc(c, e));
    }
    Ok(())
}
fn k_ivar_set(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = to_sym_arg(c, a[0])?;
    check_ivar_name(c, n)?;
    ivar_set(c, slf, n, a[1])
}
fn k_ivar_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = to_sym_arg(c, a[0])?;
    check_ivar_name(c, n)?;
    Ok(Value::bool(ivar_defined(c, slf, n)))
}
fn k_ivars(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let a = c.ary_new();
    for (n, _) in ivar_list(c, slf) { if sym_bytes(n).first() == Some(&b'@') { c.ary_push(a, Value::sym(n)); } }
    Ok(a)
}
fn k_remove_ivar(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = to_sym_arg(c, a[0])?;
    check_frozen(c, slf)?;
    match ivar_remove(c, slf, n) {
        Some(v) => Ok(v),
        None => {
            let ins = inspect_str(c, slf)?;
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("instance variable {} not defined for {}", sym_str(n), ins), Value::sym(n), slf)?;
            Err(raise_exc(c, e))
        }
    }
}
fn k_tap(c: &mut Ctx, slf: Value, _a: &[Value], b: *const Block) -> RV { yield1(c, b, slf)?; Ok(slf) }
fn k_then(c: &mut Ctx, slf: Value, _a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"then"), &[], NIL); }
    yield1(c, b, slf)
}
fn k_itself(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(slf) }

pub fn obj_dup(c: &mut Ctx, slf: Value, clone: bool, freeze: Option<bool>) -> RV {
    if !slf.is_heap() { return Ok(slf); }
    // the copied payload is built before the allocation; slf (a local) keeps its Values alive
    let body = match c.body_of(slf) {
        Body::Object(iv) => Body::Object(iv.clone()),
        Body::String(s) => Body::String(RString { b: s.b.clone(), enc: s.enc }),
        Body::Array(a) => Body::Array(a.clone()),
        Body::Hash(_) => {
            let n = c.hash_dup(slf);
            obj_set_klass(n, real_class_of(c, slf));
            return finish_dup(c, slf, n, clone, freeze);
        }
        Body::Float(f) => Body::Float(*f),
        Body::Bignum(b) => Body::Bignum(b.clone()),
        Body::Range(a, b, x) => Body::Range(*a, *b, *x),
        Body::Proc(p) => Body::Proc(Box::new(ProcData { blk: p.blk, is_lambda: p.is_lambda, from_method: p.from_method })),
        Body::Method(m) => Body::Method(Box::new(MethodObj { recv: m.recv, me: m.me, name: m.name, unbound: m.unbound, owner: m.owner })),
        Body::Class(_) => {
            let n = crate::builtins::module::clone_class(c, slf)?;
            return finish_dup(c, slf, n, clone, freeze);
        }
        Body::Env(v, p) => Body::Env(v.clone(), *p),
        Body::Data(_) => {
            if let Some(n) = crate::builtins::dup_data(c, slf)? { return finish_dup(c, slf, n, clone, freeze); }
            return type_err(c, format!("allocator undefined for {}", class_path(c, real_class_of(c, slf))));
        }
        Body::Free(_) => unreachable!(),
    };
    let n = c.alloc(real_class_of(c, slf), body);
    if is_proc(n) { c.proc_data_mut(n).blk.proc_val = n; }
    finish_dup(c, slf, n, clone, freeze)
}

fn finish_dup(c: &mut Ctx, slf: Value, n: Value, clone: bool, freeze: Option<bool>) -> RV {
    if obj_flags(slf) & FL_EXIVAR != 0 { for (k, v) in ivar_list(c, slf) { ivar_set_raw(c, n, k, v); } }
    if clone {
        // singleton class copy
        let k = obj_klass(slf);
        if c.class(k).is_singleton && !is_class_or_module(slf) {
            let sk = singleton_class(c, n);
            let mt: Vec<(Sym, *const MethodEntry)> = c.class(k).mtbl.iter().map(|(a, b)| (*a, *b)).collect();
            let nt = &mut c.class_mut(sk).mtbl;
            for (m, me) in mt { nt.insert(m, me); }
            let inc = c.class(k).includes.clone();
            for m in inc { include_module_c(c, sk, m); }
        }
    }
    if obj_flags(slf) & FL_KWHASH != 0 { obj_set_flags(n, FL_KWHASH); }
    let mid = if clone { s::mid_initialize_clone } else { s::mid_initialize_dup };
    let me = lookup_method(c, class_of(n), mid);
    if !me.is_null() && !matches!(me_kind(c, me), Some(MKind::Cfunc(..))) {
        if clone && freeze.is_some() {
            let h = c.hash_new();
            hash_aset(c, h, crate::vm::sym_value("freeze"), Value::bool(freeze.unwrap()))?;
            funcall_kw(c, n, mid, &[slf, h], std::ptr::null(), true)?;
        } else { funcall(c, n, mid, &[slf], std::ptr::null())?; }
    } else {
        funcall(c, n, s::initialize_copy, &[slf], std::ptr::null())?;
    }
    if clone {
        let fz = freeze.unwrap_or(obj_flags(slf) & FL_FROZEN != 0);
        if fz { obj_set_flags(n, FL_FROZEN); } else { obj_clear_flags(n, FL_FROZEN); }
    }
    Ok(n)
}

fn k_dup(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { obj_dup(c, slf, false, None) }
fn k_clone(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mut fz = None;
    if let Some(h) = kw_arg(a) {
        if let Some(v) = hash_get_opt(c, h, sym_value("freeze"))? {
            if v != NIL && v != TRUE && v != FALSE { let i = inspect_str(c, v)?; return arg_err(c, format!("unexpected value for freeze: {}", i)); }
            if v != NIL { fz = Some(v == TRUE); }
        }
    }
    if !slf.is_heap() || is_heap_float(slf) || is_bignum(slf) {
        if fz == Some(false) { return arg_err(c, format!("can't unfreeze {}", class_path(c, real_class_of(c, slf)))); }
        return Ok(slf);
    }
    obj_dup(c, slf, true, fz)
}
fn k_init_copy(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if slf == a[0] { return Ok(slf); }
    check_frozen(c, slf)?;
    if real_class_of(c, slf) != real_class_of(c, a[0]) { return type_err(c, "initialize_copy should take same class object"); }
    Ok(slf)
}
fn k_init_dup(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    funcall(c, slf, s::initialize_copy, &[a[0]], std::ptr::null())?;
    Ok(slf)
}
fn k_extend(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    for &m in a.iter().rev() {
        if !is_class_or_module(m) || !c.class(m).is_module {
            return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, m)));
        }
        funcall(c, m, intern(b"extend_object"), &[slf], std::ptr::null())?;
        funcall(c, m, s::extended, &[slf], std::ptr::null())?;
    }
    Ok(slf)
}
fn m_extend_object(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if is_frozen(a[0]) && a[0].is_heap() { return Err(raise_frozen(c, a[0])); }
    let k = singleton_class(c, a[0]);
    if k == NIL { return type_err(c, "can't define singleton"); }
    include_module_c(c, k, slf);
    c.class_mut(slf).extended_by.push(a[0]);
    Ok(a[0])
}
fn k_display(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let s = to_s_value(c, slf)?;
    let b = c.str_bytes(s).to_vec();
    stdout_write(c, &b)?;
    Ok(NIL)
}
fn k_hash(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { obj_hash(c, slf, a, b) }

/// (name, visibility) of the defined (not undef'ed) entries of class `k`.
fn mtbl_entries(c: &Ctx, k: Value) -> Vec<(Sym, u8)> {
    c.class(k).mtbl.iter().filter_map(|(n, me)| c.me_ref(*me).filter(|m| !matches!(m.kind, MKind::Undef)).map(|m| (*n, m.visi))).collect()
}
fn me_kind(c: &Ctx, me: *const MethodEntry) -> Option<MKind> { c.me_ref(me).map(|m| m.kind) }
fn me_owner(c: &Ctx, me: *const MethodEntry) -> Value { c.me_ref(me).map_or(NIL, |m| m.owner) }
fn me_visi(c: &Ctx, me: *const MethodEntry) -> u8 { c.me_ref(me).map_or(VISI_PUBLIC, |m| m.visi) }
/// A copy of an entry, to define under another owner / name.
fn me_copy(c: &Ctx, me: *const MethodEntry) -> Option<MethodEntry> {
    c.me_ref(me).map(|o| MethodEntry { name: o.name, owner: o.owner, visi: o.visi, kind: o.kind, orig_owner: o.orig_owner, cref: o.cref })
}
/// The lexical scope the running builtin was called under.
fn cur_cref(c: &Ctx) -> *const Cref { c.cur_frame().map_or(std::ptr::null(), |f| f.cref()) }
fn caller_cref(c: &Ctx) -> *const Cref { c.caller_frame().map_or(std::ptr::null(), |f| f.cref()) }

pub fn method_list(c: &mut Ctx, slf_class: Value, filter: impl Fn(u8) -> bool, inherit: bool) -> Value {
    let a = c.ary_new();
    let mut seen = std::collections::HashSet::new();
    let anc = ancestors(c, slf_class).to_vec();
    for k in anc {
        if !inherit && k != slf_class && !(c.class(k).is_singleton) && !c.class(slf_class).is_singleton { break; }
        let mut names: Vec<(Sym, *const MethodEntry)> = c.class(k).mtbl.iter().map(|(a, b)| (*a, *b)).collect();
        names.sort_by_key(|x| x.0);
        for (n, me) in names {
            if !seen.insert(n) { continue; }
            let Some(me) = c.me_ref(me) else { continue };
            if matches!(me.kind, MKind::Undef) { continue; }
            if filter(me.visi) { c.ary_push(a, Value::sym(n)); }
        }
        if !inherit && k == slf_class { break; }
    }
    a
}

fn k_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let all = a.first().map(|v| v.truthy()).unwrap_or(true);
    if !all { return k_singleton_methods(c, slf, &[FALSE], _b); }
    Ok(method_list(c, class_of(slf), |v| v != VISI_PRIVATE, true))
}
fn k_public_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { Ok(method_list(c, class_of(slf), |v| v == VISI_PUBLIC, a.first().is_none_or(|v| v.truthy()))) }
fn k_private_methods(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(method_list(c, class_of(slf), |v| v == VISI_PRIVATE, true)) }
fn k_protected_methods(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(method_list(c, class_of(slf), |v| v == VISI_PROTECTED, true)) }
fn k_singleton_methods(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let a = c.ary_new();
    if !slf.is_heap() { return Ok(a); }
    let k = obj_klass(slf);
    if !c.class(k).is_singleton { return Ok(a); }
    let mut names: Vec<(Sym, *const MethodEntry)> = c.class(k).mtbl.iter().map(|(a, b)| (*a, *b)).collect();
    names.sort_by_key(|x| x.0);
    for (n, me) in names { if me_visi(c, me) != VISI_PRIVATE && !matches!(me_kind(c, me), Some(MKind::Undef)) { c.ary_push(a, Value::sym(n)); } }
    // singleton_methods(false): the singleton class's own methods only
    if !_a.first().is_none_or(|v| v.truthy()) { return Ok(a); }
    for &m in c.class(k).includes.clone().iter() {
        for (n, v) in mtbl_entries(c, m) { if v == VISI_PUBLIC && !c.ary(a).contains(&Value::sym(n)) { c.ary_push(a, Value::sym(n)); } }
    }
    if is_class_or_module(slf) {
        // class methods inherited through the metaclass chain
        let mut sup = c.class(slf).superclass;
        while sup != NIL {
            let mk = obj_klass(sup);
            if c.class(mk).is_singleton { for (n, v) in mtbl_entries(c, mk) { if v == VISI_PUBLIC && !c.ary(a).contains(&Value::sym(n)) { c.ary_push(a, Value::sym(n)); } } }
            sup = c.class(sup).superclass;
        }
    }
    Ok(a)
}
fn k_method(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mid = to_sym_arg(c, a[0])?;
    crate::builtins::proc::method_object(c, slf, mid, false)
}
fn k_public_method(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mid = to_sym_arg(c, a[0])?;
    crate::builtins::proc::method_object(c, slf, mid, true)
}
fn k_singleton_method(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mid = to_sym_arg(c, a[0])?;
    if slf.is_heap() {
        let k = obj_klass(slf);
        if c.class(k).is_singleton && c.class(k).mtbl.contains_key(&mid) { return crate::builtins::proc::method_object(c, slf, mid, false); }
    }
    let ins = inspect_str(c, slf)?;
    let e = make_name_error(c, c.vm().c_name_error.get(), &format!("undefined singleton method '{}' for {}", sym_str(mid), ins), Value::sym(mid), slf)?;
    Err(raise_exc(c, e))
}
fn k_define_singleton_method(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let k = singleton_class(c, slf);
    m_define_method(c, k, a, b)
}
fn k_instance_variables_hash(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(NIL) }
fn k_enum_for(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let mid = if a.is_empty() { s::each } else { to_sym_arg(c, a[0])? };
    let size_fn = if b.is_null() { NIL } else { block_to_proc(c, b)? };
    crate::builtins::enumerator::enum_for(c, slf, mid, if a.is_empty() { &[] } else { &a[1..] }, size_fn)
}

// ---- Module --------------------------------------------------------------------------------

pub fn call_method_added(c: &mut Ctx, klass: Value, mid: Sym) -> Result<(), Esc> {
    if c.class(klass).is_singleton {
        let att = c.class(klass).attached;
        let me = lookup_method(c, class_of(att), s::singleton_method_added);
        if !me.is_null() && me_owner(c, me) != c.vm().c_basic_object.get() {
            funcall(c, att, s::singleton_method_added, &[Value::sym(mid)], std::ptr::null())?;
        }
        return Ok(());
    }
    let me = lookup_method(c, class_of(klass), s::method_added);
    if !me.is_null() && me_owner(c, me) != c.vm().c_module.get() {
        funcall(c, klass, s::method_added, &[Value::sym(mid)], std::ptr::null())?;
    }
    Ok(())
}

pub fn call_inherited(c: &mut Ctx, sup: Value, k: Value) -> Result<(), Esc> {
    let me = lookup_method(c, class_of(sup), s::inherited);
    if !me.is_null() && me_owner(c, me) != c.vm().c_class.get() {
        funcall(c, sup, s::inherited, &[k], std::ptr::null())?;
    }
    Ok(())
}

/// Alias entries keep the original name; __callee__ finds the alias name in Vm.alias_names.
pub fn callee_name(c: &Ctx, me: *const MethodEntry) -> Sym {
    match c.vm().alias_names.borrow().get(&(me as usize)) { Some(&n) => n, None => c.me_ref(me).map_or(Sym(0), |m| m.name) }
}

pub fn alias_method(c: &mut Ctx, klass: Value, newname: Sym, oldname: Sym) -> Result<(), Esc> {
    let mut me = search_method(c, klass, oldname);
    if (me.is_null() || matches!(me_kind(c, me), Some(MKind::Undef))) && c.class(klass).is_module {
        me = search_method(c, c.vm().c_object.get(), oldname);
    }
    if me.is_null() || matches!(me_kind(c, me), Some(MKind::Undef)) {
        let msg = format!("undefined method '{}' for {} '{}'", sym_str(oldname), if c.class(klass).is_module { "module" } else { "class" }, class_path(c, klass));
        let e = make_name_error(c, c.vm().c_name_error.get(), &msg, Value::sym(oldname), klass)?;
        return Err(raise_exc(c, e));
    }
    let Some(o) = me_copy(c, me) else { return Ok(()) };
    let oname = o.name;
    let n = Box::into_raw(Box::new(o)) as *const MethodEntry;
    if newname != oname { c.vm().alias_names.borrow_mut().insert(n as usize, newname); }
    crate::rt::note_redefinition(c, klass, newname);
    define_method_me(c, klass, newname, n);
    call_method_added(c, klass, newname)?;
    Ok(())
}

pub fn undef_method(c: &mut Ctx, klass: Value, mid: Sym) -> Result<(), Esc> {
    let mut me = search_method(c, klass, mid);
    // a refinement may undef a method of the class it refines
    if me.is_null() && c.class(klass).refined { me = search_method(c, c.class(klass).attached, mid); }
    if me.is_null() || matches!(me_kind(c, me), Some(MKind::Undef)) {
        let msg = format!("undefined method '{}' for {} '{}'", sym_str(mid), if c.class(klass).is_module { "module" } else { "class" }, class_path(c, klass));
        let e = make_name_error(c, c.vm().c_name_error.get(), &msg, Value::sym(mid), klass)?;
        return Err(raise_exc(c, e));
    }
    define_method_me(c, klass, mid, new_me(mid, klass, VISI_PUBLIC, MKind::Undef));
    crate::rt::note_redefinition(c, klass, mid);
    Ok(())
}

fn m_alias_method(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 2, 2)?;
    let n = to_sym_arg(c, a[0])?; let o = to_sym_arg(c, a[1])?;
    alias_method(c, slf, n, o)?;
    Ok(Value::sym(n))
}
fn m_undef_method(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    for &x in a { let m = to_sym_arg(c, x)?; undef_method(c, slf, m)?; }
    Ok(slf)
}
fn m_remove_method(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    for &x in a {
        let m = to_sym_arg(c, x)?;
        if c.class_mut(slf).mtbl.remove(&m).is_none() {
            let msg = format!("method '{}' not defined in {}", sym_str(m), class_path(c, slf));
            let e = make_name_error(c, c.vm().c_name_error.get(), &msg, Value::sym(m), slf)?;
            return Err(raise_exc(c, e));
        }
        bump_method_serial(); c.vm().mcache.borrow_mut().clear();
        crate::rt::note_redefinition(c, slf, m);
        if c.class(slf).is_singleton {
            let att = c.class(slf).attached;
            if respond_to(c, att, intern(b"singleton_method_removed"), true) { funcall(c, att, intern(b"singleton_method_removed"), &[Value::sym(m)], std::ptr::null())?; }
        } else if respond_to(c, slf, intern(b"method_removed"), true) {
            funcall(c, slf, intern(b"method_removed"), &[Value::sym(m)], std::ptr::null())?;
        }
    }
    Ok(slf)
}

fn attr_names(c: &mut Ctx, a: &[Value]) -> Result<Vec<Sym>, Esc> {
    let mut v = vec![];
    for &x in a {
        let n = to_sym_arg(c, x)?;
        let b = sym_bytes(n);
        let ok = !b.is_empty() && (b[0].is_ascii_alphabetic() || b[0] == b'_' || b[0] >= 0x80) && b.iter().all(|&x| x.is_ascii_alphanumeric() || x == b'_' || x >= 0x80 || x == b'?' || x == b'!');
        if !ok || b.ends_with(b"?") || b.ends_with(b"!") {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("invalid attribute name '{}'", sym_str(n)), Value::sym(n), NIL)?;
            return Err(raise_exc(c, e));
        }
        v.push(n);
    }
    Ok(v)
}
fn def_attr(c: &mut Ctx, k: Value, n: Sym, reader: bool, writer: bool) -> Result<(), Esc> {
    let visi = cur_default_visi(c);
    let iv = intern(format!("@{}", sym_str(n)).as_bytes());
    if reader { define_method_me(c, k, n, new_me(n, k, visi, MKind::AttrReader(iv))); call_method_added(c, k, n)?; }
    if writer {
        let w = intern(format!("{}=", sym_str(n)).as_bytes());
        define_method_me(c, k, w, new_me(w, k, visi, MKind::AttrWriter(iv)));
        call_method_added(c, k, w)?;
    }
    Ok(())
}
fn cur_default_visi(c: &Ctx) -> u8 {
    let Some(p) = c.caller_frame() else { return VISI_PUBLIC };
    if p.kind == FK_CLASS { if p.visi == VISI_MODFUNC { VISI_PRIVATE } else { p.visi } } else { VISI_PUBLIC }
}
fn m_attr_reader(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let names = attr_names(c, a)?;
    let out = c.ary_new();
    for n in names { def_attr(c, slf, n, true, false)?; c.ary_push(out, Value::sym(n)); }
    Ok(out)
}
fn m_attr_writer(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let names = attr_names(c, a)?;
    let out = c.ary_new();
    for n in names { def_attr(c, slf, n, false, true)?; c.ary_push(out, Value::sym(intern(format!("{}=", sym_str(n)).as_bytes()))); }
    Ok(out)
}
fn m_attr_accessor(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let names = attr_names(c, a)?;
    let out = c.ary_new();
    for n in names { def_attr(c, slf, n, true, true)?; c.ary_push(out, Value::sym(n)); c.ary_push(out, Value::sym(intern(format!("{}=", sym_str(n)).as_bytes()))); }
    Ok(out)
}
fn m_attr(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if a.len() == 2 && (a[1] == TRUE || a[1] == FALSE) {
        let n = to_sym_arg(c, a[0])?;
        def_attr(c, slf, n, true, a[1] == TRUE)?;
        return Ok(c.ary_new_from(&[Value::sym(n)]));
    }
    m_attr_reader(c, slf, a, b)
}

/// private/public/protected/module_function with or without names.
fn set_visibility(c: &mut Ctx, slf: Value, a: &[Value], visi: u8) -> RV {
    if a.is_empty() {
        // a class_eval / Class.new body block keeps its default for the rest of that block
        let set = c.caller_frame().is_some_and(|p| p.kind == FK_CLASS || p.kind == FK_TOP || p.kind == FK_EVAL || (p.kind == FK_BLOCK && p.defined_class() == slf));
        if set { c.set_caller_visi(visi); }
        return Ok(NIL);
    }
    let list: Vec<Value> = if a.len() == 1 && is_array(a[0]) { c.ary(a[0]).to_vec() } else { a.to_vec() };
    let keep = c.ary_new_from(&list);   // to_sym_arg may run Ruby code; list is not scanned
    for &x in &list {
        let m = to_sym_arg(c, x)?;
        let me = search_method(c, slf, m);
        // a refinement changes the visibility of the refined class's method
        let me = if me.is_null() && c.class(slf).refined { search_method(c, c.class(slf).attached, m) } else { me };
        let me = if me.is_null() && c.class(slf).is_module { search_method(c, c.vm().c_object.get(), m) } else { me };
        if me.is_null() || matches!(me_kind(c, me), Some(MKind::Undef)) {
            let msg = format!("undefined method '{}' for {} '{}'", sym_str(m), if c.class(slf).is_module { "module" } else { "class" }, class_path(c, slf));
            let e = make_name_error(c, c.vm().c_name_error.get(), &msg, Value::sym(m), slf)?;
            return Err(raise_exc(c, e));
        }
        let Some(o) = me_copy(c, me) else { continue };
        if visi == VISI_MODFUNC {
            let n = Box::into_raw(Box::new(MethodEntry { name: o.name, owner: slf, visi: VISI_PRIVATE, kind: o.kind, orig_owner: o.orig_owner, cref: o.cref }));
            define_method_me(c, slf, m, n);
            let sk = singleton_class(c, slf);
            let n2 = Box::into_raw(Box::new(MethodEntry { name: o.name, owner: sk, visi: VISI_PUBLIC, kind: o.kind, orig_owner: o.orig_owner, cref: o.cref }));
            define_method_me(c, sk, m, n2);
            continue;
        }
        if o.owner == slf { c.me_set_visi(me, visi); bump_method_serial(); c.vm().mcache.borrow_mut().clear(); }
        else {
            let n = Box::into_raw(Box::new(MethodEntry { name: o.name, owner: slf, visi, kind: o.kind, orig_owner: o.orig_owner, cref: o.cref }));
            define_method_me(c, slf, m, n);
        }
    }
    std::hint::black_box(keep);
    Ok(if a.len() == 1 { a[0] } else { c.ary_new_from(a) })
}
fn m_private(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { set_visibility(c, slf, a, VISI_PRIVATE) }
fn m_public(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { set_visibility(c, slf, a, VISI_PUBLIC) }
fn m_protected(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { set_visibility(c, slf, a, VISI_PROTECTED) }
fn m_module_function(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if !c.class(slf).is_module { return type_err(c, "module_function must be called for modules"); }
    set_visibility(c, slf, a, VISI_MODFUNC)
}
fn top_private(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { set_visibility(c, c.vm().c_object.get(), a, VISI_PRIVATE) }
fn top_public(c: &mut Ctx, _slf: Value, a: &[Value], _b: *const Block) -> RV { set_visibility(c, c.vm().c_object.get(), a, VISI_PUBLIC) }
fn m_private_class_method(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let k = singleton_class(c, slf); set_visibility(c, k, a, VISI_PRIVATE)?; Ok(NIL) }
fn m_public_class_method(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let k = singleton_class(c, slf); set_visibility(c, k, a, VISI_PUBLIC)?; Ok(NIL) }

pub fn m_define_method(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let mid = to_sym_arg(c, a[0])?;
    let visi = { let v = cur_default_visi(c); if mid == s::initialize { VISI_PRIVATE } else { v } };
    let me = if a.len() == 2 {
        let body = a[1];
        if is_proc(body) {
            new_me(mid, slf, visi, MKind::Bmethod(body))
        } else if is_method_obj(body) {
            let Some(o) = me_copy(c, c.method_obj(body).me) else { return type_err(c, "bad method object") };
            if !c.class(o.owner).is_module && !is_kind_of_class(c, slf, o.owner) {
                return type_err(c, format!("bind argument must be a subclass of {}", class_path(c, o.owner)));
            }
            // like an alias: __method__ stays the original name, __callee__ is the new one
            let n = Box::into_raw(Box::new(MethodEntry { name: o.name, owner: slf, visi, kind: o.kind, orig_owner: o.orig_owner, cref: o.cref }));
            if mid != o.name { c.vm().alias_names.borrow_mut().insert(n as usize, mid); }
            n as *const MethodEntry
        } else {
            return type_err(c, format!("wrong argument type {} (expected Proc/Method/UnboundMethod)", class_name_for_err(c, body)));
        }
    } else {
        if b.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
        let p = block_to_proc(c, b)?;
        // define_method's block body runs with lambda semantics
        let np = obj_dup(c, p, false, None)?;
        c.proc_data_mut(np).is_lambda = true;
        new_me(mid, slf, visi, MKind::Bmethod(np))
    };
    crate::rt::note_redefinition(c, slf, mid);
    define_method_me(c, slf, mid, me);
    call_method_added(c, slf, mid)?;
    Ok(Value::sym(mid))
}

fn is_kind_of_class(c: &Ctx, k: Value, owner: Value) -> bool { k == owner || ancestors(c, k).contains(&owner) }

fn m_instance_method(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mid = to_sym_arg(c, a[0])?;
    crate::builtins::proc::unbound_method_object(c, slf, mid, false)
}
fn m_public_instance_method(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let mid = to_sym_arg(c, a[0])?;
    crate::builtins::proc::unbound_method_object(c, slf, mid, true)
}
fn method_defined_impl(c: &mut Ctx, slf: Value, a: &[Value], f: impl Fn(u8) -> bool) -> RV {
    let mid = to_sym_arg(c, a[0])?;
    let inherit = a.len() < 2 || a[1].truthy();
    let me = if inherit { search_method(c, slf, mid) } else { c.class(slf).mtbl.get(&mid).copied().unwrap_or(std::ptr::null()) };
    if me.is_null() || matches!(me_kind(c, me), Some(MKind::Undef)) { return Ok(FALSE); }
    Ok(Value::bool(f(me_visi(c, me))))
}
fn m_method_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { method_defined_impl(c, slf, a, |v| v != VISI_PRIVATE) }
fn m_public_method_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { method_defined_impl(c, slf, a, |v| v == VISI_PUBLIC) }
fn m_private_method_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { method_defined_impl(c, slf, a, |v| v == VISI_PRIVATE) }
fn m_protected_method_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { method_defined_impl(c, slf, a, |v| v == VISI_PROTECTED) }
fn m_instance_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let inherit = a.first().map(|v| v.truthy()).unwrap_or(true);
    Ok(method_list(c, slf, |v| v != VISI_PRIVATE, inherit))
}
fn m_public_instance_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let inherit = a.first().map(|v| v.truthy()).unwrap_or(true);
    Ok(method_list(c, slf, |v| v == VISI_PUBLIC, inherit))
}
fn m_private_instance_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let inherit = a.first().map(|v| v.truthy()).unwrap_or(true);
    Ok(method_list(c, slf, |v| v == VISI_PRIVATE, inherit))
}
fn m_protected_instance_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let inherit = a.first().map(|v| v.truthy()).unwrap_or(true);
    Ok(method_list(c, slf, |v| v == VISI_PROTECTED, inherit))
}

fn m_name(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    match c.class(slf).name { Some(n) => Ok(n), None => Ok(NIL) }
}
fn m_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let cd = c.class(slf);
    if cd.is_singleton {
        let att = cd.attached;
        let inner = if is_class_or_module(att) { inspect_str(c, att)? } else { inspect_str(c, att)? };
        return Ok(c.str_new(format!("#<Class:{}>", inner).as_bytes()));
    }
    Ok(c.str_new(class_path(c, slf).as_bytes()))
}
fn m_eqq(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(is_kind_of(c, a[0], slf))) }
fn m_lt(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if !is_class_or_module(a[0]) { return type_err(c, "compared with non class/module"); }
    if slf == a[0] { return Ok(FALSE); }
    if ancestors(c, slf).contains(&a[0]) { return Ok(TRUE); }
    if ancestors(c, a[0]).contains(&slf) { return Ok(FALSE); }
    Ok(NIL)
}
fn m_le(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { if slf == a[0] { return Ok(TRUE); } m_lt(c, slf, a, b) }
fn m_gt(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if !is_class_or_module(a[0]) { return type_err(c, "compared with non class/module"); }
    m_lt(c, a[0], &[slf], b)
}
fn m_ge(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { if slf == a[0] { return Ok(TRUE); } m_gt(c, slf, a, b) }
fn m_cmp(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if slf == o { return Ok(Value::fix(0)); }
    if !is_class_or_module(o) { return Ok(NIL); }
    if ancestors(c, slf).contains(&o) { return Ok(Value::fix(-1)); }
    if ancestors(c, o).contains(&slf) { return Ok(Value::fix(1)); }
    Ok(NIL)
}
fn m_ancestors(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let v: Vec<Value> = ancestors(c, slf).iter().copied().filter(|&k| !c.class(k).is_singleton || slf == k || true).collect();
    Ok(c.ary_from_vec(v))
}
fn m_include(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    for &m in a.iter() {
        if !is_class_or_module(m) || !c.class(m).is_module {
            return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, m)));
        }
        if m == slf { return arg_err(c, "cyclic include detected"); }
    }
    for &m in a.iter().rev() {
        funcall(c, m, intern(b"append_features"), &[slf], std::ptr::null())?;
        funcall(c, m, s::included, &[slf], std::ptr::null())?;
    }
    Ok(slf)
}
fn m_append_features(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { include_module_c(c, a[0], slf); Ok(slf) }
fn m_prepend(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    for &m in a.iter() {
        if !is_class_or_module(m) || !c.class(m).is_module {
            return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, m)));
        }
    }
    for &m in a.iter().rev() {
        funcall(c, m, intern(b"prepend_features"), &[slf], std::ptr::null())?;
        funcall(c, m, s::prepended, &[slf], std::ptr::null())?;
    }
    Ok(slf)
}
fn m_prepend_features(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { prepend_module(c, a[0], slf); Ok(slf) }
fn m_hook_noop(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(NIL) }
fn m_include_p(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if !is_class_or_module(a[0]) || !c.class(a[0]).is_module { return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, a[0]))); }
    Ok(Value::bool(slf != a[0] && ancestors(c, slf).contains(&a[0])))
}
fn m_included_modules(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let v: Vec<Value> = ancestors(c, slf).iter().copied().filter(|&k| c.class(k).is_module).collect();
    Ok(c.ary_from_vec(v))
}
fn m_class_eval(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if b.is_null() {
        if a.is_empty() { return argc_err(c, 0, 1, 3); }
        let src = expect_str(c, a[0])?;
        let bytes = c.str_bytes(src).to_vec();
        let file: &'static str = if a.len() > 1 { Box::leak(to_rstring(c, a[1])?.into_boxed_str()) } else { crate::builtins::binding::eval_file(c) };
        return crate::builtins::binding::eval_in_class(c, slf, &bytes, file, if a.len() > 2 { a[2].as_fix() as i32 } else { 1 });
    }
    yield_with_self(c, b, &[slf], slf, slf)
}
fn m_class_exec(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return err(c, c.vm().c_local_jump_error.get(), "no block given (yield)"); }
    yield_with_self(c, b, a, slf, slf)
}
fn m_const_get(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 2)?;
    let inherit = a.len() < 2 || a[1].truthy();
    let name = if a[0].is_sym() { sym_str(a[0].as_sym()) } else { to_rstring(c, a[0])? };
    let mut cur = slf;
    let parts: Vec<&str> = name.split("::").collect();
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() { if i == 0 { cur = c.vm().c_object.get(); continue; } return err(c, c.vm().c_name_error.get(), format!("wrong constant name {}", name)); }
        if !p.chars().next().unwrap().is_uppercase() {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("wrong constant name {}", p), Value::sym(intern(p.as_bytes())), slf)?;
            return Err(raise_exc(c, e));
        }
        let sym = intern(p.as_bytes());
        if !is_class_or_module(cur) { return type_err(c, format!("{} does not refer to class/module", name)); }
        cur = if inherit {
            match const_get_inherit(c, cur, sym)? { Some(v) => v, None => const_missing(c, cur, sym)? }
        } else {
            match const_get_at(c, cur, sym) { Some(v) => v, None => const_missing(c, cur, sym)? }
        };
    }
    Ok(cur)
}
fn m_const_set(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = to_sym_arg(c, a[0])?;
    let b = sym_bytes(n);
    if b.is_empty() || !b[0].is_ascii_uppercase() || !b.iter().all(|x| x.is_ascii_alphanumeric() || *x == b'_' || *x >= 0x80) {
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("wrong constant name {}", sym_str(n)), Value::sym(n), slf)?;
        return Err(raise_exc(c, e));
    }
    check_frozen(c, slf)?;
    const_set(c, slf, n, a[1]);
    Ok(a[1])
}
fn m_const_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let name = if a[0].is_sym() { sym_str(a[0].as_sym()) } else { to_rstring(c, a[0])? };
    let inherit = a.len() < 2 || a[1].truthy();
    let mut cur = slf;
    for p in name.split("::") {
        if p.is_empty() { cur = c.vm().c_object.get(); continue; }
        if !p.chars().next().unwrap().is_uppercase() {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("wrong constant name {}", p), Value::sym(intern(p.as_bytes())), slf)?;
            return Err(raise_exc(c, e));
        }
        let sym = intern(p.as_bytes());
        if !is_class_or_module(cur) { return Ok(FALSE); }
        let found = if inherit {
            let mut r = None;
            for &k in ancestors(c, cur).iter() { if let Some(v) = const_get_at(c, k, sym) { r = Some(v); break; } if c.class(k).autoload.contains_key(&sym) { r = Some(NIL); break; } }
            if r.is_none() && c.class(cur).is_module { r = const_get_at(c, c.vm().c_object.get(), sym); }
            r
        } else { const_get_at(c, cur, sym).or_else(|| c.class(cur).autoload.get(&sym).map(|_| NIL)) };
        match found { Some(v) => cur = v, None => return Ok(FALSE) }
    }
    Ok(TRUE)
}
fn m_constants(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let inherit = a.first().map(|v| v.truthy()).unwrap_or(true);
    let out = c.ary_new();
    let mut seen = std::collections::HashSet::new();
    let list: Vec<Value> = if inherit { ancestors(c, slf).to_vec() } else { vec![slf] };
    for k in list {
        if inherit && k == c.vm().c_object.get() && slf != c.vm().c_object.get() { break; }
        for &n in c.class(k).const_order.clone().iter() {
            if c.class(k).consts.contains_key(&n) && !c.class(k).private_consts.contains(&n) && seen.insert(n) { c.ary_push(out, Value::sym(n)); }
        }
        for (&n, _) in c.class(k).autoload.clone().iter() { if seen.insert(n) { c.ary_push(out, Value::sym(n)); } }
    }
    Ok(out)
}
fn m_remove_const(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = to_sym_arg(c, a[0])?;
    match c.class_mut(slf).consts.remove(&n) {
        Some(v) => { c.class_mut(slf).const_order.retain(|x| *x != n); c.vm().const_serial.set(c.vm().const_serial.get() + 1); Ok(v) }
        None => {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("constant {}::{} not defined", class_path(c, slf), sym_str(n)), Value::sym(n), slf)?;
            Err(raise_exc(c, e))
        }
    }
}
fn m_private_constant(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    for &x in a { let n = to_sym_arg(c, x)?; c.class_mut(slf).private_consts.push(n); }
    Ok(NIL)
}
fn m_const_missing(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    // the name may be any object (rb_mod_const_missing formats it with %s)
    let ns = if a[0].is_sym() { sym_str(a[0].as_sym()).to_string() } else { to_rstring(c, a[0])? };
    let msg = if slf == c.vm().c_object.get() { format!("uninitialized constant {}", ns) } else { format!("uninitialized constant {}::{}", class_path(c, slf), ns) };
    let e = make_name_error(c, c.vm().c_name_error.get(), &msg, a[0], slf)?;
    Err(raise_exc(c, e))
}
fn m_cvar_get(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = to_sym_arg(c, a[0])?;
    match crate::rt::cvar_lookup(c, slf, n) {
        Some((_, v)) => Ok(v),
        None => {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("uninitialized class variable {} in {}", sym_str(n), class_path(c, slf)), Value::sym(n), slf)?;
            Err(raise_exc(c, e))
        }
    }
}
fn m_cvar_set(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = to_sym_arg(c, a[0])?;
    match crate::rt::cvar_lookup(c, slf, n) { Some((o, _)) => { c.class_mut(o).cvars.insert(n, a[1]); } None => { c.class_mut(slf).cvars.insert(n, a[1]); } }
    Ok(a[1])
}
fn m_cvar_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let n = to_sym_arg(c, a[0])?;
    Ok(Value::bool(crate::rt::cvar_lookup(c, slf, n).is_some()))
}
fn m_cvars(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let out = c.ary_new();
    for &k in ancestors(c, slf).to_vec().iter() {
        let names: Vec<Sym> = c.class(k).cvars.keys().copied().collect();
        for n in names { if !c.ary(out).contains(&Value::sym(n)) { c.ary_push(out, Value::sym(n)); } }
    }
    Ok(out)
}
fn m_freeze(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { obj_set_flags(slf, FL_FROZEN); Ok(slf) }
fn m_module_new(c: &mut Ctx, slf: Value, _a: &[Value], b: *const Block) -> RV {
    let m = new_module(c);
    obj_set_klass(m, if slf == c.vm().c_module.get() { c.vm().c_module.get() } else { slf });
    if !b.is_null() { yield_with_self(c, b, &[m], m, m)?; }
    Ok(m)
}
fn m_instance_variable_names(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(c.ary_new()) }
fn m_singleton_class_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(c.class(slf).is_singleton)) }
fn m_hash(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix((crate::astrust::hash_u64(slf.raw()) >> 2) as i64)) }
fn refinements_of(c: &mut Ctx, m: Value) -> Value {
    let key = intern(b"__refinements");
    let l = ivar_get(c, m, key);
    if l != NIL { return l; }
    let l = c.ary_new();
    ivar_set_raw(c, m, key, l);
    l
}
fn m_refine(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let k = a[0];
    if !is_class_or_module(k) { return type_err(c, format!("wrong argument type {} (expected Class or Module)", class_name_for_err(c, k))); }
    if b.is_null() { return arg_err(c, "no block given"); }
    let list = refinements_of(c, slf);
    let found = c.ary(list).iter().copied().find(|&r| c.class(r).attached == k);
    let r = match found {
        Some(r) => r,
        None => {
            let r = new_module(c);
            if let Some(rc) = const_get_at(c, c.vm().c_object.get(), intern(b"Refinement")) { obj_set_klass(r, rc); }
            let nm = c.str_new_frozen(format!("#<refinement:{}@{}>", class_path(c, k), class_path(c, slf)).as_bytes());
            let cd = c.class_mut(r);
            cd.refined = true; cd.attached = k; cd.name = Some(nm);
            c.ary_push(list, r);
            r
        }
    };
    // the body sees its module's refinements, this one included
    let Some(mut blk) = c.block_copy(b) else { return arg_err(c, "no block given") };
    let outer = cref_refinements(blk.cref);
    let mut v = if outer != NIL { c.ary(outer).to_vec() } else { vec![] };
    for &x in c.ary(list) { if !v.contains(&x) { v.push(x); } }
    let active = c.ary_from_vec(v);
    let cr = new_cref(r, blk.cref);
    c.cref_set_refinements(cr, active);
    c.set_refinements_on();
    blk.cref = cr;
    yield_with_self(c, &blk, &[r], r, r)?;
    Ok(r)
}
/// `using mod`: the caller's cref is replaced by a copy that has mod's refinements activated.
fn m_using(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let m = a[0];
    if !is_class_or_module(m) || !c.class(m).is_module { return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, m))); }
    let Some(pkind) = c.caller_frame().map(|f| f.kind) else { return Ok(slf) };
    if pkind == FK_METHOD { return err(c, c.vm().c_runtime_error.get(), "Module#using is not permitted in methods"); }
    let old = caller_cref(c);
    let outer = cref_refinements(old);
    let mut v = if outer != NIL && is_array(outer) { c.ary(outer).to_vec() } else { vec![] };
    for &anc in ancestors(c, m).iter().rev() {
        let l = ivar_get(c, anc, intern(b"__refinements"));
        if l == NIL { continue; }
        for &x in c.ary(l) { v.retain(|&y| y != x); v.push(x); }
    }
    let active = c.ary_from_vec(v);
    let (klass, prev) = match c.cref_ref(old) { None => (c.vm().c_object.get(), std::ptr::null()), Some(o) => (o.klass, o.prev) };
    let cr = new_cref(klass, prev);
    c.cref_set_refinements(cr, active);
    c.set_caller_cref(cr);
    c.set_refinements_on();
    bump_method_serial();
    Ok(slf)
}
fn m_refinements(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let l = ivar_get(c, slf, intern(b"__refinements"));
    Ok(if l == NIL { c.ary_new() } else { c.ary_from_vec(c.ary(l).to_vec()) })
}
fn refinement_target(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(c.class(slf).attached) }
/// Refinement#import_methods: copies of the modules' Ruby-defined methods.
fn refinement_import_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    for &m in a {
        if !is_class_or_module(m) || !c.class(m).is_module { return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, m))); }
        let mt: Vec<(Sym, *const MethodEntry)> = c.class(m).mtbl.iter().map(|(a, b)| (*a, *b)).collect();
        for (name, me) in mt {
            let Some(me) = me_copy(c, me) else { continue };
            match me.kind {
                MKind::Iseq(_) | MKind::Bmethod(_) | MKind::AttrReader(_) | MKind::AttrWriter(_) => {
                    // the copies run under the refine block's scope (its refinements are active)
                    let cref = if c.caller_frame().is_none() { me.cref } else { caller_cref(c) };
                    let n = new_me_cref(name, slf, me.visi, me.kind, cref);
                    define_method_me(c, slf, name, n);
                }
                _ => return arg_err(c, format!("Can't import method which is not defined with Ruby code: {}#{}", class_path(c, m), sym_str(name))),
            }
        }
    }
    Ok(slf)
}
fn m_attached_object(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if !c.class(slf).is_singleton { return type_err(c, format!("'{}' is not a singleton class", class_path(c, slf))); }
    Ok(c.class(slf).attached)
}

// ---- Class ----------------------------------------------------------------------------------

pub fn alloc_instance(c: &mut Ctx, klass: Value) -> RV {
    let cd = c.class(klass);
    if cd.is_singleton { return type_err(c, "can't create instance of singleton class"); }
    Ok(match cd.alloc {
        AllocKind::Object | AllocKind::Exception => new_object(c, klass),
        AllocKind::String => c.alloc(klass, Body::String(RString { b: crate::sbuf::SBuf::new(), enc: ENC_UTF8 })),
        AllocKind::Array => c.alloc(klass, Body::Array(crate::avec::AVec::new())),
        AllocKind::Hash => { let h = c.hash_new(); obj_set_klass(h, klass); h }
        AllocKind::Range => c.alloc(klass, Body::Range(NIL, NIL, false)),
        AllocKind::Class => { let k = new_class(c, c.vm().c_object.get()); k }
        AllocKind::Module => { let m = new_module(c); obj_set_klass(m, klass); m }
        AllocKind::Proc | AllocKind::None => {
            if let Some(v) = crate::builtins::alloc_data(c, klass)? { return Ok(v); }
            return type_err(c, format!("allocator undefined for {}", class_path(c, klass)));
        }
    })
}

fn c_allocate(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { alloc_instance(c, slf) }
pub fn c_new(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let o = alloc_instance(c, slf)?;
    let kw = kw_arg(a).is_some();
    let me = lookup_method(c, class_of(o), s::initialize);
    match me_kind(c, me) {
        Some(MKind::Cfunc(..)) => if me == c.vm().ids.bo_init.get() {
            if !a.is_empty() && c.class(slf).alloc == AllocKind::Object && me_owner(c, me) == c.vm().c_basic_object.get() {
                return argc_err(c, a.len(), 0, 0);
            }
            return Ok(o);
        },
        Some(MKind::Iseq(is)) if !kw && trace_mask() & EV_CALLS == 0 => {
            let done = c.with_iseq(is, |c, is| -> Result<bool, Esc> {
                let Some(is) = is else { return Ok(false) };
                if !(is.lead_only && is.params.lead.len() == a.len()) { return Ok(false); }
                // initialize with only required params: call it directly
                let base = c.top();
                stack_check(c, a.len() + 4)?;
                for (i, x) in a.iter().enumerate() { base[2 + i].set(*x); }
                c.set_sp(base, 2 + a.len());
                let r = invoke_iseq_simple(c, me, is, o, base, a.len() as u32, b);
                c.set_sp(base, 0);
                r?;
                Ok(true)
            })?;
            if done { return Ok(o); }
        }
        _ => {}
    }
    funcall_kw(c, o, s::initialize, a, b, kw)?;
    Ok(o)
}
fn c_class_new(c: &mut Ctx, _slf: Value, a: &[Value], b: *const Block) -> RV {
    let sup = if a.is_empty() { c.vm().c_object.get() } else { a[0] };
    if !is_class_or_module(sup) || c.class(sup).is_module { return type_err(c, format!("superclass must be an instance of Class (given an instance of {})", class_name_for_err(c, sup))); }
    if c.class(sup).is_singleton { return type_err(c, "can't make subclass of singleton class"); }
    if sup == c.vm().c_class.get() { return type_err(c, "can't make subclass of Class"); }
    let k = new_class(c, sup);
    call_inherited(c, sup, k)?;
    if !b.is_null() { yield_with_self(c, b, &[k], k, k)?; }
    Ok(k)
}
fn c_superclass(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let mut s = c.class(slf).superclass;
    while s != NIL && c.class(s).is_module { s = c.class(s).superclass; }
    Ok(s)
}
fn c_subclasses(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let v: Vec<Value> = c.class(slf).subclasses.iter().rev().copied().filter(|&k| !c.class(k).is_singleton).collect();
    Ok(c.ary_from_vec(v))
}
fn c_inherited(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(NIL) }

/// Method identities object::init records (Vm.ids).
pub struct InitIds { pub bo_init: *const MethodEntry, pub bo_eq: *const MethodEntry, pub kernel_hash: *const MethodEntry, pub m_eqq: *const MethodEntry, pub c_new: *const MethodEntry }

/// Boot: BasicObject / Kernel / Module / Class methods.
pub fn init(c: &mut Ctx) -> InitIds {
    let vmr = c.vm();
    let bo = vmr.c_basic_object.get();
    define_private_cfunc(c, bo, "initialize", bo_initialize, -1);
    let id_bo_init = search_method(c, bo, s::initialize);
    define_cfunc(c, bo, "==", bo_eq, 1);
    let id_bo_eq = search_method(c, bo, s::eq);
    define_cfunc(c, bo, "equal?", bo_eq, 1);
    define_cfunc(c, bo, "!", bo_not, 0);
    define_cfunc(c, bo, "!=", bo_neq, 1);
    define_cfunc(c, bo, "__id__", bo_id, 0);
    define_cfunc(c, bo, "__send__", bo_send, -1);
    define_cfunc(c, bo, "instance_eval", bo_instance_eval, -1);
    define_cfunc(c, bo, "instance_exec", bo_instance_exec, -1);
    define_private_cfunc(c, bo, "method_missing", bo_method_missing, -1);
    define_private_cfunc(c, bo, "singleton_method_added", bo_singleton_method_added, -1);
    define_private_cfunc(c, bo, "singleton_method_removed", bo_singleton_method_added, -1);
    define_private_cfunc(c, bo, "singleton_method_undefined", bo_singleton_method_added, -1);

    let k = vmr.m_kernel.get();
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, k, n, f, a);
    d(c, "class", k_class, 0);
    d(c, "singleton_class", k_singleton_class, 0);
    d(c, "frozen?", k_frozen, 0);
    d(c, "freeze", k_freeze, 0);
    d(c, "nil?", k_nil_p, 0);
    d(c, "===", k_eqq, 1);
    d(c, "=~", k_match, 1);
    d(c, "!~", k_nmatch, 1);
    d(c, "<=>", k_cmp, 1);
    d(c, "eql?", k_eql, 1);
    d(c, "to_s", k_to_s, 0);
    d(c, "inspect", k_inspect, 0);
    d(c, "is_a?", k_is_a, 1);
    d(c, "kind_of?", k_is_a, 1);
    d(c, "instance_of?", k_instance_of, 1);
    d(c, "respond_to?", k_respond_to, -1);
    define_private_cfunc(c, k, "respond_to_missing?", k_respond_to_missing, 2);
    d(c, "instance_variable_get", k_ivar_get, 1);
    d(c, "instance_variable_set", k_ivar_set, 2);
    d(c, "instance_variable_defined?", k_ivar_defined, 1);
    d(c, "instance_variables", k_ivars, 0);
    d(c, "remove_instance_variable", k_remove_ivar, 1);
    d(c, "tap", k_tap, 0);
    d(c, "then", k_then, 0);
    d(c, "yield_self", k_then, 0);
    d(c, "itself", k_itself, 0);
    d(c, "dup", k_dup, 0);
    d(c, "clone", k_clone, -1);
    define_private_cfunc(c, k, "initialize_copy", k_init_copy, 1);
    define_private_cfunc(c, k, "initialize_dup", k_init_dup, 1);
    define_private_cfunc(c, k, "initialize_clone", k_init_dup, -1);
    d(c, "extend", k_extend, -1);
    d(c, "display", k_display, -1);
    d(c, "hash", k_hash, 0);
    let id_kernel_hash = search_method(c, k, s::hash);
    d(c, "object_id", bo_id, 0);
    d(c, "send", bo_send, -1);
    d(c, "public_send", k_public_send, -1);
    d(c, "methods", k_methods, -1);
    d(c, "public_methods", k_public_methods, -1);
    d(c, "private_methods", k_private_methods, -1);
    d(c, "protected_methods", k_protected_methods, -1);
    d(c, "singleton_methods", k_singleton_methods, -1);
    d(c, "method", k_method, 1);
    d(c, "public_method", k_public_method, 1);
    d(c, "singleton_method", k_singleton_method, 1);
    d(c, "define_singleton_method", k_define_singleton_method, -1);
    d(c, "enum_for", k_enum_for, -1);
    d(c, "to_enum", k_enum_for, -1);
    let _ = k_instance_variables_hash;

    let m = vmr.c_module.get();
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, m, n, f, a);
    d(c, "alias_method", m_alias_method, -1);
    d(c, "undef_method", m_undef_method, -1);
    d(c, "remove_method", m_remove_method, -1);
    d(c, "attr_reader", m_attr_reader, -1);
    d(c, "attr_writer", m_attr_writer, -1);
    d(c, "attr_accessor", m_attr_accessor, -1);
    d(c, "attr", m_attr, -1);
    d(c, "private", m_private, -1);
    d(c, "public", m_public, -1);
    d(c, "protected", m_protected, -1);
    d(c, "module_function", m_module_function, -1);
    d(c, "private_class_method", m_private_class_method, -1);
    d(c, "public_class_method", m_public_class_method, -1);
    d(c, "define_method", m_define_method, -1);
    d(c, "instance_method", m_instance_method, 1);
    d(c, "public_instance_method", m_public_instance_method, 1);
    d(c, "method_defined?", m_method_defined, -1);
    d(c, "public_method_defined?", m_public_method_defined, -1);
    d(c, "private_method_defined?", m_private_method_defined, -1);
    d(c, "protected_method_defined?", m_protected_method_defined, -1);
    d(c, "instance_methods", m_instance_methods, -1);
    d(c, "public_instance_methods", m_public_instance_methods, -1);
    d(c, "private_instance_methods", m_private_instance_methods, -1);
    d(c, "protected_instance_methods", m_protected_instance_methods, -1);
    d(c, "name", m_name, 0);
    d(c, "to_s", m_to_s, 0);
    d(c, "inspect", m_to_s, 0);
    d(c, "===", m_eqq, 1);
    let id_m_eqq = search_method(c, m, s::eqq);
    d(c, "<", m_lt, 1);
    d(c, "<=", m_le, 1);
    d(c, ">", m_gt, 1);
    d(c, ">=", m_ge, 1);
    d(c, "<=>", m_cmp, 1);
    d(c, "ancestors", m_ancestors, 0);
    d(c, "include", m_include, -1);
    d(c, "prepend", m_prepend, -1);
    define_private_cfunc(c, m, "append_features", m_append_features, 1);
    define_private_cfunc(c, m, "prepend_features", m_prepend_features, 1);
    define_private_cfunc(c, m, "extend_object", m_extend_object, 1);
    define_private_cfunc(c, m, "included", m_hook_noop, 1);
    define_private_cfunc(c, m, "extended", m_hook_noop, 1);
    define_private_cfunc(c, m, "prepended", m_hook_noop, 1);
    define_private_cfunc(c, m, "method_added", m_hook_noop, 1);
    define_private_cfunc(c, m, "method_removed", m_hook_noop, 1);
    define_private_cfunc(c, m, "method_undefined", m_hook_noop, 1);
    define_private_cfunc(c, m, "const_added", m_hook_noop, 1);
    d(c, "include?", m_include_p, 1);
    d(c, "included_modules", m_included_modules, 0);
    d(c, "class_eval", m_class_eval, -1);
    d(c, "module_eval", m_class_eval, -1);
    d(c, "class_exec", m_class_exec, -1);
    d(c, "module_exec", m_class_exec, -1);
    d(c, "const_get", m_const_get, -1);
    d(c, "const_set", m_const_set, 2);
    d(c, "const_defined?", m_const_defined, -1);
    d(c, "constants", m_constants, -1);
    d(c, "remove_const", m_remove_const, 1);
    d(c, "private_constant", m_private_constant, -1);
    d(c, "public_constant", m_hook_noop, -1);
    d(c, "const_missing", m_const_missing, 1);
    d(c, "class_variable_get", m_cvar_get, 1);
    d(c, "class_variable_set", m_cvar_set, 2);
    d(c, "class_variable_defined?", m_cvar_defined, 1);
    d(c, "class_variables", m_cvars, -1);
    d(c, "freeze", m_freeze, 0);
    d(c, "hash", m_hash, 0);
    d(c, "singleton_class?", m_singleton_class_p, 0);
    d(c, "refine", m_refine, 1);
    d(c, "using", m_using, 1);
    d(c, "refinements", m_refinements, 0);
    let rf = define_class(c, "Refinement", vmr.c_module.get());
    define_cfunc(c, rf, "target", refinement_target, 0);
    define_private_cfunc(c, rf, "import_methods", refinement_import_methods, -1);
    d(c, "attached_object", m_attached_object, 0);
    d(c, "instance_variable_names", m_instance_variable_names, 0);
    define_singleton_cfunc(c, m, "new", m_module_new, -1);

    let kc = vmr.c_class.get();
    define_cfunc(c, kc, "allocate", c_allocate, 0);
    define_cfunc(c, kc, "new", c_new, -1);
    let id_c_new = search_method(c, kc, crate::symbol::intern_str("new"));
    define_cfunc(c, kc, "superclass", c_superclass, 0);
    define_cfunc(c, kc, "subclasses", c_subclasses, 0);
    define_private_cfunc(c, kc, "inherited", c_inherited, 1);
    define_singleton_cfunc(c, kc, "new", c_class_new, -1);

    let main = vmr.main_obj.get();
    define_singleton_cfunc(c, main, "to_s", |c, _s, _a, _b| Ok(c.str_new(b"main")), 0);
    define_singleton_cfunc(c, main, "inspect", |c, _s, _a, _b| Ok(c.str_new(b"main")), 0);
    let sk = singleton_class(c, main);
    define_private_cfunc(c, sk, "private", top_private, -1);
    define_private_cfunc(c, sk, "public", top_public, -1);
    define_private_cfunc(c, sk, "include", |c, _s, a, b| m_include(c, c.vm().c_object.get(), a, b), -1);
    define_private_cfunc(c, sk, "define_method", |c, _s, a, b| m_define_method(c, c.vm().c_object.get(), a, b), -1);
    define_private_cfunc(c, sk, "using", m_using, 1);
    InitIds { bo_init: id_bo_init, bo_eq: id_bo_eq, kernel_hash: id_kernel_hash, m_eqq: id_m_eqq, c_new: id_c_new }
}
