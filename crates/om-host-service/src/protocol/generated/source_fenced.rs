//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceFenced`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFenced {
    /// Contract field `type`.
    pub r#type: SourceFencedType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `fence`.
    pub fence: NativeEditorFence,
}
impl std::fmt::Debug for SourceFenced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceFenced { redacted }")
    }
}
