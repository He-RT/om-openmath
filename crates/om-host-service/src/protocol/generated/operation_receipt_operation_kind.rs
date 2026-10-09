//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `OperationReceiptOperationKind`.
pub enum OperationReceiptOperationKind {
    /// Contract value `source_edit`.
    #[serde(rename = "source_edit")]
    SourceEdit,
    /// Contract value `undo`.
    #[serde(rename = "undo")]
    Undo,
    /// Contract value `run_cells`.
    #[serde(rename = "run_cells")]
    RunCells,
    /// Contract value `scratch`.
    #[serde(rename = "scratch")]
    Scratch,
    /// Contract value `media_prepare`.
    #[serde(rename = "media_prepare")]
    MediaPrepare,
    /// Contract value `save_file`.
    #[serde(rename = "save_file")]
    SaveFile,
}
