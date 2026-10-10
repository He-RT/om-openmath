//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `KernelSettled`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelSettled {
    /// Contract field `type`.
    pub r#type: KernelSettledType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `proof`.
    pub proof: NativeKernelNoCommit,
}
impl std::fmt::Debug for KernelSettled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KernelSettled { redacted }")
    }
}
