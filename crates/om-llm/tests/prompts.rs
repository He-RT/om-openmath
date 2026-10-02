//! Exact PLAN examples, single-pass substitutions and actual feature/schema request routing.
use om_llm::{prompts::*, *};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[test]
fn embedded_translate_keeps_all_eight_plan_examples_and_bilingual_rules() {
    for lang in ["English", "Simplified Chinese"] {
        let messages = translate_messages(
            lang,
            "Solve, Reduce, N, NSolve, Factor, Sin",
            "α, x",
            "求解 {{function_list}}",
        );
        let system = &messages[0].content;
        assert!(system.contains(lang));
        assert!(
            system.contains("Use only these functions: Solve, Reduce, N, NSolve, Factor, Sin.")
        );
        assert!(system.contains("Symbols already defined in the notebook: α, x."));
        for expression in [
            "Solve[x^2 + 2*x == 3, x]",
            "Solve[x^3 - 2*x + 1 == 0, x, Reals]",
            "Solve[{x + y == 10, x - y == 2}, {x, y}]",
            "Solve[Sin[x] == 1/2 && 0 <= x <= 2*Pi, x]",
            "Factor[x^4 - 1]",
            "Reduce[x^2 < 4, x, Reals]",
            "NSolve[x^5 - x + 1 == 0, x]",
            "Solve[{x^2 + y^2 == 25, y == x + 1}, {x, y}]",
        ] {
            assert!(system.contains(expression), "{expression}");
        }
        assert!(!system.contains("{{"));
        assert_eq!(messages[1].content, "求解 {{function_list}}");
    }
    assert!(
        translate_messages("English", "{{lang}}", "x", "task")[0]
            .content
            .contains("functions: {{lang}}.")
    );
}
#[test]
fn supplied_data_is_json_encoded_and_never_expands_instruction_templates() {
    let steps = json!({"root":[{"id":"S1","children":[]}]});
    let messages = explain_messages("English", "x/x", "1", &steps);
    let user: Value = serde_json::from_str(&messages[1].content).unwrap();
    assert_eq!(user, json!({"input":"x/x","result":"1","steps":steps}));
    assert!(messages[0].content.contains("[S{n}]"));
    assert!(messages[0].content.contains("$…$"));
    let messages = fix_messages(
        "Simplified Chinese",
        "\"{{lang}}\"",
        "modern",
        &json!([{"code":"E023"}]),
        &json!([{"text":"真实消息"}]),
    );
    let user: Value = serde_json::from_str(&messages[1].content).unwrap();
    assert_eq!(user["source"], "\"{{lang}}\"");
    assert_eq!(user["messages"][0]["text"], "真实消息");
    assert!(messages[0].content.contains("Simplified Chinese"));
}
#[test]
fn chat_tools_have_actual_plan_schemas_and_roundtrip_into_checked_request() {
    let history = vec![ChatMessage {
        role: Role::User,
        content: "solve x+1=2".into(),
        tool_calls: vec![],
        tool_call_id: None,
    }];
    let messages = chat_messages("English", &history);
    assert!(messages[0].content.contains("MUST call a CAS tool"));
    assert!(
        messages[0]
            .content
            .contains("MUST cite the actual tool results")
    );
    assert_eq!(messages[1].content, history[0].content);
    let tools = chat_tools();
    assert_eq!(
        tools.iter().map(|t| t.name).collect::<Vec<_>>(),
        ["evaluate", "solve", "propose_cell"]
    );
    assert_eq!(
        tools[1].parameters["properties"]["domain"]["enum"],
        json!(["Complexes", "Reals", "Integers"])
    );
    assert_eq!(
        tools[2].parameters["properties"]["dialect"]["enum"],
        json!(["modern", "wolfram"])
    );
    let profile = Profile {
        name: "mock".into(),
        kind: ProviderKind::OpenaiChat,
        base_url: "https://fixture.invalid/v1".into(),
        model: "editable".into(),
        api_key: None,
        temperature: 0.2,
        max_tokens: 128,
        supports_tools: true,
        supports_json_mode: false,
        timeout_ms: 5000,
        extra_headers: BTreeMap::new(),
        extra_body: BTreeMap::new(),
    };
    let (mut job, step) = Job::new(
        Feature::Chat,
        profile,
        JobInput::Chat { messages, tools },
        Target::Browser,
    );
    let JobStep::Http(request) = step else {
        panic!("{step:?}")
    };
    let body: Value = serde_json::from_str(&request.body).unwrap();
    assert_eq!(
        body["tools"][1]["function"]["parameters"]["properties"]["domain"]["default"],
        "Complexes"
    );
    job.on_bytes("data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call-real\",\"function\":{\"name\":\"solve\",\"arguments\":\"{\\\"equations\\\":[\\\"x+1==2\\\"],\\\"variables\\\":[\\\"x\\\"]}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n\ndata: [DONE]\n\n");
    let JobStep::RunTools(calls) = job.on_http_end(200, None) else {
        panic!()
    };
    assert_eq!(calls[0].name, "solve");
    let JobStep::Http(request) = job.tool_results(vec![(
        calls[0].id.clone(),
        "{\"solutions\":\"{{x -> 1}}\",\"steps_summary\":[\"step.ApplyFormula\"]}".into(),
    )]) else {
        panic!()
    };
    assert!(request.body.contains("step.ApplyFormula"));
}
