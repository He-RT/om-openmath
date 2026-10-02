//! One real Session Job owns raw native transport and genuine readonly CAS tool rounds.
#![cfg(feature = "native")]
use om_kernel::{KernelConfig, Session, native::*, protocol::*};
use om_llm::{Target, drive_native_http, native_client};
use om_num::ctx::Clock;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method},
};
struct Time(Instant);
impl Clock for Time {
    fn now_ms(&self) -> f64 {
        self.0.elapsed().as_secs_f64() * 1000.0
    }
}
struct Credentials;
impl CredentialProvider for Credentials {
    fn environment(&self, name: &str) -> Result<Option<String>, CredentialError> {
        Ok((name == "TEST_ONLY_KEY").then(|| "synthetic-env-secret".into()))
    }
    fn load(&self, _: &str) -> Result<Option<String>, CredentialError> {
        Ok(Some("synthetic-vault-secret".into()))
    }
    fn store(&self, _: &str, _: Option<&str>) -> Result<(), CredentialError> {
        Ok(())
    }
}
async fn drive(s: &mut Session, id: &str, mut http: HttpRequest) -> Vec<Event> {
    let client = native_client().unwrap();
    let mut events = vec![];
    loop {
        let cancel = s.llm_cancellation_handle(id).unwrap();
        let timeout = s.llm_http_timeout(id).unwrap();
        let (status, error) =
            drive_native_http(&http, timeout, &client, &cancel.token(), |status, bytes| {
                events.extend(s.llm_http_bytes(id, status, bytes).1);
                s.llm_cancellation_handle(id).is_some()
            })
            .await;
        let next = s
            .handle(Request::LlmHttpEnd {
                request_id: id.into(),
                status,
                error,
            })
            .1;
        let request = next.iter().find_map(|e| {
            if let Event::LlmHttp { http, .. } = e {
                Some(http.clone())
            } else {
                None
            }
        });
        events.extend(next);
        if let Some(request) = request {
            http = request;
        } else {
            return events;
        }
    }
}
fn chat() -> Request {
    Request::LlmChat {
        request_id: "chat".into(),
        messages: vec![ChatMessage {
            role: Role::User,
            content: "solve x squared = 4".into(),
            tool_calls: vec![],
            tool_call_id: None,
        }],
    }
}
fn text() -> String {
    format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":{"content":"CAS confirms x = ±2"},"finish_reason":"stop"}]})
    )
}
#[tokio::test]
async fn native_credentials_real_http_tools_and_portable_events_share_one_job() {
    let server = MockServer::start().await;
    let count = Arc::new(AtomicUsize::new(0));
    let counter = count.clone();
    Mock::given(method("POST")).and(header("authorization","Bearer synthetic-env-secret")).respond_with(move |_:&wiremock::Request| {
        let n=counter.fetch_add(1,Ordering::Relaxed);
        let body=if n==0{format!("data: {}\n\ndata: [DONE]\n\n",json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"solve-real","function":{"name":"solve","arguments":"{\"equations\":[\"x^2==4\"],\"variables\":[\"x\"],\"domain\":\"Reals\"}"}}]},"finish_reason":"tool_calls"}]}))}else{text()};
        ResponseTemplate::new(200).set_body_string(body)
    }).expect(2).mount(&server).await;
    let path = std::env::temp_dir().join(format!(
        "openmath-llm-native-{}-config.toml",
        std::process::id()
    ));
    let store = ConfigStore::new(path, Arc::new(Credentials), KeyStorage::Vault);
    let mut s = Session::new_native(store, Some(Arc::new(Time(Instant::now())))).unwrap();
    s.config.llm.profiles[0].api_key_env = Some("TEST_ONLY_KEY".into());
    s.config.llm.profiles[0].base_url = server.uri();
    let Response::LlmStarted {
        http: Some(http), ..
    } = s.handle(chat()).0
    else {
        panic!()
    };
    let events = drive(&mut s, "chat", http).await;
    let tool = events
        .iter()
        .find_map(|e| {
            if let Event::LlmToolCall { result_summary, .. } = e {
                Some(result_summary)
            } else {
                None
            }
        })
        .unwrap();
    let tool: Value = serde_json::from_str(tool).unwrap();
    assert_eq!(tool["supported"], true);
    assert!(tool["solutions"].as_str().unwrap().contains('2'));
    assert!(
        events
            .iter()
            .any(|e| matches!(e,Event::LlmDelta{text,..}if text=="CAS confirms x = ±2"))
    );
    assert!(events.iter().any(|e| matches!(e, Event::LlmDone { .. })));
    assert!(s.notebook.cells.is_empty());
    assert!(
        !serde_json::to_string(&s.handle(Request::GetConfig).0)
            .unwrap()
            .contains("synthetic-env-secret")
    );
    let requests = server.received_requests().await.unwrap();
    assert!(
        std::str::from_utf8(&requests[1].body)
            .unwrap()
            .contains("solve-real")
    );
    // The same actual recorded HTTP fixture bytes yield the same public results in Browser.
    let mut portable = Session::new(
        KernelConfig::default(),
        Some(Arc::new(Time(Instant::now()))),
    );
    portable.set_llm_target(Target::Browser).unwrap();
    portable.config.llm.profiles[0].extra_headers = BTreeMap::new();
    portable.handle(chat());
    let chunks = json!({"choices":[{"index":0,"delta":{"content":"CAS confirms x = ±2"},"finish_reason":"stop"}]});
    let body = format!("data: {chunks}\n\ndata: [DONE]\n\n");
    let mut got = vec![];
    for bytes in body.as_bytes().chunks(1) {
        got.extend(portable.llm_http_bytes("chat", 200, bytes).1);
    }
    got.extend(
        portable
            .handle(Request::LlmHttpEnd {
                request_id: "chat".into(),
                status: 200,
                error: None,
            })
            .1,
    );
    assert!(got.iter().any(|e| matches!(e, Event::LlmDone { .. })));
    assert_eq!(
        got.iter()
            .filter_map(|e| if let Event::LlmDelta { text, .. } = e {
                Some(text.as_str())
            } else {
                None
            })
            .collect::<String>(),
        "CAS confirms x = ±2"
    );
}
#[tokio::test]
async fn native_http_cancellation_and_status_error_remain_session_results() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(403)
                .set_body_json(json!({"error":{"message":"bad synthetic-private"}})),
        )
        .mount(&server)
        .await;
    let mut s = Session::new(
        KernelConfig::default(),
        Some(Arc::new(Time(Instant::now()))),
    );
    s.set_llm_target(Target::Native).unwrap();
    s.config.llm.profiles[0].base_url = server.uri();
    s.config.llm.profiles[0].api_key = Some("synthetic-private".into());
    let Response::LlmStarted {
        http: Some(http), ..
    } = s.handle(chat()).0
    else {
        panic!()
    };
    let events = drive(&mut s, "chat", http).await;
    assert!(events.iter().any(|e|matches!(e,Event::LlmError{message,..}if message.contains("403")&&!message.contains("synthetic-private"))));
    assert!(!events.iter().any(|e| matches!(e, Event::LlmDelta { .. })));
    let Response::LlmStarted {
        http: Some(http), ..
    } = s
        .handle(Request::LlmTestProfile {
            request_id: "cancel".into(),
            profile: "deepseek".into(),
        })
        .0
    else {
        panic!()
    };
    let cancel = s.llm_cancellation_handle("cancel").unwrap();
    cancel.cancel();
    let (status, error) = drive_native_http(
        &http,
        s.llm_http_timeout("cancel").unwrap(),
        &native_client().unwrap(),
        &cancel.token(),
        |_, _| panic!(),
    )
    .await;
    let events = s
        .handle(Request::LlmHttpEnd {
            request_id: "cancel".into(),
            status,
            error,
        })
        .1;
    assert!(events.iter().any(|e|matches!(e,Event::LlmError{message,..}if message.contains("取消")||message.contains("cancel"))));
}
