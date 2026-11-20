// Symbol table: interned byte strings, never collected.  Ids of the predefined
// symbols below are fixed (interned first, in order).
//
// Names are leaked (`&'static [u8]` is literally true) and live in an append-only table of
// chunks whose slots are `OnceLock`s, so `sym_bytes` reads without a lock (two acquire loads,
// plain loads on x86). Only `intern` / `lookup` take the map's `Mutex`; writers are serialized
// by it, so a slot is set once, before its id is handed out.
#![forbid(unsafe_code)]
use crate::value::Sym;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const CHUNK_BITS: u32 = 12;
const CHUNK: usize = 1 << CHUNK_BITS;
const NCHUNK: usize = 1 << 12;   // 16M symbols
type Chunk = Box<[OnceLock<&'static [u8]>]>;
static NAMES: [OnceLock<Chunk>; NCHUNK] = [const { OnceLock::new() }; NCHUNK];
struct Map { ids: HashMap<&'static [u8], u32>, n: u32 }
static MAP: Mutex<Option<Map>> = Mutex::new(None);

#[inline(never)]
fn add(m: &mut Map, b: &[u8]) -> Sym {
    if let Some(&i) = m.ids.get(b) { return Sym(i); }
    let i = m.n;
    assert!((i as usize) < CHUNK * NCHUNK, "saruby: symbol table full");
    let s: &'static [u8] = Box::leak(b.to_vec().into_boxed_slice());
    let chunk = NAMES[i as usize >> CHUNK_BITS].get_or_init(|| (0..CHUNK).map(|_| OnceLock::new()).collect());
    let _ = chunk[i as usize & (CHUNK - 1)].set(s);
    m.ids.insert(s, i);
    m.n = i + 1;
    Sym(i)
}
#[inline]
fn with_map<R>(f: impl FnOnce(&mut Map) -> R) -> R {
    let mut g = MAP.lock().unwrap_or_else(|e| e.into_inner());
    match g.as_mut() { Some(m) => f(m), None => f(g.insert(new_map())) }
}
#[cold]
fn new_map() -> Map {
    let mut m = Map { ids: HashMap::new(), n: 0 };
    for s in PREDEF { add(&mut m, s.as_bytes()); }
    m
}

#[inline]
pub fn intern(b: &[u8]) -> Sym { with_map(|m| match m.ids.get(b) { Some(&i) => Sym(i), None => add(m, b) }) }
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
}

/// Build the table (the first intern does it too).
pub fn init() { with_map(|_| ()); }
