use std::num::Wrapping;
use super::ChunkIncr;
use std::fmt;

const BLOBBITS: u8 = 13;
const BLOBSIZE: u32 = 1 << (BLOBBITS as u32);

const WINDOW_BITS: u8 = 6;
const WINDOW_SIZE: usize = 1 << (WINDOW_BITS as usize);

const ROLLSUM_CHAR_OFFSET: usize = 31;

/// Rolling sum used by [`Bup`] for splitting
///
/// - https://github.com/bup/bup/blob/0ab7c3a958729b4723e6fe254da771aff608c2bf/lib/bup/bupsplit.c
/// - https://github.com/bup/bup/blob/0ab7c3a958729b4723e6fe254da771aff608c2bf/lib/bup/bupsplit.h
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollSum {
    s1: Wrapping<u32>,
    s2: Wrapping<u32>,
    window_len: usize,
}

impl RollSum {
    pub fn with_window(window_size: usize) -> Self {
        let ws = Wrapping(window_size as u32);
        Self {
            s1: ws * Wrapping(ROLLSUM_CHAR_OFFSET as u32),
            s2: ws * (ws-Wrapping(1)) * Wrapping(ROLLSUM_CHAR_OFFSET as u32),
            window_len: window_size as usize,
        }
    }
}

impl Default for RollSum {
    fn default() -> Self {
        Self::with_window(WINDOW_SIZE)
    }
}

/// Incrimental instance of [`RollSum`]
///
/// Performance note: Bup's Roll sum algorithm requires tracking the entire window. As a result,
/// this includes a circular buffer which all inputs are copied through. If your use case allows
/// it, use the non-incrimental variant for improved performance.
// Note: the [u8;WINDOW_SIZE] blocks most derives (Clone, Debug, PartialEq, Eq) due to the lack of
// impls for [u8;WINDOW_SIZE]. Explore the potential for using a custom derive plugin/macro to
// generate these impls more easily.
pub struct RollSumIncr {
    s1: Wrapping<u32>,
    s2: Wrapping<u32>,

    /// window offset
    wofs: Wrapping<usize>,
    window: Box<[u8]>,
}

impl Default for RollSumIncr {
    fn default() -> Self {
        From::from(RollSum::default())
    }
}

impl fmt::Debug for RollSumIncr {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error>
    {
        f.debug_struct("RollSumIncr")
            .field("s1", &self.s1)
            .field("s2", &self.s2)
            .field("window", &::fmt_extra::Hs(&self.window[..]))
            .field("wofs", &self.wofs)
            .finish()
    }
}

impl Clone for RollSumIncr {
    fn clone(&self) -> Self {
        RollSumIncr {
            window: self.window.clone(),
            ..*self
        }
    }
}

impl PartialEq for RollSumIncr {
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

impl Eq for RollSumIncr {}

impl From<RollSum> for RollSumIncr {
    fn from(params: RollSum) -> Self {
        Self {
            s1: params.s1,
            s2: params.s2,
            window: vec![0;params.window_len].into_boxed_slice(),
            wofs: Wrapping(0),
        }
    }
}

impl RollSumIncr {
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

impl ChunkIncr for RollSumIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize>
    {
        for (i, &v) in data.iter().enumerate() {
            self.roll_byte(v);
            if self.at_split() {
                return Some(i+1);
            }
        }

        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rollsum::Engine;
    use rand::RngCore;

    #[test]
    fn rs() {
        let mut m = RollSumIncr::default();
        m.roll_byte(3);
        assert_eq!(m.digest(), 130279491);
    }

    #[test]
    fn compare_rollsum() {
        let mut m1 = RollSumIncr::default();
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
        use super::ChunkIncr;
        let mut m1 = RollSumIncr::default();
        let mut m2 = rollsum::Bup::default();

        let mut r = rand::thread_rng();
        let mut b = [0u8;2048];

        r.fill_bytes(&mut b);

        let mut x = &b[..];
        loop {
            let v1 = m1.push(&x);
            let v2 = m2.find_chunk_edge(&x);
            assert_eq!(v1, v2);

            match v1 {
                None => break,
                Some(v) => { x = &x[v..]; }
            }
        }
    }
}


