//! Pure numerical analysis: no parsing, expressions, clocks, filesystem or networking.
#![forbid(unsafe_code)]
/// Adaptive one-dimensional machine integration with explicit estimates and failure states.
pub mod integration;
/// Small dense real matrices and actual LU/Householder QR factorizations.
pub mod matrix;
/// Real machine special functions and probability tails; no arbitrary-precision claim.
pub mod special;
use om_num::ctx::Abort;
/// Numerical failures remain distinct from successful approximate outputs.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Host cancellation, deadline or operation budget.
    #[error(transparent)]
    Abort(#[from] Abort),
    /// Invalid dimension, shape or requested method.
    #[error("{0}")]
    Input(&'static str),
    /// An intermediate or result cannot be represented by a finite machine number.
    #[error("计算超出有限机器数范围")]
    NonFinite,
    /// No unique solution exists for the requested square solve.
    #[error("矩阵奇异或问题欠定")]
    Singular,
    /// Iteration exhausted its numerical convergence budget.
    #[error("数值算法未收敛")]
    NoConvergence,
    /// Pivots are too small for the documented numerical tolerance.
    #[error("矩阵病态或在当前容差下数值秩不足")]
    IllConditioned,
}
