//! Actual byte feeds, empty-vs-missing output, provider probes and late-phase boundaries.
use om_llm::*;
use om_num::rng::SplitMix64;
use serde_json::json;
use std::collections::BTreeMap;
fn p(kind: ProviderKind) -> Profile {
    Profile {
        name: "profile".into(),
        kind,
        base_url: "https://server.example.test".into(),
        model: "custom".into(),
        api_key: None,
        temperature: 0.2,
        max_tokens: 123,
        supports_tools: true,
        supports_json_mode: false,
        timeout_ms: 10,
        extra_headers: BTreeMap::new(),
        extra_body: BTreeMap::new(),
    }
}
fn msg() -> ChatMessage {
    ChatMessage {
        role: Role::User,
        content: "question".into(),
        tool_calls: vec![],
        tool_call_id: None,
    }
}
fn chat() -> JobInput {
    JobInput::Chat {
        messages: vec![msg()],
        tools: vec![
            ToolSpec {
                name: "solve",
                description: "readonly",
                parameters: json!({"type":"object"}),
            },
            ToolSpec {
                name: "evaluate",
                description: "readonly",
                parameters: json!({"type":"object"}),
            },
        ],
    }
}
#[test]
fn actual_job_byte_split_results_and_tool_requests_match_whole_fixture_feeds() {
    for (kind, fixture) in [
        (
            ProviderKind::OpenaiChat,
            include_bytes!("fixtures/openai_tools.sse").as_slice(),
        ),
        (
            ProviderKind::Anthropic,
            include_bytes!("fixtures/anthropic_tools.sse").as_slice(),
        ),
    ] {
        let (mut whole, _) = Job::new(Feature::Chat, p(kind), chat(), Target::Browser);
        let events = whole.on_raw_bytes(fixture);
        let JobStep::RunTools(calls) = whole.on_http_end(200, None) else {
            panic!()
        };
        for seed in 0..32 {
            let (mut job, _) = Job::new(Feature::Chat, p(kind), chat(), Target::Browser);
            let mut rng = SplitMix64::new(seed);
            let mut i = 0;
            let mut actual = vec![];
            while i < fixture.len() {
                let j = (i + rng.next_range(1, 8) as usize).min(fixture.len());
                actual.extend(job.on_raw_bytes(&fixture[i..j]));
                i = j;
            }
            assert_eq!(actual, events);
            let JobStep::RunTools(actual) = job.on_http_end(200, None) else {
                panic!()
            };
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                serde_json::to_value(&calls).unwrap()
            );
        }
    }
}
#[test]
fn genuine_empty_text_finishes_but_metadata_or_truncated_frames_do_not() {
    let (mut job, _) = Job::new(
        Feature::Explain,
        p(ProviderKind::OpenaiChat),
        JobInput::Text {
            messages: vec![msg()],
        },
        Target::Native,
    );
    job.on_bytes("data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\"},\"finish_reason\":\"stop\"}]}\n\n");
    assert!(matches!(job.on_http_end(200,None),JobStep::Done(JobResult::Text(t)) if t.is_empty()));
    for text in [
        "data: [DONE]\n\n",
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"text\"},\"finish_reason\":\"stop\"}]}\n",
    ] {
        let (mut job, _) = Job::new(
            Feature::Explain,
            p(ProviderKind::OpenaiChat),
            JobInput::Text {
                messages: vec![msg()],
            },
            Target::Native,
        );
        job.on_bytes(text);
        assert!(matches!(
            job.on_http_end(200, None),
            JobStep::Failed(LlmError::Incomplete)
        ));
    }
}
#[test]
fn target_correct_completion_and_test_profile_probes_support_real_provider_formats() {
    for (kind, body) in [
        (ProviderKind::OpenaiFim, r#"{"choices":[{"text":"pong"}]}"#),
        (
            ProviderKind::OllamaFim,
            r#"{"response":"pong","done":true}"#,
        ),
        (
            ProviderKind::MistralFim,
            r#"{"choices":[{"message":{"content":"pong"}}]}"#,
        ),
    ] {
        let (mut job, step) = Job::new(
            Feature::TestProfile,
            p(kind),
            JobInput::Text {
                messages: vec![msg()],
            },
            Target::Native,
        );
        let JobStep::Http(r) = step else { panic!() };
        assert!(!r.stream);
        assert!(r.body.contains('8'));
        job.on_bytes(body);
        assert!(matches!(job.on_http_end(200,None),JobStep::Done(JobResult::Text(t)) if t=="pong"));
    }
    let (_, step) = Job::new(
        Feature::Complete,
        p(ProviderKind::Anthropic),
        JobInput::Completion {
            prefix: "prefix".into(),
            suffix: "suffix".into(),
        },
        Target::Browser,
    );
    let JobStep::Http(r) = step else { panic!() };
    assert!(
        r.headers
            .iter()
            .any(|(k, v)| k == "anthropic-dangerous-direct-browser-access" && v == "true")
    );
}
#[test]
fn conflicting_tool_metadata_postfinish_data_and_duplicate_http_end_fail_explicitly() {
    let (mut job, _) = Job::new(
        Feature::Chat,
        p(ProviderKind::OpenaiChat),
        chat(),
        Target::Native,
    );
    let one = json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"a","function":{"name":"solve","arguments":"{"}}]},"finish_reason":null}]});
    let two = json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"different"}]},"finish_reason":null}]});
    job.on_bytes(&format!("data: {one}\n\n"));
    let e = job.on_bytes(&format!("data: {two}\n\n"));
    assert!(matches!(e[0], StreamEvent::Error(_)));
    assert!(matches!(
        job.on_http_end(200, None),
        JobStep::Failed(LlmError::Tools)
    ));
    let (mut job, _) = Job::new(
        Feature::Chat,
        p(ProviderKind::OpenaiChat),
        chat(),
        Target::Native,
    );
    job.on_bytes(include_str!("fixtures/openai_tools.sse"));
    assert!(matches!(job.on_http_end(200, None), JobStep::RunTools(_)));
    assert!(job.on_bytes("late data").is_empty());
    assert!(matches!(
        job.on_http_end(200, None),
        JobStep::Failed(LlmError::State(_))
    ));
}
