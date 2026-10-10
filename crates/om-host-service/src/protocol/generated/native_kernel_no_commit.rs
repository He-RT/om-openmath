//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeKernelNoCommit`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeKernelNoCommit {
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
    /// Contract field `accepted_source_revision`.
    pub accepted_source_revision: Serial,
    /// Contract field `accepted_snapshot_hash`.
    pub accepted_snapshot_hash: String,
    /// Contract field `expected_parent_checkpoint_id`.
    pub expected_parent_checkpoint_id: Nullable<String>,
    /// Contract field `expected_kernel_state_revision`.
    pub expected_kernel_state_revision: Serial,
}
impl std::fmt::Debug for NativeKernelNoCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeKernelNoCommit { redacted }")
    }
}
