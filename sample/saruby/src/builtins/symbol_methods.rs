// Symbol.
#![forbid(unsafe_code)]
use crate::array::*;
use crate::builtins::kernel::*;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::string::*;
use crate::symbol::sym_bytes;
use crate::value::*;
use crate::vm::*;

/// The encoding of a symbol's String: its own (crate::symbol::intern_enc), else US-ASCII or UTF-8.
pub fn sym_encoding(s: crate::value::Sym) -> u8 {
    if let Some(e) = crate::symbol::sym_enc(s) { return e; }
    if is_ascii(sym_bytes(s)) { ENC_ASCII } else { ENC_UTF8 }
}
pub fn sym_str_v(c: &mut Ctx, v: Value) -> Value { let s = v.as_sym(); c.str_new_enc(sym_bytes(s), sym_encoding(s)) }

fn sym_to_s(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { let k = s.as_sym(); Ok(c.str_new_chilled(sym_bytes(k), sym_encoding(k), crate::object::FL_CHILLED_SYM)) }
fn sym_name(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let k = s.as_sym();
    if let Some(&v) = c.vm().sym_names.borrow().get(&k) { return Ok(v); }
    let v = sym_str_v(c, s); obj_set_flags(v, FL_FROZEN);
    c.vm().sym_names.borrow_mut().insert(k, v);
    Ok(v)
}
fn sym_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV {
    let e = crate::symbol::sym_enc(s.as_sym()).unwrap_or(ENC_UTF8);
    let (r, re) = crate::string::sym_inspect(sym_bytes(s.as_sym()), e);
    Ok(c.str_new_enc(&r, re))
}
fn sym_to_proc(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { crate::builtins::proc::sym_to_proc(c, s) }
fn sym_eq(_c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV { Ok(Value::bool(s == a[0])) }
fn sym_to_sym(_c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(s) }
fn sym_length(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV {
    if crate::symbol::sym_enc(s.as_sym()).is_some() { return via_str(c, s, crate::builtins::string_methods::s_length, a, b); }
    Ok(Value::fix(utf8_len(sym_bytes(s.as_sym())) as i64))
}
fn sym_cmp(_c: &mut Ctx, s: Value, a: &[Value], _b: Blk) -> RV {
    if !a[0].is_sym() { return Ok(NIL); }
    Ok(Value::fix(sym_bytes(s.as_sym()).cmp(sym_bytes(a[0].as_sym())) as i64))
}
// String's builtin, not a dispatch: Symbol#upcase must not see a user's String#upcase (CRuby)
fn via_str(c: &mut Ctx, s: Value, f: crate::vm::CFunc, a: &[Value], b: Blk) -> RV {
    let st = sym_str_v(c, s);
    f(c, st, a, b)
}
fn sym_upcase(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { let r = via_str(c, s, crate::builtins::string_methods::s_upcase, a, b)?; Ok(c.str_to_sym(r)) }
fn sym_downcase(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { let r = via_str(c, s, crate::builtins::string_methods::s_downcase, a, b)?; Ok(c.str_to_sym(r)) }
fn sym_capitalize(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { let r = via_str(c, s, crate::builtins::string_methods::s_capitalize, a, b)?; Ok(c.str_to_sym(r)) }
fn sym_swapcase(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { let r = via_str(c, s, crate::builtins::string_methods::s_swapcase, a, b)?; Ok(c.str_to_sym(r)) }
fn sym_succ(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { let r = via_str(c, s, crate::builtins::string_methods::s_succ, a, b)?; Ok(c.str_to_sym(r)) }
fn sym_aref(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { via_str(c, s, crate::builtins::string_methods::s_aref, a, b) }
fn sym_start_with(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { via_str(c, s, crate::builtins::string_methods::s_start_with, a, b) }
fn sym_end_with(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { via_str(c, s, crate::builtins::string_methods::s_end_with, a, b) }
fn sym_match(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { via_str(c, s, crate::builtins::string_methods::s_match_op, a, b) }
fn sym_match_m(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { via_str(c, s, crate::builtins::string_methods::s_match, a, b) }
fn sym_match_p(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { check_argc(c, a, 1, 2)?; via_str(c, s, crate::builtins::string_methods::s_match_p, a, b) }
fn sym_empty(_c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(sym_bytes(s.as_sym()).is_empty())) }
fn sym_casecmp(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { if !a[0].is_sym() { return Ok(NIL); } let o = sym_str_v(c, a[0]); via_str(c, s, crate::builtins::string_methods::s_casecmp, &[o], b) }
fn sym_casecmp_p(c: &mut Ctx, s: Value, a: &[Value], b: Blk) -> RV { if !a[0].is_sym() { return Ok(NIL); } let o = sym_str_v(c, a[0]); via_str(c, s, crate::builtins::string_methods::s_casecmp_p, &[o], b) }
fn sym_all(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let a = c.ary_new();
    for i in 1..crate::symbol::all_count() { c.ary_push(a, Value::sym(Sym(i as u32))); }
    Ok(a)
}
fn sym_encoding_m(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { crate::builtins::misc::encoding_value(c, sym_encoding(s.as_sym())) }

// nil / true / false #to_s: one frozen US-ASCII String each (rb_cNilClass_to_s ...)
static NIL_S: crate::heap::Global = crate::heap::Global::new();
static TRUE_S: crate::heap::Global = crate::heap::Global::new();
static FALSE_S: crate::heap::Global = crate::heap::Global::new();
fn special_to_s(c: &mut Ctx, s: Value, _a: &[Value], _b: Blk) -> RV { Ok(if s == NIL { NIL_S.get(c) } else if s == TRUE { TRUE_S.get(c) } else { FALSE_S.get(c) }) }
fn nil_inspect(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.str_new_enc(b"nil", ENC_ASCII)) }
pub fn init(c: &mut Ctx, k: Value) {
    for (g, t) in [(&NIL_S, &b""[..]), (&TRUE_S, b"true"), (&FALSE_S, b"false")] { let v = c.str_new_enc(t, ENC_ASCII); obj_set_flags(v, crate::object::FL_FROZEN); g.set(c, v); }
    for kk in [c.vm().c_nil.get(), c.vm().c_true.get(), c.vm().c_false.get()] { define_cfunc(c, kk, "to_s", special_to_s, 0); }
    define_cfunc(c, c.vm().c_nil.get(), "inspect", nil_inspect, 0);
    // rb_undef_method(CLASS_OF(rb_cSymbol / rb_cNilClass / rb_cTrueClass / rb_cFalseClass), "new")
    let vmr = c.vm();
    for kk in [k, vmr.c_nil.get(), vmr.c_true.get(), vmr.c_false.get()] { let sk = singleton_class(c, kk); define_method_me(c, sk, crate::symbol::s::new, new_me(crate::symbol::s::new, sk, VISI_PUBLIC, MKind::Undef)); }
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, k, n, f, a);
    d(c, "to_s", sym_to_s, 0); crate::builtins::object::alias_builtin(c, k, "id2name", "to_s"); d(c, "name", sym_name, 0); d(c, "inspect", sym_inspect, 0);
    d(c, "to_proc", sym_to_proc, 0); d(c, "to_sym", sym_to_sym, 0); d(c, "length", sym_length, 0); d(c, "size", sym_length, 0);
    d(c, "<=>", sym_cmp, 1); d(c, "upcase", sym_upcase, -1); d(c, "downcase", sym_downcase, -1); d(c, "capitalize", sym_capitalize, -1);
    d(c, "swapcase", sym_swapcase, -1); d(c, "succ", sym_succ, 0); d(c, "next", sym_succ, 0); d(c, "[]", sym_aref, -1); d(c, "slice", sym_aref, -1);
    d(c, "start_with?", sym_start_with, -1); d(c, "end_with?", sym_end_with, -1); d(c, "=~", sym_match, 1); d(c, "match", sym_match_m, -1);
    d(c, "match?", sym_match_p, -1); d(c, "empty?", sym_empty, 0); d(c, "casecmp", sym_casecmp, 1); d(c, "casecmp?", sym_casecmp_p, 1);
    d(c, "encoding", sym_encoding_m, 0); d(c, "==", sym_eq, 1); d(c, "===", sym_eq, 1); crate::builtins::object::alias_builtin(c, k, "intern", "to_sym");
    define_singleton_cfunc(c, k, "all_symbols", sym_all, 0);
    let _ = inspect_str;
}
