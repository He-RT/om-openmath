//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `PreviewReplaceText`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewReplaceText {
    /// Contract field `type`.
    pub r#type: PreviewReplaceTextType,
    /// Contract field `target`.
    pub target: PreviewExistingCell,
    /// Contract field `expected_source_hash`.
    pub expected_source_hash: String,
    /// Contract field `match`.
    pub r#match: String,
    /// Contract field `replacement`.
    pub replacement: String,
}
impl std::fmt::Debug for PreviewReplaceText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreviewReplaceText { redacted }")
    }
}
