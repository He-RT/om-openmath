//! Conversation associations, checked endpoint/header policy and real capability boundaries.
use om_llm::*;
use serde_json::json;
use std::collections::BTreeMap;
fn profile(kind: ProviderKind) -> Profile {
    Profile {
        name: "profile".into(),
        kind,
        base_url: "https://endpoint.example.test".into(),
        model: "custom-model".into(),
        api_key: None,
        temperature: 0.3,
        max_tokens: 128,
        supports_tools: true,
        supports_json_mode: true,
        timeout_ms: 10,
        extra_headers: BTreeMap::new(),
        extra_body: BTreeMap::new(),
    }
}
fn msg(role: Role) -> ChatMessage {
    ChatMessage {
        role,
        content: "text".into(),
        tool_calls: vec![],
        tool_call_id: None,
    }
}
#[test]
fn checked_chat_replay_rejects_orphan_duplicate_and_unfinished_tool_associations() {
    let p = profile(ProviderKind::OpenaiChat);
    let mut orphan = msg(Role::Tool);
    orphan.tool_call_id = Some("unknown".into());
    assert!(try_build_chat_request(&p, &[orphan], &[], false, Target::Native).is_err());
    let mut a = msg(Role::Assistant);
    a.tool_calls = vec![
        ToolCall {
            id: "id".into(),
            name: "solve".into(),
            arguments: "{}".into(),
        },
        ToolCall {
            id: "id".into(),
            name: "evaluate".into(),
            arguments: "{}".into(),
        },
    ];
    assert!(try_build_chat_request(&p, &[a], &[], false, Target::Native).is_err());
    let mut a = msg(Role::Assistant);
    a.tool_calls = vec![ToolCall {
        id: "id".into(),
        name: "solve".into(),
        arguments: "{}".into(),
    }];
    assert!(try_build_chat_request(&p, &[a], &[], false, Target::Native).is_err());
}
#[test]
fn actual_two_tool_result_order_and_anthropic_initial_objects_are_preserved() {
    let mut a = msg(Role::Assistant);
    a.tool_calls = vec![
        ToolCall {
            id: "1".into(),
            name: "solve".into(),
            arguments: "{}".into(),
        },
        ToolCall {
            id: "2".into(),
            name: "evaluate".into(),
            arguments: "{\"code\":\"2+2\"}".into(),
        },
    ];
    let mut r2 = msg(Role::Tool);
    r2.tool_call_id = Some("2".into());
    let mut r1 = msg(Role::Tool);
    r1.tool_call_id = Some("1".into());
    for kind in [ProviderKind::OpenaiChat, ProviderKind::Anthropic] {
        let req = try_build_chat_request(
            &profile(kind),
            &[msg(Role::User), a.clone(), r2.clone(), r1.clone()],
            &[],
            false,
            Target::Native,
        )
        .unwrap();
        let body: serde_json::Value = serde_json::from_str(&req.body).unwrap();
        assert_eq!(body["model"], "custom-model");
        if kind == ProviderKind::Anthropic {
            assert_eq!(body["messages"][1]["content"][1]["input"], json!({}));
            assert_eq!(body["messages"][2]["content"][0]["tool_use_id"], "2");
        }
    }
}
#[test]
fn checked_profiles_and_browser_headers_use_real_targets_and_no_masked_credential() {
    let mut p = profile(ProviderKind::Anthropic);
    p.extra_headers.insert(
        "Anthropic-Dangerous-Direct-Browser-Access".into(),
        "false".into(),
    );
    let req = try_build_chat_request(&p, &[msg(Role::User)], &[], false, Target::Browser).unwrap();
    assert!(req.headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("anthropic-dangerous-direct-browser-access") && v == "true"
    }));
    let req = try_build_chat_request(&p, &[msg(Role::User)], &[], false, Target::Native).unwrap();
    assert!(
        !req.headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("anthropic-dangerous-direct-browser-access"))
    );
    p.api_key = Some("***".into());
    assert!(try_build_chat_request(&p, &[msg(Role::User)], &[], false, Target::Native).is_err());
    for url in [
        "https://",
        "file:///tmp/config",
        "https://user:secret@example.test",
        "https://endpoint.example.test?key=private",
        "http://bad host",
    ] {
        let mut p = profile(ProviderKind::OpenaiChat);
        p.base_url = url.into();
        assert!(
            try_build_chat_request(&p, &[msg(Role::User)], &[], false, Target::Native).is_err(),
            "{url}"
        );
    }
}
#[test]
fn tool_names_and_schema_roots_are_checked_before_request_creation() {
    let p = profile(ProviderKind::OpenaiChat);
    for (name, parameters) in [
        ("bad name", json!({"type":"object"})),
        ("solve", json!(true)),
    ] {
        assert!(
            try_build_chat_request(
                &p,
                &[msg(Role::User)],
                &[ToolSpec {
                    name,
                    description: "",
                    parameters
                }],
                false,
                Target::Native
            )
            .is_err()
        );
    }
}

#[test]
fn malformed_authorities_do_not_escape_checked_endpoint_validation() {
    for url in [
        "http://[",
        "http://[]",
        "http://[not-v6]",
        "http://host:wrong",
        "http://host:70000",
        "http://host\\evil",
    ] {
        let mut p = profile(ProviderKind::OpenaiChat);
        p.base_url = url.into();
        assert!(
            try_build_chat_request(&p, &[msg(Role::User)], &[], false, Target::Native).is_err(),
            "{url}"
        );
    }
    for url in [
        "http://127.0.0.1:11434/v1",
        "http://[::1]:11434/v1",
        "HTTPS://api.example.test/v1",
    ] {
        let mut p = profile(ProviderKind::OpenaiChat);
        p.base_url = url.into();
        assert!(
            try_build_chat_request(&p, &[msg(Role::User)], &[], false, Target::Native).is_ok(),
            "{url}"
        );
    }
}

#[test]
fn anthropic_malformed_json_is_rejected_after_a_valid_tool_association() {
    let mut a = msg(Role::Assistant);
    a.tool_calls.push(ToolCall {
        id: "1".into(),
        name: "solve".into(),
        arguments: "{private-payload".into(),
    });
    let mut result = msg(Role::Tool);
    result.tool_call_id = Some("1".into());
    let error = try_build_chat_request(
        &profile(ProviderKind::Anthropic),
        &[a, result],
        &[],
        false,
        Target::Native,
    )
    .unwrap_err();
    assert_eq!(error, LlmError::Json);
    assert!(!format!("{error:?}").contains("private-payload"));
}

#[test]
fn completed_rounds_can_preserve_provider_ids_reused_in_a_later_round() {
    let mut a = msg(Role::Assistant);
    a.tool_calls.push(ToolCall {
        id: "provider_0".into(),
        name: "solve".into(),
        arguments: "{}".into(),
    });
    let mut result = msg(Role::Tool);
    result.tool_call_id = Some("provider_0".into());
    let messages = [msg(Role::User), a.clone(), result.clone(), a, result];
    assert!(
        try_build_chat_request(
            &profile(ProviderKind::OpenaiChat),
            &messages,
            &[],
            false,
            Target::Native
        )
        .is_ok()
    );
}
