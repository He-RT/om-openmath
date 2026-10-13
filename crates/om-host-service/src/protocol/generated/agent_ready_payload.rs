//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentReadyPayload`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentReadyPayload {
    /// Contract field `core_version`.
    pub core_version: String,
    /// Contract field `adapter_version`.
    pub adapter_version: String,
    /// Contract field `schema_hash`.
    pub schema_hash: String,
}
impl std::fmt::Debug for AgentReadyPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentReadyPayload { redacted }")
    }
}
