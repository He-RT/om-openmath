//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceBeginAgentUndo`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBeginAgentUndo {
    /// Contract field `type`.
    pub r#type: SourceBeginAgentUndoType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `records`.
    pub records: Nullable<Vec<NativeStoredSourceTransaction>>,
    /// Contract field `allowed_transaction_refs`.
    pub allowed_transaction_refs: Nullable<Vec<String>>,
    /// Contract field `failure_code`.
    pub failure_code: Nullable<SourceBeginAgentUndoFailureCode>,
}
impl std::fmt::Debug for SourceBeginAgentUndo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceBeginAgentUndo { redacted }")
    }
}
