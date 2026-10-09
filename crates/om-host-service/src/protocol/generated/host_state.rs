//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `HostState`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostState {
    /// Contract field `contract_version`.
    pub contract_version: u32,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `host_phase`.
    pub host_phase: HostPhase,
    /// Contract field `ui_event_sequence`.
    pub ui_event_sequence: Serial,
    /// Contract field `rust_event_sequence`.
    pub rust_event_sequence: Serial,
    /// Contract field `document`.
    pub document: Nullable<DocumentState>,
    /// Contract field `agent_task`.
    pub agent_task: Nullable<AgentTaskState>,
    /// Contract field `pi`.
    pub pi: PiState,
    /// Contract field `operations`.
    pub operations: Vec<Operation>,
}
impl std::fmt::Debug for HostState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HostState { redacted }")
    }
}
