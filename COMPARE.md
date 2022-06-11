
# Algorithms

 - Gear
 - FastCDC
 - Zpaq
 - AE
 - RollSum
 - BuzHash
 - LMC (chunking)
 - RAM Chunking (Rapid  Asymmetric  Maximum)
   - doi:10.1016/j.future.2017.02.013 
 - MII (minimal incremental interval)
   - doi:10.1109/access.2019.2926195 
 - [TTTD](https://scholarworks.sjsu.edu/cgi/viewcontent.cgi?referer=&httpsredir=1&article=1041&context=etd_projects)
 - [FBC](doi:10.1109/mascots.2010.37)

# Algorithm Points of Comparison

 - Ability to constrain block size
   - distribution
   - tuneability of distribution
 - Speed
   - on different distributions
 - Common chunk discovery
   - on different distributions
 - Common chuck discovery after a byte shift
   - on different distributions
 - Common chuck discovery after edit
   - on different data distributions
   - under different edit kinds

# Impl Features

 - incremental input: rather than require a single `&[u8]` up front, allow
   providing a number of `&[u8]`s over the life of the splitter/hasher.

 - Slice input vs byte-at-a-time: By allowing algorithms to take in larger
   slices of data at a time, it enables them to potentially impliment
   optimizations to speed up computation.

# Implimentations

 - [cdc](https://lib.rs/crates/cdc)
   - latest release: 2017-09-09
     - inactive development (as of 2020-06-21)
   - algorithm(s): "Rabin64" (polynomial based, 64-bit)
   - incremental input: no
     - no documentation indicates incremental input is possible
     - while one could use a special impl of `Iterator<Item=u8>` that can be
       extended, this would only work if the `SeperatorIter` or `ChunkIter` had
       not emitted a final incomplete chunk/seperator.
   - includes `RollingHash64` trait
   - structure includes mutable context, no non-mutable representation
   - input format(s): `Iterator<Item=u8>`, `u8`
     - may limit performance capability
   - input is fully buffered by cdc structures
   - provides both rolling hash and content splitting features
   - has _explicit_ representation for "prefilling" of the rolling hash.
   - includes multiple iterator adapters
     - splits the concept of a "seperator" (index + hash) vs a "chunk" (index +
       hash + size).
     - iterator adaptors don't generalize over rolling hashes, they are
       hard-coded to the `Rabin64` impl
   - documentation is lacking (almost universally missing)
 - [fastcdc](https://lib.rs/crates/fastcdc)
   - latest release: 2020-03-19, v1.0.3
     - active development (as of 2020-06-21)
   - algorithm(s): FastCDC
   - incremental input: no
   - api:
     - input: one `&[u8]`
     - output: `Iterator<Item=Chunk> where Chunk: (offset: usize, size:
       usize)`. Returns the remaining chunk as the last item even if not
       considered a complete chunk.
   - only struct mixes mutable and immutable data, no configuration representation
   - "chunks" are an offset and a size
     - iow: no rolling hash support
   - single struct, no traits
   - provides a fixed table for fastcdc (generated via a reproducable mechanism initially)
 - [quickcdc](https://lib.rs/crates/quickcdc)
   - latest release: 2018-12-17 v1.0.0 (no other releases)
     - inactive development (as of 2020-06-21)
   - algorithm(s): AE (with modifications/extensions)
   - incremental input: no
   - api:
     - input: one `&[u8]`
     - output: `Iterator<Item=&[u8]>`
   - no struct representation of configuration (only mixes mutable and immutable)
   - api: iterator over slices
   - single struct, no traits
   - includes improper use of unsafe in a non-public function (passes pointers
     into a function that dereferences them but the function is not marked
     unsafe).
 - [gearhash](https://lib.rs/crates/gearhash)
   - latest release: 2020-04-12 v0.1.3
     - active development (as of 2020-06-21)
   - algorithm(s): gear
   - incremental input: yes
   - provides simd & scalar impls
   - includes a static table for gearhash
   - api: call `next_match()` repeatedly with new slices. Returns a
     `Option<usize>` indicating where a split point is (if any) in the slice
     passed to `next_match()`.
   - `Hasher` struct provides both content splitting and rolling hash features.
     - in-place splitting
     - lacks helpers present in `cdchunking`
   - single struct, no traits
   - no struct representation of configuration (only mixes mutable and immutable data)
 - [cdchunking](https://lib.rs/crates/cdchunking)
   - latest release: 2019-11-02 v1.0.0
     - inactive development (as of 2020-06-21)
   - algorithm(s): Zpaq
   - provides a chunker-impl trait
   - api: call `next_boundary()` repeatedly with new slices. Returns a
     `Option<usize>` indicating what a split point is (if any) in the slice.
     - must explicitly call a `reset()` after a match to reset internal state
       for subsequent matches.
   - provides a `Chunker` which takes a `ChunkerImpl` and provides a number of ease-of-use apis:
     - from a `Read` into a `Iterator<Item=Result<Vec<u8>>>`
     - from a `Read` into a `Result<Vec<Vec<u8>>>`
     - from a `Read` into a series of one of `Data(&[u8])` or `End`, where the
       `Data(&[u8])` are references to an internal buffer and `End` indicate
       the end of a chunk.
     - from a `Read` to an iterator of (start, len, end) (ie: no data returned)
     - from a `&[u8]` to an `Iterator<Item=[u8]>`
 - [rollsum](https://lib.rs/crates/rollsum) aka [rsroll](https://github.com/aidanhs/rsroll)
   - latest release: (commit 2019-12-22, publish 2020-09-27) v0.3.0
     - uncertain inactive development (as of 2020-10-08)
   - algorithm(s):
     - rollsum (based on bupsplit, based on rsync chunking)
     - gear
   - incremental input: yes
   - includes a static table for gearhash
   - low level trait has byte-by-byte and slice based interfaces
   - exposes conditionality of chunk edge (ie: like a rolling-sum) in trait,
     but provides a helper on the specific struct that uses it's defaults.
   - requires explicit state resets after finding a chunk edge to find the next
     chunk edge (doesn't reset internal state)
   - api: call `find_chunk_edge()` with different slices until Some((usize, Sum)) is
     returned. the `usize` here is the offset after the end of the chunk (ie:
     start of the next chunk).
   - provides access to the underlying Sum on each edge
 - [rededup-cdc](https://lib.rs/crates/rdedup-cdc)
   - `rollsum` fork
 - [bitar](https://lib.rs/crates/bitar)
   - latest release: 2020-06-09 v0.7.0
     - active development (as of 2020-06-21)
   - algorithms(s): BuzHash, RollSum
   - uses enum to abstract over algorithms (`Config` and `Chunker`)
   - includes seperate immutable "configuration object" concept (`Config`)
   - supports/requires use of `tokio::AsyncRead` as input
   - api: provide a `AsyncRead` when constructing the `Chunker`. Use the
     `futures::Stream<Item=Result<(u64, Bytes)>>` it returns
   - low-level trait for each hash is byte-at-a-time
   - many other items included in the library (designed to support the cmdline tool `bita`)
 - [zvault](https://github.com/dswd/zvault)
   - algorithm(s): AE, fastcdc, rabin, fixed (non content defined)
   - low level trait requires a Read & a Write instance
   - provides run-time generic over creation & extraction of some details (`Chunker`)
   - Instantiation for each provides a seed and average size
   - inactive development (last change 2018-03-08 (as of 2020-05-10))
   - includes many non-chunking items
