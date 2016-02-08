
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

    /* state, mut */
    chunk_end: u64,
    sum : u64,
}

/*
 * Track state while searching for a single block
 */
struct RsyncableChunkScan {
    /* mutable state */
    sum : u64,
    window: Vec<u8>,
    window_start: usize,

    /* parameters, from outer */
    window_len: usize,
}

impl RsyncableChunkScan {
    pub fn new(window: usize) -> RsyncableChunkScan {
        RsyncableChunkScan {
            window: vec![],
            sum: 0,
            window_len: window,
            window_start: 0,
        }
    }

    pub fn feed(&mut self, new: u8) -> Option<Vec<u8>>{
        if (self.window.len() < self.window_len) {
            self.sum += new;
            return None;
        }

        self.sum -= self.window[
    }
}
    fn roll(&mut self, val: A) -> bool {
        if (start < self.window_len) {
            for i in range(start, self.window_len) {
                if i == start + num {
                    return;
                }
                self.sum += window[i];
            }

            num -= self.window_len - start
        }

        for i in range(start, start + num) {
            self.sum += window[i];
            self.sum -= window[i - self.window_len];

            if self.chunk_end == -1 && rsync_sum_match(self.sum) {
                self.chunk_end = i;
            }
        }
    }

impl<A, T: Intertor<Item=A>> Iterator for Rsyncable<A, T>
{
    type Item = Vec<A>;
    fn next(&mut self) -> Option<Self::Item> {
        let window = vec![];

        loop {
            match self.inner.next() {
                Some(v) => {
                    window += 
                }
                None => {
                    return Some(window);
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


struct Stupid {
    
}

#[test]
fn it_works() {
}
*/
