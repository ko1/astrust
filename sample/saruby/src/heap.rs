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
use crate::vm::{Block, BlockKind, Cref, Ctx, Frame, Iseq, MethodEntry, Params, Vm};

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
    payload!(ary_vec, ary_mut, Array, crate::avec::AVec, "an Array");
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
    pub fn ary_mut_and(&mut self, v: Value) -> (&mut crate::avec::AVec, Others<'_>) {
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

// ---- strings (string.rs builds its API on these) ---------------------------------------------
impl Ctx {
    /// String `v` for writing, and read access to every other object meanwhile.
    #[inline(always)]
    pub fn str_mut_and(&mut self, v: Value) -> (&mut RString, Others<'_>) {
        let c: &Ctx = self;
        // SAFETY: H1, H2; `Others` refuses `v`, and distinct objects own disjoint payloads
        match unsafe { &mut (*robj(v)).body } { Body::String(x) => (x, Others { except: v, c }), _ => wrong_kind(v, "a String") }
    }
}
impl<'a> Others<'a> {
    /// Payload of String `v`; None when `v` is the object being written.
    #[inline(always)]
    pub fn rstring(&self, v: Value) -> Option<&'a RString> { if v == self.except { None } else { Some(self.c.rstring(v)) } }
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

/// The same for the context-free String API (str_legacy.rs); a module of its own so that a
/// converted module opts in once it no longer calls them.
#[allow(dead_code)]
pub mod no_legacy_str {
    macro_rules! ban { ($($n:ident)*) => { $(pub fn $n(_: super::Token) {})* }; }
    ban!(str_new_enc str_with_capa str_new_frozen str_enc str_eq str_to_sym str_append_same_enc char_offsets str_char_len str_dup ary_of_strs);
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

// ---- third wave: frame chain (read-only), code objects ---------------------------------------
// Reads of what a builtin's caller left on the frame chain (kernel.rs, proc.rs, object.rs).
// H5: the frames from `c.frame` down are alive while the builtin reading them runs (each lives
// on a caller's stack until that caller returns, and the builtin returns first).

impl Ctx {
    /// The whole body of `v`, any kind (Object#dup, Object#inspect dispatch on it); `body` made public.
    #[inline(always)]
    pub fn body_of(&self, v: Value) -> &Body { body(self, v) }
    /// The frame of the code that called the running builtin; None at the bottom.
    #[inline(always)]
    pub fn caller_frame(&self) -> Option<&Frame> {
        // SAFETY: H5 (c.frame and its prev are null or live frames)
        unsafe { self.frame.as_ref().and_then(|f| f.prev.as_ref()) }
    }
    /// The running builtin's own frame (its `me` is the entry being run).
    #[inline(always)]
    pub fn cur_frame(&self) -> Option<&Frame> { unsafe { self.frame.as_ref() } }   // SAFETY: H5
    /// The frame below `f` on the chain.
    #[inline(always)]
    pub fn prev_frame<'a>(&'a self, f: &'a Frame) -> Option<&'a Frame> { unsafe { f.prev.as_ref() } }   // SAFETY: H5
    /// A method entry by pointer: entries are leaked Boxes (vm::new_me) that nothing frees.
    #[inline(always)]
    pub fn me_ref(&self, me: *const MethodEntry) -> Option<&MethodEntry> { unsafe { me.as_ref() } }   // SAFETY: never freed
    /// A lexical scope by pointer: crefs are leaked by vm::new_cref (listed in vm.crefs) and never freed.
    #[inline(always)]
    pub fn cref_ref(&self, cr: *const Cref) -> Option<&Cref> { unsafe { cr.as_ref() } }   // SAFETY: never freed
    /// An iseq by pointer: iseqs are never freed.
    #[inline(always)]
    pub fn iseq_ref(&self, is: *const Iseq) -> Option<&Iseq> { unsafe { is.as_ref() } }   // SAFETY: never freed
    /// vm::REFINEMENTS_ON (a `static mut bool` of vm.rs), by value.
    #[inline(always)]
    pub fn refinements_on(&self) -> bool { unsafe { crate::vm::REFINEMENTS_ON } }   // SAFETY: one interpreter thread; read by value, no reference made
    #[inline(always)]
    pub fn set_refinements_on(&mut self) { unsafe { crate::vm::REFINEMENTS_ON = true; } }   // SAFETY: as refinements_on
    /// `f(c, iseq)`: an iseq beside the context. Sound because nothing makes a `&mut Iseq`
    /// (they are built, leaked and then only read, `Cell` fields included), so a shared
    /// reference cannot conflict with anything `&mut Ctx` reaches; the closure scopes it.
    #[inline(always)]
    pub fn with_iseq<R>(&mut self, is: *const Iseq, f: impl FnOnce(&mut Ctx, Option<&Iseq>) -> R) -> R {
        let r = unsafe { is.as_ref() };   // SAFETY: as above; iseqs are never freed
        f(self, r)
    }
    /// The Block a builtin received, by value (H4).
    #[inline(always)]
    pub fn block_copy(&self, blk: *const Block) -> Option<Block> { unsafe { blk.as_ref().copied() } }   // SAFETY: H4
    /// Change a method entry's visibility in place (Module#private on an owned method). A place
    /// write: no reference is made; `me_ref` borrows are excluded by `&mut self`.
    #[inline(always)]
    pub fn me_set_visi(&mut self, me: *const MethodEntry, visi: u8) {
        if !me.is_null() { unsafe { (*(me as *mut MethodEntry)).visi = visi; } }   // SAFETY: entries are never freed
    }
    /// Default visibility of the caller's frame (`private` with no arguments): a place write.
    #[inline(always)]
    pub fn set_caller_visi(&mut self, visi: u8) {
        // SAFETY: H5
        unsafe { if let Some(f) = self.frame.as_ref() { if !f.prev.is_null() { (*f.prev).visi = visi; } } }
    }
    /// Replace the caller frame's lexical scope (`using`).
    #[inline(always)]
    pub fn set_caller_cref(&mut self, cr: *const Cref) {
        // SAFETY: H5
        unsafe { if let Some(f) = self.frame.as_ref() { if let Some(p) = f.prev.as_mut() { p.set_cref(cr); } } }
    }
    /// Set the refinements of a cref (one just made by vm::new_cref): a place write.
    #[inline(always)]
    pub fn cref_set_refinements(&mut self, cr: *const Cref, refs: Value) {
        if !cr.is_null() { unsafe { (*(cr as *mut Cref)).refinements = refs; } }   // SAFETY: crefs are never freed
    }
}
