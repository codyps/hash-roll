use std::num::Wrapping;
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

/// Cyclic polynomial hash (buzhash)
///
/// BuzHash is used in:
///   - [Borg](https://github.com/borgbackup/borg)
///   - [Attic](https://github.com/jborg/attic)
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct BuzHash {
    /// current value of the hash.
    ///
    /// Note: if we had a function `h()` to hash characters prior to
    /// entry, the size of this hash could be made larger.
    ///
    /// Mutable data.
    h: u8,

    /// number of characters to consider at once
    ///
    /// Immutable parameter.
    k: usize,
}

impl Default for BuzHash {
    fn default() -> Self {
        BuzHash::with_capacity(8)
    }
}

impl BuzHash {
    pub fn add(&mut self, drop: u8, add: u8) {
        let h = self.h.rotate_left(1);
        let drop = drop.rotate_left((self.k % 8) as u32);
        self.h = h ^ drop ^ add;
    }

    /// Create an instance with the given capacity (k).
    ///
    /// Capacity is the number of bytes that are taken into account for a given hash.
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0);
        BuzHash { h: 0, k: capacity }
    }

    /// Current hash value
    pub fn hash(&self) -> u8 {
        self.h
    }

    pub fn capacity(&self) -> usize {
        self.k
    }


}

/// Self-contained buzhash which buffers it's window of values internally
///
/// Note that this will be less efficient than using [`BuzHash`] on a slice directly,
/// but may be more convenient.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct BuzHashBuf {
    bh: BuzHash,
    buf: Box<[u8]>,
    buf_idx: Wrapping<usize>,
}

impl BuzHashBuf {
    pub fn with_capacity(capacity: usize) -> Self {
        BuzHashBuf::from(BuzHash::with_capacity(capacity))
    }

    pub fn push_byte(&mut self, val: u8) {
        let o = self.buf[self.buf_idx.0];
        self.bh.add(o, val);
        self.buf[self.buf_idx.0] = val;
        self.buf_idx += Wrapping(1);
        self.buf_idx %= Wrapping(self.bh.capacity());
    }

    pub fn push(&mut self, data: &[u8]) {
        for &v in data {
            self.push_byte(v)
        }
    }

    /// Current hash value
    pub fn hash(&self) -> u8 {
        self.bh.h
    }

    pub fn capacity(&self) -> usize {
        self.bh.k
    }

    /// Return the index in `data` immeidately following the hash matching.
    ///
    /// Note that you can call this multiple times to examine "subsequent" `data` slices, but the
    /// index returned will always refer to the current `data` slice.
    pub fn find_match(&mut self, other_hash: u8, data: &[u8]) -> usize {
        for (i, &v) in data.iter().enumerate() {
            self.push_byte(v);
            if self.hash() == other_hash {
                return i+1;
            }
        }

        0
    }
}

impl From<BuzHashBuf> for Box<[u8]> {
    fn from(x: BuzHashBuf) -> Self {
        x.buf
    }
}

impl Default for BuzHashBuf {
    fn default() -> Self {
        From::from(BuzHash::default())
    }
}

impl From<BuzHash> for BuzHashBuf {
    fn from(x: BuzHash) -> Self {
        BuzHashBuf {
            buf: vec![0;x.capacity()].into_boxed_slice(),
            bh: x,
            buf_idx: Wrapping(0)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn bh_eq() {
        let x = [92, 6, 28, 35, 68, 82, 35, 71, 34, 19, 9, 45, 97, 17, 11, 6, 53, 39, 93, 49, 29, 17, 37, 6, 39];
        let mut b = BuzHashBuf::default();

        b.push(&x[..]);
        assert_eq!(b.hash(), 4);
    }

    #[test]
    fn bh_match() {
        let x = [92, 6, 28, 35, 68, 82, 35, 71, 34, 19, 9, 45, 97, 17, 11, 6, 53, 39, 93, 49, 29, 17, 37, 6, 39];
        let mut b = BuzHashBuf::from(BuzHash::with_capacity(4));
        let h = {
            let mut m = b.clone();
            m.push(&[9,45,97,17]);
            m.hash()
        };

        assert_eq!(b.find_match(h, &x[..]), 14);
    }
}
