//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `HostError`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostError {
    /// Contract field `code`.
    pub code: HostErrorCode,
    /// Contract field `message`.
    pub message: String,
    /// Contract field `field_path`.
    pub field_path: Nullable<String>,
    /// Contract field `operation_ref`.
    pub operation_ref: Nullable<String>,
    /// Contract field `outcome_known`.
    pub outcome_known: bool,
    /// Contract field `retryable`.
    pub retryable: bool,
}
impl std::fmt::Debug for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HostError { redacted }")
    }
}
