//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `NativeSourceOperation`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NativeSourceOperation {
    /// Variant `InsertSourceCell`.
    InsertSourceCell(InsertSourceCell),
    /// Variant `UpdateSourceCell`.
    UpdateSourceCell(UpdateSourceCell),
    /// Variant `DeleteSourceCells`.
    DeleteSourceCells(DeleteSourceCells),
    /// Variant `MoveSourceCells`.
    MoveSourceCells(MoveSourceCells),
    /// Variant `RenameSourceNotebook`.
    RenameSourceNotebook(RenameSourceNotebook),
}
