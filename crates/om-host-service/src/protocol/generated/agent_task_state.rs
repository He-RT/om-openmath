//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentTaskState`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTaskState {
    /// Contract field `task_id`.
    pub task_id: String,
    /// Contract field `task_generation`.
    pub task_generation: Serial,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `phase`.
    pub phase: AgentPhase,
    /// Contract field `mode`.
    pub mode: AgentTaskStateMode,
    /// Contract field `active_operation_refs`.
    pub active_operation_refs: Vec<String>,
    /// Contract field `model_config_revision`.
    pub model_config_revision: Serial,
    /// Contract field `context_revision`.
    pub context_revision: Serial,
}
impl std::fmt::Debug for AgentTaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentTaskState { redacted }")
    }
}
