use super::ChunkIncr;
use std::fmt;
use std::num::Wrapping;

// these masks are taken from the paper and could be adjusted/adjustable.
const MASK_S: u64 = 0x0003590703530000;
//const MASK_A: u64 = 0x0000d90303530000;
const MASK_L: u64 = 0x0000d90003530000;

/// An instance of the "FastCDC" algorithm
///
/// Default parameters:
///  - Minimum chunk size: 2 KiB
///  - Maximum chunk size: 64 KiB
///  - Normal size: 8 KiB
///  - internal 64-bit gear table: [`super::gear_table::GEAR_64`]
///
/// Reference:
///  - https://www.usenix.org/system/files/conference/atc16/atc16-paper-xia.pdf
#[derive(Clone, Copy)]
pub struct FastCdc<'a> {
    gear: &'a [u64; 256],
    min_size: u64,
    max_size: u64,
    normal_size: u64,
}

impl<'a> PartialEq for FastCdc<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.min_size == other.min_size
            && self.max_size == other.max_size
            && self.normal_size == other.normal_size
            && &self.gear[..] == &other.gear[..]
    }
}

impl<'a> Eq for FastCdc<'a> {}

impl<'a> Default for FastCdc<'a> {
    fn default() -> Self {
        FastCdc {
            min_size: 2 * 1024,    // 2 KiB
            max_size: 64 * 1024,   // 64 KiB
            normal_size: 8 * 1024, // 8 KiB
            gear: &super::gear_table::GEAR_64,
        }
    }
}

impl<'a> fmt::Debug for FastCdc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FastCdc")
            .field("gear", &"[...]")
            .field("min_size", &self.min_size)
            .field("max_size", &self.max_size)
            .field("normal_size", &self.normal_size)
            .finish()
    }
}

impl<'a> FastCdc<'a> {
    /// Create a custom FastCDC instance
    pub fn new(gear: &'a [u64; 256], min_size: u64, normal_size: u64, max_size: u64) -> Self {
        Self {
            gear,
            min_size,
            max_size,
            normal_size,
        }
    }
}

/// FastCdcIncr provides an incrimental interface to `FastCdc`
///
/// This impl does not buffer data passing through it (the FastCDC algorithm does not require
/// look-back) making it very efficient.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FastCdcIncr<'a> {
    params: FastCdc<'a>,

    /// Number of bytes we've "examined"
    ///
    /// varying state.
    l: u64,

    /// Current fingerprint
    ///
    /// varying state.
    fp: Wrapping<u64>,
}

impl<'a> FastCdcIncr<'a> {
    /// Use custom parameters (different gear, different size)
    pub fn with_params(params: FastCdc<'a>) -> Self {
        Self {
            params,
            ..Default::default()
        }
    }

    fn reset(&mut self) {
        self.l = 0;
        self.fp = Wrapping(0);
    }
}

impl<'a> ChunkIncr for FastCdcIncr<'a> {
    fn push(&mut self, src: &[u8]) -> Option<usize> {
        // global start/index
        let mut gi = self.l;
        // global end
        let ge = src.len() as u64 + gi;

        if ge <= self.params.min_size {
            // No split, no processing of data, but we've "consumed" the bytes.
            self.l = ge;
            return None;
        }

        // skip elements prior to MIN_SIZE and track offset of new `src` in argument `src` for
        // return value
        let mut i = if gi <= self.params.min_size {
            let skip = self.params.min_size - gi;
            gi += skip;
            skip
        } else {
            0
        } as usize;

        let mut fp = self.fp;

        loop {
            if i >= src.len() {
                break;
            }
            if gi >= self.params.normal_size {
                // go to next set of matches
                break;
            }

            let v = src[i];
            fp = (fp << 1) + Wrapping(self.params.gear[v as usize]);
            if (fp.0 & MASK_S) == 0 {
                self.reset();
                return Some(i);
            }

            gi += 1;
            i += 1;
        }

        loop {
            if gi >= self.params.max_size {
                // no match found, emit fixed match at MAX_SIZE
                self.reset();
                return Some(i);
            }
            if i >= src.len() {
                break;
            }

            let v = src[i];
            fp = (fp << 1) + Wrapping(self.params.gear[v as usize]);
            if (fp.0 & MASK_L) == 0 {
                self.reset();
                return Some(i);
            }

            gi += 1;
            i += 1;
        }

        // no match, but not at MAX_SIZE yet, so store context for next time.
        self.fp = fp;
        self.l = ge;

        None
    }
}
