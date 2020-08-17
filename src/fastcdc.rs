#![cfg(feature = "fastcdc")]

//! FastCDC is a chunking algorithm using some features from [Gear](super::gear)
//!
//! Reference:
//!  - https://www.usenix.org/system/files/conference/atc16/atc16-paper-xia.pdf

use crate::{Chunk, ChunkIncr, ToChunkIncr};
use std::fmt;
use std::num::Wrapping;

// these masks are taken from the paper and could be adjusted/adjustable.
const MASK_S: u64 = 0x0003590703530000;
//const MASK_A: u64 = 0x0000d90303530000;
const MASK_L: u64 = 0x0000d90003530000;

/// An instance of the "FastCDC" algorithm
///
/// Default parameters:
///  - Minimum chunk size: 2 KiB
///  - Maximum chunk size: 64 KiB
///  - Normal size: 8 KiB
///  - internal 64-bit gear table: [`super::gear_table::GEAR_64`]
///
#[derive(Clone, Copy)]
pub struct FastCdc<'a> {
    gear: &'a [u64; 256],
    min_size: u64,
    max_size: u64,
    normal_size: u64,
}

impl<'a> PartialEq for FastCdc<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.min_size == other.min_size
            && self.max_size == other.max_size
            && self.normal_size == other.normal_size
            && &self.gear[..] == &other.gear[..]
    }
}

impl<'a> Eq for FastCdc<'a> {}

impl<'a> Default for FastCdc<'a> {
    fn default() -> Self {
        FastCdc {
            min_size: 2 * 1024,    // 2 KiB
            max_size: 64 * 1024,   // 64 KiB
            normal_size: 8 * 1024, // 8 KiB
            gear: &super::gear_table::GEAR_64,
        }
    }
}

impl<'a> fmt::Debug for FastCdc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FastCdc")
            .field("gear", &"[...]")
            .field("min_size", &self.min_size)
            .field("max_size", &self.max_size)
            .field("normal_size", &self.normal_size)
            .finish()
    }
}

impl<'a> Chunk for FastCdc<'a> {
    type SearchState = FastCdcState;

    fn to_search_state(&self) -> Self::SearchState {
        Default::default()
    }

    fn find_chunk_edge(
        &self,
        state: &mut Self::SearchState,
        data: &[u8],
    ) -> (Option<usize>, usize) {
        match state.push(self, data) {
            Some(i) => (Some(i + 1), i + 1),
            None => (None, data.len()),
        }
    }
}

impl<'a> FastCdc<'a> {
    /// Create a custom FastCDC instance
    pub fn new(gear: &'a [u64; 256], min_size: u64, normal_size: u64, max_size: u64) -> Self {
        Self {
            gear,
            min_size,
            max_size,
            normal_size,
        }
    }
}

impl<'a> ToChunkIncr for FastCdc<'a> {
    type Incr = FastCdcIncr<'a>;

    fn to_chunk_incr(&self) -> Self::Incr {
        self.into()
    }
}

impl<'a> From<&FastCdc<'a>> for FastCdcIncr<'a> {
    fn from(params: &FastCdc<'a>) -> Self {
        Self {
            params: params.clone(),
            state: Default::default(),
        }
    }
}

/// FastCdcIncr provides an incrimental interface to `FastCdc`
///
/// This impl does not buffer data passing through it (the FastCDC algorithm does not require
/// look-back) making it very efficient.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FastCdcIncr<'a> {
    params: FastCdc<'a>,
    state: FastCdcState,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FastCdcState {
    /// Number of bytes we've "examined"
    ///
    /// varying state.
    l: u64,

    /// Current fingerprint
    ///
    /// varying state.
    fp: Wrapping<u64>,
}

impl FastCdcState {
    fn reset(&mut self) {
        self.l = 0;
        self.fp = Wrapping(0);
    }

    fn push(&mut self, params: &FastCdc<'_>, data: &[u8]) -> Option<usize> {
        // global start/index
        let mut gi = self.l;
        // global end
        let ge = data.len() as u64 + gi;

        if ge <= params.min_size {
            // No split, no processing of data, but we've "consumed" the bytes.
            self.l = ge;
            return None;
        }

        // skip elements prior to MIN_SIZE and track offset of new `data` in argument `data` for
        // return value
        let mut i = if gi <= params.min_size {
            let skip = params.min_size - gi;
            gi += skip;
            skip
        } else {
            0
        } as usize;

        let mut fp = self.fp;

        loop {
            if i >= data.len() {
                break;
            }
            if gi >= params.normal_size {
                // go to next set of matches
                break;
            }

            let v = data[i];
            fp = (fp << 1) + Wrapping(params.gear[v as usize]);
            if (fp.0 & MASK_S) == 0 {
                self.reset();
                return Some(i);
            }

            gi += 1;
            i += 1;
        }

        loop {
            if gi >= params.max_size {
                // no match found, emit fixed match at MAX_SIZE
                self.reset();
                return Some(i);
            }
            if i >= data.len() {
                break;
            }

            let v = data[i];
            fp = (fp << 1) + Wrapping(params.gear[v as usize]);
            if (fp.0 & MASK_L) == 0 {
                self.reset();
                return Some(i);
            }

            gi += 1;
            i += 1;
        }

        // no match, but not at MAX_SIZE yet, so store context for next time.
        self.fp = fp;
        self.l = ge;

        None
    }
}

impl<'a> ChunkIncr for FastCdcIncr<'a> {
    fn push(&mut self, src: &[u8]) -> Option<usize> {
        self.state.push(&self.params, src)
    }
}
