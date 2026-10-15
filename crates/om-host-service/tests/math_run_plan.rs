//! Actual read-issued refs and actual worker states. Receipts here are synthetic matching unit
//! acknowledgements; this file does not claim physical storage/Agent/Pi execution integration.
use om_host_service::{
    agent::ingress::ToolIntent,
    document::{
        SourceDocument,
        commit::{CommitError, DocumentCommitController},
        coordinator::SourceCoordinator,
        preview::PreviewError,
    },
    kernel::{
        acceptance::{AcceptanceContext, AcceptedKernelState, FrozenKernelPlan},
        worker::{KernelJob, KernelWorker},
    },
    protocol::{Serial, generated::*},
    references::ReferenceScope,
    tools::{
        read::ReadNotebookService,
        run::{self, RunSource},
    },
};
use om_kernel::{checkpoint::CheckpointLimits, config::GeneralConfig};
use om_num::ctx::Interrupt;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
fn setup() -> (
    DocumentCommitController,
    ReferenceScope,
    ReadNotebookService,
) {
    let owner = SourceDocument::new(
        "doc".into(),
        Serial::new(1).unwrap(),
        NativeSourceFile {
            version: 1,
            title: "actual plan".into(),
            cells: [
                ("answer", "b+1"),
                ("unrelated", "seed_random(99);rand()"),
                ("b", "let b=a+1;b"),
                ("a", "let a=2;a"),
            ]
            .into_iter()
            .map(|(id, source)| NativeSourceCell {
                id: id.into(),
                source: source.into(),
                kind: NativeCellKind::Math,
                dialect: Dialect::Modern,
            })
            .collect(),
        },
    )
    .unwrap();
    let s = owner.snapshot();
    let scope = ReferenceScope {
        runtime: "runtime".into(),
        document: "doc".into(),
        generation: 1,
        revision: 0,
        execution_epoch: 0,
        snapshot_hash: s.snapshot_hash.clone(),
        task: Some("task".into()),
        task_generation: 1,
        grant_revision: 1,
        config_revision: 0,
        definition_revision: 0,
        metadata_revision: 26,
        editor_state_hash: String::new(),
        execution_mode: true,
        can_read: true,
        can_preview: true,
        can_write: false,
    };
    (
        DocumentCommitController::new(owner, "runtime".into(), [7; 32]),
        scope,
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
fn intent(args: &[u8]) -> ToolIntent {
    ToolIntent {
        operation_id: "actual-native-intent".into(),
        turn_id: "turn".into(),
        call_id: "call".into(),
        proposal_event: "acked-proposal".into(),
        name: AgentToolName::RunCells,
        arguments_hash: format!("{:x}", Sha256::digest(args)),
    }
}
fn allow(c: &DocumentCommitController) -> BTreeSet<String> {
    c.owner()
        .snapshot()
        .file
        .cells
        .iter()
        .map(|c| c.id.clone())
        .collect()
}
fn source<'a>(
    reads: &'a mut ReadNotebookService,
    scope: &'a ReferenceScope,
    accepted: Option<&'a AcceptedKernelState>,
    allowed: BTreeSet<String>,
) -> RunSource<'a> {
    RunSource {
        reads,
        scope,
        editor_busy: false,
        general: GeneralConfig::default(),
        accepted,
        allowed,
        now: 6,
    }
}
#[test]
fn real_outline_token_plans_only_required_cells_off_owner_and_never_upgrades_source_exposure() {
    let (mut c, s, mut reads) = setup();
    let reference = read(&mut c, &mut reads, &s, None);
    let before = serde_json::to_vec(c.owner().snapshot()).unwrap();
    let args = json!({"snapshot_ref":reference,"cell_ids":["answer"]}).to_string();
    let frozen = run::prepare(
        &intent(args.as_bytes()),
        args.as_bytes(),
        source(&mut reads, &s, None, allow(&c)),
    )
    .unwrap();
    assert_eq!(frozen.scope(), &s);
    let result = std::thread::spawn(move || frozen.plan(&Interrupt::default()))
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(result.plan().ordered_cell_ids, ["a", "b", "answer"]);
    assert!(!result.continue_on_error());
    let patch=json!({"snapshot_ref":reference,"input":{"kind":"patch","operations":[{"type":"update_cell","target":{"cell_id":"a"},"expected_source_hash":format!("{:x}",Sha256::digest(b"let a=2;a")),"source":"8+8"}]}}).to_string();
    assert!(matches!(
        c.preview(patch.as_bytes(), &s, 7, "unit-time".into()),
        Err(CommitError::Preview(PreviewError::IncompleteSource))
    ));
    assert_eq!(before, serde_json::to_vec(c.owner().snapshot()).unwrap());
    println!(
        "actual bounded source plan={}",
        serde_json::to_string(&result).unwrap()
    );
}
#[test]
fn acknowledged_argument_hash_closed_parameters_and_native_allowlist_are_authoritative() {
    let (mut c, s, mut reads) = setup();
    let reference = read(&mut c, &mut reads, &s, None);
    for value in [
        json!({"snapshot_ref":reference,"cell_ids":["answer"],"source":"invented"}),
        json!({"snapshot_ref":reference,"cell_ids":["answer"],"grants":["all"]}),
        json!({"snapshot_ref":reference,"cell_ids":["answer"],"scope":null}),
        json!({"snapshot_ref":reference,"cell_ids":["answer"],"continue_on_error":null}),
    ] {
        let a = value.to_string();
        assert!(
            run::prepare(
                &intent(a.as_bytes()),
                a.as_bytes(),
                source(&mut reads, &s, None, allow(&c))
            )
            .is_err()
        );
    }
    let raw = json!({"snapshot_ref":reference,"cell_ids":["answer"],"continue_on_error":true})
        .to_string();
    let wrong = intent(b"{}");
    assert_eq!(
        run::prepare(
            &wrong,
            raw.as_bytes(),
            source(&mut reads, &s, None, allow(&c))
        )
        .err()
        .unwrap(),
        "IPC_CALL_SCOPE_MISMATCH"
    );
    let frozen = run::prepare(
        &intent(raw.as_bytes()),
        raw.as_bytes(),
        source(&mut reads, &s, None, ["answer".into()].into()),
    )
    .unwrap();
    assert_eq!(
        frozen.plan(&Interrupt::default()).unwrap_err(),
        "PERMISSION_DENIED"
    );
    let frozen = run::prepare(
        &intent(raw.as_bytes()),
        raw.as_bytes(),
        source(&mut reads, &s, None, allow(&c)),
    )
    .unwrap();
    assert!(
        frozen
            .plan(&Interrupt::default())
            .unwrap()
            .continue_on_error()
    );
}
#[test]
fn historical_or_cross_task_refs_and_clock_rewind_cannot_choose_current_run_input() {
    for change in [
        "revision",
        "definition",
        "task",
        "editor",
        "readonly",
        "expiry",
        "clock",
    ] {
        let (mut c, s, mut reads) = setup();
        let reference = read(&mut c, &mut reads, &s, None);
        let raw = json!({"snapshot_ref":reference,"cell_ids":["answer"]}).to_string();
        let mut current = s.clone();
        let mut now = 6;
        match change {
            "revision" => {
                current.revision += 1;
                current.snapshot_hash = "b".repeat(64)
            }
            "definition" => current.definition_revision += 1,
            "task" => current.task_generation += 1,
            "editor" => current.editor_state_hash = "a".repeat(64),
            "readonly" => current.execution_mode = false,
            "expiry" => now = 600006,
            "clock" => now = 4,
            _ => unreachable!(),
        }
        let mut input = source(&mut reads, &current, None, allow(&c));
        input.now = now;
        assert!(
            run::prepare(&intent(raw.as_bytes()), raw.as_bytes(), input).is_err(),
            "{change}"
        );
    }
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
        committed_at: "2026-10-15T00:00:00Z".into(),
    }
}
#[test]
fn actual_accepted_definition_skips_ready_prerequisite_and_checkpoint_bytes_remain_exact() {
    let (mut c, mut s, mut reads) = setup();
    let ctx = AcceptanceContext {
        runtime_instance_id: "runtime".into(),
        store_id: "store".into(),
        document_generation: Serial::new(1).unwrap(),
        source: c.owner().snapshot().clone(),
        general: GeneralConfig::default(),
        config_revision: Serial::new(0).unwrap(),
        build: "real-plan-unit".into(),
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
            document_generation: Serial::new(1).unwrap(),
            operation_id: "actual-a".into(),
            parent_checkpoint_ref: parent.registry_ref().into(),
            source: ctx.source.clone(),
            general: ctx.general.clone(),
            config_revision: ctx.config_revision,
            cell_id: "a".into(),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .unwrap()
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let frozen = FrozenKernelPlan::candidate(candidate, &parent, &ctx, &worker).unwrap();
    frozen.enter_commit(&ctx, Some(&parent)).unwrap();
    let accepted = frozen.accept_receipt(&receipt(frozen.plan())).unwrap();
    s.definition_revision = accepted.state().binding().kernel_state_revision;
    let before = accepted.state().bytes().to_vec();
    let reference = read(&mut c, &mut reads, &s, Some(&accepted));
    let raw = json!({"snapshot_ref":reference,"cell_ids":["answer"]}).to_string();
    let frozen = run::prepare(
        &intent(raw.as_bytes()),
        raw.as_bytes(),
        source(&mut reads, &s, Some(&accepted), allow(&c)),
    )
    .unwrap();
    let actual = std::thread::spawn(move || frozen.plan(&Interrupt::default()))
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(actual.plan().ordered_cell_ids, ["b", "answer"]);
    assert_eq!(before, accepted.state().bytes());
    assert_eq!(
        run::prepare(
            &intent(raw.as_bytes()),
            raw.as_bytes(),
            source(&mut reads, &s, None, allow(&c))
        )
        .err()
        .unwrap(),
        "CONTEXT_NOT_READY"
    );
    println!(
        "actual accepted checkpoint plan={} /{} unchanged bytes",
        serde_json::to_string(&actual).unwrap(),
        before.len()
    );
    // Replace the unit's confirmed source owner from actual non-evaluating mutation plans;
    // no SQLite receipt is claimed here. Title-only source preserves executed a, while changing
    // its Math source retires the actual definition in the private reconciled planning session.
    for (index, operation, expected) in [
        (
            0,
            NativeSourceOperation::RenameSourceNotebook(RenameSourceNotebook {
                kind: RenameSourceNotebookKind::RenameNotebook,
                title: "new title".into(),
            }),
            vec!["b", "answer"],
        ),
        (
            1,
            NativeSourceOperation::UpdateSourceCell(UpdateSourceCell {
                kind: UpdateSourceCellKind::UpdateCell,
                cell: NativeSourceCell {
                    id: "a".into(),
                    source: "let a=9;a".into(),
                    kind: NativeCellKind::Math,
                    dialect: Dialect::Modern,
                },
            }),
            vec!["a", "b", "answer"],
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
            SourceDocument::restore(head, Serial::new(1).unwrap()).unwrap(),
            "runtime".into(),
            [17 + index as u8; 32],
        );
        let page = reads
            .read(
                b"{}",
                &mut c,
                &SourceCoordinator::default(),
                Some(&accepted),
                &s,
                5,
            )
            .unwrap();
        let a = page.data["cells"]
            .as_array()
            .unwrap()
            .iter()
            .find(|cell| cell["cell_id"] == "a")
            .unwrap();
        assert_eq!(a["status"], if index == 0 { "Done" } else { "Stale" });
        let symbols = reads
            .read(
                br#"{"view":"symbols"}"#,
                &mut c,
                &SourceCoordinator::default(),
                Some(&accepted),
                &s,
                5,
            )
            .unwrap();
        assert_eq!(symbols.data["symbols"][0]["is_current"], index == 0);
        let reference = page.data["snapshot_ref"].as_str().unwrap().to_owned();
        let raw = json!({"snapshot_ref":reference,"cell_ids":["answer"]}).to_string();
        let frozen = run::prepare(
            &intent(raw.as_bytes()),
            raw.as_bytes(),
            source(&mut reads, &s, Some(&accepted), allow(&c)),
        )
        .unwrap();
        let plan = std::thread::spawn(move || frozen.plan(&Interrupt::default()))
            .join()
            .unwrap()
            .unwrap();
        assert_eq!(plan.plan().ordered_cell_ids, expected);
        assert_eq!(before, accepted.state().bytes());
        println!(
            "actual source-change plan {index}={}",
            serde_json::to_string(&plan).unwrap()
        );
    }
    if let Some(thread) = worker.begin_close().unwrap() {
        thread.join().unwrap();
    }
}
