//! Actual FIM requests and raw completion responses, without source insertion or IO.
use crate::{
    ChatMessage, HttpRequest, LlmError, Profile, ProviderKind, Role, Target, try_build_chat_request,
};
use serde_json::{Value, json};
const LIMIT: usize = 1_048_576;
fn message(role: Role, content: String) -> ChatMessage {
    ChatMessage {
        role,
        content,
        tool_calls: vec![],
        tool_call_id: None,
    }
}
/// Build the prescribed Native FIM request for validated inputs.
/// Runtime/browser paths must use [`try_build_fim_request`] for checked errors and Target.
pub fn build_fim_request(p: &Profile, prefix: &str, suffix: &str) -> HttpRequest {
    match try_build_fim_request(p, prefix, suffix, Target::Native) {
        Ok(request) => request,
        Err(error) => panic!("invalid FIM request: {error}"),
    }
}
/// Checked provider-specific nonstreaming completion, including real chat fallbacks.
pub fn try_build_fim_request(
    p: &Profile,
    prefix: &str,
    suffix: &str,
    target: Target,
) -> Result<HttpRequest, LlmError> {
    let mut effective = p.clone();
    effective.temperature = 0.0;
    crate::chat::validate_profile(&effective)?;
    if matches!(p.kind, ProviderKind::OpenaiChat | ProviderKind::Anthropic) {
        let messages = [
            message(
                Role::System,
                "Output only the text to insert at the cursor. Do not explain or add code fences."
                    .into(),
            ),
            message(Role::User, format!("{prefix}⟨CURSOR⟩{suffix}")),
        ];
        let mut request = try_build_chat_request(&effective, &messages, &[], false, target)?;
        let mut body: Value = serde_json::from_str(&request.body).map_err(|_| LlmError::Json)?;
        body["stream"] = false.into();
        body[if p.kind == ProviderKind::Anthropic {
            "stop_sequences"
        } else {
            "stop"
        }] = json!(["\n"]);
        request.body = serde_json::to_string(&body).map_err(|_| LlmError::Json)?;
        request.stream = false;
        return Ok(request);
    }
    let (endpoint, body) = match p.kind {
        ProviderKind::OpenaiFim => (
            "/completions",
            json!({"model":p.model,"prompt":prefix,"suffix":suffix,"max_tokens":p.max_tokens,"temperature":0,"stop":["\n"],"stream":false}),
        ),
        ProviderKind::OllamaFim => (
            "/api/generate",
            json!({"model":p.model,"prompt":prefix,"suffix":suffix,"stream":false,"options":{"temperature":0,"num_predict":p.max_tokens,"stop":["\n"]}}),
        ),
        ProviderKind::MistralFim => (
            "/v1/fim/completions",
            json!({"model":p.model,"prompt":prefix,"suffix":suffix,"max_tokens":p.max_tokens,"temperature":0,"stop":["\n"]}),
        ),
        _ => return Err(LlmError::Provider),
    };
    Ok(HttpRequest {
        method: "POST".into(),
        url: format!("{}{endpoint}", p.base_url.trim_end_matches('/')),
        headers: crate::chat::build_headers(p, target)?,
        body: serde_json::to_string(&body).map_err(|_| LlmError::Json)?,
        stream: false,
    })
}
fn text(value: Option<&Value>) -> Result<String, LlmError> {
    value
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(LlmError::Json)
}
fn remote(value: &Value) -> Option<LlmError> {
    value.get("error").filter(|e| !e.is_null()).map(|e| {
        LlmError::Remote(
            e.as_str()
                .or_else(|| e.get("message").and_then(Value::as_str))
                .unwrap_or("provider error")
                .into(),
        )
    })
}
fn choice(value: &Value) -> Result<&Value, LlmError> {
    let choices = value
        .get("choices")
        .and_then(Value::as_array)
        .ok_or(LlmError::Json)?;
    let mut chosen = None;
    for c in choices {
        if !c.is_object() {
            return Err(LlmError::Json);
        }
        let i = match c.get("index") {
            None if choices.len() == 1 => 0,
            Some(i) => i.as_u64().ok_or(LlmError::Json)?,
            _ => return Err(LlmError::Json),
        };
        if i == 0 {
            if chosen.is_some() {
                return Err(LlmError::Json);
            }
            chosen = Some(c);
        }
    }
    chosen.ok_or(LlmError::Json)
}
fn assistant(value: &Value) -> Result<String, LlmError> {
    let selected = choice(value)?;
    if selected
        .get("finish_reason")
        .and_then(Value::as_str)
        .is_some_and(|r| matches!(r, "tool_calls" | "function_call"))
    {
        return Err(LlmError::Message("completion response requires tools"));
    }
    let message = selected
        .get("message")
        .filter(|m| m.is_object())
        .ok_or(LlmError::Json)?;
    if message.get("role").is_some_and(|r| r != "assistant") {
        return Err(LlmError::Message(
            "completion response has a nonassistant role",
        ));
    }
    if message
        .get("tool_calls")
        .filter(|v| !v.is_null())
        .is_some_and(|v| v.as_array().is_none_or(|c| !c.is_empty()))
    {
        return Err(LlmError::Message("completion response contains tool calls"));
    }
    text(message.get("content"))
}
/// Parse exact raw completion text; trimming, first-line/CAS checks and insertion are later stages.
pub fn parse_fim_response(kind: ProviderKind, body: &str) -> Result<String, LlmError> {
    if body.len() > LIMIT {
        return Err(LlmError::Limit { limit: LIMIT });
    }
    let value: Value = serde_json::from_str(body).map_err(|_| LlmError::Json)?;
    if !value.is_object() {
        return Err(LlmError::Json);
    }
    if let Some(error) = remote(&value) {
        return Err(error);
    }
    match kind {
        ProviderKind::OpenaiFim => text(choice(&value)?.get("text")),
        ProviderKind::OllamaFim => {
            if value.get("done").is_some_and(|done| done != true) {
                return Err(LlmError::Message("incomplete completion response"));
            }
            text(value.get("response"))
        }
        ProviderKind::MistralFim | ProviderKind::OpenaiChat => assistant(&value),
        ProviderKind::Anthropic => {
            if value.get("role").is_some_and(|role| role != "assistant") {
                return Err(LlmError::Message(
                    "completion response has a nonassistant role",
                ));
            }
            if value.get("stop_reason").and_then(Value::as_str) == Some("tool_use") {
                return Err(LlmError::Message("completion response requires tools"));
            }
            let blocks = value
                .get("content")
                .and_then(Value::as_array)
                .ok_or(LlmError::Json)?;
            let mut result = String::new();
            for block in blocks {
                if matches!(
                    block.get("type").and_then(Value::as_str),
                    Some("thinking" | "redacted_thinking")
                ) {
                    continue;
                }
                if block.get("type").and_then(Value::as_str) != Some("text") {
                    return Err(LlmError::Message(
                        "completion response contains nontext blocks",
                    ));
                }
                result.push_str(&text(block.get("text"))?);
            }
            Ok(result)
        }
    }
}
