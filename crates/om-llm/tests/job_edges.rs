//! Job lifecycle/resource/security boundaries through actual fixture/request APIs.
use om_llm::*;
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
fn p() -> Profile {
    Profile {
        name: "profile".into(),
        kind: ProviderKind::OpenaiChat,
        base_url: "https://server.example.test".into(),
        model: "model".into(),
        api_key: Some("sensitive-api-key".into()),
        temperature: 0.2,
        max_tokens: 64,
        supports_tools: true,
        supports_json_mode: true,
        timeout_ms: 10,
        extra_headers: BTreeMap::new(),
        extra_body: BTreeMap::new(),
    }
}
fn msg(s: &str) -> ChatMessage {
    ChatMessage {
        role: Role::User,
        content: s.into(),
        tool_calls: vec![],
        tool_call_id: None,
    }
}
fn chat() -> (Job, JobStep) {
    Job::new(
        Feature::Chat,
        p(),
        JobInput::Chat {
            messages: vec![msg("question")],
            tools: vec![ToolSpec {
                name: "evaluate",
                description: "readonly",
                parameters: json!({"type":"object"}),
            }],
        },
        Target::Native,
    )
}
fn tool_frame(id: &str, index: u32, args: &str) -> String {
    format!(
        "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":index,"id":id,"function":{"name":"evaluate","arguments":args}}]},"finish_reason":null}]}),
        json!({"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]})
    )
}
fn text_frame(text: &str) -> String {
    format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":{"content":text},"finish_reason":"stop"}]})
    )
}
#[test]
fn six_real_tool_rounds_allow_final_text_but_a_seventh_invocation_fails() {
    let (mut job, _) = chat();
    for _ in 0..6 {
        job.on_bytes(&tool_frame("same-id", 0, "{}"));
        let JobStep::RunTools(calls) = job.on_http_end(200, None) else {
            panic!()
        };
        assert_eq!(calls[0].id, "same-id");
        assert!(matches!(
            job.tool_results(vec![("same-id".into(), "actual".into())]),
            JobStep::Http(_)
        ));
    }
    job.on_bytes(&text_frame("final"));
    assert!(matches!(job.on_http_end(200,None),JobStep::Done(JobResult::Text(s)) if s=="final"));
    let (mut job, _) = chat();
    for _ in 0..6 {
        job.on_bytes(&tool_frame("id", 0, "{}"));
        job.on_http_end(200, None);
        job.tool_results(vec![("id".into(), "actual".into())]);
    }
    job.on_bytes(&tool_frame("id", 0, "{}"));
    assert!(matches!(
        job.on_http_end(200, None),
        JobStep::Failed(LlmError::ToolRounds)
    ));
}
#[test]
fn wrong_partial_duplicate_results_and_invalid_active_phase_calls_never_issue_http() {
    for results in [
        vec![],
        vec![("wrong".into(), "result".into())],
        vec![("id".into(), "one".into()), ("id".into(), "two".into())],
    ] {
        let (mut job, _) = chat();
        job.on_bytes(&tool_frame("id", 0, "{}"));
        assert!(matches!(job.on_http_end(200, None), JobStep::RunTools(_)));
        assert!(matches!(
            job.tool_results(results),
            JobStep::Failed(LlmError::Tools)
        ));
        assert!(job.is_finished());
    }
    let (mut job, _) = chat();
    assert!(matches!(
        job.tool_results(vec![]),
        JobStep::Failed(LlmError::State(_))
    ));
    assert!(job.on_bytes(&text_frame("late")).is_empty());
}
struct Reject;
impl SuggestionValidator for Reject {
    fn validate(&self, _: &str) -> Result<Suggestion, String> {
        Err("invalid syntax sensitive-api-key".into())
    }
}
#[test]
fn actual_validation_retries_stop_after_two_and_retry_diagnostics_redact_keys() {
    let (mut job, _) = Job::new(
        Feature::Fix,
        p(),
        JobInput::Structured {
            messages: vec![msg("fix")],
            validator: Arc::new(Reject),
        },
        Target::Native,
    );
    for _ in 0..2 {
        job.on_bytes(&text_frame("untrusted"));
        let JobStep::Http(r) = job.on_http_end(200, None) else {
            panic!()
        };
        assert!(!r.body.contains("sensitive-api-key"));
        assert!(r.body.contains("invalid syntax"));
    }
    job.on_bytes(&text_frame("still invalid"));
    let JobStep::Failed(error) = job.on_http_end(200, None) else {
        panic!()
    };
    assert!(matches!(error, LlmError::Validation(_)));
    assert!(!format!("{error:?}: {error}").contains("sensitive-api-key"));
}
#[test]
fn missing_and_malformed_tools_do_not_become_completed_text() {
    let (mut job, _) = chat();
    job.on_bytes(&tool_frame("id", 0, "{"));
    assert!(matches!(
        job.on_http_end(200, None),
        JobStep::Failed(LlmError::Tools)
    ));
    let (mut job, _) = chat();
    job.on_bytes(
        "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
    );
    assert!(matches!(
        job.on_http_end(200, None),
        JobStep::Failed(LlmError::Tools)
    ));
    let (mut job, _) = chat();
    job.on_bytes("data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},\"finish_reason\":null}]}\n\n");
    assert!(matches!(
        job.on_http_end(200, None),
        JobStep::Failed(LlmError::Incomplete)
    ));
}
#[test]
fn arbitrary_large_indices_are_bounded_by_count_and_real_http_data_limits() {
    let (mut job, _) = chat();
    job.on_bytes(&tool_frame("id", u32::MAX, "{}"));
    let JobStep::RunTools(c) = job.on_http_end(200, None) else {
        panic!()
    };
    assert_eq!(c.len(), 1);
    let (mut job, _) = chat();
    let events = job.on_raw_bytes(&vec![b'x'; 1_048_577]);
    assert!(matches!(events[0], StreamEvent::Error(_)));
    assert!(matches!(
        job.on_http_end(200, None),
        JobStep::Failed(LlmError::Limit { .. })
    ));
    let (mut job, _) = chat();
    assert!(matches!(
        job.on_raw_bytes(b"data: \xff\n\n")[0],
        StreamEvent::Error(_)
    ));
    assert!(matches!(
        job.on_http_end(200, None),
        JobStep::Failed(LlmError::Encoding)
    ));
}
#[test]
fn terminal_failures_do_not_change_with_late_http_metadata_or_transport_messages() {
    let (mut job, _) = Job::new(
        Feature::Complete,
        p(),
        JobInput::Text {
            messages: vec![msg("wrong")],
        },
        Target::Native,
    );
    assert!(matches!(
        job.on_http_end(503, Some("late".into())),
        JobStep::Failed(LlmError::Input)
    ));
    let (mut job, _) = chat();
    let first = job.on_http_end(200, None);
    assert!(matches!(first, JobStep::Failed(LlmError::Incomplete)));
    assert!(matches!(
        job.on_http_end(401, None),
        JobStep::Failed(LlmError::Incomplete)
    ));
}
#[test]
fn stream_and_transport_errors_redact_actual_profile_and_custom_header_credentials() {
    let mut profile = p();
    profile
        .extra_headers
        .insert("X-Credential".into(), "header-private-value".into());
    let (mut job, _) = Job::new(
        Feature::Explain,
        profile,
        JobInput::Text {
            messages: vec![msg("explain")],
        },
        Target::Native,
    );
    let events = job
        .on_bytes("data: {\"error\":{\"message\":\"sensitive-api-key header-private-value\"}}\n\n");
    assert!(
        matches!(&events[0],StreamEvent::Error(s) if !s.contains("sensitive-api-key")&&!s.contains("header-private-value"))
    );
    let step = job.on_http_end(403, None);
    let s = format!("{step:?}");
    assert!(!s.contains("sensitive-api-key") && !s.contains("header-private-value"));
    assert!(s.contains("403") && s.contains("API Key"));
}

#[test]
fn tool_argument_roots_must_be_real_objects_before_host_execution() {
    for arguments in ["[]", "null", "7", "\"code\""] {
        let (mut job, _) = chat();
        job.on_bytes(&tool_frame("id", 0, arguments));
        assert!(
            matches!(job.on_http_end(200, None), JobStep::Failed(LlmError::Tools)),
            "{arguments}"
        );
    }
}

#[test]
fn retry_conversations_never_copy_known_credentials_from_a_reflected_model_response() {
    let (mut job, _) = Job::new(
        Feature::Fix,
        p(),
        JobInput::Structured {
            messages: vec![msg("fix")],
            validator: Arc::new(Reject),
        },
        Target::Native,
    );
    job.on_bytes(&text_frame("reflected sensitive-api-key"));
    let JobStep::Http(request) = job.on_http_end(200, None) else {
        panic!()
    };
    assert!(!request.body.contains("sensitive-api-key"));
    assert!(
        request
            .headers
            .iter()
            .any(|(_, value)| value.contains("sensitive-api-key"))
    );
}
