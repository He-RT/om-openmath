//! Actual multi-round protocol fixtures drive the pure state machine; no network or CAS execution.
use om_llm::*;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
fn profile(kind: ProviderKind) -> Profile {
    Profile {
        name: "fixture".into(),
        kind,
        base_url: "https://provider.example.test".into(),
        model: "editable".into(),
        api_key: Some("sk-test".into()),
        temperature: 0.2,
        max_tokens: 128,
        supports_tools: true,
        supports_json_mode: true,
        timeout_ms: 1000,
        extra_headers: BTreeMap::new(),
    }
}
fn msg(role: Role, s: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: s.into(),
        tool_calls: vec![],
        tool_call_id: None,
    }
}
fn input() -> JobInput {
    JobInput::Chat {
        messages: vec![msg(Role::User, "question")],
        tools: vec![
            ToolSpec {
                name: "evaluate",
                description: "evaluate readonly",
                parameters: json!({"type":"object"}),
            },
            ToolSpec {
                name: "solve",
                description: "solve readonly",
                parameters: json!({"type":"object"}),
            },
        ],
    }
}
fn http(step: JobStep) -> HttpRequest {
    let JobStep::Http(r) = step else {
        panic!("{step:?}")
    };
    r
}
fn tools(step: JobStep) -> Vec<ToolCall> {
    let JobStep::RunTools(c) = step else {
        panic!("{step:?}")
    };
    c
}
#[test]
fn two_actual_tool_rounds_then_final_text_replay_real_openai_and_anthropic_requests() {
    for kind in [ProviderKind::OpenaiChat, ProviderKind::Anthropic] {
        let (mut job, step) = Job::new(Feature::Chat, profile(kind), input(), Target::Native);
        let first = http(step);
        assert!(first.stream);
        let fixture = if kind == ProviderKind::OpenaiChat {
            include_bytes!("fixtures/openai_tools.sse").as_slice()
        } else {
            include_bytes!("fixtures/anthropic_tools.sse").as_slice()
        };
        for round in 0..2 {
            let mut events = vec![];
            for b in fixture.chunks(3) {
                events.extend(job.on_raw_bytes(b));
            }
            assert!(!events.iter().any(|e| matches!(e, StreamEvent::Error(_))));
            assert!(!job.is_finished());
            let calls = tools(job.on_http_end(200, None));
            assert_eq!(calls.len(), 2);
            assert_eq!(calls[0].name, "evaluate");
            assert_eq!(calls[1].name, "solve");
            let args: Value = serde_json::from_str(&calls[0].arguments).unwrap();
            assert_eq!(args["code"], "α+1");
            let results = calls
                .iter()
                .rev()
                .map(|c| (c.id.clone(), format!("fixture-result-{round}-{}", c.name)))
                .collect();
            let request = http(job.tool_results(results));
            let body: Value = serde_json::from_str(&request.body).unwrap();
            let encoded = serde_json::to_string(&body).unwrap();
            assert!(encoded.contains(&format!("fixture-result-{round}-evaluate")));
            assert!(encoded.contains("α+1"));
            assert!(encoded.contains(&calls[0].id));
        }
        let final_text = if kind == ProviderKind::OpenaiChat {
            include_str!("fixtures/openai_text.sse")
        } else {
            include_str!("fixtures/anthropic_text.sse")
        };
        job.on_bytes(final_text);
        let JobStep::Done(JobResult::Text(text)) = job.on_http_end(200, None) else {
            panic!()
        };
        assert_eq!(text, "你好 α=2");
        assert!(job.is_finished());
    }
}
#[test]
fn finish_events_do_not_dispatch_before_http_end_and_duplicate_terminal_calls_are_harmless() {
    let (mut job, _) = Job::new(
        Feature::Explain,
        profile(ProviderKind::OpenaiChat),
        JobInput::Text {
            messages: vec![msg(Role::User, "explain")],
        },
        Target::Native,
    );
    let events = job.on_bytes(include_str!("fixtures/openai_text.sse"));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, StreamEvent::Finish { .. }))
    );
    assert!(!job.is_finished());
    let step = job.on_http_end(200, None);
    assert!(matches!(step, JobStep::Done(JobResult::Text(_))));
    assert!(job.on_bytes("late data").is_empty());
    assert!(matches!(
        job.on_http_end(200, None),
        JobStep::Done(JobResult::Text(_))
    ));
}
struct Validator;
impl SuggestionValidator for Validator {
    fn validate(&self, response: &str) -> Result<Suggestion, String> {
        let value: Value =
            serde_json::from_str(response).map_err(|_| "invalid JSON".to_string())?;
        let source = value["wolfram"].as_str().ok_or("missing source")?;
        let expr = om_parse::parse_expr(source, om_parse::Dialect::Wolfram)
            .map_err(|_| "CAS parse failed".to_string())?;
        Ok(Suggestion {
            wolfram: om_format::input_form(&expr),
            modern: om_format::modern_form(&expr),
            latex: om_format::latex(&expr),
            explanation: value["explanation"].as_str().unwrap_or("").into(),
        })
    }
}
fn text_frame(text: &str) -> String {
    format!(
        "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]}),
        json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})
    )
}
#[test]
fn structured_results_delegate_to_actual_host_parser_and_retry_actual_diagnostics() {
    let input = JobInput::Structured {
        messages: vec![msg(Role::User, "translate")],
        validator: Arc::new(Validator),
    };
    let (mut job, step) = Job::new(
        Feature::Translate,
        profile(ProviderKind::OpenaiChat),
        input,
        Target::Native,
    );
    assert!(http(step).body.contains("response_format"));
    job.on_bytes(&text_frame(r#"{"wolfram":"Solve[","explanation":"bad"}"#));
    let retry = http(job.on_http_end(200, None));
    assert!(retry.body.contains("CAS parse failed"));
    job.on_bytes(&text_frame(r#"{"wolfram":"Solve[x^2==4,x]","modern":"UNTRUSTED","latex":"UNTRUSTED","explanation":"solve"}"#));
    let JobStep::Done(JobResult::Suggestion(s)) = job.on_http_end(200, None) else {
        panic!()
    };
    assert!(!s.modern.contains("UNTRUSTED"));
    assert!(!s.latex.contains("UNTRUSTED"));
    assert!(om_parse::parse_expr(&s.wolfram, om_parse::Dialect::Wolfram).is_ok());
}
#[test]
fn completion_uses_actual_fim_body_and_http_json_without_streaming_or_trim() {
    let (mut job, step) = Job::new(
        Feature::Complete,
        profile(ProviderKind::OpenaiFim),
        JobInput::Completion {
            prefix: "x=".into(),
            suffix: String::new(),
        },
        Target::Native,
    );
    assert!(!http(step).stream);
    assert!(
        job.on_bytes(include_str!("fixtures/deepseek_fim.json"))
            .is_empty()
    );
    let JobStep::Done(JobResult::Completion(text)) = job.on_http_end(200, None) else {
        panic!()
    };
    assert_eq!(text, "  x^2+α\nnext");
}
#[test]
fn invalid_new_cancel_missing_stream_and_http_errors_are_real_failures() {
    let (mut invalid, step) = Job::new(
        Feature::Chat,
        profile(ProviderKind::OpenaiFim),
        input(),
        Target::Native,
    );
    assert!(matches!(step, JobStep::Failed(_)));
    assert!(invalid.is_finished());
    assert!(invalid.on_bytes("data").is_empty());
    let (mut job, _) = Job::new(
        Feature::Chat,
        profile(ProviderKind::OpenaiChat),
        input(),
        Target::Native,
    );
    assert!(matches!(job.cancel(), JobStep::Failed(LlmError::Cancelled)));
    assert!(
        job.on_bytes(include_str!("fixtures/openai_text.sse"))
            .is_empty()
    );
    assert!(matches!(
        job.on_http_end(200, None),
        JobStep::Failed(LlmError::Cancelled)
    ));
    let (mut job, _) = Job::new(
        Feature::Chat,
        profile(ProviderKind::OpenaiChat),
        input(),
        Target::Native,
    );
    assert!(matches!(job.on_http_end(200, None), JobStep::Failed(_)));
    let (mut job, _) = Job::new(
        Feature::Chat,
        profile(ProviderKind::OpenaiChat),
        input(),
        Target::Native,
    );
    job.on_bytes(r#"{"error":{"message":"invalid key sk-test"}}"#);
    let step = job.on_http_end(401, None);
    assert!(matches!(step, JobStep::Failed(_)));
    let debug = format!("{step:?}");
    assert!(!debug.contains("sk-test"));
    assert!(debug.contains("401") && debug.contains("API Key"));
}
