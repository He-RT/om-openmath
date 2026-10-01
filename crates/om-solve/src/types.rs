//! Solver contracts introduced at their first real consumer.
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
