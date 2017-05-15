extern crate rollsum;
extern crate hash_roll;
#[macro_use]
extern crate quickcheck;

use hash_roll::Splitter;

quickcheck! {
    fn simple_eq(xs: Vec<u8>) -> bool {
        let m1 = hash_roll::Bup::default();
        let mut m2 = rollsum::Bup::default();

        let v1 = m1.find_chunk_edge(&xs);
        let v2 = m2.find_chunk_edge(&xs).map(|x| x+1).unwrap_or(0);

        v1 == v2
    }

    fn iter_eq(xs: Vec<u8>) -> bool {
        let m1 = hash_roll::Bup::default();
        let mut m2 = rollsum::Bup::default();

        let mut x = &xs[..];
        loop {
            let v1 = m1.find_chunk_edge(&x);
            let v2 = m2.find_chunk_edge(&x).unwrap_or(0);

            if v1 != v2 {
                return false
            }

            if v1 == 0 {
                return true
            }

            x = &x[v1..];
            if x.len() == 0 {
                return true
            }
        }
    }
}

#[test]
fn simple_eq_1() {
    let x = [92, 6, 28, 35, 68, 82, 35, 71, 34, 19, 9, 45, 97, 17, 11, 6, 53, 39, 93, 49, 29, 17, 37, 6, 39];
    let m1 = hash_roll::Bup::default();
    let mut m2 = rollsum::Bup::default();

    let v1 = m1.find_chunk_edge(&x);
    let v2 = m2.find_chunk_edge(&x).unwrap_or(0);

    assert_eq!(v1,v2);
}

#[test]
fn simple_eq_1b() {
    use rollsum::Engine;
    let x = [92, 6, 28, 35, 68, 82, 35, 71, 34, 19, 9, 45, 97, 17, 11, 6, 53, 39, 93, 49, 29, 17, 37, 6, 39];
    let mut m1 = hash_roll::bup::RollSum::default();
    let mut m2 = rollsum::Bup::default();
    let cm = (1<<rollsum::bup::CHUNK_BITS) - 1;

    for (i, &v) in x.iter().enumerate() {
        m1.roll_byte(v);
        m2.roll_byte(v);
        println!("i={}, v={}", i, v);
        assert_eq!(m1.digest(), m2.digest());
        assert_eq!(m1.at_split(), (m2.digest() & cm) == cm);
    }
}
