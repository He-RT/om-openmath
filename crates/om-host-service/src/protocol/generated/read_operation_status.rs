//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ReadOperationStatus`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadOperationStatus {
    /// Contract field `kind`.
    pub kind: ReadOperationStatusKind,
    /// Contract field `operation_ref`.
    pub operation_ref: String,
}
impl std::fmt::Debug for ReadOperationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadOperationStatus { redacted }")
    }
}
