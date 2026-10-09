//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `HostSnapshot`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostSnapshot {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `host_phase`.
    pub host_phase: HostPhase,
    /// Contract field `rust_event_sequence`.
    pub rust_event_sequence: Serial,
    /// Contract field `document_binding`.
    pub document_binding: Nullable<DocumentBinding>,
    /// Contract field `document_storage_ready`.
    pub document_storage_ready: bool,
    /// Contract field `operation_history_complete`.
    pub operation_history_complete: bool,
    /// Contract field `operations`.
    pub operations: Vec<HostOperationStatus>,
    /// Contract field `unavailable_operation_refs`.
    pub unavailable_operation_refs: Vec<String>,
}
impl std::fmt::Debug for HostSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HostSnapshot { redacted }")
    }
}
