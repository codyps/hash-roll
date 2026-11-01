#![cfg(feature = "leap")]
//! Leap-based CDC with the published 180-byte / 24-window judgment layout.
//! Reference: <https://doi.org/10.1109/MSST.2015.7208290>.
//! Matrix provenance: see THIRD-PARTY-NOTICES.
use crate::{ChunkIncr, ChunkSizes};
#[path = "leap_table.rs"]
mod table;
const HISTORY: usize = 204;
#[derive(Debug, Clone)]
pub struct Leap {
    sizes: ChunkSizes,
}
impl Leap {
    /// Panics if the minimum is smaller than the 204-byte judgment history.
    pub fn new(sizes: ChunkSizes) -> Self {
        assert!(sizes.min() >= HISTORY);
        Self { sizes }
    }
    fn state(&self) -> LeapIncr {
        LeapIncr {
            params: self.clone(),
            history: [0; HISTORY],
            pos: 0,
            len: 0,
            wait: 0,
        }
    }
}
impl Default for Leap {
    fn default() -> Self {
        Self::new(ChunkSizes::new(8192, 12288, 16384))
    }
}
#[derive(Debug, Clone)]
pub struct LeapIncr {
    params: Leap,
    history: [u8; HISTORY],
    pos: usize,
    len: usize,
    wait: usize,
}
impl LeapIncr {
    fn qualified(&self, window: usize) -> bool {
        let mut value = 0;
        for column in 0..5 {
            let age = window + 1 + column * 42;
            let byte = self.history[(self.pos + HISTORY - age) % HISTORY];
            value ^= table::EF_MATRIX[byte as usize][column];
        }
        value != 0
    }
}
impl ChunkIncr for LeapIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        for (i, &b) in data.iter().enumerate() {
            self.history[self.pos] = b;
            self.pos = (self.pos + 1) % HISTORY;
            self.len += 1;
            let mut cut = self.len == self.params.sizes.max();
            if self.len >= self.params.sizes.min() && !cut {
                if self.wait > 0 {
                    self.wait -= 1;
                } else {
                    cut = true;
                    for window in (2..24).chain(0..2) {
                        if !self.qualified(window) {
                            let leap = if window >= 2 {
                                24 - window
                            } else {
                                22 - window
                            };
                            self.wait = leap - 1;
                            cut = false;
                            break;
                        }
                    }
                }
            }
            if cut {
                *self = self.params.state();
                return Some(i + 1);
            }
        }
        None
    }
}
impl_stream_chunker!(Leap, LeapIncr);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Chunk, ToChunkIncr};
    #[test]
    fn leaps_preserve_first_satisfied_candidate() {
        let mut x = 137u64;
        let data: Vec<_> = (0..65536)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect();
        let c = Leap::new(ChunkSizes::new(204, 512, 1024));
        let mut start = 0;
        let mut expected = Vec::new();
        for end in 204..=data.len() {
            if end - start < 204 {
                continue;
            }
            let qualified = (0..24).all(|w| {
                (0..5).fold(0u8, |v, col| {
                    v ^ table::EF_MATRIX[data[end - w - 1 - col * 42] as usize][col]
                }) != 0
            });
            if qualified || end - start == 1024 {
                expected.push(end);
                start = end;
            }
        }
        let mut offset = 0;
        let actual: Vec<_> = c
            .to_chunk_incr()
            .iter_slices_strict(&data)
            .map(|chunk| {
                offset += chunk.len();
                offset
            })
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(c.chunks(&data).map(|b| b.len()).sum::<usize>(), data.len());
    }
}
