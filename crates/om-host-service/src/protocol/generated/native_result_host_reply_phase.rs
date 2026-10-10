//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeResultHostReplyPhase`.
pub enum NativeResultHostReplyPhase {
    /// Contract value `queued`.
    #[serde(rename = "queued")]
    Queued,
    /// Contract value `running`.
    #[serde(rename = "running")]
    Running,
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
