#![cfg(feature = "ultracdc")]
//! Experimental UltraCDC: popcount judgments and a repeated-window fast path.
//! Reference: <https://doi.org/10.1109/IPCCC55026.2022.9894295>.
//! Uses the cut rule of sleepybishop/ultracdc; see THIRD-PARTY-NOTICES.
//! Pair/block look-ahead requires retained slice input (`Chunk` only).
use crate::{Chunk, ChunkSizes};
#[derive(Debug, Clone)]
pub struct UltraCdc {
    sizes: ChunkSizes,
}
#[derive(Debug, Clone, Default)]
pub struct UltraCdcState {
    offset: usize,
    previous: Option<u64>,
    equal: usize,
}
fn word(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[..8].try_into().unwrap())
}
use std::convert::TryInto;
impl UltraCdc {
    /// Panics unless all sizes are multiples of 8 and max >= min + 8.
    pub fn new(sizes: ChunkSizes) -> Self {
        assert!(
            sizes.min().is_multiple_of(8)
                && sizes.average().is_multiple_of(8)
                && sizes.max().is_multiple_of(8)
        );
        assert!(sizes.max() - sizes.min() >= 8);
        Self { sizes }
    }
}
impl Default for UltraCdc {
    fn default() -> Self {
        Self::new(ChunkSizes::new(2048, 10240, 65536))
    }
}
impl Chunk for UltraCdc {
    type SearchState = UltraCdcState;
    fn to_search_state(&self) -> Self::SearchState {
        Self::SearchState {
            offset: self.sizes.min() + 8,
            ..Default::default()
        }
    }
    fn find_chunk_edge(&self, s: &mut Self::SearchState, data: &[u8]) -> (Option<usize>, usize) {
        let end = data.len().min(self.sizes.max());
        if end < s.offset {
            return (None, 0);
        }
        if s.previous.is_none() {
            s.previous = Some(word(&data[self.sizes.min()..]));
        }
        while end - s.offset >= 8 {
            let i = s.offset;
            let next = word(&data[i..]);
            let mut cut = None;
            if s.previous == Some(next) {
                s.equal += 1;
                if s.equal == 64 {
                    cut = Some(i + 8)
                }
            } else {
                s.equal = 0;
                let mask = if i < self.sizes.average() { 0x2f } else { 0x2c };
                for j in 0..8 {
                    if (word(&data[i + j - 8..]) ^ 0xaaaaaaaaaaaaaaaa).count_ones() & mask == 0 {
                        cut = Some(i + j);
                        break;
                    }
                }
            }
            if let Some(n) = cut {
                *s = self.to_search_state();
                return (Some(n), n);
            }
            s.previous = Some(next);
            s.offset += 8;
        }
        if end == self.sizes.max() {
            *s = self.to_search_state();
            (Some(end), end)
        } else {
            (None, 0)
        }
    }
}
