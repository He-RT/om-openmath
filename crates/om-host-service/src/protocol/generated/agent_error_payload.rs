//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AgentErrorPayload`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentErrorPayload {
    /// Contract field `code`.
    pub code: String,
    /// Contract field `recoverable`.
    pub recoverable: bool,
}
impl std::fmt::Debug for AgentErrorPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentErrorPayload { redacted }")
    }
}
