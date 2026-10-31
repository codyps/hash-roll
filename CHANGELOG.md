# Changelog

## Unreleased

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
