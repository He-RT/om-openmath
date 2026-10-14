//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ResultNotebookReferences`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultNotebookReferences {
    /// Contract field `type`.
    pub r#type: ResultNotebookReferencesType,
    /// Contract field `request_id`.
    pub request_id: String,
    /// Contract field `kernel_state_revision`.
    pub kernel_state_revision: Serial,
    /// Contract field `cells`.
    pub cells: Vec<NotebookResultCell>,
}
impl std::fmt::Debug for ResultNotebookReferences {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResultNotebookReferences { redacted }")
    }
}
