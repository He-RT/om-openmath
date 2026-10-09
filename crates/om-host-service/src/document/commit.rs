//! Original-ID admission, source gate and receipt reconciliation. No SQLite or UI waiting here.
use super::{
    SourceDocument,
    preview::{FrozenPreviewPlan, PreviewError, PreviewService},
};
use crate::{
    protocol::{Nullable, generated::*},
    references::ReferenceScope,
};
use std::{collections::BTreeMap, sync::Arc};

/// Commit transitions are actual owner facts, separate from operation receipt acknowledgement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommitError {
    /// Another unknown/pending write holds this document's logical gate.
    Busy,
    /// Wrong identity, source, fence or acknowledgement shape.
    Invalid,
    /// Requested cancellation won before the commit barrier.
    Cancelled,
    /// Actual preview/grant/epoch/expiry failure.
    Preview(PreviewError),
}
struct Pending {
    plan: Option<Arc<FrozenPreviewPlan>>,
    state: NativeCommitState,
    fence: Option<NativeEditorFence>,
    admitted: bool,
    manual: bool,
}
/// One document, one logical write gate; reads/cancel are short methods while storage works
/// independently. Completed original IDs can be reconciled before checking old preview versions.
pub struct DocumentCommitController {
    owner: SourceDocument,
    previews: PreviewService,
    pending: BTreeMap<String, Pending>,
    preview_operations: BTreeMap<String, String>,
    gate: Option<String>,
    runtime: String,
}
impl DocumentCommitController {
    /// Trusted attachment from an actually verified store head, with a fresh host entropy key.
    pub fn new(owner: SourceDocument, runtime: String, key: [u8; 32]) -> Self {
        Self {
            owner,
            previews: PreviewService::new(key),
            pending: BTreeMap::new(),
            preview_operations: BTreeMap::new(),
            gate: None,
            runtime,
        }
    }
    /// Read only confirmed source; an in-flight IO task cannot update it speculatively.
    pub fn owner(&self) -> &SourceDocument {
        &self.owner
    }
    /// Inspect temporary source with current trusted host bindings.
    pub fn snapshot(
        &mut self,
        context: super::coordinator::SourceCoordinator,
        complete: std::collections::BTreeSet<String>,
        scope: &ReferenceScope,
        now: u64,
    ) -> Result<String, CommitError> {
        self.previews
            .snapshot(&self.owner, context, complete, scope, now)
            .map_err(CommitError::Preview)
    }
    /// Preview is still separate from admission/writing.
    pub fn preview(
        &mut self,
        input: &[u8],
        scope: &ReferenceScope,
        now: u64,
        time: String,
    ) -> Result<NativePreviewData, CommitError> {
        self.previews
            .inspect(input, &self.owner, scope, now, time)
            .map_err(CommitError::Preview)
    }
    /// Begin only after original immutable preview resolution. Return a plan for physical durable
    /// admission; this does not mean a write has run or that a COMMIT may enter yet.
    pub fn begin(
        &mut self,
        reference: &str,
        scope: &ReferenceScope,
        now: u64,
    ) -> Result<Option<Arc<FrozenPreviewPlan>>, CommitError> {
        if let Some(operation) = self.preview_operations.get(reference)
            && let Some(pending) = self.pending.get(operation)
        {
            // The caller must first query storage for already-completed/unknown original IDs.
            if self.runtime != scope.runtime
                || pending.state.document_id != scope.document
                || !scope.can_read
            {
                return Err(CommitError::Invalid);
            }
            return Ok(pending.plan.clone());
        }
        if self.gate.is_some() || self.pending.len() >= 4096 {
            return Err(CommitError::Busy);
        }
        let plan = self
            .previews
            .resolve_for_commit(reference, &self.owner, scope, now)
            .map_err(CommitError::Preview)?;
        let record = &plan.commit.commit;
        let operation = record.operation_id.clone();
        let state = NativeCommitState {
            protocol_version: 1,
            document_id: record.document_id.clone(),
            operation_id: operation.clone(),
            request_hash: record.request_hash.clone(),
            phase: NativeCommitStatePhase::AwaitingAdmission,
            cancel_requested: false,
            receipt: Nullable(None),
        };
        self.pending.insert(
            operation.clone(),
            Pending {
                plan: Some(plan.clone()),
                state,
                fence: None,
                admitted: false,
                manual: false,
            },
        );
        self.preview_operations
            .insert(reference.into(), operation.clone());
        self.gate = Some(operation);
        Ok(Some(plan))
    }
    /// Host manual edit can preserve syntactically incomplete source; it is not an Agent preview.
    pub fn manual(
        &mut self,
        operations: &[crate::protocol::generated::NativeSourceOperation],
        scope: &ReferenceScope,
        coordinator: &super::coordinator::SourceCoordinator,
        time: String,
    ) -> Result<NativeSourceCommit, CommitError> {
        if self.gate.is_some() || self.pending.len() >= 4096 {
            return Err(CommitError::Busy);
        }
        let operation = format!("manual-{}", uuid_like_id()?);
        let transaction = format!("transaction-{}", uuid_like_id()?);
        let event = format!("event-{}", uuid_like_id()?);
        let mutation = coordinator
            .prepare(&self.owner, operations, operation, transaction, event, time)
            .map_err(|_| CommitError::Invalid)?;
        let plan = Arc::new(FrozenPreviewPlan {
            scope: scope.clone(),
            commit: mutation.commit.clone(),
            operations: operations.to_vec(),
            invalidation: mutation.invalidation,
            assigned: BTreeMap::new(),
            plan_hash: String::new(),
        });
        let record = &plan.commit.commit;
        let id = record.operation_id.clone();
        self.pending.insert(
            id.clone(),
            Pending {
                state: NativeCommitState {
                    protocol_version: 1,
                    document_id: record.document_id.clone(),
                    operation_id: id.clone(),
                    request_hash: record.request_hash.clone(),
                    phase: NativeCommitStatePhase::AwaitingAdmission,
                    cancel_requested: false,
                    receipt: Nullable(None),
                },
                plan: Some(plan),
                fence: None,
                admitted: false,
                manual: true,
            },
        );
        self.gate = Some(id);
        Ok(mutation.commit)
    }
    /// Receipt from real StorageService admission; unknown/failed admission cannot execute source.
    pub fn admitted(
        &mut self,
        operation: &str,
        admission: &NativeSourceAdmission,
    ) -> Result<NativeCommitState, CommitError> {
        let item = self
            .pending
            .get_mut(operation)
            .ok_or(CommitError::Invalid)?;
        if admission.protocol_version != 1
            || admission.document_id != item.state.document_id
            || admission.operation_id != operation
            || admission.request_hash != item.state.request_hash
        {
            return Err(CommitError::Invalid);
        }
        if admission.phase != NativeSourceAdmissionPhase::Completed && admission.receipt.0.is_some()
        {
            return Err(CommitError::Invalid);
        }
        if admission.phase == NativeSourceAdmissionPhase::Completed {
            let receipt = admission.receipt.0.clone().ok_or(CommitError::Invalid)?;
            return self.accept_receipt(operation, &receipt);
        }
        // Reconnection cannot rewind an active barrier or revive a settled original operation.
        if item.plan.is_none()
            || item.admitted
            || matches!(
                item.state.phase,
                NativeCommitStatePhase::Committing
                    | NativeCommitStatePhase::Unknown
                    | NativeCommitStatePhase::Failed
                    | NativeCommitStatePhase::Completed
            )
        {
            return Ok(item.state.clone());
        }
        if matches!(
            admission.phase,
            NativeSourceAdmissionPhase::Cancelled | NativeSourceAdmissionPhase::Failed
        ) {
            item.state.phase = if admission.phase == NativeSourceAdmissionPhase::Cancelled {
                NativeCommitStatePhase::Cancelled
            } else {
                NativeCommitStatePhase::Failed
            };
            item.plan = None;
            self.gate = None;
            return Ok(item.state.clone());
        }
        if admission.phase != NativeSourceAdmissionPhase::Accepted
            || admission.created_by_runtime != self.runtime
        {
            item.state.phase = NativeCommitStatePhase::Unknown;
            return Ok(item.state.clone());
        }
        item.admitted = true;
        item.state.phase = if item.state.cancel_requested {
            NativeCommitStatePhase::Cancelled
        } else {
            NativeCommitStatePhase::AwaitingFence
        };
        // Cancelled admission is settled durably by the physical port before releasing the gate.
        Ok(item.state.clone())
    }
    /// Bind the exact native MainActor fence. No draft/marked text is implicitly submitted here.
    pub fn fenced(
        &mut self,
        operation: &str,
        fence: NativeEditorFence,
        now: u64,
    ) -> Result<NativeCommitState, CommitError> {
        let item = self
            .pending
            .get_mut(operation)
            .ok_or(CommitError::Invalid)?;
        if item.state.cancel_requested {
            return Err(CommitError::Cancelled);
        }
        if !item.admitted
            || item.state.phase != NativeCommitStatePhase::AwaitingFence
            || !valid_fence(
                item.plan.as_ref().ok_or(CommitError::Invalid)?,
                &self.runtime,
                &fence,
                now,
                item.manual,
            )
        {
            return Err(CommitError::Invalid);
        }
        item.fence = Some(fence);
        item.state.phase = NativeCommitStatePhase::Writing;
        Ok(item.state.clone())
    }
    /// Final short owner barrier, called by the physical writer immediately before SQLite COMMIT.
    /// cancel/current scope/native fence validation linearizes with this transition.
    pub fn enter_commit(
        &mut self,
        operation: &str,
        fence_id: &str,
        current: &ReferenceScope,
        now: u64,
    ) -> Result<NativeCommitState, CommitError> {
        let item = self
            .pending
            .get_mut(operation)
            .ok_or(CommitError::Invalid)?;
        if item.state.cancel_requested {
            return Err(CommitError::Cancelled);
        }
        let fence = item.fence.as_ref().ok_or(CommitError::Invalid)?;
        if item.state.phase != NativeCommitStatePhase::Writing
            || item.plan.as_ref().ok_or(CommitError::Invalid)?.scope != *current
            || fence.fence_id != fence_id
            || !valid_fence(
                item.plan.as_ref().ok_or(CommitError::Invalid)?,
                &self.runtime,
                fence,
                now,
                item.manual,
            )
            || self.owner.snapshot().snapshot_hash
                != item
                    .plan
                    .as_ref()
                    .ok_or(CommitError::Invalid)?
                    .commit
                    .before
                    .snapshot_hash
        {
            return Err(CommitError::Invalid);
        }
        item.state.phase = NativeCommitStatePhase::Committing;
        Ok(item.state.clone())
    }
    /// Direct stop never waits for disk. After the barrier it is pending reconciliation, not undo.
    pub fn cancel(&mut self, operation: &str) -> Result<NativeCommitState, CommitError> {
        let item = self
            .pending
            .get_mut(operation)
            .ok_or(CommitError::Invalid)?;
        if !matches!(
            item.state.phase,
            NativeCommitStatePhase::Completed
                | NativeCommitStatePhase::Cancelled
                | NativeCommitStatePhase::Failed
        ) {
            item.state.cancel_requested = true;
            if !matches!(
                item.state.phase,
                NativeCommitStatePhase::Committing | NativeCommitStatePhase::Unknown
            ) {
                item.state.phase = NativeCommitStatePhase::Cancelled;
            }
        }
        Ok(item.state.clone())
    }
    /// Any unproven acknowledgement stops dependent writes until the original ID is read back.
    pub fn unknown(&mut self, operation: &str) -> Result<NativeCommitState, CommitError> {
        let item = self
            .pending
            .get_mut(operation)
            .ok_or(CommitError::Invalid)?;
        if item.state.phase != NativeCommitStatePhase::Completed {
            item.state.phase = NativeCommitStatePhase::Unknown;
        }
        Ok(item.state.clone())
    }
    /// Publish only matching actual durable source/receipt facts. A late stop cannot overwrite it.
    pub fn accept_receipt(
        &mut self,
        operation: &str,
        receipt: &NativeDurableSourceReceipt,
    ) -> Result<NativeCommitState, CommitError> {
        let item = self
            .pending
            .get_mut(operation)
            .ok_or(CommitError::Invalid)?;
        if item.state.phase == NativeCommitStatePhase::Completed {
            let original = item.state.receipt.0.as_ref().ok_or(CommitError::Invalid)?;
            if serde_json::to_vec(original).map_err(|_| CommitError::Invalid)?
                == serde_json::to_vec(receipt).map_err(|_| CommitError::Invalid)?
            {
                return Ok(item.state.clone());
            }
            return Err(CommitError::Invalid);
        }
        if !matches!(
            item.state.phase,
            NativeCommitStatePhase::Committing
                | NativeCommitStatePhase::Unknown
                | NativeCommitStatePhase::AwaitingAdmission
        ) {
            return Err(CommitError::Invalid);
        }
        self.owner
            .accept(
                &item.plan.as_ref().ok_or(CommitError::Invalid)?.commit,
                receipt,
            )
            .map_err(|_| CommitError::Invalid)?;
        item.state.phase = NativeCommitStatePhase::Completed;
        item.state.receipt = Nullable(Some(receipt.clone()));
        item.plan = None;
        item.fence = None;
        self.gate = None;
        Ok(item.state.clone())
    }
    /// Known physical rollback/cancellation acknowledgement may release the document gate.
    /// Absence alone is not enough: the caller checks a healthy current store before invoking it.
    pub fn settle_no_commit(
        &mut self,
        operation: &str,
        cancelled: bool,
    ) -> Result<NativeCommitState, CommitError> {
        let item = self
            .pending
            .get_mut(operation)
            .ok_or(CommitError::Invalid)?;
        if item.state.phase == NativeCommitStatePhase::Completed {
            return Err(CommitError::Invalid);
        }
        item.state.phase = if cancelled {
            NativeCommitStatePhase::Cancelled
        } else {
            NativeCommitStatePhase::Failed
        };
        item.plan = None;
        item.fence = None;
        if self.gate.as_deref() == Some(operation) {
            self.gate = None;
        }
        Ok(item.state.clone())
    }
    /// A current source read exposes actual in-flight/unknown operation state separately.
    pub fn operation(&self, id: &str) -> Option<NativeCommitState> {
        self.pending.get(id).map(|p| p.state.clone())
    }
    /// Original preview IDs retain compact settled facts without holding full source snapshots.
    pub fn preview_operation(&self, reference: &str) -> Option<NativeCommitState> {
        self.preview_operations
            .get(reference)
            .and_then(|id| self.operation(id))
    }
}
fn valid_fence(
    plan: &FrozenPreviewPlan,
    runtime: &str,
    fence: &NativeEditorFence,
    now: u64,
    manual: bool,
) -> bool {
    if fence.protocol_version != 1
        || fence.runtime_instance_id != runtime
        || fence.document_id != plan.commit.commit.document_id
        || fence.document_generation != plan.commit.generation
        || fence.source_revision != plan.commit.before.revision
        || fence.source_snapshot_hash != plan.commit.before.snapshot_hash
        || now < fence.issued_ms.get()
        || now >= fence.expires_ms.get()
        || fence.expires_ms.get().saturating_sub(fence.issued_ms.get()) > 2000
        || fence.targets.len() > 1000
    {
        return false;
    }
    let mut ids = std::collections::BTreeSet::new();
    for target in &fence.targets {
        if !ids.insert(&target.cell_id) || target.is_composing || (target.is_dirty && !manual) {
            return false;
        }
        let Some(cell) = plan
            .commit
            .before
            .file
            .cells
            .iter()
            .find(|c| c.id == target.cell_id)
        else {
            return false;
        };
        let Some(revision) = plan
            .commit
            .before
            .cell_revisions
            .iter()
            .find(|r| r.cell_id == target.cell_id)
        else {
            return false;
        };
        if revision.revision != target.base_cell_revision
            || target.source_hash
                != format!(
                    "{:x}",
                    sha2::Sha256::digest(if manual {
                        plan.commit
                            .after
                            .file
                            .cells
                            .iter()
                            .find(|c| c.id == target.cell_id)
                            .map_or(cell.source.as_bytes(), |c| c.source.as_bytes())
                    } else {
                        cell.source.as_bytes()
                    })
                )
        {
            return false;
        }
    }
    // Every existing cell that changes/deletes/moves must have a verified native barrier target.
    plan.commit
        .commit
        .changed_cell_ids
        .iter()
        .filter(|id| plan.commit.before.file.cells.iter().any(|c| &c.id == *id))
        .all(|id| ids.contains(id))
}
use sha2::Digest;

fn uuid_like_id() -> Result<String, CommitError> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|_| CommitError::Invalid)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
