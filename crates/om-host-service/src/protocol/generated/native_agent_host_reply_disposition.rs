//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeAgentHostReplyDisposition`.
pub enum NativeAgentHostReplyDisposition {
    /// Contract value `hello`.
    #[serde(rename = "hello")]
    Hello,
    /// Contract value `ready`.
    #[serde(rename = "ready")]
    Ready,
    /// Contract value `duplicate`.
    #[serde(rename = "duplicate")]
    Duplicate,
    /// Contract value `transcript`.
    #[serde(rename = "transcript")]
    Transcript,
    /// Contract value `tool_intent`.
    #[serde(rename = "tool_intent")]
    ToolIntent,
    /// Contract value `request_intent`.
    #[serde(rename = "request_intent")]
    RequestIntent,
    /// Contract value `data`.
    #[serde(rename = "data")]
    Data,
    /// Contract value `ack`.
    #[serde(rename = "ack")]
    Ack,
    /// Contract value `sent`.
    #[serde(rename = "sent")]
    Sent,
    /// Contract value `cancel`.
    #[serde(rename = "cancel")]
    Cancel,
    /// Contract value `closed`.
    #[serde(rename = "closed")]
    Closed,
    /// Contract value `status`.
    #[serde(rename = "status")]
    Status,
}
