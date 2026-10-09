//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeEditorState`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEditorState {
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `source_revision`.
    pub source_revision: Serial,
    /// Contract field `targets`.
    pub targets: Vec<NativeEditorTarget>,
}
impl std::fmt::Debug for NativeEditorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeEditorState { redacted }")
    }
}
