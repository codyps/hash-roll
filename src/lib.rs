
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

pub mod circ;
pub mod window;
pub mod slice;

use slice::SliceExt;

/**
 * Data is fed in, and blocks of data are emitted
 */
trait Block<T> {
    /*
     * FIXME: allow something more general for [T]
     * FIXME: return value should probably not be a Vec<>
     * FIXME: can we have any errors?
     */
    fn feed<'a>(&mut self, input: &'a [T]) -> Option<Vec<T>>;
    fn finish(self) -> Option<Vec<T>>;
}

/*
trait BlockFast<T>
    where Self::SplitIter: Iterator<Item=[u8]>
{
    /* XXX: consider using 'self' directly */
    /*
     * last element of vector is either empty (in the case where we triggered a split on the last
     * element) or contains elements in the case where no split was triggered.
     *
     * Returns an iterator
     */
    type SplitIter;
    type SplitParam;
    fn split<'a>(&self, param: SplitParam) -> SplitIter;
}
*/

/* zpaq
 *
 * Rabin derivative
 *
 * http://encode.ru/threads/456-zpaq-updates?p=45192&viewfull=1#post45192
 * https://github.com/klauspost/dedup/blob/master/writer.go#L668
 *  'zpaqWriter'
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


#[derive(Debug, Clone)]
pub struct Zpaq
{
    max_size: usize,
    min_size: usize,
    max_hash: u32,
}

struct ZpaqState {
    pub hash: u32,
    pub last_byte: u8,
    pub predicted_byte: [u8;256],
}

impl Zpaq {
    pub fn with_max_size(max_size: usize) -> Self
    {
        Self::with_max_and_min_size(max_size, max_size / 4)
    }

    pub fn with_max_and_min_size(max_size: usize, min_size: usize) -> Self
    {
        Zpaq {
            max_size: max_size,
            min_size: min_size,
            max_hash: Self::max_hash_from_max_size(max_size),
        }
    }

    pub fn max_hash_from_max_size(max_size: usize) -> u32
    {
        /* TODO: convert this to pure integer math */
        let fragment = (max_size as f64 / (64f64 * 64f64)).log2();
        //1 << (22 - fragment)
        (22f64 - fragment).exp2() as u32
    }

    /*
    pub fn split<'a, 'b, P>(&'a self, data: &'b [u8]) -> slice::SplitOn<u8, P>
    {

        data.split(
    }
    */
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
pub struct Rsyncable<T: Iterator<Item=u8>> {
    window_len: usize,
    modulus: u64,
    inner : T,
}

/*
 * Track state while searching for a single block
 */
struct RsyncableChunkScan {
    /* mutable state */
    sum : Wrapping<u64>,
    modulus: u64,
    window: window::Buf<u8>,
}

impl RsyncableChunkScan {
    pub fn new(window: usize, modulus: u64) -> RsyncableChunkScan {
        RsyncableChunkScan {
            window: window::Buf::new(window),
            sum: Wrapping(0),
            modulus: modulus,
        }
    }

    fn rsync_sum_match(&self) -> bool {
        //((self.sum) & (Wrapping(self.window.limit() as u64) - Wrapping(1))) == Wrapping(0)
        (self.sum.0 % self.modulus) == 0
    }

    pub fn feed(&mut self, new: u8) -> Option<Vec<u8>>{
        match self.window.push(new) {
            Some(old) => { self.sum = self.sum - Wrapping(*old as u64) },
            None => {}
        }

        self.sum = self.sum + Wrapping(new as u64);

        if self.rsync_sum_match() {
            // FIXME: ideally, this would be into_vec(), and the old self would be gone.
            Some(self.window.to_vec())
        } else {
            None
        }
    }

    /// Extract the currently queued internal bytes
    ///
    /// Should be used when we run out of input to split
    pub fn into_vec(self) -> Vec<u8> {
        self.window.into_vec()
    }

    pub fn len(&self) -> usize {
        self.window.len()
    }
}

impl<T: Iterator<Item=u8>> Iterator for Rsyncable<T>
{
    type Item = Vec<T::Item>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut s = RsyncableChunkScan::new(self.window_len, self.modulus);

        loop {
            match self.inner.next() {
                Some(v) => {
                    match s.feed(v) {
                        Some(x) => {
                            return Some(x);
                        },
                        None => {}
                    }
                },
                None => {
                    if s.len() > 0 {
                        return Some(s.into_vec());
                    } else {
                        return None
                    }
                }
            }
        }
    }
}

impl<T: Iterator<Item=u8>> Rsyncable<T> {
    pub fn from(window: usize, modulus: u64, inner: T) -> Self {
        Rsyncable {
            window_len: window,
            inner: inner,
            modulus: modulus,
        }
    }
}

#[test]
fn test_rsyncable() {
    use std::collections::HashSet;

    let d1 = b"hello, this is some bytes";
    let mut d2 = d1.clone();
    d2[4] = ':' as u8;

    let b1 = Rsyncable::from(4, 8, d1.iter().cloned());
    let b2 = Rsyncable::from(4, 8, d2.iter().cloned());

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
