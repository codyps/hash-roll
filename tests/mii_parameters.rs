#![cfg(feature = "mii")]

use hash_roll::{mii::Mii, Chunk, ChunkIncr, ToChunkIncr};

#[test]
fn all_reachable_increment_counts_work_in_both_apis() {
    let data: Vec<u8> = (0..=255).collect();
    for w in 1..=255 {
        let c = Mii::with_w(w);
        assert_eq!(c.to_chunk_incr().push(&data), Some(w as usize + 1));
        assert_eq!(
            c.find_chunk_edge(&mut c.to_search_state(), &data).0,
            Some(w as usize + 1)
        );
    }
}

#[test]
fn unreachable_increment_counts_are_rejected() {
    for w in [0, 256, 257, u64::MAX] {
        assert!(std::panic::catch_unwind(|| Mii::with_w(w)).is_err());
    }
}
