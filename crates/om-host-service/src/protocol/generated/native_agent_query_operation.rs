//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentQueryOperation`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentQueryOperation {
    /// Contract field `type`.
    pub r#type: NativeAgentQueryOperationType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `observation`.
    pub observation: Nullable<NativeOperationObservation>,
    /// Contract field `failure_code`.
    pub failure_code: Nullable<NativeAgentQueryOperationFailureCode>,
}
impl std::fmt::Debug for NativeAgentQueryOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentQueryOperation { redacted }")
    }
}
