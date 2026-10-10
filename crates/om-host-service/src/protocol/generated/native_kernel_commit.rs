//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeKernelCommit`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeKernelCommit {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `kind`.
    pub kind: NativeKernelCommitKind,
    /// Contract field `store_id`.
    pub store_id: String,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `request_hash`.
    pub request_hash: String,
    /// Contract field `checkpoint_id`.
    pub checkpoint_id: String,
    /// Contract field `expected_parent_checkpoint_id`.
    pub expected_parent_checkpoint_id: Nullable<String>,
    /// Contract field `expected_kernel_state_revision`.
    pub expected_kernel_state_revision: Serial,
    /// Contract field `checkpoint_blob_hash`.
    pub checkpoint_blob_hash: String,
    /// Contract field `checkpoint_byte_length`.
    pub checkpoint_byte_length: Serial,
    /// Contract field `codec_version`.
    pub codec_version: u32,
    /// Contract field `producer`.
    pub producer: NativeKernelProducer,
    /// Contract field `source`.
    pub source: NativeSourceSnapshot,
    /// Contract field `acceptance_source`.
    pub acceptance_source: NativeSourceSnapshot,
    /// Contract field `result_id`.
    pub result_id: Nullable<String>,
    /// Contract field `outbox_event_id`.
    pub outbox_event_id: String,
}
impl std::fmt::Debug for NativeKernelCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeKernelCommit { redacted }")
    }
}
