//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentTranscriptEventPayload`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTranscriptEventPayload {
    /// Contract field `event_id`.
    pub event_id: String,
    /// Contract field `source_sequence`.
    pub source_sequence: Serial,
    /// Contract field `message`.
    pub message: AgentCanonicalMessage,
}
impl std::fmt::Debug for AgentTranscriptEventPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentTranscriptEventPayload { redacted }")
    }
}
