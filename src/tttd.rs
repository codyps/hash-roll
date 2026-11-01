#![cfg(feature = "tttd")]
//! Two Thresholds, Two Divisors (TTTD) over Rabin fingerprints.
//!
//! Prefer the primary divisor; remember the most recent secondary match. At
//! the maximum choose that fallback, or force the maximum if none exists.
//! Returning a previously examined fallback requires retained input (`Chunk`
//! only); it cannot satisfy the current-buffer-only `ChunkIncr` contract.
//! Reference: <https://shiftleft.com/mirrors/www.hpl.hp.com/techreports/2005/HPL-2005-30R1.pdf>.
use crate::rabin::{RabinFingerprint, DEFAULT_POLYNOMIAL};
use crate::{Chunk, ChunkSizes, RollingHash};
#[derive(Debug, Clone)]
pub struct Tttd {
    hash: RabinFingerprint,
    sizes: ChunkSizes,
    primary: u64,
    secondary: u64,
}
#[derive(Debug, Clone)]
pub struct TttdState {
    hash: RabinFingerprint,
    offset: usize,
    fallback: Option<usize>,
}
impl Tttd {
    /// Panics unless min >= window and 0 < secondary_bits < primary_bits <= 56.
    pub fn new(
        polynomial: u64,
        window: usize,
        sizes: ChunkSizes,
        primary_bits: u8,
        secondary_bits: u8,
    ) -> Self {
        assert!(
            sizes.min() >= window
                && secondary_bits > 0
                && secondary_bits < primary_bits
                && primary_bits <= 56
        );
        Self {
            hash: RabinFingerprint::new(polynomial, window),
            sizes,
            primary: (1u64 << primary_bits) - 1,
            secondary: (1u64 << secondary_bits) - 1,
        }
    }
}
impl Default for Tttd {
    fn default() -> Self {
        Self::new(DEFAULT_POLYNOMIAL, 64, ChunkSizes::default(), 13, 11)
    }
}
impl Chunk for Tttd {
    type SearchState = TttdState;
    fn to_search_state(&self) -> Self::SearchState {
        TttdState {
            hash: self.hash.clone(),
            offset: 0,
            fallback: None,
        }
    }
    fn find_chunk_edge(&self, s: &mut Self::SearchState, data: &[u8]) -> (Option<usize>, usize) {
        let end = data.len().min(self.sizes.max());
        while s.offset < end {
            let hash = s.hash.roll_byte(data[s.offset]);
            s.offset += 1;
            if s.offset >= self.sizes.min() {
                if hash & self.primary == 0 {
                    let n = s.offset;
                    *s = self.to_search_state();
                    return (Some(n), n);
                }
                if hash & self.secondary == 0 {
                    s.fallback = Some(s.offset)
                }
            }
        }
        if end == self.sizes.max() {
            let n = s.fallback.unwrap_or(end);
            *s = self.to_search_state();
            (Some(n), n)
        } else {
            (None, 0)
        }
    }
}
