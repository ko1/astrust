// $~ and $_: local to a method's frame, shared with its blocks (CRuby's svar), at no cost per call.
//
// Each context (fiber) keeps its entries in one hidden Array in Ctx.last_match:
// [frame address, home id, $~, $_] per home frame that wrote one.  A frame's `pad` bit SVAR_SET says
// whether this activation wrote: unset, it reads nil and its first write replaces whatever entry a
// dead frame left at the same address.  Entries of frames below the innermost one are dropped on a
// write (they returned).  A block finds its home on the frame chain by home id; a home that is not
// live there (an escaped proc, a block run by another fiber or thread) is keyed by id alone.
// Builtin (cfunc, prelude, ARGF) and eval frames are transparent, but an eval in a Binding sees the binding's home (a
// redirect entry [eval frame, -1, home id, nil] while it runs); a define_method body is its own home.
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
use crate::value::*;
use crate::vm::*;

const SVAR_SET: u8 = 1;
const W: usize = 4;          // words per entry
const MAX_ESCAPED: usize = 16;
const PRUNE_AT: usize = 16;   // entries
fn redirect_tag() -> Value { Value::fix(-1) }

/// The frame whose $~ / $_ the running code sees: (its address or 0, home id, whether this
/// activation has not written yet).  `mark`: it writes now.
#[inline(always)]
fn home(c: &Ctx, mark: bool) -> (usize, u64, bool) {
    let mut f = c.cur_frame();
    while let Some(x) = f {
        if x.kind != FK_CFUNC && builtin_code(x) { f = c.prev_frame(x); continue; }
        let hid = match x.kind {
            FK_CFUNC => { f = c.prev_frame(x); continue; }
            FK_EVAL => match redirect_of(c, x) { Some(h) => h, None => { f = c.prev_frame(x); continue; } },
            FK_BLOCK | FK_LAMBDA => match block_home(c, x) { Some(h) => h, None => return own(x, mark) },
            _ => return own(x, mark),
        };
        let mut g = c.prev_frame(x);
        while let Some(y) = g { if y.id == hid && y.kind != FK_CFUNC && y.kind != FK_BLOCK { break; } g = c.prev_frame(y); }
        if g.is_none() { return (0, hid, false); }
        f = g;   // a lambda or eval home resolves further
    }
    (0, 0, false)
}
/// Ruby code standing in for CRuby's C methods (the prelude, ARGF) sets and reads its caller's.
#[inline(always)]
fn builtin_code(x: &Frame) -> bool {
    let f = frame_file(x);
    f.as_bytes().first() == Some(&b'<') && (f == "<internal:prelude>" || f == "<internal:saruby/argf>")
}
/// The home id of a block frame; None for a define_method body (its own home).
fn block_home(c: &Ctx, x: &Frame) -> Option<u64> {
    let b = c.block_copy(x.self_blk());
    if b.is_some_and(|b| b.me != x.me) { return None; }
    Some(b.map_or(x.id, |b| b.home_id))
}
/// The svar home id a Binding of frame `f` keeps (binding.rs).
pub fn frame_home_id(c: &Ctx, f: &Frame) -> u64 {
    if matches!(f.kind, FK_BLOCK | FK_LAMBDA) { block_home(c, f).unwrap_or(f.id) } else { f.id }
}
/// Is home `hid`'s frame on the chain?
pub fn home_live(c: &Ctx, hid: u64) -> bool {
    let mut g = c.cur_frame();
    while let Some(y) = g { if y.id == hid && y.kind != FK_CFUNC && y.kind != FK_BLOCK { return true; } g = c.prev_frame(y); }
    false
}
/// Binding#evals running (any context): none, and an eval frame needs no lookup.
static REDIRECTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
fn redirect_of(c: &Ctx, x: &Frame) -> Option<u64> {
    if REDIRECTS.load(std::sync::atomic::Ordering::Relaxed) == 0 { return None; }
    let i = find(c, c.last_match, x as *const Frame as usize, u64::MAX)?;
    Some(c.ary(c.last_match)[i * W + 2].as_fix() as u64)
}
/// Binding#eval: the running eval frame sees home `hid`'s $~ / $_ (0: none) until `end_redirect`.
pub fn redirect(c: &mut Ctx, hid: u64) {
    if hid == 0 { return; }
    let addr = c.cur_frame().map_or(0, |f| f as *const Frame as usize);
    if c.last_match == NIL { let t = c.ary_new(); c.last_match = t; }
    let t = c.last_match;
    let e = [Value::fix(addr as i64), redirect_tag(), Value::fix(hid as i64), NIL];
    c.ary_mut(t).extend_from_slice(&e);
    REDIRECTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}
pub fn end_redirect(c: &mut Ctx, hid: u64) {
    if hid == 0 { return; }
    let addr = c.cur_frame().map_or(0, |f| f as *const Frame as usize);
    let t = c.last_match;
    if let Some(i) = find(c, t, addr, u64::MAX) { let a = c.ary_mut(t); a.drain(i * W..i * W + W); }
    REDIRECTS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
}
fn own(x: &Frame, mark: bool) -> (usize, u64, bool) {
    let p = x.pad.get();
    if mark { x.pad.set(p | SVAR_SET); }
    (x as *const Frame as usize, x.id, p & SVAR_SET == 0)
}
fn find(c: &Ctx, t: Value, addr: usize, id: u64) -> Option<usize> {
    if t == NIL { return None; }
    let a = c.ary(t);
    let (fa, fi) = (Value::fix(addr as i64), Value::fix(id as i64));
    (0..a.len() / W).rev().find(|&i| a[i * W] == fa && a[i * W + 1] == fi)
}

#[inline(always)]
fn get(c: &Ctx, which: usize) -> Value {
    let (addr, id, fresh) = home(c, false);
    if fresh { return NIL; }
    let t = c.last_match;
    if t == NIL { return NIL; }
    let a = c.ary(t);
    let n = a.len();
    if n >= W && a[n - W] == Value::fix(addr as i64) && a[n - W + 1] == Value::fix(id as i64) { return a[n - W + which]; }
    match find(c, t, addr, id) { Some(i) => c.ary(t)[i * W + which], None => NIL }
}

#[inline(always)]
fn set(c: &mut Ctx, which: usize, v: Value) {
    let (addr, id, fresh) = home(c, true);
    let t = c.last_match;
    let (fa, fi) = (Value::fix(addr as i64), Value::fix(id as i64));
    if t != NIL {
        // the common case: the last entry is this frame's (or a returned one's at its address)
        let a = c.ary_mut(t);
        let n = a.len();
        if n >= W && a[n - W] == fa && (fresh || a[n - W + 1] == fi) {
            let e = &mut a[n - W..];
            if fresh { e[1] = fi; e[2] = NIL; e[3] = NIL; }
            e[which] = v;
            return;
        }
    }
    set_slow(c, which, v, addr, id, fresh)
}
#[inline(never)]
fn set_slow(c: &mut Ctx, which: usize, v: Value, addr: usize, id: u64, fresh: bool) {
    let mut t = c.last_match;
    let (fa, fi) = (Value::fix(addr as i64), Value::fix(id as i64));
    if t == NIL { if fresh && v == NIL { return; } t = c.ary_new(); c.last_match = t; }
    // entries are pushed in call order: the returned frames' are at the end
    let low = c.cur_frame().map_or(0, |f| f as *const Frame as usize);
    let a = c.ary_mut(t);
    let mut n = a.len();
    while n >= W && { let ea = a[n - W].as_fix() as usize; ea != 0 && ea < low } { n -= W; }
    a.truncate(n);
    let a = c.ary(t);
    // a fresh activation takes over the entry a returned one left at its address
    let i = if fresh { (0..a.len() / W).rev().find(|&i| a[i * W] == fa && a[i * W + 1] != redirect_tag()) } else { find(c, t, addr, id) };
    let i = match i {
        Some(i) => { if fresh { let e = &mut c.ary_mut(t)[i * W..i * W + W]; e[1] = fi; e[2] = NIL; e[3] = NIL; } i }
        None => {
            if a.len() >= PRUNE_AT * W { prune(c, t); }
            c.ary_mut(t).extend_from_slice(&[fa, fi, NIL, NIL]);
            c.ary_len(t) / W - 1
        }
    };
    c.ary_mut(t)[i * W + which] = v;
}
/// Drop the entries of frames below the innermost one (they returned) and the oldest escaped ones.
fn prune(c: &mut Ctx, t: Value) {
    let low = c.cur_frame().map_or(0, |f| f as *const Frame as usize);
    let a = c.ary(t);
    let mut keep: Vec<Value> = Vec::with_capacity(a.len());
    let mut escaped = 0;
    for e in a.chunks(W).rev() {
        let ea = e[0].as_fix() as usize;
        if ea != 0 && ea < low { continue; }
        if ea == 0 { escaped += 1; if escaped > MAX_ESCAPED { continue; } }
        keep.extend_from_slice(e);
    }
    let out: Vec<Value> = keep.chunks(W).rev().flatten().copied().collect();
    let av = c.ary_mut(t);
    av.clear();
    av.extend_from_slice(&out);
}

/// $~
#[inline(never)] pub fn last_match(c: &Ctx) -> Value { get(c, 2) }
#[inline(never)] pub fn set_last_match(c: &mut Ctx, v: Value) { set(c, 2, v) }
/// $_
#[inline(never)] pub fn last_line(c: &Ctx) -> Value { get(c, 3) }
pub fn set_last_line(c: &mut Ctx, v: Value) { set(c, 3, v) }
