//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentConfirm`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentConfirm {
    /// Contract field `type`.
    pub r#type: NativeAgentConfirmType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `message_id`.
    pub message_id: String,
    /// Contract field `event_id`.
    pub event_id: String,
    /// Contract field `canonical_hash`.
    pub canonical_hash: String,
    /// Contract field `event_sequence`.
    pub event_sequence: Serial,
    /// Contract field `receipt_hash`.
    pub receipt_hash: String,
}
impl std::fmt::Debug for NativeAgentConfirm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentConfirm { redacted }")
    }
}
