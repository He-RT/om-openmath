//! One frozen request and one result. No transport, authorization, retry or tool executor.
pub use om_agent_contract::{AgentCanonicalMessage, AgentCanonicalMessageRole, AgentMessagePart};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Wire protocols implemented by this single-turn codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    /// Chat completions text and function tools.
    ChatCompletions,
    /// Anthropic Messages text/thinking and tool blocks.
    AnthropicMessages,
}
/// A non-secret exact replay compatibility fence supplied by the trusted model preparer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// Frozen provider connection revision reference.
    pub provider_ref: String,
    /// Frozen route/protocol revision reference.
    pub route_ref: String,
    /// Frozen model revision reference.
    pub model_ref: String,
    /// Actual wire adapter version.
    pub adapter_version: String,
    /// Frozen wire protocol.
    pub protocol: Protocol,
}
/// Original provider message identity and replay-only body, never authority or user text.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replay {
    /// Exact compatibility scope.
    pub binding: Binding,
    /// Original provider response/message ID.
    pub response_id: String,
    /// Original provider model identifier.
    pub provider_model: String,
    /// Original role-assistant body/blocks needed for compatible replay.
    pub assistant: Value,
}
impl std::fmt::Debug for Replay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Replay { redacted }")
    }
}
/// Canonical event plus optional compatible provider-specific replay facts.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    /// OpenMath business message.
    pub canonical: AgentCanonicalMessage,
    /// Replay facts from a complete original response.
    pub replay: Option<Replay>,
}
/// Actual function schema controlled by the host.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    /// Stable declared name.
    pub name: String,
    /// Host-authored description.
    pub description: String,
    /// Data-only parameter schema.
    pub parameters: Value,
}
/// Prepared tool result text resolved from the canonical result_ref by the host.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolResult {
    /// Canonical tool message identity.
    pub message_id: String,
    /// Actual call association.
    pub tool_call_id: String,
    /// Complete bounded model-facing result, never an arbitrary URL/file reader.
    pub text: String,
    /// Original host result outcome indicates error.
    pub is_error: bool,
}
/// Exactly one immutable body request. No keys/headers/URL or loop state.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Trusted binding.
    pub binding: Binding,
    /// Provider API model ID (distinct from native immutable reference).
    pub api_model: String,
    /// Canonical message history selected by the host.
    pub messages: Vec<Message>,
    /// Resolved tool results, one per canonical call result.
    pub results: Vec<ToolResult>,
    /// Actual enabled schemas; codec cannot grant them.
    pub tools: Vec<Tool>,
    /// Explicit output token budget.
    pub max_output_tokens: u32,
    /// Optional supported sampling setting.
    pub temperature: Option<f64>,
    /// Whether native transport expects SSE.
    pub stream: bool,
}
/// Body only; trusted Swift endpoint/auth policy is applied later.
#[derive(Clone)]
pub struct Wire {
    /// Frozen binding.
    pub binding: Binding,
    /// Exact UTF8 serialized body.
    pub body: Vec<u8>,
    /// Actual streaming mode.
    pub stream: bool,
}
impl std::fmt::Debug for Wire {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Wire { redacted }")
    }
}
/// Usage absence stays unknown, never fabricated as zero.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    /// Provider observed input tokens.
    pub input: Option<u64>,
    /// Provider observed output tokens.
    pub output: Option<u64>,
    /// Provider observed total tokens, if reported.
    pub total: Option<u64>,
    /// Provider observed cache-read tokens.
    pub cache_read: Option<u64>,
    /// Provider observed cache-write tokens.
    pub cache_write: Option<u64>,
}
/// Semantic end state, separate from provider raw reason and transport EOF.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Finish {
    /// Complete assistant text/refusal response.
    Complete,
    /// Complete valid pending tool calls; no tools have executed.
    ToolCalls,
    /// Truncated/unknown/server-pause finish; no eligible calls.
    Incomplete,
}
/// A final canonical message and actual provider facts, not an executed task/result.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Result {
    /// Original complete canonical assistant message.
    pub message: AgentCanonicalMessage,
    /// Native normalized finish state.
    pub finish: Finish,
    /// Original provider stop_reason/finish_reason.
    pub provider_reason: String,
    /// Provider-observed usage, missing values unknown.
    pub usage: Usage,
    /// Original incomplete calls retained only as diagnostics.
    pub partial_calls: Vec<PartialCall>,
    /// Original compatible replay-only metadata.
    pub replay: Option<Replay>,
}
impl std::fmt::Debug for Result {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SingleTurnResult { redacted }")
    }
}
/// Original partial provider calls retained for diagnostics, never eligible tool proposals.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialCall {
    /// Actual provider block/choice index.
    pub index: u32,
    /// Original possibly incomplete provider ID.
    pub id: String,
    /// Original possibly incomplete name.
    pub name: String,
    /// Exact original argument bytes as UTF8 data.
    pub arguments: String,
}
impl std::fmt::Debug for PartialCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PartialCall { redacted }")
    }
}
/// Trusted identity and decoding policy frozen before dispatch.
pub struct ResponseScope {
    /// Original request binding.
    pub binding: Binding,
    /// Host-owned canonical output message ID.
    pub message_id: String,
    /// Host-owned current task ID.
    pub task_id: String,
    /// Host-owned current turn ID.
    pub turn_id: String,
    /// Declared current request tool names, not permission grants.
    pub tools: Vec<String>,
    /// Original streaming choice.
    pub stream: bool,
}
/// Presentation-only incremental updates. Tool fragments never become eligible calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Delta {
    /// Exact text UTF8 bytes, with scalar boundaries preserved.
    Text(String),
    /// Provider-indexed original argument fragment.
    ToolArguments {
        /// Original provider block/choice index.
        index: u32,
        /// Original argument data, not executable code.
        fragment: String,
    },
}
/// Constant diagnostic categories never echo provider bytes or credentials.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Invalid body/canonical input.
    #[error("invalid single-turn input")]
    Input,
    /// Unsupported rich part/response item, never silently discarded.
    #[error("unsupported single-turn content")]
    Unsupported,
    /// Incompatible replay scope.
    #[error("incompatible provider replay")]
    Replay,
    /// Bad UTF8/JSON/frame.
    #[error("invalid single-turn response")]
    Response,
    /// Incomplete SSE/semantic end.
    #[error("incomplete single-turn response")]
    Incomplete,
    /// Bounded bytes/blocks/calls exceeded.
    #[error("single-turn bound exceeded")]
    Limit,
    /// HTTP non-success; body is deliberately not formatted.
    #[error("single-turn HTTP failure")]
    Http {
        /// Actual HTTP status.
        status: u16,
    },
    /// Explicit native cancellation.
    #[error("single-turn cancelled")]
    Cancelled,
    /// Prior stream failed/settled.
    #[error("single-turn decoder closed")]
    Closed,
    /// Known provider stream error without its untrusted message.
    #[error("single-turn provider error")]
    Provider,
}
