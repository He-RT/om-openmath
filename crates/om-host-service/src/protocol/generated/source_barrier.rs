//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceBarrier`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBarrier {
    /// Contract field `type`.
    pub r#type: SourceBarrierType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `fence_id`.
    pub fence_id: String,
}
impl std::fmt::Debug for SourceBarrier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceBarrier { redacted }")
    }
}
