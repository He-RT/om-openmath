//! Actual configured chat request construction; no network or substitute execution.
mod anthropic;
mod headers;
use crate::{ChatMessage, HttpRequest, ProviderKind, Role};
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
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LlmError {
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
fn validate(p: &Profile, msgs: &[ChatMessage], tools: &[ToolSpec]) -> Result<(), LlmError> {
    if !matches!(p.kind, ProviderKind::OpenaiChat | ProviderKind::Anthropic) {
        return Err(LlmError::Provider);
    }
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
    Ok(HttpRequest {
        method: "POST".into(),
        url: format!("{}{suffix}", p.base_url.trim_end_matches('/')),
        headers: headers::build(p, target)?,
        body,
        stream: true,
    })
}
