//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `KernelRestoreFinish`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelRestoreFinish {
    /// Contract field `type`.
    pub r#type: KernelRestoreFinishType,
    /// Contract field `operation_id`.
    pub operation_id: String,
}
impl std::fmt::Debug for KernelRestoreFinish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KernelRestoreFinish { redacted }")
    }
}
