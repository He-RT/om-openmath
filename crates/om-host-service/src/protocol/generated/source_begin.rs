//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceBegin`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBegin {
    /// Contract field `type`.
    pub r#type: SourceBeginType,
    /// Contract field `preview_ref`.
    pub preview_ref: String,
}
impl std::fmt::Debug for SourceBegin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceBegin { redacted }")
    }
}
