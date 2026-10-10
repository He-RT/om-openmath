//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `KernelStatus`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelStatus {
    /// Contract field `type`.
    pub r#type: KernelStatusType,
    /// Contract field `operation_id`.
    pub operation_id: String,
}
impl std::fmt::Debug for KernelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KernelStatus { redacted }")
    }
}
