//! Actual HTTP shapes over synthetic profiles; no transport or network.
use om_llm::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn profile(kind: ProviderKind) -> Profile {
    Profile {
        name: "fixture".into(),
        kind,
        base_url: "https://provider.example.test/v1/".into(),
        model: "editable-model".into(),
        api_key: Some("sk-test".into()),
        temperature: 0.2,
        max_tokens: 123,
        supports_tools: true,
        supports_json_mode: true,
        timeout_ms: 60000,
        extra_headers: BTreeMap::new(),
        extra_body: BTreeMap::new(),
    }
}
fn msg(role: Role, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.into(),
        tool_calls: vec![],
        tool_call_id: None,
    }
}
fn tool() -> ToolSpec {
    ToolSpec {
        name: "solve",
        description: "Solve supplied equations",
        parameters: json!({"type":"object","properties":{"eq":{"type":"string"}},"required":["eq"]}),
    }
}
#[test]
fn openai_request_has_actual_headers_body_capabilities_and_multi_role_tool_replay() {
    let p = profile(ProviderKind::OpenaiChat);
    let mut assistant = msg(Role::Assistant, "");
    assistant.tool_calls.push(ToolCall {
        id: "call_1".into(),
        name: "solve".into(),
        arguments: "{\"eq\":\"x^2==2\"}".into(),
    });
    let mut result = msg(Role::Tool, "{{x->Sqrt[2]}}");
    result.tool_call_id = Some("call_1".into());
    let req = build_chat_request(
        &p,
        &[
            msg(Role::System, "Math assistant"),
            msg(Role::User, "求解"),
            assistant,
            result,
        ],
        &[tool()],
        true,
        Target::Native,
    );
    assert_eq!(req.method, "POST");
    assert_eq!(req.url, "https://provider.example.test/v1/chat/completions");
    assert!(req.stream);
    assert!(
        req.headers
            .contains(&("Authorization".into(), "Bearer sk-test".into()))
    );
    let body: Value = serde_json::from_str(&req.body).unwrap();
    assert_eq!(
        body,
        json!({"model":"editable-model","messages":[{"role":"system","content":"Math assistant"},{"role":"user","content":"求解"},{"role":"assistant","content":null,"tool_calls":[{"id":"call_1","type":"function","function":{"name":"solve","arguments":"{\"eq\":\"x^2==2\"}"}}]},{"role":"tool","tool_call_id":"call_1","content":"{{x->Sqrt[2]}}"}],"temperature":0.2,"max_tokens":123,"stream":true,"tools":[{"type":"function","function":{"name":"solve","description":"Solve supplied equations","parameters":tool().parameters}}],"response_format":{"type":"json_object"}})
    );
}
#[test]
fn anthropic_extracts_system_and_merges_tool_results_with_real_tool_input_blocks() {
    let mut p = profile(ProviderKind::Anthropic);
    p.base_url = "https://anthropic.example.test".into();
    let mut assistant = msg(Role::Assistant, "Use CAS");
    assistant.tool_calls = vec![ToolCall {
        id: "t1".into(),
        name: "solve".into(),
        arguments: "{\"eq\":\"x==2\"}".into(),
    }];
    let mut result = msg(Role::Tool, "x=2");
    result.tool_call_id = Some("t1".into());
    let req = build_chat_request(
        &p,
        &[
            msg(Role::System, "one"),
            msg(Role::System, "two"),
            msg(Role::User, "question"),
            assistant,
            result,
            msg(Role::User, "continue"),
        ],
        &[tool()],
        false,
        Target::Browser,
    );
    assert_eq!(req.url, "https://anthropic.example.test/v1/messages");
    assert!(
        req.headers
            .contains(&("x-api-key".into(), "sk-test".into()))
    );
    assert!(req.headers.contains(&(
        "anthropic-dangerous-direct-browser-access".into(),
        "true".into()
    )));
    let body: Value = serde_json::from_str(&req.body).unwrap();
    assert_eq!(body["system"], "one\ntwo");
    assert_eq!(
        body["messages"],
        json!([{"role":"user","content":[{"type":"text","text":"question"}]},{"role":"assistant","content":[{"type":"text","text":"Use CAS"},{"type":"tool_use","id":"t1","name":"solve","input":{"eq":"x==2"}}]},{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"x=2"},{"type":"text","text":"continue"}]}])
    );
    assert_eq!(body["tools"][0]["input_schema"], tool().parameters);
    let native = build_chat_request(&p, &[msg(Role::User, "hi")], &[], false, Target::Native);
    assert!(
        !native
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("anthropic-dangerous-direct-browser-access"))
    );
}
#[test]
fn disabled_capabilities_optional_keys_and_extra_headers_are_deterministic_and_case_insensitive() {
    let mut p = profile(ProviderKind::OpenaiChat);
    p.api_key = Some(String::new());
    p.supports_tools = false;
    p.supports_json_mode = false;
    p.extra_headers
        .insert("content-type".into(), "custom/type".into());
    p.extra_headers.insert("X-Custom".into(), "value".into());
    let req = try_build_chat_request(
        &p,
        &[msg(Role::User, "hi")],
        &[tool()],
        true,
        Target::Browser,
    )
    .unwrap();
    let body: Value = serde_json::from_str(&req.body).unwrap();
    assert!(body.get("tools").is_none() && body.get("response_format").is_none());
    assert!(
        !req.headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("authorization"))
    );
    assert_eq!(
        req.headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .count(),
        1
    );
    assert!(req.headers.iter().any(|(_, v)| v == "custom/type"));
    assert_eq!(
        serde_json::to_value(&req).unwrap(),
        serde_json::to_value(build_chat_request(
            &p,
            &[msg(Role::User, "hi")],
            &[tool()],
            true,
            Target::Browser
        ))
        .unwrap()
    );
}
#[test]
fn checked_invalid_profiles_messages_and_tool_json_fail_without_leaking_secrets() {
    for kind in [
        ProviderKind::OpenaiFim,
        ProviderKind::OllamaFim,
        ProviderKind::MistralFim,
    ] {
        assert!(
            try_build_chat_request(
                &profile(kind),
                &[msg(Role::User, "hi")],
                &[],
                false,
                Target::Native
            )
            .is_err()
        );
    }
    let mut p = profile(ProviderKind::OpenaiChat);
    p.api_key = Some("should-never-leak".into());
    p.temperature = f32::NAN;
    let e = try_build_chat_request(&p, &[], &[], false, Target::Native).unwrap_err();
    assert!(!format!("{e:?}: {e}").contains("should-never-leak"));
    p.temperature = 0.2;
    p.extra_headers
        .insert("X-Header".into(), "value\r\nInjected: secret".into());
    assert!(
        try_build_chat_request(&p, &[msg(Role::User, "hi")], &[], false, Target::Native).is_err()
    );
    let p = profile(ProviderKind::Anthropic);
    let mut m = msg(Role::Assistant, "");
    m.tool_calls.push(ToolCall {
        id: "t".into(),
        name: "solve".into(),
        arguments: "{".into(),
    });
    assert!(try_build_chat_request(&p, &[m], &[], false, Target::Native).is_err());
}
#[test]
fn debug_redacts_keys_headers_endpoint_and_private_prompt_but_explicit_transport_is_real() {
    let mut p = profile(ProviderKind::OpenaiChat);
    p.extra_headers
        .insert("X-Private".into(), "header-secret".into());
    let req = build_chat_request(
        &p,
        &[msg(Role::User, "private-prompt")],
        &[],
        false,
        Target::Native,
    );
    let debug = format!("{p:?} {req:?}");
    for secret in ["sk-test", "header-secret", "private-prompt"] {
        assert!(!debug.contains(secret), "{debug}");
    }
    assert!(serde_json::to_string(&req).unwrap().contains("sk-test"));
}
