//! Expression-independent polynomial algorithms.
#![forbid(unsafe_code)]

mod alg;
mod complex_roots;
mod content;
mod dimension;
mod division;
mod factor_z;
mod fglm;
mod field_alg;
mod finite_factor;
mod fp;
mod gcd;
mod groebner;
mod hensel;
mod hermite;
mod isolation;
mod linear;
mod mpoly;
mod multivariate_factor;
mod resultant;
mod ring;
mod squarefree;
mod upoly;

pub use alg::{Algebraic, ComplexAlg, RealAlg, algebraic_root, algebraic_roots, real_alg};
pub use complex_roots::{RootDisk, complex_roots};
pub use dimension::{IdealDimension, ideal_dimension, is_zero_dimensional};
pub use factor_z::{FactorStatus, IntegerFactorization};
pub use fglm::fglm;
pub use fp::FpElem;
pub use groebner::{groebner, normal_form, s_polynomial};
pub use hensel::{HenselPair, hensel_lift, hensel_step};
pub use hermite::{
    HermiteForm, IntegerLinearResult, IntegerLinearSolution, column_hnf, integer_linear_solve,
};
pub use isolation::{RootInterval, isolate, refine};
pub use linear::{
    BareissResult, ExactDomain, ExactFraction, LinearResult, LinearSolution, bareiss, determinant,
    linear_solve,
};
pub use mpoly::{MPoly, MonoOrder, Monomial};
pub use multivariate_factor::MultivariateFactorization;
pub use ring::{EuclideanRing, Field, Ring};
pub use squarefree::SquareFree;
pub use upoly::UPoly;
