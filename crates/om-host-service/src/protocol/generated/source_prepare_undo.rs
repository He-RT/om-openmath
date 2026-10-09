//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourcePrepareUndo`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePrepareUndo {
    /// Contract field `type`.
    pub r#type: SourcePrepareUndoType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `group_id`.
    pub group_id: String,
    /// Contract field `records`.
    pub records: Vec<NativeStoredSourceTransaction>,
}
impl std::fmt::Debug for SourcePrepareUndo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourcePrepareUndo { redacted }")
    }
}
