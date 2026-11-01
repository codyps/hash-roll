//! Independent arithmetic/cut oracles and fragmented-buffer regressions.
#![allow(dead_code)]
use hash_roll::Chunk;

fn bytes(n: usize) -> Vec<u8> {
    let mut x = 0x123456789abcdefu64;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}
fn cuts<C: Chunk>(c: &C, data: &[u8], blocks: &[usize]) -> Vec<usize> {
    let mut state = c.to_search_state();
    let (mut start, mut end, mut read) = (0, 0, 0);
    let mut out = Vec::new();
    loop {
        let (cut, discard) = c.find_chunk_edge(&mut state, &data[start..end]);
        assert!(discard <= end - start);
        if let Some(n) = cut {
            assert!((1..=end - start).contains(&n) && discard <= n);
            assert!(out.last().is_none_or(|&last| start + n > last));
            out.push(start + n);
            start += discard;
        } else {
            start += discard;
            if end == data.len() {
                break;
            }
            end = data.len().min(end + blocks[read % blocks.len()]);
            read += 1;
        }
    }
    out
}
fn buffered<C: Chunk>(c: C, min: usize, max: usize) {
    for data in [
        bytes(32771),
        vec![0; 32771],
        vec![255; 32771],
        (0..32771).map(|i| (i % 251) as u8).collect(),
    ] {
        let expected = cuts(&c, &data, &[data.len()]);
        for blocks in [&[1][..], &[3, 31, 128, 513, 4096][..]] {
            assert_eq!(expected, cuts(&c, &data, blocks));
        }
        let mut last = 0;
        for end in expected {
            assert!((min..=max).contains(&(end - last)));
            last = end;
        }
        let chunks: Vec<_> = c.chunks(&data).collect();
        assert!(chunks.iter().all(|b| !b.is_empty()));
        assert_eq!(chunks.concat(), data);
        let mut iter = c.chunks(&[]);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }
}

#[cfg(feature = "rabin")]
fn gf2(data: &[u8], poly: u64) -> u64 {
    let degree = 63 - poly.leading_zeros();
    data.iter().fold(0u64, |h, &b| {
        let mut v = (u128::from(h) << 8) | u128::from(b);
        for bit in (degree..degree + 8).rev() {
            if v & (1u128 << bit) != 0 {
                v ^= u128::from(poly) << (bit - degree);
            }
        }
        v as u64
    })
}
#[cfg(feature = "rabin")]
#[test]
fn rabin_matches_long_division_after_every_slide_and_reset() {
    use hash_roll::{rabin::RabinFingerprint, RollingHash};
    let data = bytes(1024);
    for poly in [
        0x11bu64,
        hash_roll::rabin::DEFAULT_POLYNOMIAL,
        (1 << 56) | 0x95,
    ] {
        for window in [1, 7, 64, 65] {
            let mut hash = RabinFingerprint::new(poly, window);
            for pass in 0..2 {
                for end in 1..=data.len() {
                    assert_eq!(
                        hash.roll_byte(data[end - 1]),
                        gf2(&data[end.saturating_sub(window)..end], poly),
                        "window={window} pass={pass}"
                    );
                }
                hash.reset();
                assert_eq!(hash.digest(), 0);
            }
        }
    }
}
#[cfg(feature = "rabin-karp")]
#[test]
fn integer_rabin_karp_matches_librsync_seeded_polynomial() {
    use hash_roll::{rabin_karp::RabinKarp, RollingHash};
    let data = bytes(2048);
    for window in [1, 7, 64, 65, 1024] {
        let mut h = RabinKarp::new(window);
        for _ in 0..2 {
            for end in 1..=data.len() {
                let expected = data[end.saturating_sub(window)..end]
                    .iter()
                    .fold(1u32, |v, &b| {
                        v.wrapping_mul(0x08104225).wrapping_add(b.into())
                    });
                assert_eq!(h.roll_byte(data[end - 1]), expected);
            }
            h.reset();
            assert_eq!(h.digest(), 1);
        }
    }
}
#[cfg(feature = "rabin")]
#[test]
fn rabin_cdc_matches_naive_window_and_restic_zero_preset() {
    use hash_roll::{
        rabin::{Rabin, DEFAULT_POLYNOMIAL},
        ChunkSizes,
    };
    let data = bytes(16384);
    let sizes = ChunkSizes::new(16, 64, 256);
    let c = Rabin::new(DEFAULT_POLYNOMIAL, 8, sizes, 6);
    let (mut start, mut expected) = (0, Vec::new());
    for end in 16..=data.len() {
        if end - start >= 16
            && (end - start == 256 || gf2(&data[end - 8..end], DEFAULT_POLYNOMIAL) & 63 == 0)
        {
            expected.push(end);
            start = end;
        }
    }
    assert_eq!(cuts(&c, &data, &[31]), expected);
    assert_eq!(
        cuts(
            &Rabin::restic(DEFAULT_POLYNOMIAL),
            &vec![0; (1 << 19) + 17],
            &[4093]
        ),
        vec![1 << 19]
    );
    buffered(c, 16, 256);
}
#[cfg(feature = "ae")]
#[test]
fn ae_strict_extrema_ties_and_scalar_acceleration_agree() {
    use hash_roll::ae::{Ae, Extremum};
    for mode in [Extremum::Minimum, Extremum::Maximum] {
        for horizon in [1, 7, 32, 257] {
            let c = Ae::new(horizon, mode, 1024);
            for data in [bytes(10003), vec![128; 10003]] {
                let mut expected = Vec::new();
                let mut start = 0;
                while data.len() - start > horizon {
                    // Search each candidate's following horizon independently.
                    let n = ((start + 1)..=data.len()).find(|&end| {
                        if end - start == 1024 {
                            return true;
                        }
                        if end - start <= horizon {
                            return false;
                        }
                        let prefix = &data[start..end - horizon];
                        let extreme = match mode {
                            Extremum::Minimum => *prefix.iter().min().unwrap(),
                            Extremum::Maximum => *prefix.iter().max().unwrap(),
                        };
                        data[end - horizon..end].iter().all(|&b| match mode {
                            Extremum::Minimum => b >= extreme,
                            Extremum::Maximum => b <= extreme,
                        })
                    });
                    match n {
                        Some(n) => {
                            expected.push(n);
                            start = n
                        }
                        None => break,
                    }
                }
                assert_eq!(cuts(&c, &data, &[data.len()]), expected);
                assert_eq!(
                    cuts(&c.clone().with_acceleration(false), &data, &[31, 67]),
                    expected
                );
            }
            buffered(c, horizon + 1, 1024);
        }
    }
}
#[cfg(feature = "gear64")]
#[test]
fn gear64_matches_wrapping_polynomial_and_resets() {
    use hash_roll::{gear64::Gear64, RollingHash, ToChunkIncr};
    let c = Gear64::default();
    let mut h = c.to_chunk_incr();
    let data = bytes(1024);
    for _ in 0..2 {
        for end in 1..=data.len() {
            let expected = data[end.saturating_sub(64)..end]
                .iter()
                .fold(0u64, |v, &b| {
                    (v << 1).wrapping_add(hash_roll::gear_table::GEAR_64[b as usize])
                });
            assert_eq!(h.roll_byte(data[end - 1]), expected);
        }
        h.reset();
        assert_eq!(h.digest(), 0);
    }
}
#[cfg(feature = "fastcdc2020")]
#[test]
fn fastcdc2020_matches_independent_reference_crate() {
    use hash_roll::{fastcdc2020::FastCdc2020, ChunkSizes};
    for (min, avg, max) in [(64, 256, 1024), (2048, 8192, 65536)] {
        let c = FastCdc2020::new(ChunkSizes::new(min, avg, max), 1);
        for data in [bytes(262147), vec![0; 262147], vec![255; 262147]] {
            let mut expected: Vec<_> = fastcdc::v2020::FastCDC::new(&data, min, avg, max)
                .map(|v| v.offset + v.length)
                .collect();
            // The reference iterator emits its incomplete tail, too.
            let last = *expected.last().unwrap();
            let prev = expected.iter().rev().nth(1).copied().unwrap_or(0);
            if c.find_chunk_edge(&mut c.to_search_state(), &data[prev..last])
                .0
                .is_none()
            {
                expected.pop();
            }
            assert_eq!(cuts(&c, &data, &[113, 1, 8192]), expected);
        }
        buffered(c, min, max);
    }
}
#[cfg(feature = "seqcdc")]
#[test]
fn seqcdc_direction_ties_skipping_and_limits() {
    use hash_roll::{
        seqcdc::{Direction, SeqCdc},
        ChunkSizes,
    };
    for direction in [Direction::Increasing, Direction::Decreasing] {
        let c = SeqCdc::new(ChunkSizes::new(8, 32, 128), 3, 2, 7, direction);
        let mut data = vec![100; 5];
        data.extend_from_slice(match direction {
            Direction::Increasing => &[101, 101, 102, 103],
            Direction::Decreasing => &[99, 99, 98, 97],
        });
        assert_eq!(cuts(&c, &data, &[1]), vec![9]);
        assert_eq!(cuts(&c, &[1; 129], &[1]), vec![128]);
        buffered(c, 8, 128);
    }
}
#[cfg(feature = "leap")]
#[test]
fn leap_fragmentation_and_hard_maximum() {
    buffered(
        hash_roll::leap::Leap::new(hash_roll::ChunkSizes::new(204, 512, 1024)),
        204,
        1024,
    );
}
#[cfg(feature = "ultracdc")]
#[test]
fn ultracdc_repeat_fast_path_and_lookahead_fragmentation() {
    let c = hash_roll::ultracdc::UltraCdc::new(hash_roll::ChunkSizes::new(16, 128, 1024));
    assert_eq!(cuts(&c, &vec![7; 537], &[1]), vec![536]);
    buffered(c, 16, 1024);
}
#[cfg(feature = "tttd")]
#[test]
fn tttd_uses_last_secondary_before_max_and_reprocesses_lookahead() {
    use hash_roll::{rabin::DEFAULT_POLYNOMIAL, tttd::Tttd, ChunkSizes};
    let c = Tttd::new(DEFAULT_POLYNOMIAL, 8, ChunkSizes::new(16, 64, 128), 8, 3);
    let data = bytes(16384);
    let (mut start, mut out) = (0, Vec::new());
    let mut fallback_seen = false;
    while data.len() - start >= 16 {
        let end = (start + 128).min(data.len());
        let mut primary = None;
        let mut secondary = None;
        for n in start + 16..=end {
            let h = gf2(&data[n - 8..n], DEFAULT_POLYNOMIAL);
            if h & 255 == 0 {
                primary = Some(n);
                break;
            }
            if h & 7 == 0 {
                secondary = Some(n)
            }
        }
        let cut = primary.or_else(|| {
            if end - start == 128 {
                fallback_seen |= secondary.is_some();
                Some(secondary.unwrap_or(end))
            } else {
                None
            }
        });
        match cut {
            Some(n) => {
                out.push(n);
                start = n
            }
            None => break,
        }
    }
    assert!(fallback_seen);
    assert_eq!(cuts(&c, &data, &[7, 29]), out);
    buffered(c, 16, 128);
}
#[cfg(feature = "supercdc")]
#[test]
fn supercdc_history_is_source_local_and_fragmentation_independent() {
    use hash_roll::{supercdc::SuperCdc, ChunkSizes};
    buffered(SuperCdc::new(ChunkSizes::new(16, 64, 256), 4), 16, 256);
    buffered(SuperCdc::new(ChunkSizes::new(16, 64, 256), 0), 16, 256);
}

#[cfg(feature = "ultracdc")]
#[test]
fn ultracdc_matches_upstream_c99_cut_vectors() {
    // sleepybishop/ultracdc cut(), generated with this file's xorshift fixture.
    // Restrict to full max-sized windows: its EOF API forces an incomplete tail.
    let c = hash_roll::ultracdc::UltraCdc::new(hash_roll::ChunkSizes::new(16, 128, 1024));
    let data = bytes(32771);
    let expected = vec![
        1024, 1057, 2081, 3105, 4129, 5153, 6177, 7201, 7830, 8854, 9878, 10902, 11926, 12950,
        13873, 14897, 15921, 16945, 17969, 18798, 19822, 20846, 21100, 22124, 22791, 23815, 24839,
        25863, 26088, 27112, 27933, 28957, 29981, 31005, 32029,
    ];
    let actual: Vec<_> = cuts(&c, &data, &[3, 128])
        .into_iter()
        .take(expected.len())
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn chunk_size_validation_rejects_empty_and_unordered_limits() {
    for (min, avg, max) in [(0, 1, 2), (4, 2, 8), (1, 9, 8)] {
        assert!(std::panic::catch_unwind(|| hash_roll::ChunkSizes::new(min, avg, max)).is_err());
    }
}

#[cfg(feature = "seqcdc")]
#[test]
fn seqcdc_matches_window_oracle_with_and_without_content_skips() {
    use hash_roll::{
        seqcdc::{Direction, SeqCdc},
        ChunkSizes,
    };
    let data = bytes(16384);
    for direction in [Direction::Increasing, Direction::Decreasing] {
        for skip in [0, 7, usize::MAX] {
            let c = SeqCdc::new(ChunkSizes::new(16, 64, 256), 4, 3, skip, direction);
            let mut expected = Vec::new();
            let mut start = 0;
            while data.len() - start >= 16 {
                let end = data.len().min(start + 256);
                let mut scan = start + 12;
                let mut run_start = scan;
                let mut opposite = 0;
                let mut cut = None;
                while scan < end {
                    if scan > run_start {
                        let a = data[scan - 1];
                        let b = data[scan];
                        let good = match direction {
                            Direction::Increasing => b > a,
                            Direction::Decreasing => b < a,
                        };
                        if !good {
                            run_start = scan;
                            if b != a {
                                opposite += 1;
                            }
                        }
                    }
                    if scan + 1 >= start + 16 && scan + 1 - run_start >= 4 {
                        cut = Some(scan + 1);
                        break;
                    }
                    if opposite == 3 {
                        scan = scan.saturating_add(skip);
                        run_start = scan.saturating_add(1);
                        opposite = 0;
                    }
                    scan = scan.saturating_add(1);
                }
                if cut.is_none() && end - start == 256 {
                    cut = Some(end);
                }
                match cut {
                    Some(n) => {
                        expected.push(n);
                        start = n
                    }
                    None => break,
                }
            }
            assert_eq!(cuts(&c, &data, &[13, 7]), expected);
        }
    }
}
