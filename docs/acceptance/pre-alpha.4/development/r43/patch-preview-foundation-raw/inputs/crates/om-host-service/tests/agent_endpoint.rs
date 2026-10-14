//! Actual source-owned command port: no synthetic document scope is supplied by peer activation.
use om_host_service::{
    agent::endpoint::decode_command,
    document::{SourceDocument, endpoint::SourceEndpoint},
    protocol::{Serial, generated::*},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn scope(generation: u64) -> Value {
    json!({"agent_session_id":"session","task_id":"task","task_generation":generation,"connection_id":"connection","connection_generation":generation,"document_generation":1})
}
fn endpoint() -> SourceEndpoint {
    let owner = SourceDocument::new(
        "actual-document".into(),
        Serial::new(1).unwrap(),
        NativeSourceFile {
            version: 1,
            title: "实际文档".into(),
            cells: vec![NativeSourceCell {
                id: "a".into(),
                kind: NativeCellKind::Math,
                source: "2+2".into(),
                dialect: Dialect::Modern,
            }],
        },
    )
    .unwrap();
    SourceEndpoint::open(
        owner.snapshot().clone(),
        "actual-store".into(),
        "actual-runtime".into(),
        Serial::new(1).unwrap(),
        None,
        Serial::new(0).unwrap(),
    )
    .unwrap()
}
fn command(owner: &mut SourceEndpoint, v: Value) -> Result<NativeAgentHostReply, String> {
    owner.agent_command(decode_command(&serde_json::to_vec(&v).unwrap())?)
}
fn activate(owner: &mut SourceEndpoint, generation: u64, readonly: bool) -> Value {
    let reply=command(owner,json!({"type":"agent_activate","scope":scope(generation),"hello_id":"hello","core_version":"1.0.4","adapter_version":"adapter-1","grants":["read_notebook","run_cells"],"readonly":readonly})).unwrap();
    assert_eq!(reply.document_id, "actual-document");
    assert_eq!(reply.runtime_instance_id, "actual-runtime");
    serde_json::from_str(reply.outbound_json.0.as_ref().unwrap()).unwrap()
}
fn frame(hello: &Value, kind: &str, sequence: u64, payload: Value) -> Value {
    let mut v = hello.clone();
    v["type"] = json!(kind);
    v["sequence"] = json!(sequence);
    v["message_id"] = json!(format!("peer-{sequence}"));
    v["reply_to"] = Value::Null;
    v["payload"] = payload;
    v
}
fn receive(
    owner: &mut SourceEndpoint,
    v: &Value,
    generation: u64,
) -> Result<NativeAgentHostReply, String> {
    command(
        owner,
        json!({"type":"agent_receive","scope":scope(generation),"frame_json":serde_json::to_string(v).unwrap()}),
    )
}
fn ready(owner: &mut SourceEndpoint, hello: &Value, generation: u64) {
    let mut v = frame(hello, "ready", 1, hello["payload"].clone());
    v["reply_to"] = hello["message_id"].clone();
    assert_eq!(
        receive(owner, &v, generation).unwrap().phase,
        NativeAgentHostReplyPhase::Ready
    );
}
fn preview_task(owner: &mut SourceEndpoint) -> Value {
    let hello=command(owner,json!({"type":"agent_activate","scope":scope(1),"hello_id":"hello","core_version":"1.0.4","adapter_version":"adapter-1","grants":["read_notebook","preview_source","apply_notebook_patch"],"readonly":false})).unwrap();
    let hello: Value = serde_json::from_str(hello.outbound_json.0.as_ref().unwrap()).unwrap();
    ready(owner, &hello, 1);
    hello
}
/// These are unit-level synthetic ledger acknowledgements. Native Swift fixtures separately prove
/// the original SQLite receipt path; this helper does not claim a physical storage commit.
fn data_call(
    owner: &mut SourceEndpoint,
    hello: &Value,
    sequence: &mut u64,
    name: &str,
    args: Value,
) -> NativeAgentReadonlyResult {
    let event = format!("proposal-{}", *sequence);
    let call = format!("call-{}", *sequence);
    let proposal = frame(
        hello,
        "transcript_event",
        *sequence,
        json!({"event_id":event,"source_sequence":*sequence/2,"message":{"record_type":"canonical_agent_message","message_version":1,"message_id":event,"role":"assistant","agent_turn_id":"turn","task_id":"task","parts":[{"kind":"tool_call","part_id":call,"tool_call_id":call,"name":name,"arguments_json":serde_json::to_string(&args).unwrap()}]}}),
    );
    let pending = receive(owner, &proposal, 1).unwrap();
    command(owner,json!({"type":"agent_confirm","scope":scope(1),"message_id":proposal["message_id"],"event_id":event,"canonical_hash":format!("{:x}",Sha256::digest(pending.canonical_json.0.unwrap().as_bytes())),"event_sequence":*sequence,"receipt_hash":"a".repeat(64)})).unwrap();
    *sequence += 1;
    let mapped = receive(
        owner,
        &frame(
            hello,
            "tool_call",
            *sequence,
            json!({"agent_turn_id":"turn","tool_call_id":call,"proposal_event_ref":event}),
        ),
        1,
    )
    .unwrap()
    .tool_intent
    .0
    .unwrap();
    *sequence += 1;
    let query =
        json!({"type":"agent_read_tool","scope":scope(1),"operation_id":mapped.operation_id});
    let result = command(owner, query.clone())
        .unwrap()
        .readonly_result
        .0
        .unwrap();
    assert_eq!(
        result.result_json,
        command(owner, query)
            .unwrap()
            .readonly_result
            .0
            .unwrap()
            .result_json
    );
    assert_eq!(
        result.result_hash,
        format!("{:x}", Sha256::digest(result.result_json.as_bytes()))
    );
    result
}
#[test]
fn agent_preview_consumes_exact_read_issued_scope_without_widening_source_exposure() {
    let mut owner = endpoint();
    let hello = preview_task(&mut owner);
    let mut sequence = 2;
    let before = owner.binding();
    let outline = data_call(
        &mut owner,
        &hello,
        &mut sequence,
        "read_notebook",
        json!({}),
    );
    let outline: Value = serde_json::from_str(&outline.result_json).unwrap();
    let patch = |reference: &Value| json!({"snapshot_ref":reference,"input":{"kind":"patch","operations":[{"type":"update_cell","target":{"cell_id":"a"},"expected_source_hash":format!("{:x}",Sha256::digest(b"2+2")),"source":"3+3"}]}});
    let denied = data_call(
        &mut owner,
        &hello,
        &mut sequence,
        "preview_source",
        patch(&outline["data"]["snapshot_ref"]),
    );
    let denied: Value = serde_json::from_str(&denied.result_json).unwrap();
    assert_eq!(denied["error"]["code"], "INVALID_PATCH");
    assert_eq!(denied["error"]["recovery"], "refresh_snapshot");
    let full = data_call(
        &mut owner,
        &hello,
        &mut sequence,
        "read_notebook",
        json!({"view":"cells","cell_ids":["a"]}),
    );
    let full: Value = serde_json::from_str(&full.result_json).unwrap();
    assert_eq!(full["data"]["cells"][0]["source"], "2+2");
    let valid = data_call(
        &mut owner,
        &hello,
        &mut sequence,
        "preview_source",
        patch(&full["data"]["snapshot_ref"]),
    );
    let valid: Value = serde_json::from_str(&valid.result_json).unwrap();
    assert_eq!(valid["data"]["valid"], true);
    assert!(
        valid["data"]["preview_ref"]
            .as_str()
            .unwrap()
            .starts_with("preview-")
    );
    assert_eq!(valid["operation_ref"], Value::Null);
    // The default manual port has task=None; it cannot consume this real Agent-bound plan.
    assert!(
        owner
            .command(
                serde_json::from_value(
                    json!({"type":"source_begin","preview_ref":valid["data"]["preview_ref"]})
                )
                .unwrap()
            )
            .is_err()
    );
    let original_outline = data_call(
        &mut owner,
        &hello,
        &mut sequence,
        "preview_source",
        patch(&outline["data"]["snapshot_ref"]),
    );
    let original_outline: Value = serde_json::from_str(&original_outline.result_json).unwrap();
    assert_eq!(original_outline["status"], "failed");
    assert_eq!(owner.binding().document_revision, before.document_revision);
    assert_eq!(owner.binding().execution_epoch, before.execution_epoch);
    println!(
        "Actual Agent read/preview envelopes={}",
        json!({"outline":outline,"denied":denied,"full":full,"valid":valid,"original_outline_still_denied":original_outline})
    );
}
#[test]
fn agent_source_inspection_is_noncommittable_and_failed_whole_patch_has_no_effect() {
    let mut owner = endpoint();
    let hello = preview_task(&mut owner);
    let mut sequence = 2;
    let full = data_call(
        &mut owner,
        &hello,
        &mut sequence,
        "read_notebook",
        json!({"view":"cells","cell_ids":["a"]}),
    );
    let full: Value = serde_json::from_str(&full.result_json).unwrap();
    let reference = full["data"]["snapshot_ref"].clone();
    let source = data_call(
        &mut owner,
        &hello,
        &mut sequence,
        "preview_source",
        json!({"snapshot_ref":reference,"input":{"kind":"source","source":"rand()","dialect":"Modern"}}),
    );
    let source: Value = serde_json::from_str(&source.result_json).unwrap();
    assert_eq!(source["status"], "completed");
    assert_eq!(source["data"]["preview_kind"], "source");
    assert!(source["data"]["preview_ref"].is_null());
    let failed = data_call(
        &mut owner,
        &hello,
        &mut sequence,
        "preview_source",
        json!({"snapshot_ref":reference,"input":{"kind":"patch","operations":[{"type":"update_cell","target":{"cell_id":"a"},"expected_source_hash":format!("{:x}",Sha256::digest(b"2+2")),"source":"3+3"},{"type":"delete_cell","target":{"cell_id":"missing"},"expected_source_hash":"0".repeat(64)}]}}),
    );
    let failed: Value = serde_json::from_str(&failed.result_json).unwrap();
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["error"]["code"], "INVALID_PATCH");
    let extra = data_call(
        &mut owner,
        &hello,
        &mut sequence,
        "preview_source",
        json!({"snapshot_ref":reference,"input":{"kind":"source","source":"2+2","dialect":"Modern"},"doc":"forged"}),
    );
    let extra: Value = serde_json::from_str(&extra.result_json).unwrap();
    assert_eq!(extra["error"]["code"], "INVALID_ARGUMENT");
    let unchanged = data_call(
        &mut owner,
        &hello,
        &mut sequence,
        "read_notebook",
        json!({"view":"cells","cell_ids":["a"]}),
    );
    let unchanged: Value = serde_json::from_str(&unchanged.result_json).unwrap();
    assert_eq!(unchanged["data"]["cells"][0]["source"], "2+2");
    assert_eq!(unchanged["document"]["revision"], 0);
    println!(
        "Actual Agent non-effect preview={}",
        json!({"source":source,"failed":failed,"extra":extra,"unchanged":unchanged})
    );
}
#[test]
fn actual_readonly_producer_requires_acked_original_arguments_and_durable_confirmation() {
    for (name, args, expected) in [
        (
            "read_notebook",
            "{ \"view\" : \"cells\", \"cell_ids\":[\"a\"] }",
            "completed",
        ),
        ("search_functions", "{\"query\":\"sin\"}", "completed"),
        ("get_function_docs", "{\"name\":\"sin\"}", "completed"),
        ("read_notebook", "{\"limit\":0}", "failed"),
        ("read_notebook", "{\"limit\":1,\"limit\":2}", "failed"),
    ] {
        let mut owner = endpoint();
        let hello=command(&mut owner,json!({"type":"agent_activate","scope":scope(1),"hello_id":"hello","core_version":"1.0.4","adapter_version":"adapter-1","grants":[name],"readonly":true})).unwrap();
        let hello: Value = serde_json::from_str(hello.outbound_json.0.as_ref().unwrap()).unwrap();
        ready(&mut owner, &hello, 1);
        let query = |operation: &str| json!({"type":"agent_read_tool","scope":scope(1),"operation_id":operation});
        assert!(command(&mut owner, query("invented")).is_err());
        let event = frame(
            &hello,
            "transcript_event",
            2,
            json!({"event_id":"proposal","source_sequence":1,"message":{"record_type":"canonical_agent_message","message_version":1,"message_id":"canonical","role":"assistant","agent_turn_id":"turn","task_id":"task","parts":[{"kind":"tool_call","part_id":"part","tool_call_id":"call","name":name,"arguments_json":args}]}}),
        );
        // Duplicate raw arguments cannot pass canonical framing. Test that rejection separately.
        if args.contains("\"limit\":1") {
            assert!(receive(&mut owner, &event, 1).is_err());
            continue;
        }
        let pending = receive(&mut owner, &event, 1).unwrap();
        let call = frame(
            &hello,
            "tool_call",
            3,
            json!({"agent_turn_id":"turn","tool_call_id":"call","proposal_event_ref":"proposal"}),
        );
        assert!(receive(&mut owner, &call, 1).is_err());
        command(&mut owner,json!({"type":"agent_confirm","scope":scope(1),"message_id":"peer-2","event_id":"proposal","canonical_hash":format!("{:x}",Sha256::digest(pending.canonical_json.0.unwrap().as_bytes())),"event_sequence":1,"receipt_hash":"a".repeat(64)})).unwrap();
        let intent = receive(&mut owner, &call, 1)
            .unwrap()
            .tool_intent
            .0
            .unwrap();
        let mut overridden = query(&intent.operation_id);
        overridden["arguments_json"] = json!("{}");
        assert!(command(&mut owner, overridden).is_err());
        assert_eq!(
            intent.arguments_hash,
            format!("{:x}", Sha256::digest(args.as_bytes()))
        );
        let read = command(&mut owner, query(&intent.operation_id))
            .unwrap()
            .readonly_result
            .0
            .unwrap();
        assert_eq!(
            read.result_json,
            command(&mut owner, query(&intent.operation_id))
                .unwrap()
                .readonly_result
                .0
                .unwrap()
                .result_json
        );
        let envelope: Value = serde_json::from_str(&read.result_json).unwrap();
        assert_eq!(envelope["status"], expected);
        assert!(envelope["operation_ref"].is_null());
        if name == "read_notebook" && expected == "completed" {
            assert_eq!(envelope["data"]["cells"][0]["source"], "2+2");
        }
        let payload = json!({"agent_turn_id":"turn","tool_call_id":"call","operation_ref":null,"result_ref":read.result_ref,"result_json":read.result_json,"outcome":expected});
        let wire = frame(&hello, "tool_result", 3, payload);
        let send = |owner: &mut SourceEndpoint, v: &Value| {
            command(
                owner,
                json!({"type":"agent_send","scope":scope(1),"frame_json":serde_json::to_string(v).unwrap()}),
            )
        };
        assert!(send(&mut owner, &wire).is_err());
        let mut confirm = json!({"type":"agent_confirm_tool","scope":scope(1),"operation_id":intent.operation_id,"result_ref":read.result_ref,"result_hash":read.result_hash,"event_id":format!("readonly-{}",intent.operation_id),"event_sequence":2,"receipt_hash":"b".repeat(64)});
        confirm["result_hash"] = json!("0".repeat(64));
        assert!(command(&mut owner, confirm.clone()).is_err());
        confirm["result_hash"] = json!(read.result_hash);
        command(&mut owner, confirm.clone()).unwrap();
        command(&mut owner, confirm).unwrap();
        let mut forged = wire.clone();
        forged["payload"]["result_json"] = json!("{}");
        assert!(send(&mut owner, &forged).is_err());
        let sent = send(&mut owner, &wire).unwrap();
        assert_eq!(
            sent.outbound_json.0,
            send(&mut owner, &wire).unwrap().outbound_json.0
        );
        owner.command(serde_json::from_value(json!({"type":"source_editor_changed","state":{"runtime_instance_id":"actual-runtime","document_id":"actual-document","document_generation":1,"source_revision":0,"targets":[]}})).unwrap()).unwrap();
        assert!(command(&mut owner, query(&intent.operation_id)).is_err());
        assert!(send(&mut owner, &wire).is_err()); // exact already-sent frame cannot bypass changed scope
        command(
            &mut owner,
            json!({"type":"agent_cancel","scope":scope(1),"reason":"user_stop"}),
        )
        .unwrap();
        assert!(command(&mut owner, query(&intent.operation_id)).is_err());
        println!(
            "Actual readonly {name} original args={args}; result={}",
            read.result_json
        );
    }
}
#[test]
fn durable_confirmation_replay_and_document_close_revoke_real_owner_authority() {
    let mut owner = endpoint();
    let hello = activate(&mut owner, 1, false);
    ready(&mut owner, &hello, 1);
    let message = json!({"record_type":"canonical_agent_message","message_version":1,"message_id":"canonical-1","role":"assistant","agent_turn_id":"turn","task_id":"task","parts":[{"kind":"text","part_id":"text","text":"完整 中文🙂 / é"},{"kind":"tool_call","part_id":"call","tool_call_id":"model-call","name":"read_notebook","arguments_json":"{}"}]});
    let event = frame(
        &hello,
        "transcript_event",
        2,
        json!({"event_id":"event","source_sequence":1,"message":message}),
    );
    let pending = receive(&mut owner, &event, 1).unwrap();
    assert!(pending.outbound_json.0.is_none());
    assert!(receive(&mut owner,&frame(&hello,"tool_call",3,json!({"agent_turn_id":"turn","tool_call_id":"model-call","proposal_event_ref":"event"})),1).is_err());
    let canonical = pending.canonical_json.0.unwrap();
    let hash = format!("{:x}", Sha256::digest(canonical.as_bytes()));
    let mut confirm = json!({"type":"agent_confirm","scope":scope(1),"message_id":"peer-2","event_id":"event","canonical_hash":"0".repeat(64),"event_sequence":1,"receipt_hash":"a".repeat(64)});
    assert!(command(&mut owner, confirm.clone()).is_err());
    confirm["canonical_hash"] = json!(hash);
    let ack = command(&mut owner, confirm.clone()).unwrap();
    assert_eq!(ack.disposition, NativeAgentHostReplyDisposition::Ack);
    assert_eq!(
        ack.outbound_json.0,
        command(&mut owner, confirm).unwrap().outbound_json.0
    );
    let replay = receive(&mut owner, &event, 1).unwrap();
    assert_eq!(replay.outbound_json.0, ack.outbound_json.0);
    assert!(replay.canonical_json.0.is_none());
    let call = frame(
        &hello,
        "tool_call",
        3,
        json!({"agent_turn_id":"turn","tool_call_id":"model-call","proposal_event_ref":"event"}),
    );
    let mapped = receive(&mut owner, &call, 1)
        .unwrap()
        .tool_intent
        .0
        .unwrap();
    assert_ne!(mapped.operation_id, mapped.tool_call_id);
    let mut wrong = call.clone();
    wrong["document_id"] = json!("other-document");
    assert!(receive(&mut owner, &wrong, 1).is_err());
    owner.close_kernel_begin();
    assert!(receive(&mut owner, &call, 1).is_err());
    let new_hello = activate(&mut owner, 2, true);
    ready(&mut owner, &new_hello, 2);
    assert!(receive(&mut owner, &call, 2).is_err());
    let fresh_call = frame(
        &new_hello,
        "tool_call",
        2,
        json!({"agent_turn_id":"turn","tool_call_id":"model-call","proposal_event_ref":"event"}),
    );
    assert!(receive(&mut owner, &fresh_call, 2).is_err()); // recovered proposal never recreates grants
}
#[test]
fn request_ids_are_correlated_without_dispatch_and_peer_model_streams_are_denied() {
    let mut owner = endpoint();
    let hello = activate(&mut owner, 1, false);
    ready(&mut owner, &hello, 1);
    let request = |turn: &str, context: &str| json!({"agent_turn_id":turn,"request_id":"provider-request","context_ref":context,"model_config_ref":"model-config"});
    assert!(
        receive(
            &mut owner,
            &frame(&hello, "model_request", 2, request("turn", "context")),
            1
        )
        .is_err()
    );
    let prepare = frame(
        &hello,
        "prepare_request",
        2,
        json!({"agent_turn_id":"turn","request_id":"provider-request"}),
    );
    let original = receive(&mut owner, &prepare, 1)
        .unwrap()
        .request_intent
        .0
        .unwrap();
    assert_ne!(original.operation_id, original.request_id);
    assert!(
        receive(
            &mut owner,
            &frame(&hello, "model_request", 3, request("other-turn", "context")),
            1
        )
        .is_err()
    );
    let binding = frame(&hello, "model_request", 2, request("turn", "context"));
    command(&mut owner,json!({"type":"agent_send","scope":scope(1),"frame_json":serde_json::to_string(&binding).unwrap()})).unwrap();
    let mapped = receive(
        &mut owner,
        &frame(&hello, "model_request", 3, request("turn", "context")),
        1,
    )
    .unwrap()
    .request_intent
    .0
    .unwrap();
    assert_eq!(mapped.operation_id, original.operation_id);
    assert!(
        receive(
            &mut owner,
            &frame(&hello, "model_request", 4, request("turn", "other-context")),
            1
        )
        .is_err()
    );
    assert!(receive(&mut owner,&frame(&hello,"model_event",4,json!({"agent_turn_id":"turn","request_id":"provider-request","byte_offset":0,"text":"claimed output","phase":"delta","canonical_message":null})),1).is_err());
    command(
        &mut owner,
        json!({"type":"agent_cancel","scope":scope(1),"reason":"user_stop"}),
    )
    .unwrap();
    assert!(
        receive(
            &mut owner,
            &frame(
                &hello,
                "prepare_request",
                4,
                json!({"agent_turn_id":"turn","request_id":"new-request"})
            ),
            1
        )
        .is_err()
    );
    assert_eq!(
        receive(
            &mut owner,
            &frame(&hello, "settled", 4, json!({"reason":"cancelled"})),
            1
        )
        .unwrap()
        .phase,
        NativeAgentHostReplyPhase::AwaitingSettlement
    );
}
#[test]
fn native_envelope_budget_preserves_raw_frame_and_rejects_duplicate_fields() {
    let mut owner = endpoint();
    let hello = activate(&mut owner, 1, false);
    ready(&mut owner, &hello, 1);
    let text = "\"".repeat(200_000);
    let parts: Vec<Value> = (0..2)
        .map(|i| json!({"kind":"text","part_id":format!("part-{i}"),"text":text}))
        .collect();
    let event = frame(
        &hello,
        "transcript_event",
        2,
        json!({"event_id":"large-event","source_sequence":1,"message":{"record_type":"canonical_agent_message","message_version":1,"message_id":"large-message","role":"assistant","agent_turn_id":"turn","task_id":"task","parts":parts}}),
    );
    let raw = serde_json::to_string(&event).unwrap();
    let control =
        serde_json::to_vec(&json!({"type":"agent_receive","scope":scope(1),"frame_json":raw}))
            .unwrap();
    assert!(control.len() > 1024 * 1024 && control.len() < 2 * 1024 * 1024);
    assert_eq!(
        owner
            .agent_command(decode_command(&control).unwrap())
            .unwrap()
            .disposition,
        NativeAgentHostReplyDisposition::Transcript
    );
    let duplicate = b"{\"type\":\"agent_status\",\"type\":\"agent_close\",\"scope\":{}}";
    assert!(decode_command(duplicate).is_err());
}

#[test]
fn host_stream_has_exact_offsets_one_terminal_and_no_fabricated_tool_receipt() {
    let mut owner = endpoint();
    let hello = activate(&mut owner, 1, false);
    ready(&mut owner, &hello, 1);
    receive(
        &mut owner,
        &frame(
            &hello,
            "prepare_request",
            2,
            json!({"agent_turn_id":"turn","request_id":"request"}),
        ),
        1,
    )
    .unwrap();
    let send = |owner: &mut SourceEndpoint, v: Value| {
        command(
            owner,
            json!({"type":"agent_send","scope":scope(1),"frame_json":serde_json::to_string(&v).unwrap()}),
        )
    };
    let delta = |sequence, offset, text| {
        frame(
            &hello,
            "model_event",
            sequence,
            json!({"agent_turn_id":"turn","request_id":"request","byte_offset":offset,"text":text,"phase":"delta","canonical_message":null}),
        )
    };
    assert!(send(&mut owner, delta(2, 0, "🙂")).is_err());
    send(&mut owner,frame(&hello,"model_request",2,json!({"agent_turn_id":"turn","request_id":"request","context_ref":"native-context","model_config_ref":"native-config"}))).unwrap();
    receive(&mut owner,&frame(&hello,"model_request",3,json!({"agent_turn_id":"turn","request_id":"request","context_ref":"native-context","model_config_ref":"native-config"})),1).unwrap();
    send(&mut owner, delta(3, 0, "🙂")).unwrap();
    assert!(send(&mut owner, delta(4, 3, "x")).is_err());
    let finished = frame(
        &hello,
        "model_event",
        4,
        json!({"agent_turn_id":"turn","request_id":"request","byte_offset":4,"text":"","phase":"finished","canonical_message":{"record_type":"canonical_agent_message","message_version":1,"message_id":"model-final","role":"assistant","agent_turn_id":"turn","task_id":"task","parts":[{"kind":"text","part_id":"text","text":"🙂"}]}}),
    );
    let mut stale = finished.clone();
    stale["payload"]["canonical_message"]["task_id"] = json!("old-task");
    assert!(send(&mut owner, stale).is_err());
    let original = send(&mut owner, finished.clone()).unwrap();
    assert_eq!(
        original.outbound_json.0,
        send(&mut owner, finished).unwrap().outbound_json.0
    );
    assert!(send(&mut owner, delta(5, 4, "late")).is_err());
    assert!(send(&mut owner,frame(&hello,"tool_result",5,json!({"agent_turn_id":"turn","tool_call_id":"invented","operation_ref":null,"result_ref":null,"outcome":"completed","result_json":null}))).is_err());
}
