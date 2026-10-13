//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentControlScope`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentControlScope {
    /// Contract field `agent_session_id`.
    pub agent_session_id: String,
    /// Contract field `task_id`.
    pub task_id: String,
    /// Contract field `task_generation`.
    pub task_generation: Serial,
    /// Contract field `connection_id`.
    pub connection_id: String,
    /// Contract field `connection_generation`.
    pub connection_generation: Serial,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
}
impl std::fmt::Debug for NativeAgentControlScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentControlScope { redacted }")
    }
}
