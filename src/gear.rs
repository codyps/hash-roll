#![cfg(feature = "gear")]

use crate::{Chunk, ChunkIncr, ToChunkIncr};
use std::fmt;
use std::num::Wrapping;

/// Gear Content Defined Chunking using 32bit expansion.
///
/// Reference:
///
///  Xia, W., Jiang, H., Feng, D., Tian, L., Fu, M., and Zhou, Y. Ddelta: A dedulication-inspired
///  fast delta compression approach. Performance Evaluation 79 (2014), 258-271.
///
///  http://wxia.hustbackup.cn/pub/DElta-PEVA-2014.pdf
#[derive(Clone)]
pub struct Gear32<'a> {
    /// A mask with an appropriate number of bits set for the desired average chunk size.
    ///
    /// fixed configuration.
    mask: u32,

    /// value to match (fp & mask) against.
    ///
    /// fixed configuration.
    xxx: u32,

    /// A table to map bytes to 32bit values
    ///
    /// fixed configuration.
    gear: &'a [u32; 256],
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct GearState32 {
    /// current fingerprint/hash
    ///
    /// varying state.
    fp: Wrapping<u32>,
}

#[derive(Debug, Clone)]
pub struct GearIncr32<'a> {
    params: Gear32<'a>,

    state: GearState32,
}

impl<'a> Chunk for Gear32<'a> {
    type SearchState = GearState32;

    fn to_search_state(&self) -> Self::SearchState {
        Default::default()
    }

    fn find_chunk_edge(
        &self,
        state: &mut Self::SearchState,
        data: &[u8],
    ) -> (Option<usize>, usize) {
        for (i, v) in data.iter().enumerate() {
            if state.push(self, *v) {
                *state = self.to_search_state();
                return (Some(i + 1), i + 1);
            }
        }

        (None, data.len())
    }
}

impl<'a> ToChunkIncr for Gear32<'a> {
    type Incr = GearIncr32<'a>;

    fn to_chunk_incr(&self) -> Self::Incr {
        Self::Incr {
            params: self.clone(),
            state: Default::default(),
        }
    }
}

impl<'a> fmt::Debug for Gear32<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Gear32")
            .field("mask", &self.mask)
            .field("xxx", &self.xxx)
            .field("gear", &&self.gear[..])
            .finish()
    }
}

impl GearState32 {
    fn push(&mut self, params: &Gear32<'_>, add: u8) -> bool {
        self.fp = (self.fp << 1) + Wrapping(params.gear[add as usize]);
        self.fp.0 & params.mask == params.xxx
    }

    fn reset(&mut self) {
        self.fp.0 = 0;
    }
}

impl<'a> ChunkIncr for GearIncr32<'a> {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        for (i, v) in data.iter().enumerate() {
            if self.state.push(&self.params, *v) {
                self.state.reset();
                return Some(i);
            }
        }

        None
    }
}

fn msb_mask(log2: usize) -> u32 {
    // at least 1 bit & not all the bits
    // FIXME: probably could relax those requirements with better math.
    //debug_assert!(log2 > 0);
    //debug_assert!(log2 < 32);

    ((1 << log2) - 1) << (32 - log2)
}

impl<'a> Gear32<'a> {
    /// Create a gear chunker which emits blocks with average size `(1<<average_size_log2)`, (or:
    /// `2**average_size_log2`
    pub fn with_average_size_log2(average_size_log2: usize) -> Self {
        Gear32 {
            mask: msb_mask(average_size_log2),
            xxx: 0,
            gear: &super::gear_table::GEAR_32,
        }
    }
}

impl<'a> Default for Gear32<'a> {
    fn default() -> Self {
        // 8KB average size
        Self::with_average_size_log2(13)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn mm() {
        use super::msb_mask;
        assert_eq!(0b1 << 31, msb_mask(1));
        assert_eq!(0b11 << 30, msb_mask(2));
        assert_eq!(0b111 << 29, msb_mask(3));
    }
}
