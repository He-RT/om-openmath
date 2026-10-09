//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeEditorFence`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEditorFence {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `fence_id`.
    pub fence_id: String,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `source_revision`.
    pub source_revision: Serial,
    /// Contract field `source_snapshot_hash`.
    pub source_snapshot_hash: String,
    /// Contract field `issued_ms`.
    pub issued_ms: Serial,
    /// Contract field `expires_ms`.
    pub expires_ms: Serial,
    /// Contract field `targets`.
    pub targets: Vec<NativeEditorTarget>,
}
impl std::fmt::Debug for NativeEditorFence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeEditorFence { redacted }")
    }
}
