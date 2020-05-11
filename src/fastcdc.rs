use std::num::Wrapping;
use super::{Split2};

// these masks are taken from the paper and could be adjusted/adjustable.
const MASK_S: u64 = 0x0003590703530000;
//const MASK_A: u64 = 0x0000d90303530000;
const MASK_L: u64 = 0x0000d90003530000;

// again, might be useful to allow tuning here.
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
    fp: Wrapping<u64>,
}

impl<'a> ::std::fmt::Debug for FastCdc8<'a> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error>
    {
        f.debug_struct("FastCdc8")
            .field("gear", &"[...]")
            .field("l", &self.l)
            .field("fp", &self.fp.0)
            .finish()
    }
}

impl<'a> Clone for FastCdc8<'a> {
    fn clone(&self) -> Self {
        FastCdc8 {
            ..*self
        }
    }
}

impl<'a> PartialEq for FastCdc8<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.fp == other.fp &&
            self.l == other.l &&
            {
                for i in 0..self.gear.len() {
                    if self.gear[i] != other.gear[i] {
                        return false;
                    }
                }

                true
            }
    }
}

impl<'a> Eq for FastCdc8<'a> {}

impl<'a> FastCdc8<'a> {
    fn reset(&mut self)
    {
        self.l = 0;
        self.fp = Wrapping(0);
    }
}

impl<'a> Default for FastCdc8<'a> {
    fn default() -> Self {
        FastCdc8 {
            fp: Wrapping(0),
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

        if gl <= MIN_SIZE {
            // No split, no processing of data, but we've "consumed" the bytes.
            self.l = gl;
            return 0;
        }

        // skip elements prior to MIN_SIZE
        let ibase = if gi <= MIN_SIZE {
            let skip = MIN_SIZE - gi;
            src = &src[skip as usize..];
            gi += skip;
            skip
        } else {
            0
        } as usize;

        let mut src = src.iter().enumerate();
        let mut fp = self.fp;

        for (i, &v) in &mut src {
            gi += 1;

            if gi >= NORMAL_SIZE {
                // go to next set of matches
                break;
            }

            fp = (fp << 1) + Wrapping(self.gear[v as usize]);
            if (fp.0 & MASK_S) == 0{
                self.reset();
                return ibase + i;
            }
        }

        for (i, &v) in &mut src {
            gi += 1;

            if gi >= MAX_SIZE {
                // no match found, emit fixed match at MAX_SIZE
                self.reset();
                return ibase + i;
            }

            fp = (fp << 1) + Wrapping(self.gear[v as usize]);
            if (fp.0 & MASK_L) == 0 {
                self.reset();
                return ibase + i;
            }
        }

        // no match, but not at MAX_SIZE yet, so store context for next time.
        self.fp = fp;
        self.l = gi;

        0
    }
}

/// A 1-buffer implimentation of FastCDC8KB designed to match the reference pseudocode
#[cfg(test)]
fn fast_cdc_8kb(src: &[u8]) -> usize
{
    let mut fp = Wrapping(0);
    let mut n = src.len();
    let mut normal_size = NORMAL_SIZE as usize;
    if n <= (MIN_SIZE as usize) {
        // Diverge from the reference here:
        //  return 0 to indicate no split found rather than src.len()
        return 0;
    }

    if n >= (MAX_SIZE as usize){
        n = MAX_SIZE as usize;
    } else if n <= normal_size {
        normal_size = n;
    }

    for i in (MIN_SIZE as usize)..normal_size {
        fp = (fp << 1) + Wrapping(super::gear_table::GEAR_64[src[i] as usize]);
        if (fp.0 & MASK_S) == 0 {
            return i;
        }
    }

    for i in normal_size..n {
        fp = (fp << 1) + Wrapping(super::gear_table::GEAR_64[src[i] as usize]);
        if (fp.0 & MASK_L) == 0 {
            return i;
        }
    }

    // Diverge from the reference here:
    //  return 0 to indicate no split found rather than src.len()
    0
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug,Clone,PartialEq,Eq)]
    struct Vec8K {
        data: Vec<u8>
    }

    impl quickcheck::Arbitrary for Vec8K {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            // FIXME: the intention is to raise this >8KB, but that makes the tests take far too
            // long to run.
            let l = 1 * 1024 + g.size();

            let mut d = vec![0;l];

            g.fill_bytes(&mut d[..]);

            Vec8K {
                data: d
            }
        }

        fn shrink(&self) -> Box<dyn Iterator<Item=Self>> {
            // use the normal Vec shrinkers
            let chain = self.data.shrink().map(|x| Vec8K { data: x });
            Box::new(chain)
        }
    }

    fn oracle_1(d: Vec8K) -> bool {
        let mut cdc = FastCdc8::default();
        let v1 = fast_cdc_8kb(&d.data[..]);
        let v2 = cdc.push(&d.data[..]);

        v1 == v2
    }

    fn oracle_1_test(data: Vec<u8>) {
        let mut cdc = FastCdc8::default();
        let v1 = fast_cdc_8kb(&data[..]);
        let v2 = cdc.push(&data[..]);

        assert_eq!(v1, v2);
    }

    #[test]
    fn o1_empty() {
        oracle_1_test(vec![0]);
    }

    #[test]
    fn o1_qc() {
        quickcheck::quickcheck(oracle_1 as fn(Vec8K) -> bool);
    }

    #[test]
    fn o1_8k1() {
        use rand::RngCore;
        let mut d = Vec::with_capacity(8*1024*1024 + 1);
        let c = d.capacity();
        unsafe { d.set_len(c) };
        let mut rng = ::rand::thread_rng();
        rng.fill_bytes(&mut d);
        oracle_1_test(d);
    }

    #[test]
    fn feed_until_5_chunks() {
        use rand::RngCore;
        let mut cdc = FastCdc8::default();
        let mut ct = 0;
        let mut rng = ::rand::thread_rng();
        let mut d = [0u8;256];
        rng.fill_bytes(&mut d);
        loop {
            rng.fill_bytes(&mut d);
            let mut data = &d[..];
            loop {
                let p = cdc.push(&data[..]);
                println!("p: {}, cdc: {:?}", p, cdc);

                if p == 0 || p == data.len() {
                    break;
                } else {
                    ct += 1;
                    if ct > 5 {
                        return;
                    }
                    data = &data[p..];
                }
            }
        }
    }
}
