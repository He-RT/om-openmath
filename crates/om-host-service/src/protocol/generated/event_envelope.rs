//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `EventEnvelope`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `rust_event_sequence`.
    pub rust_event_sequence: Serial,
    /// Contract field `request_ref`.
    pub request_ref: Nullable<String>,
    /// Contract field `operation_ref`.
    pub operation_ref: Nullable<String>,
    /// Contract field `document_binding`.
    pub document_binding: Nullable<DocumentBinding>,
    /// Contract field `event_kind`.
    pub event_kind: EventEnvelopeEventKind,
    /// Contract field `payload`.
    pub payload: serde_json::Value,
}
impl std::fmt::Debug for EventEnvelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EventEnvelope { redacted }")
    }
}
