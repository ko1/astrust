// This file is auto-generated from node.def by astrustgen. Do not edit.
// astrogre match nodes (continuation-passing).  Every node takes the current position, checks
// its piece, and calls `next` with the new position.  Result: FAIL (0), OK (1), ABORT (2: the
// loop recursion budget ran out; the search is redone on the iterative VM).  A node undoes its
// own logged side effects when its continuation fails.
use super::*;
use super::cps::*;
use super::node::*;

// ---- terminals --------------------------------------------------------------------




// ---- characters -------------------------------------------------------------------





// Greedy/lazy loop over a single-char matcher: iterates inside the node (no nesting).
// nb >= 0: the continuation starts with that literal byte, so only those positions are tried.

// ---- zero-width ---------------------------------------------------------------------



// ---- groups and backrefs ------------------------------------------------------------

// Backtrackable group start (Onigmo MEMORY_START_PUSH): logs the old span, unsets the end.

// Group start outside alternation/variable repeat/negation (Onigmo MEMORY_START): not undone.

// Group end (Onigmo MEMORY_END): never undone, so backrefs can see a stale end.


// ---- subexpression calls (only in patterns with recursive calls or level backrefs) -------
// Captures there pair each close with its own open across recursion (Onigmo's
// MEMORY_START_PUSH / MEMORY_END_PUSH_REC), through the mark stack c.mstk.



// \g<g>: runs group g's subroutine; its node_re_ret continues with `next`.




// ---- choice and repetition ----------------------------------------------------------


// x? : `body` already ends with the continuation, `skip` is the continuation itself.

// General loop; body's tail is node_re_rep_cont.  body/outer are entered through the frame.


// Lookahead / lookbehind / atomic group: `body` ends with node_re_sub_succ(_at).

// ---- scanners: the search loop as a node, so it fuses with the pattern under AOT -----------
// `pos` is the search start; each candidate resets the per-attempt state.  A match starting
// at p must contain the required literals at or after p (c.must_next / Ctx::must_refresh).






#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_succ(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    c.slots[1] = pos;
    OK
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_sub_succ(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    c.sub_end = pos;
    OK
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_sub_succ_at(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    if pos == c.lb_target { OK } else { FAIL }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_byte<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, b: u8, next: NodePtr) -> u32 {
    let s = c.subj();
    if pos < s.len() && s[pos] == b {
        let np = pos + 1;
        <D_next as Disp>::call(c, next, np)
    } else {
        FAIL
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_lit<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, bytes: Bytes, next: NodePtr) -> u32 {
    let s = c.subj();
    let l = bytes.len();
    if s.len() - pos >= l && &s[pos..pos + l] == bytes {
        let np = pos + l;
        <D_next as Disp>::call(c, next, np)
    } else {
        FAIL
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_one<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, kind: u32, ch: u32, cls: ClassRef, next: NodePtr) -> u32 {
    match one_k(c.enc, kind, ch, cls, c.subj(), pos) {
        Some(np) => <D_next as Disp>::call(c, next, np),
        None => FAIL,
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_grapheme<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, next: NodePtr) -> u32 {
    match grapheme_k(c, pos) {
        Some(np) => <D_next as Disp>::call(c, next, np),
        None => FAIL,
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_rep1<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, kind: u32, ch: u32, cls: ClassRef, min: u32, max: u32, greedy: bool, nb: i32, next: NodePtr) -> u32 {
    let s = c.subj();
    let e = c.enc;
    let mut p = pos;
    let mut cnt = 0u32;
    while cnt < min {
        match one_k(e, kind, ch, cls, s, p) {
            Some(np) => { p = np; cnt += 1; }
            None => return FAIL,
        }
    }
    if greedy {
        let minpos = p;
        let rem = if max == INF { INF } else { max - min };
        let maxpos = scan_k(e, kind, ch, cls, s, p, rem);
        let mut q = if nb >= 0 {
            match find_back(s, minpos, maxpos, nb as u8) { Some(q) => q, None => return FAIL }
        } else {
            maxpos
        };
        loop {
            let r = <D_next as Disp>::call(c, next, q);
            if r != FAIL { return r; }
            if q <= minpos { return FAIL; }
            q = if nb >= 0 {
                match find_back(s, minpos, q - 1, nb as u8) { Some(x) => x, None => return FAIL }
            } else {
                prev_start_e(e, s, q, minpos)
            };
        }
    } else {
        let (mut q, mut k) = match lazy_advance_k(e, kind, ch, cls, s, p, cnt, max, nb) { Some(x) => x, None => return FAIL };
        loop {
            let r = <D_next as Disp>::call(c, next, q);
            if r != FAIL { return r; }
            if k >= max { return FAIL; }
            let np = match one_k(e, kind, ch, cls, s, q) { Some(x) => x, None => return FAIL };
            match lazy_advance_k(e, kind, ch, cls, s, np, k + 1, max, nb) {
                Some((a, b)) => { q = a; k = b; }
                None => return FAIL,
            }
        }
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_assert<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, kind: u32, next: NodePtr) -> u32 {
    if assert_k(kind, c, pos) { <D_next as Disp>::call(c, next, pos) } else { FAIL }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_keep<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, next: NodePtr) -> u32 {
    let mark = c.log.len();
    c.set_slot(0, pos);
    let r = <D_next as Disp>::call(c, next, pos);
    if r == FAIL { c.undo(mark); }
    r
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_open<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, g: u32, next: NodePtr) -> u32 {
    let mark = c.log.len();
    let k = 2 * g as usize;
    c.set_slot(k, pos);
    c.set_slot(k + 1, NONE);
    let r = <D_next as Disp>::call(c, next, pos);
    if r == FAIL { c.undo(mark); }
    r
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_open_nr<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, g: u32, next: NodePtr) -> u32 {
    let k = 2 * g as usize;
    c.slots[k] = pos;
    c.slots[k + 1] = NONE;
    <D_next as Disp>::call(c, next, pos)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_close<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, g: u32, next: NodePtr) -> u32 {
    c.slots[2 * g as usize + 1] = pos;
    <D_next as Disp>::call(c, next, pos)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_backref<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, groups: Groups, icase: bool, next: NodePtr) -> u32 {
    match backref_k(c, groups, icase, pos) {
        Some(np) => {
            let mark = c.log.len();
            if np == pos && c.sprev_bug != pos { c.set_sprev(pos); }
            let r = <D_next as Disp>::call(c, next, np);
            if r == FAIL { c.undo(mark); }
            r
        }
        None => FAIL,
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_open_rec<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, g: u32, next: NodePtr) -> u32 {
    let mark = c.log.len();
    open_rec_k(c, g as usize, pos);
    let r = <D_next as Disp>::call(c, next, pos);
    if r == FAIL { c.undo(mark); }
    r
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_close_rec<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, g: u32, next: NodePtr) -> u32 {
    let mark = c.log.len();
    close_rec_k(c, g as usize, pos);
    let r = <D_next as Disp>::call(c, next, pos);
    if r == FAIL { c.undo(mark); }
    r
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_call<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, g: u32, next: NodePtr) -> u32 {
    call_k(c, g as usize, next, pos)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_ret(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    ret_k(c, pos)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_backref_lv<D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, groups: Groups, icase: bool, nest: i32, next: NodePtr) -> u32 {
    match backref_lv_k(c, groups, icase, nest, pos) {
        Some(np) => <D_next as Disp>::call(c, next, np),
        None => FAIL,
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_cond<D_yes: Disp, D_no: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, groups: Groups, yes: NodePtr, no: NodePtr) -> u32 {
    if cond_k(c, groups) { <D_yes as Disp>::call(c, yes, pos) } else { <D_no as Disp>::call(c, no, pos) }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_alt<D_a: Disp, D_b: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, a: NodePtr, b: NodePtr) -> u32 {
    let r = <D_a as Disp>::call(c, a, pos);
    if r != FAIL { return r; }
    <D_b as Disp>::call(c, b, pos)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_opt<D_body: Disp, D_skip: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, body: NodePtr, skip: NodePtr, greedy: bool) -> u32 {
    if greedy {
        let r = <D_body as Disp>::call(c, body, pos);
        if r != FAIL { return r; }
        <D_skip as Disp>::call(c, skip, pos)
    } else {
        let r = <D_skip as Disp>::call(c, skip, pos);
        if r != FAIL { return r; }
        <D_body as Disp>::call(c, body, pos)
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_rep<D_body: Disp, D_outer: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, body: NodePtr, outer: NodePtr, min: u32, max: u32, greedy: bool, mem: bool, shape: u32, cost: u32) -> u32 {
    rep_enter(c, body, outer, min, max, greedy, mem, shape, cost, pos)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_rep_cont(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    rep_cont(c, pos)
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_re_look<D_body: Disp, D_next: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, kind: u32, body: NodePtr, next: NodePtr, minlen: u32, maxlen: u32) -> u32 {
    let mark = c.log.len();
    if kind == LK_AHEAD || kind == LK_ATOMIC {
        let sprev0 = c.sprev_bug;
        let r = <D_body as Disp>::call(c, body, pos);
        if r != OK { return r; }
        // Onigmo's POP_POS restores the previous-char pointer after a lookahead.
        if kind == LK_AHEAD && c.sprev_bug != sprev0 { c.set_sprev(sprev0); }
        let np = if kind == LK_ATOMIC { c.sub_end } else { pos };
        let r = <D_next as Disp>::call(c, next, np);
        if r == FAIL { c.undo(mark); }
        return r;
    }
    if kind == LK_NEG_AHEAD {
        let r = <D_body as Disp>::call(c, body, pos);
        if r == ABORT { return r; }
        if r == OK { c.undo(mark); return FAIL; }
        return <D_next as Disp>::call(c, next, pos);
    }
    let s = c.subj();
    let saved = c.lb_target;
    c.lb_target = pos;
    let mut found = false;
    let mut q = pos;
    let mut k = 0u32;
    loop {
        if k >= minlen {
            let r = <D_body as Disp>::call(c, body, q);
            if r == ABORT { c.lb_target = saved; return r; }
            if r == OK { found = true; break; }
        }
        if k >= maxlen || q == 0 { break; }
        q = prev_start_e(c.enc, s, q, 0);
        k += 1;
    }
    c.lb_target = saved;
    if kind == LK_BEHIND {
        if !found { return FAIL; }
        let r = <D_next as Disp>::call(c, next, pos);
        if r == FAIL { c.undo(mark); }
        r
    } else {
        if found { c.undo(mark); return FAIL; }
        <D_next as Disp>::call(c, next, pos)
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_search<D_body: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, body: NodePtr, anchor: u32) -> u32 {
    let s = c.subj();
    let len = s.len();
    let start = pos;
    let mut p = pos;
    loop {
        if p > c.must_next && !c.must_refresh(p) { return FAIL; }
        if anchor == AN_STR_BEGIN {
            if p != 0 { return FAIL; }
        } else if anchor == AN_START {
            if p != start { return FAIL; }
        } else if anchor == AN_LINE && p != start && s[p - 1] != b'\n' {
            match memchr_from(s, b'\n', p) { Some(k) => p = k + 1, None => return FAIL }
            if p > c.must_next && !c.must_refresh(p) { return FAIL; }
        }
        c.reset(p);
        let r = <D_body as Disp>::call(c, body, p);
        if r != FAIL { return r; }
        if p >= len { return FAIL; }
        p += char_len_e(c.enc, s, p);
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_search_memchr<D_body: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, body: NodePtr, b: u8) -> u32 {
    let s = c.subj();
    let mut p = pos;
    loop {
        if p > c.must_next && !c.must_refresh(p) { return FAIL; }
        match memchr_from(s, b, p) { Some(q) => p = q, None => return FAIL }
        if p > c.must_next && !c.must_refresh(p) { return FAIL; }
        c.reset(p);
        let r = <D_body as Disp>::call(c, body, p);
        if r != FAIL { return r; }
        p += char_len_e(c.enc, s, p);
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_search_memmem<D_body: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, body: NodePtr, needle: Bytes) -> u32 {
    let s = c.subj();
    let mut p = pos;
    loop {
        if p > c.must_next && !c.must_refresh(p) { return FAIL; }
        match find_from(s, needle, p) { Some(q) => p = q, None => return FAIL }
        if p > c.must_next && !c.must_refresh(p) { return FAIL; }
        c.reset(p);
        let r = <D_body as Disp>::call(c, body, p);
        if r != FAIL { return r; }
        p += char_len_e(c.enc, s, p);
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_search_byteset<D_body: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, body: NodePtr, set: ByteSet) -> u32 {
    let s = c.subj();
    let len = s.len();
    let fs = unsafe { &*set };
    let mut p = pos;
    loop {
        if p > c.must_next && !c.must_refresh(p) { return FAIL; }
        while p < len && !fs[s[p] as usize] { p += char_len_e(c.enc, s, p); }
        if p >= len || (p > c.must_next && !c.must_refresh(p)) { return FAIL; }
        c.reset(p);
        let r = <D_body as Disp>::call(c, body, p);
        if r != FAIL { return r; }
        p += char_len_e(c.enc, s, p);
    }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_match_at<D_body: Disp>(c: &mut Ctx, n: NodePtr, pos: usize, body: NodePtr) -> u32 {
    c.reset(pos);
    <D_body as Disp>::call(c, body, pos)
}

