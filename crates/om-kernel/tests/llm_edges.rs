//! Actual stream/status/route/tool/clock boundaries, without fake provider or evaluator success.
use om_kernel::{KernelConfig, Session, protocol::*};
use om_num::ctx::Clock;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
#[derive(Default)]
struct Time(AtomicU64);
impl Clock for Time {
    fn now_ms(&self) -> f64 {
        self.0.fetch_add(6000, Ordering::Relaxed) as f64
    }
}
struct Fixed;
impl Clock for Fixed {
    fn now_ms(&self) -> f64 {
        0.0
    }
}
fn session() -> Session {
    Session::new(KernelConfig::default(), Some(Arc::new(Fixed)))
}
fn request(id: &str) -> Request {
    Request::LlmChat {
        request_id: id.into(),
        messages: vec![ChatMessage {
            role: Role::User,
            content: "CAS".into(),
            tool_calls: vec![],
            tool_call_id: None,
        }],
    }
}
fn begin(s: &mut Session, id: &str) {
    assert!(matches!(
        s.handle(request(id)).0,
        Response::LlmStarted { .. }
    ));
}
fn tools(s: &mut Session, id: &str, name: &str, args: &str) -> Vec<Event> {
    let chunk = format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call","function":{"name":name,"arguments":args}}]},"finish_reason":"tool_calls"}]})
    );
    s.llm_http_bytes(id, 200, chunk.as_bytes());
    s.handle(Request::LlmHttpEnd {
        request_id: id.into(),
        status: 200,
        error: None,
    })
    .1
}
fn result(events: &[Event]) -> Value {
    serde_json::from_str(
        events
            .iter()
            .find_map(|e| {
                if let Event::LlmToolCall { result_summary, .. } = e {
                    Some(result_summary)
                } else {
                    None
                }
            })
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn status_errors_never_stream_error_bodies_or_leak_credentials() {
    let mut s = session();
    s.config.llm.profiles[0].api_key = Some("secret-token".into());
    begin(&mut s, "unauthorized");
    assert!(
        s.llm_http_bytes(
            "unauthorized",
            401,
            br#"{"error":{"message":"bad secret-token"}}"#
        )
        .1
        .is_empty()
    );
    let events = s
        .handle(Request::LlmHttpEnd {
            request_id: "unauthorized".into(),
            status: 401,
            error: None,
        })
        .1;
    assert!(
        matches!(&events[0],Event::LlmError{message,..} if message.contains("401")&&message.contains("***")&&message.contains("API Key")&&!message.contains("secret-token"))
    );
    begin(&mut s, "conflict");
    s.llm_http_bytes("conflict", 200, b"data: ");
    assert!(
        s.llm_http_bytes("conflict", 403, b"invalid")
            .1
            .iter()
            .any(|e| matches!(e, Event::LlmError { .. }))
    );
    assert!(
        s.handle(Request::LlmHttpEnd {
            request_id: "conflict".into(),
            status: 403,
            error: None
        })
        .1
        .is_empty()
    );
}
#[test]
fn invalid_streams_terminal_limits_missing_finish_and_unknown_ids_have_real_errors() {
    let mut s = session();
    assert!(matches!(
        s.llm_http_bytes("missing", 200, b"x").0,
        Response::Error { .. }
    ));
    for (id, body) in [
        ("invalid", b"data: {bad}\n\n".to_vec()),
        ("utf8", vec![0xff]),
        ("limit", vec![b'x'; 1_048_577]),
    ] {
        begin(&mut s, id);
        let events = s.llm_http_bytes(id, 200, &body).1;
        assert!(
            events.iter().any(|e| matches!(e, Event::LlmError { .. })),
            "{id} {events:?}"
        );
        assert!(s.llm_http_bytes(id, 200, b"late").1.is_empty());
    }
    begin(&mut s, "incomplete");
    assert!(
        s.handle(Request::LlmHttpEnd {
            request_id: "incomplete".into(),
            status: 200,
            error: None
        })
        .1
        .iter()
        .any(|e| matches!(e, Event::LlmError { .. }))
    );
}
#[test]
fn real_tool_schema_unknown_functions_unsupported_solver_and_missing_clock_are_honest() {
    let mut s = session();
    begin(&mut s, "schema");
    for (name, args) in [
        ("evaluate", r#"{"code":"2+2","extra":true}"#),
        ("evaluate", r#"{"code":2}"#),
        (
            "solve",
            r#"{"equations":["x==1"],"variables":["x"],"domain":"Rationals"}"#,
        ),
        ("solve", r#"{"equations":["x==1"],"variables":["x","x"]}"#),
        ("missing", "{}"),
        ("propose_cell", r#"{"code":"x","dialect":"auto"}"#),
    ] {
        let events = tools(&mut s, "schema", name, args);
        assert!(result(&events)["error"].is_string());
    }
    let mut s = session();
    begin(&mut s, "unsupported");
    let events = tools(
        &mut s,
        "unsupported",
        "solve",
        r#"{"equations":["Sin[x]+x==0"],"variables":["x"]}"#,
    );
    let output = result(&events);
    assert_eq!(output["supported"], false);
    assert!(output["solutions"].as_str().unwrap().contains("Solve"));
    assert!(!output["messages"].as_array().unwrap().is_empty());
    let mut s = Session::new(KernelConfig::default(), None);
    begin(&mut s, "clock");
    assert!(
        result(&tools(&mut s, "clock", "evaluate", r#"{"code":"2+2"}"#))["error"]
            .as_str()
            .unwrap()
            .contains("clock")
    );
}
#[test]
fn injected_deadline_stops_real_tools_and_recovery_leaves_notebook_and_interrupt_untouched() {
    let mut s = Session::new(KernelConfig::default(), Some(Arc::new(Time::default())));
    begin(&mut s, "timeout");
    let events = tools(&mut s, "timeout", "evaluate", r#"{"code":"2+2"}"#);
    assert!(
        result(&events)["error"]
            .as_str()
            .unwrap()
            .contains("time limit")
    );
    assert!(s.notebook.cells.is_empty());
    assert!(!s.interrupt_handle().load(Ordering::Relaxed));
    let mut s = session();
    begin(&mut s, "writes");
    for code in [
        "Unset[a]",
        "1+(a=2)",
        "a=1;b=2",
        "f[x_]:=x",
        "Hold[Clear[a]]",
    ] {
        let events = tools(
            &mut s,
            "writes",
            "evaluate",
            &json!({"code":code}).to_string(),
        );
        assert!(result(&events)["error"].is_string());
    }
    assert!(s.notebook.cells.is_empty());
}
#[test]
fn active_limits_tombstones_cancel_and_notebook_load_cleanup() {
    let mut s = session();
    for i in 0..16 {
        begin(&mut s, &format!("id{i}"));
    }
    assert!(matches!(
        s.handle(request("over")).0,
        Response::Error { .. }
    ));
    let (duplicate, events) = s.handle(request("id0"));
    assert!(matches!(duplicate, Response::Error { .. }));
    assert!(
        events.is_empty(),
        "duplicate start must not terminate existing work: {events:?}"
    );
    assert!(s.llm_cancellation_handle("id0").is_some());
    let events = s
        .handle(Request::LlmCancel {
            request_id: "id0".into(),
        })
        .1;
    assert!(events.iter().any(|e| matches!(e, Event::LlmError { .. })));
    assert!(
        s.handle(Request::LlmCancel {
            request_id: "id0".into()
        })
        .1
        .is_empty()
    );
    assert!(matches!(s.handle(request("id0")).0, Response::Error { .. }));
    let (reply, events) = s.handle(Request::LoadNotebook {
        file: NotebookFile {
            version: 1,
            title: "new".into(),
            cells: vec![],
        },
    });
    assert!(matches!(reply, Response::Ok));
    assert_eq!(events.len(), 15);
    assert!(s.llm_cancellation_handle("id1").is_none());
    begin(&mut s, "new");
}
#[test]
fn effective_profiles_capabilities_legacy_status_and_genuine_fix_explain_requests() {
    let mut s = session();
    s.config.general.language = om_kernel::config::Language::En;
    s.config.llm.profiles[0].supports_tools = false;
    assert!(matches!(
        s.handle(request("no-tools")).0,
        Response::Error { .. }
    ));
    s.config.llm.enabled = false;
    assert!(matches!(
        s.handle(Request::LlmTestProfile {
            request_id: "probe".into(),
            profile: "deepseek".into(),
            config: None
        })
        .0,
        Response::LlmStarted { .. }
    ));
    s.config.llm.enabled = true;
    s.config.llm.profiles[0].supports_tools = true;
    s.handle(Request::Evaluate {
        cell_id: "real".into(),
        source: "Solve[x^2==4,x]".into(),
        dialect: Dialect::Wolfram,
    });
    let Response::LlmStarted {
        http: Some(http), ..
    } = s
        .handle(Request::LlmExplain {
            request_id: "explain".into(),
            cell_id: "real".into(),
            step_id: None,
            out_index: None,
        })
        .0
    else {
        panic!()
    };
    assert!(http.body.contains("steps"));
    assert!(http.body.contains("S1"));
    s.handle(Request::Evaluate {
        cell_id: "broken".into(),
        source: "Solve[".into(),
        dialect: Dialect::Wolfram,
    });
    let Response::LlmStarted {
        http: Some(http), ..
    } = s
        .handle(Request::LlmFixError {
            request_id: "fix".into(),
            cell_id: "broken".into(),
        })
        .0
    else {
        panic!()
    };
    assert!(http.body.contains("E023"));
    let events=s.handle(Request::LlmHttpChunk {request_id:"explain".into(),chunk:"data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"real steps [S1]\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n".into(),status:None}).1;
    assert!(
        events
            .iter()
            .any(|e| matches!(e,Event::LlmDelta{text,..}if text.contains("[S1]")))
    );
    assert!(
        s.handle(Request::LlmHttpEnd {
            request_id: "explain".into(),
            status: 200,
            error: None
        })
        .1
        .iter()
        .any(|e| matches!(e, Event::LlmDone { .. }))
    );
}
