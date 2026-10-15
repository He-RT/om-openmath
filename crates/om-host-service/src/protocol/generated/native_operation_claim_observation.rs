//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeOperationClaimObservation`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOperationClaimObservation {
    /// Contract field `phase`.
    pub phase: NativeOperationClaimObservationPhase,
    /// Contract field `event`.
    pub event: NativeRunEventAck,
}
impl std::fmt::Debug for NativeOperationClaimObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeOperationClaimObservation { redacted }")
    }
}
