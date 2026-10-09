//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ProducerBinding`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerBinding {
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `generation`.
    pub generation: Serial,
    /// Contract field `source_document_revision`.
    pub source_document_revision: Serial,
    /// Contract field `execution_epoch`.
    pub execution_epoch: Serial,
    /// Contract field `calculation_config_revision`.
    pub calculation_config_revision: Serial,
    /// Contract field `kernel_base_revision`.
    pub kernel_base_revision: Serial,
    /// Contract field `cell_id`.
    pub cell_id: String,
    /// Contract field `source_hash`.
    pub source_hash: String,
}
impl std::fmt::Debug for ProducerBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProducerBinding { redacted }")
    }
}
