//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeAgentEffectResultOutcome`.
pub enum NativeAgentEffectResultOutcome {
    /// Contract value `completed`.
    #[serde(rename = "completed")]
    Completed,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `cancelled`.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// Contract value `unknown`.
    #[serde(rename = "unknown")]
    Unknown,
    /// Contract value `partial`.
    #[serde(rename = "partial")]
    Partial,
}
