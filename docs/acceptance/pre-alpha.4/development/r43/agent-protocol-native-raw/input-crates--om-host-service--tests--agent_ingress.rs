//! Real typed/raw frames, entropy IDs and explicit host ledger confirmation boundaries.
use om_host_service::{
    agent::ingress::*,
    protocol::{Serial, generated::AgentToolName},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn scope() -> OwnerScope {
    OwnerScope {
        runtime: "runtime-1".into(),
        session: "session-1".into(),
        document: "document-1".into(),
        document_generation: Serial::new(1).unwrap(),
        task: "task-1".into(),
        task_generation: Serial::new(1).unwrap(),
        connection: "connection-1".into(),
        connection_generation: Serial::new(1).unwrap(),
    }
}
fn frame(kind: &str, sequence: u64, payload: Value) -> Value {
    json!({"protocol_version":1,"connection_id":"connection-1","connection_generation":1,"runtime_instance_id":"runtime-1","agent_session_id":"session-1","document_id":"document-1","document_generation":1,"task_id":"task-1","task_generation":1,"message_id":format!("message-{sequence}"),"sequence":sequence,"reply_to":null,"type":kind,"payload":payload})
}
fn ready() -> Value {
    let mut v = frame(
        "ready",
        1,
        json!({"core_version":"1.0.4","adapter_version":"openmath-adapter-1","schema_hash":format!("{:x}",Sha256::digest(include_bytes!("../../../docs/design/native-agent.schema.json")))}),
    );
    v["reply_to"] = json!("hello-1");
    v
}
fn ingress(readonly: bool) -> AgentIngress {
    AgentIngress::new(
        scope(),
        vec![AgentToolName::ReadNotebook, AgentToolName::RunCells],
        readonly,
        "hello-1".into(),
        "1.0.4".into(),
        "openmath-adapter-1".into(),
    )
    .unwrap()
}
fn accept(owner: &mut AgentIngress, value: &Value) -> Result<IngressDisposition, &'static str> {
    owner.accept(&serde_json::to_vec(value).unwrap(), &scope())
}
fn proposal(name: &str) -> Value {
    frame(
        "transcript_event",
        2,
        json!({"event_id":"proposal-1","source_sequence":1,"message":{"record_type":"canonical_agent_message","message_version":1,"message_id":"canonical-1","role":"assistant","agent_turn_id":"turn-1","task_id":"task-1","parts":[{"kind":"tool_call","part_id":"part-1","tool_call_id":"model-call-1","name":name,"arguments_json":"{}"}]}}),
    )
}
fn call(seq: u64) -> Value {
    frame(
        "tool_call",
        seq,
        json!({"agent_turn_id":"turn-1","tool_call_id":"model-call-1","proposal_event_ref":"proposal-1"}),
    )
}
#[test]
fn handshake_scope_duplicates_and_gap_never_resume_a_different_activation() {
    let mut owner = ingress(false);
    let mut bad = ready();
    bad["payload"]["schema_hash"] = json!("0".repeat(64));
    assert!(accept(&mut owner, &bad).is_err());
    assert!(matches!(
        accept(&mut owner, &ready()),
        Ok(IngressDisposition::Ready)
    ));
    assert!(matches!(
        accept(&mut owner, &ready()),
        Ok(IngressDisposition::Duplicate)
    ));
    let mut conflicting = ready();
    conflicting["payload"]["adapter_version"] = json!("different");
    assert!(matches!(
        accept(&mut owner, &conflicting),
        Err("IPC_ID_CONFLICT")
    ));
    for key in [
        "runtime_instance_id",
        "agent_session_id",
        "document_id",
        "task_id",
        "connection_id",
    ] {
        let mut wrong = frame("error", 2, json!({"code":"ERROR","recoverable":true}));
        wrong[key] = json!("different");
        assert!(matches!(
            accept(&mut owner, &wrong),
            Err("STALE_AGENT_SCOPE")
        ));
    }
    let mut newer = scope();
    newer.document_generation = Serial::new(2).unwrap();
    assert!(matches!(
        owner.accept(&serde_json::to_vec(&ready()).unwrap(), &newer),
        Err("STALE_AGENT_SCOPE")
    ));
    let gap = frame("error", 3, json!({"code":"ERROR","recoverable":true}));
    assert!(matches!(accept(&mut owner, &gap), Err("IPC_SEQUENCE_GAP")));
    assert_eq!(owner.phase(), ConnectionPhase::Failed);
    assert!(accept(&mut owner, &ready()).is_err());
}
#[test]
fn tool_mapping_waits_for_confirmed_canonical_event_and_uses_distinct_host_ids() {
    let mut owner = ingress(false);
    accept(&mut owner, &ready()).unwrap();
    let proposed = proposal("read_notebook");
    let IngressDisposition::Transcript(actual) = accept(&mut owner, &proposed).unwrap() else {
        panic!("not canonical event")
    };
    assert!(matches!(
        accept(&mut owner, &call(3)),
        Err("IPC_LEDGER_ACK_REQUIRED")
    ));
    let canonical = serde_json::to_vec(&actual.payload.message).unwrap();
    let canonical_hash = format!("{:x}", Sha256::digest(canonical));
    assert_eq!(
        owner.confirm_transcript("message-2", "proposal-1", &"0".repeat(64)),
        Err("IPC_LEDGER_RECEIPT_MISMATCH")
    );
    owner
        .confirm_transcript("message-2", "proposal-1", &canonical_hash)
        .unwrap();
    let IngressDisposition::ToolIntent(first) = accept(&mut owner, &call(3)).unwrap() else {
        panic!("no mapped intent")
    };
    assert!(first.operation_id.starts_with("agent-operation-"));
    assert_ne!(first.operation_id, first.call_id);
    let IngressDisposition::ToolIntent(retry) = accept(&mut owner, &call(4)).unwrap() else {
        panic!("no same intent")
    };
    assert_eq!(first.operation_id, retry.operation_id);
    assert_eq!(first.arguments_hash, format!("{:x}", Sha256::digest(b"{}")));
    owner.cancel();
    assert!(matches!(
        accept(&mut owner, &call(5)),
        Err("IPC_TASK_NOT_READY")
    ));
    let stopped = frame("settled", 5, json!({"reason":"cancelled"}));
    assert!(matches!(
        accept(&mut owner, &stopped),
        Ok(IngressDisposition::AwaitingSettlement)
    ));
    assert_eq!(owner.phase(), ConnectionPhase::AwaitingSettlement);
    owner.close();
    assert!(accept(&mut owner, &call(6)).is_err());
}
#[test]
fn readonly_canonical_forgery_and_turn_proposal_isolation_do_not_create_grants() {
    let mut owner = ingress(true);
    accept(&mut owner, &ready()).unwrap();
    let p = proposal("run_cells");
    let IngressDisposition::Transcript(event) = accept(&mut owner, &p).unwrap() else {
        panic!()
    };
    let expected = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&event.payload.message).unwrap())
    );
    owner
        .confirm_transcript("message-2", "proposal-1", &expected)
        .unwrap();
    assert!(matches!(
        accept(&mut owner, &call(3)),
        Err("PERMISSION_DENIED")
    ));
    let mut wrong = call(3);
    wrong["payload"]["agent_turn_id"] = json!("other-turn");
    assert!(matches!(
        accept(&mut owner, &wrong),
        Err("IPC_CALL_SCOPE_MISMATCH")
    ));
    let mut owner = ingress(false);
    accept(&mut owner, &ready()).unwrap();
    let mut p = proposal("read_notebook");
    p["payload"]["message"]["role"] = json!("system");
    assert!(matches!(
        accept(&mut owner, &p),
        Err("IPC_INVALID_TRANSCRIPT")
    ));
    p = proposal("read_notebook");
    p["payload"]["message"]["parts"][0] = json!({"kind":"tool_result","part_id":"part-1","tool_call_id":"model-call-1","operation_ref":null,"result_ref":null,"outcome":"completed"});
    assert!(matches!(
        accept(&mut owner, &p),
        Err("IPC_FORGED_TOOL_RESULT")
    ));
    p = proposal("read_notebook");
    p["payload"]["message"]["parts"][0]["arguments_json"] = json!("{\"x\":1,\"x\":2}");
    assert!(accept(&mut owner, &p).is_err());
}
