//! Real CAS jobs, independent queues, terminal ordering and direct stop.
use om_host_service::{
    protocol::wire,
    scheduler::{NativeHost, Phase},
};
use serde_json::{Value, json};
use std::{
    thread,
    time::{Duration, Instant},
};
fn host(events: u32) -> NativeHost {
    NativeHost::new(wire::decode_init(&serde_json::to_vec(&json!({"protocol_version":1,"runtime_instance_id":"runtime-1","max_pending_operations":32,"event_capacity":events})).unwrap()).unwrap()).unwrap()
}
fn submit(host: &NativeHost, id: &str, body: Value) {
    let bytes=serde_json::to_vec(&json!({"protocol_version":1,"runtime_instance_id":"runtime-1","request_ref":id,"operation_id":id,"document_binding":null,"task_binding":null,"body":body})).unwrap();
    assert!(host.submit(&bytes).unwrap().accepted);
}
fn math(source: &str) -> Value {
    json!({"kind":"evaluate_scratch","source":source,"dialect":"Modern","definition_snapshot_ref":null,"use_notebook_definitions":false,"timeout_ms":10000})
}
fn settle(host: &NativeHost, id: &str) -> om_host_service::scheduler::OperationStatus {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = host.operation(id)
            && matches!(
                status.phase,
                Phase::Completed | Phase::Failed | Phase::Cancelled
            )
        {
            return status;
        }
        assert!(Instant::now() < deadline, "unsettled operation {id}");
        thread::sleep(Duration::from_millis(2));
    }
}
#[test]
fn actual_math_and_source_definitions_are_isolated_between_operations() {
    let host = host(32);
    submit(&host, "first", math("let a=5; a+1"));
    let first = settle(&host, "first");
    assert_eq!(first.phase, Phase::Completed);
    assert_eq!(
        first.result.unwrap()["response"]["output"]["items"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["input_form"],
        "6"
    );
    submit(&host, "second", math("a"));
    let result = settle(&host, "second").result.unwrap();
    assert_eq!(result["response"]["output"]["items"][0]["input_form"], "a");
    assert_eq!(result["effect_committed"], false);
    host.close_finish().unwrap();
}
#[test]
fn long_math_does_not_delay_state_read_and_direct_cancellation_settles() {
    let host = host(32);
    submit(&host, "long", math("map(fn(k)=>sin(k),range(1,100000))"));
    let running_deadline = Instant::now() + Duration::from_secs(2);
    while host.operation("long").unwrap().phase == Phase::Queued {
        assert!(Instant::now() < running_deadline);
        thread::yield_now();
    }
    assert_eq!(host.operation("long").unwrap().phase, Phase::Running);
    let started = Instant::now();
    submit(&host, "state", json!({"kind":"get_state"}));
    assert_eq!(settle(&host, "state").phase, Phase::Completed);
    assert!(started.elapsed() < Duration::from_secs(1));
    host.cancel("long").unwrap();
    assert_eq!(settle(&host, "long").phase, Phase::Cancelled);
    submit(&host, "later", math("2+2"));
    assert_eq!(settle(&host, "later").phase, Phase::Completed);
    assert!(!host.cancel("later").unwrap());
    assert_eq!(host.operation("later").unwrap().phase, Phase::Completed);
    host.close_finish().unwrap();
}
#[test]
fn slow_subscriber_gets_explicit_resync_and_retained_outcome() {
    let host = host(1);
    submit(&host, "one", math("1+1"));
    settle(&host, "one");
    submit(&host, "two", math("2+2"));
    settle(&host, "two");
    let batch = host.next_events(0, 512 * 1024).unwrap();
    assert!(batch.needs_resync);
    assert_eq!(host.operation("one").unwrap().phase, Phase::Completed);
    assert!(host.next_events(101, 512 * 1024).is_err());
    host.close_finish().unwrap();
}
#[test]
fn close_stops_admission_and_waits_only_on_the_finishing_caller() {
    let host = host(1);
    submit(&host, "long", math("map(fn(k)=>sin(k),range(1,100000))"));
    let start = Instant::now();
    host.close_begin();
    assert!(start.elapsed() < Duration::from_secs(1));
    let bytes=serde_json::to_vec(&json!({"protocol_version":1,"runtime_instance_id":"runtime-1","request_ref":"late","operation_id":null,"document_binding":null,"task_binding":null,"body":{"kind":"get_state"}})).unwrap();
    assert!(host.submit(&bytes).is_err());
    host.close_finish().unwrap();
    assert_eq!(host.operation("long").unwrap().phase, Phase::Cancelled);
}

#[test]
fn completed_reads_do_not_consume_active_capacity_or_recursively_embed_old_results() {
    let host = host(4);
    for i in 0..70 {
        let id = format!("read-{i}");
        submit(&host, &id, json!({"kind":"get_state"}));
        let status = settle(&host, &id);
        assert_eq!(status.phase, Phase::Completed);
        let result = status.result.unwrap();
        assert!(serde_json::to_vec(&result).unwrap().len() < 32 * 1024);
        for operation in result["operations"].as_array().unwrap() {
            assert!(operation["result"].is_null());
        }
    }
    host.close_finish().unwrap();
}

#[test]
fn rejected_stale_scope_and_unknown_operation_do_not_start_a_job() {
    let host = host(4);
    let value = json!({"protocol_version":1,"runtime_instance_id":"old-runtime","request_ref":"stale","operation_id":"stale","document_binding":null,"task_binding":null,"body":math("2+2")});
    assert!(host.submit(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(host.operation("stale").is_none());
    assert!(host.cancel("unknown").is_err());
    host.close_finish().unwrap();
}

#[test]
fn editor_preview_runs_on_an_independent_owner_during_real_math() {
    use sha2::{Digest, Sha256};
    let host = host(32);
    submit(&host, "long", math("map(fn(k)=>sin(k),range(1,100000))"));
    let source = "let f(x)=\n x+1";
    let key = json!({"runtime_instance_id":"runtime-1","document_id":"doc-editor-snapshot","document_generation":1,"cell_id":"cell-1","editor_id":"editor-1","editor_generation":1,"draft_sequence":1,"source_hash":format!("{:x}",Sha256::digest(source.as_bytes())),"requested_dialect":"Modern","effective_dialect":"Modern","analysis_generation":1,"metadata_version":26,"definition_snapshot_revision":0,"config_revision":0,"cursor_utf8":null,"selection_utf16":{"location":0,"length":0}});
    submit(
        &host,
        "editor",
        json!({"kind":"analyze_editor","key":key,"source":source,"analysis_kind":"preview","non_evaluating":true}),
    );
    let result = settle(&host, "editor");
    assert_eq!(result.phase, Phase::Completed);
    assert_eq!(result.result.unwrap()["response"]["type"], "preview");
    host.cancel("long").unwrap();
    settle(&host, "long");
    host.close_finish().unwrap();
}

#[test]
fn invalid_direct_init_and_source_provenance_are_rejected() {
    let mut init = wire::decode_init(br#"{"protocol_version":1,"runtime_instance_id":"runtime-1","max_pending_operations":32,"event_capacity":1}"#).unwrap();
    init.event_capacity = 0;
    assert!(NativeHost::new(init).is_err());
    let host = host(4);
    let source = "😀 + 2";
    let key = json!({"runtime_instance_id":"runtime-1","document_id":"doc-editor-snapshot","document_generation":1,"cell_id":"cell-1","editor_id":"editor-1","editor_generation":1,"draft_sequence":1,"source_hash":"0".repeat(64),"requested_dialect":"Modern","effective_dialect":"Modern","analysis_generation":1,"metadata_version":26,"definition_snapshot_revision":0,"config_revision":0,"cursor_utf8":null,"selection_utf16":{"location":0,"length":0}});
    submit(
        &host,
        "stale-source",
        json!({"kind":"analyze_editor","key":key,"source":source,"analysis_kind":"preview","non_evaluating":true}),
    );
    let status = settle(&host, "stale-source");
    assert_eq!(status.phase, Phase::Failed);
    assert_eq!(status.error_code.as_deref(), Some("STALE_SOURCE"));
    host.close_finish().unwrap();
}

#[test]
fn overload_queued_stop_and_close_are_bounded_and_terminal() {
    let init = wire::decode_init(br#"{"protocol_version":1,"runtime_instance_id":"runtime-1","max_pending_operations":2,"event_capacity":2}"#).unwrap();
    let host = host_with_init(init);
    submit(&host, "long", math("map(fn(k)=>sin(k),range(1,100000))"));
    let deadline = Instant::now() + Duration::from_secs(2);
    while host.operation("long").unwrap().phase == Phase::Queued {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    submit(&host, "queued", math("let should_not_run=8;should_not_run"));
    let third=serde_json::to_vec(&json!({"protocol_version":1,"runtime_instance_id":"runtime-1","request_ref":"overload","operation_id":"overload","document_binding":null,"task_binding":null,"body":math("2+2")})).unwrap();
    assert_eq!(host.submit(&third).unwrap_err(), "BUDGET_EXCEEDED");
    assert!(host.operation("overload").is_none());
    assert!(host.cancel("queued").unwrap());
    host.close_begin();
    host.close_finish().unwrap();
    for id in ["long", "queued"] {
        assert_eq!(host.operation(id).unwrap().phase, Phase::Cancelled);
        assert!(!host.cancel(id).unwrap());
    }
}
fn host_with_init(init: om_host_service::protocol::generated::HostInit) -> NativeHost {
    NativeHost::new(init).unwrap()
}

#[test]
fn actual_catalog_pagination_and_failed_math_never_claim_success() {
    let host = host(32);
    let expected = om_kernel::capabilities::function_catalog();
    let mut offset = 0;
    let mut collected = Vec::new();
    loop {
        let id = format!("catalog-{offset}");
        submit(
            &host,
            &id,
            json!({"kind":"get_function_catalog","offset":offset,"limit":32}),
        );
        let status = settle(&host, &id);
        assert_eq!(status.phase, Phase::Completed);
        let result = status.result.unwrap();
        assert!(serde_json::to_vec(&result).unwrap().len() <= 128 * 1024);
        collected.extend(
            result["catalog"]["functions"]
                .as_array()
                .unwrap()
                .iter()
                .cloned(),
        );
        match result["next_offset"].as_u64() {
            Some(next) => offset = next,
            None => break,
        }
    }
    assert_eq!(
        collected,
        serde_json::to_value(expected.functions)
            .unwrap()
            .as_array()
            .unwrap()
            .clone()
    );
    submit(&host, "cap", json!({"kind":"get_capabilities"}));
    let cap = settle(&host, "cap").result.unwrap();
    assert_eq!(cap["kernel"]["platform"], "desktop");
    assert_eq!(cap["native_renderers_ready"], false);
    submit(&host, "parse-error", math("let f(x)=;"));
    let result = settle(&host, "parse-error").result.unwrap();
    assert_eq!(result["run_outcome"], "failed");
    assert_eq!(result["effect_committed"], false);
    host.close_finish().unwrap();
}
