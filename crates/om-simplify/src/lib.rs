//! Symbolic simplification and expression-polynomial conversion.
#![forbid(unsafe_code)]

/// Exact elementary-function values shared by evaluation and solving.
pub mod special;

/// Interruptible numerical expression evaluation with guarded precision.
pub mod numeval;
