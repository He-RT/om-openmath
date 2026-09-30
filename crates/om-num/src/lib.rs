//! Exact and arbitrary-precision arithmetic for OpenMath.
#![forbid(unsafe_code)]

/// Computation budgets, cancellation and injectable clocks.
pub mod ctx;
/// Exact and approximate scalar and complex numbers.
pub mod number;

/// Certified real and complex interval balls.
pub mod ball;
/// Arithmetic in prime fields with machine-sized moduli.
pub mod modp;
/// Integer arithmetic, primality testing and bounded factorization.
pub mod ntheory;
/// Deterministic pseudorandom numbers for reproducible algorithms.
pub mod rng;

pub use ball::{Ball, CBall};
pub use dashu::base::BitTest;
pub use dashu::integer::IBig;
pub use dashu::rational::RBig;
pub use modp::Fp;
pub use ntheory::{
    exact_root, ext_gcd, extract_root_factor, factor_integer, gcd, is_probable_prime, isqrt,
    perfect_power,
};
pub use number::{BigFloat, Complex, Integer, NumError, Number, Precision, Rational, Real};
