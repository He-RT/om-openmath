//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeAgentEffectToolFailureCode`.
pub enum NativeAgentEffectToolFailureCode {
    /// Contract value `EDITING_BUSY`.
    #[serde(rename = "EDITING_BUSY")]
    EditingBusy,
    /// Contract value `PREVIEW_MISMATCH`.
    #[serde(rename = "PREVIEW_MISMATCH")]
    PreviewMismatch,
}
