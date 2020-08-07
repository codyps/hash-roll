use crate::{Chunk, ChunkIncr, Splitter, ToChunkIncr};
use std::collections::VecDeque;
use std::num::Wrapping;

/// Window-based splitter using a simple accumulator & modulus hash.
///
/// Used by the gzip rsyncable patch (still not merged, but widely distributed) as
/// well as the rsyncrypto project to split the unerlying content into variable sized blocks prior
/// to applying a filter (compression and/or encryption) to the blocks, which the intent of allowing
/// the resulting filtered data to be more easily propogated via rsync.
///
///  - No maximum block size is provided.
///  - No minimum block size is provided.
///
/// PDF of block sizes: ???
///
/// Note that the defacto-standard parameters allow a slightly more efficient check for a block
/// split (by replacing a modulus with a bitwise-and). This impl currently doesn't allow that
/// optimization even if you provide appropriate parameters (we need type-level integers for that).
///
/// Parameters:
///
///  - window-len: The maximum number of bytes to be examined when deciding to split a block.
///              set to 8192 by default in gzip-rsyncable & rsyncrypto)
///  - modulus:    set to half of window-len (so, 4096) in gzip-rsyncable & rsyncrypto.
///
/// In-block state:
///  - window of window-len bytes (use of the iterator interface means we also track more bytes than
///      this)
///  - sum (u64)
///
/// Between-block state:
///
/// - none
///
/// References:
///
/// - http://rsyncrypto.lingnu.com/index.php/Algorithm
/// - https://www.samba.org/~tridge/phd_thesis.pdf
///
/// S(n) = sum(c_i, var=i, top=n, bottom=n-8196)
///
/// A(n) = S(n) / 8192
///
/// H(n) = S(n) mod 4096
///
/// Trigger splits when H(n) == 0
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GzipRsyncable {
    /*
     * TODO: if we can avoid loading entire files into memory, this could be u64
     */
    window_len: usize,
    modulus: u64,
}

impl GzipRsyncable {
    pub fn with_window_and_modulus(window: usize, modulus: u64) -> GzipRsyncable {
        Self {
            window_len: window,
            modulus,
        }
    }
}

impl Default for GzipRsyncable {
    fn default() -> Self {
        Self::with_window_and_modulus(8192, 4096)
    }
}

impl Chunk for GzipRsyncable {
    type SearchState = GzipRsyncableSearchState;

    fn to_search_state(&self) -> Self::SearchState {
        Self::SearchState::default()
    }

    fn find_chunk_edge(
        &self,
        state: &mut Self::SearchState,
        data: &[u8],
    ) -> (Option<usize>, usize) {
        for i in state.offset..data.len() {
            let v = data[i];

            if state.state.add(data, self, i, v) {
                state.reset();
                return (Some(i + 1), i + 1);
            }
        }

        // keep k elements = discard all but k
        let discard_ct = data.len().saturating_sub(self.window_len);
        state.offset = data.len() - discard_ct;
        (None, discard_ct)
    }
}

impl From<&GzipRsyncable> for GzipRsyncableIncr {
    fn from(src: &GzipRsyncable) -> Self {
        src.clone().into()
    }
}

impl ToChunkIncr for GzipRsyncable {
    type Incr = GzipRsyncableIncr;
    fn to_chunk_incr(&self) -> Self::Incr {
        self.into()
    }
}

#[derive(Debug, Default, Clone)]
struct GzipRsyncableState {
    accum: Wrapping<u64>,
}

impl GzipRsyncableState {
    fn reset(&mut self) {
        self.accum.0 = 0;
    }
}

/// Intermediate state for [`GzipRsyncable::find_chunk_edge`]
///
/// Using this avoids re-computation of data when no edge is found
#[derive(Debug, Default, Clone)]
pub struct GzipRsyncableSearchState {
    offset: usize,
    state: GzipRsyncableState,
}

impl GzipRsyncableSearchState {
    fn reset(&mut self) {
        self.offset = 0;
        self.state.reset();
    }
}

/// Provides an incremental interface to [`GzipRsyncable`]
///
/// Performance Note: [`GzipRsyncable`] requires look-back. As a result, [`GzipRsyncableIncr`] internally
/// buffers data up to the window size. This additional copying may affect performance. If
/// possible for your use case, use the non-incremental interface.
///
/// See [`GzipRsyncable`] for details on the underlying algorithm
#[derive(Debug, Clone)]
pub struct GzipRsyncableIncr {
    params: GzipRsyncable,

    accum: Wrapping<u64>,
    // really poor efficiency
    window: VecDeque<u8>,
}

impl GzipRsyncableIncr {
    fn reset(&mut self) {
        self.window.clear();
        self.accum = Wrapping(0);
    }
}

impl From<GzipRsyncable> for GzipRsyncableIncr {
    fn from(params: GzipRsyncable) -> Self {
        let window = VecDeque::with_capacity(params.window_len);
        GzipRsyncableIncr {
            params,
            accum: Wrapping(0),
            window,
        }
    }
}

impl GzipRsyncableState {
    fn add(&mut self, data: &[u8], parent: &GzipRsyncable, i: usize, v: u8) -> bool {
        if i >= parent.window_len {
            self.accum -= Wrapping(data[i - parent.window_len] as u64);
        }
        self.accum += Wrapping(v as u64);
        (self.accum % Wrapping(parent.modulus)).0 == 0
    }
}

impl ChunkIncr for GzipRsyncableIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        for (i, &v) in data.iter().enumerate() {
            if self.window.len() >= self.params.window_len {
                self.accum -= Wrapping(self.window.pop_front().unwrap() as u64);
            }

            self.accum += Wrapping(v as u64);
            self.window.push_back(v);

            if (self.accum % Wrapping(self.params.modulus)).0 == 0 {
                self.reset();
                return Some(i + 1);
            }
        }

        None
    }
}

impl Splitter for GzipRsyncable {
    fn find_chunk_edge<'a, 'b>(&'a self, data: &'b [u8]) -> usize {
        let mut hs = GzipRsyncableState::default();

        let mut l = 0;
        for (i, &v) in data.iter().enumerate() {
            if hs.add(data, self, i, v) {
                l = i + 1;
                break;
            }
        }

        l
    }

    fn next_iter<'a, T: Iterator<Item = u8>>(&'a self, iter: T) -> Option<Vec<u8>> {
        let mut hs = GzipRsyncableState::default();

        let a = self.window_len + self.window_len / 2;
        let mut data = Vec::with_capacity(a);
        for (i, v) in iter.enumerate() {
            data.push(v);

            if hs.add(&data, self, i, v) {
                return Some(data);
            }
        }

        if data.is_empty() {
            None
        } else {
            Some(data)
        }
    }
}
