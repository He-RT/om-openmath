//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentRequestIntent`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentRequestIntent {
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `agent_turn_id`.
    pub agent_turn_id: String,
    /// Contract field `request_id`.
    pub request_id: String,
}
impl std::fmt::Debug for NativeAgentRequestIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentRequestIntent { redacted }")
    }
}
