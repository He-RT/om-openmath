//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `PiPhase`.
pub enum PiPhase {
    /// Contract value `stopped`.
    #[serde(rename = "stopped")]
    Stopped,
    /// Contract value `starting`.
    #[serde(rename = "starting")]
    Starting,
    /// Contract value `ready`.
    #[serde(rename = "ready")]
    Ready,
    /// Contract value `running`.
    #[serde(rename = "running")]
    Running,
    /// Contract value `stopping`.
    #[serde(rename = "stopping")]
    Stopping,
    /// Contract value `crashed`.
    #[serde(rename = "crashed")]
    Crashed,
}
