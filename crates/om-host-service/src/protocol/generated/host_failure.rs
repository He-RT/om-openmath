//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `HostFailure`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostFailure {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `error`.
    pub error: HostError,
}
impl std::fmt::Debug for HostFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HostFailure { redacted }")
    }
}
