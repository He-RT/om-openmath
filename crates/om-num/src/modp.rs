//! Prime-field arithmetic using u128 intermediates.

/// A u64 prime field; direct construction requires p>=2 and a prime modulus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fp {
    /// Field modulus.
    pub p: u64,
}

impl Fp {
    /// Validate and construct a prime field.
    pub fn new(p: u64) -> Option<Self> {
        crate::is_probable_prime(&p.into()).then_some(Self { p })
    }
    /// Add two residues.
    pub fn add(&self, a: u64, b: u64) -> u64 {
        assert!(self.p >= 2, "modulus must be at least two");
        ((a as u128 + b as u128) % self.p as u128) as u64
    }
    /// Subtract two residues.
    pub fn sub(&self, a: u64, b: u64) -> u64 {
        assert!(self.p >= 2, "modulus must be at least two");
        ((a as u128 + self.p as u128 - (b % self.p) as u128) % self.p as u128) as u64
    }
    /// Multiply two residues.
    pub fn mul(&self, a: u64, b: u64) -> u64 {
        assert!(self.p >= 2, "modulus must be at least two");
        ((a as u128 * b as u128) % self.p as u128) as u64
    }
    /// Raise a residue to an unsigned power, with 0^0=1.
    pub fn pow(&self, a: u64, mut e: u64) -> u64 {
        assert!(self.p >= 2, "modulus must be at least two");
        let mut a = a % self.p;
        let mut value = 1;
        while e > 0 {
            if e & 1 == 1 {
                value = self.mul(value, a);
            }
            a = self.mul(a, a);
            e >>= 1;
        }
        value
    }
    /// Invert a unit; zero and other nonunits have no inverse.
    pub fn inv(&self, a: u64) -> Option<u64> {
        assert!(self.p >= 2, "modulus must be at least two");
        let (mut old_r, mut r) = (self.p as i128, (a % self.p) as i128);
        let (mut old_t, mut t) = (0, 1);
        while r != 0 {
            let q = old_r / r;
            (old_r, r) = (r, old_r - q * r);
            (old_t, t) = (t, old_t - q * t);
        }
        (old_r == 1).then(|| old_t.rem_euclid(self.p as i128) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_moduli_and_normalizes_residues() {
        assert_eq!(Fp::new(0), None);
        assert_eq!(Fp::new(1), None);
        assert_eq!(Fp::new(15), None);
        let f = Fp::new(7).unwrap();
        assert_eq!(f.add(10, 13), 2);
        assert_eq!(f.sub(1, 3), 5);
        assert_eq!(f.mul(10, 13), 4);
        assert_eq!(f.inv(3), Some(5));
        assert_eq!(f.inv(0), None);
        assert_eq!(f.inv(14), None);
        assert_eq!(f.pow(3, 6), 1);
        assert_eq!(f.pow(0, 0), 1);
        assert_eq!(Fp { p: 15 }.inv(3), None);
    }
    #[test]
    fn near_u64_limit_does_not_overflow() {
        let f = Fp::new(18_446_744_073_709_551_557).unwrap();
        assert_eq!(f.add(f.p - 1, f.p - 1), f.p - 2);
        assert_eq!(f.mul(f.p - 1, f.p - 1), 1);
        assert_eq!(f.sub(0, 1), f.p - 1);
        assert_eq!(f.mul(u64::MAX, u64::MAX), 58 * 58);
        assert_eq!(f.inv(f.p - 1), Some(f.p - 1));
    }
    #[test]
    fn all_small_field_units_and_distributivity() {
        for p in [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
            let f = Fp::new(p).unwrap();
            for a in 0..p {
                if a > 0 {
                    assert_eq!(f.mul(a, f.inv(a).unwrap()), 1);
                }
                for b in 0..p {
                    for c in 0..p {
                        assert_eq!(f.mul(a, f.add(b, c)), f.add(f.mul(a, b), f.mul(a, c)));
                    }
                }
            }
        }
    }
}
