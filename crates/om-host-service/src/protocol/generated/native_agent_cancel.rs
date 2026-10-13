//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentCancel`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentCancel {
    /// Contract field `type`.
    pub r#type: NativeAgentCancelType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `reason`.
    pub reason: AgentCancelPayloadReason,
}
impl std::fmt::Debug for NativeAgentCancel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentCancel { redacted }")
    }
}
