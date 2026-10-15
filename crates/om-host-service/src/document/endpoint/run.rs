//! Original Agent run -> isolated planning -> real main owner. No IO, evaluator or caller plan on
//! this source lock. Every actual child still crosses the original physical acceptance barrier.
use super::*;
use crate::{
    agent::ingress::ToolIntent,
    kernel::{
        acceptance::AcceptanceContext,
        endpoint::{KERNEL_BUILD_ID, KernelEndpoint},
    },
    tools::run::{self, PreparedRun, RunExecutor, RunPhase, RunUpdate},
};
use om_num::ctx::{Clock, Interrupt};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Instant,
};
struct PlanningClock(Instant);
impl Clock for PlanningClock {
    fn now_ms(&self) -> f64 {
        self.0.elapsed().as_secs_f64() * 1000.
    }
}
struct PlannedRun {
    prepared: PreparedRun,
    descriptor: NativeRunDescriptor,
    source: NativeSourceSnapshot,
}
pub(super) struct PendingRun {
    pub(super) intent: ToolIntent,
    pub(super) scope: ReferenceScope,
    pub(super) token: Arc<AtomicBool>,
    receiver: Option<mpsc::Receiver<Result<PlannedRun, String>>>,
    pub(super) executor: Option<RunExecutor>,
    pub(super) descriptor: Option<NativeRunDescriptor>,
    acks: BTreeMap<String, String>,
    group_ack: Option<String>,
    pub(super) failure: Option<String>,
    source: Option<NativeSourceSnapshot>,
    result_request: Option<String>,
}
impl SourceEndpoint {
    pub(super) fn run_context(&self) -> Result<AcceptanceContext, String> {
        Ok(AcceptanceContext {
            runtime_instance_id: self.scope.runtime.clone(),
            store_id: self.store.clone(),
            document_generation: Serial::new(self.scope.generation)?,
            source: self.controller.owner().snapshot().clone(),
            general: self.general.clone(),
            config_revision: Serial::new(self.scope.config_revision)?,
            build: KERNEL_BUILD_ID.into(),
        })
    }
    fn live_run_scope(&self, id: &str) -> Result<ReferenceScope, String> {
        let agent = self.agent.as_ref().ok_or("AGENT_NOT_ACTIVE")?;
        let (_, _, scope) = agent.run_input(id, &self.binding(), self.current_source_scope())?;
        if let Some(run) = self.runs.get(id)
            && (run.scope.task != scope.task || run.scope.task_generation != scope.task_generation)
        {
            return Err("STALE_AGENT_SCOPE".into());
        }
        Ok(scope)
    }
    pub(super) fn prepare_agent_run(
        &mut self,
        c: NativeAgentPrepareRun,
    ) -> Result<NativeAgentHostReply, String> {
        let agent = self.agent.as_ref().ok_or("AGENT_NOT_ACTIVE")?;
        agent.check_native_scope(&c.scope, &self.binding())?;
        let (intent, args, scope) = agent.run_input(
            &c.operation_id,
            &self.binding(),
            self.current_source_scope(),
        )?;
        if self.runs.contains_key(&c.operation_id) {
            return self.poll_agent_run(NativeAgentPollRun {
                r#type: NativeAgentPollRunType::AgentPollRun,
                scope: c.scope,
                operation_id: c.operation_id,
            });
        }
        if self.runs.len() >= 32 {
            return Err("LIMIT_EXCEEDED".into());
        }
        let accepted = self.kernel.as_ref().and_then(|k| k.accepted_state());
        // Native restrictions are outside original/model args and cannot widen a cached plan.
        let allowed = c
            .allowed_cell_ids
            .0
            .map(|ids| ids.into_iter().collect())
            .unwrap_or_else(|| {
                self.controller
                    .owner()
                    .snapshot()
                    .file
                    .cells
                    .iter()
                    .map(|c| c.id.clone())
                    .collect()
            });
        let input = run::prepare(
            &intent,
            args.as_bytes(),
            run::RunSource {
                reads: self.reads.as_mut().ok_or("AGENT_NOT_ACTIVE")?,
                scope: &scope,
                editor_busy: self.editor_busy,
                general: self.general.clone(),
                accepted: accepted.as_ref(),
                allowed,
                now: self.clock.elapsed().as_millis() as u64,
            },
        );
        let token = Arc::new(AtomicBool::new(false));
        let (receiver, failure) = match input {
            Ok(input) => {
                let (sender, receiver) = mpsc::sync_channel(1);
                let cancel = token.clone();
                let owner = thread::Builder::new()
                    .name("openmath-agent-run-planner".into())
                    .spawn(move || {
                        let clock: Arc<dyn Clock> = Arc::new(PlanningClock(Instant::now()));
                        let ctx = Interrupt {
                            flag: cancel,
                            clock: Some(clock.clone()),
                            deadline_ms: Some(clock.now_ms() + 5000.),
                            steps_left: std::cell::Cell::new(4 * 1024 * 1024),
                        };
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let prepared = input.plan(&ctx)?;
                            if prepared.plan().ordered_cell_ids.len()
                                + prepared.plan().cycle_cell_ids.len()
                                + prepared.plan().blocked_cell_ids.len()
                                > 1000
                            {
                                return Err("LIMIT_EXCEEDED".into());
                            }
                            let descriptor = prepared.descriptor()?;
                            let source = prepared.source().clone();
                            if bounded_report_bytes(&descriptor, &source)? > 256 * 1024 {
                                return Err("LIMIT_EXCEEDED".into());
                            }
                            Ok(PlannedRun {
                                prepared,
                                descriptor,
                                source,
                            })
                        }))
                        .unwrap_or_else(|_| Err("PLANNING_FAILED".into()));
                        let _ = sender.send(result);
                    })
                    .map_err(|_| "INTERNAL_ERROR")?;
                self.run_threads.push(owner);
                (Some(receiver), None)
            }
            Err(code) => (None, Some(code)),
        };
        self.runs.insert(
            c.operation_id.clone(),
            PendingRun {
                intent,
                scope,
                token,
                receiver,
                executor: None,
                descriptor: None,
                acks: BTreeMap::new(),
                group_ack: None,
                failure,
                source: None,
                result_request: None,
            },
        );
        self.poll_agent_run(NativeAgentPollRun {
            r#type: NativeAgentPollRunType::AgentPollRun,
            scope: c.scope,
            operation_id: c.operation_id,
        })
    }
    pub(super) fn poll_agent_run(
        &mut self,
        c: NativeAgentPollRun,
    ) -> Result<NativeAgentHostReply, String> {
        self.agent
            .as_ref()
            .ok_or("AGENT_NOT_ACTIVE")?
            .check_native_scope(&c.scope, &self.binding())?;
        let current = self.live_run_scope(&c.operation_id)?;
        if let Some(cached) = self
            .agent
            .as_ref()
            .expect("checked")
            .cached_effect(&c.operation_id, &current)?
        {
            return Ok(cached);
        }
        let mut entry = self
            .runs
            .remove(&c.operation_id)
            .ok_or("RUN_NOT_PREPARED")?;
        let result = self.advance_agent_run(&mut entry, &current);
        self.runs.insert(c.operation_id, entry);
        result
    }
    fn advance_agent_run(
        &mut self,
        entry: &mut PendingRun,
        current: &ReferenceScope,
    ) -> Result<NativeAgentHostReply, String> {
        if let Some(receiver) = &entry.receiver {
            match receiver.try_recv() {
                Ok(Ok(planned)) => {
                    entry.receiver = None;
                    entry.source = Some(planned.source);
                    entry.descriptor = Some(planned.descriptor);
                    entry.executor = Some(planned.prepared.into_executor());
                }
                Ok(Err(code)) => {
                    entry.receiver = None;
                    entry.failure = Some(code);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    return self.run_progress_reply(entry, None, NativeRunProgressPhase::Planning);
                }
                Err(_) => {
                    entry.receiver = None;
                    entry.failure = Some("PLANNING_FAILED".into());
                }
            }
        }
        if entry.failure.is_some() {
            return self.finish_agent_run(entry, current);
        }
        if self.kernel.is_none() {
            self.kernel = Some(KernelEndpoint::new()?);
        }
        if entry.token.load(Ordering::Acquire) {
            entry
                .executor
                .as_mut()
                .expect("prepared")
                .cancel(self.kernel.as_mut().expect("created"));
        }
        self.flush_result_state();
        if self.pending_result_state.is_some() {
            return self.run_progress_reply(entry, None, NativeRunProgressPhase::Running);
        }
        let context = self.run_context()?;
        let update = entry.executor.as_mut().ok_or("RUN_NOT_PREPARED")?.poll(
            current,
            &context,
            self.kernel.as_mut().expect("created"),
            &mut self.controller,
        )?;
        let phase = serde_json::from_value(
            serde_json::to_value(update.report.phase).map_err(|_| "INTERNAL_ERROR")?,
        )
        .map_err(|_| "INTERNAL_ERROR")?;
        if matches!(
            update.report.phase,
            RunPhase::Completed
                | RunPhase::Partial
                | RunPhase::Failed
                | RunPhase::Cancelled
                | RunPhase::Unknown
        ) {
            return self.finish_agent_run(entry, current);
        }
        self.run_progress_reply(entry, Some(update), phase)
    }
    fn run_progress_reply(
        &self,
        entry: &PendingRun,
        update: Option<RunUpdate>,
        phase: NativeRunProgressPhase,
    ) -> Result<NativeAgentHostReply, String> {
        let report = entry.executor.as_ref().map(|e| e.report());
        let admission = update.as_ref().and_then(|u| u.admission.as_ref());
        let index = admission.and_then(|a| {
            entry
                .descriptor
                .as_ref()?
                .children
                .iter()
                .position(|c| c.operation_id == a.operation_id)
        });
        let mut reply = self
            .agent
            .as_ref()
            .ok_or("AGENT_NOT_ACTIVE")?
            .pending_readonly();
        reply.run_progress = Nullable(Some(NativeRunProgress {
            operation_id: entry.intent.operation_id.clone(),
            phase,
            descriptor: Nullable(entry.descriptor.clone()),
            next_child_index: Nullable(index.map(|i| Serial::new(i as u64)).transpose()?),
            next_operation_id: Nullable(admission.map(|a| a.operation_id.clone())),
            child: Nullable(update.and_then(|u| u.child)),
            accepted_cell_ids: report
                .map(|r| {
                    r.accepted_cells
                        .iter()
                        .filter_map(|r| r.producer.cell_id.0.clone())
                        .collect()
                })
                .unwrap_or_default(),
            stopped_cells: report
                .map(|r| {
                    r.stopped_cells
                        .iter()
                        .map(|s| NativeRunStoppedCell {
                            cell_id: s.cell_id.clone(),
                            operation_id: Nullable(s.operation_id.clone()),
                            code: s.code.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            stop_code: Nullable(
                entry
                    .failure
                    .clone()
                    .or_else(|| report.and_then(|r| r.stop_code.clone())),
            ),
            outcome_was_unknown: report.is_some_and(|r| r.outcome_was_unknown),
        }));
        Ok(reply)
    }
    fn finish_agent_run(
        &mut self,
        entry: &mut PendingRun,
        current: &ReferenceScope,
    ) -> Result<NativeAgentHostReply, String> {
        let report = entry.executor.as_ref().map(|e| e.report().clone());
        let mut outputs = json!({});
        if let Some(report) = &report
            && !report.accepted_cells.is_empty()
        {
            self.flush_result_state();
            if self.pending_result_state.is_some() {
                return self.run_progress_reply(entry, None, NativeRunProgressPhase::Running);
            }
            let results = self.results.as_ref().ok_or("RESULT_NOT_AVAILABLE")?;
            let result = if let Some(id) = &entry.result_request {
                results.command(
                    NativeResultHostCommand::ResultStatus(ResultStatus {
                        r#type: ResultStatusType::ResultStatus,
                        request_id: id.clone(),
                    }),
                    current,
                )?
            } else {
                let id = format!(
                    "run-results-{:x}",
                    Sha256::digest(entry.intent.operation_id.as_bytes())
                );
                match results.run_outputs(&id, report.accepted_cells.clone(), current) {
                    Ok(result) => {
                        entry.result_request = Some(id);
                        result
                    }
                    Err(error) if error == "RESULT_QUEUE_BUSY" => {
                        return self.run_progress_reply(
                            entry,
                            None,
                            NativeRunProgressPhase::Running,
                        );
                    }
                    // An exhausted immutable-result lifetime budget cannot undo actual math
                    // receipts or admit another computation. Report honest absent references.
                    Err(error) if error == "RESULT_NOT_AVAILABLE" => {
                        return self.complete_agent_run(entry, current, &outputs);
                    }
                    Err(error) => return Err(error),
                }
            };
            match result.phase {
                NativeResultHostReplyPhase::Queued | NativeResultHostReplyPhase::Running => {
                    return self.run_progress_reply(entry, None, NativeRunProgressPhase::Running);
                }
                NativeResultHostReplyPhase::Completed => outputs = result.payload,
                _ => (), // Known immutable-result availability failure cannot undo main receipts.
            }
        }
        self.complete_agent_run(entry, current, &outputs)
    }
    fn complete_agent_run(
        &mut self,
        entry: &mut PendingRun,
        current: &ReferenceScope,
        outputs: &Value,
    ) -> Result<NativeAgentHostReply, String> {
        let value = run_envelope(entry, current, outputs)?;
        let phase =
            serde_json::from_value(value["status"].clone()).map_err(|_| "INVALID_TOOL_RESULT")?;
        let mut reply = self
            .agent
            .as_mut()
            .ok_or("AGENT_NOT_ACTIVE")?
            .register_effect(
                &entry.intent,
                current,
                entry
                    .descriptor
                    .as_ref()
                    .map(|_| entry.intent.operation_id.clone()),
                value.to_string(),
            )?;
        reply.run_progress = self.run_progress_reply(entry, None, phase)?.run_progress;
        Ok(reply)
    }
    pub(super) fn acknowledge_agent_run(
        &mut self,
        c: NativeAgentAcknowledgeRunChild,
    ) -> Result<NativeAgentHostReply, String> {
        self.agent
            .as_ref()
            .ok_or("AGENT_NOT_ACTIVE")?
            .check_native_scope(&c.scope, &self.binding())?;
        self.live_run_scope(&c.operation_id)?;
        let entry = self
            .runs
            .get_mut(&c.operation_id)
            .ok_or("RUN_NOT_PREPARED")?;
        let encoded = serde_json::to_vec(&c).map_err(|_| "INVALID_ARGUMENT")?;
        let hash = format!("{:x}", Sha256::digest(&encoded));
        if let Some(original) = entry.acks.get(&c.child_operation_id) {
            if original != &hash {
                return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
            }
        } else {
            let a = &c.group_admission;
            let s = &c.group_started;
            let ca = &c.child_admission;
            let cs = &c.child_started;
            if a.event_id != format!("admit-{}", c.operation_id)
                || s.event_id != format!("run-started-{}", c.operation_id)
                || ca.event_id != format!("run-admit-{}", c.child_operation_id)
                || cs.event_id != format!("run-start-{}", c.child_operation_id)
                || a.event_sequence.get() == 0
                || !(a.event_sequence < s.event_sequence
                    && s.event_sequence < ca.event_sequence
                    && ca.event_sequence < cs.event_sequence)
            {
                return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
            }
            let group = serde_json::to_string(&(a, s)).map_err(|_| "INVALID_ARGUMENT")?;
            if entry.group_ack.as_ref().is_some_and(|old| old != &group) {
                return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
            }
            entry
                .executor
                .as_mut()
                .ok_or("RUN_NOT_PREPARED")?
                .acknowledge_admission(&c.child_operation_id)?;
            entry.group_ack = Some(group);
            entry.acks.insert(c.child_operation_id.clone(), hash);
        }
        self.poll_agent_run(NativeAgentPollRun {
            r#type: NativeAgentPollRunType::AgentPollRun,
            scope: c.scope,
            operation_id: c.operation_id,
        })
    }
    pub(super) fn cancel_agent_run(&mut self, id: &str) -> bool {
        let Some(entry) = self.runs.get_mut(id) else {
            return false;
        };
        entry.token.store(true, Ordering::Release);
        if let (Some(executor), Some(kernel)) = (&mut entry.executor, &mut self.kernel) {
            executor.cancel(kernel);
        }
        true
    }
    pub(super) fn cancel_agent_runs(&mut self) {
        for id in self.runs.keys().cloned().collect::<Vec<_>>() {
            self.cancel_agent_run(&id);
        }
    }
    /// Every reserved child stays owned by its group. A generic caller cannot start/replace it.
    pub(super) fn agent_run_physical(
        &mut self,
        command: &NativeKernelHostCommand,
    ) -> Result<Option<NativeKernelHostReply>, String> {
        let id = match command {
            NativeKernelHostCommand::KernelBootstrap(c) => &c.operation_id,
            NativeKernelHostCommand::KernelRunCell(c) => &c.operation_id,
            NativeKernelHostCommand::KernelStatus(c) => &c.operation_id,
            NativeKernelHostCommand::KernelReadBlob(c) => &c.operation_id,
            NativeKernelHostCommand::KernelBarrier(c) => &c.operation_id,
            NativeKernelHostCommand::KernelCompleted(c) => &c.operation_id,
            NativeKernelHostCommand::KernelUnknown(c) => &c.operation_id,
            NativeKernelHostCommand::KernelDiscard(c) => &c.operation_id,
            NativeKernelHostCommand::KernelSettled(c) => &c.operation_id,
            NativeKernelHostCommand::KernelRestoreStart(c) => &c.operation_id,
            NativeKernelHostCommand::KernelRestoreWrite(c) => &c.operation_id,
            NativeKernelHostCommand::KernelRestoreFinish(c) => &c.operation_id,
            NativeKernelHostCommand::KernelState(_) => return Ok(None),
        };
        let group = self.runs.iter().find_map(|(group, entry)| {
            entry
                .descriptor
                .as_ref()
                .is_some_and(|d| d.children.iter().any(|c| &c.operation_id == id))
                .then(|| group.clone())
        });
        let Some(group) = group else {
            if self.runs.contains_key(id) {
                return Err("RUN_OPERATION_REQUIRED".into());
            }
            return Ok(None);
        };
        if matches!(
            command,
            NativeKernelHostCommand::KernelBootstrap(_)
                | NativeKernelHostCommand::KernelRunCell(_)
                | NativeKernelHostCommand::KernelRestoreStart(_)
                | NativeKernelHostCommand::KernelRestoreWrite(_)
                | NativeKernelHostCommand::KernelRestoreFinish(_)
        ) {
            return Err("RUN_OPERATION_REQUIRED".into());
        }
        if matches!(command, NativeKernelHostCommand::KernelStatus(_)) {
            return Ok(None);
        }
        let current = if matches!(command, NativeKernelHostCommand::KernelBarrier(_)) {
            self.live_run_scope(&group)?
        } else {
            self.agent
                .as_ref()
                .map_or(self.current_source_scope(), |a| {
                    a.readonly_scope(self.current_source_scope())
                })
        };
        let context = self.run_context()?;
        let before = self
            .kernel
            .as_ref()
            .and_then(|k| k.accepted_state())
            .map(|s| s.receipt().checkpoint_id.clone());
        let entry = self.runs.get_mut(&group).expect("known");
        let reply = entry
            .executor
            .as_mut()
            .ok_or("RUN_NOT_PREPARED")?
            .physical_command(
                command.clone(),
                &current,
                &context,
                self.kernel.as_mut().ok_or("CONTEXT_NOT_READY")?,
                &mut self.controller,
            )?;
        let kernel = self.kernel.as_ref().expect("checked");
        let accepted = kernel.accepted_state();
        self.scope.definition_revision = accepted
            .as_ref()
            .map_or(0, |s| s.receipt().kernel_state_revision.get());
        if let Some(facts) = kernel.accepted_context() {
            self.coordinator.known_functions = facts.known_functions.clone();
            self.coordinator.owned = facts.owned.clone();
        }
        if accepted.as_ref().map(|s| s.receipt().checkpoint_id.clone()) != before {
            self.pending_result_state = accepted;
            self.flush_result_state();
        }
        Ok(Some(reply))
    }
}

fn diagnostic(cell: &str, code: &str, message: &str, severity: &str) -> Value {
    json!({"code":code,"message":message,"severity":severity,"cell_id":cell,"start_utf8":null,"end_utf8":null})
}
fn bounded_report_bytes(
    d: &NativeRunDescriptor,
    source: &NativeSourceSnapshot,
) -> Result<usize, String> {
    // Account for every real ID and the warning required when older outputs leave retention.
    // At most 32 retained results can instead carry an issued ref and an original diagnostic.
    // Reserve JSON's worst six-byte character escapes for those bounded original messages.
    // This rejects an oversized report before any admission, including a 1000-cell cold run.
    let cells = d.children.iter().filter_map(|c| c.cell_id.0.as_ref())
        .chain(d.blocked_cells.iter().map(|c| &c.cell_id))
        .map(|id| json!({"cell_id":id,
            "source_hash":source.file.cells.iter().find(|c|&c.id==id).map(|c|format!("{:x}",Sha256::digest(c.source.as_bytes()))),
            "status":"cancelled","result_ref":null,
            "diagnostics":[diagnostic(id,"RESULT_NOT_AVAILABLE","Original computation was accepted; its result projection is unavailable","warning")]}))
        .collect::<Vec<_>>();
    let count = cells.len();
    let raw = serde_json::to_vec(&json!({"cells":cells,
        "executed_cell_ids":d.children.iter().filter_map(|c|c.cell_id.0.clone()).collect::<Vec<_>>()}))
        .map_err(|_|"INVALID_ARGUMENT")?.len();
    let original = serde_json::to_vec(&diagnostic(
        "",
        &"\u{1}".repeat(80),
        &"\u{1}".repeat(256),
        "warning",
    ))
    .map_err(|_| "INVALID_ARGUMENT")?
    .len();
    Ok(raw + count.min(32) * (original + 96) + 4096)
}
fn run_envelope(
    entry: &PendingRun,
    current: &ReferenceScope,
    outputs: &Value,
) -> Result<Value, String> {
    let report = entry.executor.as_ref().map(|e| e.report());
    let phase = report.map(|r| r.phase);
    let status = match phase {
        Some(RunPhase::Completed) => "completed",
        Some(RunPhase::Partial) => "partial",
        Some(RunPhase::Cancelled) => "cancelled",
        Some(RunPhase::Unknown) => "unknown",
        _ if entry.token.load(Ordering::Acquire)
            || entry.failure.as_deref() == Some("CANCELLED") =>
        {
            "cancelled"
        }
        _ => "failed",
    };
    let mut cells = Vec::new();
    let mut executed = Vec::new();
    if let (Some(d), Some(source), Some(report)) = (&entry.descriptor, &entry.source, report) {
        for id in d
            .children
            .iter()
            .filter_map(|c| c.cell_id.0.as_ref())
            .chain(d.blocked_cells.iter().map(|c| &c.cell_id))
        {
            // An unresolved active cell is neither claimed accepted nor falsely called skipped.
            if status == "unknown"
                && report.active_operation.as_ref().is_some_and(|op| {
                    d.children
                        .iter()
                        .any(|c| &c.operation_id == op && c.cell_id.0.as_ref() == Some(id))
                })
            {
                continue;
            }
            let receipt = report
                .accepted_cells
                .iter()
                .find(|r| r.producer.cell_id.0.as_ref() == Some(id));
            let failure = report.stopped_cells.iter().find(|r| &r.cell_id == id);
            let state = match receipt {
                Some(r) if r.producer.terminal_status == NativeKernelTerminalStatus::Done => "done",
                Some(_) => "error",
                None if failure.is_some_and(|f| f.code == "CANCELLED") => "cancelled",
                _ => "skipped",
            };
            let mut diagnostics = outputs
                .get(id)
                .and_then(|v| v["diagnostics"].as_array())
                .cloned()
                .unwrap_or_default();
            if let Some(receipt) = receipt {
                if outputs.get(id).is_none() {
                    diagnostics.push(diagnostic(
                        id,
                        "RESULT_NOT_AVAILABLE",
                        "Original computation was accepted; its result projection is unavailable",
                        "warning",
                    ));
                }
                executed.push(id.clone());
                if receipt.producer.terminal_status == NativeKernelTerminalStatus::Error
                    && diagnostics.is_empty()
                {
                    diagnostics.push(diagnostic(
                        id,
                        "EXECUTION_FAILED",
                        "Original accepted cell ended with an evaluation error",
                        "error",
                    ));
                }
            } else if let Some(failure) = failure {
                diagnostics.push(diagnostic(id, &failure.code, &failure.code, "error"));
            }
            let hash = source
                .file
                .cells
                .iter()
                .find(|c| &c.id == id)
                .map(|c| format!("{:x}", Sha256::digest(c.source.as_bytes())))
                .ok_or("INVALID_CELL_REFERENCE")?;
            cells.push(json!({"cell_id":id,"source_hash":hash,"status":state,"result_ref":outputs.get(id).and_then(|v|v.get("result_ref")).cloned().unwrap_or(Value::Null),"diagnostics":diagnostics}));
        }
    }
    let code = if status == "completed" {
        None
    } else {
        Some(match status {
            "unknown" => "UNKNOWN_OUTCOME",
            "cancelled" => "CANCELLED",
            _ => match entry
                .failure
                .as_deref()
                .or_else(|| report.and_then(|r| r.stop_code.as_deref()))
            {
                Some("INVALID_ARGUMENT") => "INVALID_ARGUMENT",
                Some("EDITING_BUSY") => "EDITING_BUSY",
                Some("PERMISSION_DENIED") => "PERMISSION_DENIED",
                Some("INVALID_REFERENCE") => "INVALID_REFERENCE",
                Some("STALE_SNAPSHOT" | "STALE_SOURCE" | "STALE_AGENT_SCOPE") => "STALE_SNAPSHOT",
                Some("LIMIT_EXCEEDED" | "BUDGET_EXCEEDED") => "LIMIT_EXCEEDED",
                Some("CONTEXT_NOT_READY") => "CONTEXT_NOT_READY",
                Some("DEPENDENCY_NOT_READY") => "DEPENDENCY_NOT_READY",
                Some("TIMEOUT") => "TIMEOUT",
                _ => "EXECUTION_FAILED",
            },
        })
    };
    let value = json!({"ok":status=="completed","status":status,"request_ref":entry.intent.operation_id,"operation_ref":entry.descriptor.as_ref().map(|_|&entry.intent.operation_id),
        "document":{"id":current.document,"generation":current.generation,"revision":current.revision},
        "data":entry.descriptor.as_ref().map(|_|json!({"run_ref":entry.intent.operation_id,"run_outcome":if status=="unknown" {if executed.is_empty(){"failed"}else{"partial"}}else{status},"cells":cells,"executed_cell_ids":executed})),
        "error":code.map(|code|json!({"code":code,"message":code,"recovery":if code=="UNKNOWN_OUTCOME"{"query_operation"}else if code=="LIMIT_EXCEEDED"{"reduce_request"}else{"none"},"details":{"native_stop":entry.failure.as_ref().or_else(||report.and_then(|r|r.stop_code.as_ref())),"unresolved_operation":report.and_then(|r|r.active_operation.as_ref())}})),
        "pagination":{"truncated":false,"next_cursor":null}});
    crate::tools::checked_data("run_cells_result", value).map_err(|_| "TOOL_RESULT_LIMIT".into())
}
