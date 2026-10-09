//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `HostSnapshotQuery`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostSnapshotQuery {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `operation_refs`.
    pub operation_refs: Vec<String>,
    /// Contract field `include_results`.
    pub include_results: bool,
}
impl std::fmt::Debug for HostSnapshotQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HostSnapshotQuery { redacted }")
    }
}
