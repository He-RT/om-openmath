//! Actual worker/control requests and original scopes; synthetic receipts here are not IO proof.
use om_host_service::{
    document::SourceDocument,
    kernel::{
        acceptance::{AcceptanceContext, AcceptedKernelState, FrozenKernelPlan},
        worker::{KernelJob, KernelWorker},
    },
    protocol::{Serial, generated::*},
    references::ReferenceScope,
    results::runtime::{ResultRuntime, ResultWorkerProbes, decode_command},
};
use om_kernel::{checkpoint::CheckpointLimits, config::GeneralConfig};
use om_num::ctx::Interrupt;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Condvar, Mutex, atomic::AtomicBool},
    time::{Duration, Instant},
};
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
fn owned_setup() -> (AcceptedKernelState, ReferenceScope, KernelWorker) {
    let serial = |n| Serial::new(n).unwrap();
    let source = SourceDocument::new(
        "runtime-result-doc".into(),
        serial(1),
        NativeSourceFile {
            version: 1,
            title: "read".into(),
            cells: vec![NativeSourceCell {
                id: "list".into(),
                kind: NativeCellKind::Math,
                source: "let a=2; seed_random(42); [1/3,decimal(\"0.1\",precision:80),\"中🙂\"]"
                    .into(),
                dialect: Dialect::Modern,
            }],
        },
    )
    .unwrap()
    .snapshot()
    .clone();
    let context = AcceptanceContext {
        runtime_instance_id: "runtime".into(),
        store_id: "store".into(),
        document_generation: serial(1),
        source,
        general: GeneralConfig::default(),
        config_revision: serial(0),
        build: "result-read-fixture".into(),
    };
    let worker = KernelWorker::new(64 * 1024 * 1024, 8, CheckpointLimits::default()).unwrap();
    let seed = FrozenKernelPlan::bootstrap(
        &context,
        "boot",
        &worker,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    seed.enter_commit(&context, None).unwrap();
    let parent = seed.accept_receipt(&receipt(seed.plan())).unwrap();
    let candidate = worker
        .submit(KernelJob {
            runtime_instance_id: "runtime".into(),
            document_generation: serial(1),
            operation_id: "producer".into(),
            parent_checkpoint_ref: parent.registry_ref().into(),
            source: context.source.clone(),
            general: context.general.clone(),
            config_revision: serial(0),
            cell_id: "list".into(),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .unwrap()
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let frozen = FrozenKernelPlan::candidate(candidate, &parent, &context, &worker).unwrap();
    frozen.enter_commit(&context, Some(&parent)).unwrap();
    let accepted = frozen.accept_receipt(&receipt(frozen.plan())).unwrap();
    let scope = ReferenceScope {
        runtime: "runtime".into(),
        document: context.source.document_id,
        generation: 1,
        revision: 0,
        execution_epoch: 0,
        snapshot_hash: context.source.snapshot_hash,
        task: None,
        task_generation: 0,
        grant_revision: 1,
        config_revision: 0,
        definition_revision: 1,
        metadata_revision: 26,
        editor_state_hash: String::new(),
        execution_mode: true,
        can_read: true,
        can_preview: false,
        can_write: false,
    };
    (accepted, scope, worker)
}
fn setup() -> (AcceptedKernelState, ReferenceScope) {
    let (accepted, scope, worker) = owned_setup();
    worker.begin_close().unwrap().unwrap().join().unwrap();
    (accepted, scope)
}
fn command(value: Value) -> NativeResultHostCommand {
    decode_command(&serde_json::to_vec(&value).unwrap()).unwrap()
}
fn complete(
    runtime: &ResultRuntime,
    cmd: NativeResultHostCommand,
    scope: &ReferenceScope,
) -> NativeResultHostReply {
    let mut reply = runtime.command(cmd, scope).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while matches!(
        reply.phase,
        NativeResultHostReplyPhase::Queued | NativeResultHostReplyPhase::Running
    ) {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
        reply = runtime
            .command(
                command(json!({"type":"result_status","request_id":reply.request_id})),
                scope,
            )
            .unwrap();
    }
    reply
}
#[test]
fn notebook_output_refs_resolve_real_exact_value_and_never_substitute_a_later_state() {
    let (accepted, mut scope, worker) = owned_setup();
    scope.task = Some("actual-read-task".into());
    scope.task_generation = 1;
    let before = accepted.state().bytes().to_vec();
    let runtime = ResultRuntime::new().unwrap();
    runtime.accepted(accepted.clone()).unwrap();
    let source = accepted
        .receipt()
        .producer
        .cell_source_hash
        .0
        .as_ref()
        .unwrap();
    let query = |id: &str, upper: u64, hash: &str| {
        command(
            json!({"type":"result_notebook_references","request_id":id,"kernel_state_revision":upper,"cells":[{"cell_id":"list","source_hash":hash}]}),
        )
    };
    let reply = complete(
        &runtime,
        query("notebook-outputs", scope.definition_revision, source),
        &scope,
    );
    assert_eq!(reply.phase, NativeResultHostReplyPhase::Completed);
    let token = reply.payload["list"].as_str().unwrap();
    assert!(token.starts_with("result-"));
    let inspected = complete(
        &runtime,
        command(
            json!({"type":"result_inspect","request_id":"actual-ref-page","result_ref":token,"query":{"kind":"page","path":[],"offset":0,"limit":1,"column_offset":0,"column_limit":1}}),
        ),
        &scope,
    );
    assert_eq!(
        inspected.payload["rows"][0]["cells"][0]["source"]["input_form"],
        "1/3"
    );
    assert!(complete(&runtime, query("prior-state", 0, source), &scope).payload["list"].is_null());
    assert!(
        complete(
            &runtime,
            query("wrong-source", scope.definition_revision, &"0".repeat(64)),
            &scope
        )
        .payload["list"]
            .is_null()
    );
    assert_eq!(
        complete(
            &runtime,
            query("future-state", scope.definition_revision + 1, source),
            &scope
        )
        .phase,
        NativeResultHostReplyPhase::Failed
    );
    let mut other = scope.clone();
    other.task_generation += 1;
    let denied = complete(
        &runtime,
        command(
            json!({"type":"result_inspect","request_id":"old-task-ref","result_ref":token,"query":{"kind":"summary"}}),
        ),
        &other,
    );
    assert_eq!(denied.phase, NativeResultHostReplyPhase::Failed);
    let doc = SourceDocument::new(
        scope.document.clone(),
        Serial::new(scope.generation).unwrap(),
        serde_json::from_value(serde_json::to_value(accepted.state().source()).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(doc.snapshot().snapshot_hash, scope.snapshot_hash);
    let context = AcceptanceContext {
        runtime_instance_id: scope.runtime.clone(),
        store_id: "store".into(),
        document_generation: Serial::new(scope.generation).unwrap(),
        source: doc.snapshot().clone(),
        general: accepted.state().general().clone(),
        config_revision: Serial::new(scope.config_revision).unwrap(),
        build: "result-read-fixture".into(),
    };
    let candidate = worker
        .submit(KernelJob {
            runtime_instance_id: scope.runtime.clone(),
            document_generation: Serial::new(scope.generation).unwrap(),
            operation_id: "real-second-output".into(),
            parent_checkpoint_ref: accepted.registry_ref().into(),
            source: context.source.clone(),
            general: context.general.clone(),
            config_revision: Serial::new(scope.config_revision).unwrap(),
            cell_id: "list".into(),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .unwrap()
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let plan = FrozenKernelPlan::candidate(candidate, &accepted, &context, &worker).unwrap();
    plan.enter_commit(&context, Some(&accepted)).unwrap();
    let newer = plan.accept_receipt(&receipt(plan.plan())).unwrap();
    runtime.accepted(newer.clone()).unwrap();
    let mut next_scope = scope.clone();
    next_scope.definition_revision = newer.receipt().kernel_state_revision.get();
    let historical = complete(
        &runtime,
        query("historical-output", scope.definition_revision, source),
        &next_scope,
    );
    let latest = complete(
        &runtime,
        query("newest-output", next_scope.definition_revision, source),
        &next_scope,
    );
    let old = complete(
        &runtime,
        command(
            json!({"type":"result_inspect","request_id":"old-producer","result_ref":historical.payload["list"],"query":{"kind":"summary"}}),
        ),
        &next_scope,
    );
    let new = complete(
        &runtime,
        command(
            json!({"type":"result_inspect","request_id":"new-producer","result_ref":latest.payload["list"],"query":{"kind":"summary"}}),
        ),
        &next_scope,
    );
    assert_eq!(
        old.binding.0.as_ref().unwrap().kernel_state_revision.get(),
        scope.definition_revision
    );
    assert_eq!(
        new.binding.0.as_ref().unwrap().kernel_state_revision.get(),
        next_scope.definition_revision
    );
    assert_eq!(
        old.binding.0.as_ref().unwrap().freshness,
        ResultBindingFreshness::Stale
    );
    assert_eq!(before, accepted.state().bytes());
    println!(
        "actual notebook result reference={}; actual exact row={}, checkpoint bytes unchanged",
        token, inspected.payload["rows"][0]
    );
    runtime.take_thread().unwrap().join().unwrap();
    worker.begin_close().unwrap().unwrap().join().unwrap();
}
#[test]
fn actual_manifest_pages_source_numeric_and_readonly_controls_have_scoped_original_identity() {
    let (accepted, scope) = setup();
    let runtime = ResultRuntime::new().unwrap();
    runtime.accepted(accepted.clone()).unwrap();
    let manifest = complete(
        &runtime,
        command(
            json!({"type":"result_manifest","request_id":"manifest","root_result_id":accepted.receipt().result_id.0,"offset":0,"limit":16}),
        ),
        &scope,
    );
    assert_eq!(manifest.phase, NativeResultHostReplyPhase::Completed);
    let entry = manifest.payload["entries"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    let token = entry["binding"]["result_ref"].as_str().unwrap();
    let inspect = |id: &str, query: Value| {
        command(json!({"type":"result_inspect","request_id":id,"result_ref":token,"query":query}))
    };
    let page = complete(
        &runtime,
        inspect(
            "page",
            json!({"kind":"page","path":[],"offset":0,"limit":1,"column_offset":0,"column_limit":1}),
        ),
        &scope,
    );
    assert_eq!(page.payload["rows"][0]["cells"][0]["nature"], "exact");
    assert_eq!(
        page.payload["rows"][0]["cells"][0]["source"]["input_form"],
        "1/3"
    );
    let numeric = complete(
        &runtime,
        inspect("numeric", json!({"kind":"numeric","path":[0],"digits":50})),
        &scope,
    );
    assert!(
        numeric.payload["input_form"]
            .as_str()
            .unwrap()
            .contains("0.333333")
    );
    let source = complete(
        &runtime,
        inspect(
            "source",
            json!({"kind":"source","path":[2],"format":"input_form","byte_offset":0,"byte_limit":64}),
        ),
        &scope,
    );
    assert_eq!(source.payload["text"], "\"中🙂\"");
    assert_eq!(source.payload["complete"], true);
    let forbidden = complete(
        &runtime,
        inspect(
            "set",
            json!({"kind":"readonly_expression","source":"Set[a,9]","numeric":false}),
        ),
        &scope,
    );
    assert_eq!(forbidden.phase, NativeResultHostReplyPhase::Failed);
    let scratch = complete(
        &runtime,
        inspect(
            "scratch",
            json!({"kind":"scratch","source":"let a=9; a","dialect":"Modern"}),
        ),
        &scope,
    );
    assert_eq!(scratch.payload["effect_committed"], false);
    assert_eq!(
        scratch.payload["response"]["output"]["items"][0]["input_form"],
        "9"
    );
    let old = complete(
        &runtime,
        inspect(
            "original",
            json!({"kind":"readonly_expression","source":"a","numeric":false}),
        ),
        &scope,
    );
    assert_eq!(old.payload["input_form"], "2");
    let duplicate=runtime.command(inspect("page",json!({"kind":"page","path":[],"offset":0,"limit":1,"column_offset":0,"column_limit":1})),&scope).unwrap();
    assert_eq!(duplicate.payload, page.payload);
    let mut other = scope.clone();
    other.document = "other-doc".into();
    assert!(
        runtime
            .command(
                command(json!({"type":"result_status","request_id":"page"})),
                &other
            )
            .is_err()
    );
    other = scope.clone();
    other.grant_revision += 1;
    assert!(
        runtime
            .command(
                command(json!({"type":"result_status","request_id":"page"})),
                &other
            )
            .is_err()
    );
    assert!(
        runtime
            .command(inspect("page", json!({"kind":"summary"})), &scope)
            .is_err()
    );
    runtime.take_thread().unwrap().join().unwrap();
}
#[test]
fn result_read_schema_rejects_unknown_options_overlarge_pages_or_invented_channels() {
    assert!(decode_command(br#"{"type":"result_manifest","request_id":"read","root_result_id":"result","offset":0,"limit":33}"#).is_err());
    assert!(decode_command(br#"{"type":"result_inspect","request_id":"read","result_ref":"result-ref","query":{"kind":"numeric","path":[],"digits":20,"extra":true}}"#).is_err());
    assert!(decode_command(br#"{"type":"result_inspect","request_id":"read","result_ref":"result-ref","query":{"kind":"geometry_page","channel":"inferred_pixels","object_index":0,"segment_index":0,"offset":0,"limit":1}}"#).is_err());
}
#[test]
fn readonly_result_retention_does_not_keep_an_obsolete_main_pool_envelope_pinned() {
    let (accepted, _scope, worker) = owned_setup();
    let before = worker.reserved_bytes().unwrap();
    let record = om_host_service::results::StoredResult::capture(
        &accepted,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    worker.discard(accepted.registry_ref()).unwrap();
    assert_eq!(worker.reserved_bytes().unwrap(), before);
    drop(accepted);
    assert!(worker.reserved_bytes().unwrap() < before);
    assert!(!record.summary().statements.is_empty());
    worker.begin_close().unwrap().unwrap().join().unwrap();
}
struct Gate {
    state: Mutex<(bool, bool)>,
    signal: Condvar,
}
impl Gate {
    fn pause(&self) {
        let mut s = self.state.lock().unwrap();
        s.0 = true;
        self.signal.notify_all();
        let _guard = self
            .signal
            .wait_timeout_while(s, Duration::from_secs(10), |s| !s.1)
            .unwrap();
    }
    fn release(&self) {
        self.state.lock().unwrap().1 = true;
        self.signal.notify_all();
    }
}
#[test]
fn direct_cancel_and_readback_remain_available_while_a_real_result_worker_is_paused() {
    for notebook in [false, true] {
        let gate = Arc::new(Gate {
            state: Mutex::new((false, false)),
            signal: Condvar::new(),
        });
        let callback = gate.clone();
        let runtime = ResultRuntime::with_probes(ResultWorkerProbes {
            before_query: Some(Arc::new(move || callback.pause())),
        })
        .unwrap();
        let (accepted, scope) = setup();
        runtime.accepted(accepted.clone()).unwrap();
        let cmd = if notebook {
            command(
                json!({"type":"result_notebook_references","request_id":"paused","kernel_state_revision":scope.definition_revision,"cells":[{"cell_id":"list","source_hash":accepted.receipt().producer.cell_source_hash.0}]}),
            )
        } else {
            command(
                json!({"type":"result_manifest","request_id":"paused","root_result_id":accepted.receipt().result_id.0,"offset":0,"limit":1}),
            )
        };
        runtime.command(cmd, &scope).unwrap();
        let lock = gate.state.lock().unwrap();
        let (lock, timeout) = gate
            .signal
            .wait_timeout_while(lock, Duration::from_secs(10), |s| !s.0)
            .unwrap();
        assert!(!timeout.timed_out() && lock.0);
        drop(lock);
        let start = Instant::now();
        assert_eq!(
            runtime
                .command(
                    command(json!({"type":"result_status","request_id":"paused"})),
                    &scope
                )
                .unwrap()
                .phase,
            NativeResultHostReplyPhase::Running
        );
        assert_eq!(runtime.cancel("paused"), Some(true));
        assert!(start.elapsed() < Duration::from_secs(1));
        gate.release();
        let reply = complete(
            &runtime,
            command(json!({"type":"result_status","request_id":"paused"})),
            &scope,
        );
        assert_eq!(reply.phase, NativeResultHostReplyPhase::Cancelled);
        runtime.take_thread().unwrap().join().unwrap();
    }
}
