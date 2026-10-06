//! Fresh result provenance, separate from user-created records and retained symbolic values.
use om_core::Expr;
/// Evidence that an actual registered callback produced the current statement's tail value.
#[derive(Clone)]
pub struct ScientificResult {
    /// Canonical callback name, resolved to a stable catalog identity by the kernel.
    pub name: &'static str,
    /// Actual produced value, used to reject wrappers or later discarded/replaced results.
    pub value: Expr,
}
pub(crate) fn supported(name: &str) -> bool {
    matches!(
        name,
        "Integrate"
            | "NIntegrate"
            | "Ode"
            | "Optimize"
            | "Fit"
            | "Lu"
            | "Qr"
            | "Svd"
            | "Cholesky"
            | "LeastSquares"
            | "Eigensystem"
            | "LinearSolve"
    )
}
