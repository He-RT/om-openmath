//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `AgentToolResultPayloadOutcome`.
pub enum AgentToolResultPayloadOutcome {
    /// Contract value `completed`.
    #[serde(rename = "completed")]
    Completed,
    /// Contract value `partial`.
    #[serde(rename = "partial")]
    Partial,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `unavailable`.
    #[serde(rename = "unavailable")]
    Unavailable,
}
