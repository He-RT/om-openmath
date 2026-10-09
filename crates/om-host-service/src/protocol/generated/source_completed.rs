//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceCompleted`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCompleted {
    /// Contract field `type`.
    pub r#type: SourceCompletedType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `receipt`.
    pub receipt: NativeDurableSourceReceipt,
}
impl std::fmt::Debug for SourceCompleted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceCompleted { redacted }")
    }
}
