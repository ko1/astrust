// digest.so and digest/{md5,sha1,sha2,rmd160,bubblebabble}.so: ext/digest/digest.c and the hash
// functions (MD5 RFC 1321, SHA-1 / SHA-2 FIPS 180-4, RIPEMD-160), native.
#![forbid(unsafe_code)]
use crate::array::*;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

// ---- the hash functions ---------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Algo { Md5, Sha1, Sha256, Sha384, Sha512, Rmd160 }
const ALGOS: [Algo; 6] = [Algo::Md5, Algo::Sha1, Algo::Sha256, Algo::Sha384, Algo::Sha512, Algo::Rmd160];

impl Algo {
    fn digest_len(self) -> usize { match self { Algo::Md5 => 16, Algo::Sha1 | Algo::Rmd160 => 20, Algo::Sha256 => 32, Algo::Sha384 => 48, Algo::Sha512 => 64 } }
    fn block_len(self) -> usize { match self { Algo::Sha384 | Algo::Sha512 => 128, _ => 64 } }
    fn id(self) -> i64 { ALGOS.iter().position(|&a| a == self).unwrap() as i64 }
}

#[derive(Clone)]
pub struct Hasher { algo: Algo, h: [u64; 8], buf: [u8; 128], blen: usize, total: u128 }

impl Hasher {
    pub fn new(algo: Algo) -> Hasher {
        let h: [u64; 8] = match algo {
            Algo::Md5 => [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0, 0, 0, 0],
            Algo::Sha1 | Algo::Rmd160 => [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0, 0, 0, 0],
            Algo::Sha256 => [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19],
            Algo::Sha384 => [0xcbbb9d5dc1059ed8, 0x629a292a367cd507, 0x9159015a3070dd17, 0x152fecd8f70e5939,
                0x67332667ffc00b31, 0x8eb44a8768581511, 0xdb0c2e0d64f98fa7, 0x47b5481dbefa4fa4],
            Algo::Sha512 => [0x6a09e667f3bcc908, 0xbb67ae8584caa73b, 0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
                0x510e527fade682d1, 0x9b05688c2b3e6c1f, 0x1f83d9abfb41bd6b, 0x5be0cd19137e2179],
        };
        Hasher { algo, h, buf: [0; 128], blen: 0, total: 0 }
    }
    pub fn update(&mut self, mut d: &[u8]) {
        let bl = self.algo.block_len();
        self.total += d.len() as u128;
        if self.blen > 0 {
            let n = (bl - self.blen).min(d.len());
            self.buf[self.blen..self.blen + n].copy_from_slice(&d[..n]);
            self.blen += n; d = &d[n..];
            if self.blen < bl { return; }
            let b = self.buf;
            self.compress(&b[..bl]);
            self.blen = 0;
        }
        while d.len() >= bl { self.compress(&d[..bl]); d = &d[bl..]; }
        self.buf[..d.len()].copy_from_slice(d);
        self.blen = d.len();
    }
    pub fn finish(mut self) -> Vec<u8> {
        let bl = self.algo.block_len();
        let bits = self.total.wrapping_mul(8);
        let lenbytes = if bl == 128 { 16 } else { 8 };
        let mut pad = vec![0x80u8];
        let used = (self.blen + 1) % bl;
        let zeros = if used <= bl - lenbytes { bl - lenbytes - used } else { 2 * bl - lenbytes - used };
        pad.resize(1 + zeros, 0);
        match self.algo {
            Algo::Md5 | Algo::Rmd160 => pad.extend_from_slice(&(bits as u64).to_le_bytes()),
            Algo::Sha384 | Algo::Sha512 => pad.extend_from_slice(&bits.to_be_bytes()),
            _ => pad.extend_from_slice(&(bits as u64).to_be_bytes()),
        }
        let total = self.total;
        self.update(&pad);
        self.total = total;
        let mut out = Vec::with_capacity(64);
        match self.algo {
            Algo::Md5 | Algo::Rmd160 => for w in &self.h[..self.algo.digest_len() / 4] { out.extend_from_slice(&(*w as u32).to_le_bytes()); },
            Algo::Sha1 | Algo::Sha256 => for w in &self.h[..self.algo.digest_len() / 4] { out.extend_from_slice(&(*w as u32).to_be_bytes()); },
            _ => for w in &self.h[..self.algo.digest_len() / 8] { out.extend_from_slice(&w.to_be_bytes()); },
        }
        out
    }
    fn compress(&mut self, b: &[u8]) {
        match self.algo {
            Algo::Md5 => md5_block(&mut self.h, b),
            Algo::Sha1 => sha1_block(&mut self.h, b),
            Algo::Sha256 => sha256_block(&mut self.h, b),
            Algo::Sha384 | Algo::Sha512 => sha512_block(&mut self.h, b),
            Algo::Rmd160 => rmd160_block(&mut self.h, b),
        }
    }
}

fn le32(b: &[u8], i: usize) -> u32 { u32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]) }
fn be32(b: &[u8], i: usize) -> u32 { u32::from_be_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]) }

const K_MD5: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391];

fn md5_block(h: &mut [u64; 8], b: &[u8]) {
    const S: [u32; 64] = [7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
        4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21];
    let m: Vec<u32> = (0..16).map(|i| le32(b, i)).collect();
    let (mut a, mut bb, mut c, mut d) = (h[0] as u32, h[1] as u32, h[2] as u32, h[3] as u32);
    for i in 0..64 {
        let (f, g) = match i / 16 {
            0 => ((bb & c) | (!bb & d), i),
            1 => ((d & bb) | (!d & c), (5 * i + 1) % 16),
            2 => (bb ^ c ^ d, (3 * i + 5) % 16),
            _ => (c ^ (bb | !d), (7 * i) % 16),
        };
        let k = K_MD5[i];
        let t = d; d = c; c = bb;
        bb = bb.wrapping_add(a.wrapping_add(f).wrapping_add(k).wrapping_add(m[g]).rotate_left(S[i]));
        a = t;
    }
    h[0] = (h[0] as u32).wrapping_add(a) as u64; h[1] = (h[1] as u32).wrapping_add(bb) as u64;
    h[2] = (h[2] as u32).wrapping_add(c) as u64; h[3] = (h[3] as u32).wrapping_add(d) as u64;
}

fn sha1_block(h: &mut [u64; 8], b: &[u8]) {
    let mut w = [0u32; 80];
    for i in 0..16 { w[i] = be32(b, i); }
    for i in 16..80 { w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1); }
    let mut v = [h[0] as u32, h[1] as u32, h[2] as u32, h[3] as u32, h[4] as u32];
    for (i, &wi) in w.iter().enumerate() {
        let (f, k) = match i / 20 {
            0 => ((v[1] & v[2]) | (!v[1] & v[3]), 0x5a827999),
            1 => (v[1] ^ v[2] ^ v[3], 0x6ed9eba1),
            2 => ((v[1] & v[2]) | (v[1] & v[3]) | (v[2] & v[3]), 0x8f1bbcdc),
            _ => (v[1] ^ v[2] ^ v[3], 0xca62c1d6u32),
        };
        let t = v[0].rotate_left(5).wrapping_add(f).wrapping_add(v[4]).wrapping_add(k).wrapping_add(wi);
        v = [t, v[0], v[1].rotate_left(30), v[2], v[3]];
    }
    for i in 0..5 { h[i] = (h[i] as u32).wrapping_add(v[i]) as u64; }
}

const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2];

fn sha256_block(h: &mut [u64; 8], b: &[u8]) {
    let mut w = [0u32; 64];
    for i in 0..16 { w[i] = be32(b, i); }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
    }
    let mut v = [0u32; 8];
    for i in 0..8 { v[i] = h[i] as u32; }
    for i in 0..64 {
        let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
        let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
        let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K256[i]).wrapping_add(w[i]);
        let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
        let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
        let t2 = s0.wrapping_add(maj);
        v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
    }
    for i in 0..8 { h[i] = (h[i] as u32).wrapping_add(v[i]) as u64; }
}

const K512: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc, 0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2, 0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65, 0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4, 0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df, 0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30, 0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8, 0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec, 0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178, 0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c, 0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817];

fn sha512_block(h: &mut [u64; 8], b: &[u8]) {
    let mut w = [0u64; 80];
    for i in 0..16 { let mut x = [0u8; 8]; x.copy_from_slice(&b[8 * i..8 * i + 8]); w[i] = u64::from_be_bytes(x); }
    for i in 16..80 {
        let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
        let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
        w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
    }
    let mut v = *h;
    for i in 0..80 {
        let s1 = v[4].rotate_right(14) ^ v[4].rotate_right(18) ^ v[4].rotate_right(41);
        let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
        let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K512[i]).wrapping_add(w[i]);
        let s0 = v[0].rotate_right(28) ^ v[0].rotate_right(34) ^ v[0].rotate_right(39);
        let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
        let t2 = s0.wrapping_add(maj);
        v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
    }
    for i in 0..8 { h[i] = h[i].wrapping_add(v[i]); }
}

fn rmd160_block(h: &mut [u64; 8], b: &[u8]) {
    const R: [usize; 80] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 7, 4, 13, 1, 10, 6, 15, 3, 12, 0, 9, 5, 2, 14, 11, 8,
        3, 10, 14, 4, 9, 15, 8, 1, 2, 7, 0, 6, 13, 11, 5, 12, 1, 9, 11, 10, 0, 8, 12, 4, 13, 3, 7, 15, 14, 5, 6, 2,
        4, 0, 5, 9, 7, 12, 2, 10, 14, 1, 3, 8, 11, 6, 15, 13];
    const RP: [usize; 80] = [5, 14, 7, 0, 9, 2, 11, 4, 13, 6, 15, 8, 1, 10, 3, 12, 6, 11, 3, 7, 0, 13, 5, 10, 14, 15, 8, 12, 4, 9, 1, 2,
        15, 5, 1, 3, 7, 14, 6, 9, 11, 8, 12, 2, 10, 0, 4, 13, 8, 6, 4, 1, 3, 11, 15, 0, 5, 12, 2, 13, 9, 7, 10, 14,
        12, 15, 10, 4, 1, 5, 8, 7, 6, 2, 13, 14, 0, 3, 9, 11];
    const S: [u32; 80] = [11, 14, 15, 12, 5, 8, 7, 9, 11, 13, 14, 15, 6, 7, 9, 8, 7, 6, 8, 13, 11, 9, 7, 15, 7, 12, 15, 9, 11, 7, 13, 12,
        11, 13, 6, 7, 14, 9, 13, 15, 14, 8, 13, 6, 5, 12, 7, 5, 11, 12, 14, 15, 14, 15, 9, 8, 9, 14, 5, 6, 8, 6, 5, 12,
        9, 15, 5, 11, 6, 8, 13, 12, 5, 12, 13, 14, 11, 8, 5, 6];
    const SP: [u32; 80] = [8, 9, 9, 11, 13, 15, 15, 5, 7, 7, 8, 11, 14, 14, 12, 6, 9, 13, 15, 7, 12, 8, 9, 11, 7, 7, 12, 7, 6, 15, 13, 11,
        9, 7, 15, 11, 8, 6, 6, 14, 12, 13, 5, 14, 13, 13, 7, 5, 15, 5, 8, 11, 14, 14, 6, 14, 6, 9, 12, 9, 12, 5, 15, 8,
        8, 5, 12, 9, 12, 5, 14, 6, 8, 13, 6, 5, 15, 13, 11, 11];
    const K: [u32; 5] = [0, 0x5a827999, 0x6ed9eba1, 0x8f1bbcdc, 0xa953fd4e];
    const KP: [u32; 5] = [0x50a28be6, 0x5c4dd124, 0x6d703ef3, 0x7a6d76e9, 0];
    fn f(j: usize, x: u32, y: u32, z: u32) -> u32 {
        match j / 16 { 0 => x ^ y ^ z, 1 => (x & y) | (!x & z), 2 => (x | !y) ^ z, 3 => (x & z) | (y & !z), _ => x ^ (y | !z) }
    }
    let x: Vec<u32> = (0..16).map(|i| le32(b, i)).collect();
    let h0 = [h[0] as u32, h[1] as u32, h[2] as u32, h[3] as u32, h[4] as u32];
    let (mut l, mut r) = (h0, h0);
    for j in 0..80 {
        let t = l[0].wrapping_add(f(j, l[1], l[2], l[3])).wrapping_add(x[R[j]]).wrapping_add(K[j / 16]).rotate_left(S[j]).wrapping_add(l[4]);
        l = [l[4], t, l[1], l[2].rotate_left(10), l[3]];
        let t = r[0].wrapping_add(f(79 - j, r[1], r[2], r[3])).wrapping_add(x[RP[j]]).wrapping_add(KP[j / 16]).rotate_left(SP[j]).wrapping_add(r[4]);
        r = [r[4], t, r[1], r[2].rotate_left(10), r[3]];
    }
    let t = h0[1].wrapping_add(l[2]).wrapping_add(r[3]);
    let n = [t, h0[2].wrapping_add(l[3]).wrapping_add(r[4]), h0[3].wrapping_add(l[4]).wrapping_add(r[0]),
        h0[4].wrapping_add(l[0]).wrapping_add(r[1]), h0[0].wrapping_add(l[1]).wrapping_add(r[2])];
    for i in 0..5 { h[i] = n[i] as u64; }
}

// ---- Ruby classes ---------------------------------------------------------------------------

pub struct RDigest { st: Hasher }
impl RData for RDigest {
    fn visit(&mut self, _m: &mut crate::gc::Visitor) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

fn nb() -> *const Block { std::ptr::null() }
fn call0(c: &mut Ctx, v: Value, m: &[u8]) -> RV { funcall(c, v, intern(m), &[], nb()) }
fn call1(c: &mut Ctx, v: Value, m: &[u8], a: Value) -> RV { funcall(c, v, intern(m), &[a], nb()) }
fn hidden(n: &[u8]) -> Sym { intern(n) }
fn digest_mod(c: &mut Ctx) -> Value { let o = c.vm().c_object.get(); const_get_at(o, intern(b"Digest")).unwrap_or(NIL) }

fn hexencode(c: &mut Ctx, v: Value) -> RV {
    let s = expect_str(c, v)?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = Vec::with_capacity(c.str_bytes(s).len() * 2);
    for &b in c.str_bytes(s) { out.push(HEX[(b >> 4) as usize]); out.push(HEX[(b & 15) as usize]); }
    Ok(c.str_new_enc(&out, ENC_ASCII))
}
fn unimpl<T>(c: &mut Ctx, s: Value, m: &str) -> Result<T, Esc> {
    let n = class_path(real_class_of(s));
    err(c, c.vm().c_runtime_error.get(), format!("{} does not implement {}()", n, m))
}
fn obj_clone(c: &mut Ctx, s: Value) -> RV { call0(c, s, b"clone") }
fn check_str(c: &mut Ctx, v: Value) -> RV {
    if is_string(v) || (v.is_heap() && respond_to(v, intern(b"to_str"), true)) { expect_str(c, v) } else { Ok(NIL) }
}

fn s_hexencode(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { hexencode(c, a[0]) }
fn i_update(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { unimpl(c, s, "update") }
fn i_finish(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { unimpl(c, s, "finish") }
fn i_reset(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { unimpl(c, s, "reset") }
fn i_block_length(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { unimpl(c, s, "block_length") }
fn i_new(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let n = obj_clone(c, s)?; call0(c, n, b"reset")?; Ok(n) }
fn digest_of(c: &mut Ctx, s: Value, a: &[Value]) -> RV {
    check_argc(c, a, 0, 1)?;
    if let Some(&str) = a.first() {
        call0(c, s, b"reset")?;
        call1(c, s, b"update", str)?;
        let v = call0(c, s, b"finish")?;
        call0(c, s, b"reset")?;
        Ok(v)
    } else {
        let n = obj_clone(c, s)?;
        call0(c, n, b"finish")
    }
}
fn i_digest(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { digest_of(c, s, a) }
fn i_digest_bang(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let v = call0(c, s, b"finish")?; call0(c, s, b"reset")?; Ok(v) }
fn i_hexdigest(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { let v = digest_of(c, s, a)?; hexencode(c, v) }
fn i_hexdigest_bang(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { let v = i_digest_bang(c, s, a, b)?; hexencode(c, v) }
fn i_to_s(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { call0(c, s, b"hexdigest") }
fn i_inspect(c: &mut Ctx, s: Value, _a: &[Value], b: *const Block) -> RV {
    let h = i_hexdigest(c, s, &[], b)?;
    let mut out = format!("#<{}: ", class_path(real_class_of(s))).into_bytes();
    out.extend_from_slice(c.str_bytes(h));
    out.push(b'>');
    Ok(c.str_new(&out))
}
fn i_equal(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV {
    let inst = const_get_at(digest_mod(c), intern(b"Instance")).unwrap_or(NIL);
    let (s1, s2) = if is_kind_of(a[0], inst) {
        (i_digest(c, s, &[], b)?, i_digest(c, a[0], &[], b)?)
    } else {
        let s1 = i_to_s(c, s, &[], b)?;
        let s2 = check_str(c, a[0])?;
        if s2 == NIL { return Ok(FALSE); }
        (s1, s2)
    };
    let (s1, s2) = (expect_str(c, s1)?, expect_str(c, s2)?);
    Ok(Value::bool(c.str_bytes(s1) == c.str_bytes(s2)))
}
fn i_digest_length(c: &mut Ctx, s: Value, _a: &[Value], b: *const Block) -> RV {
    let d = i_digest(c, s, &[], b)?;
    let d = expect_str(c, d)?;
    Ok(Value::fix(c.str_bytes(d).len() as i64))
}
fn i_length(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { call0(c, s, b"digest_length") }

fn c_s_digest(c: &mut Ctx, k: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return arg_err(c, "no data given"); }
    let str = expect_str(c, a[0])?;
    let o = call0(c, k, b"allocate")?;
    funcall(c, o, intern(b"initialize"), &a[1..], nb())?;
    call1(c, o, b"digest", str)
}
fn c_s_hexdigest(c: &mut Ctx, k: Value, a: &[Value], _b: *const Block) -> RV { let v = funcall(c, k, intern(b"digest"), a, nb())?; hexencode(c, v) }
fn c_init(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(s) }

fn superclass(c: &Ctx, k: Value) -> Value {
    let mut s = c.class(k).superclass;
    while s != NIL && c.class(s).is_module { s = c.class(s).superclass; }
    s
}
/// get_digest_base_metadata: the algorithm of the nearest class carrying `metadata`.
fn algo_of_class(c: &mut Ctx, klass: Value) -> Result<Algo, Esc> {
    let mut p = klass;
    while p != NIL {
        if ivar_defined(p, hidden(b"metadata")) {
            let m = ivar_get(p, hidden(b"metadata"));
            return match if m.is_fixnum() { ALGOS.get(m.as_fix() as usize).copied() } else { None } {
                Some(a) => Ok(a),
                None if p == klass => type_err(c, format!("{}::metadata is not initialized properly", class_path(klass))),
                None => type_err(c, format!("{}({})::metadata is not initialized properly", class_path(klass), class_path(p))),
            };
        }
        p = superclass(c, p);
    }
    err(c, c.vm().c_runtime_error.get(), "Digest::Base cannot be directly inherited in Ruby")
}
fn is_base(k: Value) -> bool { ivar_defined(k, hidden(b"__digest_base")) }
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let mut p = klass;
    while p != NIL && !is_base(p) { p = superclass(c, p); }
    if p == NIL { return Ok(None); }
    if p == klass { return err(c, c.vm().c_not_impl_error.get(), "Digest::Base is an abstract class"); }
    let a = algo_of_class(c, klass)?;
    Ok(Some(c.alloc(klass, Body::Data(Box::new(RDigest { st: Hasher::new(a) })))))
}
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    let Some(d) = c.data::<RDigest>(v) else { return Ok(None) };
    let a = d.st.algo;
    Ok(Some(c.alloc(real_class_of(v), Body::Data(Box::new(RDigest { st: Hasher::new(a) })))))
}
fn ctx(c: &mut Ctx, s: Value) -> Result<&mut RDigest, Esc> {
    if c.data::<RDigest>(s).is_none() { return type_err(c, format!("wrong argument type {} (expected digest)", class_path(real_class_of(s)))); }
    Ok(c.data_mut::<RDigest>(s).unwrap())
}
fn b_copy(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if s == a[0] { return Ok(s); }
    check_frozen(c, s)?;
    let k1 = real_class_of(s); let k2 = real_class_of(a[0]);
    let (a1, a2) = (algo_of_class(c, k1)?, algo_of_class(c, k2)?);
    if a1 != a2 { return type_err(c, "different algorithms"); }
    let st = ctx(c, a[0])?.st.clone();
    ctx(c, s)?.st = st;
    Ok(s)
}
fn b_reset(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let a = algo_of_class(c, real_class_of(s))?;
    ctx(c, s)?.st = Hasher::new(a);
    Ok(s)
}
fn b_update(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    algo_of_class(c, real_class_of(s))?;
    let str = expect_str(c, a[0])?;
    let bytes = c.str_bytes(str).to_vec();
    ctx(c, s)?.st.update(&bytes);
    Ok(s)
}
fn b_finish(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let a = algo_of_class(c, real_class_of(s))?;
    let d = ctx(c, s)?;
    let st = std::mem::replace(&mut d.st, Hasher::new(a));
    let out = st.finish();
    Ok(c.str_new_enc(&out, ENC_BINARY))
}
fn b_digest_length(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let a = algo_of_class(c, real_class_of(s))?; Ok(Value::fix(a.digest_len() as i64)) }
fn b_block_length(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let a = algo_of_class(c, real_class_of(s))?; Ok(Value::fix(a.block_len() as i64)) }

/// Init_digest
pub fn init(c: &mut Ctx) {
    let ob = c.vm().c_object.get();
    let m = match const_get_at(ob, intern(b"Digest")) { Some(v) if is_class_or_module(v) && c.class(v).is_module => v, _ => define_module("Digest") };
    define_module_function(m, "hexencode", s_hexencode, 1);
    let i = define_module_under(m, "Instance");
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(i, n, f, a);
    d("update", i_update, 1); d("<<", i_update, 1);
    define_private_cfunc(i, "finish", i_finish, 0);
    d("reset", i_reset, 0); d("digest_length", i_digest_length, 0); d("block_length", i_block_length, 0);
    d("==", i_equal, 1); d("inspect", i_inspect, 0); d("new", i_new, 0); d("digest", i_digest, -1); d("digest!", i_digest_bang, 0);
    d("hexdigest", i_hexdigest, -1); d("hexdigest!", i_hexdigest_bang, 0); d("to_s", i_to_s, 0); d("length", i_length, 0); d("size", i_length, 0);
    let k = define_class_under(m, "Class", ob);
    define_private_cfunc(k, "initialize", c_init, 0);
    include_module_c(c, k, i);
    define_singleton_cfunc(k, "digest", c_s_digest, -1);
    define_singleton_cfunc(k, "hexdigest", c_s_hexdigest, -1);
    let b = define_class_under(m, "Base", k);
    class_set_alloc(b, AllocKind::None);
    ivar_set_raw(b, hidden(b"__digest_base"), TRUE);
    define_private_cfunc(b, "initialize_copy", b_copy, 1);
    define_cfunc(b, "reset", b_reset, 0);
    define_cfunc(b, "update", b_update, 1);
    define_cfunc(b, "<<", b_update, 1);
    define_private_cfunc(b, "finish", b_finish, 0);
    define_cfunc(b, "digest_length", b_digest_length, 0);
    define_cfunc(b, "block_length", b_block_length, 0);
}

/// Init_md5 / Init_sha1 / Init_sha2 / Init_rmd160 (after `require "digest"`).
pub fn init_algo(c: &mut Ctx, which: &str) {
    let m = digest_mod(c);
    let base = const_get_at(m, intern(b"Base")).unwrap_or(NIL);
    let list: &[(&str, Algo)] = match which {
        "md5" => &[("MD5", Algo::Md5)],
        "sha1" => &[("SHA1", Algo::Sha1)],
        "rmd160" => &[("RMD160", Algo::Rmd160)],
        _ => &[("SHA256", Algo::Sha256), ("SHA384", Algo::Sha384), ("SHA512", Algo::Sha512)],
    };
    for &(n, a) in list {
        let k = define_class_under(m, n, base);
        ivar_set_raw(k, hidden(b"metadata"), Value::fix(a.id()));
    }
}

fn bubblebabble(c: &mut Ctx, v: Value) -> RV {
    let s = expect_str(c, v)?;
    const V: &[u8; 6] = b"aeiouy";
    const C: &[u8; 17] = b"bcdfghklmnprstvzx";
    let d = c.str_bytes(s).to_vec();
    let mut p = vec![b'x'];
    let (mut i, mut seed) = (0usize, 1usize);
    loop {
        if i >= d.len() { p.push(V[seed % 6]); p.push(C[16]); p.push(V[seed / 6]); break; }
        let b1 = d[i] as usize; i += 1;
        p.push(V[(((b1 >> 6) & 3) + seed) % 6]); p.push(C[(b1 >> 2) & 15]); p.push(V[((b1 & 3) + (seed / 6)) % 6]);
        if i >= d.len() { break; }
        let b2 = d[i] as usize; i += 1;
        p.push(C[(b2 >> 4) & 15]); p.push(b'-'); p.push(C[b2 & 15]);
        seed = (seed * 5 + b1 * 7 + b2) % 36;
    }
    p.push(b'x');
    Ok(c.str_new_enc(&p, ENC_BINARY))
}
fn bb_s(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { bubblebabble(c, a[0]) }
fn bb_class(c: &mut Ctx, k: Value, a: &[Value], _b: *const Block) -> RV { let v = funcall(c, k, intern(b"digest"), a, nb())?; bubblebabble(c, v) }
fn bb_inst(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let v = call0(c, s, b"digest")?; bubblebabble(c, v) }
/// Init_bubblebabble
pub fn init_bubblebabble(c: &mut Ctx) {
    let m = digest_mod(c);
    define_module_function(m, "bubblebabble", bb_s, 1);
    let k = const_get_at(m, intern(b"Class")).unwrap_or(NIL);
    define_singleton_cfunc(k, "bubblebabble", bb_class, -1);
    let i = const_get_at(m, intern(b"Instance")).unwrap_or(NIL);
    define_cfunc(i, "bubblebabble", bb_inst, 0);
}
