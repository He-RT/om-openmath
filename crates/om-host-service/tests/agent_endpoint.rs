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
    assert!(receive(&mut owner,&frame(&hello,"model_event",4,json!({"agent_turn_id":"turn","request_id":"provider-request","byte_offset":0,"text":"claimed output","phase":"delta"})),1).is_err());
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
