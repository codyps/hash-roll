use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use hash_roll::{Chunk, ChunkIncr, ToChunkIncr};
use rand::Rng;
use std::hint::black_box;

fn compare<C: Chunk + ToChunkIncr>(c: &mut Criterion, name: &str, chunker: C, data: &[u8]) {
    let mut group = c.benchmark_group(name);
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_with_input(BenchmarkId::new("slice", data.len()), data, |b, data| {
        b.iter(|| {
            let mut state = chunker.to_search_state();
            let mut remaining = black_box(data);
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
        });
    });
    group.bench_with_input(
        BenchmarkId::new("incremental", data.len()),
        data,
        |b, data| {
            b.iter(|| {
                let mut chunker = chunker.to_chunk_incr();
                let mut chunks = 0;
                for block in black_box(data).chunks(4096) {
                    let mut remaining = block;
                    while let Some(cut) = chunker.push(remaining) {
                        chunks += 1;
                        remaining = &remaining[cut..];
                    }
                }
                black_box(chunks)
            });
        },
    );
    group.finish();
}

fn benchmarks(c: &mut Criterion) {
    let mut data = vec![0; 1 << 20];
    rand_pcg::Pcg64::new(0, 1).fill_bytes(&mut data);
    #[cfg(feature = "bup")]
    compare(c, "bup", hash_roll::bup::RollSum::default(), &data);
    #[cfg(feature = "bup")]
    {
        let mut group = c.benchmark_group("bup_reference");
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_function("rollsum", |b| {
            b.iter(|| {
                let mut chunker = rollsum::Bup::default();
                let mut remaining = black_box(data.as_slice());
                let mut chunks = 0;
                while let Some((cut, _)) = chunker.find_chunk_edge(remaining) {
                    chunks += 1;
                    remaining = &remaining[cut..];
                    rollsum::Engine::reset(&mut chunker);
                }
                black_box(chunks)
            })
        });
        group.finish();
    }
    #[cfg(feature = "buzhash")]
    compare(c, "buzhash", hash_roll::buzhash::BuzHash::new_nom(0), &data);
    #[cfg(feature = "fastcdc")]
    compare(c, "fastcdc", hash_roll::fastcdc::FastCdc::default(), &data);
    #[cfg(feature = "gear")]
    compare(c, "gear", hash_roll::gear::Gear32::default(), &data);
    #[cfg(feature = "gzip")]
    compare(c, "gzip", hash_roll::gzip::GzipRsyncable::default(), &data);
    #[cfg(feature = "mii")]
    compare(c, "mii", hash_roll::mii::Mii::default(), &data);
    #[cfg(feature = "pigz")]
    compare(c, "pigz", hash_roll::pigz::PigzRsyncable::default(), &data);
    #[cfg(feature = "ram")]
    compare(c, "ram", hash_roll::ram::Ram::with_w(8192), &data);
    #[cfg(feature = "zpaq")]
    compare(c, "zpaq", hash_roll::zpaq::Zpaq::default(), &data);
    #[cfg(feature = "zstd")]
    compare(c, "zstd", hash_roll::zstd::Zstd::default(), &data);
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
