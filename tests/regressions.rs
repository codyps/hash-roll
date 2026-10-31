#![cfg(any(
    feature = "bup",
    feature = "buzhash",
    feature = "fastcdc",
    feature = "gear",
    feature = "mii",
    feature = "ram",
    feature = "zpaq"
))]

use hash_roll::ToChunkIncr;
#[cfg(any(
    feature = "buzhash",
    feature = "fastcdc",
    feature = "gear",
    feature = "mii",
    feature = "ram",
    feature = "zpaq"
))]
use hash_roll::{Chunk, ChunkIncr};

#[test]
#[cfg(feature = "gear")]
fn gear_offset() {
    let c = hash_roll::gear::Gear32::with_average_size_log2(1);
    let data: Vec<u8> = (0..=255).collect();
    let a = c.find_chunk_edge(&mut c.to_search_state(), &data).0;
    let b = c.to_chunk_incr().push(&data);
    assert_eq!(a, b);
}

#[test]
#[cfg(feature = "fastcdc")]
fn fastcdc_max_bound() {
    let table = [1u64 << 16; 256];
    let c = hash_roll::fastcdc::FastCdc::new(&table, 1, 2, 4);
    let data = [0u8; 4];
    let (cut, discard) = c.find_chunk_edge(&mut c.to_search_state(), &data);
    assert!(
        cut.unwrap() <= data.len() && discard <= data.len(),
        "cut={:?}, discard={}",
        cut,
        discard
    );
}

#[test]
#[cfg(feature = "fastcdc")]
fn fastcdc_zero_progress() {
    let table = [0u64; 256];
    let c = hash_roll::fastcdc::FastCdc::new(&table, 1, 2, 4);
    let mut incr = c.to_chunk_incr();
    assert_eq!(incr.push(&[0]), None);
    assert_ne!(incr.push(&[0]), Some(0));
}

#[test]
#[cfg(feature = "ram")]
fn ram_offset() {
    let c = hash_roll::ram::Ram::with_w(1);
    let data = [0u8; 4];
    assert_eq!(
        c.find_chunk_edge(&mut c.to_search_state(), &data).0,
        c.to_chunk_incr().push(&data)
    );
}

#[test]
#[cfg(feature = "mii")]
fn mii_reset() {
    let c = hash_roll::mii::Mii::with_w(1);
    let data = [1u8, 2, 1, 2];
    let mut s = c.to_search_state();
    let mut i = c.to_chunk_incr();
    assert_eq!(c.find_chunk_edge(&mut s, &data).0, i.push(&data));
    assert_eq!(c.find_chunk_edge(&mut s, &data[2..]).0, i.push(&data[2..]));
}

#[test]
#[cfg(feature = "zpaq")]
fn zpaq_bound() {
    let c = hash_roll::zpaq::Zpaq::with_average_and_range(32, 0..4);
    let data = [1u8; 8];
    assert_eq!(
        c.find_chunk_edge(&mut c.to_search_state(), &data).0,
        c.to_chunk_incr().push(&data)
    );
}

#[test]
#[cfg(feature = "buzhash")]
fn buzhash_max() {
    let table = [0u32; 256];
    let c = hash_roll::buzhash::BuzHash::new(
        7,
        u32::MAX,
        hash_roll::buzhash::BuzHashTableHash::from(&table),
        4,
    );
    let data = [0u8; 8];
    assert_eq!(
        c.find_chunk_edge(&mut c.to_search_state(), &data).0,
        c.to_chunk_incr().push(&data)
    );
}

#[test]
#[cfg(feature = "bup")]
fn bup_non_power_two_window() {
    let c = hash_roll::bup::RollSum::with_window(3);
    let mut i = c.to_chunk_incr();
    for b in [1u8, 2, 3, 4] {
        i.roll_byte(b);
    }
    // s1's high 16 bits must contain the sum of the final three bytes plus 3*31.
    assert_eq!(i.digest() >> 16, 2 + 3 + 4 + 3 * 31);
}

#[test]
#[cfg(feature = "buzhash")]
fn buzhash_window_rotation() {
    let table = hash_roll::buzhash_table::GO_BUZHASH;
    use rand::Rng;
    let mut data = vec![0u8; 4096];
    rand_pcg::Pcg64::new(0, 1).fill_bytes(&mut data);
    let k = 9;
    let mask = 255;
    let c = hash_roll::buzhash::BuzHash::new(
        k,
        mask,
        hash_roll::buzhash::BuzHashTableHash::from(&table),
        u64::MAX,
    );
    let expected = (0..data.len()).find_map(|end| {
        let start = (end + 1).saturating_sub(k);
        let hash = data[start..=end]
            .iter()
            .fold(0u32, |h, b| h.rotate_left(1) ^ table[*b as usize]);
        if hash & mask == mask {
            Some(end + 1)
        } else {
            None
        }
    });
    assert_eq!(
        c.find_chunk_edge(&mut c.to_search_state(), &data).0,
        expected
    );
}
