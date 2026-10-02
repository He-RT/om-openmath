//! Actual configured chat request construction; no network or substitute execution.
mod anthropic;
mod headers;
use crate::{ChatMessage, HttpRequest, ProviderKind, Role};
pub(crate) use headers::build as build_headers;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Runtime provider profile from PLAN §11.1; keys are never Debug-printed.
#[derive(Clone)]
pub struct Profile {
    /// Configured routing name.
    pub name: String,
    /// Provider protocol.
    pub kind: ProviderKind,
    /// Configured endpoint prefix.
    pub base_url: String,
    /// Editable model identifier.
    pub model: String,
    /// Actual runtime credential, not a frontend mask.
    pub api_key: Option<String>,
    /// Configured sampling temperature.
    pub temperature: f32,
    /// Requested output token budget.
    pub max_tokens: u32,
    /// Declared tools support.
    pub supports_tools: bool,
    /// Declared OpenAI JSON-object mode support.
    pub supports_json_mode: bool,
    /// Transport deadline, used by the later IO driver.
    pub timeout_ms: u64,
    /// Configured additional headers.
    pub extra_headers: BTreeMap<String, String>,
    /// Explicit bounded vendor body fields that cannot override protocol-owned fields.
    pub extra_body: BTreeMap<String, Value>,
}
impl std::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Profile")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("model", &self.model)
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("supports_tools", &self.supports_tools)
            .field("supports_json_mode", &self.supports_json_mode)
            .field("timeout_ms", &self.timeout_ms)
            .field(
                "extra_headers",
                &self.extra_headers.keys().collect::<Vec<_>>(),
            )
            .field("extra_body", &self.extra_body.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}
/// Host target; browser Anthropic requires its explicit direct-access header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// Native host transport.
    Native,
    /// Browser fetch transport.
    Browser,
}
/// Actual function tool schema supplied by the kernel, not interpreted here.
pub struct ToolSpec {
    /// Stable tool name.
    pub name: &'static str,
    /// Tool description.
    pub description: &'static str,
    /// Real JSON Schema object.
    pub parameters: Value,
}
/// Checked sans-IO construction/decoding errors never include input payloads or keys.
#[derive(Clone, PartialEq, Eq, thiserror::Error)]
pub enum LlmError {
    /// Feature/input variants do not match or required host state is missing.
    #[error("invalid Job feature/input pairing")]
    Input,
    /// An active-phase method was called out of sequence.
    #[error("invalid Job state: {0}")]
    State(&'static str),
    /// No actual complete response/finish was received.
    #[error("incomplete model response")]
    Incomplete,
    /// Tool metadata/results are invalid or exceed the active-tool bound.
    #[error("invalid or excessive tool calls/results")]
    Tools,
    /// The seventh requested tool round exceeds the specified six-round limit.
    #[error("tool round limit exceeded")]
    ToolRounds,
    /// Actual host verification failed after bounded retries.
    #[error("suggestion validation failed: {0}")]
    Validation(String),
    /// Actual non-success HTTP result, already sanitized by the Job profile.
    #[error("HTTP {status}: {message}")]
    Http {
        /// Actual provider status.
        status: u16,
        /// Sanitized actual provider message.
        message: String,
    },
    /// Actual transport failure, already sanitized by the Job profile.
    #[error("transport failed: {0}")]
    Transport(String),
    /// Explicit cancellation is terminal.
    #[error("LLM request cancelled")]
    Cancelled,
    /// Chat construction received a non-chat provider.
    #[error("profile is not a chat provider")]
    Provider,
    /// A required profile field is invalid.
    #[error("invalid profile field: {0}")]
    Profile(&'static str),
    /// Invalid role or tool association.
    #[error("invalid chat message: {0}")]
    Message(&'static str),
    /// Invalid function schema.
    #[error("invalid tool schema")]
    Tool,
    /// Header syntax is invalid.
    #[error("invalid HTTP header syntax")]
    Header,
    /// A provider/frame payload is invalid JSON.
    #[error("invalid provider JSON")]
    Json,
    /// A stream has invalid or truncated UTF-8.
    #[error("invalid or truncated UTF-8 stream")]
    Encoding,
    /// Buffered line/event exceeded its configured byte limit.
    #[error("stream frame limit exceeded ({limit} bytes)")]
    Limit {
        /// Configured bound.
        limit: usize,
    },
    /// Actual remote message, accessible explicitly but excluded from error formatting.
    #[error("provider reported an error")]
    Remote(String),
    /// A failed decoder cannot resume without a new instance.
    #[error("stream decoder is terminal")]
    Terminal,
}
fn validate_base(base: &str) -> Result<(), LlmError> {
    let base = base.trim_end_matches('/');
    let (scheme, rest) = base
        .split_once("://")
        .ok_or(LlmError::Profile("base_url"))?;
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        || base
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '?' | '#' | '@' | '\\'))
    {
        return Err(LlmError::Profile("base_url"));
    }
    let authority = rest
        .split('/')
        .next()
        .ok_or(LlmError::Profile("base_url"))?;
    let valid_port = |p: &str| {
        !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) && p.parse::<u16>().is_ok()
    };
    if let Some(v6) = authority.strip_prefix('[') {
        let (address, tail) = v6.split_once(']').ok_or(LlmError::Profile("base_url"))?;
        if address.parse::<std::net::Ipv6Addr>().is_err()
            || !tail.is_empty() && !tail.strip_prefix(':').is_some_and(valid_port)
        {
            return Err(LlmError::Profile("base_url"));
        }
    } else {
        let (host, port) = authority
            .rsplit_once(':')
            .map_or((authority, None), |(h, p)| (h, Some(p)));
        if host.is_empty()
            || host
                .chars()
                .any(|c| !(c.is_alphanumeric() || matches!(c, '.' | '-' | '_')))
            || port.is_some_and(|p| !valid_port(p))
        {
            return Err(LlmError::Profile("base_url"));
        }
    }
    Ok(())
}
pub(crate) fn validate_profile(p: &Profile) -> Result<(), LlmError> {
    if p.name.trim().is_empty() {
        return Err(LlmError::Profile("name"));
    }
    if p.model.trim().is_empty() {
        return Err(LlmError::Profile("model"));
    }
    if p.max_tokens == 0 {
        return Err(LlmError::Profile("max_tokens"));
    }
    let max = if p.kind == ProviderKind::Anthropic {
        1.0
    } else {
        2.0
    };
    if !p.temperature.is_finite() || p.temperature < 0.0 || p.temperature > max {
        return Err(LlmError::Profile("temperature"));
    }
    validate_base(&p.base_url)?;
    if p.extra_body.keys().any(|name| {
        matches!(
            name.as_str(),
            "model"
                | "messages"
                | "tools"
                | "stream"
                | "temperature"
                | "max_tokens"
                | "prompt"
                | "suffix"
                | "stop"
                | "stop_sequences"
                | "response_format"
                | "system"
                | "options"
                | "headers"
                | "base_url"
                | "api_key"
        )
    }) {
        return Err(LlmError::Profile("extra_body"));
    }
    if serde_json::to_vec(&p.extra_body)
        .map_err(|_| LlmError::Json)?
        .len()
        > 1_048_576
    {
        return Err(LlmError::Limit { limit: 1_048_576 });
    }
    Ok(())
}
impl std::fmt::Debug for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if matches!(self, Self::Remote(_)) {
            f.write_str("Remote(<provider error>)")
        } else {
            std::fmt::Display::fmt(self, f)
        }
    }
}
impl LlmError {
    /// Untrusted provider message for the later profile-aware error sanitizer.
    pub fn remote_message(&self) -> Option<&str> {
        if let Self::Remote(message) = self {
            Some(message)
        } else {
            None
        }
    }
}
fn validate(p: &Profile, msgs: &[ChatMessage], tools: &[ToolSpec]) -> Result<(), LlmError> {
    if !matches!(p.kind, ProviderKind::OpenaiChat | ProviderKind::Anthropic) {
        return Err(LlmError::Provider);
    }
    validate_profile(p)?;
    if msgs.is_empty() || msgs.iter().all(|m| m.role == Role::System) {
        return Err(LlmError::Message("missing conversation"));
    }
    let mut pending = BTreeSet::new();
    for m in msgs {
        if m.role != Role::Tool && !pending.is_empty() {
            return Err(LlmError::Message("unfinished tool results"));
        }
        if m.role != Role::Assistant && !m.tool_calls.is_empty() {
            return Err(LlmError::Message("tool calls require assistant role"));
        }
        if m.role == Role::Tool {
            if m.tool_call_id.as_ref().is_none_or(|id| id.is_empty()) {
                return Err(LlmError::Message("missing tool call id"));
            }
            if !m.tool_call_id.as_ref().is_some_and(|id| pending.remove(id)) {
                return Err(LlmError::Message("unknown or duplicate tool result"));
            }
        } else if m.tool_call_id.is_some() {
            return Err(LlmError::Message("unexpected tool call id"));
        }
        if !m.tool_calls.is_empty() && !p.supports_tools {
            return Err(LlmError::Message("profile does not support tool history"));
        }
        for call in &m.tool_calls {
            if call.id.is_empty() || call.name.is_empty() {
                return Err(LlmError::Message("missing tool id/name"));
            }
            if !pending.insert(call.id.clone()) {
                return Err(LlmError::Message("duplicate tool call id"));
            }
        }
    }
    if !pending.is_empty() {
        return Err(LlmError::Message("unfinished tool results"));
    }
    if p.supports_tools
        && tools.iter().any(|t| {
            t.name.is_empty()
                || t.name.len() > 64
                || !t
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                || !t.parameters.is_object()
        })
    {
        return Err(LlmError::Tool);
    }
    Ok(())
}
#[derive(Serialize)]
struct Body<'a> {
    model: &'a str,
    messages: Vec<Value>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<Value>,
}
fn openai(
    p: &Profile,
    msgs: &[ChatMessage],
    tools: &[ToolSpec],
    json_mode: bool,
) -> Result<String, LlmError> {
    let messages=msgs.iter().map(|m| {
        let role=match m.role{Role::System=>"system",Role::User=>"user",Role::Assistant=>"assistant",Role::Tool=>"tool"};
        let mut value=json!({"role":role,"content":m.content});
        if !m.tool_calls.is_empty(){value["tool_calls"]=Value::Array(m.tool_calls.iter().map(|c|json!({"id":c.id,"type":"function","function":{"name":c.name,"arguments":c.arguments}})).collect());if m.content.is_empty(){value["content"]=Value::Null;}}
        if let Some(id)=&m.tool_call_id {value["tool_call_id"]=id.clone().into();}value
    }).collect();
    let tools=(!tools.is_empty()&&p.supports_tools).then(||tools.iter().map(|t|json!({"type":"function","function":{"name":t.name,"description":t.description,"parameters":t.parameters}})).collect());
    serde_json::to_string(&Body {
        model: &p.model,
        messages,
        temperature: p.temperature,
        max_tokens: p.max_tokens,
        stream: true,
        tools,
        response_format: (json_mode && p.supports_json_mode).then(|| json!({"type":"json_object"})),
    })
    .map_err(|_| LlmError::Json)
}
/// Build the prescribed request for validated arguments; invalid inputs panic as programmer errors.
/// Runtime/user-input paths must use [`try_build_chat_request`] to return a checked failure.
pub fn build_chat_request(
    p: &Profile,
    msgs: &[ChatMessage],
    tools: &[ToolSpec],
    json_mode: bool,
    target: Target,
) -> HttpRequest {
    match try_build_chat_request(p, msgs, tools, json_mode, target) {
        Ok(request) => request,
        Err(error) => panic!("invalid chat request: {error}"),
    }
}
/// Checked actual chat request construction; no request is returned for invalid input.
pub fn try_build_chat_request(
    p: &Profile,
    msgs: &[ChatMessage],
    tools: &[ToolSpec],
    json_mode: bool,
    target: Target,
) -> Result<HttpRequest, LlmError> {
    validate(p, msgs, tools)?;
    let suffix = if p.kind == ProviderKind::Anthropic {
        "/v1/messages"
    } else {
        "/chat/completions"
    };
    let body = if p.kind == ProviderKind::Anthropic {
        anthropic::body(p, msgs, tools)?
    } else {
        openai(p, msgs, tools, json_mode)?
    };
    let body = merge_extra(p, &body)?;
    Ok(HttpRequest {
        method: "POST".into(),
        url: format!("{}{suffix}", p.base_url.trim_end_matches('/')),
        headers: headers::build(p, target)?,
        body,
        stream: true,
    })
}
pub(crate) fn merge_extra(p: &Profile, body: &str) -> Result<String, LlmError> {
    if p.extra_body.is_empty() {
        return Ok(body.into());
    }
    let mut value: Value = serde_json::from_str(body).map_err(|_| LlmError::Json)?;
    let object = value.as_object_mut().ok_or(LlmError::Json)?;
    for (name, value) in &p.extra_body {
        if object.contains_key(name) {
            return Err(LlmError::Profile("extra_body"));
        }
        object.insert(name.clone(), value.clone());
    }
    serde_json::to_string(&value).map_err(|_| LlmError::Json)
}
