//! Real main CAS/immutable states/acceptance. Matching IO receipts in this Rust test are
//! synthetic: durable group admission, physical SQLite and Agent/Pi wiring remain separate.
use om_host_service::{
    agent::ingress::ToolIntent,
    document::{SourceDocument, commit::DocumentCommitController, coordinator::SourceCoordinator},
    kernel::{
        acceptance::AcceptanceContext,
        endpoint::{KERNEL_BUILD_ID, KernelEndpoint},
    },
    protocol::{Serial, generated::*},
    references::ReferenceScope,
    tools::{
        read::ReadNotebookService,
        run::{self, RunExecutor, RunPhase, RunSource, RunUpdate},
    },
};
use om_kernel::{
    checkpoint::{CheckpointLimits, CheckpointRestore},
    config::GeneralConfig,
};
use om_num::ctx::Interrupt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
fn n(n: u64) -> Serial {
    Serial::new(n).unwrap()
}
struct Fixture {
    source: DocumentCommitController,
    scope: ReferenceScope,
    ctx: AcceptanceContext,
    kernel: KernelEndpoint,
    reads: ReadNotebookService,
    originals: std::collections::BTreeMap<String, String>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for thread in self.kernel.take_join_handles() {
            thread.join().unwrap();
        }
    }
}
impl Fixture {
    fn new(cells: &[(&str, &str, Dialect)]) -> Self {
        let owner = SourceDocument::new(
            "run-doc".into(),
            n(1),
            NativeSourceFile {
                version: 1,
                title: "actual explicit execution".into(),
                cells: cells
                    .iter()
                    .map(|(id, source, dialect)| NativeSourceCell {
                        id: (*id).into(),
                        source: (*source).into(),
                        kind: NativeCellKind::Math,
                        dialect: dialect.clone(),
                    })
                    .collect(),
            },
        )
        .unwrap();
        let scope = ReferenceScope {
            runtime: "runtime".into(),
            document: "run-doc".into(),
            generation: 1,
            revision: 0,
            execution_epoch: 0,
            snapshot_hash: owner.snapshot().snapshot_hash.clone(),
            task: Some("task".into()),
            task_generation: 1,
            grant_revision: 1,
            config_revision: 0,
            definition_revision: 0,
            metadata_revision: 26,
            editor_state_hash: String::new(),
            execution_mode: true,
            can_read: true,
            can_preview: false,
            can_write: false,
        };
        let ctx = AcceptanceContext {
            runtime_instance_id: scope.runtime.clone(),
            store_id: "store".into(),
            document_generation: n(1),
            source: owner.snapshot().clone(),
            general: GeneralConfig {
                reactive: false,
                ..Default::default()
            },
            config_revision: n(0),
            build: KERNEL_BUILD_ID.into(),
        };
        Self {
            source: DocumentCommitController::new(owner, scope.runtime.clone(), [7; 32]),
            scope,
            ctx,
            kernel: KernelEndpoint::new().unwrap(),
            reads: ReadNotebookService::new([8; 32]),
            originals: Default::default(),
        }
    }
    fn chain() -> Self {
        Self::new(&[
            ("answer", "b+1", Dialect::Modern),
            ("unrelated", "seed_random(99);rand()", Dialect::Modern),
            ("b", "let b=a+1;b", Dialect::Modern),
            ("a", "let a=2;a", Dialect::Modern),
        ])
    }
    fn refresh(&mut self) {
        self.scope.definition_revision = self
            .kernel
            .accepted_state()
            .map_or(0, |a| a.receipt().kernel_state_revision.get());
    }
    fn freeze(
        &mut self,
        operation: &str,
        targets: &[&str],
        scope: &str,
        keep_going: bool,
    ) -> RunExecutor {
        self.refresh();
        let accepted = self.kernel.accepted_state();
        let reference = self
            .reads
            .read(
                b"{}",
                &mut self.source,
                &SourceCoordinator::default(),
                accepted.as_ref(),
                &self.scope,
                5,
            )
            .unwrap()
            .data["snapshot_ref"]
            .as_str()
            .unwrap()
            .to_owned();
        let new_args = json!({"snapshot_ref":reference,"cell_ids":targets,"scope":scope,"continue_on_error":keep_going}).to_string();
        let args = self
            .originals
            .entry(operation.into())
            .or_insert(new_args)
            .clone();
        let intent = ToolIntent {
            operation_id: operation.into(),
            turn_id: "turn".into(),
            call_id: operation.into(),
            proposal_event: "acked-proposal".into(),
            name: AgentToolName::RunCells,
            arguments_hash: format!("{:x}", Sha256::digest(args.as_bytes())),
        };
        let input = run::prepare(
            &intent,
            args.as_bytes(),
            RunSource {
                reads: &mut self.reads,
                scope: &self.scope,
                general: self.ctx.general.clone(),
                accepted: accepted.as_ref(),
                allowed: self
                    .ctx
                    .source
                    .file
                    .cells
                    .iter()
                    .map(|c| c.id.clone())
                    .collect(),
                now: 6,
            },
        )
        .unwrap();
        std::thread::spawn(move || input.plan(&Interrupt::default()))
            .join()
            .unwrap()
            .unwrap()
            .into_executor()
    }
    fn poll(&mut self, run: &mut RunExecutor) -> RunUpdate {
        self.refresh();
        run.poll(&self.scope, &self.ctx, &mut self.kernel, &mut self.source)
            .unwrap()
    }
    fn physical(
        &mut self,
        run: &mut RunExecutor,
        command: NativeKernelHostCommand,
    ) -> NativeKernelHostReply {
        run.physical_command(
            command,
            &self.scope,
            &self.ctx,
            &mut self.kernel,
            &mut self.source,
        )
        .unwrap()
    }
    fn candidate(&mut self, run: &mut RunExecutor) -> NativeKernelCommit {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let next = self.poll(run);
            if next.report.phase == RunPhase::AwaitingAcceptance {
                return next.child.unwrap().plan.0.unwrap();
            }
            assert_eq!(
                next.report.phase,
                RunPhase::Running,
                "{}",
                serde_json::to_string(&next.report).unwrap()
            );
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    fn barrier(&mut self, run: &mut RunExecutor, p: &NativeKernelCommit) {
        let mut bytes = Vec::new();
        while (bytes.len() as u64) < p.checkpoint_byte_length.get() {
            let remaining = p.checkpoint_byte_length.get() - bytes.len() as u64;
            let chunk = self.physical(
                run,
                NativeKernelHostCommand::KernelReadBlob(KernelReadBlob {
                    r#type: KernelReadBlobType::KernelReadBlob,
                    operation_id: p.operation_id.clone(),
                    offset: n(bytes.len() as u64),
                    count: remaining.min(65536) as u32,
                }),
            );
            bytes.extend(chunk.blob_bytes.iter().map(|v| *v as u8));
        }
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            p.checkpoint_blob_hash
        );
        let reply = self.physical(
            run,
            NativeKernelHostCommand::KernelBarrier(KernelBarrier {
                r#type: KernelBarrierType::KernelBarrier,
                operation_id: p.operation_id.clone(),
            }),
        );
        assert_eq!(reply.phase, NativeKernelHostReplyPhase::Committing);
    }
    fn complete(&mut self, run: &mut RunExecutor, p: &NativeKernelCommit) {
        let receipt = NativeKernelReceipt {
            protocol_version: 1,
            store_id: p.store_id.clone(),
            document_id: p.producer.document_id.clone(),
            operation_id: p.operation_id.clone(),
            request_hash: p.request_hash.clone(),
            checkpoint_id: p.checkpoint_id.clone(),
            checkpoint_blob_hash: p.checkpoint_blob_hash.clone(),
            checkpoint_byte_length: p.checkpoint_byte_length,
            codec_version: 1,
            producer: p.producer.clone(),
            accepted_source_revision: p.acceptance_source.revision,
            accepted_snapshot_hash: p.acceptance_source.snapshot_hash.clone(),
            kernel_state_revision: p.producer.kernel_state_revision,
            result_id: p.result_id.clone(),
            outbox_event_id: p.outbox_event_id.clone(),
            committed_at: "2026-10-15T00:00:00.000Z".into(),
        };
        assert_eq!(
            self.physical(
                run,
                NativeKernelHostCommand::KernelCompleted(Box::new(KernelCompleted {
                    r#type: KernelCompletedType::KernelCompleted,
                    operation_id: p.operation_id.clone(),
                    receipt,
                }))
            )
            .phase,
            NativeKernelHostReplyPhase::Completed
        );
    }
    fn commit(&mut self, run: &mut RunExecutor, p: &NativeKernelCommit) {
        self.barrier(run, p);
        self.complete(run, p);
    }
    fn finish(&mut self, run: &mut RunExecutor) -> RunPhase {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let next = self.poll(run);
            match next.report.phase {
                RunPhase::AwaitingAcceptance => {
                    self.commit(run, &next.child.unwrap().plan.0.unwrap())
                }
                RunPhase::Running => std::thread::yield_now(),
                other => {
                    println!(
                        "actual main run={}",
                        serde_json::to_string(&next.report).unwrap()
                    );
                    return other;
                }
            }
            assert!(Instant::now() < deadline);
        }
    }
    fn outputs(&self) -> Value {
        let a = self.kernel.accepted_state().unwrap();
        let p = a.state();
        let work = p
            .restore(
                CheckpointRestore {
                    binding: p.binding(),
                    source: p.source(),
                    general: p.general(),
                    clock: None,
                    cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                },
                CheckpointLimits::default(),
                &Interrupt::default(),
            )
            .unwrap();
        let session = work.into_session();
        session
            .notebook
            .cells
            .into_iter()
            .map(|c| (c.id, json!({"status":c.status,"output":c.output})))
            .collect()
    }
}
fn ids(run: &RunExecutor) -> Vec<&str> {
    run.report()
        .accepted_cells
        .iter()
        .map(|r| r.producer.cell_id.0.as_deref().unwrap())
        .collect()
}
#[test]
fn required_executes_actual_topological_cells_with_distinct_original_receipts_and_no_runall() {
    let mut f = Fixture::chain();
    let mut run = f.freeze("required-original", &["answer"], "required", false);
    assert_eq!(run.plan().ordered_cell_ids, ["a", "b", "answer"]);
    let seed = run.child_operation(0);
    assert_eq!(seed, run.child_operation(0));
    assert_eq!(f.finish(&mut run), RunPhase::Completed);
    assert_eq!(ids(&run), ["a", "b", "answer"]);
    assert_eq!(
        run.report()
            .bootstrap_receipt
            .as_ref()
            .unwrap()
            .operation_id,
        seed
    );
    for (index, receipt) in run.report().accepted_cells.iter().enumerate() {
        assert_eq!(receipt.operation_id, run.child_operation(index + 1));
        assert_eq!(receipt.kernel_state_revision.get(), index as u64 + 1);
    }
    let output = f.outputs();
    assert_eq!(output["answer"]["output"]["items"][0]["input_form"], "4");
    assert_eq!(output["unrelated"]["output"], Value::Null);
    let final_head = f
        .kernel
        .accepted_state()
        .unwrap()
        .receipt()
        .checkpoint_id
        .clone();
    assert_eq!(f.poll(&mut run).report.phase, RunPhase::Completed);
    assert_eq!(
        f.kernel.accepted_state().unwrap().receipt().checkpoint_id,
        final_head
    );
}
#[test]
fn selected_requires_ready_dependencies_even_when_reactive_is_off_and_continues_independent_branch()
{
    let mut f = Fixture::chain();
    let mut run = f.freeze("selected", &["answer", "unrelated"], "selected", true);
    assert_eq!(run.plan().unavailable_prerequisite_cell_ids, ["b"]);
    assert_eq!(f.finish(&mut run), RunPhase::Partial);
    assert_eq!(ids(&run), ["unrelated"]);
    assert!(
        run.report()
            .stopped_cells
            .iter()
            .any(|c| c.cell_id == "answer" && c.code == "DEPENDENCY_NOT_READY")
    );
    let output = f.outputs();
    assert_eq!(output["answer"]["output"], Value::Null);
    assert_eq!(output["a"]["output"], Value::Null);
}
#[test]
fn affected_reruns_real_descendants_and_a_new_model_call_gets_new_original_children() {
    let mut f = Fixture::chain();
    let mut first = f.freeze("original-one", &["answer"], "required", false);
    assert_eq!(f.finish(&mut first), RunPhase::Completed);
    let mut next = f.freeze("original-two", &["a"], "affected", false);
    assert_eq!(next.plan().ordered_cell_ids, ["a", "b", "answer"]);
    assert_ne!(first.child_operation(1), next.child_operation(1));
    assert_eq!(f.finish(&mut next), RunPhase::Completed);
    assert_eq!(ids(&next), ["a", "b", "answer"]);
    assert!(next.report().bootstrap_receipt.is_none());
    assert_eq!(
        next.report()
            .accepted_cells
            .last()
            .unwrap()
            .kernel_state_revision
            .get(),
        6
    );
}
#[test]
fn accepted_error_keeps_actual_statement_effects_but_blocks_its_dependents_and_obeys_continue_policy()
 {
    for keep_going in [false, true] {
        let mut f = Fixture::new(&[
            ("bad", "a=99;\nloop:=loop\nloop\nnever=5", Dialect::Wolfram),
            ("dependent", "a+1", Dialect::Wolfram),
            ("independent", "8+8", Dialect::Modern),
        ]);
        let mut run = f.freeze(
            "partial",
            &["bad", "dependent", "independent"],
            "required",
            keep_going,
        );
        assert_eq!(f.finish(&mut run), RunPhase::Partial);
        assert_eq!(
            ids(&run),
            if keep_going {
                vec!["bad", "independent"]
            } else {
                vec!["bad"]
            }
        );
        assert_eq!(
            run.report().accepted_cells[0].producer.terminal_status,
            NativeKernelTerminalStatus::Error
        );
        assert_eq!(
            run.report().accepted_cells[0]
                .producer
                .successful_statements
                .get(),
            2
        );
        if keep_going {
            assert!(
                run.report()
                    .stopped_cells
                    .iter()
                    .any(|c| c.cell_id == "dependent" && c.code == "DEPENDENCY_NOT_READY")
            );
        }
        let mut inspect = f.freeze("read-partial-def", &["independent"], "selected", false);
        // Read/plan must not erase the genuinely accepted partial statement prefix.
        let before = f.kernel.accepted_state().unwrap().state().bytes().to_vec();
        inspect.cancel(&mut f.kernel);
        assert_eq!(f.finish(&mut inspect), RunPhase::Cancelled);
        assert_eq!(f.kernel.accepted_state().unwrap().state().bytes(), before);
    }
}
#[test]
fn unknown_stops_following_admission_and_only_original_receipt_settlement_preserves_prefix() {
    let mut f = Fixture::chain();
    let mut run = f.freeze("unknown", &["answer"], "required", true);
    let seed = f.candidate(&mut run);
    f.commit(&mut run, &seed);
    let child = f.candidate(&mut run);
    f.barrier(&mut run, &child);
    f.physical(
        &mut run,
        NativeKernelHostCommand::KernelUnknown(KernelUnknown {
            r#type: KernelUnknownType::KernelUnknown,
            operation_id: child.operation_id.clone(),
        }),
    );
    for _ in 0..8 {
        let current = f.poll(&mut run);
        assert_eq!(current.report.phase, RunPhase::Unknown);
        assert_eq!(
            current.report.active_operation.as_deref(),
            Some(child.operation_id.as_str())
        );
    }
    assert!(!f.kernel.has_operation(&run.child_operation(2)));
    f.complete(&mut run, &child);
    assert_eq!(f.finish(&mut run), RunPhase::Partial);
    assert_eq!(ids(&run), ["a"]);
    assert_eq!(
        run.report().stop_code.as_deref(),
        Some("STOPPED_AFTER_UNKNOWN")
    );
    assert!(!f.kernel.has_operation(&run.child_operation(2)));
}
#[test]
fn stop_before_barrier_discards_candidate_but_stop_after_physical_commit_keeps_real_receipt() {
    for committed in [false, true] {
        let mut f = Fixture::chain();
        let mut run = f.freeze("stop", &["answer"], "required", true);
        let seed = f.candidate(&mut run);
        f.commit(&mut run, &seed);
        let child = f.candidate(&mut run);
        if committed {
            f.barrier(&mut run, &child);
        }
        run.cancel(&mut f.kernel);
        if committed {
            f.complete(&mut run, &child);
        }
        assert_eq!(f.finish(&mut run), RunPhase::Cancelled);
        assert_eq!(
            ids(&run),
            if committed {
                vec!["a"]
            } else {
                Vec::<&str>::new()
            }
        );
        assert!(!f.kernel.has_operation(&run.child_operation(2)));
    }
}
#[test]
fn final_barrier_rechecks_task_grants_drafts_source_config_and_actual_parent() {
    for change in [
        "task",
        "grants",
        "editor",
        "mode",
        "source",
        "config",
        "definition",
    ] {
        let mut f = Fixture::chain();
        let mut run = f.freeze("scope", &["answer"], "required", false);
        let seed = f.candidate(&mut run);
        f.commit(&mut run, &seed);
        let child = f.candidate(&mut run);
        match change {
            "task" => f.scope.task_generation += 1,
            "grants" => f.scope.grant_revision += 1,
            "editor" => f.scope.editor_state_hash = "a".repeat(64),
            "mode" => f.scope.execution_mode = false,
            "source" => f.ctx.source.snapshot_hash = "a".repeat(64),
            "config" => f.ctx.config_revision = n(1),
            "definition" => f.scope.definition_revision += 1,
            _ => unreachable!(),
        }
        let result = run.physical_command(
            NativeKernelHostCommand::KernelBarrier(KernelBarrier {
                r#type: KernelBarrierType::KernelBarrier,
                operation_id: child.operation_id.clone(),
            }),
            &f.scope,
            &f.ctx,
            &mut f.kernel,
            &mut f.source,
        );
        assert!(result.is_err(), "{change}");
        run.cancel(&mut f.kernel);
        assert_eq!(f.finish(&mut run), RunPhase::Cancelled);
        assert!(run.report().accepted_cells.is_empty());
    }
}
#[test]
fn duplicate_original_group_cannot_reclaim_bootstrap_or_fabricate_alternate_child_effects() {
    let mut f = Fixture::chain();
    let mut original = f.freeze("same-original", &["answer"], "required", false);
    let mut duplicate = f.freeze("same-original", &["answer"], "required", false);
    assert_eq!(original.child_operation(0), duplicate.child_operation(0));
    let candidate = f.candidate(&mut original);
    let wrong = original.physical_command(
        NativeKernelHostCommand::KernelBarrier(KernelBarrier {
            r#type: KernelBarrierType::KernelBarrier,
            operation_id: "caller-invented".into(),
        }),
        &f.scope,
        &f.ctx,
        &mut f.kernel,
        &mut f.source,
    );
    assert_eq!(wrong.unwrap_err(), "IPC_CALL_SCOPE_MISMATCH");
    let other = f.poll(&mut duplicate);
    assert_eq!(other.report.phase, RunPhase::Failed);
    assert_eq!(
        other.report.stop_code.as_deref(),
        Some("ORIGINAL_OPERATION_REQUIRES_RECONCILIATION")
    );
    assert!(other.report.accepted_cells.is_empty());
    f.commit(&mut original, &candidate);
    assert_eq!(f.finish(&mut original), RunPhase::Completed);
}
