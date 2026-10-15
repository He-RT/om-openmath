//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeOperationClaimObservationPhase`.
pub enum NativeOperationClaimObservationPhase {
    /// Contract value `queued`.
    #[serde(rename = "queued")]
    Queued,
    /// Contract value `running`.
    #[serde(rename = "running")]
    Running,
    /// Contract value `unknown`.
    #[serde(rename = "unknown")]
    Unknown,
}
