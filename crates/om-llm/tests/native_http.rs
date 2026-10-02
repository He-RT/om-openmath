//! Real loopback transport drives the shared Job; no live providers or credentials.
#![cfg(feature = "http")]
use om_llm::*;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};
fn profile(kind: ProviderKind, base: String) -> Profile {
    Profile {
        name: "mock".into(),
        kind,
        base_url: base,
        model: "editable".into(),
        api_key: Some("sk-fixture-private".into()),
        temperature: 0.2,
        max_tokens: 128,
        supports_tools: true,
        supports_json_mode: true,
        timeout_ms: 2000,
        extra_headers: BTreeMap::new(),
        extra_body: BTreeMap::new(),
    }
}
fn message(role: Role, text: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: text.into(),
        tool_calls: vec![],
        tool_call_id: None,
    }
}
fn text_job(p: Profile) -> Job {
    Job::new(
        Feature::Explain,
        p,
        JobInput::Text {
            messages: vec![message(Role::User, "explain actual result")],
        },
        Target::Native,
    )
    .0
}
fn sse(text: &str) -> String {
    format!(
        "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]}),
        json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})
    )
}
#[tokio::test]
async fn actual_openai_anthropic_text_and_authorization_then_cached_terminal() {
    let client = native_client().unwrap();
    for kind in [ProviderKind::OpenaiChat, ProviderKind::Anthropic] {
        let server = MockServer::start().await;
        let (endpoint, auth, value, body) = if kind == ProviderKind::OpenaiChat {
            (
                "/chat/completions",
                "authorization",
                "Bearer sk-fixture-private",
                include_str!("fixtures/openai_text.sse"),
            )
        } else {
            (
                "/v1/messages",
                "x-api-key",
                "sk-fixture-private",
                include_str!("fixtures/anthropic_text.sse"),
            )
        };
        Mock::given(method("POST"))
            .and(path(endpoint))
            .and(header(auth, value))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        let mut job = text_job(profile(kind, server.uri()));
        let mut streamed = String::new();
        let result = drive_native(&mut job, &client, |event| {
            if let StreamEvent::Text(text) = event {
                streamed.push_str(&text)
            }
        })
        .await;
        let JobStep::Done(JobResult::Text(text)) = result else {
            panic!("{result:?}")
        };
        assert_eq!(streamed, text);
        assert!(!text.is_empty());
        assert!(matches!(
            drive_native(&mut job, &client, |_| panic!("duplicate event")).await,
            JobStep::Done(_)
        ));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}
#[tokio::test]
async fn actual_nonstream_fim_providers_and_chat_fallback_return_raw_insertions() {
    let client = native_client().unwrap();
    for (kind, endpoint, body, expected) in [
        (
            ProviderKind::OpenaiFim,
            "/completions",
            r#"{"choices":[{"text":" + α"}]}"#,
            " + α",
        ),
        (
            ProviderKind::OllamaFim,
            "/api/generate",
            r#"{"response":" + α","done":true}"#,
            " + α",
        ),
        (
            ProviderKind::MistralFim,
            "/v1/fim/completions",
            r#"{"choices":[{"message":{"content":" + α"}}]}"#,
            " + α",
        ),
        (
            ProviderKind::OpenaiChat,
            "/chat/completions",
            r#"{"choices":[{"message":{"role":"assistant","content":" + α"}}]}"#,
            " + α",
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        let (mut job, _) = Job::new(
            Feature::Complete,
            profile(kind, server.uri()),
            JobInput::Completion {
                prefix: "x".into(),
                suffix: "".into(),
            },
            Target::Native,
        );
        let step = drive_native(&mut job, &client, |_| {}).await;
        let JobStep::Done(JobResult::Completion(text)) = step else {
            panic!("{step:?}")
        };
        assert_eq!(text, expected);
        let requests = server.received_requests().await.unwrap();
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["model"], "editable");
        assert!(!body["stream"].as_bool().unwrap_or(false));
    }
}
#[tokio::test]
async fn two_tool_rounds_return_to_host_then_replay_actual_results_and_final_text() {
    let server = MockServer::start().await;
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(move |_: &wiremock::Request| {
            let round = calls.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_string(if round < 2 {
                include_str!("fixtures/openai_tools.sse").into()
            } else {
                sse("CAS result confirmed")
            })
        })
        .expect(3)
        .mount(&server)
        .await;
    let (mut job, _) = Job::new(
        Feature::Chat,
        profile(ProviderKind::OpenaiChat, server.uri()),
        JobInput::Chat {
            messages: prompts::chat_messages("English", &[message(Role::User, "compute")]),
            tools: prompts::chat_tools(),
        },
        Target::Native,
    );
    let client = native_client().unwrap();
    for round in 0..2 {
        let JobStep::RunTools(calls) = drive_native(&mut job, &client, |_| {}).await else {
            panic!()
        };
        assert_eq!(calls.len(), 2);
        // Waiting for host execution must not initiate another network request.
        assert!(matches!(
            drive_native(&mut job, &client, |_| {}).await,
            JobStep::RunTools(_)
        ));
        assert_eq!(count.load(Ordering::SeqCst), round + 1);
        assert!(matches!(
            job.tool_results(
                calls
                    .iter()
                    .map(|c| (c.id.clone(), format!("host-result-{round}-{}", c.name)))
                    .collect()
            ),
            JobStep::Http(_)
        ));
    }
    assert!(
        matches!(drive_native(&mut job,&client,|_|{}).await,JobStep::Done(JobResult::Text(t)) if t=="CAS result confirmed")
    );
    let requests = server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[2].body).unwrap();
    assert!(body.contains("host-result-0-evaluate"));
    assert!(body.contains("host-result-1-solve"));
}
struct RealValidator;
impl SuggestionValidator for RealValidator {
    fn validate(&self, response: &str) -> Result<Suggestion, String> {
        let value: Value = serde_json::from_str(response).map_err(|_| "JSON".to_owned())?;
        let source = value["wolfram"].as_str().ok_or("missing source")?;
        let expr = om_parse::parse_expr(source, om_parse::Dialect::Wolfram)
            .map_err(|d| d.iter().map(|d| d.code).collect::<Vec<_>>().join(","))?;
        Ok(Suggestion {
            wolfram: om_format::input_form(&expr),
            modern: om_format::modern_form(&expr),
            latex: om_format::latex(&expr),
            explanation: "verified".into(),
        })
    }
}
#[tokio::test]
async fn actual_parser_retry_http_rounds_are_driven_until_real_result() {
    let server = MockServer::start().await;
    let rounds = Arc::new(AtomicUsize::new(0));
    let counter = rounds.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            let n = counter.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_string(sse(if n < 2 {
                r#"{"wolfram":"Solve["}"#
            } else {
                r#"{"wolfram":"Solve[x==1,x]"}"#
            }))
        })
        .expect(3)
        .mount(&server)
        .await;
    let (mut job, _) = Job::new(
        Feature::Translate,
        profile(ProviderKind::OpenaiChat, server.uri()),
        JobInput::Structured {
            messages: prompts::translate_messages("English", "Solve", "", "solve x=1"),
            validator: Arc::new(RealValidator),
        },
        Target::Native,
    );
    let step = drive_native(&mut job, &native_client().unwrap(), |_| {}).await;
    assert!(
        matches!(step,JobStep::Done(JobResult::Suggestion(s)) if s.wolfram=="Solve[x == 1, x]")
    );
    let requests = server.received_requests().await.unwrap();
    assert!(
        std::str::from_utf8(&requests[2].body)
            .unwrap()
            .contains("E023")
    );
}
#[tokio::test]
async fn provider_status_and_reflected_secrets_are_real_bounded_and_safe() {
    for status in [401, 403, 500] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_json(json!({"error":{"message":"bad sk-fixture-private"}})),
            )
            .mount(&server)
            .await;
        let mut job = text_job(profile(ProviderKind::OpenaiChat, server.uri()));
        let mut events = vec![];
        let step = drive_native(&mut job, &native_client().unwrap(), |e| events.push(e)).await;
        let JobStep::Failed(LlmError::Http {
            status: actual,
            message,
        }) = step
        else {
            panic!("{step:?}")
        };
        assert_eq!(actual, status);
        assert!(message.contains("bad ***"));
        assert!(!message.contains("sk-fixture-private"));
        assert_eq!(
            message.contains("check API Key"),
            status == 401 || status == 403
        );
        assert!(events.is_empty());
    }
}
#[tokio::test]
async fn configured_timeout_and_early_inflight_cancellation_drop_actual_requests() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(300))
                .set_body_string(sse("late")),
        )
        .mount(&server)
        .await;
    let client = native_client().unwrap();
    let mut p = profile(ProviderKind::OpenaiChat, server.uri());
    p.timeout_ms = 20;
    let mut job = text_job(p);
    assert!(
        matches!(drive_native(&mut job,&client,|_|{}).await,JobStep::Failed(LlmError::Transport(t)) if t.contains("timed out"))
    );
    let token = CancellationToken::new();
    token.cancel();
    let mut job = text_job(profile(ProviderKind::OpenaiChat, server.uri()));
    assert!(matches!(
        drive_native_cancellable(&mut job, &client, &token, |_| panic!()).await,
        JobStep::Failed(LlmError::Cancelled)
    ));
    let token = CancellationToken::new();
    let cancel = token.clone();
    let mut job = text_job(profile(ProviderKind::OpenaiChat, server.uri()));
    let canceller = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel.cancel()
    });
    assert!(matches!(
        drive_native_cancellable(&mut job, &client, &token, |_| panic!()).await,
        JobStep::Failed(LlmError::Cancelled)
    ));
    canceller.await.unwrap();
}
#[tokio::test]
async fn safe_factory_refuses_redirect_without_forwarding_custom_credentials() {
    let destination = MockServer::start().await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(307)
                .insert_header("location", format!("{}/collect", destination.uri())),
        )
        .mount(&server)
        .await;
    let mut p = profile(ProviderKind::Anthropic, server.uri());
    p.extra_headers
        .insert("x-custom-secret".into(), "extra-private".into());
    let mut job = text_job(p);
    assert!(matches!(
        drive_native(&mut job, &native_client().unwrap(), |_| {}).await,
        JobStep::Failed(LlmError::Http { status: 307, .. })
    ));
    assert!(destination.received_requests().await.unwrap().is_empty());
}
#[tokio::test]
async fn malformed_invalid_utf8_missing_finish_and_oversized_responses_do_not_complete() {
    for body in [
        b"data: {invalid}\n\n".to_vec(),
        b"data: \xff\n\n".to_vec(),
        b"data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"x\"}}]}\n\n".to_vec(),
        vec![b'x'; 1_048_577],
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
            .mount(&server)
            .await;
        let mut job = text_job(profile(ProviderKind::OpenaiChat, server.uri()));
        assert!(matches!(
            drive_native(&mut job, &native_client().unwrap(), |_| {}).await,
            JobStep::Failed(_)
        ));
    }
}
