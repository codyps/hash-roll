use hash_roll::ChunkIncr;

#[derive(Debug, Default)]
struct Counter(usize);

impl ChunkIncr for Counter {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        for (i, _) in data.iter().enumerate() {
            self.0 += 1;
            if self.0 == 3 {
                self.0 = 0;
                return Some(i + 1);
            }
        }
        None
    }
}

#[test]
fn strict_iterator_does_not_feed_the_remainder_twice() {
    let data = b"abcde";
    let mut iter = Counter::default().iter_slices_strict(data);
    assert_eq!(iter.next(), Some(&data[..3]));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);
    let (state, tail) = iter.into_parts();
    assert_eq!(state.0, 2);
    assert_eq!(tail, b"de");
}

#[test]
fn iterators_cover_empty_complete_and_partial_inputs() {
    for len in 0..10 {
        let data = vec![1; len];
        let slices: Vec<_> = Counter::default().iter_slices(&data).collect();
        assert_eq!(slices.concat(), data);
        assert!(slices.iter().all(|s| !s.is_empty()));
        let mut strict = Counter::default().iter_slices_strict(&data);
        assert_eq!(strict.by_ref().count(), len / 3);
        assert_eq!(strict.take_rem().len(), len % 3);
        assert_eq!(strict.next(), None);
    }
}
