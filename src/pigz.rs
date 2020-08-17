#![cfg(feature = "pigz")]
use crate::{Chunk, ChunkIncr, ToChunkIncr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PigzRsyncable {
    bits: u8,

    /// directly derived from `bits`
    mask: u32,
    /// directly derived from `mask`
    hit: u32,
}

impl PigzRsyncable {
    pub fn with_bits(bits: u8) -> PigzRsyncable {
        let mask = (1 << bits) - 1;
        let hit = mask >> 1;
        PigzRsyncable { bits, mask, hit }
    }
}

impl Default for PigzRsyncable {
    fn default() -> Self {
        Self::with_bits(12)
    }
}

impl Chunk for PigzRsyncable {
    type SearchState = PigzRsyncableSearchState;

    fn to_search_state(&self) -> Self::SearchState {
        self.into()
    }

    fn find_chunk_edge(
        &self,
        state: &mut Self::SearchState,
        data: &[u8],
    ) -> (Option<usize>, usize) {
        for i in 0..data.len() {
            let v = data[i];

            if state.state.add(self, v) {
                *state = self.to_search_state();
                return (Some(i + 1), i + 1);
            }
        }

        (None, data.len())
    }
}

impl From<&PigzRsyncable> for PigzRsyncableIncr {
    fn from(src: &PigzRsyncable) -> Self {
        src.clone().into()
    }
}

impl ToChunkIncr for PigzRsyncable {
    type Incr = PigzRsyncableIncr;
    fn to_chunk_incr(&self) -> Self::Incr {
        self.into()
    }
}

#[derive(Debug, Clone)]
struct PigzRsyncableState {
    hash: u32,
}

impl From<&PigzRsyncable> for PigzRsyncableState {
    fn from(params: &PigzRsyncable) -> Self {
        PigzRsyncableState { hash: params.hit }
    }
}

/// Intermediate state for [`PigzRsyncable::find_chunk_edge`]
///
/// Using this avoids re-computation of data when no edge is found
#[derive(Debug, Clone)]
pub struct PigzRsyncableSearchState {
    state: PigzRsyncableState,
}

impl From<&PigzRsyncable> for PigzRsyncableSearchState {
    fn from(params: &PigzRsyncable) -> Self {
        PigzRsyncableSearchState {
            state: params.into(),
        }
    }
}

/// Provides an incremental interface to [`PigzRsyncable`]
///
/// Performance Note: [`PigzRsyncable`] requires look-back. As a result, [`PigzRsyncableIncr`] internally
/// buffers data up to the window size. This additional copying may affect performance. If
/// possible for your use case, use the non-incremental interface.
///
/// See [`PigzRsyncable`] for details on the underlying algorithm
#[derive(Debug, Clone)]
pub struct PigzRsyncableIncr {
    params: PigzRsyncable,
    state: PigzRsyncableState,
}

impl PigzRsyncableIncr {}

impl From<PigzRsyncable> for PigzRsyncableIncr {
    fn from(params: PigzRsyncable) -> Self {
        let state = (&params).into();
        PigzRsyncableIncr { params, state }
    }
}

impl PigzRsyncableState {
    fn add(&mut self, parent: &PigzRsyncable, v: u8) -> bool {
        self.hash = ((self.hash << 1) ^ (v as u32)) & parent.mask;
        self.hash == parent.hit
    }
}

impl ChunkIncr for PigzRsyncableIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        for (i, &v) in data.iter().enumerate() {
            if self.state.add(&self.params, v) {
                self.state = (&self.params).into();
                return Some(i + 1);
            }
        }

        None
    }
}
