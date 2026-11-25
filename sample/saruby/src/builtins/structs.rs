// Struct (members are hidden ivars named by the member symbol; accessors are attr methods).
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::array::*;
use crate::builtins::kernel::*;
use crate::hash::*;
use crate::string::*;
use crate::symbol::{intern, s, sym_bytes, sym_str};
use crate::value::*;
use crate::vm::*;

fn members_of(c: &mut Ctx, k: Value) -> Value {
    let key = intern(b"__members__");
    let mut kk = k;
    while kk != NIL { if let Some(m) = const_get_at(c, kk, key) { return m; } kk = c.class(kk).superclass; }
    c.ary_new()
}
fn kw_init(c: &Ctx, k: Value) -> Value {
    let key = intern(b"__keyword_init__");
    let mut kk = k;
    while kk != NIL { if let Some(m) = const_get_at(c, kk, key) { return m; } kk = c.class(kk).superclass; }
    NIL
}

fn st_s_new(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let mut names = vec![];
    let mut kwinit = NIL;
    let mut cname = NIL;
    let mut a = a;
    if let Some(h) = kw_arg(c, a) {
        a = &a[..a.len() - 1];
        let mut unknown = vec![];
        let mut i = 0;
        while let Some((key, v)) = c.hash_entry_at(h, i) {
            i += 1;
            if key == UNDEF { continue; }
            if key == sym_value("keyword_init") { kwinit = v; } else { unknown.push(inspect_str(c, key)?); }
        }
        if !unknown.is_empty() { return arg_err(c, format!("unknown keyword{}: {}", if unknown.len() > 1 { "s" } else { "" }, unknown.join(", "))); }
    }
    for (i, &x) in a.iter().enumerate() {
        if i == 0 && !x.is_sym() {
            if x == NIL { continue; }
            let sv = expect_str(c, x)?;
            let b = c.str_bytes(sv);
            if b.is_empty() || !b[0].is_ascii_uppercase() && b[0] < 0x80 {
                let e = make_name_error(c, c.vm().c_name_error.get(), &format!("identifier {} needs to be constant", String::from_utf8_lossy(b)), NIL, slf)?;
                return Err(raise_exc(c, e));
            }
            cname = sv;
            continue;
        }
        let n = if x.is_sym() { x.as_sym() } else if is_string(x) { intern(c.str_bytes(x)) } else {
            let ins = inspect_str(c, x)?;
            return type_err(c, format!("{} is not a symbol nor a string", ins));
        };
        let nb = sym_bytes(n);
        if nb.ends_with(b"=") && !nb.ends_with(b"==") { return arg_err(c, format!("invalid struct member: {}", sym_str(n))); }
        if names.contains(&n) { return arg_err(c, format!("duplicate member: {}", sym_str(n))); }
        names.push(n);
    }
    let k = new_class(c, slf);
    crate::builtins::object::call_inherited(c, slf, k)?;
    let mem = c.ary_from_vec(names.iter().map(|n| Value::sym(*n)).collect());
    obj_set_flags(mem, FL_FROZEN);
    let consts = &mut c.class_mut(k).consts;
    consts.insert(intern(b"__members__"), mem);
    consts.insert(intern(b"__keyword_init__"), kwinit);
    for &n in &names {
        define_method_me(c, k, n, new_me(n, k, VISI_PUBLIC, MKind::AttrReader(n)));
        let w = intern(format!("{}=", sym_str(n)).as_bytes());
        define_method_me(c, k, w, new_me(w, k, VISI_PUBLIC, MKind::AttrWriter(n)));
    }
    let sk = singleton_class(c, k);
    define_cfunc(c, sk, "new", crate::builtins::object::c_new, -1);
    define_cfunc(c, sk, "[]", crate::builtins::object::c_new, -1);
    define_cfunc(c, sk, "members", |c, s2, _a, _b| { let m = members_of(c, s2); Ok(c.ary_from_vec(c.ary(m).to_vec())) }, 0);
    if kwinit.truthy() {
        define_cfunc(c, sk, "inspect", |c, s2, _a, _b| {
            let base = if c.class(s2).name.is_some() { class_path(c, s2) } else { let r = funcall(c, s2, intern(b"to_s"), &[], std::ptr::null())?; String::from_utf8_lossy(c.str_bytes(r)).into_owned() };
            let v = kw_init(c, s2);
            Ok(c.str_new(format!("{}(keyword_init: {})", base, if v.truthy() { "true" } else { "false" }).as_bytes()))
        }, 0);
    }
    define_cfunc(c, sk, "keyword_init?", |c, s2, _a, _b| { let v = kw_init(c, s2); Ok(if v == NIL { NIL } else { Value::bool(v.truthy()) }) }, 0);
    if cname != NIL {
        let n = intern(c.str_bytes(cname));
        // new_struct: rb_warn and remove_const before the new one
        if c.class(slf).consts.contains_key(&n) && c.vm().verbose.get() != NIL {
            let at = source_location(c).map_or(String::new(), |(f, l)| format!("{}:{}: ", f, l));
            let m = format!("{}warning: redefining constant {}::{}\n", at, class_path(c, slf), sym_str(n));
            crate::builtins::kernel::warning_write(c, m.as_bytes())?;
        }
        const_set(c, slf, n, k);
    }
    if !blk.is_null() { yield_with_self(c, blk, &[k], k, k)?; }
    Ok(k)
}
fn st_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let k = real_class_of(c, slf);
    let mem = members_of(c, k);
    let n = c.ary_len(mem);
    let kw = kw_init(c, k);
    for i in 0..n { ivar_set_raw(c, slf, c.ary_get(mem, i).as_sym(), NIL); }
    if (kw.truthy() || (kw == NIL && a.len() == 1 && kw_arg(c, a).is_some())) && !a.is_empty() {
        if a.len() > 1 || !is_hash(a[0]) { return argc_err(c, a.len(), 0, 0); }
        let mut unknown = vec![];
        let mut i = 0;
        // by slot: to_sym_arg may run Ruby code
        while let Some((key, v)) = c.hash_entry_at(a[0], i) {
            i += 1;
            if key == UNDEF { continue; }
            let ks = if key.is_sym() { key.as_sym() } else { crate::builtins::object::to_sym_arg(c, key)? };
            if !c.ary(mem).contains(&Value::sym(ks)) { unknown.push(sym_str(ks)); continue; }
            ivar_set_raw(c, slf, ks, v);
        }
        if !unknown.is_empty() { return arg_err(c, format!("unknown keywords: {}", unknown.join(", "))); }
        return Ok(NIL);
    }
    if a.len() > n { return arg_err(c, "struct size differs"); }
    for (i, &x) in a.iter().enumerate() { ivar_set_raw(c, slf, c.ary_get(mem, i).as_sym(), x); }
    Ok(NIL)
}
fn st_to_a(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let mem = members_of(c, real_class_of(c, slf));
    Ok(c.ary_from_vec((0..c.ary_len(mem)).map(|i| ivar_get(c, slf, c.ary_get(mem, i).as_sym())).collect()))
}
fn st_to_h(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV {
    let mem = members_of(c, real_class_of(c, slf));
    let h = c.hash_new();
    for i in 0..c.ary_len(mem) {
        let k = c.ary_get(mem, i); let v = ivar_get(c, slf, k.as_sym());
        if blk.is_null() { hash_set(c, h, k, v)?; } else {
            let p = yield_block(c, blk, &[k, v])?;
            let Some(pa) = check_array(c, p)? else { return type_err(c, format!("wrong element type {} (expected array)", class_name_for_err(c, p))); };
            if c.ary_len(pa) != 2 { return arg_err(c, format!("element has wrong array length (expected 2, was {})", c.ary_len(pa))); }
            let (k2, v2) = (c.ary_get(pa, 0), c.ary_get(pa, 1));
            hash_aset(c, h, k2, v2)?;
        }
    }
    Ok(h)
}
fn idx_of(c: &mut Ctx, slf: Value, k: Value) -> Result<Sym, Esc> {
    let mem = members_of(c, real_class_of(c, slf));
    if k.is_sym() || is_string(k) {
        let n = if k.is_sym() { k.as_sym() } else { intern(c.str_bytes(k)) };
        if !c.ary(mem).contains(&Value::sym(n)) {
            let e = make_name_error(c, c.vm().c_name_error.get(), &format!("no member '{}' in struct", sym_str(n)), Value::sym(n), slf)?;
            return Err(raise_exc(c, e));
        }
        return Ok(n);
    }
    let i = crate::builtins::numeric_methods::num_to_long(c, k)?;
    let len = c.ary_len(mem) as i64;
    let j = if i < 0 { i + len } else { i };
    if j < 0 || j >= len {
        return bad_offset(c, i, len);
    }
    Ok(c.ary_get(mem, j as usize).as_sym())
}
/// struct.c: the offset is reported through LONG2FIX (wraps at 2**62) and FIX2INT (int range)
#[cold]
fn bad_offset(c: &mut Ctx, i: i64, len: i64) -> Result<Sym, Esc> {
    let f = (i << 1) >> 1;
    if f != f as i32 as i64 { return err(c, c.vm().c_range_error.get(), format!("integer {} too {} to convert to 'int'", f, if f < 0 { "small" } else { "big" })); }
    let msg = if f < 0 { format!("offset {} too small for struct(size:{})", f, len) } else { format!("offset {} too large for struct(size:{})", f, len) };
    err(c, c.vm().c_index_error.get(), msg)
}
fn st_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let n = idx_of(c, slf, a[0])?; Ok(ivar_get(c, slf, n)) }
fn st_aset(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { let n = idx_of(c, slf, a[0])?; check_frozen(c, slf)?; ivar_set_raw(c, slf, n, a[1]); Ok(a[1]) }
fn st_cmp_members(c: &mut Ctx, slf: Value, o: Value, eql: bool) -> RV {
    if slf == o { return Ok(TRUE); }
    if !o.is_heap() || real_class_of(c, o) != real_class_of(c, slf) { return Ok(FALSE); }
    // rb_exec_recursive_paired: a pair met again compares equal
    let tag = intern(if eql { b"__struct_eql" } else { b"__struct_eq" });
    if !crate::legacy::recursion_enter_paired(slf, o, tag) { return Ok(TRUE); }
    let mem = members_of(c, real_class_of(c, slf));
    let r: Result<bool, Esc> = (|| {
        for i in 0..c.ary_len(mem) {
            let n = c.ary_get(mem, i).as_sym();
            let (x, y) = (ivar_get(c, slf, n), ivar_get(c, o, n));
            let same = if eql { crate::hash::key_eql(c, x, y)? } else { crate::builtins::array_methods::rb_eq(c, x, y)? };
            if !same { return Ok(false); }
        }
        Ok(true)
    })();
    crate::legacy::recursion_leave_paired(slf, o, tag);
    Ok(Value::bool(r?))
}
fn st_eq(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { st_cmp_members(c, slf, a[0], false) }
fn st_eql(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { st_cmp_members(c, slf, a[0], true) }
fn st_hash(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let seed = real_class_of(c, slf).raw();
    // a recursive struct hashes by its class alone (rb_exec_recursive_outer)
    let h = crate::hash::hash_outer(c, slf, seed, |c| {
        let vals = st_to_a(c, slf, &[], Blk::NONE)?;
        let mut h = seed;
        let mut i = 0;
        while i < c.ary_len(vals) { let x = c.ary_get(vals, i); h = crate::astrust::hash_merge(h, crate::hash::ruby_hash(c, x)?); i += 1; }
        Ok(h)
    })?;
    Ok(Value::fix((h >> 2) as i64))
}
fn st_members(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let m = members_of(c, real_class_of(c, slf)); Ok(c.ary_from_vec(c.ary(m).to_vec())) }
/// rb_struct_select
fn st_select(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 0, 0)?;
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, intern(b"select"), &[]); }
    let v = st_to_a(c, slf, &[], Blk::NONE)?;
    let out = c.ary_new();
    let mut i = 0;
    while let Some(&x) = c.ary(v).get(i) { if yield1(c, blk, x)?.truthy() { c.ary_push(out, x); } i += 1; }
    Ok(out)
}
fn st_each(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, s::each, &[]); }
    let v = st_to_a(c, slf, &[], Blk::NONE)?;
    let mut i = 0;
    while let Some(&x) = c.ary(v).get(i) { yield1(c, blk, x)?; i += 1; }
    Ok(slf)
}
fn st_each_pair(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for_sized(c, slf, s::each_pair, &[]); }
    let mem = members_of(c, real_class_of(c, slf));
    for i in 0..c.ary_len(mem) { let k = c.ary_get(mem, i); let p = c.ary_new_from(&[k, ivar_get(c, slf, k.as_sym())]); yield1(c, blk, p)?; }
    Ok(slf)
}
fn st_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let k = real_class_of(c, slf);
    let mem = members_of(c, k);
    // rb_str_buf_new2("#<struct ") + appends: the encodings follow the parts
    let out = c.str_new_enc(b"#<struct ", crate::object::ENC_BINARY);
    let lit = |c: &mut Ctx, b: &[u8]| c.rstring_mut(out).b.extend_from_slice(b);
    // rb_struct_inspect: the class path unless anonymous (or under an anonymous module)
    let named = c.class(k).name.is_some() && !class_path(c, k).starts_with('#');
    if named { let p = class_path(c, k); let ps = c.str_new_enc(p.as_bytes(), if p.is_ascii() { crate::object::ENC_ASCII } else { crate::object::ENC_UTF8 }); crate::string::str_append_str(c, out, ps)?; }
    if !rec_enter(c, slf, s::inspect)? { lit(c, b":...>"); return Ok(out); }
    if named { lit(c, b" "); }
    let r = (|| -> Result<(), Esc> {
        for i in 0..c.ary_len(mem) {
            if i > 0 { lit(c, b", "); }
            let mv = c.ary_get(mem, i);
            let n = mv.as_sym();
            let nb = sym_bytes(n);
            // rb_is_local_id || rb_is_const_id: the bare name, else the Symbol's inspect
            let word = |x: u8| x.is_ascii_alphanumeric() || x == b'_' || x >= 0x80;
            let ns = if nb.first().is_some_and(|&x| !x.is_ascii_digit() && word(x)) && nb.iter().all(|&x| word(x)) { crate::builtins::symbol_methods::sym_str_v(c, mv) } else { inspect_value(c, mv)? };
            crate::string::str_append_str(c, out, ns)?;
            lit(c, b"=");
            let vs = inspect_value(c, ivar_get(c, slf, n))?;
            crate::string::str_append_str(c, out, vs)?;
        }
        Ok(())
    })();
    rec_leave(c, slf, s::inspect);
    r?;
    lit(c, b">");
    Ok(out)
}
fn st_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let m = members_of(c, real_class_of(c, slf)); Ok(Value::fix(c.ary_len(m) as i64)) }
fn st_dig(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    if a.is_empty() { return argc_err(c, 0, 1, -1); }
    let mem = members_of(c, real_class_of(c, slf));
    let v = if a[0].is_sym() || is_string(a[0]) {
        let n = if a[0].is_sym() { a[0].as_sym() } else { intern(c.str_bytes(a[0])) };
        if !c.ary(mem).contains(&Value::sym(n)) { NIL } else { ivar_get(c, slf, n) }
    } else {
        let i = crate::builtins::numeric_methods::num_to_long(c, a[0])?;
        let len = c.ary_len(mem) as i64;
        let j = if i < 0 { i + len } else { i };
        if j < 0 || j >= len { NIL } else { ivar_get(c, slf, c.ary_get(mem, j as usize).as_sym()) }
    };
    if a.len() == 1 || v == NIL { return Ok(v); }
    if !respond_to(c, v, intern(b"dig"), true) { return type_err(c, format!("{} does not have #dig method", class_name_for_err(c, v))); }
    funcall(c, v, intern(b"dig"), &a[1..], std::ptr::null())
}
fn st_values_at(c: &mut Ctx, slf: Value, a: &[Value], b: Blk) -> RV {
    let arr = st_to_a(c, slf, &[], b)?;
    let len = c.ary_len(arr) as i64;
    for &x in a {
        if is_range(x) { continue; }
        let i = crate::builtins::numeric_methods::num_to_long(c, x)?;
        if i < -len || i >= len { return bad_offset(c, i, len).map(|_| NIL); }
    }
    crate::builtins::array_methods::a_values_at(c, arr, a, b)
}
fn st_deconstruct_keys(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let h = c.hash_new();
    let mem = members_of(c, real_class_of(c, slf));
    if a[0] == NIL {
        for i in 0..c.ary_len(mem) { let k = c.ary_get(mem, i); let v = ivar_get(c, slf, k.as_sym()); hash_set(c, h, k, v)?; }
        return Ok(h);
    }
    if !is_array(a[0]) { return type_err(c, format!("wrong argument type {} (expected Array or nil)", class_name_for_err(c, a[0]))); }
    let keys = a[0];
    let len = c.ary_len(mem) as i64;
    if c.ary_len(keys) as i64 > len { return Ok(h); }
    let mut i = 0;
    while i < c.ary_len(keys) {
        let k = c.ary_get(keys, i);
        let n = if k.is_sym() || is_string(k) {
            let n = if k.is_sym() { k.as_sym() } else { intern(c.str_bytes(k)) };
            if !c.ary(mem).contains(&Value::sym(n)) { break; }
            n
        } else {
            // NUM2LEN: anything else through to_int
            let j = crate::builtins::numeric_methods::num_to_long(c, k)?;
            let j = if j < 0 { j + len } else { j };
            if j < 0 || j >= len { break; }
            c.ary_get(mem, j as usize).as_sym()
        };
        let v = ivar_get(c, slf, n);
        hash_aset(c, h, k, v)?;
        i += 1;
    }
    Ok(h)
}
fn st_init_copy(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    for (n, v) in ivar_list(c, a[0]) { ivar_set_raw(c, slf, n, v); }
    Ok(slf)
}

/// Defines Struct (under `object`, including `enumerable`) and returns it.
pub fn init(c: &mut Ctx, object: Value, enumerable: Value) -> Value {
    let st = define_class(c, "Struct", object);
    include_module(c, st, enumerable);
    define_singleton_cfunc(c, st, "new", st_s_new, -1);
    define_private_cfunc(c, st, "initialize", st_initialize, -1);
    define_private_cfunc(c, st, "initialize_copy", st_init_copy, 1);
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, st, n, f, a);
    d(c, "select", st_select, -1); d(c, "filter", st_select, -1);
    d(c, "to_a", st_to_a, 0); d(c, "deconstruct", st_to_a, 0); d(c, "values", st_to_a, 0); d(c, "to_h", st_to_h, 0); d(c, "[]", st_aref, 1); d(c, "[]=", st_aset, 2);
    d(c, "==", st_eq, 1); d(c, "eql?", st_eql, 1); d(c, "hash", st_hash, 0); d(c, "members", st_members, 0); d(c, "each", st_each, 0); d(c, "each_pair", st_each_pair, 0);
    d(c, "inspect", st_inspect, 0); crate::builtins::object::alias_builtin(c, st, "to_s", "inspect"); d(c, "size", st_size, 0); d(c, "length", st_size, 0); d(c, "dig", st_dig, -1);
    d(c, "values_at", st_values_at, -1); d(c, "deconstruct_keys", st_deconstruct_keys, 1);
    st
}
