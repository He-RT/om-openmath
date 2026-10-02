//! Explicit vendor parameters preserve the checked protocol and generic provider bodies.
use om_llm::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn profile() -> Profile {
    Profile {
        name: "configured".into(),
        kind: ProviderKind::OpenaiChat,
        base_url: "https://provider.invalid/v1".into(),
        model: "editable".into(),
        api_key: None,
        temperature: 0.2,
        max_tokens: 128,
        supports_tools: true,
        supports_json_mode: true,
        timeout_ms: 1000,
        extra_headers: BTreeMap::new(),
        extra_body: BTreeMap::from([("thinking".into(), json!({"type":"disabled"}))]),
    }
}
#[test]
fn vendor_body_options_are_actual_request_parameters_and_not_prompt_data() {
    let p = profile();
    let messages = [ChatMessage {
        role: Role::User,
        content: "task".into(),
        tool_calls: vec![],
        tool_call_id: None,
    }];
    let request = try_build_chat_request(&p, &messages, &[], false, Target::Native).unwrap();
    let body: Value = serde_json::from_str(&request.body).unwrap();
    assert_eq!(body["thinking"], json!({"type":"disabled"}));
    assert_eq!(body["model"], "editable");
    assert_eq!(body["messages"][0]["content"], "task");
    let request = try_build_fim_request(&p, "a", "b", Target::Native).unwrap();
    let body: Value = serde_json::from_str(&request.body).unwrap();
    assert_eq!(body["thinking"], json!({"type":"disabled"}));
    assert_eq!(body["stream"], false);
}
#[test]
fn protocol_overrides_are_rejected_and_sensitive_extra_values_are_not_debugged() {
    for name in [
        "model",
        "messages",
        "tools",
        "stream",
        "temperature",
        "max_tokens",
        "prompt",
        "suffix",
        "stop",
        "response_format",
        "system",
        "options",
        "headers",
        "base_url",
    ] {
        let mut p = profile();
        p.extra_body.insert(name.into(), json!("invalid"));
        assert!(
            try_build_fim_request(&p, "a", "b", Target::Native).is_err(),
            "{name}"
        );
    }
    let mut p = profile();
    p.extra_body
        .insert("vendor_token".into(), json!("synthetic-extra-private"));
    assert!(!format!("{p:?}").contains("synthetic-extra-private"));
}
#[test]
fn oversized_vendor_data_is_rejected_without_including_payloads() {
    let mut p = profile();
    p.extra_body
        .insert("metadata".into(), json!("a".repeat(1_048_577)));
    let error = try_build_fim_request(&p, "x", "", Target::Native).unwrap_err();
    assert!(matches!(error, LlmError::Limit { .. }));
    assert!(!error.to_string().contains("metadata"));
}
