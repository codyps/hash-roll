# Changelog

## Unreleased

- Add GF(2) Rabin CDC and rolling fingerprints with a restic preset, integer
  Rabin–Karp rolling checksums, AE Min/Max, Gear64, versioned FastCDC 2020,
  SeqCDC, and experimental UltraCDC, Leap, SuperCDC, and TTTD chunkers.
- Add shared `ChunkSizes`, `RollingHash`, and `Chunk::chunks` slice iteration.
  Preserve the retained-buffer contract for algorithms with look-ahead/fallbacks.
- Add optional runtime-detected AE AVX2 acceleration with scalar equivalence tests.
- Check new algorithms against independent arithmetic/cut oracles, reference
  FastCDC and UltraCDC implementations, bytewise fragmented input, hard limits,
  reset behavior, and bounded source-local prediction history.
- Extend corpus benchmarks, individual-feature CI, and upstream notices to the
  added algorithms; keep all new algorithm runtime code dependency-free.

- Normalize Zpaq configuration to byte units, correct its default maximum to
  508 KiB, and respect inclusive and exclusive length bounds. Maximum-based
  constructors now support small and non-power-of-two sizes without floating
  point calculations or overflow. Invalid configurations are rejected explicitly.
- Restrict MII's consecutive-increment parameter to the reachable range 1..=255.
- Independently test Zpaq size guarantees, byte-unit conversions, predictor
  cutoffs, range normalization, and MII's supported parameter boundaries.
- Benchmark multiple input types and edited copies, report chunk-size
  distributions and exact chunk reuse, and exclude setup from timed batches.
- Run release tests plus feature-specific Clippy and documentation checks in CI,
  including benchmarks with no algorithm features enabled.

- License hash-roll's original code under OSL-3.0 and include upstream notices.
- Return exclusive chunk end offsets consistently from both chunking APIs.
  Correct Gear, FastCDC, RAM, and Zstd incremental offsets, including FastCDC's
  forced maximum boundary and Zstd's retained window after a split.
- Reset Bup and MII state consistently across APIs and support non-power-of-two
  Bup windows.
- Correct BuzHash's outgoing-byte rotation for 32-bit hashes and enforce its
  maximum chunk size in both APIs. Correct Zpaq's slice API size accounting.
- Stop strict slice iterators after processing their incomplete remainder once.
- Validate Gear, FastCDC, Bup, BuzHash, Gzip, and Pigz constructor parameters.
- Test fragmented input, empty input, multiple boundaries, independent BuzHash
  window recomputation, and equivalence between both APIs.
- Update development dependencies, repair benchmarks for all ten algorithms,
  and check individual features in CI. The minimum supported Rust version is 1.88.
- Correct documentation links, examples, and the spelling of incremental.

Chunk boundaries can change because of these correctness fixes. Applications
that persist chunk boundaries should account for this when upgrading. Published
versions retain the licenses under which they were distributed.
