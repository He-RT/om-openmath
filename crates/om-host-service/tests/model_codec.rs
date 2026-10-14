//! Actual native single-turn state, scope, bounds and terminal behavior.
use om_host_service::{model::ModelChannel, protocol::generated::*, scheduler::NativeHost};
use serde_json::{Value, json};
fn request() -> Value {
    json!({"binding":{"provider_ref":"provider","route_ref":"route","model_ref":"model","adapter_version":"single-turn-1","protocol":"chat_completions"},"api_model":"actual-alias","messages":[{"canonical":{"record_type":"canonical_agent_message","message_version":1,"message_id":"user","role":"user","task_id":"task","agent_turn_id":"turn","parts":[{"kind":"text","part_id":"text","text":"中文🙂"}]},"replay":null}],"tools":[],"results":[],"stream":true,"max_output_tokens":64,"temperature":null})
}
fn prepare(id: &str) -> Value {
    json!({"type":"model_codec_prepare","runtime_instance_id":"runtime","request_ref":id,"context_ref":format!("context-{id}"),"task_id":"task","agent_turn_id":"turn","message_id":"native-message","request_json":request().to_string()})
}
fn command(c: &mut ModelChannel, value: Value) -> Result<NativeModelCodecReply, String> {
    c.command("runtime", &serde_json::to_vec(&value).unwrap())
}
fn next(kind: &str, id: &str) -> Value {
    json!({"type":format!("model_codec_{kind}"),"runtime_instance_id":"runtime","codec_ref":id})
}
#[test]
fn raw_single_turn_scope_eof_unknown_usage_and_terminal_nonce_are_actual() {
    let mut c = ModelChannel::default();
    let first = command(&mut c, prepare("request")).unwrap();
    assert_eq!(first.phase, NativeModelCodecReplyPhase::Prepared);
    let body: Value = serde_json::from_str(first.body_json.0.as_ref().unwrap()).unwrap();
    assert_eq!(body["model"], "actual-alias");
    assert!(body.get("api_key").is_none());
    let id = &first.codec_ref;
    let mut headers = next("headers", id);
    headers["status"] = json!(200);
    command(&mut c, headers).unwrap();
    let stream = format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({"id":"provider-id","model":"served-alias","choices":[{"index":0,"delta":{"content":"中文🙂"},"finish_reason":"stop"}]})
    );
    for b in stream.as_bytes() {
        let mut feed = next("feed", id);
        feed["bytes"] = json!([b]);
        command(&mut c, feed).unwrap();
    }
    let result = command(&mut c, next("finish", id)).unwrap();
    assert_eq!(result.phase, NativeModelCodecReplyPhase::Decoded);
    let decoded: Value = serde_json::from_str(result.result_json.0.as_ref().unwrap()).unwrap();
    assert_eq!(decoded["message"]["message_id"], "native-message");
    assert_eq!(decoded["replay"]["response_id"], "provider-id");
    assert!(decoded["usage"]["input"].is_null());
    assert!(command(&mut c, next("finish", id)).is_err());
    assert!(command(&mut c, prepare("request")).is_err());
    let mut duplicate = prepare("new-request");
    duplicate["context_ref"] = json!("context-request");
    assert!(command(&mut c, duplicate).is_err());
}
#[test]
fn http_error_echo_cancellation_invalid_bytes_duplicate_members_and_host_close_fail_closed() {
    let mut c = ModelChannel::default();
    let first = command(&mut c, prepare("bad")).unwrap();
    let mut headers = next("headers", &first.codec_ref);
    headers["status"] = json!(401);
    command(&mut c, headers).unwrap();
    let mut feed = next("feed", &first.codec_ref);
    feed["bytes"] = json!(b"secret-sentinel-error-body");
    command(&mut c, feed).unwrap();
    let failed = command(&mut c, next("finish", &first.codec_ref)).unwrap();
    assert_eq!(
        failed.error_code.0,
        Some(NativeModelCodecReplyErrorCode::Http)
    );
    assert_eq!(failed.http_status.0, Some(401));
    assert!(
        !serde_json::to_string(&failed)
            .unwrap()
            .contains("secret-sentinel")
    );
    let first = command(&mut c, prepare("cancel")).unwrap();
    command(&mut c, next("cancel", &first.codec_ref)).unwrap();
    assert!(command(&mut c, next("headers", &first.codec_ref)).is_err());
    let first = command(&mut c, prepare("bounds")).unwrap();
    let mut feed = next("feed", &first.codec_ref);
    feed["bytes"] = json!([256]);
    assert!(command(&mut c, feed).is_err());
    assert!(c.command("runtime",br#"{"type":"model_codec_cancel","type":"model_codec_finish","runtime_instance_id":"runtime","codec_ref":"x"}"#).is_err());
    let host = NativeHost::new(HostInit {
        protocol_version: 1,
        runtime_instance_id: "runtime".into(),
        max_pending_operations: 8,
        event_capacity: 32,
    })
    .unwrap();
    let bytes = serde_json::to_vec(&prepare("host")).unwrap();
    host.model_codec_command(&bytes).unwrap();
    host.close_begin();
    assert!(host.model_codec_command(&bytes).is_err());
    host.close_finish().unwrap();
}
#[test]
fn no_model_url_auth_unknown_input_or_cross_runtime_and_eight_live_bound() {
    let mut c = ModelChannel::default();
    for i in 0..8 {
        command(&mut c, prepare(&format!("req-{i}"))).unwrap();
    }
    assert!(command(&mut c, prepare("ninth")).is_err());
    let mut c = ModelChannel::default();
    let mut bad = prepare("foreign");
    bad["runtime_instance_id"] = json!("foreign");
    assert!(command(&mut c, bad).is_err());
    let mut body = request();
    body["api_key"] = json!("not-accepted");
    let mut bad = prepare("auth");
    bad["request_json"] = json!(body.to_string());
    assert!(command(&mut c, bad).is_err());
    let mut bad = prepare("url");
    bad["url"] = json!("https://untrusted.invalid");
    assert!(command(&mut c, bad).is_err());
}
