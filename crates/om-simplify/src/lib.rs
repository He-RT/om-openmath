//! Symbolic simplification and expression-polynomial conversion.
#![forbid(unsafe_code)]

/// Exact expression and rational-polynomial conversion with normalized generators.
pub mod convert;

/// Rational transforms, arithmetic expansion and certified polynomial factorization.
pub mod algebra;

/// Exact elementary-function values shared by evaluation and solving.
pub mod special;

/// Interruptible numerical expression evaluation with guarded precision.
pub mod numeval;
