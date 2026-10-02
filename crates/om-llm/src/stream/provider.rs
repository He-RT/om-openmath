//! Provider schemas become actual deltas; malformed chunks cannot masquerade as progress.
use super::{StreamEvent as E, lines::LIMIT};
use crate::LlmError;
use serde_json::Value;
fn json(data: &str) -> Result<Value, LlmError> {
    if data.len() > LIMIT {
        return Err(LlmError::Limit { limit: LIMIT });
    }
    let v: Value = serde_json::from_str(data).map_err(|_| LlmError::Json)?;
    if !v.is_object() {
        return Err(LlmError::Json);
    }
    Ok(v)
}
fn string(v: Option<&Value>) -> Result<Option<String>, LlmError> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        _ => Err(LlmError::Json),
    }
}
fn index(v: &Value) -> Result<u32, LlmError> {
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or(LlmError::Json)
}
fn error(v: &Value) -> Option<E> {
    v.get("error").filter(|v| !v.is_null()).map(|e| {
        E::Error(
            e.get("message")
                .and_then(Value::as_str)
                .unwrap_or("provider error")
                .into(),
        )
    })
}
fn decoded(result: Result<Vec<E>, LlmError>) -> Vec<E> {
    result.unwrap_or_else(|e| vec![E::Error(e.to_string())])
}
/// Decode actual choice-zero OpenAI deltas; additional choices and reasoning fields are ignored.
pub fn decode_openai_chunk(data: &str) -> Vec<E> {
    if data.trim() == "[DONE]" {
        return vec![E::Finish {
            reason: "done".into(),
        }];
    }
    if data.trim().is_empty() {
        return vec![];
    }
    decoded(openai(data))
}
fn openai(data: &str) -> Result<Vec<E>, LlmError> {
    let value = json(data)?;
    if let Some(e) = error(&value) {
        return Ok(vec![e]);
    }
    let choices = value
        .get("choices")
        .and_then(Value::as_array)
        .ok_or(LlmError::Json)?;
    let mut chosen = None;
    for choice in choices {
        if !choice.is_object() {
            return Err(LlmError::Json);
        }
        let i = match choice.get("index") {
            None if choices.len() == 1 => 0,
            Some(v) => index(v)?,
            _ => return Err(LlmError::Json),
        };
        if i == 0 {
            if chosen.is_some() {
                return Err(LlmError::Json);
            }
            chosen = Some(choice);
        }
    }
    let Some(choice) = chosen else {
        return Ok(vec![]);
    };
    let mut events = vec![];
    if let Some(delta) = choice.get("delta").filter(|v| !v.is_null()) {
        if !delta.is_object() {
            return Err(LlmError::Json);
        }
        if let Some(text) = string(delta.get("content"))? {
            events.push(E::Text(text));
        }
        if let Some(calls) = delta.get("tool_calls").filter(|v| !v.is_null()) {
            for call in calls.as_array().ok_or(LlmError::Json)? {
                let i = index(call.get("index").ok_or(LlmError::Json)?)?;
                if call.get("type").is_some_and(|v| v != "function") {
                    return Err(LlmError::Json);
                }
                let function = call.get("function").filter(|v| !v.is_null());
                if function.is_some_and(|v| !v.is_object()) {
                    return Err(LlmError::Json);
                }
                events.push(E::ToolCallDelta {
                    index: i,
                    id: string(call.get("id"))?,
                    name: string(function.and_then(|v| v.get("name")))?,
                    args_fragment: string(function.and_then(|v| v.get("arguments")))?
                        .unwrap_or_default(),
                });
            }
        }
    }
    if let Some(reason) = string(choice.get("finish_reason"))? {
        events.push(E::Finish { reason });
    }
    Ok(events)
}
/// Decode actual Anthropic named blocks/deltas, ignoring unknown and thinking metadata.
pub fn decode_anthropic_event(event: &str, data: &str) -> Vec<E> {
    if !matches!(
        event,
        "message"
            | ""
            | "message_start"
            | "content_block_start"
            | "content_block_delta"
            | "content_block_stop"
            | "message_delta"
            | "message_stop"
            | "error"
    ) {
        return vec![];
    }
    decoded(anthropic(event, data))
}
fn anthropic(event: &str, data: &str) -> Result<Vec<E>, LlmError> {
    let value = json(data)?;
    let event = if event.is_empty() || event == "message" {
        value
            .get("type")
            .and_then(Value::as_str)
            .ok_or(LlmError::Json)?
    } else {
        event
    };
    if value.get("type").is_some_and(|v| v != event) {
        return Err(LlmError::Json);
    }
    if event == "error" {
        return Ok(vec![
            error(&value).unwrap_or_else(|| E::Error("provider error".into())),
        ]);
    }
    Ok(match event {
        "content_block_start" => {
            let i = index(value.get("index").ok_or(LlmError::Json)?)?;
            let block = value.get("content_block").ok_or(LlmError::Json)?;
            if !block.is_object() {
                return Err(LlmError::Json);
            }
            match block.get("type").and_then(Value::as_str) {
                Some("text") => string(block.get("text"))?
                    .filter(|s| !s.is_empty())
                    .map(E::Text)
                    .into_iter()
                    .collect(),
                Some("tool_use") => {
                    let input = block.get("input").ok_or(LlmError::Json)?;
                    if !input.is_object() {
                        return Err(LlmError::Json);
                    }
                    let id = string(block.get("id"))?
                        .filter(|s| !s.is_empty())
                        .ok_or(LlmError::Json)?;
                    let name = string(block.get("name"))?
                        .filter(|s| !s.is_empty())
                        .ok_or(LlmError::Json)?;
                    let args_fragment = if input.as_object().is_some_and(|m| !m.is_empty()) {
                        serde_json::to_string(input).map_err(|_| LlmError::Json)?
                    } else {
                        String::new()
                    };
                    vec![E::ToolCallDelta {
                        index: i,
                        id: Some(id),
                        name: Some(name),
                        args_fragment,
                    }]
                }
                _ => vec![],
            }
        }
        "content_block_delta" => {
            let i = index(value.get("index").ok_or(LlmError::Json)?)?;
            let delta = value.get("delta").ok_or(LlmError::Json)?;
            match delta.get("type").and_then(Value::as_str) {
                Some("text_delta") => {
                    vec![E::Text(string(delta.get("text"))?.ok_or(LlmError::Json)?)]
                }
                Some("input_json_delta") => vec![E::ToolCallDelta {
                    index: i,
                    id: None,
                    name: None,
                    args_fragment: string(delta.get("partial_json"))?.ok_or(LlmError::Json)?,
                }],
                _ => vec![],
            }
        }
        "message_delta" => string(value.get("delta").and_then(|v| v.get("stop_reason")))?
            .map(|reason| E::Finish { reason })
            .into_iter()
            .collect(),
        "message_stop" => vec![E::Finish {
            reason: "done".into(),
        }],
        _ => vec![],
    })
}
