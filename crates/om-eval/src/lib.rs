//! OpenMath expression evaluator.
#![forbid(unsafe_code)]

mod algebra;
mod algebra_registry;
mod builtins;
mod composition;
mod definitions;
mod elementary;
mod elementary_registry;
mod engine;
mod evaluator;
/// Held exploration and credential-free readonly state.
pub mod explore;
mod logic;
/// Real numeric compilation and existing N support.
pub mod numeric;
mod plot_registry;
mod scalar;
mod scalar_registry;
mod scene_registry;
mod science;
mod scientific;
mod solver;
mod solver_registry;
mod structure;
mod structure_registry;
mod types;

/// Structural patterns, variable bindings and simultaneous raw substitution.
pub mod pattern;
mod pure;

pub use builtins::BuiltinTable;
pub use definitions::{Definitions, Rule};
pub use evaluator::Evaluator;
pub use scientific::ScientificResult;
pub use solver::SolverResult;
pub use types::{Arity, Attributes, BuiltinFn, BuiltinSpec, DocEntry, EvalError, EvalSettings};

/// Serialize already retained pure tabular data, without invoking the evaluator.
pub fn data_csv(value: &om_core::Expr, ctx: &om_core::Interrupt) -> Result<String, EvalError> {
    science::csv_data::encode(value, ctx)
}
/// Serialize already retained pure JSON data, without invoking the evaluator.
pub fn data_json(value: &om_core::Expr, ctx: &om_core::Interrupt) -> Result<String, EvalError> {
    science::json_data::encode(value, ctx)
}
