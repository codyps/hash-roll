#![cfg(feature = "bfbc")]

//! Bytes frequency-based chunking
//!
//! ## General procedure
//!
//! 1. determine frequency of pairs of bytes in the file
//! 2. select 10 most frequent byte pairs as divisors
//! 3. 
//!
//! ## Performance notes
//!
//! BFBC must examine the data stream twice, once to select divisors and a second time to perform
//! chunking. This is contrary to most other chunking proceedures which are single-pass. As a
//! result, this is a poor fit for our currently implimented APIs which are designed assuming a
//! single pass model and try very hard to minimize retained data (because retaining data with the
//! current APIs generally means copying data through a buffer).
//!
//! ## Reference
//!
//! Data Deduplication System Based on Content-Defined Chunking Using Bytes Pair Frequency
//! Occurrence
//! doi:10.3390@sym12111841
//!
//!

use crate::{ChunkIncr, ToChunkIncr};

/// Parameters for a BFBC instance
struct Bfbc {
    t_min: (),
    divisors: [[u8;2];10],
    t_max: (),
}



