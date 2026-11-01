#![cfg(feature = "ae")]
//! Asymmetric Extremum (AE), with byte-stream and optional AVX2 paths.
//! References: <https://doi.org/10.1109/INFOCOM.2015.7218510> and
//! <https://arxiv.org/abs/2610.13704> (target-size calibration).
use crate::ChunkIncr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extremum {
    Maximum,
    Minimum,
}

fn better(byte: u8, extreme: u8, mode: Extremum) -> bool {
    match mode {
        Extremum::Maximum => byte > extreme,
        Extremum::Minimum => byte < extreme,
    }
}
fn first_better(data: &[u8], extreme: u8, mode: Extremum, accelerated: bool) -> Option<usize> {
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    if accelerated && std::arch::is_x86_feature_detected!("avx2") {
        // SAFETY: CPU support was detected; the callee only loads full slices.
        return unsafe { first_better_avx2(data, extreme, mode) };
    }
    let _ = accelerated;
    data.iter().position(|&b| better(b, extreme, mode))
}
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn first_better_avx2(data: &[u8], extreme: u8, mode: Extremum) -> Option<usize> {
    use std::arch::x86_64::*;
    let sign = _mm256_set1_epi8(i8::MIN);
    let threshold = _mm256_set1_epi8((extreme ^ 128) as i8);
    let mut offset = 0;
    while data.len() - offset >= 32 {
        // SAFETY: 32 readable bytes remain; unaligned loads are supported.
        let bytes = unsafe { _mm256_loadu_si256(data.as_ptr().add(offset).cast()) };
        let bytes = _mm256_xor_si256(bytes, sign);
        let matched = match mode {
            Extremum::Maximum => _mm256_cmpgt_epi8(bytes, threshold),
            Extremum::Minimum => _mm256_cmpgt_epi8(threshold, bytes),
        };
        let bits = _mm256_movemask_epi8(matched) as u32;
        if bits != 0 {
            return Some(offset + bits.trailing_zeros() as usize);
        }
        offset += 32;
    }
    data[offset..]
        .iter()
        .position(|&b| better(b, extreme, mode))
        .map(|n| offset + n)
}

/// A cut follows an extremum that survives `horizon` subsequent bytes.
/// Equal bytes do not replace the first extremum. Min/max modes are explicit.
#[derive(Debug, Clone)]
pub struct Ae {
    horizon: usize,
    mode: Extremum,
    max: usize,
    accelerated: bool,
}
impl Ae {
    /// Panics unless `0 < horizon < max`. The minimum chunk is horizon + 1.
    pub fn new(horizon: usize, mode: Extremum, max: usize) -> Self {
        assert!(horizon > 0 && horizon < max);
        Self {
            horizon,
            mode,
            max,
            accelerated: true,
        }
    }
    /// Approximate a target of at least 2 KiB on uniformly random byte input.
    /// Uses horizon = target - 256; this is not a promise for other input.
    pub fn with_average_size(target: usize, max: usize) -> Self {
        assert!(target >= 2048 && target <= max);
        Self::new(target - 256, Extremum::Maximum, max)
    }
    /// Disable runtime AVX2 dispatch for comparison with the scalar oracle.
    pub fn with_acceleration(mut self, enabled: bool) -> Self {
        self.accelerated = enabled;
        self
    }
    fn state(&self) -> AeIncr {
        AeIncr {
            params: self.clone(),
            len: 0,
            extreme: None,
            age: 0,
        }
    }
}
impl Default for Ae {
    fn default() -> Self {
        Self::with_average_size(8192, 65536)
    }
}
#[derive(Debug, Clone)]
pub struct AeIncr {
    params: Ae,
    len: usize,
    extreme: Option<u8>,
    age: usize,
}
impl ChunkIncr for AeIncr {
    fn push(&mut self, data: &[u8]) -> Option<usize> {
        let mut i = 0;
        if self.extreme.is_none() && !data.is_empty() {
            self.extreme = Some(data[0]);
            self.len = 1;
            i = 1;
        }
        while i < data.len() {
            let take = (self.params.horizon - self.age)
                .min(self.params.max - self.len)
                .min(data.len() - i);
            let extreme = self.extreme.unwrap();
            if let Some(n) = first_better(
                &data[i..i + take],
                extreme,
                self.params.mode,
                self.params.accelerated,
            ) {
                self.extreme = Some(data[i + n]);
                self.age = 0;
                self.len += n + 1;
                i += n + 1;
            } else {
                self.len += take;
                self.age += take;
                i += take;
            }
            if self.age == self.params.horizon || self.len == self.params.max {
                self.len = 0;
                self.age = 0;
                self.extreme = None;
                return Some(i);
            }
        }
        None
    }
}
impl_stream_chunker!(Ae, AeIncr);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vector_search_matches_scalar_for_unsigned_thresholds_ties_and_tails() {
        let data: Vec<_> = (0..600).map(|i| ((i * 73 + 129) % 256) as u8).collect();
        for mode in [Extremum::Minimum, Extremum::Maximum] {
            for threshold in 0..=255 {
                for start in 0..32 {
                    for len in [0, 1, 31, 32, 33, 63, 64, 65, 257] {
                        let slice = &data[start..start + len];
                        assert_eq!(
                            first_better(slice, threshold, mode, true),
                            first_better(slice, threshold, mode, false)
                        );
                    }
                }
            }
        }
    }
}
