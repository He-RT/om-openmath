//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ResultRevoke`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultRevoke {
    /// Contract field `type`.
    pub r#type: ResultRevokeType,
    /// Contract field `request_id`.
    pub request_id: String,
    /// Contract field `result_ref`.
    pub result_ref: String,
}
impl std::fmt::Debug for ResultRevoke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResultRevoke { redacted }")
    }
}
