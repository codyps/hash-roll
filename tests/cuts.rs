use hash_roll::{Chunk, ChunkIncr, ToChunkIncr};
use rand::RngCore;
use rand_pcg::Pcg64;

fn cut_test_incr<C: ChunkIncr>(
    seed: u128,
    size: usize,
    chunker: C,
    expected_splits: &[usize],
) {
    let mut fill_rng = Pcg64::new(seed, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);
    let mut buf = vec![0u8; size];
    fill_rng.fill_bytes(&mut buf);

    // Note: this is only basic equivalance checking via byte-at-a-time. More full equivalance
    // checking will be done via quickcheck tests.
    let mut incr_splits = Vec::with_capacity(expected_splits.len());
    {
        let mut incr = chunker;
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

    assert_eq!(expected_splits, &incr_splits[..]);
}


fn cut_test_sz<C: Chunk + ToChunkIncr>(
    seed: u128,
    size: usize,
    chunker: C,
    expected_splits: &[usize],
) {
    let mut fill_rng = Pcg64::new(seed, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);
    let mut buf = vec![0u8; size];
    fill_rng.fill_bytes(&mut buf);

    // Note: this doesn't validate SearchState at all
    let mut splits = Vec::with_capacity(expected_splits.len());
    {
        let mut state = chunker.to_search_state();
        let mut discard_idx = 0;
        let mut last_chunk_idx = 0;
        loop {
            let b = &buf[discard_idx..];
            let (split_point, discard_ct) = chunker.find_chunk_edge(&mut state, b);
            match split_point {
                Some(split_point) => {
                    let split_point_global = discard_idx + split_point;
                    if last_chunk_idx > split_point_global {
                        panic!("last_chunk_idx: {}, split_point_global: {}, split_point: {}, discard_idx: {}",
                            last_chunk_idx, split_point_global, split_point, discard_idx);
                    }
                    let split_len = split_point_global - last_chunk_idx;
                    last_chunk_idx = split_point_global;
                    splits.push(split_len);
                }
                None => {
                    break;
                }
            }
            discard_idx += discard_ct;
        }
    }

    // Note: this is only basic equivalance checking via byte-at-a-time. More full equivalance
    // checking will be done via quickcheck tests.
    let mut incr_splits = Vec::with_capacity(expected_splits.len());
    {
        let mut incr = chunker.to_chunk_incr();
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

fn cut_test<C: Chunk + ToChunkIncr>(seed: u128, chunker: C, expected_splits: &[usize]) {
    cut_test_sz(seed, 8192 * 4, chunker, expected_splits)
}

#[cfg(feature = "mii")]
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

#[cfg(feature = "bup")]
#[test]
fn bup_cuts_1() {
    cut_test(0, hash_roll::bup::RollSum::default(), &[2600, 6245])
}

#[cfg(feature = "gzip")]
#[test]
fn gzip_cuts_1() {
    cut_test(
        0,
        hash_roll::gzip::GzipRsyncable::default(),
        &[2941, 2077, 5263, 7263, 392, 4371, 5204],
    )
}

#[cfg(feature = "gzip")]
#[test]
fn gzip_cuts_2() {
    // chosen so we check window removal
    cut_test(
        2,
        hash_roll::gzip::GzipRsyncable::default(),
        &[9277, 2758, 3074, 7415, 3579, 4141],
    )
}

#[cfg(feature = "buzhash")]
#[test]
fn buzhash_cuts_1() {
    cut_test(
        0,
        hash_roll::buzhash::BuzHash::new_nom(0),
        &[6265, 1745, 11527, 6851, 1089],
    )
}

#[cfg(feature = "zpaq")]
#[test]
fn zpaq_cuts_0() {
    // These match edges from Zpaq 7.15 (with modification to print the fragment sizes).
    //
    //     cargo run --example generate-test-data 0 >test_data_0.bin
    //     zpaq a foo.zpaq ~/p/hash-roll/test_data_0.bin -fragment 3
    cut_test(
        0,
        hash_roll::zpaq::Zpaq::with_average_size_pow_2(3),
        &[10785, 6329, 1287, 860, 4716, 7419],
    )
}

#[cfg(feature = "zpaq")]
#[test]
fn zpaq_cuts_3() {
    // These match edges from Zpaq 7.15 (with modification to print the fragment sizes).
    //
    //     cargo run --example generate-test-data 3 >test_data_3.bin
    //     zpaq a foo.zpaq ~/p/hash-roll/test_data_3.bin -fragment 3
    cut_test(
        3,
        hash_roll::zpaq::Zpaq::with_average_size_pow_2(3),
        &[16353, 2334, 970, 5326, 1557],
    )
}

#[cfg(feature = "pigz")]
#[test]
fn pigz_cuts_0() {
    cut_test(
        0,
        hash_roll::pigz::PigzRsyncable::default(),
        &[9069, 1191, 3685, 8629, 2119, 2939],
    )
}

/*
 * 0
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 0, tss: 8388608 -> (131072, 0) 
 * 1
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 131072, tss: 8388608 -> (131072, 0) 
 * 2
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 262144, tss: 8388608 -> (131072, 0) 
 * 3
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 393216, tss: 8388608 -> (131072, 0) 
 * 4
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 524288, tss: 8388608 -> (131072, 0) 
 * 5
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 655360, tss: 8388608 -> (131072, 0) 
 * 6
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 786432, tss: 8388608 -> (131072, 0) 
 7
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 917504, tss: 8388608 -> (131072, 0) 
 8
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 1048576, tss: 8388608 -> (131072, 0) 
 9
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 1179648, tss: 8388608 -> (131072, 0) 
 10
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 1310720, tss: 8388608 -> (131072, 0) 
 11
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 1441792, tss: 8388608 -> (131072, 0) 
 12
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 1572864, tss: 8388608 -> (87647, 1) 
 13
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (87647, 131072), inbf: 0, tss: 8388608 -> (43425, 0) 
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 43425, tss: 8388608 -> (131072, 0) 
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 174497, tss: 8388608 -> (131072, 0) 
../lib/compress/zstdmt_compress.c: findSynchronizationPoint: input: (0, 131072), inbf: 305569, tss: 8388608 -> (131072, 0)
 */
#[cfg(feature = "zstd")]
#[test]
fn zstd_cuts_0_2mb() {
    cut_test_sz(
        0,
        1024 * 1024 * 2,
        hash_roll::zstd::Zstd::default(),
        //&[1660511],
        &[12 * 131072 + 87647],
    )
}

#[cfg(feature = "gear")]
#[test]
fn gear32_cuts_0() {
    cut_test(
        0,
        hash_roll::gear::Gear32::default(),
        &[11031, 7789, 10463],
    )
}

#[cfg(feature = "fastcdc")]
#[test]
fn fastcdc_cuts_incr_0() {
    cut_test_incr(
        0,
        8192 * 4,
        hash_roll::fastcdc::FastCdcIncr::default(),
        &[8463, 9933, 9029],
    )
}

#[cfg(feature = "fastcdc")]
#[test]
fn fastcdc_cuts_0() {
    cut_test(
        0,
        hash_roll::fastcdc::FastCdc::default(),
        &[8463, 9933, 9029],
    )
}

#[cfg(feature = "ram")]
#[test]
fn ram_cuts_0() {
    cut_test(
        0,
        hash_roll::ram::Ram::with_w(8192),
        &[8264, 8368, 8341]
    )
}
