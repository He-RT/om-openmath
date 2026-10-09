//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `PreviewMoveCell`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewMoveCell {
    /// Contract field `type`.
    pub r#type: PreviewMoveCellType,
    /// Contract field `target`.
    pub target: PreviewCellSelector,
    /// Contract field `after`.
    pub after: Nullable<PreviewCellSelector>,
}
impl std::fmt::Debug for PreviewMoveCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreviewMoveCell { redacted }")
    }
}
