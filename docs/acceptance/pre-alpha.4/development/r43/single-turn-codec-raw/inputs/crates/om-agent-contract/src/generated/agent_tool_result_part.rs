//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentToolResultPart`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentToolResultPart {
    /// Contract field `kind`.
    pub kind: AgentToolResultPartKind,
    /// Contract field `part_id`.
    pub part_id: String,
    /// Contract field `tool_call_id`.
    pub tool_call_id: String,
    /// Contract field `operation_ref`.
    pub operation_ref: Nullable<String>,
    /// Contract field `result_ref`.
    pub result_ref: Nullable<String>,
    /// Contract field `outcome`.
    pub outcome: AgentToolResultPartOutcome,
}
impl std::fmt::Debug for AgentToolResultPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentToolResultPart { redacted }")
    }
}
