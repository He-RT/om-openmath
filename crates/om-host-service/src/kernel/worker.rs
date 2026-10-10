//! Real main CAS work is restored from an explicit immutable state. No worker-owned active pointer.
mod lifecycle;
mod runtime;
use super::{KernelState, KernelStateError, KernelStatePool};
use crate::{
    document::{coordinator::kernel_file, validate_snapshot},
    protocol::{Serial, generated::NativeSourceSnapshot},
};
pub(crate) use lifecycle::Lifecycle;
use om_kernel::{
    CellBoundary,
    checkpoint::{CheckpointBinding, CheckpointLimits, CheckpointRestore},
    config::GeneralConfig,
};
use om_num::ctx::{Clock, Interrupt};
pub use runtime::KernelWorker;
use sha2::{Digest, Sha256};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

/// Trusted internal job. Public model DTOs cannot choose this state or fabricate source authority.
pub struct KernelJob {
    /// Current actual host runtime identity.
    pub runtime_instance_id: String,
    /// Current open document lifetime, distinct from a historical checkpoint producer.
    pub document_generation: Serial,
    /// Operation identity already chosen by the coordinator.
    pub operation_id: String,
    /// Explicit state selected by the coordinator; registry presence does not mean accepted.
    pub parent_checkpoint_ref: String,
    /// Exact committed source snapshot, including its canonical hash and cell revisions.
    pub source: NativeSourceSnapshot,
    /// Current trusted calculation/display settings, without credentials.
    pub general: GeneralConfig,
    /// Current calculation config revision.
    pub config_revision: Serial,
    /// One selected current Math cell; each boundary is independently eligible for acceptance.
    pub cell_id: String,
    /// Fresh operation token. Never clear or share a previous operation's token.
    pub cancel: Arc<AtomicBool>,
}
/// Actual immutable candidate production facts. Only this worker constructs them.
pub struct KernelCandidate {
    pub(super) checkpoint_ref: String,
    pub(super) state: Arc<KernelState>,
    pub(super) provenance: CandidateProvenance,
    pub(super) boundary: CellBoundary,
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) lifecycle: Arc<Lifecycle>,
}
/// Producer scope for the future final source/config/active-parent acceptance barrier.
#[derive(Clone, Debug)]
pub struct CandidateProvenance {
    /// Actual submitting host runtime.
    pub runtime_instance_id: String,
    /// Actual current document lifetime.
    pub document_generation: Serial,
    /// Actual producing operation.
    pub operation_id: String,
    /// Selected base state; candidate must never replace a different active parent.
    pub parent_checkpoint_ref: String,
    /// Source snapshot used for reconcile and execution.
    pub source: NativeSourceSnapshot,
    /// Current calculation revision.
    pub config_revision: Serial,
    /// SHA256 of exact GeneralConfig JSON used by the real session.
    pub general_hash: String,
    /// SHA256 of raw selected cell UTF8 bytes.
    pub cell_source_hash: String,
}
impl KernelCandidate {
    /// Temporary frozen state ref. This is not an accepted result or write grant.
    pub fn checkpoint_ref(&self) -> &str {
        &self.checkpoint_ref
    }
    /// Original immutable encoded state for physical publication, still unaccepted.
    pub fn state(&self) -> &Arc<KernelState> {
        &self.state
    }
    /// Exact real production source/operation/config facts.
    pub fn provenance(&self) -> &CandidateProvenance {
        &self.provenance
    }
    /// Actual end status and successful statement history facts, including partial error effects.
    pub fn boundary(&self) -> &CellBoundary {
        &self.boundary
    }
    /// Stop after freezing still prevents future coordinator acceptance.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }
}
/// No failure silently advances main state or creates an accepted checkpoint.
#[derive(Debug)]
pub enum KernelWorkerError {
    /// Wrong trusted source/provenance or invalid target cell.
    Invalid,
    /// Stop before running/restoring.
    Cancelled,
    /// Closing rejects admission/publication; background join remains separate.
    Closing,
    /// Two bounded waiting jobs are already queued.
    QueueFull,
    /// Operation identity was previously admitted.
    DuplicateOperation,
    /// A previous operation's actual token was reused.
    ReusedToken,
    /// Lifetime admission budget reached.
    Limit,
    /// Actual state/codec/resource failure.
    State(KernelStateError),
    /// Actual kernel/parser/dependency failure code.
    Kernel(String),
    /// Worker panic or poisoned internal owner, never a fabricated CAS success.
    Internal,
}
impl From<KernelStateError> for KernelWorkerError {
    fn from(value: KernelStateError) -> Self {
        Self::State(value)
    }
}
impl std::fmt::Display for KernelWorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::State(e) => e.fmt(f),
            Self::Kernel(code) => f.write_str(code),
            _ => write!(f, "{self:?}"),
        }
    }
}
impl std::error::Error for KernelWorkerError {}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn identity(id: &str) -> bool {
    !id.is_empty() && id.len() <= 256 && !id.chars().any(char::is_control)
}
fn execute(
    job: KernelJob,
    lifecycle: Arc<Lifecycle>,
    parent: Arc<KernelState>,
    pool: &Mutex<KernelStatePool>,
    limits: CheckpointLimits,
    clock: Arc<dyn Clock>,
    closed: &AtomicBool,
) -> Result<KernelCandidate, KernelWorkerError> {
    validate_snapshot(&job.source).map_err(|_| KernelWorkerError::Invalid)?;
    let old = parent.binding();
    let source = kernel_file(&job.source.file);
    let changed = om_kernel::source::math_source_changed(parent.source(), &source)
        || om_kernel::source::calculation_settings_changed(parent.general(), &job.general);
    if !identity(&job.runtime_instance_id)
        || !identity(&job.operation_id)
        || job.document_generation.get() == 0
        || job.source.document_id != old.document_id
        || job.source.revision.get() < old.source_revision
        || job.source.execution_epoch.get() < old.execution_epoch
        || (changed && job.source.execution_epoch.get() == old.execution_epoch)
        || (job.source.revision.get() == old.source_revision
            && job.source.snapshot_hash != old.source_snapshot_hash)
        || job.general.eval_timeout_ms == 0
        || job.general.eval_timeout_ms > crate::protocol::MAX_SERIAL
    {
        return Err(KernelWorkerError::Invalid);
    }
    if closed.load(Ordering::Acquire) {
        return Err(KernelWorkerError::Closing);
    }
    if job.cancel.load(Ordering::Acquire) {
        return Err(KernelWorkerError::Cancelled);
    }
    let binding = CheckpointBinding {
        document_id: job.source.document_id.clone(),
        document_generation: job.document_generation.get(),
        source_revision: job.source.revision.get(),
        execution_epoch: job.source.execution_epoch.get(),
        kernel_state_revision: Serial::new(old.kernel_state_revision)
            .map_err(|_| KernelWorkerError::Invalid)?
            .checked_next()
            .map_err(|_| KernelWorkerError::Limit)?
            .get(),
        source_snapshot_hash: job.source.snapshot_hash.clone(),
        build: old.build.clone(),
    };
    // Restore and compute without any registry/control lock. The old checkpoint remains pinned.
    let restore_ctx = Interrupt {
        flag: job.cancel.clone(),
        ..Interrupt::default()
    };
    let mut work = parent.restore(
        CheckpointRestore {
            binding: old,
            source: parent.source(),
            general: parent.general(),
            clock: Some(clock),
            cancel: job.cancel.clone(),
        },
        limits,
        &restore_ctx,
    )?;
    let delta = job.source.execution_epoch.get() - old.execution_epoch;
    work.reconcile(
        source,
        job.general.clone(),
        delta > 1 || (delta > 0 && !changed),
    )
    .map_err(KernelWorkerError::Kernel)?;
    if job.cancel.load(Ordering::Acquire) {
        return Err(KernelWorkerError::Cancelled);
    }
    let cell = work
        .session_mut()
        .notebook
        .cells
        .iter()
        .find(|c| c.id == job.cell_id)
        .ok_or(KernelWorkerError::Invalid)?;
    let cell_source_hash = hash(cell.source.as_bytes());
    let boundary = work
        .run_current_cell(&job.cell_id)
        .map_err(KernelWorkerError::Kernel)?;
    // A real failed/cancelled statement may have earlier effects. Freeze them exactly for reporting;
    // do not claim rollback. The independent operation flag still forbids later main acceptance.
    let freeze_ctx = Interrupt::default();
    let frozen = KernelState::capture(work.session_mut(), binding, limits, &freeze_ctx)?;
    let general_hash =
        hash(&serde_json::to_vec(&job.general).map_err(|_| KernelWorkerError::Invalid)?);
    if closed.load(Ordering::Acquire) {
        return Err(KernelWorkerError::Closing);
    }
    let (checkpoint_ref, state) = {
        let mut pool = pool.lock().map_err(|_| KernelWorkerError::Internal)?;
        let id = pool.register(frozen)?;
        let state = pool.state(&id)?;
        (id, state)
    };
    lifecycle.prepared()?;
    Ok(KernelCandidate {
        checkpoint_ref,
        state,
        boundary,
        cancel: job.cancel,
        lifecycle,
        provenance: CandidateProvenance {
            runtime_instance_id: job.runtime_instance_id,
            document_generation: job.document_generation,
            operation_id: job.operation_id,
            parent_checkpoint_ref: job.parent_checkpoint_ref,
            source: job.source,
            config_revision: job.config_revision,
            general_hash,
            cell_source_hash,
        },
    })
}
