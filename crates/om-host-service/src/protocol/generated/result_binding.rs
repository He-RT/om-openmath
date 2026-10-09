//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ResultBinding`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultBinding {
    /// Contract field `record_type`.
    pub record_type: ResultBindingRecordType,
    /// Contract field `result_id`.
    pub result_id: String,
    /// Contract field `result_ref`.
    pub result_ref: String,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `cell_id`.
    pub cell_id: String,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `source_hash`.
    pub source_hash: String,
    /// Contract field `source_revision`.
    pub source_revision: Serial,
    /// Contract field `execution_epoch`.
    pub execution_epoch: Serial,
    /// Contract field `kernel_state_revision`.
    pub kernel_state_revision: Serial,
    /// Contract field `out_index`.
    pub out_index: Serial,
    /// Contract field `view_id`.
    pub view_id: Nullable<String>,
    /// Contract field `acceptance`.
    pub acceptance: ResultBindingAcceptance,
    /// Contract field `freshness`.
    pub freshness: ResultBindingFreshness,
    /// Contract field `source_fully_available`.
    pub source_fully_available: bool,
}
impl std::fmt::Debug for ResultBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResultBinding { redacted }")
    }
}
