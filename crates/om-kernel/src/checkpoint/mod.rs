//! Whole-session snapshots never replay notebook source or carry model/credential/file owners.
pub(crate) mod records;
pub(crate) mod wire;
use om_core::checkpoint::{ExprCodecError, ExprLimits};
use om_eval::state::{EvalStateError, EvalStateLimits};
use om_solve::checkpoint::{EvidenceError, EvidenceLimits};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, atomic::AtomicBool};

/// Trusted expected persistent provenance and current source/config with new runtime facilities.
pub struct CheckpointRestore<'a> {
    /// Verified original checkpoint descriptor, not a grant for a current writable document.
    pub binding: &'a CheckpointBinding,
    /// Currently verified authoritative source file.
    pub source: &'a crate::protocol::NotebookFile,
    /// Currently verified calculation configuration.
    pub general: &'a crate::config::GeneralConfig,
    /// New caller clock; no clock/handle is serialized.
    pub clock: Option<Arc<dyn om_num::ctx::Clock>>,
    /// New operation-owned cancellation token.
    pub cancel: Arc<AtomicBool>,
}

/// Trusted host provenance. It is a contract input, not a model-granted capability.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CheckpointBinding {
    /// Actual active document identity.
    pub document_id: String,
    /// Actual open document lifetime.
    pub document_generation: u64,
    /// Actual committed source revision.
    pub source_revision: u64,
    /// Actual source/calculation execution epoch.
    pub execution_epoch: u64,
    /// Actual accepted mathematical state revision.
    pub kernel_state_revision: u64,
    /// Host verified source snapshot hash, checked with the expected source file on restore.
    pub source_snapshot_hash: String,
    /// Exact trusted kernel build identity, independent of display version.
    pub build: String,
}
impl CheckpointBinding {
    pub(crate) fn validate(&self) -> Result<(), CheckpointError> {
        if self.document_id.is_empty()
            || self.document_id.len() > 256
            || self.document_generation == 0
            || [
                self.document_generation,
                self.source_revision,
                self.execution_epoch,
                self.kernel_state_revision,
            ]
            .iter()
            .any(|&n| n > ((1u64 << 53) - 1))
            || self.source_snapshot_hash.len() != 64
            || !self
                .source_snapshot_hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.build.is_empty()
            || self.build.len() > 128
        {
            return Err(CheckpointError::Invalid);
        }
        Ok(())
    }
}
/// Combined packet/source/record limits with existing math/graph/evidence budgets.
#[derive(Clone, Copy, Debug)]
pub struct CheckpointLimits {
    /// Complete binary packet bytes.
    pub max_bytes: usize,
    /// Flat JSON source/result metadata bytes.
    pub max_metadata_bytes: usize,
    /// Ordered source cells.
    pub max_cells: usize,
    /// Total retained statements, including suppressed statements.
    pub max_records: usize,
    /// Actual source UTF8 bytes across the document.
    pub max_source_bytes: usize,
    /// Per readonly Explore state encoded bytes, outside metadata.
    pub max_context_bytes: usize,
    /// Persistent evaluator limits.
    pub evaluator: EvalStateLimits,
    /// Shared statement expression graph limits.
    pub expressions: ExprLimits,
    /// Raw solver evidence limits.
    pub evidence: EvidenceLimits,
}
impl Default for CheckpointLimits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_metadata_bytes: 16 * 1024 * 1024,
            max_cells: 10000,
            max_records: 10000,
            max_source_bytes: 2 * 1024 * 1024,
            max_context_bytes: 4 * 1024 * 1024,
            evaluator: EvalStateLimits::default(),
            expressions: ExprLimits::default(),
            evidence: EvidenceLimits::default(),
        }
    }
}
/// A rejected snapshot has no partial writable session and cannot pretend that its state recovered.
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    /// Wrong identity/build/source/config, malformed record or packet.
    #[error("invalid session checkpoint")]
    Invalid,
    /// Actual combined resource limit exceeded.
    #[error("session checkpoint exceeds its limits")]
    Limit,
    /// A running session is not a persistent boundary.
    #[error("session checkpoint needs an idle boundary")]
    NotIdle,
    /// Real graph or numeric validation.
    #[error(transparent)]
    Expr(#[from] ExprCodecError),
    /// Real evaluator validation.
    #[error(transparent)]
    Eval(#[from] EvalStateError),
    /// Real structured evidence validation.
    #[error(transparent)]
    Evidence(#[from] EvidenceError),
    /// Actual host interruption/budget/deadline.
    #[error(transparent)]
    Abort(#[from] om_core::Abort),
}
