//! Expression-independent polynomial algorithms.
#![forbid(unsafe_code)]

mod fp;
mod mpoly;
mod ring;
mod upoly;

pub use fp::FpElem;
pub use mpoly::{MPoly, MonoOrder, Monomial};
pub use ring::{EuclideanRing, Field, Ring};
pub use upoly::UPoly;
