#![cfg(feature = "seqcdc")]
//! Sequence-based CDC with content-triggered skipping.
//! Reference: <https://arxiv.org/abs/2505.21194>.
use crate::{ChunkIncr, ChunkSizes};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Increasing,
    Decreasing,
}
#[derive(Debug, Clone)]
pub struct SeqCdc {
    sizes: ChunkSizes,
    sequence: usize,
    trigger: usize,
    skip: usize,
    direction: Direction,
}
impl SeqCdc {
    /// Sequence is a byte count; each adjacent pair must be strictly ordered.
    /// Equal bytes end the run without counting as an opposing slope.
    /// Panics unless 2<=sequence<=256, min>=sequence, and skip_trigger>0.
    pub fn new(
        sizes: ChunkSizes,
        sequence: usize,
        skip_trigger: usize,
        skip_size: usize,
        direction: Direction,
    ) -> Self {
        assert!((2..=256).contains(&sequence) && sizes.min() >= sequence && skip_trigger > 0);
        Self {
            sizes,
            sequence,
            trigger: skip_trigger,
            skip: skip_size,
            direction,
        }
    }
    fn state(&self) -> SeqCdcIncr {
        SeqCdcIncr {
            params: self.clone(),
            len: 0,
            previous: None,
            run: 0,
            opposite: 0,
            skip: self.sizes.min() - self.sequence,
        }
    }
}
impl Default for SeqCdc {
    fn default() -> Self {
        Self::new(
            ChunkSizes::new(4096, 8192, 65536),
            5,
            50,
            256,
            Direction::Increasing,
        )
    }
}
#[derive(Debug, Clone)]
pub struct SeqCdcIncr {
    params: SeqCdc,
    len: usize,
    previous: Option<u8>,
    run: usize,
    opposite: usize,
    skip: usize,
}
impl ChunkIncr for SeqCdcIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        let mut i = 0;
        while i < data.len() {
            if self.skip > 0 {
                let take = self
                    .skip
                    .min(data.len() - i)
                    .min(self.params.sizes.max() - self.len);
                self.skip -= take;
                self.len += take;
                i += take;
                self.previous = None;
                self.run = 0;
            } else {
                let b = data[i];
                i += 1;
                self.len += 1;
                if let Some(prev) = self.previous {
                    let good = match self.params.direction {
                        Direction::Increasing => b > prev,
                        Direction::Decreasing => b < prev,
                    };
                    if good {
                        self.run += 1;
                    } else {
                        self.run = 1;
                        if b != prev {
                            self.opposite += 1;
                            if self.opposite == self.params.trigger {
                                self.skip = self.params.skip;
                                self.run = 0;
                                self.opposite = 0;
                            }
                        }
                    }
                } else {
                    self.run = 1;
                }
                self.previous = (self.run != 0).then_some(b);
            }
            if self.len == self.params.sizes.max()
                || (self.len >= self.params.sizes.min() && self.run >= self.params.sequence)
            {
                *self = self.params.state();
                return Some(i);
            }
        }
        None
    }
}
impl_stream_chunker!(SeqCdc, SeqCdcIncr);
