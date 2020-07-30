//! zstd's `--rsyncable` option performs content defined chunking
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
//! emitted. In other words: the rolling hash isn't "reset" on block emittion. The current `Chunk`
//! API doesn't account for this (it assumes chunks are independent). `ChunkIncr` is able to
//! represent this (because it controls resetting of the state between chunks).
//!
use std::num::Wrapping;
use crate::{Chunk, ChunkIncr};

/*
*
*
 if (params.rsyncable) {
       /* Aim for the targetsectionSize as the average job size. */
       U32 const jobSizeMB = (U32)(mtctx->targetSectionSize >> 20);
       U32 const rsyncBits = ZSTD_highbit32(jobSizeMB) + 20;
       assert(jobSizeMB >= 1);
       DEBUGLOG(4, "rsyncLog = %u", rsyncBits);
       mtctx->rsync.hash = 0;
       mtctx->rsync.hitMask = (1ULL << rsyncBits) - 1;
       mtctx->rsync.primePower = ZSTD_rollingHash_primePower(RSYNC_LENGTH);
   }
*/

const RSYNC_LENGTH: usize = 32;
const PRIME_8_BYTES: Wrapping<u64> = Wrapping(0xCF1BBCDCB7A56463);
const ROLL_HASH_CHAR_OFFSET: Wrapping<u64> = Wrapping(10);

pub struct Zstd {}

#[derive(Default)]
struct ZstdState {
    hash: Wrapping<u64>,
}

impl ZstdState {
    fn append(&mut self, data: &[u8]) {
        for i in data {
            self.hash *= PRIME_8_BYTES;
            self.hash += Wrapping(*i as u64) + ROLL_HASH_CHAR_OFFSET;
        }
    }
}

#[derive(Default)]
pub struct ZstdSearchState {
    state: ZstdState,
    offset: usize,
}

/// Performance note: Zstd's chunking requires buffer look back to remove previously inserted data,
/// and as a result requires `ZstdIncr` to maintain an internal buffer. This internal buffer may
/// reduce performance.
#[derive(Default)]
pub struct ZstdIncr {
    state: ZstdState,

    window: Box<[u8]>,
    window_offs: usize,
}

/*
impl Chunk for Zstd {
    type SearchState = ZstdSearchState;
    type Incr = ZstdIncr;

    fn find_chunk_edge(
            &self,
            state: Option<Self::SearchState>,
            data: &[u8],
        ) -> Result<usize, Self::SearchState> {
        let mut hs = match state {
            Some(v) => v,
            None => Self::SearchState::default(),
        };

        for i in hs.offset..data.len() {
            hs.append(
            let h = hs.feed(data[i]);
            if self.split_here(h, (i + 1) as u64) {
                return Ok(i + 1);
            }
        }

        hs.offset = data.len();
        Err(hs)
    }

    fn incrimental(&self) -> Self::Incr {
        Default::default()
    }
}
*/

impl ChunkIncr for ZstdIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        todo!();
    }
}
