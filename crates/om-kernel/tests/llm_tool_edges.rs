//! The real readonly evaluator follows notebook function semantics and explicit tool bounds.
use om_kernel::{KernelConfig, Session, protocol::*};
use om_num::ctx::Clock;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
struct Fixed;
impl Clock for Fixed {
    fn now_ms(&self) -> f64 {
        0.0
    }
}
fn session() -> Session {
    Session::new(KernelConfig::default(), Some(Arc::new(Fixed)))
}
fn begin(s: &mut Session) {
    let response = s
        .handle(Request::LlmChat {
            request_id: "tools".into(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: "use tools".into(),
                tool_calls: vec![],
                tool_call_id: None,
            }],
        })
        .0;
    assert!(
        matches!(response, Response::LlmStarted { .. }),
        "{response:?}"
    );
}
fn tool(s: &mut Session, name: &str, args: Value) -> Vec<Event> {
    let body = format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"same-actual-id","function":{"name":name,"arguments":args.to_string()}}]},"finish_reason":"tool_calls"}]})
    );
    s.llm_http_bytes("tools", 200, body.as_bytes());
    s.handle(Request::LlmHttpEnd {
        request_id: "tools".into(),
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
fn live_functions_and_readonly_nested_defined_writes_preserve_parent_history() {
    let mut s = session();
    s.handle(Request::Evaluate {
        cell_id: "defs".into(),
        source: "f[x_]:=x+1; g[x_]:=(a=7); a=9".into(),
        dialect: Dialect::Wolfram,
    });
    begin(&mut s);
    let got = tool(&mut s, "evaluate", json!({"code":"f[2]"}));
    assert_eq!(result(&got)["input_form"], "3");
    let got = tool(&mut s, "evaluate", json!({"code":"g[2]"}));
    assert!(
        result(&got)["error"]
            .as_str()
            .unwrap()
            .contains("read-only")
    );
    let got = tool(&mut s, "evaluate", json!({"code":"a"}));
    assert_eq!(result(&got)["input_form"], "9");
    let got = tool(
        &mut s,
        "propose_cell",
        json!({"code":"f(2)","dialect":"modern"}),
    );
    assert!(
        got.iter()
            .any(|e| matches!(e,Event::LlmSuggestion{suggestion,..}if suggestion.wolfram=="f[2]"))
    );
    assert_eq!(s.notebook.cells.len(), 1);
    assert_eq!(s.notebook.cells[0].exec_count, Some(1));
}
#[test]
fn reflected_credentials_are_rejected_before_execution_and_removed_from_public_arguments() {
    let mut s = session();
    s.config.llm.profiles[0].api_key = Some("synthetic-secret".into());
    begin(&mut s);
    let got = tool(
        &mut s,
        "propose_cell",
        json!({"code":"\"synthetic-secret\"","dialect":"wolfram"}),
    );
    assert!(
        result(&got)["error"]
            .as_str()
            .unwrap()
            .contains("credential")
    );
    assert!(!got.iter().any(|e| matches!(e, Event::LlmSuggestion { .. })));
    for event in &got {
        if let Event::LlmToolCall {
            arguments,
            result_summary,
            ..
        } = event
        {
            assert!(!arguments.contains("synthetic-secret"));
            assert!(!result_summary.contains("synthetic-secret"));
        }
        if let Event::LlmHttp { http, .. } = event {
            assert!(!http.body.contains("synthetic-secret"));
            assert!(
                http.headers
                    .iter()
                    .any(|(_, v)| v.contains("synthetic-secret"))
            );
        }
    }
}
#[test]
fn seventh_real_tool_round_fails_before_cas_execution_and_no_future_http() {
    let mut s = session();
    begin(&mut s);
    for _ in 0..6 {
        let events = tool(&mut s, "evaluate", json!({"code":"2+2"}));
        assert_eq!(result(&events)["input_form"], "4");
    }
    let events = tool(&mut s, "evaluate", json!({"code":"2+3"}));
    assert!(
        events
            .iter()
            .any(|e| matches!(e,Event::LlmError{message,..}if message.contains("round limit")))
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::LlmToolCall { .. } | Event::LlmHttp { .. }))
    );
}
struct Cancelling(Mutex<Option<om_kernel::LlmCancellation>>);
impl Clock for Cancelling {
    fn now_ms(&self) -> f64 {
        if let Some(handle) = self.0.lock().unwrap().as_ref() {
            handle.cancel();
        }
        0.0
    }
}
#[test]
fn independent_cancellation_stops_tool_execution_without_resetting_notebook_interrupt() {
    let clock = Arc::new(Cancelling(Mutex::new(None)));
    let mut s = Session::new(KernelConfig::default(), Some(clock.clone()));
    begin(&mut s);
    *clock.0.lock().unwrap() = s.llm_cancellation_handle("tools");
    let events = tool(&mut s, "evaluate", json!({"code":"Factor[x^8-1]"}));
    assert!(events.iter().any(|e|matches!(e,Event::LlmError{message,..}if message.contains("取消")||message.contains("cancel"))));
    assert!(!events.iter().any(|e| matches!(e, Event::LlmHttp { .. })));
    assert!(
        !s.interrupt_handle()
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    assert!(s.notebook.cells.is_empty());
}
