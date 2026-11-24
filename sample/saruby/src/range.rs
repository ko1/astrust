// Range core.  Methods in builtins/range_methods.rs.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]

#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::heap::obj_set_flags;
use crate::object::*;
use crate::value::*;
use crate::vm::*;

impl Ctx {
    pub fn range_new(&mut self, b: Value, e: Value, excl: bool) -> Value {
        let k = self.vm().c_range.get();
        self.alloc(k, Body::Range(b, e, excl))
    }
    /// (begin, end, exclusive) of a Range; (nil, nil, false) for anything else.
    #[inline]
    pub fn range_parts(&self, r: Value) -> (Value, Value, bool) { if is_range(r) { self.range(r) } else { (NIL, NIL, false) } }
}
/// range_init's check: the ends must be comparable.
pub fn range_check(c: &mut Ctx, b: Value, e: Value) -> Result<(), Esc> {
    let ok = (b.is_fixnum() && e.is_fixnum()) || b == NIL || e == NIL
        || (crate::numeric::is_numeric(b) && crate::numeric::is_numeric(e))
        || (is_string(b) && is_string(e));
    if !ok {
        let r = funcall(c, b, crate::symbol::s::cmp, &[e], std::ptr::null())?;
        if r == NIL { return arg_err(c, "bad value for range"); }
    }
    Ok(())
}
pub fn range_new_checked(c: &mut Ctx, b: Value, e: Value, excl: bool) -> RV {
    range_check(c, b, e)?;
    let r = c.range_new(b, e, excl);
    obj_set_flags(r, FL_FROZEN);
    Ok(r)
}

/// Resolve a range against a length (for Array#[] / String#[]): (start, len), None if out of range.
pub fn range_beg_len(c: &mut Ctx, r: Value, len: i64, err: i32) -> Result<Option<(i64, i64)>, Esc> {
    let (b, e, excl) = c.range_parts(r);
    let mut beg = if b == NIL { 0 } else { crate::builtins::numeric_methods::num_to_long(c, b)? };
    let mut end = if e == NIL { len } else { crate::builtins::numeric_methods::num_to_long(c, e)? };
    let excl = if e == NIL { true } else { excl };
    let ob = beg; let oe = end;
    if beg < 0 { beg += len; if beg < 0 { return out_of_range(c, r, err, ob, oe); } }
    if end < 0 { end += len; }
    if !excl { end = end.saturating_add(1); }
    if err == 0 || err == 2 {
        if beg > len { return out_of_range(c, r, err, ob, oe); }
        if end > len { end = len; }
    }
    let l = end.saturating_sub(beg).max(0);
    Ok(Some((beg, l)))
}
fn out_of_range(c: &mut Ctx, r: Value, err: i32, _b: i64, _e: i64) -> Result<Option<(i64, i64)>, Esc> {
    if err != 0 {
        let s = crate::builtins::kernel::inspect_str(c, r)?;
        return err_range(c, format!("{} out of range", s));
    }
    Ok(None)
}
fn err_range<T>(c: &mut Ctx, m: String) -> Result<T, Esc> { crate::vm::err(c, c.vm().c_range_error.get(), m) }
