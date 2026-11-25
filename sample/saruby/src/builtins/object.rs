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

fn str_sym_checked(c: &mut Ctx, s: Value) -> Result<Sym, Esc> {
    if c.str_enc(s) == crate::object::ENC_UTF8 { return crate::builtins::string_methods::utf8_to_sym(c, s).map(|v| v.as_sym()); }
    Ok(c.str_to_sym(s).as_sym())
}
pub fn to_sym_arg(c: &mut Ctx, v: Value) -> Result<Sym, Esc> {
    if v.is_sym() { return Ok(v.as_sym()); }
    if is_string(v) { return str_sym_checked(c, v); }
    if v.is_heap() && respond_to(c, v, s::to_str, true) { let s2 = expect_str(c, v)?; return str_sym_checked(c, s2); }
    let ins = inspect_str(c, v)?;
    type_err(c, format!("{} is not a symbol nor a string", ins))
}

// ---- BasicObject --------------------------------------------------------------------

fn bo_initialize(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(NIL) }
/// BasicObject#== entry (function pointers are not reliably unique, so identify it by entry).
pub fn bo_eq(_c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { Ok(Value::bool(s == a[0] || (s.is_flonum() && false))) }
fn bo_not(_c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(s.falsy())) }
fn bo_neq(c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    let r = funcall(c, s, crate::symbol::s::eq, &[a[0]], std::ptr::null())?;
    Ok(Value::bool(r.falsy()))
}
fn bo_id(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.int_from_i128(object_id(s) as i128)) }
fn bo_send(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if a.is_empty() { return arg_err(c, "no method name given"); }
    let mid = to_sym_arg(c, a[0])?;
    let kw = kw_arg(c, a).is_some();
    if c.refinements_on() {
        if let Some(r) = funcall_refined(c, std::ptr::null(), slf, mid, &a[1..], b, CF_FCALL | if kw { CF_KW } else { 0 }) { return r; }
    }
    funcall_kw(c, slf, mid, &a[1..], b, kw)
}
fn k_public_send(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if a.is_empty() { return arg_err(c, "no method name given"); }
    let mid = to_sym_arg(c, a[0])?;
    let kw = kw_arg(c, a).is_some();
    if c.refinements_on() {
        if let Some(r) = funcall_refined(c, std::ptr::null(), slf, mid, &a[1..], b, CF_PUBLIC_SEND | if kw { CF_KW } else { 0 }) { return r; }
    }
    funcall_flags(c, slf, mid, &a[1..], b, CF_PUBLIC_SEND | if kw { CF_KW } else { 0 })
}
fn bo_instance_eval(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if b.is_null() {
        if a.is_empty() || a.len() > 3 { return argc_err(c, a.len(), 1, 3); }
        let src = expect_str(c, a[0])?;
        let (file, line) = crate::builtins::binding::eval_file_line(c, a.get(1).copied(), a.get(2).copied())?;
        let bytes = c.str_bytes(src).to_vec();
        let enc = c.str_enc(src);
        return crate::builtins::binding::eval_with_self(c, slf, &bytes, enc, file, line);
    }
    if !a.is_empty() { return argc_err(c, a.len(), 0, 0); }
    let dc = crate::rt::eval_definee(c, slf);
    yield_with_self(c, b, &[slf], slf, dc)
}
fn bo_instance_exec(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if b.is_null() { return err(c, c.vm().c_local_jump_error.get(), "no block given (yield)"); }
    let dc = crate::rt::eval_definee(c, slf);
    let kw = kw_given(c);
    yield_with_self_kw(c, b, a, slf, dc, kw)
}
pub(crate) fn bo_method_missing(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.is_empty() { return arg_err(c, "no method name given"); }
    if !a[0].is_sym() { return arg_err(c, format!("method name must be a Symbol but {} is given", class_name_for_err(c, a[0]))); }
    if let Some(v) = crate::vm::mm_reason_take() { return crate::vm::visibility_raise(c, v, a[0].as_sym(), slf); }
    no_method_error_args(c, slf, a[0].as_sym(), 0, &a[1..])
}
fn bo_singleton_method_added(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(NIL) }

// ---- Kernel instance methods ---------------------------------------------------------------

fn k_class(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(real_class_of(c, slf)) }
fn k_singleton_class(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if slf.is_fixnum() || slf.is_sym() || slf.is_flonum() || is_bignum(slf) || is_heap_float(slf) || crate::rt::is_fstring(c, slf) { return type_err(c, "can't define singleton"); }
    if slf.is_heap() && obj_flags(slf) & FL_CHILLED_ANY != 0 { crate::vm::unchill(c, slf)?; }
    Ok(singleton_class_exposed(c, slf))
}
fn k_frozen(_c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(is_frozen(slf))) }
fn k_freeze(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if slf.is_heap() {
        obj_set_flags(slf, FL_FROZEN); obj_clear_flags(slf, FL_CHILLED_ANY);
        // rb_freeze_singleton_class (not on to a metaclass's metaclass)
        let k = crate::heap::obj_klass(slf);
        if c.class(k).is_singleton && c.class(k).attached == slf && !(is_class_or_module(slf) && c.class(slf).is_singleton) { obj_set_flags(k, FL_FROZEN); }
    }
    Ok(slf)
}
fn k_nil_p(_c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(slf == NIL)) }
fn k_eqq(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if slf == a[0] { return Ok(TRUE); }
    let r = funcall(c, slf, s::eq, &[a[0]], std::ptr::null())?;
    Ok(Value::bool(r.truthy()))
}
fn k_match(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(NIL) }
fn k_nmatch(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let r = funcall(c, slf, s::match_op, &[a[0]], std::ptr::null())?;
    Ok(Value::bool(r.falsy()))
}
fn k_cmp(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if slf == a[0] { return Ok(Value::fix(0)); }
    let r = funcall(c, slf, s::eq, &[a[0]], std::ptr::null())?;
    Ok(if r.truthy() { Value::fix(0) } else { NIL })
}
fn k_eql(_c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { Ok(Value::bool(slf == a[0])) }
fn k_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if slf == c.vm().main_obj.get() { return Ok(c.str_new(b"main")); }
    Ok(str_new_rb(c, &any_to_s_str(c, slf)))
}
pub fn k_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if slf == c.vm().main_obj.get() { return Ok(c.str_new(b"main")); }
    let ivs = ivar_list(c, slf);
    let mut ivs: Vec<(Sym, Value)> = ivs.into_iter().filter(|(n, _)| sym_bytes(*n).first() == Some(&b'@')).collect();
    // 4.0: a (private) #instance_variables_to_inspect picks the ivars, in its order
    let vti = intern(b"instance_variables_to_inspect");
    if !ivs.is_empty() && !lookup_method(c, class_of(slf), vti).is_null() {
        let r = funcall(c, slf, vti, &[], std::ptr::null())?;
        if r != NIL {
            if !crate::array::is_array(r) { return type_err(c, format!("Expected #instance_variables_to_inspect to return an Array or nil, but it returned {}", class_name_for_err(c, r))); }
            let names = c.ary(r).to_vec();
            ivs = names.iter().filter_map(|&n| if n.is_sym() { ivs.iter().find(|x| x.0 == n.as_sym()).copied() } else { None }).collect();
        }
    }
    let k = real_class_of(c, slf);
    if ivs.is_empty() { return Ok(str_new_rb(c, &format!("#<{}:{:#018x}>", class_path(c, k), slf.raw()))); }
    if !rec_enter(c, slf, s::inspect)? { return Ok(str_new_rb(c, &format!("#<{}:{:#018x} ...>", class_path(c, k), slf.raw()))); }
    let mut out = format!("#<{}:{:#018x}", class_path(c, k), slf.raw());
    let keep = c.ary_new();
    for (i, (n, v)) in ivs.iter().enumerate() {
        c.ary_push(keep, *v);
        let r = inspect_str(c, *v);
        let r = match r { Ok(r) => r, Err(e) => { rec_leave(c, slf, s::inspect); return Err(e); } };
        out.push_str(if i == 0 { " " } else { ", " });
        out.push_str(&sym_str(*n)); out.push('='); out.push_str(&r);
    }
    rec_leave(c, slf, s::inspect);
    out.push('>');
    Ok(str_new_rb(c, &out))
}
fn k_is_a(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if !is_class_or_module(a[0]) { return type_err(c, "class or module required"); }
    Ok(Value::bool(is_kind_of(c, slf, a[0])))
}
fn k_instance_of(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if !is_class_or_module(a[0]) { return type_err(c, "class or module required"); }
    Ok(Value::bool(real_class_of(c, slf) == a[0]))
}
pub(crate) fn k_respond_to(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
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
pub(crate) fn k_respond_to_missing(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(FALSE) }
fn k_ivar_get(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let n = to_sym_arg(c, a[0])?;
    check_ivar_name(c, n, slf, a[0])?;
    let v = ivar_get(c, slf, n);
    if crate::builtins::ractor::multi() { crate::builtins::ractor::class_ivar_get_check(c, slf, n, v)?; }
    Ok(v)
}
fn check_ivar_name(c: &mut Ctx, n: Sym, slf: Value, orig: Value) -> Result<(), Esc> {
    let b = sym_bytes(n);
    if b.len() < 2 || b[0] != b'@' || b[1] == b'@' || b[1].is_ascii_digit() {
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("'{}' is not allowed as an instance variable name", sym_str(n)), orig, slf)?;
        return Err(raise_exc(c, e));
    }
    Ok(())
}
fn k_ivar_set(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let n = to_sym_arg(c, a[0])?;
    check_ivar_name(c, n, slf, a[0])?;
    if crate::builtins::ractor::multi() { crate::builtins::ractor::class_ivar_set_check(c, slf, n)?; }
    ivar_set(c, slf, n, a[1])
}
fn k_ivar_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let n = to_sym_arg(c, a[0])?;
    check_ivar_name(c, n, slf, a[0])?;
    Ok(Value::bool(ivar_defined(c, slf, n)))
}
fn k_ivars(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let a = c.ary_new();
    for (n, _) in ivar_list(c, slf) { if sym_bytes(n).first() == Some(&b'@') { c.ary_push(a, Value::sym(n)); } }
    Ok(a)
}
fn k_remove_ivar(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let n = to_sym_arg(c, a[0])?;
    let nb = sym_bytes(n);
    if !(nb.len() > 1 && nb[0] == b'@' && nb[1] != b'@' && !nb[1].is_ascii_digit() && nb[1..].iter().all(|x| x.is_ascii_alphanumeric() || *x == b'_' || *x >= 0x80)) {
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("'{}' is not allowed as an instance variable name", sym_str(n)), Value::sym(n), slf)?;
        return Err(raise_exc(c, e));
    }
    if !slf.is_heap() || is_frozen(slf) { return Err(raise_frozen(c, slf)); }
    match ivar_remove(c, slf, n) {
        Some(v) => Ok(v),
        None => {
            let ins = inspect_str(c, slf)?;
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("instance variable {} not defined for {}", sym_str(n), ins), Value::sym(n), slf)?;
            Err(raise_exc(c, e))
        }
    }
}
fn k_tap(c: &mut Ctx, slf: Value, _a: &[Value], b: Blk) -> RV { yield1(c, b, slf)?; Ok(slf) }
fn k_then(c: &mut Ctx, slf: Value, _a: &[Value], b: Blk) -> RV {
    if b.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"then"), &[], Value::fix(1)); }
    yield1(c, b, slf)
}
fn k_itself(_c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(slf) }

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
    crate::builtins::misc::copy_finalizer(c, slf, n)?;
    if obj_flags(slf) & FL_PRIVATE_CALL != 0 { obj_set_flags(n, FL_PRIVATE_CALL); }   // an ivar in CRuby
    if obj_flags(slf) & FL_EXIVAR != 0 { for (k, v) in ivar_list(c, slf) { ivar_set_raw(c, n, k, v); } }
    if clone {
        // singleton class copy
        let k = obj_klass(slf);
        if c.class(k).is_singleton && !is_class_or_module(slf) {
            let sk = singleton_class(c, n);
            // entries owned by the copy (rb_singleton_class_clone: Method#owner is the new singleton class)
            let mt: Vec<(Sym, MethodEntry)> = c.class(k).mtbl.iter().filter_map(|(a, b)| c.me_ref(*b).map(|o| (*a, MethodEntry {
                name: o.name, owner: sk, visi: o.visi, kind: o.kind, orig_owner: if o.orig_owner == k { sk } else { o.orig_owner }, cref: o.cref }))).collect();
            let nt = &mut c.class_mut(sk).mtbl;
            for (m, e) in mt { nt.insert(m, Box::into_raw(Box::new(e))); }
            let inc = c.class(k).includes.clone();
            for m in inc { include_module_c(c, sk, m); }
            let cs: Vec<(Sym, Value)> = c.class(k).consts.iter().map(|(a, b)| (*a, *b)).collect();
            for (n, v) in cs { c.class_mut(sk).consts.insert(n, v); }
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
        if freeze.is_none() { obj_set_flags(n, obj_flags(slf) & FL_CHILLED_ANY); }
        let k = obj_klass(n);
        if c.class(k).is_singleton && c.class(k).attached == n && !is_class_or_module(n) {
            if fz { obj_set_flags(k, FL_FROZEN); } else { obj_clear_flags(k, FL_FROZEN); }
        }
    }
    Ok(n)
}

pub(crate) fn k_dup(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { obj_dup(c, slf, false, None) }
pub(crate) fn k_clone(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut fz = None;
    if let Some(h) = kw_arg(c, a) {
        if let Some(v) = hash_get_opt(c, h, sym_value("freeze"))? {
            if v != NIL && v != TRUE && v != FALSE { let n = crate::builtins::kernel::class_name_for_err(c, v); return arg_err(c, format!("unexpected value for freeze: {}", n)); }
            if v != NIL { fz = Some(v == TRUE); }
        }
    }
    if !slf.is_heap() || is_heap_float(slf) || is_bignum(slf) {
        if fz == Some(false) { return arg_err(c, format!("can't unfreeze {}", class_path(c, real_class_of(c, slf)))); }
        return Ok(slf);
    }
    obj_dup(c, slf, true, fz)
}
fn k_init_copy(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if slf == a[0] { return Ok(slf); }
    check_frozen(c, slf)?;
    if real_class_of(c, slf) != real_class_of(c, a[0]) { return type_err(c, "initialize_copy should take same class object"); }
    Ok(slf)
}
fn k_init_dup(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    funcall(c, slf, s::initialize_copy, &[a[0]], std::ptr::null())?;
    Ok(slf)
}
fn k_extend(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    for &m in a.iter().rev() {
        if !is_class_or_module(m) || !c.class(m).is_module {
            return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, m)));
        }
        if c.class(m).refined { return type_err(c, "Cannot extend object with refinement"); }
        funcall(c, m, intern(b"extend_object"), &[slf], std::ptr::null())?;
        funcall(c, m, s::extended, &[slf], std::ptr::null())?;
    }
    Ok(slf)
}
fn m_extend_object(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if !c.class(slf).is_module { return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, slf))); }
    if c.class(slf).refined { return arg_err(c, "refinement module is not allowed"); }
    if is_frozen(a[0]) && a[0].is_heap() { return Err(raise_frozen(c, a[0])); }
    let k = singleton_class(c, a[0]);
    if k == NIL { return type_err(c, "can't define singleton"); }
    include_module_c(c, k, slf);
    c.class_mut(slf).extended_by.push(a[0]);
    Ok(a[0])
}
fn k_display(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let s = to_s_value(c, slf)?;
    let b = c.str_bytes(s).to_vec();
    stdout_write(c, &b)?;
    Ok(NIL)
}
fn k_hash(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { obj_hash(c, slf, a, b) }

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
    let mut anc = ancestors(c, slf_class).to_vec();
    // without inherited ones: from the origin (modules prepended to the class are not its own)
    if !inherit { if let Some(i) = anc.iter().position(|&k| k == slf_class) { anc.drain(..i); } }
    else if c.class(slf_class).refined { let k = c.class(slf_class).attached; anc.extend(ancestors(c, k).iter().copied()); }
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

fn k_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let all = a.first().map(|v| v.truthy()).unwrap_or(true);
    if !all { return k_singleton_methods(c, slf, &[FALSE], _b); }
    Ok(method_list(c, class_of(slf), |v| v != VISI_PRIVATE, true))
}
fn k_public_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { Ok(obj_method_list(c, slf, |v| v == VISI_PUBLIC, a.first().is_none_or(|v| v.truthy()))) }
/// class_instance_method_list(obj = 1): without recursion, the singleton classes and modules above the
/// object's class of its own, through its first ordinary class.
fn obj_method_list(c: &mut Ctx, slf: Value, filter: impl Fn(u8) -> bool, recur: bool) -> Value {
    let k = class_of(slf);
    if recur { return method_list(c, k, filter, true); }
    let a = c.ary_new();
    let mut seen = std::collections::HashSet::new();
    for m in ancestors(c, k).to_vec() {
        let mut names: Vec<(Sym, *const MethodEntry)> = c.class(m).mtbl.iter().map(|(a, b)| (*a, *b)).collect();
        names.sort_by_key(|x| x.0);
        for (n, me) in names {
            if !seen.insert(n) { continue; }
            let Some(me) = c.me_ref(me) else { continue };
            if !matches!(me.kind, MKind::Undef) && filter(me.visi) { c.ary_push(a, Value::sym(n)); }
        }
        if !c.class(m).is_singleton && !c.class(m).is_module { break; }
    }
    a
}
fn k_private_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { Ok(obj_method_list(c, slf, |v| v == VISI_PRIVATE, a.first().is_none_or(|v| v.truthy()))) }
fn k_protected_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { Ok(obj_method_list(c, slf, |v| v == VISI_PROTECTED, a.first().is_none_or(|v| v.truthy()))) }
/// rb_obj_singleton_methods: the singleton class, then (all = true) the modules and singleton classes
/// above it up to the first ordinary class; the first entry of a name decides, private / undef ones hide it.
fn k_singleton_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let all = a.first().is_none_or(|v| v.truthy());
    let out = c.ary_new();
    if !slf.is_heap() { return Ok(out); }
    let k = obj_klass(slf);
    if !c.class(k).is_singleton { return Ok(out); }
    let chain: Vec<Value> = ancestors(c, k).to_vec();
    let start = chain.iter().position(|&x| x == k).unwrap_or(0);
    // up to the first ordinary class, less the modules prepended to it
    let mut end = chain.iter().skip(start + 1).position(|&x| !c.class(x).is_singleton && !c.class(x).is_module).map_or(chain.len(), |j| j + start + 1);
    if end < chain.len() { let ce = chain[end]; end -= ancestors(c, ce).iter().position(|&x| x == ce).unwrap_or(0); }
    if !all { end = start + 1; }
    let mut seen: Vec<Sym> = vec![];
    for (i, &m) in chain.iter().enumerate().take(end) {
        if i < start { continue; }   // prepended to the singleton class: CRuby's origin is skipped too
        let mut names: Vec<(Sym, *const MethodEntry)> = c.class(m).mtbl.iter().map(|(a, b)| (*a, *b)).collect();
        names.sort_by_key(|x| x.0);
        for (n, me) in names {
            if seen.contains(&n) { continue; }
            seen.push(n);
            if me_visi(c, me) != VISI_PRIVATE && !matches!(me_kind(c, me), Some(MKind::Undef)) { c.ary_push(out, Value::sym(n)); }
        }
    }
    Ok(out)
}
fn k_method(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mid = to_sym_arg(c, a[0])?;
    crate::builtins::proc::method_object(c, slf, mid, false)
}
fn k_public_method(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mid = to_sym_arg(c, a[0])?;
    crate::builtins::proc::method_object(c, slf, mid, true)
}
fn k_singleton_method(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mid = to_sym_arg(c, a[0])?;
    if slf.is_heap() {
        let k = obj_klass(slf);
        if c.class(k).is_singleton {
            let found = ancestors(c, k).iter().copied().take_while(|&m| c.class(m).is_singleton || c.class(m).is_module)
                .find_map(|m| c.class(m).mtbl.get(&mid).copied()).is_some_and(|me| !matches!(me_kind(c, me), Some(MKind::Undef)));
            if found { return crate::builtins::proc::method_object(c, slf, mid, false); }
        }
    }
    let ins = inspect_str(c, slf)?;
    let e = make_name_error(c, c.vm().c_name_error.get(), &format!("undefined singleton method '{}' for {}", sym_str(mid), ins), Value::sym(mid), slf)?;
    Err(raise_exc(c, e))
}
fn k_define_singleton_method(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let k = singleton_class(c, slf);
    m_define_method(c, k, a, b)
}
fn k_enum_for(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let mid = if a.is_empty() { s::each } else { to_sym_arg(c, a[0])? };
    let size_fn = if b.is_null() { NIL } else { block_to_proc(c, b)? };
    let e = crate::builtins::enumerator::enum_for(c, slf, mid, if a.is_empty() { &[] } else { &a[1..] }, size_fn)?;
    if b.is_null() { crate::builtins::enumerator::renum_mut(c, e).lz |= crate::builtins::lazy::SIZE_EXACT; }
    Ok(e)
}

// ---- Module --------------------------------------------------------------------------------

pub fn call_method_added(c: &mut Ctx, klass: Value, mid: Sym) -> Result<(), Esc> {
    call_method_hook(c, klass, mid, s::method_added, s::singleton_method_added)
}
/// CALL_METHOD_HOOK: klass.hook(mid), or the attached object's singleton_hook(mid) for a singleton class.
/// The default (Module's / BasicObject's empty one) is skipped; an undefined hook goes to method_missing.
pub fn call_method_hook(c: &mut Ctx, klass: Value, mid: Sym, hook: Sym, singleton_hook: Sym) -> Result<(), Esc> {
    let (recv, h, dflt) = if c.class(klass).is_singleton { (c.class(klass).attached, singleton_hook, c.vm().c_basic_object.get()) } else { (klass, hook, c.vm().c_module.get()) };
    let me = lookup_method(c, class_of(recv), h);
    // the default runs only when traced (its c-call event)
    if (me.is_null() && crate::rt::booted()) || (!me.is_null() && (me_owner(c, me) != dflt || crate::vm::trace_mask() & crate::vm::EV_C_CALL != 0)) {
        funcall(c, recv, h, &[Value::sym(mid)], std::ptr::null())?;
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

/// A copy of an alias entry (visibility change, module_function) is still called by the alias' name.
fn keep_alias_name(c: &Ctx, from: *const MethodEntry, to: *const MethodEntry) {
    let n = c.vm().alias_names.borrow().get(&(from as usize)).copied();
    if let Some(n) = n { c.vm().alias_names.borrow_mut().insert(to as usize, n); }
}

/// An alias made at boot: the entry is `klass`'s own, nobody listens to method_added yet.
pub fn boot_alias(c: &mut Ctx, klass: Value, newname: Sym, oldname: Sym) {
    let Some(&me) = c.class(klass).mtbl.get(&oldname) else { return };
    let Some(o) = me_copy(c, me) else { return };
    let n = Box::into_raw(Box::new(o)) as *const MethodEntry;
    c.vm().alias_names.borrow_mut().insert(n as usize, newname);
    define_method_me(c, klass, newname, n);
}
pub fn alias_method(c: &mut Ctx, klass: Value, newname: Sym, oldname: Sym) -> Result<(), Esc> {
    check_frozen(c, klass)?;
    let mut me = search_method(c, klass, oldname);
    if (me.is_null() || matches!(me_kind(c, me), Some(MKind::Undef))) && c.class(klass).is_module {
        me = search_method(c, c.vm().c_object.get(), oldname);
    }
    if me.is_null() || matches!(me_kind(c, me), Some(MKind::Undef)) {
        let msg = format!("undefined method '{}' for {} '{}'", sym_str(oldname), if c.class(klass).is_module { "module" } else { "class" }, class_path(c, klass));
        let e = make_name_error(c, c.vm().c_name_error.get(), &msg, Value::sym(oldname), klass)?;
        return Err(raise_exc(c, e));
    }
    let Some(mut o) = me_copy(c, me) else { return Ok(()) };
    if is_special_private(newname) { o.visi = VISI_PRIVATE; }
    // the alias belongs to klass (Method#owner); super still searches from the original's class (orig_owner)
    o.owner = klass;
    let oname = o.name;
    let n = Box::into_raw(Box::new(o)) as *const MethodEntry;
    if newname != oname { c.vm().alias_names.borrow_mut().insert(n as usize, newname); }
    crate::rt::note_redefinition(c, klass, newname);
    define_user_method(c, klass, newname, n);
    call_method_added(c, klass, newname)?;
    Ok(())
}

pub fn undef_method(c: &mut Ctx, klass: Value, mid: Sym) -> Result<(), Esc> {
    if crate::rt::booted() { check_frozen(c, klass)?; }
    let mut me = search_method(c, klass, mid);
    // a refinement may undef a method of the class it refines
    if me.is_null() && c.class(klass).refined { me = search_method(c, c.class(klass).attached, mid); }
    if me.is_null() || matches!(me_kind(c, me), Some(MKind::Undef)) {
        // rb_print_undef: a class's (module's) singleton class is named by the class
        let att = c.class(klass).attached;
        let shown = if c.class(klass).is_singleton && is_class_or_module(att) { class_path(c, att) } else { class_path(c, klass) };
        let msg = format!("undefined method '{}' for {} '{}'", sym_str(mid), if c.class(klass).is_module { "module" } else { "class" }, shown);
        let e = make_name_error(c, c.vm().c_name_error.get(), &msg, Value::sym(mid), klass)?;
        return Err(raise_exc(c, e));
    }
    if crate::builtins::misc::warning_category_enabled(2) { warn_opt_redefinition(c, klass, mid, std::ptr::null())?; }
    define_method_me(c, klass, mid, new_me(mid, klass, VISI_PUBLIC, MKind::Undef));
    crate::rt::note_redefinition(c, klass, mid);
    if crate::rt::booted() { call_method_hook(c, klass, mid, intern(b"method_undefined"), intern(b"singleton_method_undefined"))?; }
    Ok(())
}

fn m_alias_method(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 2, 2)?;
    let n = to_sym_arg(c, a[0])?; let o = to_sym_arg(c, a[1])?;
    alias_method(c, slf, n, o)?;
    Ok(Value::sym(n))
}
fn m_undef_method(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    for &x in a { let m = to_sym_arg(c, x)?; undef_method(c, slf, m)?; }
    Ok(slf)
}
fn m_remove_method(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    for &x in a {
        let m = to_sym_arg(c, x)?;
        check_frozen(c, slf)?;
        if crate::builtins::misc::warning_category_enabled(2) { warn_opt_redefinition(c, slf, m, std::ptr::null())?; }
        if c.class_mut(slf).mtbl.remove(&m).is_none() {
            let msg = format!("method '{}' not defined in {}", sym_str(m), class_path(c, slf));
            let e = make_name_error(c, c.vm().c_name_error.get(), &msg, Value::sym(m), slf)?;
            return Err(raise_exc(c, e));
        }
        bump_method_serial(); c.vm().mcache.borrow_mut().clear();
        crate::rt::note_redefinition(c, slf, m);
        call_method_hook(c, slf, m, intern(b"method_removed"), intern(b"singleton_method_removed"))?;
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
    let visi = match scope_visi(c, k) { VISI_MODFUNC => VISI_PRIVATE, v => v };
    let iv = intern(format!("@{}", sym_str(n)).as_bytes());
    if reader { crate::rt::note_redefinition(c, k, n); define_user_method(c, k, n, new_me(n, k, visi, MKind::AttrReader(iv))); call_method_added(c, k, n)?; }
    if writer {
        let w = intern(format!("{}=", sym_str(n)).as_bytes());
        crate::rt::note_redefinition(c, k, w);
        define_user_method(c, k, w, new_me(w, k, visi, MKind::AttrWriter(iv)));
        call_method_added(c, k, w)?;
    }
    Ok(())
}
fn cur_default_visi(c: &Ctx) -> u8 {
    let Some(p) = c.caller_frame() else { return VISI_PUBLIC };
    if p.kind == FK_CLASS { if p.visi == VISI_MODFUNC { VISI_PRIVATE } else { p.visi } } else { VISI_PUBLIC }
}
fn m_attr_reader(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let names = attr_names(c, a)?;
    let out = c.ary_new();
    for n in names { def_attr(c, slf, n, true, false)?; c.ary_push(out, Value::sym(n)); }
    Ok(out)
}
fn m_attr_writer(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let names = attr_names(c, a)?;
    let out = c.ary_new();
    for n in names { def_attr(c, slf, n, false, true)?; c.ary_push(out, Value::sym(intern(format!("{}=", sym_str(n)).as_bytes()))); }
    Ok(out)
}
fn m_attr_accessor(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let names = attr_names(c, a)?;
    let out = c.ary_new();
    for n in names { def_attr(c, slf, n, true, true)?; c.ary_push(out, Value::sym(n)); c.ary_push(out, Value::sym(intern(format!("{}=", sym_str(n)).as_bytes()))); }
    Ok(out)
}
fn m_attr(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if a.len() == 2 && (a[1] == TRUE || a[1] == FALSE) {
        // rb_category_warning(DEPRECATED): verbose and Warning[:deprecated]
        if c.vm().verbose.get() == TRUE && crate::builtins::misc::warning_category_enabled(0) {
            let at = c.caller_frame().map(|f| format!("{}:{}: ", frame_file(f), f.line())).unwrap_or_default();
            crate::builtins::kernel::warning_write(c, format!("{}warning: optional boolean argument is obsoleted\n", at).as_bytes())?;
        }
        let n = to_sym_arg(c, a[0])?;
        def_attr(c, slf, n, true, a[1] == TRUE)?;
        if a[1] == TRUE { let w = Value::sym(intern(format!("{}=", sym_str(n)).as_bytes())); return Ok(c.ary_new_from(&[Value::sym(n), w])); }
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
        // from the origin: a prepended module's method is not the one changed
        let anc = ancestors(c, slf);
        let first = anc[anc.iter().position(|&k| k == slf).unwrap_or(0)..].iter().find_map(|&k| c.class(k).mtbl.get(&m).copied());
        let me = match first {
            Some(f) if matches!(me_kind(c, f), Some(MKind::ZSuper)) => if me_owner(c, f) == slf { f } else { search_method(c, slf, m) },
            Some(f) => f,
            None => search_method(c, slf, m),
        };
        // a refinement changes the visibility of the refined class's method
        let me = if me.is_null() && c.class(slf).refined { search_method(c, c.class(slf).attached, m) } else { me };
        let me = if me.is_null() && c.class(slf).is_module { search_method(c, c.vm().c_object.get(), m) } else { me };
        if me.is_null() || matches!(me_kind(c, me), Some(MKind::Undef)) {
            let msg = format!("undefined method '{}' for {} '{}'", sym_str(m), if c.class(slf).is_module { "module" } else { "class" }, class_path(c, slf));
            let e = make_name_error(c, c.vm().c_name_error.get(), &msg, Value::sym(m), slf)?;
            return Err(raise_exc(c, e));
        }
        let Some(o) = me_copy(c, me) else { continue };
        // a private Integer#+ leaves the fast path too (CRuby raises NoMethodError)
        crate::rt::note_redefinition(c, slf, m);
        if visi == VISI_MODFUNC {
            let n = Box::into_raw(Box::new(MethodEntry { name: o.name, owner: slf, visi: VISI_PRIVATE, kind: o.kind, orig_owner: o.orig_owner, cref: o.cref }));
            keep_alias_name(c, me, n);
            define_method_me(c, slf, m, n);   // its own entry turns private in place: no redefinition
            let sk = singleton_class(c, slf);
            let n2 = Box::into_raw(Box::new(MethodEntry { name: o.name, owner: sk, visi: VISI_PUBLIC, kind: o.kind, orig_owner: sk, cref: o.cref }));
            keep_alias_name(c, me, n2);
            define_user_method(c, sk, m, n2);
            call_method_added(c, sk, m)?;
            continue;
        }
        // rb_export_method: the entry found first (a ZSuper one included) changes in place when it is slf's own
        if o.owner == slf { c.me_set_visi(me, visi); bump_method_serial(); c.vm().mcache.borrow_mut().clear(); }
        else if o.visi == visi { continue; }   // an inherited one already so is left alone
        else if c.class(slf).refined {
            // in a refinement: a copy with the new visibility (refined lookups do not follow ZSuper entries)
            let n = Box::into_raw(Box::new(MethodEntry { name: o.name, owner: slf, visi, kind: o.kind, orig_owner: o.orig_owner, cref: o.cref }));
            keep_alias_name(c, me, n);
            define_method_me(c, slf, m, n);
        } else {
            // a ZSuper entry: calls reach the method above (a later redefinition there too)
            define_method_me(c, slf, m, new_me(m, slf, visi, MKind::ZSuper));
            crate::builtins::object::call_method_added(c, slf, m)?;
        }
    }
    std::hint::black_box(keep);
    Ok(if a.len() == 1 { a[0] } else { c.ary_new_from(a) })
}
fn m_private(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { set_visibility(c, slf, a, VISI_PRIVATE) }
fn m_public(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { set_visibility(c, slf, a, VISI_PUBLIC) }
fn m_protected(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { set_visibility(c, slf, a, VISI_PROTECTED) }
fn m_module_function(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if !c.class(slf).is_module { return type_err(c, "module_function must be called for modules"); }
    set_visibility(c, slf, a, VISI_MODFUNC)
}
fn top_private(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { set_visibility(c, c.vm().c_object.get(), a, VISI_PRIVATE) }
fn top_public(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV { set_visibility(c, c.vm().c_object.get(), a, VISI_PUBLIC) }
fn m_private_class_method(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let k = singleton_class(c, slf); set_visibility(c, k, a, VISI_PRIVATE)?; Ok(NIL) }
fn m_public_class_method(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let k = singleton_class(c, slf); set_visibility(c, k, a, VISI_PUBLIC)?; Ok(NIL) }

// a private constant referenced with a scope: (receiver, name) for the const_missing it calls
thread_local! { pub static PRIVATE_REF: std::cell::Cell<Option<(Value, Sym)>> = const { std::cell::Cell::new(None) }; }
/// The scope visibility define_method uses (rb_vm_cref_in_context(mod, mod)): the caller's default when it
/// runs in `m` itself (class body, class_exec block), public otherwise.
fn scope_visi(c: &Ctx, m: Value) -> u8 {
    let Some(p) = c.caller_frame() else { return VISI_PUBLIC };
    if p.fp[0].get() != m { return VISI_PUBLIC; }
    let in_m = if p.kind == FK_BLOCK { p.defined_class() == m } else { c.cref_ref(p.cref()).map_or(c.vm().c_object.get(), |r| r.klass) == m };
    if !in_m { return VISI_PUBLIC; }
    if p.kind != FK_BLOCK || p.visi != VISI_PUBLIC { return p.visi; }
    // a block inside the scope shares its visibility (CRuby keeps it in the cref): the enclosing blocks' setting
    let mut f = c.prev_frame(p);
    while let Some(x) = f {
        if x.kind == FK_CFUNC { f = c.prev_frame(x); continue; }
        if x.kind != FK_BLOCK || x.fp[0].get() != m || x.defined_class() != m { break; }
        if x.visi != VISI_PUBLIC { return x.visi; }
        f = c.prev_frame(x);
    }
    VISI_PUBLIC
}
fn is_special_private(mid: Sym) -> bool {
    mid == s::initialize || mid == s::initialize_copy || mid == s::respond_to_missing_p || mid == s::mid_initialize_dup || mid == s::mid_initialize_clone
}
pub fn m_define_method(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let mid = to_sym_arg(c, a[0])?;
    let sv = scope_visi(c, slf);
    let visi = if is_special_private(mid) && sv != VISI_MODFUNC { VISI_PRIVATE } else { sv };
    let modfunc = visi == VISI_MODFUNC;
    let visi = if modfunc { VISI_PRIVATE } else { visi };
    check_frozen(c, slf)?;
    let me = if a.len() == 2 {
        let body = a[1];
        if is_proc(body) {
            new_me(mid, slf, visi, MKind::Bmethod(body))
        } else if is_method_obj(body) {
            let Some(o) = me_copy(c, c.method_obj(body).me) else { return type_err(c, "bad method object") };
            if !c.class(o.owner).is_module && !is_kind_of_class(c, slf, o.owner) {
                if c.class(o.owner).is_singleton { return type_err(c, "can't bind singleton method to a different class"); }
                return type_err(c, format!("bind argument must be a subclass of {}", class_path(c, o.owner)));
            }
            // like an alias: __method__ stays the original name, __callee__ is the new one
            let n = Box::into_raw(Box::new(MethodEntry { name: o.name, owner: slf, visi, kind: o.kind, orig_owner: slf, cref: o.cref }));
            if mid != o.name { c.vm().alias_names.borrow_mut().insert(n as usize, mid); }
            n as *const MethodEntry
        } else {
            return type_err(c, format!("wrong argument type {} (expected Proc/Method/UnboundMethod)", class_path(c, real_class_of(c, body))));
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
    define_user_method(c, slf, mid, me);
    if modfunc {
        let Some(o) = me_copy(c, me) else { return Ok(Value::sym(mid)) };
        let sk = singleton_class(c, slf);
        let n = Box::into_raw(Box::new(MethodEntry { name: o.name, owner: sk, visi: VISI_PUBLIC, kind: o.kind, orig_owner: sk, cref: o.cref }));
        keep_alias_name(c, me, n);
        crate::rt::note_redefinition(c, sk, mid);
        define_user_method(c, sk, mid, n);
    }
    call_method_added(c, slf, mid)?;
    Ok(Value::sym(mid))
}

fn is_kind_of_class(c: &Ctx, k: Value, owner: Value) -> bool { k == owner || ancestors(c, k).contains(&owner) }

fn m_instance_method(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mid = to_sym_arg(c, a[0])?;
    crate::builtins::proc::unbound_method_object(c, slf, mid, false)
}
fn m_public_instance_method(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
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
fn m_method_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { method_defined_impl(c, slf, a, |v| v != VISI_PRIVATE) }
fn m_public_method_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { method_defined_impl(c, slf, a, |v| v == VISI_PUBLIC) }
fn m_private_method_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { method_defined_impl(c, slf, a, |v| v == VISI_PRIVATE) }
fn m_protected_method_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { method_defined_impl(c, slf, a, |v| v == VISI_PROTECTED) }
fn m_instance_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let inherit = a.first().map(|v| v.truthy()).unwrap_or(true);
    Ok(method_list(c, slf, |v| v != VISI_PRIVATE, inherit))
}
fn m_public_instance_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let inherit = a.first().map(|v| v.truthy()).unwrap_or(true);
    Ok(method_list(c, slf, |v| v == VISI_PUBLIC, inherit))
}
fn m_private_instance_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let inherit = a.first().map(|v| v.truthy()).unwrap_or(true);
    Ok(method_list(c, slf, |v| v == VISI_PRIVATE, inherit))
}
fn m_protected_instance_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let inherit = a.first().map(|v| v.truthy()).unwrap_or(true);
    Ok(method_list(c, slf, |v| v == VISI_PROTECTED, inherit))
}

fn m_name(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    match c.class(slf).name { Some(n) => Ok(n), None => Ok(NIL) }
}
/// Module#set_temporary_name: a name for an anonymous module (nil removes it); modules named under it follow.
fn m_set_temporary_name(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if has_perm_name(c, slf) { return err(c, c.vm().c_runtime_error.get(), "can't change permanent name"); }
    let name = if a[0] == NIL { None } else {
        let s = expect_str(c, a[0])?;
        let b = c.str_bytes(s).to_vec();
        if b.is_empty() { return arg_err(c, "empty class/module name"); }
        let seg_ok = |p: &[u8]| !p.is_empty() && p[0].is_ascii_uppercase() && p.iter().all(|x| x.is_ascii_alphanumeric() || *x == b'_' || *x >= 0x80);
        let t = b.strip_prefix(b"::").unwrap_or(&b);
        let segs: Vec<&[u8]> = t.split(|&x| x == b':').collect();
        // "A::B": the parts between "::" (pairs of ':') are all constant names
        if segs.len() % 2 == 1 && segs.iter().enumerate().all(|(i, p)| if i % 2 == 1 { p.is_empty() } else { seg_ok(p) }) {
            return arg_err(c, "the temporary name must not be a constant path to avoid confusion");
        }
        Some(String::from_utf8_lossy(&b).into_owned())
    };
    set_temp_name(c, slf, name, &mut vec![]);
    Ok(slf)
}
/// `seen`: modules renamed so far (a nested module may refer back: m::N::Back = m).
fn set_temp_name(c: &mut Ctx, k: Value, name: Option<String>, seen: &mut Vec<Value>) {
    if seen.contains(&k) { return; }
    seen.push(k);
    let v = name.as_ref().map(|n| c.str_new_frozen(n.as_bytes()));
    let cd = c.class_mut(k);
    cd.name = v; cd.temp_name = v.is_some();
    let consts: Vec<(Sym, Value)> = cd.consts.iter().map(|(a, b)| (*a, *b)).collect();
    for (cn, cv) in consts {
        if is_class_or_module(cv) && c.class(cv).temp_name && c.class(cv).lexical_parent == k && c.class(cv).base_name == cn {
            set_temp_name(c, cv, name.as_ref().map(|n| format!("{}::{}", n, sym_str(cn))), seen);
        }
    }
}
fn m_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let cd = c.class(slf);
    if cd.refined {
        let (k, at) = (cd.attached, ivar_get(c, slf, intern(b"__defined_at__")));
        let s = format!("#<refinement:{}@{}>", inspect_str(c, k)?, inspect_str(c, at)?);
        return Ok(c.str_new(s.as_bytes()));
    }
    if cd.is_singleton {
        let att = cd.attached;
        let inner = if is_class_or_module(att) { inspect_str(c, att)? } else { crate::builtins::kernel::any_to_s_str(c, att) };
        return Ok(str_new_rb(c, &format!("#<Class:{}>", inner)));
    }
    let n = class_path(c, slf);
    if c.class(slf).name.is_some() { return Ok(c.str_new_enc(n.as_bytes(), if n.is_ascii() { ENC_ASCII } else { ENC_UTF8 })); }
    Ok(str_new_rb(c, &n))
}
fn m_eqq(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { Ok(Value::bool(is_kind_of(c, a[0], slf))) }
fn m_lt(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if !is_class_or_module(a[0]) { return type_err(c, "compared with non class/module"); }
    if slf == a[0] { return Ok(FALSE); }
    if ancestors(c, slf).contains(&a[0]) { return Ok(TRUE); }
    if ancestors(c, a[0]).contains(&slf) { return Ok(FALSE); }
    Ok(NIL)
}
fn m_le(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { if slf == a[0] { return Ok(TRUE); } m_lt(c, slf, a, b) }
fn m_gt(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if !is_class_or_module(a[0]) { return type_err(c, "compared with non class/module"); }
    m_lt(c, a[0], &[slf], b)
}
fn m_ge(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { if slf == a[0] { return Ok(TRUE); } m_gt(c, slf, a, b) }
fn m_cmp(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let o = a[0];
    if slf == o { return Ok(Value::fix(0)); }
    if !is_class_or_module(o) { return Ok(NIL); }
    if ancestors(c, slf).contains(&o) { return Ok(Value::fix(-1)); }
    if ancestors(c, o).contains(&slf) { return Ok(Value::fix(1)); }
    Ok(NIL)
}
fn m_ancestors(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let v = ancestors_listed(c, slf);
    Ok(c.ary_from_vec(v))
}
fn m_include(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if c.class(slf).refined { return type_err(c, "Refinement#include has been removed"); }
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    for &m in a.iter() {
        if !is_class_or_module(m) || !c.class(m).is_module {
            return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, m)));
        }
        if c.class(m).refined { return type_err(c, "Cannot include refinement"); }
        if m == slf { return arg_err(c, "cyclic include detected"); }
    }
    for &m in a.iter().rev() {
        funcall(c, m, intern(b"append_features"), &[slf], std::ptr::null())?;
        funcall(c, m, s::included, &[slf], std::ptr::null())?;
    }
    Ok(slf)
}
fn m_append_features(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { features_check(c, slf, a[0], "include")?; include_module_c(c, a[0], slf); Ok(slf) }
/// rb_include_module / rb_prepend_module's checks: a class receiver (rebound), frozen target, cycles.
fn features_check(c: &mut Ctx, m: Value, target: Value, what: &str) -> Result<(), Esc> {
    if !c.class(m).is_module { return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, m))); }
    if c.class(m).refined { return arg_err(c, "refinement module is not allowed"); }
    if !is_class_or_module(target) { return type_err(c, format!("wrong argument type {} (expected Class)", class_name_for_err(c, target))); }
    check_frozen(c, target)?;
    if m == target || ancestors(c, m).contains(&target) { return arg_err(c, format!("cyclic {} detected", what)); }
    Ok(())
}
fn m_prepend(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if c.class(slf).refined { return type_err(c, "Refinement#prepend has been removed"); }
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    for &m in a.iter() {
        if !is_class_or_module(m) || !c.class(m).is_module {
            return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, m)));
        }
        if c.class(m).refined { return type_err(c, "Cannot prepend refinement"); }
    }
    for &m in a.iter().rev() {
        funcall(c, m, intern(b"prepend_features"), &[slf], std::ptr::null())?;
        funcall(c, m, s::prepended, &[slf], std::ptr::null())?;
    }
    Ok(slf)
}
fn m_prepend_features(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { features_check(c, slf, a[0], "prepend")?; prepend_module(c, a[0], slf); Ok(slf) }
fn m_hook_noop(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(NIL) }
fn m_include_p(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if !is_class_or_module(a[0]) || !c.class(a[0]).is_module { return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, a[0]))); }
    Ok(Value::bool(slf != a[0] && ancestors(c, slf).contains(&a[0])))
}
fn m_included_modules(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let v: Vec<Value> = ancestors(c, slf).iter().copied().filter(|&k| k != slf && c.class(k).is_module).collect();
    Ok(c.ary_from_vec(v))
}
fn m_class_eval(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if b.is_null() {
        if a.is_empty() || a.len() > 3 { return argc_err(c, a.len(), 1, 3); }
        let src = expect_str(c, a[0])?;
        let (file, line) = crate::builtins::binding::eval_file_line(c, a.get(1).copied(), a.get(2).copied())?;
        let bytes = c.str_bytes(src).to_vec();
        let enc = c.str_enc(src);
        return crate::builtins::binding::eval_in_class(c, slf, &bytes, enc, file, line);
    }
    if !a.is_empty() { return argc_err(c, a.len(), 0, 0); }
    yield_with_self(c, b, &[slf], slf, slf)
}
fn m_class_exec(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if b.is_null() { return err(c, c.vm().c_local_jump_error.get(), "no block given (yield)"); }
    let kw = kw_given(c);
    yield_with_self_kw(c, b, a, slf, slf, kw)
}
/// rb_is_const_name: an uppercase letter, then identifier characters.
pub fn is_const_name(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    let mut ch = t.chars();
    // titlecase (Lt) starts one too
    ch.next().is_some_and(|f| f.is_uppercase() || (f.is_alphabetic() && !f.is_lowercase() && f.to_lowercase().next() != Some(f))) && ch.all(|x| x.is_alphanumeric() || x == '_' || !x.is_ascii())
}
fn wrong_const_name(c: &mut Ctx, slf: Value, p: &[u8]) -> Esc {
    let n = Value::sym(intern(p));
    match make_name_error(c, c.vm().c_name_error.get(), &format!("wrong constant name {}", String::from_utf8_lossy(p)), n, slf) { Ok(e) => raise_exc(c, e), Err(e) => e }
}
/// One constant of `cur` for const_get / const_defined? (rb_const_get / _from / _at): `top` is the first
/// part of a path not starting with "::" (a module then sees Object's constants); later parts skip Object's.
fn const_part(c: &mut Ctx, cur: Value, sym: Sym, inherit: bool, top: bool, define_only: bool) -> Result<Option<Value>, Esc> {
    let o = c.vm().c_object.get();
    if !inherit {
        if let Some(v) = const_get_at(c, cur, sym) { return Ok(Some(v)); }
        if !c.class(cur).autoload.contains_key(&sym) { return Ok(None); }
        if define_only { return Ok(autoload_defined(c, cur, sym).then_some(NIL)); }
        return do_autoload(c, cur, sym);
    }
    for &k in ancestors(c, cur).iter() {
        if !top && k == o && cur != o { break; }
        if let Some(v) = const_get_at(c, k, sym) { return Ok(Some(v)); }
        if c.class(k).autoload.contains_key(&sym) {
            if define_only { if autoload_defined(c, k, sym) { return Ok(Some(NIL)); } continue; }
            if let Some(v) = do_autoload(c, k, sym)? { return Ok(Some(v)); }
        }
    }
    if top && c.class(cur).is_module {
        if let Some(v) = const_get_at(c, o, sym) { return Ok(Some(v)); }
        if c.class(o).autoload.contains_key(&sym) {
            if define_only { return Ok(autoload_defined(c, o, sym).then_some(NIL)); }
            return do_autoload(c, o, sym);
        }
    }
    Ok(None)
}
/// rb_mod_const_get / rb_mod_const_defined: a Symbol is one name; a String may be a path "A::B" / "::A".
fn const_path(c: &mut Ctx, slf: Value, a: &[Value], defined: bool) -> RV {
    check_argc(c, a, 1, 2)?;
    let inherit = a.len() < 2 || a[1].truthy();
    let (name, is_sym, sval) = if a[0].is_sym() { (sym_bytes(a[0].as_sym()).to_vec(), true, a[0]) } else { let s = expect_str(c, a[0])?; (c.str_bytes(s).to_vec(), false, s) };
    if is_sym || !name.windows(2).any(|w| w == b"::") {
        if !is_const_name(&name) { return Err(wrong_const_name(c, slf, &name)); }
        let sym = if is_sym { sval.as_sym() } else { c.str_to_sym(sval).as_sym() };
        return match const_part(c, slf, sym, inherit, true, defined)? {
            Some(v) => {
                if !defined {
                    const_access_check(c, slf, sym, false)?;
                    if crate::builtins::ractor::multi() { crate::builtins::ractor::const_check(c, slf, false, sym, v)?; }
                }
                Ok(if defined { TRUE } else { v })
            }
            None if defined => Ok(FALSE),
            None => const_missing(c, slf, sym),
        };
    }
    let mut cur = slf;
    let mut rest: &[u8] = &name;
    let mut top = true;
    if rest.starts_with(b"::") { cur = c.vm().c_object.get(); rest = &rest[2..]; top = false; if rest.is_empty() { return Err(wrong_const_name(c, slf, &name)); } }
    let parts: Vec<&[u8]> = { let mut v = vec![]; let mut r = rest; loop { match r.windows(2).position(|w| w == b"::") { Some(i) => { v.push(&r[..i]); r = &r[i + 2..]; } None => { v.push(r); break; } } } v };
    for (i, p) in parts.iter().enumerate() {
        if !is_const_name(p) { return Err(wrong_const_name(c, slf, if p.is_empty() { &name } else { p })); }
        if !is_class_or_module(cur) { return type_err(c, format!("{} does not refer to class/module", String::from_utf8_lossy(&name))); }
        let sym = intern(p);
        let last = i + 1 == parts.len();
        let parent = cur;
        match const_part(c, cur, sym, inherit, top && i == 0, defined && last)? {
            Some(v) => cur = v,
            None if defined => return Ok(FALSE),
            None => cur = const_missing(c, cur, sym)?,
        }
        if !defined && crate::builtins::ractor::multi() { crate::builtins::ractor::const_check(c, parent, false, sym, cur)?; }
    }
    Ok(if defined { TRUE } else { cur })
}
fn m_const_get(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { const_path(c, slf, a, false) }
fn m_const_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { const_path(c, slf, a, true) }
/// rb_mod_const_source_location: [file, line] of the constant, [] for one defined in Rust, nil when undefined.
fn m_const_source_location(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let inherit = a.len() < 2 || a[1].truthy();
    let name = if a[0].is_sym() { sym_str(a[0].as_sym()).to_string() } else { let s = expect_str(c, a[0])?; String::from_utf8_lossy(c.str_bytes(s)).into_owned() };
    let bad = |c: &mut Ctx, p: &str| -> RV {
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("wrong constant name {}", p), Value::sym(intern(p.as_bytes())), slf)?;
        Err(raise_exc(c, e))
    };
    if a[0].is_sym() && name.contains("::") { return bad(c, &name); }
    let parts: Vec<&str> = name.split("::").collect();
    let mut cur = slf;
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() { if i == 0 && parts.len() > 1 { cur = c.vm().c_object.get(); continue; } return bad(c, &name); }
        let b = p.as_bytes();
        if !(b[0].is_ascii_uppercase() || (b[0] >= 0x80 && p.chars().next().is_some_and(|ch| ch.is_uppercase()))) || !b.iter().all(|x| x.is_ascii_alphanumeric() || *x == b'_' || *x >= 0x80) { return bad(c, p); }
        let sym = intern(b);
        if !is_class_or_module(cur) { return type_err(c, format!("{} does not refer to class/module", name)); }
        if i + 1 < parts.len() {
            cur = if inherit { match const_get_inherit(c, cur, sym)? { Some(v) => v, None => const_missing(c, cur, sym)? } }
                  else { match const_get_at(c, cur, sym) { Some(v) => v, None => const_missing(c, cur, sym)? } };
            continue;
        }
        let mut list: Vec<Value> = if inherit { ancestors(c, cur).to_vec() } else { vec![cur] };
        if inherit && c.class(cur).is_module { list.push(c.vm().c_object.get()); }
        for k in list {
            let loc = if c.class(k).consts.contains_key(&sym) { Some(c.class(k).const_loc_of(sym)) }
                else if c.class(k).autoload.contains_key(&sym) {
                    match autoload_state(c, k, sym) {
                        Autoload::Own(Some(_), l) => Some(l),
                        Autoload::Ready(_) | Autoload::Own(None, _) => Some(c.class(k).const_loc_of(sym)),
                        _ => None,
                    }
                } else { None };
            if let Some(loc) = loc {
                return Ok(match loc {
                    Some((f, l)) => { let fs = c.str_new(f.as_bytes()); c.ary_new_from(&[fs, Value::fix(l as i64)]) }
                    None => c.ary_new(),
                });
            }
        }
        return Ok(NIL);
    }
    Ok(NIL)
}
fn m_const_set(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let n = to_sym_arg(c, a[0])?;
    let b = sym_bytes(n);
    if !is_const_name(b) {
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("wrong constant name {}", sym_str(n)), Value::sym(n), slf)?;
        return Err(raise_exc(c, e));
    }
    check_frozen(c, slf)?;
    if crate::builtins::ractor::multi() { crate::builtins::ractor::cdecl_check(c, a[1])?; }
    const_set_here(c, slf, n, a[1])?;
    Ok(a[1])
}
fn m_constants(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let inherit = a.first().map(|v| v.truthy()).unwrap_or(true);
    let out = c.ary_new();
    let mut seen = std::collections::HashSet::new();
    let list: Vec<Value> = if inherit { ancestors(c, slf).to_vec() } else { vec![slf] };
    for k in list {
        if inherit && k == c.vm().c_object.get() && slf != c.vm().c_object.get() { break; }
        for &n in c.class(k).const_order.clone().iter() {
            if c.class(k).consts.contains_key(&n) && !c.class(k).private_consts.contains(&n) && is_const_name(sym_bytes(n)) && seen.insert(n) { c.ary_push(out, Value::sym(n)); }
        }
        for (&n, _) in c.class(k).autoload.clone().iter() { if is_const_name(sym_bytes(n)) && seen.insert(n) { c.ary_push(out, Value::sym(n)); } }
    }
    Ok(out)
}
/// Module.constants: with no argument, the constants of the caller's lexical scopes and Object's.
fn m_s_constants(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if !a.is_empty() || slf != c.vm().c_module.get() { return m_constants(c, slf, a, b); }
    let mut scopes = vec![];
    let mut cr = c.caller_frame().map_or(std::ptr::null(), |f| f.cref());
    while let Some(r) = c.cref_ref(cr) { if !r.prev.is_null() { scopes.push(r.klass); } cr = r.prev; }
    let out = c.ary_new();
    let o = c.vm().c_object.get();
    for k in scopes {
        let l = m_constants(c, k, &[FALSE], Blk::NONE)?;
        for v in c.ary(l).to_vec() { if !c.ary(out).contains(&v) { c.ary_push(out, v); } }
    }
    let l = m_constants(c, o, &[], Blk::NONE)?;
    for v in c.ary(l).to_vec() { if !c.ary(out).contains(&v) { c.ary_push(out, v); } }
    Ok(out)
}
fn m_remove_const(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let n = to_sym_arg(c, a[0])?;
    let removed = match c.class_mut(slf).consts.remove(&n) {
        Some(v) => Some(v),
        None => c.class_mut(slf).autoload.remove(&n).map(|_| NIL),   // an autoload: nil
    };
    if removed.is_some() && c.class(slf).is_deprecated(n) { warn_deprecated_const(c, slf, n)?; }
    match removed {
        Some(v) => {
            let cd = c.class_mut(slf);
            if let Some(x) = cd.cx.as_mut() { x.deprecated.retain(|y| *y != n); x.const_loc.remove(&n); }
            cd.const_order.retain(|x| *x != n); cd.private_consts.retain(|x| *x != n);
            c.vm().const_serial.set(c.vm().const_serial.get() + 1);
            Ok(v)
        }
        None => {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("constant {}::{} not defined", class_path(c, slf), sym_str(n)), Value::sym(n), slf)?;
            Err(raise_exc(c, e))
        }
    }
}
fn m_private_constant(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { set_const_visi(c, slf, a, true) }
fn m_public_constant(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { set_const_visi(c, slf, a, false) }
fn set_const_visi(c: &mut Ctx, slf: Value, a: &[Value], private: bool) -> RV {
    for &x in a {
        let n = to_sym_arg(c, x)?;
        if !c.class(slf).consts.contains_key(&n) && !c.class(slf).autoload.contains_key(&n) {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("constant {}::{} not defined", class_path(c, slf), sym_str(n)), Value::sym(n), slf)?;
            return Err(raise_exc(c, e));
        }
        if !private { c.class_mut(slf).private_consts.retain(|x| *x != n); }
        else if !c.class(slf).private_consts.contains(&n) { c.class_mut(slf).private_consts.push(n); }
        CONST_VIS.store(true, std::sync::atomic::Ordering::Relaxed);
        c.vm().const_serial.set(c.vm().const_serial.get() + 1);
    }
    Ok(NIL)
}
pub fn private_ref_error(c: &mut Ctx, n: Sym) -> Result<Option<Esc>, Esc> {
    let Some((k, pn)) = PRIVATE_REF.take() else { return Ok(None) };
    if pn != n { return Ok(None); }
    let e = make_name_error(c, c.vm().c_name_error.get(), &format!("private constant {}::{} referenced", class_path(c, k), sym_str(n)), Value::sym(n), k)?;
    Ok(Some(raise_exc(c, e)))
}
/// name_err_mesg_to_str: a module is shown by its #name, else its #inspect (errors fall back to the path).
pub fn mesg_receiver_name(c: &mut Ctx, k: Value) -> String {
    let saved = (c.esc, c.esc_val);
    for m in [intern(b"name"), s::inspect] {
        match funcall(c, k, m, &[], std::ptr::null()) {
            Ok(v) if is_string(v) => return String::from_utf8_lossy(c.str_bytes(v)).into_owned(),
            Ok(_) => {}
            Err(_) => { c.esc = saved.0; c.esc_val = saved.1; }
        }
    }
    class_path(c, k)
}
fn m_const_missing(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    // a private constant referenced with a scope comes here first (rb_const_missing after private_const_reference)
    if a[0].is_sym() { if let Some(e) = private_ref_error(c, a[0].as_sym())? { return Err(e); } }
    // the name may be any object (rb_mod_const_missing formats it with %s)
    let ns = if a[0].is_sym() { sym_str(a[0].as_sym()).to_string() } else { to_rstring(c, a[0])? };
    let msg = if slf == c.vm().c_object.get() { format!("uninitialized constant {}", ns) } else { format!("uninitialized constant {}::{}", mesg_receiver_name(c, slf), ns) };
    let e = make_name_error(c, c.vm().c_name_error.get(), &msg, a[0], slf)?;
    Err(raise_exc(c, e))
}
/// rb_is_class_name: NameError naming the argument as given (a String stays a String).
pub fn check_cvar_name(c: &mut Ctx, n: Sym, slf: Value, orig: Value) -> Result<(), Esc> {
    let nb = sym_bytes(n);
    if nb.len() > 2 && nb.starts_with(b"@@") && !nb[2].is_ascii_digit() && nb[2..].iter().all(|x| x.is_ascii_alphanumeric() || *x == b'_' || *x >= 0x80) { return Ok(()); }
    let e = make_name_error(c, c.vm().c_name_error.get(), &format!("'{}' is not allowed as a class variable name", sym_str(n)), orig, slf)?;
    Err(raise_exc(c, e))
}
fn m_cvar_get(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let n = to_sym_arg(c, a[0])?;
    check_cvar_name(c, n, slf, a[0])?;
    if crate::builtins::ractor::multi() { crate::builtins::ractor::cvar_check(c, slf, n)?; }
    match crate::rt::cvar_lookup_checked(c, slf, n)? {
        Some((_, v)) => Ok(v),
        None => {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("uninitialized class variable {} in {}", sym_str(n), class_path(c, slf)), Value::sym(n), slf)?;
            Err(raise_exc(c, e))
        }
    }
}
fn m_cvar_set(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let n = to_sym_arg(c, a[0])?;
    let nb = sym_bytes(n);
    if !(nb.len() > 2 && nb.starts_with(b"@@") && !nb[2].is_ascii_digit() && nb[2..].iter().all(|x| x.is_ascii_alphanumeric() || *x == b'_' || *x >= 0x80)) {
        let e = make_name_error(c, c.vm().c_name_error.get(), &format!("'{}' is not allowed as a class variable name", sym_str(n)), a[0], slf)?;
        return Err(raise_exc(c, e));
    }
    check_frozen(c, slf)?;
    if crate::builtins::ractor::multi() { crate::builtins::ractor::cvar_check(c, slf, n)?; }
    match crate::rt::cvar_lookup_checked(c, slf, n)? { Some((o, _)) => { c.class_mut(o).cvars.insert(n, a[1]); } None => { c.class_mut(slf).cvars.insert(n, a[1]); } }
    Ok(a[1])
}
fn m_cvar_defined(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let n = to_sym_arg(c, a[0])?;
    check_cvar_name(c, n, slf, a[0])?;
    Ok(Value::bool(crate::rt::cvar_lookup(c, slf, n).is_some()))
}
fn m_cvars(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let inherit = a.first().is_none_or(|v| v.truthy());
    let out = c.ary_new();
    let anc = if inherit { ancestors(c, slf).to_vec() } else { vec![slf] };
    for &k in anc.iter() {
        let names: Vec<Sym> = c.class(k).cvars.keys().copied().collect();
        for n in names { if !c.ary(out).contains(&Value::sym(n)) { c.ary_push(out, Value::sym(n)); } }
    }
    Ok(out)
}
fn m_freeze(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV { k_freeze(c, slf, a, b) }
/// Module#initialize: the block runs as module_eval.
fn m_initialize(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    // checked in its frame: the ArgumentError is Module#initialize's
    check_argc(c, a, 0, 0)?;
    if !b.is_null() { yield_with_self(c, b, &[slf], slf, slf)?; }
    Ok(NIL)
}
fn m_singleton_class_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(c.class(slf).is_singleton)) }
fn refinements_of(c: &mut Ctx, m: Value) -> Value {
    let key = intern(b"__refinements");
    let l = ivar_get(c, m, key);
    if l != NIL { return l; }
    let l = c.ary_new();
    ivar_set_raw(c, m, key, l);
    l
}
fn m_refine(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
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
            let cd = c.class_mut(r);
            cd.refined = true; cd.attached = k;
            ivar_set_raw(c, r, intern(b"__defined_at__"), slf);
            c.ary_push(list, r);
            // refine blocks run before see it too (a module's refinements are activated together)
            let act = ivar_get(c, slf, intern(b"__activated"));
            if act != NIL { for a in c.ary(act).to_vec() { if !c.ary(a).contains(&r) { c.ary_push(a, r); } } }
            r
        }
    };
    // the body sees its module's refinements, this one included
    let Some(mut blk) = c.block_copy(b) else { return arg_err(c, "no block given") };
    let outer = cref_refinements(blk.cref);
    let mut v = if outer != NIL { c.ary(outer).to_vec() } else { vec![] };
    for &x in c.ary(list) { if !v.contains(&x) { v.push(x); } }
    let active = c.ary_from_vec(v);
    let act = match ivar_get(c, slf, intern(b"__activated")) { NIL => { let a = c.ary_new(); ivar_set_raw(c, slf, intern(b"__activated"), a); a } a => a };
    c.ary_push(act, active);
    let cr = new_cref(r, blk.cref);
    c.cref_set_refinements(cr, active);
    c.set_refinements_on();
    blk.cref = cr;
    yield_with_self(c, &blk, &[r], r, r)?;
    Ok(r)
}
/// rb_top_main_class: the wrapper module in a wrapped load, else Object.
fn top_main_class(c: &mut Ctx, method: &str) -> RV {
    let w = crate::builtins::kernel::top_wrapper();
    if w == NIL { return Ok(c.vm().c_object.get()); }
    if c.vm().verbose.get() == TRUE { crate::builtins::kernel::gvar_warn(c, &format!("main.{} in the wrapped load is effective only in wrapper module", method), false)?; }
    Ok(w)
}
/// main.using: only at the top level (of a wrapped load too).
fn top_using(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let cr = caller_cref(c);
    let next = c.cref_ref(cr).map_or(std::ptr::null(), |x| x.prev);
    let next = if crate::builtins::kernel::top_wrapper() != NIL { c.cref_ref(next).map_or(std::ptr::null(), |x| x.prev) } else { next };
    if !next.is_null() || c.caller_frame().is_some_and(|f| f.kind == FK_METHOD) {
        return err(c, c.vm().c_runtime_error.get(), "main.using is permitted only at toplevel");
    }
    m_using(c, slf, a, b)
}
/// `using mod`: the caller's cref is replaced by a copy that has mod's refinements activated.
fn m_using(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
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
/// Module.used_refinements: the refinements active where it is called.
fn m_s_used_refinements(c: &mut Ctx, _slf: Value, _a: &[Value], _b: Blk) -> RV {
    let r = cref_refinements(caller_cref(c));
    let mut v: Vec<Value> = vec![];
    if r != NIL { for &x in c.ary(r).iter() { if !v.contains(&x) { v.push(x); } } }
    Ok(c.ary_from_vec(v))
}
fn m_refinements(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let l = ivar_get(c, slf, intern(b"__refinements"));
    Ok(if l == NIL { c.ary_new() } else { c.ary_from_vec(c.ary(l).to_vec()) })
}
fn refinement_target(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.class(slf).attached) }
/// Refinement#import_methods: copies of the modules' Ruby-defined methods.
fn refinement_import_methods(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    for &m in a {
        if !is_class_or_module(m) || !c.class(m).is_module { return type_err(c, format!("wrong argument type {} (expected Module)", class_name_for_err(c, m))); }
        if ancestors(c, m).len() > 1 {
            let msg = format!("{} has ancestors, but Refinement#import_methods doesn't import their methods", class_path(c, m));
            crate::builtins::kernel::gvar_warn(c, &msg, false)?;
        }
    }
    for &m in a {
        let mt: Vec<(Sym, *const MethodEntry)> = c.class(m).mtbl.iter().map(|(a, b)| (*a, *b)).collect();
        for (name, me) in mt {
            let Some(me) = me_copy(c, me) else { continue };
            match me.kind {
                MKind::Iseq(_) | MKind::Bmethod(_) | MKind::AttrReader(_) | MKind::AttrWriter(_) => {
                    // the copies run under the refine block's scope (its refinements are active)
                    let cref = if c.caller_frame().is_none() { me.cref } else { caller_cref(c) };
                    let n = new_me_cref(name, slf, me.visi, me.kind, cref);
                    crate::rt::note_redefinition(c, slf, name);
                    define_user_method(c, slf, name, n);
                }
                _ => return arg_err(c, format!("Can't import method which is not defined with Ruby code: {}#{}", class_path(c, m), sym_str(name))),
            }
        }
    }
    Ok(slf)
}
fn m_attached_object(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if !c.class(slf).is_singleton { return type_err(c, format!("'{}' is not a singleton class", class_path(c, slf))); }
    Ok(c.class(slf).attached)
}

// ---- Class ----------------------------------------------------------------------------------

pub fn alloc_instance(c: &mut Ctx, klass: Value) -> RV {
    let cd = c.class(klass);
    if cd.is_singleton { return type_err(c, "can't create instance of singleton class"); }
    Ok(match cd.alloc {
        AllocKind::Object | AllocKind::Exception => new_object(c, klass),
        AllocKind::String => c.alloc(klass, Body::String(RString { b: crate::sbuf::SBuf::new(), enc: ENC_BINARY })),
        AllocKind::Array => c.alloc(klass, Body::Array(crate::avec::AVec::new())),
        AllocKind::Hash => { let h = c.hash_new(); obj_set_klass(h, klass); h }
        AllocKind::Range => c.alloc(klass, Body::Range(NIL, NIL, false)),
        // rb_class_s_alloc: no superclass until Class#initialize
        AllocKind::Class => new_class(c, NIL),
        AllocKind::Module => { let m = new_module(c); obj_set_klass(m, klass); m }
        AllocKind::Proc | AllocKind::None => {
            if let Some(v) = crate::builtins::alloc_data(c, klass)? { return Ok(v); }
            // a Class#dup / #clone of a data class: its allocator is the original's
            let (mut k, key) = (klass, intern(b"__alloc_of"));
            while k != NIL {
                let o = ivar_get(c, k, key);
                if o != NIL { if let Some(v) = crate::builtins::alloc_data(c, o)? { obj_set_klass(v, klass); return Ok(v); } break; }
                k = c.class(k).superclass;
            }
            return type_err(c, format!("allocator undefined for {}", class_path(c, klass)));
        }
    })
}

fn c_allocate(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { alloc_instance(c, slf) }
pub fn c_new(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    if uninit_class(c, slf) { return type_err(c, "can't instantiate uninitialized class"); }
    let o = alloc_instance(c, slf)?;
    let kw = kw_arg(c, a).is_some();
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
/// A class from Class.allocate (no superclass yet).
fn uninit_class(c: &Ctx, k: Value) -> bool { let cd = c.class(k); cd.superclass == NIL && !cd.is_module && !cd.is_singleton && k != c.vm().c_basic_object.get() }
fn c_superclass(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if uninit_class(c, slf) { return type_err(c, "uninitialized class"); }
    let mut s = c.class(slf).superclass;
    while s != NIL && c.class(s).is_module { s = c.class(s).superclass; }
    Ok(s)
}
fn c_subclasses(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let v: Vec<Value> = c.class(slf).subclasses.iter().rev().copied().filter(|&k| !c.class(k).is_singleton).collect();
    Ok(c.ary_from_vec(v))
}
fn c_inherited(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(NIL) }
/// Class#initialize: every class saruby has is made initialized (Class.new does not call this).
/// rb_class_initialize: an allocated class gets its superclass (Object by default), then inherited and the block.
fn c_initialize(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    if !uninit_class(c, slf) { return type_err(c, "already initialized class"); }
    let sup = if a.is_empty() { c.vm().c_object.get() } else { a[0] };
    if !is_class_or_module(sup) || c.class(sup).is_module { return type_err(c, format!("superclass must be an instance of Class (given an instance of {})", class_name_for_err(c, sup))); }
    if c.class(sup).is_singleton { return type_err(c, "can't make subclass of singleton class"); }
    if sup == c.vm().c_class.get() { return type_err(c, "can't make subclass of Class"); }
    init_class(c, slf, sup);
    call_inherited(c, sup, slf)?;
    if !b.is_null() { yield_with_self(c, b, &[slf], slf, slf)?; }
    Ok(NIL)
}

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
    // Kernel#=~ is gone (4.0); NilClass#=~ is in core.rb
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

    let m = vmr.c_module.get();
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, m, n, f, a);
    d(c, "alias_method", m_alias_method, -1);
    d(c, "undef_method", m_undef_method, -1);
    d(c, "remove_method", m_remove_method, -1);
    d(c, "attr_reader", m_attr_reader, -1);
    d(c, "attr_writer", m_attr_writer, -1);
    d(c, "attr_accessor", m_attr_accessor, -1);
    d(c, "attr", m_attr, -1);
    define_private_cfunc(c, m, "private", m_private, -1);
    define_private_cfunc(c, m, "public", m_public, -1);
    define_private_cfunc(c, m, "protected", m_protected, -1);
    define_private_cfunc(c, m, "module_function", m_module_function, -1);
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
    d(c, "set_temporary_name", m_set_temporary_name, 1);
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
    d(c, "const_source_location", m_const_source_location, -1);
    d(c, "const_defined?", m_const_defined, -1);
    d(c, "constants", m_constants, -1);
    define_private_cfunc(c, m, "remove_const", m_remove_const, 1);
    d(c, "private_constant", m_private_constant, -1);
    d(c, "public_constant", m_public_constant, -1);
    d(c, "const_missing", m_const_missing, 1);
    d(c, "class_variable_get", m_cvar_get, 1);
    d(c, "class_variable_set", m_cvar_set, 2);
    d(c, "class_variable_defined?", m_cvar_defined, 1);
    d(c, "class_variables", m_cvars, -1);
    d(c, "freeze", m_freeze, 0);
    d(c, "==", bo_eq, 1);
    d(c, "singleton_class?", m_singleton_class_p, 0);
    define_private_cfunc(c, m, "refine", m_refine, 1);
    define_private_cfunc(c, m, "using", m_using, 1);
    d(c, "refinements", m_refinements, 0);
    let rf = define_class(c, "Refinement", vmr.c_module.get());
    define_cfunc(c, rf, "target", refinement_target, 0);
    define_private_cfunc(c, rf, "import_methods", refinement_import_methods, -1);
    for n in ["extend_object", "append_features", "prepend_features"] { undef_method(c, rf, intern(n.as_bytes())).expect("Module's method"); }
    define_singleton_cfunc(c, m, "used_refinements", m_s_used_refinements, 0);
    define_cfunc(c, vmr.c_class.get(), "attached_object", m_attached_object, 0);
    define_singleton_cfunc(c, m, "constants", m_s_constants, -1);

    let kc = vmr.c_class.get();
    for n in ["module_function", "extend_object", "append_features", "prepend_features", "refine"] { undef_method(c, kc, intern(n.as_bytes())).expect("Module's method"); }
    define_cfunc(c, kc, "allocate", c_allocate, 0);
    define_cfunc(c, kc, "new", c_new, -1);
    let id_c_new = search_method(c, kc, crate::symbol::intern_str("new"));
    define_cfunc(c, kc, "superclass", c_superclass, 0);
    define_cfunc(c, kc, "subclasses", c_subclasses, 0);
    define_private_cfunc(c, kc, "inherited", c_inherited, 1);
    define_private_cfunc(c, kc, "initialize", c_initialize, -1);
    define_private_cfunc(c, vmr.c_module.get(), "initialize", m_initialize, -1);
    // Class.new / Module.new are Class#new: allocate, then #initialize (rb_class_initialize / rb_mod_initialize)
    define_singleton_cfunc(c, kc, "allocate", c_allocate, 0);

    let main = vmr.main_obj.get();
    define_singleton_cfunc(c, main, "to_s", |c, _s, _a, _b| Ok(c.str_new(b"main")), 0);
    define_singleton_cfunc(c, main, "inspect", |c, _s, _a, _b| Ok(c.str_new(b"main")), 0);
    let sk = singleton_class(c, main);
    define_private_cfunc(c, sk, "private", top_private, -1);
    define_private_cfunc(c, sk, "public", top_public, -1);
    define_private_cfunc(c, sk, "include", |c, _s, a, b| { let k = top_main_class(c, "include")?; m_include(c, k, a, b) }, -1);
    define_private_cfunc(c, sk, "define_method", |c, _s, a, b| m_define_method(c, c.vm().c_object.get(), a, b), -1);
    define_private_cfunc(c, sk, "using", top_using, 1);
    InitIds { bo_init: id_bo_init, bo_eq: id_bo_eq, kernel_hash: id_kernel_hash, m_eqq: id_m_eqq, c_new: id_c_new }
}
