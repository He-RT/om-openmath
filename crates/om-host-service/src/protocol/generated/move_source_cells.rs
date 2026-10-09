//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `MoveSourceCells`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveSourceCells {
    /// Contract field `kind`.
    pub kind: MoveSourceCellsKind,
    /// Contract field `cell_ids`.
    pub cell_ids: Vec<String>,
    /// Contract field `after_cell_id`.
    pub after_cell_id: Nullable<String>,
}
impl std::fmt::Debug for MoveSourceCells {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MoveSourceCells { redacted }")
    }
}
