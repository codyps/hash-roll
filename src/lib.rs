#![cfg_attr(feature = "nightly", feature(test))]

#[cfg(all(feature = "nightly", test))]
extern crate test;
#[cfg(all(feature = "nightly", test))]
extern crate rand;
#[cfg(all(feature = "nightly", test))]
extern crate histogram;


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
 *
 * bupsplit, part of bup's "hashsplit" library
 * rollsum of librsync
 *
 */

use std::num::Wrapping;
use std::borrow::Borrow;

pub mod circ;
pub mod window;
pub mod slice;

#[cfg(all(feature = "nightly", test))]
mod bench;

use slice::SliceExt;

pub trait Splitter
{
    fn into_slices<'a>(self, data: &'a [u8]) -> SplitterSplit<'a, Self>
        where Self: Sized
    {
        SplitterSplit::from(self, data)
    }

    fn as_slices<'a>(&'a self, data: &'a [u8]) -> SplitterSplit<'a, &Self>
        where Self: Sized
    {
        SplitterSplit::from(self, data)
    }

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


    /**
     * Split data into 2 pieces using a given splitter.
     *
     * It is expected that in most cases the second element of the return value will be split
     * further by calling this function again.
     */
    fn split<'b>(&self, data: &'b [u8]) -> (&'b[u8], &'b[u8]);

    /**
     * Return chunks from a given iterator, split according to the splitter used.
     */
    fn next_iter<T: Iterator<Item=u8>>(&self, iter: T) -> Option<Vec<u8>>;
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

/* zpaq
 *
 * Rabin derivative
 *
 *
 *
 unsigned h=0;  // rolling hash for finding fragment boundaries
 while (true) {
     c=in.get();
     if (c==o1[c1])
         h=(h+c+1)*314159265u;
     else
         h=(h+c+1)*271828182u;
     o1[c1]=c;
     c1=c;
     if (fragment<=22 && h<(1u<<(22-fragment)))
         break;
 }
 *
 */

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

/**
 * zpaq - a splitter used in go 'dedup' and zpaq that does not require looking back in the source
 *        data to update
 *
 * PDF: ??
 *
 * Note: go-dedup & zpaq calculate the relationship between their parameters slightly differently.
 * We support both of these (via the seperate with_*() constructors, but it'd be nice to clarify
 * why they differ and what affect the differences have.
 *
 * References:
 *   http://encode.ru/threads/456-zpaq-updates?p=45192&viewfull=1#post45192
 *   https://github.com/klauspost/dedup/blob/master/writer.go#L668
 *      'zpaqWriter'
 *   https://github.com/zpaq/zpaq/blob/master/zpaq.cpp
 *
 *
 * Parameters:
 *  fragment (aka average_size_base_2): average size = 2**fragment KiB
 *      in Zpaq (the compressor), this defaults to 6
 *  min_size, max_size: additional bounds on the blocks. Not technically needed for the algorithm
 *      to function
 *
 *  In Zpaq-compressor, min & max size are calculated using the fragment value
 *  In go's dedup, fragment is calculated using a min & max size
 *
 * In-block state:
 *  hash: u32, current hash
 *  last_byte: u8, previous byte read
 *  predicted_byte: array of 256 u8's.
 *
 * Between-block state:
 *  None
 */
#[derive(Debug, Clone)]
pub struct Zpaq
{
    /* FIXME: layout optimization? Is that even needed in rust? */
    range: Range<usize>,
    fragment: u8,
    max_hash: u32,
}

impl Zpaq {
    /* this is taken from go-dedup */
    fn fragment_ave_from_max(max: usize) -> u8
    {
        /* TODO: convert this to pure integer math */
        (max as f64 / (64f64 * 64f64)).log2() as u8
    }

    /* these are based on the zpaq (not go-dedup) calculations */
    fn fragment_ave_from_range(range: &Range<usize>) -> u8
    {
        let v = match range.last {
            Bound::Included(i) => i,
            Bound::Excluded(i) => i - 1,
            Bound::Unbounded => {
                /* try to guess based on first */
                64 * match range.first {
                    Bound::Included(i) => i,
                    Bound::Excluded(i) => i + 1,
                    Bound::Unbounded => {
                        /* welp, lets use the default */
                        return 6;
                    }
                }
            }
        };

        Self::fragment_ave_from_max(v)
    }

    /* these are based on the zpaq (not go-dedup) calculations */
    fn range_from_fragment_ave(fragment_ave: u8) -> Range<usize>
    {
        assert!(fragment_ave <= 22);
        Range::from_inclusive(64<<fragment_ave..8128<<fragment_ave)
    }

    fn range_from_max(max: usize) -> Range<usize>
    {
        Range::from_inclusive(max/64..max)
    }

    fn max_hash_from_fragment_ave(fragment_ave: u8) -> u32
    {
        1 << (22 - fragment_ave)
        /*
         * go-dedup does this:
         * (22f64 - fragment_ave).exp2() as u32
         *
         * Which should be equivalent to the integer math above (which is used by zpaq).
         */
    }


    /**
     * Create a splitter using the range of output block sizes.
     *
     * The average block size will be the max block size (if any) divided by 4, using the same
     * algorithm to calculate it as go-dedup.
     */
    pub fn with_range(range: Range<usize>) -> Self
    {
        let f = Self::fragment_ave_from_range(&range);
        Self::with_average_and_range(f, range)
    }

    /**
     * Create a splitter using the defaults from Zpaq (the compressor) given a average size is base
     * 2 (zpaq argument "-fragment")
     */
    pub fn with_average_size(average_size_base_2: u8) -> Self
    {
        let r = Self::range_from_fragment_ave(average_size_base_2);
        Self::with_average_and_range(average_size_base_2, r)
    }

    /**
     * Use the defaults from go-dedup to generate a splitter given the max size of a split.
     *
     * The average block size will be the max block size (if any) divided by 4, using the same
     * algorithm to calculate it as go-dedup.
     */
    pub fn with_max_size(max: usize) -> Self
    {
        Self::with_average_and_range(
            Self::fragment_ave_from_max(max),
            Self::range_from_max(max)
        )
    }

    /**
     * Create a splitter with control of all parameters
     *
     * All the other constructors use this internally
     */
    pub fn with_average_and_range(average_size_base_2: u8, range: Range<usize>) -> Self
    {
        Zpaq {
            range: range,
            fragment: average_size_base_2,
            max_hash: Self::max_hash_from_fragment_ave(average_size_base_2),
        }
    }

    /**
     * Create a splitter using the defaults from Zpaq (the compressor)
     *
     * Average size is 65536 bytes (64KiB), max is 520192 bytes (508KiB), min is 4096 bytes (4KiB)
     */
    pub fn new() -> Self
    {
        Self::with_average_size(6)
    }

    fn average_block_size(&self) -> usize
    {
        /* I don't know If i really trust this, do some more confirmation */
        1024 << self.fragment
    }

    fn split_here(&self, hash: u32, index: usize) -> bool
    {
        (hash < self.max_hash && !self.range.under_min(&index))
            || self.range.exceeds_max(&index)
    }

    pub fn split<'a, 'b>(&'a self, data: &'b [u8]) -> (&'b[u8], &'b[u8])
    {
        let mut s = ZpaqHash::new();
        let mut l = 0;
        for (i, &v) in data.iter().enumerate() {
            if self.split_here(s.feed(v), i + 1) {
                l = i + 1;
                break;
            }
        }

        data.split_at(l)
    }

    fn next_iter<'a, T: Iterator<Item=u8>>(&'a self, iter: T) -> Option<Vec<u8>>
    {
        let a = self.average_block_size();
        /* FIXME: ideally we'd allocate enough capacity to contain a large percentage of the
         * blocks. Just doing average probably will net us ~50% of blocks not needing additional
         * allocation. We really need to know the PDF (and standard-deviation) to make a better
         * prediction here. That said, even with additional data, this is a trade off with extra
         * space consumed vs number of allocations/reallocations
         */
        let mut w = Vec::with_capacity(a + a / 2);
        let mut s = ZpaqHash::new();
        for v in iter {
            w.push(v);
            if self.split_here(s.feed(v), w.len()) {
                return Some(w)
            }
        }

        if w.is_empty() {
            None
        } else {
            Some(w)
        }
    }

    pub fn into_slices<'a>(self, data: &'a [u8]) -> ZpaqSplit<'a, Zpaq>
    {
        ZpaqSplit::from(self, data)
    }

    pub fn as_slices<'a>(&'a self, data: &'a [u8]) -> ZpaqSplit<'a, &Zpaq>
    {
        ZpaqSplit::from(self, data)
    }

    pub fn into_vecs<'a, T: Iterator<Item=u8>>(self, data: T) -> ZpaqVecs<T, Zpaq>
    {
        ZpaqVecs::from(self, data)
    }

    pub fn as_vecs<'a, T: Iterator<Item=u8>>(&'a self, data: T) -> ZpaqVecs<T, &Zpaq>
    {
        ZpaqVecs::from(self, data)
    }
}

/**
 * The rolling hash component of the zpaq splitter
 */
struct ZpaqHash {
    pub hash: Wrapping<u32>,
    pub last_byte: u8,
    pub predicted_byte: [u8;256],
}

impl ZpaqHash {
    #[inline]
    pub fn new() -> Self {
        ZpaqHash {
            hash: Wrapping(0),
            last_byte: 0,
            predicted_byte: [0;256]
        }
    }

    /*
     * we can only get away with this because Zpaq doesn't need to look at old data to make it's
     * splitting decision, it only examines it's state + current value (and the state is
     * relatively large, but isn't a window into past data).
     */
    #[inline]
    pub fn feed(&mut self, c: u8) -> u32
    {
        self.hash = if c == self.predicted_byte[self.last_byte as usize] {
            (self.hash + Wrapping(c as u32) + Wrapping(1)) * Wrapping(314159265)
        } else {
            (self.hash + Wrapping(c as u32) + Wrapping(1)) * Wrapping(271828182)
        };

        self.predicted_byte[self.last_byte as usize] = c;
        self.last_byte = c;
        self.hash.0
    }
}

#[derive(Debug, Clone)]
pub struct ZpaqSplit<'a, T: Borrow<Zpaq> + 'a> {
    parent: T,
    d: &'a [u8],
}

impl<'a, T: Borrow<Zpaq> + 'a> ZpaqSplit<'a, T> {
    pub fn from(i: T, d : &'a [u8]) -> Self
    {
        ZpaqSplit {
            parent: i,
            d: d,
        }
    }
}

impl<'a, T: Borrow<Zpaq> + 'a> Iterator for ZpaqSplit<'a, T> {
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

#[derive(Debug, Clone)]
pub struct ZpaqVecs<T, P: Borrow<Zpaq>> {
    parent: P,
    d: T,
}

impl<'a, T: 'a, P: Borrow<Zpaq> + 'a> ZpaqVecs<T, P> {
    pub fn from(i: P, d: T) -> Self
    {
        ZpaqVecs {
            parent: i,
            d: d,
        }
    }
}

impl<'a, T: Iterator<Item=u8> + 'a, P: Borrow<Zpaq> + 'a> Iterator for ZpaqVecs<T, P> {
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

/*
 * rsync
 * Efficient Algorithms for Sorting and Synchronization
 * https://www.samba.org/~tridge/phd_thesis.pdf
 */

/**
 * 'Rsyncable' is used by the gzip rsyncable patch (still not merged, but widely distributed) as
 * well as the rsyncrypto project to split the unerlying content into variable sized blocks prior
 * to applying a filter (compression and/or encryption) to the blocks, which the intent of allowing
 * the resulting filtered data to be more easily propogated via rsync.
 *
 * No maximum block size is provided.
 * No minimum block size is provided.
 *
 * PDF of block sizes: ???
 *
 * Note that the defacto-standard parameters allow a slightly more efficient check for a block
 * split (by replacing a modulus with a bitwise-and). This impl currently doesn't allow that
 * optimization even if you provide appropriate parameters (we need type-level integers for that).
 *
 * Parameters:
 *  window-len: The maximum number of bytes to be examined when deciding to split a block.
 *              set to 8192 by default in gzip-rsyncable & rsyncrypto)
 *  modulus:    set to half of window-len (so, 4096) in gzip-rsyncable & rsyncrypto.
 *
 * In-block state:
 *  window of window-len bytes (use of the iterator interface means we also track more bytes than
 *      this)
 *  sum (u64)
 *
 * Between-block state:
 *  none
 *
 * References:
 *  http://rsyncrypto.lingnu.com/index.php/Algorithm
 *
 *
 * S(n) = sum(c_i, var=i, top=n, bottom=n-8196)
 * A(n) = S(n) / 8192
 * H(n) = S(n) mod 4096
 *
 * Trigger splits when H(n) == 0
 *
 * FIXME:
 *  Operating using iterators (like this) generally means we'll end up copying the data a number of
 *  times (not ideal). The interface may be adjusted (or an additional one provided) in the future
 *  to avoid performing the extra copies by working with an underlying slice directly.
 */
#[derive(Clone, Debug)]
pub struct Rsyncable {
    /*
     * TODO: if we can avoid loading entire files into memory, this could be u64
     */
    window_len: usize,
    modulus: u64,
}

impl Splitter for Rsyncable {
    fn split<'a, 'b>(&'a self, data: &'b [u8]) -> (&'b[u8], &'b[u8])
    {
        let mut accum = Wrapping(0u64);

        let mut l = 0;
        for (i, &v) in data.iter().enumerate() {
            if i >= self.window_len {
                accum = accum - Wrapping(data[i - self.window_len] as u64);
            }
            accum = accum + Wrapping(v as u64);

            if (accum % Wrapping(self.modulus)).0 == 0 {
                l = i + 1;
                break;
            }
        }

        data.split_at(l)
    }

    fn next_iter<'a, T: Iterator<Item=u8>>(&'a self, iter: T) -> Option<Vec<u8>>
    {
        let mut accum = Wrapping(0u64);

        let a = self.window_len + self.window_len / 2;
        let mut data = Vec::with_capacity(a);
        for (i, v) in iter.enumerate() {
            data.push(v);

            if i >= self.window_len {
                accum = accum - Wrapping(data[i - self.window_len] as u64);
            }
            accum = accum + Wrapping(v as u64);

            if (accum % Wrapping(self.modulus)).0 == 0 {
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

impl Rsyncable {
    pub fn new() -> Rsyncable
    {
        Self::with_window_and_modulus(8192, 4096)
    }

    pub fn with_window_and_modulus(window: usize, modulus: u64) -> Rsyncable
    {
        Rsyncable { window_len: window, modulus: modulus }
    }
}

#[derive(Debug, Clone)]
pub struct SplitterSplit<'a, T: Splitter + 'a> {
    parent: T,
    d: &'a [u8],
}

impl<'a, T: Splitter> SplitterSplit<'a, T> {
    pub fn from(i: T, d : &'a [u8]) -> Self
    {
        SplitterSplit {
            parent: i,
            d: d,
        }
    }
}

impl<'a, T: Splitter> Iterator for SplitterSplit<'a, T> {
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

#[cfg(all(feature = "nightly", test))]
/* 8 MiB */
const BENCH_BYTES : usize = 1024 * 1024 * 8;

#[cfg(all(feature = "nightly", test))]
const BENCH_RANGE : Range<usize> = Range { first: Bound::Unbounded, last: Bound::Unbounded };

#[cfg(feature = "nightly")]
#[bench]
fn bench_rsyncable_vecs (b: &mut test::Bencher) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut d = vec![0u8; BENCH_BYTES];
    b.iter(|| {
        rng.fill_bytes(&mut d);
        let s = Rsyncable::new().into_vecs(d.iter().cloned());
        for _ in s {}
    })
}

#[cfg(feature = "nightly")]
#[bench]
fn bench_rsyncable_slices (b: &mut test::Bencher) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut d = vec![0u8; BENCH_BYTES];
    b.iter(|| {
        rng.fill_bytes(&mut d);
        let s = Rsyncable::new().into_slices(&d[..]);
        for _ in s {}
    })
}

#[cfg(feature = "nightly")]
#[bench]
fn bench_zpaq (b: &mut test::Bencher) {
    bench::split_histogram(b, BENCH_BYTES, module_path!(), |data| {
        let z = Zpaq::with_range(BENCH_RANGE);
        let mut c = &data[..];
        Box::new(move || {
            let (a, b) = z.split(c);
            if b.is_empty() || a.is_empty() {
                None
            } else {
                c = b;
                Some(b.len() as u64)
            }
        })
    });
}


#[cfg(feature = "nightly")]
#[bench]
fn bench_zpaq_iter_slice(b: &mut test::Bencher) {
    bench::split_histogram(b, BENCH_BYTES, "zpaq_iter_slice", |data| {
        let zb : Zpaq = Zpaq::with_range(BENCH_RANGE);
        let mut z = zb.into_slices(data);
        Box::new(move || {
            z.next().map(|x| x.len() as u64)
        })
    })
}

#[cfg(feature = "nightly")]
#[bench]
fn bench_zpaq_iter_vec(b: &mut test::Bencher) {
    bench::split_histogram(b, BENCH_BYTES, module_path!(), |data| {
        let z = Zpaq::with_range(BENCH_RANGE);
        let mut zi = z.into_vecs(data.iter().cloned());
        Box::new(move || {
            zi.next().map(|x| x.len() as u64)
        })
    })
}

