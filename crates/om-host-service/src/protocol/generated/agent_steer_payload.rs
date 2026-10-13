//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentSteerPayload`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSteerPayload {
    /// Contract field `event_ref`.
    pub event_ref: String,
    /// Contract field `message`.
    pub message: AgentCanonicalMessage,
}
impl std::fmt::Debug for AgentSteerPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentSteerPayload { redacted }")
    }
}
