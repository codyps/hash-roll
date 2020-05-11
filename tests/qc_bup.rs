use quickcheck::quickcheck;

use hash_roll::Splitter;

quickcheck! {
    fn simple_eq(xs: Vec<u8>) -> bool {
        let m1 = hash_roll::Bup::default();
        let mut m2 = rollsum::Bup::default();

        let v1 = m1.find_chunk_edge(&xs);
        let v2 = m2.find_chunk_edge(&xs).unwrap_or(0);

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

fn chk_a(x: &[u8]) {
    let m1 = hash_roll::Bup::default();
    let mut m2 = rollsum::Bup::default();

    let v1 = m1.find_chunk_edge(&x);
    let v2 = m2.find_chunk_edge(&x).unwrap_or(0);

    assert_eq!(v1,v2);
}

fn chk_b(x: &[u8]) {
    use rollsum::Engine;
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


#[test]
fn simple_eq_1() {
    chk_a(&[92, 6, 28, 35, 68, 82, 35, 71, 34, 19, 9, 45, 97, 17, 11, 6, 53, 39, 93, 49, 29, 17, 37, 6, 39]);
}

#[test]
fn simple_eq_1b() {
    chk_b(&[92, 6, 28, 35, 68, 82, 35, 71, 34, 19, 9, 45, 97, 17, 11, 6, 53, 39, 93, 49, 29, 17, 37, 6, 39]);
}

#[test]
fn simple_eq_2() {
    chk_a(&[67, 3, 23, 73, 86, 64, 26, 25, 81, 53, 26, 82, 98, 86, 28]);
}

#[test]
fn simple_eq_3() {
    chk_a(&[40, 58, 57, 0, 16, 2, 32, 88, 0, 22, 23, 74, 90, 88, 95, 99, 86]);
}
