//! Symbolic expressions and canonical constructors.
#![forbid(unsafe_code)]

/// Fixed built-in symbol identifiers.
pub mod builtins;
/// Shared symbol interning.
pub mod symbol;

pub use builtins::BUILTIN;
pub use symbol::Symbol;
