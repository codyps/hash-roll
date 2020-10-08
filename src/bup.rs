use crate::{Chunk, ChunkIncr, ToChunkIncr};
use std::fmt;
use std::num::Wrapping;

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
    window_len: usize,
}

impl ToChunkIncr for RollSum {
    type Incr = RollSumIncr;

    fn to_chunk_incr(&self) -> Self::Incr {
        self.into()
    }
}

impl RollSum {
    pub fn with_window(window_len: usize) -> Self {
        Self { window_len }
    }
}

impl Chunk for RollSum {
    type SearchState = RollSumSearchState;

    fn to_search_state(&self) -> Self::SearchState {
        self.into()
    }

    fn find_chunk_edge(
        &self,
        state: &mut Self::SearchState,
        data: &[u8],
    ) -> (Option<usize>, usize) {
        for i in state.offset..data.len() {
            let a = data[i];
            let d = if i >= self.window_len {
                data[i - self.window_len]
            } else {
                0
            };

            state.state.add(self.window_len, d, a);

            if state.state.at_split() {
                state.reset(self);
                return (Some(i + 1), i + 1);
            }
        }

        // keep k elements = discard all but k
        let discard_ct = data.len().saturating_sub(self.window_len);
        state.offset = data.len() - discard_ct;
        (None, discard_ct)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollSumState {
    // NOTE: in bup, these are `unsigned`, but masking indicates they'll end up being used as
    // u16's. In `librsync`, these are `uint_fast16_t`, which end up being u32 on most platforms.
    // Both only require `u16` values to be represented. We use `u32` here as it's likely to be
    // somewhat more performant, but this should be examined
    s1: Wrapping<u32>,
    s2: Wrapping<u32>,
}

impl From<&RollSum> for RollSumState {
    fn from(s: &RollSum) -> Self {
        let ws = Wrapping(s.window_len as u32);
        // NOTE: bup uses this initialization, but librsync uses zeros.
        //
        // I believe the idea is to allow a slightly different implimentation of the "setup"
        // portion of the processing (ie: before the window is filled)
        Self {
            s1: ws * Wrapping(ROLLSUM_CHAR_OFFSET as u32),
            s2: ws * (ws - Wrapping(1)) * Wrapping(ROLLSUM_CHAR_OFFSET as u32),
        }
    }
}

impl RollSumState {
    fn reset(&mut self, params: &RollSum) {
        *self = params.into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollSumSearchState {
    state: RollSumState,
    offset: usize,
}

impl From<&RollSum> for RollSumSearchState {
    fn from(s: &RollSum) -> Self {
        Self {
            state: s.into(),
            offset: 0,
        }
    }
}

impl RollSumSearchState {
    fn reset(&mut self, params: &RollSum) {
        self.offset = 0;
        self.state.reset(params);
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
#[derive(Clone, PartialEq, Eq)]
pub struct RollSumIncr {
    state: RollSumState,

    /// window offset
    wofs: Wrapping<usize>,
    window: Box<[u8]>,
}

impl fmt::Debug for RollSumIncr {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
        f.debug_struct("RollSumIncr")
            .field("state", &self.state)
            .field("window", &::fmt_extra::Hs(&self.window[..]))
            .field("wofs", &self.wofs)
            .finish()
    }
}

impl From<&RollSum> for RollSumIncr {
    fn from(params: &RollSum) -> Self {
        Self {
            state: params.into(),
            window: vec![0; params.window_len].into_boxed_slice(),
            wofs: Wrapping(0),
        }
    }
}

impl Default for RollSumIncr {
    fn default() -> Self {
        (&RollSum::default()).into()
    }
}

impl RollSumState {
    fn add(&mut self, window_len: usize, drop: u8, add: u8) {
        let d = Wrapping(drop as u32);
        self.s1 += Wrapping(add as u32);
        self.s1 -= d;
        self.s2 += self.s1;
        self.s2 -= Wrapping(window_len as u32) * (d + Wrapping(ROLLSUM_CHAR_OFFSET as u32));
    }

    fn digest(&self) -> u32 {
        (self.s1.0 << 16) | (self.s2.0 & 0xffff)
    }

    fn at_split(&self) -> bool {
        (self.digest() & (BLOBSIZE - 1)) == (BLOBSIZE - 1)
    }
}

impl RollSumIncr {
    pub fn digest(&self) -> u32 {
        self.state.digest()
    }

    fn add(&mut self, drop: u8, add: u8) {
        self.state.add(self.window.len(), drop, add);
    }

    pub fn roll_byte(&mut self, ch: u8) {
        let w = self.window[self.wofs.0];
        self.add(w, ch);
        self.window[self.wofs.0] = ch;
        self.wofs = Wrapping((self.wofs + Wrapping(1)).0 & (self.window.len() - 1));
    }

    #[cfg(test)]
    pub(crate) fn roll(&mut self, data: &[u8]) {
        for &i in data.iter() {
            self.roll_byte(i);
        }
    }

    /*
    fn sum(data: &[u8]) -> u32 {
        let mut x = Self::default();
        x.roll(data);
        x.digest()
    }
    */

    pub fn at_split(&self) -> bool {
        self.state.at_split()
    }
}

impl ChunkIncr for RollSumIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        for (i, &v) in data.iter().enumerate() {
            self.roll_byte(v);
            if self.at_split() {
                return Some(i + 1);
            }
        }

        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::RngCore;
    use rollsum::Engine;

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
        let mut b = [0u8; 2048];

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
        let mut b = [0u8; 2048];

        r.fill_bytes(&mut b);

        let mut x = &b[..];
        loop {
            let v1 = m1.push(&x);
            let v2 = m2.find_chunk_edge(&x);
            assert_eq!(v1, v2.map(|x| x.0));

            match v1 {
                None => break,
                Some(v) => {
                    x = &x[v..];
                }
            }
        }
    }
}
