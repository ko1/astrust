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
    Array(crate::avec::AVec),
    Hash(Box<RHash>),
    Float(f64),
    Bignum(Box<crate::bignum::BigInt>),
    Range(Value, Value, bool),
    Proc(Box<ProcData>),
    Env(crate::envvals::EnvVals, Value),   // up to 2 values in the slot (envvals.rs)
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
    pub iter_lev: u32,             // running iterations (CRuby's iter_lev): no new key, no rehash meanwhile
    pub first: usize,              // every slot before it is deleted (CRuby's entries_start): shift is O(1)
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
    // (hierarchy serial, ancestors): a cache that `vm::ancestors` fills under a shared borrow
    pub ancestors: std::cell::RefCell<Option<(u64, std::rc::Rc<[Value]>)>>,
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
            ancestors: std::cell::RefCell::new(None), alloc, subclasses: vec![], private_consts: vec![], refined: false,
        }
    }
}

// Payloads are reached only through the context-borrowing accessors of heap.rs (docs/safe_heap.md).

pub use crate::heap::{float_val, is_array, is_bignum, is_class_or_module, is_float, is_frozen, is_hash, is_method_obj, is_object, is_proc, is_range, is_string};
#[inline(always)] pub fn is_integer(v: Value) -> bool { v.is_fixnum() || is_bignum(v) }



pub use crate::legacy::*;
