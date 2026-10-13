//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentCanonicalMessage`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentCanonicalMessage {
    /// Contract field `record_type`.
    pub record_type: AgentCanonicalMessageRecordType,
    /// Contract field `message_version`.
    pub message_version: u32,
    /// Contract field `message_id`.
    pub message_id: String,
    /// Contract field `role`.
    pub role: AgentCanonicalMessageRole,
    /// Contract field `agent_turn_id`.
    pub agent_turn_id: Nullable<String>,
    /// Contract field `task_id`.
    pub task_id: Nullable<String>,
    /// Contract field `parts`.
    pub parts: Vec<AgentMessagePart>,
}
impl std::fmt::Debug for AgentCanonicalMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentCanonicalMessage { redacted }")
    }
}
