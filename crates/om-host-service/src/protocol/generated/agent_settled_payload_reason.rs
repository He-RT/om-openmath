//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `AgentSettledPayloadReason`.
pub enum AgentSettledPayloadReason {
    /// Contract value `loop_stopped`.
    #[serde(rename = "loop_stopped")]
    LoopStopped,
    /// Contract value `cancelled`.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `interrupted`.
    #[serde(rename = "interrupted")]
    Interrupted,
}
