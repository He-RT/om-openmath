//! Production prompt and validation tests use genuine parsing, records and editor items.
use om_kernel::{
    KernelConfig, Session,
    assistant::{SuggestionParser, filter_completion},
    protocol::*,
};
use om_llm::{
    Feature, Job, JobInput, JobResult, JobStep, Profile, ProviderKind, SuggestionValidator, Target,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn profile() -> Profile {
    Profile {
        name: "fixture".into(),
        kind: ProviderKind::OpenaiChat,
        base_url: "https://fixture.invalid/v1".into(),
        model: "editable".into(),
        api_key: Some("never-context".into()),
        temperature: 0.2,
        max_tokens: 128,
        supports_tools: true,
        supports_json_mode: true,
        timeout_ms: 1000,
        extra_headers: BTreeMap::new(),
    }
}
fn evaluate(s: &mut Session, id: &str, source: &str) -> CellOutput {
    let (r, _) = s.handle(Request::Evaluate {
        cell_id: id.into(),
        source: source.into(),
        dialect: Dialect::Wolfram,
    });
    let Response::Evaluated { output, .. } = r else {
        panic!("{r:?}")
    };
    output
}
fn messages(input: JobInput) -> Vec<ChatMessage> {
    match input {
        JobInput::Text { messages }
        | JobInput::Structured { messages, .. }
        | JobInput::Chat { messages, .. } => messages,
        _ => panic!(),
    }
}
fn reply(job: &mut Job, text: &str) -> JobStep {
    let part = json!({"choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]});
    job.on_bytes(&format!("data: {part}\n\ndata: {{\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}]}}\n\ndata: [DONE]\n\n"));
    job.on_http_end(200, None)
}
#[test]
fn genuine_validator_ignores_model_renderings_and_preserves_poles() {
    for code in [
        "Solve[x/x == 1, x]",
        "Solve[(x^2-1)/(x-1)==0,x]",
        "Root[#^5-#-1&,1]",
        "Solve[α^2 == 4, α, Reals]",
    ] {
        let text = format!(
            "```json\n{}\n```",
            json!({"wolfram":code,"explanation":"真实建议","modern":"bogus","latex":"bogus"})
        );
        let suggestion = SuggestionParser.validate(&text).unwrap();
        let raw = om_parse::parse_expr(code, om_parse::Dialect::Wolfram).unwrap();
        assert_eq!(suggestion.wolfram, om_format::input_form(&raw));
        assert_eq!(suggestion.modern, om_format::modern_form(&raw));
        assert_eq!(suggestion.latex, om_format::latex(&raw));
        assert_ne!(suggestion.modern, "bogus");
    }
    let suggestion = SuggestionParser
        .validate(r#"{"wolfram":"Solve[x/x==1,x]","explanation":"domain"}"#)
        .unwrap();
    assert!(suggestion.wolfram.contains("x/x"));
    let mut s = Session::new(KernelConfig::default(), None);
    let output = evaluate(&mut s, "proposal", &suggestion.wolfram);
    let OutputItem::Solutions { view, .. } = &output.items[0] else {
        panic!("{output:?}")
    };
    assert!(view.solutions.iter().any(|solution| {
        solution
            .condition_latex
            .as_deref()
            .is_some_and(|c| c.contains('0'))
    }));
}
#[test]
fn malformed_empty_multiple_and_duplicate_replies_are_actual_validation_errors() {
    for text in [
        "plain text",
        r#"{"wolfram":"","explanation":"x"}"#,
        r#"{"wolfram":"x\ny","explanation":"x"}"#,
        r#"{"wolfram":"x;","explanation":"x"}"#,
        r#"{"wolfram":"x","wolfram":"y","explanation":"x"}"#,
        r#"{"wolfram":2,"explanation":"x"}"#,
        r#"{"wolfram":"x"}"#,
        r#"{"wolfram":"Solve[x==1,x","explanation":"x"}"#,
    ] {
        assert!(SuggestionParser.validate(text).is_err(), "{text}");
    }
    assert!(SuggestionParser.validate(&"a".repeat(1_048_577)).is_err());
    assert!(
        SuggestionParser
            .validate(r#"{"wolfram":"🙂","explanation":"x"}"#)
            .unwrap_err()
            .contains("E001")
    );
}
#[test]
fn prepared_translate_uses_registered_functions_and_live_symbols_without_mutation() {
    let mut s = Session::new(KernelConfig::default(), None);
    evaluate(&mut s, "definitions", "a=9");
    s.config.general.language = om_kernel::config::Language::En;
    s.config.llm.profiles[0].api_key = Some("do-not-send-key".into());
    let before = s.handle(Request::SaveNotebook).0;
    let req = Request::LlmTranslate {
        request_id: "t".into(),
        text: "solve {{lang}} x squared".into(),
        cell_id: None,
    };
    let (feature, input) = s.prepare_llm_input(&req).unwrap();
    assert_eq!(feature, Feature::Translate);
    let msgs = messages(input);
    assert_eq!(msgs[0].role, Role::System);
    assert!(msgs[0].content.contains("English"));
    assert!(msgs[0].content.contains("Solve"));
    assert!(
        msgs[0]
            .content
            .contains("Symbols already defined in the notebook: a")
    );
    assert_eq!(msgs[1].content, "solve {{lang}} x squared");
    assert!(!format!("{msgs:?}").contains("do-not-send-key"));
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(s.handle(Request::SaveNotebook).0).unwrap()
    );
    assert_eq!(s.notebook.cells[0].exec_count, Some(1));
}
#[test]
fn actual_validator_retries_twice_then_runs_a_valid_proposal_only_on_explicit_evaluate() {
    let mut s = Session::new(KernelConfig::default(), None);
    let (feature, input) = s
        .prepare_llm_input(&Request::LlmTranslate {
            request_id: "t".into(),
            text: "x squared = 4".into(),
            cell_id: None,
        })
        .unwrap();
    let (mut job, step) = Job::new(feature, profile(), input, Target::Browser);
    assert!(matches!(step, JobStep::Http(_)));
    for _ in 0..2 {
        let JobStep::Http(request) = reply(&mut job, r#"{"wolfram":"Solve[","explanation":"x"}"#)
        else {
            panic!()
        };
        assert!(request.body.contains("E023"));
    }
    let JobStep::Done(JobResult::Suggestion(suggestion)) = reply(
        &mut job,
        r#"{"wolfram":"Solve[x^2==4,x]","explanation":"x"}"#,
    ) else {
        panic!()
    };
    assert!(s.notebook.cells.is_empty());
    let output = evaluate(&mut s, "confirmed", &suggestion.wolfram);
    let OutputItem::Solutions { view, .. } = &output.items[0] else {
        panic!()
    };
    assert_eq!(view.solutions.len(), 2);
}
#[test]
fn explanation_contains_real_input_result_step_tree_and_selected_descendants() {
    let mut s = Session::new(KernelConfig::default(), None);
    assert!(
        s.prepare_llm_input(&Request::LlmExplain {
            request_id: "e".into(),
            cell_id: "missing".into(),
            step_id: None
        })
        .is_err()
    );
    let output = evaluate(&mut s, "solve", "Solve[x^2-4==0,x]");
    let OutputItem::Solutions {
        input_form,
        steps: Some(steps),
        ..
    } = &output.items[0]
    else {
        panic!()
    };
    let (_, input) = s
        .prepare_llm_input(&Request::LlmExplain {
            request_id: "e".into(),
            cell_id: "solve".into(),
            step_id: None,
        })
        .unwrap();
    let msgs = messages(input);
    let data: Value = serde_json::from_str(&msgs[1].content).unwrap();
    assert_eq!(data["input"], "Solve[(x^2 - 4) == 0, x]");
    assert_eq!(data["result"], *input_form);
    assert_eq!(data["steps"], serde_json::to_value(steps).unwrap());
    assert!(msgs[0].content.contains("[S{n}]"));
    let root = &steps.root[0];
    let (_, input) = s
        .prepare_llm_input(&Request::LlmExplain {
            request_id: "e".into(),
            cell_id: "solve".into(),
            step_id: Some(root.id.clone()),
        })
        .unwrap();
    let data: Value = serde_json::from_str(&messages(input)[1].content).unwrap();
    assert_eq!(data["steps"]["root"], json!([root]));
    assert!(
        s.prepare_llm_input(&Request::LlmExplain {
            request_id: "e".into(),
            cell_id: "solve".into(),
            step_id: Some("S999999".into())
        })
        .is_err()
    );
    evaluate(&mut s, "plain", "2+2");
    assert!(
        s.prepare_llm_input(&Request::LlmExplain {
            request_id: "e".into(),
            cell_id: "plain".into(),
            step_id: None
        })
        .is_err()
    );
}
#[test]
fn completion_context_uses_three_preceding_math_sources_and_inert_dialect_comments() {
    let mut s = Session::new(KernelConfig::default(), None);
    for (i, source) in [
        "secret_old=1",
        "a=2",
        "(* tricky *) b=3",
        "c=4\n%wl\n*) attack",
    ]
    .iter()
    .enumerate()
    {
        s.handle(Request::UpsertCell {
            cell: CellInput {
                id: i.to_string(),
                source: (*source).into(),
                kind: CellKind::Math,
                dialect: Dialect::Wolfram,
            },
        });
    }
    for dialect in [Dialect::Modern, Dialect::Wolfram] {
        let (_, JobInput::Completion { prefix, suffix }) = s
            .prepare_llm_input(&Request::LlmComplete {
                request_id: "c".into(),
                prefix: "x+".into(),
                suffix: "1".into(),
                dialect,
            })
            .unwrap()
        else {
            panic!()
        };
        assert!(!prefix.contains("secret_old"));
        assert!(prefix.contains("a=2"));
        assert!(prefix.ends_with("x+"));
        assert_eq!(suffix, "1");
        let context = prefix.strip_suffix("x+").unwrap();
        let parsed = om_parse::parse(context, dialect.into());
        assert!(parsed.statements.is_empty(), "{context} {parsed:?}");
        assert!(parsed.diagnostics.is_empty(), "{parsed:?}");
    }
    s.config.llm.send_context = false;
    let (_, JobInput::Completion { prefix, suffix }) = s
        .prepare_llm_input(&Request::LlmComplete {
            request_id: "c".into(),
            prefix: "x+".into(),
            suffix: "1".into(),
            dialect: Dialect::Auto,
        })
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(prefix, "x+");
    assert_eq!(suffix, "1");
}
#[test]
fn completion_preserves_spaces_and_partial_syntax_but_rejects_illegal_tokens_and_duplicates() {
    assert_eq!(
        filter_completion("x", "", "\r\n + 1 \nmore", om_parse::Dialect::Modern, None),
        Some(" + 1 ".into())
    );
    assert_eq!(
        filter_completion("solve(", "", "x^", om_parse::Dialect::Modern, None),
        Some("x^".into())
    );
    assert_eq!(
        filter_completion("Sqrt[", "", "x]", om_parse::Dialect::Wolfram, None),
        Some("x]".into())
    );
    for text in ["🙂", "\\[UnknownName]", "\"\\q\""] {
        assert!(
            filter_completion("", "", text, om_parse::Dialect::Wolfram, None).is_none(),
            "{text}"
        );
    }
    assert!(filter_completion("", "", "sqrt", om_parse::Dialect::Modern, Some("sqrt")).is_none());
    let mut s = Session::new(KernelConfig::default(), None);
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
    assert!(!items.is_empty());
    let insertion = items[0].insert_text.strip_prefix("sq").unwrap();
    assert!(
        s.filter_llm_completion("sq", "", insertion, Dialect::Modern)
            .is_none()
    );
}
#[test]
fn fix_is_grounded_in_original_error_source_and_actual_diagnostics() {
    let mut s = Session::new(KernelConfig::default(), None);
    evaluate(&mut s, "broken", "Solve[x==1,x");
    let (_, input) = s
        .prepare_llm_input(&Request::LlmFixError {
            request_id: "f".into(),
            cell_id: "broken".into(),
        })
        .unwrap();
    let msgs = messages(input);
    let data: Value = serde_json::from_str(&msgs[1].content).unwrap();
    assert_eq!(data["source"], "Solve[x==1,x");
    assert_eq!(data["dialect"], "wolfram");
    assert!(
        data["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E023")
    );
    assert!(msgs[0].content.contains("JSON"));
}
