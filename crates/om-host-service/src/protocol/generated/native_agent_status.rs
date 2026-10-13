//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentStatus`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentStatus {
    /// Contract field `type`.
    pub r#type: NativeAgentStatusType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
}
impl std::fmt::Debug for NativeAgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentStatus { redacted }")
    }
}
