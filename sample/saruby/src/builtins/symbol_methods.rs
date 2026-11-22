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

fn sym_str_v(c: &mut Ctx, v: Value) -> Value { let b = sym_bytes(v.as_sym()); c.str_new_enc(b, if is_ascii(b) { ENC_ASCII } else { ENC_UTF8 }) }

fn sym_to_s(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(sym_str_v(c, s)) }
fn sym_name(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let k = s.as_sym();
    if let Some(&v) = c.vm().sym_names.borrow().get(&k) { return Ok(v); }
    let v = sym_str_v(c, s); obj_set_flags(v, FL_FROZEN);
    c.vm().sym_names.borrow_mut().insert(k, v);
    Ok(v)
}
fn sym_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(c.str_new(crate::string::sym_inspect(sym_bytes(s.as_sym())).as_bytes())) }
fn sym_to_proc(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { crate::builtins::proc::sym_to_proc(c, s) }
fn sym_eq(_c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(s == a[0])) }
fn sym_to_sym(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(s) }
fn sym_length(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(utf8_len(sym_bytes(s.as_sym())) as i64)) }
fn sym_cmp(_c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if !a[0].is_sym() { return Ok(NIL); }
    Ok(Value::fix(sym_bytes(s.as_sym()).cmp(sym_bytes(a[0].as_sym())) as i64))
}
// String's builtin, not a dispatch: Symbol#upcase must not see a user's String#upcase (CRuby)
fn via_str(c: &mut Ctx, s: Value, f: crate::vm::CFunc, a: &[Value], b: *const Block) -> RV {
    let st = sym_str_v(c, s);
    f(c, st, a, b)
}
fn sym_upcase(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { let r = via_str(c, s, crate::builtins::string_methods::s_upcase, a, b)?; Ok(c.str_to_sym(r)) }
fn sym_downcase(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { let r = via_str(c, s, crate::builtins::string_methods::s_downcase, a, b)?; Ok(c.str_to_sym(r)) }
fn sym_capitalize(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { let r = via_str(c, s, crate::builtins::string_methods::s_capitalize, a, b)?; Ok(c.str_to_sym(r)) }
fn sym_swapcase(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { let r = via_str(c, s, crate::builtins::string_methods::s_swapcase, a, b)?; Ok(c.str_to_sym(r)) }
fn sym_succ(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { let r = via_str(c, s, crate::builtins::string_methods::s_succ, a, b)?; Ok(c.str_to_sym(r)) }
fn sym_aref(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { via_str(c, s, crate::builtins::string_methods::s_aref, a, b) }
fn sym_start_with(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { via_str(c, s, crate::builtins::string_methods::s_start_with, a, b) }
fn sym_end_with(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { via_str(c, s, crate::builtins::string_methods::s_end_with, a, b) }
fn sym_match(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { via_str(c, s, crate::builtins::string_methods::s_match_op, a, b) }
fn sym_match_m(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { via_str(c, s, crate::builtins::string_methods::s_match, a, b) }
fn sym_match_p(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { via_str(c, s, crate::builtins::string_methods::s_match_p, a, b) }
fn sym_empty(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(sym_bytes(s.as_sym()).is_empty())) }
fn sym_casecmp(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { if !a[0].is_sym() { return Ok(NIL); } let o = sym_str_v(c, a[0]); via_str(c, s, crate::builtins::string_methods::s_casecmp, &[o], b) }
fn sym_casecmp_p(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { if !a[0].is_sym() { return Ok(NIL); } let o = sym_str_v(c, a[0]); via_str(c, s, crate::builtins::string_methods::s_casecmp_p, &[o], b) }
fn sym_all(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let a = c.ary_new();
    for i in 1..crate::symbol::all_count() { c.ary_push(a, Value::sym(Sym(i as u32))); }
    Ok(a)
}
fn sym_encoding(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let b = sym_bytes(s.as_sym());
    crate::builtins::misc::encoding_value(c, if is_ascii(b) { ENC_ASCII } else { ENC_UTF8 })
}

pub fn init(k: Value) {
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(k, n, f, a);
    d("to_s", sym_to_s, 0); d("id2name", sym_to_s, 0); d("name", sym_name, 0); d("inspect", sym_inspect, 0);
    d("to_proc", sym_to_proc, 0); d("to_sym", sym_to_sym, 0); d("length", sym_length, 0); d("size", sym_length, 0);
    d("<=>", sym_cmp, 1); d("upcase", sym_upcase, -1); d("downcase", sym_downcase, -1); d("capitalize", sym_capitalize, -1);
    d("swapcase", sym_swapcase, -1); d("succ", sym_succ, 0); d("next", sym_succ, 0); d("[]", sym_aref, -1); d("slice", sym_aref, -1);
    d("start_with?", sym_start_with, -1); d("end_with?", sym_end_with, -1); d("=~", sym_match, 1); d("match", sym_match_m, -1);
    d("match?", sym_match_p, -1); d("empty?", sym_empty, 0); d("casecmp", sym_casecmp, 1); d("casecmp?", sym_casecmp_p, 1);
    d("encoding", sym_encoding, 0); d("==", sym_eq, 1); d("===", sym_eq, 1); d("intern", sym_to_sym, 0);
    define_singleton_cfunc(k, "all_symbols", sym_all, 0);
    let _ = inspect_str;
}
