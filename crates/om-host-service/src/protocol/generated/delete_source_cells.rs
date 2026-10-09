//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `DeleteSourceCells`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteSourceCells {
    /// Contract field `kind`.
    pub kind: DeleteSourceCellsKind,
    /// Contract field `cell_ids`.
    pub cell_ids: Vec<String>,
}
impl std::fmt::Debug for DeleteSourceCells {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeleteSourceCells { redacted }")
    }
}
