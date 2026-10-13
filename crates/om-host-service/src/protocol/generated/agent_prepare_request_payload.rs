//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentPrepareRequestPayload`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPrepareRequestPayload {
    /// Contract field `agent_turn_id`.
    pub agent_turn_id: String,
    /// Contract field `request_id`.
    pub request_id: String,
}
impl std::fmt::Debug for AgentPrepareRequestPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentPrepareRequestPayload { redacted }")
    }
}
