// Builtin iterators run inline at baked block call sites (node_call_blk's `it`).
//
// A `recv.m(args) { |..| body }` site whose cache held one of the iterators below (identified as
// inline_builtin.rs identifies builtins: by the method entry boot defined, CallCache.bi) is baked
// with the iterator's number and the block's entry compiled in (`body@spec`).  When the cache
// still holds that builtin for the receiver's class, node_call_blk runs the builtin's own loop
// (iter_loop: the `*_loop` function the cfunc itself runs, generic over how it yields), with a
// yield that puts the step's arguments in the block's frame and calls the block entry directly
// (no yield1, no dispatcher).  So the iterator is written once, in the builtin, and the baked site
// only specializes its yield.  The loop runs under the builtin's cfunc frame (backtraces name it,
// as for the out-of-line call).  A receiver or argument the loop does not take (a Bignum, a Float
// limit, an endless Range ...) gives None before anything ran, and the site calls the builtin out
// of line.  The loops neither raise nor stage on the VM stack between steps (IT_SHARE relies on it).
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
    let b = cc.bi.get() & 0xff;
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
pub fn block_frame(b: &Block, bfp: Fp, prev: *mut Frame, is: &'static Iseq) -> FrameFull {
    bfp[0].set(b.self_);
    bfp[1].set(b.env);
    FrameFull {
        h: Frame { prev, fp: head(bfp), me: b.me, blk: b.outer_blk, id: b.home_id, pos: crate::vm::Pos::at(&is.info.line), kind: FK_BLOCK, visi: VISI_PUBLIC, lite: false, pad: std::cell::Cell::new(0) },
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

/// One step's arguments in the block's frame at `bb`: slots 2.., missing parameters nil, sp at its base.
#[inline(always)]
pub fn put_args(c: &mut Ctx, bb: Stk, bfp: Fp, np: usize, a: &[Value]) {
    let m = a.len().min(np);
    let mut i = 0;
    while i < m { bfp[2 + i].set(a[i]); i += 1; }
    while i < np { bfp[2 + i].set(NIL); i += 1; }
    c.set_sp(bb, 0);
}

/// The builtin's own loop for iterator `it` (baked) on r0 with the site's arguments `a`, its block called
/// through `y`; None: a case that loop does not take (nothing has run), which the site calls out of line.
#[inline(always)]
pub fn iter_loop(c: &mut Ctx, it: u32, r0: Value, a: &[Value], y: &mut impl FnMut(&mut Ctx, &[Value]) -> RV) -> Option<RV> {
    use crate::builtins::{array_methods as am, enum_native as en, hash_methods as hm, numeric_methods as nm, range_methods as rm};
    match it & 0xff {
        IT_TIMES => nm::int_times_loop(c, r0, y),
        IT_UPTO => nm::int_upto_loop(c, r0, a[0], 1, y),
        IT_DOWNTO => nm::int_upto_loop(c, r0, a[0], -1, y),
        IT_STEP => nm::num_step_loop(c, r0, a[0], a[1], y),
        IT_RANGE_EACH => rm::r_each_loop(c, r0, y),
        IT_RANGE_MAP => en::en_map_range_loop(c, r0, y),
        IT_ARY_EACH => Some(am::a_each_loop(c, r0, y)),
        IT_ARY_MAP => Some(am::a_map_loop(c, r0, y)),
        IT_ARY_SELECT => Some(am::a_select_loop(c, r0, true, y)),
        IT_ARY_REJECT => Some(am::a_select_loop(c, r0, false, y)),
        IT_ARY_INJECT => am::a_inject_loop(c, r0, a, y),
        IT_ARY_EACH_WI => Some(am::a_each_wi_loop(c, r0, y)),
        IT_HASH_EACH => Some(hm::h_each_loop(c, r0, y)),
        _ => None,
    }
}

/// The loop's guard missed.  An empty or stale cache (first run, a redefinition) is filled without
/// calling, so that this run already takes the loop; a cache holding another class (a polymorphic
/// site) is left alone.  The block is not in the tree's hash, so sites with other blocks share the
/// baked code: the entry's hash `h` must be this site's, remembered in cc.fast_hash (0 for a cfunc
/// entry, reset by every fill).
#[inline(never)]
pub fn iter_refill(c: &mut Ctx, cc: &CallCache, r0: Value, mid: Sym, flags: u32, it: u32, blk: &Iseq, h: u64) -> bool {
    if !crate::inline_builtin::bi_hit(cc, r0, it & 0xff) {
        if cc.klass.get() != 0 && cc.serial.get() == crate::vm::method_serial() { return false; }
        crate::vm::lookup_cc(c, cc, r0, mid, flags);
        if !crate::inline_builtin::bi_hit(cc, r0, it & 0xff) { return false; }
    }
    if crate::node::hash_node(blk.bentry) != h { return false; }
    cc.fast_hash.set(h);
    true
}
