//! Runtime prime-field coefficients and neutral generic identity factories.
use crate::{Field, Ring};
use om_num::Fp;

/// A residue in F_p. Bound values require prime p>=2 and v<p.
/// p=0 is an unbound signed-i64 integer factory constant encoded in v.
#[derive(Clone, Copy, Debug, Eq)]
pub struct FpElem {
    /// Normalized residue, or the bits of an unbound signed constant.
    pub v: u64,
    /// Prime modulus; zero marks an unbound generic constant.
    pub p: u64,
}
impl PartialEq for FpElem {
    fn eq(&self, o: &Self) -> bool {
        // Zero and one are universal identity values, including the factories.
        // Other residues carry their prime-field context in equality.
        self.v == o.v && (matches!(self.v, 0 | 1) || self.p == o.p)
    }
}
impl FpElem {
    /// Validate a prime modulus and normalize the residue.
    pub fn new(v: u64, p: u64) -> Option<Self> {
        Fp::new(p).map(|_| Self { v: v % p, p })
    }
    fn modulus(&self, o: &Self) -> u64 {
        assert!(
            self.p == 0 || o.p == 0 || self.p == o.p,
            "cannot mix different prime fields"
        );
        self.p.max(o.p)
    }
    fn bound(&self, p: u64) -> u64 {
        if self.p == 0 {
            (self.v as i64 as i128).rem_euclid(p as i128) as u64
        } else {
            self.v % p
        }
    }
    fn neutral(value: i64) -> Self {
        Self {
            v: value as u64,
            p: 0,
        }
    }
    fn binary(&self, o: &Self, operation: u8) -> Self {
        let p = self.modulus(o);
        if p == 0 {
            let (a, b) = (self.v as i64, o.v as i64);
            let value = match operation {
                0 => a.checked_add(b),
                1 => a.checked_sub(b),
                _ => a.checked_mul(b),
            }
            .expect("invariant: bind neutral constants before exceeding signed i64");
            return Self::neutral(value);
        }
        assert!(p >= 2, "a bound field modulus must be at least two");
        let f = Fp { p };
        let (a, b) = (self.bound(p), o.bound(p));
        Self {
            v: match operation {
                0 => f.add(a, b),
                1 => f.sub(a, b),
                _ => f.mul(a, b),
            },
            p,
        }
    }
}
impl Ring for FpElem {
    fn zero() -> Self {
        Self::neutral(0)
    }
    fn one() -> Self {
        Self::neutral(1)
    }
    fn is_zero(&self) -> bool {
        if self.p == 0 {
            self.v == 0
        } else {
            self.v.is_multiple_of(self.p)
        }
    }
    fn add(&self, o: &Self) -> Self {
        self.binary(o, 0)
    }
    fn sub(&self, o: &Self) -> Self {
        self.binary(o, 1)
    }
    fn mul(&self, o: &Self) -> Self {
        self.binary(o, 2)
    }
    fn neg(&self) -> Self {
        if self.p == 0 {
            Self::neutral(
                (self.v as i64)
                    .checked_neg()
                    .expect("invariant: bind neutral constants before negating i64::MIN"),
            )
        } else {
            Self {
                v: Fp { p: self.p }.sub(0, self.v),
                p: self.p,
            }
        }
    }
}
impl Field for FpElem {
    fn inv(&self) -> Option<Self> {
        if self.p == 0 {
            return matches!(self.v as i64, 1 | -1).then_some(*self);
        }
        Fp { p: self.p }.inv(self.v).map(|v| Self { v, p: self.p })
    }
}
