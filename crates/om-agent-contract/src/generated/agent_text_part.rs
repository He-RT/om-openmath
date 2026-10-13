//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentTextPart`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTextPart {
    /// Contract field `kind`.
    pub kind: AgentTextPartKind,
    /// Contract field `part_id`.
    pub part_id: String,
    /// Contract field `text`.
    pub text: String,
}
impl std::fmt::Debug for AgentTextPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentTextPart { redacted }")
    }
}
