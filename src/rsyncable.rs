use std::num::Wrapping;
use super::{Splitter};

/*
 * rsync
 * Efficient Algorithms for Sorting and Synchronization
 * https://www.samba.org/~tridge/phd_thesis.pdf
 */

/**
 * Window-based splitter using a simple accumulator & modulus hash.
 *
 * Used by the gzip rsyncable patch (still not merged, but widely distributed) as
 * well as the rsyncrypto project to split the unerlying content into variable sized blocks prior
 * to applying a filter (compression and/or encryption) to the blocks, which the intent of allowing
 * the resulting filtered data to be more easily propogated via rsync.
 *
 *  - No maximum block size is provided.
 *  - No minimum block size is provided.
 *
 * PDF of block sizes: ???
 *
 * Note that the defacto-standard parameters allow a slightly more efficient check for a block
 * split (by replacing a modulus with a bitwise-and). This impl currently doesn't allow that
 * optimization even if you provide appropriate parameters (we need type-level integers for that).
 *
 * Parameters:
 * 
 *  - window-len: The maximum number of bytes to be examined when deciding to split a block.
 *              set to 8192 by default in gzip-rsyncable & rsyncrypto)
 *  - modulus:    set to half of window-len (so, 4096) in gzip-rsyncable & rsyncrypto.
 *
 * In-block state:
 *  - window of window-len bytes (use of the iterator interface means we also track more bytes than
 *      this)
 *  - sum (u64)
 *
 * Between-block state:
 * 
 * - none
 *
 * References:
 * 
 * - http://rsyncrypto.lingnu.com/index.php/Algorithm
 *
 * S(n) = sum(c_i, var=i, top=n, bottom=n-8196)
 * 
 * A(n) = S(n) / 8192
 * 
 * H(n) = S(n) mod 4096
 *
 * Trigger splits when H(n) == 0
 *
 */
#[derive(Clone, Debug)]
pub struct Rsyncable {
    /*
     * TODO: if we can avoid loading entire files into memory, this could be u64
     */
    window_len: usize,
    modulus: u64,
}

struct HashState {
    accum: Wrapping<u64>
}

impl Default for HashState {
    fn default() -> Self {
        HashState { accum: Wrapping(0) }
    }
}

impl HashState {
    pub fn add(&mut self, data: &[u8], base: &Rsyncable, i: usize, v: u8) -> bool {
        if i >= base.window_len {
            self.accum -= Wrapping(data[i - base.window_len] as u64);
        }
        self.accum += Wrapping(v as u64);
        (self.accum % Wrapping(base.modulus)).0 == 0
    }
}

impl Splitter for Rsyncable {
    fn find_chunk_edge<'a, 'b>(&'a self, data: &'b [u8]) -> usize
    {
        let mut hs = HashState::default();

        let mut l = 0;
        for (i, &v) in data.iter().enumerate() {
            if hs.add(data, self, i, v) {
                l = i + 1;
                break
            }
        }

        l
    }

    fn next_iter<'a, T: Iterator<Item=u8>>(&'a self, iter: T) -> Option<Vec<u8>>
    {
        let mut hs = HashState::default();

        let a = self.window_len + self.window_len / 2;
        let mut data = Vec::with_capacity(a);
        for (i, v) in iter.enumerate() {
            data.push(v);

            if hs.add(&data, self, i, v) {
                return Some(data)
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
    pub fn with_window_and_modulus(window: usize, modulus: u64) -> Rsyncable
    {
        Rsyncable { window_len: window, modulus: modulus }
    }
}

impl Default for Rsyncable {
    fn default() -> Self {
        Self::with_window_and_modulus(8192, 4096)
    }
}

