use crate::{Chunk, ChunkIncr};

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
        PigzRsyncable {
            bits,
            mask,
            hit
        }
    }
}

impl Default for PigzRsyncable {
    fn default() -> Self {
        Self::with_bits(12)
    }
}

impl Chunk for PigzRsyncable {
    type SearchState = PigzRsyncableSearchState;
    type Incr = PigzRsyncableIncr;

    fn find_chunk_edge(
        &self,
        state: Option<Self::SearchState>,
        data: &[u8],
    ) -> Result<usize, Self::SearchState> {
        let mut hs = match state {
            Some(v) => v,
            None => Self::SearchState::from(self),
        };

        for i in hs.offset..data.len() {
            let v = data[i];

            if hs.state.add(self, v) {
                return Ok(i + 1);
            }
        }

        hs.offset = data.len();
        Err(hs)
    }

    fn incrimental(&self) -> Self::Incr {
        From::from(self.clone())
    }
}

#[derive(Debug, Clone)]
struct PigzRsyncableState {
    hash: u32,
}

impl From<&PigzRsyncable> for PigzRsyncableState {
    fn from(params: &PigzRsyncable) -> Self {
        PigzRsyncableState {
            hash: params.hit
        }
    }
}

/// Intermediate state for [`PigzRsyncable::find_chunk_edge`]
///
/// Using this avoids re-computation of data when no edge is found
#[derive(Debug, Clone)]
pub struct PigzRsyncableSearchState {
    offset: usize,
    state: PigzRsyncableState,
}

impl From<&PigzRsyncable> for PigzRsyncableSearchState {
    fn from(params: &PigzRsyncable) -> Self {
        PigzRsyncableSearchState {
            offset: 0,
            state: From::from(params),
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

impl PigzRsyncableIncr {
}

impl From<PigzRsyncable> for PigzRsyncableIncr {
    fn from(params: PigzRsyncable) -> Self {
        let state = PigzRsyncableState::from(&params);
        PigzRsyncableIncr {
            params,
            state,
        }
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
                self.state = PigzRsyncableState::from(&self.params);
                return Some(i + 1);
            }
        }

        None
    }
}