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
