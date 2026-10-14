//! Trusted host source bridge. Physical IO stays in Swift; controller methods are short owner steps.
mod patch;
use super::{SourceDocument, commit::DocumentCommitController, coordinator::SourceCoordinator};
use crate::{
    protocol::{Nullable, Serial, generated::*, validation},
    references::ReferenceScope,
};
use sha2::{Digest, Sha256};
use std::{sync::OnceLock, time::Instant};
struct PendingRead {
    scope: ReferenceScope,
    kind: PendingReadKind,
    issued: Instant,
}
enum PendingReadKind {
    Outputs(String),
    Inspect,
}

/// One source owner for the native runtime, separate from CAS/editor/network queues.
pub struct SourceEndpoint {
    controller: DocumentCommitController,
    scope: ReferenceScope,
    store: String,
    clock: Instant,
    coordinator: SourceCoordinator,
    general: om_kernel::config::GeneralConfig,
    kernel: Option<crate::kernel::endpoint::KernelEndpoint>,
    results: Option<crate::results::runtime::ResultRuntime>,
    agent: Option<crate::agent::endpoint::AgentEndpoint>,
    reads: Option<crate::tools::read::ReadNotebookService>,
    pending_reads: std::collections::BTreeMap<String, PendingRead>,
    patches: std::collections::BTreeMap<String, patch::PendingPatch>,
    pending_result_state: Option<crate::kernel::acceptance::AcceptedKernelState>,
}
impl SourceEndpoint {
    /// Native operation namespaces share cancellation; admission cannot shadow an existing ID.
    pub fn has_native_operation(&self, id: &str) -> bool {
        self.controller.operation(id).is_some()
            || self.kernel.as_ref().is_some_and(|k| k.has_operation(id))
            || self.results.as_ref().is_some_and(|r| r.has_request(id))
    }
    /// Restore an actually read source head; host generates a fresh reference key, never the model.
    pub fn open(
        snapshot: NativeSourceSnapshot,
        store: String,
        runtime: String,
        generation: Serial,
        calculation: Option<NativeCalculationSettings>,
        config_revision: Serial,
    ) -> Result<Self, String> {
        if store.is_empty() || store.len() > 256 {
            return Err("INVALID_ARGUMENT".into());
        }
        let owner = SourceDocument::restore(snapshot, generation)?;
        let mut key = [0; 32];
        getrandom::fill(&mut key).map_err(|_| "ENTROPY_UNAVAILABLE")?;
        let s = owner.snapshot();
        let scope = ReferenceScope {
            runtime: runtime.clone(),
            document: s.document_id.clone(),
            generation: generation.get(),
            revision: s.revision.get(),
            execution_epoch: s.execution_epoch.get(),
            snapshot_hash: s.snapshot_hash.clone(),
            task: None,
            task_generation: 0,
            grant_revision: 1,
            config_revision: config_revision.get(),
            definition_revision: 0,
            metadata_revision: u64::from(
                om_kernel::capabilities::function_catalog().metadata_version,
            ),
            editor_state_hash: String::new(),
            execution_mode: true,
            can_read: true,
            can_preview: true,
            can_write: true,
        };
        let mut coordinator = SourceCoordinator::default();
        let general = general_configuration(calculation.as_ref());
        coordinator.dialect = general.dialect;
        coordinator.constants = general.constants;
        Ok(Self {
            controller: DocumentCommitController::new(owner, runtime, key),
            scope,
            store,
            clock: Instant::now(),
            coordinator,
            general,
            kernel: None,
            results: None,
            pending_result_state: None,
            agent: None,
            reads: None,
            pending_reads: std::collections::BTreeMap::new(),
            patches: std::collections::BTreeMap::new(),
        })
    }
    /// Actual runtime/document binding for host readback and stale request checks.
    pub fn binding(&self) -> DocumentBinding {
        DocumentBinding {
            document_id: self.scope.document.clone(),
            generation: Serial::new(self.scope.generation).expect("checked scope"),
            document_revision: self.controller.owner().snapshot().revision,
            execution_epoch: self.controller.owner().snapshot().execution_epoch,
        }
    }
    /// Private Agent activation derives document/runtime from this exact owner.
    pub fn agent_command(
        &mut self,
        command: NativeAgentHostCommand,
    ) -> Result<NativeAgentHostReply, String> {
        let binding = self.binding();
        if let NativeAgentHostCommand::NativeAgentActivate(c) = command {
            if self
                .agent
                .as_ref()
                .is_some_and(|agent| !agent.can_replace(&c.scope))
            {
                return Err("AGENT_TASK_ACTIVE".into());
            }
            let (endpoint, reply) = crate::agent::endpoint::AgentEndpoint::activate(
                c,
                self.scope.runtime.clone(),
                binding,
            )?;
            let mut read_key = [0; 32];
            getrandom::fill(&mut read_key).map_err(|_| "ENTROPY_UNAVAILABLE")?;
            self.reads = Some(crate::tools::read::ReadNotebookService::new(read_key));
            self.pending_reads.clear();
            self.patches.clear();
            self.agent = Some(endpoint);
            return Ok(reply);
        }
        let mut current = self.scope.clone();
        let owner = self.controller.owner().snapshot();
        current.revision = owner.revision.get();
        current.execution_epoch = owner.execution_epoch.get();
        current.snapshot_hash = owner.snapshot_hash.clone();
        let current = self
            .agent
            .as_ref()
            .ok_or("AGENT_NOT_ACTIVE")?
            .readonly_scope(current);
        if let NativeAgentHostCommand::NativeAgentEffectTool(c) = command {
            self.agent
                .as_ref()
                .expect("checked")
                .check_native_scope(&c.scope, &binding)?;
            return self.agent_patch_result(&c.operation_id, c.failure_code.0, current);
        }
        if let NativeAgentHostCommand::NativeAgentBindHistory(c) = command {
            self.agent
                .as_ref()
                .expect("checked")
                .check_native_scope(&c.scope, &binding)?;
            if c.expected_snapshot_hash != current.snapshot_hash {
                return Err("STALE_DOCUMENT".into());
            }
            self.reads
                .as_mut()
                .ok_or("AGENT_NOT_ACTIVE")?
                .bind_history(&current, c.transaction_refs)
                .map_err(|_| "INVALID_HISTORY")?;
            return self.agent.as_mut().expect("checked").command(
                NativeAgentHostCommand::NativeAgentStatus(NativeAgentStatus {
                    r#type: NativeAgentStatusType::AgentStatus,
                    scope: c.scope,
                }),
                &binding,
                &current,
            );
        }
        if let NativeAgentHostCommand::NativeAgentReadTool(c) = command {
            let agent = self.agent.as_ref().expect("checked");
            let (intent, args, cached) = agent.readonly_input(&c, &binding, &current)?;
            if let Some(result) = cached {
                return Ok(agent.readonly_reply(result));
            }
            if let Some(pending) = self.pending_reads.get(&intent.operation_id) {
                if pending.scope != current || pending.issued.elapsed().as_secs() >= 10 {
                    if let Some(results) = &self.results {
                        let _ = results.cancel(&intent.operation_id);
                    }
                    self.pending_reads.remove(&intent.operation_id);
                    return Err("STALE_SNAPSHOT".into());
                }
                let reply = self
                    .results
                    .as_ref()
                    .ok_or("RESULT_NOT_AVAILABLE")?
                    .command(
                        NativeResultHostCommand::ResultStatus(ResultStatus {
                            r#type: ResultStatusType::ResultStatus,
                            request_id: intent.operation_id.clone(),
                        }),
                        &current,
                    )?;
                if matches!(
                    reply.phase,
                    NativeResultHostReplyPhase::Queued | NativeResultHostReplyPhase::Running
                ) {
                    return Ok(self.agent.as_ref().expect("checked").pending_readonly());
                }
                let pending = self
                    .pending_reads
                    .remove(&intent.operation_id)
                    .expect("checked");
                if let PendingReadKind::Inspect = pending.kind {
                    let envelope = if reply.phase == NativeResultHostReplyPhase::Completed {
                        reply.payload
                    } else {
                        crate::tools::results::failure(
                            &intent.operation_id,
                            &current,
                            reply.error_code.0.as_deref().unwrap_or("NOT_AVAILABLE"),
                        )
                    };
                    let envelope = crate::tools::checked_data("inspect_result_result", envelope)
                        .map_err(|_| "TOOL_RESULT_LIMIT")?;
                    let success = envelope["ok"].as_bool().ok_or("INTERNAL_ERROR")?;
                    return self.agent.as_mut().expect("checked").register_readonly(
                        &intent,
                        &current,
                        serde_json::to_string(&envelope).map_err(|_| "INTERNAL_ERROR")?,
                        success,
                    );
                }
                let PendingReadKind::Outputs(json) = pending.kind else {
                    unreachable!()
                };
                let mut envelope: serde_json::Value =
                    serde_json::from_str(&json).map_err(|_| "INTERNAL_ERROR")?;
                let success = reply.phase == NativeResultHostReplyPhase::Completed;
                if success {
                    for cell in envelope["data"]["cells"]
                        .as_array_mut()
                        .ok_or("INTERNAL_ERROR")?
                    {
                        cell["result_ref"] = reply
                            .payload
                            .get(cell["cell_id"].as_str().ok_or("INTERNAL_ERROR")?)
                            .cloned()
                            .ok_or("RESULT_NOT_AVAILABLE")?;
                    }
                } else {
                    envelope["ok"] = serde_json::json!(false);
                    envelope["status"] = serde_json::json!("failed");
                    envelope["data"] = serde_json::Value::Null;
                    envelope["error"] = serde_json::json!({"code":"NOT_AVAILABLE","message":"Actual retained output metadata unavailable","recovery":"retry_read","details":{}});
                }
                let envelope = crate::tools::checked_data("read_notebook_result", envelope)
                    .map_err(|_| "TOOL_RESULT_LIMIT")?;
                return self.agent.as_mut().expect("checked").register_readonly(
                    &intent,
                    &current,
                    serde_json::to_string(&envelope).map_err(|_| "INTERNAL_ERROR")?,
                    success,
                );
            }
            self.flush_result_state();
            if intent.name == AgentToolName::InspectResult {
                if self.results.is_none() {
                    self.results = Some(crate::results::runtime::ResultRuntime::new()?);
                }
                let request = crate::tools::results::FrozenInspection::original(&intent, &args)?;
                self.results
                    .as_ref()
                    .expect("created")
                    .agent_inspect(request, &current)?;
                self.pending_reads.insert(
                    intent.operation_id.clone(),
                    PendingRead {
                        scope: current,
                        kind: PendingReadKind::Inspect,
                        issued: Instant::now(),
                    },
                );
                return Ok(self.agent.as_ref().expect("checked").pending_readonly());
            }
            let accepted = self.kernel.as_ref().and_then(|k| k.accepted_state());
            let result = crate::tools::dispatch::execute(
                &intent,
                args.as_bytes(),
                crate::tools::dispatch::ReadonlySource {
                    controller: &mut self.controller,
                    reads: self.reads.as_mut().ok_or("AGENT_NOT_ACTIVE")?,
                    coordinator: &self.coordinator,
                    accepted: accepted.as_ref(),
                    scope: &current,
                    now: self.clock.elapsed().as_millis() as u64,
                },
            )?;
            if result.success
                && let Some((revision, cells)) = result.outputs
                && !cells.is_empty()
                && let Some(results) = &self.results
            {
                results.command(
                    NativeResultHostCommand::ResultNotebookReferences(ResultNotebookReferences {
                        r#type: ResultNotebookReferencesType::ResultNotebookReferences,
                        request_id: intent.operation_id.clone(),
                        kernel_state_revision: Serial::new(revision)?,
                        cells,
                    }),
                    &current,
                )?;
                self.pending_reads.insert(
                    intent.operation_id.clone(),
                    PendingRead {
                        scope: current,
                        kind: PendingReadKind::Outputs(result.json),
                        issued: Instant::now(),
                    },
                );
                return Ok(self.agent.as_ref().expect("checked").pending_readonly());
            }
            return self.agent.as_mut().expect("checked").register_readonly(
                &intent,
                &current,
                result.json,
                result.success,
            );
        }
        let closing = matches!(
            &command,
            NativeAgentHostCommand::NativeAgentClose(_)
                | NativeAgentHostCommand::NativeAgentCancel(_)
        );
        let reply = self
            .agent
            .as_mut()
            .expect("checked")
            .command(command, &binding, &current);
        if closing && reply.is_ok() {
            for patch in self.patches.values() {
                let _ = self
                    .controller
                    .cancel(&patch.plan.commit.commit.operation_id);
            }
            for id in self.pending_reads.keys() {
                if let Some(results) = &self.results {
                    let _ = results.cancel(id);
                }
            }
            self.pending_reads.clear();
            self.reads = None;
        }
        reply
    }
    /// Direct cancellation does not queue behind SQLite, CAS or SourcePreview's physical IO.
    pub fn cancel(&mut self, operation: &str) -> Option<bool> {
        if let Some(result) = self
            .results
            .as_ref()
            .and_then(|results| results.cancel(operation))
        {
            return Some(result);
        }
        if let Some(result) = self
            .kernel
            .as_mut()
            .and_then(|kernel| kernel.cancel(operation))
        {
            return Some(result);
        }
        let previous = self.controller.operation(operation)?;
        let current = self.controller.cancel(operation).ok()?;
        Some(previous.phase != NativeCommitStatePhase::Completed && current.cancel_requested)
    }
    /// Trusted math port shares this exact source controller, confirmed head and lifetime.
    pub fn kernel_command(
        &mut self,
        command: NativeKernelHostCommand,
    ) -> Result<NativeKernelHostReply, String> {
        let id = match &command {
            NativeKernelHostCommand::KernelBootstrap(b) => Some(&b.operation_id),
            NativeKernelHostCommand::KernelRunCell(b) => Some(&b.operation_id),
            NativeKernelHostCommand::KernelRestoreStart(b) => Some(&b.operation_id),
            _ => None,
        };
        if id.is_some_and(|id| self.results.as_ref().is_some_and(|r| r.has_request(id))) {
            return Err("DUPLICATE_OPERATION".into());
        }
        if self.kernel.is_none() {
            self.kernel = Some(crate::kernel::endpoint::KernelEndpoint::new()?);
        }
        let context = crate::kernel::acceptance::AcceptanceContext {
            runtime_instance_id: self.scope.runtime.clone(),
            store_id: self.store.clone(),
            document_generation: Serial::new(self.scope.generation)?,
            source: self.controller.owner().snapshot().clone(),
            general: self.general.clone(),
            config_revision: Serial::new(self.scope.config_revision)?,
            build: crate::kernel::endpoint::KERNEL_BUILD_ID.into(),
        };
        let kernel = self.kernel.as_mut().expect("created");
        let previous_checkpoint = kernel
            .accepted_state()
            .map(|state| state.receipt().checkpoint_id.clone());
        let newly_accepted = matches!(&command,NativeKernelHostCommand::KernelCompleted(body) if !kernel.operation_completed(&body.operation_id));
        let reply = kernel.command(command, &context, &mut self.controller)?;
        self.scope.definition_revision = reply.kernel_state_revision.get();
        if let Some(facts) = kernel.accepted_context() {
            self.coordinator.known_functions = facts.known_functions.clone();
            self.coordinator.owned = facts.owned.clone();
        }
        if (newly_accepted && reply.phase == NativeKernelHostReplyPhase::Completed)
            || (previous_checkpoint.is_none() && reply.active_checkpoint_id.0.is_some())
        {
            self.pending_result_state = kernel.accepted_state();
            self.flush_result_state();
        }
        Ok(reply)
    }
    fn flush_result_state(&mut self) {
        if self.pending_result_state.is_none() {
            return;
        }
        if self.results.is_none() {
            self.results = crate::results::runtime::ResultRuntime::new().ok();
        }
        if let (Some(results), Some(accepted)) = (&self.results, &self.pending_result_state)
            && results.accepted(accepted.clone()).is_ok()
        {
            self.pending_result_state = None;
        }
    }
    /// Trusted readonly data access. Heavy decode/projection work stays off this source lock.
    pub fn result_command(
        &mut self,
        command: NativeResultHostCommand,
    ) -> Result<NativeResultHostReply, String> {
        let id = match &command {
            NativeResultHostCommand::ResultManifest(b) => Some(&b.request_id),
            NativeResultHostCommand::ResultInspect(b) => Some(&b.request_id),
            NativeResultHostCommand::ResultRevoke(b) => Some(&b.request_id),
            NativeResultHostCommand::ResultNotebookReferences(b) => Some(&b.request_id),
            _ => None,
        };
        if id.is_some_and(|id| {
            self.controller.operation(id).is_some()
                || self.kernel.as_ref().is_some_and(|k| k.has_operation(id))
        }) {
            return Err("DUPLICATE_OPERATION".into());
        }
        self.flush_result_state();
        let mut current = self.scope.clone();
        let source = self.controller.owner().snapshot();
        current.revision = source.revision.get();
        current.execution_epoch = source.execution_epoch.get();
        current.snapshot_hash = source.snapshot_hash.clone();
        self.results
            .as_ref()
            .ok_or("RESULT_NOT_AVAILABLE")?
            .command(command, &current)
    }
    /// Short confirmed terminal lookup; no operation or disk access occurs.
    pub fn kernel_operation_completed(&self, operation: &str) -> bool {
        self.kernel
            .as_ref()
            .is_some_and(|kernel| kernel.operation_completed(operation))
    }
    /// Revoke work without joining mathematical threads while holding the source owner lock.
    pub fn close_kernel_begin(&mut self) {
        self.pending_reads.clear();
        self.reads = None;
        if let Some(agent) = self.agent.as_mut() {
            agent.close();
        }
        if let Some(results) = self.results.as_ref() {
            results.begin_close();
        }
        if let Some(kernel) = self.kernel.as_mut() {
            kernel.begin_close();
        }
    }
    /// Extract only; background host close performs joins after dropping the source owner lock.
    pub fn take_kernel_threads(&mut self) -> Vec<std::thread::JoinHandle<()>> {
        let mut threads = self
            .kernel
            .as_mut()
            .map_or_else(Vec::new, |kernel| kernel.take_join_handles());
        if let Some(thread) = self
            .results
            .as_ref()
            .and_then(|results| results.take_thread())
        {
            threads.push(thread);
        }
        threads
    }
    /// Reads/source/draft binding stay available while another operation waits for physical commit.
    pub fn command(
        &mut self,
        command: NativeSourceHostCommand,
    ) -> Result<NativeSourceHostReply, String> {
        let now = self.clock.elapsed().as_millis() as u64;
        let mut reply = NativeSourceHostReply {
            protocol_version: 1,
            kind: NativeSourceHostReplyKind::State,
            document_generation: Serial::new(self.scope.generation)?,
            owner_time_ms: Serial::new(now)?,
            snapshot: Nullable(None),
            plan: Nullable(None),
            preview: Nullable(None),
            reference: Nullable(None),
            operation: Nullable(None),
        };
        let error = |error: super::commit::CommitError| match error {
            super::commit::CommitError::Busy => "OPERATION_IN_PROGRESS".to_owned(),
            super::commit::CommitError::Invalid => "PREVIEW_MISMATCH".to_owned(),
            super::commit::CommitError::Cancelled => "CANCELLED".to_owned(),
            super::commit::CommitError::UndoConflict => "UNDO_CONFLICT".to_owned(),
            super::commit::CommitError::Preview(_) => "STALE_SNAPSHOT".to_owned(),
        };
        match command {
            NativeSourceHostCommand::SourceBeginAgent(body) => {
                self.begin_agent_patch(&body.operation_id, &mut reply)?;
            }
            NativeSourceHostCommand::SourceRecoverUndo(body) => {
                if body.receipt.receipt.store_id != self.store {
                    return Err("INVALID_RECEIPT".into());
                }
                reply.operation = Nullable(Some(
                    self.controller.recover_undo(&body.receipt).map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourcePrepareUndo(body) => {
                if self
                    .results
                    .as_ref()
                    .is_some_and(|r| r.has_request(&body.operation_id))
                    || self
                        .kernel
                        .as_ref()
                        .is_some_and(|k| k.has_operation(&body.operation_id))
                {
                    return Err("DUPLICATE_OPERATION".into());
                }
                for record in &body.records {
                    let receipt = &record.receipt;
                    let plan = &record.plan;
                    super::validate_commit(plan).map_err(|_| "INVALID_UNDO_RECORD")?;
                    if record.store_id != self.store
                        || record.transaction_id != plan.commit.transaction_id
                        || plan.commit.document_id != self.scope.document
                        || plan.calculation_change.0.is_some()
                        || receipt.protocol_version != 1
                        || receipt.receipt.store_id != self.store
                        || receipt.receipt.document_id.0.as_ref() != Some(&self.scope.document)
                        || receipt.receipt.transaction_id.0.as_ref() != Some(&record.transaction_id)
                        || receipt.receipt.operation_id != plan.commit.operation_id
                        || receipt.receipt.request_hash != plan.commit.request_hash
                        || receipt.receipt.phase != OperationReceiptPhase::Completed
                        || receipt.receipt.committed_revision.0 != Some(plan.after.revision)
                        || receipt.receipt.error_code.0.is_some()
                        || !receipt.receipt.accepted_result_ids.is_empty()
                        || receipt.receipt.operation_kind
                            != if plan.commit.actor == DocumentCommitActor::Undo {
                                OperationReceiptOperationKind::Undo
                            } else {
                                OperationReceiptOperationKind::SourceEdit
                            }
                        || receipt.snapshot_hash != plan.after.snapshot_hash
                        || receipt.inverse_plan_hash != plan.before.snapshot_hash
                        || receipt.execution_epoch != plan.after.execution_epoch
                        || receipt.outbox_event_id != plan.commit.outbox_event_id
                    {
                        return Err("INVALID_UNDO_RECORD".into());
                    }
                }
                let operation = body.operation_id.clone();
                reply.plan = Nullable(
                    self.controller
                        .undo(
                            &body.records,
                            body.operation_id,
                            body.group_id,
                            &self.scope,
                            &self.coordinator,
                        )
                        .map_err(error)?,
                );
                reply.operation = Nullable(self.controller.operation(&operation));
                reply.kind = if reply.plan.0.is_some() {
                    NativeSourceHostReplyKind::CommitPlan
                } else {
                    NativeSourceHostReplyKind::State
                };
            }
            NativeSourceHostCommand::SourcePrepareManual(body) => {
                reply.kind = NativeSourceHostReplyKind::CommitPlan;
                reply.plan = Nullable(Some(
                    self.controller
                        .manual(
                            &body.operations,
                            &self.scope,
                            &self.coordinator,
                            body.input_group_id,
                            "host-manual-time".into(),
                        )
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceRead(_) => {
                reply.kind = NativeSourceHostReplyKind::Source;
                reply.snapshot = Nullable(Some(self.controller.owner().snapshot().clone()));
            }
            NativeSourceHostCommand::SourceSnapshotRef(body) => {
                reply.kind = NativeSourceHostReplyKind::SnapshotRef;
                reply.reference = Nullable(Some(
                    self.controller
                        .snapshot(
                            self.coordinator.clone(),
                            body.complete_cell_ids.into_iter().collect(),
                            &self.scope,
                            now,
                        )
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourcePreview(body) => {
                let data = serde_json::to_vec(&body.arguments).map_err(|_| "INVALID_ARGUMENT")?;
                let result = self
                    .controller
                    .preview(&data, &self.scope, now, "host-preview-time".into())
                    .map_err(error)?;
                reply.kind = NativeSourceHostReplyKind::Preview;
                reply.preview = Nullable(Some(
                    serde_json::to_value(result).map_err(|_| "INTERNAL_ERROR")?,
                ));
            }
            NativeSourceHostCommand::SourceBegin(body) => {
                let plan = self
                    .controller
                    .begin(&body.preview_ref, &self.scope, now)
                    .map_err(error)?;
                reply.kind = if plan.is_some() {
                    NativeSourceHostReplyKind::CommitPlan
                } else {
                    NativeSourceHostReplyKind::State
                };
                reply.plan = Nullable(plan.map(|p| p.commit.clone()));
                reply.operation = Nullable(self.controller.preview_operation(&body.preview_ref));
            }
            NativeSourceHostCommand::SourceAdmitted(body) => {
                if body
                    .admission
                    .receipt
                    .0
                    .as_ref()
                    .is_some_and(|receipt| receipt.receipt.store_id != self.store)
                {
                    return Err("INVALID_RECEIPT".into());
                }
                reply.operation = Nullable(Some(
                    self.controller
                        .admitted(&body.operation_id, &body.admission)
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceFenced(body) => {
                reply.operation = Nullable(Some(
                    self.controller
                        .fenced(&body.operation_id, body.fence, now)
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceBarrier(body) => {
                let current = self.patch_barrier_scope(&body.operation_id)?;
                reply.operation = Nullable(Some(
                    self.controller
                        .enter_commit(&body.operation_id, &body.fence_id, &current, now)
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceUnknown(body) => {
                reply.operation = Nullable(Some(
                    self.controller.unknown(&body.operation_id).map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceCompleted(body) => {
                if body.receipt.receipt.store_id != self.store {
                    return Err("INVALID_RECEIPT".into());
                }
                let settings = self
                    .controller
                    .pending_calculation_change(&body.operation_id)
                    .map(|change| change.after.clone());
                reply.operation = Nullable(Some(
                    self.controller
                        .accept_receipt(&body.operation_id, &body.receipt)
                        .map_err(error)?,
                ));
                let source = self.controller.owner().snapshot();
                self.scope.revision = source.revision.get();
                self.scope.execution_epoch = source.execution_epoch.get();
                self.scope.snapshot_hash = source.snapshot_hash.clone();
                if let Some(settings) = settings {
                    self.general.dialect = match settings.dialect {
                        NativeCalculationSettingsDialect::Auto => {
                            om_kernel::config::ConfigDialect::Auto
                        }
                        NativeCalculationSettingsDialect::Modern => {
                            om_kernel::config::ConfigDialect::Modern
                        }
                        NativeCalculationSettingsDialect::Wolfram => {
                            om_kernel::config::ConfigDialect::Wolfram
                        }
                    };
                    self.general.constants = match settings.constants {
                        NativeCalculationSettingsConstants::Math => {
                            om_kernel::config::Constants::Math
                        }
                        NativeCalculationSettingsConstants::Strict => {
                            om_kernel::config::Constants::Strict
                        }
                    };
                    self.general.reactive = settings.reactive;
                    self.general.auto_run_dependents = settings.auto_run_dependents;
                    self.general.show_steps = settings.show_steps;
                    self.general.auto_plot = settings.auto_plot;
                    self.general.eval_timeout_ms = settings.eval_timeout_ms.get();
                    self.coordinator.dialect = self.general.dialect;
                    self.coordinator.constants = self.general.constants;
                    self.scope.config_revision = source.revision.get();
                }
                reply.snapshot = Nullable(Some(source.clone()));
            }
            NativeSourceHostCommand::SourceSettled(body) => {
                reply.operation = Nullable(Some(
                    self.controller
                        .settle_no_commit(&body.operation_id, body.cancelled)
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceStatus(body) => {
                reply.operation = Nullable(self.controller.operation(&body.operation_id));
            }
            NativeSourceHostCommand::SourceEditorChanged(body) => {
                if body.state.runtime_instance_id != self.scope.runtime
                    || body.state.document_id != self.scope.document
                    || body.state.document_generation.get() != self.scope.generation
                    || body.state.source_revision != self.controller.owner().snapshot().revision
                {
                    return Err("STALE_DOCUMENT".into());
                }
                self.scope.editor_state_hash = format!(
                    "{:x}",
                    Sha256::digest(
                        serde_json::to_vec(&body.state).map_err(|_| "INVALID_ARGUMENT")?
                    )
                );
                reply.kind = NativeSourceHostReplyKind::EditorUpdated;
            }
            NativeSourceHostCommand::SourceOpen(_) => return Err("SOURCE_ALREADY_OPEN".into()),
        }
        if reply
            .operation
            .0
            .as_ref()
            .is_some_and(|state| state.phase == NativeCommitStatePhase::Completed)
        {
            let source = self.controller.owner().snapshot();
            self.scope.revision = source.revision.get();
            self.scope.execution_epoch = source.execution_epoch.get();
            self.scope.snapshot_hash = source.snapshot_hash.clone();
            reply.snapshot = Nullable(Some(source.clone()));
        }
        Ok(reply)
    }
}
/// Exact closed host union; this does not authorize a Pi/model to send host-internal commands.
pub fn decode_source_command(bytes: &[u8]) -> Result<NativeSourceHostCommand, String> {
    static SCHEMA: OnceLock<serde_json::Value> = OnceLock::new();
    let root = SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/design/native-source-store.schema.json"
        ))
        .expect("checked source schema")
    });
    validation::decode(bytes, &root["$defs"]["NativeSourceHostCommand"], root)
}

/// Reconstruct the exact supported non-secret calculation configuration for a stored producer.
pub(crate) fn general_configuration(
    calculation: Option<&NativeCalculationSettings>,
) -> om_kernel::config::GeneralConfig {
    let mut general = om_kernel::config::GeneralConfig::default();
    if let Some(setting) = calculation {
        general.dialect = match setting.dialect {
            NativeCalculationSettingsDialect::Auto => om_kernel::config::ConfigDialect::Auto,
            NativeCalculationSettingsDialect::Modern => om_kernel::config::ConfigDialect::Modern,
            NativeCalculationSettingsDialect::Wolfram => om_kernel::config::ConfigDialect::Wolfram,
        };
        general.constants = match setting.constants {
            NativeCalculationSettingsConstants::Math => om_kernel::config::Constants::Math,
            NativeCalculationSettingsConstants::Strict => om_kernel::config::Constants::Strict,
        };
        general.reactive = setting.reactive;
        general.auto_run_dependents = setting.auto_run_dependents;
        general.show_steps = setting.show_steps;
        general.auto_plot = setting.auto_plot;
        general.eval_timeout_ms = setting.eval_timeout_ms.get();
    }
    general
}
