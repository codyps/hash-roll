#![cfg(feature = "gzip")]
use hash_roll::Splitter;

#[test]
fn simple() {
    let x = hash_roll::gzip::GzipRsyncable::with_window_and_modulus(4, 8);
    let d_ = b"32266fsdasas";
    let d = d_;
    let mut c = 0;
    let l = x.find_chunk_edge(d);
    c += l;
    assert_eq!(l, 5);
    let d = &d[l..];
    let l = x.find_chunk_edge(d);
    c += l;
    assert_eq!(l, 7);

    assert_eq!(c, d_.len());

    let d = &d[l..];
    let l = x.find_chunk_edge(d);
    c += l;
    assert_eq!(d.len(), 0);
    assert_eq!(l, 0);

    // if nothing remains, we'll keep returning 0 (ie: no split)
    let d = &d[l..];
    let l = x.find_chunk_edge(d);
    assert_eq!(l, 0);

    // we've generated blocks that cover the entire input
    assert_eq!(c, d_.len());
}
