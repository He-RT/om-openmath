//! Result cache/reference/type projection tests. Synthetic receipts test contracts, not disk IO.
use om_host_service::{
    document::SourceDocument,
    kernel::{
        acceptance::{AcceptanceContext, AcceptedKernelState, FrozenKernelPlan},
        worker::{KernelJob, KernelWorker},
    },
    protocol::{Serial, generated::*},
    references::{ReferenceError, ReferenceScope},
    results::{ResultError, ResultStore, StoredResult},
};
use om_kernel::{
    checkpoint::CheckpointLimits,
    config::GeneralConfig,
    protocol::{Dialect as KernelDialect, OutputItem, Request, Response, ValueQuery},
    retained_results::ResultSourceFormat,
};
use om_num::ctx::Interrupt;
use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
fn s(n: u64) -> Serial {
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
        committed_at: "2026-10-11T00:00:00.000Z".into(),
    }
}
fn setup() -> (KernelWorker, AcceptanceContext, AcceptedKernelState) {
    let source = SourceDocument::new(
        "result-doc".into(),
        s(1),
        NativeSourceFile {
            version: 1,
            title: "results".into(),
            cells: vec![
                NativeSourceCell {
                    id: "a".into(),
                    kind: NativeCellKind::Math,
                    source: "let a=2; seed_random(42); a".into(),
                    dialect: Dialect::Modern,
                },
                NativeSourceCell {
                    id: "b".into(),
                    kind: NativeCellKind::Math,
                    source: "let b=a+1; b".into(),
                    dialect: Dialect::Modern,
                },
                NativeSourceCell {
                    id: "list".into(),
                    kind: NativeCellKind::Math,
                    source: "[1/3,decimal(\"0.1\",precision:80),[\"中🙂\",4]]".into(),
                    dialect: Dialect::Modern,
                },
            ],
        },
    )
    .unwrap()
    .snapshot()
    .clone();
    let context = AcceptanceContext {
        runtime_instance_id: "runtime".into(),
        store_id: "store".into(),
        document_generation: s(1),
        source,
        general: GeneralConfig::default(),
        config_revision: s(0),
        build: "result-fixture".into(),
    };
    let worker = KernelWorker::new(64 * 1024 * 1024, 16, CheckpointLimits::default()).unwrap();
    let seed = FrozenKernelPlan::bootstrap(
        &context,
        "boot",
        &worker,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    seed.enter_commit(&context, None).unwrap();
    let accepted = seed.accept_receipt(&receipt(seed.plan())).unwrap();
    (worker, context, accepted)
}
fn run(
    worker: &KernelWorker,
    context: &AcceptanceContext,
    parent: &AcceptedKernelState,
    cell: &str,
    id: &str,
) -> AcceptedKernelState {
    let candidate = worker
        .submit(KernelJob {
            runtime_instance_id: context.runtime_instance_id.clone(),
            document_generation: context.document_generation,
            operation_id: id.into(),
            parent_checkpoint_ref: parent.registry_ref().into(),
            source: context.source.clone(),
            general: context.general.clone(),
            config_revision: context.config_revision,
            cell_id: cell.into(),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .unwrap()
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let frozen = FrozenKernelPlan::candidate(candidate, parent, context, worker).unwrap();
    frozen.enter_commit(context, Some(parent)).unwrap();
    frozen.accept_receipt(&receipt(frozen.plan())).unwrap()
}
fn scope(context: &AcceptanceContext, state: &AcceptedKernelState) -> ReferenceScope {
    ReferenceScope {
        runtime: context.runtime_instance_id.clone(),
        document: context.source.document_id.clone(),
        generation: 1,
        revision: context.source.revision.get(),
        execution_epoch: context.source.execution_epoch.get(),
        snapshot_hash: context.source.snapshot_hash.clone(),
        task: None,
        task_generation: 0,
        grant_revision: 1,
        config_revision: 0,
        definition_revision: state.receipt().kernel_state_revision.get(),
        metadata_revision: 26,
        editor_state_hash: String::new(),
        execution_mode: true,
        can_read: true,
        can_preview: true,
        can_write: true,
    }
}
fn close(worker: &KernelWorker) {
    worker.begin_close().unwrap().unwrap().join().unwrap();
}
#[test]
fn actual_result_occurrences_scope_history_and_exact_pages_survive_other_executions() {
    let (worker, ctx, parent) = setup();
    let accepted = run(&worker, &ctx, &parent, "list", "list-one");
    let record = StoredResult::capture(
        &accepted,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let id = record.id().to_owned();
    let desc = record.summary();
    let r = &desc.statements[0];
    let mut store = ResultStore::new([9; 32], 64 * 1024 * 1024, 16).unwrap();
    store.register(record).unwrap();
    store.set_current(&accepted, &Interrupt::default()).unwrap();
    let current = scope(&ctx, &accepted);
    let token = store
        .issue(&id, r.out_index, Some(&r.view_id), current.clone(), 0)
        .unwrap();
    let resolved = store.resolve(&token, &current, 1).unwrap();
    assert_eq!(
        resolved.binding().freshness,
        ResultBindingFreshness::Current
    );
    let q = ValueQuery {
        cell_id: "list".into(),
        out_index: r.out_index,
        view_id: r.view_id.clone(),
        path: Vec::new(),
        offset: 0,
        limit: 1,
        column_offset: 0,
        column_limit: 1,
        include_source: false,
    };
    assert_eq!(
        resolved.page(&q, &Interrupt::default()).unwrap().rows[0].cells[0]
            .source
            .as_ref()
            .unwrap()
            .input_form,
        "1/3"
    );
    let other_worker =
        KernelWorker::new(64 * 1024 * 1024, 16, CheckpointLimits::default()).unwrap();
    let other_source = SourceDocument::new("different-doc".into(), s(1), ctx.source.file.clone())
        .unwrap()
        .snapshot()
        .clone();
    let other_context = AcceptanceContext {
        source: other_source,
        ..ctx.clone()
    };
    let seed = FrozenKernelPlan::bootstrap(
        &other_context,
        "other-boot",
        &other_worker,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    seed.enter_commit(&other_context, None).unwrap();
    let other_parent = seed.accept_receipt(&receipt(seed.plan())).unwrap();
    let other_list = run(
        &other_worker,
        &other_context,
        &other_parent,
        "list",
        "other-list",
    );
    store
        .set_current(&other_list, &Interrupt::default())
        .unwrap();
    assert_eq!(
        store
            .resolve(&token, &current, 1)
            .unwrap()
            .binding()
            .freshness,
        ResultBindingFreshness::Stale
    );
    store.set_current(&accepted, &Interrupt::default()).unwrap();
    close(&other_worker);
    let later = run(&worker, &ctx, &accepted, "a", "unrelated-a");
    store.set_current(&later, &Interrupt::default()).unwrap();
    let later_scope = scope(&ctx, &later);
    assert_eq!(
        store
            .resolve(&token, &later_scope, 2)
            .unwrap()
            .binding()
            .freshness,
        ResultBindingFreshness::Current
    );
    let overwritten = run(&worker, &ctx, &later, "list", "list-two");
    store
        .set_current(&overwritten, &Interrupt::default())
        .unwrap();
    let latest = scope(&ctx, &overwritten);
    let history = store.resolve(&token, &latest, 3).unwrap();
    assert_eq!(
        history.binding().acceptance,
        ResultBindingAcceptance::HistoryOnly
    );
    assert_eq!(
        history
            .source(&[0], ResultSourceFormat::InputForm, &Interrupt::default())
            .unwrap(),
        "1/3"
    );
    let mut bad = latest.clone();
    bad.generation = 2;
    assert!(matches!(
        store.resolve(&token, &bad, 3),
        Err(ResultError::Reference(ReferenceError::StaleScope))
    ));
    bad = latest.clone();
    bad.can_read = false;
    assert!(matches!(
        store.resolve(&token, &bad, 3),
        Err(ResultError::Reference(ReferenceError::PermissionDenied))
    ));
    assert!(
        store
            .issue(&id, r.out_index, Some("wrong-view"), latest.clone(), 4)
            .is_err()
    );
    assert!(matches!(
        store.resolve(&token, &latest, 600000),
        Err(ResultError::Reference(ReferenceError::Expired))
    ));
    store.revoke_reference(&token).unwrap();
    assert!(matches!(
        store.resolve(&token, &latest, 5),
        Err(ResultError::Reference(ReferenceError::Revoked))
    ));
    close(&worker);
}
#[test]
fn scratch_and_readonly_random_out_set_are_isolated_from_original_state() {
    let (worker, ctx, parent) = setup();
    let accepted = run(&worker, &ctx, &parent, "a", "a-one");
    let before = accepted.state().bytes().to_vec();
    let record = StoredResult::capture(
        &accepted,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let id = record.id().to_owned();
    let summary = record.summary();
    let r = summary.statements.last().unwrap();
    let mut store = ResultStore::new([7; 32], 64 * 1024 * 1024, 16).unwrap();
    store.register(record).unwrap();
    store.set_current(&accepted, &Interrupt::default()).unwrap();
    let scope = scope(&ctx, &accepted);
    let token = store
        .issue(&id, r.out_index, Some(&r.view_id), scope.clone(), 0)
        .unwrap();
    let result = store.resolve(&token, &scope, 1).unwrap();
    assert!(
        result
            .readonly_expression("Set[a,9]", false, &Interrupt::default())
            .is_err()
    );
    assert!(
        result
            .readonly_expression("SeedRandom[9]", false, &Interrupt::default())
            .is_err()
    );
    let first = result
        .readonly_expression("RandomUniform[]", false, &Interrupt::default())
        .unwrap();
    let second = result
        .readonly_expression("RandomUniform[]", false, &Interrupt::default())
        .unwrap();
    assert_eq!(first.input_form, second.input_form);
    let Response::Evaluated { output, .. } = result
        .scratch(
            "let a=99; random_uniform(); a".into(),
            KernelDialect::Modern,
            &Interrupt::default(),
        )
        .unwrap()
    else {
        panic!("no real scratch")
    };
    let OutputItem::Expr { input_form, .. } = output.items.last().unwrap() else {
        panic!("no expr")
    };
    assert_eq!(input_form, "99");
    assert_eq!(
        result
            .readonly_expression("a", false, &Interrupt::default())
            .unwrap()
            .input_form,
        "2"
    );
    assert_eq!(accepted.state().bytes(), before);
    close(&worker);
}
#[test]
fn reexecuted_definition_invalidates_and_retires_dependent_owned_value_without_cascade() {
    let (worker, ctx, parent) = setup();
    let a = run(&worker, &ctx, &parent, "a", "a-one");
    let b = run(&worker, &ctx, &a, "b", "b-one");
    let record =
        StoredResult::capture(&b, CheckpointLimits::default(), &Interrupt::default()).unwrap();
    let id = record.id().to_owned();
    let desc = record.summary();
    let r = desc.statements.last().unwrap();
    let mut store = ResultStore::new([6; 32], 64 * 1024 * 1024, 16).unwrap();
    store.register(record).unwrap();
    store.set_current(&b, &Interrupt::default()).unwrap();
    let current = scope(&ctx, &b);
    let token = store
        .issue(&id, r.out_index, Some(&r.view_id), current, 0)
        .unwrap();
    let repeated = run(&worker, &ctx, &b, "a", "a-two");
    store.set_current(&repeated, &Interrupt::default()).unwrap();
    let current = scope(&ctx, &repeated);
    let stale = store.resolve(&token, &current, 1).unwrap();
    assert_eq!(stale.binding().freshness, ResultBindingFreshness::Stale);
    assert_eq!(
        stale
            .readonly_expression("b", false, &Interrupt::default())
            .unwrap()
            .input_form,
        "3"
    );
    let state = repeated.state();
    let mut work = state
        .restore(
            om_kernel::checkpoint::CheckpointRestore {
                binding: state.binding(),
                source: state.source(),
                general: state.general(),
                clock: None,
                cancel: Arc::new(AtomicBool::new(false)),
            },
            CheckpointLimits::default(),
            &Interrupt::default(),
        )
        .unwrap();
    let Response::Evaluated { output, .. } = work
        .session_mut()
        .handle(Request::Evaluate {
            cell_id: "probe".into(),
            source: "b".into(),
            dialect: KernelDialect::Modern,
        })
        .0
    else {
        panic!("missing output")
    };
    let OutputItem::Expr { input_form, .. } = &output.items[0] else {
        panic!("no scalar")
    };
    assert_eq!(input_form, "b");
    close(&worker);
}
#[test]
fn revoked_cache_lookup_keeps_reader_reservation_until_actual_pin_release() {
    let (worker, ctx, parent) = setup();
    let accepted = run(&worker, &ctx, &parent, "list", "list-one");
    let record = StoredResult::capture(
        &accepted,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let id = record.id().to_owned();
    let desc = record.summary();
    let r = &desc.statements[0];
    let mut store = ResultStore::new([2; 32], 64 * 1024 * 1024, 1).unwrap();
    store.register(record).unwrap();
    store.set_current(&accepted, &Interrupt::default()).unwrap();
    let current = scope(&ctx, &accepted);
    let token = store
        .issue(&id, r.out_index, Some(&r.view_id), current.clone(), 0)
        .unwrap();
    let pinned = store.resolve(&token, &current, 1).unwrap();
    let bytes = store.reserved_bytes();
    store.evict(&id).unwrap();
    assert_eq!(store.reserved_bytes(), bytes);
    assert!(store.resolve(&token, &current, 2).is_err());
    assert!(matches!(
        store.register(
            StoredResult::capture(
                &accepted,
                CheckpointLimits::default(),
                &Interrupt::default()
            )
            .unwrap()
        ),
        Err(ResultError::Budget)
    ));
    assert_eq!(
        pinned
            .source(&[0], ResultSourceFormat::InputForm, &Interrupt::default())
            .unwrap(),
        "1/3"
    );
    drop(pinned);
    store.reap();
    assert_eq!(store.reserved_bytes(), 0);
    close(&worker);
}
