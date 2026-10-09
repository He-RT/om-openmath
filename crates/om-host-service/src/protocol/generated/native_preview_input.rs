//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `NativePreviewInput`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NativePreviewInput {
    /// Variant `PreviewSourceInput`.
    PreviewSourceInput(PreviewSourceInput),
    /// Variant `PreviewPatchInput`.
    PreviewPatchInput(PreviewPatchInput),
}
