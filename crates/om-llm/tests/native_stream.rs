//! A loopback chunked socket exercises actual body timing and UTF-8 packet boundaries.
#![cfg(feature = "http")]
use om_llm::*;
use std::{collections::BTreeMap, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;

fn job(base: String, timeout_ms: u64) -> Job {
    let p = Profile {
        name: "stream".into(),
        kind: ProviderKind::OpenaiChat,
        base_url: base,
        model: "editable".into(),
        api_key: Some("sk-test".into()),
        temperature: 0.0,
        max_tokens: 32,
        supports_tools: false,
        supports_json_mode: false,
        timeout_ms,
        extra_headers: BTreeMap::new(),
    };
    Job::new(
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
    )
    .0
}
async fn server(
    body: Vec<u8>,
    packet: usize,
    pause: Duration,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![];
        let mut bytes = [0; 1024];
        loop {
            let n = socket.read(&mut bytes).await.unwrap();
            if n == 0 {
                return;
            }
            request.extend_from_slice(&bytes[..n]);
            if request.windows(4).any(|s| s == b"\r\n\r\n") {
                break;
            }
        }
        if socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.is_err(){return;}
        for bytes in body.chunks(packet) {
            let frame = [format!("{:x}\r\n", bytes.len()).as_bytes(), bytes, b"\r\n"].concat();
            if socket.write_all(&frame).await.is_err() {
                return;
            }
            if socket.flush().await.is_err() {
                return;
            }
            tokio::time::sleep(pause).await;
        }
        let _ = socket.write_all(b"0\r\n\r\n").await;
    });
    (base, handle)
}
const BODY: &[u8] = include_bytes!("fixtures/openai_text.sse");
#[tokio::test]
async fn genuine_single_byte_http_chunks_preserve_multibyte_text() {
    let (base, server) = server(BODY.to_vec(), 1, Duration::ZERO).await;
    let mut job = job(base, 5000);
    let mut stream = String::new();
    let step = drive_native(&mut job, &native_client().unwrap(), |event| {
        if let StreamEvent::Text(text) = event {
            stream.push_str(&text)
        }
    })
    .await;
    let JobStep::Done(JobResult::Text(text)) = step else {
        panic!("{step:?}")
    };
    assert_eq!(text, stream);
    assert!(text.contains('α'));
    server.await.unwrap();
}
#[tokio::test]
async fn actual_body_stall_uses_profile_timeout() {
    let (base, server) = server(BODY.to_vec(), 1, Duration::from_millis(100)).await;
    let mut job = job(base, 25);
    assert!(
        matches!(drive_native(&mut job,&native_client().unwrap(),|_|{}).await,JobStep::Failed(LlmError::Transport(t)) if t.contains("timed out"))
    );
    server.abort();
    let _ = server.await;
}
#[tokio::test]
async fn cancellation_from_a_real_body_delta_stops_further_events_and_keeps_terminal_state() {
    let first=b"data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"first\"},\"finish_reason\":null}]}\n\n";
    let mut body = first.to_vec();
    body.extend_from_slice(BODY);
    let (base, server) = server(body, first.len(), Duration::from_millis(200)).await;
    let token = CancellationToken::new();
    let cancel = token.clone();
    let mut events = 0;
    let mut job = job(base, 5000);
    let client = native_client().unwrap();
    let step = drive_native_cancellable(&mut job, &client, &token, |event| {
        if matches!(event, StreamEvent::Text(_)) {
            events += 1;
            cancel.cancel()
        }
    })
    .await;
    assert!(matches!(step, JobStep::Failed(LlmError::Cancelled)));
    assert_eq!(events, 1);
    assert!(matches!(
        drive_native(&mut job, &client, |_| panic!("late event")).await,
        JobStep::Failed(LlmError::Cancelled)
    ));
    server.abort();
    let _ = server.await;
}
#[tokio::test]
async fn connection_failures_do_not_format_configured_url_credentials_or_replay_partial_http() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let mut failed = job(
        format!("http://{address}/private-url-path/private-url-token"),
        1000,
    );
    let step = drive_native(&mut failed, &native_client().unwrap(), |_| {}).await;
    assert!(matches!(&step, JobStep::Failed(LlmError::Transport(_))));
    let formatted = format!("{step:?}");
    assert!(!formatted.contains("private-url-token"));
    assert!(!formatted.contains("private-url-path"));
    let mut partial = job("http://127.0.0.1:1".into(), 1000);
    partial.on_bytes("data: ");
    assert!(matches!(
        drive_native(&mut partial, &native_client().unwrap(), |_| panic!()).await,
        JobStep::Failed(LlmError::State(_))
    ));
}
