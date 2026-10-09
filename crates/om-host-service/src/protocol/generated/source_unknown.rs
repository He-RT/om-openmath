//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceUnknown`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceUnknown {
    /// Contract field `type`.
    pub r#type: SourceUnknownType,
    /// Contract field `operation_id`.
    pub operation_id: String,
}
impl std::fmt::Debug for SourceUnknown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceUnknown { redacted }")
    }
}
