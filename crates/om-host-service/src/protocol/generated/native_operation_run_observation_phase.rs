//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeOperationRunObservationPhase`.
pub enum NativeOperationRunObservationPhase {
    /// Contract value `queued`.
    #[serde(rename = "queued")]
    Queued,
    /// Contract value `running`.
    #[serde(rename = "running")]
    Running,
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
    /// Contract value `unknown`.
    #[serde(rename = "unknown")]
    Unknown,
}
