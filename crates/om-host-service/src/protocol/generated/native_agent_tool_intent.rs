//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentToolIntent`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentToolIntent {
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `agent_turn_id`.
    pub agent_turn_id: String,
    /// Contract field `tool_call_id`.
    pub tool_call_id: String,
    /// Contract field `proposal_event_ref`.
    pub proposal_event_ref: String,
    /// Contract field `name`.
    pub name: AgentToolName,
    /// Contract field `arguments_hash`.
    pub arguments_hash: String,
}
impl std::fmt::Debug for NativeAgentToolIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentToolIntent { redacted }")
    }
}
