use std::marker::PhantomData;
use std::num::Wrapping;
use super::Splitter;

const BLOBBITS: u8 = 13;
const BLOBSIZE: u32 = 1 << (BLOBBITS as u32);

const WINDOW_BITS: u8 = 6;
const WINDOW_SIZE: usize = 1 << (WINDOW_BITS as usize);

const ROLLSUM_CHAR_OFFSET: usize = 31;

/**
 * Rolling sum used in Bup. This is based on the one in librsync.
 *
 * A new instance is used for each block splitting. In other words: after finding the first edge, a
 * new `RollSum` is instantiated to find the next edge.
 */
// Note: the [u8;WINDOW_SIZE] blocks most derives (Clone, Debug, PartialEq, Eq) due to the lack of
// impls for [u8;WINDOW_SIZE]. Explore the potential for using a custom derive plugin/macro to
// generate these impls more easily.
pub struct RollSum {
    s1: Wrapping<u32>,
    s2: Wrapping<u32>,

    /// window offset
    wofs: Wrapping<usize>,
    window: [u8; WINDOW_SIZE],
}

impl ::std::fmt::Debug for RollSum {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        f.debug_struct("RollSum")
            .field("s1", &self.s1)
            .field("s2", &self.s2)
            .field("window", &::fmt_extra::Hs(&self.window[..]))
            .field("wofs", &self.wofs)
            .finish()
    }
}

impl Clone for RollSum {
    fn clone(&self) -> Self {
        RollSum {
            ..*self
        }
    }
}

impl PartialEq for RollSum {
    fn eq(&self, other: &Self) -> bool {
        self.s1 == other.s1 &&
            self.s2 == other.s2 &&
            self.wofs == other.wofs &&
            {
                for i in 0..self.window.len() {
                    if self.window[i] != other.window[i] {
                        return false;
                    }
                }

                true
            }
    }
}

impl Eq for RollSum {}

impl RollSum {
    pub fn digest(&self) -> u32 {
        (self.s1.0 << 16) | (self.s2.0 & 0xffff)
    }

    pub fn add(&mut self, drop: u8, add: u8) {
        let d = Wrapping(drop as u32);
        self.s1 += Wrapping(add as u32);
        self.s1 -= d;
        self.s2 += self.s1;
        self.s2 -= Wrapping(self.window.len() as u32) * (d + Wrapping(ROLLSUM_CHAR_OFFSET as u32));
    }

    pub fn roll_byte(&mut self, ch: u8) {
        let w = self.window[self.wofs.0];
        self.add(w, ch);
        self.window[self.wofs.0] = ch;
        self.wofs = Wrapping((self.wofs + Wrapping(1)).0 & (self.window.len() - 1));
    }

    pub fn roll(&mut self, data: &[u8]) {
        for &i in data.iter() {
            self.roll_byte(i);
        }
    }

    pub fn sum(data: &[u8]) -> u32 {
        let mut x = Self::default(); 
        x.roll(data);
        x.digest()
    }

    pub fn at_split(&self) -> bool {
        (self.digest() & (BLOBSIZE-1)) == (BLOBSIZE-1)
    }
}

impl Default for RollSum {
    fn default() -> Self {
        let ws = Wrapping(WINDOW_SIZE as u32);
        RollSum {
            s1: ws * Wrapping(ROLLSUM_CHAR_OFFSET as u32),
            s2: ws * (ws-Wrapping(1)) * Wrapping(ROLLSUM_CHAR_OFFSET as u32),
            window: [0;WINDOW_SIZE],
            wofs: Wrapping(0),
        }
    }
}

#[derive(Clone,Debug, Eq, PartialEq)]
pub struct Bup {
    _x: PhantomData<()>
}

impl Default for Bup {
    fn default() -> Self {
        Bup { _x: PhantomData }
    }
}

impl Splitter for Bup {
    fn find_chunk_edge(&self, data: &[u8]) -> usize {
        let mut r = RollSum::default();

        for (i, &v) in data.iter().enumerate() {
            r.roll_byte(v);
            if r.at_split() {
                return i+1;
            }
        }

        return 0;
    }

    fn next_iter<'a, T: Iterator<Item=u8>>(&'a self, iter: T) -> Option<Vec<u8>>
    {
        let mut r = RollSum::default();
        let a = r.window.len() + r.window.len() / 2;
        let mut data = Vec::with_capacity(a);
        for v in iter {
            data.push(v);
            r.roll_byte(v);
            if r.at_split() {
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

/*
 for (count = 0; count < len; count++)
    {
	rollsum_roll(&r, buf[count]);
	if ((r.s2 & (BUP_BLOBSIZE-1)) == ((~0) & (BUP_BLOBSIZE-1)))
	{
	    if (bits)
	    {
		unsigned rsum = rollsum_digest(&r);
		*bits = BUP_BLOBBITS;
		rsum >>= BUP_BLOBBITS;
		for (*bits = BUP_BLOBBITS; (rsum >>= 1) & 1; (*bits)++)
		    ;
	    }
	    return count+1;
	}
    }
return 0;
    }
}
*/

#[cfg(test)]
mod test {
    use super::*;
    use super::super::*;
    use rollsum::Engine;
    use rand::Rng;

    #[test]
    fn rs() {
        let mut m = RollSum::default();
        m.roll_byte(3);
        assert_eq!(m.digest(), 130279491);
    }

    #[test]
    fn compare_rollsum() {
        let mut m1 = RollSum::default();
        let mut m2 = rollsum::Bup::default();

        assert_eq!(m1.digest(), m2.digest());

        m1.roll_byte(4);
        m2.roll_byte(4);
        
        assert_eq!(m1.digest(), m2.digest());

        m1.roll_byte(18);
        m2.roll_byte(18);
        
        assert_eq!(m1.digest(), m2.digest());

        let mut r = rand::thread_rng();
        let mut b = [0u8;2048];

        r.fill_bytes(&mut b);

        for (i, &v) in b.iter().enumerate() {
            m1.roll_byte(v);
            m2.roll_byte(v);
            println!("i={}, v={}", i, v);
            assert_eq!(m1.digest(), m2.digest());
        }


        m1.roll(&b);
        m2.roll(&b);

        assert_eq!(m1.digest(), m2.digest());
    }

    #[test]
    fn compare_bup() {
        let m1 = Bup::default();
        let mut m2 = rollsum::Bup::default();

        let mut r = rand::thread_rng();
        let mut b = [0u8;2048];

        r.fill_bytes(&mut b);

        let v1 = m1.find_chunk_edge(&b);
        let v2 = m2.find_chunk_edge(&b).unwrap_or(0);


        assert_eq!(v1, v2);
    }
}


