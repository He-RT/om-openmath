//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeKernelHostReplyPhase`.
pub enum NativeKernelHostReplyPhase {
    /// Contract value `state`.
    #[serde(rename = "state")]
    State,
    /// Contract value `queued`.
    #[serde(rename = "queued")]
    Queued,
    /// Contract value `running`.
    #[serde(rename = "running")]
    Running,
    /// Contract value `waiting_source`.
    #[serde(rename = "waiting_source")]
    WaitingSource,
    /// Contract value `candidate`.
    #[serde(rename = "candidate")]
    Candidate,
    /// Contract value `committing`.
    #[serde(rename = "committing")]
    Committing,
    /// Contract value `unknown`.
    #[serde(rename = "unknown")]
    Unknown,
    /// Contract value `completed`.
    #[serde(rename = "completed")]
    Completed,
    /// Contract value `discarded`.
    #[serde(rename = "discarded")]
    Discarded,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `blob`.
    #[serde(rename = "blob")]
    Blob,
}
