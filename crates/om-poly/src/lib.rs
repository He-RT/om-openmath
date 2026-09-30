//! Expression-independent polynomial algorithms.
#![forbid(unsafe_code)]

mod content;
mod division;
mod field_alg;
mod finite_factor;
mod fp;
mod gcd;
mod hensel;
mod mpoly;
mod resultant;
mod ring;
mod squarefree;
mod upoly;

pub use fp::FpElem;
pub use hensel::{HenselPair, hensel_lift, hensel_step};
pub use mpoly::{MPoly, MonoOrder, Monomial};
pub use ring::{EuclideanRing, Field, Ring};
pub use squarefree::SquareFree;
pub use upoly::UPoly;
