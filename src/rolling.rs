//! Non-cryptographic rolling fingerprints, independent of chunk boundary policy.

/// A rolling hash over a moving byte window.
///
/// Before the window fills, the digest describes the available prefix. These
/// fingerprints are for matching or boundary selection, not content integrity.
pub trait RollingHash {
    type Digest: Copy;
    /// Incorporate a byte and return the new fingerprint.
    fn roll_byte(&mut self, byte: u8) -> Self::Digest;
    fn digest(&self) -> Self::Digest;
    /// Restore the initial state, including the empty window.
    fn reset(&mut self);
}
