// inflate of zlib 1.3 (inflate.c, inftrees.c, inffast.c), ported to Rust.
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
// Altered: Rust port of the default build (GUNZIP, no INFLATE_STRICT); no gz_header
// (inflateGetHeader is not ported); the fixed tables are built by inflate_table.
#![forbid(unsafe_code)]
use crate::builtins::zlib_check::*;

#[derive(Clone, Copy, Default)]
struct Code { op: u8, bits: u8, val: u16 }

const ENOUGH_LENS: usize = 852;
const ENOUGH_DISTS: usize = 592;
const ENOUGH: usize = ENOUGH_LENS + ENOUGH_DISTS;
const FIXED_LEN: usize = ENOUGH;
const FIXED_DIST: usize = ENOUGH + 512;
const MAXBITS: usize = 15;

#[derive(Clone, Copy, PartialEq)]
enum CodeType { Codes, Lens, Dists }

static LBASE: [u16; 31] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0];
static LEXT: [u16; 31] = [16, 16, 16, 16, 16, 16, 16, 16, 17, 17, 17, 17, 18, 18, 18, 18, 19, 19, 19, 19, 20, 20, 20, 20, 21, 21, 21, 21, 16, 198, 203];
static DBASE: [u16; 32] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0];
static DEXT: [u16; 32] = [16, 16, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 23, 24, 24, 25, 25, 26, 26, 27, 27, 28, 28, 29, 29, 64, 64];

/// inflate_table: fills `table[*next..]`, advances *next, sets *bits; 0 ok, -1 bad code, 1 too big.
fn inflate_table(ty: CodeType, lens: &[u16], codes: usize, table: &mut [Code], next_at: &mut usize, bits: &mut u32, work: &mut [u16; 288]) -> i32 {
    let mut count = [0u16; MAXBITS + 1];
    let mut offs = [0u16; MAXBITS + 1];
    for &l in &lens[..codes] { count[l as usize] += 1; }
    let mut root = *bits;
    let mut max = MAXBITS as u32;
    while max >= 1 { if count[max as usize] != 0 { break; } max -= 1; }
    if root > max { root = max; }
    let tb = *next_at;
    if max == 0 {
        let here = Code { op: 64, bits: 1, val: 0 };
        table[tb] = here;
        table[tb + 1] = here;
        *next_at += 2;
        *bits = 1;
        return 0;
    }
    let mut min = 1u32;
    while min < max { if count[min as usize] != 0 { break; } min += 1; }
    if root < min { root = min; }
    let mut left: i32 = 1;
    for len in 1..=MAXBITS {
        left <<= 1;
        left -= count[len] as i32;
        if left < 0 { return -1; }
    }
    if left > 0 && (ty == CodeType::Codes || max != 1) { return -1; }
    offs[1] = 0;
    for len in 1..MAXBITS { offs[len + 1] = offs[len] + count[len]; }
    for sym in 0..codes {
        if lens[sym] != 0 { work[offs[lens[sym] as usize] as usize] = sym as u16; offs[lens[sym] as usize] += 1; }
    }
    let (base, extra, mtch): (&[u16], &[u16], u32) = match ty {
        CodeType::Codes => (&[], &[], 20),
        CodeType::Lens => (&LBASE, &LEXT, 257),
        CodeType::Dists => (&DBASE, &DEXT, 0),
    };
    let mut huff: u32 = 0;
    let mut sym: usize = 0;
    let mut len = min;
    let mut next = tb;
    let mut curr = root;
    let mut drop = 0u32;
    let mut low: u32 = u32::MAX;
    let mut used: u32 = 1 << root;
    let mask = used - 1;
    if (ty == CodeType::Lens && used as usize > ENOUGH_LENS) || (ty == CodeType::Dists && used as usize > ENOUGH_DISTS) { return 1; }
    loop {
        let w = work[sym] as u32;
        let here = if w + 1 < mtch { Code { op: 0, bits: (len - drop) as u8, val: w as u16 } }
            else if w >= mtch { Code { op: extra[(w - mtch) as usize] as u8, bits: (len - drop) as u8, val: base[(w - mtch) as usize] } }
            else { Code { op: 32 + 64, bits: (len - drop) as u8, val: 0 } };
        let incr = 1u32 << (len - drop);
        let mut fill = 1u32 << curr;
        min = fill;
        loop {
            fill -= incr;
            table[next + ((huff >> drop) + fill) as usize] = here;
            if fill == 0 { break; }
        }
        let mut incr = 1u32 << (len - 1);
        while huff & incr != 0 { incr >>= 1; }
        if incr != 0 { huff &= incr - 1; huff += incr; } else { huff = 0; }
        sym += 1;
        count[len as usize] -= 1;
        if count[len as usize] == 0 {
            if len == max { break; }
            len = lens[work[sym] as usize] as u32;
        }
        if len > root && (huff & mask) != low {
            if drop == 0 { drop = root; }
            next += min as usize;
            curr = len - drop;
            let mut left: i32 = 1 << curr;
            while curr + drop < max {
                left -= count[(curr + drop) as usize] as i32;
                if left <= 0 { break; }
                curr += 1;
                left <<= 1;
            }
            used += 1 << curr;
            if (ty == CodeType::Lens && used as usize > ENOUGH_LENS) || (ty == CodeType::Dists && used as usize > ENOUGH_DISTS) { return 1; }
            low = huff & mask;
            table[tb + low as usize] = Code { op: curr as u8, bits: root as u8, val: (next - tb) as u16 };
        }
    }
    if huff != 0 {
        table[next + huff as usize] = Code { op: 64, bits: (len - drop) as u8, val: 0 };
    }
    *next_at += used as usize;
    *bits = root;
    0
}

const HEAD: u32 = 16180;
const FLAGS: u32 = 16181;
const TIME: u32 = 16182;
const OS: u32 = 16183;
const EXLEN: u32 = 16184;
const EXTRA: u32 = 16185;
const NAME: u32 = 16186;
const COMMENT: u32 = 16187;
const HCRC: u32 = 16188;
const DICTID: u32 = 16189;
const DICT: u32 = 16190;
const TYPE: u32 = 16191;
const TYPEDO: u32 = 16192;
const STORED: u32 = 16193;
const COPY_: u32 = 16194;
const COPY: u32 = 16195;
const TABLE: u32 = 16196;
const LENLENS: u32 = 16197;
const CODELENS: u32 = 16198;
const LEN_: u32 = 16199;
const LEN: u32 = 16200;
const LENEXT: u32 = 16201;
const DIST: u32 = 16202;
const DISTEXT: u32 = 16203;
const MATCH: u32 = 16204;
const LIT: u32 = 16205;
const CHECK: u32 = 16206;
const LENGTH: u32 = 16207;
const DONE: u32 = 16208;
const BAD: u32 = 16209;
const MEM: u32 = 16210;
const SYNC: u32 = 16211;

static ORDER: [u16; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

#[derive(Clone)]
pub struct Inflate {
    pub total_in: u64,
    pub total_out: u64,
    pub adler: u32,
    pub data_type: i32,
    pub msg: Option<&'static str>,
    mode: u32,
    last: bool,
    wrap: i32,
    havedict: bool,
    flags: i32,
    check: u32,
    total: u64,
    wbits: u32,
    wsize: u32,
    whave: u32,
    wnext: u32,
    window: Vec<u8>,
    hold: u64,
    bits: u32,
    length: u32,
    offset: u32,
    extra: u32,
    lencode: usize,
    distcode: usize,
    lenbits: u32,
    distbits: u32,
    ncode: u32,
    nlen: u32,
    ndist: u32,
    have: u32,
    next: usize,
    lens: [u16; 320],
    work: [u16; 288],
    codes: Vec<Code>,
    back: i32,
    was: u32,
}

#[inline(always)]
fn swap32(q: u64) -> u32 { (q as u32).swap_bytes() }

impl Inflate {
    /// inflateInit2.
    pub fn new(window_bits: i32) -> Result<Box<Inflate>, i32> {
        let mut s = Box::new(Inflate {
            total_in: 0, total_out: 0, adler: 0, data_type: 0, msg: None, mode: HEAD, last: false, wrap: 0, havedict: false, flags: -1,
            check: 0, total: 0, wbits: 0, wsize: 0, whave: 0, wnext: 0, window: Vec::new(), hold: 0, bits: 0, length: 0, offset: 0, extra: 0,
            lencode: 0, distcode: 0, lenbits: 0, distbits: 0, ncode: 0, nlen: 0, ndist: 0, have: 0, next: 0, lens: [0; 320], work: [0; 288],
            codes: vec![Code::default(); ENOUGH + 544], back: -1, was: 0,
        });
        s.build_fixed();
        let r = s.reset2(window_bits);
        if r != Z_OK { return Err(r); }
        Ok(s)
    }
    fn build_fixed(&mut self) {
        let mut lens = [0u16; 320];
        for (i, l) in lens.iter_mut().enumerate().take(288) { *l = if i < 144 { 8 } else if i < 256 { 9 } else if i < 280 { 7 } else { 8 }; }
        let mut next = FIXED_LEN;
        let mut bits = 9;
        let mut work = [0u16; 288];
        inflate_table(CodeType::Lens, &lens, 288, &mut self.codes, &mut next, &mut bits, &mut work);
        for l in lens.iter_mut().take(32) { *l = 5; }
        let mut next = FIXED_DIST;
        let mut bits = 5;
        inflate_table(CodeType::Dists, &lens, 32, &mut self.codes, &mut next, &mut bits, &mut work);
    }
    fn reset_keep(&mut self) {
        self.total_in = 0;
        self.total_out = 0;
        self.total = 0;
        self.msg = None;
        if self.wrap != 0 { self.adler = (self.wrap & 1) as u32; }
        self.mode = HEAD;
        self.last = false;
        self.havedict = false;
        self.flags = -1;
        self.hold = 0;
        self.bits = 0;
        self.lencode = 0;
        self.distcode = 0;
        self.next = 0;
        self.back = -1;
    }
    /// inflateReset.
    pub fn reset(&mut self) {
        self.wsize = 0;
        self.whave = 0;
        self.wnext = 0;
        self.reset_keep();
    }
    fn reset2(&mut self, mut window_bits: i32) -> i32 {
        let wrap;
        if window_bits < 0 {
            if window_bits < -15 { return Z_STREAM_ERROR; }
            wrap = 0;
            window_bits = -window_bits;
        } else {
            wrap = (window_bits >> 4) + 5;
            if window_bits < 48 { window_bits &= 15; }
        }
        if window_bits != 0 && !(8..=15).contains(&window_bits) { return Z_STREAM_ERROR; }
        if !self.window.is_empty() && self.wbits != window_bits as u32 { self.window = Vec::new(); }
        self.wrap = wrap;
        self.wbits = window_bits as u32;
        self.reset();
        Z_OK
    }

    /// updatewindow with the last `src.len()` bytes written.
    fn updatewindow(&mut self, src: &[u8]) {
        if self.window.is_empty() { self.window = vec![0; 1 << self.wbits]; }
        if self.wsize == 0 {
            self.wsize = 1 << self.wbits;
            self.wnext = 0;
            self.whave = 0;
        }
        let mut copy = src.len() as u32;
        let end = src.len();
        if copy >= self.wsize {
            let ws = self.wsize as usize;
            self.window[..ws].copy_from_slice(&src[end - ws..]);
            self.wnext = 0;
            self.whave = self.wsize;
        } else {
            let dist = (self.wsize - self.wnext).min(copy);
            let wn = self.wnext as usize;
            self.window[wn..wn + dist as usize].copy_from_slice(&src[end - copy as usize..end - copy as usize + dist as usize]);
            copy -= dist;
            if copy != 0 {
                self.window[..copy as usize].copy_from_slice(&src[end - copy as usize..]);
                self.wnext = copy;
                self.whave = self.wsize;
            } else {
                self.wnext += dist;
                if self.wnext == self.wsize { self.wnext = 0; }
                if self.whave < self.wsize { self.whave += dist; }
            }
        }
    }

    #[inline(always)]
    fn update_check(&self, check: u32, buf: &[u8]) -> u32 { if self.flags != 0 { crc32(check, buf) } else { adler32(check, buf) } }

    /// inflate().
    pub fn inflate(&mut self, io: &mut ZIo, flush: i32) -> i32 {
        if self.mode == TYPE { self.mode = TYPEDO; }
        let input = io.input;
        let mut next = io.ip;
        let mut have = io.avail_in() as u32;
        let mut put = io.op;
        let mut left = io.avail_out() as u32;
        let mut hold = self.hold;
        let mut bits = self.bits;
        let in0 = have;
        let mut out = left;
        let mut ret = Z_OK;
        macro_rules! pullbyte { ($l:lifetime) => {{ if have == 0 { break $l; } have -= 1; hold += (input[next] as u64) << bits; next += 1; bits += 8; }}; }
        macro_rules! needbits { ($l:lifetime, $n:expr) => {{ while bits < ($n) as u32 { pullbyte!($l); } }}; }
        macro_rules! getbits { ($n:expr) => { (hold as u32) & ((1u32 << ($n)) - 1) }; }
        macro_rules! dropbits { ($n:expr) => {{ let n = ($n) as u32; hold >>= n; bits -= n; }}; }
        macro_rules! initbits { () => {{ hold = 0; bits = 0; }}; }
        macro_rules! bytebits { () => {{ hold >>= bits & 7; bits -= bits & 7; }}; }
        macro_rules! crc2 { ($w:expr) => {{ let w = $w; self.check = crc32(self.check, &[w as u8, (w >> 8) as u8]); }}; }
        macro_rules! crc4 { ($w:expr) => {{ let w = $w; self.check = crc32(self.check, &[w as u8, (w >> 8) as u8, (w >> 16) as u8, (w >> 24) as u8]); }}; }
        'leave: loop {
            match self.mode {
                HEAD => {
                    if self.wrap == 0 { self.mode = TYPEDO; continue; }
                    needbits!('leave, 16);
                    if (self.wrap & 2) != 0 && hold == 0x8b1f {
                        if self.wbits == 0 { self.wbits = 15; }
                        self.check = 0;
                        crc2!(hold);
                        initbits!();
                        self.mode = FLAGS;
                        continue;
                    }
                    if (self.wrap & 1) == 0 || ((getbits!(8) << 8) + (hold >> 8) as u32) % 31 != 0 {
                        self.msg = Some("incorrect header check");
                        self.mode = BAD;
                        continue;
                    }
                    if getbits!(4) != Z_DEFLATED as u32 {
                        self.msg = Some("unknown compression method");
                        self.mode = BAD;
                        continue;
                    }
                    dropbits!(4);
                    let len = getbits!(4) + 8;
                    if self.wbits == 0 { self.wbits = len; }
                    if len > 15 || len > self.wbits {
                        self.msg = Some("invalid window size");
                        self.mode = BAD;
                        continue;
                    }
                    self.flags = 0;
                    self.check = 1;
                    self.adler = 1;
                    self.mode = if hold & 0x200 != 0 { DICTID } else { TYPE };
                    initbits!();
                }
                FLAGS => {
                    needbits!('leave, 16);
                    self.flags = hold as i32;
                    if (self.flags & 0xff) != Z_DEFLATED {
                        self.msg = Some("unknown compression method");
                        self.mode = BAD;
                        continue;
                    }
                    if self.flags & 0xe000 != 0 {
                        self.msg = Some("unknown header flags set");
                        self.mode = BAD;
                        continue;
                    }
                    if (self.flags & 0x0200) != 0 && (self.wrap & 4) != 0 { crc2!(hold); }
                    initbits!();
                    self.mode = TIME;
                }
                TIME => {
                    needbits!('leave, 32);
                    if (self.flags & 0x0200) != 0 && (self.wrap & 4) != 0 { crc4!(hold); }
                    initbits!();
                    self.mode = OS;
                }
                OS => {
                    needbits!('leave, 16);
                    if (self.flags & 0x0200) != 0 && (self.wrap & 4) != 0 { crc2!(hold); }
                    initbits!();
                    self.mode = EXLEN;
                }
                EXLEN => {
                    if self.flags & 0x0400 != 0 {
                        needbits!('leave, 16);
                        self.length = hold as u32;
                        if (self.flags & 0x0200) != 0 && (self.wrap & 4) != 0 { crc2!(hold); }
                        initbits!();
                    }
                    self.mode = EXTRA;
                }
                EXTRA => {
                    if self.flags & 0x0400 != 0 {
                        let copy = self.length.min(have);
                        if copy != 0 {
                            if (self.flags & 0x0200) != 0 && (self.wrap & 4) != 0 { self.check = crc32(self.check, &input[next..next + copy as usize]); }
                            have -= copy;
                            next += copy as usize;
                            self.length -= copy;
                        }
                        if self.length != 0 { break 'leave; }
                    }
                    self.length = 0;
                    self.mode = NAME;
                }
                NAME | COMMENT => {
                    let bit = if self.mode == NAME { 0x0800 } else { 0x1000 };
                    if self.flags & bit != 0 {
                        if have == 0 { break 'leave; }
                        let mut copy = 0u32;
                        let mut len;
                        loop {
                            len = input[next + copy as usize];
                            copy += 1;
                            if !(len != 0 && copy < have) { break; }
                        }
                        if (self.flags & 0x0200) != 0 && (self.wrap & 4) != 0 { self.check = crc32(self.check, &input[next..next + copy as usize]); }
                        have -= copy;
                        next += copy as usize;
                        if len != 0 { break 'leave; }
                    }
                    self.length = 0;
                    self.mode = if self.mode == NAME { COMMENT } else { HCRC };
                }
                HCRC => {
                    if self.flags & 0x0200 != 0 {
                        needbits!('leave, 16);
                        if (self.wrap & 4) != 0 && hold != (self.check & 0xffff) as u64 {
                            self.msg = Some("header crc mismatch");
                            self.mode = BAD;
                            continue;
                        }
                        initbits!();
                    }
                    self.check = 0;
                    self.adler = 0;
                    self.mode = TYPE;
                }
                DICTID => {
                    needbits!('leave, 32);
                    self.check = swap32(hold);
                    self.adler = self.check;
                    initbits!();
                    self.mode = DICT;
                }
                DICT => {
                    if !self.havedict {
                        io.op = put; io.ip = next; self.hold = hold; self.bits = bits;
                        return Z_NEED_DICT;
                    }
                    self.check = 1;
                    self.adler = 1;
                    self.mode = TYPE;
                }
                TYPE | TYPEDO => {
                    if self.mode == TYPE && (flush == Z_BLOCK || flush == Z_TREES) { break 'leave; }
                    if self.last {
                        bytebits!();
                        self.mode = CHECK;
                        continue;
                    }
                    needbits!('leave, 3);
                    self.last = getbits!(1) != 0;
                    dropbits!(1);
                    match getbits!(2) {
                        0 => self.mode = STORED,
                        1 => {
                            self.lencode = FIXED_LEN;
                            self.lenbits = 9;
                            self.distcode = FIXED_DIST;
                            self.distbits = 5;
                            self.mode = LEN_;
                            if flush == Z_TREES { dropbits!(2); break 'leave; }
                        }
                        2 => self.mode = TABLE,
                        _ => {
                            self.msg = Some("invalid block type");
                            self.mode = BAD;
                        }
                    }
                    dropbits!(2);
                }
                STORED => {
                    bytebits!();
                    needbits!('leave, 32);
                    if (hold & 0xffff) != ((hold >> 16) ^ 0xffff) {
                        self.msg = Some("invalid stored block lengths");
                        self.mode = BAD;
                        continue;
                    }
                    self.length = (hold & 0xffff) as u32;
                    initbits!();
                    self.mode = COPY_;
                    if flush == Z_TREES { break 'leave; }
                }
                COPY_ => { self.mode = COPY; }
                COPY => {
                    let mut copy = self.length;
                    if copy != 0 {
                        if copy > have { copy = have; }
                        if copy > left { copy = left; }
                        if copy == 0 { break 'leave; }
                        io.out[put..put + copy as usize].copy_from_slice(&input[next..next + copy as usize]);
                        have -= copy;
                        next += copy as usize;
                        left -= copy;
                        put += copy as usize;
                        self.length -= copy;
                        continue;
                    }
                    self.mode = TYPE;
                }
                TABLE => {
                    needbits!('leave, 14);
                    self.nlen = getbits!(5) + 257;
                    dropbits!(5);
                    self.ndist = getbits!(5) + 1;
                    dropbits!(5);
                    self.ncode = getbits!(4) + 4;
                    dropbits!(4);
                    if self.nlen > 286 || self.ndist > 30 {
                        self.msg = Some("too many length or distance symbols");
                        self.mode = BAD;
                        continue;
                    }
                    self.have = 0;
                    self.mode = LENLENS;
                }
                LENLENS => {
                    while self.have < self.ncode {
                        needbits!('leave, 3);
                        self.lens[ORDER[self.have as usize] as usize] = getbits!(3) as u16;
                        self.have += 1;
                        dropbits!(3);
                    }
                    while self.have < 19 {
                        self.lens[ORDER[self.have as usize] as usize] = 0;
                        self.have += 1;
                    }
                    self.next = 0;
                    self.lencode = 0;
                    self.lenbits = 7;
                    let mut nx = self.next;
                    let mut lb = self.lenbits;
                    let r = inflate_table(CodeType::Codes, &self.lens, 19, &mut self.codes, &mut nx, &mut lb, &mut self.work);
                    self.next = nx;
                    self.lenbits = lb;
                    if r != 0 {
                        self.msg = Some("invalid code lengths set");
                        self.mode = BAD;
                        continue;
                    }
                    self.have = 0;
                    self.mode = CODELENS;
                }
                CODELENS => {
                    while self.have < self.nlen + self.ndist {
                        let mut here;
                        loop {
                            here = self.codes[self.lencode + getbits!(self.lenbits) as usize];
                            if here.bits as u32 <= bits { break; }
                            pullbyte!('leave);
                        }
                        if here.val < 16 {
                            dropbits!(here.bits);
                            self.lens[self.have as usize] = here.val;
                            self.have += 1;
                        } else {
                            let len;
                            let mut copy;
                            if here.val == 16 {
                                needbits!('leave, here.bits as u32 + 2);
                                dropbits!(here.bits);
                                if self.have == 0 {
                                    self.msg = Some("invalid bit length repeat");
                                    self.mode = BAD;
                                    break;
                                }
                                len = self.lens[self.have as usize - 1];
                                copy = 3 + getbits!(2);
                                dropbits!(2);
                            } else if here.val == 17 {
                                needbits!('leave, here.bits as u32 + 3);
                                dropbits!(here.bits);
                                len = 0;
                                copy = 3 + getbits!(3);
                                dropbits!(3);
                            } else {
                                needbits!('leave, here.bits as u32 + 7);
                                dropbits!(here.bits);
                                len = 0;
                                copy = 11 + getbits!(7);
                                dropbits!(7);
                            }
                            if self.have + copy > self.nlen + self.ndist {
                                self.msg = Some("invalid bit length repeat");
                                self.mode = BAD;
                                break;
                            }
                            while copy > 0 { copy -= 1; self.lens[self.have as usize] = len; self.have += 1; }
                        }
                    }
                    if self.mode == BAD { continue; }
                    if self.lens[256] == 0 {
                        self.msg = Some("invalid code -- missing end-of-block");
                        self.mode = BAD;
                        continue;
                    }
                    self.next = 0;
                    self.lencode = 0;
                    self.lenbits = 9;
                    let mut nx = 0;
                    let mut lb = 9;
                    let nlen = self.nlen as usize;
                    let r = inflate_table(CodeType::Lens, &self.lens, nlen, &mut self.codes, &mut nx, &mut lb, &mut self.work);
                    self.lenbits = lb;
                    if r != 0 {
                        self.next = nx;
                        self.msg = Some("invalid literal/lengths set");
                        self.mode = BAD;
                        continue;
                    }
                    self.distcode = nx;
                    self.distbits = 6;
                    let mut db = 6;
                    let lens2 = &self.lens[nlen..];
                    let r = inflate_table(CodeType::Dists, lens2, self.ndist as usize, &mut self.codes, &mut nx, &mut db, &mut self.work);
                    self.next = nx;
                    self.distbits = db;
                    if r != 0 {
                        self.msg = Some("invalid distances set");
                        self.mode = BAD;
                        continue;
                    }
                    self.mode = LEN_;
                    if flush == Z_TREES { break 'leave; }
                }
                LEN_ => { self.mode = LEN; }
                LEN => {
                    if have >= 6 && left >= 258 {
                        let r = self.inflate_fast(input, &mut next, &mut have, io.out, &mut put, &mut left, &mut hold, &mut bits, out);
                        let _ = r;
                        if self.mode == TYPE { self.back = -1; }
                        continue;
                    }
                    self.back = 0;
                    let mut here;
                    loop {
                        here = self.codes[self.lencode + getbits!(self.lenbits) as usize];
                        if here.bits as u32 <= bits { break; }
                        pullbyte!('leave);
                    }
                    if here.op != 0 && (here.op & 0xf0) == 0 {
                        let last = here;
                        loop {
                            here = self.codes[self.lencode + last.val as usize + (getbits!(last.bits as u32 + last.op as u32) >> last.bits) as usize];
                            if (last.bits as u32 + here.bits as u32) <= bits { break; }
                            pullbyte!('leave);
                        }
                        dropbits!(last.bits);
                        self.back += last.bits as i32;
                    }
                    dropbits!(here.bits);
                    self.back += here.bits as i32;
                    self.length = here.val as u32;
                    if here.op == 0 { self.mode = LIT; continue; }
                    if here.op & 32 != 0 { self.back = -1; self.mode = TYPE; continue; }
                    if here.op & 64 != 0 {
                        self.msg = Some("invalid literal/length code");
                        self.mode = BAD;
                        continue;
                    }
                    self.extra = (here.op & 15) as u32;
                    self.mode = LENEXT;
                }
                LENEXT => {
                    if self.extra != 0 {
                        needbits!('leave, self.extra);
                        self.length += getbits!(self.extra);
                        dropbits!(self.extra);
                        self.back += self.extra as i32;
                    }
                    self.was = self.length;
                    self.mode = DIST;
                }
                DIST => {
                    let mut here;
                    loop {
                        here = self.codes[self.distcode + getbits!(self.distbits) as usize];
                        if here.bits as u32 <= bits { break; }
                        pullbyte!('leave);
                    }
                    if (here.op & 0xf0) == 0 {
                        let last = here;
                        loop {
                            here = self.codes[self.distcode + last.val as usize + (getbits!(last.bits as u32 + last.op as u32) >> last.bits) as usize];
                            if (last.bits as u32 + here.bits as u32) <= bits { break; }
                            pullbyte!('leave);
                        }
                        dropbits!(last.bits);
                        self.back += last.bits as i32;
                    }
                    dropbits!(here.bits);
                    self.back += here.bits as i32;
                    if here.op & 64 != 0 {
                        self.msg = Some("invalid distance code");
                        self.mode = BAD;
                        continue;
                    }
                    self.offset = here.val as u32;
                    self.extra = (here.op & 15) as u32;
                    self.mode = DISTEXT;
                }
                DISTEXT => {
                    if self.extra != 0 {
                        needbits!('leave, self.extra);
                        self.offset += getbits!(self.extra);
                        dropbits!(self.extra);
                        self.back += self.extra as i32;
                    }
                    self.mode = MATCH;
                }
                MATCH => {
                    if left == 0 { break 'leave; }
                    let mut copy = out - left;
                    if self.offset > copy {
                        copy = self.offset - copy;
                        if copy > self.whave {
                            self.msg = Some("invalid distance too far back");
                            self.mode = BAD;
                            continue;
                        }
                        let from = if copy > self.wnext { copy -= self.wnext; (self.wsize - copy) as usize } else { (self.wnext - copy) as usize };
                        if copy > self.length { copy = self.length; }
                        if copy > left { copy = left; }
                        left -= copy;
                        self.length -= copy;
                        io.out[put..put + copy as usize].copy_from_slice(&self.window[from..from + copy as usize]);
                        put += copy as usize;
                    } else {
                        let from = put - self.offset as usize;
                        copy = self.length;
                        if copy > left { copy = left; }
                        left -= copy;
                        self.length -= copy;
                        for i in 0..copy as usize { io.out[put + i] = io.out[from + i]; }
                        put += copy as usize;
                    }
                    if self.length == 0 { self.mode = LEN; }
                }
                LIT => {
                    if left == 0 { break 'leave; }
                    io.out[put] = self.length as u8;
                    put += 1;
                    left -= 1;
                    self.mode = LEN;
                }
                CHECK => {
                    if self.wrap != 0 {
                        needbits!('leave, 32);
                        out -= left;
                        self.total_out += out as u64;
                        self.total += out as u64;
                        if (self.wrap & 4) != 0 && out != 0 {
                            self.check = self.update_check(self.check, &io.out[put - out as usize..put]);
                            self.adler = self.check;
                        }
                        out = left;
                        if (self.wrap & 4) != 0 && (if self.flags != 0 { hold as u32 } else { swap32(hold) }) != self.check {
                            self.msg = Some("incorrect data check");
                            self.mode = BAD;
                            continue;
                        }
                        initbits!();
                    }
                    self.mode = LENGTH;
                }
                LENGTH => {
                    if self.wrap != 0 && self.flags != 0 {
                        needbits!('leave, 32);
                        if (self.wrap & 4) != 0 && hold != (self.total & 0xffff_ffff) {
                            self.msg = Some("incorrect length check");
                            self.mode = BAD;
                            continue;
                        }
                        initbits!();
                    }
                    self.mode = DONE;
                }
                DONE => { ret = Z_STREAM_END; break 'leave; }
                BAD => { ret = Z_DATA_ERROR; break 'leave; }
                MEM => { return Z_MEM_ERROR; }
                _ => { return Z_STREAM_ERROR; }
            }
        }
        // inf_leave
        io.op = put;
        io.ip = next;
        self.hold = hold;
        self.bits = bits;
        let avail_out = left;
        if self.wsize != 0 || (out != avail_out && self.mode < BAD && (self.mode < CHECK || flush != Z_FINISH)) {
            let n = (out - avail_out) as usize;
            let (s, e) = (put - n, put);
            let tmp: &[u8] = &io.out[s..e];
            self.updatewindow(tmp);
        }
        let inn = in0 - have;
        let outn = out - avail_out;
        self.total_in += inn as u64;
        self.total_out += outn as u64;
        self.total += outn as u64;
        if (self.wrap & 4) != 0 && outn != 0 {
            self.check = self.update_check(self.check, &io.out[put - outn as usize..put]);
            self.adler = self.check;
        }
        self.data_type = self.bits as i32 + if self.last { 64 } else { 0 } + if self.mode == TYPE { 128 } else { 0 } + if self.mode == LEN_ || self.mode == COPY_ { 256 } else { 0 };
        if ((inn == 0 && outn == 0) || flush == Z_FINISH) && ret == Z_OK { ret = Z_BUF_ERROR; }
        ret
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    fn inflate_fast(&mut self, input: &[u8], next: &mut usize, have: &mut u32, out: &mut [u8], put: &mut usize, left: &mut u32, hold_r: &mut u64, bits_r: &mut u32, start: u32) {
        let mut inp = *next;
        let last = inp + (*have as usize - 5);
        let mut o = *put;
        let beg = o - (start - *left) as usize;
        let end = o + (*left as usize - 257);
        let wsize = self.wsize as usize;
        let whave = self.whave as usize;
        let wnext = self.wnext as usize;
        let mut hold = *hold_r;
        let mut bits = *bits_r;
        let lcode = self.lencode;
        let dcode = self.distcode;
        let lmask = (1u64 << self.lenbits) - 1;
        let dmask = (1u64 << self.distbits) - 1;
        let codes = &self.codes;
        let window = &self.window;
        let mut mode_set: Option<(u32, &'static str)> = None;
        'outer: loop {
            if bits < 15 {
                hold += (input[inp] as u64) << bits; inp += 1; bits += 8;
                hold += (input[inp] as u64) << bits; inp += 1; bits += 8;
            }
            let mut here = codes[lcode + (hold & lmask) as usize];
            loop {
                // dolen
                let op = here.bits as u32;
                hold >>= op;
                bits -= op;
                let op = here.op as u32;
                if op == 0 {
                    out[o] = here.val as u8;
                    o += 1;
                    break;
                } else if op & 16 != 0 {
                    let mut len = here.val as usize;
                    let op = op & 15;
                    if op != 0 {
                        if bits < op { hold += (input[inp] as u64) << bits; inp += 1; bits += 8; }
                        len += (hold as usize) & ((1usize << op) - 1);
                        hold >>= op;
                        bits -= op;
                    }
                    if bits < 15 {
                        hold += (input[inp] as u64) << bits; inp += 1; bits += 8;
                        hold += (input[inp] as u64) << bits; inp += 1; bits += 8;
                    }
                    let mut here = codes[dcode + (hold & dmask) as usize];
                    loop {
                        // dodist
                        let op = here.bits as u32;
                        hold >>= op;
                        bits -= op;
                        let op = here.op as u32;
                        if op & 16 != 0 {
                            let mut dist = here.val as usize;
                            let op = op & 15;
                            if bits < op {
                                hold += (input[inp] as u64) << bits; inp += 1; bits += 8;
                                if bits < op { hold += (input[inp] as u64) << bits; inp += 1; bits += 8; }
                            }
                            dist += (hold as usize) & ((1usize << op) - 1);
                            hold >>= op;
                            bits -= op;
                            let op = o - beg;
                            if dist > op {
                                let mut op = dist - op;
                                if op > whave {
                                    mode_set = Some((BAD, "invalid distance too far back"));
                                    break 'outer;
                                }
                                let mut from;
                                let mut from_win = true;
                                if wnext == 0 {
                                    from = wsize - op;
                                    if op < len {
                                        len -= op;
                                        out[o..o + op].copy_from_slice(&window[from..from + op]);
                                        o += op;
                                        from = o - dist;
                                        from_win = false;
                                    }
                                } else if wnext < op {
                                    from = wsize + wnext - op;
                                    op -= wnext;
                                    if op < len {
                                        len -= op;
                                        out[o..o + op].copy_from_slice(&window[from..from + op]);
                                        o += op;
                                        from = 0;
                                        if wnext < len {
                                            let op = wnext;
                                            len -= op;
                                            out[o..o + op].copy_from_slice(&window[..op]);
                                            o += op;
                                            from = o - dist;
                                            from_win = false;
                                        }
                                    }
                                } else {
                                    from = wnext - op;
                                    if op < len {
                                        len -= op;
                                        out[o..o + op].copy_from_slice(&window[from..from + op]);
                                        o += op;
                                        from = o - dist;
                                        from_win = false;
                                    }
                                }
                                if from_win {
                                    out[o..o + len].copy_from_slice(&window[from..from + len]);
                                    o += len;
                                } else {
                                    for i in 0..len { out[o + i] = out[from + i]; }
                                    o += len;
                                }
                            } else {
                                let from = o - dist;
                                if dist >= len { out.copy_within(from..from + len, o); } else { for i in 0..len { out[o + i] = out[from + i]; } }
                                o += len;
                            }
                            break;
                        } else if op & 64 == 0 {
                            here = codes[dcode + here.val as usize + (hold & ((1u64 << op) - 1)) as usize];
                            continue;
                        } else {
                            mode_set = Some((BAD, "invalid distance code"));
                            break 'outer;
                        }
                    }
                    break;
                } else if op & 64 == 0 {
                    here = codes[lcode + here.val as usize + (hold & ((1u64 << op) - 1)) as usize];
                    continue;
                } else if op & 32 != 0 {
                    mode_set = Some((TYPE, ""));
                    break 'outer;
                } else {
                    mode_set = Some((BAD, "invalid literal/length code"));
                    break 'outer;
                }
            }
            if !(inp < last && o < end) { break; }
        }
        if let Some((m, msg)) = mode_set {
            self.mode = m;
            if m == BAD { self.msg = Some(msg); }
        }
        let len = (bits >> 3) as usize;
        inp -= len;
        bits -= (len as u32) << 3;
        hold &= (1u64 << bits) - 1;
        *next = inp;
        *put = o;
        *have = (5 + last as isize - inp as isize) as u32;
        *left = (257 + end as isize - o as isize) as u32;
        *hold_r = hold;
        *bits_r = bits;
    }

    /// inflateSetDictionary.
    pub fn set_dictionary(&mut self, dict: &[u8]) -> i32 {
        if self.wrap != 0 && self.mode != DICT { return Z_STREAM_ERROR; }
        if self.mode == DICT && adler32(1, dict) != self.check { return Z_DATA_ERROR; }
        self.updatewindow(dict);
        self.havedict = true;
        Z_OK
    }

    /// inflateSync; `io.ip` advances past what was searched.
    pub fn sync(&mut self, io: &mut ZIo) -> i32 {
        if io.avail_in() == 0 && self.bits < 8 { return Z_BUF_ERROR; }
        if self.mode != SYNC {
            self.mode = SYNC;
            self.hold <<= self.bits & 7;
            self.bits -= self.bits & 7;
            let mut buf = [0u8; 4];
            let mut len = 0;
            while self.bits >= 8 {
                buf[len] = self.hold as u8;
                len += 1;
                self.hold >>= 8;
                self.bits -= 8;
            }
            self.have = 0;
            let mut h = self.have;
            syncsearch(&mut h, &buf[..len]);
            self.have = h;
        }
        let mut h = self.have;
        let len = syncsearch(&mut h, &io.input[io.ip..]);
        self.have = h;
        io.ip += len;
        self.total_in += len as u64;
        if self.have != 4 { return Z_DATA_ERROR; }
        if self.flags == -1 { self.wrap = 0; } else { self.wrap &= !4; }
        let flags = self.flags;
        let (i, o) = (self.total_in, self.total_out);
        self.reset();
        self.total_in = i;
        self.total_out = o;
        self.flags = flags;
        self.mode = TYPE;
        Z_OK
    }
    /// inflateSyncPoint.
    pub fn sync_point(&self) -> bool { self.mode == STORED && self.bits == 0 }
}

fn syncsearch(have: &mut u32, buf: &[u8]) -> usize {
    let mut got = *have;
    let mut next = 0;
    while next < buf.len() && got < 4 {
        if buf[next] as u32 == (if got < 2 { 0 } else { 0xff }) { got += 1; }
        else if buf[next] != 0 { got = 0; }
        else { got = 4 - got; }
        next += 1;
    }
    *have = got;
    next
}
