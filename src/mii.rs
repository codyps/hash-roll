use crate::ChunkIncr;


/// C. Zhang et al., "MII: A Novel Content Defined Chunking Algorithm for Finding Incremental Data
/// in Data Synchronization," in IEEE Access, vol. 7, pp. 86932-86945, 2019, doi:
/// 10.1109/ACCESS.2019.2926195.
///
/// https://ieeexplore.ieee.org/abstract/document/8752387
#[derive(Debug, Clone)]
pub struct Mii {
    w: u64,
}

impl Mii {
    /// Create a new splitter with parameter `w`
    ///
    /// `w` is the number of "increments" (positive changes in byte value) after which we split the
    /// input
    ///
    // TODO: determine distribution and expected size of chunks
    //
    // 1: P(curr > prev) = 0    (prev set to 0xff)
    // 2: P(curr > prev) = 0.5  (prev and curr assumed to be randomly distributed)
    // 3: P(curr > prev) |  t2 = ??? 
    //    P(curr > prev) | !t2 = ???
    pub fn with_w(w: u64) -> Self {
        Self {
            w,
        }
    }
}

impl Default for Mii {
    /// The window of 5 is used in the paper for the generated graphs
    ///
    /// It is compared against Rabin with a window of 7 and AE/LMC/RAM with a window of 700
    fn default() -> Self {
        Mii::with_w(5)
    }
}

impl crate::Chunk for Mii {
    type SearchState = MiiSearchState;
    type Incr = MiiIncr;

    fn find_chunk_edge(&self, state: Option<Self::SearchState>, data: &[u8]) -> Result<usize, Self::SearchState> {
        let mut state = match state {
            Some(s) => s,
            None => self.incrimental().into()
        };

        match state.push(data) {
            Some(v) => Ok(v),
            None => Err(state),
        }
    }

    fn incrimental(&self) -> Self::Incr {
        From::from(self.clone())
    }
}

#[derive(Debug)]
pub struct MiiSearchState {
    offset: usize,
    incr: MiiIncr,
}

impl From<MiiIncr> for MiiSearchState {
    fn from(incr: MiiIncr) -> Self {
        Self {
            offset: 0,
            incr,
        }
    }
}

impl MiiSearchState {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        let d = &data[self.offset..];
        assert!(data.len() >= self.offset);
        let r = self.incr.push(d).map(|x| x + self.offset);
        self.offset = data.len();
        r
    }
}

#[derive(Debug)]
pub struct MiiIncr {
    /// After this many increments, split the file
    w: u64,

    /// previous examined byte, if any
    prev: u8,

    /// number of times a byte was greater than the previous value
    increment: u64,
}

impl From<Mii> for MiiIncr {
    fn from(p: Mii) -> Self {
        MiiIncr {
            w: p.w,
            // we use 0xff to ensure that the first examined byte does not trigger an increment
            prev: 0xff,
            increment: 0,
        }
    }
}

impl ChunkIncr for MiiIncr {
    fn push(&mut self, input: &[u8]) -> Option<usize> {
        for (i, b) in input.iter().cloned().enumerate() {
            if b > self.prev {
                self.increment += 1;
                if self.increment == self.w {
                    // this is a split
                    self.increment = 0;
                    self.prev = 0;
                    return Some(i + 1);
                }
            } else {
                self.increment = 0;
            }
            self.prev = b;
        }

        None
    }
}
