use super::ChunkIncr;
use std::fmt;
use std::num::Wrapping;

/// Gear Content Defined Chunking using 32bit expansion.
///
/// Reference:
///
///  Xia, W., Jiang, H., Feng, D., Tian, L., Fu, M., and Zhou, Y. Ddelta: A dedulication-inspired
///  fast delta compression approach. Performance Evaluation 79 (2014), 258-271.
///
///  http://wxia.hustbackup.cn/pub/DElta-PEVA-2014.pdf
pub struct Gear32<'a> {
    /// A mask with an appropriate number of bits set for the desired average chunk size.
    ///
    /// fixed configuration.
    mask: u32,

    /// value to match (fp & mask) against.
    ///
    /// fixed configuration.
    xxx: u32,

    /// A table to map bytes to 32bit values
    ///
    /// fixed configuration.
    gear: &'a [u32; 256],

    /// current fingerprint/hash
    ///
    /// varying state.
    fp: Wrapping<u32>,
}

impl<'a> fmt::Debug for Gear32<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Gear32")
            .field("mask", &self.mask)
            .field("xxx", &self.xxx)
            .field("gear", &&self.gear[..])
            .field("fp", &self.fp)
            .finish()
    }
}

impl<'a> ChunkIncr for Gear32<'a> {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        let mut fp = self.fp;
        for (i, v) in data.iter().enumerate() {
            fp = (fp << 1) + Wrapping(self.gear[*v as usize]);
            if fp.0 & self.mask == self.xxx {
                self.fp = Wrapping(0);
                return Some(i);
            }
        }

        // no match
        self.fp = fp;

        None
    }
}

fn msb_mask(log2: usize) -> u32 {
    // at least 1 bit & not all the bits
    // FIXME: probably could relax those requirements with better math.
    //debug_assert!(log2 > 0);
    //debug_assert!(log2 < 32);

    ((1 << log2) - 1) << (32 - log2)
}

impl<'a> Gear32<'a> {
    /// Create a gear chunker which emits blocks with average size `(1<<average_size_log2)`, (or:
    /// `2**average_size_log2`
    fn with_average_size_log2(average_size_log2: usize) -> Self {
        Gear32 {
            fp: Wrapping(0),
            mask: msb_mask(average_size_log2),
            xxx: 0,
            gear: &super::gear_table::GEAR_32,
        }
    }
}

impl<'a> Default for Gear32<'a> {
    fn default() -> Self {
        // 8KB average size
        Self::with_average_size_log2(13)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn mm() {
        use super::msb_mask;
        assert_eq!(0b1 << 31, msb_mask(1));
        assert_eq!(0b11 << 30, msb_mask(2));
        assert_eq!(0b111 << 29, msb_mask(3));
    }
}
