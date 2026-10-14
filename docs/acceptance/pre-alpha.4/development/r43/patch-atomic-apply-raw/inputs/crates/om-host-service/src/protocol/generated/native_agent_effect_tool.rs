//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentEffectTool`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentEffectTool {
    /// Contract field `type`.
    pub r#type: NativeAgentEffectToolType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `failure_code`.
    pub failure_code: Nullable<NativeAgentEffectToolFailureCode>,
}
impl std::fmt::Debug for NativeAgentEffectTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentEffectTool { redacted }")
    }
}
