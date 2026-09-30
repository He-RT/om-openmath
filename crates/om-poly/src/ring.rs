//! Expression-independent coefficient domains with the plan's fixed signatures.
use om_num::{Integer, Rational};
use std::fmt::Debug;

/// Coefficient arithmetic; operands must belong to a compatible ring context.
pub trait Ring: Clone + PartialEq + Debug {
    /// Additive identity, possibly a context-free factory value.
    fn zero() -> Self;
    /// Multiplicative identity, possibly a context-free factory value.
    fn one() -> Self;
    /// Whether this coefficient is zero.
    fn is_zero(&self) -> bool;
    /// Add compatible coefficients.
    fn add(&self, o: &Self) -> Self;
    /// Subtract compatible coefficients.
    fn sub(&self, o: &Self) -> Self;
    /// Multiply compatible coefficients.
    fn mul(&self, o: &Self) -> Self;
    /// Additive inverse.
    fn neg(&self) -> Self;
}
/// Exact division and Euclidean remainder arithmetic for coefficient domains.
pub trait EuclideanRing: Ring {
    /// Quotient/remainder; the divisor must be nonzero.
    fn divrem(&self, o: &Self) -> (Self, Self);
    /// A quotient only when division is exact; zero divisors return None.
    fn exact_div(&self, o: &Self) -> Option<Self>;
}
/// Inversion for coefficients in a field with a bound runtime context.
pub trait Field: Ring {
    /// Invert a unit; zero or an unbound nonunit returns None.
    fn inv(&self) -> Option<Self>;
}

impl Ring for Integer {
    fn zero() -> Self {
        Self::ZERO
    }
    fn one() -> Self {
        Self::ONE
    }
    fn is_zero(&self) -> bool {
        Integer::is_zero(self)
    }
    fn add(&self, o: &Self) -> Self {
        self + o
    }
    fn sub(&self, o: &Self) -> Self {
        self - o
    }
    fn mul(&self, o: &Self) -> Self {
        self * o
    }
    fn neg(&self) -> Self {
        -self
    }
}
impl EuclideanRing for Integer {
    fn divrem(&self, o: &Self) -> (Self, Self) {
        assert!(
            !o.is_zero(),
            "integer Euclidean division requires a nonzero divisor"
        );
        let (mut q, mut r) = (self / o, self % o);
        if r < Self::ZERO {
            if o > &Self::ZERO {
                q -= 1;
                r += o;
            } else {
                q += 1;
                r -= o;
            }
        }
        (q, r)
    }
    fn exact_div(&self, o: &Self) -> Option<Self> {
        (!o.is_zero() && (self % o).is_zero()).then(|| self / o)
    }
}
impl Ring for Rational {
    fn zero() -> Self {
        Self::ZERO
    }
    fn one() -> Self {
        Self::ONE
    }
    fn is_zero(&self) -> bool {
        Rational::is_zero(self)
    }
    fn add(&self, o: &Self) -> Self {
        self + o
    }
    fn sub(&self, o: &Self) -> Self {
        self - o
    }
    fn mul(&self, o: &Self) -> Self {
        self * o
    }
    fn neg(&self) -> Self {
        -self
    }
}
impl Field for Rational {
    fn inv(&self) -> Option<Self> {
        (!self.is_zero()).then(|| Self::ONE / self)
    }
}
