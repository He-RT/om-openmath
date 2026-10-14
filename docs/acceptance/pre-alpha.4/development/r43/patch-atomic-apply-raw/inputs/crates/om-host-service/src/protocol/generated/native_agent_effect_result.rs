//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentEffectResult`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentEffectResult {
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `agent_turn_id`.
    pub agent_turn_id: String,
    /// Contract field `tool_call_id`.
    pub tool_call_id: String,
    /// Contract field `name`.
    pub name: AgentToolName,
    /// Contract field `result_ref`.
    pub result_ref: String,
    /// Contract field `result_json`.
    pub result_json: String,
    /// Contract field `result_hash`.
    pub result_hash: String,
    /// Contract field `outcome`.
    pub outcome: NativeAgentEffectResultOutcome,
    /// Contract field `operation_ref`.
    pub operation_ref: Nullable<String>,
}
impl std::fmt::Debug for NativeAgentEffectResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentEffectResult { redacted }")
    }
}
