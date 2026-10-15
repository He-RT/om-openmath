//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentPollRun`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentPollRun {
    /// Contract field `type`.
    pub r#type: NativeAgentPollRunType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `operation_id`.
    pub operation_id: String,
}
impl std::fmt::Debug for NativeAgentPollRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentPollRun { redacted }")
    }
}
