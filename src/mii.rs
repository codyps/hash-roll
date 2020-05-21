/// https://ieeexplore.ieee.org/abstract/document/8752387
#[derive(Debug)]
pub struct Mii {
    /// After this many increments, split the file
    w: u64,

    /// previous examined byte, if any
    prev: u8,

    /// number of times a byte was greater than the previous value
    increment: u64,
}

impl Mii {
    /// Create a new splitter with parameter `w`
    ///
    /// `w` is the number of "increments" (positive changes in byte value) after which we split the
    /// input
    pub fn with_w(w: u64) -> Self {
        Self {
            w,
            // we use 0xff to ensure that the first examined byte does not trigger an increment
            prev: 0xff,
            increment: 0,
        }
    }
}

impl super::Chunker for Mii {
    fn push(&mut self, input: &[u8]) -> Option<usize> {
        for (i, b) in input.iter().cloned().enumerate() {
            if b > self.prev {
                self.increment += 1;
                if self.increment == self.w {
                    // this is a split
                    self.increment = 0;
                    self.prev = 0;
                    return Some(i);
                }
            } else {
                self.increment = 0;
            }
            self.prev = b;
        }

        None
    }
}
