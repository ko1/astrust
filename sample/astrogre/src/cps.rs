// The primary matcher: the pattern lowered right-to-left into a tree of continuation-passing
// match nodes (node.def), wrapped in a scanner node that walks the start positions.  Every
// match node checks its piece at `pos` and calls its `next` with the new position; a failing
// continuation returns 0 and each node undoes its own side effects.
//
// Repetition (node_re_rep) keeps its loop state in a frame on the native stack and wires the
// body's tail to node_re_rep_cont, which reads the innermost frame (c.rep_top) and decides
// "another iteration / leave the loop".  Iterations therefore nest native frames, so the
// recursion is budgeted (DEPTH_LIMIT, in body-node units); when a match would exceed it, the
// whole search returns ABORT and is rerun on the iterative VM in re.rs, which has the same
// semantics.  Single-char loops (node_re_rep1) iterate inside one node and never nest.
//
// Side effects that Onigmo undoes on backtracking go through an undo log (c.log), the CPS
// counterpart of the VM's restore entries; the log is also what the empty-iteration check
// (Onigmo's NULL_CHECK_END_MEMST) inspects.
use super::node::*;
use super::*;
use crate::astrust::*;

pub type Bytes = &'static [u8];
pub type ClassRef = *const ClassBox;
pub type Groups = &'static [usize];
pub type ByteSet = *const [bool; 256];

pub struct ClassBox {
    pub(crate) cc: CClass,
    pub hash: u64,
}

pub const FAIL: u32 = 0;
pub const OK: u32 = 1;
pub const ABORT: u32 = 2;

/// Recursion budget for nested loop iterations, in body nodes (~ native frames).
pub const DEPTH_LIMIT: usize = 6000;
/// Pseudo slot indices in the undo log: Ctx::sprev_bug, and the length of Ctx::mstk.
const SPREV: usize = usize::MAX;
const MSTK: usize = usize::MAX - 1;


// single-char matcher kinds (node_re_one / node_re_rep1)
pub const K_CH: u32 = 0;
pub const K_CHIC: u32 = 1;
pub const K_ANY: u32 = 2;
pub const K_ANYNL: u32 = 3;
pub const K_CLASS: u32 = 4;

// lookaround kinds (node_re_look)
pub const LK_AHEAD: u32 = 0;
pub const LK_NEG_AHEAD: u32 = 1;
pub const LK_BEHIND: u32 = 2;
pub const LK_NEG_BEHIND: u32 = 3;
pub const LK_ATOMIC: u32 = 4;

// loop shapes (node_re_rep), mirroring the VM's code shapes
pub const SH_STAR: u32 = 0; // {0,inf}: Split(body, exit) ... EmptyCheck, Jmp
pub const SH_PLUS: u32 = 1; // {1,inf}: body, EmptyCheck, Split(top, exit)
pub const SH_COUNT: u32 = 2; // counted with a null check
pub const SH_COUNT_NC: u32 = 3; // counted without (Onigmo expands small finite repeats)

// search anchors (node_search)
pub const AN_NONE: u32 = 0;
pub const AN_STR_BEGIN: u32 = 1;
pub const AN_START: u32 = 2;
pub const AN_LINE: u32 = 3;

pub struct Ctx {
    pub s: *const u8,
    pub len: usize,
    pub slots: Vec<usize>,
    pub log: Vec<(usize, usize)>,
    pub sprev_bug: usize,
    pub gpos: usize,
    pub rep_top: *mut RepFrame,
    pub depth: usize,
    pub sub_end: usize,
    pub lb_target: usize,
    pub must: *const [MustLit],
    pub must_occ: Vec<usize>,
    pub must_next: usize,
    pub ngroups: usize,
    pub enc: Enc,
    pub mstk: Vec<MEnt>,
    /// continuations of the active subexpression calls
    pub rets: Vec<NodePtr>,
    /// subroutine entry and its recursion cost, by group number
    pub entries: Vec<(NodePtr, u32)>,
}

impl Ctx {
    pub fn new(s: &[u8], ngroups: usize, gpos: usize, enc: Enc) -> Ctx {
        Ctx {
            s: s.as_ptr(), len: s.len(), slots: vec![NONE; 2 * (ngroups + 1)], log: Vec::new(),
            sprev_bug: NONE, gpos, rep_top: std::ptr::null_mut(), depth: 0, sub_end: 0,
            lb_target: NONE, must: &[], must_occ: Vec::new(), must_next: usize::MAX, ngroups, enc,
            mstk: Vec::new(), rets: Vec::new(), entries: Vec::new(),
        }
    }
    /// Advances the required-literal occurrences to >= p; false when one no longer occurs
    /// (then no match can start at or after p).
    pub fn must_refresh(&mut self, p: usize) -> bool {
        let s = self.subj();
        let must = unsafe { &*self.must };
        let mut next = usize::MAX;
        for (i, m) in must.iter().enumerate() {
            if self.must_occ[i] < p || self.must_occ[i] == NONE {
                match find_must(s, m, p) { Some(q) => self.must_occ[i] = q, None => return false }
            }
            next = next.min(self.must_occ[i]);
        }
        self.must_next = next;
        true
    }
    /// The subject.  The lifetime is erased: a Ctx only lives during one search call.
    #[inline(always)]
    pub fn subj(&self) -> &'static [u8] {
        unsafe { std::slice::from_raw_parts(self.s, self.len) }
    }
    #[inline]
    pub fn undo(&mut self, mark: usize) {
        while self.log.len() > mark {
            let (i, old) = self.log.pop().unwrap();
            if i < MSTK { self.slots[i] = old } else if i == SPREV { self.sprev_bug = old } else { self.mstk.truncate(old) }
        }
    }
    #[inline]
    pub fn set_slot(&mut self, i: usize, v: usize) {
        self.log.push((i, self.slots[i]));
        self.slots[i] = v;
    }
    /// Pushes a mark-stack entry, undone with the log.
    #[inline]
    pub fn mpush(&mut self, e: MEnt) {
        self.log.push((MSTK, self.mstk.len()));
        self.mstk.push(e);
    }
    #[inline]
    pub fn set_sprev(&mut self, v: usize) {
        self.log.push((SPREV, self.sprev_bug));
        self.sprev_bug = v;
    }
    /// Fresh state for an attempt at `p`.
    #[inline]
    pub fn reset(&mut self, p: usize) {
        for x in self.slots.iter_mut() { *x = NONE; }
        self.log.clear();
        self.sprev_bug = NONE;
        self.rep_top = std::ptr::null_mut();
        self.depth = 0;
        self.slots[0] = p;
    }
}

#[inline(always)]
pub fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    unsafe { ((*n).head.dispatcher)(c, n, pos) }
}

/// `b"..."` literal for baking a Bytes operand into specialized code.
pub fn bytes_lit(b: &[u8]) -> String {
    let mut s = String::from("&b\"");
    for &x in b { s.push_str(&format!("\\x{:02x}", x)); }
    s.push_str("\"[..]");
    s
}

// ---- single-char matchers ------------------------------------------------------

#[inline(always)]
pub fn one_k(e: Enc, kind: u32, ch: u32, cls: ClassRef, s: &[u8], pos: usize) -> Option<usize> {
    if pos >= s.len() { return None; }
    let b = s[pos];
    match kind {
        K_CH => {
            if ch < 128 {
                if b as u32 == ch { Some(pos + 1) } else { None }
            } else {
                let (d, l) = decode_e(e, s, pos);
                if d == ch { Some(pos + l) } else { None }
            }
        }
        K_CHIC => {
            if b < 128 {
                if b.to_ascii_lowercase() as u32 == ch { Some(pos + 1) } else { None }
            } else {
                let (d, l) = decode_e(e, s, pos);
                if fold(d) == ch { Some(pos + l) } else { None }
            }
        }
        K_ANY => {
            if b == b'\n' { None } else if b < 128 { Some(pos + 1) } else { Some(pos + decode_e(e, s, pos).1) }
        }
        K_ANYNL => Some(pos + char_len_e(e, s, pos)),
        _ => {
            let cc = unsafe { &(*cls).cc };
            if b < 128 {
                if cc.bit(b) { Some(pos + 1) } else { None }
            } else {
                let (d, l) = decode_e(e, s, pos);
                if cc.test(d) { Some(pos + l) } else { None }
            }
        }
    }
}

/// Greedily consumes up to `rem` more chars (scan_greedy for node operands).
#[inline(always)]
pub fn scan_k(e: Enc, kind: u32, ch: u32, cls: ClassRef, s: &[u8], mut p: usize, rem: u32) -> usize {
    let len = s.len();
    if rem == INF {
        match kind {
            K_ANY => return match s[p..].iter().position(|&b| b == b'\n') { Some(k) => p + k, None => len },
            K_ANYNL => return len,
            K_CH if ch < 128 => {
                let c = ch as u8;
                while p < len && s[p] == c { p += 1; }
                return p;
            }
            K_CLASS => {
                let cc = unsafe { &(*cls).cc };
                while p < len {
                    let b = s[p];
                    if b < 128 {
                        if !cc.bit(b) { break; }
                        p += 1;
                    } else {
                        let (d, l) = decode_e(e, s, p);
                        if !cc.test(d) { break; }
                        p += l;
                    }
                }
                return p;
            }
            _ => {}
        }
    }
    let mut n = 0u32;
    while n < rem {
        match one_k(e, kind, ch, cls, s, p) {
            Some(np) => { p = np; n += 1; }
            None => break,
        }
    }
    p
}

#[inline(always)]
pub fn lazy_advance_k(e: Enc, kind: u32, ch: u32, cls: ClassRef, s: &[u8], mut p: usize, mut cnt: u32, max: u32, nb: i32) -> Option<(usize, u32)> {
    if nb >= 0 {
        let b = nb as u8;
        while !(p < s.len() && s[p] == b) {
            if cnt >= max { return None; }
            p = one_k(e, kind, ch, cls, s, p)?;
            cnt += 1;
        }
    }
    Some((p, cnt))
}

#[inline(always)]
pub fn assert_k(kind: u32, c: &Ctx, pos: usize) -> bool {
    let a = match kind {
        0 => As::Bol, 1 => As::Eol, 2 => As::StrBegin, 3 => As::StrEnd, 4 => As::StrEndNl,
        5 => As::WordB, 6 => As::NotWordB, _ => As::Gpos,
    };
    assert_at(c.enc, a, c.subj(), pos, c.gpos, c.sprev_bug)
}

fn assert_code(a: As) -> u32 {
    match a {
        As::Bol => 0, As::Eol => 1, As::StrBegin => 2, As::StrEnd => 3, As::StrEndNl => 4,
        As::WordB => 5, As::NotWordB => 6, As::Gpos => 7,
    }
}

/// Position after a backref match, trying the groups last-defined first.
#[inline]
pub fn backref_k(c: &Ctx, groups: Groups, icase: bool, pos: usize) -> Option<usize> {
    let s = c.subj();
    for &g in groups.iter().rev() {
        let (a, e) = (c.slots[2 * g], c.slots[2 * g + 1]);
        if a == NONE || e == NONE { continue; }
        let sub = if a > e { &s[0..0] } else { &s[a..e] };
        if icase {
            if let Some(np) = match_ic(c.enc, s, pos, sub) { return Some(np); }
        } else if s.len() - pos >= sub.len() && &s[pos..pos + sub.len()] == sub {
            return Some(pos + sub.len());
        }
    }
    None
}

#[inline]
pub fn grapheme_k(c: &Ctx, pos: usize) -> Option<usize> { grapheme(c.enc, c.subj(), pos) }

#[inline]
pub fn cond_k(c: &Ctx, groups: Groups) -> bool {
    groups.iter().any(|&g| c.slots[2 * g] != NONE && c.slots[2 * g + 1] != NONE)
}

// ---- subexpression calls ----------------------------------------------------------

pub fn open_rec_k(c: &mut Ctx, g: usize, pos: usize) {
    c.set_slot(2 * g, pos);
    c.set_slot(2 * g + 1, NONE);
    c.mpush(MEnt::Start(g, pos));
}

pub fn close_rec_k(c: &mut Ctx, g: usize, pos: usize) {
    let st = rec_start(&c.mstk, g).unwrap_or(c.slots[2 * g]);
    // end first: an (open, open+1) pair in the log would read as an open (empty_iter_verdict)
    c.set_slot(2 * g + 1, pos);
    c.set_slot(2 * g, st);
    c.mpush(MEnt::End(g, pos));
}

pub fn call_k(c: &mut Ctx, g: usize, next: NodePtr, pos: usize) -> u32 {
    let (entry, cost) = c.entries[g];
    let cost = cost as usize;
    c.depth += cost;
    if c.depth > DEPTH_LIMIT { return ABORT; }
    let mark = c.log.len();
    c.rets.push(next);
    c.mpush(MEnt::Call);
    let r = call(c, entry, pos);
    c.rets.pop();
    if r == FAIL { c.undo(mark); }
    c.depth -= cost;
    r
}

pub fn ret_k(c: &mut Ctx, pos: usize) -> u32 {
    let next = c.rets.pop().expect("a subexpression call");
    let mark = c.log.len();
    c.mpush(MEnt::Ret);
    let r = call(c, next, pos);
    if r == FAIL { c.undo(mark); }
    c.rets.push(next);
    r
}

#[inline]
pub fn backref_lv_k(c: &Ctx, groups: Groups, icase: bool, nest: i32, pos: usize) -> Option<usize> {
    backref_lv(c.enc, &c.mstk, c.subj(), groups, icase, nest, pos)
}

// ---- repetition -------------------------------------------------------------------

pub struct RepFrame {
    body: NodePtr,
    outer: NodePtr,
    min: u32,
    max: u32,
    greedy: bool,
    mem: bool,
    shape: u32,
    cost: u32,
    count: u32,
    start: usize,
    mark: usize,
    prev: *mut RepFrame,
}

#[allow(clippy::too_many_arguments)]
pub fn rep_enter(c: &mut Ctx, body: NodePtr, outer: NodePtr, min: u32, max: u32, greedy: bool, mem: bool, shape: u32, cost: u32, pos: usize) -> u32 {
    let mut f = RepFrame { body, outer, min, max, greedy, mem, shape, cost, count: 0, start: NONE, mark: 0, prev: c.rep_top };
    let fp: *mut RepFrame = &mut f;
    c.rep_top = fp;
    let r = rep_loop(c, fp, pos);
    c.rep_top = unsafe { (*fp).prev };
    r
}

/// The loop head ("top" in the VM's code shapes).
fn rep_loop(c: &mut Ctx, f: *mut RepFrame, pos: usize) -> u32 {
    unsafe {
        match (*f).shape {
            SH_PLUS => rep_iter(c, f, pos),
            SH_STAR => rep_split(c, f, pos),
            _ => {
                if (*f).count < (*f).min { rep_iter(c, f, pos) }
                else if (*f).count >= (*f).max { rep_exit(c, f, pos) }
                else { rep_split(c, f, pos) }
            }
        }
    }
}

fn rep_split(c: &mut Ctx, f: *mut RepFrame, pos: usize) -> u32 {
    if unsafe { (*f).greedy } {
        let r = rep_iter(c, f, pos);
        if r != FAIL { return r; }
        rep_exit(c, f, pos)
    } else {
        let r = rep_exit(c, f, pos);
        if r != FAIL { return r; }
        rep_iter(c, f, pos)
    }
}

fn rep_iter(c: &mut Ctx, f: *mut RepFrame, pos: usize) -> u32 {
    unsafe {
        let cost = (*f).cost as usize;
        c.depth += cost;
        if c.depth > DEPTH_LIMIT { return ABORT; }
        let (os, om) = ((*f).start, (*f).mark);
        (*f).start = pos;
        (*f).mark = c.log.len();
        let r = call(c, (*f).body, pos);
        (*f).start = os;
        (*f).mark = om;
        c.depth -= cost;
        r
    }
}

/// Leave the loop: the rest of the pattern runs with the enclosing frame on top.
fn rep_exit(c: &mut Ctx, f: *mut RepFrame, pos: usize) -> u32 {
    unsafe {
        c.rep_top = (*f).prev;
        let r = call(c, (*f).outer, pos);
        c.rep_top = f;
        r
    }
}

fn rep_verdict(c: &Ctx, f: *mut RepFrame, pos: usize) -> u8 {
    unsafe {
        if !(*f).mem { return 0; }
        empty_iter_verdict(&c.log[(*f).mark..], &c.slots, c.ngroups, pos)
    }
}

/// End of one loop iteration (the body's tail).
pub fn rep_cont(c: &mut Ctx, pos: usize) -> u32 {
    let f = c.rep_top;
    unsafe {
        let empty = pos == (*f).start;
        match (*f).shape {
            SH_STAR => {
                if !empty { return rep_loop(c, f, pos); }
                match rep_verdict(c, f, pos) { 0 => rep_exit(c, f, pos), 1 => rep_loop(c, f, pos), _ => FAIL }
            }
            SH_PLUS => {
                if !empty { return rep_split(c, f, pos); }
                match rep_verdict(c, f, pos) { 0 => rep_exit(c, f, pos), 1 => rep_split(c, f, pos), _ => FAIL }
            }
            sh => {
                (*f).count += 1;
                let r = if sh == SH_COUNT && empty {
                    match rep_verdict(c, f, pos) { 0 => rep_exit(c, f, pos), 1 => rep_loop(c, f, pos), _ => FAIL }
                } else {
                    rep_loop(c, f, pos)
                };
                (*f).count -= 1;
                r
            }
        }
    }
}

// ---- lowering: AST -> match nodes (right to left) ---------------------------------------

thread_local! {
    static ALLOCATED: std::cell::RefCell<Vec<NodePtr>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// ALLOC_* hook: records every node so the owning Regex can free it.
pub fn node_allocated(n: NodePtr) {
    ALLOCATED.with(|a| a.borrow_mut().push(n));
}

struct Lowerer<'a> {
    bt_depth: usize,
    backrefed: &'a [usize],
    /// captures paired through the mark stack (the pattern has calls or level backrefs)
    rec: bool,
    /// continuations of \g calls (AOT entries: node_re_ret dispatches to them)
    call_nexts: Vec<NodePtr>,
    rep_cont: NodePtr,
    sub_succ: NodePtr,
    sub_succ_at: NodePtr,
    bytes: Vec<Box<[u8]>>,
    classes: Vec<Box<ClassBox>>,
    groups: Vec<Box<[usize]>>,
    count: usize,
}

/// A lowered chain plus, when it starts (in VM program order, past group saves) with a
/// literal, that literal's first byte: the VM's single-char-loop peephole looks at it.
type Chain = (NodePtr, Option<u8>);

impl<'a> Lowerer<'a> {
    fn bytes(&mut self, b: Vec<u8>) -> Bytes {
        let bx = b.into_boxed_slice();
        let r: Bytes = unsafe { std::slice::from_raw_parts(bx.as_ptr(), bx.len()) };
        self.bytes.push(bx);
        r
    }
    fn class(&mut self, cc: &CClass) -> ClassRef {
        let hash = hash_str(&format!("{:?}", cc));
        let bx = Box::new(ClassBox { cc: cc.clone(), hash });
        let r: ClassRef = &*bx;
        self.classes.push(bx);
        r
    }
    fn groups(&mut self, g: &[usize]) -> Groups {
        let bx: Box<[usize]> = g.to_vec().into_boxed_slice();
        let r: Groups = unsafe { std::slice::from_raw_parts(bx.as_ptr(), bx.len()) };
        self.groups.push(bx);
        r
    }
    fn n(&mut self, p: NodePtr) -> NodePtr {
        self.count += 1;
        p
    }

    fn single(&mut self, n: &Ast) -> Option<(u32, u32, ClassRef)> {
        Some(match n {
            Ast::Char(c, false) => (K_CH, *c, std::ptr::null()),
            Ast::Char(c, true) => (K_CHIC, fold(*c), std::ptr::null()),
            Ast::Any(false) => (K_ANY, 0, std::ptr::null()),
            Ast::Any(true) => (K_ANYNL, 0, std::ptr::null()),
            Ast::Class(cc) => (K_CLASS, 0, self.class(cc)),
            _ => return None,
        })
    }

    fn lit(&mut self, bytes: Vec<u8>, tail: NodePtr) -> Chain {
        let first = bytes[0];
        let p = if bytes.len() == 1 { ALLOC_node_re_byte(first, tail) } else { let b = self.bytes(bytes); ALLOC_node_re_lit(b, tail) };
        (self.n(p), Some(first))
    }

    fn lower_bt(&mut self, n: &Ast, tail: Chain) -> Result<Chain, String> {
        self.bt_depth += 1;
        let r = self.lower(n, tail);
        self.bt_depth -= 1;
        r
    }

    /// `tail.1` is the tail's leading literal byte when the tail is adjacent in VM order.
    fn lower(&mut self, n: &Ast, tail: Chain) -> Result<Chain, String> {
        Ok(match n {
            Ast::Empty => tail,
            Ast::Char(c, false) => {
                let mut v = Vec::new();
                push_char(&mut v, *c);
                self.lit(v, tail.0)
            }
            Ast::Char(..) | Ast::Any(_) | Ast::Class(_) => {
                let (k, ch, cls) = self.single(n).unwrap();
                let p = ALLOC_node_re_one(k, ch, cls, tail.0);
                (self.n(p), None)
            }
            Ast::Seq(items) => {
                let mut cur = tail;
                let mut i = items.len();
                while i > 0 {
                    if let Ast::Char(_, false) = items[i - 1] {
                        let mut j = i;
                        while j > 0 && matches!(items[j - 1], Ast::Char(_, false)) { j -= 1; }
                        let mut v = Vec::new();
                        for it in &items[j..i] { if let Ast::Char(c, _) = it { push_char(&mut v, *c); } }
                        cur = self.lit(v, cur.0);
                        i = j;
                    } else {
                        cur = self.lower(&items[i - 1], cur)?;
                        i -= 1;
                    }
                }
                cur
            }
            Ast::Alt(alts) => {
                // Non-last branches end with a jump in the VM, so their tail is not adjacent.
                let mut cur = self.lower_bt(alts.last().unwrap(), tail)?.0;
                for a in alts[..alts.len() - 1].iter().rev() {
                    let b = self.lower_bt(a, (tail.0, None))?.0;
                    cur = ALLOC_node_re_alt(b, cur);
                    cur = self.n(cur);
                }
                (cur, None)
            }
            Ast::Group(g, b) if self.rec => {
                let g = *g as u32;
                let close = ALLOC_node_re_close_rec(g, tail.0);
                let close = self.n(close);
                let (inner, lit) = self.lower(b, (close, tail.1))?;
                let open = ALLOC_node_re_open_rec(g, inner);
                (self.n(open), lit)
            }
            Ast::Call(BRef::Groups(g)) => {
                self.call_nexts.push(tail.0);
                let p = ALLOC_node_re_call(g[0] as u32, tail.0);
                (self.n(p), None)
            }
            Ast::BackrefLv(BRef::Groups(g), ic, nest) => {
                let g = self.groups(g);
                let p = ALLOC_node_re_backref_lv(g, *ic, *nest, tail.0);
                (self.n(p), None)
            }
            Ast::Group(g, b) => {
                let g = *g as u32;
                if self.bt_depth > 0 || self.backrefed.contains(&(g as usize)) {
                    let close = ALLOC_node_re_close(g, tail.0);
                    let close = self.n(close);
                    let (inner, lit) = self.lower(b, (close, tail.1))?;
                    let open = ALLOC_node_re_open(g, inner);
                    (self.n(open), lit)
                } else {
                    let close = ALLOC_node_re_close(g, tail.0);
                    let close = self.n(close);
                    let (inner, lit) = self.lower(b, (close, tail.1))?;
                    let open = ALLOC_node_re_open_nr(g, inner);
                    (self.n(open), lit)
                }
            }
            Ast::Repeat(b, mn, mx, k) => self.repeat(b, *mn, *mx, *k, tail)?,
            Ast::Assert(a) => {
                let p = ALLOC_node_re_assert(assert_code(*a), tail.0);
                (self.n(p), None)
            }
            Ast::Look(behind, neg, b) => {
                let (minlen, maxlen) = if *behind { len_range(b).ok_or("invalid pattern in look-behind")? } else { (0, 0) };
                let kind = match (behind, neg) {
                    (false, false) => LK_AHEAD,
                    (false, true) => LK_NEG_AHEAD,
                    (true, false) => LK_BEHIND,
                    (true, true) => LK_NEG_BEHIND,
                };
                if *neg { self.bt_depth += 1; }
                let r = self.sub(kind, b, minlen, maxlen, tail.0);
                if *neg { self.bt_depth -= 1; }
                (r?, None)
            }
            Ast::Atomic(b) => (self.sub(LK_ATOMIC, b, 0, 0, tail.0)?, None),
            Ast::Backref(BRef::Groups(g), ic) => {
                let g = self.groups(g);
                let p = ALLOC_node_re_backref(g, *ic, tail.0);
                (self.n(p), None)
            }
            Ast::Cond(BRef::Groups(g), y, no) => {
                let g = self.groups(g);
                let yes = self.lower_bt(y, (tail.0, None))?.0;
                let no = self.lower_bt(no, tail)?.0;
                let p = ALLOC_node_re_cond(g, yes, no);
                (self.n(p), None)
            }
            Ast::Backref(..) | Ast::BackrefLv(..) | Ast::Cond(..) | Ast::Call(_) => unreachable!(),
            Ast::Keep => {
                let p = ALLOC_node_re_keep(tail.0);
                (self.n(p), None)
            }
            Ast::Grapheme => {
                let p = ALLOC_node_re_grapheme(tail.0);
                (self.n(p), None)
            }
        })
    }

    fn sub(&mut self, kind: u32, b: &Ast, minlen: u32, maxlen: u32, next: NodePtr) -> Result<NodePtr, String> {
        let end = if kind == LK_BEHIND || kind == LK_NEG_BEHIND { self.sub_succ_at } else { self.sub_succ };
        let body = self.lower(b, (end, None))?.0;
        let p = ALLOC_node_re_look(kind, body, next, minlen, maxlen);
        Ok(self.n(p))
    }

    fn repeat(&mut self, b: &Ast, min: u32, max: u32, kind: Kind, tail: Chain) -> Result<Chain, String> {
        if max == 0 { return Ok(tail); }
        if len_range(b) == Some((0, 0)) && !has_any_group(b) && !has_keep(b) {
            return if min == 0 { Ok(tail) } else { self.lower(b, tail) };
        }
        if min != max {
            self.bt_depth += 1;
            let r = self.repeat_inner(b, min, max, kind, tail);
            self.bt_depth -= 1;
            return r;
        }
        self.repeat_inner(b, min, max, kind, tail)
    }

    fn loop_node(&mut self, b: &Ast, outer: NodePtr, min: u32, max: u32, greedy: bool, shape: u32) -> Result<NodePtr, String> {
        let c0 = self.count;
        let rc = self.rep_cont;
        let body = self.lower(b, (rc, None))?.0;
        let cost = (self.count - c0 + 2) as u32;
        let p = ALLOC_node_re_rep(body, outer, min, max, greedy, has_group(b), shape, cost);
        Ok(self.n(p))
    }

    fn repeat_inner(&mut self, b: &Ast, min: u32, max: u32, kind: Kind, tail: Chain) -> Result<Chain, String> {
        if kind == Kind::Possessive {
            let r = Ast::Repeat(Box::new(b.clone()), min, max, Kind::Greedy);
            return Ok((self.sub(LK_ATOMIC, &r, 0, 0, tail.0)?, None));
        }
        let greedy = kind == Kind::Greedy;
        if let Some((k, ch, cls)) = self.single(b) {
            let nb = tail.1.map_or(-1, |x| x as i32);
            let p = ALLOC_node_re_rep1(k, ch, cls, min, max, greedy, nb, tail.0);
            return Ok((self.n(p), None));
        }
        if min == 1 && max == 1 { return self.lower(b, tail); }
        if min == 0 && max == 1 {
            let body = self.lower(b, tail)?.0;
            let p = ALLOC_node_re_opt(body, tail.0, greedy);
            return Ok((self.n(p), None));
        }
        if max == INF && min == 0 {
            return Ok((self.loop_node(b, tail.0, 0, INF, greedy, SH_STAR)?, None));
        }
        let tlen = onig_len(b);
        if max == INF && (min == 1 && tlen <= ONIG_EXPAND_LIMIT || min >= 2 && tlen.saturating_mul(min as usize) <= ONIG_EXPAND_LIMIT) {
            let mut cur = self.repeat_inner(b, 0, INF, kind, tail)?;
            for _ in 0..min { cur = self.lower(b, cur)?; }
            return Ok(cur);
        }
        if max == INF && min == 1 {
            return Ok((self.loop_node(b, tail.0, 1, INF, greedy, SH_PLUS)?, None));
        }
        let checked = !(max != INF && greedy && (tlen + 5).saturating_mul(max as usize) <= ONIG_EXPAND_LIMIT);
        Ok((self.loop_node(b, tail.0, min, max, greedy, if checked { SH_COUNT } else { SH_COUNT_NC })?, None))
    }
}

// ---- the public Regex ----------------------------------------------------------------------

/// A compiled pattern: the CPS node tree (primary) plus the VM program (fallback).
pub struct Regex {
    vm: Vm,
    /// The encoding the pattern was parsed in; the default for matching.
    pub enc: Enc,
    pub search_root: NodePtr,
    /// search_root without byte prefilters (they could stop inside a legacy multibyte char)
    pub plain_root: NodePtr,
    pub at_root: NodePtr,
    /// subroutines of the called groups (empty without subexpression calls)
    entries: Vec<(NodePtr, u32)>,
    call_nexts: Vec<NodePtr>,
    nodes: Vec<NodePtr>,
    _bytes: Vec<Box<[u8]>>,
    _classes: Vec<Box<ClassBox>>,
    _groups: Vec<Box<[usize]>>,
    _sets: Vec<Box<[bool; 256]>>,
}

// Nodes are immutable while matching; each match uses its own Ctx.
unsafe impl Send for Regex {}
unsafe impl Sync for Regex {}

impl Drop for Regex {
    fn drop(&mut self) {
        for &n in &self.nodes { unsafe { free_node(n) } }
    }
}

pub type Captures = Vec<Option<(usize, usize)>>;

impl Regex {
    pub fn new(pat: &[u8], opts: u32) -> Result<Regex, String> {
        Self::new_enc(pat, opts, Enc::UTF8)
    }

    /// Parses and checks the pattern without building the matcher.
    pub fn check(pat: &[u8], opts: u32, enc: Enc) -> Result<(), String> { Vm::new(pat, opts, enc).map(|_| ()) }

    /// Pattern chars are read in `enc`, which is also the default subject encoding.
    pub fn new_enc(pat: &[u8], opts: u32, enc: Enc) -> Result<Regex, String> {
        let vm = Vm::new(pat, opts, enc)?;
        ALLOCATED.with(|a| a.borrow_mut().clear());
        let r = Self::build(vm, enc);
        let nodes = ALLOCATED.with(|a| std::mem::take(&mut *a.borrow_mut()));
        match r {
            Ok(mut re) => { re.nodes = nodes; Ok(re) }
            Err(e) => {
                for n in nodes { unsafe { free_node(n) } }
                Err(e)
            }
        }
    }

    fn build(vm: Vm, enc: Enc) -> Result<Regex, String> {
        let rep_cont = ALLOC_node_re_rep_cont();
        let mut lw = Lowerer {
            bt_depth: 0, backrefed: &vm.backrefed, rec: vm.rec, call_nexts: vec![], rep_cont, sub_succ: ALLOC_node_re_sub_succ(),
            sub_succ_at: ALLOC_node_re_sub_succ_at(), bytes: vec![], classes: vec![], groups: vec![], count: 0,
        };
        let succ = ALLOC_node_re_succ();
        let body = lw.lower(&vm.ast, (succ, None))?.0;
        let mut entries = vec![];
        if !vm.callees.is_empty() {
            let ret = ALLOC_node_re_ret();
            entries = vec![(std::ptr::null_mut(), 0); vm.callees.iter().map(|x| x.0).max().unwrap() + 1];
            for (k, b) in &vm.callees {
                let c0 = lw.count;
                let e = lw.lower_bt(b, (ret, None))?.0;
                entries[*k] = (e, (lw.count - c0 + 2) as u32);
            }
        }
        let mut sets = vec![];
        let search_root = match vm.anchor {
            Anchor::StrBegin => ALLOC_node_search(body, AN_STR_BEGIN),
            Anchor::Start => ALLOC_node_search(body, AN_START),
            Anchor::Line => ALLOC_node_search(body, AN_LINE),
            Anchor::None => {
                if let Some(pre) = &vm.prefix {
                    if pre.len() == 1 { ALLOC_node_search_memchr(body, pre[0]) } else { let b = lw.bytes(pre.to_vec()); ALLOC_node_search_memmem(body, b) }
                } else if let Some(fs) = &vm.first {
                    let bx = fs.clone();
                    let p: ByteSet = &*bx;
                    sets.push(bx);
                    ALLOC_node_search_byteset(body, p)
                } else {
                    ALLOC_node_search(body, AN_NONE)
                }
            }
        };
        let plain_root = if matches!(vm.anchor, Anchor::None) && search_root_prefilters(&vm) { ALLOC_node_search(body, AN_NONE) } else { search_root };
        let at_root = ALLOC_node_match_at(body);
        let (bytes, classes, groups) = (std::mem::take(&mut lw.bytes), std::mem::take(&mut lw.classes), std::mem::take(&mut lw.groups));
        let call_nexts = std::mem::take(&mut lw.call_nexts);
        Ok(Regex { vm, enc, search_root, plain_root, at_root, entries, call_nexts, nodes: vec![], _bytes: bytes, _classes: classes, _groups: groups, _sets: sets })
    }

    pub fn group_count(&self) -> usize { self.vm.group_count() }
    pub fn linear_time(&self) -> bool { self.vm.linear }
    pub fn names(&self) -> Vec<(String, Vec<usize>)> { self.vm.names() }
    pub fn group_index(&self, name: &str) -> Vec<usize> { self.vm.group_index(name) }

    fn captures(&self, c: &Ctx) -> Captures {
        let mut slots = c.slots.clone();
        if slots[0] > slots[1] { slots[0] = slots[1]; } // \K inside a lookahead can point past the end
        (0..=self.vm.group_count()).map(|g| {
            let (a, b) = (slots[2 * g], slots[2 * g + 1]);
            if a != NONE && b != NONE && a <= b { Some((a, b)) } else { None }
        }).collect()
    }

    /// Leftmost match whose start is >= `start`.
    pub fn search(&self, s: &[u8], start: usize) -> Option<Captures> { self.search_enc(s, start, self.enc) }

    /// search with the subject split into chars as `enc` says (an ASCII-only pattern can be
    /// used on any ASCII-compatible encoding).
    pub fn search_enc(&self, s: &[u8], start: usize, enc: Enc) -> Option<Captures> {
        if start > s.len() { return None; }
        let mut c = Ctx::new(s, self.vm.group_count(), start, enc);
        if !self.entries.is_empty() { c.entries = self.entries.clone(); }
        if !self.vm.must.is_empty() {
            c.must = &self.vm.must[..];
            c.must_occ = vec![NONE; self.vm.must.len()];
            if !c.must_refresh(start) { return None; }
        }
        let root = if enc.is_mb() { self.plain_root } else { self.search_root };
        match call(&mut c, root, start) {
            OK => Some(self.captures(&c)),
            FAIL => None,
            _ => self.vm.search(s, start, enc),
        }
    }

    /// Match anchored at `pos`.
    pub fn match_at(&self, s: &[u8], pos: usize) -> Option<Captures> { self.match_at_enc(s, pos, self.enc) }

    pub fn match_at_enc(&self, s: &[u8], pos: usize, enc: Enc) -> Option<Captures> {
        if pos > s.len() || !self.vm.must_ok_from(s, pos) { return None; }
        let mut c = Ctx::new(s, self.vm.group_count(), pos, enc);
        if !self.entries.is_empty() { c.entries = self.entries.clone(); }
        match call(&mut c, self.at_root, pos) {
            OK => Some(self.captures(&c)),
            FAIL => None,
            _ => self.vm.match_at(s, pos, enc),
        }
    }

    /// Search with the VM only (for tests and comparisons).
    pub fn search_vm(&self, s: &[u8], start: usize) -> Option<Captures> { self.vm.search(s, start, self.enc) }

    /// Nodes whose dispatchers may be called indirectly (scanner roots, loop bodies and
    /// continuations): the AOT entries.
    pub fn entry_nodes(&self) -> Vec<NodePtr> {
        let mut v = vec![self.search_root, self.at_root];
        v.extend(self.entries.iter().map(|e| e.0).filter(|e| !e.is_null()));
        v.extend_from_slice(&self.call_nexts);
        for &n in &self.nodes {
            if unsafe { (*n).head.kind.name } == "node_re_rep" {
                let u = unsafe { &(*(n as *mut NodeOf<node_re_rep_struct>)).u };
                v.push(u.body);
                v.push(u.outer);
            }
        }
        v
    }
}

// ---- scanner helpers ----------------------------------------------------------------------

/// Tries `body` at p after resetting the per-attempt state.
#[inline(always)]
pub fn attempt<D: Disp>(c: &mut Ctx, body: NodePtr, p: usize) -> u32 {
    c.reset(p);
    D::call(c, body, p)
}

#[inline(always)]
pub fn memchr_from(s: &[u8], b: u8, from: usize) -> Option<usize> {
    s[from..].iter().position(|&x| x == b).map(|k| from + k)
}

#[inline(always)]
pub fn find_from(s: &[u8], needle: &[u8], from: usize) -> Option<usize> { find_sub(s, needle, from) }

fn search_root_prefilters(vm: &Vm) -> bool { vm.prefix.is_some() || vm.first.is_some() }
