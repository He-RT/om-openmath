//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentStepLinkPart`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentStepLinkPart {
    /// Contract field `kind`.
    pub kind: AgentStepLinkPartKind,
    /// Contract field `part_id`.
    pub part_id: String,
    /// Contract field `result_ref`.
    pub result_ref: String,
    /// Contract field `step_id`.
    pub step_id: String,
}
impl std::fmt::Debug for AgentStepLinkPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentStepLinkPart { redacted }")
    }
}
