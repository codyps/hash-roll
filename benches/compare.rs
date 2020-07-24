/*
 */
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hash_roll::{Chunk, ChunkIncr, Splitter};
use rand;

/*
pub fn split_hashmap<F, I>(b: &mut Criterion, bytes: usize, init: F)
    where F: Fn(&[u8]) -> I,
          I: FnMut() -> Option<usize>
{
    use rand::RngCore;
    use std::collections::HashMap;
    let mut rng = rand::thread_rng();
    let mut d = vec![0u8; bytes];
    let mut lenghts = HashMap::new();
    b.iter(|| {
        rng.fill_bytes(&mut d);

        let mut i = init(&d[..]);
        loop {
            match i() {
                Some(l) => {
                    let mut v = lenghts.entry(l).or_insert(0u64);
                    *v = *v + 1;
                },
                None => {
                    break;
                }
            }
        }
    })

    /* TODO: analize length data */
}
*/

/*
pub fn split<F>(b: &mut Criterion, bytes: usize, _name: &'static str, init: F)
    where for<'a> F: Fn(&'a [u8]) -> Box<FnMut() -> Option<u64> + 'a>
{
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut d = vec![0u8; bytes];
    b.iter(|| {
        rng.fill_bytes(&mut d);
        let mut i = test::black_box(init(&d[..]));
        loop {
            match test::black_box(i()) {
                None => {
                    break;
                },
                _ => {},
            }
        }
    });
}
*/

pub fn split_histogram<F>(c: &mut Criterion, bytes: usize, name: &'static str, init: F)
where
    for<'a> F: Fn(&'a [u8]) -> Box<dyn FnMut() -> Option<u64> + 'a>,
{
    use histogram::*;
    use rand::RngCore;
    use rand::SeedableRng;
    let mut rng = rand_pcg::Pcg64::from_rng(rand::thread_rng()).unwrap();
    let mut d = vec![0u8; bytes];
    rng.fill_bytes(&mut d);
    let mut lenghts = Histogram::new();
    c.bench_function(name, |b| {
        b.iter(|| {
            let mut i = black_box(init(&d[..]));
            loop {
                match black_box(i()) {
                    Some(l) => {
                        lenghts.increment(l).unwrap();
                    }
                    None => {
                        break;
                    }
                }
            }
        })
    });

    /* FIXME: for some reason cargo runs this outer code many times over instead of just running
     * the inner code many times over, causing this info to be printied far to much.
     */
    /*
    println!("{}({} bytes) p50: {} bytes, p90: {} bytes, p99: {} bytes, p999: {} bytes",
        name, bytes,
        lenghts.percentile(50.0).unwrap(),
        lenghts.percentile(90.0).unwrap(),
        lenghts.percentile(99.0).unwrap(),
        lenghts.percentile(99.9).unwrap(),
    );
    */
}

/* 4 MiB */
const BENCH_BYTES: usize = 1024 * 1024 / 2;

//const BENCH_RANGE : Range<usize> = Range { first: Bound::Unbounded, last: Bound::Unbounded };

fn bench_rsyncable_vecs(c: &mut Criterion) {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut d = vec![0u8; BENCH_BYTES];
    c.bench_function("rsyncable vecs", |b| {
        b.iter(|| {
            rng.fill_bytes(&mut d);
            let s = hash_roll::rsyncable::Rsyncable::default().into_vecs(d.iter().cloned());
            for _ in s {}
        })
    });
}

fn bench_rsyncable_slices(c: &mut Criterion) {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut d = vec![0u8; BENCH_BYTES];
    c.bench_function("rsyncable slices", |b| {
        b.iter(|| {
            rng.fill_bytes(&mut d);
            let s = hash_roll::rsyncable::Rsyncable::default().into_slices(&d[..]);
            for _ in s {}
        })
    });
}

fn bench_zpaq(b: &mut Criterion) {
    split_histogram(b, BENCH_BYTES, "bench_zpaq", |data| {
        let z = hash_roll::zpaq::Zpaq::default();
        let mut c = &data[..];
        Box::new(move || {
            let (a, b) = z.split(c);
            if b.is_empty() || a.is_empty() {
                None
            } else {
                c = b;
                Some(b.len() as u64)
            }
        })
    });
}

fn bench_zpaq_iter_slice(b: &mut Criterion) {
    split_histogram(b, BENCH_BYTES, "zpaq_iter_slice", |data| {
        let z = hash_roll::zpaq::Zpaq::default();
        let mut zi = z.into_slices(data);
        Box::new(move || zi.next().map(|x| x.len() as u64))
    })
}

fn bench_zpaq_iter_vec(b: &mut Criterion) {
    split_histogram(b, BENCH_BYTES, "zpaq_iter_vec", |data| {
        let z = hash_roll::zpaq::Zpaq::default();
        let mut zi = z.into_vecs(data.iter().cloned());
        Box::new(move || zi.next().map(|x| x.len() as u64))
    })
}

fn bench_rollsum_bup(b: &mut Criterion) {
    split_histogram(b, BENCH_BYTES, "rollsum_bup", |data| {
        let mut z = rollsum::Bup::default();
        let mut pos = 0;
        Box::new(move || {
            let l = z.find_chunk_edge(&data[pos..]).map(|x| (x as u64) + 1);
            match l {
                Some(x) => pos += x as usize,
                None => {}
            }
            l
        })
    })
}

fn bench_bup(b: &mut Criterion) {
    split_histogram(b, BENCH_BYTES, "bup", |data| {
        let mut z = hash_roll::bup::RollSumIncr::default();
        let mut pos = 0;
        Box::new(move || {
            let l = z.push(&data[pos..]);
            match l {
                Some(x) => pos += x,
                None => {}
            }
            l.map(|x| x as u64)
        })
    })
}

criterion_group!(
    benches,
    bench_bup,
    bench_rollsum_bup,
    bench_rsyncable_vecs,
    bench_zpaq,
    bench_zpaq_iter_vec,
    bench_zpaq_iter_slice,
    bench_rsyncable_slices
);
criterion_main!(benches);
