use std::num::Wrapping;
use super::{Split2};

const MASK_S: u64 = 0x0003590703530000;
const MASK_A: u64 = 0x0000d90303530000;
const MASK_L: u64 = 0x0000d90003530000;

const MIN_SIZE: u64 = 2 * 1024; // 2KB
const MAX_SIZE: u64 = 64 * 1024; // 64KB
const NORMAL_SIZE: u64 = 8 * 1024; // 8KB

/// Intermediate state for FastCDC8KB while it is processing data
///
/// Reference:
///
///  https://www.usenix.org/system/files/conference/atc16/atc16-paper-xia.pdf
pub struct FastCdc8<'a> {
    /// A map from every byte value to a "random" 64 bit number.
    ///
    /// Fixed configuration.
    gear: &'a [u64;256],

    /// Number of bytes we've "examined"
    ///
    /// varying state.
    l: u64,

    /// Current fingerprint
    ///
    /// varying state.
    fp: u64,
}

impl<'a> FastCdc8<'a> {
    fn reset(&mut self)
    {
        self.l = 0;
        self.fp = 0;
    }
}

impl<'a> Default for FastCdc8<'a> {
    fn default() -> Self {
        FastCdc8 {
            fp: 0,
            l: 0,
            gear: &super::gear_table::GEAR_64,
        }
    }
}

impl<'a> Split2 for FastCdc8<'a> {
    fn push(&mut self, mut src: &[u8]) -> usize {
        // global index
        let mut gi = self.l;
        // global length
        let gl = src.len() as u64 + gi;

        let mut normal_size = NORMAL_SIZE;
        if gl <= MIN_SIZE {
            // No split
            return 0;
        }

        if gl >= MAX_SIZE {
            // only examine up to max size
            // XXX: this is likely wrong, need to use src.len()
            src = &src[..(MAX_SIZE - gi) as usize];
        } else if gl <= normal_size {
            // XXX: confirm @wl and not @src.len() is right here
            normal_size = gl;
        }

        let mut fp = self.fp;
        for i in ((MIN_SIZE - gi) as usize)..((normal_size - gi) as usize) {
            // FIXME: `i` is wrong here due to us being incremental
            fp = (fp << 1) + self.gear[src[i] as usize];
            if (fp & MASK_S) == 0{
                self.reset();
                return i;
            }
        }

        let large_start = (normal_size - gi) as usize;
        for i in large_start..src.len() {
            fp = (fp << 1) + self.gear[src[i] as usize];
            if (fp & MASK_L) == 0 {
                self.reset();
                return i;
            }
        }

        0
    }
}

fn fast_cdc_8kb(src: &[u8]) -> usize
{
    let mut fp = 0;
    let mut n = src.len();
    let mut normal_size = NORMAL_SIZE as usize;
    if n <= (MIN_SIZE as usize) {
        return n;
    }

    if n >= (MAX_SIZE as usize){
        n = MAX_SIZE as usize;
    } else if n <= normal_size {
        normal_size = n;
    }

    for i in (MIN_SIZE as usize)..normal_size {
        fp = (fp << 1) + super::gear_table::GEAR_64[src[i] as usize];
        if (fp & MASK_S) == 0 {
            return i;
        }
    }

    for i in normal_size..n {
        fp = (fp << 1) + super::gear_table::GEAR_64[src[i] as usize];
        if (fp & MASK_L) == 0 {
            return i;
        }
    }

    n
}

#[cfg(test)]
mod test {
    #[test]
    fn fc8() {
        
    }
}
