//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `AgentModelEventPayloadPhase`.
pub enum AgentModelEventPayloadPhase {
    /// Contract value `delta`.
    #[serde(rename = "delta")]
    Delta,
    /// Contract value `finished`.
    #[serde(rename = "finished")]
    Finished,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `cancelled`.
    #[serde(rename = "cancelled")]
    Cancelled,
}
