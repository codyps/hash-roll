# hash-roll

Provides a generic API for content defined chunking, with implementations of
Bup, BuzHash, FastCDC, Gear, Gzip rsyncable, MII, Pigz rsyncable, RAM, Zpaq,
and Zstd rsyncable chunking.

Requires Rust 1.88 or later. Each algorithm has a Cargo feature of the same name
in lowercase; all algorithms are enabled by default.

```rust
use hash_roll::{ChunkIncr, ToChunkIncr};

let algorithm = hash_roll::fastcdc::FastCdc::default();
let data = b"some input bytes";
let chunks: Vec<&[u8]> = algorithm.to_chunk_incr().iter_slices(data).collect();
assert_eq!(chunks.concat(), data);
```

`iter_slices` includes the final incomplete chunk. Use `iter_slices_strict` when
only complete chunks should be emitted, or `ChunkIncr::push` for fragmented input.
A returned offset is the exclusive end of a chunk in the current input slice.

See [CHANGELOG.md](CHANGELOG.md) for changes to chunk boundaries and licensing.
Run `cargo bench --bench compare` to compare slice and incremental throughput
for every enabled algorithm, plus a `rollsum` Bup reference. Workloads include
seeded random bytes, zeros, repeated patterns, repository source text, and a copy
of the random input with insertions, replacements, and deletions.

The benchmark prints complete-chunk counts, minimum/mean/median/p95/maximum
lengths, incomplete-tail sizes, and the fraction of edited-input bytes reusable
as exact chunks from the original. Corpus generation, these summaries, and
chunker construction run outside the timed measurements. To measure a subset,
use a Criterion filter, for example `cargo bench --bench compare -- zpaq/`.
CI runs `--test` smoke checks; these do not produce performance estimates.

## Metrics

 - DER: duplicate elimination ratio

## CDC References

 - https://www.usenix.org/legacy/events/fast10/tech/full_papers/kruus.pdf

## [Comparison of the Chunking options available in Rust](COMPARE.md)


## License

Copyright (c) 2015-2026 Cody P. Schafer.

Licensed under the Open Software License version 3.0

See [LICENSE-OSL-3.0](LICENSE-OSL-3.0) for the license terms.
Third-party material retains its upstream notices and license terms; see
[THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES).
