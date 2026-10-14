//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NotebookResultCell`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotebookResultCell {
    /// Contract field `cell_id`.
    pub cell_id: String,
    /// Contract field `source_hash`.
    pub source_hash: String,
}
impl std::fmt::Debug for NotebookResultCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NotebookResultCell { redacted }")
    }
}
