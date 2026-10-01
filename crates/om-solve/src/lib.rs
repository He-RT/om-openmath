//! Equation solving with structured derivation steps.
#![forbid(unsafe_code)]

mod steps;
mod types;
pub use steps::{
    ExclReason, Formula, Level, NoSteps, RowOp, Sign, Step, StepKind, StepRecorder, StepSink,
    Steps, rule_ids,
};
pub use types::{Domain, SolveError};
/// Original-input normalization with preserved domain exclusions.
pub mod normalize;
