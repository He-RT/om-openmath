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

/// Certified expression conversion to exact algebraic numbers and principal radicals.
pub mod root_reduce;

/// Layered exact and numerical zero decisions, with explicit inconclusive results.
pub mod zero;

/// Bounded best-first expression simplification with explicit positive assumptions.
pub mod simplify;
