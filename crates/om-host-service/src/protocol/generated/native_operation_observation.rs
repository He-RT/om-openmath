//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeOperationObservation`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOperationObservation {
    /// Contract field `operation_ref`.
    pub operation_ref: String,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `store_id`.
    pub store_id: String,
    /// Contract field `source_receipt`.
    pub source_receipt: Nullable<NativeDurableSourceReceipt>,
    /// Contract field `kernel_receipt`.
    pub kernel_receipt: Nullable<NativeKernelReceipt>,
    /// Contract field `run`.
    pub run: Nullable<NativeOperationRunObservation>,
    /// Contract field `claim`.
    pub claim: Nullable<NativeOperationClaimObservation>,
}
impl std::fmt::Debug for NativeOperationObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeOperationObservation { redacted }")
    }
}
