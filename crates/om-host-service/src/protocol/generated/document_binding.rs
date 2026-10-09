//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `DocumentBinding`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentBinding {
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `generation`.
    pub generation: Serial,
    /// Contract field `document_revision`.
    pub document_revision: Serial,
    /// Contract field `execution_epoch`.
    pub execution_epoch: Serial,
}
impl std::fmt::Debug for DocumentBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DocumentBinding { redacted }")
    }
}
