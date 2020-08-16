#![cfg(feature = "zstd")]

//! zstd's `--rsyncable` option performs content defined chunking
//!
//! This has been minimally validated to match the implimentation from zstd, with the following
//! caveats:
//!
//!   - Maximum chunk size is not implimented
//!   - Only 1 test case with a single chunk edge (ie: 2 chunks) has been tested
//!
//! It uses a internal [rolling
//! hash](https://github.com/facebook/zstd/blob/01261bc8b6fcfc77801788f8b1e2a2e5dd2e8e25/lib/compress/zstd_compress_internal.h#L658-L698)
//! with 1 multiple and 2 additions. (see `ZSTD_rollingHash_append()` for core functionality).
//!
//! The rolling hash is then used by
//! [`findSynchronizationPoint()`](https://github.com/facebook/zstd/blob/15c5e200235edc520c1bd678ed126a6dd05736e1/lib/compress/zstdmt_compress.c#L1931-L2001)
//! in various ways to find "syncronization points" (ie: edges of chunks).
//!
//! [This issue thread comment ](https://github.com/facebook/zstd/issues/1155#issuecomment-520258862) also
//! includes some explanation on the mechanism.
//!
//! The zstd code _does_ include in it's context information about _previous_ block that was
//! emitted. In other words: the rolling hash isn't "reset" on block emittion. (Most chunking
//! algorithms are reset on block emittion).
use crate::{Chunk, ChunkIncr, ToChunkIncr};
use std::convert::TryInto;
use std::num::Wrapping;

const RSYNC_LENGTH: usize = 32;
const PRIME_8_BYTES: Wrapping<u64> = Wrapping(0xCF1BBCDCB7A56463);
const ROLL_HASH_CHAR_OFFSET: Wrapping<u64> = Wrapping(10);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Zstd {
    hit_mask: u64,
    prime_power: u64,
}

impl Default for Zstd {
    fn default() -> Self {
        // ../lib/compress/zstdmt_compress.c: jobSizeMB: 8, rsyncBits: 23, hitMask: 7fffff, primePower: f5507fe35f91f8cb
        Self::with_target_section_size(8 << 20)
    }
}

impl Zstd {
    /*
     * ```notrust
        /* Aim for the targetsectionSize as the average job size. */
        U32 const jobSizeMB = (U32)(mtctx->targetSectionSize >> 20);
        U32 const rsyncBits = ZSTD_highbit32(jobSizeMB) + 20;
        assert(jobSizeMB >= 1);
        DEBUGLOG(4, "rsyncLog = %u", rsyncBits);
        mtctx->rsync.hash = 0;
        mtctx->rsync.hitMask = (1ULL << rsyncBits) - 1;
        mtctx->rsync.primePower = ZSTD_rollingHash_primePower(RSYNC_LENGTH);
        ```
    */
    pub fn with_target_section_size(target_section_size: u64) -> Self {
        let job_size_mb: u32 = (target_section_size >> 20).try_into().unwrap();
        assert_ne!(job_size_mb, 0);
        let rsync_bits = (job_size_mb.leading_zeros() ^ 31) + 20;
        let hit_mask = (1u64 << rsync_bits) - 1;
        let prime_power = PRIME_8_BYTES
            .0
            .wrapping_pow((RSYNC_LENGTH - 1).try_into().unwrap());
        Self {
            hit_mask,
            prime_power,
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_zstd_init_matches_upstream() {
        let zstd = super::Zstd::default();
        assert_eq!(zstd.hit_mask, 0x7f_ffff);
        assert_eq!(zstd.prime_power, 0xf5507fe35f91f8cb);
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
struct ZstdState {
    hash: Wrapping<u64>,
}

impl ZstdState {
    // `ZSTD_rollingHash_append()`
    fn append(&mut self, data: &[u8]) {
        for i in data {
            self.hash *= PRIME_8_BYTES;
            self.hash += Wrapping(*i as u64) + ROLL_HASH_CHAR_OFFSET;
        }
    }

    // `ZSTD_rollingHash_rotate()`
    fn rotate(&mut self, to_remove: u8, to_add: u8, prime_power: u64) {
        self.hash -= (Wrapping(to_remove as u64) + ROLL_HASH_CHAR_OFFSET) * Wrapping(prime_power);
        self.hash *= PRIME_8_BYTES;
        self.hash += Wrapping(to_add as u64) + ROLL_HASH_CHAR_OFFSET;
    }

    fn at_split(&mut self, params: &Zstd) -> bool {
        (self.hash.0 & params.hit_mask) == params.hit_mask
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct ZstdSearchState {
    state: ZstdState,
    offset: usize,
}

impl ZstdSearchState {
    fn append(&mut self, data: &[u8]) {
        self.state.append(data);
    }

    fn rotate(&mut self, to_remove: u8, to_add: u8, prime_power: u64) {
        self.state.rotate(to_remove, to_add, prime_power);
    }

    fn at_split(&mut self, params: &Zstd) -> bool {
        self.state.at_split(params)
    }
}

/// Incrimental chunking using Zstd's rsyncable algorithm
///
/// Performance note: Zstd's chunking requires buffer look back to remove previously inserted data,
/// and as a result requires `ZstdIncr` to maintain an internal buffer. This internal buffer may
/// reduce performance.
#[derive(Debug, PartialEq, Eq)]
pub struct ZstdIncr {
    params: Zstd,

    state: ZstdState,

    window: Box<[u8]>,
    // insert into the window at this offset
    window_offs: usize,
    // if true, we need to remove bytes from the window when inserting
    //
    // NOTE: by pre-filling `self.hash` with an appropriate value, we might be able to remove this
    // variable and always treat the window as full (of zeros initially).
    window_full: bool,

    // how many byte since last emitted block
    // used to cap the block size as zstd does
    input_offs: u64,
}

impl ToChunkIncr for Zstd {
    type Incr = ZstdIncr;

    fn to_chunk_incr(&self) -> Self::Incr {
        self.into()
    }
}

impl From<Zstd> for ZstdIncr {
    fn from(params: Zstd) -> Self {
        Self {
            params,
            state: Default::default(),
            window: vec![0; RSYNC_LENGTH].into_boxed_slice(),
            window_offs: 0,
            window_full: false,
            input_offs: 0,
        }
    }
}

impl From<&Zstd> for ZstdIncr {
    fn from(params: &Zstd) -> Self {
        params.clone().into()
    }
}

impl Chunk for Zstd {
    type SearchState = ZstdSearchState;

    fn to_search_state(&self) -> Self::SearchState {
        Self::SearchState::default()
    }

    fn find_chunk_edge(
        &self,
        state: &mut Self::SearchState,
        data: &[u8],
    ) -> (Option<usize>, usize) {
        if state.offset < RSYNC_LENGTH {
            // push some data in
            let seed_b = &data[state.offset..std::cmp::min(RSYNC_LENGTH, data.len())];
            state.append(seed_b);
            state.offset += seed_b.len();

            if state.offset < RSYNC_LENGTH {
                // not enough data
                return (None, 0);
            }
        }

        // TODO: track input_offs to split over-size blocks

        // we've got enough data, do rotations
        for i in state.offset..data.len() {
            let to_remove = data[i - RSYNC_LENGTH];
            let to_add = data[i];
            state.rotate(to_remove, to_add, self.prime_power);
            if state.at_split(self) {
                let discard_ct = data.len().saturating_sub(RSYNC_LENGTH);
                return (Some(i + 1), discard_ct);
            }
        }

        let discard_ct = data.len().saturating_sub(RSYNC_LENGTH);
        let keep_ct = data.len() - discard_ct;
        state.offset = keep_ct;
        (None, discard_ct)
    }
}

impl ChunkIncr for ZstdIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        let use_len = if !self.window_full {
            let use_len = std::cmp::min(self.window.len() - self.window_offs, data.len());
            self.window[self.window_offs..(self.window_offs + use_len)]
                .copy_from_slice(&data[..use_len]);
            self.window_offs += use_len;

            if self.window_offs != self.window.len() {
                return None;
            }

            self.window_full = true;
            self.window_offs = 0;
            self.state.append(&self.window[..]);
            use_len
        } else {
            0
        };

        // TODO: track input_offs to split over-size blocks

        // we have a full window, now rotate data through
        for (i, &v) in data[use_len..].iter().enumerate() {
            let to_remove = self.window[self.window_offs];
            let to_add = v;
            self.state
                .rotate(to_remove, to_add, self.params.prime_power);
            self.window[self.window_offs] = to_add;
            self.window_offs = (self.window_offs + 1) % self.window.len();

            if self.state.at_split(&self.params) {
                // NOTE: don't clear window
                return Some(i + use_len);
            }
        }

        None
    }
}
