//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeDurableSourceReceipt`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeDurableSourceReceipt {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `receipt`.
    pub receipt: OperationReceipt,
    /// Contract field `snapshot_hash`.
    pub snapshot_hash: String,
    /// Contract field `inverse_plan_hash`.
    pub inverse_plan_hash: String,
    /// Contract field `execution_epoch`.
    pub execution_epoch: Serial,
    /// Contract field `outbox_event_id`.
    pub outbox_event_id: String,
}
impl std::fmt::Debug for NativeDurableSourceReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeDurableSourceReceipt { redacted }")
    }
}
