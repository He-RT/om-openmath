//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AdmissionReceipt`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionReceipt {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `request_ref`.
    pub request_ref: String,
    /// Contract field `operation_ref`.
    pub operation_ref: Nullable<String>,
    /// Contract field `accepted`.
    pub accepted: bool,
    /// Contract field `error`.
    pub error: Nullable<HostError>,
}
impl std::fmt::Debug for AdmissionReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AdmissionReceipt { redacted }")
    }
}
