//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `AgentPhase`.
pub enum AgentPhase {
    /// Contract value `ready`.
    #[serde(rename = "ready")]
    Ready,
    /// Contract value `running`.
    #[serde(rename = "running")]
    Running,
    /// Contract value `waiting_user`.
    #[serde(rename = "waiting_user")]
    WaitingUser,
    /// Contract value `paused`.
    #[serde(rename = "paused")]
    Paused,
    /// Contract value `cancelling`.
    #[serde(rename = "cancelling")]
    Cancelling,
    /// Contract value `completed`.
    #[serde(rename = "completed")]
    Completed,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `cancelled`.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// Contract value `interrupted`.
    #[serde(rename = "interrupted")]
    Interrupted,
}
