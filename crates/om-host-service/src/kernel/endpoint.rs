//! Trusted native document/kernel port. Source controller is the only shared logical write gate.
mod commands;
use super::{
    acceptance::{AcceptanceContext, AcceptedKernelState, FrozenKernelPlan},
    worker::{KernelCandidate, KernelJob, KernelWorker, KernelWorkerError},
};
use crate::{
    document::commit::DocumentCommitController,
    protocol::{Nullable, Serial, generated::*, validation},
};
use om_kernel::checkpoint::CheckpointLimits;
use om_num::ctx::Interrupt;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};

/// Actual compiled Rust/schema/catalog/target content; does not depend on a display release string.
pub const KERNEL_BUILD_ID: &str = env!("OPENMATH_NATIVE_KERNEL_BUILD_ID");
enum Receiver {
    Bootstrap(mpsc::Receiver<Result<FrozenKernelPlan, KernelWorkerError>>),
    Cell(mpsc::Receiver<Result<KernelCandidate, KernelWorkerError>>),
}
enum Identity {
    Bootstrap { source: String },
    Cell { source: String, cell: String },
}
struct Entry {
    identity: Identity,
    phase: NativeKernelHostReplyPhase,
    receiver: Option<Receiver>,
    candidate: Option<KernelCandidate>,
    plan: Option<FrozenKernelPlan>,
    receipt: Option<NativeKernelReceipt>,
    error: Option<String>,
    cancel: Arc<AtomicBool>,
    seed_started: Arc<AtomicBool>,
}
/// Native single-document math coordinator, never a second source owner or physical writer.
pub struct KernelEndpoint {
    worker: Arc<KernelWorker>,
    active: Option<AcceptedKernelState>,
    operations: BTreeMap<String, Entry>,
    seed_threads: Vec<JoinHandle<()>>,
    closing: bool,
    math_join: Option<JoinHandle<()>>,
}
fn terminal(phase: &NativeKernelHostReplyPhase) -> bool {
    matches!(
        phase,
        NativeKernelHostReplyPhase::Completed
            | NativeKernelHostReplyPhase::Discarded
            | NativeKernelHostReplyPhase::Failed
    )
}
fn error(error: KernelWorkerError) -> String {
    match error {
        KernelWorkerError::Cancelled => "CANCELLED".into(),
        KernelWorkerError::Closing => "HOST_CLOSING".into(),
        KernelWorkerError::QueueFull | KernelWorkerError::Limit => "BUDGET_EXCEEDED".into(),
        KernelWorkerError::DuplicateOperation => "DUPLICATE_OPERATION".into(),
        KernelWorkerError::Kernel(code) => code,
        KernelWorkerError::Invalid => "STALE_SOURCE".into(),
        KernelWorkerError::State(super::KernelStateError::Limit) => "BUDGET_EXCEEDED".into(),
        _ => "INTERNAL_ERROR".into(),
    }
}
impl KernelEndpoint {
    /// Known main operation identity; a readonly request cannot shadow its direct cancel token.
    pub fn has_operation(&self, id: &str) -> bool {
        self.operations.contains_key(id)
    }
    /// Create bounded worker ownership without evaluating any source or doing physical IO.
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            worker: Arc::new(
                KernelWorker::new(64 * 1024 * 1024, 32, CheckpointLimits::default())
                    .map_err(error)?,
            ),
            active: None,
            operations: BTreeMap::new(),
            seed_threads: Vec::new(),
            closing: false,
            math_join: None,
        })
    }
    fn busy(&self) -> bool {
        self.operations
            .values()
            .any(|entry| !terminal(&entry.phase))
    }
    fn base_reply(
        &self,
        ctx: &AcceptanceContext,
        source: &DocumentCommitController,
    ) -> NativeKernelHostReply {
        let active = self.active.as_ref();
        NativeKernelHostReply {
            protocol_version: 1,
            phase: NativeKernelHostReplyPhase::State,
            operation_id: Nullable(None),
            document_id: ctx.source.document_id.clone(),
            document_generation: ctx.document_generation,
            source_revision: ctx.source.revision,
            execution_epoch: ctx.source.execution_epoch,
            kernel_build: KERNEL_BUILD_ID.into(),
            active_checkpoint_id: Nullable(active.map(|a| a.receipt().checkpoint_id.clone())),
            kernel_state_revision: active.map_or(Serial::new(0).expect("zero"), |a| {
                a.receipt().kernel_state_revision
            }),
            kernel_projection_revision: Nullable(
                active.map(|a| a.receipt().producer.source_revision),
            ),
            definitions_current: active.is_some_and(|a| {
                a.state().binding().execution_epoch == ctx.source.execution_epoch.get()
            }),
            pending_document_write: source.source_gate_busy() || source.kernel_gate().is_some(),
            plan: Nullable(None),
            receipt: Nullable(None),
            error_code: Nullable(None),
            blob_offset: Nullable(None),
            blob_bytes: Vec::new(),
        }
    }
    fn status(
        &self,
        id: &str,
        ctx: &AcceptanceContext,
        source: &DocumentCommitController,
    ) -> Result<NativeKernelHostReply, String> {
        let entry = self.operations.get(id).ok_or("INVALID_REFERENCE")?;
        let mut reply = self.base_reply(ctx, source);
        reply.operation_id = Nullable(Some(id.into()));
        reply.phase = entry.phase.clone();
        if entry.phase == NativeKernelHostReplyPhase::Queued
            && (entry.seed_started.load(Ordering::Acquire) || self.worker.started(id))
        {
            reply.phase = NativeKernelHostReplyPhase::Running;
        }
        reply.plan = Nullable(entry.plan.as_ref().map(|p| p.plan().clone()));
        reply.receipt = Nullable(entry.receipt.clone());
        reply.error_code = Nullable(entry.error.clone());
        Ok(reply)
    }
    fn advance(
        &mut self,
        ctx: &AcceptanceContext,
        source: &mut DocumentCommitController,
    ) -> Result<(), String> {
        for (id, entry) in &mut self.operations {
            let received = match entry.receiver.as_ref() {
                Some(Receiver::Bootstrap(rx)) => match rx.try_recv() {
                    Ok(value) => Some(value.map(Some)),
                    Err(mpsc::TryRecvError::Empty) => None,
                    Err(_) => Some(Err(KernelWorkerError::Internal)),
                },
                Some(Receiver::Cell(rx)) => match rx.try_recv() {
                    Ok(Ok(candidate)) => {
                        entry.candidate = Some(candidate);
                        Some(Ok(None))
                    }
                    Ok(Err(e)) => Some(Err(e)),
                    Err(mpsc::TryRecvError::Empty) => None,
                    Err(_) => Some(Err(KernelWorkerError::Internal)),
                },
                None => None,
            };
            if let Some(received) = received {
                entry.receiver = None;
                match received {
                    Ok(Some(plan)) => {
                        entry.plan = Some(plan);
                        entry.phase = NativeKernelHostReplyPhase::Candidate;
                    }
                    Ok(None) => entry.phase = NativeKernelHostReplyPhase::WaitingSource,
                    Err(e) => {
                        entry.error = Some(error(e));
                        entry.phase = NativeKernelHostReplyPhase::Failed;
                    }
                }
            }
            if entry.phase == NativeKernelHostReplyPhase::WaitingSource
                && !source.source_gate_busy()
            {
                let candidate = entry.candidate.take().ok_or("INTERNAL_ERROR")?;
                let active = self.active.as_ref().ok_or("CONTEXT_NOT_READY")?;
                if entry.cancel.load(Ordering::Acquire) {
                    let _ = self.worker.discard(candidate.checkpoint_ref());
                    entry.phase = NativeKernelHostReplyPhase::Discarded;
                    entry.error = Some("CANCELLED".into());
                } else {
                    match FrozenKernelPlan::candidate(candidate, active, ctx, &self.worker) {
                        Ok(plan) => {
                            if source.hold_kernel_gate(id).is_err() {
                                plan.discard(&self.worker).map_err(error)?;
                                entry.phase = NativeKernelHostReplyPhase::Failed;
                                entry.error = Some("OPERATION_IN_PROGRESS".into());
                            } else {
                                entry.plan = Some(plan);
                                entry.phase = NativeKernelHostReplyPhase::Candidate;
                            }
                        }
                        Err(e) => {
                            entry.phase = NativeKernelHostReplyPhase::Failed;
                            entry.error = Some(error(e));
                        }
                    }
                }
            }
            if terminal(&entry.phase) && source.kernel_gate() == Some(id.as_str()) {
                source
                    .release_kernel_gate(id)
                    .map_err(|_| "INTERNAL_ERROR")?;
            }
        }
        // Finished seed handles are joined only after is_finished; no control thread waits on CAS.
        let mut i = 0;
        while i < self.seed_threads.len() {
            if self.seed_threads[i].is_finished() {
                let _ = self.seed_threads.swap_remove(i).join();
            } else {
                i += 1;
            }
        }
        Ok(())
    }
    /// Direct stop signals actual operation tokens and never queues behind mathematical work.
    pub fn cancel(&mut self, id: &str) -> Option<bool> {
        let entry = self.operations.get_mut(id)?;
        if terminal(&entry.phase) {
            return Some(false);
        }
        if let Some(plan) = &entry.plan {
            Some(plan.cancel().unwrap_or(false))
        } else {
            entry.cancel.store(true, Ordering::Release);
            let _ = self.worker.cancel(id);
            Some(true)
        }
    }
    /// Actual accepted context for parser/write ownership. Old epochs are not current definitions.
    pub fn accepted_context(&self) -> Option<&om_kernel::source::SourceExecutionContext> {
        self.active.as_ref().map(|a| a.state().execution_context())
    }
    /// Immutable genuinely accepted state for background result retention; no source/math writer.
    pub fn accepted_state(&self) -> Option<AcceptedKernelState> {
        self.active.clone()
    }
    /// Original terminal fact, so repeated receipt reconciliation does not publish a second event.
    pub fn operation_completed(&self, operation: &str) -> bool {
        self.operations
            .get(operation)
            .is_some_and(|entry| entry.phase == NativeKernelHostReplyPhase::Completed)
    }
    /// Revoke admission and stop work now. Caller joins extracted threads on a background queue.
    pub fn begin_close(&mut self) {
        self.closing = true;
        for entry in self.operations.values() {
            if let Some(plan) = &entry.plan {
                let _ = plan.cancel();
            } else {
                entry.cancel.store(true, Ordering::Release);
            }
        }
        if self.math_join.is_none() {
            self.math_join = self.worker.begin_close().ok().flatten();
        }
    }
    /// No blocking join while a source/control lock is held.
    pub fn take_join_handles(&mut self) -> Vec<JoinHandle<()>> {
        self.begin_close();
        let mut handles = std::mem::take(&mut self.seed_threads);
        if let Some(handle) = self.math_join.take() {
            handles.push(handle);
        }
        handles
    }
}
/// Closed trusted host union; this API does not register model-facing tools or writable grants.
pub fn decode_command(bytes: &[u8]) -> Result<NativeKernelHostCommand, String> {
    static ROOT: OnceLock<serde_json::Value> = OnceLock::new();
    let root = ROOT.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/design/native-kernel-store.schema.json"
        ))
        .expect("checked kernel schema")
    });
    validation::decode(bytes, &root["$defs"]["NativeKernelHostCommand"], root)
}
