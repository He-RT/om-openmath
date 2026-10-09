//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceEditorChanged`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEditorChanged {
    /// Contract field `type`.
    pub r#type: SourceEditorChangedType,
    /// Contract field `state`.
    pub state: NativeEditorState,
}
impl std::fmt::Debug for SourceEditorChanged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceEditorChanged { redacted }")
    }
}
