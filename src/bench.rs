/*
 */
use test;
use rand;


/*
pub fn split_hashmap<F, I>(b: &mut test::Bencher, bytes: usize, init: F)
    where F: Fn(&[u8]) -> I,
          I: FnMut() -> Option<usize>
{
    use rand::Rng;
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

pub fn split_histogram<F>(b: &mut test::Bencher, bytes: usize, _name: &'static str, init: F)
    where for<'a> F: Fn(&'a [u8]) -> Box<FnMut() -> Option<u64> + 'a>
{
    use rand::Rng;
    use histogram::*;
    let mut rng = rand::thread_rng();
    let mut d = vec![0u8; bytes];
    let mut lenghts = Histogram::new().unwrap();
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
