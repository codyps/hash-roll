/*
 */
use test;
use rand;
use super::*;
use super::bup::Bup;


/*
pub fn split_hashmap<F, I>(b: &mut test::Bencher, bytes: usize, init: F)
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
pub fn split<F>(b: &mut test::Bencher, bytes: usize, _name: &'static str, init: F)
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

pub fn split_histogram<F>(b: &mut test::Bencher, bytes: usize, _name: &'static str, init: F)
    where for<'a> F: Fn(&'a [u8]) -> Box<FnMut() -> Option<u64> + 'a>
{
    use rand::RngCore;
    use histogram::*;
    let mut rng = rand::thread_rng();
    let mut d = vec![0u8; bytes];
    let mut lenghts = Histogram::new();
    b.iter(|| {
        rng.fill_bytes(&mut d);
        let mut i = test::black_box(init(&d[..]));
        loop {
            match test::black_box(i()) {
                Some(l) => {
                    lenghts.increment(l).unwrap();
                },
                None => {
                    break;
                }
            }
        }
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
const BENCH_BYTES : usize = 1024 * 1024 * 4;

//const BENCH_RANGE : Range<usize> = Range { first: Bound::Unbounded, last: Bound::Unbounded };

#[bench]
fn bench_rsyncable_vecs (b: &mut test::Bencher) {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut d = vec![0u8; BENCH_BYTES];
    b.iter(|| {
        rng.fill_bytes(&mut d);
        let s = Rsyncable::default().into_vecs(d.iter().cloned());
        for _ in s {}
    })
}

#[bench]
fn bench_rsyncable_slices (b: &mut test::Bencher) {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut d = vec![0u8; BENCH_BYTES];
    b.iter(|| {
        rng.fill_bytes(&mut d);
        let s = Rsyncable::default().into_slices(&d[..]);
        for _ in s {}
    })
}

#[bench]
fn bench_zpaq (b: &mut test::Bencher) {
    bench::split_histogram(b, BENCH_BYTES, module_path!(), |data| {
        let z = Zpaq::default();
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

#[bench]
fn bench_zpaq_iter_slice(b: &mut test::Bencher) {
    bench::split_histogram(b, BENCH_BYTES, "zpaq_iter_slice", |data| {
        let z = Zpaq::default();
        let mut zi = z.into_slices(data);
        Box::new(move || {
            zi.next().map(|x| x.len() as u64)
        })
    })
}

#[bench]
fn bench_zpaq_iter_vec(b: &mut test::Bencher) {
    bench::split_histogram(b, BENCH_BYTES, module_path!(), |data| {
        let z = Zpaq::default();
        let mut zi = z.into_vecs(data.iter().cloned());
        Box::new(move || {
            zi.next().map(|x| x.len() as u64)
        })
    })
}

#[bench]
fn bench_rollsum_bup(b: &mut test::Bencher) {
    bench::split_histogram(b, BENCH_BYTES, module_path!(), |data| {
        let mut z = rollsum::Bup::default();
        let mut pos = 0;
        Box::new(move || {
            let l = z.find_chunk_edge(&data[pos..]).map(|x| (x as u64) + 1);
            match l {
                Some(x) => { pos += x as usize },
                None => {},
            }
            l
        })
    })
}

#[bench]
fn bench_bup(b: &mut test::Bencher) {
    bench::split_histogram(b, BENCH_BYTES, module_path!(), |data| {
        let z = Bup::default();
        let mut pos = 0;
        Box::new(move || {
            let l = z.find_chunk_edge(&data[pos..]);
            if l == 0 {
                None 
            } else {
                pos += l;
                Some(l as u64)
            }
        })
    })
}
