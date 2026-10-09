//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `PreviewRenameNotebook`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewRenameNotebook {
    /// Contract field `type`.
    pub r#type: PreviewRenameNotebookType,
    /// Contract field `title`.
    pub title: String,
}
impl std::fmt::Debug for PreviewRenameNotebook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreviewRenameNotebook { redacted }")
    }
}
