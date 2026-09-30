//! Expression-independent polynomial algorithms.
#![forbid(unsafe_code)]

mod content;
mod division;
mod factor_z;
mod field_alg;
mod finite_factor;
mod fp;
mod gcd;
mod hensel;
mod mpoly;
mod multivariate_factor;
mod resultant;
mod ring;
mod squarefree;
mod upoly;

pub use factor_z::{FactorStatus, IntegerFactorization};
pub use fp::FpElem;
pub use hensel::{HenselPair, hensel_lift, hensel_step};
pub use mpoly::{MPoly, MonoOrder, Monomial};
pub use multivariate_factor::MultivariateFactorization;
pub use ring::{EuclideanRing, Field, Ring};
pub use squarefree::SquareFree;
pub use upoly::UPoly;
