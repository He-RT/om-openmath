//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `KernelRestoreStart`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelRestoreStart {
    /// Contract field `type`.
    pub r#type: KernelRestoreStartType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `expected_source_hash`.
    pub expected_source_hash: String,
    /// Contract field `byte_length`.
    pub byte_length: Serial,
    /// Contract field `payload_hash`.
    pub payload_hash: String,
}
impl std::fmt::Debug for KernelRestoreStart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KernelRestoreStart { redacted }")
    }
}
