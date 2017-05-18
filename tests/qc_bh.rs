extern crate hash_roll;
#[macro_use]
extern crate quickcheck;

/*
#[derive(Debug,Clone,PartialEq,Eq)]
struct Fma {
    data: Vec<u8>,
    msize: usize,
    moffs: usize,
}

impl quickcheck::Arbitrary for Fma {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        // lenght at least 1
        let d = {
            let mut x = g.gen();
            while x.len() == 0 {
                x = g.gen();
            }
        };

        // 1 to d.len()
        let s = if d.len() == 1 {
            1
        } else {
            (g.gen() % (d.len() - 1)) + 1
        };

        // 0 to (d.len() - s)
        let o = if d.len() - s == 0 {
            0
        } else {
            g.gen() % (d.len() - s)
        };


        Fma {
            data: d,
            msize: s,
            moffs: o,
        }
    }

    fn shrink(&self) -> Box<Iterator<Item=Self>> {
        
    }
}
*/

quickcheck! {
    // choose a substring of `data` and use buzhash to find it
    fn find_match(data: Vec<u8>, size: usize, offs: usize)  -> bool {
        // d.len() > 0
        if data.len() == 0 {
            return true
        }
        // 1..d.len()
        let size = if size == 0 {
            1
        } else {
            if data.len() == 1 {
                1
            } else {
                (size % (data.len() - 1)) + 1
            }
        };


        let offs = if offs == 0 {
            0
        } else {
            if data.len() - size == 0 {
                0
            } else {
                offs % (data.len() - size)
            }
        };
        let ms = &data[offs..(offs+size)];
        println!("size: {}, offs: {}", size, offs);
        let mut b = ::hash_roll::buzhash::BuzHashBuf::with_capacity(size);
        let mut b2 = b.clone();

        b.push(ms);
        let h = b.hash();

        b2.find_match(h, &data[..]) == offs + size
    }
}
