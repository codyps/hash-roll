// check the following are equivalent:
//  - find_chunk_edge() with 1 set of buffer sizes vs another set of buffer sizes
//  - incrimental with 1 set of buffer sizes vs another set of buffer sizes
//  - find_chunk_edge() vs incrimental
//
//  - simd vs non-simd algorithms

use hash_roll::Chunk;
use proptest::prelude::*;

fn splits_fce<C: Chunk>(chunker: &C, buf: &[u8], buf_sizes: &[usize]) -> Vec<usize> {
    let mut splits = Vec::new();
    let mut i = 0;
    let mut ss = chunker.to_search_state();
    let mut last_split_point = 0;
    let mut curr_discard = 0;
    let mut prev_buf_size = 0;
    loop {
        if buf.len() == curr_discard {
            break;
        }

        // use adative methods to ensure buf size grows
        let buf_size = buf_sizes[i % buf_sizes.len()] + prev_buf_size;
        i += 1;
        let buf_size = std::cmp::min(buf_size, buf.len() - curr_discard);

        let b = &buf[curr_discard..(buf_size + curr_discard)];
        println!("{{ PRE: curr_discard: {}, buf_size: {}", curr_discard, buf_size);
        let (split, discard_ct) = chunker.find_chunk_edge(&mut ss, b);
        println!("}} POST: discard_ct: {}, next_discard: {}", discard_ct, curr_discard + discard_ct);

        match split {
            Some(split_point) => {
                // `split_point` is translated into the entire buffer (from the one passed to fce),
                // and the length is determined by tracking the previous split.
                let split_point_global = curr_discard + split_point;
                let split_len = split_point_global - last_split_point;
                splits.push(split_len);
                last_split_point = split_point_global;
                prev_buf_size = 0;
            }
            None => {
                // at end of buffer without a split point
                if buf_size == (buf.len() - curr_discard) {
                    break;
                }

                prev_buf_size = buf_size;
            }
        }

        curr_discard += discard_ct;
        println!("-- curr_discard = {}", curr_discard);

    }

    splits
}

proptest! {
    #[test]
    #[cfg(feature = "gzip")]
    fn gzip_fce_self_consistent_with_varying_buf_size(
        data in prop::collection::vec(0u8..=255u8, 0..10000),
        buf_sizes_1 in prop::collection::vec(1usize..5000, 1..10000),
        buf_sizes_2 in prop::collection::vec(1usize..5000, 1..10000))
    {
        let chunker = hash_roll::gzip::GzipRsyncable::default();
        let s1 = splits_fce(&chunker, &data[..], &buf_sizes_1[..]);
        let s2 = splits_fce(&chunker, &data[..], &buf_sizes_2[..]);
        assert_eq!(s1, s2);
    }

    #[test]
    #[cfg(feature = "mii")]
    fn mii_fce_self_consistent_with_varying_buf_size(
        data in prop::collection::vec(0u8..=255u8, 0..10000),
        buf_sizes_1 in prop::collection::vec(1usize..5000, 1..10000),
        buf_sizes_2 in prop::collection::vec(1usize..5000, 1..10000))
    {
        let chunker = hash_roll::mii::Mii::default();
        let s1 = splits_fce(&chunker, &data[..], &buf_sizes_1[..]);
        let s2 = splits_fce(&chunker, &data[..], &buf_sizes_2[..]);
        assert_eq!(s1, s2);
    }

    #[test]
    #[cfg(feature = "buzhash_big")]
    fn buzhash_fce_self_consistent_with_varying_buf_size(
        seed: u8,
        data in prop::collection::vec(0u8..=255u8, 0..10000),
        buf_sizes_1 in prop::collection::vec(1usize..5000, 1..10000),
        buf_sizes_2 in prop::collection::vec(1usize..5000, 1..10000))
    {
        let chunker = hash_roll::buzhash::BuzHash::new_nom(seed);
        let s1 = splits_fce(&chunker, &data[..], &buf_sizes_1[..]);
        let s2 = splits_fce(&chunker, &data[..], &buf_sizes_2[..]);
        assert_eq!(s1, s2);
    }

    #[test]
    #[cfg(feature = "buzhash")]
    fn buzhash_short_fce_self_consistent_with_varying_buf_size(
        data in prop::collection::vec(0u8..=255u8, 0..100),
        buf_sizes_1 in prop::collection::vec(1usize..100, 1..100),
        buf_sizes_2 in prop::collection::vec(1usize..100, 1..100))
    {
        let chunker = hash_roll::buzhash::BuzHash::new(
            7,
            (1 << 4u32) - 1,
            hash_roll::buzhash::BuzHashTableByteSaltHash::from((0, &hash_roll::buzhash_table::GO_BUZHASH)),
            1 << 10,
        );
        let s1 = splits_fce(&chunker, &data[..], &buf_sizes_1[..]);
        let s2 = splits_fce(&chunker, &data[..], &buf_sizes_2[..]);
        assert_eq!(s1, s2);
    }

    #[test]
    #[cfg(feature = "zpaq")]
    fn zpaq_fce_self_consistent_with_varying_buf_size(
        data in prop::collection::vec(0u8..=255u8, 0..10000),
        buf_sizes_1 in prop::collection::vec(1usize..5000, 1..10000),
        buf_sizes_2 in prop::collection::vec(1usize..5000, 1..10000))
    {
        let chunker = hash_roll::zpaq::Zpaq::with_average_size_pow_2(3);
        let s1 = splits_fce(&chunker, &data[..], &buf_sizes_1[..]);
        let s2 = splits_fce(&chunker, &data[..], &buf_sizes_2[..]);
        assert_eq!(s1, s2);
    }

    #[test]
    #[cfg(feature = "pigz")]
    fn pigz_fce_self_consistent_with_varying_buf_size(
        data in prop::collection::vec(0u8..=255u8, 0..10000),
        buf_sizes_1 in prop::collection::vec(1usize..5000, 1..10000),
        buf_sizes_2 in prop::collection::vec(1usize..5000, 1..10000))
    {
        let chunker = hash_roll::pigz::PigzRsyncable::default();
        let s1 = splits_fce(&chunker, &data[..], &buf_sizes_1[..]);
        let s2 = splits_fce(&chunker, &data[..], &buf_sizes_2[..]);
        assert_eq!(s1, s2);
    }

    #[test]
    #[cfg(feature = "bup")]
    fn bup_fce_self_consistent_with_varying_buf_size(
        data in prop::collection::vec(0u8..=255u8, 0..10000),
        buf_sizes_1 in prop::collection::vec(1usize..5000, 1..10000),
        buf_sizes_2 in prop::collection::vec(1usize..5000, 1..10000))
    {
        let chunker = hash_roll::bup::RollSum::default();
        let s1 = splits_fce(&chunker, &data[..], &buf_sizes_1[..]);
        let s2 = splits_fce(&chunker, &data[..], &buf_sizes_2[..]);
        assert_eq!(s1, s2);
    }
}
