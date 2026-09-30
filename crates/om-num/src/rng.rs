//! SplitMix64, with a fixed caller-provided seed and unbiased half-open ranges.

/// Reproducible SplitMix64 pseudorandom stream.
#[derive(Clone, Debug)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// Initialize a stream with a caller-provided seed.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    /// Produce the next 64-bit word.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    /// Uniformly sample `[lo, hi)`, requiring `lo < hi`.
    pub fn next_range(&mut self, lo: u64, hi: u64) -> u64 {
        assert!(lo < hi, "range must be nonempty");
        let width = hi - lo;
        let threshold = width.wrapping_neg() % width;
        loop {
            let word = self.next_u64();
            if word >= threshold {
                return lo + word % width;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seed_zero_matches_known_stream() {
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xe220a8397b1dcdaf);
        assert_eq!(rng.next_u64(), 0x6e789e6aa1b965f4);
        assert_eq!(rng.next_u64(), 0x06c45d188009454f);
    }
    #[test]
    fn same_seed_reproduces_ranges_including_large_widths() {
        let (mut a, mut b) = (SplitMix64::new(7), SplitMix64::new(7));
        for (lo, hi) in [
            (3, 17),
            (0, u64::MAX),
            (u64::MAX - 1, u64::MAX),
            (0, (1 << 63) + 1),
        ] {
            for _ in 0..1000 {
                let value = a.next_range(lo, hi);
                assert!(value >= lo && value < hi);
                assert_eq!(value, b.next_range(lo, hi));
            }
        }
    }
}
