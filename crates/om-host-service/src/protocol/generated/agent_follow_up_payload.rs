//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentFollowUpPayload`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentFollowUpPayload {
    /// Contract field `event_ref`.
    pub event_ref: String,
    /// Contract field `message`.
    pub message: AgentCanonicalMessage,
}
impl std::fmt::Debug for AgentFollowUpPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentFollowUpPayload { redacted }")
    }
}
