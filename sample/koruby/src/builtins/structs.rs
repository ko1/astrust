// Struct (members are hidden ivars named by the member symbol; accessors are attr methods).
use crate::array::*;
use crate::builtins::kernel::*;
use crate::hash::*;
use crate::string::*;
use crate::symbol::{intern, s, sym_bytes, sym_str};
use crate::value::*;
use crate::vm::*;

fn members_of(k: Value) -> Value {
    let key = intern(b"__members__");
    let mut kk = k;
    while kk != NIL { if let Some(m) = const_get_at(kk, key) { return m; } kk = rclass(kk).superclass; }
    ary_new()
}
fn kw_init(k: Value) -> Value {
    let key = intern(b"__keyword_init__");
    let mut kk = k;
    while kk != NIL { if let Some(m) = const_get_at(kk, key) { return m; } kk = rclass(kk).superclass; }
    NIL
}

fn st_s_new(c: &mut Ctx, slf: Value, a: &[Value], blk: *const Block) -> RV {
    if slf != vm().c_struct { return crate::builtins::object::c_new(c, slf, a, blk); }
    let mut names = vec![];
    let mut kwinit = NIL;
    let mut cname = NIL;
    for (i, &x) in a.iter().enumerate() {
        if is_hash(x) && i == a.len() - 1 { if let Some(v) = hash_get_opt(c, x, sym_value("keyword_init"))? { kwinit = v; } continue; }
        if i == 0 && is_string(x) { cname = x; continue; }
        let n = crate::builtins::object::to_sym_arg(c, x)?;
        if names.contains(&n) { return arg_err(c, format!("duplicate member: {}", sym_str(n))); }
        names.push(n);
    }
    let k = new_class(slf);
    let mem = ary_from_vec(names.iter().map(|n| Value::sym(*n)).collect());
    obj(mem).flags |= FL_FROZEN;
    rclass(k).consts.insert(intern(b"__members__"), mem);
    rclass(k).consts.insert(intern(b"__keyword_init__"), kwinit);
    for &n in &names {
        define_method_me(k, n, new_me(n, k, VISI_PUBLIC, MKind::AttrReader(n)));
        let w = intern(format!("{}=", sym_str(n)).as_bytes());
        define_method_me(k, w, new_me(w, k, VISI_PUBLIC, MKind::AttrWriter(n)));
    }
    let sk = singleton_class(k);
    define_cfunc(sk, "new", crate::builtins::object::c_new, -1);
    define_cfunc(sk, "[]", crate::builtins::object::c_new, -1);
    define_cfunc(sk, "members", |_c, s2, _a, _b| { let m = members_of(s2); Ok(ary_from_vec(rary(m).clone())) }, 0);
    define_cfunc(sk, "keyword_init?", |_c, s2, _a, _b| { let v = kw_init(s2); Ok(if v == NIL { NIL } else { Value::bool(v.truthy()) }) }, 0);
    if cname != NIL { let n = intern(str_bytes(cname)); const_set(slf, n, k); }
    if !blk.is_null() { yield_with_self(c, blk, &[k], k, k)?; }
    Ok(k)
}
fn st_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let k = real_class_of(slf);
    let mem = members_of(k);
    let n = ary_len(mem);
    let kw = kw_init(k);
    for i in 0..n { ivar_set_raw(slf, ary_get(mem, i).as_sym(), NIL); }
    if (kw.truthy() || (kw == NIL && a.len() == 1 && kw_arg(a).is_some())) && !a.is_empty() {
        if a.len() > 1 || !is_hash(a[0]) { return argc_err(c, a.len(), 0, 0); }
        let mut unknown = vec![];
        for (key, v) in hash_pairs(a[0]) {
            let ks = if key.is_sym() { key.as_sym() } else { crate::builtins::object::to_sym_arg(c, key)? };
            if !rary(mem).contains(&Value::sym(ks)) { unknown.push(sym_str(ks)); continue; }
            ivar_set_raw(slf, ks, v);
        }
        if !unknown.is_empty() { return arg_err(c, format!("unknown keywords: {}", unknown.join(", "))); }
        return Ok(NIL);
    }
    if a.len() > n { return arg_err(c, "struct size differs"); }
    for (i, &x) in a.iter().enumerate() { ivar_set_raw(slf, ary_get(mem, i).as_sym(), x); }
    Ok(NIL)
}
fn st_to_a(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let mem = members_of(real_class_of(slf));
    Ok(ary_from_vec((0..ary_len(mem)).map(|i| ivar_get(slf, ary_get(mem, i).as_sym())).collect()))
}
fn st_to_h(c: &mut Ctx, slf: Value, _a: &[Value], blk: *const Block) -> RV {
    let mem = members_of(real_class_of(slf));
    let h = hash_new();
    for i in 0..ary_len(mem) {
        let k = ary_get(mem, i); let v = ivar_get(slf, k.as_sym());
        if blk.is_null() { hash_set(c, h, k, v)?; } else { let p = yield_block(c, blk, &[k, v])?; hash_aset(c, h, ary_get(p, 0), ary_get(p, 1))?; }
    }
    Ok(h)
}
fn idx_of(c: &mut Ctx, slf: Value, k: Value) -> Result<Sym, Esc> {
    let mem = members_of(real_class_of(slf));
    if k.is_sym() || is_string(k) {
        let n = if k.is_sym() { k.as_sym() } else { intern(str_bytes(k)) };
        if !rary(mem).contains(&Value::sym(n)) {
            let e = make_name_error(c, vm().c_name_error, &format!("no member '{}' in struct", sym_str(n)), Value::sym(n), slf)?;
            return Err(raise_exc(c, e));
        }
        return Ok(n);
    }
    let i = crate::builtins::numeric_methods::num_to_long(c, k)?;
    let len = ary_len(mem) as i64;
    let j = if i < 0 { i + len } else { i };
    if j < 0 || j >= len {
        let msg = if i < 0 { format!("offset {} too small for struct(size:{})", i, len) } else { format!("offset {} too large for struct(size:{})", i, len) };
        return err(c, vm().c_index_error, msg);
    }
    Ok(ary_get(mem, j as usize).as_sym())
}
fn st_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let n = idx_of(c, slf, a[0])?; Ok(ivar_get(slf, n)) }
fn st_aset(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let n = idx_of(c, slf, a[0])?; check_frozen(c, slf)?; ivar_set_raw(slf, n, a[1]); Ok(a[1]) }
fn st_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if slf == a[0] { return Ok(TRUE); }
    if real_class_of(a[0]) != real_class_of(slf) { return Ok(FALSE); }
    let x = st_to_a(c, slf, &[], std::ptr::null())?; let y = st_to_a(c, a[0], &[], std::ptr::null())?;
    funcall(c, x, s::eq, &[y], std::ptr::null())
}
fn st_eql(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if real_class_of(a[0]) != real_class_of(slf) { return Ok(FALSE); }
    let x = st_to_a(c, slf, &[], std::ptr::null())?; let y = st_to_a(c, a[0], &[], std::ptr::null())?;
    funcall(c, x, s::eql_p, &[y], std::ptr::null())
}
fn st_hash(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let x = st_to_a(c, slf, &[], std::ptr::null())?; let h = key_hash(c, x)?; Ok(Value::fix(((h ^ real_class_of(slf).raw()) >> 2) as i64)) }
fn st_members(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ary_from_vec(rary(members_of(real_class_of(slf))).clone())) }
fn st_each(c: &mut Ctx, slf: Value, _a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, s::each, &[]); }
    let v = st_to_a(c, slf, &[], std::ptr::null())?;
    for i in 0..ary_len(v) { yield1(c, blk, ary_get(v, i))?; }
    Ok(slf)
}
fn st_each_pair(c: &mut Ctx, slf: Value, _a: &[Value], blk: *const Block) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, s::each_pair, &[]); }
    let mem = members_of(real_class_of(slf));
    for i in 0..ary_len(mem) { let k = ary_get(mem, i); let p = ary_new_from(&[k, ivar_get(slf, k.as_sym())]); yield1(c, blk, p)?; }
    Ok(slf)
}
fn st_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let k = real_class_of(slf);
    let mem = members_of(k);
    let mut out = String::from("#<struct ");
    if rclass(k).name.is_some() { out.push_str(&class_path(k)); out.push(' '); }
    if !recursion_enter(slf, s::inspect) { out.push_str(":...>"); return Ok(str_new(out.as_bytes())); }
    for i in 0..ary_len(mem) {
        if i > 0 { out.push_str(", "); }
        let n = ary_get(mem, i).as_sym();
        let nb = sym_bytes(n);
        if crate::string::sym_is_simple(nb) && !nb.ends_with(b"=") { out.push_str(&sym_str(n)); } else { out.push_str(&crate::string::inspect_bytes(nb, ENC_UTF8)); }
        out.push('=');
        match inspect_str(c, ivar_get(slf, n)) { Ok(s2) => out.push_str(&s2), Err(e) => { recursion_leave(slf, s::inspect); return Err(e); } }
    }
    recursion_leave(slf, s::inspect);
    out.push('>');
    Ok(str_new(out.as_bytes()))
}
fn st_size(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(ary_len(members_of(real_class_of(slf))) as i64)) }
fn st_dig(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let v = if a[0].is_fixnum() { let a2 = st_to_a(c, slf, &[], std::ptr::null())?; ary_get(a2, a[0].as_fix() as usize) } else { let mem = members_of(real_class_of(slf)); let n = if a[0].is_sym() { a[0].as_sym() } else { intern(str_bytes(a[0])) }; if !rary(mem).contains(&Value::sym(n)) { NIL } else { ivar_get(slf, n) } };
    if a.len() == 1 || v == NIL { return Ok(v); }
    funcall(c, v, intern(b"dig"), &a[1..], std::ptr::null())
}
fn st_values_at(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { let arr = st_to_a(c, slf, &[], b)?; crate::builtins::array_methods::a_values_at(c, arr, a, b) }
fn st_deconstruct_keys(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let h = hash_new();
    let mem = members_of(real_class_of(slf));
    let want: Option<Vec<Value>> = if a[0] == NIL { None } else { Some(rary(a[0]).clone()) };
    for i in 0..ary_len(mem) {
        let k = ary_get(mem, i);
        if let Some(w) = &want { if !w.contains(&k) { continue; } }
        hash_set(c, h, k, ivar_get(slf, k.as_sym()))?;
    }
    Ok(h)
}
fn st_init_copy(_c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    for (n, v) in ivar_list(a[0]) { ivar_set_raw(slf, n, v); }
    Ok(slf)
}

pub fn init() {
    let vm = vm();
    let st = define_class("Struct", vm.c_object);
    include_module(st, vm.m_enumerable);
    vm.c_struct = st;
    define_singleton_cfunc(st, "new", st_s_new, -1);
    define_private_cfunc(st, "initialize", st_initialize, -1);
    define_private_cfunc(st, "initialize_copy", st_init_copy, 1);
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(st, n, f, a);
    d("to_a", st_to_a, 0); d("deconstruct", st_to_a, 0); d("values", st_to_a, 0); d("to_h", st_to_h, 0); d("[]", st_aref, 1); d("[]=", st_aset, 2);
    d("==", st_eq, 1); d("eql?", st_eql, 1); d("hash", st_hash, 0); d("members", st_members, 0); d("each", st_each, 0); d("each_pair", st_each_pair, 0);
    d("inspect", st_inspect, 0); d("to_s", st_inspect, 0); d("size", st_size, 0); d("length", st_size, 0); d("dig", st_dig, -1);
    d("values_at", st_values_at, -1); d("deconstruct_keys", st_deconstruct_keys, 1);
}
