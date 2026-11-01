# hash-roll

Provides a generic API for content defined chunking, with implementations of
Bup, BuzHash, FastCDC, Gear, Gzip rsyncable, MII, Pigz rsyncable, RAM, Zpaq,
Zstd rsyncable, Rabin, AE, Gear64, FastCDC 2020, SeqCDC, UltraCDC, Leap,
SuperCDC, and TTTD. Integer Rabin–Karp is also available as a rolling checksum.

Requires Rust 1.88 or later. Each algorithm has a Cargo feature of the same name
in lowercase (integer Rabin–Karp uses `rabin-karp`); all algorithms are enabled
by default. The optional `simd` feature enables AE AVX2 acceleration on x86-64
with runtime CPU detection and a portable scalar fallback.

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

All CDC algorithms support `Chunk::chunks`, including methods that scan ahead:

```rust
use hash_roll::Chunk;
let algorithm = hash_roll::fastcdc2020::FastCdc2020::default();
let data = b"some input bytes";
let chunks: Vec<_> = algorithm.chunks(data).collect();
assert_eq!(chunks.concat(), data);
```

| Feature / type | APIs | Boundary policy |
| --- | --- | --- |
| `rabin` / `rabin::Rabin` | Slice + incremental | GF(2) window, minimum skip, bit mask, hard maximum |
| `ae` / `ae::Ae` | Slice + incremental | First strict minimum/maximum surviving a horizon; ties preserve it |
| `gear64` / `gear64::Gear64` | Slice + incremental + rolling | 64-bit Gear using the existing rsroll table and high-bit mask |
| `fastcdc2020` / `fastcdc2020::FastCdc2020` | Slice | Reference two-byte cut rule, generated table, spread-bit masks |
| `seqcdc` / `seqcdc::SeqCdc` | Slice + incremental | Strict monotone byte sequences, sub-minimum and content-based skips |
| `ultracdc` / `ultracdc::UltraCdc` | Slice | Popcount mask and repeated-window fast path; experimental |
| `leap` / `leap::Leap` | Slice + incremental | 24-window judgment with safe leaps; experimental |
| `supercdc` / `supercdc::SuperCdc` | Slice | Normalized Gear, fallback, bounded history prediction; experimental |
| `tttd` / `tttd::Tttd` | Slice | Primary Rabin mask, last secondary fallback at maximum; experimental |
| `rabin-karp` / `rabin_karp::RabinKarp` | Rolling only | librsync seeded integer polynomial modulo 2^32 |

`RollingHash` exposes `roll_byte`, `digest`, and `reset`. Rabin's standalone
fingerprint is `rabin::RabinFingerprint`; Gear64's incremental state also
implements this trait. These hashes are not cryptographic content digests.

`ChunkSizes` validates positive, ordered minimum/target/maximum byte lengths.
A target describes configuration, not a guaranteed observed mean. Algorithms
impose additional constraints in their constructors. AE's minimum is horizon+1;
Gear64 has no minimum or forced maximum. Other new CDC types enforce their
configured minimum and maximum, excluding an incomplete final tail.

FastCDC 2020, UltraCDC, TTTD, and SuperCDC can select a boundary before data they
have already examined. They implement `Chunk`, with retained-buffer extension
through `find_chunk_edge`, but cannot implement the current-buffer-only
`ChunkIncr` contract. Respect the returned discard count when buffering input.
The common `chunks` iterator handles complete inputs and includes the final tail.

Rabin's `restic(polynomial)` preset uses restic's 64-byte window and
512 KiB / 1 MiB / 8 MiB sizes. Pass the polynomial from the repository being
read; the default polynomial is a published test polynomial, not a repository
secret. FastCDC 2020 matches the reference v2020 cut positions for even sizes,
power-of-two targets, and the same normalization. The original `fastcdc` and
`gear` modules retain their existing tables and cut rules.

The experimental algorithms are configurable research implementations, not
interchangeable on-disk format presets. SuperCDC uses a bounded FIFO cache from
a boundary fingerprint to the next chunk length; predictions cover one chunk
and are not recursively learned. Its low-bit masks, first weak fallback after
target, and cache policy define this variant. SeqCDC counts sequence length in
bytes, resets on equality, and counts only opposing comparisons toward skips.
Both specify their policy rather than claiming compatibility with every
published scalar/vector implementation. Create fresh search state for each
independent source, especially when using SuperCDC history.

References and third-party table/rule notices are linked in each module and
[THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES). No new runtime dependencies are needed.

See [CHANGELOG.md](CHANGELOG.md) for changes to chunk boundaries and licensing.
Run `cargo bench --bench compare` to compare slice and incremental throughput
for every enabled CDC algorithm, plus a `rollsum` Bup reference and a
Rabin–Karp rolling-checksum benchmark. Buffered algorithms have slice benchmarks
only. Use `--all-features` to also compare AE scalar and SIMD dispatch. Workloads include
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
