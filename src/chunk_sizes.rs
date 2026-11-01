//! Validated byte-size limits shared by the newer chunkers.

/// Minimum, target, and inclusive maximum chunk lengths, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkSizes {
    min: usize,
    average: usize,
    max: usize,
}
impl ChunkSizes {
    /// Panics unless `0 < min <= average <= max`.
    pub fn new(min: usize, average: usize, max: usize) -> Self {
        assert!(
            min > 0 && min <= average && average <= max,
            "expected 0 < min <= average <= max"
        );
        Self { min, average, max }
    }
    pub fn min(self) -> usize {
        self.min
    }
    pub fn average(self) -> usize {
        self.average
    }
    pub fn max(self) -> usize {
        self.max
    }
}
impl Default for ChunkSizes {
    fn default() -> Self {
        Self::new(2048, 8192, 65536)
    }
}
