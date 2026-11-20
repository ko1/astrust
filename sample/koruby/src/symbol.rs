// Symbol table: interned byte strings, never collected.  Ids of the predefined
// symbols below are fixed (interned first, in order).
use crate::value::Sym;
use std::collections::HashMap;

pub struct SymTab { pub names: Vec<&'static [u8]>, pub map: HashMap<&'static [u8], u32> }

static mut SYMTAB: Option<SymTab> = None;

#[allow(static_mut_refs)]
#[inline]
fn tab() -> &'static mut SymTab {
    unsafe {
        if SYMTAB.is_none() { init(); }
        SYMTAB.as_mut().unwrap_unchecked()
    }
}

pub fn intern(b: &[u8]) -> Sym {
    let t = tab();
    if let Some(&i) = t.map.get(b) { return Sym(i); }
    let s: &'static [u8] = Box::leak(b.to_vec().into_boxed_slice());
    let i = t.names.len() as u32;
    t.names.push(s);
    t.map.insert(s, i);
    Sym(i)
}
#[inline] pub fn intern_str(s: &str) -> Sym { intern(s.as_bytes()) }
#[inline] pub fn sym_bytes(s: Sym) -> &'static [u8] { tab().names[s.0 as usize] }
pub fn sym_str(s: Sym) -> String { String::from_utf8_lossy(sym_bytes(s)).into_owned() }
pub fn lookup(b: &[u8]) -> Option<Sym> { tab().map.get(b).map(|&i| Sym(i)) }
pub fn all_count() -> usize { tab().names.len() }

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

#[allow(static_mut_refs)]
pub fn init() {
    unsafe {
        if SYMTAB.is_some() { return; }
        SYMTAB = Some(SymTab { names: vec![], map: HashMap::new() });
    }
    for s in PREDEF { intern(s.as_bytes()); }
}
