//! Exact and finite approximate arithmetic.

use dashu::base::BitTest;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Arbitrary-precision signed integer.
pub type Integer = dashu::integer::IBig;
/// Reduced rational with a positive denominator.
pub type Rational = dashu::rational::RBig;
/// Binary arbitrary-precision float, rounded to nearest with ties to even.
pub type BigFloat = dashu::float::FBig<dashu::float::round::mode::HalfEven, 2>;

/// A finite approximate real value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Real {
    /// Finite IEEE binary64 value.
    Machine(f64),
    /// Finite binary float with a bit precision.
    #[serde(with = "big_float_serde")]
    Big(BigFloat),
}

/// An exact or finite approximate number.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Number {
    /// Exact integer.
    Integer(Integer),
    /// Exact rational; normalize denominator-one values to integers.
    Rational(Rational),
    /// Approximate real.
    Real(Real),
    /// Complex value with scalar components of matching numeric category.
    Complex(Box<Complex>),
}

/// Scalar real and imaginary components; neither may itself be complex.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Complex {
    /// Real component.
    pub re: Number,
    /// Imaginary component.
    pub im: Number,
}

/// Numeric precision; exact values carry no approximation error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Precision {
    /// Exact integer or rational.
    Exact,
    /// IEEE binary64.
    Machine,
    /// Binary arbitrary precision in bits.
    Bits(u32),
}

/// Arithmetic errors requiring symbolic handling by the expression layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NumError {
    /// The denominator is zero.
    #[error("division by zero")]
    DivByZero,
    /// The operation has no unique numeric result, such as zero to zero.
    #[error("indeterminate numeric operation")]
    Indeterminate,
    /// An exact power would exceed the expression layer's bit-size limit.
    #[error("exact result exceeds the bit-size limit")]
    ExactOverflow,
}

impl Number {
    /// Reduce rationals and align complex component precision.
    pub fn normalize(self) -> Number {
        match self {
            Self::Rational(q) if q.denominator().is_one() => Self::Integer(q.into_parts().0),
            Self::Complex(c) => {
                let re = c.re.normalize();
                let im = c.im.normalize();
                if matches!(re, Self::Complex(_)) || matches!(im, Self::Complex(_)) {
                    let (a, b) = re.components();
                    let (c, d) = im.components();
                    return Self::Complex(Box::new(Complex {
                        re: a.add(&d.neg()),
                        im: b.add(&c),
                    }))
                    .normalize();
                }
                if im.is_exact() && im.is_zero() {
                    return re;
                }
                let mut p = combine_precision(re.precision(), im.precision());
                if p == Precision::Machine && (re.to_f64().is_none() || im.to_f64().is_none()) {
                    p = Precision::Bits(53);
                }
                Self::Complex(Box::new(Complex {
                    re: re.coerce(p),
                    im: im.coerce(p),
                }))
            }
            Self::Real(Real::Machine(x)) => {
                assert!(x.is_finite(), "Real::Machine requires a finite value");
                self
            }
            Self::Real(Real::Big(ref x)) => {
                assert!(x.repr().is_finite(), "Real::Big requires a finite value");
                if x.precision() == 0 {
                    Self::Real(Real::Big(x.clone().with_precision(53).value()))
                } else {
                    self
                }
            }
            _ => self,
        }
    }
    /// Whether the value is exact.
    pub fn is_exact(&self) -> bool {
        match self {
            Self::Integer(_) | Self::Rational(_) => true,
            Self::Real(_) => false,
            Self::Complex(c) => c.re.is_exact() && c.im.is_exact(),
        }
    }
    /// Whether both components are zero.
    pub fn is_zero(&self) -> bool {
        match self {
            Self::Integer(n) => n.is_zero(),
            Self::Rational(q) => q.is_zero(),
            Self::Real(Real::Machine(x)) => *x == 0.0,
            Self::Real(Real::Big(x)) => x == &BigFloat::ZERO,
            Self::Complex(c) => c.re.is_zero() && c.im.is_zero(),
        }
    }
    /// Whether the number equals one.
    pub fn is_one(&self) -> bool {
        match self {
            Self::Integer(n) => n.is_one(),
            Self::Rational(q) => q.is_one(),
            Self::Real(Real::Machine(x)) => *x == 1.0,
            Self::Real(Real::Big(x)) => x.repr().is_one(),
            Self::Complex(_) => false,
        }
    }
    /// Whether a scalar is negative; complex values return false.
    pub fn is_negative(&self) -> bool {
        match self {
            Self::Integer(n) => n < &Integer::ZERO,
            Self::Rational(q) => q.numerator() < &Integer::ZERO,
            Self::Real(Real::Machine(x)) => *x < 0.0,
            Self::Real(Real::Big(x)) => x < &BigFloat::ZERO,
            Self::Complex(_) => false,
        }
    }
    /// Add using numeric precision contagion.
    pub fn add(&self, o: &Number) -> Number {
        self.binary(o, false)
    }
    /// Multiply using numeric precision contagion.
    pub fn mul(&self, o: &Number) -> Number {
        self.binary(o, true)
    }
    /// Additive inverse.
    pub fn neg(&self) -> Number {
        match self {
            Self::Integer(n) => Self::Integer(-n),
            Self::Rational(q) => Self::Rational(-q),
            Self::Real(Real::Machine(x)) => Self::Real(Real::Machine(-x)),
            Self::Real(Real::Big(x)) => Self::Real(Real::Big(-x)),
            Self::Complex(c) => Self::Complex(Box::new(Complex {
                re: c.re.neg(),
                im: c.im.neg(),
            })),
        }
        .normalize()
    }
    /// Multiplicative inverse, rejecting every representation of zero.
    pub fn recip(&self) -> Result<Number, NumError> {
        let n = self.clone().normalize();
        if n.is_zero() {
            return Err(NumError::DivByZero);
        }
        if let Self::Complex(c) = &n {
            if n.is_exact() {
                let scale = c.re.mul(&c.re).add(&c.im.mul(&c.im)).recip()?;
                return Ok(Self::Complex(Box::new(Complex {
                    re: c.re.mul(&scale),
                    im: c.im.neg().mul(&scale),
                }))
                .normalize());
            }
            // Extended exponents avoid both squared-magnitude overflow and underflow.
            let p = n.precision();
            let bits = precision_bits(p);
            let re = c.re.to_big(bits.saturating_add(8));
            let im = c.im.to_big(bits.saturating_add(8));
            let norm = &re * &re + &im * &im;
            return Ok(Self::Complex(Box::new(Complex {
                re: finish_float(re / &norm, p),
                im: finish_float(-im / norm, p),
            }))
            .normalize());
        }
        if n.is_exact() {
            return Ok(Self::Rational(Rational::ONE / n.as_rational()).normalize());
        }
        let p = n.precision();
        let bits = precision_bits(p);
        Ok(finish_float(
            BigFloat::ONE.with_precision(bits).value() / n.to_big(bits),
            p,
        ))
    }
    /// Signed arbitrary-precision integer power.
    /// Exact results above 2^24 bits return [`NumError::ExactOverflow`].
    pub fn pow_int(&self, e: &Integer) -> Result<Number, NumError> {
        if e.is_zero() {
            return if self.is_zero() {
                Err(NumError::Indeterminate)
            } else {
                Ok(Self::Integer(Integer::ONE))
            };
        }
        let mut base = if e < &Integer::ZERO {
            self.recip()?
        } else {
            self.clone().normalize()
        };
        let exponent = e.clone().into_parts().1;
        let mut result = Self::Integer(Integer::ONE);
        for bit in 0..exponent.bit_len() {
            if exponent.bit(bit) {
                result = result.mul(&base);
                check_exact_size(&result)?;
            }
            if bit + 1 < exponent.bit_len() {
                base = base.mul(&base);
                check_exact_size(&base)?;
            }
        }
        Ok(result)
    }
    /// Finite binary64 approximation; complex or out-of-range values return None.
    pub fn to_f64(&self) -> Option<f64> {
        let x = match self {
            Self::Integer(n) => n.to_f64().value(),
            Self::Rational(q) => q.to_f64().value(),
            Self::Real(Real::Machine(x)) => *x,
            Self::Real(Real::Big(x)) => x.to_f64().value(),
            Self::Complex(_) => return None,
        };
        x.is_finite().then_some(x)
    }
    /// Binary64 approximation of the two components.
    /// Out-of-range components export as signed infinity.
    pub fn to_complex_f64(&self) -> (f64, f64) {
        let (re, im) = self.clone().normalize().components();
        let export = |x: &Number| {
            x.to_f64().unwrap_or_else(|| {
                if x.is_negative() {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                }
            })
        };
        (export(&re), export(&im))
    }
    /// The propagated precision.
    pub fn precision(&self) -> Precision {
        match self {
            Self::Integer(_) | Self::Rational(_) => Precision::Exact,
            Self::Real(Real::Machine(_)) => Precision::Machine,
            Self::Real(Real::Big(x)) => Precision::Bits(if x.precision() == 0 {
                53
            } else {
                u32::try_from(x.precision()).expect("invariant: Number bit precision fits u32")
            }),
            Self::Complex(c) => combine_precision(c.re.precision(), c.im.precision()),
        }
    }
    /// Mathematical ordering of scalar values, without binary64 coercion.
    pub fn cmp_real(&self, o: &Number) -> Option<Ordering> {
        if matches!(self, Self::Complex(_)) || matches!(o, Self::Complex(_)) {
            None
        } else {
            Some(self.as_rational().cmp(&o.as_rational()))
        }
    }

    fn components(self) -> (Number, Number) {
        match self {
            Self::Complex(c) => (c.re, c.im),
            _ => (self, Self::Integer(Integer::ZERO)),
        }
    }

    fn as_rational(&self) -> Rational {
        match self {
            Self::Integer(n) => Rational::from(n.clone()),
            Self::Rational(q) => q.clone(),
            Self::Real(Real::Machine(x)) => {
                Rational::try_from(*x).expect("invariant: real is finite")
            }
            Self::Real(Real::Big(x)) => {
                Rational::try_from(x.clone()).expect("invariant: real is finite")
            }
            Self::Complex(_) => unreachable!("invariant: scalar-only conversion"),
        }
    }

    fn to_big(&self, bits: usize) -> BigFloat {
        match self {
            Self::Real(Real::Big(x)) => x.clone().with_precision(bits).value(),
            _ => self.as_rational().to_float(bits).value(),
        }
    }

    fn coerce(self, p: Precision) -> Number {
        match p {
            Precision::Exact => self,
            Precision::Machine => match self.to_f64() {
                Some(x) => Self::Real(Real::Machine(x)),
                None => Self::Real(Real::Big(self.to_big(53))),
            },
            Precision::Bits(bits) => Self::Real(Real::Big(self.to_big(bits as usize))),
        }
    }

    fn binary(&self, o: &Number, multiply: bool) -> Number {
        let left = self.clone().normalize();
        let right = o.clone().normalize();
        let p = combine_precision(left.precision(), right.precision());
        if matches!(left, Self::Complex(_)) || matches!(right, Self::Complex(_)) {
            let (a, b) = left.components();
            let (c, d) = right.components();
            let (a, b, c, d) = (a.coerce(p), b.coerce(p), c.coerce(p), d.coerce(p));
            let (re, im) = if multiply {
                (a.mul(&c).add(&b.mul(&d).neg()), a.mul(&d).add(&b.mul(&c)))
            } else {
                (a.add(&c), b.add(&d))
            };
            return Self::Complex(Box::new(Complex { re, im })).normalize();
        }
        if let (Self::Integer(a), Self::Integer(b)) = (&left, &right) {
            return Self::Integer(if multiply { a * b } else { a + b });
        }
        if p == Precision::Exact {
            let (a, b) = (left.as_rational(), right.as_rational());
            return Self::Rational(if multiply { a * b } else { a + b }).normalize();
        }
        if p == Precision::Machine
            && let (Some(a), Some(b)) = (left.to_f64(), right.to_f64())
        {
            let result = if multiply { a * b } else { a + b };
            if result.is_finite() {
                return Self::Real(Real::Machine(result));
            }
        }
        let bits = precision_bits(p);
        let (a, b) = (left.to_big(bits), right.to_big(bits));
        let result = if multiply { a * b } else { a + b };
        // Overflow fallback retains Big even if cancellation brings the result back in range.
        Self::Real(Real::Big(result.with_precision(bits).value()))
    }
}

fn combine_precision(a: Precision, b: Precision) -> Precision {
    match (a, b) {
        (Precision::Machine, _) | (_, Precision::Machine) => Precision::Machine,
        (Precision::Exact, p) | (p, Precision::Exact) => p,
        (Precision::Bits(a), Precision::Bits(b)) => Precision::Bits(a.min(b)),
    }
}

fn precision_bits(p: Precision) -> usize {
    match p {
        Precision::Bits(bits) => bits as usize,
        _ => 53,
    }
}

fn finish_float(x: BigFloat, p: Precision) -> Number {
    let x = x.with_precision(precision_bits(p)).value();
    if p == Precision::Machine {
        let machine = x.to_f64().value();
        if machine.is_finite() {
            return Number::Real(Real::Machine(machine));
        }
    }
    Number::Real(Real::Big(x))
}

fn check_exact_size(n: &Number) -> Result<(), NumError> {
    const LIMIT: usize = 1 << 24;
    let too_large = match n {
        Number::Integer(n) => n.clone().into_parts().1.bit_len() > LIMIT,
        Number::Rational(q) => {
            q.numerator().clone().into_parts().1.bit_len() > LIMIT
                || q.denominator().bit_len() > LIMIT
        }
        Number::Complex(c) => return check_exact_size(&c.re).and(check_exact_size(&c.im)),
        Number::Real(_) => false,
    };
    if too_large {
        Err(NumError::ExactOverflow)
    } else {
        Ok(())
    }
}

#[path = "number_serde.rs"]
mod big_float_serde;

#[cfg(test)]
#[path = "number_tests.rs"]
mod tests;
