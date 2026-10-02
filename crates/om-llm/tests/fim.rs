//! Real provider-specific FIM proposals and unmodified response text, without transport.
use om_llm::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn p(kind: ProviderKind) -> Profile {
    Profile {
        name: "fim".into(),
        kind,
        base_url: "https://server.example.test/base/".into(),
        model: "editable-completion-model".into(),
        api_key: Some("sk-test".into()),
        temperature: 0.7,
        max_tokens: 64,
        supports_tools: true,
        supports_json_mode: true,
        timeout_ms: 2500,
        extra_headers: BTreeMap::new(),
    }
}
#[test]
fn three_fim_requests_have_exact_endpoints_json_sampling_and_headers() {
    for kind in [
        ProviderKind::OpenaiFim,
        ProviderKind::OllamaFim,
        ProviderKind::MistralFim,
    ] {
        let req = build_fim_request(&p(kind), "let α=", "\nsolve(α,x)");
        assert_eq!(req.method, "POST");
        assert!(!req.stream);
        assert!(
            req.headers
                .contains(&("Authorization".into(), "Bearer sk-test".into()))
        );
        let body: Value = serde_json::from_str(&req.body).unwrap();
        let suffix = match kind {
            ProviderKind::OpenaiFim => "/completions",
            ProviderKind::OllamaFim => "/api/generate",
            _ => "/v1/fim/completions",
        };
        assert_eq!(req.url, format!("https://server.example.test/base{suffix}"));
        let expected = match kind {
            ProviderKind::OpenaiFim => {
                json!({"model":"editable-completion-model","prompt":"let α=","suffix":"\nsolve(α,x)","max_tokens":64,"temperature":0,"stop":["\n"],"stream":false})
            }
            ProviderKind::OllamaFim => {
                json!({"model":"editable-completion-model","prompt":"let α=","suffix":"\nsolve(α,x)","stream":false,"options":{"temperature":0,"num_predict":64,"stop":["\n"]}})
            }
            _ => {
                json!({"model":"editable-completion-model","prompt":"let α=","suffix":"\nsolve(α,x)","max_tokens":64,"temperature":0,"stop":["\n"]})
            }
        };
        assert_eq!(body, expected);
        assert!(!format!("{req:?}").contains("sk-test"));
        assert!(serde_json::to_string(&req).unwrap().contains("sk-test"));
    }
}
#[test]
fn chat_fallback_is_nonstreaming_target_correct_and_has_insertion_only_prompt() {
    for kind in [ProviderKind::OpenaiChat, ProviderKind::Anthropic] {
        let req = try_build_fim_request(&p(kind), "prefix α", "suffix β", Target::Browser).unwrap();
        assert!(!req.stream);
        let body: Value = serde_json::from_str(&req.body).unwrap();
        assert_eq!(body["stream"], false);
        assert_eq!(body["temperature"].as_f64(), Some(0.0));
        assert!(body.get("tools").is_none() && body.get("response_format").is_none());
        if kind == ProviderKind::OpenaiChat {
            assert!(
                body["messages"][0]["content"]
                    .as_str()
                    .unwrap()
                    .contains("insert")
            );
            assert_eq!(body["messages"][1]["content"], "prefix α⟨CURSOR⟩suffix β");
            assert_eq!(body["stop"], json!(["\n"]));
        } else {
            assert!(body["system"].as_str().unwrap().contains("insert"));
            assert_eq!(
                body["messages"][0]["content"][0]["text"],
                "prefix α⟨CURSOR⟩suffix β"
            );
            assert_eq!(body["stop_sequences"], json!(["\n"]));
            assert!(
                req.headers
                    .iter()
                    .any(|(k, v)| k == "anthropic-dangerous-direct-browser-access" && v == "true")
            );
        }
    }
}
#[test]
fn mandated_provider_fixtures_and_chat_fallback_fields_return_exact_raw_text() {
    for (kind, body, expected) in [
        (
            ProviderKind::OpenaiFim,
            include_str!("fixtures/deepseek_fim.json"),
            "  x^2+α\nnext",
        ),
        (
            ProviderKind::OllamaFim,
            include_str!("fixtures/ollama_fim.json"),
            "β+1",
        ),
        (
            ProviderKind::MistralFim,
            include_str!("fixtures/mistral_fim.json"),
            " + y ",
        ),
        (
            ProviderKind::OpenaiChat,
            r#"{"choices":[{"index":0,"message":{"role":"assistant","content":" raw\ntext "}}]}"#,
            " raw\ntext ",
        ),
        (
            ProviderKind::Anthropic,
            r#"{"content":[{"type":"text","text":"你好 "},{"type":"text","text":"α"}]}"#,
            "你好 α",
        ),
    ] {
        assert_eq!(parse_fim_response(kind, body).unwrap(), expected);
    }
}
#[test]
fn checked_profile_header_errors_and_empty_prefix_suffix_are_real() {
    let mut profile = p(ProviderKind::OpenaiFim);
    profile.api_key = None;
    let req = try_build_fim_request(&profile, "", "", Target::Native).unwrap();
    assert!(
        !req.headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("authorization"))
    );
    let body: Value = serde_json::from_str(&req.body).unwrap();
    assert_eq!(body["prompt"], "");
    assert_eq!(body["suffix"], "");
    profile.max_tokens = 0;
    assert!(try_build_fim_request(&profile, "a", "b", Target::Native).is_err());
    profile.max_tokens = 8;
    profile
        .extra_headers
        .insert("Invalid\nHeader".into(), "secret".into());
    assert!(try_build_fim_request(&profile, "a", "b", Target::Native).is_err());
}
#[test]
fn wrong_or_malformed_response_fields_never_become_completion_text() {
    for (kind, body) in [
        (ProviderKind::OpenaiFim, "{bad"),
        (ProviderKind::OpenaiFim, r#"{"choices":[]}"#),
        (ProviderKind::OpenaiFim, r#"{"choices":[{"text":null}]}"#),
        (ProviderKind::OllamaFim, r#"{"response":7}"#),
        (
            ProviderKind::MistralFim,
            r#"{"choices":[{"text":"wrong"}]}"#,
        ),
        (
            ProviderKind::OpenaiChat,
            r#"{"choices":[{"message":{"content":null,"tool_calls":[{"id":"x"}]}}]}"#,
        ),
        (
            ProviderKind::Anthropic,
            r#"{"content":[{"type":"tool_use","name":"solve","input":{}}]}"#,
        ),
    ] {
        assert!(parse_fim_response(kind, body).is_err(), "{kind:?}: {body}");
    }
    assert_eq!(
        parse_fim_response(ProviderKind::OpenaiFim, r#"{"choices":[{"text":""}]}"#).unwrap(),
        ""
    );
    let err = parse_fim_response(
        ProviderKind::OpenaiFim,
        include_str!("fixtures/openai_error_401.json"),
    )
    .unwrap_err();
    assert!(matches!(err, LlmError::Remote(_)));
    assert!(!format!("{err:?}").contains("Incorrect API key"));
}
