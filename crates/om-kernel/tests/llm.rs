//! Actual protocol requests, provider fixtures and readonly CAS results end to end.
use om_kernel::{KernelConfig, Session, protocol::*};
use om_llm::{ProviderKind, Target};
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
        self.0.load(Ordering::Relaxed) as f64
    }
}
fn session() -> (Session, Arc<Time>) {
    let clock = Arc::new(Time::default());
    let mut config = KernelConfig::default();
    config.llm.profiles[0].base_url = "https://fixture.invalid/v1".into();
    config.llm.profiles[0].api_key = Some("synthetic-private".into());
    config.llm.profiles[1].base_url = "https://fixture.invalid/beta".into();
    (Session::new(config, Some(clock.clone())), clock)
}
fn translate(id: &str) -> Request {
    Request::LlmTranslate {
        request_id: id.into(),
        text: "solve x squared equals 4".into(),
        cell_id: None,
    }
}
fn start(s: &mut Session, request: Request) -> HttpRequest {
    let (reply, events) = s.handle(request);
    assert!(events.is_empty(), "{events:?}");
    let Response::LlmStarted {
        http: Some(http), ..
    } = reply
    else {
        panic!("{reply:?}")
    };
    http
}
fn text(value: &str) -> String {
    format!(
        "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":{"content":value},"finish_reason":null}]}),
        json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})
    )
}
fn feed(s: &mut Session, id: &str, value: &str) -> Vec<Event> {
    let mut events = s
        .handle(Request::LlmHttpChunk {
            request_id: id.into(),
            chunk: text(value),
            status: Some(200),
        })
        .1;
    events.extend(
        s.handle(Request::LlmHttpEnd {
            request_id: id.into(),
            status: 200,
            error: None,
        })
        .1,
    );
    events
}
fn chat() -> Request {
    Request::LlmChat {
        request_id: "chat".into(),
        messages: vec![ChatMessage {
            role: Role::User,
            content: "compute with CAS".into(),
            tool_calls: vec![],
            tool_call_id: None,
        }],
    }
}
fn tool_chunk(name: &str, args: Value, id: &str) -> String {
    format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":id,"function":{"name":name,"arguments":args.to_string()}}]},"finish_reason":"tool_calls"}]})
    )
}
fn tools(s: &mut Session, name: &str, args: Value, id: &str) -> Vec<Event> {
    s.handle(Request::LlmHttpChunk {
        request_id: "chat".into(),
        chunk: tool_chunk(name, args, id),
        status: Some(200),
    });
    s.handle(Request::LlmHttpEnd {
        request_id: "chat".into(),
        status: 200,
        error: None,
    })
    .1
}
#[test]
fn translation_is_a_real_checked_suggestion_and_runs_only_after_explicit_action() {
    let (mut s, _) = session();
    let http = start(&mut s, translate("t"));
    assert_eq!(http.url, "https://fixture.invalid/v1/chat/completions");
    assert!(
        http.headers.iter().any(
            |(k, v)| k.eq_ignore_ascii_case("authorization") && v.contains("synthetic-private")
        )
    );
    let events = feed(
        &mut s,
        "t",
        r#"{"wolfram":"Solve[x/x==1,x]","explanation":"domain","latex":"bogus"}"#,
    );
    let suggestion = events
        .iter()
        .find_map(|e| {
            if let Event::LlmSuggestion { suggestion, .. } = e {
                Some(suggestion)
            } else {
                None
            }
        })
        .unwrap();
    assert!(suggestion.wolfram.contains("x/x"));
    assert_ne!(suggestion.latex, "bogus");
    assert!(events.iter().any(|e| matches!(e, Event::LlmDone { .. })));
    assert!(s.notebook.cells.is_empty());
    let Response::Evaluated { output, .. } = s
        .handle(Request::Evaluate {
            cell_id: "confirmed".into(),
            source: suggestion.wolfram.clone(),
            dialect: Dialect::Wolfram,
        })
        .0
    else {
        panic!()
    };
    let OutputItem::Solutions { view, .. } = &output.items[0] else {
        panic!()
    };
    assert!(view.solutions.iter().any(|v| v.condition_latex.is_some()));
    assert!(
        s.handle(Request::LlmHttpEnd {
            request_id: "t".into(),
            status: 200,
            error: None
        })
        .1
        .is_empty()
    );
}
#[test]
fn genuine_validation_retries_then_exhaustion_and_late_chunks_are_terminal() {
    let (mut s, _) = session();
    start(&mut s, translate("retry"));
    for _ in 0..2 {
        let events = feed(
            &mut s,
            "retry",
            r#"{"wolfram":"Solve[","explanation":"bad"}"#,
        );
        let Event::LlmHttp { http, .. } = events.last().unwrap() else {
            panic!("{events:?}")
        };
        assert!(http.body.contains("E023"));
    }
    let events = feed(
        &mut s,
        "retry",
        r#"{"wolfram":"Solve[","explanation":"bad"}"#,
    );
    assert!(events.iter().any(|e| matches!(e, Event::LlmError { .. })));
    assert!(!events.iter().any(|e| matches!(e, Event::LlmDone { .. })));
    assert!(
        s.handle(Request::LlmHttpChunk {
            request_id: "retry".into(),
            chunk: text("late"),
            status: Some(200)
        })
        .1
        .is_empty()
    );
}
#[test]
fn completion_filters_real_fim_text_and_local_duplicates_before_delivery() {
    let (mut s, _) = session();
    let request = |id: &str, prefix: &str| Request::LlmComplete {
        request_id: id.into(),
        prefix: prefix.into(),
        suffix: "".into(),
        dialect: Dialect::Modern,
    };
    let http = start(&mut s, request("c", "x"));
    assert!(!http.stream);
    s.handle(Request::LlmHttpChunk {
        request_id: "c".into(),
        chunk: r#"{"choices":[{"text":" + α\nignored"}]}"#.into(),
        status: Some(200),
    });
    let events = s
        .handle(Request::LlmHttpEnd {
            request_id: "c".into(),
            status: 200,
            error: None,
        })
        .1;
    assert!(
        events
            .iter()
            .any(|e| matches!(e,Event::LlmDelta {text,..} if text==" + α"))
    );
    let Response::Completions { items, .. } = s
        .handle(Request::Complete {
            source: "sq".into(),
            cursor: 2,
            dialect: Dialect::Modern,
        })
        .0
    else {
        panic!()
    };
    let insertion = items[0].insert_text.strip_prefix("sq").unwrap();
    start(&mut s, request("duplicate", "sq"));
    s.handle(Request::LlmHttpChunk {
        request_id: "duplicate".into(),
        chunk: json!({"choices":[{"text":insertion}]}).to_string(),
        status: Some(200),
    });
    let events = s
        .handle(Request::LlmHttpEnd {
            request_id: "duplicate".into(),
            status: 200,
            error: None,
        })
        .1;
    assert!(!events.iter().any(|e| matches!(e, Event::LlmDelta { .. })));
    assert!(events.iter().any(|e| matches!(e, Event::LlmDone { .. })));
}
#[test]
fn actual_evaluate_and_solve_tools_replay_results_without_parent_state_changes() {
    let (mut s, _) = session();
    s.handle(Request::Evaluate {
        cell_id: "defs".into(),
        source: "a=9".into(),
        dialect: Dialect::Wolfram,
    });
    let before = serde_json::to_value(s.handle(Request::SaveNotebook).0).unwrap();
    start(&mut s, chat());
    let events = tools(&mut s, "evaluate", json!({"code":"a+1"}), "call1");
    let Event::LlmToolCall { result_summary, .. } = &events[0] else {
        panic!("{events:?}")
    };
    let value: Value = serde_json::from_str(result_summary).unwrap();
    assert_eq!(value["input_form"], "10");
    let events = tools(
        &mut s,
        "solve",
        json!({"equations":["x^2==4"],"variables":["x"],"domain":"Reals"}),
        "call2",
    );
    let Event::LlmToolCall { result_summary, .. } = &events[0] else {
        panic!("{events:?}")
    };
    let value: Value = serde_json::from_str(result_summary).unwrap();
    assert_eq!(value["supported"], true);
    assert!(value["solutions"].as_str().unwrap().contains("2"));
    assert!(!value["steps_summary"].as_array().unwrap().is_empty());
    let Event::LlmHttp { http, .. } = events.last().unwrap() else {
        panic!()
    };
    assert!(http.body.contains("call1"));
    assert!(http.body.contains("call2"));
    assert!(
        feed(&mut s, "chat", "Based on the CAS results: 10 and roots ±2")
            .iter()
            .any(|e| matches!(e, Event::LlmDone { .. }))
    );
    assert_eq!(
        before,
        serde_json::to_value(s.handle(Request::SaveNotebook).0).unwrap()
    );
    assert_eq!(s.notebook.cells[0].exec_count, Some(1));
}
#[test]
fn nested_writes_bad_schema_and_proposals_are_checked_without_execution() {
    let (mut s, _) = session();
    start(&mut s, chat());
    for (i, code) in ["a=1", "Hold[Set[a,1]]", "Clear[a]", "f[x_]:=x"]
        .iter()
        .enumerate()
    {
        let events = tools(
            &mut s,
            "evaluate",
            json!({"code":code}),
            &format!("write{i}"),
        );
        let Event::LlmToolCall { result_summary, .. } = &events[0] else {
            panic!()
        };
        assert!(result_summary.contains("error"));
    }
    let events = tools(
        &mut s,
        "solve",
        json!({"equations":["x==1"],"variables":["x+1"]}),
        "badvar",
    );
    assert!(
        matches!(&events[0],Event::LlmToolCall {result_summary,..} if result_summary.contains("error"))
    );
    let events = tools(
        &mut s,
        "propose_cell",
        json!({"code":"solve(x/x==1,x)","dialect":"modern"}),
        "proposal",
    );
    assert!(events.iter().any(
        |e| matches!(e,Event::LlmSuggestion {suggestion,..} if suggestion.wolfram.contains("x/x"))
    ));
    assert!(s.notebook.cells.is_empty());
}
#[test]
fn real_probe_byte_timing_cancellation_and_route_errors() {
    let (mut s, clock) = session();
    clock.0.store(100, Ordering::Relaxed);
    let http = start(
        &mut s,
        Request::LlmTestProfile {
            request_id: "probe".into(),
            profile: "deepseek".into(),
            config: None,
        },
    );
    let body: Value = serde_json::from_str(&http.body).unwrap();
    assert_eq!(body["max_tokens"], 8);
    clock.0.store(130, Ordering::Relaxed);
    s.llm_http_bytes("probe", 200, text("pong").as_bytes());
    clock.0.store(180, Ordering::Relaxed);
    let events = s
        .handle(Request::LlmHttpEnd {
            request_id: "probe".into(),
            status: 200,
            error: None,
        })
        .1;
    assert!(events.iter().any(|e|matches!(e,Event::LlmProfileTest {latency_ms:Some(80.0),first_byte_ms:Some(30.0),response,..} if response=="pong")));
    start(&mut s, translate("cancel"));
    let handle = s.llm_cancellation_handle("cancel").unwrap();
    handle.cancel();
    let events = s.llm_http_bytes("cancel", 200, text("late").as_bytes()).1;
    assert!(events.iter().any(|e| matches!(e, Event::LlmError { .. })));
    s.config.llm.enabled = false;
    assert!(matches!(
        s.handle(translate("disabled")).0,
        Response::Error { .. }
    ));
    s.config.llm.enabled = true;
    s.config.llm.translate = "missing".into();
    assert!(matches!(
        s.handle(translate("missing")).0,
        Response::Error { .. }
    ));
    s.config.llm.translate = "deepseek".into();
    s.config.llm.profiles[0].api_key = Some("***".into());
    assert!(matches!(
        s.handle(translate("masked")).0,
        Response::Error { .. }
    ));
    assert!(s.set_llm_target(Target::Browser).is_ok());
    assert_eq!(s.config.llm.profiles[0].kind, ProviderKind::OpenaiChat);
}
