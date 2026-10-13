//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentReceive`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentReceive {
    /// Contract field `type`.
    pub r#type: NativeAgentReceiveType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `frame_json`.
    pub frame_json: String,
}
impl std::fmt::Debug for NativeAgentReceive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentReceive { redacted }")
    }
}
