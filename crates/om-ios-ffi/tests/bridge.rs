//! Real shared Session results and independent cancellation, without native IO.
use om_ios_ffi::Host;
use serde_json::{Value, json};
use std::sync::Arc;

fn request(host: &Host, body: Value) -> Value {
    serde_json::from_str(
        &host
            .request(&json!({"id":1,"body":body}).to_string())
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn bridge_solves_reacts_preserves_source_and_rejects_bad_envelopes() {
    let host = Host::new(None).unwrap();
    let solved = request(
        &host,
        json!({"type":"evaluate","cell_id":"solve","source":"solve(x^2==4,x)","dialect":"Modern"}),
    );
    assert_eq!(
        solved["response"]["body"]["output"]["items"][0]["view"]["solutions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    request(
        &host,
        json!({"type":"evaluate","cell_id":"a","source":"let a=2","dialect":"Modern"}),
    );
    request(
        &host,
        json!({"type":"evaluate","cell_id":"b","source":"a+1","dialect":"Modern"}),
    );
    let changed = request(
        &host,
        json!({"type":"evaluate","cell_id":"a","source":"let a=5","dialect":"Modern"}),
    );
    assert!(changed.to_string().contains("6"));
    let saved = request(&host, json!({"type":"save_notebook"}));
    assert!(saved["response"]["body"]["file"].get("config").is_none());
    assert!(host.request("bad-json").is_err());
    assert!(
        host.request(r#"{"id":0,"body":{"type":"run_all"}}"#)
            .is_err()
    );
    host.close();
    assert!(
        host.request(r#"{"id":1,"body":{"type":"run_all"}}"#)
            .is_err()
    );
}

#[test]
fn independent_interrupt_recovers_without_resetting_definitions() {
    let host = Arc::new(Host::new(None).unwrap());
    request(
        &host,
        json!({"type":"evaluate","cell_id":"a","source":"let a=9","dialect":"Modern"}),
    );
    let worker = host.clone();
    let running = std::thread::spawn(move || {
        request(
            &worker,
            json!({"type":"evaluate","cell_id":"slow","source":"factorial(1000000)","dialect":"Modern"}),
        )
    });
    std::thread::sleep(std::time::Duration::from_millis(40));
    host.interrupt();
    assert!(running.join().unwrap().to_string().contains("Aborted"));
    let next = request(
        &host,
        json!({"type":"evaluate","cell_id":"next","source":"a+1","dialect":"Modern"}),
    );
    assert_eq!(
        next["response"]["body"]["output"]["items"][0]["input_form"],
        "10"
    );
}

#[test]
fn mobile_scratch_forks_live_definitions_without_modifying_the_notebook() {
    let host = Host::new(None).unwrap();
    request(
        &host,
        json!({"type":"evaluate","cell_id":"a","source":"let a=7","dialect":"Modern"}),
    );
    let before = request(&host, json!({"type":"save_notebook"}));
    let scratch = request(
        &host,
        json!({"type":"mobile_scratch","source":"let a=99;a+1","dialect":"Modern","definitions":true}),
    );
    assert_eq!(
        scratch["response"]["body"]["output"]["items"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["input_form"],
        "100",
        "{scratch}"
    );
    let main = request(
        &host,
        json!({"type":"evaluate","cell_id":"b","source":"a+1","dialect":"Modern"}),
    );
    assert_eq!(
        main["response"]["body"]["output"]["items"][0]["input_form"],
        "8"
    );
    let saved = request(&host, json!({"type":"save_notebook"}));
    assert_eq!(
        before["response"]["body"]["file"]["cells"][0],
        saved["response"]["body"]["file"]["cells"][0]
    );
    assert_eq!(
        saved["response"]["body"]["file"]["cells"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn mobile_model_codec_preserves_scope_unicode_and_terminal_ownership() {
    let host = Host::new(None).unwrap();
    let input = json!({"binding":{"provider_ref":"provider-1","route_ref":"route-1","model_ref":"model-1","adapter_version":"single-turn-v1","protocol":"chat_completions"},
      "api_model":"loopback","messages":[{"canonical":{"record_type":"canonical_agent_message","message_version":1,"message_id":"user-1","role":"user","task_id":"task-1","agent_turn_id":null,"parts":[{"kind":"text","part_id":"text-1","text":"读取笔记本"}]},"replay":null}],"results":[],"tools":[],"max_output_tokens":1024,"temperature":null,"stream":false});
    let prepare = json!({"op":"prepare","id":"codec-1","request":input,"message":"assistant-1","task":"task-1","turn":"turn-1"});
    let prepared: Value =
        serde_json::from_str(&host.model_command(&prepare.to_string()).unwrap()).unwrap();
    assert!(prepared["body"].as_str().unwrap().contains("读取笔记本"));
    assert!(host.model_command(&prepare.to_string()).is_err());
    host.model_command(r#"{"op":"headers","id":"codec-1","status":200}"#)
        .unwrap();
    let wire = r#"{"id":"provider-response","model":"loopback","choices":[{"index":0,"message":{"role":"assistant","content":"中文🙂"},"finish_reason":"stop"}]}"#;
    for byte in wire.as_bytes() {
        host.model_command(&json!({"op":"feed","id":"codec-1","bytes":[*byte]}).to_string())
            .unwrap();
    }
    let result: Value = serde_json::from_str(
        &host
            .model_command(r#"{"op":"finish","id":"codec-1"}"#)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(result["message"]["parts"][0]["text"], "中文🙂");
    assert_eq!(result["message"]["task_id"], "task-1");
    assert!(result["usage"]["input"].is_null());
    assert!(
        host.model_command(r#"{"op":"finish","id":"codec-1"}"#)
            .is_err()
    );
    host.close();
    assert!(host.model_command(&prepare.to_string()).is_err());
}
