//! Shared transport and conversation DTOs; no HTTP or job runtime.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
/// ProviderKind values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// OpenaiChat.
    OpenaiChat,
    /// Anthropic.
    Anthropic,
    /// OpenaiFim.
    OpenaiFim,
    /// OllamaFim.
    OllamaFim,
    /// MistralFim.
    MistralFim,
}

/// Role values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// System.
    System,
    /// User.
    User,
    /// Assistant.
    Assistant,
    /// Tool.
    Tool,
}

/// A complete sans-IO request for native or browser transport.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct HttpRequest {
    /// HTTP method.
    pub method: String,
    /// Configured endpoint.
    pub url: String,
    /// Ordered header pairs.
    pub headers: Vec<(String, String)>,
    /// Encoded request body.
    pub body: String,
    /// Whether the response is streamed.
    pub stream: bool,
}

/// A conversation turn including tool-call associations.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct ChatMessage {
    /// Speaker role.
    pub role: Role,
    /// Text content.
    pub content: String,
    /// Assistant tool calls.
    pub tool_calls: Vec<ToolCall>,
    /// Associated call for a tool result.
    pub tool_call_id: Option<String>,
}

/// An LLM-proposed tool invocation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct ToolCall {
    /// Provider call identifier.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// JSON argument text; interpreted by the later tool handler.
    pub arguments: String,
}

/// Source and display forms checked by the CAS before delivery.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct Suggestion {
    /// Wolfram source form.
    pub wolfram: String,
    /// Modern source form.
    pub modern: String,
    /// CAS-rendered LaTeX.
    pub latex: String,
    /// Short model explanation.
    pub explanation: String,
}
