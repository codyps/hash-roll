#![cfg(feature = "ram")]

//! Rapid Asymmetric Maximum (RAM) is a fast chunking algorithm
//!
//! - Has a minimum block size (it's "window" size)
//! - Does not provide an upper bound on block size (though paper discusses a RAML variant that
//!   does).
//!
//! doi:10.1016/j.future.2017.02.013
//!
use crate::{Chunk, ToChunkIncr, ChunkIncr};

/// Parameters for the Rapid Asymmetric Maximum (RAM) chunking algorithm
///
/// Is window free, with very small (
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ram {
    /// window size
    ///
    /// fixed data
    w: u64,
}

impl Ram {
    /// Construct a RAM instance with window size `w`
    ///
    /// `w` is also the minimum block size
    pub fn with_w(w: u64) -> Self {
        Self {
            w,
        }
    }
}

impl Chunk for Ram {
    type SearchState = RamState;

    fn to_search_state(&self) -> Self::SearchState {
        Default::default()
    }

    fn find_chunk_edge(&self, state: &mut Self::SearchState, data: &[u8]) -> (Option<usize>, usize) {
        match state.push(self, data) {
            Some(i) => (Some(i + 1), i + 1),
            None => (None, data.len())
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct RamState {
    /// global index (number of processed bytes since split)
    i: u64,

    ///
    max_val: u8,
}

impl RamState {
    fn push(&mut self, params: &Ram, data: &[u8]) -> Option<usize> {
        let i = self.i;

        for (l_i, b) in data.iter().cloned().enumerate() {
            if b >= self.max_val {
                // minimum block size
                let ri = l_i as u64 + i;
                if ri > params.w {
                    self.i = 0;
                    self.max_val = 0;
                    return Some(l_i);
                }

                self.max_val = b;
            }
        }

        self.i += data.len() as u64;
        None
    }
}

impl ToChunkIncr for Ram {
    type Incr = RamIncr;

    fn to_chunk_incr(&self) -> Self::Incr {
        self.into()
    }
}

impl From<&Ram> for RamIncr {
    fn from(params: &Ram) -> Self {
        Self {
            params: params.clone(),
            state: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RamIncr {
    params: Ram,
    state: RamState,
}

impl ChunkIncr for RamIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        self.state.push(&self.params, data) 
    }
}
