// Adler-32 and CRC-32 of zlib 1.3 (adler32.c, crc32.c), ported to Rust.
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
// Altered: Rust port; crc32 uses slicing-by-8 instead of zlib's braided loop (same values).
#![forbid(unsafe_code)]

const BASE: u32 = 65521;
const NMAX: usize = 5552;

pub fn adler32(adler: u32, buf: &[u8]) -> u32 {
    let mut sum2 = (adler >> 16) & 0xffff;
    let mut a = adler & 0xffff;
    if buf.len() == 1 {
        a += buf[0] as u32;
        if a >= BASE { a -= BASE; }
        sum2 += a;
        if sum2 >= BASE { sum2 -= BASE; }
        return a | (sum2 << 16);
    }
    if buf.len() < 16 {
        for &b in buf { a += b as u32; sum2 += a; }
        if a >= BASE { a -= BASE; }
        sum2 %= BASE;
        return a | (sum2 << 16);
    }
    for chunk in buf.chunks(NMAX) {
        for &b in chunk { a += b as u32; sum2 += a; }
        a %= BASE;
        sum2 %= BASE;
    }
    a | (sum2 << 16)
}

pub fn adler32_combine(adler1: u32, adler2: u32, len2: i64) -> u32 {
    if len2 < 0 { return 0xffff_ffff; }
    let rem = (len2 % BASE as i64) as u32;
    let mut sum1 = adler1 & 0xffff;
    let mut sum2 = (rem as u64 * sum1 as u64 % BASE as u64) as u32;
    sum1 += (adler2 & 0xffff) + BASE - 1;
    sum2 += ((adler1 >> 16) & 0xffff) + ((adler2 >> 16) & 0xffff) + BASE - rem;
    if sum1 >= BASE { sum1 -= BASE; }
    if sum1 >= BASE { sum1 -= BASE; }
    if sum2 >= BASE << 1 { sum2 -= BASE << 1; }
    if sum2 >= BASE { sum2 -= BASE; }
    sum1 | (sum2 << 16)
}

const POLY: u32 = 0xedb8_8320;

const fn make_tables() -> [[u32; 256]; 8] {
    let mut t = [[0u32; 256]; 8];
    let mut i = 0;
    while i < 256 {
        let mut p = i as u32;
        let mut j = 0;
        while j < 8 { p = if p & 1 != 0 { (p >> 1) ^ POLY } else { p >> 1 }; j += 1; }
        t[0][i] = p;
        i += 1;
    }
    let mut k = 1;
    while k < 8 {
        let mut i = 0;
        while i < 256 { let p = t[k - 1][i]; t[k][i] = (p >> 8) ^ t[0][(p & 0xff) as usize]; i += 1; }
        k += 1;
    }
    t
}
static CRC_TABLES: [[u32; 256]; 8] = make_tables();

pub fn crc_table() -> &'static [u32; 256] { &CRC_TABLES[0] }

pub fn crc32(crc: u32, buf: &[u8]) -> u32 {
    let t = &CRC_TABLES;
    let mut c = !crc;
    let mut ch = buf.chunks_exact(8);
    for w in &mut ch {
        let lo = c ^ u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
        c = t[7][(lo & 0xff) as usize] ^ t[6][((lo >> 8) & 0xff) as usize] ^ t[5][((lo >> 16) & 0xff) as usize] ^ t[4][(lo >> 24) as usize]
            ^ t[3][w[4] as usize] ^ t[2][w[5] as usize] ^ t[1][w[6] as usize] ^ t[0][w[7] as usize];
    }
    for &b in ch.remainder() { c = (c >> 8) ^ t[0][((c ^ b as u32) & 0xff) as usize]; }
    !c
}

fn multmodp(a: u32, mut b: u32) -> u32 {
    let mut m = 1u32 << 31;
    let mut p = 0;
    loop {
        if a & m != 0 {
            p ^= b;
            if a & (m - 1) == 0 { break; }
        }
        m >>= 1;
        b = if b & 1 != 0 { (b >> 1) ^ POLY } else { b >> 1 };
    }
    p
}
fn x2n_table() -> [u32; 32] {
    let mut t = [0u32; 32];
    let mut p = 1u32 << 30;
    t[0] = p;
    for e in t.iter_mut().skip(1) { p = multmodp(p, p); *e = p; }
    t
}
fn x2nmodp(mut n: i64, mut k: u32) -> u32 {
    let t = x2n_table();
    let mut p = 1u32 << 31;
    while n != 0 {
        if n & 1 != 0 { p = multmodp(t[(k & 31) as usize], p); }
        n >>= 1;
        if n == -1 { break; }   // zlib loops forever on a negative length
        k += 1;
    }
    p
}
pub fn crc32_combine(crc1: u32, crc2: u32, len2: i64) -> u32 { multmodp(x2nmodp(len2, 3), crc1) ^ crc2 }

// zlib.h constants shared by the deflate and inflate ports.
pub const Z_NO_FLUSH: i32 = 0;
pub const Z_PARTIAL_FLUSH: i32 = 1;
pub const Z_SYNC_FLUSH: i32 = 2;
pub const Z_FULL_FLUSH: i32 = 3;
pub const Z_FINISH: i32 = 4;
pub const Z_BLOCK: i32 = 5;
pub const Z_TREES: i32 = 6;
pub const Z_OK: i32 = 0;
pub const Z_STREAM_END: i32 = 1;
pub const Z_NEED_DICT: i32 = 2;
pub const Z_STREAM_ERROR: i32 = -2;
pub const Z_DATA_ERROR: i32 = -3;
pub const Z_MEM_ERROR: i32 = -4;
pub const Z_BUF_ERROR: i32 = -5;
pub const Z_VERSION_ERROR: i32 = -6;
pub const Z_FILTERED: i32 = 1;
pub const Z_HUFFMAN_ONLY: i32 = 2;
pub const Z_RLE: i32 = 3;
pub const Z_FIXED: i32 = 4;
pub const Z_DEFAULT_STRATEGY: i32 = 0;
pub const Z_BINARY: i32 = 0;
pub const Z_TEXT: i32 = 1;
pub const Z_UNKNOWN: i32 = 2;
pub const Z_DEFLATED: i32 = 8;
pub const MAX_WBITS: i32 = 15;
pub const DEF_MEM_LEVEL: i32 = 8;
pub const MAX_MEM_LEVEL: i32 = 9;
pub const OS_CODE: u8 = 3;

/// zError / ERR_MSG.
pub fn err_msg(err: i32) -> &'static str {
    match err {
        2 => "need dictionary", 1 => "stream end", -1 => "file error", -2 => "stream error", -3 => "data error",
        -4 => "insufficient memory", -5 => "buffer error", -6 => "incompatible version", _ => "",
    }
}

/// z_stream's buffers for one call: `input[ip..]` is next_in/avail_in, `out[op..]` next_out/avail_out.
pub struct ZIo<'a> { pub input: &'a [u8], pub ip: usize, pub out: &'a mut [u8], pub op: usize }
impl ZIo<'_> {
    #[inline] pub fn avail_in(&self) -> usize { self.input.len() - self.ip }
    #[inline] pub fn avail_out(&self) -> usize { self.out.len() - self.op }
}
