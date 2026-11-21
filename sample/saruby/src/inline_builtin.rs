// Builtin method bodies inlined at baked call sites (node_call's `bi`).
//
// At bake time a site whose cache holds one of the builtins below (with the argument count it
// takes) is baked with the number of its body; the baked site evaluates the receiver and the
// argument into registers, and when the cache holds that builtin for the receiver's class (bi_hit:
// CallCache.bi, set by fill_cc, and the method serial), runs the body here.  Anything else, including a
// body that declines (None: a Bignum, a frozen Array, ...), stages and takes the call out of line.
// The bodies must not raise, call Ruby or look at a frame: there is no cfunc frame around them.
#![forbid(unsafe_code)]
use crate::heap::{float_val, is_float, obj_flags, obj_klass};
use crate::object::FL_FROZEN;
use crate::symbol::intern;
use crate::value::*;
use crate::symbol::s;
use crate::vm::{lookup_method, method_serial, vm, CallCache, Ctx, MKind, MethodEntry, SITE_CFUNC};

// receiver classes of the table
const C_INT: u8 = 0;
const C_FLO: u8 = 1;
const C_ARY: u8 = 2;
const C_HASH: u8 = 3;
const C_STR: u8 = 4;
const C_RANGE: u8 = 5;
use crate::inline_iter::*;

/// (class, name, body): `body` is what a site bakes as `bi` (bi_body's arm).
/// Several names may share a body (`size` / `length`).
const TABLE: [(u8, &str, u32); 44] = [
    (C_INT, "even?", 1), (C_INT, "odd?", 2), (C_INT, "zero?", 3), (C_INT, "-@", 4), (C_INT, "abs", 5),
    (C_INT, "to_i", 6), (C_INT, "to_int", 6), (C_INT, "succ", 7), (C_INT, "pred", 8), (C_INT, "to_f", 9),
    (C_FLO, "to_i", 10),
    (C_ARY, "size", 11), (C_ARY, "length", 11), (C_ARY, "empty?", 12), (C_ARY, "first", 13), (C_ARY, "last", 14),
    (C_ARY, "push", 15),
    (C_HASH, "size", 16), (C_HASH, "length", 16), (C_HASH, "empty?", 17),
    (C_STR, "length", 18), (C_STR, "size", 18), (C_STR, "bytesize", 19), (C_STR, "empty?", 20),
    (C_FLO, "to_int", 10), (C_INT, "[]", 21),
    // iterators (inline_iter.rs): node_call_blk sites only
    (C_INT, "times", IT_TIMES), (C_INT, "upto", IT_UPTO), (C_INT, "downto", IT_DOWNTO), (C_INT, "step", IT_STEP),
    (C_RANGE, "each", IT_RANGE_EACH), (C_RANGE, "map", IT_RANGE_MAP), (C_RANGE, "collect", IT_RANGE_MAP),
    (C_ARY, "each", IT_ARY_EACH), (C_ARY, "map", IT_ARY_MAP), (C_ARY, "collect", IT_ARY_MAP),
    (C_ARY, "select", IT_ARY_SELECT), (C_ARY, "filter", IT_ARY_SELECT), (C_ARY, "reject", IT_ARY_REJECT),
    (C_ARY, "inject", IT_ARY_INJECT), (C_ARY, "reduce", IT_ARY_INJECT), (C_ARY, "each_with_index", IT_ARY_EACH_WI),
    (C_HASH, "each", IT_HASH_EACH), (C_HASH, "each_pair", IT_HASH_EACH),
];

// cache_key marks: the receivers of the cached class are fixnums / flonums
const K_INT: u32 = 1 << 8;
const K_FLO: u32 = 2 << 8;
/// Integer#[] (an index site's out-of-line path tests for it, srb_aref_out)
pub const BI_INT_AREF: u32 = 21;
/// cache_key of Kernel#send / BasicObject#__send__ (vm.rs send_fast); no body here.  Not public_send:
/// CRuby keeps its frame on the backtrace.
pub const BI_SEND: u32 = 1 << 10;

/// The receivers body b expects: C_INT (fixnums), C_FLO (flonums), else any heap object.
#[inline(always)]
const fn recv_of(b: u32) -> u8 { match b { 1..=9 | BI_INT_AREF | IT_TIMES..=IT_STEP => C_INT, 10 => C_FLO, _ => C_ARY } }

/// The builtins' method entries as boot defined them (an entry is replaced, never changed, by a
/// redefinition or alias), with their bodies.  Function pointers are not compared: LLVM may merge
/// identical functions, so the same builtin can have two addresses.
static BI_ME: std::sync::OnceLock<Vec<(usize, u32, Value)>> = std::sync::OnceLock::new();
static SEND_ME: std::sync::OnceLock<[(usize, u32); 2]> = std::sync::OnceLock::new();

/// After the builtin classes are made, before the prelude runs.
pub fn init(c: &Ctx) {
    let v = c.vm();
    let t = TABLE.iter().filter_map(|&(k, name, body)| {
        let klass = [v.c_integer, v.c_float, v.c_array, v.c_hash, v.c_string, v.c_range][k as usize];
        let m = c.method_entry(klass, intern(name.as_bytes()))?;
        // an iterator may be inherited (Numeric#step, Enumerable#map): the class is then checked in cache_key
        (matches!(m.kind, MKind::Cfunc(..)) && (m.owner == klass || body >= IT_FIRST)).then_some((m as *const MethodEntry as usize, body, klass))
    }).collect();
    let _ = BI_ME.set(t);
    let me = |k: Value, n: &str| c.method_entry(k, intern(n.as_bytes())).map_or(0, |m| m as *const MethodEntry as usize);
    let _ = SEND_ME.set([(me(v.m_kernel, "send"), BI_SEND), (me(v.c_basic_object, "__send__"), BI_SEND)]);
}

/// CallCache.bi of a cache filled with entry me for class klass: its body and the mark of klass (0: none).
pub fn cache_key(me: *const MethodEntry, klass: Value) -> u32 {
    if let Some(&(_, k)) = SEND_ME.get().and_then(|t| t.iter().find(|&&(m, _)| m != 0 && m == me as usize)) { return k; }
    let Some(&(_, body, _)) = BI_ME.get().and_then(|t| t.iter().find(|&&(m, b, k)| m == me as usize && (k == klass || b < IT_FIRST))) else { return 0 };
    let v = vm();
    // an iterator on an Array / Range / Hash: only that class itself (a subclass may have its own #each),
    // and Range#map only while Range#each is the builtin (its loop is Range#each's)
    if body >= IT_FIRST && recv_of(body) != C_INT {
        if body == IT_RANGE_MAP && !std::ptr::eq(lookup_method(v.c_range, s::each), v.ids.r_each) { return 0; }
        return body;
    }
    body | if klass == v.c_integer { K_INT } else if klass == v.c_float { K_FLO } else { 0 }
}

/// argc a body takes
fn body_argc(b: u32) -> usize { if b == 15 || b == BI_INT_AREF { 1 } else { 0 } }

/// Bake time: the builtin a site's cache holds, if it takes this argc (0: none).
pub fn site_builtin(cc: &CallCache, argc: usize, kind: u32) -> u32 {
    let b = cc.bi & 0xff;
    if std::env::var_os("SARUBY_NO_BI").is_some() { return 0; }
    if kind & 15 != SITE_CFUNC || b == 0 || b >= IT_FIRST || body_argc(b) != argc { return 0; }
    // the receivers the body expects (Integer / Float / any heap object) are the cached class's
    let ok = match recv_of(b) { C_INT => cc.bi & K_INT != 0, C_FLO => cc.bi & K_FLO != 0, _ => cc.bi & (K_INT | K_FLO) == 0 };
    if ok { b } else { 0 }
}

/// The cache holds builtin `bi` (a constant in baked code) for r's class: one tag test and one compare
/// for an immediate receiver, the class and one compare for a heap one; then the serial.
#[inline(always)]
pub fn bi_hit(cc: &CallCache, r: Value, bi: u32) -> bool {
    let k = match recv_of(bi) {
        C_INT => r.is_fixnum() && cc.bi == bi | K_INT,
        C_FLO => r.is_flonum() && cc.bi == bi | K_FLO,
        _ => r.is_heap() && obj_klass(r).raw() == cc.klass && cc.bi == bi,
    };
    k && cc.serial == method_serial()
}

/// The body of builtin `bi` (a constant in baked code) on receiver r (bi_hit: of the class the body
/// expects); None: call the builtin.  Most are the builtin's cfunc itself, called directly (its
/// code inlined here): those that neither raise nor call Ruby without a frame.
#[inline(always)]
pub fn bi_body(c: &mut Ctx, bi: u32, r: Value, a0: Value) -> Option<Value> {
    use crate::builtins::{array_methods as am, hash_methods as hm, numeric_methods as nm, string_methods as sm};
    let nb = std::ptr::null();
    let fix = r.is_fixnum();
    match bi {
        1 => nm::int_even(c, r, &[], nb).ok(),
        2 => nm::int_odd(c, r, &[], nb).ok(),
        3 => nm::int_zero(c, r, &[], nb).ok(),
        4 => nm::int_uminus(c, r, &[], nb).ok(),
        5 => nm::int_abs(c, r, &[], nb).ok(),
        6 => nm::int_to_i(c, r, &[], nb).ok(),
        // Integer#succ / #pred are the generic int_arith: the fixnum case here, the rest in the builtin
        7 => if fix { r.fix_add(Value::fix(1)) } else { None },
        8 => if fix { r.fix_sub(Value::fix(1)) } else { None },
        9 => nm::int_to_f(c, r, &[], nb).ok(),
        // Float#to_i raises for NaN and the infinities (a frame for the backtrace): the finite case here
        10 => {
            if !is_float(r) { return None; }
            // the open interval truncates into the fixnum range; false for NaN and the infinities, which raise
            let f = float_val(r);
            const L: f64 = (1u64 << 62) as f64;
            if f > -L && f < L { Some(Value::fix(f as i64)) } else { None }
        }
        11 => am::a_length(c, r, &[], nb).ok(),
        12 => am::a_empty(c, r, &[], nb).ok(),
        13 => am::a_first(c, r, &[], nb).ok(),
        14 => am::a_last(c, r, &[], nb).ok(),
        // a frozen Array raises (in the builtin, with its frame)
        15 => if obj_flags(r) & FL_FROZEN == 0 { am::a_push(c, r, &[a0], nb).ok() } else { None },
        16 => hm::h_size(c, r, &[], nb).ok(),
        17 => hm::h_empty(c, r, &[], nb).ok(),
        18 => sm::s_length(c, r, &[], nb).ok(),
        19 => sm::s_bytesize(c, r, &[], nb).ok(),
        20 => sm::s_empty(c, r, &[], nb).ok(),
        // Integer#[] takes ranges and two arguments too: one fixnum index here.
        // bit i of a fixnum (two's complement, so the sign beyond bit 62); 0 for a negative i
        BI_INT_AREF => (fix && a0.is_fixnum()).then(|| {
            let i = a0.as_fix();
            Value::fix(if i < 0 { 0 } else { (r.as_fix() >> i.min(63)) & 1 })
        }),
        _ => None,
    }
}
