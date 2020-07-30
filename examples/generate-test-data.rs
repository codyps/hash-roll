use rand::RngCore;
use rand_pcg::Pcg64;
use std::io::Write;

fn main() {
    let mut args = std::env::args();
    if args.len() != 2 {
        panic!("need 2 args");
    }

    let _ = args.next().unwrap();
    let seed: u128 = args.next().unwrap().parse().unwrap();

    let mut fill_rng = Pcg64::new(seed, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);
    let mut buf = [0u8; 8192 * 4];
    fill_rng.fill_bytes(&mut buf[..]);
    std::io::stdout().lock().write_all(&buf[..]).unwrap();
}
