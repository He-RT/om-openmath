//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `PreviewPatchOperation`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewPatchOperation {
    /// Variant `PreviewInsertCell`.
    PreviewInsertCell(PreviewInsertCell),
    /// Variant `PreviewUpdateCell`.
    PreviewUpdateCell(PreviewUpdateCell),
    /// Variant `PreviewReplaceText`.
    PreviewReplaceText(PreviewReplaceText),
    /// Variant `PreviewDeleteCell`.
    PreviewDeleteCell(PreviewDeleteCell),
    /// Variant `PreviewMoveCell`.
    PreviewMoveCell(PreviewMoveCell),
    /// Variant `PreviewRenameNotebook`.
    PreviewRenameNotebook(PreviewRenameNotebook),
}
