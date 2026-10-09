//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativePreviewDataPreviewKind`.
pub enum NativePreviewDataPreviewKind {
    /// Contract value `source`.
    #[serde(rename = "source")]
    Source,
    /// Contract value `patch`.
    #[serde(rename = "patch")]
    Patch,
}
