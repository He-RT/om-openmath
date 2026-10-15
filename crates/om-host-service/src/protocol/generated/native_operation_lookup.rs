//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeOperationLookup`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOperationLookup {
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `arguments_hash`.
    pub arguments_hash: String,
    /// Contract field `operation_ref`.
    pub operation_ref: String,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `store_id`.
    pub store_id: String,
}
impl std::fmt::Debug for NativeOperationLookup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeOperationLookup { redacted }")
    }
}
