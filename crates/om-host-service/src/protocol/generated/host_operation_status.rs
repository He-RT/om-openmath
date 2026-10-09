//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `HostOperationStatus`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostOperationStatus {
    /// Contract field `operation_ref`.
    pub operation_ref: String,
    /// Contract field `phase`.
    pub phase: HostOperationStatusPhase,
    /// Contract field `result`.
    pub result: Nullable<serde_json::Value>,
    /// Contract field `error_code`.
    pub error_code: Nullable<String>,
}
impl std::fmt::Debug for HostOperationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HostOperationStatus { redacted }")
    }
}
