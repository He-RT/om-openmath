//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeEditorTarget`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEditorTarget {
    /// Contract field `cell_id`.
    pub cell_id: String,
    /// Contract field `editor_generation`.
    pub editor_generation: Serial,
    /// Contract field `draft_sequence`.
    pub draft_sequence: Serial,
    /// Contract field `base_cell_revision`.
    pub base_cell_revision: Serial,
    /// Contract field `source_hash`.
    pub source_hash: String,
    /// Contract field `is_composing`.
    pub is_composing: bool,
    /// Contract field `is_dirty`.
    pub is_dirty: bool,
}
impl std::fmt::Debug for NativeEditorTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeEditorTarget { redacted }")
    }
}
