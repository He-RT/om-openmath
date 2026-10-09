//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `PreviewDeleteCell`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewDeleteCell {
    /// Contract field `type`.
    pub r#type: PreviewDeleteCellType,
    /// Contract field `target`.
    pub target: PreviewExistingCell,
    /// Contract field `expected_source_hash`.
    pub expected_source_hash: String,
}
impl std::fmt::Debug for PreviewDeleteCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreviewDeleteCell { redacted }")
    }
}
