//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentToolProposalPart`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentToolProposalPart {
    /// Contract field `kind`.
    pub kind: AgentToolProposalPartKind,
    /// Contract field `part_id`.
    pub part_id: String,
    /// Contract field `tool_call_id`.
    pub tool_call_id: String,
    /// Contract field `name`.
    pub name: AgentToolName,
    /// Contract field `arguments_json`.
    pub arguments_json: String,
}
impl std::fmt::Debug for AgentToolProposalPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentToolProposalPart { redacted }")
    }
}
