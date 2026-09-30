//! Exact and arbitrary-precision arithmetic for OpenMath.
#![forbid(unsafe_code)]

/// Exact and approximate scalar and complex numbers.
pub mod number;

pub use dashu::integer::IBig;
pub use dashu::rational::RBig;
pub use number::{BigFloat, Complex, Integer, NumError, Number, Precision, Rational, Real};
