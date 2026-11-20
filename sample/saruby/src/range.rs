// Range core.  Methods in builtins/range_methods.rs.
use crate::object::*;
use crate::value::*;
use crate::vm::*;

pub fn range_new(b: Value, e: Value, excl: bool) -> Value {
    crate::gc::alloc(vm().c_range, Body::Range(b, e, excl))
}
pub fn range_new_checked(c: &mut Ctx, b: Value, e: Value, excl: bool) -> RV {
    let ok = (b.is_fixnum() && e.is_fixnum()) || b == NIL || e == NIL
        || (crate::numeric::is_numeric(b) && crate::numeric::is_numeric(e))
        || (is_string(b) && is_string(e));
    if !ok {
        let r = funcall(c, b, crate::symbol::s::cmp, &[e], std::ptr::null())?;
        if r == NIL { return arg_err(c, "bad value for range"); }
    }
    let r = range_new(b, e, excl);
    obj(r).flags |= FL_FROZEN;
    Ok(r)
}
#[inline]
pub fn range_parts(r: Value) -> (Value, Value, bool) {
    match &obj(r).body { Body::Range(a, b, x) => (*a, *b, *x), _ => (NIL, NIL, false) }
}

/// Resolve a range against a length (for Array#[] / String#[]): (start, len), None if out of range.
pub fn range_beg_len(c: &mut Ctx, r: Value, len: i64, err: i32) -> Result<Option<(i64, i64)>, Esc> {
    let (b, e, excl) = range_parts(r);
    let mut beg = if b == NIL { 0 } else { crate::builtins::numeric_methods::num_to_long(c, b)? };
    let mut end = if e == NIL { len } else { crate::builtins::numeric_methods::num_to_long(c, e)? };
    let excl = if e == NIL { true } else { excl };
    let ob = beg; let oe = end;
    if beg < 0 { beg += len; if beg < 0 { return out_of_range(c, r, err, ob, oe); } }
    if end < 0 { end += len; }
    if !excl { end += 1; }
    if err == 0 || err == 2 {
        if beg > len { return out_of_range(c, r, err, ob, oe); }
        if end > len { end = len; }
    }
    let l = (end - beg).max(0);
    Ok(Some((beg, l)))
}
fn out_of_range(c: &mut Ctx, r: Value, err: i32, _b: i64, _e: i64) -> Result<Option<(i64, i64)>, Esc> {
    if err != 0 {
        let s = crate::builtins::kernel::inspect_str(c, r)?;
        return err_range(c, format!("{} out of range", s));
    }
    Ok(None)
}
fn err_range<T>(c: &mut Ctx, m: String) -> Result<T, Esc> { crate::vm::err(c, vm().c_range_error, m) }
