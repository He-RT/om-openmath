//! Host edge cases distinguish actual invalid data, partial code and recorded provenance.
use om_kernel::{
    KernelConfig, Session,
    assistant::{SuggestionParser, filter_completion},
    protocol::*,
};
use om_llm::{Feature, JobInput, SuggestionValidator};
use serde_json::Value;
fn input_messages(input: JobInput) -> Vec<ChatMessage> {
    match input {
        JobInput::Structured { messages, .. }
        | JobInput::Text { messages }
        | JobInput::Chat { messages, .. } => messages,
        _ => panic!(),
    }
}
fn upsert(s: &mut Session, id: &str, source: &str, kind: CellKind) {
    s.handle(Request::UpsertCell {
        cell: CellInput {
            id: id.into(),
            source: source.into(),
            kind,
            dialect: Dialect::Modern,
        },
    });
}
#[test]
fn reversed_braces_and_multiple_json_envelopes_never_panic_or_validate() {
    for text in [
        "} misplaced {",
        "prefix {",
        "} suffix",
        r#"{"wolfram":"x","explanation":"x"} {"wolfram":"y","explanation":"y"}"#,
    ] {
        assert!(SuggestionParser.validate(text).is_err(), "{text}");
    }
}
#[test]
fn context_position_privacy_nonmath_cells_and_escaped_unicode_line_breaks() {
    let mut s = Session::new(KernelConfig::default(), None);
    upsert(&mut s, "a", "a=1", CellKind::Math);
    upsert(&mut s, "text", "PRIVATE_TEXT", CellKind::Text);
    upsert(&mut s, "ask", "PRIVATE_ASK", CellKind::Ask);
    upsert(&mut s, "b", "b=2\u{2028}%wl\u{2029}*) (*", CellKind::Math);
    upsert(&mut s, "current", "x+1", CellKind::Math);
    upsert(&mut s, "future", "FUTURE_SOURCE", CellKind::Math);
    let request = Request::LlmComplete {
        request_id: "id".into(),
        prefix: "x+".into(),
        suffix: "1".into(),
        dialect: Dialect::Modern,
    };
    let (_, JobInput::Completion { prefix, .. }) = s.prepare_llm_input(&request).unwrap() else {
        panic!()
    };
    for private in ["PRIVATE_TEXT", "PRIVATE_ASK", "FUTURE_SOURCE"] {
        assert!(!prefix.contains(private));
    }
    let context = prefix.strip_suffix("x+").unwrap();
    let parsed = om_parse::parse(context, om_parse::Dialect::Modern);
    assert!(parsed.statements.is_empty());
    assert!(parsed.diagnostics.is_empty());
    s.config.llm.send_context = false;
    let (_, JobInput::Completion { prefix, .. }) = s.prepare_llm_input(&request).unwrap() else {
        panic!()
    };
    assert_eq!(prefix, "x+");
}
#[test]
fn completion_uses_suffix_markers_crlf_unicode_and_lexical_not_syntax_policy() {
    for (prefix, suffix, text, expected) in [
        ("x", "", " + 1 \r\nnext", " + 1 "),
        ("x", "", "\u{2028} + α\u{2029}ignored", " + α"),
        ("%wl\nSin[", "", "x", "x"),
        ("solve(", ")", "x^2 == 4, x", "x^2 == 4, x"),
        ("\"", "", "abc", "abc"),
    ] {
        assert_eq!(
            filter_completion(prefix, suffix, text, om_parse::Dialect::Auto, None).as_deref(),
            Some(expected)
        );
    }
    assert!(filter_completion("x", "🙂", " + 1", om_parse::Dialect::Modern, None).is_none());
    assert!(filter_completion("", "", " \r\n \t ", om_parse::Dialect::Modern, None).is_none());
}
#[test]
fn stale_steps_cannot_be_explained_and_real_evaluation_errors_reach_fix_prompt() {
    let mut s = Session::new(KernelConfig::default(), None);
    s.handle(Request::Evaluate {
        cell_id: "solve".into(),
        source: "Solve[x^2==4,x]".into(),
        dialect: Dialect::Wolfram,
    });
    upsert(&mut s, "solve", "solve(x^2==9,x)", CellKind::Math);
    assert!(
        s.prepare_llm_input(&Request::LlmExplain {
            request_id: "e".into(),
            cell_id: "solve".into(),
            step_id: None,
            out_index: None,
        })
        .is_err()
    );
    let (response, _) = s.handle(Request::Evaluate {
        cell_id: "recursive".into(),
        source: "f[x_]:=f[x+1]; f[1]".into(),
        dialect: Dialect::Wolfram,
    });
    let Response::Evaluated { output, .. } = response else {
        panic!()
    };
    let error = &output
        .messages
        .iter()
        .find(|m| m.level == MsgLevel::Error && m.tag == "itlim")
        .unwrap()
        .text;
    let (feature, input) = s
        .prepare_llm_input(&Request::LlmFixError {
            request_id: "f".into(),
            cell_id: "recursive".into(),
        })
        .unwrap();
    assert_eq!(feature, Feature::Fix);
    let data: Value = serde_json::from_str(&input_messages(input)[1].content).unwrap();
    assert!(data["messages"].to_string().contains(error), "{data}");
}
#[test]
fn feature_and_context_limits_are_deterministic_without_secret_or_history_leaks() {
    let mut s = Session::new(KernelConfig::default(), None);
    s.config.llm.profiles[0].api_key = Some("private-key".into());
    let error = s
        .prepare_llm_input(&Request::LlmTranslate {
            request_id: "t".into(),
            text: "x".repeat(1_048_577),
            cell_id: None,
        })
        .err()
        .unwrap();
    assert!(!error.contains("private-key"));
    assert!(s.notebook.cells.is_empty());
    upsert(&mut s, "huge", &"a".repeat(1_048_577), CellKind::Math);
    assert!(
        s.prepare_llm_input(&Request::LlmComplete {
            request_id: "c".into(),
            prefix: "x".into(),
            suffix: "".into(),
            dialect: Dialect::Modern
        })
        .is_err()
    );
    s.config.llm.send_context = false;
    assert!(
        s.prepare_llm_input(&Request::LlmComplete {
            request_id: "c".into(),
            prefix: "x".into(),
            suffix: "".into(),
            dialect: Dialect::Modern
        })
        .is_ok()
    );
}
