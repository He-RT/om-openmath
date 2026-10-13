//! Real encoded canonical frames split at byte boundaries, not pre-decoded mock callbacks.
use om_host_service::agent::framing::{self, FrameDecoder, MAX_FRAME_BYTES};
use serde_json::{Value, json};
fn frame() -> Value {
    json!({"protocol_version":1,"connection_id":"connection-1","connection_generation":1,
      "runtime_instance_id":"runtime-1","agent_session_id":"session-1","document_id":"document-1",
      "document_generation":1,"task_id":"task-1","task_generation":1,"message_id":"message-1",
      "sequence":1,"reply_to":null,"type":"model_event","payload":{"agent_turn_id":"turn-1",
      "request_id":"request-1","byte_offset":0,"text":"中文🙂e\u{301}","phase":"delta","canonical_message":null}})
}
#[test]
fn every_actual_utf8_byte_cut_and_multiple_frames_preserve_text() {
    let mut bytes = serde_json::to_vec(&frame()).unwrap();
    bytes.push(b'\n');
    for cut in 0..=bytes.len() {
        let mut decoder = FrameDecoder::default();
        let mut frames = decoder.push(&bytes[..cut]).unwrap();
        frames.extend(decoder.push(&bytes[cut..]).unwrap());
        assert_eq!(frames.len(), 1);
        assert_eq!(serde_json::to_value(&frames[0]).unwrap(), frame());
        decoder.finish().unwrap();
    }
    let mut decoder = FrameDecoder::default();
    let mut pair = bytes.clone();
    pair.extend(&bytes);
    assert_eq!(decoder.push(&pair).unwrap().len(), 2);
    decoder.finish().unwrap();
}
#[test]
fn frame_contract_denies_credentials_urls_pi_state_missing_null_and_wrong_versions() {
    for key in ["api_key", "authorization", "base_url", "pi_state"] {
        let mut value = frame();
        value["payload"][key] = json!("private-value");
        assert_eq!(
            framing::decode(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
            "IPC_INVALID_SHAPE"
        );
    }
    let mut missing = frame();
    missing.as_object_mut().unwrap().remove("reply_to");
    assert!(framing::decode(&serde_json::to_vec(&missing).unwrap()).is_err());
    let mut wrong = frame();
    wrong["protocol_version"] = json!(2);
    assert!(framing::decode(&serde_json::to_vec(&wrong).unwrap()).is_err());
    wrong = frame();
    wrong["sequence"] = json!(9_007_199_254_740_992_u64);
    assert!(framing::decode(&serde_json::to_vec(&wrong).unwrap()).is_err());
}
#[test]
fn unique_json_members_utf8_depth_node_byte_and_eof_limits_are_real() {
    for bytes in [
        b"{\"a\":1,\"a\":2}".as_slice(),
        b"{\"a\":{\"x\":1,\"\\u0078\":2}}",
        b"{\"a\":1} junk",
        b"\xff",
        b"[1e9999]",
    ] {
        assert!(framing::strict_json(bytes).is_err());
    }
    let deep = format!("{}0{}", "[".repeat(65), "]".repeat(65));
    assert!(framing::strict_json(deep.as_bytes()).is_err());
    let many = format!("[{}]", vec!["0"; 32768].join(","));
    assert!(framing::strict_json(many.as_bytes()).is_err());
    assert!(framing::strict_json(&vec![b' '; MAX_FRAME_BYTES + 1]).is_err());
    let mut decoder = FrameDecoder::default();
    decoder.push(b"{\"a\"").unwrap();
    assert_eq!(decoder.finish(), Err("IPC_INCOMPLETE_EOF"));
    assert_eq!(decoder.push(b"}\n").unwrap_err(), "IPC_CONNECTION_FAILED");
    let mut decoder = FrameDecoder::default();
    assert_eq!(
        decoder.push(b"console log\n").unwrap_err(),
        "IPC_INVALID_JSON"
    );
    let mut decoder = FrameDecoder::default();
    assert_eq!(
        decoder.push(&vec![b' '; MAX_FRAME_BYTES + 1]).unwrap_err(),
        "IPC_FRAME_SIZE"
    );
}
#[test]
fn canonical_media_and_tool_parts_retain_ids_without_executable_urls_or_framework_state() {
    let mut value = frame();
    value["type"] = json!("transcript_event");
    value["payload"] = json!({"event_id":"event-1","source_sequence":1,
      "message":{"record_type":"canonical_agent_message","message_version":1,"message_id":"canonical-1","role":"assistant","agent_turn_id":"turn-1","task_id":"task-1","parts":[
      {"kind":"text","part_id":"part-1","text":"读取真实输出"},
      {"kind":"media","part_id":"part-2","media_ref":"media-1","submission_kind":"audio"},
      {"kind":"tool_call","part_id":"part-3","tool_call_id":"call-1","name":"read_notebook","arguments_json":"{}"}]}});
    let encoded = serde_json::to_vec(&value).unwrap();
    let decoded = framing::decode(&encoded).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), value);
    value["payload"]["message"]["parts"][1]["url"] = json!("https://invalid.test/private");
    assert!(framing::decode(&serde_json::to_vec(&value).unwrap()).is_err());
}
