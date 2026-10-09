//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeSourceTransactionTombstone`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSourceTransactionTombstone {
    /// Contract field `record_type`.
    pub record_type: NativeSourceTransactionTombstoneRecordType,
    /// Contract field `codec_version`.
    pub codec_version: u32,
    /// Contract field `commit`.
    pub commit: DocumentCommit,
    /// Contract field `calculation_change`.
    pub calculation_change: Nullable<NativeCalculationChange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Contract field `undo_group`.
    pub undo_group: Option<NativeUndoGroup>,
    /// Contract field `forward_plan_hash`.
    pub forward_plan_hash: String,
    /// Contract field `inverse_payload_hash`.
    pub inverse_payload_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Contract field `input_group_id`.
    pub input_group_id: Option<String>,
}
impl std::fmt::Debug for NativeSourceTransactionTombstone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeSourceTransactionTombstone { redacted }")
    }
}
