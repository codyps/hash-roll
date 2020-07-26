use hash_roll::{Chunk, ChunkIncr};
use rand::RngCore;
use rand_pcg::Pcg64;

fn cut_test<C: Chunk>(seed: u128, chunker: C, expected_splits: &[usize]) {
    let mut fill_rng = Pcg64::new(seed, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);
    let mut buf = [0u8; 8192 * 4];
    fill_rng.fill_bytes(&mut buf);

    // Note: this doesn't validate SearchState at all
    let mut splits = Vec::with_capacity(expected_splits.len());
    {
        let mut buf = &buf[..];
        loop {
            match chunker.find_chunk_edge(None, &buf[..]) {
                Ok(split_point) => {
                    splits.push(split_point);
                    buf = &buf[split_point..];
                }
                Err(_) => {
                    break;
                }
            }
        }
    }

    // Note: this is only basic equivalance checking via byte-at-a-time. More full equivalance
    // checking will be done via quickcheck tests.
    let mut incr_splits = Vec::with_capacity(expected_splits.len());
    {
        let mut incr = chunker.incrimental();
        let buf = &buf[..];
        let mut last_split = 0;
        for (i, v) in buf.iter().enumerate() {
            match incr.push(&[*v]) {
                Some(_split_point) => {
                    let sp = i + 1;
                    incr_splits.push(sp - last_split);
                    last_split = sp;
                }
                None => {}
            }
        }
    }

    assert_eq!(&splits[..], &incr_splits[..]);
    assert_eq!(expected_splits, &splits[..]);
}

#[test]
fn mii_cuts_1() {
    cut_test(
        0,
        hash_roll::mii::Mii::default(),
        &[
            1212, 40, 261, 1548, 1881, 312, 2043, 285, 1062, 677, 542, 1473, 303, 172, 318, 839,
            2560, 3242, 396, 202, 123, 898, 2454, 544, 3541, 571, 483, 383, 103, 2629, 929, 47,
            524,
        ],
    );
}

/*
#[test]
fn bup_cuts_1() {
    cut_test(0,
        hash_roll::bup::RollSum::default(),
        &[]
    )
}
*/

#[test]
fn rsyncable_cuts_1() {
    cut_test(
        0,
        hash_roll::rsyncable::Rsyncable::default(),
        &[2941, 2077, 5263, 7263, 392, 4371, 5204],
    )
}

#[test]
fn rsyncable_cuts_2() {
    // chosen so we check window removal
    cut_test(
        2,
        hash_roll::rsyncable::Rsyncable::default(),
        &[9277, 2758, 3074, 7415, 3579, 4141],
    )
}

#[test]
fn buzhash_cuts_1() {
    cut_test(
        0,
        hash_roll::buzhash::BuzHash::new_nom(0),
        &[7839, 1342, 9741, 2703, 875, 3549],
    )
}

#[test]
fn zpaq_cuts_0() {
    // These match edges from Zpaq 7.15 (with modification to print the fragment sizes).
    //     
    //     cargo run --example generate-test-data 0 >test_data_0.bin
    //     zpaq a foo.zpaq ~/p/hash-roll/test_data_0.bin -fragment 3
    cut_test(
        0,
        hash_roll::zpaq::Zpaq::with_average_size(3),
        &[10785, 6329, 1287, 860, 4716, 7419],
    )
}

#[test]
fn zpaq_cuts_3() {
    // These match edges from Zpaq 7.15 (with modification to print the fragment sizes).
    //     
    //     cargo run --example generate-test-data 3 >test_data_3.bin
    //     zpaq a foo.zpaq ~/p/hash-roll/test_data_0.bin -fragment 3
    cut_test(
        3,
        hash_roll::zpaq::Zpaq::with_average_size(3),
        &[16353, 2334, 970, 5326, 1557],
    )
}
