//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentHelloPayload`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentHelloPayload {
    /// Contract field `core_version`.
    pub core_version: String,
    /// Contract field `adapter_version`.
    pub adapter_version: String,
    /// Contract field `schema_hash`.
    pub schema_hash: String,
}
impl std::fmt::Debug for AgentHelloPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentHelloPayload { redacted }")
    }
}
