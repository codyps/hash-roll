#![cfg(feature = "gear64")]
//! 64-bit Gear fingerprints and CDC. The default table is shared with FastCDC.
use crate::{ChunkIncr, RollingHash};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Gear64 {
    table: Arc<[u64; 256]>,
    mask: u64,
}
impl Gear64 {
    /// Use a caller-provided byte table. Panics unless bits is in 1..=63.
    pub fn new(table: [u64; 256], bits: u8) -> Self {
        assert!((1..=63).contains(&bits));
        Self {
            table: Arc::new(table),
            mask: u64::MAX << (64 - bits),
        }
    }
    pub fn with_average_size_log2(bits: u8) -> Self {
        Self::new(crate::gear_table::GEAR_64, bits)
    }
    fn state(&self) -> Gear64Incr {
        Gear64Incr {
            params: self.clone(),
            hash: 0,
        }
    }
}
impl Default for Gear64 {
    fn default() -> Self {
        Self::with_average_size_log2(13)
    }
}
#[derive(Debug, Clone)]
pub struct Gear64Incr {
    params: Gear64,
    hash: u64,
}
impl RollingHash for Gear64Incr {
    type Digest = u64;
    fn roll_byte(&mut self, byte: u8) -> u64 {
        self.hash = self
            .hash
            .wrapping_shl(1)
            .wrapping_add(self.params.table[byte as usize]);
        self.hash
    }
    fn digest(&self) -> u64 {
        self.hash
    }
    fn reset(&mut self) {
        self.hash = 0;
    }
}
impl ChunkIncr for Gear64Incr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        for (i, &byte) in data.iter().enumerate() {
            if self.roll_byte(byte) & self.params.mask == 0 {
                self.reset();
                return Some(i + 1);
            }
        }
        None
    }
}
impl_stream_chunker!(Gear64, Gear64Incr);
