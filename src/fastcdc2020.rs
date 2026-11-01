#![cfg(feature = "fastcdc2020")]
//! The versioned FastCDC 2020 two-byte cut rule.
//!
//! Uses the reference MD5-generated table and spread-bit masks. Unlike the
//! older `fastcdc` module, the reference cut is *before* a tested byte. Its
//! pair look-ahead requires retained slice input, so this type implements
//! `Chunk` only. Use the existing FastCdc for the current-buffer push API.
//! Reference: <https://doi.org/10.1109/TPDS.2020.2984632>.
use crate::{Chunk, ChunkSizes};
#[path = "fastcdc2020_table.rs"]
mod table;
pub(crate) use table::TABLE;
const MASKS: [u64; 26] = [
    0,                  // padding
    0,                  // padding
    0,                  // padding
    0,                  // padding
    0,                  // padding
    0x0000000001804110, // unused except for NC 3
    0x0000000001803110, // 64B
    0x0000000018035100, // 128B
    0x0000001800035300, // 256B
    0x0000019000353000, // 512B
    0x0000590003530000, // 1KB
    0x0000d90003530000, // 2KB
    0x0000d90103530000, // 4KB
    0x0000d90303530000, // 8KB
    0x0000d90313530000, // 16KB
    0x0000d90f03530000, // 32KB
    0x0000d90303537000, // 64KB
    0x0000d90703537000, // 128KB
    0x0000d90707537000, // 256KB
    0x0000d91707537000, // 512KB
    0x0000d91747537000, // 1MB
    0x0000d91767537000, // 2MB
    0x0000d93767537000, // 4MB
    0x0000d93777537000, // 8MB
    0x0000d93777577000, // 16MB
    0x0000db3777577000, // unused except for NC 3
];

#[derive(Debug, Clone)]
pub struct FastCdc2020 {
    pub(crate) sizes: ChunkSizes,
    pub(crate) strict: u64,
    pub(crate) relaxed: u64,
}
#[derive(Debug, Clone, Default)]
pub struct FastCdc2020State {
    offset: usize,
    hash: u64,
}
impl FastCdc2020 {
    /// Panics unless sizes are even, target is a power of two, and the target
    /// exponent plus/minus normalization lies in the mask-table range 5..=25.
    pub fn new(sizes: ChunkSizes, normalization: u8) -> Self {
        assert!(
            sizes.min().is_multiple_of(2)
                && sizes.average().is_multiple_of(2)
                && sizes.max().is_multiple_of(2)
        );
        assert!(sizes.average().is_power_of_two());
        let bits = sizes.average().ilog2() as usize;
        let n = usize::from(normalization);
        assert!(bits >= n + 5 && bits + n <= 25);
        Self {
            sizes,
            strict: MASKS[bits + n],
            relaxed: MASKS[bits - n],
        }
    }
}
impl Default for FastCdc2020 {
    fn default() -> Self {
        Self::new(ChunkSizes::default(), 1)
    }
}
impl Chunk for FastCdc2020 {
    type SearchState = FastCdc2020State;
    fn to_search_state(&self) -> Self::SearchState {
        Self::SearchState {
            offset: self.sizes.min(),
            hash: 0,
        }
    }
    fn find_chunk_edge(&self, s: &mut Self::SearchState, data: &[u8]) -> (Option<usize>, usize) {
        let end = data.len().min(self.sizes.max());
        while s.offset < end && end - s.offset >= 2 {
            let i = s.offset;
            let mask = if i < self.sizes.average() {
                self.strict
            } else {
                self.relaxed
            };
            s.hash = (s.hash << 2).wrapping_add(TABLE[data[i] as usize].wrapping_shl(1));
            let cut = if s.hash & (mask << 1) == 0 {
                Some(i)
            } else {
                s.hash = s.hash.wrapping_add(TABLE[data[i + 1] as usize]);
                (s.hash & mask == 0).then_some(i + 1)
            };
            if let Some(n) = cut {
                *s = self.to_search_state();
                return (Some(n), n);
            }
            s.offset += 2;
        }
        if end == self.sizes.max() {
            let n = end;
            *s = self.to_search_state();
            (Some(n), n)
        } else {
            (None, 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_generated_table_entries_match_reference() {
        let (reference, _) = ::fastcdc::v2020::get_gear_with_seed(0);
        assert_eq!(TABLE.as_slice(), reference.as_ref());
    }
}
