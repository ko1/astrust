// VM core: execution context, frames, blocks, methods, classes, calls, exceptions.
#![allow(clippy::missing_safety_doc)]
use crate::heap::{obj_clear_flags, obj_flags, obj_klass, obj_set_flags, obj_set_klass};
use crate::gc::{EdgeVisitor, Saruby, RootVisitor};
use crate::node::{NodePtr, DispatchFn};
use crate::object::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_body::*;   // no object accessor (docs/safe_heap.md)
use crate::symbol::{intern, intern_str, s, sym_str};
use crate::value::*;
pub use crate::stack::*;
use crate::fxhash::HashMap;
use std::cell::{Cell, RefCell};

// ---- non-local exits -----------------------------------------------------------

/// The error side of RV carries no data: the kind and payload are in the Ctx, so
/// `Result<Value, Esc>` is one register (Value is never 0, Err is the 0 niche).
#[derive(Debug, Clone, Copy)]
pub struct Esc;
pub type RV = Result<Value, Esc>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EscKind { None, Raise, Break, Next, Return, LoopBreak, LoopNext, Retry, Throw, Redo, Exit }

// ---- code descriptors --------------------------------------------------------------

pub struct CodeInfo { pub file: &'static str, pub label: String, pub line: u32, pub end_line: u32 }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IseqKind { Method, Block, Lambda, Class, Top, Eval, Main }

pub struct KwParam { pub name: Sym, pub slot: u32, pub default: Option<NodePtr>, pub required: bool, pub static_default: bool }

#[derive(Default)]
pub struct Params {
    pub lead: Vec<u32>,
    pub opt: Vec<(u32, NodePtr)>,      // (slot, assignment node evaluated when the arg is missing)
    pub rest: Option<u32>,
    pub post: Vec<u32>,
    pub kw: Vec<KwParam>,
    pub kwrest: Option<u32>,
    pub no_kw: bool,                    // **nil
    pub block: Option<u32>,
    pub destructure: Vec<NodePtr>,      // run after setup: ((a, b)) params
    pub forwarding: bool,               // (...)
    pub has_trailing_comma: bool,       // |a,| in blocks
    pub anon_rest: bool,
    pub anon_kwrest: bool,
    pub anon_block: bool,
    pub slot_names: Vec<(u32, Sym)>,    // parameter slot -> name
}

pub struct Iseq {
    pub body: NodePtr,
    pub entry: NodePtr,                 // node_method_entry over body (required-params-only methods), else null
    pub bentry: NodePtr,                // node_block_entry over body (required-params-only blocks), else null
    pub kentry: NodePtr,                // dense keyword method: entry taking lead + every keyword in order, else null
    pub dentry: NodePtr,                // dense keyword method without required keywords: entry taking lead only (defaults run)
    pub body_used: std::cell::Cell<bool>, // a block with bentry (method with kentry) was also run through its body: bake the body too
    pub nlocals: u32,                   // frame size including self (0) and env (1)
    pub params: Params,
    pub env_size: u32,
    pub env_params: Vec<(u32, u32)>,    // captured params: (slot, env index)
    pub info: CodeInfo,
    pub kind: IseqKind,
    pub simple: bool,                   // lead-only params at slots 2.. and no env
    pub lead_only: bool,                // only lead params (env allowed)
    pub name: Sym,
    pub local_names: Vec<(Sym, i32, i32)>, // (name, slot or -1, env index or -1)
    pub arity: i32,
    pub uses_block: bool,
    pub parent: std::cell::Cell<*const Iseq>,          // lexically enclosing iseq (set when it is finished)
    pub outer_names: Vec<(Sym, u32, u32)>,             // eval: the binding's names (name, hops, idx)
    pub ruby2_keywords: std::cell::Cell<bool>,         // Module#ruby2_keywords: flag a trailing keyword Hash in *rest
    pub traced: bool,                                  // compiled with trace points (TRACE_COMPILE)
}

impl Iseq {
    /// The entry a call with lead positional arguments and no keywords may take (null: none).
    #[inline(always)]
    pub fn plain_entry(&self) -> NodePtr { if self.entry.is_null() { self.dentry } else { self.entry } }
}

// ---- blocks ---------------------------------------------------------------------------

pub type CBlockFn = fn(&mut Ctx, Value, &[Value], *const Block) -> RV;

#[derive(Clone, Copy)]
pub enum BlockKind { Iseq(*const Iseq), Sym(Sym), Method(Value), Cfunc(CBlockFn, Value), Proc(Value) }

#[derive(Clone, Copy)]
pub struct Block {
    pub kind: BlockKind,
    pub self_: Value,
    pub env: Value,
    pub me: *const MethodEntry,
    pub outer_blk: *const Block,
    pub cref: *const Cref,
    pub home_id: u64,
    pub tag: u64,
    pub proc_val: Value,
    pub defined_class: Value,
}

impl Block {
    pub fn cfunc(c: &mut Ctx, f: CBlockFn, data: Value) -> Block {
        let tag = c.next_serial();
        Block { kind: BlockKind::Cfunc(f, data), self_: NIL, env: NIL, me: std::ptr::null(), outer_blk: std::ptr::null(),
                cref: std::ptr::null(), home_id: 0, tag, proc_val: UNDEF, defined_class: NIL }
    }
}

// ---- methods ------------------------------------------------------------------------------

pub type CFunc = fn(&mut Ctx, Value, &[Value], *const Block) -> RV;

pub const VISI_PUBLIC: u8 = 0;
pub const VISI_PRIVATE: u8 = 1;
pub const VISI_PROTECTED: u8 = 2;
pub const VISI_MODFUNC: u8 = 3;

#[derive(Clone, Copy)]
pub enum MKind { Iseq(*const Iseq), Cfunc(CFunc, i32), AttrReader(Sym), AttrWriter(Sym), Bmethod(Value), Undef, ZSuper }

pub struct MethodEntry { pub name: Sym, pub owner: Value, pub visi: u8, pub kind: MKind, pub orig_owner: Value, pub cref: *const Cref }

pub fn new_me(name: Sym, owner: Value, visi: u8, kind: MKind) -> *const MethodEntry {
    Box::into_raw(Box::new(MethodEntry { name, owner, visi, kind, orig_owner: owner, cref: std::ptr::null() }))
}
pub fn new_me_cref(name: Sym, owner: Value, visi: u8, kind: MKind, cref: *const Cref) -> *const MethodEntry {
    Box::into_raw(Box::new(MethodEntry { name, owner, visi, kind, orig_owner: owner, cref }))
}

// ---- lexical scope -------------------------------------------------------------------------

pub struct Cref { pub klass: Value, pub prev: *const Cref, pub refinements: Value }

pub fn new_cref(klass: Value, prev: *const Cref) -> *const Cref {
    let p = Box::into_raw(Box::new(Cref { klass, prev, refinements: NIL })) as *const Cref;
    vm().crefs.borrow_mut().push(p);
    p
}

// ---- inline caches -------------------------------------------------------------------------

// Bumped when a method table, a visibility or the hierarchy changes; call caches compare it.
// A static (not a Vm field): one load in baked code, and no `static mut` on the call path.
static METHOD_SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
#[inline(always)]
pub fn method_serial() -> u64 { METHOD_SERIAL.load(std::sync::atomic::Ordering::Relaxed) }
pub fn bump_method_serial() { METHOD_SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }

// ---- tracing (TracePoint, set_trace_func; builtins/tracepoint.rs) --------------------------------
// Event bits as CRuby's RUBY_EVENT_*.  The mask is the union of the enabled hooks' events; it is
// read only on slow paths and in trace points that compile.rs puts into code compiled for tracing.
pub const EV_LINE: u32 = 0x1;
pub const EV_CLASS: u32 = 0x2;
pub const EV_END: u32 = 0x4;
pub const EV_CALL: u32 = 0x8;
pub const EV_RETURN: u32 = 0x10;
pub const EV_C_CALL: u32 = 0x20;
pub const EV_C_RETURN: u32 = 0x40;
pub const EV_RAISE: u32 = 0x80;
pub const EV_B_CALL: u32 = 0x100;
pub const EV_B_RETURN: u32 = 0x200;
pub const EV_THREAD_BEGIN: u32 = 0x400;
pub const EV_THREAD_END: u32 = 0x800;
pub const EV_SCRIPT_COMPILED: u32 = 0x2000;
pub const EV_RESCUE: u32 = 0x4000;
pub const EV_CALLS: u32 = EV_CALL | EV_RETURN | EV_C_CALL | EV_C_RETURN | EV_B_CALL | EV_B_RETURN;
static TRACE_MASK: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
#[inline(always)]
pub fn trace_mask() -> u32 { TRACE_MASK.load(std::sync::atomic::Ordering::Relaxed) }
/// While call events are traced, caches keep no fast path (fill_cc), so every call reaches invoke.
pub fn set_trace_mask(m: u32) {
    let old = TRACE_MASK.swap(m, std::sync::atomic::Ordering::Relaxed);
    if (old ^ m) & EV_CALLS != 0 { bump_method_serial(); }
    update_raise_hooks();
}
/// Code compiled from now on gets trace points (compile.rs); set by the first source that names
/// TracePoint or set_trace_func, or by creating a hook.
pub static TRACE_COMPILE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Default)]
// spec_klass: cc.klass while the entry it holds is the one baked into the site (call_spec_out)
// bi: which inlinable builtin it holds, and for which receivers (inline_builtin::cache_key)
pub struct CallCache { pub klass: u64, pub serial: u64, pub me: usize, pub fast: usize, pub fast_hash: u64, pub fast_argc: u32, pub poly_next: u32, pub poly: usize, pub cfunc: Option<CFunc>, pub arity: i32, pub bi: u32, pub spec_klass: u64 }
/// An operator site's call cache, made when the site first calls a method: most sites stay on
/// their Integer / Float / Array paths, and a node holding one word keeps the tree dense.
#[derive(Default)]
pub struct OpCc(Option<Box<CallCache>>);
impl OpCc {
    #[inline(always)] pub fn get(&mut self) -> &mut CallCache { self.0.get_or_insert_with(Default::default) }
    #[inline(always)] pub fn peek(&self) -> Option<&CallCache> { self.0.as_deref() }
}
/// A `yield` site: `seen` = the block iseq it ran (1: several), what the bake speculates on;
/// `fast` = the iseq whose entry is compiled into the site, set once its hash was checked.
#[derive(Default)]
pub struct YieldCache { pub fast: usize, pub seen: usize }
impl YieldCache {
    #[inline(always)]
    pub fn note(&mut self, is: usize) { if self.seen != is && self.seen != 1 { self.seen = if self.seen == 0 { is } else { 1 }; } }
}
#[derive(Default)]
pub struct ConstCache { pub serial: u64, pub cref: usize, pub val: u64 }
#[derive(Default)]
pub struct IvarCache { pub klass: u64, pub idx: u32 }
/// `when Klass` site: the class tested (pat), kind_serial then, and subject classes asked with the answer in bit 0
#[derive(Default)]
pub struct KindMemo { pub pat: u64, pub ser: u64, pub memo: [u64; KIND_WAYS] }
pub const KIND_WAYS: usize = 8;
/// The memo entry of a subject class (classes sit 56 bytes apart: multiply to spread them).
#[inline(always)]
pub fn kind_way(k: u64) -> usize { (k.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 61) as usize }
/// Changes whenever a method or an ancestry does (both counters only grow).
#[inline(always)]
pub fn kind_serial() -> u64 { method_serial() + vm().hierarchy_serial.get() }

// ---- frames ---------------------------------------------------------------------------------

// node_method_entry flags
pub const ME_ENV: u32 = 1;    // the frame has a heap env
pub const ME_ID: u32 = 2;     // the frame needs an id: a block literal (or eval) in the body can return to it
pub const ME_LEAF: u32 = 4;   // the body cannot call, raise or observe a frame: no frame is made
pub const ME_LAMBDA: u32 = 8; // node_block_entry of a `->` literal: lambda frame (own return target)
pub const ME_NOCALL: u32 = 16; // the body has no call site: the entry restores c.frame from a register, not from the frame

pub const FK_METHOD: u8 = 0;
pub const FK_BLOCK: u8 = 1;
pub const FK_LAMBDA: u8 = 2;
pub const FK_TOP: u8 = 3;
pub const FK_CLASS: u8 = 4;
pub const FK_CFUNC: u8 = 5;
pub const FK_EVAL: u8 = 6;

#[repr(C)]
/// The part of a frame every call writes.  A `lite` frame (node_method_entry) is only this;
/// any other frame is the head of a FrameFull.
#[repr(C)]
pub struct Frame {
    pub prev: *mut Frame,
    pub fp: FpHead,       // the frame's self and env slots
    pub me: *const MethodEntry,
    pub blk: *const Block,
    pub id: u64,          // return target: method frame id (blocks: the home method's; lambdas: own)
    pub pos: Pos,         // the current line: where it is kept (a node's), read only when it is needed
    pub kind: u8,
    pub visi: u8,
    pub lite: bool,
    pub pad: u8,          // written with kind..lite as one word
}

/// A source line as a frame keeps it: a reference to the line operand of the node being run (or of an
/// iseq), not the number.  A site then stores its own address, which specialized code shared by sites
/// at other lines computes from the tree's root, and no line number enters specialized code or its hash.
/// A body's patch-bound copy (aot.rs) stores the number itself instead, tagged: `line << 1 | 1`.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Pos(*const u32);
impl Pos {
    #[inline(always)] pub fn at(l: &'static u32) -> Pos { Pos(l) }
    /// # Safety
    /// `p` is `line << 1 | 1` (a hole the loader bound: aot.rs `hole_value`).
    #[inline(always)] pub const unsafe fn bound(p: *const u8) -> Pos { Pos(p as *const u32) }
    #[inline]
    pub fn line(self) -> u32 {
        let a = self.0.addr();
        // SAFETY: an untagged Pos is made only by `at`, from a &'static u32.
        if a & 1 != 0 { (a >> 1) as u32 } else { unsafe { *self.0 } }
    }
}
pub static LINE_0: u32 = 0;
pub static LINE_1: u32 = 1;

/// A node's line as its body gets it (@hole `line`): `LineAt` reads the node, a bound copy's unit
/// struct (aot.rs, generated per hole) is the number, stored with one instruction.
pub trait LineHole: Copy {
    fn pos(self) -> Pos;
    /// The frame is at the current line.
    fn store(self, f: &mut Frame);
}
#[derive(Clone, Copy)]
pub struct LineAt(pub &'static u32);
impl LineHole for LineAt {
    #[inline(always)] fn pos(self) -> Pos { Pos::at(self.0) }
    #[inline(always)] fn store(self, f: &mut Frame) { f.pos = Pos::at(self.0); }
}

#[repr(C)]
pub struct FrameFull {
    pub h: Frame,
    pub cref: *const Cref,
    pub iseq: *const Iseq,
    pub self_blk: *const Block, // for block frames: the block being run
    pub defined_class: Value,
}
impl std::ops::Deref for FrameFull { type Target = Frame; #[inline(always)] fn deref(&self) -> &Frame { &self.h } }
impl std::ops::DerefMut for FrameFull { #[inline(always)] fn deref_mut(&mut self) -> &mut Frame { &mut self.h } }

impl Frame {
    #[inline] pub fn line(&self) -> u32 { self.pos.line() }
    pub fn empty(st: Stk) -> FrameFull {
        FrameFull {
            h: Frame { prev: std::ptr::null_mut(), fp: head(st), me: std::ptr::null(), blk: std::ptr::null(),
                       id: 0, pos: Pos::at(&LINE_0), kind: FK_TOP, visi: VISI_PRIVATE, lite: false, pad: 0 },
            cref: std::ptr::null(), iseq: std::ptr::null(), self_blk: std::ptr::null(), defined_class: NIL,
        }
    }
    #[inline(always)]
    fn full(&self) -> Option<&FrameFull> {
        if self.lite { None } else { Some(unsafe { &*(self as *const Frame as *const FrameFull) }) }
    }
    // a lite frame is a method's (these come from its entry) or a cfunc's (its caller's)
    #[inline] pub fn cref(&self) -> *const Cref {
        match self.full() {
            Some(x) => x.cref,
            None => unsafe { if self.kind == FK_CFUNC { (*self.prev).cref() } else { (*self.me).cref } },
        }
    }
    #[inline] pub fn iseq(&self) -> *const Iseq {
        match self.full() {
            // a frame made by node_block_entry leaves it to the block
            Some(x) if x.iseq.is_null() && !x.self_blk.is_null() => match unsafe { (*x.self_blk).kind } { BlockKind::Iseq(is) => is, _ => std::ptr::null() },
            Some(x) => x.iseq,
            None if self.kind == FK_CFUNC => unsafe { (*self.prev).iseq() },
            None => match unsafe { (*self.me).kind } { MKind::Iseq(is) => is, _ => std::ptr::null() },
        }
    }
    /// The frame of a cfunc called from frame `pf`.
    #[inline(always)]
    pub fn cfunc(pf: *mut Frame, me: *const MethodEntry, blk: *const Block) -> Frame {
        unsafe { Frame { prev: pf, fp: (*pf).fp, me, blk, id: (*pf).id, pos: (*pf).pos, kind: FK_CFUNC, visi: (*pf).visi, lite: true, pad: 0 } }
    }
    /// Replace the lexical scope (`using`): of the frame that owns one.
    pub fn set_cref(&mut self, cr: *const Cref) {
        if !self.lite { unsafe { (*(self as *mut Frame as *mut FrameFull)).cref = cr; } }
        else if self.kind == FK_CFUNC && !self.prev.is_null() { unsafe { (*self.prev).set_cref(cr) } }
    }
    #[inline] pub fn self_blk(&self) -> *const Block { match self.full() { Some(x) => x.self_blk, None => std::ptr::null() } }
    #[inline] pub fn defined_class(&self) -> Value { match self.full() { Some(x) => x.defined_class, None => NIL } }
}

// ---- execution context ---------------------------------------------------------------------

#[repr(C)]
pub struct Ctx {
    pub sp: Sp,           // the stack pointer, as the stack backend keeps it (stack.rs / stack_raw.rs)
    pub frame: *mut Frame,
    pub serial: u64,
    // arguments of node_method_entry.  Not adjacent: written as two words and copied into a frame's
    // (me, blk) pair, they would be read back as one 16-byte load that cannot be store-forwarded.
    pub call_me: *const MethodEntry,
    pub native_limit: usize,
    pub call_blk: *const Block,
    pub stk: StackRef,    // this context's value stack
    pub esc: EscKind,
    pub esc_val: Value,
    pub esc_tag: u64,
    pub esc_tag_v: Value,
    pub errinfo: Value,
    pub last_match: Value,
    pub in_ensure: u32,
    // copy of rt::REDEF, so that baked code tests it from the (noalias) context: rt::note_redefinition, fiber::leave
    pub redef: u32,
    pub tok: crate::heap::Token,   // makes this the heap token (heap.rs)
}

/// What an entry node takes from its caller: in registers when the caller compiled the entry in
/// (EVAL_SPEC!) or calls it directly (a recursive site: as a CallPass), else through c.call_me / c.call_blk.
/// `shared`: the entry runs in its caller's frame (a compiled-in self call of a method without
/// parameters, locals, env or block literal).  `inl`: the entry is compiled into its caller (a constant there).
#[derive(Clone, Copy)]
pub struct CallArgs { pub me: *const MethodEntry, pub blk: *const Block, pub shared: bool, pub inl: bool }
impl CallArgs {
    #[inline(always)] pub fn new(me: *const MethodEntry, blk: *const Block) -> CallArgs { CallArgs { me, blk, shared: false, inl: false } }
    /// From a site that compiled the entry in (EVAL_SPEC): the entry's body is part of the caller's function.
    #[inline(always)] pub fn inline(me: *const MethodEntry, blk: *const Block) -> CallArgs { CallArgs { me, blk, shared: false, inl: true } }
    #[inline(always)] pub fn load(c: &Ctx) -> CallArgs { CallArgs::new(c.call_me, c.call_blk) }
    #[inline(always)] pub fn store(self, c: &mut Ctx) {
        // a shared frame cannot go through the context: spec_shared only offers it to specialized entries
        assert!(!self.shared, "shared frame passed to an entry out of line");
        c.call_me = self.me; c.call_blk = self.blk;
    }
    #[inline(always)] pub fn pass(self) -> CallPass { CallPass { me: self.me, bs: self.blk as usize | self.shared as usize } }
}
/// CallArgs as two words (shared in bit 0 of the block's), for a call that is not inlined: S_<hash>::run.
#[derive(Clone, Copy)]
pub struct CallPass { me: *const MethodEntry, bs: usize }
impl CallPass {
    #[inline(always)] pub fn unpass(self) -> CallArgs { CallArgs { me: self.me, blk: (self.bs & !1) as *const Block, shared: self.bs & 1 != 0, inl: false } }
}

/// Can the self call at a site run its compiled-in callee `p` in the caller's frame?
pub fn spec_shared(recv: NodePtr, argc: usize, p: NodePtr) -> u32 {
    if argc != 0 || p.is_null() || recv.is_null() || !crate::node::node_laid(p) { return 0; }
    if !std::ptr::eq(unsafe { (*recv).head.kind }, &crate::node::KIND_node_self) { return 0; }
    if !std::ptr::eq(unsafe { (*p).head.kind }, &crate::node::KIND_node_method_entry) { return 0; }
    let u = unsafe { &(*(p as *mut crate::node::NodeOf<crate::node::node_method_entry_struct>)).u };
    // 2: a leaf, which cannot raise or look at a frame (no line needed)
    if !shareable(u.nlocals, u.nparams, u.flags) { 0 } else if u.flags & ME_LEAF != 0 { 2 } else { 1 }
}

/// Can a method entry with these (baked) operands run in its caller's frame (spec_shared)?
#[inline(always)]
pub fn shareable(nlocals: u32, nparams: u32, flags: u32) -> bool { nparams == 0 && nlocals == 2 && flags & (ME_ID | ME_ENV) == 0 }

impl Ctx {
    #[inline(always)] pub fn next_serial(&mut self) -> u64 { self.serial += 1; self.serial }
    /// Is any of Integer's basic operators redefined (rt::R_INT_ALL)?
    #[inline(always)] pub fn int_redef(&self) -> bool { self.redef & crate::rt::R_INT_ALL != 0 }
    // SAFETY: H5; the borrow of the context excludes every call that pushes, pops or switches frames
    #[inline(always)] pub fn frame(&self) -> &Frame { unsafe { &*self.frame } }
    #[inline(always)] pub fn frame_mut(&mut self) -> &mut Frame { unsafe { &mut *self.frame } }
    #[inline(always)] pub fn self_val(&self) -> Value { let fp = unsafe { (*self.frame).fp }; fp[0].get() }
}

// ---- global VM state ------------------------------------------------------------------------

#[repr(C)]
pub struct Vm {
    pub c_basic_object: Cell<Value>, pub c_object: Cell<Value>, pub c_module: Cell<Value>, pub c_class: Cell<Value>,
    pub m_kernel: Cell<Value>, pub m_comparable: Cell<Value>, pub m_enumerable: Cell<Value>,
    pub c_nil: Cell<Value>, pub c_true: Cell<Value>, pub c_false: Cell<Value>,
    pub c_numeric: Cell<Value>, pub c_integer: Cell<Value>, pub c_float: Cell<Value>,
    pub c_string: Cell<Value>, pub c_symbol: Cell<Value>, pub c_array: Cell<Value>, pub c_hash: Cell<Value>, pub c_range: Cell<Value>,
    pub c_proc: Cell<Value>, pub c_method: Cell<Value>, pub c_unbound_method: Cell<Value>, pub c_env: Cell<Value>,
    pub c_exception: Cell<Value>, pub c_standard_error: Cell<Value>, pub c_runtime_error: Cell<Value>, pub c_argument_error: Cell<Value>,
    pub c_type_error: Cell<Value>, pub c_name_error: Cell<Value>, pub c_no_method_error: Cell<Value>, pub c_zero_div_error: Cell<Value>,
    pub c_index_error: Cell<Value>, pub c_key_error: Cell<Value>, pub c_range_error: Cell<Value>, pub c_stop_iteration: Cell<Value>,
    pub c_frozen_error: Cell<Value>, pub c_local_jump_error: Cell<Value>, pub c_float_domain_error: Cell<Value>, pub c_io_error: Cell<Value>,
    pub c_eof_error: Cell<Value>, pub c_system_exit: Cell<Value>, pub c_script_error: Cell<Value>, pub c_load_error: Cell<Value>,
    pub c_not_impl_error: Cell<Value>, pub c_syntax_error: Cell<Value>, pub c_system_stack_error: Cell<Value>,
    pub c_no_matching_pattern_error: Cell<Value>, pub c_no_matching_pattern_key_error: Cell<Value>, pub c_fiber_error: Cell<Value>,
    pub c_uncaught_throw_error: Cell<Value>, pub c_regexp_error: Cell<Value>, pub c_encoding_error: Cell<Value>, pub c_math_domain_error: Cell<Value>,
    pub c_system_call_error: Cell<Value>, pub c_interrupt: Cell<Value>, pub c_signal_exception: Cell<Value>, pub c_security_error: Cell<Value>,
    pub c_thread_error: Cell<Value>, pub c_closed_queue_error: Cell<Value>, pub c_no_memory_error: Cell<Value>, pub c_compat_error: Cell<Value>,
    pub c_invalid_byte_seq_error: Cell<Value>, pub c_undef_conv_error: Cell<Value>, pub c_converter_not_found_error: Cell<Value>,
    pub c_struct: Cell<Value>, pub c_regexp: Cell<Value>, pub c_match_data: Cell<Value>, pub c_time: Cell<Value>, pub c_io: Cell<Value>, pub c_file: Cell<Value>,
    pub c_enumerator: Cell<Value>, pub c_lazy: Cell<Value>, pub c_yielder: Cell<Value>, pub c_generator: Cell<Value>, pub c_random: Cell<Value>, pub c_encoding: Cell<Value>,
    pub c_rational: Cell<Value>, pub c_complex: Cell<Value>, pub c_binding: Cell<Value>, pub c_data: Cell<Value>, pub c_set: Cell<Value>,
    pub c_fiber: Cell<Value>, pub c_thread: Cell<Value>, pub c_mutex: Cell<Value>, pub c_queue: Cell<Value>, pub c_arith_seq: Cell<Value>, pub c_chain: Cell<Value>,
    pub c_object_space_weak_map: Cell<Value>, pub c_string_io: Cell<Value>, pub c_dir: Cell<Value>, pub c_stat: Cell<Value>, pub c_monitor: Cell<Value>,
    pub m_errno: Cell<Value>, pub m_math: Cell<Value>, pub m_process: Cell<Value>, pub m_gc: Cell<Value>, pub m_object_space: Cell<Value>, pub m_marshal: Cell<Value>,
    pub m_signal: Cell<Value>, pub m_warning: Cell<Value>, pub m_file_test: Cell<Value>,
    pub c_string_scanner: Cell<Value>, pub c_string_scanner_error: Cell<Value>, pub c_econv: Cell<Value>, pub c_tracepoint: Cell<Value>, pub trace_list: Cell<Value>,
    pub c_location: Cell<Value>,
    /// Random::DEFAULT (the generator of Kernel#rand); a class-field root like the ones above.
    pub default_random: Cell<Value>,
    pub main_obj: Cell<Value>,
    pub stdout: Cell<Value>, pub stderr: Cell<Value>, pub stdin: Cell<Value>,
    pub globals: RefCell<HashMap<Sym, Value>>,
    pub global_aliases: RefCell<HashMap<Sym, Sym>>,
    pub const_serial: Cell<u64>,
    pub hierarchy_serial: Cell<u64>,
    pub mcache: RefCell<HashMap<(u64, Sym), *const MethodEntry>>,
    pub gen_ivars: RefCell<HashMap<usize, Vec<(Sym, Value)>>>,
    pub node_values: RefCell<Vec<Value>>,
    pub crefs: RefCell<Vec<*const Cref>>,
    pub top_cref: Cell<*const Cref>,
    pub loaded_features: Cell<Value>,
    pub load_path: Cell<Value>,
    pub script_name: RefCell<String>,
    pub at_exit: Cell<Value>,
    pub object_ids: RefCell<HashMap<u64, u64>>,
    pub next_object_id: Cell<u64>,
    pub sources: RefCell<Vec<&'static str>>,
    pub verbose: Cell<Value>,
    pub debug_flag: Cell<bool>,
    pub raise_hooks: Cell<bool>,   // debug_flag, or a hook on raise events (update_raise_hooks)
    pub aot_entries: RefCell<Vec<NodePtr>>,
    pub aot_collect: Cell<bool>,
    pub code_store: RefCell<Option<crate::astrust::CodeStore>>,
    pub sd_hits: Cell<usize>,
    pub frozen_lits: RefCell<HashMap<Vec<u8>, Value>>,
    pub symproc_cache: RefCell<HashMap<Sym, Value>>,
    /// Symbol#name: one frozen String per symbol, as in CRuby.
    pub sym_names: RefCell<HashMap<Sym, Value>>,
    /// __callee__ of entries made by alias / define_method(name, method): entry address -> name.
    pub alias_names: RefCell<HashMap<usize, Sym>>,
    pub data_free_hooks: RefCell<Vec<fn(*mut RObj)>>,
    pub pinned: RefCell<Vec<Value>>,
    /// Encoding objects by encoding id (kept alive by `pinned`).
    pub enc_objs: RefCell<Vec<Value>>,
    pub recursive_guard: RefCell<Vec<(u64, Sym)>>,
    pub argv: Cell<Value>,
    pub warn_disabled: Cell<bool>,
    pub exit_status: Cell<i32>,
    pub main_file: RefCell<String>,
    pub dollar_zero: Cell<Value>,
    pub rand_state: Cell<u64>,
    pub fiber_roots: RefCell<Vec<Value>>,
    /// green-thread waits in flight (builtins/blop.rs); made at the first wait
    pub blop: RefCell<Option<Box<crate::builtins::blop::Blop>>>,
    pub ids: MethodIds,
    pub send_cache: Box<[Cell<SendWay>; SEND_WAYS]>,
    pub main_ctx: Cell<*mut Ctx>,   // the main thread's context, for the root scan
}

/// Builtin methods the VM recognizes by identity (set once at boot).
pub struct MethodIds {
    pub bo_eq: Cell<*const MethodEntry>, pub bo_init: Cell<*const MethodEntry>, pub c_new: Cell<*const MethodEntry>,
    pub m_eqq: Cell<*const MethodEntry>, pub kernel_hash: Cell<*const MethodEntry>, pub r_each: Cell<*const MethodEntry>,
}

pub static mut VM: Option<Box<Vm>> = None;

/// The VM, shared: every field is a `Cell` / `RefCell`, so no `&mut Vm` exists.
#[allow(static_mut_refs)]
#[inline(always)]
pub fn vm() -> &'static Vm { unsafe { VM.as_deref().unwrap_unchecked() } }   // SAFETY: written once by heap::boot_vm before any use

// ---- class model --------------------------------------------------------------------------

#[inline]
pub fn class_of(v: Value) -> Value {
    if v.is_heap() { return obj_klass(v); }
    let vm = vm();
    if v.is_fixnum() { vm.c_integer.get() }
    else if v.is_flonum() { vm.c_float.get() }
    else if v.is_sym() { vm.c_symbol.get() }
    else if v == NIL { vm.c_nil.get() }
    else if v == TRUE { vm.c_true.get() }
    else { vm.c_false.get() }
}

/// The non-singleton class of v.
pub fn real_class_of(c: &Ctx, v: Value) -> Value {
    let mut k = class_of(v);
    while c.class(k).is_singleton { k = c.class(k).superclass; }
    k
}

pub fn alloc_class(c: &mut Ctx, meta: Value, superclass: Value, is_module: bool, alloc: AllocKind) -> Value {
    let k = c.alloc(meta, Body::Class(Box::new(ClassData::new(superclass, is_module, alloc))));
    vm().hierarchy_serial.set(vm().hierarchy_serial.get() + 1);
    k
}

/// Create a class with its metaclass (superclass's metaclass as the metaclass's super).
pub fn new_class(c: &mut Ctx, superclass: Value) -> Value {
    let vm = vm();
    let alloc = if superclass == NIL { AllocKind::Object } else { c.class(superclass).alloc };
    let k = alloc_class(c, vm.c_class.get(), superclass, false, alloc);
    if superclass != NIL {
        c.class_mut(superclass).subclasses.push(k);
        let sm = singleton_class(c, superclass);
        let meta = alloc_class(c, vm.c_class.get(), sm, false, AllocKind::Class);
        let md = c.class_mut(meta);
        md.is_singleton = true;
        md.attached = k;
        obj_set_klass(k, meta);
    }
    k
}

pub fn new_module(c: &mut Ctx) -> Value { alloc_class(c, vm().c_module.get(), NIL, true, AllocKind::None) }

pub fn singleton_class(c: &mut Ctx, v: Value) -> Value {
    if !v.is_heap() {
        if v == NIL || v == TRUE || v == FALSE { return class_of(v); }
        return NIL;
    }
    let k = obj_klass(v);
    if c.class(k).is_singleton && c.class(k).attached == v { return k; }
    let vm = vm();
    let superclass = if is_class_or_module(v) && !c.class(v).is_module {
        if c.class(v).is_singleton {
            // metaclass of a metaclass
            k
        } else {
            let sup = c.class(v).superclass;
            if sup == NIL { vm.c_class.get() } else { singleton_class(c, sup) }
        }
    } else { k };
    let a = c.class(superclass).alloc;
    let meta = alloc_class(c, vm.c_class.get(), superclass, false, a);
    c.class_mut(meta).is_singleton = true;
    c.class_mut(meta).attached = v;
    if is_class_or_module(v) && !c.class(v).is_module && !c.class(v).is_singleton {
        // keep the metaclass chain consistent for classes created before this metaclass
    }
    obj_set_klass(meta, vm.c_class.get());
    obj_set_klass(v, meta);
    bump_method_serial();
    vm.mcache.borrow_mut().clear();
    meta
}

pub fn class_name_sym(c: &Ctx, k: Value) -> Option<Value> { c.class(k).name }

pub fn define_class_under(c: &mut Ctx, outer: Value, name: &str, superclass: Value) -> Value {
    let k = new_class(c, superclass);
    name_class(c, k, outer, intern_str(name));
    const_set(c, outer, intern_str(name), k);
    k
}
/// How instances of class `k` are allocated (boot-time setup of builtin classes).
pub fn class_set_alloc(c: &mut Ctx, k: Value, a: AllocKind) { c.class_mut(k).alloc = a; }
pub fn define_class(c: &mut Ctx, name: &str, superclass: Value) -> Value { define_class_under(c, vm().c_object.get(), name, superclass) }
pub fn define_module_under(c: &mut Ctx, outer: Value, name: &str) -> Value {
    let m = new_module(c);
    name_class(c, m, outer, intern_str(name));
    const_set(c, outer, intern_str(name), m);
    m
}
pub fn define_module(c: &mut Ctx, name: &str) -> Value { define_module_under(c, vm().c_object.get(), name) }

pub fn name_class(c: &mut Ctx, k: Value, outer: Value, name: Sym) {
    if c.class(k).name.is_some() { return; }
    let vm = vm();
    let path = if outer == vm.c_object.get() || outer == NIL { sym_str(name) } else {
        match c.class(outer).name { Some(n) => format!("{}::{}", String::from_utf8_lossy(c.str_bytes(n)), sym_str(name)), None => return }
    };
    let s = c.str_new_frozen(path.as_bytes());
    let cd = c.class_mut(k);
    cd.name = Some(s);
    cd.base_name = name;
    cd.lexical_parent = outer;
    // name nested anonymous modules too (still in k's table: rooted while name_class allocates)
    let consts: Vec<(Sym, Value)> = cd.consts.iter().map(|(a, b)| (*a, *b)).collect();
    for (cn, cv) in consts {
        if is_class_or_module(cv) && c.class(cv).name.is_none() { name_class(c, cv, k, cn); }
    }
}

pub fn class_path(c: &Ctx, k: Value) -> String {
    let cd = c.class(k);
    if let Some(n) = cd.name { return String::from_utf8_lossy(c.str_bytes(n)).into_owned(); }
    if cd.is_singleton {
        let a = cd.attached;
        let inner = if is_class_or_module(a) { class_path(c, a) } else { crate::builtins::kernel::any_to_s_str(c, a) };
        return format!("#<Class:{}>", inner);
    }
    format!("#<{}:{:#018x}>", if cd.is_module { "Module" } else { "Class" }, k.raw())
}

/// Include without a context (class setup): must not redefine a basic operation after boot.
pub fn include_module(c: &mut Ctx, klass: Value, m: Value) {
    assert!(!include_module_raw(c, klass, m), "include changed a basic operation without a context");
}
pub fn include_module_c(c: &mut Ctx, klass: Value, m: Value) {
    if include_module_raw(c, klass, m) { c.redef = crate::rt::redef_word(); }
}
fn include_module_raw(c: &mut Ctx, klass: Value, m: Value) -> bool {
    if c.class(klass).includes.contains(&m) { return false; }
    if ancestors(c, klass).contains(&m) { return false; }
    // an included method shadows an inherited one (e.g. NilClass's ==): operator fast paths must notice
    let mut changed = false;
    for &am in ancestors(c, m).iter() {
        let mids: Vec<Sym> = c.class(am).mtbl.keys().copied().collect();
        for mid in mids { changed |= crate::rt::redef_note(c, klass, mid); }
    }
    c.class_mut(klass).includes.push(m);
    bump_hierarchy();
    changed
}
pub fn prepend_module(c: &mut Ctx, klass: Value, m: Value) {
    if c.class(klass).prepends.contains(&m) { return; }
    // a prepended method overrides the class's own: operator fast paths must notice
    for &am in ancestors(c, m).iter() {
        let mids: Vec<Sym> = c.class(am).mtbl.keys().copied().collect();
        for mid in mids { crate::rt::note_redefinition(c, klass, mid); }
    }
    c.class_mut(klass).prepends.push(m);
    bump_hierarchy();
}
pub fn bump_hierarchy() {
    let vm = vm();
    vm.hierarchy_serial.set(vm.hierarchy_serial.get() + 1);
    bump_method_serial();
    vm.const_serial.set(vm.const_serial.get() + 1);
    vm.mcache.borrow_mut().clear();
}

fn module_chain(c: &Ctx, m: Value, out: &mut Vec<Value>) {
    let cd = c.class(m);
    for &p in cd.prepends.iter().rev() { module_chain(c, p, out); }
    out.push(m);
    // in include order: a later include goes in front, minus what an earlier one already brought in
    let mut chain: Vec<Value> = vec![];
    for &i in cd.includes.iter() {
        let mut new = vec![];
        module_chain(c, i, &mut new);
        new.retain(|x| !chain.contains(x));
        new.extend(chain);
        chain = new;
    }
    out.extend(chain);
}

/// Method resolution order (cached per class). An Rc: callers iterate it while using the context.
pub fn ancestors(c: &Ctx, klass: Value) -> std::rc::Rc<[Value]> {
    let ser = vm().hierarchy_serial.get();
    if let Some((s, a)) = &*c.class(klass).ancestors.borrow() { if *s == ser { return a.clone(); } }
    let mut out = vec![];
    let mut k = klass;
    while k != NIL {
        let mut part = vec![];
        module_chain(c, k, &mut part);
        for x in part {
            if !out.contains(&x) { out.push(x); }
        }
        k = c.class(k).superclass;
    }
    let a: std::rc::Rc<[Value]> = out.into();
    *c.class(klass).ancestors.borrow_mut() = Some((ser, a.clone()));
    a
}

pub fn define_method_me(c: &mut Ctx, klass: Value, name: Sym, me: *const MethodEntry) {
    c.class_mut(klass).mtbl.insert(name, me);
    let vm = vm();
    bump_method_serial();
    vm.mcache.borrow_mut().clear();
}

pub fn define_cfunc(c: &mut Ctx, klass: Value, name: &str, f: CFunc, arity: i32) {
    let s = intern_str(name);
    define_method_me(c, klass, s, new_me(s, klass, VISI_PUBLIC, MKind::Cfunc(f, arity)));
}
pub fn define_private_cfunc(c: &mut Ctx, klass: Value, name: &str, f: CFunc, arity: i32) {
    let s = intern_str(name);
    define_method_me(c, klass, s, new_me(s, klass, VISI_PRIVATE, MKind::Cfunc(f, arity)));
}
pub fn define_singleton_cfunc(c: &mut Ctx, v: Value, name: &str, f: CFunc, arity: i32) {
    let k = singleton_class(c, v);
    define_cfunc(c, k, name, f, arity);
}
pub fn define_module_function(c: &mut Ctx, m: Value, name: &str, f: CFunc, arity: i32) {
    define_private_cfunc(c, m, name, f, arity);
    define_singleton_cfunc(c, m, name, f, arity);
}

pub fn search_method(c: &Ctx, klass: Value, mid: Sym) -> *const MethodEntry {
    for &k in ancestors(c, klass).iter() {
        if let Some(&me) = c.class(k).mtbl.get(&mid) {
            if let MKind::ZSuper = unsafe { (*me).kind } { continue; }
            return me;
        }
    }
    std::ptr::null()
}

/// Lookup with the global method cache; undefined entries come back null.
#[inline]
pub fn lookup_method(c: &Ctx, klass: Value, mid: Sym) -> *const MethodEntry {
    let vm = vm();
    if let Some(&me) = vm.mcache.borrow().get(&(klass.raw(), mid)) { return me; }
    let mut me = search_method(c, klass, mid);
    if !me.is_null() { if let MKind::Undef = unsafe { (*me).kind } { me = std::ptr::null(); } }
    vm.mcache.borrow_mut().insert((klass.raw(), mid), me);
    me
}

// ---- refinements ----------------------------------------------------------------------------
// A refinement is a Module (ClassData.refined, attached = the refined class) outside every
// ancestry.  `using` puts it on the caller's cref; call sites under that cref look there first.

/// Set by the first `using`: call-site lookups consult the frame's cref from then on.
pub static REFINEMENTS_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[inline(always)] pub fn refinements_on() -> bool { REFINEMENTS_ON.load(std::sync::atomic::Ordering::Relaxed) }

/// The refinements active for a cref (Array, later entries win) or NIL.
pub fn cref_refinements(mut cr: *const Cref) -> Value {
    unsafe {
        while !cr.is_null() {
            if (*cr).refinements != NIL { return (*cr).refinements; }
            cr = (*cr).prev;
        }
    }
    NIL
}

/// Method lookup at a call site whose cref may have refinements activated.
#[inline(never)]
pub fn lookup_method_refined(c: &Ctx, cref: *const Cref, klass: Value, mid: Sym) -> *const MethodEntry {
    let refs = cref_refinements(cref);
    if refs == NIL { return lookup_method(c, klass, mid); }
    let n = c.ary(refs).len();
    for &k in ancestors(c, klass).iter() {
        for i in (0..n).rev() {
            let r = c.ary(refs)[i];
            if c.class(r).attached != k { continue; }
            let me = search_method(c, r, mid);
            if me.is_null() { continue; }
            // `undef` in the refine block hides the method
            return if matches!(unsafe { (*me).kind }, MKind::Undef) { std::ptr::null() } else { me };
        }
        if c.class(k).mtbl.contains_key(&mid) { break; }
    }
    lookup_method(c, klass, mid)
}

/// Next method after `owner` in klass's ancestors (super).
pub fn super_method(c: &Ctx, klass: Value, owner: Value, mid: Sym) -> *const MethodEntry {
    let anc = ancestors(c, klass);
    let mut found = false;
    // super from a refined method: the refined class's own method comes next
    let (owner, refined) = if c.class(owner).refined { (c.class(owner).attached, true) } else { (owner, false) };
    for &k in anc.iter() {
        if refined && k == owner { found = true; }
        if found {
            if let Some(&me) = c.class(k).mtbl.get(&mid) {
                if let MKind::ZSuper = unsafe { (*me).kind } { continue; }
                if let MKind::Undef = unsafe { (*me).kind } { return std::ptr::null(); }
                return me;
            }
        } else if k == owner { found = true; }
    }
    std::ptr::null()
}

pub fn respond_to(c: &Ctx, v: Value, mid: Sym, include_private: bool) -> bool {
    let me = lookup_method(c, class_of(v), mid);
    if me.is_null() { return false; }
    include_private || unsafe { (*me).visi } == VISI_PUBLIC
}

// ---- instance variables --------------------------------------------------------------------

pub fn ivar_get(c: &Ctx, v: Value, name: Sym) -> Value {
    if v.is_heap() {
        if let Some(ivs) = c.obj_ivars(v) {
            let k = real_class_of(c, v);
            if let Some(&i) = c.class(k).ivar_index.get(&name) {
                if let Some(x) = ivs.get(i as usize) { if x != UNDEF { return x; } }
            }
            return NIL;
        }
        if obj_flags(v) & FL_EXIVAR != 0 {
            if let Some(iv) = vm().gen_ivars.borrow().get(&(v.raw() as usize)) {
                for (n, x) in iv { if *n == name { return *x; } }
            }
        }
    }
    NIL
}

pub fn ivar_defined(c: &Ctx, v: Value, name: Sym) -> bool {
    if v.is_heap() {
        if let Some(ivs) = c.obj_ivars(v) {
            let k = real_class_of(c, v);
            if let Some(&i) = c.class(k).ivar_index.get(&name) {
                return ivs.get(i as usize).map(|x| x != UNDEF).unwrap_or(false);
            }
            return false;
        }
        if let Some(iv) = vm().gen_ivars.borrow().get(&(v.raw() as usize)) { return iv.iter().any(|(n, _)| *n == name); }
    }
    false
}

pub fn ivar_index_for(c: &mut Ctx, k: Value, name: Sym) -> u32 {
    let cd = c.class_mut(k);
    if let Some(&i) = cd.ivar_index.get(&name) { return i; }
    let i = cd.ivar_names.len() as u32;
    cd.ivar_names.push(name);
    cd.ivar_index.insert(name, i);
    i
}

pub fn ivar_set(c: &mut Ctx, v: Value, name: Sym, val: Value) -> RV {
    if !v.is_heap() || obj_flags(v) & FL_FROZEN != 0 { return Err(raise_frozen(c, v)); }
    ivar_set_raw(c, v, name, val);
    Ok(val)
}

pub fn ivar_set_raw(c: &mut Ctx, v: Value, name: Sym, val: Value) {
    if is_object(v) {
        let k = real_class_of(c, v);
        let i = ivar_index_for(c, k, name) as usize;
        let n = c.class(k).ivar_names.len();
        let ivs = c.ivars_mut(v);
        // room for every ivar the class knows: later sets of this object hit
        if ivs.len() <= i { ivs.resize(n); }
        ivs.set(i, val);
        return;
    }
    ivar_set_generic(v, name, val);
}

/// An ivar of an object that is not a plain object (a class, a String, ..): the generic table and
/// the header only, so no context is needed (boot-time setup uses it).
pub fn ivar_set_generic(v: Value, name: Sym, val: Value) {
    assert!(!is_object(v), "ivar_set_generic on a plain object");
    obj_set_flags(v, FL_EXIVAR);
    let mut g = vm().gen_ivars.borrow_mut();
    let e = g.entry(v.raw() as usize).or_default();
    for (n, x) in e.iter_mut() { if *n == name { *x = val; return; } }
    e.push((name, val));
}

pub fn ivar_remove(c: &mut Ctx, v: Value, name: Sym) -> Option<Value> {
    if !v.is_heap() { return None; }
    if is_object(v) {
        let k = real_class_of(c, v);
        if let Some(&i) = c.class(k).ivar_index.get(&name) {
            let ivs = c.ivars_mut(v);
            if let Some(old) = ivs.get(i as usize) { if old != UNDEF { ivs.set(i as usize, UNDEF); return Some(old); } }
        }
        return None;
    }
    if let Some(iv) = vm().gen_ivars.borrow_mut().get_mut(&(v.raw() as usize)) {
        if let Some(p) = iv.iter().position(|(n, _)| *n == name) { return Some(iv.remove(p).1); }
    }
    None
}

/// (name, value) of the set ivars in definition order.
pub fn ivar_list(c: &Ctx, v: Value) -> Vec<(Sym, Value)> {
    let mut out = vec![];
    if !v.is_heap() { return out; }
    if let Some(ivs) = c.obj_ivars(v) {
        let k = real_class_of(c, v);
        let names = &c.class(k).ivar_names;
        for (i, x) in ivs.iter().enumerate() { if x != UNDEF { out.push((names[i], x)); } }
        return out;
    }
    if let Some(iv) = vm().gen_ivars.borrow().get(&(v.raw() as usize)) { out.extend(iv.iter().cloned()); }
    out
}

// ---- constants -----------------------------------------------------------------------------

pub fn const_set(c: &mut Ctx, klass: Value, name: Sym, val: Value) {
    let cd = c.class_mut(klass);
    if cd.consts.insert(name, val).is_none() { cd.const_order.push(name); }
    cd.autoload.remove(&name);
    vm().const_serial.set(vm().const_serial.get() + 1);
    if is_class_or_module(val) && c.class(val).name.is_none() { name_class(c, val, klass, name); }
}

pub fn const_get_at(c: &Ctx, klass: Value, name: Sym) -> Option<Value> { c.class(klass).consts.get(&name).copied() }

pub fn const_get_inherit(c: &mut Ctx, klass: Value, name: Sym) -> Result<Option<Value>, Esc> {
    for &k in ancestors(c, klass).iter() {
        if let Some(v) = const_get_at(c, k, name) { return Ok(Some(v)); }
        if let Some(&path) = c.class(k).autoload.get(&name) { return do_autoload(c, k, name, path); }
    }
    if c.class(klass).is_module {
        let o = vm().c_object.get();
        if let Some(v) = const_get_at(c, o, name) { return Ok(Some(v)); }
        if let Some(&path) = c.class(o).autoload.get(&name) { return do_autoload(c, o, name, path); }
    }
    Ok(None)
}

fn do_autoload(c: &mut Ctx, k: Value, name: Sym, path: Value) -> Result<Option<Value>, Esc> {
    c.class_mut(k).autoload.remove(&name);
    crate::builtins::kernel::require_feature(c, path, false)?;
    Ok(const_get_at(c, k, name))
}

/// Lexical scope first (not including the top level), then the cref class's ancestors, then Object.
pub fn const_lookup(c: &mut Ctx, cref: *const Cref, name: Sym) -> RV {
    let vm = vm();
    let mut cr = cref;
    unsafe {
        while !cr.is_null() && !(*cr).prev.is_null() {
            let k = (*cr).klass;
            if let Some(v) = const_get_at(c, k, name) { return Ok(v); }
            if let Some(&path) = c.class(k).autoload.get(&name) { if let Some(v) = do_autoload(c, k, name, path)? { return Ok(v); } }
            cr = (*cr).prev;
        }
    }
    let start = if cref.is_null() { vm.c_object.get() } else { unsafe { (*cref).klass } };
    if let Some(v) = const_get_inherit(c, start, name)? { return Ok(v); }
    if let Some(v) = const_get_inherit(c, vm.c_object.get(), name)? { return Ok(v); }
    const_missing(c, start, name)
}

pub fn const_missing(c: &mut Ctx, klass: Value, name: Sym) -> RV {
    let me = lookup_method(c, class_of(klass), s::const_missing);
    if !me.is_null() && !matches!(unsafe { (*me).kind }, MKind::Cfunc(..)) {
        return funcall(c, klass, s::const_missing, &[Value::sym(name)], std::ptr::null());
    }
    let msg = if klass == vm().c_object.get() { format!("uninitialized constant {}", sym_str(name)) }
              else { format!("uninitialized constant {}::{}", class_path(c, klass), sym_str(name)) };
    let e = make_name_error(c, vm().c_name_error.get(), &msg, Value::sym(name), klass)?;
    Err(raise_exc(c, e))
}

// ---- object allocation helpers ------------------------------------------------------------

#[inline]
pub fn new_object(c: &mut Ctx, klass: Value) -> Value {
    // slots for every ivar an instance of the class has had: initialize never grows it
    let n = c.class(klass).ivar_names.len();
    c.alloc_with(klass, move || Body::Object(crate::ivbuf::IvBuf::filled(n)))
}

pub fn object_id(v: Value) -> u64 {
    if v.is_fixnum() { return (v.as_fix() as u64).wrapping_mul(2).wrapping_add(1); }
    if !v.is_heap() { return v.raw(); }
    let vm = vm();
    let key = v.raw();
    if let Some(&id) = vm.object_ids.borrow().get(&key) { return id; }
    vm.next_object_id.set(vm.next_object_id.get() + 8);
    let id = vm.next_object_id.get();
    vm.object_ids.borrow_mut().insert(key, id);
    id
}

pub fn on_free_data(p: *mut RObj) {
    let vm = vm();
    let hooks = vm.data_free_hooks.borrow().clone();
    for f in hooks { f(p); }
}

// ---- exceptions -----------------------------------------------------------------------------

#[inline]
pub fn raise_exc(c: &mut Ctx, e: Value) -> Esc {
    if ivar_get(c, e, s::bt) == NIL && is_exception(c, e) {
        let bt = make_backtrace_lazy(c, 0);
        ivar_set_raw(c, e, s::bt, bt);
    }
    if is_exception(c, e) && c.errinfo != NIL && c.errinfo != e && ivar_get(c, e, s::cause) == NIL && !ivar_defined(c, e, s::cause) {
        ivar_set_raw(c, e, s::cause, c.errinfo);
    }
    // one test here (raise_exc is inlined everywhere): $DEBUG or a raise hook
    if vm().raise_hooks.get() && is_exception(c, e) { raise_hooks(c, e); }
    c.esc = EscKind::Raise;
    c.esc_val = e;
    Esc
}

#[cold]
#[inline(never)]
fn raise_hooks(c: &mut Ctx, e: Value) {
    if vm().debug_flag.get() { debug_report(c, e); }
    if trace_mask() & EV_RAISE != 0 { crate::builtins::tracepoint::trace_raise(c, e); }
}
/// Vm::raise_hooks after $DEBUG or the trace mask changed.
pub fn update_raise_hooks() { let v = vm(); v.raise_hooks.set(v.debug_flag.get() || trace_mask() & EV_RAISE != 0); }

/// $DEBUG: "Exception 'X' at file:line - message" on every raise.
#[cold]
fn debug_report(c: &mut Ctx, e: Value) {
    let at = match backtrace_lines(c, 0).first() { Some(l) => l.split(":in ").next().unwrap_or("").to_string(), None => String::new() };
    let (esc, val) = (c.esc, c.esc_val);
    let msg = crate::builtins::exception::exc_message_str(c, e).unwrap_or_default();
    c.esc = esc; c.esc_val = val;
    eprintln!("Exception '{}' at {} - {}", class_path(c, real_class_of(c, e)), at, msg);
}

pub fn is_exception(c: &Ctx, v: Value) -> bool {
    v.is_heap() && is_kind_of(c, v, vm().c_exception.get())
}

pub fn is_kind_of(c: &Ctx, v: Value, k: Value) -> bool {
    let kl = class_of(v);
    if kl == k { return true; }
    ancestors(c, kl).contains(&k)
}

pub fn make_exception(c: &mut Ctx, klass: Value, msg: &str) -> Value {
    let e = new_object(c, klass);
    let m = c.str_new(msg.as_bytes());
    ivar_set_raw(c, e, s::mesg, m);
    e
}

pub fn make_name_error(c: &mut Ctx, klass: Value, msg: &str, name: Value, recv: Value) -> RV {
    let e = make_exception(c, klass, msg);
    ivar_set_raw(c, e, s::name, name);
    ivar_set_raw(c, e, s::receiver, recv);
    let _ = c;
    Ok(e)
}

pub fn raise_msg(c: &mut Ctx, klass: Value, msg: &str) -> Esc {
    let e = make_exception(c, klass, msg);
    raise_exc(c, e)
}

pub fn err<T>(c: &mut Ctx, klass: Value, msg: impl AsRef<str>) -> Result<T, Esc> { Err(raise_msg(c, klass, msg.as_ref())) }
pub fn arg_err<T>(c: &mut Ctx, msg: impl AsRef<str>) -> Result<T, Esc> { err(c, vm().c_argument_error.get(), msg) }
pub fn type_err<T>(c: &mut Ctx, msg: impl AsRef<str>) -> Result<T, Esc> { err(c, vm().c_type_error.get(), msg) }

pub fn argc_err<T>(c: &mut Ctx, given: usize, min: i32, max: i32) -> Result<T, Esc> {
    let exp = if max < 0 { format!("{}+", min) } else if min == max { format!("{}", min) } else { format!("{}..{}", min, max) };
    arg_err(c, format!("wrong number of arguments (given {}, expected {})", given, exp))
}

#[inline]
pub fn check_argc(c: &mut Ctx, args: &[Value], min: usize, max: usize) -> Result<(), Esc> {
    if args.len() < min || args.len() > max { return argc_err(c, args.len(), min as i32, max as i32); }
    Ok(())
}

pub fn raise_frozen(c: &mut Ctx, v: Value) -> Esc {
    // rb_error_frozen_object: an #inspect that modifies the object gives " ..." (rb_exec_recursive)
    let tag = crate::symbol::intern(b"__frozen_inspect__");
    let insp = if crate::legacy::recursion_enter(v, tag) {
        let r = crate::builtins::kernel::inspect_str(c, v);
        crate::legacy::recursion_leave(v, tag);
        match r { Ok(s) => s, Err(e) => return e }
    } else { " ...".into() };
    let msg = format!("can't modify frozen {}: {}", class_path(c, real_class_of(c, v)), insp);
    let e = make_exception(c, vm().c_frozen_error.get(), &msg);
    ivar_set_raw(c, e, s::receiver, v);
    raise_exc(c, e)
}

#[inline]
pub fn check_frozen(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    if is_frozen(v) { return Err(raise_frozen(c, v)); }
    Ok(())
}

pub fn frame_label(c: &Ctx, f: &Frame) -> String { frame_label_of(c, f.kind, f.me, f.iseq()) }

fn frame_label_of(c: &Ctx, kind: u8, fme: *const MethodEntry, iseq: *const Iseq) -> String {
    unsafe {
        if kind == FK_CFUNC {
            let k = real_class_of_for_label(c, NIL, fme);
            return format!("{}{}", k, sym_str((*fme).name));
        }
        if iseq.is_null() { return "<main>".into(); }
        let is = &*iseq;
        match is.kind {
            IseqKind::Method => {
                if !fme.is_null() {
                    let me = &*fme;
                    let owner = me.owner;
                    let pre = method_owner_label(c, owner);
                    return format!("{}{}", pre, sym_str(me.name));
                }
                is.info.label.clone()
            }
            IseqKind::Block | IseqKind::Lambda if !fme.is_null() => {
                // "block in m" -> "block in Owner#m" when the block's home is that method
                let me = &*fme;
                let l = &is.info.label;
                let mname = sym_str(me.name);
                match l.find(" in ") {
                    Some(p) if l[p + 4..] == *mname => format!("{}{}{}", &l[..p + 4], method_owner_label(c, me.owner), mname),
                    _ => l.clone(),
                }
            }
            _ => is.info.label.clone(),
        }
    }
}

fn real_class_of_for_label(c: &Ctx, recv: Value, me: *const MethodEntry) -> String {
    let owner = if me.is_null() { real_class_of(c, recv) } else { unsafe { (*me).owner } };
    let cd = c.class(owner);
    if cd.is_singleton {
        if is_class_or_module(cd.attached) { return format!("{}.", class_path(c, cd.attached)); }
        return String::new();
    }
    format!("{}#", class_path(c, owner))
}

fn method_owner_label(c: &Ctx, owner: Value) -> String {
    let cd = c.class(owner);
    if cd.is_singleton {
        if is_class_or_module(cd.attached) { return format!("{}.", class_path(c, cd.attached)); }
        return String::new();
    }
    if cd.name.is_none() { return String::new(); }
    if owner == vm().c_object.get() { return "Object#".into(); }
    format!("{}#", class_path(c, owner))
}

pub fn frame_file(f: &Frame) -> &'static str {
    unsafe { if f.iseq().is_null() { "-" } else { (*f.iseq()).info.file } }
}

/// Backtrace as an Array of "file:line:in 'label'" strings.
pub fn make_backtrace(c: &mut Ctx, skip: usize) -> Value {
    let lines = backtrace_lines(c, skip);
    lines_to_array(c, lines)
}
fn lines_to_array(c: &mut Ctx, lines: Vec<String>) -> Value {
    let a = c.ary_new_capa(lines.len());
    for l in lines { let s = c.str_new(l.as_bytes()); c.ary_push(a, s); }
    a
}

/// One frame of a captured backtrace: what frame_label needs, plus the file/line.
#[derive(Clone, Copy)]
pub struct RawFrame { kind: u8, me: *const MethodEntry, iseq: *const Iseq, file: &'static str, line: u32 }

/// An exception's backtrace kept as raw frames: formatted when first read (most never are).
pub struct RawBt { frames: Vec<RawFrame>, skip: usize }
impl RData for RawBt {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { for f in &self.frames { visit_me(m, f.me); } }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// The backtrace at this point, unformatted (stored in an exception's `bt`).
pub fn make_backtrace_lazy(c: &mut Ctx, skip: usize) -> Value {
    let frames = capture_frames(c);
    c.alloc(vm().c_object.get(), Body::Data(Box::new(RawBt { frames, skip })))
}

/// An exception's backtrace (Array of String, or nil), formatting a captured one in place.
pub fn exc_bt(c: &mut Ctx, e: Value) -> Value {
    let v = ivar_get(c, e, s::bt);
    let Some(raw) = c.data::<RawBt>(v) else { return v };
    let lines = format_frames(c, &raw.frames, raw.skip);
    let a = lines_to_array(c, lines);

    ivar_set_raw(c, e, s::bt, a);
    a
}

fn capture_frames(c: &Ctx) -> Vec<RawFrame> {
    let mut frames: Vec<RawFrame> = Vec::with_capacity(16);
    let mut f = c.frame;
    unsafe {
        while !f.is_null() {
            let fr = &*f;
            frames.push(RawFrame { kind: fr.kind, me: fr.me, iseq: fr.iseq(), file: frame_file(fr), line: fr.line() });
            f = fr.prev;
        }
    }
    // cfunc frames take the file/line of the nearest Ruby caller
    let mut last_file: &'static str = "-";
    let mut last_line = 0;
    for fr in frames.iter_mut().rev() {
        if fr.kind == FK_CFUNC { fr.file = last_file; fr.line = last_line; } else { last_file = fr.file; last_line = fr.line; }
    }
    frames
}

fn format_frames(c: &Ctx, frames: &[RawFrame], skip: usize) -> Vec<String> { format_frames_n(c, frames, skip, usize::MAX).unwrap_or_default() }
/// Lines skip..skip+n only: the others are not formatted (caller(0, 1) deep in a stack).
/// None: fewer than `skip` frames (caller's nil).
fn format_frames_n(c: &Ctx, frames: &[RawFrame], skip: usize, n: usize) -> Option<Vec<String>> {
    let mut out = vec![];
    let mut seen = 0usize;
    unsafe {
        for fr in frames {
            if seen >= skip && out.len() >= n { break; }
            if fr.kind != FK_CFUNC && fr.iseq.is_null() { continue; }
            // CRuby's send/__send__ and Proc#call/yield/===/[] do not push a frame
            if fr.kind == FK_CFUNC && (fr.me.is_null() == false && ((*fr.me).name == s::send || (*fr.me).name == s::__send__)) { continue; }
            if fr.kind == FK_CFUNC && fr.me == vm().ids.c_new.get() { continue; }   // CRuby 4.0 has no Class#new frame
            if fr.kind == FK_CFUNC && !fr.me.is_null() && (*fr.me).owner == vm().c_proc.get() && matches!(sym_str((*fr.me).name).as_ref(), "call" | "yield" | "===" | "[]") { continue; }
            seen += 1;
            if seen <= skip { continue; }
            out.push(format!("{}:{}:in '{}'", fr.file, fr.line, frame_label_of(c, fr.kind, fr.me, fr.iseq)));
        }
    }
    if seen < skip { None } else { Some(out) }
}

pub fn backtrace_lines(c: &Ctx, skip: usize) -> Vec<String> { format_frames(c, &capture_frames(c), skip) }
pub fn backtrace_lines_n(c: &Ctx, skip: usize, n: Option<usize>) -> Option<Vec<String>> {
    if n == Some(0) { return Some(vec![]); }   // CRuby: [] before the range is checked
    format_frames_n(c, &capture_frames(c), skip, n.unwrap_or(usize::MAX))
}

// ---- calls -----------------------------------------------------------------------------------

pub const CF_FCALL: u32 = 1;
pub const CF_VCALL: u32 = 2;
pub const CF_KW: u32 = 4;          // last arg is a keyword Hash
pub const CF_SAFE: u32 = 8;
pub const CF_SUPER: u32 = 16;
pub const CF_KW_SPLAT: u32 = 32;   // keywords came from **h (an empty hash passes nothing)

#[inline(always)]
pub fn stack_check(c: &mut Ctx, need: usize) -> Result<(), Esc> {
    let probe = 0u8;
    let addr = &probe as *const u8 as usize;
    if !c.has_room(need) || addr < c.native_limit {
        return Err(raise_stack_overflow(c));
    }
    Ok(())
}

#[cold]
#[inline(never)]
pub fn raise_stack_overflow(c: &mut Ctx) -> Esc {
    let e = make_exception(c, vm().c_system_stack_error.get(), "stack level too deep");
    let bt = c.ary_new();
    ivar_set_raw(c, e, s::bt, bt);
    raise_exc(c, e)
}

/// Call a method from Rust.  args are copied into the VM stack.
pub fn funcall(c: &mut Ctx, recv: Value, mid: Sym, args: &[Value], blk: *const Block) -> RV {
    funcall_flags(c, recv, mid, args, blk, CF_FCALL)
}

pub fn funcall_flags(c: &mut Ctx, recv: Value, mid: Sym, args: &[Value], blk: *const Block, flags: u32) -> RV {
    stack_check(c, args.len() + 2)?;   // native depth too: cfunc -> funcall recursion uses little VM stack
    let base = stage(c, 2 + args.len())?;
    base[0].set(recv);
    base[1].set(NIL);
    put(base, 2, args);
    let klass = class_of(recv);
    let me = lookup_method(c, klass, mid);
    let r = if me.is_null() {
        method_missing(c, recv, mid, base, args.len() as u32, blk, flags)
    } else {
        unsafe {
            if (*me).visi != VISI_PUBLIC && flags & CF_FCALL == 0 {
                visibility_error(c, me, recv, base, args.len() as u32, blk, flags)
            } else {
                invoke(c, me, recv, base, args.len() as u32, blk, flags)
            }
        }
    };
    c.set_sp(base, 0);
    r
}

/// send / public_send from a scope with refinements: a refined method is called directly.
pub fn funcall_refined(c: &mut Ctx, cref: *const Cref, recv: Value, mid: Sym, args: &[Value], blk: *const Block, flags: u32) -> Option<RV> {
    // null: the current frame's scope (a cfunc frame carries its caller's cref)
    let cref = if cref.is_null() { unsafe { (*c.frame).cref() } } else { cref };
    let me = lookup_method_refined(c, cref, class_of(recv), mid);
    if me.is_null() || !c.class(unsafe { (*me).owner }).refined { return None; }
    if unsafe { (*me).visi } != VISI_PUBLIC && flags & CF_FCALL == 0 { return None; }
    let base = match stage(c, 2 + args.len()) { Ok(b) => b, Err(e) => return Some(Err(e)) };
    base[0].set(recv);
    base[1].set(NIL);
    put(base, 2, args);
    let r = invoke(c, me, recv, base, args.len() as u32, blk, flags);
    c.set_sp(base, 0);
    Some(r)
}

/// Public call from Rust (respects visibility).
pub fn funcall_public(c: &mut Ctx, recv: Value, mid: Sym, args: &[Value], blk: *const Block) -> RV {
    funcall_flags(c, recv, mid, args, blk, 0)
}

pub fn funcall_kw(c: &mut Ctx, recv: Value, mid: Sym, args: &[Value], blk: *const Block, kw: bool) -> RV {
    funcall_flags(c, recv, mid, args, blk, CF_FCALL | if kw { CF_KW } else { 0 })
}

/// Call on a staged frame: base[0] = recv, base[2..2+argc] = args; sp is above them.
#[inline(always)]
pub fn call_cached(c: &mut Ctx, cc: &mut CallCache, mid: Sym, base: Stk, argc: u32, blk: *const Block, flags: u32) -> RV {
    let recv = base[0].get();
    let klass = class_of(recv);
    let hit = cc.klass == klass.raw() && cc.serial == method_serial();
    #[cfg(feature = "nodeprof")] if hit && cc.cfunc.is_some() { crate::nodeprof::hit_cfunc(c, klass, mid, argc); }
    if hit {
        if let Some(r) = cc_fast(c, cc, recv, base, argc, blk, flags) { return r; }
    }
    call_slow(c, cc, mid, base, argc, blk, flags, hit)
}

/// The inline fast paths of a cache entry that matches recv (see fill_cc); None: use invoke.
#[inline(always)]
fn cc_fast(c: &mut Ctx, cc: &CallCache, recv: Value, base: Stk, argc: u32, blk: *const Block, flags: u32) -> Option<RV> {
    // fast_argc matches only when cc.fast is a method entry node
    if argc == cc.fast_argc && flags & CF_KW == 0 {
        #[cfg(feature = "nodeprof")] crate::nodeprof::hit("site: entry");
        c.call_me = cc.me as *const MethodEntry; c.call_blk = blk;
        let r = run_entry(c, cc.fast as NodePtr, base, argc);
        c.set_sp(base, 0);
        return Some(r);
    }
    if cc.fast != 0 && flags & CF_KW == 0 {
        match cc.fast & 3 {
            // tag 0 is also a method entry called with the wrong argc (fast_argc set): not ours
            0 if cc.fast_argc == u32::MAX => {
                let ni = unsafe { &*(cc.fast as *const NewInfo) };
                if argc == ni.init_argc {
                    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("site: new");
                    let o = new_object(c, ni.klass);
                    base[0].set(o);   // self of initialize: keeps o alive and gives it back
                    c.call_me = ni.init_me; c.call_blk = blk;
                    let r = run_entry(c, ni.init_entry, base, argc);
                    c.set_sp(base, 0);
                    return Some(match r { Ok(_) => Ok(base[0].get()), Err(e) => Err(e) });
                }
            }
            1 if argc == 0 => {
                // attr_reader on an Object whose class assigned the ivar index
                // (is_heap first: the accessors' own check would add a panic path to every call site)
                if let Some(ivs) = if recv.is_heap() { c.obj_ivars(recv) } else { None } {
                    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("site: attr_reader");
                    let v = ivs.get(cc.fast >> 2).unwrap_or(UNDEF);
                    c.set_sp(base, 0);
                    return Some(Ok(if v == UNDEF { NIL } else { v }));
                }
            }
            2 if argc == 1 => {
                if recv.is_heap() && obj_flags(recv) & FL_FROZEN == 0 {
                    if let Some(ivs) = c.obj_ivars_mut(recv) {
                        let i = cc.fast >> 2;
                        let v = base[2].get();
                        #[cfg(feature = "nodeprof")] crate::nodeprof::hit("site: attr_writer");
                        if ivs.set(i, v) { c.set_sp(base, 0); return Some(Ok(v)); }
                    }
                }
            }
            3 if argc == 1 => {
                // BasicObject#== / equal?: identity
                let r = Value::bool(recv == base[2].get());
                c.set_sp(base, 0);
                return Some(Ok(r));
            }
            _ => {}
        }
    }
    if let (Some(f), true) = (cc.cfunc, flags & CF_KW == 0 && (cc.arity < 0 || cc.arity == argc as i32)) {
        if cc.bi >= BI_SEND { if let Some(r) = send_fast(c, recv, base, argc, blk) { return Some(r); } }
        #[cfg(feature = "nodeprof")] crate::nodeprof::hit(if blk.is_null() { "site: cfunc" } else { "site: cfunc +blk" });
        let pf = c.frame;
        let mut fr = Frame::cfunc(pf, cc.me as *const MethodEntry, blk);
        c.frame = &mut fr;
        let r = with_args(base, argc as usize, |a| f(c, recv, a, blk));
        c.frame = pf;
        c.set_sp(base, 0);
        return Some(r);
    }
    None
}

// What a call site's cache held when its code was baked: only that case is inline in the baked
// code (every case inline at every site made it too large for the instruction cache).
pub const SITE_INTERP: u32 = 0;   // not baked: call_cached, all cases
pub const SITE_ENTRY: u32 = 1;
pub const SITE_ATTR_R: u32 = 2;
pub const SITE_ATTR_W: u32 = 3;
pub const SITE_CFUNC: u32 = 4;
pub const SITE_NEW: u32 = 6;      // Klass.new with a plain initialize (NewInfo)
pub const SITE_OTHER: u32 = 5;    // never run, Klass.new, keywords, ...: out of line
pub const SITE_SEND: u32 = 7;     // Kernel#send / __send__ (send_fast)

pub const SITE_IMM: u32 = 16;      // flag: the cached receiver class is an immediate's (Integer, Symbol, nil, ...)

pub fn site_kind(cc: &CallCache, argc: usize, flags: u32) -> u32 {
    if flags & CF_KW != 0 || cc.klass == 0 { return SITE_OTHER; }
    let v = vm();
    let imm = [v.c_integer.get(), v.c_float.get(), v.c_symbol.get(), v.c_nil.get(), v.c_true.get(), v.c_false.get()].iter().any(|k| k.raw() == cc.klass);
    let k = if cc.fast_argc as usize == argc { SITE_ENTRY }
        else if cc.fast != 0 { match cc.fast & 3 { 1 if argc == 0 => SITE_ATTR_R, 2 if argc == 1 => SITE_ATTR_W,
            0 if cc.fast_argc == u32::MAX => SITE_NEW, _ => return SITE_OTHER } }
        else if cc.cfunc.is_some() { if cc.bi >= BI_SEND && argc >= 1 { SITE_SEND } else { SITE_CFUNC } } else { return SITE_OTHER };
    k | if imm { SITE_IMM } else { 0 }
}

#[inline(never)]
pub fn call_cached_out(c: &mut Ctx, cc: &mut CallCache, mid: Sym, base: Stk, argc: u32, blk: *const Block, flags: u32) -> RV {
    call_cached(c, cc, mid, base, argc, blk, flags)
}

#[inline(never)]
fn class_of_imm(v: Value) -> u64 { class_of(v).raw() }

/// Does the site's cache hold recv's class?  `kind` (baked) says which receivers the site saw,
/// so only that test is in the baked code.
#[inline(always)]
pub fn site_hit(cc: &CallCache, recv: Value, kind: u32) -> bool { klass_hit(cc.klass, cc, recv, kind) }

/// site_hit for the callee compiled into the site: spec_klass also says the entry is that callee.
#[inline(always)]
pub fn spec_hit(cc: &CallCache, recv: Value, kind: u32) -> bool { klass_hit(cc.spec_klass, cc, recv, kind) }

/// A call site whose compiled-in callee (entry hash h) was not hit: the call out of line (which
/// refills the cache), then spec_klass from what the cache holds now.
#[inline(never)]
pub fn call_spec_out(c: &mut Ctx, cc: &mut CallCache, mid: Sym, base: Stk, argc: u32, blk: *const Block, flags: u32, h: u64) -> RV {
    let r = call_cached(c, cc, mid, base, argc, blk, flags);
    cc.spec_klass = if cc.fast_hash == h && cc.fast_argc == argc { cc.klass } else { 0 };
    r
}

#[inline(always)]
fn klass_hit(ck: u64, cc: &CallCache, recv: Value, kind: u32) -> bool {
    if kind & 15 == SITE_OTHER { return false; }
    // Integer and Float receivers inline (n.even?, x.to_i ...; a Float may also be a heap object)
    let k = if kind & SITE_IMM != 0 {
        if recv.is_fixnum() { vm().c_integer.get().raw() } else if recv.is_flonum() { vm().c_float.get().raw() }
        else if recv.is_heap() { obj_klass(recv).raw() } else { class_of_imm(recv) }
    }
            else { if !recv.is_heap() { return false; } obj_klass(recv).raw() };
    ck == k && cc.serial == method_serial()
}

/// The one fast path of a site whose cache matched (site_hit); None: go out of line.
#[inline(always)]
pub fn site_fast(c: &mut Ctx, cc: &mut CallCache, recv: Value, base: Stk, argc: u32, blk: *const Block, kind: u32) -> Option<RV> {
    match kind & 15 {
        SITE_ENTRY => if argc == cc.fast_argc {
            c.call_me = cc.me as *const MethodEntry; c.call_blk = blk;
            if let Err(e) = native_check(c) { return Some(Err(e)); }
            // the site just staged: sp is where the entry expects it (no at_entry)
            let r = run_body(c, cc.fast as NodePtr, base);
            c.set_sp(base, 0);
            return Some(r);
        }
        SITE_NEW => if cc.fast & 3 == 0 && cc.fast_argc == u32::MAX {
            let ni = unsafe { &*(cc.fast as *const NewInfo) };
            if argc == ni.init_argc {
                let o = new_object(c, ni.klass);
                base[0].set(o);   // self of initialize: keeps o alive and gives it back
                c.call_me = ni.init_me; c.call_blk = blk;
                let r = run_entry(c, ni.init_entry, base, argc);
                c.set_sp(base, 0);
                return Some(match r { Ok(_) => Ok(base[0].get()), Err(e) => Err(e) });
            }
        }
        SITE_SEND => if cc.bi >= BI_SEND { return send_fast(c, recv, base, argc, blk); }
        SITE_ATTR_R => if cc.fast & 3 == 1 {
            if let Some(ivs) = if recv.is_heap() { c.obj_ivars(recv) } else { None } {
                let v = ivs.get(cc.fast >> 2).unwrap_or(UNDEF);
                c.set_sp(base, 0);
                return Some(Ok(if v == UNDEF { NIL } else { v }));
            }
        }
        SITE_ATTR_W => if cc.fast & 3 == 2 {
            if recv.is_heap() && obj_flags(recv) & FL_FROZEN == 0 {

                if let Some(ivs) = c.obj_ivars_mut(recv) {
                    let v = base[2].get();
                    if ivs.set(cc.fast >> 2, v) { c.set_sp(base, 0); return Some(Ok(v)); }
                }
            }
        }
        _ => if let (0, Some(f), true) = (cc.fast, cc.cfunc, cc.arity < 0 || cc.arity == argc as i32) {
            let pf = c.frame;
            let mut fr = Frame::cfunc(pf, cc.me as *const MethodEntry, blk);
            c.frame = &mut fr;
            let r = with_args(base, argc as usize, |a| f(c, recv, a, blk));
            c.frame = pf;
            c.set_sp(base, 0);
            return Some(r);
        }
    }
    None
}

use crate::inline_builtin::BI_SEND;

/// `recv.send(:name, args...)` at a site whose cache holds the builtin send (cc.bi): the call of
/// `name` itself on the staged frame, the name taken out; no frame for send.  None: the builtin.
#[inline(never)]
pub fn send_fast(c: &mut Ctx, recv: Value, base: Stk, argc: u32, blk: *const Block) -> Option<RV> {
    if argc == 0 { return None; }
    let a0 = base[2].get();
    // String names, refinements, method_missing: the builtin
    if !a0.is_sym() || c.refinements_on() { return None; }
    let w = send_way(c, class_of(recv), a0)?;
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("site: send");
    let n = argc as usize - 1;
    for i in 0..n { base[2 + i].set(base[3 + i].get()); }
    c.set_sp(base, 2 + n);
    let r = if w.argc == n as u32 {
        c.call_me = w.me; c.call_blk = blk;
        match native_check(c) { Ok(()) => run_body(c, w.entry, base), Err(e) => Err(e) }
    } else { invoke_impl(c, w.me, recv, base, n as u32, blk, CF_FCALL) };
    c.set_sp(base, 0);
    Some(r)
}

/// node_send_sym: the receiver class whose #send (or #__send__) was Kernel's, at a method serial.
#[derive(Default)]
pub struct SendGuard { klass: u64, serial: u64 }

/// Is recv's `smid` (send / __send__) the builtin?  Refinements in scope: no (the call of send decides).
#[inline(always)]
pub fn send_guard(c: &Ctx, g: &mut SendGuard, recv: Value, smid: Sym) -> bool {
    let k = class_of(recv);
    if g.klass == k.raw() && g.serial == method_serial() && !c.refinements_on() { return true; }
    send_guard_fill(c, g, k, smid)
}

#[inline(never)]
fn send_guard_fill(c: &Ctx, g: &mut SendGuard, k: Value, smid: Sym) -> bool {
    if c.refinements_on() { return false; }
    let ok = crate::inline_builtin::cache_key(c, lookup_method(c, k, smid), k) == BI_SEND;
    if ok { g.klass = k.raw(); g.serial = method_serial(); }
    ok
}

/// (class, name) -> method of send_fast, direct-mapped; valid while the method serial is unchanged.
#[derive(Clone, Copy)]
pub struct SendWay { klass: u64, mid: u64, serial: u64, me: *const MethodEntry, entry: NodePtr, argc: u32 }
pub const SEND_WAYS: usize = 256;
pub fn send_cache_new() -> Box<[Cell<SendWay>; SEND_WAYS]> {
    Box::new(std::array::from_fn(|_| Cell::new(SendWay { klass: 0, mid: 0, serial: 0, me: std::ptr::null(), entry: std::ptr::null_mut(), argc: u32::MAX })))
}

/// The way for klass (a receiver's class, raw) and the Symbol mid; None: no such method.
#[inline(always)]
fn send_way(c: &Ctx, k: Value, mid: Value) -> Option<SendWay> {
    let klass = k.raw();
    let i = (klass.wrapping_add(mid.raw()).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as usize % SEND_WAYS;
    let serial = method_serial();
    let w = vm().send_cache[i].get();
    if w.klass == klass && w.mid == mid.raw() && w.serial == serial { return Some(w); }
    send_fill(c, i, k, mid, serial)
}

#[inline(never)]
fn send_fill(c: &Ctx, i: usize, k: Value, mid: Value, serial: u64) -> Option<SendWay> {
    let m = c.method_entry(k, mid.as_sym())?;
    // an entry the staged frame can run directly: only required parameters (argc = their count)
    let (entry, argc) = match m.kind {
        MKind::Iseq(is) => match c.iseq_ref(is) { Some(is) if !is.entry.is_null() => (is.entry, is.params.lead.len() as u32), _ => (std::ptr::null_mut(), u32::MAX) },
        _ => (std::ptr::null_mut(), u32::MAX),
    };
    let w = SendWay { klass: k.raw(), mid: mid.raw(), serial, me: m, entry, argc };
    vm().send_cache[i].set(w);
    Some(w)
}

/// cc.fast of a `Klass.new` site (tag 0 with no fast_argc): allocate, then run initialize's entry.
pub struct NewInfo { pub klass: Value, pub init_me: *const MethodEntry, pub init_entry: NodePtr, pub init_argc: u32 }

const POLY_WAYS: usize = 4;

/// The rest of call_cached: a hit without an inline fast path, or a miss.  A site that misses
/// with a filled cache is polymorphic: its other classes go to a small table behind the cache.
#[inline(never)]
pub fn call_slow(c: &mut Ctx, cc: &mut CallCache, mid: Sym, base: Stk, argc: u32, blk: *const Block, flags: u32, hit: bool) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit(if hit { "site: slow (cache hit, no fast path)" } else { "site: slow (miss)" });
    let recv = base[0].get();
    let me = if hit { cc.me as *const MethodEntry } else {
        let klass = class_of(recv);
        let serial = method_serial();
        let e: &mut CallCache = if cc.klass == 0 || cc.serial != serial {
            if cc.poly != 0 { for w in unsafe { &mut *(cc.poly as *mut [CallCache; POLY_WAYS]) }.iter_mut() { w.klass = 0; } }
            let me = lookup_site(c, klass, mid);
            fill_cc(c, cc, recv, klass, me, serial, flags);
            cc
        } else {
            if cc.poly == 0 { cc.poly = Box::into_raw(Box::new(<[CallCache; POLY_WAYS]>::default())) as usize; }
            let t = unsafe { &mut *(cc.poly as *mut [CallCache; POLY_WAYS]) };
            match t.iter().position(|w| w.klass == klass.raw() && w.serial == serial) {
                Some(i) => &mut t[i],
                None => {
                    let i = cc.poly_next as usize % POLY_WAYS;
                    cc.poly_next = cc.poly_next.wrapping_add(1);
                    let me = lookup_site(c, klass, mid);
                    fill_cc(c, &mut t[i], recv, klass, me, serial, flags);
                    &mut t[i]
                }
            }
        };
        if let Some(r) = cc_fast(c, e, recv, base, argc, blk, flags) { return r; }
        e.me as *const MethodEntry
    };
    if me.is_null() {
        let r = method_missing(c, recv, mid, base, argc, blk, flags);
        c.set_sp(base, 0);
        return r;
    }
    unsafe {
        if (*me).visi != VISI_PUBLIC && flags & CF_FCALL == 0 {
            let r = visibility_error(c, me, recv, base, argc, blk, flags);
            c.set_sp(base, 0);
            return r;
        }
    }
    let r = invoke_impl(c, me, recv, base, argc, blk, flags);
    c.set_sp(base, 0);
    r
}

/// Method lookup for a call site: refinements active in the frame's cref come first.
#[inline(always)]
fn lookup_site(c: &Ctx, klass: Value, mid: Sym) -> *const MethodEntry {
    if refinements_on() { lookup_method_refined(c, unsafe { (*c.frame).cref() }, klass, mid) } else { lookup_method(c, klass, mid) }
}

/// Method entry for recv.mid through the call cache (null: not found).
#[inline(always)]
pub fn lookup_cc(c: &mut Ctx, cc: &mut CallCache, recv: Value, mid: Sym, flags: u32) -> *const MethodEntry {
    let klass = class_of(recv);
    let serial = method_serial();
    if cc.klass == klass.raw() && cc.serial == serial { return cc.me as *const MethodEntry; }
    let me = lookup_site(c, klass, mid);
    fill_cc(c, cc, recv, klass, me, serial, flags);
    me
}

/// CallCache.fast_argc of a keyword site whose cache holds the callee's kentry (| the staged count).
pub const KW_DENSE: u32 = 1 << 30;

/// A keyword site passing `names` after npos positionals: if they are the callee's keywords in
/// declared order, put its kentry in the cache (checked once per fill).
#[inline(never)]
pub fn kw_dense_fill(cc: &mut CallCache, me: *const MethodEntry, npos: usize, names: &[Sym], flags: u32) {
    let m = unsafe { &*me };
    if m.visi != VISI_PUBLIC && flags & CF_FCALL == 0 { return; }
    let MKind::Iseq(is) = m.kind else { return };
    let is = unsafe { &*is };
    if is.kentry.is_null() || is.params.lead.len() != npos || is.params.kw.len() != names.len()
        || !is.params.kw.iter().zip(names).all(|(k, n)| k.name == *n) { return; }
    cc.fast = is.kentry as usize;
    cc.fast_argc = KW_DENSE | (npos + names.len()) as u32;
    cc.fast_hash = crate::node::hash_node(is.kentry);
}

/// Can the staged literal keywords go straight into the callee's slots?
pub fn kw_direct_ok(me: *const MethodEntry, npos: usize, names: &[Sym], flags: u32) -> bool {
    let m = unsafe { &*me };
    if m.visi != VISI_PUBLIC && flags & CF_FCALL == 0 { return false; }
    let MKind::Iseq(is) = m.kind else { return false };
    let is = unsafe { &*is };
    let p = &is.params;
    if !p.opt.is_empty() || p.rest.is_some() || !p.post.is_empty() || p.kwrest.is_some() || p.block.is_some()
        || !p.destructure.is_empty() || p.no_kw || p.kw.is_empty() || is.env_size != 0 || p.lead.len() != npos { return false; }
    if !p.lead.iter().enumerate().all(|(i, &s)| s == 2 + i as u32) { return false; }
    names.iter().all(|n| p.kw.iter().any(|k| k.name == *n))
}

pub fn invoke_iseq_kw(c: &mut Ctx, me: *const MethodEntry, is: &'static Iseq, recv: Value, base: Stk, npos: usize, names: &[Sym], blk: *const Block) -> RV {
    let n = is.nlocals as usize;
    stack_check(c, n + 16)?;
    if !is.kentry.is_null() { is.body_used.set(true); }
    let mut vals = [NIL; 16];
    for j in 0..names.len() { vals[j] = base[2 + npos + j].get(); }
    let id = c.next_serial();
    let mut fr = FrameFull {
        h: Frame { prev: c.frame, fp: head(base), me, blk, id, pos: Pos::at(&is.info.line), kind: FK_METHOD, visi: VISI_PUBLIC, lite: false, pad: 0 },
        cref: unsafe { (*me).cref }, iseq: is, self_blk: std::ptr::null(), defined_class: NIL,
    };
    base[0].set(recv);
    base[1].set(NIL);
    nil_fill(base, 2 + npos, n);
    c.set_sp(base, n);
    c.frame = &mut fr.h;
    let mut missing: Vec<Sym> = vec![];
    for kp in is.params.kw.iter() {
        match names.iter().position(|x| *x == kp.name) {
            Some(j) => { base[kp.slot as usize].set(vals[j]); },
            None => if kp.required { missing.push(kp.name); },
        }
    }
    let r = if !missing.is_empty() {
        let ms: Vec<String> = missing.iter().map(|m| format!(":{}", crate::symbol::sym_str(*m))).collect();
        arg_err(c, format!("missing keyword{}: {}", if missing.len() > 1 { "s" } else { "" }, ms.join(", ")))
    } else {
        let mut r = Ok(NIL);
        // static (literal) defaults first, as CRuby
        for kp in is.params.kw.iter().filter(|k| k.static_default).chain(is.params.kw.iter().filter(|k| !k.static_default)) {
            if !names.contains(&kp.name) { if let Some(d) = kp.default { if let Err(e) = run_body(c, d, base) { r = Err(e); break; } } }
        }
        match r { Ok(_) => run_body(c, is.body, base), e => e }
    };
    c.frame = fr.prev;
    match r {
        Ok(v) => Ok(v),
        Err(e) => {
            if c.esc == EscKind::Return && c.esc_tag == id { c.esc = EscKind::None; Ok(c.esc_val) } else { Err(e) }
        }
    }
}

#[inline(never)]
fn fill_cc(c: &mut Ctx, cc: &mut CallCache, recv: Value, klass: Value, me: *const MethodEntry, serial: u64, flags: u32) {
    cc.klass = klass.raw(); cc.serial = serial; cc.me = me as usize; cc.fast = 0; cc.fast_argc = u32::MAX; cc.fast_hash = 0; cc.cfunc = None; cc.bi = 0;
    // calls traced: the entry never hits (baked fast paths trust what a hit implies, e.g. a NewInfo),
    // so every call goes out of line to invoke (tracepoint::invoke_traced)
    if trace_mask() & EV_CALLS != 0 { cc.serial = 0; return; }
    if me.is_null() { return; }
    let m = unsafe { &*me };
    if m.visi != VISI_PUBLIC && flags & CF_FCALL == 0 { return; }
    match m.kind {
        MKind::Iseq(is) => {
            let is = unsafe { &*is };
            let e = is.plain_entry();
            if !e.is_null() { cc.fast = e as usize; cc.fast_argc = is.params.lead.len() as u32; cc.fast_hash = crate::node::hash_node(e); }
        }
        MKind::Cfunc(..) if me == vm().ids.bo_eq.get() => { cc.fast = 3; }
        MKind::Cfunc(f, arity) if me == vm().ids.c_new.get() => {
            cc.cfunc = Some(f); cc.arity = arity;
            // Class#new of a plain-object class whose initialize has a method entry
            let cd = c.class(recv);
            if cd.alloc == AllocKind::Object && !cd.is_singleton && !cd.is_module {
                let im = lookup_method(c, recv, s::initialize);
                if let Some(MKind::Iseq(is)) = (!im.is_null()).then(|| unsafe { (*im).kind }) {
                    let is = unsafe { &*is };
                    if !is.plain_entry().is_null() {
                        let ni = Box::new(NewInfo { klass: recv, init_me: im, init_entry: is.plain_entry(), init_argc: is.params.lead.len() as u32 });
                        cc.fast = Box::into_raw(ni) as usize;
                    }
                }
            }
        }
        MKind::Cfunc(f, arity) => { cc.cfunc = Some(f); cc.arity = arity; cc.bi = crate::inline_builtin::cache_key(c, me, klass); }
        MKind::AttrReader(name) | MKind::AttrWriter(name) => {
            let cd = c.class(klass);
            if !cd.is_singleton && cd.alloc == AllocKind::Object {
                let i = ivar_index_for(c, klass, name) as usize;
                cc.fast = (i << 2) | if matches!(m.kind, MKind::AttrReader(_)) { 1 } else { 2 };
            }
        }
        _ => {}
    }
}

/// The method entry node a call site's cache holds, as the callee to compile into the site (null: none).
pub fn spec_callee(cc: &CallCache, argc: usize, flags: u32) -> NodePtr {
    if cc.fast_argc as usize != argc || flags & CF_KW != 0 { return std::ptr::null_mut(); }
    // only small callees: the site inlines them (size by the length of the dump)
    let mut d = String::new();
    crate::node::dump_node(&mut d, cc.fast as NodePtr);
    if d.len() <= 600 { cc.fast as NodePtr } else { std::ptr::null_mut() }
}

/// spec_callee without the size limit: a recursive site calls its callee, it does not compile it in.
pub fn spec_callee_rec(cc: &CallCache, argc: usize, flags: u32) -> NodePtr {
    if cc.fast_argc as usize != argc || flags & CF_KW != 0 { std::ptr::null_mut() } else { cc.fast as NodePtr }
}

/// site_kind of a literal keyword site: SITE_ENTRY when its cache holds a kentry it can compile in.
pub fn site_kind_kw(cc: &CallCache, argc: usize, ci: &crate::compile::CallInfo) -> u32 {
    if cc.klass == 0 || ci.blk.is_some() || cc.fast_argc != KW_DENSE | argc as u32 { return SITE_OTHER; }
    let v = vm();
    let imm = [v.c_integer.get(), v.c_float.get(), v.c_symbol.get(), v.c_nil.get(), v.c_true.get(), v.c_false.get()].iter().any(|k| k.raw() == cc.klass);
    SITE_ENTRY | if imm { SITE_IMM } else { 0 }
}

/// spec_callee of a literal keyword site: the kentry its cache holds.
pub fn spec_callee_kw(cc: &CallCache, argc: usize, ci: &crate::compile::CallInfo) -> NodePtr {
    if site_kind_kw(cc, argc, ci) == SITE_OTHER { return std::ptr::null_mut(); }
    let mut d = String::new();
    crate::node::dump_node(&mut d, cc.fast as NodePtr);
    if d.len() <= 600 { cc.fast as NodePtr } else { std::ptr::null_mut() }
}

/// spec_callee_kw without the size limit (recursion).
pub fn spec_callee_kw_rec(cc: &CallCache, argc: usize, ci: &crate::compile::CallInfo) -> NodePtr {
    if site_kind_kw(cc, argc, ci) == SITE_OTHER { std::ptr::null_mut() } else { cc.fast as NodePtr }
}

/// The block entry a yield site has always run, as the callee to compile into it (null: none).
/// Only where block_fast would take it with argc arguments; lambda-ness is tested at run time.
pub fn spec_block(yc: &YieldCache, argc: usize) -> NodePtr {
    if yc.seen <= 1 { return std::ptr::null_mut(); }
    let is = unsafe { &*(yc.seen as *const Iseq) };
    if is.bentry.is_null() || is.kind == IseqKind::Lambda || (argc == 1 && is.params.lead.len() >= 2) { return std::ptr::null_mut(); }
    let mut d = String::new();
    crate::node::dump_node(&mut d, is.bentry);
    if d.len() <= 600 { is.bentry } else { std::ptr::null_mut() }
}
/// 1 + the lead parameter count of spec_block's block (0: none).
pub fn spec_block_np(yc: &YieldCache, argc: usize) -> u32 {
    if spec_block(yc, argc).is_null() { 0 } else { 1 + unsafe { (*(yc.seen as *const Iseq)).params.lead.len() as u32 } }
}

/// The heap Env of a frame whose variables are captured by blocks; params copied in.
#[inline(never)]
pub fn make_env(c: &mut Ctx, is: &Iseq, base: Stk, parent: Value) {
    let env = c.alloc_with(vm().c_env.get(), move || Body::Env(crate::envvals::EnvVals::new(is.env_size as usize), parent));
    base[1].set(env);
    for &(sl, ei) in is.env_params.iter() { c.env_set(env, ei as u32, base[sl as usize].get()); }
}

/// Call of an Iseq method with only required, slot-contiguous params.
#[inline(always)]
pub fn invoke_iseq_simple(c: &mut Ctx, me: *const MethodEntry, is: &'static Iseq, recv: Value, base: Stk, argc: u32, blk: *const Block) -> RV {
    if is.plain_entry().is_null() { return invoke_iseq(c, me, is, recv, base, argc, blk, 0); }
    base[0].set(recv);
    c.call_me = me; c.call_blk = blk;
    run_entry(c, is.plain_entry(), base, argc)
}

#[cold]
fn visibility_error(c: &mut Ctx, me: *const MethodEntry, recv: Value, base: Stk, argc: u32, blk: *const Block, flags: u32) -> RV {
    let me_r = unsafe { &*me };
    if me_r.visi == VISI_PROTECTED {
        let slf = c.self_val();
        let owner = if c.class(me_r.owner).is_module { me_r.owner } else { me_r.owner };
        if is_kind_of(c, slf, owner) { return invoke(c, me, recv, base, argc, blk, flags); }
    }
    // private setters may be called with self as explicit receiver
    if me_r.visi == VISI_PRIVATE && flags & CF_SUPER != 0 { return invoke(c, me, recv, base, argc, blk, flags); }
    let kind = if me_r.visi == VISI_PROTECTED { "protected" } else { "private" };
    let desc = crate::builtins::kernel::nomethod_desc(c, recv)?;
    let msg = format!("{} method '{}' called for {}", kind, sym_str(me_r.name), desc);
    let e = make_name_error(c, vm().c_no_method_error.get(), &msg, Value::sym(me_r.name), recv)?;
    Err(raise_exc(c, e))
}

pub fn method_missing(c: &mut Ctx, recv: Value, mid: Sym, base: Stk, argc: u32, blk: *const Block, flags: u32) -> RV {
    let klass = class_of(recv);
    let mm = lookup_method(c, klass, s::method_missing);
    if !mm.is_null() && unsafe { (*mm).owner } != vm().c_basic_object.get() {
        // shift args up by one to insert the method name
        let argc = argc as usize;
        let new_base = stage(c, 3 + argc)?;
        new_base[0].set(recv); new_base[1].set(NIL); new_base[2].set(Value::sym(mid));
        for i in 0..argc { new_base[3 + i].set(base[2 + i].get()); }
        let r = invoke(c, mm, recv, new_base, argc as u32 + 1, blk, (flags & CF_KW) | CF_FCALL);
        c.set_sp(base, 0);
        return r;
    }
    no_method_error(c, recv, mid, flags)
}

pub fn no_method_error(c: &mut Ctx, recv: Value, mid: Sym, flags: u32) -> RV {
    let desc = crate::builtins::kernel::nomethod_desc(c, recv)?;
    let (cls, msg) = if flags & CF_VCALL != 0 {
        (vm().c_name_error.get(), format!("undefined local variable or method '{}' for {}", sym_str(mid), desc))
    } else if flags & CF_SUPER != 0 {
        (vm().c_no_method_error.get(), format!("super: no superclass method '{}' for {}", sym_str(mid), desc))
    } else {
        (vm().c_no_method_error.get(), format!("undefined method '{}' for {}", sym_str(mid), desc))
    };
    let e = make_name_error(c, cls, &msg, Value::sym(mid), recv)?;
    Err(raise_exc(c, e))
}

#[inline]
pub fn invoke(c: &mut Ctx, me: *const MethodEntry, recv: Value, base: Stk, argc: u32, blk: *const Block, flags: u32) -> RV {
    invoke_impl(c, me, recv, base, argc, blk, flags)
}

#[inline(always)]
fn invoke_impl(c: &mut Ctx, me: *const MethodEntry, recv: Value, base: Stk, argc: u32, blk: *const Block, flags: u32) -> RV {
    if trace_mask() & EV_CALLS != 0 { return crate::builtins::tracepoint::invoke_traced(c, me, recv, base, argc, blk, flags); }
    invoke_untraced(c, me, recv, base, argc, blk, flags)
}

/// invoke without the call events (tracepoint::invoke_traced reports them around this).
#[inline(always)]
pub fn invoke_untraced(c: &mut Ctx, me: *const MethodEntry, recv: Value, base: Stk, argc: u32, blk: *const Block, flags: u32) -> RV {
    let mer = unsafe { &*me };
    match mer.kind {
        MKind::Iseq(is) => invoke_iseq(c, me, unsafe { &*is }, recv, base, argc, blk, flags),
        MKind::Cfunc(f, arity) => invoke_cfunc(c, me, f, arity, recv, base, argc, blk, flags),
        MKind::AttrReader(id) => {
            if argc != 0 { return argc_err(c, argc as usize, 0, 0); }
            Ok(ivar_get(c, recv, id))
        }
        MKind::AttrWriter(id) => {
            if argc != 1 { return argc_err(c, argc as usize, 1, 1); }
            let v = base[2].get();
            ivar_set(c, recv, id, v)
        }
        MKind::Bmethod(p) => invoke_bmethod(c, me, p, recv, base, argc, blk, flags),
        MKind::Undef | MKind::ZSuper => no_method_error(c, recv, mer.name, flags),
    }
}

// out of line: inlined into invoke_untraced's callers it changed call_slow's register allocation (+2 instructions per call)
#[inline(never)]
fn invoke_bmethod(c: &mut Ctx, me: *const MethodEntry, p: Value, recv: Value, base: Stk, argc: u32, blk: *const Block, flags: u32) -> RV {
    let mut fr_blk = c.proc_data(p).blk;
    fr_blk.self_ = recv;
    let args = to_vec(&base[2..2 + argc as usize]);
    invoke_block_full(c, &fr_blk, &args, blk, recv, true, me, flags & CF_KW != 0)
}

#[inline]
pub fn invoke_cfunc(c: &mut Ctx, me: *const MethodEntry, f: CFunc, arity: i32, recv: Value, base: Stk, argc: u32, blk: *const Block, flags: u32) -> RV {
    let mut argc = argc;
    if flags & CF_KW != 0 && argc > 0 {
        // keywords to a cfunc: an empty **h passes nothing
        let last = base[1 + argc as usize].get();
        if flags & CF_KW_SPLAT != 0 && is_hash(last) && c.hash(last).live == 0 { argc -= 1; }
        else if is_hash(last) { obj_set_flags(last, FL_KWHASH); }
    }
    if arity >= 0 && argc as i32 != arity { return argc_err(c, argc as usize, arity, arity); }
    let pf = c.frame;
    let mut fr = Frame::cfunc(pf, me, blk);
    c.frame = &mut fr;
    let r = with_args(base, argc as usize, |a| f(c, recv, a, blk));
    c.frame = pf;
    if flags & CF_KW != 0 && argc > 0 {
        let last = base[1 + argc as usize].get();
        if is_hash(last) { obj_clear_flags(last, FL_KWHASH); }
    }
    r
}

/// Is the last argument of a cfunc a keyword hash?
pub fn kw_arg(args: &[Value]) -> Option<Value> {
    let l = *args.last()?;
    if is_hash(l) && obj_flags(l) & FL_KWHASH != 0 { Some(l) } else { None }
}

pub fn invoke_iseq(c: &mut Ctx, me: *const MethodEntry, is: &'static Iseq, recv: Value, base: Stk, argc: u32, blk: *const Block, flags: u32) -> RV {
    if !is.plain_entry().is_null() && argc as usize == is.params.lead.len() && flags & CF_KW == 0 {
        let r = invoke_iseq_simple(c, me, is, recv, base, argc, blk);
        c.set_sp(base, 0);
        return r;
    }
    stack_check(c, is.nlocals as usize + 16)?;
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("invoke_iseq: general (no entry node or argc mismatch)");
    if !is.kentry.is_null() { is.body_used.set(true); }
    let mut fr = Frame::empty(base);
    fr.prev = c.frame;
    fr.me = me;
    fr.blk = blk;
    fr.iseq = is;
    fr.kind = FK_METHOD;
    fr.visi = VISI_PUBLIC;
    fr.id = c.next_serial();
    fr.cref = unsafe { (*me).cref };
    fr.pos = Pos::at(&is.info.line);
    base[0].set(recv);
    c.frame = &mut fr.h;
    let r = if is.simple && argc as usize == is.params.lead.len() && flags & CF_KW == 0 {
        base[1].set(NIL);
        let n = is.nlocals as usize;
        nil_fill(base, 2 + argc as usize, n);
        c.set_sp(base, n);
        run_iseq_body(c, is, base, EV_CALL, EV_RETURN)
    } else {
        match setup_params(c, is, base, argc, blk, flags, false, NIL) {
            Ok(()) => run_iseq_body(c, is, base, EV_CALL, EV_RETURN),
            Err(e) => Err(e),
        }
    };
    c.frame = fr.prev;
    c.set_sp(base, 0);
    match r {
        Ok(v) => Ok(v),
        Err(e) => {
            if c.esc == EscKind::Return && c.esc_tag == fr.id {
                c.esc = EscKind::None;
                Ok(c.esc_val)
            } else { Err(e) }
        }
    }
}

/// run_body of an iseq run in its own frame; code compiled for tracing reports its call / return
/// events (`ev`, `ret`) there.
#[inline(always)]
pub fn run_iseq_body(c: &mut Ctx, is: &'static Iseq, base: Stk, ev: u32, ret: u32) -> RV {
    if is.traced && trace_mask() & (ev | ret) != 0 { return crate::builtins::tracepoint::run_traced_body(c, is, base, ev, ret); }
    run_body(c, is.body, base)
}

#[inline(always)]
pub fn run_body(c: &mut Ctx, body: NodePtr, base: Stk) -> RV {
    let fp = frame(c, base)?;
    unsafe { ((*body).head.dispatcher)(c, body, fp) }
}

/// The native stack check of a method call, made where an entry is called out of line: an entry
/// compiled into its call site adds no native frame, and every recursion passes an out-of-line call.
#[inline(always)]
pub fn native_check(c: &mut Ctx) -> Result<(), Esc> {
    // only its address is used: no store
    let probe = std::mem::MaybeUninit::<u8>::uninit();
    if (probe.as_ptr() as usize) < c.native_limit { return Err(raise_stack_overflow(c)); }
    Ok(())
}

/// run_body for a method entry node.
#[inline(always)]
pub fn run_entry(c: &mut Ctx, entry: NodePtr, base: Stk, argc: u32) -> RV {
    native_check(c)?;
    at_entry(c, base, argc as usize);
    run_body(c, entry, base)
}

/// Place arguments into parameter slots.  For blocks (`is_block`), non-lambda
/// semantics apply: missing args are nil, extra args dropped, a lone Array is splatted.
pub fn setup_params(c: &mut Ctx, is: &Iseq, base: Stk, argc: u32, blk: *const Block, flags: u32, is_block: bool, outer_env: Value) -> Result<(), Esc> {
    let p = &is.params;
    let n = is.nlocals as usize;
    let lambda = !is_block;
    // copy args out (they overlap the local area)
    let mut args = to_vec(&base[2..2 + argc as usize]);
    // keep the args reachable while we allocate; made while they are still on the VM stack
    let tmp = c.ary_new_from(&args);
    nil_fill(base, 1, n);
    c.set_sp(base, n.max(2));
    // keywords
    let mut kwh: Value = NIL;
    let accepts_kw = !p.kw.is_empty() || p.kwrest.is_some() || p.forwarding;
    if flags & CF_KW != 0 && !args.is_empty() {
        let h = *args.last().unwrap();
        if accepts_kw && !p.forwarding {
            kwh = h; args.pop();
        } else if (flags & CF_KW_SPLAT != 0 || is_block) && is_hash(h) && c.hash(h).live == 0 {
            args.pop();
        } else if is.ruby2_keywords.get() && p.rest.is_some() && is_hash(h) {
            let d = c.hash_new();
            c.ary_push(tmp, d);
            crate::hash::hash_copy_entries(c, d, h)?;
            obj_set_flags(d, FL_KWHASH);
            *args.last_mut().unwrap() = d;
        } else if p.forwarding {
            // keep as positional for forwarding; remember it's keywords
        }
    } else if accepts_kw && !p.forwarding && is_block && args.len() > p.lead.len() + p.post.len() + p.opt.len() && p.rest.is_none() {
        // block given a trailing hash
        let h = *args.last().unwrap();
        if is_hash(h) && p.kw.iter().all(|k| !k.required) { kwh = h; args.pop(); }
    }
    if p.no_kw && flags & CF_KW != 0 && args.last().is_some_and(|&h| is_hash(h) && c.hash(h).live > 0) {
        return arg_err(c, "no keywords accepted");
    }
    let lead = p.lead.len();
    let post = p.post.len();
    let opt = p.opt.len();
    // block auto-splat
    // a non-simple block given keywords (even an empty **h) does not auto-splat
    let simple_block = opt == 0 && !p.no_kw && p.rest.is_none() && post == 0 && !accepts_kw && p.block.is_none() && p.destructure.is_empty();
    if is_block && args.len() == 1 && (flags & CF_KW == 0 || simple_block) && (lead + post + opt > 1 || (p.rest.is_some() && lead + post + opt > 0) || p.has_trailing_comma || (lead == 1 && p.rest.is_some()))
        && !p.forwarding {
        let a = args[0];
        let arr = if is_array(a) { Some(a) } else if a.is_heap() && !is_hash(a) && respond_to(c, a, s::to_ary, true) {
            let r = funcall(c, a, s::to_ary, &[], std::ptr::null())?;
            if is_array(r) { Some(r) } else if r == NIL { None } else {
                return type_err(c, format!("can't convert {} to Array ({}#to_ary gives {})", class_path(c, real_class_of(c, a)), class_path(c, real_class_of(c, a)), class_path(c, real_class_of(c, r))));
            }
        } else { None };
        if let Some(arr) = arr { args = c.ary(arr).to_vec(); }
    }
    let given = args.len();
    let required = lead + post;
    if lambda {
        let maxp = if p.rest.is_some() || p.forwarding { usize::MAX } else { required + opt };
        if given < required || given > maxp {
            if !(accepts_kw && kwh == NIL && given == required + opt + 1 && false) {
                let exp = if maxp == usize::MAX { format!("{}+", required) } else if required == maxp { format!("{}", required) } else { format!("{}..{}", required, maxp) };
                // missing required keywords are reported along with the arity
                let missing: Vec<String> = p.kw.iter().filter(|k| k.required).map(|k| sym_str(k.name).to_string()).collect();
                let extra = match missing.len() { 0 => String::new(), 1 => format!("; required keyword: {}", missing[0]), _ => format!("; required keywords: {}", missing.join(", ")) };
                return arg_err(c, format!("wrong number of arguments (given {}, expected {}{})", given, exp, extra));
            }
        }
    }
    let mut ai = 0;
    for &sl in p.lead.iter() { base[sl as usize].set(if ai < given { args[ai] } else { NIL }); ai += 1; }
    let avail_for_opt = given.saturating_sub(required);
    let nopt_given = avail_for_opt.min(opt);
    for (k, (sl, _)) in p.opt.iter().enumerate() {
        if k < nopt_given { base[*sl as usize].set(args[ai]); ai += 1; }
    }
    if let Some(rs) = p.rest {
        let nrest = given.saturating_sub(ai + post);
        let r = c.ary_new_from(if nrest > 0 { &args[ai..ai + nrest] } else { &[] });
        base[rs as usize].set(r);
        ai += nrest;
    } else if p.forwarding {
        let nrest = given.saturating_sub(ai + post);
        let r = c.ary_new_from(if nrest > 0 { &args[ai..ai + nrest] } else { &[] });
        ai += nrest;
        // forwarding stores rest at the hidden slot recorded in kwrest position
        if let Some(fs) = p.kwrest { base[fs as usize].set(r); }
        if flags & CF_KW != 0 { obj_set_flags(r, FL_KWHASH); }
    }
    for &sl in p.post.iter() { base[sl as usize].set(if ai < given { args[ai] } else { NIL }); ai += 1; }
    // keywords; kw_unset: not passed (their slots stay nil until their defaults run)
    let mut kw_unset: Vec<bool> = vec![];
    if !p.kw.is_empty() || (p.kwrest.is_some() && !p.forwarding) {
        let mut seen = 0;
        if kwh != NIL && !is_hash(kwh) { kwh = NIL; }
        for k in p.kw.iter() {
            let v = if kwh != NIL { crate::hash::hash_get_opt(c, kwh, Value::sym(k.name))? } else { None };
            match v {
                Some(v) => { base[k.slot as usize].set(v); seen += 1; kw_unset.push(false); }
                None => { base[k.slot as usize].set(NIL); kw_unset.push(true); }
            }
        }
        let missing: Vec<Sym> = p.kw.iter().zip(&kw_unset).filter(|(k, u)| k.required && **u).map(|(k, _)| k.name).collect();
        if !missing.is_empty() {
            let names: Vec<String> = missing.iter().map(|m| format!(":{}", sym_str(*m))).collect();
            return arg_err(c, format!("missing keyword{}: {}", if missing.len() > 1 { "s" } else { "" }, names.join(", ")));
        }
        if let Some(kr) = p.kwrest {
            let h = c.hash_new();
            if kwh != NIL {
                let mut i = 0;
                while let Some(e) = c.hash_entry_at(kwh, i) {
                    i += 1;
                    if e.0 == UNDEF || e.0.is_sym() && p.kw.iter().any(|k| k.name == e.0.as_sym()) { continue; }
                    crate::hash::hash_set(c, h, e.0, e.1)?;
                }
            }
            base[kr as usize].set(h);
        } else if kwh != NIL && seen < c.hash(kwh).live {
            let mut unknown: Vec<String> = vec![];
            let mut i = 0;
            while let Some(e) = c.hash_entry_at(kwh, i) {
                i += 1;
                if e.0 == UNDEF || e.0.is_sym() && p.kw.iter().any(|k| k.name == e.0.as_sym()) { continue; }
                unknown.push(crate::builtins::kernel::inspect_str(c, e.0).unwrap_or_default());
            }
            if !unknown.is_empty() {
                return arg_err(c, format!("unknown keyword{}: {}", if unknown.len() > 1 { "s" } else { "" }, unknown.join(", ")));
            }
        }
    } else if kwh != NIL && is_hash(kwh) && c.hash(kwh).live > 0 && p.no_kw {

        return arg_err(c, "no keywords accepted");
    }
    if let Some(bs) = p.block {
        base[bs as usize].set(if blk.is_null() { NIL } else { block_to_proc(c, blk)? });
    }
    // env
    if is.env_size > 0 {
        let env = c.alloc_with(vm().c_env.get(), move || Body::Env(crate::envvals::EnvVals::new(is.env_size as usize), outer_env));
        base[1].set(env);
        for &(sl, ei) in is.env_params.iter() {
            c.env_set(env, ei as u32, base[sl as usize].get());

        }
    } else {
        base[1].set(outer_env);
    }
    std::hint::black_box(tmp);
    // defaults (run in the callee frame): static keyword defaults are in place before any other runs (CRuby)
    for (k, u) in p.kw.iter().zip(&kw_unset) {
        if *u && k.static_default { if let Some(d) = k.default { run_body(c, d, base)?; } }
    }
    for (k, (sl, node)) in p.opt.iter().enumerate() {
        if k >= nopt_given { let _ = sl; run_body(c, *node, base)?; }
    }
    for (k, u) in p.kw.iter().zip(&kw_unset) {
        if *u && !k.static_default { if let Some(d) = k.default { run_body(c, d, base)?; } }
    }
    for d in p.destructure.iter() { run_body(c, *d, base)?; }
    Ok(())
}

// ---- blocks and procs ------------------------------------------------------------------------

pub fn block_to_proc(c: &mut Ctx, blk: *const Block) -> RV {
    if blk.is_null() { return Ok(NIL); }
    let b = unsafe { &*blk };
    if b.proc_val != UNDEF { return Ok(b.proc_val); }
    if let BlockKind::Proc(p) = b.kind { return Ok(p); }
    let outer = if b.outer_blk.is_null() { std::ptr::null() } else { let op = block_to_proc(c, b.outer_blk)?; proc_block(c, op) };
    let is_lambda = matches!(b.kind, BlockKind::Iseq(is) if unsafe { (*is).kind } == IseqKind::Lambda) || matches!(b.kind, BlockKind::Method(_));
    // copied from *b into the box, then patched there: a Block patched on the stack first was
    // read back with wide loads straddling the patch (a store-forwarding stall per Proc)
    let mut pd = Box::new(ProcData { blk: *b, is_lambda, from_method: NIL });
    pd.blk.outer_blk = outer;
    let p = c.alloc_with(vm().c_proc.get(), move || Body::Proc(pd));
    c.proc_data_mut(p).blk.proc_val = p;
    unsafe { (*(blk as *mut Block)).proc_val = p; }
    Ok(p)
}

/// A Block view of a Proc value (for passing a proc as &blk).
pub fn proc_block(c: &Ctx, p: Value) -> *const Block { &c.proc_data(p).blk }

pub fn is_lambda_block(c: &Ctx, b: *const Block) -> bool {
    let b = unsafe { &*b };
    if b.proc_val != UNDEF && is_proc(b.proc_val) { return c.proc_data(b.proc_val).is_lambda; }
    matches!(b.kind, BlockKind::Iseq(is) if unsafe { (*is).kind } == IseqKind::Lambda)
}

/// yield: run the block with args (copied to the VM stack).  One copy: its callers are in every codegen unit,
/// and an #[inline] copy per unit made a builtin's instruction count depend on how cargo split them.
#[inline(never)]
pub fn yield_block(c: &mut Ctx, blk: *const Block, args: &[Value]) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("yield: from a builtin (yield_block)");
    if blk.is_null() { return err(c, vm().c_local_jump_error.get(), "no block given (yield)"); }
    let b = unsafe { &*blk };
    if let Some(is) = block_fast(c, b, args.len()) {
        let base = c.top();
        stack_check(c, is.nlocals as usize + args.len() + 16)?;
        put(base, 2, args);
        let r = run_block_fast(c, b, is, base, args.len() as u32);
        c.set_sp(base, 0);
        return r;
    }
    invoke_block_full(c, b, args, std::ptr::null(), b.self_, false, std::ptr::null(), false)
}

/// The iseq of a non-lambda block with only required params that `argc` args bind to without spreading.
#[inline(always)]
pub fn block_fast(c: &Ctx, b: &Block, argc: usize) -> Option<&'static Iseq> {
    if let BlockKind::Iseq(is) = b.kind {
        let is = unsafe { &*is };
        // extra args are dropped and missing ones are nil; one arg for 2+ params may be an Array to spread
        // a `->` block entry is only reached as a lambda proc, which the last test excludes
        if !is.bentry.is_null() && !(argc == 1 && is.params.lead.len() >= 2)
            && (b.proc_val == UNDEF || !c.proc_data(b.proc_val).is_lambda) { return Some(is); }
    }
    None
}

/// Run a block_fast block with args already at base[2..]; sp is at base.
#[inline(always)]
pub fn run_block_fast(c: &mut Ctx, b: &Block, is: &Iseq, base: Stk, argc: u32) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit_body("block_fast", is.body, is.env_size > 0, is.params.lead.len(), false);
    // missing arguments are nil; the entry node (frame setup, sizes baked) does the rest
    let np = is.params.lead.len();
    if (argc as usize) < np { nil_fill(base, 2 + argc as usize, 2 + np); }
    c.call_blk = b;
    run_body(c, is.bentry, base)
}

/// One out-of-line copy of the one-argument fast path (not left to yield_block's inlining luck).
#[inline(never)]
pub fn yield1(c: &mut Ctx, blk: *const Block, a: Value) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("yield: from a builtin, 1 arg (yield1)");
    if !blk.is_null() {
        let b = unsafe { &*blk };
        if let Some(is) = block_fast(c, b, 1) {
            let base = c.top();
            stack_check(c, is.nlocals as usize + 17)?;
            base[2].set(a);
            let r = run_block_fast(c, b, is, base, 1);
            c.set_sp(base, 0);
            return r;
        }
    }
    yield_block(c, blk, &[a])
}

pub fn yield_with_self(c: &mut Ctx, blk: *const Block, args: &[Value], self_: Value, defined_class: Value) -> RV {
    let b = unsafe { &*blk };
    let mut b2 = *b;
    b2.self_ = self_;
    if defined_class != NIL { b2.defined_class = defined_class; }
    invoke_block_full(c, &b2, args, std::ptr::null(), self_, false, std::ptr::null(), false)
}

pub fn call_proc(c: &mut Ctx, p: Value, args: &[Value], blk: *const Block) -> RV { call_proc_kw(c, p, args, blk, false) }

pub fn call_proc_kw(c: &mut Ctx, p: Value, args: &[Value], blk: *const Block, kw: bool) -> RV {
    let (b, p_lambda) = { let pd = c.proc_data(p); (pd.blk, pd.is_lambda) };
    // a `->` lambda called with its exact arity: its block entry (lambda frame, sizes baked)
    if let BlockKind::Iseq(is) = b.kind {
        let is = unsafe { &*is };
        if is.kind == IseqKind::Lambda && !is.bentry.is_null() && !kw && blk.is_null() && args.len() == is.params.lead.len() {
            let base = c.top();
            stack_check(c, is.nlocals as usize + args.len() + 16)?;
            put(base, 2, args);
            c.call_blk = &b;
            let r = run_body(c, is.bentry, base);
            c.set_sp(base, 0);
            return r;
        }
    }
    invoke_block_full(c, &b, args, blk, b.self_, p_lambda, std::ptr::null(), kw)
}

/// Run a block.  `lambda_call`: strict arity / own return target (lambda procs, bmethods).
pub fn invoke_block_full(c: &mut Ctx, b: &Block, args: &[Value], blkarg: *const Block, self_: Value, lambda_call: bool, bme: *const MethodEntry, kw: bool) -> RV {
    match b.kind {
        BlockKind::Iseq(is) => {
            let is = unsafe { &*is };
            let base = c.top();
            stack_check(c, is.nlocals as usize + args.len() + 16)?;
            base[0].set(self_);
            put(base, 2, args);
            c.set_sp(base, 2 + args.len());
            let r = run_block_iseq(c, b, is, base, args.len() as u32, blkarg, lambda_call || is.kind == IseqKind::Lambda, bme, kw);
            c.set_sp(base, 0);
            r
        }
        BlockKind::Sym(mid) => {
            if args.is_empty() { return arg_err(c, "no receiver given"); }
            let recv = args[0];
            let fl = if kw && args.len() > 1 { CF_KW } else { 0 };   // :m.to_proc.(recv, k: v) passes keywords on
            if refinements_on() {
                if let Some(r) = funcall_refined(c, b.cref, recv, mid, &args[1..], blkarg, fl) { return r; }
            }
            funcall_flags(c, recv, mid, &args[1..], blkarg, fl)
        }
        BlockKind::Method(m) => {
            let (recv, me) = { let mo = c.method_obj(m); (mo.recv, mo.me) };
            let base = stage(c, 2 + args.len())?;
            base[0].set(recv); base[1].set(NIL);
            put(base, 2, args);
            let r = invoke(c, me, recv, base, args.len() as u32, blkarg, CF_FCALL | if kw { CF_KW } else { 0 });
            c.set_sp(base, 0);
            r
        }
        BlockKind::Cfunc(f, data) => f(c, data, args, blkarg),
        BlockKind::Proc(p) => {
            let (inner, p_lambda) = { let pd = c.proc_data(p); (pd.blk, pd.is_lambda) };
            invoke_block_full(c, &inner, args, blkarg, if self_ == b.self_ { inner.self_ } else { self_ }, p_lambda || lambda_call, bme, kw)

        }
    }
}

pub fn run_block_iseq(c: &mut Ctx, b: &Block, is: &'static Iseq, base: Stk, argc: u32, blkarg: *const Block, lambda: bool, bme: *const MethodEntry, kw: bool) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit_body(if lambda { "block_full lambda" } else { "block_full" }, is.body, is.env_size > 0, is.params.lead.len(), false);
    if !is.bentry.is_null() { is.body_used.set(true); }
    let mut fr = Frame::empty(base);
    fr.prev = c.frame;
    fr.me = if bme.is_null() { b.me } else { bme };
    fr.blk = if !bme.is_null() { blkarg } else { b.outer_blk };
    fr.cref = b.cref;
    fr.iseq = is;
    fr.kind = if lambda { FK_LAMBDA } else { FK_BLOCK };
    fr.visi = VISI_PUBLIC;
    fr.id = if lambda { c.next_serial() } else { b.home_id };
    fr.self_blk = b;
    fr.pos = Pos::at(&is.info.line);
    fr.defined_class = b.defined_class;
    c.frame = &mut fr.h;
    let fast = is.lead_only && is.env_size == 0 && argc as usize == is.params.lead.len() && (argc != 1 || is.params.lead.len() == 1) && !kw;
    let r = if fast {
        base[1].set(b.env);
        let n = is.nlocals as usize;
        nil_fill(base, 2 + argc as usize, n);
        c.set_sp(base, n);
        run_iseq_body(c, is, base, EV_B_CALL, EV_B_RETURN)
    } else {
        let flags = if kw { CF_KW } else { 0 };
        match setup_params(c, is, base, argc, blkarg, flags, !lambda, b.env) {
            Ok(()) => run_iseq_body(c, is, base, EV_B_CALL, EV_B_RETURN),
            Err(e) => Err(e),
        }
    };
    // redo: run the body again with the same arguments
    let mut r = r;
    while r.is_err() && c.esc == EscKind::Redo { c.esc = EscKind::None; r = run_body(c, is.body, base); }
    c.frame = fr.prev;
    match r {
        Ok(v) => Ok(v),
        Err(e) => {
            match c.esc {
                EscKind::Next => { c.esc = EscKind::None; Ok(c.esc_val) }
                EscKind::Return if lambda && c.esc_tag == fr.id => { c.esc = EscKind::None; Ok(c.esc_val) }
                EscKind::Break if lambda && (c.esc_tag == b.tag) => { c.esc = EscKind::None; Ok(c.esc_val) }
                _ => Err(e),
            }
        }
    }
}

// ---- GC roots -----------------------------------------------------------------------------------

/// The exact root slots of the VM, and the value stacks (ambiguous).
/// `cur`: the context that allocates or collects, scanned through this borrow only.
pub fn visit_roots<V: RootVisitor<Saruby>>(m: &mut V, cur: &mut Ctx) {
    let vm = vm();
    visit_ctx(m, cur);
    let p: *mut Ctx = cur;
    SCAN_CTX.store(p, std::sync::atomic::Ordering::Relaxed);
    if vm.main_ctx.get() != p { visit_main_ctx(m); }
    for v in [&vm.c_basic_object, &vm.c_object, &vm.main_obj, &vm.stdout, &vm.stderr, &vm.stdin, &vm.loaded_features,
              &vm.load_path, &vm.at_exit, &vm.verbose, &vm.argv, &vm.dollar_zero] { visit_cell(m, v); }
    // a mutator holding one of these borrows across an allocation panics here instead of aliasing
    for v in vm.globals.borrow_mut().values_mut() { m.visit(v); }
    for v in vm.node_values.borrow_mut().iter_mut() { m.visit(v); }
    for v in vm.pinned.borrow_mut().iter_mut() { m.visit(v); }
    for v in vm.fiber_roots.borrow_mut().iter_mut() { m.visit(v); }
    for v in vm.frozen_lits.borrow_mut().values_mut() { m.visit(v); }
    for v in vm.symproc_cache.borrow_mut().values_mut() { m.visit(v); }
    for v in vm.sym_names.borrow_mut().values_mut() { m.visit(v); }
    for &cr in vm.crefs.borrow().iter() { let cr = unsafe { &mut *(cr as *mut Cref) }; m.visit(&mut cr.klass); m.visit(&mut cr.refinements); }
    // every builtin class field (nil until boot fills it in)
    let nclass_fields = (std::mem::offset_of!(Vm, main_obj)) / 8;
    // SAFETY: Vm is repr(C) and starts with nclass_fields Cell<Value> fields
    let cells = unsafe { std::slice::from_raw_parts(vm as *const Vm as *const Cell<Value>, nclass_fields) };
    for k in cells { visit_cell(m, k); }
}

static SCAN_CTX: std::sync::atomic::AtomicPtr<Ctx> = std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
/// The context the running collection scans through its caller's borrow; compared, never dereferenced.
pub fn scan_ctx() -> *mut Ctx { SCAN_CTX.load(std::sync::atomic::Ordering::Relaxed) }

#[inline(always)]
fn visit_cell<V: RootVisitor<Saruby>>(m: &mut V, k: &Cell<Value>) { let mut v = k.get(); m.visit(&mut v); k.set(v); }

/// The main context, by place, while a fiber allocates: main is suspended in a switch that holds its `&mut Ctx`.
fn visit_main_ctx<V: RootVisitor<Saruby>>(m: &mut V) {
    let p = vm().main_ctx.get();
    if p.is_null() { return; }   // boot: no context yet
    // SAFETY: run_main's context, which outlives every collection
    unsafe {
        let live = (*p).live();
        m.visit_ambiguous(std::slice::from_raw_parts(live.as_ptr() as *const usize, live.len()));
        for f in [&raw mut (*p).esc_val, &raw mut (*p).errinfo, &raw mut (*p).last_match, &raw mut (*p).esc_tag_v] {
            let mut v = f.read();
            m.visit(&mut v);
            if v != f.read() { f.write(v); }   // only a moving collector writes
        }
    }
}

/// One execution context: its value stack and the values it holds.
pub fn visit_ctx<V: RootVisitor<Saruby> + ?Sized>(m: &mut V, c: &mut Ctx) {
    // ambiguous, as before the framework: slots below sp that no frame wrote yet hold stale Values
    let live = c.live();
    // SAFETY: a Slot is one Value word
    unsafe { m.visit_ambiguous(std::slice::from_raw_parts(live.as_ptr() as *const usize, live.len())); }
    m.visit(&mut c.esc_val); m.visit(&mut c.errinfo); m.visit(&mut c.last_match); m.visit(&mut c.esc_tag_v);
}

pub fn visit_class<V: EdgeVisitor<Saruby> + ?Sized>(m: &mut V, cd: &mut ClassData) {
    if let Some(n) = &mut cd.name { m.visit(n); }
    m.visit(&mut cd.superclass); m.visit(&mut cd.attached); m.visit(&mut cd.lexical_parent);
    for (_, &me) in cd.mtbl.iter() { visit_me(m, me); }
    for v in cd.consts.values_mut() { m.visit(v); }
    for v in cd.autoload.values_mut() { m.visit(v); }
    for v in cd.cvars.values_mut() { m.visit(v); }
    for v in cd.includes.iter_mut() { m.visit(v); }
    for v in cd.prepends.iter_mut() { m.visit(v); }
    for v in cd.subclasses.iter_mut() { m.visit(v); }
    for v in cd.extended_by.iter_mut() { m.visit(v); }
    // a cache of classes reachable from superclass / includes / prepends: marked by value (a moving
    // collector would drop the cache instead of updating a shared Rc)
    if let Some((_, a)) = cd.ancestors.get_mut() { for &v in a.iter() { let mut x = v; m.visit(&mut x); } }
}

pub fn visit_me<V: EdgeVisitor<Saruby> + ?Sized>(m: &mut V, me: *const MethodEntry) {
    if me.is_null() { return; }
    let me = unsafe { &mut *(me as *mut MethodEntry) };
    m.visit(&mut me.owner);
    m.visit(&mut me.orig_owner);
    if let MKind::Bmethod(p) = &mut me.kind { m.visit(p); }
}

pub fn visit_block<V: EdgeVisitor<Saruby> + ?Sized>(m: &mut V, b: &mut Block) {
    m.visit(&mut b.self_); m.visit(&mut b.env); m.visit(&mut b.proc_val); m.visit(&mut b.defined_class);
    match &mut b.kind { BlockKind::Method(v) | BlockKind::Cfunc(_, v) | BlockKind::Proc(v) => m.visit(v), _ => {} }
    if !b.outer_blk.is_null() {
        let ob = unsafe { &mut *(b.outer_blk as *mut Block) };
        if ob.proc_val != UNDEF { m.visit(&mut ob.proc_val); }
    }
    if !b.me.is_null() { visit_me(m, b.me); }
}

// ---- misc helpers --------------------------------------------------------------------------

pub fn sym_value(name: &str) -> Value { Value::sym(intern_str(name)) }
pub fn global_get(name: &str) -> Value { *vm().globals.borrow().get(&intern_str(name)).unwrap_or(&NIL) }
pub fn global_set(name: &str, v: Value) { vm().globals.borrow_mut().insert(intern_str(name), v); }

pub fn intern_value(b: &[u8]) -> Value { Value::sym(intern(b)) }

pub fn current_line(c: &Ctx) -> u32 { unsafe { if c.frame.is_null() { 0 } else { (*c.frame).line() } } }

pub fn the_dispatch_type_check(_f: DispatchFn) {}
