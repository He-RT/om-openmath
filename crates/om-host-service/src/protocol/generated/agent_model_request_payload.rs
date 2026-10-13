//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentModelRequestPayload`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentModelRequestPayload {
    /// Contract field `agent_turn_id`.
    pub agent_turn_id: String,
    /// Contract field `request_id`.
    pub request_id: String,
    /// Contract field `context_ref`.
    pub context_ref: String,
    /// Contract field `model_config_ref`.
    pub model_config_ref: String,
}
impl std::fmt::Debug for AgentModelRequestPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentModelRequestPayload { redacted }")
    }
}
