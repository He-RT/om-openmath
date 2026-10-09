//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `GhostCandidatePhase`.
pub enum GhostCandidatePhase {
    /// Contract value `ready`.
    #[serde(rename = "ready")]
    Ready,
    /// Contract value `expired`.
    #[serde(rename = "expired")]
    Expired,
    /// Contract value `cancelled`.
    #[serde(rename = "cancelled")]
    Cancelled,
}
