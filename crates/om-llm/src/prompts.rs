//! Embedded feature instructions; untrusted task data is kept in separate user turns.
use crate::{ChatMessage, Role, ToolSpec};
use serde_json::{Value, json};

fn message(role: Role, content: String) -> ChatMessage {
    ChatMessage {
        role,
        content,
        tool_calls: vec![],
        tool_call_id: None,
    }
}
fn pair(system: String, user: String) -> Vec<ChatMessage> {
    vec![message(Role::System, system), message(Role::User, user)]
}
fn template(source: &str, variables: &[(&str, &str)]) -> String {
    // Scan only template bytes: substituted names/data cannot expand other placeholders.
    let mut result = String::new();
    let mut remaining = source;
    while let Some(start) = remaining.find("{{") {
        result.push_str(&remaining[..start]);
        let tail = &remaining[start + 2..];
        let Some(end) = tail.find("}}") else {
            result.push_str(&remaining[start..]);
            return result;
        };
        let key = &tail[..end];
        if let Some((_, value)) = variables.iter().find(|(name, _)| *name == key) {
            result.push_str(value);
        } else {
            result.push_str(&remaining[start..start + end + 4]);
        }
        remaining = &tail[end + 2..];
    }
    result.push_str(remaining);
    result
}
/// PLAN translation examples and the host's actual implemented function/name lists.
pub fn translate_messages(
    lang: &str,
    functions: &str,
    defined: &str,
    text: &str,
) -> Vec<ChatMessage> {
    pair(
        template(
            include_str!("../prompts/translate.md"),
            &[
                ("lang", lang),
                ("function_list", functions),
                ("defined_symbols", defined),
            ],
        ),
        text.into(),
    )
}
/// Explain supplied actual input/result and the host-selected real StepsView JSON.
pub fn explain_messages(lang: &str, input: &str, result: &str, steps: &Value) -> Vec<ChatMessage> {
    pair(
        template(include_str!("../prompts/explain.md"), &[("lang", lang)]),
        json!({"input":input,"result":result,"steps":steps}).to_string(),
    )
}
/// Correct source using its original syntax, real parser diagnostics and CAS messages.
pub fn fix_messages(
    lang: &str,
    source: &str,
    dialect: &str,
    diagnostics: &Value,
    messages: &Value,
) -> Vec<ChatMessage> {
    pair(
        template(include_str!("../prompts/fix.md"), &[("lang", lang)]),
        json!({"source":source,"dialect":dialect,"diagnostics":diagnostics,"messages":messages})
            .to_string(),
    )
}
/// Prepend the mandatory mathematical tool/reference policy to actual conversation turns.
pub fn chat_messages(lang: &str, history: &[ChatMessage]) -> Vec<ChatMessage> {
    let mut messages = vec![message(
        Role::System,
        template(include_str!("../prompts/chat.md"), &[("lang", lang)]),
    )];
    messages.extend_from_slice(history);
    messages
}
/// The three PLAN host tools, with explicit object schemas and domain/dialect enums.
pub fn chat_tools() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "evaluate",
            description: "Evaluate Wolfram source using the readonly CAS (5 second limit).",
            parameters: json!({"type":"object","properties":{"code":{"type":"string"}},"required":["code"],"additionalProperties":false}),
        },
        ToolSpec {
            name: "solve",
            description: "Solve using the readonly CAS; return actual solutions and recorded step titles (5 second limit).",
            parameters: json!({"type":"object","properties":{"equations":{"type":"array","items":{"type":"string"},"minItems":1},"variables":{"type":"array","items":{"type":"string"},"minItems":1},"domain":{"type":"string","enum":["Complexes","Reals","Integers"],"default":"Complexes"}},"required":["equations","variables"],"additionalProperties":false}),
        },
        ToolSpec {
            name: "propose_cell",
            description: "Propose source without executing it; the user chooses whether to insert or run it.",
            parameters: json!({"type":"object","properties":{"code":{"type":"string"},"dialect":{"type":"string","enum":["modern","wolfram"]}},"required":["code","dialect"],"additionalProperties":false}),
        },
    ]
}
