
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
 - MII (minimal incrimental interval)
   - doi:10.1109/access.2019.2926195 
 - 

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

 - Incrimental input: rather than require a single `&[u8]` up front, allow
   providing a number of `&[u8]`s over the life of the splitter/hasher.

 - Slice input vs byte-at-a-time: By allowing algorithms to take in larger
   slices of data at a time, it enables them to potentially impliment
   optimizations to speed up computation.

# Implimentations

 - [cdc](https://lib.rs/crates/cdc)
   - algorithm(s): "Rabin64" (polynomial based, 64-bit)
   - low level trait is byte-at-a-time
   - provides both rolling hash and content splitting features
   - includes multiple iterator adapters
   - lacks completeness in documentation
 - [fastcdc](https://lib.rs/crates/fastcdc)
   - algorithm(s): FastCDC
   - incrimental input: no
   - api: iterator over slices
   - single struct, no traits
 - [quickcdc](https://lib.rs/crates/quickcdc)
   - algorithm(s): AE (with modifications/extensions)
   - incrimental input: no
   - api: iterator over slices
   - single struct, no traits
 - [gearhash](https://docs.rs/gearhash)
   - algorithm(s): gear
   - used in FastCDC
   - provides simd & scalar impls
   - includes a static table for gearhash
   - `Hasher` trait provides both content splitting and rolling hash features.
     - in-place splitting
     - lacks helpers present in `cdchunking`
 - [cdchunking](https://docs.rs/crate/cdchunking)
   - algorithm(s): Zpaq
   - provides 3 seperate mechanisms for splitting using the same underlying splitter api
 - [rededup-cdc](https://docs.rs/crate/rdedup-cdc)
   - `rollsum` fork
 - [bitar](https://crates.io/crates/bitar)
   - active development (as of v0.6.4 release on 2020-05-07)
   - algorithms(s): BuzHash, RollSum
   - low-level trait for each hash is byte-at-a-time
   - many other items included in the library (designed to support the cmdline tool `bita`)
 - [zvault](https://github.com/dswd/zvault)
   - algorithm(s): AE, fastcdc, rabin, fixed (non content defined)
   - low level trait requires a Read & a Write instance
   - provides run-time generic over creation & extraction of some details (`Chunker`)
   - Instantiation for each provides a seed and average size
   - inactive development (last change 2018-03-08 (as of 2020-05-10))
   - includes many non-chunking items
