//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `RequestEnvelope`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `request_ref`.
    pub request_ref: String,
    /// Contract field `operation_id`.
    pub operation_id: Nullable<String>,
    /// Contract field `document_binding`.
    pub document_binding: Nullable<DocumentBinding>,
    /// Contract field `task_binding`.
    pub task_binding: Nullable<InvocationBinding>,
    /// Contract field `body`.
    pub body: HostRequestBody,
}
impl std::fmt::Debug for RequestEnvelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RequestEnvelope { redacted }")
    }
}
