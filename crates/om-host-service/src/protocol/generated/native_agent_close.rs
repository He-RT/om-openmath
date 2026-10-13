//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentClose`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentClose {
    /// Contract field `type`.
    pub r#type: NativeAgentCloseType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
}
impl std::fmt::Debug for NativeAgentClose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentClose { redacted }")
    }
}
