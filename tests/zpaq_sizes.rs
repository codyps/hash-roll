#![cfg(feature = "zpaq")]

use hash_roll::{zpaq::Zpaq, Chunk, ChunkIncr, ToChunkIncr};
use std::ops::Bound::{Excluded, Included, Unbounded};

// Choose bytes whose independent hash never causes a natural cut. This
// exercises size limits without depending on a lucky random input.
fn no_natural_cut_data(len: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(len);
    let mut predicted = [0u8; 256];
    let (mut hash, mut last) = (0u32, 0u8);
    for _ in 0..len {
        let (byte, next) = (0..=255u8)
            .find_map(|b| {
                let multiplier = if b == predicted[last as usize] {
                    314159265u32
                } else {
                    271828182u32
                };
                let h = hash.wrapping_add(b as u32 + 1).wrapping_mul(multiplier);
                (h >= 65536).then_some((b, h))
            })
            .unwrap();
        predicted[last as usize] = byte;
        last = byte;
        hash = next;
        data.push(byte);
    }
    data
}

#[test]
fn default_maximum_is_508_kibibytes_in_both_apis() {
    let data = no_natural_cut_data(520193);
    let c = Zpaq::default();
    assert_eq!(
        c.find_chunk_edge(&mut c.to_search_state(), &data),
        (Some(520192), 520192)
    );
    assert_eq!(c.to_chunk_incr().push(&data), Some(520192));
    let mut incr = c.to_chunk_incr();
    assert_eq!(incr.push(&data[..520191]), None);
    assert_eq!(incr.push(&data[520191..]), Some(1));
}

#[test]
fn range_bounds_are_normalized_and_enforced() {
    let cases = [
        ((Included(1), Included(4)), 1, 4),
        ((Included(1), Excluded(4)), 1, 3),
        ((Excluded(2), Included(4)), 3, 4),
        ((Excluded(2), Excluded(4)), 3, 3),
        ((Unbounded, Included(4)), 1, 4),
        ((Included(4), Unbounded), 4, 32),
    ];
    for (range, min, max) in cases {
        // Exponent zero accepts every hash, so the lower bound decides the cut.
        let c = Zpaq::with_average_and_range(0, range);
        assert_eq!(c.to_chunk_incr().push(&[1; 32]), Some(min));
        // Exponent 32 accepts only hash zero. These bytes do not produce it.
        let c = Zpaq::with_average_and_range(32, range);
        let expected = if range.1 == Unbounded {
            None
        } else {
            Some(max)
        };
        assert_eq!(
            c.find_chunk_edge(&mut c.to_search_state(), &[1; 32]).0,
            expected
        );
        assert_eq!(c.to_chunk_incr().push(&[1; 32]), expected);
    }
}

#[test]
fn inclusive_maximum_is_never_exceeded_across_multiple_chunks() {
    let c = Zpaq::with_average_and_range(32, 1..=4);
    let chunks: Vec<_> = c
        .to_chunk_incr()
        .iter_slices_strict(&[1; 16])
        .map(|s| s.len())
        .collect();
    assert_eq!(chunks, vec![4; 4]);
}

#[test]
fn max_size_constructors_use_byte_units() {
    for exponent in 2..=34 {
        let max = 1u64 << exponent;
        assert_eq!(
            Zpaq::with_max_size(max),
            Zpaq::with_average_and_range(exponent - 2, (max / 64).max(1)..=max)
        );
        assert_eq!(
            Zpaq::with_max_size(max),
            Zpaq::with_range((max / 64).max(1)..=max)
        );
    }
    assert_eq!(
        Zpaq::with_max_size(4096),
        Zpaq::with_average_and_range(10, 64..=4096)
    );
    assert_eq!(Zpaq::with_range(1..5), Zpaq::with_range(1..=4));
}

#[test]
fn non_power_of_two_cutoff_matches_integer_reference() {
    // Infer the expected boundary using a standalone predictor and rational
    // probability 4/max, independently of the constructor implementation.
    use rand::Rng;
    let mut data = vec![0; 65536];
    rand_pcg::Pcg64::new(1, 7).fill_bytes(&mut data);
    for max in [1u64, 3, 7, 4095, 10000, 65535] {
        let mut hash = 0u32;
        let mut predicted = [0u8; 256];
        let mut last = 0u8;
        let mut expected = None;
        let cutoff = ((1u64 << 34) / max).min(1u64 << 32);
        for (i, &b) in data.iter().enumerate() {
            hash = hash
                .wrapping_add(b as u32 + 1)
                .wrapping_mul(if b == predicted[last as usize] {
                    314159265
                } else {
                    271828182
                });
            predicted[last as usize] = b;
            last = b;
            let len = i as u64 + 1;
            if (u64::from(hash) < cutoff && len >= (max / 64).max(1)) || len == max {
                expected = Some(i + 1);
                break;
            }
        }
        assert_eq!(
            Zpaq::with_max_size(max).to_chunk_incr().push(&data),
            expected
        );
    }
}

#[test]
fn unbounded_ranges_have_explicit_targets() {
    assert_eq!(Zpaq::with_range(..), Zpaq::with_average_and_range(16, ..));
    assert_eq!(
        Zpaq::with_range(4096..),
        Zpaq::with_average_and_range(16, 4096..)
    );
}

#[test]
fn invalid_configurations_panic_consistently() {
    for max in [0, (1u64 << 34) + 1, u64::MAX] {
        assert!(std::panic::catch_unwind(|| Zpaq::with_max_size(max)).is_err());
    }
    for range in [
        (Included(0), Included(0)),
        (Included(4), Excluded(4)),
        (Excluded(u64::MAX), Unbounded),
        (Unbounded, Excluded(0)),
    ] {
        assert!(std::panic::catch_unwind(|| Zpaq::with_range(range)).is_err());
        assert!(std::panic::catch_unwind(|| Zpaq::with_average_and_range(16, range)).is_err());
    }
    assert!(std::panic::catch_unwind(|| Zpaq::with_range(u64::MAX..)).is_err());
    assert!(std::panic::catch_unwind(|| Zpaq::with_average_and_range(33, ..)).is_err());
    for exponent in [0, 9, 33, 255] {
        assert!(std::panic::catch_unwind(|| Zpaq::with_average_size_pow_2(exponent)).is_err());
    }
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(64))]
    #[test]
    fn configured_lengths_are_respected(
        min in 1usize..64, width in 0usize..128, exponent in 0u8..=32,
        exclude_lower in proptest::bool::ANY, exclude_upper in proptest::bool::ANY,
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..4096)) {
        let max = min + width;
        let range = (if exclude_lower { Excluded((min - 1) as u64) } else { Included(min as u64) },
                     if exclude_upper { Excluded((max + 1) as u64) } else { Included(max as u64) });
        let c = Zpaq::with_average_and_range(exponent, range);
        let mut iter = c.to_chunk_incr().iter_slices_strict(&data);
        let lengths: Vec<_> = iter.by_ref().map(|s| s.len()).collect();
        proptest::prop_assert!(lengths.iter().all(|n| (min..=max).contains(n)));
        let tail = iter.take_rem().len();
        proptest::prop_assert!(tail < max);
        proptest::prop_assert_eq!(lengths.iter().sum::<usize>() + tail, data.len());
        let mut state = c.to_search_state();
        let mut remaining = data.as_slice();
        let mut slice_lengths = Vec::new();
        while let (Some(cut), discard) = c.find_chunk_edge(&mut state, remaining) {
            slice_lengths.push(cut);
            remaining = &remaining[discard..];
        }
        proptest::prop_assert_eq!(slice_lengths, lengths);
    }
}
