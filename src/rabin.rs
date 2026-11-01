#![cfg(feature = "rabin")]
//! Sliding polynomial fingerprints over GF(2), with a restic-compatible preset.
//! Reference: <https://github.com/restic/chunker>.
use crate::{ChunkIncr, ChunkSizes, RollingHash};
use std::sync::Arc;

/// A degree-53 irreducible polynomial used in restic's published test vectors.
pub const DEFAULT_POLYNOMIAL: u64 = 0x3da3358b4dc173;

fn remainder(mut n: u128, polynomial: u64) -> u64 {
    let degree = 63 - polynomial.leading_zeros();
    while n != 0 && 127 - n.leading_zeros() >= degree {
        n ^= u128::from(polynomial) << (127 - n.leading_zeros() - degree);
    }
    n as u64
}
#[derive(Debug)]
struct Tables {
    polynomial: u64,
    degree: u32,
    reduce: [u64; 256],
    out: [u64; 256],
}
impl Tables {
    fn new(polynomial: u64, window: usize) -> Self {
        assert!(
            polynomial > 0 && polynomial & 1 == 1,
            "polynomial must be odd and nonzero"
        );
        let degree = 63 - polynomial.leading_zeros();
        assert!(
            polynomial & 1 == 1 && (8..=56).contains(&degree),
            "expected an odd polynomial of degree 8..=56"
        );
        let mut t = Self {
            polynomial,
            degree,
            reduce: [0; 256],
            out: [0; 256],
        };
        for byte in 0..256 {
            let top = (byte as u128) << degree;
            t.reduce[byte] = top as u64 ^ remainder(top, polynomial);
            let mut h = byte as u64;
            for _ in 1..window {
                h = remainder(u128::from(h) << 8, polynomial);
            }
            t.out[byte] = h;
        }
        t
    }
}
/// Rolling GF(2) fingerprint; arbitrary odd polynomials are accepted.
/// Choose an irreducible polynomial when collision properties matter.
#[derive(Debug, Clone)]
pub struct RabinFingerprint {
    tables: Arc<Tables>,
    window: Box<[u8]>,
    pos: usize,
    hash: u64,
}
impl RabinFingerprint {
    /// Panics for an empty window or an even/unsupported polynomial.
    pub fn new(polynomial: u64, window: usize) -> Self {
        assert!(window > 0, "window must be positive");
        Self {
            tables: Arc::new(Tables::new(polynomial, window)),
            window: vec![0; window].into_boxed_slice(),
            pos: 0,
            hash: 0,
        }
    }
    pub fn polynomial(&self) -> u64 {
        self.tables.polynomial
    }
}
impl RollingHash for RabinFingerprint {
    type Digest = u64;
    fn roll_byte(&mut self, byte: u8) -> u64 {
        self.hash ^= self.tables.out[self.window[self.pos] as usize];
        self.window[self.pos] = byte;
        self.pos = (self.pos + 1) % self.window.len();
        let index = (self.hash >> (self.tables.degree - 8)) as usize;
        self.hash = ((self.hash << 8) | u64::from(byte)) ^ self.tables.reduce[index];
        self.hash
    }
    fn digest(&self) -> u64 {
        self.hash
    }
    fn reset(&mut self) {
        self.hash = 0;
        self.pos = 0;
        self.window.fill(0);
    }
}
/// Rabin CDC with minimum skipping, a low-bit mask, and an inclusive maximum.
#[derive(Debug, Clone)]
pub struct Rabin {
    hash: RabinFingerprint,
    sizes: ChunkSizes,
    mask: u64,
}
impl Rabin {
    /// Panics unless the minimum is at least the window and bits is in 1..=56.
    pub fn new(polynomial: u64, window: usize, sizes: ChunkSizes, bits: u8) -> Self {
        assert!(sizes.min() >= window && (1..=56).contains(&bits));
        Self {
            hash: RabinFingerprint::new(polynomial, window),
            sizes,
            mask: (1u64 << bits) - 1,
        }
    }
    /// Restic's 64-byte window, 512 KiB minimum, 1 MiB target and 8 MiB maximum.
    /// Supply the polynomial from the repository whose boundaries must match.
    pub fn restic(polynomial: u64) -> Self {
        Self::new(
            polynomial,
            64,
            ChunkSizes::new(1 << 19, 1 << 20, 1 << 23),
            20,
        )
    }
    fn state(&self) -> RabinIncr {
        RabinIncr {
            params: self.clone(),
            hash: self.hash.clone(),
            len: 0,
        }
    }
}
impl Default for Rabin {
    fn default() -> Self {
        Self::new(DEFAULT_POLYNOMIAL, 64, ChunkSizes::default(), 13)
    }
}
#[derive(Debug, Clone)]
pub struct RabinIncr {
    params: Rabin,
    hash: RabinFingerprint,
    len: usize,
}
impl ChunkIncr for RabinIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        for (i, &byte) in data.iter().enumerate() {
            self.len += 1;
            if self.len > self.params.sizes.min() - self.hash.window.len() {
                self.hash.roll_byte(byte);
            }
            if self.len >= self.params.sizes.min()
                && (self.hash.digest() & self.params.mask == 0
                    || self.len == self.params.sizes.max())
            {
                self.hash.reset();
                self.len = 0;
                return Some(i + 1);
            }
        }
        None
    }
}
impl_stream_chunker!(Rabin, RabinIncr);
