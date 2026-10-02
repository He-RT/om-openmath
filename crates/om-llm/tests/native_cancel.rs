//! Cancellation takes precedence even when several SSE events arrive in one network chunk.
#![cfg(feature = "http")]
use om_llm::*;
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
#[tokio::test]
async fn callback_cancel_does_not_forward_later_events_from_the_same_response_chunk() {
    let server = MockServer::start().await;
    let body = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"first\"},\"finish_reason\":null}]}\n\ndata: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"second\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
    let p = Profile {
        name: "mock".into(),
        kind: ProviderKind::OpenaiChat,
        base_url: server.uri(),
        model: "editable".into(),
        api_key: None,
        temperature: 0.0,
        max_tokens: 32,
        supports_tools: false,
        supports_json_mode: false,
        timeout_ms: 5000,
        extra_headers: BTreeMap::new(),
        extra_body: BTreeMap::new(),
    };
    let (mut job, _) = Job::new(
        Feature::Explain,
        p,
        JobInput::Text {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "explain".into(),
                tool_calls: vec![],
                tool_call_id: None,
            }],
        },
        Target::Native,
    );
    let token = CancellationToken::new();
    let cancel = token.clone();
    let mut events = vec![];
    let step = drive_native_cancellable(&mut job, &native_client().unwrap(), &token, |event| {
        events.push(event);
        cancel.cancel()
    })
    .await;
    assert!(
        matches!(step, JobStep::Failed(LlmError::Cancelled)),
        "{step:?}"
    );
    assert_eq!(events.len(), 1, "{events:?}");
}
