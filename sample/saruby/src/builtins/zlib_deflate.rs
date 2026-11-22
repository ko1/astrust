// deflate of zlib 1.3 (deflate.c, trees.c), ported to Rust so that the output is byte-identical.
//
// zlib 1.3, Copyright (C) 1995-2023 Jean-loup Gailly and Mark Adler
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.
//
// Altered: Rust port of the default (non-FASTEST, non-debug) build; gzip headers only without a
// gz_header (deflateSetHeader is not ported); pending_buf and sym_buf are separate buffers.
#![forbid(unsafe_code)]
use crate::builtins::zlib_check::*;

const LENGTH_CODES: usize = 29;
const LITERALS: usize = 256;
const L_CODES: usize = LITERALS + 1 + LENGTH_CODES;
const D_CODES: usize = 30;
const BL_CODES: usize = 19;
const HEAP_SIZE: usize = 2 * L_CODES + 1;
const MAX_BITS: usize = 15;
const MAX_BL_BITS: i32 = 7;
const BUF_SIZE: i32 = 16;
const END_BLOCK: usize = 256;
const REP_3_6: usize = 16;
const REPZ_3_10: usize = 17;
const REPZ_11_138: usize = 18;
const MIN_MATCH: u32 = 3;
const MAX_MATCH: u32 = 258;
const MIN_LOOKAHEAD: u32 = MAX_MATCH + MIN_MATCH + 1;
const WIN_INIT: u64 = MAX_MATCH as u64;
const TOO_FAR: u32 = 4096;
const MAX_STORED: u32 = 65535;
const STORED_BLOCK: u32 = 0;
const STATIC_TREES: u32 = 1;
const DYN_TREES: u32 = 2;
const PRESET_DICT: u32 = 0x20;

const INIT_STATE: i32 = 42;
const GZIP_STATE: i32 = 57;
const BUSY_STATE: i32 = 113;
const FINISH_STATE: i32 = 666;

static EXTRA_LBITS: [i32; LENGTH_CODES] = [0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4,5,5,5,5,0];
static EXTRA_DBITS: [i32; D_CODES] = [0,0,0,0,1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,10,10,11,11,12,12,13,13];
static EXTRA_BLBITS: [i32; BL_CODES] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,3,7];
static BL_ORDER: [u8; BL_CODES] = [16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15];

/// ct_data: `fc` is Freq or Code, `dl` is Dad or Len (two unions in C).
#[derive(Clone, Copy, Default)]
struct Ct { fc: u16, dl: u16 }

const fn bi_reverse(mut code: u32, mut len: i32) -> u32 {
    let mut res = 0;
    loop {
        res |= code & 1;
        code >>= 1;
        res <<= 1;
        len -= 1;
        if len <= 0 { break; }
    }
    res >> 1
}

struct Statics { ltree: [Ct; L_CODES + 2], dtree: [Ct; D_CODES], dist_code: [u8; 512], length_code: [u8; 256], base_length: [i32; LENGTH_CODES], base_dist: [i32; D_CODES] }

const fn make_statics() -> Statics {
    let mut s = Statics { ltree: [Ct { fc: 0, dl: 0 }; L_CODES + 2], dtree: [Ct { fc: 0, dl: 0 }; D_CODES], dist_code: [0; 512], length_code: [0; 256], base_length: [0; LENGTH_CODES], base_dist: [0; D_CODES] };
    let mut length = 0usize;
    let mut code = 0usize;
    while code < LENGTH_CODES - 1 {
        s.base_length[code] = length as i32;
        let mut n = 0;
        while n < (1 << EXTRA_LBITS[code]) { s.length_code[length] = code as u8; length += 1; n += 1; }
        code += 1;
    }
    s.length_code[length - 1] = code as u8;
    let mut dist = 0usize;
    code = 0;
    while code < 16 {
        s.base_dist[code] = dist as i32;
        let mut n = 0;
        while n < (1 << EXTRA_DBITS[code]) { s.dist_code[dist] = code as u8; dist += 1; n += 1; }
        code += 1;
    }
    dist >>= 7;
    while code < D_CODES {
        s.base_dist[code] = (dist << 7) as i32;
        let mut n = 0;
        while n < (1 << (EXTRA_DBITS[code] - 7)) { s.dist_code[256 + dist] = code as u8; dist += 1; n += 1; }
        code += 1;
    }
    let mut bl_count = [0u16; MAX_BITS + 1];
    let mut n = 0;
    while n <= 143 { s.ltree[n].dl = 8; bl_count[8] += 1; n += 1; }
    while n <= 255 { s.ltree[n].dl = 9; bl_count[9] += 1; n += 1; }
    while n <= 279 { s.ltree[n].dl = 7; bl_count[7] += 1; n += 1; }
    while n <= 287 { s.ltree[n].dl = 8; bl_count[8] += 1; n += 1; }
    // gen_codes(static_ltree, L_CODES+1, bl_count)
    let mut next_code = [0u16; MAX_BITS + 1];
    let mut c = 0u32;
    let mut bits = 1;
    while bits <= MAX_BITS { c = (c + bl_count[bits - 1] as u32) << 1; next_code[bits] = c as u16; bits += 1; }
    n = 0;
    while n <= L_CODES + 1 {
        let len = s.ltree[n].dl as usize;
        if len != 0 { s.ltree[n].fc = bi_reverse(next_code[len] as u32, len as i32) as u16; next_code[len] += 1; }
        n += 1;
    }
    n = 0;
    while n < D_CODES { s.dtree[n].dl = 5; s.dtree[n].fc = bi_reverse(n as u32, 5) as u16; n += 1; }
    s
}
static ST: Statics = make_statics();

#[derive(Clone, Copy, PartialEq)]
enum Tree { L, D, BL }
struct StaticDesc { stree: Option<Tree>, extra: &'static [i32], extra_base: i32, elems: i32, max_length: i32 }
static STATIC_L_DESC: StaticDesc = StaticDesc { stree: Some(Tree::L), extra: &EXTRA_LBITS, extra_base: LITERALS as i32 + 1, elems: L_CODES as i32, max_length: MAX_BITS as i32 };
static STATIC_D_DESC: StaticDesc = StaticDesc { stree: Some(Tree::D), extra: &EXTRA_DBITS, extra_base: 0, elems: D_CODES as i32, max_length: MAX_BITS as i32 };
static STATIC_BL_DESC: StaticDesc = StaticDesc { stree: None, extra: &EXTRA_BLBITS, extra_base: 0, elems: BL_CODES as i32, max_length: MAX_BL_BITS };

#[inline(always)]
fn d_code(dist: usize) -> usize { if dist < 256 { ST.dist_code[dist] as usize } else { ST.dist_code[256 + (dist >> 7)] as usize } }

#[derive(Clone, Copy, PartialEq)]
enum BlockState { NeedMore, BlockDone, FinishStarted, FinishDone }

#[derive(Clone, Copy, PartialEq)]
enum Func { Stored, Fast, Slow }
struct Config { good: u16, lazy: u16, nice: u16, chain: u16, func: Func }
static CONFIG: [Config; 10] = [
    Config { good: 0, lazy: 0, nice: 0, chain: 0, func: Func::Stored },
    Config { good: 4, lazy: 4, nice: 8, chain: 4, func: Func::Fast },
    Config { good: 4, lazy: 5, nice: 16, chain: 8, func: Func::Fast },
    Config { good: 4, lazy: 6, nice: 32, chain: 32, func: Func::Fast },
    Config { good: 4, lazy: 4, nice: 16, chain: 16, func: Func::Slow },
    Config { good: 8, lazy: 16, nice: 32, chain: 32, func: Func::Slow },
    Config { good: 8, lazy: 16, nice: 128, chain: 128, func: Func::Slow },
    Config { good: 8, lazy: 32, nice: 128, chain: 256, func: Func::Slow },
    Config { good: 32, lazy: 128, nice: 258, chain: 1024, func: Func::Slow },
    Config { good: 32, lazy: 258, nice: 258, chain: 4096, func: Func::Slow },
];

#[inline(always)]
fn rank(f: i32) -> i32 { f * 2 - if f > 4 { 9 } else { 0 } }

/// The Huffman-tree scratch of trees.c (heap, depth, bl_count, block lengths).
#[derive(Clone)]
struct Hp { bl_count: [u16; MAX_BITS + 1], heap: [i32; 2 * L_CODES + 1], heap_len: i32, heap_max: i32, depth: [u8; 2 * L_CODES + 1], opt_len: u64, static_len: u64 }

#[inline(always)]
fn smaller(tree: &[Ct], n: usize, m: usize, depth: &[u8]) -> bool {
    tree[n].fc < tree[m].fc || (tree[n].fc == tree[m].fc && depth[n] <= depth[m])
}
fn pqdownheap(h: &mut Hp, tree: &[Ct], mut k: i32) {
    let v = h.heap[k as usize];
    let mut j = k << 1;
    while j <= h.heap_len {
        if j < h.heap_len && smaller(tree, h.heap[j as usize + 1] as usize, h.heap[j as usize] as usize, &h.depth) { j += 1; }
        if smaller(tree, v as usize, h.heap[j as usize] as usize, &h.depth) { break; }
        h.heap[k as usize] = h.heap[j as usize];
        k = j;
        j <<= 1;
    }
    h.heap[k as usize] = v;
}
fn stree_of(t: Tree) -> &'static [Ct] { match t { Tree::L => &ST.ltree, Tree::D => &ST.dtree, Tree::BL => &[] } }
fn gen_bitlen(h: &mut Hp, tree: &mut [Ct], max_code: i32, sd: &StaticDesc) {
    let stree = sd.stree.map(stree_of);
    let extra = sd.extra;
    let base = sd.extra_base;
    let max_length = sd.max_length;
    let mut overflow = 0;
    for b in h.bl_count.iter_mut() { *b = 0; }
    tree[h.heap[h.heap_max as usize] as usize].dl = 0;
    let mut hh = h.heap_max as usize + 1;
    while hh < HEAP_SIZE {
        let n = h.heap[hh] as usize;
        let mut bits = tree[tree[n].dl as usize].dl as i32 + 1;
        if bits > max_length { bits = max_length; overflow += 1; }
        tree[n].dl = bits as u16;
        hh += 1;
        if n as i32 > max_code { continue; }
        h.bl_count[bits as usize] += 1;
        let mut xbits = 0;
        if n as i32 >= base { xbits = extra[n - base as usize]; }
        let f = tree[n].fc as u64;
        h.opt_len = h.opt_len.wrapping_add(f * (bits + xbits) as u64);
        if let Some(st) = stree { h.static_len = h.static_len.wrapping_add(f * (st[n].dl as i32 + xbits) as u64); }
    }
    if overflow == 0 { return; }
    loop {
        let mut bits = max_length as usize - 1;
        while h.bl_count[bits] == 0 { bits -= 1; }
        h.bl_count[bits] -= 1;
        h.bl_count[bits + 1] += 2;
        h.bl_count[max_length as usize] -= 1;
        overflow -= 2;
        if overflow <= 0 { break; }
    }
    let mut hh = HEAP_SIZE;
    let mut bits = max_length as usize;
    while bits != 0 {
        let mut n = h.bl_count[bits] as i32;
        while n != 0 {
            hh -= 1;
            let m = h.heap[hh] as usize;
            if m as i32 > max_code { continue; }
            if tree[m].dl as usize != bits {
                h.opt_len = h.opt_len.wrapping_add((bits as u64).wrapping_sub(tree[m].dl as u64).wrapping_mul(tree[m].fc as u64));
                tree[m].dl = bits as u16;
            }
            n -= 1;
        }
        bits -= 1;
    }
}
fn gen_codes(tree: &mut [Ct], max_code: i32, bl_count: &[u16; MAX_BITS + 1]) {
    let mut next_code = [0u16; MAX_BITS + 1];
    let mut code = 0u32;
    for bits in 1..=MAX_BITS { code = (code + bl_count[bits - 1] as u32) << 1; next_code[bits] = code as u16; }
    for n in 0..=(max_code.max(-1)) {
        let n = n as usize;
        let len = tree[n].dl as usize;
        if len == 0 { continue; }
        tree[n].fc = bi_reverse(next_code[len] as u32, len as i32) as u16;
        next_code[len] = next_code[len].wrapping_add(1);
    }
}
/// build_tree: returns max_code.
fn build_tree(h: &mut Hp, tree: &mut [Ct], sd: &StaticDesc) -> i32 {
    let stree = sd.stree.map(stree_of);
    let elems = sd.elems;
    let mut max_code = -1;
    h.heap_len = 0;
    h.heap_max = HEAP_SIZE as i32;
    for n in 0..elems as usize {
        if tree[n].fc != 0 {
            h.heap_len += 1;
            h.heap[h.heap_len as usize] = n as i32;
            max_code = n as i32;
            h.depth[n] = 0;
        } else {
            tree[n].dl = 0;
        }
    }
    while h.heap_len < 2 {
        let node = if max_code < 2 { max_code += 1; max_code } else { 0 };
        h.heap_len += 1;
        h.heap[h.heap_len as usize] = node;
        let node = node as usize;
        tree[node].fc = 1;
        h.depth[node] = 0;
        h.opt_len = h.opt_len.wrapping_sub(1);
        if let Some(st) = stree { h.static_len = h.static_len.wrapping_sub(st[node].dl as u64); }
    }
    let mut n = h.heap_len / 2;
    while n >= 1 { pqdownheap(h, tree, n); n -= 1; }
    let mut node = elems as usize;
    loop {
        // pqremove
        let n = h.heap[1] as usize;
        h.heap[1] = h.heap[h.heap_len as usize];
        h.heap_len -= 1;
        pqdownheap(h, tree, 1);
        let m = h.heap[1] as usize;
        h.heap_max -= 1; h.heap[h.heap_max as usize] = n as i32;
        h.heap_max -= 1; h.heap[h.heap_max as usize] = m as i32;
        tree[node].fc = tree[n].fc.wrapping_add(tree[m].fc);
        h.depth[node] = (if h.depth[n] >= h.depth[m] { h.depth[n] } else { h.depth[m] }).wrapping_add(1);
        tree[n].dl = node as u16;
        tree[m].dl = node as u16;
        h.heap[1] = node as i32;
        node += 1;
        pqdownheap(h, tree, 1);
        if h.heap_len < 2 { break; }
    }
    h.heap_max -= 1;
    h.heap[h.heap_max as usize] = h.heap[1];
    gen_bitlen(h, tree, max_code, sd);
    let bl = h.bl_count;
    gen_codes(tree, max_code, &bl);
    max_code
}
fn scan_tree(bl_tree: &mut [Ct], tree: &mut [Ct], max_code: i32) {
    let mut prevlen: i32 = -1;
    let mut nextlen = tree[0].dl as i32;
    let mut count = 0;
    let (mut max_count, mut min_count) = (7, 4);
    if nextlen == 0 { max_count = 138; min_count = 3; }
    tree[(max_code + 1) as usize].dl = 0xffff;
    for n in 0..=max_code as usize {
        let curlen = nextlen;
        nextlen = tree[n + 1].dl as i32;
        count += 1;
        if count < max_count && curlen == nextlen { continue; }
        else if count < min_count { bl_tree[curlen as usize].fc = bl_tree[curlen as usize].fc.wrapping_add(count as u16); }
        else if curlen != 0 {
            if curlen != prevlen { bl_tree[curlen as usize].fc = bl_tree[curlen as usize].fc.wrapping_add(1); }
            bl_tree[REP_3_6].fc = bl_tree[REP_3_6].fc.wrapping_add(1);
        } else if count <= 10 { bl_tree[REPZ_3_10].fc = bl_tree[REPZ_3_10].fc.wrapping_add(1); }
        else { bl_tree[REPZ_11_138].fc = bl_tree[REPZ_11_138].fc.wrapping_add(1); }
        count = 0;
        prevlen = curlen;
        if nextlen == 0 { max_count = 138; min_count = 3; }
        else if curlen == nextlen { max_count = 6; min_count = 3; }
        else { max_count = 7; min_count = 4; }
    }
}

pub struct Deflate {
    // z_stream fields
    pub total_in: u64,
    pub total_out: u64,
    pub adler: u32,
    pub data_type: i32,
    pub msg: Option<&'static str>,
    /// next_out == Z_NULL (zlib.c calls deflateParams with no buffer after a detach)
    pub next_out_null: bool,
    // deflate_state
    status: i32,
    pending_buf: Vec<u8>,
    pending_buf_size: usize,
    pending_out: usize,
    pending: usize,
    wrap: i32,
    last_flush: i32,
    w_size: u32,
    w_bits: u32,
    w_mask: u32,
    window: Vec<u8>,
    window_size: u64,
    prev: Vec<u16>,
    head: Vec<u16>,
    ins_h: u32,
    hash_size: u32,
    hash_bits: u32,
    hash_mask: u32,
    hash_shift: u32,
    block_start: i64,
    match_length: u32,
    prev_match: u32,
    match_available: bool,
    strstart: u32,
    match_start: u32,
    lookahead: u32,
    prev_length: u32,
    max_chain_length: u32,
    max_lazy_match: u32,
    level: i32,
    strategy: i32,
    good_match: u32,
    nice_match: i32,
    dyn_ltree: [Ct; HEAP_SIZE],
    dyn_dtree: [Ct; 2 * D_CODES + 1],
    bl_tree: [Ct; 2 * BL_CODES + 1],
    l_max_code: i32,
    d_max_code: i32,
    hp: Hp,
    sym_buf: Vec<u8>,
    lit_bufsize: u32,
    sym_next: u32,
    sym_end: u32,
    matches: u32,
    insert: u32,
    bi_buf: u16,
    bi_valid: i32,
    high_water: u64,
}

impl Clone for Deflate {
    fn clone(&self) -> Deflate {
        Deflate {
            total_in: self.total_in, total_out: self.total_out, adler: self.adler, data_type: self.data_type, msg: self.msg, next_out_null: self.next_out_null,
            status: self.status, pending_buf: self.pending_buf.clone(), pending_buf_size: self.pending_buf_size, pending_out: self.pending_out,
            pending: self.pending, wrap: self.wrap, last_flush: self.last_flush, w_size: self.w_size, w_bits: self.w_bits, w_mask: self.w_mask,
            window: self.window.clone(), window_size: self.window_size, prev: self.prev.clone(), head: self.head.clone(), ins_h: self.ins_h,
            hash_size: self.hash_size, hash_bits: self.hash_bits, hash_mask: self.hash_mask, hash_shift: self.hash_shift, block_start: self.block_start,
            match_length: self.match_length, prev_match: self.prev_match, match_available: self.match_available, strstart: self.strstart,
            match_start: self.match_start, lookahead: self.lookahead, prev_length: self.prev_length, max_chain_length: self.max_chain_length,
            max_lazy_match: self.max_lazy_match, level: self.level, strategy: self.strategy, good_match: self.good_match, nice_match: self.nice_match,
            dyn_ltree: self.dyn_ltree, dyn_dtree: self.dyn_dtree, bl_tree: self.bl_tree, l_max_code: self.l_max_code, d_max_code: self.d_max_code,
            hp: self.hp.clone(), sym_buf: self.sym_buf.clone(), lit_bufsize: self.lit_bufsize, sym_next: self.sym_next, sym_end: self.sym_end,
            matches: self.matches, insert: self.insert, bi_buf: self.bi_buf, bi_valid: self.bi_valid, high_water: self.high_water,
        }
    }
}

impl Deflate {
    /// deflateInit2; Err(code) on bad parameters.
    pub fn new(mut level: i32, method: i32, mut window_bits: i32, mem_level: i32, strategy: i32) -> Result<Box<Deflate>, i32> {
        let mut wrap = 1;
        if level == -1 { level = 6; }
        if window_bits < 0 {
            wrap = 0;
            if window_bits < -15 { return Err(Z_STREAM_ERROR); }
            window_bits = -window_bits;
        } else if window_bits > 15 {
            wrap = 2;
            window_bits -= 16;
        }
        if !(1..=MAX_MEM_LEVEL).contains(&mem_level) || method != Z_DEFLATED || !(8..=15).contains(&window_bits) || !(0..=9).contains(&level)
            || !(0..=Z_FIXED).contains(&strategy) || (window_bits == 8 && wrap != 1) {
            return Err(Z_STREAM_ERROR);
        }
        if window_bits == 8 { window_bits = 9; }
        let w_bits = window_bits as u32;
        let w_size = 1u32 << w_bits;
        let hash_bits = mem_level as u32 + 7;
        let hash_size = 1u32 << hash_bits;
        let lit_bufsize = 1u32 << (mem_level + 6);
        let mut s = Box::new(Deflate {
            total_in: 0, total_out: 0, adler: 0, data_type: Z_UNKNOWN, msg: None, next_out_null: false,
            status: INIT_STATE, pending_buf: vec![0; lit_bufsize as usize * 4], pending_buf_size: lit_bufsize as usize * 4, pending_out: 0, pending: 0,
            wrap, last_flush: 0, w_size, w_bits, w_mask: w_size - 1, window: vec![0; w_size as usize * 2], window_size: 0,
            prev: vec![0; w_size as usize], head: vec![0; hash_size as usize], ins_h: 0, hash_size, hash_bits, hash_mask: hash_size - 1,
            hash_shift: (hash_bits + MIN_MATCH - 1) / MIN_MATCH, block_start: 0, match_length: 0, prev_match: 0, match_available: false,
            strstart: 0, match_start: 0, lookahead: 0, prev_length: 0, max_chain_length: 0, max_lazy_match: 0, level, strategy,
            good_match: 0, nice_match: 0, dyn_ltree: [Ct::default(); HEAP_SIZE], dyn_dtree: [Ct::default(); 2 * D_CODES + 1],
            bl_tree: [Ct::default(); 2 * BL_CODES + 1], l_max_code: 0, d_max_code: 0,
            hp: Hp { bl_count: [0; MAX_BITS + 1], heap: [0; 2 * L_CODES + 1], heap_len: 0, heap_max: 0, depth: [0; 2 * L_CODES + 1], opt_len: 0, static_len: 0 },
            sym_buf: vec![0; lit_bufsize as usize * 3], lit_bufsize, sym_next: 0, sym_end: (lit_bufsize - 1) * 3, matches: 0, insert: 0,
            bi_buf: 0, bi_valid: 0, high_water: 0,
        });
        s.reset();
        Ok(s)
    }

    fn reset_keep(&mut self) {
        self.total_in = 0;
        self.total_out = 0;
        self.msg = None;
        self.data_type = Z_UNKNOWN;
        self.pending = 0;
        self.pending_out = 0;
        if self.wrap < 0 { self.wrap = -self.wrap; }
        self.status = if self.wrap == 2 { GZIP_STATE } else { INIT_STATE };
        self.adler = if self.wrap == 2 { 0 } else { 1 };
        self.last_flush = -2;
        self.tr_init();
    }
    fn clear_hash(&mut self) { for h in self.head.iter_mut() { *h = 0; } }
    fn lm_init(&mut self) {
        self.window_size = 2 * self.w_size as u64;
        self.clear_hash();
        let c = &CONFIG[self.level as usize];
        self.max_lazy_match = c.lazy as u32;
        self.good_match = c.good as u32;
        self.nice_match = c.nice as i32;
        self.max_chain_length = c.chain as u32;
        self.strstart = 0;
        self.block_start = 0;
        self.lookahead = 0;
        self.insert = 0;
        self.match_length = MIN_MATCH - 1;
        self.prev_length = MIN_MATCH - 1;
        self.match_available = false;
        self.ins_h = 0;
    }
    /// deflateReset.
    pub fn reset(&mut self) { self.reset_keep(); self.lm_init(); }
    /// deflateEnd's result.
    pub fn end_status(&self) -> i32 { if self.status == BUSY_STATE { Z_DATA_ERROR } else { Z_OK } }

    #[inline(always)]
    fn max_dist(&self) -> u32 { self.w_size - MIN_LOOKAHEAD }
    #[inline(always)]
    fn update_hash(&self, h: u32, c: u8) -> u32 { ((h << self.hash_shift) ^ c as u32) & self.hash_mask }
    #[inline(always)]
    fn insert_string(&mut self, s: u32) -> u32 {
        self.ins_h = self.update_hash(self.ins_h, self.window[(s + MIN_MATCH - 1) as usize]);
        let h = self.head[self.ins_h as usize];
        self.prev[(s & self.w_mask) as usize] = h;
        self.head[self.ins_h as usize] = s as u16;
        h as u32
    }

    fn slide_hash(&mut self) {
        let wsize = self.w_size;
        for p in self.head.iter_mut() { let m = *p as u32; *p = if m >= wsize { (m - wsize) as u16 } else { 0 }; }
        for p in self.prev.iter_mut() { let m = *p as u32; *p = if m >= wsize { (m - wsize) as u16 } else { 0 }; }
    }

    /// read_buf into the window at `at`.
    fn read_buf_win(&mut self, io: &mut ZIo, at: usize, size: usize) -> usize {
        let len = io.avail_in().min(size);
        if len == 0 { return 0; }
        let src = &io.input[io.ip..io.ip + len];
        self.window[at..at + len].copy_from_slice(src);
        if self.wrap == 1 { self.adler = adler32(self.adler, src); } else if self.wrap == 2 { self.adler = crc32(self.adler, src); }
        io.ip += len;
        self.total_in += len as u64;
        len
    }
    /// read_buf straight into next_out (deflate_stored).
    fn read_buf_out(&mut self, io: &mut ZIo, size: usize) -> usize {
        let len = io.avail_in().min(size);
        if len == 0 { return 0; }
        let (ip, op) = (io.ip, io.op);
        io.out[op..op + len].copy_from_slice(&io.input[ip..ip + len]);
        let src = &io.input[ip..ip + len];
        if self.wrap == 1 { self.adler = adler32(self.adler, src); } else if self.wrap == 2 { self.adler = crc32(self.adler, src); }
        io.ip += len;
        self.total_in += len as u64;
        len
    }

    fn fill_window(&mut self, io: &mut ZIo) {
        let wsize = self.w_size;
        loop {
            let mut more = (self.window_size - self.lookahead as u64 - self.strstart as u64) as u32;
            if self.strstart >= wsize + self.max_dist() {
                let n = (wsize - more) as usize;
                self.window.copy_within(wsize as usize..wsize as usize + n, 0);
                self.match_start = self.match_start.wrapping_sub(wsize);
                self.strstart -= wsize;
                self.block_start -= wsize as i64;
                if self.insert > self.strstart { self.insert = self.strstart; }
                self.slide_hash();
                more += wsize;
            }
            if io.avail_in() == 0 { break; }
            let at = (self.strstart + self.lookahead) as usize;
            let n = self.read_buf_win(io, at, more as usize);
            self.lookahead += n as u32;
            if self.lookahead + self.insert >= MIN_MATCH {
                let mut str = self.strstart - self.insert;
                self.ins_h = self.window[str as usize] as u32;
                self.ins_h = self.update_hash(self.ins_h, self.window[str as usize + 1]);
                while self.insert != 0 {
                    self.ins_h = self.update_hash(self.ins_h, self.window[(str + MIN_MATCH - 1) as usize]);
                    self.prev[(str & self.w_mask) as usize] = self.head[self.ins_h as usize];
                    self.head[self.ins_h as usize] = str as u16;
                    str += 1;
                    self.insert -= 1;
                    if self.lookahead + self.insert < MIN_MATCH { break; }
                }
            }
            if !(self.lookahead < MIN_LOOKAHEAD && io.avail_in() != 0) { break; }
        }
        if self.high_water < self.window_size {
            let curr = self.strstart as u64 + self.lookahead as u64;
            if self.high_water < curr {
                let init = (self.window_size - curr).min(WIN_INIT);
                for b in &mut self.window[curr as usize..(curr + init) as usize] { *b = 0; }
                self.high_water = curr + init;
            } else if self.high_water < curr + WIN_INIT {
                let init = (curr + WIN_INIT - self.high_water).min(self.window_size - self.high_water);
                for b in &mut self.window[self.high_water as usize..(self.high_water + init) as usize] { *b = 0; }
                self.high_water += init;
            }
        }
    }

    /// deflateSetDictionary.
    pub fn set_dictionary(&mut self, dict: &[u8]) -> i32 {
        let wrap = self.wrap;
        if wrap == 2 || (wrap == 1 && self.status != INIT_STATE) || self.lookahead != 0 { return Z_STREAM_ERROR; }
        if wrap == 1 { self.adler = adler32(self.adler, dict); }
        self.wrap = 0;
        let mut dictionary = dict;
        if dictionary.len() >= self.w_size as usize {
            if wrap == 0 {
                self.clear_hash();
                self.strstart = 0;
                self.block_start = 0;
                self.insert = 0;
            }
            dictionary = &dictionary[dictionary.len() - self.w_size as usize..];
        }
        let mut empty: [u8; 0] = [];
        let mut io = ZIo { input: dictionary, ip: 0, out: &mut empty, op: 0 };
        self.fill_window(&mut io);
        while self.lookahead >= MIN_MATCH {
            let mut str = self.strstart;
            let mut n = self.lookahead - (MIN_MATCH - 1);
            loop {
                self.ins_h = self.update_hash(self.ins_h, self.window[(str + MIN_MATCH - 1) as usize]);
                self.prev[(str & self.w_mask) as usize] = self.head[self.ins_h as usize];
                self.head[self.ins_h as usize] = str as u16;
                str += 1;
                n -= 1;
                if n == 0 { break; }
            }
            self.strstart = str;
            self.lookahead = MIN_MATCH - 1;
            self.fill_window(&mut io);
        }
        self.strstart += self.lookahead;
        self.block_start = self.strstart as i64;
        self.insert = self.lookahead;
        self.lookahead = 0;
        self.match_length = MIN_MATCH - 1;
        self.prev_length = MIN_MATCH - 1;
        self.match_available = false;
        self.wrap = wrap;
        Z_OK
    }

    /// deflateParams (may call deflate with Z_BLOCK, so it needs the buffers).
    pub fn params(&mut self, io: &mut ZIo, mut level: i32, strategy: i32) -> i32 {
        if level == -1 { level = 6; }
        if !(0..=9).contains(&level) || !(0..=Z_FIXED).contains(&strategy) { return Z_STREAM_ERROR; }
        let func = CONFIG[self.level as usize].func;
        if (strategy != self.strategy || func != CONFIG[level as usize].func) && self.last_flush != -2 {
            let err = self.deflate(io, Z_BLOCK);
            if err == Z_STREAM_ERROR { return err; }
            if io.avail_in() != 0 || (self.strstart as i64 - self.block_start) + self.lookahead as i64 != 0 { return Z_BUF_ERROR; }
        }
        if self.level != level {
            if self.level == 0 && self.matches != 0 {
                if self.matches == 1 { self.slide_hash(); } else { self.clear_hash(); }
                self.matches = 0;
            }
            self.level = level;
            let c = &CONFIG[level as usize];
            self.max_lazy_match = c.lazy as u32;
            self.good_match = c.good as u32;
            self.nice_match = c.nice as i32;
            self.max_chain_length = c.chain as u32;
        }
        self.strategy = strategy;
        Z_OK
    }

    #[inline(always)]
    fn put_byte(&mut self, c: u8) { self.pending_buf[self.pending] = c; self.pending += 1; }
    fn put_short_msb(&mut self, b: u32) { self.put_byte((b >> 8) as u8); self.put_byte(b as u8); }
    #[inline(always)]
    fn put_short(&mut self, w: u16) { self.put_byte(w as u8); self.put_byte((w >> 8) as u8); }

    fn flush_pending(&mut self, io: &mut ZIo) {
        self.tr_flush_bits();
        let len = self.pending.min(io.avail_out());
        if len == 0 { return; }
        let op = io.op;
        io.out[op..op + len].copy_from_slice(&self.pending_buf[self.pending_out..self.pending_out + len]);
        io.op += len;
        self.pending_out += len;
        self.total_out += len as u64;
        self.pending -= len;
        if self.pending == 0 { self.pending_out = 0; }
    }

    /// deflate().
    pub fn deflate(&mut self, io: &mut ZIo, flush: i32) -> i32 {
        if !(0..=Z_BLOCK).contains(&flush) { return Z_STREAM_ERROR; }
        if self.next_out_null || (self.status == FINISH_STATE && flush != Z_FINISH) { self.msg = Some(err_msg(Z_STREAM_ERROR)); return Z_STREAM_ERROR; }
        if io.avail_out() == 0 { self.msg = Some(err_msg(Z_BUF_ERROR)); return Z_BUF_ERROR; }
        let old_flush = self.last_flush;
        self.last_flush = flush;
        if self.pending != 0 {
            self.flush_pending(io);
            if io.avail_out() == 0 { self.last_flush = -1; return Z_OK; }
        } else if io.avail_in() == 0 && rank(flush) <= rank(old_flush) && flush != Z_FINISH {
            self.msg = Some(err_msg(Z_BUF_ERROR));
            return Z_BUF_ERROR;
        }
        if self.status == FINISH_STATE && io.avail_in() != 0 { self.msg = Some(err_msg(Z_BUF_ERROR)); return Z_BUF_ERROR; }
        if self.status == INIT_STATE && self.wrap == 0 { self.status = BUSY_STATE; }
        if self.status == INIT_STATE {
            let mut header = (Z_DEFLATED as u32 + ((self.w_bits - 8) << 4)) << 8;
            let level_flags = if self.strategy >= Z_HUFFMAN_ONLY || self.level < 2 { 0 } else if self.level < 6 { 1 } else if self.level == 6 { 2 } else { 3 };
            header |= level_flags << 6;
            if self.strstart != 0 { header |= PRESET_DICT; }
            header += 31 - (header % 31);
            self.put_short_msb(header);
            if self.strstart != 0 {
                self.put_short_msb(self.adler >> 16);
                self.put_short_msb(self.adler & 0xffff);
            }
            self.adler = 1;
            self.status = BUSY_STATE;
            self.flush_pending(io);
            if self.pending != 0 { self.last_flush = -1; return Z_OK; }
        }
        if self.status == GZIP_STATE {
            self.adler = 0;
            self.put_byte(31);
            self.put_byte(139);
            self.put_byte(8);
            for _ in 0..5 { self.put_byte(0); }
            self.put_byte(if self.level == 9 { 2 } else if self.strategy >= Z_HUFFMAN_ONLY || self.level < 2 { 4 } else { 0 });
            self.put_byte(OS_CODE);
            self.status = BUSY_STATE;
            self.flush_pending(io);
            if self.pending != 0 { self.last_flush = -1; return Z_OK; }
        }
        if io.avail_in() != 0 || self.lookahead != 0 || (flush != Z_NO_FLUSH && self.status != FINISH_STATE) {
            let bstate = if self.level == 0 { self.deflate_stored(io, flush) }
                else if self.strategy == Z_HUFFMAN_ONLY { self.deflate_huff(io, flush) }
                else if self.strategy == Z_RLE { self.deflate_rle(io, flush) }
                else if CONFIG[self.level as usize].func == Func::Fast { self.deflate_fast(io, flush) }
                else { self.deflate_slow(io, flush) };
            if bstate == BlockState::FinishStarted || bstate == BlockState::FinishDone { self.status = FINISH_STATE; }
            if bstate == BlockState::NeedMore || bstate == BlockState::FinishStarted {
                if io.avail_out() == 0 { self.last_flush = -1; }
                return Z_OK;
            }
            if bstate == BlockState::BlockDone {
                if flush == Z_PARTIAL_FLUSH { self.tr_align(); }
                else if flush != Z_BLOCK {
                    self.tr_stored_block(None, 0, false);
                    if flush == Z_FULL_FLUSH {
                        self.clear_hash();
                        if self.lookahead == 0 {
                            self.strstart = 0;
                            self.block_start = 0;
                            self.insert = 0;
                        }
                    }
                }
                self.flush_pending(io);
                if io.avail_out() == 0 { self.last_flush = -1; return Z_OK; }
            }
        }
        if flush != Z_FINISH { return Z_OK; }
        if self.wrap <= 0 { return Z_STREAM_END; }
        if self.wrap == 2 {
            let (a, t) = (self.adler, self.total_in as u32);
            for b in a.to_le_bytes() { self.put_byte(b); }
            for b in t.to_le_bytes() { self.put_byte(b); }
        } else {
            self.put_short_msb(self.adler >> 16);
            self.put_short_msb(self.adler & 0xffff);
        }
        self.flush_pending(io);
        if self.wrap > 0 { self.wrap = -self.wrap; }
        if self.pending != 0 { Z_OK } else { Z_STREAM_END }
    }

    fn longest_match(&mut self, mut cur_match: u32) -> u32 {
        let mut chain_length = self.max_chain_length;
        let strstart = self.strstart as usize;
        let mut best_len = self.prev_length as usize;
        let mut nice_match = self.nice_match as usize;
        let limit = if self.strstart > self.max_dist() { self.strstart - self.max_dist() } else { 0 };
        let wmask = self.w_mask;
        let strend = strstart + MAX_MATCH as usize;
        let win = &self.window[..];
        let mut scan_end1 = win[strstart + best_len - 1];
        let mut scan_end = win[strstart + best_len];
        if self.prev_length >= self.good_match { chain_length >>= 2; }
        if nice_match > self.lookahead as usize { nice_match = self.lookahead as usize; }
        let (s0, s1) = (win[strstart], win[strstart + 1]);
        loop {
            let m = cur_match as usize;
            if !(win[m + best_len] != scan_end || win[m + best_len - 1] != scan_end1 || win[m] != s0 || win[m + 1] != s1) {
                // scan[2] is not compared (equal by the hash), as in zlib
                let mut s = strstart + 2;
                let mut mm = m + 2;
                loop {
                    s += 1; mm += 1; if win[s] != win[mm] { break; }
                    s += 1; mm += 1; if win[s] != win[mm] { break; }
                    s += 1; mm += 1; if win[s] != win[mm] { break; }
                    s += 1; mm += 1; if win[s] != win[mm] { break; }
                    s += 1; mm += 1; if win[s] != win[mm] { break; }
                    s += 1; mm += 1; if win[s] != win[mm] { break; }
                    s += 1; mm += 1; if win[s] != win[mm] { break; }
                    s += 1; mm += 1; if win[s] != win[mm] { break; }
                    if s >= strend { break; }
                }
                let len = MAX_MATCH as usize - (strend - s);
                if len > best_len {
                    self.match_start = cur_match;
                    best_len = len;
                    if len >= nice_match { break; }
                    scan_end1 = win[strstart + best_len - 1];
                    scan_end = win[strstart + best_len];
                }
            }
            cur_match = self.prev[(cur_match & wmask) as usize] as u32;
            if cur_match <= limit { break; }
            chain_length -= 1;
            if chain_length == 0 { break; }
        }
        if best_len as u32 <= self.lookahead { best_len as u32 } else { self.lookahead }
    }

    fn flush_block_only(&mut self, io: &mut ZIo, last: bool) {
        let buf = if self.block_start >= 0 { Some(self.block_start as usize) } else { None };
        let len = (self.strstart as i64 - self.block_start) as u64;
        self.tr_flush_block(buf, len, last);
        self.block_start = self.strstart as i64;
        self.flush_pending(io);
    }

    fn deflate_stored(&mut self, io: &mut ZIo, flush: i32) -> BlockState {
        let mut min_block = (self.pending_buf_size - 5).min(self.w_size as usize) as u32;
        let mut last = false;
        let used0 = io.avail_in() as u32;
        loop {
            let mut len = MAX_STORED;
            let mut have = ((self.bi_valid + 42) >> 3) as u32;
            if (io.avail_out() as u32) < have { break; }
            have = io.avail_out() as u32 - have;
            let mut left = (self.strstart as i64 - self.block_start) as u32;
            if len as u64 > left as u64 + io.avail_in() as u64 { len = left + io.avail_in() as u32; }
            if len > have { len = have; }
            if len < min_block && ((len == 0 && flush != Z_FINISH) || flush == Z_NO_FLUSH || len != left + io.avail_in() as u32) { break; }
            last = flush == Z_FINISH && len == left + io.avail_in() as u32;
            self.tr_stored_block(None, 0, last);
            let p = self.pending;
            self.pending_buf[p - 4] = len as u8;
            self.pending_buf[p - 3] = (len >> 8) as u8;
            self.pending_buf[p - 2] = !len as u8;
            self.pending_buf[p - 1] = (!len >> 8) as u8;
            self.flush_pending(io);
            if left != 0 {
                if left > len { left = len; }
                let (op, bs) = (io.op, self.block_start as usize);
                io.out[op..op + left as usize].copy_from_slice(&self.window[bs..bs + left as usize]);
                io.op += left as usize;
                self.total_out += left as u64;
                self.block_start += left as i64;
                len -= left;
            }
            if len != 0 {
                self.read_buf_out(io, len as usize);
                io.op += len as usize;
                self.total_out += len as u64;
            }
            if last { break; }
        }
        let used = used0 - io.avail_in() as u32;
        if used != 0 {
            if used >= self.w_size {
                self.matches = 2;
                let ws = self.w_size as usize;
                self.window[..ws].copy_from_slice(&io.input[io.ip - ws..io.ip]);
                self.strstart = self.w_size;
                self.insert = self.strstart;
            } else {
                if self.window_size - self.strstart as u64 <= used as u64 {
                    self.strstart -= self.w_size;
                    let (ws, ss) = (self.w_size as usize, self.strstart as usize);
                    self.window.copy_within(ws..ws + ss, 0);
                    if self.matches < 2 { self.matches += 1; }
                    if self.insert > self.strstart { self.insert = self.strstart; }
                }
                let ss = self.strstart as usize;
                self.window[ss..ss + used as usize].copy_from_slice(&io.input[io.ip - used as usize..io.ip]);
                self.strstart += used;
                self.insert += used.min(self.w_size - self.insert);
            }
            self.block_start = self.strstart as i64;
        }
        if self.high_water < self.strstart as u64 { self.high_water = self.strstart as u64; }
        if last { return BlockState::FinishDone; }
        if flush != Z_NO_FLUSH && flush != Z_FINISH && io.avail_in() == 0 && self.strstart as i64 == self.block_start { return BlockState::BlockDone; }
        let mut have = (self.window_size - self.strstart as u64) as u32;
        if io.avail_in() as u64 > have as u64 && self.block_start >= self.w_size as i64 {
            self.block_start -= self.w_size as i64;
            self.strstart -= self.w_size;
            let (ws, ss) = (self.w_size as usize, self.strstart as usize);
            self.window.copy_within(ws..ws + ss, 0);
            if self.matches < 2 { self.matches += 1; }
            have += self.w_size;
            if self.insert > self.strstart { self.insert = self.strstart; }
        }
        if have as usize > io.avail_in() { have = io.avail_in() as u32; }
        if have != 0 {
            let ss = self.strstart as usize;
            self.read_buf_win(io, ss, have as usize);
            self.strstart += have;
            self.insert += have.min(self.w_size - self.insert);
        }
        if self.high_water < self.strstart as u64 { self.high_water = self.strstart as u64; }
        let mut have = ((self.bi_valid + 42) >> 3) as u32;
        have = ((self.pending_buf_size as u32) - have).min(MAX_STORED);
        min_block = have.min(self.w_size);
        let left = (self.strstart as i64 - self.block_start) as u32;
        if left >= min_block || ((left != 0 || flush == Z_FINISH) && flush != Z_NO_FLUSH && io.avail_in() == 0 && left <= have) {
            let len = left.min(have);
            last = flush == Z_FINISH && io.avail_in() == 0 && len == left;
            let bs = self.block_start as usize;
            self.tr_stored_block(Some(bs), len as u64, last);
            self.block_start += len as i64;
            self.flush_pending(io);
        }
        if last { BlockState::FinishStarted } else { BlockState::NeedMore }
    }

    #[inline(always)]
    fn tally_lit(&mut self, c: u8) -> bool {
        let i = self.sym_next as usize;
        self.sym_buf[i] = 0;
        self.sym_buf[i + 1] = 0;
        self.sym_buf[i + 2] = c;
        self.sym_next += 3;
        self.dyn_ltree[c as usize].fc = self.dyn_ltree[c as usize].fc.wrapping_add(1);
        self.sym_next == self.sym_end
    }
    #[inline(always)]
    fn tally_dist(&mut self, distance: u32, length: u32) -> bool {
        let len = length as u8;
        let dist = distance as u16;
        let i = self.sym_next as usize;
        self.sym_buf[i] = dist as u8;
        self.sym_buf[i + 1] = (dist >> 8) as u8;
        self.sym_buf[i + 2] = len;
        self.sym_next += 3;
        let dist = dist.wrapping_sub(1) as usize;
        let lc = ST.length_code[len as usize] as usize + LITERALS + 1;
        self.dyn_ltree[lc].fc = self.dyn_ltree[lc].fc.wrapping_add(1);
        let dc = d_code(dist);
        self.dyn_dtree[dc].fc = self.dyn_dtree[dc].fc.wrapping_add(1);
        self.sym_next == self.sym_end
    }

    fn deflate_fast(&mut self, io: &mut ZIo, flush: i32) -> BlockState {
        loop {
            if self.lookahead < MIN_LOOKAHEAD {
                self.fill_window(io);
                if self.lookahead < MIN_LOOKAHEAD && flush == Z_NO_FLUSH { return BlockState::NeedMore; }
                if self.lookahead == 0 { break; }
            }
            let mut hash_head = 0;
            if self.lookahead >= MIN_MATCH { hash_head = self.insert_string(self.strstart); }
            if hash_head != 0 && self.strstart - hash_head <= self.max_dist() {
                self.match_length = self.longest_match(hash_head);
            }
            let bflush;
            if self.match_length >= MIN_MATCH {
                bflush = self.tally_dist(self.strstart - self.match_start, self.match_length - MIN_MATCH);
                self.lookahead -= self.match_length;
                if self.match_length <= self.max_lazy_match && self.lookahead >= MIN_MATCH {
                    self.match_length -= 1;
                    loop {
                        self.strstart += 1;
                        self.insert_string(self.strstart);
                        self.match_length -= 1;
                        if self.match_length == 0 { break; }
                    }
                    self.strstart += 1;
                } else {
                    self.strstart += self.match_length;
                    self.match_length = 0;
                    self.ins_h = self.window[self.strstart as usize] as u32;
                    self.ins_h = self.update_hash(self.ins_h, self.window[self.strstart as usize + 1]);
                }
            } else {
                bflush = self.tally_lit(self.window[self.strstart as usize]);
                self.lookahead -= 1;
                self.strstart += 1;
            }
            if bflush {
                self.flush_block_only(io, false);
                if io.avail_out() == 0 { return BlockState::NeedMore; }
            }
        }
        self.insert = if self.strstart < MIN_MATCH - 1 { self.strstart } else { MIN_MATCH - 1 };
        self.finish_block(io, flush)
    }

    /// The common tail of deflate_fast/slow/rle/huff.
    fn finish_block(&mut self, io: &mut ZIo, flush: i32) -> BlockState {
        if flush == Z_FINISH {
            self.flush_block_only(io, true);
            if io.avail_out() == 0 { return BlockState::FinishStarted; }
            return BlockState::FinishDone;
        }
        if self.sym_next != 0 {
            self.flush_block_only(io, false);
            if io.avail_out() == 0 { return BlockState::NeedMore; }
        }
        BlockState::BlockDone
    }

    fn deflate_slow(&mut self, io: &mut ZIo, flush: i32) -> BlockState {
        loop {
            if self.lookahead < MIN_LOOKAHEAD {
                self.fill_window(io);
                if self.lookahead < MIN_LOOKAHEAD && flush == Z_NO_FLUSH { return BlockState::NeedMore; }
                if self.lookahead == 0 { break; }
            }
            let mut hash_head = 0;
            if self.lookahead >= MIN_MATCH { hash_head = self.insert_string(self.strstart); }
            self.prev_length = self.match_length;
            self.prev_match = self.match_start;
            self.match_length = MIN_MATCH - 1;
            if hash_head != 0 && self.prev_length < self.max_lazy_match && self.strstart - hash_head <= self.max_dist() {
                self.match_length = self.longest_match(hash_head);
                if self.match_length <= 5 && (self.strategy == Z_FILTERED || (self.match_length == MIN_MATCH && self.strstart - self.match_start > TOO_FAR)) {
                    self.match_length = MIN_MATCH - 1;
                }
            }
            if self.prev_length >= MIN_MATCH && self.match_length <= self.prev_length {
                let max_insert = self.strstart + self.lookahead - MIN_MATCH;
                let bflush = self.tally_dist(self.strstart - 1 - self.prev_match, self.prev_length - MIN_MATCH);
                self.lookahead -= self.prev_length - 1;
                self.prev_length -= 2;
                loop {
                    self.strstart += 1;
                    if self.strstart <= max_insert { self.insert_string(self.strstart); }
                    self.prev_length -= 1;
                    if self.prev_length == 0 { break; }
                }
                self.match_available = false;
                self.match_length = MIN_MATCH - 1;
                self.strstart += 1;
                if bflush {
                    self.flush_block_only(io, false);
                    if io.avail_out() == 0 { return BlockState::NeedMore; }
                }
            } else if self.match_available {
                let bflush = self.tally_lit(self.window[self.strstart as usize - 1]);
                if bflush { self.flush_block_only(io, false); }
                self.strstart += 1;
                self.lookahead -= 1;
                if io.avail_out() == 0 { return BlockState::NeedMore; }
            } else {
                self.match_available = true;
                self.strstart += 1;
                self.lookahead -= 1;
            }
        }
        if self.match_available {
            self.tally_lit(self.window[self.strstart as usize - 1]);
            self.match_available = false;
        }
        self.insert = if self.strstart < MIN_MATCH - 1 { self.strstart } else { MIN_MATCH - 1 };
        self.finish_block(io, flush)
    }

    fn deflate_rle(&mut self, io: &mut ZIo, flush: i32) -> BlockState {
        loop {
            if self.lookahead <= MAX_MATCH {
                self.fill_window(io);
                if self.lookahead <= MAX_MATCH && flush == Z_NO_FLUSH { return BlockState::NeedMore; }
                if self.lookahead == 0 { break; }
            }
            self.match_length = 0;
            if self.lookahead >= MIN_MATCH && self.strstart > 0 {
                let win = &self.window;
                let mut scan = self.strstart as usize - 1;
                let prev = win[scan];
                if prev == win[scan + 1] && prev == win[scan + 2] && prev == win[scan + 3] {
                    scan += 3;
                    let strend = self.strstart as usize + MAX_MATCH as usize;
                    loop {
                        scan += 1; if prev != win[scan] { break; }
                        scan += 1; if prev != win[scan] { break; }
                        scan += 1; if prev != win[scan] { break; }
                        scan += 1; if prev != win[scan] { break; }
                        scan += 1; if prev != win[scan] { break; }
                        scan += 1; if prev != win[scan] { break; }
                        scan += 1; if prev != win[scan] { break; }
                        scan += 1; if prev != win[scan] { break; }
                        if scan >= strend { break; }
                    }
                    self.match_length = MAX_MATCH - (strend - scan) as u32;
                    if self.match_length > self.lookahead { self.match_length = self.lookahead; }
                }
            }
            let bflush;
            if self.match_length >= MIN_MATCH {
                bflush = self.tally_dist(1, self.match_length - MIN_MATCH);
                self.lookahead -= self.match_length;
                self.strstart += self.match_length;
                self.match_length = 0;
            } else {
                bflush = self.tally_lit(self.window[self.strstart as usize]);
                self.lookahead -= 1;
                self.strstart += 1;
            }
            if bflush {
                self.flush_block_only(io, false);
                if io.avail_out() == 0 { return BlockState::NeedMore; }
            }
        }
        self.insert = 0;
        self.finish_block(io, flush)
    }

    fn deflate_huff(&mut self, io: &mut ZIo, flush: i32) -> BlockState {
        loop {
            if self.lookahead == 0 {
                self.fill_window(io);
                if self.lookahead == 0 {
                    if flush == Z_NO_FLUSH { return BlockState::NeedMore; }
                    break;
                }
            }
            self.match_length = 0;
            let bflush = self.tally_lit(self.window[self.strstart as usize]);
            self.lookahead -= 1;
            self.strstart += 1;
            if bflush {
                self.flush_block_only(io, false);
                if io.avail_out() == 0 { return BlockState::NeedMore; }
            }
        }
        self.insert = 0;
        self.finish_block(io, flush)
    }

    // ---- trees.c ----

    #[inline(always)]
    fn send_bits(&mut self, value: u32, length: i32) {
        // shifts in u32: bi_valid can be 16, where C's int promotion drops the bits
        let v = value & 0xffff;
        if self.bi_valid > BUF_SIZE - length {
            self.bi_buf |= (v << self.bi_valid) as u16;
            let b = self.bi_buf;
            self.put_short(b);
            self.bi_buf = (v >> (BUF_SIZE - self.bi_valid)) as u16;
            self.bi_valid += length - BUF_SIZE;
        } else {
            self.bi_buf |= (v << self.bi_valid) as u16;
            self.bi_valid += length;
        }
    }
    fn bi_flush(&mut self) {
        if self.bi_valid == 16 {
            let b = self.bi_buf;
            self.put_short(b);
            self.bi_buf = 0;
            self.bi_valid = 0;
        } else if self.bi_valid >= 8 {
            self.put_byte(self.bi_buf as u8);
            self.bi_buf >>= 8;
            self.bi_valid -= 8;
        }
    }
    fn bi_windup(&mut self) {
        if self.bi_valid > 8 { let b = self.bi_buf; self.put_short(b); }
        else if self.bi_valid > 0 { self.put_byte(self.bi_buf as u8); }
        self.bi_buf = 0;
        self.bi_valid = 0;
    }
    fn init_block(&mut self) {
        for n in 0..L_CODES { self.dyn_ltree[n].fc = 0; }
        for n in 0..D_CODES { self.dyn_dtree[n].fc = 0; }
        for n in 0..BL_CODES { self.bl_tree[n].fc = 0; }
        self.dyn_ltree[END_BLOCK].fc = 1;
        self.hp.opt_len = 0;
        self.hp.static_len = 0;
        self.sym_next = 0;
        self.matches = 0;
    }
    fn tr_init(&mut self) {
        self.bi_buf = 0;
        self.bi_valid = 0;
        self.init_block();
    }
    fn tr_flush_bits(&mut self) { self.bi_flush(); }
    fn tr_align(&mut self) {
        self.send_bits(STATIC_TREES << 1, 3);
        let c = ST.ltree[END_BLOCK];
        self.send_bits(c.fc as u32, c.dl as i32);
        self.bi_flush();
    }
    fn tr_stored_block(&mut self, buf: Option<usize>, stored_len: u64, last: bool) {
        self.send_bits((STORED_BLOCK << 1) + last as u32, 3);
        self.bi_windup();
        self.put_short(stored_len as u16);
        self.put_short(!stored_len as u16);
        if stored_len != 0 {
            let (p, b, n) = (self.pending, buf.unwrap_or(0), stored_len as usize);
            self.pending_buf[p..p + n].copy_from_slice(&self.window[b..b + n]);
        }
        self.pending += stored_len as usize;
    }
    fn send_tree(&mut self, which: Tree, max_code: i32) {
        let mut prevlen: i32 = -1;
        let tl = |s: &Deflate, i: usize| -> i32 { (match which { Tree::L => s.dyn_ltree[i], Tree::D => s.dyn_dtree[i], Tree::BL => s.bl_tree[i] }).dl as i32 };
        let mut nextlen = tl(self, 0);
        let mut count = 0;
        let (mut max_count, mut min_count) = (7, 4);
        if nextlen == 0 { max_count = 138; min_count = 3; }
        for n in 0..=max_code as usize {
            let curlen = nextlen;
            nextlen = tl(self, n + 1);
            count += 1;
            if count < max_count && curlen == nextlen { continue; }
            else if count < min_count {
                loop { self.send_bl_code(curlen as usize); count -= 1; if count == 0 { break; } }
            } else if curlen != 0 {
                if curlen != prevlen { self.send_bl_code(curlen as usize); count -= 1; }
                self.send_bl_code(REP_3_6);
                self.send_bits(count as u32 - 3, 2);
            } else if count <= 10 {
                self.send_bl_code(REPZ_3_10);
                self.send_bits(count as u32 - 3, 3);
            } else {
                self.send_bl_code(REPZ_11_138);
                self.send_bits(count as u32 - 11, 7);
            }
            count = 0;
            prevlen = curlen;
            if nextlen == 0 { max_count = 138; min_count = 3; }
            else if curlen == nextlen { max_count = 6; min_count = 3; }
            else { max_count = 7; min_count = 4; }
        }
    }
    #[inline(always)]
    fn send_bl_code(&mut self, c: usize) { let t = self.bl_tree[c]; self.send_bits(t.fc as u32, t.dl as i32); }
    fn build_bl_tree(&mut self) -> i32 {
        scan_tree(&mut self.bl_tree, &mut self.dyn_ltree, self.l_max_code);
        scan_tree(&mut self.bl_tree, &mut self.dyn_dtree, self.d_max_code);
        build_tree(&mut self.hp, &mut self.bl_tree, &STATIC_BL_DESC);
        let mut max_blindex = BL_CODES as i32 - 1;
        while max_blindex >= 3 {
            if self.bl_tree[BL_ORDER[max_blindex as usize] as usize].dl != 0 { break; }
            max_blindex -= 1;
        }
        self.hp.opt_len = self.hp.opt_len.wrapping_add(3 * (max_blindex as u64 + 1) + 5 + 5 + 4);
        max_blindex
    }
    fn send_all_trees(&mut self, lcodes: i32, dcodes: i32, blcodes: i32) {
        self.send_bits(lcodes as u32 - 257, 5);
        self.send_bits(dcodes as u32 - 1, 5);
        self.send_bits(blcodes as u32 - 4, 4);
        for rank in 0..blcodes as usize {
            let l = self.bl_tree[BL_ORDER[rank] as usize].dl as u32;
            self.send_bits(l, 3);
        }
        self.send_tree(Tree::L, lcodes - 1);
        self.send_tree(Tree::D, dcodes - 1);
    }
    fn compress_block(&mut self, dynamic: bool) {
        let mut sx = 0usize;
        if self.sym_next != 0 {
            loop {
                let mut dist = self.sym_buf[sx] as usize;
                dist += (self.sym_buf[sx + 1] as usize) << 8;
                let mut lc = self.sym_buf[sx + 2] as usize;
                sx += 3;
                if dist == 0 {
                    let t = if dynamic { self.dyn_ltree[lc] } else { ST.ltree[lc] };
                    self.send_bits(t.fc as u32, t.dl as i32);
                } else {
                    let code = ST.length_code[lc] as usize;
                    let t = if dynamic { self.dyn_ltree[code + LITERALS + 1] } else { ST.ltree[code + LITERALS + 1] };
                    self.send_bits(t.fc as u32, t.dl as i32);
                    let extra = EXTRA_LBITS[code];
                    if extra != 0 {
                        lc -= ST.base_length[code] as usize;
                        self.send_bits(lc as u32, extra);
                    }
                    dist -= 1;
                    let code = d_code(dist);
                    let t = if dynamic { self.dyn_dtree[code] } else { ST.dtree[code] };
                    self.send_bits(t.fc as u32, t.dl as i32);
                    let extra = EXTRA_DBITS[code];
                    if extra != 0 {
                        dist -= ST.base_dist[code] as usize;
                        self.send_bits(dist as u32, extra);
                    }
                }
                if sx >= self.sym_next as usize { break; }
            }
        }
        let t = if dynamic { self.dyn_ltree[END_BLOCK] } else { ST.ltree[END_BLOCK] };
        self.send_bits(t.fc as u32, t.dl as i32);
    }
    fn detect_data_type(&self) -> i32 {
        let mut block_mask: u64 = 0xf3ffc07f;
        for n in 0..=31 {
            if block_mask & 1 != 0 && self.dyn_ltree[n].fc != 0 { return Z_BINARY; }
            block_mask >>= 1;
        }
        if self.dyn_ltree[9].fc != 0 || self.dyn_ltree[10].fc != 0 || self.dyn_ltree[13].fc != 0 { return Z_TEXT; }
        for n in 32..LITERALS { if self.dyn_ltree[n].fc != 0 { return Z_TEXT; } }
        Z_BINARY
    }
    fn tr_flush_block(&mut self, buf: Option<usize>, stored_len: u64, last: bool) {
        let mut max_blindex = 0;
        let mut opt_lenb;
        let static_lenb;
        if self.level > 0 {
            if self.data_type == Z_UNKNOWN { self.data_type = self.detect_data_type(); }
            self.l_max_code = build_tree(&mut self.hp, &mut self.dyn_ltree, &STATIC_L_DESC);
            self.d_max_code = build_tree(&mut self.hp, &mut self.dyn_dtree, &STATIC_D_DESC);
            max_blindex = self.build_bl_tree();
            opt_lenb = (self.hp.opt_len.wrapping_add(3 + 7)) >> 3;
            static_lenb = (self.hp.static_len.wrapping_add(3 + 7)) >> 3;
            if static_lenb <= opt_lenb || self.strategy == Z_FIXED { opt_lenb = static_lenb; }
        } else {
            opt_lenb = stored_len + 5;
            static_lenb = opt_lenb;
        }
        if stored_len + 4 <= opt_lenb && buf.is_some() {
            self.tr_stored_block(buf, stored_len, last);
        } else if static_lenb == opt_lenb {
            self.send_bits((STATIC_TREES << 1) + last as u32, 3);
            self.compress_block(false);
        } else {
            self.send_bits((DYN_TREES << 1) + last as u32, 3);
            self.send_all_trees(self.l_max_code + 1, self.d_max_code + 1, max_blindex + 1);
            self.compress_block(true);
        }
        self.init_block();
        if last { self.bi_windup(); }
    }
}
