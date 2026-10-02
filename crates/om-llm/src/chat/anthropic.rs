//! Messages blocks preserve actual input objects and tool-result associations.
use super::{LlmError, Profile, ToolSpec};
use crate::{ChatMessage, Role};
use serde::Serialize;
use serde_json::{Value, json};
#[derive(Serialize)]
struct Body<'a> {
    model: &'a str,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
    messages: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
}
pub(super) fn body(
    p: &Profile,
    msgs: &[ChatMessage],
    tools: &[ToolSpec],
) -> Result<String, LlmError> {
    let system: Vec<_> = msgs
        .iter()
        .filter(|m| m.role == Role::System)
        .map(|m| m.content.as_str())
        .collect();
    let mut messages: Vec<Value> = vec![];
    for msg in msgs.iter().filter(|m| m.role != Role::System) {
        let role = if msg.role == Role::Assistant {
            "assistant"
        } else {
            "user"
        };
        let mut blocks = vec![];
        if msg.role == Role::Tool {
            blocks.push(
                json!({"type":"tool_result","tool_use_id":msg.tool_call_id,"content":msg.content}),
            );
        } else {
            if !msg.content.is_empty() {
                blocks.push(json!({"type":"text","text":msg.content}));
            }
            for call in &msg.tool_calls {
                let input: Value =
                    serde_json::from_str(&call.arguments).map_err(|_| LlmError::Json)?;
                if !input.is_object() {
                    return Err(LlmError::Message("tool input must be an object"));
                }
                blocks.push(json!({"type":"tool_use","id":call.id,"name":call.name,"input":input}));
            }
        }
        if blocks.is_empty() {
            return Err(LlmError::Message("empty Anthropic content"));
        }
        if let Some(previous) = messages.last_mut().filter(|m| m["role"] == role) {
            previous["content"]
                .as_array_mut()
                .ok_or(LlmError::Json)?
                .extend(blocks);
        } else {
            messages.push(json!({"role":role,"content":blocks}));
        }
    }
    let tools = (!tools.is_empty() && p.supports_tools).then(|| {
        tools
            .iter()
            .map(|t| json!({"name":t.name,"description":t.description,"input_schema":t.parameters}))
            .collect()
    });
    serde_json::to_string(&Body {
        model: &p.model,
        max_tokens: p.max_tokens,
        temperature: p.temperature,
        stream: true,
        messages,
        system: (!system.is_empty()).then(|| system.join("\n")),
        tools,
    })
    .map_err(|_| LlmError::Json)
}
