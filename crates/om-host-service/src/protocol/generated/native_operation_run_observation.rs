//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeOperationRunObservation`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOperationRunObservation {
    /// Contract field `phase`.
    pub phase: NativeOperationRunObservationPhase,
    /// Contract field `accepted_cells`.
    pub accepted_cells: Serial,
    /// Contract field `accepted_effects`.
    pub accepted_effects: Serial,
    /// Contract field `original_active_operation`.
    pub original_active_operation: Nullable<String>,
    /// Contract field `event`.
    pub event: NativeRunEventAck,
    /// Contract field `kernel_receipts`.
    pub kernel_receipts: Vec<NativeKernelReceipt>,
}
impl std::fmt::Debug for NativeOperationRunObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeOperationRunObservation { redacted }")
    }
}
