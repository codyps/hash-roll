use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use rand::Rng;
use std::hint::black_box;

fn datasets() -> Vec<(&'static str, Vec<u8>)> {
    let mut random = vec![0; 8 << 20];
    rand_pcg::Pcg64::new(0, 1).fill_bytes(&mut random);
    let repeating = b"0123456789abcdef"
        .iter()
        .copied()
        .cycle()
        .take(1 << 20)
        .collect();
    let source = [
        include_bytes!("../README.md").as_slice(),
        include_bytes!("../src/lib.rs").as_slice(),
        include_bytes!("../src/bup.rs").as_slice(),
        include_bytes!("../src/buzhash.rs").as_slice(),
        include_bytes!("../src/fastcdc.rs").as_slice(),
        include_bytes!("../src/zpaq.rs").as_slice(),
        include_bytes!("../src/zstd.rs").as_slice(),
    ]
    .concat();
    let mut edited = random.clone();
    let middle = edited.len() / 2;
    edited.splice(middle..middle + 17, b"replacement text".iter().copied());
    let deletion = edited.len() * 3 / 4;
    edited.drain(deletion..deletion + 29);
    edited.splice(..0, b"inserted prefix\n".iter().copied());
    vec![
        ("random", random),
        ("zeros", vec![0; 1 << 20]),
        ("repeating", repeating),
        ("source", source),
        ("edited", edited),
    ]
}

// Describe complete chunks separately from the final incomplete tail. These
// calculations and all corpus generation run outside the timed measurements.
fn report(name: &str, corpus: &str, mut lengths: Vec<usize>, tail: usize) {
    if lengths.is_empty() {
        eprintln!("{name}/{corpus}: complete=0 tail={tail}");
        return;
    }
    lengths.sort_unstable();
    let count = lengths.len();
    let mean = lengths.iter().sum::<usize>() as f64 / count as f64;
    eprintln!(
        "{name}/{corpus}: complete={count} min={} mean={mean:.1} p50={} p95={} max={} tail={tail}",
        lengths[0],
        lengths[(count - 1) / 2],
        lengths[(count - 1) * 95 / 100],
        lengths[count - 1]
    );
}

#[cfg(any(
    feature = "bup",
    feature = "buzhash",
    feature = "fastcdc",
    feature = "gear",
    feature = "gzip",
    feature = "mii",
    feature = "pigz",
    feature = "ram",
    feature = "zpaq",
    feature = "zstd"
))]
fn compare<C: hash_roll::Chunk + hash_roll::ToChunkIncr>(
    c: &mut Criterion,
    name: &str,
    chunker: C,
    corpora: &[(&str, Vec<u8>)],
) {
    use hash_roll::ChunkIncr;
    use std::collections::HashSet;

    let original = &corpora[0].1;
    let original_chunks: HashSet<&[u8]> = chunker.to_chunk_incr().iter_slices(original).collect();
    let edited = &corpora[4].1;
    let reused: usize = chunker
        .to_chunk_incr()
        .iter_slices(edited)
        .filter(|chunk| original_chunks.contains(chunk))
        .map(|chunk| chunk.len())
        .sum();
    eprintln!(
        "{name}/edited: reusable_bytes={reused}/{} ({:.1}%)",
        edited.len(),
        100.0 * reused as f64 / edited.len() as f64
    );

    let mut group = c.benchmark_group(name);
    for (corpus, data) in corpora {
        let mut iter = chunker.to_chunk_incr().iter_slices_strict(data);
        let lengths: Vec<_> = iter.by_ref().map(|chunk| chunk.len()).collect();
        report(name, corpus, lengths, iter.take_rem().len());
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(BenchmarkId::new("slice", corpus), data, |b, data| {
            b.iter_batched(
                || chunker.to_search_state(),
                |mut state| {
                    let mut remaining = black_box(data.as_slice());
                    let mut chunks = 0;
                    loop {
                        let (cut, discard) = chunker.find_chunk_edge(&mut state, remaining);
                        if cut.is_none() {
                            break;
                        }
                        chunks += 1;
                        remaining = &remaining[discard..];
                    }
                    black_box(chunks)
                },
                BatchSize::SmallInput,
            );
        });
        group.bench_with_input(BenchmarkId::new("incremental", corpus), data, |b, data| {
            b.iter_batched(
                || chunker.to_chunk_incr(),
                |mut incr| {
                    let mut chunks = 0;
                    for block in black_box(data.as_slice()).chunks(4096) {
                        let mut remaining = block;
                        while let Some(cut) = incr.push(remaining) {
                            chunks += 1;
                            remaining = &remaining[cut..];
                        }
                    }
                    black_box(chunks)
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn reference(c: &mut Criterion, corpora: &[(&str, Vec<u8>)]) {
    let mut group = c.benchmark_group("bup_reference");
    for (corpus, data) in corpora {
        let mut chunker = rollsum::Bup::default();
        let mut remaining = data.as_slice();
        let mut lengths = Vec::new();
        while let Some((cut, _)) = chunker.find_chunk_edge(remaining) {
            lengths.push(cut);
            remaining = &remaining[cut..];
            rollsum::Engine::reset(&mut chunker);
        }
        report("bup_reference", corpus, lengths, remaining.len());
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(BenchmarkId::new("rollsum", corpus), data, |b, data| {
            b.iter_batched(
                rollsum::Bup::default,
                |mut chunker| {
                    let mut remaining = black_box(data.as_slice());
                    let mut chunks = 0;
                    while let Some((cut, _)) = chunker.find_chunk_edge(remaining) {
                        chunks += 1;
                        remaining = &remaining[cut..];
                        rollsum::Engine::reset(&mut chunker);
                    }
                    black_box(chunks)
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn benchmarks(c: &mut Criterion) {
    let corpora = datasets();
    #[cfg(feature = "bup")]
    compare(c, "bup", hash_roll::bup::RollSum::default(), &corpora);
    reference(c, &corpora);
    #[cfg(feature = "buzhash")]
    compare(
        c,
        "buzhash",
        hash_roll::buzhash::BuzHash::new_nom(0),
        &corpora,
    );
    #[cfg(feature = "fastcdc")]
    compare(
        c,
        "fastcdc",
        hash_roll::fastcdc::FastCdc::default(),
        &corpora,
    );
    #[cfg(feature = "gear")]
    compare(c, "gear", hash_roll::gear::Gear32::default(), &corpora);
    #[cfg(feature = "gzip")]
    compare(
        c,
        "gzip",
        hash_roll::gzip::GzipRsyncable::default(),
        &corpora,
    );
    #[cfg(feature = "mii")]
    compare(c, "mii", hash_roll::mii::Mii::default(), &corpora);
    #[cfg(feature = "pigz")]
    compare(
        c,
        "pigz",
        hash_roll::pigz::PigzRsyncable::default(),
        &corpora,
    );
    #[cfg(feature = "ram")]
    compare(c, "ram", hash_roll::ram::Ram::with_w(8192), &corpora);
    #[cfg(feature = "zpaq")]
    compare(c, "zpaq", hash_roll::zpaq::Zpaq::default(), &corpora);
    #[cfg(feature = "zstd")]
    compare(c, "zstd", hash_roll::zstd::Zstd::default(), &corpora);
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
