//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeKernelReceipt`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeKernelReceipt {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `store_id`.
    pub store_id: String,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `request_hash`.
    pub request_hash: String,
    /// Contract field `checkpoint_id`.
    pub checkpoint_id: String,
    /// Contract field `checkpoint_blob_hash`.
    pub checkpoint_blob_hash: String,
    /// Contract field `checkpoint_byte_length`.
    pub checkpoint_byte_length: Serial,
    /// Contract field `codec_version`.
    pub codec_version: u32,
    /// Contract field `producer`.
    pub producer: NativeKernelProducer,
    /// Contract field `accepted_source_revision`.
    pub accepted_source_revision: Serial,
    /// Contract field `accepted_snapshot_hash`.
    pub accepted_snapshot_hash: String,
    /// Contract field `kernel_state_revision`.
    pub kernel_state_revision: Serial,
    /// Contract field `result_id`.
    pub result_id: Nullable<String>,
    /// Contract field `outbox_event_id`.
    pub outbox_event_id: String,
    /// Contract field `committed_at`.
    pub committed_at: String,
}
impl std::fmt::Debug for NativeKernelReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeKernelReceipt { redacted }")
    }
}
