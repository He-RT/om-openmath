//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `OperationKind`.
pub enum OperationKind {
    /// Contract value `document_commit`.
    #[serde(rename = "document_commit")]
    DocumentCommit,
    /// Contract value `main_compute`.
    #[serde(rename = "main_compute")]
    MainCompute,
    /// Contract value `scratch`.
    #[serde(rename = "scratch")]
    Scratch,
    /// Contract value `sampling`.
    #[serde(rename = "sampling")]
    Sampling,
    /// Contract value `editor_analysis`.
    #[serde(rename = "editor_analysis")]
    EditorAnalysis,
    /// Contract value `save`.
    #[serde(rename = "save")]
    Save,
    /// Contract value `media_prepare`.
    #[serde(rename = "media_prepare")]
    MediaPrepare,
    /// Contract value `model_request`.
    #[serde(rename = "model_request")]
    ModelRequest,
    /// Contract value `undo`.
    #[serde(rename = "undo")]
    Undo,
}
