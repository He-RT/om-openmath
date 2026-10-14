//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentConfirmTool`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentConfirmTool {
    /// Contract field `type`.
    pub r#type: NativeAgentConfirmToolType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `result_ref`.
    pub result_ref: String,
    /// Contract field `result_hash`.
    pub result_hash: String,
    /// Contract field `event_id`.
    pub event_id: String,
    /// Contract field `event_sequence`.
    pub event_sequence: Serial,
    /// Contract field `receipt_hash`.
    pub receipt_hash: String,
}
impl std::fmt::Debug for NativeAgentConfirmTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentConfirmTool { redacted }")
    }
}
