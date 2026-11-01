#![cfg(feature = "rabin-karp")]
//! Integer Rabin–Karp rolling checksum, interoperable with librsync's arithmetic.
//! Reference: <https://librsync.github.io/rabinkarp_8h_source.html>.
use crate::RollingHash;
const BASE: u32 = 0x08104225;

/// A seeded polynomial checksum modulo 2^32 over a fixed-size window.
#[derive(Debug, Clone)]
pub struct RabinKarp {
    window: Box<[u8]>,
    pos: usize,
    filled: usize,
    hash: u32,
    power: u32,
}
impl RabinKarp {
    /// Panics for an empty window.
    pub fn new(window: usize) -> Self {
        assert!(window > 0, "window must be positive");
        Self {
            window: vec![0; window].into_boxed_slice(),
            pos: 0,
            filled: 0,
            hash: 1,
            power: 1,
        }
    }
}
impl RollingHash for RabinKarp {
    type Digest = u32;
    fn roll_byte(&mut self, byte: u8) -> u32 {
        self.hash = self.hash.wrapping_mul(BASE).wrapping_add(u32::from(byte));
        if self.filled == self.window.len() {
            let old = u32::from(self.window[self.pos]);
            self.hash = self
                .hash
                .wrapping_sub(self.power.wrapping_mul(old.wrapping_add(BASE - 1)));
        } else {
            self.filled += 1;
            self.power = self.power.wrapping_mul(BASE);
        }
        self.window[self.pos] = byte;
        self.pos = (self.pos + 1) % self.window.len();
        self.hash
    }
    fn digest(&self) -> u32 {
        self.hash
    }
    fn reset(&mut self) {
        self.pos = 0;
        self.filled = 0;
        self.hash = 1;
        self.power = 1;
    }
}
