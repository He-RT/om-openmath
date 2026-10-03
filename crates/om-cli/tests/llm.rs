//! Native HTTP is exercised only against loopback with synthetic credentials.
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
    time::Duration,
};

#[test]
fn native_probe_and_confirmed_translation_share_the_real_job_and_cas() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let config = std::env::temp_dir().join(format!("openmath-cli-llm-{}.toml", std::process::id()));
    let mut settings = om_kernel::KernelConfig::default();
    let profile = &mut settings.llm.profiles[0];
    profile.base_url = format!("http://{addr}");
    profile.api_key_env = Some("OM_CLI_FIXTURE_KEY".into());
    profile.timeout_ms = 2000;
    profile
        .extra_headers
        .insert("X-Fixture".into(), "synthetic-header".into());
    profile.extra_body.insert(
        "nested".into(),
        serde_json::json!({"value":"synthetic-body"}),
    );
    std::fs::write(&config, toml::to_string(&settings).unwrap()).unwrap();
    let server = std::thread::spawn(move || {
        for reply in [
            "fixture-pong",
            r#"{"wolfram":"Solve[x^2==4,x]","explanation":"Loopback fixture"}"#,
            r#"{"wolfram":"Solve[x^2==4,x]","explanation":"Loopback fixture"}"#,
        ] {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut input = vec![];
            let header_end = loop {
                let mut byte = [0];
                socket.read_exact(&mut byte).unwrap();
                input.push(byte[0]);
                if input.ends_with(b"\r\n\r\n") {
                    break input.len();
                }
                assert!(input.len() < 65536);
            };
            let headers = String::from_utf8(input).unwrap();
            assert!(headers.starts_with("POST /chat/completions HTTP/1.1"));
            assert!(
                headers
                    .to_ascii_lowercase()
                    .contains("authorization: bearer synthetic-cli-key")
            );
            let length: usize = headers
                .lines()
                .find_map(|s| {
                    s.split_once(':')
                        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .map(|(_, value)| value.trim().parse().unwrap())
                })
                .unwrap();
            let mut bytes = vec![0; length];
            socket.read_exact(&mut bytes).unwrap();
            let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["stream"], true);
            assert_eq!(body["model"], "deepseek-flash");
            let event = serde_json::json!({"choices":[{"index":0,"delta":{"content":reply},"finish_reason":"stop"}]});
            let output = format!("data: {event}\n\ndata: [DONE]\n\n");
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{output}", output.len()).unwrap();
            assert!(header_end > 0);
        }
    });
    let base = || {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_om"));
        cmd.args(["--config", config.to_str().unwrap(), "--language", "en"])
            .env("OM_CLI_FIXTURE_KEY", "synthetic-cli-key");
        cmd
    };
    let probe = base()
        .args(["--json", "llm", "test", "deepseek"])
        .output()
        .unwrap();
    assert!(
        probe.status.success(),
        "{}",
        String::from_utf8_lossy(&probe.stderr)
    );
    let reply: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
    assert_eq!(reply["profile_reply"], "fixture-pong");
    assert!(reply["latency_ms"].is_number());
    let shown = base().args(["config", "show"]).output().unwrap();
    assert!(shown.status.success());
    let shown = String::from_utf8(shown.stdout).unwrap();
    let durable: toml::Value = toml::from_str(&shown).unwrap();
    assert_eq!(durable["general"]["reactive"].as_bool(), Some(true));
    for secret in ["synthetic-cli-key", "synthetic-header", "synthetic-body"] {
        assert!(!shown.contains(secret));
    }
    for choice in ["e", "y"] {
        let mut child = base()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        write!(
            child.stdin.take().unwrap(),
            "? solve x squared equals four\ny\n{choice}\n:quit\n"
        )
        .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        let errors = String::from_utf8(output.stderr).unwrap();
        assert!(text.contains("Loopback fixture"), "{text}; {errors}");
        if choice == "e" {
            assert!(!text.contains("Out["));
            assert!(errors.contains("Editable source (not run)"));
        } else {
            assert!(text.contains("x = −2") && text.contains("x = 2"));
        }
        assert!(!text.contains("synthetic-cli-key") && !errors.contains("synthetic-cli-key"));
    }
    server.join().unwrap();
    std::fs::remove_file(config).unwrap();
}
