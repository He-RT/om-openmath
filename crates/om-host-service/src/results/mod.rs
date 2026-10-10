//! Result retention is independent from the active kernel pointer and never reexecutes source.
mod inspect;
/// Dedicated native result worker and bounded request readback, independent from main CAS.
pub mod runtime;
use crate::{
    kernel::{KernelState, acceptance::AcceptedKernelState},
    protocol::{Nullable, Serial, generated::*},
    references::{ReferenceError, ReferenceKind, ReferenceRegistry, ReferenceScope},
};
use om_kernel::{
    checkpoint::{CheckpointLimits, CheckpointRestore},
    retained_results::{RetainedResult, RetainedResultSummary},
};
use om_num::ctx::Interrupt;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, atomic::AtomicBool},
};

/// Original real accepted producer, source and immutable output; no active math state writer.
pub struct StoredResult {
    receipt: NativeKernelReceipt,
    kernel: Arc<KernelState>,
    value: RetainedResult,
    reserved: usize,
}
impl StoredResult {
    /// Capture only from a matching accepted state/actual receipt, on a dedicated host worker.
    /// Decoding creates an unaccepted work object solely to copy immutable records/readonly context.
    pub fn capture(
        accepted: &AcceptedKernelState,
        limits: CheckpointLimits,
        ctx: &Interrupt,
    ) -> Result<Self, ResultError> {
        let receipt = accepted.receipt().clone();
        let id = receipt
            .producer
            .cell_id
            .0
            .as_deref()
            .ok_or(ResultError::Invalid)?;
        if receipt.result_id.0.is_none() || receipt.checkpoint_blob_hash != accepted.state().hash()
        {
            return Err(ResultError::Invalid);
        }
        let state = accepted.state();
        let work = state
            .restore(
                CheckpointRestore {
                    binding: state.binding(),
                    source: state.source(),
                    general: state.general(),
                    clock: None,
                    cancel: Arc::new(AtomicBool::new(false)),
                },
                limits,
                ctx,
            )
            .map_err(|e| ResultError::Projection(e.to_string()))?;
        let value = work
            .into_session()
            .retain_result(id)
            .map_err(ResultError::Projection)?;
        let summary = value.summary();
        let tail = summary.statements.last().map(|r| u64::from(r.out_index));
        if summary.statements.len() as u64 != receipt.producer.successful_statements.get()
            || tail != receipt.producer.out_index.0.map(|n| n.get())
            || (summary.status == om_kernel::protocol::CellStatus::Done)
                != (receipt.producer.terminal_status == NativeKernelTerminalStatus::Done)
        {
            return Err(ResultError::Invalid);
        }
        // Immutable encoded state plus conservative readonly projection reservation. The caller
        // never expires an externally pinned snapshot's charge just because a lookup was revoked.
        let reserved = state
            .bytes()
            .len()
            .checked_mul(2)
            .and_then(|n| n.checked_add(4096))
            .ok_or(ResultError::Budget)?;
        Ok(Self {
            receipt,
            kernel: Arc::new(state.detached()),
            value,
            reserved,
        })
    }
    /// Original stable storage identity for this whole actual cell result.
    pub fn id(&self) -> &str {
        self.receipt.result_id.0.as_deref().expect("checked result")
    }
    /// Immutable small summary of actual records/status/types.
    pub fn summary(&self) -> RetainedResultSummary {
        self.value.summary()
    }
    /// Actual accepted original producer receipt, not a field guessed from the rendered value.
    pub fn receipt(&self) -> &NativeKernelReceipt {
        &self.receipt
    }
}
/// Rejected/expired output never silently creates another result or replays an old mathematical cell.
#[derive(Debug)]
pub enum ResultError {
    /// Bad identity/producer/snapshot/index/owner.
    Invalid,
    /// Actual retained count/byte/lifetime limit.
    Budget,
    /// A real immutable data/projection failure.
    Projection(String),
    /// Actual reference signature/scope/permission/deadline failure.
    Reference(ReferenceError),
}
impl From<ReferenceError> for ResultError {
    fn from(e: ReferenceError) -> Self {
        Self::Reference(e)
    }
}
impl std::fmt::Display for ResultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => f.write_str("invalid immutable result"),
            Self::Budget => f.write_str("result retention budget exceeded"),
            Self::Projection(e) => f.write_str(e),
            Self::Reference(e) => write!(f, "result reference {e:?}"),
        }
    }
}
impl std::error::Error for ResultError {}
/// Serializable scoped descriptor, never serialized evaluator/kernel pointers.
#[derive(Clone, Serialize)]
struct Occurrence {
    root_id: String,
    cell_id: String,
    out_index: u32,
    view_id: Option<String>,
}
/// An actual pinned result resolution plus explicit current/history/partial provenance.
pub struct ResolvedResult {
    record: Arc<StoredResult>,
    occurrence: Occurrence,
    binding: ResultBinding,
}
impl ResolvedResult {
    /// Scope and producer facts computed from current trusted source, never from value contents.
    pub fn binding(&self) -> &ResultBinding {
        &self.binding
    }
    /// Original cell metadata/record descriptors.
    pub fn summary(&self) -> RetainedResultSummary {
        self.record.summary()
    }
    /// Actual original terminal diagnostic messages.
    pub fn messages(&self) -> Vec<om_kernel::protocol::Message> {
        self.record.value.messages()
    }
}
/// Bounded immutable result registry. Public refs are read-only and bound to current trusted grants.
pub struct ResultStore {
    records: BTreeMap<String, Arc<StoredResult>>,
    retired: Vec<Arc<StoredResult>>,
    references: ReferenceRegistry<Occurrence>,
    bytes: usize,
    max_bytes: usize,
    max_results: usize,
    created: u32,
    current: Option<CurrentFacts>,
}
struct CurrentFacts {
    document: String,
    revision: u64,
    epoch: u64,
    cells: Vec<om_kernel::retained_results::CurrentResultCell>,
}
impl ResultStore {
    /// Small original occurrence metadata with scoped refs, paged independently from value data.
    pub fn manifest(
        &mut self,
        id: &str,
        scope: &ReferenceScope,
        offset: u32,
        limit: u32,
        now: u64,
        ctx: &Interrupt,
    ) -> Result<serde_json::Value, ResultError> {
        if limit == 0 || limit > 32 {
            return Err(ResultError::Budget);
        }
        let record = self.records.get(id).cloned().ok_or(ResultError::Invalid)?;
        if record.receipt.document_id != scope.document || !scope.can_read {
            return Err(ResultError::Reference(ReferenceError::PermissionDenied));
        }
        let summary = record.summary();
        let total = summary.statements.len().max(1);
        if offset as usize > total {
            return Err(ResultError::Invalid);
        }
        let mut entries = Vec::new();
        for index in offset as usize..(offset as usize + limit as usize).min(total) {
            ctx.tick()
                .map_err(|e| ResultError::Projection(e.to_string()))?;
            let statement = summary.statements.get(index);
            let reference = self.issue(
                id,
                statement.map_or(0, |s| s.out_index),
                statement.map(|s| s.view_id.as_str()),
                scope.clone(),
                now,
            )?;
            let resolved = self.resolve(&reference, scope, now)?;
            entries.push(serde_json::json!({"statement":statement,"binding":resolved.binding()}));
        }
        Ok(
            serde_json::json!({"root_result_id":id,"cell_id":summary.cell_id,"cell_kind":summary.cell_kind,"status":summary.status,"source_byte_length":summary.source_byte_length,"statement_count":summary.statements.len(),"message_count":summary.message_count,"total":total,"offset":offset,"entries":entries}),
        )
    }
    /// Host entropy is mandatory outside fixtures. Normal hard limits are 64MiB/32 cached results.
    pub fn new(key: [u8; 32], max_bytes: usize, max_results: usize) -> Result<Self, ResultError> {
        if max_bytes == 0 || max_bytes > 64 * 1024 * 1024 || max_results == 0 || max_results > 32 {
            return Err(ResultError::Budget);
        }
        Ok(Self {
            records: BTreeMap::new(),
            retired: Vec::new(),
            references: ReferenceRegistry::new(key),
            bytes: 0,
            max_bytes,
            max_results,
            created: 0,
            current: None,
        })
    }
    /// Insert an already frozen real result. Expiration/capacity never pretends it was sampled again.
    pub fn register(&mut self, record: StoredResult) -> Result<(), ResultError> {
        self.reap();
        if let Some(previous) = self.records.get(record.id()) {
            if previous.receipt.request_hash != record.receipt.request_hash
                || previous.receipt.checkpoint_blob_hash != record.receipt.checkpoint_blob_hash
            {
                return Err(ResultError::Invalid);
            }
            return Ok(());
        }
        if self.records.len() + self.retired.len() >= self.max_results
            || self.created >= 4096
            || self
                .bytes
                .checked_add(record.reserved)
                .is_none_or(|n| n > self.max_bytes)
        {
            return Err(ResultError::Budget);
        }
        self.bytes += record.reserved;
        self.created += 1;
        self.records.insert(record.id().into(), Arc::new(record));
        Ok(())
    }
    /// Cache the newest real result, evicting only older unpinned cached data when needed. Durable
    /// Blob/receipt facts are retained separately; old refs then report expired, never replay source.
    pub fn register_recent(&mut self, record: StoredResult) -> Result<(), ResultError> {
        if self.records.contains_key(record.id()) {
            return self.register(record);
        }
        if record.reserved > self.max_bytes {
            return Err(ResultError::Budget);
        }
        self.reap();
        while self.records.len() + self.retired.len() >= self.max_results
            || self
                .bytes
                .checked_add(record.reserved)
                .is_none_or(|n| n > self.max_bytes)
        {
            let oldest = self
                .records
                .iter()
                .filter(|(_, record)| Arc::strong_count(record) == 1)
                .min_by_key(|(_, record)| record.receipt.kernel_state_revision.get())
                .map(|(id, _)| id.clone())
                .ok_or(ResultError::Budget)?;
            self.evict(&oldest)?;
        }
        self.register(record)
    }
    /// Refresh current cell/occurrence facts from an actual accepted checkpoint, off control/UI.
    pub fn set_current(
        &mut self,
        accepted: &AcceptedKernelState,
        ctx: &Interrupt,
    ) -> Result<(), ResultError> {
        let state = accepted.state();
        let work = state
            .restore(
                CheckpointRestore {
                    binding: state.binding(),
                    source: state.source(),
                    general: state.general(),
                    clock: None,
                    cancel: Arc::new(AtomicBool::new(false)),
                },
                CheckpointLimits::default(),
                ctx,
            )
            .map_err(|e| ResultError::Projection(e.to_string()))?;
        self.current = Some(CurrentFacts {
            document: state.binding().document_id.clone(),
            revision: state.binding().kernel_state_revision,
            epoch: state.binding().execution_epoch,
            cells: work.into_session().current_result_cells(),
        });
        Ok(())
    }
    /// Sign the exact actual owner/view occurrence. Wrong owner/ordinal is not a new lookup alias.
    pub fn issue(
        &mut self,
        id: &str,
        out_index: u32,
        view_id: Option<&str>,
        scope: ReferenceScope,
        now: u64,
    ) -> Result<String, ResultError> {
        if !scope.can_read {
            return Err(ResultError::Reference(ReferenceError::PermissionDenied));
        }
        let record = self.records.get(id).ok_or(ResultError::Invalid)?;
        if record.receipt.document_id != scope.document {
            return Err(ResultError::Invalid);
        }
        let summary = record.summary();
        let actual = summary
            .statements
            .iter()
            .any(|r| r.out_index == out_index && Some(r.view_id.as_str()) == view_id);
        let error_only = out_index == 0 && view_id.is_none() && summary.statements.is_empty();
        if !(actual || error_only) {
            return Err(ResultError::Invalid);
        }
        Ok(self.references.issue(
            ReferenceKind::Result,
            scope,
            Occurrence {
                root_id: id.into(),
                cell_id: summary.cell_id,
                out_index,
                view_id: view_id.map(str::to_owned),
            },
            now,
            600000,
        )?)
    }
    /// Resolve a current or explicitly historical immutable result under the same actual read grant.
    pub fn resolve(
        &self,
        reference: &str,
        current: &ReferenceScope,
        now: u64,
    ) -> Result<ResolvedResult, ResultError> {
        let occurrence = self
            .references
            .resolve_result_history(reference, current, now)?;
        let record = self
            .records
            .get(&occurrence.root_id)
            .cloned()
            .ok_or(ResultError::Reference(ReferenceError::Expired))?;
        let producer = &record.receipt.producer;
        let live = self
            .current
            .as_ref()
            .filter(|facts| {
                facts.document == current.document
                    && facts.revision == current.definition_revision
                    && facts.epoch == current.execution_epoch
            })
            .and_then(|facts| {
                facts
                    .cells
                    .iter()
                    .find(|cell| cell.cell_id == occurrence.cell_id)
            })
            .is_some_and(|cell| {
                matches!(
                    cell.status,
                    om_kernel::protocol::CellStatus::Done | om_kernel::protocol::CellStatus::Error
                ) && (cell.occurrences.iter().any(|(out, view)| {
                    *out == occurrence.out_index && Some(view) == occurrence.view_id.as_ref()
                }) || (cell.occurrences.is_empty()
                    && occurrence.out_index == 0
                    && occurrence.view_id.is_none()))
            });
        let fresh = producer.execution_epoch.get() == current.execution_epoch
            && producer.config_revision.get() == current.config_revision
            && live;
        let partial = producer.terminal_status == NativeKernelTerminalStatus::Error;
        let binding = ResultBinding {
            record_type: ResultBindingRecordType::ResultBinding,
            result_id: if let Some(view) = &occurrence.view_id {
                format!("{}.{}", occurrence.root_id, view)
            } else {
                occurrence.root_id.clone()
            },
            result_ref: reference.into(),
            runtime_instance_id: current.runtime.clone(),
            document_id: current.document.clone(),
            document_generation: Serial::new(current.generation)
                .map_err(|_| ResultError::Invalid)?,
            cell_id: occurrence.cell_id.clone(),
            operation_id: record.receipt.operation_id.clone(),
            source_hash: producer
                .cell_source_hash
                .0
                .clone()
                .ok_or(ResultError::Invalid)?,
            source_revision: producer.source_revision,
            execution_epoch: producer.execution_epoch,
            kernel_state_revision: producer.kernel_state_revision,
            out_index: Serial::new(occurrence.out_index.into())
                .map_err(|_| ResultError::Invalid)?,
            view_id: Nullable(occurrence.view_id.clone()),
            acceptance: if fresh {
                ResultBindingAcceptance::Accepted
            } else {
                ResultBindingAcceptance::HistoryOnly
            },
            freshness: if !fresh {
                ResultBindingFreshness::Stale
            } else if partial {
                ResultBindingFreshness::Partial
            } else {
                ResultBindingFreshness::Current
            },
            source_fully_available: true,
        };
        Ok(ResolvedResult {
            record,
            occurrence: (*occurrence).clone(),
            binding,
        })
    }
    /// Explicit reference revocation changes no stored mathematical fact or physical Blob.
    pub fn revoke_reference(&mut self, reference: &str) -> Result<(), ResultError> {
        self.references.revoke(reference)?;
        Ok(())
    }
    /// Remove cache lookup; outstanding inspection pins keep their byte/count reservation.
    pub fn evict(&mut self, id: &str) -> Result<(), ResultError> {
        self.retired
            .push(self.records.remove(id).ok_or(ResultError::Invalid)?);
        self.reap();
        Ok(())
    }
    /// Release only genuinely unpinned retired snapshots.
    pub fn reap(&mut self) {
        self.retired.retain(|record| {
            let keep = Arc::strong_count(record) > 1;
            if !keep {
                self.bytes -= record.reserved;
            }
            keep
        });
    }
    /// Actual conservative cache reservation, including retired in-flight snapshots.
    pub fn reserved_bytes(&self) -> usize {
        self.bytes
    }
}
