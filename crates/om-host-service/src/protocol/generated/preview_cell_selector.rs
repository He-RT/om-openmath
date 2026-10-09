//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `PreviewCellSelector`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewCellSelector {
    /// Variant `PreviewExistingCell`.
    PreviewExistingCell(PreviewExistingCell),
    /// Variant `PreviewNewCell`.
    PreviewNewCell(PreviewNewCell),
}
