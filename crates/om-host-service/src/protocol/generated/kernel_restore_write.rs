//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `KernelRestoreWrite`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelRestoreWrite {
    /// Contract field `type`.
    pub r#type: KernelRestoreWriteType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `offset`.
    pub offset: Serial,
    /// Contract field `bytes`.
    pub bytes: Vec<u32>,
}
impl std::fmt::Debug for KernelRestoreWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KernelRestoreWrite { redacted }")
    }
}
