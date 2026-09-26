//! Seeded, deterministic RNG (§18.1). Never use a thread/global RNG in the sim.

#[derive(Clone, Debug)]
pub struct SimRng {
    state: u64,
}

impl SimRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    /// SplitMix64: tiny, fast, reproducible across platforms.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.unit() < p
    }

    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo) as u64) as u32
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.range(0, items.len() as u32) as usize])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let (mut a, mut b) = (SimRng::new(9), SimRng::new(9));
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn unit_and_range_stay_in_bounds() {
        let mut r = SimRng::new(1);
        for _ in 0..10_000 {
            let u = r.unit();
            assert!((0.0..1.0).contains(&u));
            let x = r.range(3, 7);
            assert!((3..7).contains(&x));
        }
        assert_eq!(r.range(5, 5), 5);
        assert!(r.pick::<u8>(&[]).is_none());
    }

    #[test]
    fn chance_is_roughly_calibrated() {
        let mut r = SimRng::new(2);
        let hits = (0..20_000).filter(|_| r.chance(0.25)).count();
        assert!((4_500..5_500).contains(&hits), "{hits}");
    }
}
