//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentReadTool`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentReadTool {
    /// Contract field `type`.
    pub r#type: NativeAgentReadToolType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `operation_id`.
    pub operation_id: String,
}
impl std::fmt::Debug for NativeAgentReadTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentReadTool { redacted }")
    }
}
