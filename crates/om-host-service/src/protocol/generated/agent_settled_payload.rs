//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentSettledPayload`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSettledPayload {
    /// Contract field `reason`.
    pub reason: AgentSettledPayloadReason,
}
impl std::fmt::Debug for AgentSettledPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentSettledPayload { redacted }")
    }
}
