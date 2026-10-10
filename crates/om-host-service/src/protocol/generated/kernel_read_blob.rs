//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `KernelReadBlob`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelReadBlob {
    /// Contract field `type`.
    pub r#type: KernelReadBlobType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `offset`.
    pub offset: Serial,
    /// Contract field `count`.
    pub count: u32,
}
impl std::fmt::Debug for KernelReadBlob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KernelReadBlob { redacted }")
    }
}
