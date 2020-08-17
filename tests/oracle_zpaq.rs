#![cfg(feature = "zpaq-broken")]
// cdchunking doesn't impliment zpaq exactly correctly

use hash_roll::{ChunkIncr, ToChunkIncr};
use quickcheck::quickcheck;
use rand::RngCore;
use rand_pcg::Pcg64;

fn test_data(seed: u128, size: usize) -> Vec<u8> {
    let mut fill_rng = Pcg64::new(seed, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);
    let mut buf = vec![0u8; size];
    fill_rng.fill_bytes(&mut buf);
    buf
}

quickcheck! {
    fn zpaq_eq_cdchunking(xs: Vec<u8>) -> bool {
        let m1 = hash_roll::zpaq::Zpaq::with_average_and_range_and_m(13, .., 123_456_791, 123_456_791 * 2);
        let m2 = cdchunking::Chunker::new(cdchunking::ZPAQ::new(13));

        let mut i1 = m1.to_chunk_incr().iter_slices(&xs);
        let mut i2 = m2.slices(&xs);

        loop {
            let v1 = i1.next();
            let v2 = i2.next();

            if v1 != v2 {
                return false;
            }

            if v1.is_none() {
                return true;
            }
        }
    }
}

fn c(xs: &[u8]) {
    let m1 =
        hash_roll::zpaq::Zpaq::with_average_and_range_and_m(13, .., 123_456_791, 123_456_791 * 2);
    let m2 = cdchunking::Chunker::new(cdchunking::ZPAQ::new(13));

    let mut i1 = m1.to_chunk_incr().iter_slices(&xs);
    let mut i2 = m2.slices(&xs);

    let mut i = 0;
    loop {
        let v1 = i1.next();
        let v2 = i2.next();

        if v1 != v2 {
            panic!("i: {}, hr_v: {:?} != cdc_v : {:?}", i, v1, v2);
        }

        if v1.is_none() {
            break;
        }

        i += 1;
    }
}

#[test]
fn zpaq_cdchunking_cuts() {
    let buf = test_data(0, 8192 * 4);
    let m: Vec<usize> = cdchunking::Chunker::new(cdchunking::ZPAQ::new(13))
        .slices(&buf)
        .map(|v| v.len())
        .collect();
    assert_eq!(&m[..], &[10785, 6329, 1287, 860, 4716, 7419],);
}

mod oracle_zpaq {
    use super::c;
    #[test]
    fn t1() {
        c(&[
            25, 5, 82, 84, 53, 94, 27, 24, 98, 47, 7, 7, 6, 34, 60, 98, 20, 64, 17, 5, 62, 40, 94,
            79, 33, 1, 0,
        ])
    }

    #[test]
    fn t2() {
        c(&[
            25, 5, 82, 84, 53, 94, 27, 24, 98, 47, 7, 7, 6, 34, 60, 98, 20, 64, 17, 5, 62, 40, 94,
            79, 33, 1,
        ])
    }

    #[test]
    fn t3() {
        c(&[
            25, 5, 82, 84, 53, 94, 27, 24, 98, 47, 7, 7, 6, 34, 60, 98, 20, 64, 17, 5, 62, 40, 94,
            79, 33, 0, 0,
        ])
    }
}
