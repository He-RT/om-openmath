//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeRunEventAck`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRunEventAck {
    /// Contract field `event_id`.
    pub event_id: String,
    /// Contract field `event_sequence`.
    pub event_sequence: Serial,
    /// Contract field `receipt_hash`.
    pub receipt_hash: String,
}
impl std::fmt::Debug for NativeRunEventAck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeRunEventAck { redacted }")
    }
}
