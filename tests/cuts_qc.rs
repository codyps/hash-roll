#![cfg(any(
    feature = "bup",
    feature = "buzhash",
    feature = "fastcdc",
    feature = "gear",
    feature = "gzip",
    feature = "mii",
    feature = "pigz",
    feature = "ram",
    feature = "zpaq",
    feature = "zstd",
    feature = "rabin",
    feature = "ae",
    feature = "gear64",
    feature = "seqcdc",
    feature = "leap"
))]

//! Check both APIs against each other, including fragmented input and repeated splits.
use hash_roll::{Chunk, ChunkIncr, ToChunkIncr};
use proptest::prelude::*;

fn splits_fce<C: Chunk>(chunker: &C, data: &[u8], sizes: &[usize]) -> Vec<usize> {
    let mut state = chunker.to_search_state();
    let mut splits = Vec::new();
    let mut start = 0;
    let mut end = 0;
    let mut reads = 0;
    loop {
        let (cut, discard) = chunker.find_chunk_edge(&mut state, &data[start..end]);
        assert!(discard <= end - start);
        if let Some(cut) = cut {
            assert!((1..=end - start).contains(&cut));
            assert!(discard <= cut);
            let global = start + cut;
            assert!(splits.last().is_none_or(|&prev| global > prev));
            splits.push(global);
            start += discard;
        } else {
            start += discard;
            if end == data.len() {
                break;
            }
            end = (end + sizes[reads % sizes.len()]).min(data.len());
            reads += 1;
        }
    }
    splits
}

fn splits_incr<C: ToChunkIncr>(chunker: &C, data: &[u8], sizes: &[usize]) -> Vec<usize> {
    let mut incr = chunker.to_chunk_incr();
    let mut splits = Vec::new();
    let mut pos = 0;
    let mut reads = 0;
    while pos < data.len() {
        let end = (pos + sizes[reads % sizes.len()]).min(data.len());
        reads += 1;
        while pos < end {
            assert_eq!(incr.push(&[]), None);
            match incr.push(&data[pos..end]) {
                Some(cut) => {
                    assert!((1..=end - pos).contains(&cut));
                    pos += cut;
                    splits.push(pos);
                }
                None => pos = end,
            }
        }
    }
    assert_eq!(incr.push(&[]), None);
    splits
}

fn check<C: Chunk + ToChunkIncr>(chunker: C, data: &[u8], sizes: &[usize]) {
    let all = [data.len().max(1)];
    let expected = splits_fce(&chunker, data, &all);
    assert_eq!(expected, splits_fce(&chunker, data, sizes));
    assert_eq!(expected, splits_incr(&chunker, data, &all));
    assert_eq!(expected, splits_incr(&chunker, data, sizes));
    let mut offset = 0;
    let actual: Vec<_> = chunker
        .to_chunk_incr()
        .iter_slices_strict(data)
        .map(|s| {
            assert!(!s.is_empty());
            offset += s.len();
            offset
        })
        .collect();
    assert_eq!(expected, actual);
    let slices: Vec<_> = chunker.to_chunk_incr().iter_slices(data).collect();
    assert!(slices.iter().all(|s| !s.is_empty()));
    assert_eq!(slices.concat(), data);
    assert_eq!(chunker.chunks(data).collect::<Vec<_>>(), slices);
}

macro_rules! consistency {
    ($feature:literal, $name:ident, $chunker:expr) => {
        #[cfg(feature = $feature)]
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(64))]
            #[test]
            fn $name(data in prop::collection::vec(any::<u8>(), 0..32768),
                     sizes in prop::collection::vec(1usize..4096, 1..16)) {
                check($chunker, &data, &sizes);
            }
        }
    };
}

consistency!("bup", bup, hash_roll::bup::RollSum::default());
consistency!(
    "bup",
    bup_odd_window,
    hash_roll::bup::RollSum::with_window(7)
);
consistency!("buzhash", buzhash, hash_roll::buzhash::BuzHash::new_nom(0));
consistency!(
    "buzhash",
    buzhash_max,
    hash_roll::buzhash::BuzHash::new(
        9,
        u32::MAX,
        hash_roll::buzhash::BuzHashTableHash::from(&hash_roll::buzhash_table::GO_BUZHASH),
        257
    )
);
consistency!("fastcdc", fastcdc, hash_roll::fastcdc::FastCdc::default());
consistency!(
    "fastcdc",
    fastcdc_small,
    hash_roll::fastcdc::FastCdc::new(&hash_roll::gear_table::GEAR_64, 8, 16, 64)
);
consistency!(
    "gear",
    gear,
    hash_roll::gear::Gear32::with_average_size_log2(8)
);
consistency!(
    "gzip",
    gzip,
    hash_roll::gzip::GzipRsyncable::with_window_and_modulus(31, 256)
);
consistency!("mii", mii, hash_roll::mii::Mii::with_w(3));
consistency!("pigz", pigz, hash_roll::pigz::PigzRsyncable::with_bits(8));
consistency!("ram", ram, hash_roll::ram::Ram::with_w(64));
consistency!(
    "zpaq",
    zpaq,
    hash_roll::zpaq::Zpaq::with_average_and_range(16, 16..512)
);
consistency!(
    "zstd",
    zstd,
    hash_roll::zstd::Zstd::with_target_section_size(1 << 20)
);

#[cfg(feature = "zstd")]
#[test]
fn zstd_multiple_boundaries() {
    use rand::Rng;
    let mut data = vec![0; 8 << 20];
    rand_pcg::Pcg64::new(0, 1).fill_bytes(&mut data);
    let chunker = hash_roll::zstd::Zstd::with_target_section_size(1 << 20);
    assert!(splits_fce(&chunker, &data, &[data.len()]).len() > 1);
    check(chunker, &data, &[7, 65536, 131072]);
}

consistency!(
    "rabin",
    rabin,
    hash_roll::rabin::Rabin::new(
        hash_roll::rabin::DEFAULT_POLYNOMIAL,
        8,
        hash_roll::ChunkSizes::new(16, 64, 256),
        6
    )
);
consistency!(
    "ae",
    ae,
    hash_roll::ae::Ae::new(16, hash_roll::ae::Extremum::Maximum, 256)
);
consistency!(
    "gear64",
    gear64,
    hash_roll::gear64::Gear64::with_average_size_log2(8)
);
consistency!(
    "seqcdc",
    seqcdc,
    hash_roll::seqcdc::SeqCdc::new(
        hash_roll::ChunkSizes::new(16, 64, 256),
        3,
        2,
        7,
        hash_roll::seqcdc::Direction::Increasing
    )
);
consistency!(
    "leap",
    leap,
    hash_roll::leap::Leap::new(hash_roll::ChunkSizes::new(204, 512, 1024))
);
