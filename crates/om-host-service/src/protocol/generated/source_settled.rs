//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceSettled`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSettled {
    /// Contract field `type`.
    pub r#type: SourceSettledType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `cancelled`.
    pub cancelled: bool,
}
impl std::fmt::Debug for SourceSettled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceSettled { redacted }")
    }
}
