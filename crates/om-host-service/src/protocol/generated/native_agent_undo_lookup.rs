//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentUndoLookup`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentUndoLookup {
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `arguments_hash`.
    pub arguments_hash: String,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `store_id`.
    pub store_id: String,
    /// Contract field `snapshot_hash`.
    pub snapshot_hash: String,
    /// Contract field `source_revision`.
    pub source_revision: Serial,
    /// Contract field `transaction_refs`.
    pub transaction_refs: Vec<String>,
}
impl std::fmt::Debug for NativeAgentUndoLookup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentUndoLookup { redacted }")
    }
}
