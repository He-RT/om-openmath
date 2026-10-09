//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `UpdateSourceCell`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateSourceCell {
    /// Contract field `kind`.
    pub kind: UpdateSourceCellKind,
    /// Contract field `cell`.
    pub cell: NativeSourceCell,
}
impl std::fmt::Debug for UpdateSourceCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UpdateSourceCell { redacted }")
    }
}
