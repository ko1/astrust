// Symbol table: interned byte strings, never collected.  Ids of the predefined
// symbols below are fixed (interned first, in order).
//
// Names are leaked (`&'static [u8]` is literally true) and live in an append-only table of
// chunks whose slots are `OnceLock`s, so `sym_bytes` reads without a lock (two acquire loads,
// plain loads on x86). `intern` answers from a lock-free cache when it can; misses and
// `lookup` take the map's `Mutex`. Writers are serialized by it, so a slot is set once,
// before its id is handed out.
#![forbid(unsafe_code)]
use crate::value::Sym;
use crate::fxhash::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

const CHUNK_BITS: u32 = 12;
const CHUNK: usize = 1 << CHUNK_BITS;
const NCHUNK: usize = 1 << 12;   // 16M symbols
type Chunk = Box<[OnceLock<&'static [u8]>]>;
static NAMES: [OnceLock<Chunk>; NCHUNK] = [const { OnceLock::new() }; NCHUNK];
// `encs`: symbols whose String is in an encoding other than US-ASCII / UTF-8 (CRuby keeps the encoding
// in the symbol); their names are not in `ids`, so the same bytes make a different symbol
struct Map { ids: HashMap<&'static [u8], u32>, n: u32, enc_ids: HashMap<(&'static [u8], u8), u32>, encs: HashMap<u32, u8> }
static MAP: Mutex<Option<Map>> = Mutex::new(None);

#[inline(never)]
fn add(m: &mut Map, b: &[u8]) -> Sym {
    if let Some(&i) = m.ids.get(b) { return Sym(i); }
    let (s, i) = push_name(m, b);
    m.ids.insert(s, i);
    Sym(i)
}
#[inline(always)]
fn push_name(m: &mut Map, b: &[u8]) -> (&'static [u8], u32) {
    let i = m.n;
    assert!((i as usize) < CHUNK * NCHUNK, "saruby: symbol table full");
    let s: &'static [u8] = Box::leak(b.to_vec().into_boxed_slice());
    let chunk = NAMES[i as usize >> CHUNK_BITS].get_or_init(|| (0..CHUNK).map(|_| OnceLock::new()).collect());
    let _ = chunk[i as usize & (CHUNK - 1)].set(s);
    m.n = i + 1;
    (s, i)
}
/// Whether any symbol has an encoding of its own (sym_enc answers None without a lock until then).
static ANY_ENC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// The symbol of `b` in encoding `enc`, for a String that is neither ASCII-only in an ASCII-compatible
/// encoding nor UTF-8 (those are `intern`'s).
#[cold]
pub fn intern_enc(b: &[u8], enc: u8) -> Sym {
    with_map(|m| {
        if let Some(&i) = m.enc_ids.get(&(b, enc)) { return Sym(i); }
        let (s, i) = push_name(m, b);
        m.enc_ids.insert((s, enc), i);
        m.encs.insert(i, enc);
        ANY_ENC.store(true, Ordering::Release);
        Sym(i)
    })
}
/// The encoding of a symbol made by `intern_enc`; None for the others.
#[inline]
pub fn sym_enc(s: Sym) -> Option<u8> {
    if !ANY_ENC.load(Ordering::Acquire) { return None; }
    with_map(|m| m.encs.get(&s.0).copied())
}
#[inline]
fn with_map<R>(f: impl FnOnce(&mut Map) -> R) -> R {
    let mut g = MAP.lock().unwrap_or_else(|e| e.into_inner());
    match g.as_mut() { Some(m) => f(m), None => f(g.insert(new_map())) }
}
#[cold]
fn new_map() -> Map {
    // ~1600 symbols exist after boot; 1700 is what 2048 buckets hold
    let mut m = Map { ids: HashMap::with_capacity_and_hasher(1700, Default::default()), n: 0, enc_ids: HashMap::default(), encs: HashMap::default() };
    for s in PREDEF { add(&mut m, s.as_bytes()); }
    m
}

// A direct-mapped cache in front of the map: slot = id + 1, verified against the name on a
// hit, so a stale or overwritten slot only costs a miss. Release / Acquire make the name of
// an id visible before the id is.
const CACHE_BITS: u32 = 14;   // 64 KiB: each slot touched is a page fault at boot (16 bits: 64 of them)
static CACHE: [AtomicU32; 1 << CACHE_BITS] = [const { AtomicU32::new(0) }; 1 << CACHE_BITS];
#[inline]
fn cache_slot(b: &[u8]) -> &'static AtomicU32 {
    use std::hash::Hasher;
    let mut h = crate::fxhash::FxHasher::default();
    h.write(b);
    &CACHE[(h.finish() >> (64 - CACHE_BITS)) as usize]
}

// out of line: inlined into its 600-odd callers, bignum AOT ran 6 % more cycles on the same instructions
#[inline(never)]
pub fn intern(b: &[u8]) -> Sym {
    let slot = cache_slot(b);
    let v = slot.load(Ordering::Acquire);
    if v != 0 && sym_bytes(Sym(v - 1)) == b { return Sym(v - 1); }
    intern_slow(b, slot)
}
#[inline(never)]
fn intern_slow(b: &[u8], slot: &AtomicU32) -> Sym {
    let s = with_map(|m| match m.ids.get(b) { Some(&i) => Sym(i), None => add(m, b) });
    slot.store(s.0 + 1, Ordering::Release);
    s
}
/// String#to_sym's answer for a String that is no symbol.
pub const BAD_SYM: Sym = Sym(u32::MAX);
#[inline] pub fn intern_str(s: &str) -> Sym { intern(s.as_bytes()) }
#[inline]
pub fn sym_bytes(s: Sym) -> &'static [u8] {
    let i = s.0 as usize;
    match NAMES.get(i >> CHUNK_BITS).and_then(|c| c.get()).and_then(|c| c[i & (CHUNK - 1)].get()) {
        Some(b) => b,
        None => sym_bytes_slow(s),
    }
}
#[cold]
#[inline(never)]
fn sym_bytes_slow(s: Sym) -> &'static [u8] {
    init();   // the predefined ids are valid before anything was interned
    let i = s.0 as usize;
    NAMES.get(i >> CHUNK_BITS).and_then(|c| c.get()).and_then(|c| c[i & (CHUNK - 1)].get()).copied().expect("saruby: not a symbol id")
}
pub fn sym_str(s: Sym) -> String { String::from_utf8_lossy(sym_bytes(s)).into_owned() }
pub fn lookup(b: &[u8]) -> Option<Sym> { with_map(|m| m.ids.get(b).map(|&i| Sym(i))) }
pub fn all_count() -> usize { with_map(|m| m.n as usize) }

macro_rules! predefined_syms {
    ($($name:ident = $s:expr),* $(,)?) => {
        #[allow(non_upper_case_globals, dead_code)]
        pub mod s {
            use crate::value::Sym;
            #[allow(non_camel_case_types, dead_code)]
            #[repr(u32)]
            enum Ids { _Empty_, $($name),* }
            $(pub const $name: Sym = Sym(Ids::$name as u32);)*
        }
        const PREDEF: &[&str] = &["", $($s),*];
    };
}

predefined_syms! {
    plus = "+", minus = "-", mul = "*", div = "/", modulo = "%", pow = "**",
    lt = "<", le = "<=", gt = ">", ge = ">=", eq = "==", neq = "!=", eqq = "===", cmp = "<=>",
    match_op = "=~", nmatch = "!~", not = "!", aref = "[]", aset = "[]=", ltlt = "<<", gtgt = ">>",
    and = "&", or = "|", xor = "^", tilde = "~", uplus = "+@", uminus = "-@",
    eql_p = "eql?", equal_p = "equal?", hash = "hash", to_s = "to_s", to_str = "to_str", inspect = "inspect",
    to_a = "to_a", to_ary = "to_ary", to_i = "to_i", to_int = "to_int", to_f = "to_f", to_h = "to_h", to_hash = "to_hash",
    to_proc = "to_proc", to_sym = "to_sym", call = "call", each = "each", new = "new", initialize = "initialize",
    initialize_copy = "initialize_copy", method_missing = "method_missing", respond_to_p = "respond_to?",
    respond_to_missing_p = "respond_to_missing?", coerce = "coerce", succ = "succ", size = "size", length = "length",
    message = "message", backtrace = "backtrace", exception = "exception", inherited = "inherited",
    included = "included", extended = "extended", prepended = "prepended", method_added = "method_added",
    singleton_method_added = "singleton_method_added", const_missing = "const_missing", name = "name",
    allocate = "allocate", dup = "dup", clone = "clone", freeze = "freeze", begin = "begin", end_ = "end",
    exclude_end_p = "exclude_end?", mesg = "mesg", bt = "bt", cause = "cause", main = "main",
    lambda = "lambda", proc = "proc", block_given_p = "block_given?", raise = "raise", puts = "puts",
    p = "p", print = "print", require = "require", require_relative = "require_relative", loop_ = "loop",
    attr_accessor = "attr_accessor", private = "private", public = "public", protected = "protected",
    module_function = "module_function", nil_p = "nil?", is_a_p = "is_a?", kind_of_p = "kind_of?", class = "class",
    send = "send", __send__ = "__send__", public_send = "public_send", object_id = "object_id",
    instance_eval = "instance_eval", instance_exec = "instance_exec", class_eval = "class_eval",
    module_eval = "module_eval", class_exec = "class_exec", define_method = "define_method",
    first = "first", last = "last", keys = "keys", values = "values", key_p = "key?", fetch = "fetch",
    push = "push", pop = "pop", shift = "shift", unshift = "unshift", concat = "concat", join = "join",
    map = "map", select = "select", reject = "reject", times = "times", upto = "upto", downto = "downto",
    step = "step", zero_p = "zero?", sort = "sort", min = "min", max = "max", sum = "sum",
    each_with_index = "each_with_index", __method__ = "__method__", binding = "binding",
    _id2ref = "_id2ref", default = "default", divmod = "divmod", fdiv = "fdiv", abs = "abs",
    nil = "nil", true_ = "true", false_ = "false", self_ = "self", super_ = "super", yield_ = "yield",
    expression = "expression", local_variable = "local-variable", global_variable = "global-variable",
    instance_variable = "instance-variable", constant = "constant", method = "method", assignment = "assignment",
    class_variable = "class variable", d_object_id = "__id__", Object = "Object", ruby2_keywords_flag = "__ruby2kw__",
    eof = "EOF", backtrace_locations = "backtrace_locations", full_message = "full_message", detailed_message = "detailed_message",
    set_backtrace = "set_backtrace", frozen_p = "frozen?", ivar_bt_locs = "bt_locations", at_exit = "at_exit",
    errno = "errno", status = "status", success_p = "success?", key = "key", receiver = "receiver",
    instance_variable_get = "instance_variable_get", __recursive_key__ = "__recursive_key__",
    ary_flatten_recursive = "__flatten_rec__", sort_by = "sort_by", each_pair = "each_pair",
    mid_initialize_dup = "initialize_dup", mid_initialize_clone = "initialize_clone",
    ary_cycle_size = "__ary_cycle_size", ary_comb_size = "__ary_comb_size", ary_perm_size = "__ary_perm_size", ary_rcomb_size = "__ary_rcomb_size", ary_rperm_size = "__ary_rperm_size",
}

/// Build the table (the first intern does it too).
pub fn init() { with_map(|_| ()); }
