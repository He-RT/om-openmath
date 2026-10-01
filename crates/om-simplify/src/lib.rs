//! Symbolic simplification and expression-polynomial conversion.
#![forbid(unsafe_code)]

/// Exact expression and rational-polynomial conversion with normalized generators.
pub mod convert;

/// Exact elementary-function values shared by evaluation and solving.
pub mod special;

/// Interruptible numerical expression evaluation with guarded precision.
pub mod numeval;
