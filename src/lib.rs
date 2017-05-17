#![cfg_attr(all(feature = "nightly", test), feature(test))]

#[cfg(all(feature = "nightly", test))]
extern crate test;
#[cfg(all(feature = "nightly", test))]
extern crate histogram;

#[cfg(test)]
extern crate rand;
#[cfg(test)]
extern crate rollsum;

extern crate fmt_extra;



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

/* TODO: Cyclic polynomial (buzhash)
 *
 * H = s ** (k -1) (h(c_1)) ^ s**(k-2)(h(c_2)) ^ ... ^ s(h(c_(k-1))) ^ h(c_k)
 * where s(x) is a barrel shift of x (ABCDEFG becomes BCDEFGA, where each letter is a bit)
 * s**y(x) is application of s(n) y times.
 *
 * Application:
 *
 *  - H <- s(H)
 *  - c_1 <- s**k(h(c_1))
 *  - H <- s(H) ^ s**k(h(c_1)) ^ h(c_(k+1))
 *
 *  Where c_1 is the character to remove,
 *      c_(k+1) is the character to add
 *
 * Parameters:
 *  - k: number of inputs to contain (can be un-capped?)
 *  - h: a hash function from inputs to integers on [2, 2**L)
 *
 * State:
 *  - every input contained in the hash (if removal is required)
 *  - previous hash result
 */

/* TODO:
 * rollsum of librsync
 */

use std::borrow::Borrow;

pub mod bup;
pub mod zpaq;
pub mod rsyncable;

pub use bup::Bup;
pub use zpaq::Zpaq;
pub use rsyncable::Rsyncable;

#[cfg(all(feature = "nightly", test))]
mod bench;

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

#[derive(Clone, Debug, Copy)]
pub enum Bound<T> {
    Included(T),
    Excluded(T),
    Unbounded,
}

#[derive(Clone, Debug, Copy)]
pub struct Range<T> {
    pub first: Bound<T>,
    pub last: Bound<T>
}

impl<T> Range<T> {
    #[allow(dead_code)]
    fn new() -> Self
    {
        Range { first: Bound::Unbounded, last: Bound::Unbounded }
    }

    #[allow(dead_code)]
    fn from_range(r: std::ops::Range<T>) -> Self
    {
        Range { first: Bound::Included(r.start), last: Bound::Excluded(r.end) }
    }

    fn from_inclusive(r: std::ops::Range<T>) -> Self
    {
        Range { first: Bound::Included(r.start), last: Bound::Included(r.end) }
    }

    fn exceeds_max(&self, item: &T) -> bool
        where T: PartialOrd<T>
    {
        match self.last {
            Bound::Included(ref i) => if item > i { return true; },
            Bound::Excluded(ref i) => if item >= i { return true; },
            Bound::Unbounded => {}
        }

        false
    }

    fn under_min(&self, item: &T) -> bool
        where T: PartialOrd<T>
    {
        match self.first {
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
        /* not excluded by first */
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
#[derive(Debug, Clone)]
pub struct SplitterSlices<'a, T: Splitter + 'a> {
    parent: T,
    d: &'a [u8],
}

impl<'a, T: Splitter> SplitterSlices<'a, T> {
    pub fn from(i: T, d : &'a [u8]) -> Self
    {
        SplitterSlices {
            parent: i,
            d: d,
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
#[derive(Debug, Clone)]
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

#[test]
fn test_rsyncable() {
    use std::collections::HashSet;

    let d1 = b"hello, this is some bytes";
    let mut d2 = d1.clone();
    d2[4] = ':' as u8;

    let b1 = Rsyncable::with_window_and_modulus(4, 8).into_vecs(d1.iter().cloned());
    let b2 = Rsyncable::with_window_and_modulus(4, 8).into_vecs(d2.iter().cloned());

    let c1 = b1.clone().count();
    let c2 = b2.clone().count();

    /* XXX: in this contrived case, we generate the same number of blocks.
     * We should generalize this test to guess at "reasonable" differences in block size
     */
    assert_eq!(c1, 4);
    assert!((c1 as i64 - c2 as i64).abs() < 1);

    /* check that some blocks match up */

    let mut blocks = HashSet::with_capacity(c1);
    let mut common_in_b1 = 0u64;
    for b in b1 {
        if !blocks.insert(b) {
            common_in_b1 += 1;
        }
    }

    println!("common in b1: {}", common_in_b1);

    let mut shared_blocks = 0u64;
    for b in b2 {
        if blocks.contains(&b) {
            shared_blocks += 1;
        }
    }

    /* XXX: this is not a generic test, we can't rely on it */
    println!("shared blocks: {}", shared_blocks);
    assert!(shared_blocks > (c1 as u64) / 2);
}
