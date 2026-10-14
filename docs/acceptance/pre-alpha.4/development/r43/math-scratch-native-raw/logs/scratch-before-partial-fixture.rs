//! Real current read tokens/KernelWorker/isolated math. Matching unit receipts are synthetic IO.
use om_host_service::{
    agent::ingress::ToolIntent,
    document::{SourceDocument, commit::DocumentCommitController, coordinator::SourceCoordinator},
    kernel::{
        acceptance::{AcceptanceContext, AcceptedKernelState, FrozenKernelPlan},
        worker::{KernelJob, KernelWorker},
    },
    protocol::{Serial, generated::*},
    references::ReferenceScope,
    results::runtime::{ResultRuntime, ScratchWorkerProbes, decode_command},
    tools::{
        read::ReadNotebookService,
        results::FrozenInspection,
        scratch::{self, FrozenScratch, ScratchResultStore, ScratchSource},
    },
};
use om_kernel::{
    checkpoint::CheckpointLimits, config::GeneralConfig, retained_results::ResultSourceFormat,
};
use om_num::ctx::{Clock, Interrupt};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    sync::{Arc, Condvar, Mutex, atomic::AtomicBool},
    time::{Duration, Instant},
};
struct HostClock(Instant);
impl Clock for HostClock {
    fn now_ms(&self) -> f64 {
        self.0.elapsed().as_secs_f64() * 1000.
    }
}
fn budget() -> Interrupt {
    Interrupt {
        clock: Some(Arc::new(HostClock(Instant::now()))),
        deadline_ms: Some(5000.),
        steps_left: std::cell::Cell::new(4 * 1024 * 1024),
        ..Default::default()
    }
}
fn serial(n: u64) -> Serial {
    Serial::new(n).unwrap()
}
fn receipt(p: &NativeKernelCommit) -> NativeKernelReceipt {
    NativeKernelReceipt {
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
    }
}
fn fixture() -> (
    DocumentCommitController,
    ReferenceScope,
    AcceptedKernelState,
    ReadNotebookService,
) {
    let owner = SourceDocument::new(
        "scratch-doc".into(),
        serial(1),
        NativeSourceFile {
            version: 1,
            title: "independent scratch".into(),
            cells: vec![
                NativeSourceCell {
                    id: "a".into(),
                    kind: NativeCellKind::Math,
                    source: "let a=2;seed_random(42);a".into(),
                    dialect: Dialect::Modern,
                },
                NativeSourceCell {
                    id: "pending".into(),
                    kind: NativeCellKind::Math,
                    source: "let pending=7;pending".into(),
                    dialect: Dialect::Modern,
                },
            ],
        },
    )
    .unwrap();
    let ctx = AcceptanceContext {
        runtime_instance_id: "runtime".into(),
        store_id: "store".into(),
        document_generation: serial(1),
        source: owner.snapshot().clone(),
        general: GeneralConfig::default(),
        config_revision: serial(0),
        build: "scratch-unit".into(),
    };
    let worker = KernelWorker::new(64 * 1024 * 1024, 8, CheckpointLimits::default()).unwrap();
    let seed = FrozenKernelPlan::bootstrap(
        &ctx,
        "seed",
        &worker,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    seed.enter_commit(&ctx, None).unwrap();
    let parent = seed.accept_receipt(&receipt(seed.plan())).unwrap();
    let candidate = worker
        .submit(KernelJob {
            runtime_instance_id: "runtime".into(),
            document_generation: serial(1),
            operation_id: "actual-a".into(),
            parent_checkpoint_ref: parent.registry_ref().into(),
            source: ctx.source.clone(),
            general: ctx.general.clone(),
            config_revision: serial(0),
            cell_id: "a".into(),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .unwrap()
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let plan = FrozenKernelPlan::candidate(candidate, &parent, &ctx, &worker).unwrap();
    plan.enter_commit(&ctx, Some(&parent)).unwrap();
    let accepted = plan.accept_receipt(&receipt(plan.plan())).unwrap();
    worker.begin_close().unwrap().unwrap().join().unwrap();
    let scope = ReferenceScope {
        runtime: "runtime".into(),
        document: ctx.source.document_id,
        generation: 1,
        revision: 0,
        execution_epoch: 0,
        snapshot_hash: ctx.source.snapshot_hash,
        task: Some("task".into()),
        task_generation: 1,
        grant_revision: 1,
        config_revision: 0,
        definition_revision: 1,
        metadata_revision: 26,
        editor_state_hash: String::new(),
        execution_mode: false,
        can_read: true,
        can_preview: false,
        can_write: false,
    };
    (
        DocumentCommitController::new(owner, "runtime".into(), [8; 32]),
        scope,
        accepted,
        ReadNotebookService::new([9; 32]),
    )
}
fn read(
    c: &mut DocumentCommitController,
    r: &mut ReadNotebookService,
    s: &ReferenceScope,
    a: Option<&AcceptedKernelState>,
) -> String {
    r.read(b"{}", c, &SourceCoordinator::default(), a, s, 5)
        .unwrap()
        .data["snapshot_ref"]
        .as_str()
        .unwrap()
        .into()
}
fn intent(id: &str, tool: AgentToolName, raw: &str) -> ToolIntent {
    ToolIntent {
        operation_id: id.into(),
        turn_id: "turn".into(),
        call_id: id.into(),
        proposal_event: "acked-unit-proposal".into(),
        name: tool,
        arguments_hash: format!("{:x}", Sha256::digest(raw.as_bytes())),
    }
}
fn frozen(
    id: &str,
    args: Value,
    reads: &mut ReadNotebookService,
    s: &ReferenceScope,
    a: Option<&AcceptedKernelState>,
) -> Result<FrozenScratch, String> {
    let raw = args.to_string();
    scratch::prepare(
        &intent(id, AgentToolName::EvaluateScratch, &raw),
        &raw,
        ScratchSource {
            reads,
            scope: s,
            general: GeneralConfig::default(),
            accepted: a,
            now: 6,
        },
    )
}
fn execute(
    id: &str,
    source: &str,
    reference: &str,
    reads: &mut ReadNotebookService,
    s: &ReferenceScope,
    a: &AcceptedKernelState,
) -> scratch::ScratchEvaluation {
    let f = frozen(
        id,
        json!({"snapshot_ref":reference,"source":source,"dialect":"Wolfram"}),
        reads,
        s,
        Some(a),
    )
    .unwrap();
    std::thread::spawn(move || f.evaluate(&budget()))
        .join()
        .unwrap()
        .unwrap()
}
fn complete(
    runtime: &ResultRuntime,
    mut reply: NativeResultHostReply,
    s: &ReferenceScope,
) -> NativeResultHostReply {
    let end = Instant::now() + Duration::from_secs(10);
    while matches!(
        reply.phase,
        NativeResultHostReplyPhase::Queued | NativeResultHostReplyPhase::Running
    ) {
        assert!(Instant::now() < end);
        std::thread::yield_now();
        reply = runtime
            .command(
                NativeResultHostCommand::ResultStatus(ResultStatus {
                    r#type: ResultStatusType::ResultStatus,
                    request_id: reply.request_id,
                }),
                s,
            )
            .unwrap();
    }
    reply
}
#[test]
fn local_set_out_and_random_are_independent_with_explicit_scratch_origin_and_real_parent() {
    let (mut c, s, a, mut reads) = fixture();
    let reference = read(&mut c, &mut reads, &s, Some(&a));
    let before = a.state().bytes().to_vec();
    let mut store = ScratchResultStore::new([11; 32]);
    let mut values = vec![];
    for id in ["local-one", "local-two"] {
        let packet = execute(
            id,
            "a=99; {a,Out[1],RandomUniform[]}",
            &reference,
            &mut reads,
            &s,
            &a,
        );
        let token = store.register(packet, &s, 10, &budget()).unwrap();
        let resolved = store.resolve(&token, &s, 11).unwrap();
        let binding = resolved.binding();
        assert_eq!(binding["origin"], "scratch");
        assert_eq!(binding["acceptance"], "isolated");
        assert_eq!(
            binding["parent_checkpoint"]["checkpoint_id"],
            a.receipt().checkpoint_id
        );
        assert!(binding.get("checkpoint_id").is_none());
        let value = resolved
            .source(&[], ResultSourceFormat::InputForm, &budget())
            .unwrap();
        assert_eq!(
            resolved
                .source(&[0], ResultSourceFormat::InputForm, &budget())
                .unwrap(),
            "99"
        );
        assert_eq!(
            resolved
                .source(&[1], ResultSourceFormat::InputForm, &budget())
                .unwrap(),
            "2"
        );
        values.push(value);
        let result = store.result(&token, &s, 11).unwrap();
        assert_eq!(result["data"]["notebook_modified"], false);
        assert_eq!(result["data"]["evaluation_status"], "done");
    }
    assert_eq!(values[0], values[1]);
    assert_eq!(a.state().bytes(), before);
    println!(
        "actual isolated Set/Out/random={} / original checkpoint {} bytes unchanged",
        values[0],
        before.len()
    );
}
#[test]
fn definitions_require_actual_ready_cells_and_false_mode_has_no_main_bindings() {
    let (mut c, s, a, mut reads) = fixture();
    let reference = read(&mut c, &mut reads, &s, Some(&a));
    let f = frozen(
        "needs-unexecuted",
        json!({"snapshot_ref":reference,"source":"pending+1","dialect":"Modern"}),
        &mut reads,
        &s,
        Some(&a),
    )
    .unwrap();
    assert_eq!(f.evaluate(&budget()).err().unwrap(), "DEPENDENCY_NOT_READY");
    let mut store = ScratchResultStore::new([12; 32]);
    for (id, code, definitions, expected) in [
        ("ready", "a+1", true, "3"),
        ("constant", "2+2", true, "4"),
        ("fresh", "a", false, "a"),
    ] {
        let f=frozen(id,json!({"snapshot_ref":reference,"source":code,"dialect":"Modern","use_notebook_definitions":definitions}),&mut reads,&s,Some(&a)).unwrap();
        let packet = f.evaluate(&budget()).unwrap();
        let token = store.register(packet, &s, 10, &budget()).unwrap();
        let r = store.resolve(&token, &s, 11).unwrap();
        assert_eq!(
            r.source(&[], ResultSourceFormat::InputForm, &budget())
                .unwrap(),
            expected
        );
        if !definitions {
            assert_eq!(r.binding()["parent_checkpoint"], Value::Null);
        }
    }
    assert_eq!(
        frozen(
            "missing-parent",
            json!({"snapshot_ref":reference,"source":"a","dialect":"Modern"}),
            &mut reads,
            &s,
            None
        )
        .err()
        .unwrap(),
        "CONTEXT_NOT_READY"
    );
}
#[test]
fn actual_source_change_retires_inherited_definitions_but_title_and_unrelated_work_are_preserved() {
    let (mut c, mut s, a, mut reads) = fixture();
    let before = a.state().bytes().to_vec();
    for (index, operation, ready) in [
        (
            0,
            NativeSourceOperation::RenameSourceNotebook(RenameSourceNotebook {
                kind: RenameSourceNotebookKind::RenameNotebook,
                title: "title-only".into(),
            }),
            true,
        ),
        (
            1,
            NativeSourceOperation::UpdateSourceCell(UpdateSourceCell {
                kind: UpdateSourceCellKind::UpdateCell,
                cell: NativeSourceCell {
                    id: "a".into(),
                    kind: NativeCellKind::Math,
                    source: "let a=9;a".into(),
                    dialect: Dialect::Modern,
                },
            }),
            false,
        ),
    ] {
        let mutation = SourceCoordinator::default()
            .prepare(
                c.owner(),
                &[operation],
                format!("unit-source-{index}"),
                format!("unit-tx-{index}"),
                format!("unit-event-{index}"),
                "unit-time".into(),
            )
            .unwrap();
        let head = mutation.commit.after;
        s.revision = head.revision.get();
        s.execution_epoch = head.execution_epoch.get();
        s.snapshot_hash = head.snapshot_hash.clone();
        c = DocumentCommitController::new(
            SourceDocument::restore(head, serial(1)).unwrap(),
            "runtime".into(),
            [21; 32],
        );
        let reference = read(&mut c, &mut reads, &s, Some(&a));
        let f = frozen(
            "actual-source-case",
            json!({"snapshot_ref":reference,"source":"a+1","dialect":"Modern"}),
            &mut reads,
            &s,
            Some(&a),
        )
        .unwrap();
        if ready {
            assert!(f.evaluate(&budget()).is_ok());
        } else {
            assert_eq!(f.evaluate(&budget()).err().unwrap(), "DEPENDENCY_NOT_READY");
        }
        assert!(
            frozen(
                "unrelated-constant",
                json!({"snapshot_ref":reference,"source":"2+2","dialect":"Modern"}),
                &mut reads,
                &s,
                Some(&a)
            )
            .unwrap()
            .evaluate(&budget())
            .is_ok()
        );
    }
    assert_eq!(a.state().bytes(), before);
}
#[test]
fn original_hash_closed_arguments_current_scope_and_direct_budgets_are_enforced() {
    let (mut c, s, a, mut reads) = fixture();
    let reference = read(&mut c, &mut reads, &s, Some(&a));
    for args in [
        json!({"snapshot_ref":reference,"source":"2+2","dialect":"Modern","use_notebook_definitions":null}),
        json!({"snapshot_ref":reference,"source":"2+2","dialect":"Modern","grants":["all"]}),
        json!({"snapshot_ref":reference,"source":"2+2","dialect":"bad"}),
    ] {
        assert_eq!(
            frozen("bad", args, &mut reads, &s, Some(&a)).err().unwrap(),
            "INVALID_ARGUMENT"
        );
    }
    let raw = json!({"snapshot_ref":reference,"source":"2+2","dialect":"Modern"}).to_string();
    let mut wrong = intent("id", AgentToolName::EvaluateScratch, &raw);
    wrong.arguments_hash = "a".repeat(64);
    assert!(
        scratch::prepare(
            &wrong,
            &raw,
            ScratchSource {
                reads: &mut reads,
                scope: &s,
                general: GeneralConfig::default(),
                accepted: Some(&a),
                now: 6
            }
        )
        .is_err()
    );
    for index in 0..4 {
        let mut changed = s.clone();
        match index {
            0 => changed.task = Some("other-task".into()),
            1 => changed.definition_revision += 1,
            2 => changed.editor_state_hash = "a".repeat(64),
            _ => changed.grant_revision += 1,
        }
        assert!(
            frozen(
                "stale",
                serde_json::from_str(&raw).unwrap(),
                &mut reads,
                &changed,
                Some(&a)
            )
            .is_err()
        );
    }
    // Refresh the real token issuer after the authority-invalidating reads above.
    let reference = read(&mut c, &mut reads, &s, Some(&a));
    for (id, ctx, expected) in [
        (
            "cancel",
            Interrupt {
                flag: Arc::new(AtomicBool::new(true)),
                ..budget()
            },
            "CANCELLED",
        ),
        (
            "budget",
            Interrupt {
                steps_left: std::cell::Cell::new(2),
                ..budget()
            },
            "LIMIT_EXCEEDED",
        ),
        (
            "deadline",
            Interrupt {
                deadline_ms: Some(-1.),
                ..budget()
            },
            "TIMEOUT",
        ),
    ] {
        let f=frozen(id,json!({"snapshot_ref":reference,"source":"map(fn(k)=>sin(k),range(1,10000))","dialect":"Modern"}),&mut reads,&s,Some(&a)).unwrap();
        assert_eq!(f.evaluate(&ctx).err().unwrap(), expected);
    }
}
#[test]
fn real_error_prefix_is_retained_without_main_effect_and_refs_expire_and_keep_origin() {
    let (mut c, s, a, mut reads) = fixture();
    let reference = read(&mut c, &mut reads, &s, Some(&a));
    let before = a.state().bytes().to_vec();
    let mut store = ScratchResultStore::new([13; 32]);
    let f=frozen("actual-partial",json!({"snapshot_ref":reference,"source":"let a=99; 1/3; loop:=loop; loop","dialect":"Modern"}),&mut reads,&s,Some(&a)).unwrap();
    let packet = f.evaluate(&budget()).unwrap();
    let token = store.register(packet, &s, 10, &budget()).unwrap();
    let result = store.result(&token, &s, 11).unwrap();
    assert_eq!(result["status"], "failed");
    assert_eq!(result["data"]["evaluation_status"], "error");
    assert_eq!(result["data"]["notebook_modified"], false);
    assert!(!result["data"]["diagnostics"].as_array().unwrap().is_empty());
    assert!(
        !result["data"]["output_summary"]["summary"]["statements"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let resolved = store.resolve(&token, &s, 11).unwrap();
    assert_eq!(resolved.binding()["acceptance"], "isolated");
    let mut later = s.clone();
    later.revision += 1;
    later.snapshot_hash = "a".repeat(64);
    assert_eq!(
        store.resolve(&token, &later, 12).unwrap().binding()["is_current"],
        false
    );
    let mut changed = s.clone();
    changed.task_generation += 1;
    assert!(store.resolve(&token, &changed, 12).is_err());
    assert!(store.resolve(&token, &s, 600010).is_err());
    assert_eq!(a.state().bytes(), before);
}
#[test]
fn paused_scratch_does_not_block_result_inspection_and_cancels_without_main_receipt() {
    let (mut c, s, a, mut reads) = fixture();
    let reference = read(&mut c, &mut reads, &s, Some(&a));
    let before = a.state().bytes().to_vec();
    let gate = Arc::new((Mutex::new((false, false)), Condvar::new()));
    let pause = gate.clone();
    let runtime = ResultRuntime::with_scratch_probes(ScratchWorkerProbes {
        before_evaluate: Some(Arc::new(move || {
            let (lock, cv) = &*pause;
            let mut p = lock.lock().unwrap();
            p.0 = true;
            cv.notify_all();
            while !p.1 {
                p = cv.wait(p).unwrap();
            }
        })),
    })
    .unwrap();
    runtime.accepted(a.clone()).unwrap();
    let f = frozen(
        "paused-scratch",
        json!({"snapshot_ref":reference,"source":"a=99;a","dialect":"Wolfram"}),
        &mut reads,
        &s,
        Some(&a),
    )
    .unwrap();
    runtime.agent_scratch(f, &s).unwrap();
    let (lock, cv) = &*gate;
    let p = lock.lock().unwrap();
    let (p, timeout) = cv
        .wait_timeout_while(p, Duration::from_secs(5), |p| !p.0)
        .unwrap();
    assert!(!timeout.timed_out());
    drop(p);
    let manifest=decode_command(&serde_json::to_vec(&json!({"type":"result_manifest","request_id":"independent-main-read","root_result_id":a.receipt().result_id.0,"offset":0,"limit":32})).unwrap()).unwrap();
    let actual = complete(&runtime, runtime.command(manifest, &s).unwrap(), &s);
    assert_eq!(actual.phase, NativeResultHostReplyPhase::Completed);
    assert_eq!(runtime.cancel("paused-scratch"), Some(true));
    lock.lock().unwrap().1 = true;
    cv.notify_all();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let reply = runtime
            .command(
                NativeResultHostCommand::ResultStatus(ResultStatus {
                    r#type: ResultStatusType::ResultStatus,
                    request_id: "paused-scratch".into(),
                }),
                &s,
            )
            .unwrap();
        if reply.phase == NativeResultHostReplyPhase::Cancelled {
            break;
        }
        assert!(Instant::now() < end);
        std::thread::yield_now();
    }
    assert_eq!(a.state().bytes(), before);
    runtime.take_thread().unwrap().join().unwrap();
}
#[test]
fn actual_worker_result_ref_can_be_inspected_as_scratch_with_no_notebook_receipt() {
    let (mut c, s, a, mut reads) = fixture();
    let reference = read(&mut c, &mut reads, &s, Some(&a));
    let runtime = ResultRuntime::new().unwrap();
    runtime.accepted(a.clone()).unwrap();
    let f = frozen(
        "native-like-scratch",
        json!({"snapshot_ref":reference,"source":"a=99;a","dialect":"Wolfram"}),
        &mut reads,
        &s,
        Some(&a),
    )
    .unwrap();
    let result = complete(&runtime, runtime.agent_scratch(f, &s).unwrap(), &s);
    assert_eq!(result.phase, NativeResultHostReplyPhase::Completed);
    assert_eq!(result.payload["data"]["notebook_modified"], false);
    let token = result.payload["data"]["result_ref"].as_str().unwrap();
    let raw = json!({"result_ref":token,"part":"value"}).to_string();
    let f = FrozenInspection::original(
        &intent("inspect-scratch", AgentToolName::InspectResult, &raw),
        &raw,
    )
    .unwrap();
    let result = complete(&runtime, runtime.agent_inspect(f, &s).unwrap(), &s);
    assert_eq!(result.payload["data"]["origin"], "scratch");
    assert_eq!(
        result.payload["data"]["payload"]["page"]["source"]["input_form"],
        "99"
    );
    assert_eq!(
        result.payload["data"]["payload"]["binding"]["acceptance"],
        "isolated"
    );
    runtime.take_thread().unwrap().join().unwrap();
}
#[test]
fn unsupported_calls_remain_symbolic_and_budget_reaches_actual_scratch_evaluation() {
    let (mut c, s, a, mut reads) = fixture();
    let reference = read(&mut c, &mut reads, &s, Some(&a));
    let before = a.state().bytes().to_vec();
    let mut store = ScratchResultStore::new([17; 32]);
    let f=frozen("unsupported-io",json!({"snapshot_ref":reference,"source":"Import[\"fixture://unavailable\"]","dialect":"Wolfram","use_notebook_definitions":false}),&mut reads,&s,Some(&a)).unwrap();
    let token = store
        .register(f.evaluate(&budget()).unwrap(), &s, 10, &budget())
        .unwrap();
    let resolved = store.resolve(&token, &s, 11).unwrap();
    assert_eq!(
        resolved
            .source(&[], ResultSourceFormat::InputForm, &budget())
            .unwrap(),
        "Import[\"fixture://unavailable\"]"
    );
    assert_eq!(
        serde_json::to_value(resolved.summary()).unwrap()["statements"][0]["nature"],
        "symbolic"
    );
    // No parent decode is needed here: enough ticks reach the actual 10,000-element map,
    // and the small caller budget must stop it instead of being replaced by Session default.
    let f=frozen("mid-eval-budget",json!({"snapshot_ref":reference,"source":"map(fn(k)=>sin(k),range(1,10000))","dialect":"Modern","use_notebook_definitions":false}),&mut reads,&s,Some(&a)).unwrap();
    let ctx = Interrupt {
        steps_left: std::cell::Cell::new(1000),
        ..budget()
    };
    assert_eq!(f.evaluate(&ctx).err().unwrap(), "LIMIT_EXCEEDED");
    assert_eq!(a.state().bytes(), before);
}
