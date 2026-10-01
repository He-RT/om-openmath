//! Solver contracts introduced at their first real consumer.
use crate::Steps;
use om_core::{BUILTIN as B, Expr, Message, Symbol};
/// Requested solution domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Domain {
    /// All finite complex values.
    Complexes,
    /// Real values only.
    Reals,
    /// Integer values only.
    Integers,
    /// Exact rational values only.
    Rationals,
}

/// Solve behavior and reproducible computation preferences.
#[derive(Clone, Debug)]
pub struct SolveOptions {
    /// Requested solution domain, enforced by candidate kernels and the dispatcher.
    pub domain: Domain,
    /// Use Cardano rather than Root for general cubics.
    pub cubics: bool,
    /// Use Ferrari rather than Root for general quartics.
    pub quartics: bool,
    /// Original-equation verification preference.
    pub verify: VerifyMode,
    /// Maximum number of explicit generic conditions attached to a solution.
    pub max_extra_conditions: MaxExtra,
    /// Head used to name generated parameters.
    pub generated_parameter: Symbol,
    /// Permit elementary inverse-function paths.
    pub inverse_functions: bool,
    /// Record computation steps.
    pub record_steps: bool,
    /// Fixed numerical probing seed.
    pub seed: u64,
}
impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            domain: Domain::Complexes,
            cubics: false,
            quartics: false,
            verify: VerifyMode::Auto,
            max_extra_conditions: MaxExtra::Zero,
            generated_parameter: B::C,
            inverse_functions: true,
            record_steps: true,
            seed: 0x0A5E_ED00_0000_0001,
        }
    }
}
/// Original-equation verification preference.
#[derive(Clone, Debug)]
pub enum VerifyMode {
    /// Use the construction/verification policy from PLAN §8.8.4.
    Auto,
    /// Explicitly verify every candidate.
    Always,
    /// Permit skipping optional polynomial checks; mandatory branch checks remain.
    Never,
}
/// Limit on explicit generic conditions, not permission to discard internal assumptions.
#[derive(Clone, Debug)]
pub enum MaxExtra {
    /// Keep conditions in computational metadata only.
    Zero,
    /// Attach all required generic conditions.
    All,
    /// Attach a full conjunction only if it contains at most this many conditions.
    Count(u32),
}
/// Exactness/verification provenance of a solution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Verification {
    /// Original residual proved zero exactly.
    Exact,
    /// Correct by an exact polynomial or linear construction.
    ByConstruction,
    /// Original residual certified numerically.
    Numeric {
        /// Certified decimal precision reported by the verifier.
        digits: u32,
    },
    /// Verification has not been completed.
    Unverified,
}
/// One solution, with conditions, multiplicity and optional numeric presentation data.
#[derive(Clone, Debug)]
pub struct Solution {
    /// Variable/value rules; free variables do not appear.
    pub rules: Vec<(Expr, Expr)>,
    /// Condition excluding the domains of generated parameters.
    pub condition: Option<Expr>,
    /// Generated parameter expressions and their required domains.
    pub constants: Vec<(Expr, Domain)>,
    /// Positive root multiplicity; Wolfram output repeats the rule list this many times.
    pub multiplicity: u32,
    /// Verification provenance.
    pub verification: Verification,
    /// Complex approximations in rule-variable order, when available.
    pub numeric: Option<Vec<(f64, f64)>>,
}
/// Solution shape shared by exact solving, numerical solving and reduction.
#[derive(Clone, Debug)]
pub enum SolutionSet {
    /// Finite candidates; an empty vector is the empty solution set.
    Finite(Vec<Solution>),
    /// All assignments satisfy the input.
    All,
    /// Boolean region with interval data for display.
    Region {
        /// Boolean condition defining the region.
        cond: Expr,
        /// Real intervals for display.
        intervals: Vec<Interval>,
    },
    /// Unsupported input; a dispatcher retains its original source call.
    Unevaluated,
}
/// Real interval for reduction and visualization.
#[derive(Clone, Debug)]
pub struct Interval {
    /// Lower endpoint.
    pub lo: Bound,
    /// Upper endpoint.
    pub hi: Bound,
}
/// Finite open/closed endpoint or an infinite endpoint.
#[derive(Clone, Debug)]
pub enum Bound {
    /// Negative infinity.
    NegInf,
    /// Positive infinity.
    PosInf,
    /// Included finite endpoint.
    Closed(Expr),
    /// Excluded finite endpoint.
    Open(Expr),
}
/// Complete computational result, before expression formatting.
#[derive(Clone, Debug)]
pub struct SolveOutcome {
    /// Solution shape.
    pub set: SolutionSet,
    /// Recorded computation, when enabled.
    pub steps: Option<Steps>,
    /// Diagnostics in emission order.
    pub messages: Vec<Message>,
}
impl SolutionSet {
    /// Wolfram result shape, repeating multiplicities and preserving conditions.
    /// Unevaluated yields a neutral marker; callers must retain their original Solve call.
    pub fn to_expr(&self) -> Expr {
        match self {
            Self::All => Expr::call(B::LIST, [Expr::call(B::LIST, [])]),
            Self::Region { cond, .. } => cond.clone(),
            Self::Unevaluated => Expr::sym(Symbol::intern("Unevaluated")),
            Self::Finite(solutions) => {
                let mut rows = vec![];
                for s in solutions {
                    let mut conditions = s.condition.iter().cloned().collect::<Vec<_>>();
                    for (c, d) in &s.constants {
                        conditions.push(Expr::call(B::ELEMENT, [c.clone(), domain_expr(*d)]));
                    }
                    let condition = match conditions.len() {
                        0 => None,
                        1 => conditions.pop(),
                        _ => Some(Expr::call(B::AND, conditions)),
                    };
                    let conditional = |value: Expr| {
                        if let Some(cond) = &condition {
                            Expr::call(B::CONDITIONAL_EXPRESSION, [value, cond.clone()])
                        } else {
                            value
                        }
                    };
                    let row = if s.rules.is_empty() {
                        conditional(Expr::call(B::LIST, []))
                    } else {
                        Expr::call(
                            B::LIST,
                            s.rules.iter().map(|(v, value)| {
                                Expr::call(B::RULE, [v.clone(), conditional(value.clone())])
                            }),
                        )
                    };
                    for _ in 0..s.multiplicity {
                        rows.push(row.clone());
                    }
                }
                Expr::call(B::LIST, rows)
            }
        }
    }
}
fn domain_expr(d: Domain) -> Expr {
    Expr::sym(match d {
        Domain::Complexes => B::COMPLEXES,
        Domain::Reals => B::REALS,
        Domain::Integers => B::INTEGERS,
        Domain::Rationals => B::RATIONALS,
    })
}

/// Public solver failure contract; unsupported results may be handled by a dispatcher.
#[derive(Debug, thiserror::Error)]
pub enum SolveError {
    /// Injected cancellation, deadline or budget failure.
    #[error(transparent)]
    Abort(#[from] om_num::ctx::Abort),
    /// An unsupported algorithm or representation.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// Malformed input or variables.
    #[error("invalid input: {0}")]
    Invalid(String),
}
