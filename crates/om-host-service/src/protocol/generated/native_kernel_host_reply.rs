//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeKernelHostReply`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeKernelHostReply {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `phase`.
    pub phase: NativeKernelHostReplyPhase,
    /// Contract field `operation_id`.
    pub operation_id: Nullable<String>,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `source_revision`.
    pub source_revision: Serial,
    /// Contract field `execution_epoch`.
    pub execution_epoch: Serial,
    /// Contract field `kernel_build`.
    pub kernel_build: String,
    /// Contract field `active_checkpoint_id`.
    pub active_checkpoint_id: Nullable<String>,
    /// Contract field `kernel_state_revision`.
    pub kernel_state_revision: Serial,
    /// Contract field `kernel_projection_revision`.
    pub kernel_projection_revision: Nullable<Serial>,
    /// Contract field `definitions_current`.
    pub definitions_current: bool,
    /// Contract field `pending_document_write`.
    pub pending_document_write: bool,
    /// Contract field `plan`.
    pub plan: Nullable<NativeKernelCommit>,
    /// Contract field `receipt`.
    pub receipt: Nullable<NativeKernelReceipt>,
    /// Contract field `error_code`.
    pub error_code: Nullable<String>,
    /// Contract field `blob_offset`.
    pub blob_offset: Nullable<Serial>,
    /// Contract field `blob_bytes`.
    pub blob_bytes: Vec<u32>,
}
impl std::fmt::Debug for NativeKernelHostReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeKernelHostReply { redacted }")
    }
}
