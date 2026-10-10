//! Equation solving with structured derivation steps.
#![forbid(unsafe_code)]

mod dispatch;
mod domain;
mod inequality;
pub use inequality::{reduce, reduce_with_options};
mod elimination;
pub use elimination::eliminate;
mod numeric;
pub use numeric::{nsolve, nsolve_with_options};
mod local;
pub use local::find_root;
mod verification;
pub use dispatch::solve;
mod steps;
mod types;
pub use steps::{
    ExclReason, Formula, Level, NoSteps, RowOp, Sign, Step, StepKind, StepRecorder, StepSink,
    Steps, rule_ids,
};
pub use types::{
    Bound, Domain, FindRootOptions, Interval, MaxExtra, Solution, SolutionSet, SolveError,
    SolveOptions, SolveOutcome, Verification, VerifyMode,
};
/// Lossless structured result/step data, referencing a shared checkpoint expression graph.
pub mod checkpoint;
mod linear;
/// Original-input normalization with preserved domain exclusions.
pub mod normalize;
/// Polynomial candidate solving with exact multiplicities and recorded formulas.
pub mod univariate;
pub use linear::linear_system;
mod polynomial_system;
pub use polynomial_system::poly_system;
mod substitution;
pub use substitution::substitution_system;
