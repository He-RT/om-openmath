//! Frozen mathematical acceptance plans. No disk IO or active pointer is changed optimistically.
mod contract;
use super::{
    KernelState,
    worker::{KernelCandidate, KernelWorker, KernelWorkerError, Lifecycle},
};
use crate::{
    document::{
        coordinator::{calculation_values, kernel_file},
        validate_snapshot,
    },
    protocol::{Nullable, Serial, generated::*},
};
pub use contract::{
    decode_commit, decode_receipt, request_hash, validate_commit, validate_receipt,
};
use om_kernel::{
    KernelConfig, Session,
    checkpoint::{CheckpointBinding, CheckpointLimits},
};
use om_num::ctx::Interrupt;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Current trusted coordinator facts. Passing DTOs does not grant model/file write authority.
#[derive(Clone)]
pub struct AcceptanceContext {
    /// Actual runtime which owns the currently open document.
    pub runtime_instance_id: String,
    /// Actual verified store identity.
    pub store_id: String,
    /// Current open lifetime.
    pub document_generation: Serial,
    /// Current confirmed source head; draft barriers happen before a job is admitted.
    pub source: NativeSourceSnapshot,
    /// Current trusted display/calculation settings, no AI credentials.
    pub general: om_kernel::config::GeneralConfig,
    /// Current mathematical configuration revision.
    pub config_revision: Serial,
    /// Exact producer build identity.
    pub build: String,
}
/// Actual verified persistence inputs; callers obtain them from a trusted physical readback.
pub struct KernelRecovery<'a> {
    /// Original frozen stored contract.
    pub plan: &'a NativeKernelCommit,
    /// Original read-back acceptance receipt.
    pub receipt: &'a NativeKernelReceipt,
    /// Actual immutable checkpoint Blob bytes.
    pub bytes: Vec<u8>,
    /// Actual original full configuration, without credentials.
    pub general: om_kernel::config::GeneralConfig,
    /// Exact currently supported build; never inferred from an untrusted record.
    pub expected_build: &'a str,
}
/// A live registry state paired with its actual matching durable receipt. Constructor is private.
#[derive(Clone)]
pub struct AcceptedKernelState {
    registry_ref: String,
    state: Arc<KernelState>,
    receipt: NativeKernelReceipt,
}
impl AcceptedKernelState {
    /// Trusted recovery of a genuinely read-back accepted record and its original checked Blob.
    /// No source is replayed and no historical runtime/token becomes a current operation grant.
    pub fn recover(
        input: KernelRecovery<'_>,
        worker: &KernelWorker,
        limits: CheckpointLimits,
        ctx: &Interrupt,
    ) -> Result<Self, KernelWorkerError> {
        let KernelRecovery {
            plan,
            receipt,
            bytes,
            general,
            expected_build,
        } = input;
        validate_receipt(plan, receipt).map_err(|_| KernelWorkerError::Invalid)?;
        if bytes.len() as u64 != receipt.checkpoint_byte_length.get()
            || receipt.producer.kernel_build != expected_build
            || format!("{:x}", Sha256::digest(&bytes)) != receipt.checkpoint_blob_hash
            || general_hash(&general)? != receipt.producer.general_hash
        {
            return Err(KernelWorkerError::Invalid);
        }
        let p = &receipt.producer;
        let binding = CheckpointBinding {
            document_id: p.document_id.clone(),
            document_generation: p.document_generation.get(),
            source_revision: p.source_revision.get(),
            execution_epoch: p.execution_epoch.get(),
            kernel_state_revision: p.kernel_state_revision.get(),
            source_snapshot_hash: p.source_snapshot_hash.clone(),
            build: p.kernel_build.clone(),
        };
        let state = KernelState::import(
            bytes,
            binding,
            kernel_file(&plan.source.file),
            general,
            limits,
            ctx,
        )?;
        let registry_ref = worker.register(state)?;
        let state = worker.state(&registry_ref)?;
        Ok(Self {
            registry_ref,
            state,
            receipt: receipt.clone(),
        })
    }
    /// Live ref explicitly selected by the coordinator for the next job.
    pub fn registry_ref(&self) -> &str {
        &self.registry_ref
    }
    /// Original actual encoded bytes, pinned against registry eviction.
    pub fn state(&self) -> &Arc<KernelState> {
        &self.state
    }
    /// Actual trusted durable original-ID receipt, independent from a UI acceptance animation.
    pub fn receipt(&self) -> &NativeKernelReceipt {
        &self.receipt
    }
}
/// Owns a frozen actual state and immutable plan until durable receipt or explicit safe discard.
pub struct FrozenKernelPlan {
    plan: NativeKernelCommit,
    registry_ref: String,
    state: Arc<KernelState>,
    lifecycle: Arc<Lifecycle>,
}
struct FrozenInput {
    state: Arc<KernelState>,
    registry_ref: String,
    lifecycle: Arc<Lifecycle>,
    source: NativeSourceSnapshot,
    operation: String,
    kind: NativeKernelCommitKind,
    cell: Option<String>,
    outcome: Option<(NativeKernelTerminalStatus, Serial, Option<Serial>)>,
}
fn entropy(prefix: &str) -> Result<String, KernelWorkerError> {
    let mut raw = [0u8; 16];
    getrandom::fill(&mut raw).map_err(|_| KernelWorkerError::Internal)?;
    Ok(format!(
        "{prefix}-{}",
        raw.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
fn general_hash(value: &om_kernel::config::GeneralConfig) -> Result<String, KernelWorkerError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(|_| KernelWorkerError::Invalid)?)
    ))
}
impl FrozenKernelPlan {
    /// Build a genuine empty mathematical owner from verified source, never replay its cells.
    /// Bootstrap still needs real Blob publication, final barrier and SQLite receipt before use.
    pub fn bootstrap(
        context: &AcceptanceContext,
        operation: &str,
        worker: &KernelWorker,
        limits: CheckpointLimits,
        ctx: &Interrupt,
    ) -> Result<Self, KernelWorkerError> {
        validate_snapshot(&context.source).map_err(|_| KernelWorkerError::Invalid)?;
        let config = KernelConfig {
            general: context.general.clone(),
            ..Default::default()
        };
        let mut session = Session::new(config, None);
        session.handle(om_kernel::protocol::Request::SetHostPlatform {
            platform: om_kernel::protocol::HostPlatform::Desktop,
        });
        session
            .apply_source_file_without_evaluation(kernel_file(&context.source.file))
            .map_err(KernelWorkerError::Kernel)?;
        let binding = CheckpointBinding {
            document_id: context.source.document_id.clone(),
            document_generation: context.document_generation.get(),
            source_revision: context.source.revision.get(),
            execution_epoch: context.source.execution_epoch.get(),
            kernel_state_revision: 0,
            source_snapshot_hash: context.source.snapshot_hash.clone(),
            build: context.build.clone(),
        };
        let state = KernelState::capture(&mut session, binding, limits, ctx)?;
        let reference = worker.register(state)?;
        let state = worker.state(&reference)?;
        let lifecycle = Arc::new(Lifecycle::new(ctx.flag.clone()));
        lifecycle.prepared()?;
        let result = Self::prepare(
            FrozenInput {
                state,
                registry_ref: reference.clone(),
                lifecycle,
                source: context.source.clone(),
                operation: operation.into(),
                kind: NativeKernelCommitKind::Bootstrap,
                cell: None,
                outcome: None,
            },
            context,
            None,
        );
        if result.is_err() {
            let _ = worker.discard(&reference);
        }
        result
    }
    /// Freeze one real computed candidate against the actually accepted parent selected earlier.
    pub fn candidate(
        candidate: KernelCandidate,
        parent: &AcceptedKernelState,
        context: &AcceptanceContext,
        worker: &KernelWorker,
    ) -> Result<Self, KernelWorkerError> {
        let reference = candidate.checkpoint_ref.clone();
        let result = (|| {
            let facts = candidate.provenance();
            if facts.parent_checkpoint_ref != parent.registry_ref
                || facts.runtime_instance_id != context.runtime_instance_id
                || facts.document_generation != context.document_generation
                || facts.source.document_id != parent.receipt.document_id
                || facts.config_revision != context.config_revision
                || facts.general_hash != general_hash(candidate.state().general())?
                || candidate.state().binding().kernel_state_revision
                    != parent
                        .receipt
                        .kernel_state_revision
                        .checked_next()
                        .map_err(|_| KernelWorkerError::Limit)?
                        .get()
            {
                return Err(KernelWorkerError::Invalid);
            }
            let status = match candidate.boundary().status {
                om_kernel::protocol::CellStatus::Done => NativeKernelTerminalStatus::Done,
                om_kernel::protocol::CellStatus::Error => NativeKernelTerminalStatus::Error,
                _ => return Err(KernelWorkerError::Invalid),
            };
            let operation = facts.operation_id.clone();
            let source = facts.source.clone();
            let cell = candidate.boundary().cell_id.clone();
            let outcome = (
                status,
                Serial::new(candidate.boundary().successful_statements as u64)
                    .map_err(|_| KernelWorkerError::Limit)?,
                candidate
                    .boundary()
                    .out_index
                    .map(|n| Serial::new(n.into()))
                    .transpose()
                    .map_err(|_| KernelWorkerError::Limit)?,
            );
            Self::prepare(
                FrozenInput {
                    state: candidate.state,
                    registry_ref: candidate.checkpoint_ref,
                    lifecycle: candidate.lifecycle,
                    source,
                    operation,
                    kind: NativeKernelCommitKind::ExecuteCell,
                    cell: Some(cell),
                    outcome: Some(outcome),
                },
                context,
                Some(parent),
            )
        })();
        if result.is_err() {
            let _ = worker.discard(&reference);
        }
        result
    }
    fn prepare(
        input: FrozenInput,
        context: &AcceptanceContext,
        parent: Option<&AcceptedKernelState>,
    ) -> Result<Self, KernelWorkerError> {
        let FrozenInput {
            state,
            registry_ref,
            lifecycle,
            operation,
            kind,
            source,
            cell,
            outcome,
        } = input;
        validate_snapshot(&source).map_err(|_| KernelWorkerError::Invalid)?;
        let b = state.binding();
        if context.document_generation.get() == 0
            || b.document_generation != context.document_generation.get()
            || b.document_id != context.source.document_id
            || b.build != context.build
            || b.execution_epoch != context.source.execution_epoch.get()
            || source.snapshot_hash != b.source_snapshot_hash
            || source.revision.get() != b.source_revision
            || serde_json::to_value(kernel_file(&source.file))
                .map_err(|_| KernelWorkerError::Invalid)?
                != serde_json::to_value(state.source()).map_err(|_| KernelWorkerError::Invalid)?
            || om_kernel::source::math_source_changed(
                state.source(),
                &kernel_file(&context.source.file),
            )
            || om_kernel::source::calculation_settings_changed(state.general(), &context.general)
        {
            return Err(KernelWorkerError::Invalid);
        }
        let cell_hash = cell
            .as_ref()
            .map(|id| {
                state
                    .source()
                    .cells
                    .iter()
                    .find(|c| c.id == *id)
                    .map(|c| format!("{:x}", Sha256::digest(c.source.as_bytes())))
                    .ok_or(KernelWorkerError::Invalid)
            })
            .transpose()?;
        let (terminal, success, out) = outcome.unwrap_or((
            NativeKernelTerminalStatus::Unexecuted,
            Serial::new(0).expect("zero"),
            None,
        ));
        let mut plan = NativeKernelCommit {
            protocol_version: 1,
            kind: kind.clone(),
            store_id: context.store_id.clone(),
            runtime_instance_id: context.runtime_instance_id.clone(),
            operation_id: operation,
            request_hash: String::new(),
            checkpoint_id: entropy("checkpoint")?,
            expected_parent_checkpoint_id: Nullable(
                parent.map(|p| p.receipt.checkpoint_id.clone()),
            ),
            expected_kernel_state_revision: parent.map_or(Serial::new(0).expect("zero"), |p| {
                p.receipt.kernel_state_revision
            }),
            checkpoint_blob_hash: state.hash().into(),
            checkpoint_byte_length: Serial::new(state.bytes().len() as u64)
                .map_err(|_| KernelWorkerError::Limit)?,
            codec_version: 1,
            producer: NativeKernelProducer {
                document_id: b.document_id.clone(),
                document_generation: context.document_generation,
                source_revision: Serial::new(b.source_revision)
                    .map_err(|_| KernelWorkerError::Invalid)?,
                execution_epoch: Serial::new(b.execution_epoch)
                    .map_err(|_| KernelWorkerError::Invalid)?,
                kernel_state_revision: Serial::new(b.kernel_state_revision)
                    .map_err(|_| KernelWorkerError::Invalid)?,
                source_snapshot_hash: b.source_snapshot_hash.clone(),
                kernel_build: b.build.clone(),
                config_revision: context.config_revision,
                calculation: calculation_values(state.general())
                    .map_err(|_| KernelWorkerError::Invalid)?,
                general_hash: general_hash(state.general())?,
                cell_id: Nullable(cell),
                cell_source_hash: Nullable(cell_hash),
                terminal_status: terminal,
                successful_statements: success,
                out_index: Nullable(out),
            },
            source,
            acceptance_source: context.source.clone(),
            result_id: Nullable(
                (kind == NativeKernelCommitKind::ExecuteCell)
                    .then(|| entropy("result"))
                    .transpose()?,
            ),
            outbox_event_id: entropy("kernel-event")?,
        };
        plan.request_hash = request_hash(&plan);
        validate_commit(&plan).map_err(|_| KernelWorkerError::Invalid)?;
        Ok(Self {
            plan,
            registry_ref,
            state,
            lifecycle,
        })
    }
    /// Frozen original contract for trusted physical IO. Neither UI nor Pi mutates it.
    pub fn plan(&self) -> &NativeKernelCommit {
        &self.plan
    }
    /// Actual producer bytes to publish; Blob hash/length in the plan must match these bytes.
    pub fn state(&self) -> &Arc<KernelState> {
        &self.state
    }
    /// Final short owner barrier, called immediately before physical SQLite COMMIT.
    pub fn enter_commit(
        &self,
        current: &AcceptanceContext,
        parent: Option<&AcceptedKernelState>,
    ) -> Result<(), KernelWorkerError> {
        validate_snapshot(&current.source).map_err(|_| KernelWorkerError::Invalid)?;
        if current.runtime_instance_id != self.plan.runtime_instance_id
            || current.store_id != self.plan.store_id
            || current.document_generation != self.plan.producer.document_generation
            || current.source.snapshot_hash != self.plan.acceptance_source.snapshot_hash
            || current.config_revision != self.plan.producer.config_revision
            || current.build != self.plan.producer.kernel_build
            || om_kernel::source::calculation_settings_changed(
                &current.general,
                self.state.general(),
            )
            || parent.map(|p| p.receipt.checkpoint_id.as_str())
                != self.plan.expected_parent_checkpoint_id.0.as_deref()
            || parent.map_or(0, |p| p.receipt.kernel_state_revision.get())
                != self.plan.expected_kernel_state_revision.get()
        {
            return Err(KernelWorkerError::Invalid);
        }
        self.lifecycle.enter()
    }
    /// Direct stop shares the final barrier lock; late stop cannot erase a committed candidate.
    pub fn cancel(&self) -> Result<bool, KernelWorkerError> {
        self.lifecycle.cancel()
    }
    /// Unknown COMMIT acknowledgement keeps all original plan/state identities for readback.
    pub fn mark_unknown(&self) -> Result<(), KernelWorkerError> {
        self.lifecycle.unknown()
    }
    /// Only a matching trusted durable physical receipt yields a next-job accepted parent.
    pub fn accept_receipt(
        &self,
        receipt: &NativeKernelReceipt,
    ) -> Result<AcceptedKernelState, KernelWorkerError> {
        validate_receipt(&self.plan, receipt).map_err(|_| KernelWorkerError::Invalid)?;
        self.lifecycle.accept()?;
        Ok(AcceptedKernelState {
            registry_ref: self.registry_ref.clone(),
            state: self.state.clone(),
            receipt: receipt.clone(),
        })
    }
    /// Safe before a commit barrier. An unknown/committing state cannot be discarded as rollback.
    pub fn discard(&self, worker: &KernelWorker) -> Result<(), KernelWorkerError> {
        self.lifecycle.discard()?;
        worker.discard(&self.registry_ref)
    }
    /// Trusted same-writer readback has settled actual IO and proved the original plan absent.
    /// Never infer this from a timeout, a lost network reply, or a different operation ID.
    pub fn confirmed_no_commit(
        &self,
        proof: &NativeKernelNoCommit,
        worker: &KernelWorker,
    ) -> Result<(), KernelWorkerError> {
        let p = &self.plan;
        if proof.protocol_version != 1
            || proof.store_id != p.store_id
            || proof.document_id != p.producer.document_id
            || proof.operation_id != p.operation_id
            || proof.request_hash != p.request_hash
            || proof.accepted_source_revision != p.acceptance_source.revision
            || proof.accepted_snapshot_hash != p.acceptance_source.snapshot_hash
            || proof.expected_parent_checkpoint_id != p.expected_parent_checkpoint_id
            || proof.expected_kernel_state_revision != p.expected_kernel_state_revision
        {
            return Err(KernelWorkerError::Invalid);
        }
        self.lifecycle.confirmed_no_commit()?;
        worker.discard(&self.registry_ref)
    }
}
