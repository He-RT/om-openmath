//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `PreviewInsertCell`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewInsertCell {
    /// Contract field `type`.
    pub r#type: PreviewInsertCellType,
    /// Contract field `client_key`.
    pub client_key: String,
    /// Contract field `after`.
    pub after: Nullable<PreviewCellSelector>,
    /// Contract field `kind`.
    pub kind: PreviewInsertCellKind,
    /// Contract field `dialect`.
    pub dialect: Dialect,
    /// Contract field `source`.
    pub source: String,
}
impl std::fmt::Debug for PreviewInsertCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreviewInsertCell { redacted }")
    }
}
