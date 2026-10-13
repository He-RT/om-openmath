//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentMediaPart`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentMediaPart {
    /// Contract field `kind`.
    pub kind: AgentMediaPartKind,
    /// Contract field `part_id`.
    pub part_id: String,
    /// Contract field `media_ref`.
    pub media_ref: String,
    /// Contract field `submission_kind`.
    pub submission_kind: AgentMediaPartSubmissionKind,
}
impl std::fmt::Debug for AgentMediaPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentMediaPart { redacted }")
    }
}
