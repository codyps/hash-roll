//! hash-roll provides various content defined chunking algorithms
//!
//! Content defined chunking (CDC) algorithms are algorithms that examine a stream of input bytes (often
//! represented as a slice like `[u8]`, and provide locations within that input stream to split or
//! chunk the stream into parts.
//!
//! CDC algorithms generally try to optimize for the following:
//!
//!  1. Processing speed (ie: bytes/second)
//!  2. Stability in split locations even when insertions/deletions of bytes occur
//!  3. Reasonable distributions of chunk lengths
//!
//! ## API Concepts
//!
//! - Configured Algorithm Instance (impliments [`Chunk`]). Normally named plainly (like
//!   [`Bup`]). These can be thought of as "parameters" for an algorithm.
//! - Incrimental (impliments [`ChunkIncr`]). Normally named with `Incr` suffix.
//!
//! Because of the various ways one might use a CDC, and the different CDC algorithm
//! characteristics, hash-roll provides a few ways to use them.
//!
//! Configured Algorithm Instances are created from the set of configuration needed for a given
//! algorithm. For example, this might mean configuring a window size or how to decide where to
//! split. These don't include any mutable data, in other words: they don't keep track of what data
//! is given to them. Configured Algorithm Instances provide the all-at-once APIs, as well as
//! methods to obtain other kinds of APIs, like incrimental style apis.
//!
//! ## CDC Algorithms and Window Buffering
//!
//! Different CDC algorithms have different constraints about how they process data. Notably, some
//! require a large amount of previous processed data to process additional data. This "large
//! amount of previously processed data" is typically referred to as the "window". That said, note
//! that some CDC algorithms that use a window concept don't need previously accessed data.
//!
//! For the window-buffering algorithms, their is an extra cost to certain types of API
//! implimentations. The documentation will note when these occur and suggest alternatives.
//!
//! Generally, CDC interfaces that are incrimental will be slower for window-buffering algorithms.
//! Using an explicitly allocating interface (which emits `Vec<u8>` or `Vec<Vec<u8>>`) will have no
//! worse performance that the incrimental API, but might be more convenient. Using an all-at-once
//! API will provide the best performance due to not requiring any buffering (the input data can be
//! used directly).
//!
//! ## Use Cases that drive API choices
//!
//!  - accumulate vecs, emits vecs
//!    - incrimental: yes
//!    - input: `Vec<u8>`
//!    - internal state: `Vec<Vec<u8>>`
//!    - output: `Vec<Vec<u8>>`
//!
//!  - stream data through
//!    - incrimenal: yes
//!    - input: `&[u8]`
//!
//!  - mmap (or read entire) file, emit
//!    - incrimenal: no
//!    - input: `&[u8]`
//!    - output: `&[u8]`

// # API Design Notes
//
// ## What methods should be in a trait? What should be in wrapper structs?
//
//  - place methods that might have more optimized variants, but can have common implimentations,
//    in a trait. This notably affects window-buffering differences: it's always possible to
//    impliment all-at-once processing using incrimental interfaces that internally buffer, but
//    it's much more efficient for window-buffering algorithms to provide implimentations that know
//    how to look into the input data directly.

#![warn(rust_2018_idioms, missing_debug_implementations)]
/* TODO: Rabin-Karp
 * H = c_1 * a ** (k-1) + c_2 * a ** (k-2) ... + c_k * a ** 0
 * where:
 *  a is a constant
 *  c_1, ..., c_k are the input characters
 *
 * All math is done modulo n. Choice of n & a critical
 *
 * Parameters:
 *  - n: mululo limit
 *  - a: a constant
 *
 * State:
 *  H
 *
 * Application:
 */

/* TODO:
 * rollsum of librsync
 */

use std::mem;

pub mod bup;
pub mod buzhash;
pub mod buzhash_table;
pub mod fastcdc;
pub mod gear;
pub mod gear_table;
pub mod gzip;
pub mod mii;
pub mod pigz;
pub mod ram;
pub mod range;
pub mod zpaq;
pub mod zstd;

pub(crate) use range::RangeExt;

/// Accept incrimental input and provide indexes of split points
///
/// Compared to [`Chunk`], [`ChunkIncr`] allows avoiding having to buffer all input data in memory,
/// and avoids the need to use a single buffer for storing the input data (even if all data is in
/// memory).
///
/// Data fed into a given [`ChunkIncr`] instance is considered to be part of the same
/// data "source". This affects chunking algorithms that maintain some state between chunks
/// (like `ZstdRsyncable` does). If you have multiple "sources", one should obtain new instances of
/// [`ChunkIncr`] for each of them (typically via [`ToChunkIncr`]).
///
/// Note that for some splitting/chunking algorithms, the incrimental api will be less efficient
/// compared to the non-incrimental API. In particular, algorithms like [`Rsyncable`] that require
/// the use of previously examined data to shift their "window" (resulting in needing a circular
/// buffer which all inputed data passes through) will perform more poorly using [`ChunkIncr`]
/// compared with non-incrimental interfaces
pub trait ChunkIncr {
    /// The data "contained" within a implimentor of this trait is the history of all data slices
    /// passed to feed.
    ///
    /// In other words, all previous data (or no previous data) may be used in determining the
    /// point to split.
    ///
    /// Returns None if the data has no split point.
    /// Otherwise, returns an index in the most recently passed `data`.
    ///
    /// Note that returning the index in the current slice makes most "look-ahead" splitting
    /// impossible (as it is permissible to pass 1 byte at a time).
    fn push(&mut self, data: &[u8]) -> Option<usize>;

    /// Given a [`ChunkIncr`] and a single slice, return a list of slices chunked by the chunker.
    ///
    /// Will always return enough slices to form the entire content of `data`, even if the trailing
    /// part of data is not a chunk (ie: does not end on a chunk boundary)
    fn iter_slices<'a>(self, data: &'a [u8]) -> IterSlices<'a, Self>
    where
        Self: std::marker::Sized,
    {
        IterSlices {
            rem: data,
            chunker: self,
        }
    }

    /// Given a [`ChunkIncr`] and a single slice, return a list of slices chunked by the chunker.
    /// Does not return the remainder (if any) in the iteration. Use [`IterSlices::take_rem()`] or
    /// [`IterSlices::into_parts()`] to get the remainder.
    ///
    /// Note that this is a non-incrimental interface. Calling this on an already fed chunker or using
    /// this multiple times on the same chunker may provide unexpected results
    fn iter_slices_strict<'a>(self, data: &'a [u8]) -> IterSlicesStrict<'a, Self>
    where
        Self: std::marker::Sized,
    {
        IterSlicesStrict {
            rem: data,
            chunker: self,
        }
    }
}

/// Returned by [`ChunkIncr::iter_slices_strict()`]
///
/// Always emits _complete_ slices durring iteration.
#[derive(Debug)]
pub struct IterSlicesStrict<'a, C: ChunkIncr> {
    rem: &'a [u8],
    chunker: C,
}

impl<'a, C: ChunkIncr> IterSlicesStrict<'a, C> {
    /// Take the remainder from this iterator. Leaves an empty slice in it's place.
    pub fn take_rem(&mut self) -> &'a [u8] {
        let mut l: &[u8] = &[];
        mem::swap(&mut self.rem, &mut l);
        l
    }

    /// Obtain the internals
    ///
    /// Useful, for example, after iteration stops to obtain the remaining slice.
    pub fn into_parts(self) -> (C, &'a [u8]) {
        (self.chunker, self.rem)
    }
}

impl<'a, C: ChunkIncr> Iterator for IterSlicesStrict<'a, C> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        match self.chunker.push(self.rem) {
            None => None,
            Some(l) => {
                let (v, rn) = self.rem.split_at(l);
                self.rem = rn;
                Some(v)
            }
        }
    }
}

/// Returned by [`ChunkIncr::iter_slices()`]
///
/// When it runs out of data, it returns the remainder as the last element of the iteration
#[derive(Debug)]
pub struct IterSlices<'a, C: ChunkIncr> {
    rem: &'a [u8],
    chunker: C,
}

impl<'a, C: ChunkIncr> IterSlices<'a, C> {
    /// Obtain the internals
    ///
    /// Useful, for example, after iteration stops to obtain the remaining slice.
    pub fn into_parts(self) -> (C, &'a [u8]) {
        (self.chunker, self.rem)
    }
}

impl<'a, C: ChunkIncr> Iterator for IterSlices<'a, C> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.rem.is_empty() {
            return None;
        }

        match self.chunker.push(self.rem) {
            None => {
                let v = self.rem;
                self.rem = &[];
                Some(v)
            }
            Some(l) => {
                let (v, rn) = self.rem.split_at(l);
                self.rem = rn;
                Some(v)
            }
        }
    }
}

/// Impl on algorthms that define methods of chunking data
///
/// This is the lowest level (but somewhat restrictive) trait for chunking algorthms.  It assumes
/// that the input is provided to it in a contiguous slice. If you don't have your input as a
/// contiguous slice, [`ChunkIncr`] may be a better choice (it allows non-contiguous input, but may
/// be slowing for some chunking algorthms).
pub trait Chunk {
    /// `SearchState` allows searching for the chunk edge to resume without duplicating work
    /// already done.
    type SearchState;

    /*
    /// Amount of data from already emitted chunks requried for determining future chunks
    ///
    /// Indicates the amount of data that _must_ be preserved for [`find_chunk_edge()`]'s
    /// `prev_data` argument. If more that this is passed, the last bytes in the slice are used. At
    /// the start of an input (where there is no previous data), an empty slice would be used.
    ///
    /// For most chunking algorithms, this is `0` (zero), indicating that `prev_data` may always be
    /// an empty slice.
    const CARRY_LEN: usize;
    */

    /// Provide an initial [`SearchState`] for use with [`find_chunk_edge()`]. Generally, for each
    /// input one should generate a new [`SearchState`].
    fn to_search_state(&self) -> Self::SearchState;

    /// Find the next "chunk" in `data` to emit
    ///
    /// The return value is a pair of a range representing the start and end of the chunk being
    /// emitted, and the offset from which subsequent `data` subsets should be passed to the next
    /// call to `find_chunk_edge`.
    ///
    /// `state` is mutated so that it does not rexamine previously examined data, even when a chunk
    /// is not emitted.
    ///
    /// `data` may be extended with additional data between calls to `find_chunk_edge()`. The bytes
    /// that were _previously_ in `data` and are not indicated by `discard_ct` must be preserved in
    /// the next `data` buffer called.
    ///
    /// ```rust
    /// use hash_roll::Chunk;
    ///
    /// fn some_chunk() -> impl Chunk {
    ///     hash_roll::mii::Mii::default()
    /// }
    ///
    /// let chunk = some_chunk();
    /// let orig_data = b"hello";
    /// let mut data = &orig_data[..];
    /// let mut ss = chunk.to_search_state();
    /// let mut prev_cut = 0;
    ///
    /// loop {
    ///    let (chunk, discard_ct) = chunk.find_chunk_edge(&mut ss, data);
    ///
    ///    match chunk {
    ///        Some(cut_point) => {
    ///            // map `cut_point` from the current slice back into the original slice so we can
    ///            // have consistent indexes
    ///            let g_cut = cut_point + orig_data.len() - data.len();
    ///            println!("chunk: {:?}", &orig_data[prev_cut..cut_point]);
    ///        },
    ///        None => {
    ///            println!("no chunk, done with data we have");
    ///            println!("remain: {:?}", &data[discard_ct..]);
    ///            break;
    ///        }
    ///    }
    ///
    ///    data = &data[discard_ct..];
    /// }
    /// ```
    ///
    /// Note: call additional times on the same `SearchState` and the required `data` to obtain
    /// subsequent chunks in the same input data. To handle a seperate input, use a new
    /// `SearchState`.
    ///
    /// Note: calling with a previous `state` with a new `data` that isn't an extention of the
    /// previous `data` will result in split points that may not follow the design of the
    /// underlying algorithm. Avoid relying on consistent cut points to reason about memory safety.
    ///
    // NOTE: the reason that we preserve `state` even when chunks are emitted is that some
    // algorthims require some state to pass between chunks for a given input. zstd includes an
    // example of an algorithm that needs this
    //
    // Potential pitfal: for better performance, keeping the return value small is a very good
    // idea. By returning ~2x64+32, we are might be less performant depending on the ABI selected.
    //
    // Consider if result should return `(&[u8], &[u8])` instead of an index (which would then be
    // given to `.split_at()`
    //
    // Consider if `state` should have a `reset()` method to avoid reallocating
    //
    // API:
    //  - `fn find_chunk_edge(&self, state: &mut Self::SearchState, data: &[u8]) -> (Option<(usize, uszie)>, usize);
    //     - Problem: unclear what indexes of slices represent: start can't be in the data being
    //       passed because we don't require `data` include the start of the chunk
    //  - `fn find_chunk_edge(&self, state: &mut Self::SearchState, data: &[u8]) -> (Option<usize>, usize);
    //     - Problem: user code to track indexing match up is somewhat difficult
    //     - mostly due to needing an extra index to track to handle the "last chunk" location not
    //     being the "slice we need to pass start"
    fn find_chunk_edge(&self, state: &mut Self::SearchState, data: &[u8])
        -> (Option<usize>, usize);
}

/// Implimented on types which can be converted to/can provide a [`ChunkIncr`] interface.
///
/// Types that impliment this generally represent a instantiation of a chunking algorithm.
// NOTE: we use this instead of just having `From<&C: Chunk> for CI: ChunkIncr` because there is
// _one_ `ChunkIncr` for each `Chunk`, and rust can't infer that when using a `From` or `Into`
// bound.
//
// We could consider adding `type Incr` into `trait Chunk`, or only having `type Incr`
pub trait ToChunkIncr {
    /// `Incr` provides the incrimental interface to this chunking instance
    type Incr: ChunkIncr;

    /// `to_chunk_incr()` returns a [`ChunkIncr`] which can be incrimentally fed data and emits
    /// chunks.
    ///
    /// Generally, this is a typically low cost operation that copies from the implimentor or does
    /// minor computation on its fields and may allocate some memory for storing additional state
    /// needed for incrimental computation.
    fn to_chunk_incr(&self) -> Self::Incr;
}
