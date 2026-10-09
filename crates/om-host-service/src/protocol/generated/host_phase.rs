//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `HostPhase`.
pub enum HostPhase {
    /// Contract value `starting`.
    #[serde(rename = "starting")]
    Starting,
    /// Contract value `ready`.
    #[serde(rename = "ready")]
    Ready,
    /// Contract value `closing`.
    #[serde(rename = "closing")]
    Closing,
    /// Contract value `closed`.
    #[serde(rename = "closed")]
    Closed,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
}
