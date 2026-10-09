//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `PiState`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PiState {
    /// Contract field `phase`.
    pub phase: PiPhase,
    /// Contract field `connection_id`.
    pub connection_id: Nullable<String>,
    /// Contract field `connection_generation`.
    pub connection_generation: Serial,
    /// Contract field `protocol_version`.
    pub protocol_version: Nullable<u32>,
}
impl std::fmt::Debug for PiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PiState { redacted }")
    }
}
