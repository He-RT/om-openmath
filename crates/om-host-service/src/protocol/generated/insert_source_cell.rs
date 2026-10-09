//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InsertSourceCell`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InsertSourceCell {
    /// Contract field `kind`.
    pub kind: InsertSourceCellKind,
    /// Contract field `cell`.
    pub cell: NativeSourceCell,
    /// Contract field `after_cell_id`.
    pub after_cell_id: Nullable<String>,
}
impl std::fmt::Debug for InsertSourceCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InsertSourceCell { redacted }")
    }
}
