//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeSourceAdmissionPhase`.
pub enum NativeSourceAdmissionPhase {
    /// Contract value `accepted`.
    #[serde(rename = "accepted")]
    Accepted,
    /// Contract value `completed`.
    #[serde(rename = "completed")]
    Completed,
    /// Contract value `cancelled`.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `unknown`.
    #[serde(rename = "unknown")]
    Unknown,
}
