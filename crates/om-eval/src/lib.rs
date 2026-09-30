//! OpenMath expression evaluator.
#![forbid(unsafe_code)]

mod builtins;
mod definitions;
mod engine;
mod evaluator;
mod types;

pub use builtins::BuiltinTable;
pub use definitions::{Definitions, Rule};
pub use evaluator::Evaluator;
pub use types::{Arity, Attributes, BuiltinFn, BuiltinSpec, DocEntry, EvalError, EvalSettings};
