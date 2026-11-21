// Builtin iterators run inline at baked block call sites (node_call_blk's `it`).
//
// A `recv.m(args) { |..| body }` site whose cache held one of the iterators below (identified as
// inline_builtin.rs identifies builtins: by the method entry boot defined, CallCache.bi) is baked
// with the iterator's number and the block's entry compiled in (`body@spec`).  When the cache
// still holds that builtin for the receiver's class, node_call_blk runs the loop: iter_next puts
// a step's arguments in the block's frame, the site calls the block entry directly (no yield1,
// no dispatcher), iter_step takes its value.  The loop runs under the builtin's cfunc frame
// (backtraces name it, as for the out-of-line call); the receiver and any accumulator are in the
// staged slots (rooted).  A receiver or argument the loop does not handle (a Bignum, a Float
// limit, an endless Range ...) gives None from iter_begin before anything ran, and the site
// calls the builtin out of line.
#![forbid(unsafe_code)]
use crate::stack::*;
use crate::value::*;
use crate::vm::{Block, CallCache, Ctx, Frame, FrameFull, Iseq, IseqKind, FK_BLOCK, RV, SITE_CFUNC, VISI_PUBLIC};

pub const IT_TIMES: u32 = 32;
pub const IT_UPTO: u32 = 33;
pub const IT_DOWNTO: u32 = 34;
pub const IT_STEP: u32 = 35;
pub const IT_RANGE_EACH: u32 = 36;
pub const IT_RANGE_MAP: u32 = 37;
pub const IT_ARY_EACH: u32 = 38;
pub const IT_ARY_MAP: u32 = 39;
pub const IT_ARY_SELECT: u32 = 40;
pub const IT_ARY_REJECT: u32 = 41;
pub const IT_ARY_INJECT: u32 = 42;
pub const IT_HASH_EACH: u32 = 43;
pub const IT_ARY_EACH_WI: u32 = 44;
/// inline_builtin.rs body numbers from here on are iterators (never baked at a node_call site)
pub const IT_FIRST: u32 = IT_TIMES;

/// Arguments the site passes (besides the block).
fn it_argc(it: u32, argc: usize) -> bool {
    match it {
        IT_UPTO | IT_DOWNTO => argc == 1,
        IT_STEP => argc == 2,
        IT_ARY_INJECT => argc <= 1,
        _ => argc == 0,
    }
}

/// Can a block with `np` lead parameters take what iterator `it` yields as plain slots?
fn it_params(it: u32, np: usize) -> bool {
    match it {
        // one element that may be an Array: 2+ parameters would spread it
        IT_ARY_EACH | IT_ARY_MAP | IT_ARY_SELECT | IT_ARY_REJECT => np <= 1,
        // |k, v| gets two values; |pair| an Array of them
        IT_HASH_EACH => np >= 2,
        // fixnums, or two values
        _ => true,
    }
}

/// Bake time: the iterator the site's cache holds and the block's lead parameter count, as
/// `it | np << 8` (0: none).
pub fn site_iter(cc: &CallCache, argc: usize, kind: u32, blk: &Iseq) -> u32 {
    if std::env::var_os("SARUBY_NO_IT").is_some() { return 0; }
    let b = cc.bi & 0xff;
    if kind & 15 != SITE_CFUNC || b < IT_FIRST || !it_argc(b, argc) { return 0; }
    if blk.bentry.is_null() || blk.kind == IseqKind::Lambda { return 0; }
    let np = blk.params.lead.len();
    if np > 8 || !it_params(b, np) { return 0; }
    // a block without env runs every step in one frame (block_frame)
    b | (np as u32) << 8 | if blk.env_size == 0 { IT_SHARE } else { 0 }
}

/// site_iter's mark: the site makes the block's frame once (block_frame), the entry runs in it.
pub const IT_SHARE: u32 = 1 << 16;

/// The block's frame for all the steps of a loop (IT_SHARE), with its self and env slots set:
/// what node_block_entry makes at every step.  The site resets `line` before each step.
#[inline(always)]
pub fn block_frame(b: &Block, bfp: Fp, prev: *mut Frame, is: &Iseq) -> FrameFull {
    bfp[0].set(b.self_);
    bfp[1].set(b.env);
    FrameFull {
        h: Frame { prev, fp: head(bfp), me: b.me, blk: b.outer_blk, id: b.home_id, line: is.info.line, kind: FK_BLOCK, visi: VISI_PUBLIC, lite: false, pad: 0 },
        cref: b.cref, iseq: std::ptr::null(), self_blk: b, defined_class: b.defined_class,
    }
}

/// Bake time: the block entry to compile into a site that site_iter accepted (null: none).
pub fn spec_iter_block(cc: &CallCache, argc: usize, kind: u32, blk: &Iseq) -> crate::node::NodePtr {
    if site_iter(cc, argc, kind, blk) == 0 { return std::ptr::null_mut(); }
    let mut d = String::new();
    crate::node::dump_node(&mut d, blk.bentry);
    if d.len() <= 600 { blk.bentry } else { std::ptr::null_mut() }
}

/// A running inline iterator: the next integer or index and its bounds, the accumulator, the
/// element of the current step, and the block's frame.
pub struct It { i: i64, hi: i64, st: i64, acc: Value, cur: Value, bb: Stk, pub bfp: Fp }

/// The block's frame for one step: the values at slots 2.., missing parameters nil, sp at its base.
#[inline(always)]
fn put_args(c: &mut Ctx, s: &It, np: usize, a: &[Value]) {
    let mut i = 0;
    while i < a.len() { s.bfp[2 + i].set(a[i]); i += 1; }
    while i < np { s.bfp[2 + i].set(NIL); i += 1; }
    c.set_sp(s.bb, 0);
}

/// Start iterator `it` (baked: `kind | np << 8`) on r0 with the arguments staged at base[2..2+argc].
/// None: not a case the loop handles (nothing has run).  An accumulator is rooted in base[1].
#[inline(always)]
pub fn iter_begin(c: &mut Ctx, it: u32, r0: Value, base: Stk, argc: usize) -> Option<It> {
    let kind = it & 0xff;
    let (mut i, hi, st) = match kind {
        IT_TIMES => (0, r0.as_fix() - 1, 1),
        IT_UPTO | IT_DOWNTO => {
            let e = base[2].get();
            if !e.is_fixnum() { return None; }
            (r0.as_fix(), e.as_fix(), if kind == IT_UPTO { 1 } else { -1 })
        }
        IT_STEP => {
            let (e, d) = (base[2].get(), base[3].get());
            if !e.is_fixnum() || !d.is_fixnum() || d == Value::fix(0) { return None; }
            (r0.as_fix(), e.as_fix(), d.as_fix())
        }
        IT_RANGE_EACH | IT_RANGE_MAP => {
            let (b, e, x) = c.range(r0);
            if !b.is_fixnum() || !e.is_fixnum() { return None; }
            (b.as_fix(), if x { e.as_fix() - 1 } else { e.as_fix() }, 1)
        }
        _ => (0, 0, 1),
    };
    let bb = c.top();
    let bfp = frame(c, bb).ok()?;
    let mut acc = NIL;
    match kind {
        IT_RANGE_MAP | IT_ARY_SELECT | IT_ARY_REJECT => { acc = c.ary_new(); base[1].set(acc); }
        IT_ARY_MAP => { let n = c.ary(r0).len(); acc = c.ary_new_capa(n); base[1].set(acc); }
        IT_ARY_INJECT => if argc == 1 { acc = base[2].get(); } else {
            // inject without an initial value: the first element, nil for an empty Array
            match c.ary(r0).first() { Some(&x) => { acc = x; i = 1; } None => { acc = UNDEF; } }
        },
        IT_HASH_EACH => c.hash_iter_enter(r0),
        _ => {}
    }
    Some(It { i, hi, st, acc, cur: NIL, bb, bfp })
}

/// Put the next step's arguments in the block's frame; false when the iteration is over.
/// Arrays and Hashes are read again at every step (the block may change them).
#[inline(always)]
pub fn iter_next(c: &mut Ctx, s: &mut It, it: u32, r0: Value) -> bool {
    let np = (it >> 8 & 0xff) as usize;
    match it & 0xff {
        // |lo|, |hi|, |st| < 2^62: i + st cannot overflow
        IT_TIMES | IT_UPTO | IT_DOWNTO | IT_STEP | IT_RANGE_EACH | IT_RANGE_MAP => {
            if if s.st > 0 { s.i > s.hi } else { s.i < s.hi } { return false; }
            put_args(c, s, np, &[Value::fix(s.i)]);
            s.i += s.st;
            true
        }
        IT_ARY_INJECT => {
            if s.acc == UNDEF { return false; }
            let Some(&x) = c.ary(r0).get(s.i as usize) else { return false };
            put_args(c, s, np, &[s.acc, x]);
            s.i += 1;
            true
        }
        IT_ARY_EACH_WI => {
            let Some(&x) = c.ary(r0).get(s.i as usize) else { return false };
            put_args(c, s, np, &[x, Value::fix(s.i)]);
            s.i += 1;
            true
        }
        IT_HASH_EACH => loop {
            let Some((k, v)) = c.hash_entry_at(r0, s.i as usize) else { return false };
            s.i += 1;
            if k != UNDEF { put_args(c, s, np, &[k, v]); return true; }
        },
        _ => {
            let Some(&x) = c.ary(r0).get(s.i as usize) else { return false };
            put_args(c, s, np, &[x]);
            s.cur = x;
            s.i += 1;
            true
        }
    }
}

/// What the block returned for the step iter_next set up.
#[inline(always)]
pub fn iter_step(c: &mut Ctx, s: &mut It, it: u32, base: Stk, y: Value) {
    match it & 0xff {
        IT_RANGE_MAP | IT_ARY_MAP => c.ary_push(s.acc, y),
        // as Array#select: the element read again after the block (nil if it is gone)
        IT_ARY_SELECT => if y.truthy() { let x = c.ary(base[0].get()).get(s.i as usize - 1).copied().unwrap_or(NIL); c.ary_push(s.acc, x); },
        IT_ARY_REJECT => if y.falsy() { c.ary_push(s.acc, s.cur); },
        // the accumulator is rooted in the staged slot while the block runs
        IT_ARY_INJECT => { s.acc = y; base[1].set(y); }
        _ => {}
    }
}

/// End of the loop (r: the first error, if any): the iterator's value; sp back at the staged call.
#[inline(always)]
pub fn iter_end(c: &mut Ctx, s: &It, it: u32, r0: Value, base: Stk, r: Result<(), crate::vm::Esc>) -> RV {
    let kind = it & 0xff;
    if kind == IT_HASH_EACH { c.hash_iter_leave(r0); }
    c.set_sp(base, 0);
    r?;
    Ok(match kind {
        IT_RANGE_MAP | IT_ARY_MAP | IT_ARY_SELECT | IT_ARY_REJECT => s.acc,
        IT_ARY_INJECT => if s.acc == UNDEF { NIL } else { s.acc },
        _ => r0,
    })
}

/// A baked iterator site whose cache is empty or stale (its first run, after a redefinition): fill
/// it without calling, so that this run already takes the loop.  A cache holding another class
/// (a polymorphic site) is left alone.
#[inline(never)]
pub fn iter_refill(c: &mut Ctx, cc: &mut CallCache, r0: Value, mid: Sym, flags: u32, it: u32) -> bool {
    if cc.klass != 0 && cc.serial == crate::vm::method_serial() { return false; }
    crate::vm::lookup_cc(c, cc, r0, mid, flags);
    crate::inline_builtin::bi_hit(cc, r0, it & 0xff)
}
