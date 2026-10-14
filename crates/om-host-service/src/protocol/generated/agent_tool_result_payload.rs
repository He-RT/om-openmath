//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentToolResultPayload`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentToolResultPayload {
    /// Contract field `agent_turn_id`.
    pub agent_turn_id: String,
    /// Contract field `tool_call_id`.
    pub tool_call_id: String,
    /// Contract field `operation_ref`.
    pub operation_ref: Nullable<String>,
    /// Contract field `result_ref`.
    pub result_ref: Nullable<String>,
    /// Contract field `outcome`.
    pub outcome: AgentToolResultPayloadOutcome,
    /// Contract field `result_json`.
    pub result_json: Nullable<String>,
}
impl std::fmt::Debug for AgentToolResultPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentToolResultPayload { redacted }")
    }
}
