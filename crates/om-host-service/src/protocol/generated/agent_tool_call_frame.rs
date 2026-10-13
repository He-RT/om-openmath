//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentToolCallFrame`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentToolCallFrame {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `connection_id`.
    pub connection_id: String,
    /// Contract field `connection_generation`.
    pub connection_generation: Serial,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `agent_session_id`.
    pub agent_session_id: String,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `task_id`.
    pub task_id: String,
    /// Contract field `task_generation`.
    pub task_generation: Serial,
    /// Contract field `message_id`.
    pub message_id: String,
    /// Contract field `sequence`.
    pub sequence: Serial,
    /// Contract field `reply_to`.
    pub reply_to: Nullable<String>,
    /// Contract field `type`.
    pub r#type: AgentToolCallFrameType,
    /// Contract field `payload`.
    pub payload: AgentToolCallPayload,
}
impl std::fmt::Debug for AgentToolCallFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentToolCallFrame { redacted }")
    }
}
