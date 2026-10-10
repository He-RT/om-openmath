//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeResultHostReply`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeResultHostReply {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `request_id`.
    pub request_id: String,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `document_revision`.
    pub document_revision: Serial,
    /// Contract field `execution_epoch`.
    pub execution_epoch: Serial,
    /// Contract field `kernel_state_revision`.
    pub kernel_state_revision: Serial,
    /// Contract field `phase`.
    pub phase: NativeResultHostReplyPhase,
    /// Contract field `binding`.
    pub binding: Nullable<ResultBinding>,
    /// Contract field `payload`.
    pub payload: serde_json::Value,
    /// Contract field `error_code`.
    pub error_code: Nullable<String>,
}
impl std::fmt::Debug for NativeResultHostReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeResultHostReply { redacted }")
    }
}
