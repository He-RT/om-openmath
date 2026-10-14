//! Genuine KernelWorker/retained values; matching unit receipts below are synthetic, not disk IO.
use om_host_service::{
    agent::ingress::ToolIntent,
    document::SourceDocument,
    kernel::{
        acceptance::{AcceptanceContext, AcceptedKernelState, FrozenKernelPlan},
        worker::{KernelJob, KernelWorker},
    },
    protocol::{Serial, generated::*},
    references::ReferenceScope,
    results::{
        ResultStore, StoredResult,
        runtime::{ResultRuntime, ResultWorkerProbes},
    },
    tools::results::{FrozenInspection, InspectionContext, ResultInspectionService},
};
use om_kernel::{checkpoint::CheckpointLimits, config::GeneralConfig};
use om_num::ctx::Interrupt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    sync::{Arc, Condvar, Mutex, atomic::AtomicBool},
    time::{Duration, Instant},
};
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
fn actual(source: &str, dialect: Dialect) -> (AcceptedKernelState, ReferenceScope) {
    let snapshot = SourceDocument::new(
        "inspect-doc".into(),
        serial(1),
        NativeSourceFile {
            version: 1,
            title: "inspect".into(),
            cells: vec![NativeSourceCell {
                id: "value".into(),
                kind: NativeCellKind::Math,
                source: source.into(),
                dialect,
            }],
        },
    )
    .unwrap()
    .snapshot()
    .clone();
    let ctx = AcceptanceContext {
        runtime_instance_id: "runtime".into(),
        store_id: "store".into(),
        document_generation: serial(1),
        source: snapshot,
        general: GeneralConfig::default(),
        config_revision: serial(0),
        build: "inspect-fixture".into(),
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
            operation_id: "actual-producer".into(),
            parent_checkpoint_ref: parent.registry_ref().into(),
            source: ctx.source.clone(),
            general: ctx.general.clone(),
            config_revision: serial(0),
            cell_id: "value".into(),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .unwrap()
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let frozen = FrozenKernelPlan::candidate(candidate, &parent, &ctx, &worker).unwrap();
    frozen.enter_commit(&ctx, Some(&parent)).unwrap();
    let accepted = frozen.accept_receipt(&receipt(frozen.plan())).unwrap();
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
    (accepted, scope)
}
fn retained(a: &AcceptedKernelState, s: &ReferenceScope) -> (ResultStore, String) {
    let record =
        StoredResult::capture(a, CheckpointLimits::default(), &Interrupt::default()).unwrap();
    let id = record.id().to_owned();
    let tail = record.summary().statements.last().cloned();
    let mut store = ResultStore::new([6; 32], 64 * 1024 * 1024, 8).unwrap();
    store.register(record).unwrap();
    store.set_current(a, &Interrupt::default()).unwrap();
    let token = store
        .issue(
            &id,
            tail.as_ref().map_or(0, |r| r.out_index),
            tail.as_ref().map(|r| r.view_id.as_str()),
            s.clone(),
            0,
        )
        .unwrap();
    (store, token)
}
fn frozen(id: &str, args: &str) -> FrozenInspection {
    FrozenInspection::original(
        &ToolIntent {
            operation_id: id.into(),
            turn_id: "turn".into(),
            call_id: id.into(),
            proposal_event: "acked-original-proposal".into(),
            name: AgentToolName::InspectResult,
            arguments_hash: format!("{:x}", Sha256::digest(args.as_bytes())),
        },
        args,
    )
    .unwrap()
}
fn read(
    service: &mut ResultInspectionService,
    store: &ResultStore,
    scope: &ReferenceScope,
    args: Value,
    now: u64,
    ctx: &Interrupt,
) -> Value {
    service.execute(
        &frozen("inspect-request", &args.to_string()),
        InspectionContext {
            store,
            scope,
            now,
            interrupt: ctx,
        },
    )
}
fn pages(
    service: &mut ResultInspectionService,
    store: &ResultStore,
    scope: &ReferenceScope,
    token: &str,
    part: &str,
    limit: u32,
) -> Vec<Value> {
    let mut cursor = Value::Null;
    let mut all = vec![];
    loop {
        assert!(all.len() < 1000, "nonterminating actual pagination");
        let p = read(
            service,
            store,
            scope,
            json!({"result_ref":token,"part":part,"limit":limit,"cursor":cursor}),
            1,
            &Interrupt::default(),
        );
        assert_eq!(p["ok"], true, "{p}");
        cursor = p["pagination"]["next_cursor"].clone();
        all.push(p);
        if cursor.is_null() {
            break;
        }
    }
    all
}
#[test]
fn values_visit_all_matrix_columns_and_nested_exact_unicode_without_mutating_checkpoint() {
    let row = (1..=35)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let (a, s) = actual(
        &format!("seed_random(42);[[{row}],[{row}]]"),
        Dialect::Modern,
    );
    let before = a.state().bytes().to_vec();
    let (store, token) = retained(&a, &s);
    let mut service = ResultInspectionService::new([7; 32]);
    let p = pages(&mut service, &store, &s, &token, "value", 1);
    assert_eq!(p.len(), 4);
    let mut ids = BTreeSet::new();
    for page in &p {
        for row in page["data"]["payload"]["page"]["rows"].as_array().unwrap() {
            for cell in row["cells"].as_array().unwrap() {
                assert!(ids.insert(cell["id"].as_str().unwrap().to_owned()));
            }
        }
    }
    assert_eq!(ids.len(), 70);
    assert_eq!(p[1]["data"]["payload"]["page"]["column_offset"], 32);
    let (b, t) = actual(
        "[1/3,decimal(\"0.1\",precision:80),[\"中文🙂\",[4,5]]]",
        Dialect::Modern,
    );
    let (other, reference) = retained(&b, &t);
    let p = pages(&mut service, &other, &t, &reference, "value", 1);
    let leaves = p
        .iter()
        .flat_map(|p| p["data"]["payload"]["page"]["rows"].as_array().unwrap())
        .flat_map(|r| r["cells"].as_array().unwrap())
        .filter_map(|c| c["source"]["input_form"].as_str())
        .collect::<Vec<_>>();
    assert!(
        leaves.contains(&"1/3") && leaves.contains(&"\"中文🙂\"") && leaves.contains(&"5"),
        "{p:?}"
    );
    assert!(p.iter().any(
        |p| p["data"]["payload"]["page"]["rows"][0]["cells"][0]["nature"] == "high_precision"
    ));
    assert_eq!(a.state().bytes(), before);
    println!(
        "actual wide matrix={} windows,{} distinct leaves; nested={} pages",
        4,
        ids.len(),
        p.len()
    );
}
#[test]
fn actual_solver_summary_multiplicities_and_depth_first_recorded_steps_paginate_completely() {
    let (a, s) = actual("solve((x-1)^20==0,x)", Dialect::Modern);
    let (store, token) = retained(&a, &s);
    let mut service = ResultInspectionService::new([3; 32]);
    let summary = pages(&mut service, &store, &s, &token, "summary", 3);
    assert_eq!(summary.len(), 7);
    assert_eq!(
        summary
            .iter()
            .map(
                |p| p["data"]["payload"]["presentation"]["presentation"]["solutions"]
                    .as_array()
                    .unwrap()
                    .len()
            )
            .sum::<usize>(),
        20
    );
    assert_eq!(
        summary[0]["data"]["payload"]["presentation"]["presentation"]["solution_multiplicities"][0],
        20
    );
    let (a, s) = actual("solve(x^2-5*x+6==0,x)", Dialect::Modern);
    let before = a.state().bytes().to_vec();
    let (store, token) = retained(&a, &s);
    let result = store.resolve(&token, &s, 1).unwrap();
    let mut expected = vec![];
    let mut pending = vec![vec![]];
    while let Some(parent) = pending.pop() {
        let p = result
            .steps(&parent, 0, 100, &Interrupt::default())
            .unwrap();
        for e in p.entries.iter().rev() {
            if e.child_count > 0 {
                pending.push(e.path.clone());
            }
        }
        expected.extend(p.entries.into_iter().map(|e| (e.path, e.step.id)));
    }
    let pages = pages(&mut service, &store, &s, &token, "steps", 1);
    let mut got = BTreeSet::new();
    for p in &pages {
        assert_eq!(p["data"]["payload"]["recorded"], true);
        for e in p["data"]["payload"]["entries"].as_array().unwrap() {
            assert!(e["step"]["children"].as_array().unwrap().is_empty());
            got.insert((
                serde_json::from_value::<Vec<u32>>(e["path"].clone()).unwrap(),
                e["step"]["id"].as_str().unwrap().to_owned(),
            ));
        }
    }
    assert_eq!(got, expected.into_iter().collect());
    assert_eq!(a.state().bytes(), before);
    println!(
        "actual solver {} summary pages and {} real recorded steps",
        summary.len(),
        got.len()
    );
}
#[test]
fn ode_and_scene_expose_actual_original_data_and_preview_never_claims_rendered_media() {
    let (a, s) = actual(
        "let curve=ode(fn(t,y)=>y,initial:1,t:0..1);curve",
        Dialect::Modern,
    );
    let (store, token) = retained(&a, &s);
    let mut service = ResultInspectionService::new([11; 32]);
    let p = pages(&mut service, &store, &s, &token, "value", 20);
    assert!(!p.is_empty());
    assert_eq!(p[0]["data"]["payload"]["page"]["kind"], "record");
    let (a, s) = actual(
        "plot(x^2-y^2,x:-1..1,y:-1..1,view:\"surface\",mesh_points:8)",
        Dialect::Modern,
    );
    let before = a.state().bytes().to_vec();
    let (store, token) = retained(&a, &s);
    let p = pages(&mut service, &store, &s, &token, "geometry", 31);
    let mut count = 0;
    for page in &p {
        assert_eq!(page["data"]["render_state"], "data_only");
        let d = &page["data"]["payload"]["page"];
        if d["channel"] == "mesh_positions" {
            for v in d["values"].as_array().unwrap() {
                let x = v[0].as_f64().unwrap();
                let y = v[1].as_f64().unwrap();
                assert!((v[2].as_f64().unwrap() - (x * x - y * y)).abs() < 1e-12);
                count += 1;
            }
        }
    }
    assert_eq!(
        count,
        p[0]["data"]["payload"]["summary"]["metadata"]["meshes"][0]["vertices"]
            .as_u64()
            .unwrap() as usize
    );
    let preview = read(
        &mut service,
        &store,
        &s,
        json!({"result_ref":token,"part":"preview"}),
        1,
        &Interrupt::default(),
    );
    assert_eq!(preview["data"]["render_state"], "data_only");
    assert_eq!(preview["data"]["media_ref"], Value::Null);
    assert_eq!(preview["data"]["payload"]["preview_available"], false);
    assert_eq!(a.state().bytes(), before);
    println!(
        "actual surface={count} original vertices,{} geometry pages, no rendered media",
        p.len()
    );
}
#[test]
fn closed_original_arguments_exact_cursor_scope_history_expiry_and_cancellation_fail_truthfully() {
    let (a, s) = actual("[1/3,2,3]", Dialect::Modern);
    let (store, token) = retained(&a, &s);
    let mut service = ResultInspectionService::new([12; 32]);
    let first = read(
        &mut service,
        &store,
        &s,
        json!({"result_ref":token,"part":"value","limit":1}),
        1,
        &Interrupt::default(),
    );
    let cursor = &first["pagination"]["next_cursor"];
    assert!(cursor.is_string());
    for args in [
        json!({"result_ref":token,"part":null}),
        json!({"result_ref":token,"limit":0}),
        json!({"result_ref":token,"limit":101}),
        json!({"result_ref":token,"source":"1+1"}),
        json!({"result_ref":token,"part":"execute"}),
    ] {
        assert_eq!(
            read(&mut service, &store, &s, args, 1, &Interrupt::default())["error"]["code"],
            "INVALID_ARGUMENT"
        );
    }
    for args in [
        json!({"result_ref":token,"part":"steps","limit":1,"cursor":cursor}),
        json!({"result_ref":token,"part":"value","limit":2,"cursor":cursor}),
        json!({"result_ref":"forged","part":"value","limit":1,"cursor":cursor}),
    ] {
        assert_eq!(
            read(&mut service, &store, &s, args, 1, &Interrupt::default())["error"]["code"],
            "INVALID_REFERENCE"
        );
    }
    let mut later = s.clone();
    later.revision += 1;
    later.execution_epoch += 1;
    later.snapshot_hash = "b".repeat(64);
    assert_eq!(
        read(
            &mut service,
            &store,
            &later,
            json!({"result_ref":token}),
            2,
            &Interrupt::default()
        )["data"]["is_current"],
        false
    );
    assert_eq!(
        read(
            &mut service,
            &store,
            &later,
            json!({"result_ref":token,"part":"value","limit":1,"cursor":cursor}),
            2,
            &Interrupt::default()
        )["error"]["code"],
        "INVALID_REFERENCE"
    );
    for (index, mut bad) in [s.clone(), s.clone(), s.clone(), s.clone(), s.clone()]
        .into_iter()
        .enumerate()
    {
        match index {
            0 => bad.task = Some("other-task".into()),
            1 => bad.generation += 1,
            2 => bad.grant_revision += 1,
            3 => bad.runtime = "other-runtime".into(),
            _ => bad.metadata_revision += 1,
        }
        assert_eq!(
            read(
                &mut service,
                &store,
                &bad,
                json!({"result_ref":token}),
                1,
                &Interrupt::default()
            )["error"]["code"],
            "INVALID_REFERENCE"
        );
    }
    assert_eq!(
        read(
            &mut service,
            &store,
            &s,
            json!({"result_ref":token}),
            600000,
            &Interrupt::default()
        )["error"]["code"],
        "INVALID_REFERENCE"
    );
    let cancelled = Interrupt {
        flag: Arc::new(AtomicBool::new(true)),
        ..Interrupt::default()
    };
    assert_eq!(
        read(
            &mut service,
            &store,
            &s,
            json!({"result_ref":token}),
            1,
            &cancelled
        )["error"]["code"],
        "CANCELLED"
    );
    let budget = Interrupt {
        steps_left: std::cell::Cell::new(0),
        ..Interrupt::default()
    };
    assert_eq!(
        read(
            &mut service,
            &store,
            &s,
            json!({"result_ref":token}),
            1,
            &budget
        )["error"]["code"],
        "LIMIT_EXCEEDED"
    );
    let mut intent = ToolIntent {
        operation_id: "id".into(),
        turn_id: "turn".into(),
        call_id: "call".into(),
        proposal_event: "proposal".into(),
        name: AgentToolName::InspectResult,
        arguments_hash: "a".repeat(64),
    };
    assert!(FrozenInspection::original(&intent, "{}").is_err());
    intent.name = AgentToolName::RunCells;
    assert!(FrozenInspection::original(&intent, "{}").is_err());
}
#[test]
fn actual_partial_failure_unrecorded_steps_and_absent_geometry_remain_explicit() {
    let (a, s) = actual("let partial=7; loop:=loop; loop", Dialect::Modern);
    let before = a.state().bytes().to_vec();
    let (store, token) = retained(&a, &s);
    let mut service = ResultInspectionService::new([8; 32]);
    let diag = pages(&mut service, &store, &s, &token, "diagnostics", 1);
    assert_eq!(diag[0]["data"]["payload"]["status"], "Error");
    assert!(diag[0]["data"]["payload"]["total"].as_u64().unwrap() > 0);
    assert_eq!(a.state().bytes(), before);
    let (a, s) = actual("1/7", Dialect::Modern);
    let (store, token) = retained(&a, &s);
    assert_eq!(
        pages(&mut service, &store, &s, &token, "steps", 1)[0]["data"]["payload"]["recorded"],
        false
    );
    assert_eq!(
        pages(&mut service, &store, &s, &token, "preview", 1)[0]["data"]["render_state"],
        "unavailable"
    );
}
#[test]
fn independent_result_worker_accepts_original_identity_and_direct_cancel_without_main_execution() {
    let (a, s) = actual("1/3", Dialect::Modern);
    let before = a.state().bytes().to_vec();
    let gate = Arc::new((Mutex::new((false, false)), Condvar::new()));
    let pause = gate.clone();
    let runtime = ResultRuntime::with_probes(ResultWorkerProbes {
        before_query: Some(Arc::new(move || {
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
    let request = frozen(
        "actual-independent-inspect",
        r#"{"result_ref":"not-issued"}"#,
    );
    let queued = runtime.agent_inspect(request, &s).unwrap();
    assert_eq!(queued.phase, NativeResultHostReplyPhase::Queued);
    let (lock, cv) = &*gate;
    let p = lock.lock().unwrap();
    let (p, timeout) = cv
        .wait_timeout_while(p, Duration::from_secs(5), |p| !p.0)
        .unwrap();
    assert!(!timeout.timed_out());
    drop(p);
    assert!(
        runtime
            .agent_inspect(
                frozen(
                    "actual-independent-inspect",
                    r#"{"result_ref":"different"}"#
                ),
                &s
            )
            .is_err()
    );
    assert_eq!(runtime.cancel("actual-independent-inspect"), Some(true));
    lock.lock().unwrap().1 = true;
    cv.notify_all();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let p = runtime
            .command(
                NativeResultHostCommand::ResultStatus(ResultStatus {
                    r#type: ResultStatusType::ResultStatus,
                    request_id: "actual-independent-inspect".into(),
                }),
                &s,
            )
            .unwrap();
        if p.phase == NativeResultHostReplyPhase::Cancelled {
            break;
        }
        assert!(Instant::now() < end);
        std::thread::yield_now();
    }
    assert_eq!(a.state().bytes(), before);
    runtime.take_thread().unwrap().join().unwrap();
}
