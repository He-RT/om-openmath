//! OpenMath expression evaluator.
#![forbid(unsafe_code)]

mod algebra;
mod algebra_registry;
mod builtins;
mod definitions;
mod elementary;
mod elementary_registry;
mod engine;
mod evaluator;
mod logic;
mod numeric;
mod scalar;
mod scalar_registry;
mod solver;
mod solver_registry;
mod structure;
mod structure_registry;
mod types;

/// Structural patterns, variable bindings and simultaneous raw substitution.
pub mod pattern;

pub use builtins::BuiltinTable;
pub use definitions::{Definitions, Rule};
pub use evaluator::Evaluator;
pub use types::{Arity, Attributes, BuiltinFn, BuiltinSpec, DocEntry, EvalError, EvalSettings};
