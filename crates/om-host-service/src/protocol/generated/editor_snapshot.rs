//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `EditorSnapshot`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorSnapshot {
    /// Contract field `record_type`.
    pub record_type: EditorSnapshotRecordType,
    /// Contract field `key`.
    pub key: EditorKey,
    /// Contract field `source`.
    pub source: String,
    /// Contract field `composition`.
    pub composition: EditorSnapshotComposition,
    /// Contract field `marked_range_utf16`.
    pub marked_range_utf16: Nullable<NativeRange>,
    /// Contract field `committed_cell_revision`.
    pub committed_cell_revision: Serial,
    /// Contract field `is_source_dirty`.
    pub is_source_dirty: bool,
}
impl std::fmt::Debug for EditorSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EditorSnapshot { redacted }")
    }
}
