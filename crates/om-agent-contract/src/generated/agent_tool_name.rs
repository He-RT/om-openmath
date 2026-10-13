//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `AgentToolName`.
pub enum AgentToolName {
    /// Contract value `read_notebook`.
    #[serde(rename = "read_notebook")]
    ReadNotebook,
    /// Contract value `search_functions`.
    #[serde(rename = "search_functions")]
    SearchFunctions,
    /// Contract value `get_function_docs`.
    #[serde(rename = "get_function_docs")]
    GetFunctionDocs,
    /// Contract value `preview_source`.
    #[serde(rename = "preview_source")]
    PreviewSource,
    /// Contract value `apply_notebook_patch`.
    #[serde(rename = "apply_notebook_patch")]
    ApplyNotebookPatch,
    /// Contract value `run_cells`.
    #[serde(rename = "run_cells")]
    RunCells,
    /// Contract value `inspect_result`.
    #[serde(rename = "inspect_result")]
    InspectResult,
    /// Contract value `evaluate_scratch`.
    #[serde(rename = "evaluate_scratch")]
    EvaluateScratch,
    /// Contract value `read_attachment`.
    #[serde(rename = "read_attachment")]
    ReadAttachment,
    /// Contract value `prepare_attachment`.
    #[serde(rename = "prepare_attachment")]
    PrepareAttachment,
    /// Contract value `get_operation_status`.
    #[serde(rename = "get_operation_status")]
    GetOperationStatus,
    /// Contract value `undo_transaction`.
    #[serde(rename = "undo_transaction")]
    UndoTransaction,
}
