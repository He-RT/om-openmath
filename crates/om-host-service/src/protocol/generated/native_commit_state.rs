//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeCommitState`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCommitState {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `request_hash`.
    pub request_hash: String,
    /// Contract field `phase`.
    pub phase: NativeCommitStatePhase,
    /// Contract field `cancel_requested`.
    pub cancel_requested: bool,
    /// Contract field `receipt`.
    pub receipt: Nullable<NativeDurableSourceReceipt>,
}
impl std::fmt::Debug for NativeCommitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeCommitState { redacted }")
    }
}
