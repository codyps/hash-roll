#![cfg(feature = "supercdc")]
//! Experimental SuperCDC-style hybrid chunking: Gear judgments, a fallback
//! boundary, and bounded stream-history prediction. Slice input is retained
//! until a cut is known; state carries history between chunks of one source.
//!
//! This configurable implementation is not a byte-compatible format preset.
//! Reference: <https://doi.org/10.1109/ICCD56317.2022.00034>.
use crate::fastcdc2020::TABLE;
use crate::{Chunk, ChunkSizes};
use std::collections::{HashMap, VecDeque};
#[derive(Debug, Clone)]
pub struct SuperCdc {
    sizes: ChunkSizes,
    capacity: usize,
    strict: u64,
    relaxed: u64,
    backup: u64,
}
#[derive(Debug, Clone, Default)]
pub struct SuperCdcState {
    offset: usize,
    hash: u64,
    backup: Option<(usize, u64)>,
    last: Option<u64>,
    pending: Option<usize>,
    history: HashMap<u64, usize>,
    order: VecDeque<u64>,
}
impl SuperCdc {
    /// Panics unless the target is a power of two >=64. A zero capacity
    /// disables prediction while preserving the underlying boundary policy.
    pub fn new(sizes: ChunkSizes, history_capacity: usize) -> Self {
        assert!(sizes.average().is_power_of_two() && sizes.average() >= 64);
        let bits = sizes.average().ilog2();
        assert!(bits <= 30);
        Self {
            sizes,
            capacity: history_capacity,
            strict: (1u64 << (bits + 1)) - 1,
            relaxed: (1u64 << (bits - 1)) - 1,
            backup: (1u64 << (bits - 2)) - 1,
        }
    }
    fn emit(&self, s: &mut SuperCdcState, n: usize, key: u64) -> (Option<usize>, usize) {
        if let Some(previous) = s.last.take() {
            if self.capacity > 0 {
                if !s.history.contains_key(&previous) {
                    if s.history.len() == self.capacity {
                        if let Some(old) = s.order.pop_front() {
                            s.history.remove(&old);
                        }
                    }
                    s.order.push_back(previous);
                }
                s.history.insert(previous, n);
            }
        }
        // Only an available, validated byte length can be predicted. Pending
        // predictions are retained across input-buffer extensions.
        s.pending = s.history.get(&key).copied();
        s.last = Some(key);
        s.offset = 0;
        s.hash = 0;
        s.backup = None;
        (Some(n), n)
    }
}
impl Default for SuperCdc {
    fn default() -> Self {
        Self::new(ChunkSizes::new(4096, 8192, 65536), 4096)
    }
}
impl Chunk for SuperCdc {
    type SearchState = SuperCdcState;
    fn to_search_state(&self) -> Self::SearchState {
        Default::default()
    }
    fn find_chunk_edge(&self, s: &mut Self::SearchState, data: &[u8]) -> (Option<usize>, usize) {
        if data.is_empty() {
            return (None, 0);
        }
        if let Some(n) = s.pending {
            if data.len() < n {
                return (None, 0);
            }
            s.pending = None;
            // Prediction covers this chunk only, rather than recursively
            // learning from a boundary that was never fingerprinted.
            s.last = None;
            s.offset = 0;
            s.hash = 0;
            s.backup = None;
            return (Some(n), n);
        }
        let end = data.len().min(self.sizes.max());
        while s.offset < end {
            let b = data[s.offset];
            s.offset += 1;
            s.hash = (s.hash << 1).wrapping_add(TABLE[b as usize]);
            if s.offset >= self.sizes.min() {
                let mask = if s.offset < self.sizes.average() {
                    self.strict
                } else {
                    self.relaxed
                };
                if s.hash & mask == 0 {
                    return self.emit(s, s.offset, s.hash);
                }
                if s.offset >= self.sizes.average()
                    && s.backup.is_none()
                    && s.hash & self.backup == 0
                {
                    s.backup = Some((s.offset, s.hash));
                }
            }
        }
        if end == self.sizes.max() {
            let (n, h) = s.backup.unwrap_or((end, s.hash));
            self.emit(s, n, h)
        } else {
            (None, 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_input_trains_real_boundary_predictions() {
        let c = SuperCdc::new(ChunkSizes::new(16, 64, 256), 4);
        let mut s = c.to_search_state();
        let data = [0u8; 256];
        for _ in 0..2 {
            assert_eq!(c.find_chunk_edge(&mut s, &data), (Some(256), 256));
        }
        assert_eq!(s.pending, Some(256));
        assert_eq!(c.find_chunk_edge(&mut s, &data[..255]), (None, 0));
        assert_eq!(c.find_chunk_edge(&mut s, &data), (Some(256), 256));
        assert_eq!(s.offset, 0);
        assert_eq!(s.hash, 0);
    }
    #[test]
    fn history_is_bounded_and_predictions_wait_for_full_input() {
        let c = SuperCdc::new(ChunkSizes::new(16, 64, 256), 2);
        let mut s = c.to_search_state();
        c.emit(&mut s, 32, 1);
        c.emit(&mut s, 48, 2);
        c.emit(&mut s, 64, 3);
        c.emit(&mut s, 80, 4);
        assert_eq!(s.history.len(), 2);
        assert_eq!(s.order.len(), 2);
        assert!(!s.history.contains_key(&1));
        c.emit(&mut s, 32, 3);
        assert_eq!(s.pending, Some(80));
        assert_eq!(c.find_chunk_edge(&mut s, &[0; 79]), (None, 0));
        assert_eq!(c.find_chunk_edge(&mut s, &[0; 80]), (Some(80), 80));
        assert!(s.last.is_none() && s.pending.is_none());
        let fresh = c.to_search_state();
        assert!(fresh.history.is_empty() && fresh.pending.is_none());
    }
}
