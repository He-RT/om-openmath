//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ResultStatus`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultStatus {
    /// Contract field `type`.
    pub r#type: ResultStatusType,
    /// Contract field `request_id`.
    pub request_id: String,
}
impl std::fmt::Debug for ResultStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResultStatus { redacted }")
    }
}
