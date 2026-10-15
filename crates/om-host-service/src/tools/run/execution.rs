//! Main-kernel execution of a frozen explicit plan. Physical persistence remains with its owner.
//! This is a native coordinator, not a model DTO, a RunAll wrapper or a recoverable write grant.
use super::{PreparedRun, RunBinding};
use crate::{
    document::commit::DocumentCommitController,
    kernel::{
        acceptance::AcceptanceContext,
        endpoint::{KERNEL_BUILD_ID, KernelEndpoint},
    },
    protocol::generated::*,
    references::ReferenceScope,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// A running group never equates a produced candidate with an accepted document result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunPhase {
    /// Stable next child is frozen, but its native durable admission/start is not acknowledged.
    AwaitingAdmission,
    /// A child is queued/running, or the next child has just been admitted.
    Running,
    /// The actual child candidate needs publication, final barrier and a durable receipt.
    AwaitingAcceptance,
    /// The original physical operation has not been settled. No next child may be admitted.
    Unknown,
    /// Every planned target completed successfully with a real accepted receipt.
    Completed,
    /// At least one cell was accepted, but the whole original plan did not succeed.
    Partial,
    /// No cell result was accepted and execution failed or was blocked.
    Failed,
    /// Host stop prevented further admission; accepted prefix remains in the report.
    Cancelled,
}
/// A nonaccepted cell or a failed accepted cell, never an invented execution receipt.
#[derive(Clone, Debug, Serialize)]
pub struct RunStoppedCell {
    /// Exact actual cell identity.
    pub cell_id: String,
    /// Original native child operation, absent for cells which were never admitted.
    pub operation_id: Option<String>,
    /// Actual worker/control failure or explicit planning/readiness condition.
    pub code: String,
}
/// Inspectable native facts only. Canonical result refs and durable group records are separate.
#[derive(Clone, Debug, Serialize)]
pub struct RunReport {
    /// Original ACKed tool operation. A new model call has a different group identity.
    pub operation_id: String,
    /// Exact original argument hash, retained for durable admission by the host adapter.
    pub arguments_hash: String,
    /// Actual main-kernel bootstrap receipt; never counted as a cell execution.
    pub bootstrap_receipt: Option<NativeKernelReceipt>,
    /// Accepted child receipts in the actual acceptance order, including partial cell errors.
    pub accepted_cells: Vec<NativeKernelReceipt>,
    /// Explicit failed/skipped cells. Successful statements in an error receipt remain real.
    pub stopped_cells: Vec<RunStoppedCell>,
    /// Original outstanding operation, preserved while a commit is uncertain.
    pub active_operation: Option<String>,
    /// Actual whole-group phase.
    pub phase: RunPhase,
    /// Group stop condition; no unknown condition authorizes a fresh-ID replay.
    pub stop_code: Option<String>,
    /// A genuine unknown transition was observed, even if original readback later resolves it.
    pub outcome_was_unknown: bool,
}
/// One short native owner step. The child reply can be consumed by the existing IO port.
pub struct RunUpdate {
    /// Immutable copy of the current real facts.
    pub report: RunReport,
    /// Original single-child reply with its actual frozen checkpoint plan, if present.
    pub child: Option<NativeKernelHostReply>,
    /// Data to durably claim before the native adapter acknowledges and permits execution.
    pub admission: Option<RunAdmission>,
}
/// Exact next native child. Serializing this data does not grant permission to resume execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RunAdmission {
    /// Original frozen group identity.
    pub group_operation_id: String,
    /// Stable child identity, shared by its admission, start, physical plan and receipt.
    pub operation_id: String,
    /// Absent only for a real initial checkpoint bootstrap.
    pub cell_id: Option<String>,
    /// Zero for bootstrap; otherwise one-based position in the actual frozen plan.
    pub ordinal: usize,
    /// Exact original source snapshot, recorded before computation starts.
    pub expected_source_hash: String,
}
struct Active {
    id: String,
    cell: Option<String>,
}
pub(super) fn child_operation(
    intent: &crate::agent::ingress::ToolIntent,
    ordinal: usize,
) -> String {
    let value = serde_json::to_vec(&(
        "openmath/run-child/v1",
        &intent.operation_id,
        &intent.arguments_hash,
        ordinal,
    ))
    .expect("string tuple");
    format!("run-{:x}", Sha256::digest(value))
}
/// An execution capability can only be constructed from a real frozen/read-issued plan.
/// A native adapter must durably claim the group and each child before calling poll; restoring
/// a serialized RunReport can never construct this executor or restart its computation.
pub struct RunExecutor {
    binding: RunBinding,
    plan: om_kernel::CellExecutionPlan,
    continue_on_error: bool,
    next: usize,
    active: Option<Active>,
    head: Option<NativeKernelReceipt>,
    report: RunReport,
    stopped: bool,
    cancelled: bool,
    admission: Option<RunAdmission>,
    admitted: bool,
}
impl PreparedRun {
    /// Consume the unforgeable native plan. Creating an executor performs no IO/evaluation.
    pub fn into_executor(self) -> RunExecutor {
        let mut stopped_cells = Vec::new();
        for (cells, code) in [
            (&self.plan.cycle_cell_ids, "DEPENDENCY_CYCLE"),
            (&self.plan.blocked_cell_ids, "DEPENDENCY_BLOCKED"),
        ] {
            stopped_cells.extend(cells.iter().map(|id| RunStoppedCell {
                cell_id: id.clone(),
                operation_id: None,
                code: code.into(),
            }));
        }
        RunExecutor {
            head: self.binding.parent.clone(),
            report: RunReport {
                operation_id: self.binding.intent.operation_id.clone(),
                arguments_hash: self.binding.intent.arguments_hash.clone(),
                bootstrap_receipt: None,
                accepted_cells: Vec::new(),
                stopped_cells,
                active_operation: None,
                phase: RunPhase::Running,
                stop_code: None,
                outcome_was_unknown: false,
            },
            binding: self.binding,
            plan: self.plan,
            continue_on_error: self.continue_on_error,
            next: 0,
            active: None,
            stopped: false,
            cancelled: false,
            admission: None,
            admitted: false,
        }
    }
}
impl RunExecutor {
    /// Stable original bootstrap/child IDs. No random bootstrap identity on resumption.
    pub fn child_operation(&self, ordinal: usize) -> String {
        child_operation(&self.binding.intent, ordinal)
    }
    /// Original planning scope, used only to validate this source owner's live activation.
    pub fn scope(&self) -> &ReferenceScope {
        &self.binding.scope
    }
    /// Frozen order for the durable native group admission. No model-supplied replacement plan.
    pub fn plan(&self) -> &om_kernel::CellExecutionPlan {
        &self.plan
    }
    /// Current facts; this data does not carry an execution capability.
    pub fn report(&self) -> &RunReport {
        &self.report
    }
    /// Called only by the trusted native adapter after both original durable claim/start ACKs.
    /// A lost ACK must be reconciled under the same ID; this method cannot choose another cell.
    pub fn acknowledge_admission(&mut self, operation: &str) -> Result<(), String> {
        if self.stopped
            || self
                .admission
                .as_ref()
                .is_none_or(|a| a.operation_id != operation)
        {
            return Err("IPC_CALL_SCOPE_MISMATCH".into());
        }
        self.admitted = true;
        Ok(())
    }
    fn fail(&mut self, code: &str) {
        self.stopped = true;
        if self.report.stop_code.is_none() {
            self.report.stop_code = Some(code.into());
        }
    }
    fn check_live(
        &self,
        current: &ReferenceScope,
        ctx: &AcceptanceContext,
        kernel: &KernelEndpoint,
    ) -> Result<(), String> {
        let mut expected = self.binding.scope.clone();
        expected.definition_revision = self
            .head
            .as_ref()
            .map_or(0, |r| r.kernel_state_revision.get());
        if current != &expected || !current.execution_mode || !current.can_read {
            return Err("STALE_AGENT_SCOPE".into());
        }
        if ctx.runtime_instance_id != expected.runtime
            || ctx.document_generation.get() != expected.generation
            || ctx.source.snapshot_hash != self.binding.source.snapshot_hash
            || ctx.source.document_id != expected.document
            || ctx.source.revision.get() != expected.revision
            || ctx.source.execution_epoch.get() != expected.execution_epoch
            || ctx.config_revision.get() != expected.config_revision
            || ctx.build != KERNEL_BUILD_ID
            || format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&ctx.general).map_err(|_| "INVALID_ARGUMENT")?)
            ) != self.binding.general_hash
        {
            return Err("STALE_SOURCE".into());
        }
        let actual = kernel.accepted_state();
        match (&self.head, actual) {
            (None, None) => Ok(()),
            (Some(expected), Some(actual))
                if expected.checkpoint_id == actual.receipt().checkpoint_id
                    && expected.checkpoint_blob_hash == actual.receipt().checkpoint_blob_hash
                    && expected.kernel_state_revision == actual.receipt().kernel_state_revision
                    && expected.store_id == actual.receipt().store_id
                    && expected.store_id == ctx.store_id =>
            {
                Ok(())
            }
            _ => Err("STALE_KERNEL_STATE".into()),
        }
    }
    fn finish(&mut self) {
        self.report.active_operation = None;
        self.admission = None;
        self.admitted = false;
        self.report.phase = if self.cancelled {
            RunPhase::Cancelled
        } else if (self.report.stop_code.is_none()
            || self.report.stop_code.as_deref() == Some("STOPPED_AFTER_UNKNOWN")
                && self.next == self.plan.ordered_cell_ids.len())
            && self.report.stopped_cells.is_empty()
        {
            RunPhase::Completed
        } else if self.report.accepted_cells.is_empty() {
            RunPhase::Failed
        } else {
            RunPhase::Partial
        };
        if self.report.phase == RunPhase::Completed {
            self.report.stop_code = None;
        }
    }
    fn update(&self, child: Option<NativeKernelHostReply>) -> RunUpdate {
        RunUpdate {
            report: self.report.clone(),
            child,
            admission: self.admission.clone(),
        }
    }
    /// Direct host stop reaches the original active token and prevents every future admission.
    /// If physical commit has already won, its original receipt is still read and reported.
    pub fn cancel(&mut self, kernel: &mut KernelEndpoint) {
        if matches!(
            self.report.phase,
            RunPhase::Completed | RunPhase::Partial | RunPhase::Failed | RunPhase::Cancelled
        ) {
            return;
        }
        self.fail("CANCELLED");
        self.cancelled = true;
        if let Some(active) = &self.active {
            let _ = kernel.cancel(&active.id);
        }
    }
    fn accept(&mut self, receipt: &NativeKernelReceipt) -> Result<(), String> {
        let active = self.active.as_ref().ok_or("INVALID_OPERATION")?;
        let p = &receipt.producer;
        let expected_revision = if active.cell.is_none() {
            0
        } else {
            self.head
                .as_ref()
                .ok_or("CONTEXT_NOT_READY")?
                .kernel_state_revision
                .checked_next()?
                .get()
        };
        if receipt.operation_id != active.id
            || p.cell_id.0 != active.cell
            || p.document_id != self.binding.scope.document
            || p.document_generation.get() != self.binding.scope.generation
            || p.source_snapshot_hash != self.binding.source.snapshot_hash
            || p.source_revision != self.binding.source.revision
            || p.execution_epoch != self.binding.source.execution_epoch
            || p.config_revision.get() != self.binding.scope.config_revision
            || p.general_hash != self.binding.general_hash
            || p.kernel_build != KERNEL_BUILD_ID
            || receipt.kernel_state_revision.get() != expected_revision
        {
            return Err("INVALID_RECEIPT".into());
        }
        if active.cell.is_none() {
            self.report.bootstrap_receipt = Some(receipt.clone());
        } else {
            self.report.accepted_cells.push(receipt.clone());
            if p.terminal_status != NativeKernelTerminalStatus::Done {
                self.report.stopped_cells.push(RunStoppedCell {
                    cell_id: active.cell.clone().expect("cell"),
                    operation_id: Some(active.id.clone()),
                    code: "CELL_ERROR".into(),
                });
                if !self.continue_on_error {
                    self.fail("CELL_ERROR");
                }
            }
        }
        self.head = Some(receipt.clone());
        Ok(())
    }
    /// Poll/dispatch only one actual main-kernel child. It never publishes a Blob, fabricates a
    /// receipt, or admits the next child while the original physical operation is unsettled.
    pub fn poll(
        &mut self,
        current: &ReferenceScope,
        ctx: &AcceptanceContext,
        kernel: &mut KernelEndpoint,
        source: &mut DocumentCommitController,
    ) -> Result<RunUpdate, String> {
        if matches!(
            self.report.phase,
            RunPhase::Completed | RunPhase::Partial | RunPhase::Failed | RunPhase::Cancelled
        ) {
            return Ok(self.update(None));
        }
        if let Some(active) = &self.active {
            let reply = kernel.command(
                NativeKernelHostCommand::KernelStatus(KernelStatus {
                    r#type: KernelStatusType::KernelStatus,
                    operation_id: active.id.clone(),
                }),
                ctx,
                source,
            )?;
            match reply.phase {
                NativeKernelHostReplyPhase::Completed => {
                    self.accept(reply.receipt.0.as_ref().ok_or("INVALID_RECEIPT")?)?;
                    self.active = None;
                    self.report.active_operation = None;
                }
                NativeKernelHostReplyPhase::Failed | NativeKernelHostReplyPhase::Discarded => {
                    let active = self.active.take().expect("active");
                    let code = reply.error_code.0.as_deref().unwrap_or("NOT_ACCEPTED");
                    if let Some(cell_id) = active.cell {
                        self.report.stopped_cells.push(RunStoppedCell {
                            cell_id,
                            operation_id: Some(active.id),
                            code: code.into(),
                        });
                    } else {
                        self.fail(code);
                    }
                    if !self.continue_on_error || code == "CANCELLED" {
                        self.fail(code);
                    }
                    self.report.active_operation = None;
                }
                NativeKernelHostReplyPhase::Unknown | NativeKernelHostReplyPhase::Committing => {
                    // Committing is also unresolved; only the original receipt/absence may settle it.
                    if reply.phase == NativeKernelHostReplyPhase::Unknown {
                        self.fail("STOPPED_AFTER_UNKNOWN");
                        self.report.outcome_was_unknown = true;
                    }
                    self.report.phase = RunPhase::Unknown;
                    return Ok(self.update(Some(reply)));
                }
                _ => {
                    if let Err(code) = self.check_live(current, ctx, kernel) {
                        self.fail(&code);
                    }
                    if self.stopped {
                        let id = self.active.as_ref().expect("active").id.clone();
                        let _ = kernel.cancel(&id);
                        if reply.phase == NativeKernelHostReplyPhase::Candidate {
                            let settled = kernel.command(
                                NativeKernelHostCommand::KernelDiscard(KernelDiscard {
                                    r#type: KernelDiscardType::KernelDiscard,
                                    operation_id: id,
                                }),
                                ctx,
                                source,
                            )?;
                            self.active = None;
                            self.finish();
                            return Ok(self.update(Some(settled)));
                        }
                    }
                    self.report.phase = if reply.phase == NativeKernelHostReplyPhase::Candidate {
                        RunPhase::AwaitingAcceptance
                    } else {
                        RunPhase::Running
                    };
                    return Ok(self.update(Some(reply)));
                }
            }
        }
        if let Err(code) = self.check_live(current, ctx, kernel) {
            self.fail(&code);
        }
        if !self.continue_on_error && !self.report.stopped_cells.is_empty() {
            self.fail("DEPENDENCY_NOT_READY");
        }
        if self.stopped || self.next == self.plan.ordered_cell_ids.len() {
            self.finish();
            return Ok(self.update(None));
        }
        let (command, active) = if self.head.is_none() {
            let id = self.child_operation(0);
            (
                NativeKernelHostCommand::KernelBootstrap(KernelBootstrap {
                    r#type: KernelBootstrapType::KernelBootstrap,
                    operation_id: id.clone(),
                    expected_source_hash: self.binding.source.snapshot_hash.clone(),
                }),
                Active { id, cell: None },
            )
        } else {
            let cell = self.plan.ordered_cell_ids[self.next].clone();
            let id = self.child_operation(self.next + 1);
            (
                NativeKernelHostCommand::KernelRunCell(KernelRunCell {
                    r#type: KernelRunCellType::KernelRunCell,
                    operation_id: id.clone(),
                    cell_id: cell.clone(),
                    expected_source_hash: self.binding.source.snapshot_hash.clone(),
                }),
                Active {
                    id,
                    cell: Some(cell),
                },
            )
        };
        // Existing child identity means another/earlier claim. Do not resume calculation by
        // calling KernelRunCell with a fresh ID or silently adopt someone else's operation.
        if kernel.has_operation(&active.id) {
            self.fail("ORIGINAL_OPERATION_REQUIRES_RECONCILIATION");
            self.finish();
            return Ok(self.update(None));
        }
        if !self.admitted {
            self.admission = Some(RunAdmission {
                group_operation_id: self.binding.intent.operation_id.clone(),
                operation_id: active.id,
                cell_id: active.cell,
                ordinal: if self.head.is_none() {
                    0
                } else {
                    self.next + 1
                },
                expected_source_hash: self.binding.source.snapshot_hash.clone(),
            });
            self.report.phase = RunPhase::AwaitingAdmission;
            return Ok(self.update(None));
        }
        if self
            .admission
            .as_ref()
            .is_none_or(|a| a.operation_id != active.id || a.cell_id != active.cell)
        {
            return Err("IPC_CALL_SCOPE_MISMATCH".into());
        }
        let submitted = match command {
            NativeKernelHostCommand::KernelRunCell(body) => {
                kernel.run_planned_cell(body, ctx, source)
            }
            other => kernel.command(other, ctx, source),
        };
        match submitted {
            Ok(reply) => {
                self.admitted = false;
                self.admission = None;
                if active.cell.is_some() {
                    self.next += 1;
                }
                self.report.active_operation = Some(active.id.clone());
                self.active = Some(active);
                self.report.phase = RunPhase::Running;
                Ok(self.update(Some(reply)))
            }
            Err(code) => {
                self.fail(&code);
                self.finish();
                Ok(self.update(None))
            }
        }
    }
    /// Route an original child's physical publication/readback. The final barrier repeats the
    /// exact live task/editor/grant/source/head check. Completion/readback can still settle an
    /// already committed operation after cancellation; it never authorizes a new effect.
    pub fn physical_command(
        &mut self,
        command: NativeKernelHostCommand,
        current: &ReferenceScope,
        ctx: &AcceptanceContext,
        kernel: &mut KernelEndpoint,
        source: &mut DocumentCommitController,
    ) -> Result<NativeKernelHostReply, String> {
        let (id, barrier) = match &command {
            NativeKernelHostCommand::KernelReadBlob(c) => (&c.operation_id, false),
            NativeKernelHostCommand::KernelBarrier(c) => (&c.operation_id, true),
            NativeKernelHostCommand::KernelCompleted(c) => (&c.operation_id, false),
            NativeKernelHostCommand::KernelUnknown(c) => (&c.operation_id, false),
            NativeKernelHostCommand::KernelSettled(c) => (&c.operation_id, false),
            NativeKernelHostCommand::KernelDiscard(c) => (&c.operation_id, false),
            _ => return Err("INVALID_RUN_COMMAND".into()),
        };
        if self.active.as_ref().is_none_or(|a| a.id != *id) {
            return Err("IPC_CALL_SCOPE_MISMATCH".into());
        }
        if barrier {
            if self.stopped {
                return Err("RUN_STOPPED".into());
            }
            if let Err(code) = self.check_live(current, ctx, kernel) {
                self.fail(&code);
                let _ = kernel.cancel(id);
                return Err(code);
            }
        }
        let reply = kernel.command(command, ctx, source)?;
        if reply.phase == NativeKernelHostReplyPhase::Unknown {
            self.fail("STOPPED_AFTER_UNKNOWN");
            self.report.outcome_was_unknown = true;
        }
        Ok(reply)
    }
}
