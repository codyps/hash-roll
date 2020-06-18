use hash_roll::fastcdc::FastCdcIncr;
use hash_roll::ChunkIncr;

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
    let mut cdc = FastCdcIncr::default();
    let v1 = fast_cdc_8kb(&d.data[..]);
    let v2 = cdc.push(&d.data[..]);

    v1 == v2.unwrap_or(0)
}

fn oracle_1_test(data: &[u8]) {
    let mut cdc = FastCdcIncr::default();
    let v1 = fast_cdc_8kb(&data[..]);
    let v2 = cdc.push(&data[..]).unwrap_or(0);

    assert_eq!(v1, v2);
}

#[test]
fn o1_empty() {
    oracle_1_test(&vec![0]);
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
    for _ in 0..10 {
        rng.fill_bytes(&mut d);
        oracle_1_test(&d);
    }
}

#[test]
fn feed_until_5_chunks() {
    use rand::RngCore;
    let mut cdc = FastCdcIncr::default();
    let mut ct = 0;
    let mut rng = ::rand::thread_rng();
    let mut d = [0u8;256];
    rng.fill_bytes(&mut d);
    loop {
        rng.fill_bytes(&mut d);
        let mut data = &d[..];
        loop {
            let p = cdc.push(&data[..]);
            println!("p: {:?}, cdc: {:?}", p, cdc);

            if p == None || p.unwrap() == data.len() {
                break;
            } else {
                ct += 1;
                if ct > 5 {
                    return;
                }
                data = &data[p.unwrap()..];
            }
        }
    }
}

/// A 1-buffer implimentation of FastCDC8KB designed to match the reference pseudocode
fn fast_cdc_8kb(src: &[u8]) -> usize
{
    use std::num::Wrapping;
    use hash_roll::gear_table::GEAR_64;
    // these masks are taken from the paper and could be adjusted/adjustable.
    const MASK_S: u64 = 0x0003590703530000;
    //const MASK_A: u64 = 0x0000d90303530000;
    const MASK_L: u64 = 0x0000d90003530000;
    const MIN_SIZE: u64 = 2 * 1024; // 2KB
    const MAX_SIZE: u64 = 64 * 1024; // 64KB
    const NORMAL_SIZE: u64 = 8 * 1024; // 8KB

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
        fp = (fp << 1) + Wrapping(GEAR_64[src[i] as usize]);
        if (fp.0 & MASK_S) == 0 {
            return i;
        }
    }

    for i in normal_size..n {
        fp = (fp << 1) + Wrapping(GEAR_64[src[i] as usize]);
        if (fp.0 & MASK_L) == 0 {
            return i;
        }
    }

    // Diverge from the reference here:
    //  return 0 to indicate no split found rather than src.len()
    0
}

