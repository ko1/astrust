// Fast non-cryptographic hasher (rustc's FxHash) for VM-internal tables keyed by
// symbols / addresses; std's SipHash dominated method-cache and ivar-index lookups.
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default, Clone, Copy)]
pub struct FxHasher { hash: u64 }
const K: u64 = 0xf1357aea2e62a9c5;
impl FxHasher {
    #[inline] fn add(&mut self, w: u64) { self.hash = (self.hash.rotate_left(26) ^ w).wrapping_mul(K); }
}
impl Hasher for FxHasher {
    #[inline] fn write(&mut self, bytes: &[u8]) { for c in bytes.chunks(8) { let mut b = [0u8; 8]; b[..c.len()].copy_from_slice(c); self.add(u64::from_le_bytes(b)); } }
    #[inline] fn write_u8(&mut self, i: u8) { self.add(i as u64) }
    #[inline] fn write_u32(&mut self, i: u32) { self.add(i as u64) }
    #[inline] fn write_u64(&mut self, i: u64) { self.add(i) }
    #[inline] fn write_usize(&mut self, i: usize) { self.add(i as u64) }
    #[inline] fn finish(&self) -> u64 { self.hash.rotate_left(26) }
}
pub type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<FxHasher>>;
