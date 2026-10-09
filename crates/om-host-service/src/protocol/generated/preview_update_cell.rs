//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `PreviewUpdateCell`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewUpdateCell {
    /// Contract field `type`.
    pub r#type: PreviewUpdateCellType,
    /// Contract field `target`.
    pub target: PreviewExistingCell,
    /// Contract field `expected_source_hash`.
    pub expected_source_hash: String,
    /// Contract field `source`.
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Contract field `kind`.
    pub kind: Option<PreviewUpdateCellKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Contract field `dialect`.
    pub dialect: Option<Dialect>,
}
impl std::fmt::Debug for PreviewUpdateCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreviewUpdateCell { redacted }")
    }
}
