
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

/* TODO: bupsplit
 */

/* TODO: rsyncable (rsyncrypto)
 *
 * Efficient Algorithms for Sorting and Synchronization
 * https://www.samba.org/~tridge/phd_thesis.pdf
 *
 * S(n) = sum(c_i, var=i, top=n, bottom=n-8196)
 * A(n) = S(n) / 8192
 * H(n) = S(n) mod 4096
 *
 * Trigger = sum(P_i, var=i, top=n, bottom=n-8196)
 *
 * State:
 *  - 
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

pub mod circ;

/*
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
 * Instead of a feed() mechanism, we could use an iterator mechanism.
 *
 * ie: an iterator that takes an iterator and emits blocks
 */

struct Rsyncable<A, T: Iterator<Item=A>> {
    /* parameters, const */
    /*
     * gzip.rsync.patch uses a value of 8192, states it must be smaller than MAX_DIST
     */
    window_len: u64,

    inner : T,
}

/*
 * Track state while searching for a single block
 */
struct RsyncableChunkScan {
    /* mutable state */
    sum : u64,
    window: circ::Buf<u8>,
}

impl RsyncableChunkScan {
    pub fn new(window: usize) -> RsyncableChunkScan {
        RsyncableChunkScan {
            window: circ::Buf::new(window),
            sum: 0,
        }
    }

    fn rsync_sum_match(&self) -> bool {
        ((self.sum) & (self.window.len() - 1)) == 0
    }

    pub fn feed(&mut self, new: u8) -> Option<Vec<u8>>{
        match self.window.push(new) {
            Some(old) => { self.sum -= old },
            None => {}
        }

        self.sum += old;


        if self.rsync_sum_match() {
            Some(self.window.to_vec())
        } else {
            None
        }
    }

    /// Extract the currently queued internal bytes
    ///
    /// Should be used when we run out of input to split
    pub fn into_vec(self) -> Vec<u8> {
        self.window.to_vec()
    }
}

impl<A, T: Intertor<Item=A>> Iterator for Rsyncable<A, T>
{
    type Item = Vec<A>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut s = RsyncableChunkScan::new(self.window_len);

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
                    return Some(s.into_vec());
                }
            }
        }
    }
}

impl<A, T: Iterator<Item=A>> Rsyncable<A, T> {
    pub fn from(window: usize, inner: T) -> Self {
        Rsyncable {
            window_len: window,

            window: vec![],
            inner: inner,

            sum: 0,
            chunk_end: 0,
        }
    }
}
*/
