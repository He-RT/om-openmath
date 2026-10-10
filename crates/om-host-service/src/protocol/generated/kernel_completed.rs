//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `KernelCompleted`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelCompleted {
    /// Contract field `type`.
    pub r#type: KernelCompletedType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `receipt`.
    pub receipt: NativeKernelReceipt,
}
impl std::fmt::Debug for KernelCompleted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KernelCompleted { redacted }")
    }
}
