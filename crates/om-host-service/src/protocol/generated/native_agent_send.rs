//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentSend`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentSend {
    /// Contract field `type`.
    pub r#type: NativeAgentSendType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `frame_json`.
    pub frame_json: String,
}
impl std::fmt::Debug for NativeAgentSend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentSend { redacted }")
    }
}
