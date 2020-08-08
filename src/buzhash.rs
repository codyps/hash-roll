use crate::{Chunk, ChunkIncr, ToChunkIncr};
use std::fmt;
use std::num::Wrapping;
/* Cyclic polynomial (buzhash)
 *
 * H = s ** (k -1) (h(c_1)) ^ s**(k-2)(h(c_2)) ^ ... ^ s(h(c_(k-1))) ^ h(c_k)
 * where s(x) is a barrel shift of x (ABCDEFG becomes BCDEFGA, where each letter is a bit)
 * s**y(x) is application of s(n) y times.
 *
 * Application:
 *
 *  - H <- s(H)
 *  - c_1 <- s**k(h(c_1))
 *  - H <- s(H) ^ s**k(h(c_1)) ^ h(c_(k+1))
 *
 *  Where c_1 is the character to remove,
 *      c_(k+1) is the character to add
 *
 * Parameters:
 *  - k: number of inputs to contain (can be un-capped?)
 *  - h: a hash function from inputs to integers on [2, 2**L)
 *
 * State:
 *  - every input contained in the hash (if removal is required)
 *  - previous hash result
 */

/// Describes an instance of BuzHash (aka cyclic polynomial hash).
///
/// Provides parameterization over the window size (`k`), hash function (`h`), chunk edge mask, and
/// max chunk size.
///
/// Uses fixed 32-bit width for the hash.
///
/// The trait [`BuzHashHash`] provides the internal hash function, see the implimentations of it
/// for built-in hash options (which include both `Borg` and `silvasur/buzhash`'s internal hash
/// tables).
///
/// Note that it's helpful for `k` to be prime to prevent repeating strings form resulting
/// in total cancelation of the internal hash, which can cause overly long chunks.
///
/// Adjusting `mask` changes the average chunk size.
///
/// BuzHash with various chunk-splitting methods is used in:
///   - [Borg](https://github.com/borgbackup/borg)
///   - [Attic](https://github.com/jborg/attic)
///     - via [silvasur/buzhash](https://github.com/silvasur/buzhash)
///   - [attic-labs/nom](https://github.com/attic-labs/noms/blob/26620a34bc8c95812037588869d4790b5581b34d/go/types/rolling_value_hasher.go#L15-L21)
///
///
/// # Performance
///
/// [`BuzHash`] requires storing bytes equal to it's window size (`k`). Because of this,
/// [`BuzHashIncr`] may have poor performance compared to [`BuzHash::find_chunk_edge()`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuzHash<H: BuzHashHash> {
    /// number of characters to consider at once
    k: usize,

    /// A hash function over a single byte that emits a 32-bit value
    h: H,

    /// the 1 bits indicates the bit in the hash which must be 1 to form a chunk edge
    /// (called `pattern` in `attic-labs/nom`)
    mask: u32,

    /// if the index grows _above_ this size, a chunk edge is formed
    max_chunk_size: u64,
}

impl<H: BuzHashHash> BuzHash<H> {
    /// Create an instance with the given capacity (k) and chunk termination `mask`, and a internal
    /// `hash` function.
    ///
    /// `capacity` is the number of bytes that are taken into account for a given hash.
    /// `mask` affects how chunk edges are determined.
    /// `hash` is applied to each byte of input prior to mixing into the rolling hash.
    pub fn new(capacity: usize, mask: u32, hash: H, max_chunk_size: u64) -> Self {
        assert!(capacity > 0);
        BuzHash {
            k: capacity,
            h: hash,
            mask,
            max_chunk_size,
        }
    }

    // fn new_attic()
    // fn new_bup()
}

impl<'a> BuzHash<BuzHashTableByteSaltHash<'a>> {
    /// Create a buzhash instance using defaults from attic-labs/nom version 7.17
    ///
    /// - `k: 67`
    /// - `hash` is the `silvasur/buzhash` table
    /// - `mask: 1<<12 -1`
    /// - `max_chunk_size: 1 << 24`
    pub fn new_nom(salt: u8) -> Self {
        BuzHash::new(
            67,
            (1 << 12u32) - 1,
            BuzHashTableByteSaltHash::from((salt, &crate::buzhash_table::GO_BUZHASH)),
            1 << 24,
        )
    }
}

impl<H: BuzHashHash + Clone> Chunk for BuzHash<H> {
    type SearchState = BuzHashSearchState;

    fn to_search_state(&self) -> Self::SearchState {
        Self::SearchState::default()
    }

    fn find_chunk_edge(
        &self,
        state: &mut Self::SearchState,
        data: &[u8],
    ) -> (Option<usize>, usize) {
        for i in state.offset..data.len() {
            state.state.add_buf(data, self, i);

            if (state.state.h & self.mask) == self.mask {
                state.reset();
                return (Some(i + 1), i + 1);
            }

            /*
             * broken: `i` is not the number of bytes since prev chunk.
             * need to track internal last chunk
            if i as u64 > self.max_chunk_size {
                state.reset();
                println!(" <- CHUNK: {}", i + 1);
                return (Some(i + 1), i + 1);
            }
            */
        }

        // keep k elements = discard all but k
        let discard_ct = data.len().saturating_sub(self.k);
        state.offset = data.len() - discard_ct;
        (None, discard_ct)
    }
}

impl<H: BuzHashHash + Clone> From<&BuzHash<H>> for BuzHashIncr<H> {
    fn from(src: &BuzHash<H>) -> Self {
        src.clone().into()
    }
}

impl<H: BuzHashHash + Clone> ToChunkIncr for BuzHash<H> {
    type Incr = BuzHashIncr<H>;
    fn to_chunk_incr(&self) -> Self::Incr {
        self.into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BuzHashSearchState {
    offset: usize,
    state: BuzHashState,
}

impl BuzHashSearchState {
    fn reset(&mut self) {
        self.offset = 0;
        self.state.reset();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct BuzHashState {
    /// current value of the hash.
    h: u32,
}

impl BuzHashState {
    fn reset(&mut self) {
        self.h = 0;
    }

    fn add_buf<H: BuzHashHash>(&mut self, data: &[u8], params: &BuzHash<H>, i: usize) {
        if i >= params.k {
            // need to find and "remove" a entry
            let drop_i = i - params.k;
            let drop = data[drop_i];
            self.add_overflow(params, data[i], drop);
        } else {
            // no removal
            self.add(params, data[i]);
        }
    }

    // insert, assuming no overflow
    fn add<H: BuzHashHash>(&mut self, params: &BuzHash<H>, v: u8) {
        self.h = self.h.rotate_left(1) ^ params.h.hash(v);
    }

    // insert with overflow
    fn add_overflow<H: BuzHashHash>(&mut self, params: &BuzHash<H>, add_v: u8, remove_v: u8) {
        let h = self.h.rotate_left(1);
        // need to find and "remove" a entry
        let drop = params.h.hash(remove_v).rotate_left((params.k % 8) as u32);
        self.h = h ^ drop ^ params.h.hash(add_v);
    }
}

/// Self-contained buzhash which buffers it's window of values internally
///
/// Note that this will be less efficient than using [`BuzHash`] on a slice directly,
/// but may be more convenient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuzHashIncr<H: BuzHashHash> {
    params: BuzHash<H>,
    state: BuzHashState,
    buf: Box<[u8]>,
    buf_idx: Wrapping<usize>,
    input_idx: u64,
}

impl<H: BuzHashHash> ChunkIncr for BuzHashIncr<H> {
    /// Return the index in `data` immeidately following the hash matching.
    ///
    /// Note that you can call this multiple times to examine "subsequent" `data` slices, but the
    /// index returned will always refer to the current `data` slice.
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        for (i, &v) in data.iter().enumerate() {
            self.push_byte(v);
            if (self.state.h & self.params.mask) == self.params.mask {
                self.reset();
                return Some(i + 1);
            }

            if self.input_idx > self.params.max_chunk_size {
                self.reset();
                return Some(i + 1);
            }
        }

        None
    }
}

impl<H: BuzHashHash> BuzHashIncr<H> {
    fn reset(&mut self) {
        self.buf_idx = Wrapping(0);
        self.input_idx = 0;
        self.state.reset();
    }

    fn push_byte(&mut self, val: u8) {
        if self.input_idx >= self.params.k as u64 {
            let o = self.buf[self.buf_idx.0];
            self.state.add_overflow(&self.params, val, o);
        } else {
            self.state.add(&self.params, val);
        }

        self.buf[self.buf_idx.0] = val;

        self.buf_idx += Wrapping(1);
        self.buf_idx.0 %= self.params.k;
        self.input_idx += 1;
    }
}

impl<H: BuzHashHash> From<BuzHash<H>> for BuzHashIncr<H> {
    fn from(params: BuzHash<H>) -> Self {
        let buf = vec![0; params.k].into_boxed_slice();
        Self {
            params,
            state: Default::default(),
            buf,
            buf_idx: Wrapping(0),
            input_idx: 0,
        }
    }
}

/// The internal byte to u32 mapping used in buzhash
pub trait BuzHashHash {
    fn hash(&self, data: u8) -> u32;
}

/// Use a referenced table to preform the `BuzHashHash` internal hashing
#[derive(Clone)]
pub struct BuzHashTableHash<'a> {
    table: &'a [u32; 256],
}

impl<'a> fmt::Debug for BuzHashTableHash<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("BuzHashTableHash").finish()
    }
}

impl<'a> From<&'a [u32; 256]> for BuzHashTableHash<'a> {
    fn from(table: &'a [u32; 256]) -> Self {
        Self { table }
    }
}

impl<'a> BuzHashHash for BuzHashTableHash<'a> {
    fn hash(&self, data: u8) -> u32 {
        self.table[data as usize]
    }
}

/// Use a owned table to perform the `BuzHashHash` internal hashing
#[derive(Clone)]
pub struct BuzHashTableBufHash {
    table: Box<[u32; 256]>,
}

impl fmt::Debug for BuzHashTableBufHash {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("BuzHashTableBufHash").finish()
    }
}

impl<'a> From<Box<[u32; 256]>> for BuzHashTableBufHash {
    fn from(table: Box<[u32; 256]>) -> Self {
        Self { table }
    }
}

impl BuzHashHash for BuzHashTableBufHash {
    fn hash(&self, data: u8) -> u32 {
        self.table[data as usize]
    }
}

/// Lookup up in a table, after applying a salt via xor to the input byte
///
/// Used by attic-labs/nom
#[derive(Clone)]
pub struct BuzHashTableByteSaltHash<'a> {
    table: &'a [u32; 256],
    salt: u8,
}

impl<'a> fmt::Debug for BuzHashTableByteSaltHash<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("BuzHashTableByteSaltHash").finish()
    }
}

impl<'a> From<(u8, &'a [u32; 256])> for BuzHashTableByteSaltHash<'a> {
    fn from((salt, table): (u8, &'a [u32; 256])) -> Self {
        Self { table, salt }
    }
}

impl<'a> BuzHashHash for BuzHashTableByteSaltHash<'a> {
    fn hash(&self, data: u8) -> u32 {
        self.table[(data ^ self.salt) as usize]
    }
}
