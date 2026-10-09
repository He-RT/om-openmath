//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `IOAck`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IOAck {
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `document_binding`.
    pub document_binding: Nullable<DocumentBinding>,
    /// Contract field `operation_ref`.
    pub operation_ref: String,
    /// Contract field `io_request_ref`.
    pub io_request_ref: String,
    /// Contract field `outcome`.
    pub outcome: IOAckOutcome,
    /// Contract field `receipt_ref`.
    pub receipt_ref: Nullable<String>,
    /// Contract field `receipt_hash`.
    pub receipt_hash: Nullable<String>,
    /// Contract field `error`.
    pub error: Nullable<HostError>,
}
impl std::fmt::Debug for IOAck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IOAck { redacted }")
    }
}
