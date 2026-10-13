//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentActivate`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentActivate {
    /// Contract field `type`.
    pub r#type: NativeAgentActivateType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `hello_id`.
    pub hello_id: String,
    /// Contract field `core_version`.
    pub core_version: String,
    /// Contract field `adapter_version`.
    pub adapter_version: String,
    /// Contract field `grants`.
    pub grants: Vec<AgentToolName>,
    /// Contract field `readonly`.
    pub readonly: bool,
}
impl std::fmt::Debug for NativeAgentActivate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentActivate { redacted }")
    }
}
