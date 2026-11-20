// Heap object layout.  Every heap object is one fixed-size slot (`RObj`) in a GC
// page; variable-sized payloads (string bytes, array elements, hash tables, class
// data) live in Rust-owned buffers that the slot owns and drops on sweep.
use crate::value::*;
use crate::vm::{Block, MethodEntry};
use std::any::Any;
use crate::fxhash::HashMap;

pub const FL_FROZEN: u32 = 1;
// bits 2 and 4 are unused (GC state is in RObj::gc)
pub const FL_EXIVAR: u32 = 8;
pub const FL_CHILLED: u32 = 16;   // string literal: frozen-on-write warning semantics (treated as mutable)
pub const FL_KWHASH: u32 = 32;    // ruby2_keywords flagged hash

#[repr(C)]
pub struct RObj {
    pub klass: Value,
    pub flags: u32,
    pub gc: crate::gc::GcHeader,      // owned by the GC backend
    pub body: Body,
}

#[repr(u8)]
pub enum Body {
    Free(*mut RObj),
    Object(crate::ivbuf::IvBuf),
    Class(Box<ClassData>),
    String(RString),
    Array(Vec<Value>),
    Hash(Box<RHash>),
    Float(f64),
    Bignum(Box<crate::bignum::BigInt>),
    Range(Value, Value, bool),
    Proc(Box<ProcData>),
    Env(Vec<Value>, Value),
    Method(Box<MethodObj>),
    Data(Box<dyn RData>),
}

// ivars and short strings live in the slot: growing it costs every object
const _: () = assert!(std::mem::size_of::<Body>() == 40 && std::mem::size_of::<RObj>() == 56);

pub const ENC_UTF8: u8 = 0;
pub const ENC_BINARY: u8 = 1;
pub const ENC_ASCII: u8 = 2;

pub struct RString { pub b: crate::sbuf::SBuf, pub enc: u8 }

#[derive(Clone, Copy)]
pub struct HEntry { pub key: Value, pub val: Value, pub hash: u64 }

pub struct RHash {
    pub entries: Vec<HEntry>,      // insertion order; deleted entries have key == UNDEF
    pub index: Vec<u32>,           // open addressing into entries (u32::MAX = empty); empty when small
    pub live: usize,
    pub default: Value,
    pub default_proc: Value,
    pub identity: bool,
}

pub struct ProcData { pub blk: Block, pub is_lambda: bool, pub from_method: Value }

pub struct MethodObj { pub recv: Value, pub me: *const MethodEntry, pub name: Sym, pub unbound: bool, pub owner: Value }

/// Built-in objects with Rust-defined payloads (Regexp, MatchData, Time, ...).
pub trait RData: Any {
    /// Visit every Value slot the payload holds.
    fn visit(&mut self, _v: &mut crate::gc::Visitor) {}
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocKind { Object, String, Array, Hash, Exception, Class, Module, Proc, Range, None }

pub struct ClassData {
    pub name: Option<Value>,          // String (the path) once named
    pub base_name: Sym,
    pub lexical_parent: Value,
    pub superclass: Value,            // NIL for BasicObject / modules
    pub is_module: bool,
    pub is_singleton: bool,
    pub attached: Value,              // singleton: the object it belongs to
    pub mtbl: HashMap<Sym, *const MethodEntry>,
    pub consts: HashMap<Sym, Value>,
    pub const_order: Vec<Sym>,
    pub autoload: HashMap<Sym, Value>,
    pub cvars: HashMap<Sym, Value>,
    pub includes: Vec<Value>,         // directly included modules, in include order
    pub prepends: Vec<Value>,
    pub extended_by: Vec<Value>,
    pub ivar_index: HashMap<Sym, u32>,
    pub ivar_names: Vec<Sym>,
    pub ancestors: Option<Vec<Value>>,
    pub ancestors_serial: u64,
    pub alloc: AllocKind,
    pub subclasses: Vec<Value>,
    pub private_consts: Vec<Sym>,
    pub refined: bool,
}

impl ClassData {
    pub fn new(superclass: Value, is_module: bool, alloc: AllocKind) -> ClassData {
        ClassData {
            name: None, base_name: Sym(0), lexical_parent: NIL, superclass, is_module, is_singleton: false, attached: NIL,
            mtbl: HashMap::default(), consts: HashMap::default(), const_order: vec![], autoload: HashMap::default(), cvars: HashMap::default(),
            includes: vec![], prepends: vec![], extended_by: vec![], ivar_index: HashMap::default(), ivar_names: vec![],
            ancestors: None, ancestors_serial: 0, alloc, subclasses: vec![], private_consts: vec![], refined: false,
        }
    }
}

// ---- legacy accessors (callers guarantee the body kind) --------------------
// Unsound: unbounded `&'static mut` into GC objects.  Kept for modules not yet converted to the
// context-borrowing accessors of heap.rs (docs/safe_heap.md); a converted module cannot name them.

#[inline(always)] pub fn obj(v: Value) -> &'static mut RObj { unsafe { &mut *v.ptr() } }

macro_rules! body_accessor {
    ($fn:ident, $variant:ident, $t:ty) => {
        #[inline(always)]
        pub fn $fn(v: Value) -> &'static mut $t {
            match &mut obj(v).body { Body::$variant(x) => x, _ => unsafe { std::hint::unreachable_unchecked() } }
        }
    };
}
body_accessor!(rstr, String, RString);
body_accessor!(rary, Array, Vec<Value>);
body_accessor!(ivars_of, Object, crate::ivbuf::IvBuf);
#[inline(always)]
pub fn rhash(v: Value) -> &'static mut RHash { match &mut obj(v).body { Body::Hash(h) => h, _ => unsafe { std::hint::unreachable_unchecked() } } }
#[inline(always)]
pub fn rclass(v: Value) -> &'static mut ClassData { match &mut obj(v).body { Body::Class(h) => h, _ => panic!("not a class: {:?}", v) } }
#[inline(always)]
pub fn rproc(v: Value) -> &'static mut ProcData { match &mut obj(v).body { Body::Proc(h) => h, _ => unsafe { std::hint::unreachable_unchecked() } } }
#[inline(always)]
pub fn rmethod(v: Value) -> &'static mut MethodObj { match &mut obj(v).body { Body::Method(h) => h, _ => unsafe { std::hint::unreachable_unchecked() } } }
#[inline(always)]
pub fn rbig(v: Value) -> &'static mut crate::bignum::BigInt { match &mut obj(v).body { Body::Bignum(h) => h, _ => unsafe { std::hint::unreachable_unchecked() } } }
#[inline(always)]
pub fn rfloat_heap(v: Value) -> f64 { match &obj(v).body { Body::Float(f) => *f, _ => unsafe { std::hint::unreachable_unchecked() } } }

pub fn rdata<T: 'static>(v: Value) -> Option<&'static mut T> {
    if !v.is_heap() { return None; }
    match &mut obj(v).body { Body::Data(d) => d.as_any_mut().downcast_mut::<T>(), _ => None }
}

pub use crate::heap::{is_array, is_bignum, is_class_or_module, is_float, is_frozen, is_hash, is_method_obj, is_proc, is_range, is_string};
#[inline(always)] pub fn is_integer(v: Value) -> bool { v.is_fixnum() || is_bignum(v) }
#[inline(always)]
pub fn float_val(v: Value) -> f64 { if v.is_flonum() { v.flonum_val() } else { rfloat_heap(v) } }

pub fn str_bytes(v: Value) -> &'static [u8] { &rstr(v).b }


// legacy Array API (no context); converted code uses the `Ctx` methods of array.rs
#[inline]
pub fn ary_new() -> Value { crate::gc::alloc(crate::vm::vm().c_array, Body::Array(Vec::new())) }
#[inline]
pub fn ary_new_capa(n: usize) -> Value {
    crate::gc::malloc_increase(n * 8);
    crate::gc::alloc(crate::vm::vm().c_array, Body::Array(Vec::with_capacity(n)))
}
#[inline]
pub fn ary_new_from(v: &[Value]) -> Value {
    crate::gc::malloc_increase(v.len() * 8);
    crate::gc::alloc(crate::vm::vm().c_array, Body::Array(v.to_vec()))
}
pub fn ary_from_vec(v: Vec<Value>) -> Value {
    crate::gc::malloc_increase(v.len() * 8);
    crate::gc::alloc(crate::vm::vm().c_array, Body::Array(v))
}
#[inline]
pub fn ary_push(a: Value, v: Value) {
    let x = rary(a);
    if x.len() == x.capacity() { crate::gc::malloc_increase(x.len() * 8 + 32); }
    x.push(v);
}
#[inline] pub fn ary_len(a: Value) -> usize { rary(a).len() }
#[inline] pub fn ary_get(a: Value, i: usize) -> Value { rary(a).get(i).copied().unwrap_or(NIL) }
pub fn ary_to_vec(a: Value) -> Vec<Value> { rary(a).clone() }
