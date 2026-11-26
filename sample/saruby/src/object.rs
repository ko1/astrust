// Heap object layout.  Every heap object is one fixed-size slot (`RObj`) in a GC
// page; variable-sized payloads (string bytes, array elements, hash tables, class
// data) live in Rust-owned buffers that the slot owns and drops on sweep.
use crate::value::*;
use crate::vm::{Block, MethodEntry};
use std::any::Any;
use crate::fxhash::HashMap;

pub const FL_FROZEN: u32 = 1;
pub const FL_TMPLOCK: u32 = 2;   // String: rb_str_locktmp (IO::Buffer.for / .string); modification raises
pub const FL_CHILLED: u32 = 16;   // string literal: frozen-on-write warning semantics (treated as mutable)
pub const FL_CHILLED_SYM: u32 = 8; // ... a String from Symbol#to_s (its own warning)
pub const FL_CHILLED_ANY: u32 = FL_CHILLED | FL_CHILLED_SYM;
/// Bits that send a write to the slow check (frozen / locked raise; chilled warns once).
pub const FL_MODIFY_CHECK: u32 = FL_FROZEN | FL_TMPLOCK | FL_CHILLED_ANY;
pub const FL_KWHASH: u32 = 32;    // ruby2_keywords flagged hash
pub const FL_SHAREABLE: u32 = 64;   // Ractor.shareable? said yes (ractor.rs); never copied by dup / clone
pub const FL_PRIVATE_CALL: u32 = 4;  // NoMethodError#private_call? (no ivar: a raise stays as cheap)
// bit 128 is unused.  The flags below name side tables and weak tables; they share one
// byte so that finalize and scan_edges test them with one byte-wide instruction.
pub const FL_EXIVAR: u32 = 0x100;
pub const FL_TRACED: u32 = 0x200;    // has an entry in objspace's allocation table (removed on free)
pub const FL_HAS_ID: u32 = 0x400;    // has an entry in Vm::object_ids (removed on free)
pub const FL_WEAK_KEYS: u32 = 0x800; // hidden Hash of a WeakMap / WeakKeyMap: the collector does not mark its keys
pub const FL_WEAK_VALS: u32 = 0x1000; // ... nor its values (WeakMap)

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

/// The bytes and the encoding, which sits in the buffer's spare byte.
pub struct RString { pub b: StrBytes }
/// A String's SBuf; not constructible outside, so `r.b = ..` (which would drop the encoding) does not compile.
pub struct StrBytes(crate::sbuf::SBuf);
impl std::ops::Deref for StrBytes { type Target = crate::sbuf::SBuf; #[inline(always)] fn deref(&self) -> &crate::sbuf::SBuf { &self.0 } }
impl std::ops::DerefMut for StrBytes { #[inline(always)] fn deref_mut(&mut self) -> &mut crate::sbuf::SBuf { &mut self.0 } }
impl RString {
    #[inline(always)]
    pub fn new(mut b: crate::sbuf::SBuf, enc: u8) -> RString { b.set_aux(enc); RString { b: StrBytes(b) } }
    #[inline(always)]
    pub fn enc(&self) -> u8 { self.b.0.aux() }
    #[inline(always)]
    pub fn set_enc(&mut self, e: u8) { self.b.0.set_aux(e) }
    /// Replace the bytes, keep the encoding.
    #[inline(always)]
    pub fn set_bytes(&mut self, b: crate::sbuf::SBuf) { self.b.0.set_bytes(b) }
}
impl Clone for RString {
    /// The bytes and the encoding (the buffer's clone carries it).
    #[inline(always)]
    fn clone(&self) -> RString { RString { b: StrBytes(self.b.0.clone()) } }
}

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
    pub temp_name: bool,              // `name` is temporary (set_temporary_name, or under an anonymous module)
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
    pub cvars: CvarTbl,
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
    pub cx: Option<Box<ConstExtra>>,   // made by the first constant defined from Ruby code (most classes have none)
}

/// Class variables in definition order (Module#class_variables lists them so).
#[derive(Clone, Default)]
pub struct CvarTbl(Vec<(Sym, Value)>);
impl CvarTbl {
    pub fn get(&self, n: &Sym) -> Option<&Value> { self.0.iter().find(|e| e.0 == *n).map(|e| &e.1) }
    pub fn contains_key(&self, n: &Sym) -> bool { self.0.iter().any(|e| e.0 == *n) }
    pub fn insert(&mut self, n: Sym, v: Value) { match self.0.iter_mut().find(|e| e.0 == n) { Some(e) => e.1 = v, None => self.0.push((n, v)) } }
    pub fn remove(&mut self, n: &Sym) -> Option<Value> { let i = self.0.iter().position(|e| e.0 == *n)?; Some(self.0.remove(i).1) }
    pub fn keys(&self) -> impl Iterator<Item = &Sym> { self.0.iter().map(|e| &e.0) }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Value> { self.0.iter_mut().map(|e| &mut e.1) }
}
impl IntoIterator for CvarTbl { type Item = (Sym, Value); type IntoIter = std::vec::IntoIter<(Sym, Value)>; fn into_iter(self) -> Self::IntoIter { self.0.into_iter() } }

/// Constant bookkeeping beside the table: locations, deprecations, autoloads in progress.
#[derive(Default)]
pub struct ConstExtra {
    pub const_loc: HashMap<Sym, SrcLoc>,   // where each constant (or its autoload) was defined
    pub deprecated: Vec<Sym>,
    pub autoloading: Vec<AutoloadRun>,
}

pub type SrcLoc = (&'static str, u32);
/// An autoload being run by `thread`: a constant it assigns waits here until the require ends.
pub struct AutoloadRun { pub name: Sym, pub thread: Value, pub value: Value, pub loc: Option<SrcLoc> }

impl ClassData {
    pub fn cx_mut(&mut self) -> &mut ConstExtra { self.cx.get_or_insert_with(Default::default) }
    pub fn const_loc_of(&self, n: Sym) -> Option<SrcLoc> { self.cx.as_ref()?.const_loc.get(&n).copied() }
    pub fn autoload_runs(&self) -> &[AutoloadRun] { self.cx.as_deref().map_or(&[], |x| &x.autoloading) }
    pub fn is_deprecated(&self, n: Sym) -> bool { self.cx.as_ref().is_some_and(|x| x.deprecated.contains(&n)) }
    pub fn new(superclass: Value, is_module: bool, alloc: AllocKind) -> ClassData {
        ClassData {
            name: None, temp_name: false, base_name: Sym(0), lexical_parent: NIL, superclass, is_module, is_singleton: false, attached: NIL,
            mtbl: HashMap::default(), consts: HashMap::default(), const_order: vec![], autoload: HashMap::default(), cvars: CvarTbl::default(),
            includes: vec![], prepends: vec![], extended_by: vec![], ivar_index: HashMap::default(), ivar_names: vec![],
            ancestors: std::cell::RefCell::new(None), alloc, subclasses: vec![], private_consts: vec![], refined: false, cx: None,
        }
    }
}

// Payloads are reached only through the context-borrowing accessors of heap.rs (docs/safe_heap.md).

pub use crate::heap::{float_val, is_array, is_bignum, is_class_or_module, is_float, is_frozen, is_hash, is_method_obj, is_object, is_proc, is_range, is_string};
#[inline(always)] pub fn is_integer(v: Value) -> bool { v.is_fixnum() || is_bignum(v) }



pub use crate::legacy::*;
