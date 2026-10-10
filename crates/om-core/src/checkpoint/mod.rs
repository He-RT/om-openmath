//! Flat ordered expression DAGs; no recursive serde, code evaluation or persisted addresses.
mod decode;
mod encode;
use crate::Abort;
use crate::{Expr, ExprKind, Interrupt};
pub use decode::decode_expressions;
pub use encode::encode_expressions;
use om_num::checkpoint::{NumberCodecError, NumberLimits};
use std::collections::BTreeSet;

/// Bounded raw equality across separately decoded graphs, including numeric category/precision/bits.
/// Pair memoization prevents shared DAGs from expanding into exponentially many comparison paths.
pub fn same_representation(
    left: &Expr,
    right: &Expr,
    limits: ExprLimits,
    ctx: &Interrupt,
) -> Result<bool, ExprCodecError> {
    let mut work = vec![(left, right)];
    let mut seen = BTreeSet::new();
    while let Some((a, b)) = work.pop() {
        ctx.tick()?;
        let key = (a.allocation_key(), b.allocation_key());
        if !seen.insert(key) {
            continue;
        }
        if seen.len() > limits.max_nodes {
            return Err(ExprCodecError::Limit);
        }
        match (a.kind(), b.kind()) {
            (ExprKind::Number(a), ExprKind::Number(b)) => {
                if om_num::checkpoint::encode_number(a, limits.numbers, ctx)?
                    != om_num::checkpoint::encode_number(b, limits.numbers, ctx)?
                {
                    return Ok(false);
                }
            }
            (ExprKind::Symbol(a), ExprKind::Symbol(b)) => {
                if a.name() != b.name() {
                    return Ok(false);
                }
            }
            (ExprKind::String(a), ExprKind::String(b)) => {
                if a != b {
                    return Ok(false);
                }
            }
            (ExprKind::Normal(a), ExprKind::Normal(b)) => {
                if a.args.len() != b.args.len() {
                    return Ok(false);
                }
                if work
                    .len()
                    .checked_add(a.args.len() + 1)
                    .is_none_or(|n| n > limits.max_edges)
                {
                    return Err(ExprCodecError::Limit);
                }
                work.push((&a.head, &b.head));
                work.extend(a.args.iter().zip(&b.args));
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}

/// Explicit encode/decode budgets; exceeding one fails rather than truncating saved state.
#[derive(Clone, Copy, Debug)]
pub struct ExprLimits {
    /// Complete graph bytes.
    pub max_bytes: usize,
    /// Stored unique expression allocations.
    pub max_nodes: usize,
    /// Ordered expression roots.
    pub max_roots: usize,
    /// Longest head/argument path.
    pub max_depth: usize,
    /// Total graph references, counting heads and ordered arguments.
    pub max_edges: usize,
    /// Unique names in this graph, including builtins.
    pub max_symbols: usize,
    /// Per-symbol/string UTF8 bytes.
    pub max_text_bytes: usize,
    /// Per-numeric-atom limits.
    pub numbers: NumberLimits,
}
impl Default for ExprLimits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_nodes: 200_000,
            max_roots: 100_000,
            max_depth: 1024,
            max_edges: 1_000_000,
            max_symbols: 8192,
            max_text_bytes: 4 * 1024 * 1024,
            numbers: NumberLimits::default(),
        }
    }
}
/// Closed decode failure; a rejected graph never becomes an executable partial result.
#[derive(Debug, thiserror::Error)]
pub enum ExprCodecError {
    /// Unknown version/node, malformed UTF8/length/reference or noncanonical input.
    #[error("invalid expression checkpoint")]
    Invalid,
    /// An explicit byte/node/depth/edge/text limit was exceeded.
    #[error("expression checkpoint exceeds its limits")]
    Limit,
    /// Actual numeric atom validation failure.
    #[error(transparent)]
    Number(#[from] NumberCodecError),
    /// Host-supplied operation cancellation/deadline/work budget.
    #[error(transparent)]
    Abort(#[from] Abort),
}
struct Writer {
    bytes: Vec<u8>,
    max: usize,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<(), ExprCodecError> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > self.max)
        {
            return Err(ExprCodecError::Limit);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn size(&mut self, size: usize) -> Result<(), ExprCodecError> {
        self.put(
            &u32::try_from(size)
                .map_err(|_| ExprCodecError::Limit)?
                .to_le_bytes(),
        )
    }
    fn blob(&mut self, bytes: &[u8]) -> Result<(), ExprCodecError> {
        self.size(bytes.len())?;
        self.put(bytes)
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, size: usize) -> Result<&'a [u8], ExprCodecError> {
        let end = self
            .position
            .checked_add(size)
            .filter(|&n| n <= self.bytes.len())
            .ok_or(ExprCodecError::Invalid)?;
        let bytes = &self.bytes[self.position..end];
        self.position = end;
        Ok(bytes)
    }
    fn size(&mut self) -> Result<usize, ExprCodecError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| ExprCodecError::Invalid)?,
        ) as usize)
    }
    fn blob(&mut self) -> Result<&'a [u8], ExprCodecError> {
        let size = self.size()?;
        self.take(size)
    }
}
