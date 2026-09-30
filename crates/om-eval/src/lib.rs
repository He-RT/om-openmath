//! OpenMath expression evaluator.
#![forbid(unsafe_code)]

mod builtins;
mod definitions;
mod engine;
mod evaluator;
mod logic;
mod scalar;
mod scalar_registry;
mod types;

/// Structural patterns, variable bindings and simultaneous raw substitution.
pub mod pattern;

pub use builtins::BuiltinTable;
pub use definitions::{Definitions, Rule};
pub use evaluator::Evaluator;
pub use types::{Arity, Attributes, BuiltinFn, BuiltinSpec, DocEntry, EvalError, EvalSettings};
