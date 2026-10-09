//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `PreviewPatchInput`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewPatchInput {
    /// Contract field `kind`.
    pub kind: PreviewPatchInputKind,
    /// Contract field `operations`.
    pub operations: Vec<PreviewPatchOperation>,
}
impl std::fmt::Debug for PreviewPatchInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreviewPatchInput { redacted }")
    }
}
