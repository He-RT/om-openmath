//! Choice selection, complete response boundaries, shared validation and error privacy.
use om_llm::*;
use std::collections::BTreeMap;
fn p(kind: ProviderKind) -> Profile {
    Profile {
        name: "fim".into(),
        kind,
        base_url: "http://[::1]:11434/".into(),
        model: "new-user-model".into(),
        api_key: None,
        temperature: 0.6,
        max_tokens: 17,
        supports_tools: false,
        supports_json_mode: false,
        timeout_ms: 0,
        extra_headers: BTreeMap::new(),
    }
}
#[test]
fn explicit_choice_zero_and_duplicate_or_missing_choices_have_truthful_semantics() {
    let body = r#"{"choices":[{"index":1,"text":"wrong"},{"index":0,"text":"right"}]}"#;
    assert_eq!(
        parse_fim_response(ProviderKind::OpenaiFim, body).unwrap(),
        "right"
    );
    for body in [
        r#"{"choices":[{"index":1,"text":"wrong"}]}"#,
        r#"{"choices":[{"index":0,"text":"a"},{"index":0,"text":"b"}]}"#,
        r#"{"choices":[false]}"#,
        r#"{"choices":[{"text":"a"},{"text":"b"}]}"#,
    ] {
        assert!(parse_fim_response(ProviderKind::OpenaiFim, body).is_err());
    }
}
#[test]
fn incomplete_ollama_wrong_assistant_roles_and_tool_content_do_not_finish_a_completion() {
    for (kind, body) in [
        (
            ProviderKind::OllamaFim,
            r#"{"response":"partial","done":false}"#,
        ),
        (
            ProviderKind::OpenaiChat,
            r#"{"choices":[{"message":{"role":"tool","content":"tool output"}}]}"#,
        ),
        (
            ProviderKind::MistralFim,
            r#"{"choices":[{"message":{"role":"assistant","content":"text","tool_calls":[{}]}}]}"#,
        ),
    ] {
        assert!(parse_fim_response(kind, body).is_err(), "{kind:?} {body}");
    }
}
#[test]
fn response_limit_and_remote_errors_keep_payloads_out_of_display_and_debug() {
    let body = " ".repeat(1_048_577);
    assert!(matches!(
        parse_fim_response(ProviderKind::OpenaiFim, &body),
        Err(LlmError::Limit { .. })
    ));
    for (kind, body) in [
        (
            ProviderKind::OllamaFim,
            r#"{"error":"reflected sk-sensitive-key"}"#,
        ),
        (
            ProviderKind::Anthropic,
            r#"{"type":"error","error":{"message":"reflected sk-sensitive-key"}}"#,
        ),
    ] {
        let e = parse_fim_response(kind, body).unwrap_err();
        assert_eq!(e.remote_message(), Some("reflected sk-sensitive-key"));
        assert!(!format!("{e:?}: {e}").contains("sk-sensitive-key"));
    }
}
#[test]
fn deterministic_requests_reuse_shared_credentials_and_endpoint_checks_for_all_kinds() {
    for kind in [
        ProviderKind::OpenaiFim,
        ProviderKind::OllamaFim,
        ProviderKind::MistralFim,
        ProviderKind::OpenaiChat,
        ProviderKind::Anthropic,
    ] {
        let profile = p(kind);
        let req = try_build_fim_request(&profile, "\"α\"", "β\r\n", Target::Native).unwrap();
        assert!(!req.stream);
        assert!(req.url.starts_with("http://[::1]:11434/"));
        assert_eq!(
            serde_json::to_string(&req).unwrap(),
            serde_json::to_string(&build_fim_request(&profile, "\"α\"", "β\r\n")).unwrap()
        );
        let mut bad = p(kind);
        bad.base_url = "file:///private".into();
        assert!(try_build_fim_request(&bad, "", "", Target::Native).is_err());
        let mut bad = p(kind);
        bad.api_key = Some("***".into());
        assert!(try_build_fim_request(&bad, "", "", Target::Native).is_err());
    }
}

#[test]
fn anthropic_fallback_keeps_text_and_empty_completion_without_returning_thinking_content() {
    assert_eq!(
        parse_fim_response(
            ProviderKind::Anthropic,
            r#"{"content":[],"stop_reason":"end_turn"}"#
        )
        .unwrap(),
        ""
    );
    let body = r#"{"content":[{"type":"thinking","thinking":"private-thought"},{"type":"redacted_thinking","data":"opaque"},{"type":"text","text":"x^2"}]}"#;
    assert_eq!(
        parse_fim_response(ProviderKind::Anthropic, body).unwrap(),
        "x^2"
    );
}

#[test]
fn explicit_tool_finish_reasons_and_nonassistant_anthropic_messages_are_not_insertions() {
    for (kind, body) in [
        (
            ProviderKind::OpenaiChat,
            r#"{"choices":[{"finish_reason":"tool_calls","message":{"content":"not an insertion"}}]}"#,
        ),
        (
            ProviderKind::Anthropic,
            r#"{"role":"user","content":[{"type":"text","text":"not assistant output"}]}"#,
        ),
        (
            ProviderKind::Anthropic,
            r#"{"stop_reason":"tool_use","content":[{"type":"text","text":"needs tools"}]}"#,
        ),
    ] {
        assert!(parse_fim_response(kind, body).is_err(), "{body}");
    }
}
