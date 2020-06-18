//! Content defined chunking
#![warn(rust_2018_idioms,missing_debug_implementations)]
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

/*
 * TODO:
 *
 * - GearCDC
 * - FastCDC
 *
 */

use std::borrow::Borrow;
use std::mem;

pub mod bup;
pub mod zpaq;
pub mod rsyncable;
pub mod buzhash;
pub mod gear;
pub mod fastcdc;
pub mod gear_table;
pub mod mii;
pub mod ram;

pub use bup::RollSumIncr as Bup;
pub use zpaq::Zpaq;
pub use rsyncable::Rsyncable;

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
}

/// emit _complete_ slices
#[derive(Debug)]
pub struct IterSlices<'a, C: ChunkIncr> {
    rem: &'a [u8],
    chunker: C,
}

impl<'a, C: ChunkIncr> IterSlices<'a, C> {
    pub fn take_rem(&mut self) -> &'a [u8] {
        let mut l: &[u8] = &[];
        mem::swap(&mut self.rem, &mut l);
        l
    }

    pub fn into_parts(self) -> (C, &'a[u8]) {
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

/// [`iter_slices`] creates this iterator over slices of a single slice
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
        if self.rem.len() == 0 {
            return None;
        }

        match self.chunker.push(self.rem) {
            None => {
                let v = self.rem;
                self.rem = &[];
                Some(v)
            },
            Some(l) => {
                let (v, rn) = self.rem.split_at(l);
                self.rem = rn;
                Some(v)
            }
        }
    }
}

/// Given a [`ChunkIncr`] and a single slice, return a list of slices chunked by the chunker
///
/// Note that this is a non-incrimental interface. Calling this on an already fed chunker or using
/// this multiple times on the same chunker 
pub fn iter_slices<'a, C: ChunkIncr>(chunker: C, data: &'a [u8]) -> IterSlices<'a, C> {
    IterSlices {
        rem: data,
        chunker,
    }
}

/// An object with transforms a stream of bytes into chunks, potentially by examining the bytes
pub trait Splitter
{
    /**
     * Find the location (if any) to split `data` based on this splitter.
     *
     * FIXME: discards internal state when the edge is not found, meaning a user of this API would
     * have to re-process the entire thing.
     *
     * *Implimentor's Note*
     *
     * The provided implimentation uses [`Splitter::split`](#method.split).
     * You must impliment either this function or `split`.
     */
    fn find_chunk_edge(&self, data: &[u8]) -> usize {
        self.split(data).0.len()
    }

    /**
     * Split data into 2 pieces using a given splitter.
     *
     * It is expected that in most cases the second element of the return value will be split
     * further by calling this function again.
     *
     * FIXME: discards internal state when the edge is not found, meaning a user of this API would
     * have to re-process the entire thing.
     *
     * *Implimentor's Note*
     *
     * The provided implimentation uses [`find_chunk_edge`](#method.find_chunk_edge).
     * You must impliment either this function or `find_chunk_edge`.
     */
    fn split<'b>(&self, data: &'b [u8]) -> (&'b[u8], &'b[u8]) {
        let l = self.find_chunk_edge(data);
        data.split_at(l)
    }

    /**
     * Return chunks from a given iterator, split according to the splitter used.
     *
     * See the iterator generator functions [`into_vecs`](#method.into_vecs) and
     * [`as_vecs`](#method.as_vecs) which provide a more ergonomic interface to this.
     *
     * FIXME: discards internal state when the edge is not found at the end of the input iterator,
     * meaning a user of this API would have to re-process the entire thing.
     *
     */
    fn next_iter<T: Iterator<Item=u8>>(&self, iter: T) -> Option<Vec<u8>>;

    /**
     * Create an iterator over slices from a slice and a splitter.
     * The splitter is consumed.
     */
    fn into_slices<'a>(self, data: &'a [u8]) -> SplitterSlices<'a, Self>
        where Self: Sized
    {
        SplitterSlices::from(self, data)
    }

    fn as_slices<'a>(&'a self, data: &'a [u8]) -> SplitterSlices<'a, &Self>
        where Self: Sized
    {
        SplitterSlices::from(self, data)
    }

    /**
     * Create an iterator of `Vec<u8>` from an input Iterator of bytes.
     * The splitter is consumed.
     */
    fn into_vecs<'a, T: Iterator<Item=u8>>(self, data: T) -> SplitterVecs<T, Self>
        where Self: Sized
    {
        SplitterVecs::from(self, data)
    }

    fn as_vecs<'a, T: Iterator<Item=u8>>(&'a self, data: T) -> SplitterVecs<T, &Self>
        where Self: Sized
    {
        SplitterVecs::from(self, data)
    }
}

impl<'a, S: Splitter + ?Sized> Splitter for &'a S {
    fn split<'b>(&self, data: &'b [u8]) -> (&'b[u8], &'b[u8])
    {
        (*self).split(data)
    }

    fn next_iter<T: Iterator<Item=u8>>(&self, iter: T) -> Option<Vec<u8>>
    {
        (*self).next_iter(iter)
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Bound<T> {
    Included(T),
    Excluded(T),
    Unbounded,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub struct Range<T> {
    pub lower: Bound<T>,
    pub upper: Bound<T>
}

impl<T> Range<T> {
    #[allow(dead_code)]
    fn new() -> Self
    {
        Range { lower: Bound::Unbounded, upper: Bound::Unbounded }
    }

    #[allow(dead_code)]
    fn from_range(r: std::ops::Range<T>) -> Self
    {
        Range { lower: Bound::Included(r.start), upper: Bound::Excluded(r.end) }
    }

    fn from_inclusive(r: std::ops::Range<T>) -> Self
    {
        Range { lower: Bound::Included(r.start), upper: Bound::Included(r.end) }
    }

    fn exceeds_max(&self, item: &T) -> bool
        where T: PartialOrd<T>
    {
        match self.upper {
            Bound::Included(ref i) => if item > i { return true; },
            Bound::Excluded(ref i) => if item >= i { return true; },
            Bound::Unbounded => {}
        }

        false
    }

    fn under_min(&self, item: &T) -> bool
        where T: PartialOrd<T>
    {
        match self.lower {
            Bound::Included(ref i) => if item < i { return true; },
            Bound::Excluded(ref i) => if item <= i { return true; },
            Bound::Unbounded => {}
        }

        false
    }

    #[allow(dead_code)]
    fn contains(&self, item: &T) -> bool
        where T: PartialOrd<T>
    {
        /* not excluded by lower */
        if self.under_min(item) {
            return false;
        }

        if self.exceeds_max(item) {
            return false;
        }

        true
    }
}

/// Iterator over slices emitted from a splitter
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SplitterSlices<'a, T: Splitter + 'a> {
    parent: T,
    d: &'a [u8],
}

impl<'a, T: Splitter> SplitterSlices<'a, T> {
    pub fn from(i: T, d : &'a [u8]) -> Self
    {
        SplitterSlices {
            parent: i,
            d,
        }
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
    fn size_hint(&self) -> (usize, Option<usize>)
    {
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
    pub fn from(i: P, d: T) -> Self
    {
        SplitterVecs {
            parent: i,
            d: d,
        }
    }
}

impl<T: Iterator<Item=u8>, P: Splitter> Iterator for SplitterVecs<T, P> {
    type Item = Vec<u8>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.parent.borrow().next_iter(&mut self.d)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>)
    {
        /* At most, we'll end up returning a vec for every byte, +1 empty slice */
        let (a, b) = self.d.size_hint();
        (a, if let Some(c) = b { Some(c + 1) } else { None })
    }
}

