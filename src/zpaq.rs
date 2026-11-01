// Upstream attribution and license terms: see THIRD-PARTY-NOTICES.
#![cfg(feature = "zpaq")]

//! `zpaq` implements the chunking algorithm used in the zpaq archiving tool

use std::fmt;
use std::num::Wrapping;

use crate::{Chunk, ChunkIncr, ToChunkIncr};
use std::ops::{Bound, RangeBounds};

/// Zpaq content-defined chunking with an order-1 byte predictor.
///
/// Sizes and exponents in this API are in bytes. Zpaq's command-line
/// `-fragment` argument is a KiB exponent, so add 10 when translating it to
/// [`Self::with_average_size_pow_2`]. The default byte exponent is 16.
///
/// References:
/// - <https://github.com/zpaq/zpaq/blob/master/zpaq.cpp>
/// - <https://github.com/klauspost/dedup/blob/master/writer.go>
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zpaq {
    min_size: u64,
    max_size: Option<u64>,
    // A u64 allows a cutoff of 2^32, which matches every possible u32 hash.
    max_hash: u64,
}

impl Zpaq {
    fn bounds(range: impl RangeBounds<u64>) -> (u64, Option<u64>) {
        let min = match range.start_bound() {
            Bound::Included(&n) => n.max(1),
            Bound::Excluded(&n) => n.checked_add(1).expect("minimum chunk size overflows"),
            Bound::Unbounded => 1,
        };
        let max = match range.end_bound() {
            Bound::Included(&n) => Some(n),
            Bound::Excluded(&n) => Some(n.checked_sub(1).expect("empty chunk size range")),
            Bound::Unbounded => None,
        };
        assert!(
            max.is_none_or(|max| min <= max),
            "range must contain a positive chunk size"
        );
        (min, max)
    }

    fn cutoff_from_max(max: u64) -> u64 {
        assert!(
            (1..=1u64 << 34).contains(&max),
            "maximum must be in 1..=2^34 bytes"
        );
        // Natural cuts have probability 4/max, as in go-dedup. Compute the
        // threshold directly to preserve non-power-of-two targets as well.
        ((1u64 << 34) / max).min(1u64 << 32)
    }

    /// Configure the permitted chunk lengths using standard Rust range bounds.
    ///
    /// A finite upper bound sets the natural-cut target to one quarter of the
    /// largest permitted length. With no upper bound, the target is 16 times
    /// the minimum length, or 64 KiB when the minimum is one byte.
    ///
    /// The final incomplete chunk can be shorter than the minimum.
    ///
    /// # Panics
    /// Panics for a range containing no positive lengths, a finite maximum
    /// above 2^34 bytes, or an inferred target above 2^32 bytes. Use
    /// [`Self::with_average_and_range`] to configure the target separately.
    pub fn with_range(range: impl RangeBounds<u64>) -> Self {
        let (min_size, max_size) = Self::bounds(range);
        let max_hash = if let Some(max) = max_size {
            Self::cutoff_from_max(max)
        } else {
            let target = if min_size == 1 {
                1 << 16
            } else {
                min_size.checked_mul(16).expect("inferred target overflows")
            };
            assert!(
                target <= 1u64 << 32,
                "inferred target must not exceed 2^32 bytes"
            );
            (1u64 << 32) / target
        };
        Self {
            min_size,
            max_size,
            max_hash,
        }
    }

    /// Use Zpaq's size limits for an average length of `2^exponent` bytes.
    ///
    /// The minimum is `64 * 2^(exponent - 10)` bytes and the inclusive maximum
    /// is `8128 * 2^(exponent - 10)` bytes, matching Zpaq's fragment limits.
    ///
    /// # Panics
    /// Panics unless the byte exponent is in `10..=32`.
    pub fn with_average_size_pow_2(exponent: u8) -> Self {
        assert!(
            (10..=32).contains(&exponent),
            "byte exponent must be in 10..=32"
        );
        let fragment = exponent - 10;
        Self::with_average_and_range(exponent, (64u64 << fragment)..=(8128u64 << fragment))
    }

    /// Use go-dedup's natural-cut target of `max / 4` bytes (at least one byte).
    ///
    /// The inclusive maximum is `max`; the minimum is `max / 64`, rounded
    /// down but never below one byte. Non-power-of-two maxima are supported.
    ///
    /// # Panics
    /// Panics unless `max` is in `1..=2^34` bytes.
    pub fn with_max_size(max: u64) -> Self {
        let max_hash = Self::cutoff_from_max(max);
        Self {
            min_size: (max / 64).max(1),
            max_size: Some(max),
            max_hash,
        }
    }

    /// Set a natural-cut target of `2^exponent` bytes and permitted lengths.
    ///
    /// Range bounds have their standard Rust meaning. Natural cuts before
    /// the minimum are ignored; reaching a finite maximum always emits a cut.
    /// The final incomplete chunk can be shorter than the minimum.
    ///
    /// # Panics
    /// Panics for an exponent above 32 or a range with no positive lengths.
    pub fn with_average_and_range(exponent: u8, range: impl RangeBounds<u64>) -> Self {
        assert!(exponent <= 32, "byte exponent must be in 0..=32");
        let (min_size, max_size) = Self::bounds(range);
        Self {
            min_size,
            max_size,
            max_hash: 1u64 << (32 - exponent),
        }
    }

    fn split_here(&self, hash: u32, length: u64) -> bool {
        (u64::from(hash) < self.max_hash && length >= self.min_size)
            || self.max_size.is_some_and(|max| length >= max)
    }
}

impl Default for Zpaq {
    /**
     * Create a splitter using the defaults from Zpaq (the compressor)
     *
     * Average size is 65536 bytes (64KiB), max is 520192 bytes (508KiB), min is 4096 bytes (4KiB)
     */
    fn default() -> Self {
        Self::with_average_size_pow_2(16)
    }
}

/// Incremental instance of [`Zpaq`].
///
/// `Zpaq` doesn't require input look back, so the incremental and non-incremental performance
/// should be similar.
#[derive(Debug)]
pub struct ZpaqIncr {
    params: Zpaq,
    state: ZpaqHash,
    idx: u64,
}

/// Intermediate state from [`Chunk::find_chunk_edge`] for [`Zpaq`].
#[derive(Default, Debug)]
pub struct ZpaqSearchState {
    state: ZpaqHash,
    idx: u64,
}

impl ZpaqSearchState {
    fn feed(&mut self, v: u8) -> u32 {
        self.idx += 1;
        self.state.feed(v)
    }
}

impl Chunk for Zpaq {
    type SearchState = ZpaqSearchState;

    fn to_search_state(&self) -> Self::SearchState {
        Default::default()
    }

    fn find_chunk_edge(
        &self,
        state: &mut Self::SearchState,
        data: &[u8],
    ) -> (Option<usize>, usize) {
        if data.is_empty() {
            return (None, 0);
        }
        for (i, v) in data.iter().enumerate() {
            let h = state.feed(*v);
            if self.split_here(h, state.idx) {
                *state = self.to_search_state();
                return (Some(i + 1), i + 1);
            }
        }

        (None, data.len())
    }
}

impl From<&Zpaq> for ZpaqSearchState {
    fn from(_: &Zpaq) -> Self {
        Default::default()
    }
}

impl From<&Zpaq> for ZpaqIncr {
    fn from(s: &Zpaq) -> Self {
        s.clone().into()
    }
}

impl ToChunkIncr for Zpaq {
    type Incr = ZpaqIncr;
    fn to_chunk_incr(&self) -> Self::Incr {
        self.into()
    }
}

impl ZpaqIncr {
    fn feed(&mut self, v: u8) -> u32 {
        self.idx += 1;
        self.state.feed(v)
    }

    fn reset(&mut self) {
        self.idx = 0;
        self.state = Default::default();
    }
}

impl ChunkIncr for ZpaqIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        for (i, &v) in data.iter().enumerate() {
            let h = self.feed(v);
            if self.params.split_here(h, self.idx) {
                self.reset();
                return Some(i + 1);
            }
        }

        None
    }
}

impl From<Zpaq> for ZpaqIncr {
    fn from(params: Zpaq) -> Self {
        Self {
            params,
            state: Default::default(),
            idx: 0,
        }
    }
}

/**
 * The rolling hash component of the zpaq splitter
 */
#[derive(Clone)]
pub struct ZpaqHash {
    hash: Wrapping<u32>,
    last_byte: u8,
    predicted_byte: [u8; 256],
}

impl PartialEq for ZpaqHash {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
            && self.last_byte == other.last_byte
            && self.predicted_byte[..] == other.predicted_byte[..]
    }
}

impl Eq for ZpaqHash {}

impl fmt::Debug for ZpaqHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("ZpaqHash")
            .field("hash", &self.hash)
            .field("last_byte", &self.last_byte)
            .field("predicted_byte", &fmt_extra::Hs(&self.predicted_byte[..]))
            .finish()
    }
}

impl Default for ZpaqHash {
    fn default() -> Self {
        ZpaqHash {
            hash: Wrapping(0),
            last_byte: 0,
            predicted_byte: [0; 256],
        }
    }
}

impl ZpaqHash {
    /*
     * we can only get away with this because Zpaq doesn't need to look at old data to make it's
     * splitting decision, it only examines it's state + current value (and the state is
     * relatively large, but isn't a window into past data).
     */
    fn feed(&mut self, c: u8) -> u32 {
        self.hash = if c == self.predicted_byte[self.last_byte as usize] {
            (self.hash + Wrapping(c as u32) + Wrapping(1)) * Wrapping(314159265)
        } else {
            (self.hash + Wrapping(c as u32) + Wrapping(1)) * Wrapping(271828182)
        };

        self.predicted_byte[self.last_byte as usize] = c;
        self.last_byte = c;
        self.hash.0
    }
}
