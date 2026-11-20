// Heap access core: the only code that turns a `Value` into a reference to an object's payload.
// Design, soundness argument and the migration recipe: docs/safe_heap.md.
//
// Rules this module relies on (the argument is in the doc):
//   H1  a heap `Value` was returned by `gc::alloc` and its object has not been collected (rooting);
//   H2  one `Ctx` runs at a time, and every payload reference is a borrow of it:
//       `&Ctx` reads, `&mut Ctx` writes, allocates, collects and runs Ruby code;
//   H3  the header (klass, flags, body kind) is read and written by value, never borrowed.
// The accessors of object.rs (`rary`, `obj`, ...) break H2; a converted module cannot name them
// (`no_legacy`).
use crate::object::*;
use crate::value::*;
use crate::vm::{Block, BlockKind, Ctx, MethodEntry, Params, Vm};

/// What makes a `Ctx` the heap token: safe code cannot build one.
pub struct Token(());
impl Token {
    /// # Safety
    /// The `Ctx` holding it is the only one running whenever it runs (H2).
    #[inline(always)]
    pub const unsafe fn mint() -> Token { Token(()) }
}

#[cold]
#[inline(never)]
fn wrong_kind(v: Value, want: &str) -> ! { panic!("saruby: {:?} is not {}", v, want) }

#[inline(always)]
fn robj(v: Value) -> *mut RObj { if v.is_heap() { v.ptr() } else { wrong_kind(v, "a heap object") } }

// ---- header: by value, no token ------------------------------------------------------------

/// Class word of a heap object (singleton class included).
#[inline(always)]
pub fn obj_klass(v: Value) -> Value { unsafe { (*robj(v)).klass } }   // SAFETY: H1, H3
#[inline(always)]
pub fn obj_set_klass(v: Value, k: Value) { unsafe { (*robj(v)).klass = k; } }   // SAFETY: H1, H3
#[inline(always)]
pub fn obj_flags(v: Value) -> u32 { unsafe { (*robj(v)).flags } }   // SAFETY: H1, H3
#[inline(always)]
pub fn obj_set_flags(v: Value, bits: u32) { unsafe { (*robj(v)).flags |= bits; } }   // SAFETY: H1, H3
#[inline(always)]
pub fn obj_clear_flags(v: Value, bits: u32) { unsafe { (*robj(v)).flags &= !bits; } }   // SAFETY: H1, H3
#[inline(always)]
pub fn is_frozen(v: Value) -> bool { !v.is_heap() || obj_flags(v) & FL_FROZEN != 0 }

macro_rules! kind_test {
    ($($f:ident $variant:pat),*) => { $(
        // SAFETY: H1; a place match that binds nothing reads the tag byte only (Body is repr(u8))
        #[inline(always)]
        pub fn $f(v: Value) -> bool { v.is_heap() && unsafe { matches!((*v.ptr()).body, $variant) } }
    )* };
}
kind_test!(is_string Body::String(_), is_array Body::Array(_), is_hash Body::Hash(_), is_class_or_module Body::Class(_),
           is_proc Body::Proc(_), is_range Body::Range(..), is_bignum Body::Bignum(_), is_method_obj Body::Method(_),
           is_heap_float Body::Float(_));
#[inline(always)]
pub fn is_float(v: Value) -> bool { v.is_flonum() || is_heap_float(v) }

// ---- payloads: borrows of the context ------------------------------------------------------

// The two functions the soundness argument is about: the result lives as long as the borrow of
// the context, shared for reading, exclusive for writing.
#[inline(always)]
fn body(_c: &Ctx, v: Value) -> &Body { unsafe { &(*robj(v)).body } }   // SAFETY: H1, H2
#[inline(always)]
fn body_mut(_c: &mut Ctx, v: Value) -> &mut Body { unsafe { &mut (*robj(v)).body } }   // SAFETY: H1, H2

macro_rules! payload {
    ($get:ident, $get_mut:ident, $variant:ident, $t:ty, $name:literal) => {
        #[inline(always)]
        pub fn $get(&self, v: Value) -> &$t { match body(self, v) { Body::$variant(x) => x, _ => wrong_kind(v, $name) } }
        #[inline(always)]
        pub fn $get_mut(&mut self, v: Value) -> &mut $t { match body_mut(self, v) { Body::$variant(x) => x, _ => wrong_kind(v, $name) } }
    };
}

impl Ctx {
    payload!(ary_vec, ary_mut, Array, Vec<Value>, "an Array");
    payload!(rstring, rstring_mut, String, RString, "a String");
    payload!(ivars, ivars_mut, Object, crate::ivbuf::IvBuf, "a plain object");
    // boxed payloads: `&mut T`, not the Box
    payload!(hash, hash_mut, Hash, RHash, "a Hash");
    payload!(class, class_mut, Class, ClassData, "a Class or Module");
    payload!(proc_data, proc_data_mut, Proc, ProcData, "a Proc");
    payload!(method_obj, method_obj_mut, Method, MethodObj, "a Method");
    payload!(big, big_mut, Bignum, crate::bignum::BigInt, "a bignum");

    /// The elements of Array `v`.
    #[inline(always)]
    pub fn ary(&self, v: Value) -> &[Value] { self.ary_vec(v) }
    #[inline(always)]
    pub fn str_bytes(&self, v: Value) -> &[u8] { &self.rstring(v).b }
    /// The native payload of a Data object, if `v` is one of type `T`.
    pub fn data<T: 'static>(&self, v: Value) -> Option<&T> {
        if !v.is_heap() { return None; }
        match body(self, v) { Body::Data(d) => d.as_any().downcast_ref::<T>(), _ => None }
    }
    pub fn data_mut<T: 'static>(&mut self, v: Value) -> Option<&mut T> {
        if !v.is_heap() { return None; }
        match body_mut(self, v) { Body::Data(d) => d.as_any_mut().downcast_mut::<T>(), _ => None }
    }

    /// Array `v` for writing, and read access to every other object meanwhile.
    #[inline(always)]
    pub fn ary_mut_and(&mut self, v: Value) -> (&mut Vec<Value>, Others<'_>) {
        let c: &Ctx = self;
        // SAFETY: H1, H2; `Others` refuses `v`, and distinct objects own disjoint payloads
        match unsafe { &mut (*robj(v)).body } { Body::Array(x) => (x, Others { except: v, c }), _ => wrong_kind(v, "an Array") }
    }

    /// Allocate an object; may collect.
    #[inline(always)]
    pub fn alloc(&mut self, klass: Value, body: Body) -> Value { crate::gc::alloc(klass, body) }
    /// alloc with the body built once the slot is known (see gc::alloc_with).
    #[inline(always)]
    pub fn alloc_with(&mut self, klass: Value, make: impl FnOnce() -> Body) -> Value { crate::gc::alloc_with(klass, make) }
    /// Account `n` bytes of payload memory to the collector's trigger.
    #[inline(always)]
    pub fn malloc_increase(&mut self, n: usize) { crate::gc::malloc_increase(n); }

    /// VM state (builtin classes, method identities, tables).
    #[allow(static_mut_refs)]
    #[inline(always)]
    pub fn vm(&self) -> &Vm { unsafe { crate::vm::VM.as_deref().unwrap_unchecked() } }   // SAFETY: set by boot before any Ctx runs; H2
    #[allow(static_mut_refs)]
    #[inline(always)]
    pub fn vm_mut(&mut self) -> &mut Vm { unsafe { crate::vm::VM.as_deref_mut().unwrap_unchecked() } }   // SAFETY: as `vm`
}

/// Read access to every object but one, while that one is borrowed mutably (`Ctx::ary_mut_and`).
pub struct Others<'a> { except: Value, c: &'a Ctx }
impl<'a> Others<'a> {
    /// Elements of Array `v`; None when `v` is the object being written.
    #[inline(always)]
    pub fn ary(&self, v: Value) -> Option<&'a [Value]> { if v == self.except { None } else { Some(self.c.ary(v)) } }
    #[inline(always)]
    pub fn str_bytes(&self, v: Value) -> Option<&'a [u8]> { if v == self.except { None } else { Some(self.c.str_bytes(v)) } }
}

// ---- allocators of unconverted modules, taking the token -----------------------------------
// A converted module must not allocate behind the context's back; these go away as string.rs
// is converted and its own functions take `&mut Ctx`.
impl Ctx {
    #[inline(always)] pub fn str_new(&mut self, b: &[u8]) -> Value { crate::string::str_new(b) }
    #[inline(always)] pub fn str_from_vec(&mut self, b: Vec<u8>) -> Value { crate::string::str_from_vec(b) }
    #[inline(always)] pub fn str_from_vec_enc(&mut self, b: Vec<u8>, enc: u8) -> Value { crate::string::str_from_vec_enc(b, enc) }
}

/// `use crate::heap::no_legacy::*;` in a converted module: every name below then has two glob
/// candidates, so using the unsound original is a compile error (E0659).
#[allow(dead_code)]
pub mod no_legacy {
    macro_rules! ban { ($($n:ident)*) => { $(pub fn $n(_: super::Token) {})* }; }
    ban!(obj rary rstr rhash rclass rproc rmethod rbig rdata ivars_of str_bytes vm
         ary_new ary_new_capa ary_new_from ary_from_vec ary_push ary_len ary_get ary_to_vec
         str_new str_from_vec str_from_vec_enc hash_new float_new);
}

// ---- hash / range / numeric / data ---------------------------------------------------------
// Accessors of the second wave (hash.rs, range.rs, numeric.rs, bignum.rs and their builtins).

/// Value of a Float (flonum or heap).  By value and without the token: a Float body is written
/// by `gc::alloc` only, and no accessor hands out a reference into one.
#[inline(always)]
pub fn float_val(v: Value) -> f64 {
    if v.is_flonum() { return v.flonum_val(); }
    // SAFETY: H1; the reference ends in this expression and nothing else borrows a Float body
    match unsafe { &(*robj(v)).body } { Body::Float(f) => *f, _ => wrong_kind(v, "a Float") }
}

impl Ctx {
    /// Fields of Range `v`: (begin, end, exclusive).
    #[inline(always)]
    pub fn range(&self, v: Value) -> (Value, Value, bool) {
        match body(self, v) { Body::Range(b, e, x) => (*b, *e, *x), _ => wrong_kind(v, "a Range") }
    }
    /// Overwrite the fields of Range `v` (Range#initialize); the kind stays.
    pub fn range_set(&mut self, v: Value, b: Value, e: Value, excl: bool) {
        match body_mut(self, v) { r @ Body::Range(..) => *r = Body::Range(b, e, excl), _ => wrong_kind(v, "a Range") }
    }

    /// Hash `v` for writing, and read access to every other object meanwhile.
    #[inline(always)]
    pub fn hash_mut_and(&mut self, v: Value) -> (&mut RHash, Others<'_>) {
        let c: &Ctx = self;
        // SAFETY: as `ary_mut_and`
        match unsafe { &mut (*robj(v)).body } { Body::Hash(x) => (x, Others { except: v, c }), _ => wrong_kind(v, "a Hash") }
    }
    /// Native payload of `v` for writing, and read access to every other object meanwhile.
    pub fn data_mut_and<T: 'static>(&mut self, v: Value) -> Option<(&mut T, Others<'_>)> {
        if !v.is_heap() { return None; }
        let c: &Ctx = self;
        // SAFETY: as `ary_mut_and`
        match unsafe { &mut (*robj(v)).body } { Body::Data(d) => d.as_any_mut().downcast_mut::<T>().map(|x| (x, Others { except: v, c })), _ => None }
    }

    /// Method `mid` as an instance of `klass` finds it; None when undefined.
    #[inline(always)]
    pub fn method_entry(&self, klass: Value, mid: Sym) -> Option<&MethodEntry> {
        // SAFETY: lookup_method gives null or an entry made by vm::new_me, a leaked Box that is never freed
        unsafe { crate::vm::lookup_method(klass, mid).as_ref() }
    }
    /// Parameters of the literal block a builtin was given; None for no block or a non-iseq one.
    pub fn block_params(&self, blk: *const Block) -> Option<&Params> {
        // SAFETY: a builtin's `blk` is null or its caller's Block, alive for the call (as `yield_block` assumes)
        let b = unsafe { blk.as_ref() }?;
        // SAFETY: iseqs are never freed
        match b.kind { BlockKind::Iseq(is) => unsafe { is.as_ref() }.map(|is| &is.params), _ => None }
    }
}
impl<'a> Others<'a> {
    #[inline(always)]
    pub fn hash(&self, v: Value) -> Option<&'a RHash> { if v == self.except { None } else { Some(self.c.hash(v)) } }
    pub fn data<T: 'static>(&self, v: Value) -> Option<&'a T> { if v == self.except { None } else { self.c.data::<T>(v) } }
}

/// As `no_legacy`, for the context-free API of hash.rs, range.rs, numeric.rs and bignum.rs.
#[allow(dead_code)]
pub mod no_legacy_hn {
    macro_rules! ban { ($($n:ident)*) => { $(pub fn $n(_: super::Token) {})* }; }
    ban!(hash_new_capa hash_pairs hash_len hash_entry_at hash_clear hash_dup range_new range_parts
         num_to_f64 big_to_value to_big int_from_i64 int_from_i128 rfloat_heap);
}
