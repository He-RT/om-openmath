//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeAgentHostReplyPhase`.
pub enum NativeAgentHostReplyPhase {
    /// Contract value `starting`.
    #[serde(rename = "starting")]
    Starting,
    /// Contract value `ready`.
    #[serde(rename = "ready")]
    Ready,
    /// Contract value `cancelling`.
    #[serde(rename = "cancelling")]
    Cancelling,
    /// Contract value `awaiting_settlement`.
    #[serde(rename = "awaiting_settlement")]
    AwaitingSettlement,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `closed`.
    #[serde(rename = "closed")]
    Closed,
}
