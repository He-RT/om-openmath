//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `EditorKey`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorKey {
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `cell_id`.
    pub cell_id: String,
    /// Contract field `editor_id`.
    pub editor_id: String,
    /// Contract field `editor_generation`.
    pub editor_generation: Serial,
    /// Contract field `draft_sequence`.
    pub draft_sequence: Serial,
    /// Contract field `source_hash`.
    pub source_hash: String,
    /// Contract field `requested_dialect`.
    pub requested_dialect: Dialect,
    /// Contract field `effective_dialect`.
    pub effective_dialect: Dialect,
    /// Contract field `analysis_generation`.
    pub analysis_generation: Serial,
    /// Contract field `metadata_version`.
    pub metadata_version: Serial,
    /// Contract field `definition_snapshot_revision`.
    pub definition_snapshot_revision: Serial,
    /// Contract field `config_revision`.
    pub config_revision: Serial,
    /// Contract field `cursor_utf8`.
    pub cursor_utf8: Nullable<u32>,
    /// Contract field `selection_utf16`.
    pub selection_utf16: NativeRange,
}
impl std::fmt::Debug for EditorKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EditorKey { redacted }")
    }
}
