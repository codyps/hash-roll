use rand::RngCore;
use rand_pcg::Pcg64;
use std::io::Write;

fn main() {
    let mut args = std::env::args();
    if args.len() != 3 {
        eprintln!("usage: generate-test-data <seed> <len-kib>");
        std::process::exit(1);
    }

    let _ = args.next().unwrap();
    let seed: u128 = args.next().unwrap().parse().unwrap();
    let len: usize = args.next().unwrap().parse().unwrap();

    let mut fill_rng = Pcg64::new(seed, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);
    // note: original len = 8192 * 4 = 1024 * 32
    let mut buf = vec![0u8; 1024 * len];
    fill_rng.fill_bytes(&mut buf[..]);
    std::io::stdout().lock().write_all(&buf[..]).unwrap();
}
