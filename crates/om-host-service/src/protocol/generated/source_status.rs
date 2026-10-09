//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceStatus`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceStatus {
    /// Contract field `type`.
    pub r#type: SourceStatusType,
    /// Contract field `operation_id`.
    pub operation_id: String,
}
impl std::fmt::Debug for SourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceStatus { redacted }")
    }
}
