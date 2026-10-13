//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentModelEventPayload`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentModelEventPayload {
    /// Contract field `agent_turn_id`.
    pub agent_turn_id: String,
    /// Contract field `request_id`.
    pub request_id: String,
    /// Contract field `byte_offset`.
    pub byte_offset: Serial,
    /// Contract field `text`.
    pub text: String,
    /// Contract field `phase`.
    pub phase: AgentModelEventPayloadPhase,
    /// Contract field `canonical_message`.
    pub canonical_message: Nullable<AgentCanonicalMessage>,
}
impl std::fmt::Debug for AgentModelEventPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentModelEventPayload { redacted }")
    }
}
