//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `HostInit`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostInit {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `max_pending_operations`.
    pub max_pending_operations: u32,
    /// Contract field `event_capacity`.
    pub event_capacity: u32,
}
impl std::fmt::Debug for HostInit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HostInit { redacted }")
    }
}
