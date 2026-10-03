//! JSON authority fixtures, leaf adapter compatibility and TypeScript export checks.
use om_kernel::{config::KernelConfig, protocol::*};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use ts_rs::{Config, TS};

fn wire_value<T: Serialize>(value: T) -> Value {
    serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap()
}

fn roundtrip<T: Serialize + DeserializeOwned>(fixture: Value) -> T {
    let value: T = serde_json::from_value(fixture.clone()).unwrap();
    assert_eq!(wire_value(&value), fixture);
    value
}

fn cell() -> Value {
    json!({"id":"cell-α", "kind":"Math", "source":"solve(x^2=2,x)", "dialect":"Modern"})
}

fn notebook() -> Value {
    json!({"version":1,"title":"二次方程", "cells":[cell(),
        {"id":"text", "kind":"Text", "source":"# 说明", "dialect":"Auto"},
        {"id":"ask", "kind":"Ask", "source":"解方程", "dialect":"Wolfram"}]})
}

fn config() -> Value {
    json!({"general":{"language":"auto","dialect":"auto","constants":"math",
    "reactive":true,"auto_run_dependents":true,"show_steps":true,"auto_plot":true,
    "eval_timeout_ms":30000},"llm":{"enabled":true,"translate":"deepseek",
    "explain":"deepseek","chat":"deepseek","complete":"deepseek-fim",
    "fix":"deepseek","send_context":true,"profiles":[
    {"name":"deepseek","kind":"openai_chat","base_url":"https://api.deepseek.com/v1",
        "model":"deepseek-chat","api_key_env":"DEEPSEEK_API_KEY","api_key":null,
        "temperature":0.2,"max_tokens":1024,"supports_tools":true,
        "supports_json_mode":true,"timeout_ms":60000,"extra_headers":{}},
    {"name":"deepseek-fim","kind":"openai_fim","base_url":"https://api.deepseek.com/beta",
        "model":"deepseek-chat","api_key_env":"DEEPSEEK_API_KEY","api_key":null,
        "temperature":0.2,"max_tokens":64,"supports_tools":false,
        "supports_json_mode":false,"timeout_ms":60000,"extra_headers":{}}
    ]}})
}

fn plot() -> Value {
    json!({"kind":"Function","exprs":["x^2","2"],"var_x":"x","var_y":null,
        "x_range":[-5.0,5.0],"y_range":null,"params":{"a":1.0},
        "points":[[1.414,2.0]],"shade":[[-1e308,0.0]],"param_ranges":{"a":[-5.0,5.0]}})
}

fn data() -> Value {
    json!({"curves":[{"label":"1/x", "segments":[[[-1.0,-1.0],[-0.1,-10.0]],
        [[0.1,10.0],[1.0,1.0]]]}], "x_range":[-1.0,1.0], "y_range":[-10.0,10.0]})
}

fn http() -> Value {
    json!({"method":"POST","url":"https://example.invalid/v1/chat/completions",
        "headers":[["Content-Type","application/json"],["X-Tag","α"]],
        "body":"{\"stream\":true}","stream":true})
}

fn preview() -> Value {
    json!({"latex":"x^2 = 2", "diagnostics":[{"span":{"start":0,"end":2},
        "severity":"Warning","code":"W001","message":"提示", "fix":{
        "span":{"start":2,"end":2},"replacement":"*","label":"插入乘号"}}],
        "tokens":[[{"start":0,"end":1},"Identifier"],[{"start":1,"end":2},"Operator"]],
        "dialect":"Modern", "actions":[{"label_key":"action.solve","source":"solve(x^2=2,x)"}]})
}

fn step(id: &str, children: Vec<Value>) -> Value {
    json!({"id":id,"rule_id":"quadratic_formula","level":"Major",
        "title_key":"step.quadratic_formula", "params":{"a":"1","b":"0","c":"-2"},
        "before_latex":["x^2-2=0"],"after_latex":["x=\\sqrt{2}"],"children":children})
}

fn output() -> Value {
    json!({"items":[
        {"type":"expr","out_index":1,"input_form":"2","modern_form":"2","latex":"2"},
        {"type":"solutions","out_index":2,"input_form":"{{x->Sqrt[2]}}",
            "modern_form":"[[x -> sqrt(2)]]", "view":{"kind":"finite","vars":["x"],
            "solutions":[{"bindings":[{"var":"x","latex":"\\sqrt{2}",
                "input_form":"Sqrt[2]","modern_form":"sqrt(2)","numeric":"1.414213562"}],
                "condition_latex":null,"verified":{"Numeric":{"digits":10}}}],
            "region_latex":null,"intervals":[{"lo":null,"hi":"0","lo_closed":false,
                "hi_closed":true,"lo_value":null,"hi_value":0.0}]},
            "steps":{"root":[step("S1",vec![step("S1.1",vec![])])]},"plot":plot()},
        {"type":"plot","request":plot(),"data":data()},
        {"type":"error","message":"未闭合","span":{"start":0,"end":3}}],
        "messages":[{"symbol":"Solve","tag":"nsmet","text":"暂不支持","level":"Warning"}],
        "timing_ms":1.25})
}

#[test]
fn every_request_variant_matches_the_authoritative_wire_shape() {
    let fixtures = vec![
        json!({"type":"evaluate","cell_id":"a","source":"x^2=2","dialect":"Wolfram"}),
        json!({"type":"upsert_cell","cell":cell()}),
        json!({"type":"delete_cell","cell_id":"a"}),
        json!({"type":"move_cell","cell_id":"a","to_index":3}),
        json!({"type":"run_all"}),
        json!({"type":"preview","source":"x","dialect":"Auto","cursor":null}),
        json!({"type":"complete","source":"so","dialect":"Modern","cursor":2}),
        json!({"type":"hover","source":"Solve","dialect":"Wolfram","cursor":3}),
        json!({"type":"sample_plot","request":plot()}),
        json!({"type":"interrupt"}),
        json!({"type":"load_notebook","file":notebook()}),
        json!({"type":"save_notebook"}),
        json!({"type":"get_config"}),
        json!({"type":"set_config","config":config()}),
        json!({"type":"llm_translate","request_id":"r1","text":"解方程","cell_id":"a"}),
        json!({"type":"llm_explain","request_id":"r2","cell_id":"a","step_id":"S1.1"}),
        json!({"type":"llm_complete","request_id":"r3","prefix":"x+","suffix":"=2","dialect":"Modern"}),
        json!({"type":"llm_chat","request_id":"r4","messages":[
            {"role":"system","content":"CAS","tool_calls":[],"tool_call_id":null},
            {"role":"user","content":"x+1","tool_calls":[],"tool_call_id":null},
            {"role":"assistant","content":"","tool_calls":[{"id":"call_1","name":"evaluate","arguments":"{\"code\":\"x+1\"}"}],"tool_call_id":null},
            {"role":"tool","content":"x+1","tool_calls":[],"tool_call_id":"call_1"}]}),
        json!({"type":"llm_fix_error","request_id":"r5","cell_id":"a"}),
        json!({"type":"llm_test_profile","request_id":"r6","profile":"deepseek"}),
        json!({"type":"llm_cancel","request_id":"r7"}),
        json!({"type":"llm_http_chunk","request_id":"r8","chunk":"data: {\"text\":\"中\"}\r\n"}),
        json!({"type":"llm_http_end","request_id":"r9","status":200,"error":null}),
    ];
    assert_eq!(fixtures.len(), 23);
    for fixture in fixtures {
        roundtrip::<Request>(fixture);
    }
}

#[test]
fn every_response_and_event_roundtrips_with_nested_renderable_dtos() {
    let mut p = preview();
    p["type"] = json!("preview"); // newtype variant is flattened, without a body/result wrapper.
    let responses = vec![
        json!({"type":"ok"}),
        json!({"type":"error","message":"bad input"}),
        json!({"type":"evaluated","cell_id":"a","output":output(),"reran":["b","c"]}),
        p,
        json!({"type":"completions","items":[{"label":"solve","insert_text":"solve(${1:equation}, ${2:x})",
            "detail":"解方程","kind":"Snippet"}],"from":0,"to":2}),
        json!({"type":"hover","info":{"signature":"Solve[eqs,vars]","summary":"求解",
            "examples":["Solve[x^2==2,x]"],"value":null,"cell_id":null}}),
        json!({"type":"plot","data":data()}),
        json!({"type":"notebook","file":notebook()}),
        json!({"type":"config","config":config()}),
        json!({"type":"llm_started","request_id":"r","http":http()}),
    ];
    assert_eq!(responses.len(), 10);
    for fixture in responses {
        roundtrip::<Response>(fixture);
    }
    let events = vec![
        json!({"type":"cell_status","cell_id":"a","status":"Running"}),
        json!({"type":"cell_output","cell_id":"b","output":output()}),
        json!({"type":"llm_delta","request_id":"r","text":"答案 [S1]"}),
        json!({"type":"llm_tool_call","request_id":"r","name":"solve","arguments":"{}","result_summary":"2 roots"}),
        json!({"type":"llm_suggestion","request_id":"r","suggestion":{"wolfram":"Solve[x==1,x]",
            "modern":"solve(x=1,x)","latex":"x=1","explanation":"解方程"}}),
        json!({"type":"llm_http","request_id":"r","http":http()}),
        json!({"type":"llm_done","request_id":"r"}),
        json!({"type":"llm_error","request_id":"r","message":"401"}),
    ];
    assert_eq!(events.len(), 8);
    for fixture in events {
        roundtrip::<Envelope<Event>>(json!({"id":0,"body":fixture}));
    }
    roundtrip::<Envelope<Request>>(json!({"id":42,"body":{"type":"run_all"}}));
    roundtrip::<Envelope<Response>>(json!({"id":u64::MAX,"body":{"type":"ok"}}));
}

#[test]
fn nullable_fields_enum_spellings_and_required_fields_are_preserved() {
    roundtrip::<Response>(json!({"type":"hover","info":null}));
    roundtrip::<Response>(json!({"type":"llm_started","request_id":"r","http":null}));
    for status in ["Queued", "Running", "Done", "Error", "Stale"] {
        roundtrip::<Event>(json!({"type":"cell_status","cell_id":"a","status":status}));
    }
    for kind in ["Function", "Symbol", "Keyword", "Snippet"] {
        roundtrip::<CompletionItem>(
            json!({"label":"x","insert_text":"x","detail":null,"kind":kind}),
        );
    }
    for kind in ["finite", "all", "none", "region"] {
        roundtrip::<SolutionSetView>(
            json!({"kind":kind,"vars":[],"solutions":[],"region_latex":null,"intervals":[]}),
        );
    }
    for verified in [
        json!("Exact"),
        json!("ByConstruction"),
        json!({"Numeric":{"digits":200}}),
        json!("Unverified"),
    ] {
        roundtrip::<SolutionView>(
            json!({"bindings":[],"condition_latex":"a\\ne0","verified":verified}),
        );
    }
    let mut implicit = plot();
    implicit["kind"] = json!("Implicit");
    implicit["var_y"] = json!("y");
    implicit["y_range"] = json!([-2.0, 2.0]);
    roundtrip::<PlotRequest>(implicit);
    let omitted: Request =
        serde_json::from_value(json!({"type":"preview","source":"x","dialect":"Auto"})).unwrap();
    assert_eq!(wire_value(omitted)["cursor"], Value::Null);
    for invalid in [
        json!({"type":"evaluate","source":"1","dialect":"Modern"}),
        json!({"type":"runAll"}),
        json!({"type":"evaluate","cell_id":"a","source":"1","dialect":"modern"}),
        json!({"type":"move_cell","cell_id":"a","to_index":-1}),
    ] {
        assert!(serde_json::from_value::<Request>(invalid).is_err());
    }
    assert!(serde_json::from_value::<NotebookFile>(json!({"title":"x","cells":[]})).is_err());
    let mut sol = output()["items"][1].clone();
    sol["steps"] = Value::Null;
    sol["plot"] = Value::Null;
    roundtrip::<OutputItem>(sol);
    roundtrip::<OutputItem>(json!({"type":"error","message":"error","span":null}));
}

#[test]
fn actual_http_status_and_probe_metrics_are_additive_wire_fields() {
    roundtrip::<Request>(
        json!({"type":"llm_http_chunk","request_id":"actual","chunk":"data: []\n\n","status":403}),
    );
    roundtrip::<Event>(
        json!({"type":"llm_profile_test","request_id":"probe","profile":"editable","latency_ms":80.0,"first_byte_ms":30.0,"response":"actual reply"}),
    );
    roundtrip::<Event>(
        json!({"type":"llm_profile_test","request_id":"probe","profile":"editable","latency_ms":null,"first_byte_ms":null,"response":"actual reply"}),
    );
}

#[test]
fn actual_inspection_fields_keep_old_binding_fixtures_compatible() {
    roundtrip::<Request>(json!({"type":"inspect_expression","source":"2^(1/2)","numeric":true}));
    let value = json!({"input_form":"2^(1/2)","modern_form":"sqrt(2)","latex":"\\sqrt{2}"});
    roundtrip::<Response>(json!({"type":"expression","value":value.clone()}));
    roundtrip::<BindingView>(
        json!({"var":"x","input_form":"Root[#^2-2&,1]","modern_form":"Root(#^2-2,1)","latex":"Root_1","numeric":"-1.4142135623730950488","var_latex":"x","root_index":1,"radicals":value}),
    );
}

#[test]
fn explanation_output_selection_is_optional_without_changing_legacy_shape() {
    roundtrip::<Request>(
        json!({"type":"llm_explain","request_id":"all","cell_id":"multi","step_id":null}),
    );
    roundtrip::<Request>(
        json!({"type":"llm_explain","request_id":"selected","cell_id":"multi","step_id":"S1.2","out_index":2}),
    );
}

#[test]
fn draft_probe_and_explicit_keyless_profiles_extend_legacy_wire_shapes() {
    let mut draft = config()["llm"]["profiles"][0].clone();
    draft["requires_api_key"] = json!(false);
    roundtrip::<om_kernel::config::ProfileConfig>(draft.clone());
    roundtrip::<Request>(
        json!({"type":"llm_test_profile","request_id":"draft","profile":"deepseek","config":draft}),
    );
    roundtrip::<Request>(
        json!({"type":"llm_test_profile","request_id":"saved","profile":"deepseek"}),
    );
}

#[test]
fn actual_recovery_metadata_has_source_only_files_and_separate_variants() {
    roundtrip::<Request>(json!({"type":"get_notebook_state"}));
    roundtrip::<Request>(json!({"type":"restore_definitions"}));
    roundtrip::<Response>(
        json!({"type":"notebook_state","state":{"file":notebook(),"cells":[{"id":"a","status":"Stale","defines":["a"],"uses":["b"]}],"definition_order":["a"],"cycles":[]}}),
    );
    roundtrip::<Event>(json!({"type":"kernel_restarted","message":"actual restart notice"}));
}

#[test]
fn title_host_locale_and_live_variable_metadata_use_additive_real_wire_shapes() {
    roundtrip::<Request>(json!({"type":"rename_notebook","title":"My equations"}));
    roundtrip::<Request>(json!({"type":"set_system_language","language":"en"}));
    roundtrip::<Request>(json!({"type":"get_variables"}));
    roundtrip::<CellState>(
        json!({"id":"a","status":"Done","defines":["a"],"uses":[],"exec_count":1}),
    );
    roundtrip::<Response>(
        json!({"type":"variables","items":[["a",{"signature":null,"summary":"Stored definition (unevaluated)","examples":[],"value":"9","cell_id":"a"}]]}),
    );
}

#[test]
fn parser_and_solver_adapters_keep_existing_json_without_static_string_leaks() {
    let parsed = om_parse::parse("α  (x)", om_parse::Dialect::Modern);
    assert!(!parsed.diagnostics.is_empty());
    for diagnostic in &parsed.diagnostics {
        let adapted = Diagnostic::from(diagnostic);
        assert_eq!(wire_value(&adapted), wire_value(diagnostic));
        roundtrip::<Diagnostic>(wire_value(adapted));
    }
    for (span, class) in &parsed.tokens {
        assert_eq!(
            wire_value((Span::from(*span), TokenClass::from(*class))),
            wire_value((span, class))
        );
    }
    for dialect in [
        om_parse::Dialect::Modern,
        om_parse::Dialect::Wolfram,
        om_parse::Dialect::Auto,
    ] {
        assert_eq!(wire_value(Dialect::from(dialect)), wire_value(dialect));
        assert_eq!(om_parse::Dialect::from(Dialect::from(dialect)), dialect);
    }
    for verification in [
        om_solve::Verification::Exact,
        om_solve::Verification::ByConstruction,
        om_solve::Verification::Numeric { digits: 200 },
        om_solve::Verification::Unverified,
    ] {
        assert_eq!(
            wire_value(Verification::from(verification)),
            wire_value(verification)
        );
    }
    for level in [
        om_core::MsgLevel::Info,
        om_core::MsgLevel::Warning,
        om_core::MsgLevel::Error,
    ] {
        let original = om_core::Message {
            symbol: "Solve".into(),
            tag: "nsmet".into(),
            text: "消息".into(),
            level,
        };
        assert_eq!(wire_value(Message::from(&original)), wire_value(original));
    }
    let owned = json!({"span":{"start":0,"end":2},"severity":"Hint","code":"future-code",
        "message":"new code","fix":null});
    roundtrip::<Diagnostic>(owned);
}

#[test]
fn config_defaults_redaction_and_submitted_masks_preserve_secrets_by_profile_name() {
    let mut current = KernelConfig::default();
    current.llm.profiles[0].api_key = Some("test-secret-a".into());
    current.llm.profiles[1].api_key = Some("test-secret-b".into());
    let wire = wire_value(&current);
    assert_eq!(wire["llm"]["profiles"][0]["api_key"], "***");
    assert!(!format!("{current:?}").contains("test-secret"));
    assert!(!wire.to_string().contains("test-secret"));
    let mut submitted: KernelConfig = serde_json::from_value(wire).unwrap();
    submitted.llm.profiles.reverse();
    submitted.merge_redacted_keys(&current);
    assert_eq!(
        submitted.llm.profiles[0].api_key.as_deref(),
        Some("test-secret-b")
    );
    assert_eq!(
        submitted.llm.profiles[1].api_key.as_deref(),
        Some("test-secret-a")
    );
    submitted.llm.profiles[0].api_key = None;
    submitted.llm.profiles[1].api_key = Some("replacement".into());
    submitted.merge_redacted_keys(&current);
    assert_eq!(submitted.llm.profiles[0].api_key, None);
    assert_eq!(
        submitted.llm.profiles[1].api_key.as_deref(),
        Some("replacement")
    );
    submitted.llm.profiles[0].name = "new".into();
    submitted.llm.profiles[0].api_key = Some("***".into());
    submitted.merge_redacted_keys(&current);
    assert_eq!(submitted.llm.profiles[0].api_key, None);
    let defaults: KernelConfig = serde_json::from_value(json!({})).unwrap();
    let mut current_defaults = config();
    current_defaults["llm"]["profiles"][0]["model"] = json!("deepseek-flash");
    current_defaults["llm"]["profiles"][1]["model"] = json!("deepseek-flash");
    current_defaults["llm"]["profiles"][0]["extra_body"] = json!({"thinking":{"type":"disabled"}});
    assert_eq!(wire_value(defaults), current_defaults);
    assert_eq!(wire_value(KernelConfig::default()), current_defaults);
    let mut fixture = config();
    fixture["general"]["language"] = json!("zh-CN");
    fixture["general"]["dialect"] = json!("wolfram");
    fixture["general"]["constants"] = json!("strict");
    for kind in [
        "openai_chat",
        "anthropic",
        "openai_fim",
        "ollama_fim",
        "mistral_fim",
    ] {
        fixture["llm"]["profiles"][0]["kind"] = json!(kind);
        roundtrip::<KernelConfig>(fixture.clone());
    }
    let minimal: KernelConfig = serde_json::from_value(json!({"llm":{"profiles":[{
        "name":"fim","kind":"openai_fim","base_url":"https://example.invalid","model":"custom","max_tokens":64}]}})).unwrap();
    assert_eq!(minimal.llm.profiles[0].temperature, 0.2);
    assert_eq!(minimal.llm.profiles[0].timeout_ms, 60_000);
    assert!(!minimal.llm.profiles[0].supports_tools);
    assert!(minimal.llm.profiles[0].api_key.is_none());
    submitted.llm.profiles[0].api_key = Some(String::new());
    submitted.merge_redacted_keys(&current);
    assert_eq!(submitted.llm.profiles[0].api_key.as_deref(), Some(""));
    assert_eq!(
        wire_value(&submitted)["llm"]["profiles"][0]["api_key"],
        "***"
    );
    assert!(
        serde_json::from_value::<KernelConfig>(json!({"general":{"dialect":"Modern"}})).is_err()
    );
}

#[test]
fn notebook_persistence_contains_source_cells_and_never_configuration_or_keys() {
    let file = roundtrip::<NotebookFile>(notebook());
    assert_eq!(file.cells.len(), 3);
    let mut incoming = notebook();
    incoming["config"] = json!({"api_key":"test-secret"});
    incoming["cells"][0]["api_key"] = json!("test-secret");
    let restored: NotebookFile = serde_json::from_value(incoming).unwrap();
    assert_eq!(wire_value(restored), notebook());
}

#[test]
fn export_bindings() {
    let cfg = Config::new();
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src/kernel/generated");
    let mut previous = None;
    for _ in 0..2 {
        Request::export_all(&cfg).unwrap();
        Response::export_all(&cfg).unwrap();
        Event::export_all(&cfg).unwrap();
        Envelope::<Request>::export_all(&cfg).unwrap();
        let snapshot: std::collections::BTreeMap<_, _> = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let raw = std::fs::read_to_string(entry.path()).unwrap();
                let clean = raw
                    .lines()
                    .map(str::trim_end)
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n";
                if clean != raw {
                    std::fs::write(entry.path(), &clean).unwrap();
                }
                (entry.file_name(), clean)
            })
            .collect();
        assert!(
            snapshot
                .values()
                .all(|text| text.lines().all(|line| line.trim_end() == line)),
            "generated bindings must pass staged git diff --check"
        );
        if let Some(previous) = &previous {
            assert_eq!(previous, &snapshot);
        }
        previous = Some(snapshot);
    }
    for name in [
        "Request",
        "Response",
        "Event",
        "Envelope",
        "Diagnostic",
        "TokenClass",
        "Span",
        "StepsView",
        "StepView",
        "NotebookFile",
        "CellInput",
        "KernelConfig",
        "CliConfig",
        "ProfileConfig",
        "HttpRequest",
        "ChatMessage",
        "Role",
        "ToolCall",
        "Suggestion",
        "Verification",
        "Curve",
    ] {
        assert!(root.join(format!("{name}.ts")).is_file(), "missing {name}");
    }
    assert_eq!(
        Request::output_path()
            .unwrap()
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap(),
        root.canonicalize().unwrap()
    );
    let envelope = std::fs::read_to_string(root.join("Envelope.ts")).unwrap();
    assert!(envelope.contains("id: number"));
    assert!(!envelope.contains("bigint"));
    let steps = std::fs::read_to_string(root.join("StepView.ts")).unwrap();
    assert!(!steps.contains("Expr"));
    assert!(!steps.contains("StepKind"));
}

#[test]
fn cli_settings_default_to_disabled_and_roundtrip_only_when_enabled() {
    let mut config = KernelConfig::default();
    assert!(serde_json::to_value(&config).unwrap().get("cli").is_none());
    config.cli.ai_hints = true;
    let wire = serde_json::to_value(&config).unwrap();
    assert_eq!(wire["cli"], json!({"ai_hints":true}));
    assert!(
        serde_json::from_value::<KernelConfig>(wire)
            .unwrap()
            .cli
            .ai_hints
    );
    assert!(
        toml::from_str::<KernelConfig>("[cli]\nai_hints = true")
            .unwrap()
            .cli
            .ai_hints
    );
}
