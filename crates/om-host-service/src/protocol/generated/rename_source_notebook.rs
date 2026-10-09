//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `RenameSourceNotebook`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenameSourceNotebook {
    /// Contract field `kind`.
    pub kind: RenameSourceNotebookKind,
    /// Contract field `title`.
    pub title: String,
}
impl std::fmt::Debug for RenameSourceNotebook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RenameSourceNotebook { redacted }")
    }
}
