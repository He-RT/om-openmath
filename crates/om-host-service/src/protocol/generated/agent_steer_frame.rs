//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentSteerFrame`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSteerFrame {
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
    pub r#type: AgentSteerFrameType,
    /// Contract field `payload`.
    pub payload: AgentSteerPayload,
}
impl std::fmt::Debug for AgentSteerFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentSteerFrame { redacted }")
    }
}
