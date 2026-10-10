//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeKernelProducer`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeKernelProducer {
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `source_revision`.
    pub source_revision: Serial,
    /// Contract field `execution_epoch`.
    pub execution_epoch: Serial,
    /// Contract field `kernel_state_revision`.
    pub kernel_state_revision: Serial,
    /// Contract field `source_snapshot_hash`.
    pub source_snapshot_hash: String,
    /// Contract field `kernel_build`.
    pub kernel_build: String,
    /// Contract field `config_revision`.
    pub config_revision: Serial,
    /// Contract field `calculation`.
    pub calculation: NativeCalculationSettings,
    /// Contract field `general_hash`.
    pub general_hash: String,
    /// Contract field `cell_id`.
    pub cell_id: Nullable<String>,
    /// Contract field `cell_source_hash`.
    pub cell_source_hash: Nullable<String>,
    /// Contract field `terminal_status`.
    pub terminal_status: NativeKernelTerminalStatus,
    /// Contract field `successful_statements`.
    pub successful_statements: Serial,
    /// Contract field `out_index`.
    pub out_index: Nullable<Serial>,
}
impl std::fmt::Debug for NativeKernelProducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeKernelProducer { redacted }")
    }
}
