//! Real confirmed-source/preview services and actual worker checkpoints; no model or paid IO.
use om_host_service::{
    document::{SourceDocument, commit::DocumentCommitController, coordinator::SourceCoordinator},
    kernel::{
        acceptance::{AcceptanceContext, AcceptedKernelState, FrozenKernelPlan},
        worker::{KernelJob, KernelWorker},
    },
    protocol::{Serial, generated::*},
    references::ReferenceScope,
    tools::read::{ReadError, ReadNotebookService, ReadPage},
};
use om_kernel::checkpoint::CheckpointLimits;
use om_num::ctx::Interrupt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
fn args(value: Value) -> Vec<u8> {
    serde_json::to_vec(&value).unwrap()
}
fn setup(sources: &[&str]) -> (DocumentCommitController, ReferenceScope) {
    let owner = SourceDocument::new(
        "doc".into(),
        Serial::new(1).unwrap(),
        NativeSourceFile {
            version: 1,
            title: "中文🙂".into(),
            cells: sources
                .iter()
                .enumerate()
                .map(|(i, source)| NativeSourceCell {
                    id: format!("cell-{i}"),
                    kind: NativeCellKind::Math,
                    source: (*source).into(),
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
        revision: s.revision.get(),
        execution_epoch: s.execution_epoch.get(),
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
        can_write: true,
    };
    (
        DocumentCommitController::new(owner, "runtime".into(), [7; 32]),
        scope,
    )
}
fn read(
    service: &mut ReadNotebookService,
    controller: &mut DocumentCommitController,
    scope: &ReferenceScope,
    value: Value,
    now: u64,
) -> Result<ReadPage, ReadError> {
    service.read(
        &args(value),
        controller,
        &SourceCoordinator::default(),
        None,
        scope,
        now,
    )
}
fn patch(
    controller: &mut DocumentCommitController,
    scope: &ReferenceScope,
    reference: &Value,
    source: &str,
) -> bool {
    let data = args(
        json!({"snapshot_ref":reference,"input":{"kind":"patch","operations":[{"type":"update_cell","target":{"cell_id":"cell-0"},"expected_source_hash":format!("{:x}",Sha256::digest(source.as_bytes())),"source":"5+5"}]}}),
    );
    controller
        .preview(&data, scope, 10, "fixture-time".into())
        .is_ok()
}
#[test]
fn outline_is_bounded_and_full_source_exposure_never_upgrades_its_original_token() {
    let sources = vec!["2+2"; 27];
    let (mut controller, scope) = setup(&sources);
    let mut service = ReadNotebookService::new([9; 32]);
    let before = serde_json::to_vec(controller.owner().snapshot()).unwrap();
    let outline = read(&mut service, &mut controller, &scope, json!({}), 0).unwrap();
    assert_eq!(outline.data["cells"].as_array().unwrap().len(), 20);
    assert_eq!(outline.pagination["truncated"], true);
    assert!(
        outline.data["cells"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["source"].is_null()
                && c["source_complete"] == false
                && c["result_ref"].is_null())
    );
    let second = read(
        &mut service,
        &mut controller,
        &scope,
        json!({"cursor":outline.pagination["next_cursor"]}),
        1,
    )
    .unwrap();
    assert_eq!(second.data["cells"].as_array().unwrap().len(), 7);
    assert_eq!(second.pagination["truncated"], false);
    assert!(!patch(
        &mut controller,
        &scope,
        &outline.data["snapshot_ref"],
        "2+2"
    ));
    let outputs = read(
        &mut service,
        &mut controller,
        &scope,
        json!({"include_outputs":true}),
        2,
    )
    .unwrap();
    assert!(outputs.include_outputs);
    assert!(outputs.data["cells"][0]["result_ref"].is_null());
    let full=read(&mut service,&mut controller,&scope,json!({"view":"cells","cell_ids":["cell-0"],"snapshot_ref":outline.data["snapshot_ref"],"include_outputs":false}),2).unwrap();
    assert_eq!(full.data["cells"][0]["source"], "2+2");
    assert_eq!(full.data["cells"][0]["source_complete"], true);
    assert_ne!(full.data["snapshot_ref"], outline.data["snapshot_ref"]);
    assert!(patch(
        &mut controller,
        &scope,
        &full.data["snapshot_ref"],
        "2+2"
    ));
    assert!(!patch(
        &mut controller,
        &scope,
        &outline.data["snapshot_ref"],
        "2+2"
    ));
    assert_eq!(
        before,
        serde_json::to_vec(controller.owner().snapshot()).unwrap()
    );
    println!(
        "outline/full actual responses: {}",
        json!({"outline":outline,"tail":second,"full":full})
    );
}
#[test]
fn unicode_fragments_reassemble_exact_source_and_final_fragment_still_is_not_full_cell() {
    let source = format!("/*{}*/ 2+2", "中🙂e\u{301}".repeat(15000));
    let (mut controller, scope) = setup(&[&source, "6+6"]);
    let mut service = ReadNotebookService::new([9; 32]);
    let mut page = read(
        &mut service,
        &mut controller,
        &scope,
        json!({"view":"cells","cell_ids":["cell-0","cell-1"],"limit":1}),
        0,
    )
    .unwrap();
    let mut reconstructed = String::new();
    let mut pages = 0;
    loop {
        let cell = &page.data["cells"][0];
        if cell["cell_id"] == "cell-0" {
            reconstructed.push_str(cell["source"].as_str().unwrap());
            assert_eq!(cell["source_complete"], false);
            assert!(!patch(
                &mut controller,
                &scope,
                &page.data["snapshot_ref"],
                &source
            ));
        } else {
            assert_eq!(cell["source"], "6+6");
            assert_eq!(cell["source_complete"], true);
        }
        pages += 1;
        if page.pagination["next_cursor"].is_null() {
            break;
        }
        page = read(
            &mut service,
            &mut controller,
            &scope,
            json!({"cursor":page.pagination["next_cursor"]}),
            pages,
        )
        .unwrap();
    }
    assert_eq!(reconstructed, source);
    assert!(pages > 3);
    assert_eq!(controller.owner().snapshot().revision.get(), 0);
    println!(
        "exact UTF8 reconstruction: {pages} pages / {} bytes / sha256 {:x}; final fragment never full",
        source.len(),
        Sha256::digest(source.as_bytes())
    );
}
#[test]
fn closed_arguments_selection_and_query_bound_cursors_reject_forgery() {
    let (mut controller, scope) = setup(&["2+2", "3+3"]);
    let mut service = ReadNotebookService::new([9; 32]);
    for value in [
        json!({"view":"cells"}),
        json!({"cell_ids":["cell-0"]}),
        json!({"view":"symbols","cell_ids":["cell-0"]}),
        json!({"view":null}),
        json!({"view":"unknown"}),
        json!({"view":"cells","cell_ids":[]}),
        json!({"view":"cells","cell_ids":["cell-0","cell-0"]}),
        json!({"limit":0}),
        json!({"limit":51}),
        json!({"limit":1.5}),
        json!({"include_outputs":null}),
        json!({"unknown":true}),
    ] {
        assert_eq!(
            read(&mut service, &mut controller, &scope, value.clone(), 0).unwrap_err(),
            ReadError::InvalidArgument,
            "{value}"
        );
    }
    assert_eq!(
        service
            .read(
                br#"{"limit":1,"limit":2}"#,
                &mut controller,
                &SourceCoordinator::default(),
                None,
                &scope,
                0
            )
            .unwrap_err(),
        ReadError::InvalidArgument
    );
    assert_eq!(
        read(
            &mut service,
            &mut controller,
            &scope,
            json!({"view":"cells","cell_ids":["missing"]}),
            0
        )
        .unwrap_err(),
        ReadError::NotFound
    );
    let page = read(
        &mut service,
        &mut controller,
        &scope,
        json!({"limit":1,"cursor":null}),
        0,
    )
    .unwrap();
    let cursor = &page.pagination["next_cursor"];
    for value in [
        json!({"cursor":cursor,"limit":2}),
        json!({"cursor":cursor,"view":"symbols"}),
        json!({"cursor":cursor,"include_outputs":true}),
        json!({"cursor":cursor,"snapshot_ref":"snapshot-forged"}),
    ] {
        assert_eq!(
            read(&mut service, &mut controller, &scope, value, 1).unwrap_err(),
            ReadError::InvalidArgument
        );
    }
    assert_eq!(
        read(
            &mut service,
            &mut controller,
            &scope,
            json!({"cursor":"cursor-forged"}),
            1
        )
        .unwrap_err(),
        ReadError::InvalidReference
    );
    assert_eq!(
        read(
            &mut service,
            &mut controller,
            &scope,
            json!({"snapshot_ref":"snapshot-forged"}),
            1
        )
        .unwrap_err(),
        ReadError::InvalidReference
    );
    let mut other = scope.clone();
    other.task_generation += 1;
    assert_eq!(
        read(
            &mut service,
            &mut controller,
            &other,
            json!({"snapshot_ref":page.data["snapshot_ref"]}),
            1
        )
        .unwrap_err(),
        ReadError::InvalidReference
    );
}
#[test]
fn historical_source_is_explicit_and_authority_expiry_and_clock_are_enforced() {
    let (mut controller, scope) = setup(&["2+2"]);
    let mut service = ReadNotebookService::new([9; 32]);
    let page = read(&mut service, &mut controller, &scope, json!({}), 5).unwrap();
    let reference = &page.data["snapshot_ref"];
    assert_eq!(
        read(
            &mut service,
            &mut controller,
            &scope,
            json!({"snapshot_ref":reference}),
            4
        )
        .unwrap_err(),
        ReadError::InvalidReference
    );
    let mut file = controller.owner().snapshot().file.clone();
    file.cells[0].source = "8+8".into();
    // Unit fixture substitutes a later confirmed owner; this is not storage acceptance evidence.
    let plan = controller
        .owner()
        .prepare(
            file,
            "op".into(),
            "tx".into(),
            "event".into(),
            "time".into(),
        )
        .unwrap();
    let mut newer = scope.clone();
    newer.revision = plan.after.revision.get();
    newer.execution_epoch = plan.after.execution_epoch.get();
    newer.snapshot_hash = plan.after.snapshot_hash.clone();
    let owner = SourceDocument::restore(plan.after, Serial::new(1).unwrap()).unwrap();
    let mut changed = DocumentCommitController::new(owner, "runtime".into(), [7; 32]);
    let history = read(
        &mut service,
        &mut changed,
        &newer,
        json!({"view":"cells","cell_ids":["cell-0"],"snapshot_ref":reference}),
        6,
    )
    .unwrap();
    assert_eq!(history.data["is_current"], false);
    assert_eq!(history.data["cells"][0]["source"], "2+2");
    assert!(!patch(&mut changed, &newer, reference, "2+2"));
    assert_eq!(
        read(
            &mut service,
            &mut changed,
            &newer,
            json!({"snapshot_ref":reference}),
            600005
        )
        .unwrap_err(),
        ReadError::InvalidReference
    );
    let mut denied = newer.clone();
    denied.can_read = false;
    assert_eq!(
        read(&mut service, &mut changed, &denied, json!({}), 7).unwrap_err(),
        ReadError::PermissionDenied
    );
    let mut stale = newer.clone();
    stale.revision = 0;
    assert_eq!(
        read(&mut service, &mut changed, &stale, json!({}), 7).unwrap_err(),
        ReadError::StaleSnapshot
    );
    println!("actual immutable history response: {}", json!(history));
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
        committed_at: "2026-10-14T00:00:00Z".into(),
    }
}
#[test]
fn native_history_binding_is_bounded_and_each_read_snapshot_retains_its_original_refs() {
    let (mut controller, scope) = setup(&["2+2"]);
    let mut service = ReadNotebookService::new([9; 32]);
    service
        .bind_history(&scope, vec!["persisted-transaction".into()])
        .unwrap();
    let page = read(&mut service, &mut controller, &scope, json!({}), 0).unwrap();
    assert_eq!(
        page.data["recent_transaction_refs"],
        json!(["persisted-transaction"])
    );
    service
        .bind_history(&scope, vec!["new-history".into()])
        .unwrap();
    let frozen = read(
        &mut service,
        &mut controller,
        &scope,
        json!({"snapshot_ref":page.data["snapshot_ref"]}),
        1,
    )
    .unwrap();
    assert_eq!(
        frozen.data["recent_transaction_refs"],
        page.data["recent_transaction_refs"]
    );
    for refs in [
        vec!["bad path".into()],
        vec!["dup".into(), "dup".into()],
        (0..33).map(|i| format!("tx-{i}")).collect(),
    ] {
        assert_eq!(
            service.bind_history(&scope, refs),
            Err(ReadError::InvalidArgument)
        );
    }
}
#[test]
fn retention_and_contract_limits_fail_closed_and_expiry_releases_the_actual_leases() {
    let (mut controller, scope) = setup(&["2+2"]);
    let mut service = ReadNotebookService::new([9; 32]);
    let mut references = std::collections::BTreeSet::new();
    for _ in 0..128 {
        let page = read(
            &mut service,
            &mut controller,
            &scope,
            json!({"limit":50}),
            0,
        )
        .unwrap();
        assert!(references.insert(page.data["snapshot_ref"].as_str().unwrap().to_owned()));
    }
    assert_eq!(
        read(&mut service, &mut controller, &scope, json!({}), 0).unwrap_err(),
        ReadError::LimitExceeded
    );
    let fresh = read(&mut service, &mut controller, &scope, json!({}), 600001).unwrap();
    assert!(!references.contains(fresh.data["snapshot_ref"].as_str().unwrap()));
    assert_eq!(
        read(
            &mut service,
            &mut controller,
            &scope,
            json!({}),
            om_host_service::protocol::MAX_SERIAL
        )
        .unwrap_err(),
        ReadError::LimitExceeded
    );
}
fn accepted(controller: &DocumentCommitController) -> (AcceptedKernelState, KernelWorker) {
    let context = AcceptanceContext {
        runtime_instance_id: "runtime".into(),
        store_id: "store".into(),
        document_generation: Serial::new(1).unwrap(),
        source: controller.owner().snapshot().clone(),
        general: om_kernel::config::GeneralConfig::default(),
        config_revision: Serial::new(0).unwrap(),
        build: "readonly-fixture".into(),
    };
    let worker = KernelWorker::new(64 * 1024 * 1024, 8, CheckpointLimits::default()).unwrap();
    let boot = FrozenKernelPlan::bootstrap(
        &context,
        "bootstrap",
        &worker,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    boot.enter_commit(&context, None).unwrap();
    let parent = boot.accept_receipt(&receipt(boot.plan())).unwrap();
    let candidate = worker
        .submit(KernelJob {
            runtime_instance_id: "runtime".into(),
            document_generation: Serial::new(1).unwrap(),
            operation_id: "produce".into(),
            parent_checkpoint_ref: parent.registry_ref().into(),
            source: context.source.clone(),
            general: context.general.clone(),
            config_revision: Serial::new(0).unwrap(),
            cell_id: "cell-0".into(),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .unwrap()
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let plan = FrozenKernelPlan::candidate(candidate, &parent, &context, &worker).unwrap();
    plan.enter_commit(&context, Some(&parent)).unwrap();
    // Synthetic matching receipt tests the real checkpoint/coordinator; no SQLite proof claimed.
    (plan.accept_receipt(&receipt(plan.plan())).unwrap(), worker)
}
#[test]
fn symbols_and_status_are_actual_accepted_facts_with_stale_producer_and_no_random_mutation() {
    let (mut controller, mut scope) =
        setup(&["seed_random(42); let sin=fn(x)=>x+1; let a=random()"]);
    let (accepted, _worker) = accepted(&controller);
    scope.definition_revision = accepted.state().binding().kernel_state_revision;
    let before = accepted.state().bytes().to_vec();
    let mut service = ReadNotebookService::new([9; 32]);
    let page = service
        .read(
            &args(json!({"view":"symbols"})),
            &mut controller,
            &SourceCoordinator::default(),
            Some(&accepted),
            &scope,
            0,
        )
        .unwrap();
    assert!(
        page.data["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["name"] == "sin" && s["callable"] == true && s["is_current"] == true)
    );
    let cells = service
        .read(
            &args(json!({})),
            &mut controller,
            &SourceCoordinator::default(),
            Some(&accepted),
            &scope,
            1,
        )
        .unwrap();
    assert_eq!(cells.data["cells"][0]["status"], "Done");
    let mut config = scope.clone();
    config.config_revision += 1;
    let stale = service
        .read(
            &args(json!({"view":"symbols"})),
            &mut controller,
            &SourceCoordinator::default(),
            Some(&accepted),
            &config,
            2,
        )
        .unwrap();
    assert!(
        stale.data["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["is_current"] == false)
    );
    assert_eq!(before, accepted.state().bytes());
    println!(
        "actual accepted symbols/current/stale: {}",
        json!({"symbols":page,"cells":cells,"stale":stale,"checkpoint_hash_unchanged":accepted.state().hash()})
    );
}
