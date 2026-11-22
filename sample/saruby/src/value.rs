// Tagged 64-bit values (CRuby-like layout, but no value is 0 so that
// `Result<Value, Esc>` fits in one register):
//   ...xxx1  Fixnum (63-bit)
//   ...xx10  Flonum (CRuby's rotated double encoding)
//   ...x000  heap object pointer (non-null, 8-aligned)
//   0x04 nil, 0x0c false, 0x14 true, 0x1c undef
//   (id << 8) | 0x34  Symbol
use std::num::NonZeroU64;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Value(NonZeroU64);

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct Sym(pub u32);

pub const NIL: Value = Value::imm(0x04);
pub const FALSE: Value = Value::imm(0x0c);
pub const TRUE: Value = Value::imm(0x14);
pub const UNDEF: Value = Value::imm(0x1c);
const SYM_TAG: u64 = 0x34;
pub const FIXNUM_MAX: i64 = (1i64 << 62) - 1;
pub const FIXNUM_MIN: i64 = -(1i64 << 62);

impl Value {
    #[inline(always)] pub const fn raw(self) -> u64 { self.0.get() }
    /// An immediate by its bits; in `const { }` (baked code) a zero is a compile error.
    pub const fn imm(r: u64) -> Value { match NonZeroU64::new(r) { Some(x) => Value(x), None => panic!("zero Value") } }
    #[inline(always)] pub const unsafe fn from_raw(r: u64) -> Value { Value(NonZeroU64::new_unchecked(r)) }
    #[inline(always)] pub fn is_fixnum(self) -> bool { self.raw() & 1 == 1 }
    #[inline(always)] pub fn fix(i: i64) -> Value { unsafe { Value::from_raw(((i << 1) | 1) as u64) } }
    #[inline(always)] pub fn fixable(i: i64) -> bool { (FIXNUM_MIN..=FIXNUM_MAX).contains(&i) }
    #[inline(always)] pub fn as_fix(self) -> i64 { (self.raw() as i64) >> 1 }
    /// Are both fixnums?
    #[inline(always)] pub fn both_fix(self, o: Value) -> bool { self.raw() & o.raw() & 1 == 1 }
    // Fixnum arithmetic on the tagged words (2n+1): one add with an overflow test instead of
    // untag / add / range check / retag.  None: the result is not a fixnum.
    // (both are fixnums) `| 1` and `& !1` change nothing but show the compiler an odd sum, so that
    // the non-zero test folds even where it cannot see each operand's tag test
    #[inline(always)] pub fn fix_add(self, o: Value) -> Option<Value> {
        ((self.raw() | 1) as i64).checked_add((o.raw() as i64 - 1) & !1).and_then(|r| std::num::NonZeroU64::new(r as u64)).map(Value)
    }
    #[inline(always)] pub fn fix_sub(self, o: Value) -> Option<Value> {
        ((self.raw() | 1) as i64).checked_sub((o.raw() as i64 - 1) & !1).and_then(|r| std::num::NonZeroU64::new(r as u64)).map(Value)
    }
    /// A fixnum shifted left by 0 <= k < 64, if the result is a fixnum: on the tagged word, 2a << k
    /// keeps its bits exactly when a << k is a fixnum.
    #[inline(always)] pub fn fix_shl(self, k: i64) -> Option<Value> {
        if !(0..64).contains(&k) { return None; }
        let t = self.raw() as i64 - 1;
        let r = t << k;
        if r >> k == t { Value::odd(r as u64) } else { None }
    }
    /// A word with bit 0 set as a fixnum (never 0, so never None).
    #[inline(always)] pub fn odd(r: u64) -> Option<Value> { NonZeroU64::new(r | 1).map(Value) }
    // 2a * b overflows 64 bits exactly when a * b is not a fixnum; `| 1` retags (and shows a non-zero word)
    #[inline(always)] pub fn fix_mul(self, o: Value) -> Option<Value> {
        (self.raw() as i64 - 1).checked_mul(o.as_fix()).and_then(|r| std::num::NonZeroU64::new(r as u64 | 1)).map(Value)
    }
    /// The tagged word as a signed integer: its order is the fixnums' order.
    #[inline(always)] pub fn fix_ord(self) -> i64 { self.raw() as i64 }
    #[inline(always)] pub fn is_flonum(self) -> bool { self.raw() & 3 == 2 }
    #[inline(always)] pub fn is_heap(self) -> bool { self.raw() & 7 == 0 }
    #[inline(always)] pub fn is_special(self) -> bool { self.raw() & 7 == 4 }
    #[inline(always)] pub fn is_imm(self) -> bool { self.raw() & 7 != 0 }
    #[inline(always)] pub fn truthy(self) -> bool { self.raw() & !8 != 4 }
    #[inline(always)] pub fn falsy(self) -> bool { self.raw() & !8 == 4 }
    #[inline(always)] pub fn is_nil(self) -> bool { self == NIL }
    #[inline(always)] pub fn is_undef(self) -> bool { self == UNDEF }
    #[inline(always)] pub fn bool(b: bool) -> Value { if b { TRUE } else { FALSE } }
    #[inline(always)] pub fn is_sym(self) -> bool { self.raw() & 0xff == SYM_TAG }
    #[inline(always)] pub fn sym(s: Sym) -> Value { unsafe { Value::from_raw(((s.0 as u64) << 8) | SYM_TAG) } }
    #[inline(always)] pub fn as_sym(self) -> Sym { Sym((self.raw() >> 8) as u32) }
    #[inline(always)] pub fn ptr(self) -> *mut crate::object::RObj { self.raw() as *mut crate::object::RObj }
    /// # Safety
    /// `p` is an object of the GC heap: heap.rs dereferences every heap Value (rule H1).
    #[inline(always)] pub unsafe fn from_ptr(p: *const crate::object::RObj) -> Value { unsafe { Value::from_raw(p as u64) } }

    /// Flonum encoding (CRuby's rb_float_new_inline); None when the double needs a heap Float.
    #[inline(always)]
    pub fn flonum(d: f64) -> Option<Value> {
        let v = d.to_bits();
        let bits = ((v >> 60) & 7) as i32;
        if v != 0x3000000000000000 && ((bits - 3) & !1) == 0 {
            Some(unsafe { Value::from_raw((v.rotate_left(3) & !1) | 2) })
        } else if v == 0 {
            Some(unsafe { Value::from_raw(0x8000000000000002) })
        } else { None }
    }
    #[inline(always)]
    pub fn flonum_val(self) -> f64 {
        let v = self.raw();
        if v != 0x8000000000000002 {
            let b63 = v >> 63;
            let t = (2u64.wrapping_sub(b63)) | (v & !3);
            f64::from_bits(t.rotate_right(3))
        } else { 0.0 }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_fixnum() { write!(f, "{}", self.as_fix()) }
        else if self.is_flonum() { write!(f, "{}f", self.flonum_val()) }
        else if *self == NIL { write!(f, "nil") }
        else if *self == TRUE { write!(f, "true") }
        else if *self == FALSE { write!(f, "false") }
        else if *self == UNDEF { write!(f, "undef") }
        else if self.is_sym() { write!(f, ":{}", String::from_utf8_lossy(crate::symbol::sym_bytes(self.as_sym()))) }
        else { write!(f, "#<obj {:#x}>", self.raw()) }
    }
}
