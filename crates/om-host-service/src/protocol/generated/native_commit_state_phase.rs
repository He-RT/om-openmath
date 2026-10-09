//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeCommitStatePhase`.
pub enum NativeCommitStatePhase {
    /// Contract value `awaiting_admission`.
    #[serde(rename = "awaiting_admission")]
    AwaitingAdmission,
    /// Contract value `awaiting_fence`.
    #[serde(rename = "awaiting_fence")]
    AwaitingFence,
    /// Contract value `writing`.
    #[serde(rename = "writing")]
    Writing,
    /// Contract value `committing`.
    #[serde(rename = "committing")]
    Committing,
    /// Contract value `unknown`.
    #[serde(rename = "unknown")]
    Unknown,
    /// Contract value `completed`.
    #[serde(rename = "completed")]
    Completed,
    /// Contract value `cancelled`.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
}
