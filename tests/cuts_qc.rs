// check the following are equivalent:
//  - find_chunk_edge() with 1 set of buffer sizes vs another set of buffer sizes
//  - incrimental with 1 set of buffer sizes vs another set of buffer sizes
//  - find_chunk_edge() vs incrimental
//
//  - simd vs non-simd algorithms

use hash_roll::Chunk;
use proptest::prelude::*;

fn splits_fce<C: Chunk>(chunker: &C, mut buf: &[u8], buf_sizes: &[usize]) -> Vec<usize> {
    let mut splits = Vec::new();
    let mut i = 0;
    let mut ss = None;
    let mut prev_buf_size = 0;
    loop {
        if buf.len() == 0 {
            break;
        }

        // use adative methods to ensure buf size grows
        let buf_size = buf_sizes[i % buf_sizes.len()] + prev_buf_size;
        i += 1;
        let buf_size = std::cmp::min(buf_size, buf.len());

        match chunker.find_chunk_edge(ss, &buf[..buf_size]) {
            Ok(split_point) => {
                splits.push(split_point);
                buf = &buf[split_point..];
                ss = None;
                prev_buf_size = 0;
            },
            Err(nss) => {
                // at end of buffer without a split point
                if buf_size == buf.len() {
                    break;
                }

                ss = Some(nss);
                prev_buf_size = buf_size;
            }
        }
    }

    splits
}

proptest! {
    #[test]
    fn rsyncable_fce_self_consistent_with_varying_buf_size(
        data in prop::collection::vec(0u8..=255u8, 0..10000),
        buf_sizes_1 in prop::collection::vec(1usize..5000, 1..10000),
        buf_sizes_2 in prop::collection::vec(1usize..5000, 1..10000))
    {
        let chunker = hash_roll::rsyncable::Rsyncable::default();
        let s1 = splits_fce(&chunker, &data[..], &buf_sizes_1[..]);
        let s2 = splits_fce(&chunker, &data[..], &buf_sizes_2[..]);
        assert_eq!(s1, s2);
    }

    #[test]
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
}
