//! Equation solving with structured derivation steps.
#![forbid(unsafe_code)]

mod steps;
mod types;
pub use steps::{
    ExclReason, Formula, Level, NoSteps, RowOp, Sign, Step, StepKind, StepRecorder, StepSink,
    Steps, rule_ids,
};
pub use types::{
    Bound, Domain, Interval, MaxExtra, Solution, SolutionSet, SolveError, SolveOptions,
    SolveOutcome, Verification, VerifyMode,
};
/// Original-input normalization with preserved domain exclusions.
pub mod normalize;
/// Polynomial candidate solving with exact multiplicities and recorded formulas.
pub mod univariate;
