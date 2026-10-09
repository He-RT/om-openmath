//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `OperationPhase`.
pub enum OperationPhase {
    /// Contract value `accepted`.
    #[serde(rename = "accepted")]
    Accepted,
    /// Contract value `queued`.
    #[serde(rename = "queued")]
    Queued,
    /// Contract value `running`.
    #[serde(rename = "running")]
    Running,
    /// Contract value `awaiting_confirmation`.
    #[serde(rename = "awaiting_confirmation")]
    AwaitingConfirmation,
    /// Contract value `cancelling`.
    #[serde(rename = "cancelling")]
    Cancelling,
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
