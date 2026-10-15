//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeRunProgressPhase`.
pub enum NativeRunProgressPhase {
    /// Contract value `planning`.
    #[serde(rename = "planning")]
    Planning,
    /// Contract value `awaiting_admission`.
    #[serde(rename = "awaiting_admission")]
    AwaitingAdmission,
    /// Contract value `running`.
    #[serde(rename = "running")]
    Running,
    /// Contract value `awaiting_acceptance`.
    #[serde(rename = "awaiting_acceptance")]
    AwaitingAcceptance,
    /// Contract value `unknown`.
    #[serde(rename = "unknown")]
    Unknown,
    /// Contract value `completed`.
    #[serde(rename = "completed")]
    Completed,
    /// Contract value `partial`.
    #[serde(rename = "partial")]
    Partial,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `cancelled`.
    #[serde(rename = "cancelled")]
    Cancelled,
}
