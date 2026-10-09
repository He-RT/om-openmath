//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `EventBatch`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventBatch {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `events`.
    pub events: Vec<EventEnvelope>,
    /// Contract field `needs_resync`.
    pub needs_resync: bool,
}
impl std::fmt::Debug for EventBatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EventBatch { redacted }")
    }
}
