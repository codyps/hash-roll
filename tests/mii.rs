use hash_roll::Chunk;
use rand_pcg::Pcg64;
use rand::RngCore;

fn cut_test<C: Chunk>(seed: u128, chunker: C, expected_splits: &[usize]) {
    let mut rng = Pcg64::new(seed, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);
    let mut buf = [0u8; 8192*4];
    rng.fill_bytes(&mut buf);

    let mut splits = Vec::with_capacity(expected_splits.len());
    let mut buf = &buf[..];
    loop {
        match chunker.find_chunk_edge(None, &buf[..]) {
            Ok(split_point) => {
                splits.push(split_point);
                buf = &buf[split_point..];
            },
            Err(_) => {
                break;
            }
        }
    }

    assert_eq!(expected_splits, &splits[..]);
}

#[test]
fn cuts_1b() {
    cut_test(0, 
        hash_roll::mii::Mii::default(),
        &[1211, 40, 261, 1548, 1881, 312, 2043, 285, 1062, 677, 542, 1473, 303, 172, 318, 839, 2560, 3242, 396, 202, 123, 898, 2454, 544, 3541, 571, 483, 383, 103, 2629, 929, 47, 524]
    );
}
