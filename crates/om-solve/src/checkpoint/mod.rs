//! Structured actual solver evidence; no display strings or inferred verification are decoded.
mod atoms;
mod kinds;
mod solutions;
mod steps;
use om_core::{Expr, Interrupt};
use serde::{Deserialize, Serialize};
pub use solutions::{SolutionsData, decode_solutions, encode_solutions};
pub use steps::{StepsData, decode_steps, encode_steps};

/// Independent evidence budgets; expression data is bounded separately by the caller's graph.
#[derive(Clone, Copy, Debug)]
pub struct EvidenceLimits {
    /// Actual flat step events.
    pub max_steps: usize,
    /// Maximum parent/child path.
    pub max_depth: usize,
    /// Total captured expression roots across the caller's packet.
    pub max_expressions: usize,
    /// Step labels/IDs/named bindings and diagnostic text.
    pub max_text_bytes: usize,
}
impl Default for EvidenceLimits {
    fn default() -> Self {
        Self {
            max_steps: 10000,
            max_depth: 128,
            max_expressions: 100000,
            max_text_bytes: 65536,
        }
    }
}
/// Rejected evidence never becomes a fabricated solved result or reconstructed derivation.
#[derive(Debug, thiserror::Error)]
pub enum EvidenceError {
    /// Unknown kind/rule, malformed reference/shape/order.
    #[error("invalid solver evidence checkpoint")]
    Invalid,
    /// Evidence/graph/text budgets exceeded.
    #[error("solver evidence checkpoint exceeds its limits")]
    Limit,
    /// Caller operation interrupted.
    #[error(transparent)]
    Abort(#[from] om_core::Abort),
}
fn put(
    roots: &mut Vec<Expr>,
    expr: &Expr,
    limits: EvidenceLimits,
    ctx: &Interrupt,
) -> Result<u32, EvidenceError> {
    ctx.tick()?;
    if roots.len() >= limits.max_expressions {
        return Err(EvidenceError::Limit);
    }
    let id = u32::try_from(roots.len()).map_err(|_| EvidenceError::Limit)?;
    roots.push(expr.clone());
    Ok(id)
}
fn get(roots: &[Expr], id: u32, ctx: &Interrupt) -> Result<Expr, EvidenceError> {
    ctx.tick()?;
    roots
        .get(id as usize)
        .cloned()
        .ok_or(EvidenceError::Invalid)
}
fn texts(text: &str, limits: EvidenceLimits) -> Result<(), EvidenceError> {
    if text.len() > limits.max_text_bytes {
        Err(EvidenceError::Limit)
    } else {
        Ok(())
    }
}
fn put_many(
    roots: &mut Vec<Expr>,
    exprs: &[Expr],
    limits: EvidenceLimits,
    ctx: &Interrupt,
) -> Result<Vec<u32>, EvidenceError> {
    exprs
        .iter()
        .map(|expr| put(roots, expr, limits, ctx))
        .collect()
}
fn get_many(roots: &[Expr], ids: &[u32], ctx: &Interrupt) -> Result<Vec<Expr>, EvidenceError> {
    ids.iter().map(|&id| get(roots, id, ctx)).collect()
}
