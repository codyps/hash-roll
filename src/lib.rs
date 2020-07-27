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
//! - Configured Algorithm Instance (impliments [`Splitter`]). Normally named plainly (like
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
//! methods to obtain other kinds of APIs, like Incrimentals.
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

use std::borrow::Borrow;
use std::mem;

pub mod bup;
pub mod buzhash;
pub mod buzhash_table;
pub mod fastcdc;
pub mod gear;
pub mod gear_table;
pub mod mii;
pub mod ram;
pub mod range;
pub mod gzip;
pub mod pigz;
pub mod zstd;
pub mod zpaq;

pub(crate) use range::RangeExt;

/// Accept incrimental input and provide indexes of split points
///
/// Note that for some splitting/chunking algorithms, this mechanism will be less efficient. In
/// particular, algorithms like [`Rsyncable`] that require the use of previously examined data to
/// shift their "window" (resulting in needing a circular buffer which all inputed data passes
/// through) will perform more poorly using [`ChunkIncr`] compared with internal or one-shot
/// interfaces.
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
    /// Does not return the remainder (if any) in the iteration. Use [`IterSlices::take_rem()`] or
    /// [`IterSlices::into_parts()`] to get the remainder.
    ///
    /// Note that this is a non-incrimental interface. Calling this on an already fed chunker or using
    /// this multiple times on the same chunker may provide unexpected results
    fn iter_slices<'a>(self, data: &'a [u8]) -> IterSlices<'a, Self>
    where
        Self: std::marker::Sized,
    {
        IterSlices {
            rem: data,
            chunker: self,
        }
    }
}

/// Returned by [`ChunkIncr::iter_slices()`]
///
/// Always emits _complete_ slices durring iteration.
#[derive(Debug)]
pub struct IterSlices<'a, C: ChunkIncr> {
    rem: &'a [u8],
    chunker: C,
}

impl<'a, C: ChunkIncr> IterSlices<'a, C> {
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

impl<'a, C: ChunkIncr> Iterator for IterSlices<'a, C> {
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

/// Returned by [`ChunkIncr::iter_slices_partial()`]
///
/// When it runs out of data, it returns the remainder as the last element of the iteration
#[derive(Debug)]
pub struct IterSlicesPartial<'a, C: ChunkIncr> {
    rem: &'a [u8],
    chunker: C,
}

impl<'a, C: ChunkIncr> Iterator for IterSlicesPartial<'a, C> {
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
pub trait Chunk {
    /// `SearchState` allows searching for the chunk edge to resume without duplicating work
    /// already done.
    type SearchState;

    /// `Incr` provides the incrimental interface to this chunking instance
    type Incr: ChunkIncr;

    /// Find the location in `data` to split the data
    ///
    /// If no split point is found, the `SearchState` is returned. One should only use the
    /// `SearchState` if `find_chunk_edge()` will be called again with a new `data` that consists
    /// of the previous `data` with a suffix added. In other words, if `data` is extended with
    /// additional bytes.
    ///
    /// An alternate that doesn't require repeatedly passing `data` is the [`incrimental()`]
    /// incrimental api.
    ///
    /// Note: calling with a previous `state` with a new `data` that isn't an extention of the
    /// previous `data` will result in split points that may not follow the design of the
    /// underlying algorithm. Avoid relying on consistent cut points to reason about memory safety.
    ///
    /// Note: if this returns a cut point (as `Ok(usize)`), and you want further splitting, you
    /// should pass the _remainder_ of the `data` with `state: None`.
    // Potential pitfal: for better performance, keeping the return value small is a very good
    // idea. By returning `SearchState`, we've potentially enlarged the return value quite a bit.
    //
    // Another alterate here is to have one pass in a `&mut SearchState`. The downside is that it
    // would then need to be cleared to prevent common issues with re-use (ie: we expect to see a
    // loop with a single `SearchState`, and requring explicit `reset()`ing of the `SearchState`
    // will increase error rates.
    //
    // Consider if result should return `(&[u8], &[u8])` instead of an index (which would then be
    // given to `.split_at()`
    fn find_chunk_edge(
        &self,
        state: Option<Self::SearchState>,
        data: &[u8],
    ) -> Result<usize, Self::SearchState>;

    /// `incrimental()` returns a [`ChunkIncr`] which can be incrimentally fed data and emits
    /// chunks.
    ///
    /// This allows avoiding having to buffer all data in memory, and avoids the need to use a
    /// single buffer (even if all data is in memory).
    ///
    /// Note that some algorithms that need to look back through their data to update their state
    /// will be less efficient when the incrimental interface is used
    fn incrimental(&self) -> Self::Incr;
}

/// `Splitter`s define how to split a stream of bytes into chunks. They are instances of CDC
/// algorthims with their parameters.
///
pub trait Splitter {
    /// Find the location (if any) to split `data` based on this splitter.
    ///
    /// Note: this doesn't preserve any intermediate state. Having intermediate state may be useful
    /// to have if you plan on extending the input data whan a split point is not found.
    ///
    /// ## For implimenting `Splitter`
    ///
    /// The provided implimentation uses [`Splitter::split`](#method.split).
    /// You must impliment either this function or `split`.
    fn find_chunk_edge(&self, data: &[u8]) -> usize {
        self.split(data).0.len()
    }

    ///
    /// Split data into 2 pieces using a given splitter.
    ///
    /// It is expected that in most cases the second element of the return value will be split
    /// further by calling this function again.
    ///
    /// Note: this doesn't preserve any intermediate state. Having intermediate state may be useful
    /// to have if you plan on extending the input data whan a split point is not found.
    ///
    /// *Implimentor's Note*
    ///
    /// The provided implimentation uses [`find_chunk_edge`](#method.find_chunk_edge).
    /// You must impliment either this function or `find_chunk_edge`.
    ///
    fn split<'b>(&self, data: &'b [u8]) -> (&'b [u8], &'b [u8]) {
        let l = self.find_chunk_edge(data);
        data.split_at(l)
    }

    /// Consumes the iterator of bytes `iter`, and returns a vector of the next chunk (if any)
    ///
    /// See the iterator generator functions [`into_vecs`](#method.into_vecs) and
    /// [`as_vecs`](#method.as_vecs) which provide a more ergonomic interface to this.
    ///
    /// Note: performance of this function is _really_ bad. This iterating over bytes and copying
    /// every byte into a `Vec` is not cheap.
    ///
    /// Note: this doesn't preserve any intermediate state. Having intermediate state may be useful
    /// to have if you plan on extending the input data whan a split point is not found.
    ///
    fn next_iter<T: Iterator<Item = u8>>(&self, iter: T) -> Option<Vec<u8>>;

    /**
     * Create an iterator over slices from a slice and a splitter.
     * The splitter is consumed.
     */
    fn into_slices<'a>(self, data: &'a [u8]) -> SplitterSlices<'a, Self>
    where
        Self: Sized,
    {
        SplitterSlices::from(self, data)
    }

    fn as_slices<'a>(&'a self, data: &'a [u8]) -> SplitterSlices<'a, &Self>
    where
        Self: Sized,
    {
        SplitterSlices::from(self, data)
    }

    ///
    /// Create an iterator of `Vec<u8>` from an input Iterator of bytes.
    /// The splitter is consumed.
    ///
    fn into_vecs<T: Iterator<Item = u8>>(self, data: T) -> SplitterVecs<T, Self>
    where
        Self: Sized,
    {
        SplitterVecs::from(self, data)
    }

    fn as_vecs<T: Iterator<Item = u8>>(&self, data: T) -> SplitterVecs<T, &Self>
    where
        Self: Sized,
    {
        SplitterVecs::from(self, data)
    }
}

impl<'a, S: Splitter + ?Sized> Splitter for &'a S {
    fn split<'b>(&self, data: &'b [u8]) -> (&'b [u8], &'b [u8]) {
        (*self).split(data)
    }

    fn next_iter<T: Iterator<Item = u8>>(&self, iter: T) -> Option<Vec<u8>> {
        (*self).next_iter(iter)
    }
}

/// Iterator over slices emitted from a splitter
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SplitterSlices<'a, T: Splitter + 'a> {
    parent: T,
    d: &'a [u8],
}

impl<'a, T: Splitter> SplitterSlices<'a, T> {
    pub fn from(i: T, d: &'a [u8]) -> Self {
        SplitterSlices { parent: i, d }
    }
}

impl<'a, T: Splitter> Iterator for SplitterSlices<'a, T> {
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.d.is_empty() {
            return None;
        }

        let (a, b) = self.parent.borrow().split(self.d);
        if a.is_empty() {
            /* FIXME: this probably means we won't emit an empty slice */
            self.d = a;
            Some(b)
        } else {
            self.d = b;
            Some(a)
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        /* At most, we'll end up returning a slice for every byte, +1 empty slice */
        if self.d.is_empty() {
            (0, Some(0))
        } else {
            (1, Some(self.d.len() + 1))
        }
    }
}

/// Iterator over vecs emitted from a splitter
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SplitterVecs<T, P: Splitter> {
    parent: P,
    d: T,
}

impl<T, P: Splitter> SplitterVecs<T, P> {
    pub fn from(i: P, d: T) -> Self {
        SplitterVecs { parent: i, d }
    }
}

impl<T: Iterator<Item = u8>, P: Splitter> Iterator for SplitterVecs<T, P> {
    type Item = Vec<u8>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.parent.borrow().next_iter(&mut self.d)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        /* At most, we'll end up returning a vec for every byte, +1 empty slice */
        let (a, b) = self.d.size_hint();
        (a, if let Some(c) = b { Some(c + 1) } else { None })
    }
}
