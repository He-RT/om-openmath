//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeRunDescriptor`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRunDescriptor {
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `arguments_hash`.
    pub arguments_hash: String,
    /// Contract field `source_snapshot_hash`.
    pub source_snapshot_hash: String,
    /// Contract field `run_scope`.
    pub run_scope: NativeRunDescriptorRunScope,
    /// Contract field `requested_cell_ids`.
    pub requested_cell_ids: Vec<String>,
    /// Contract field `children`.
    pub children: Vec<NativeRunChild>,
    /// Contract field `blocked_cells`.
    pub blocked_cells: Vec<NativeRunStoppedCell>,
    /// Contract field `continue_on_error`.
    pub continue_on_error: bool,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `source_revision`.
    pub source_revision: Serial,
    /// Contract field `execution_epoch`.
    pub execution_epoch: Serial,
    /// Contract field `config_revision`.
    pub config_revision: Serial,
    /// Contract field `kernel_build`.
    pub kernel_build: String,
    /// Contract field `general_hash`.
    pub general_hash: String,
    /// Contract field `initial_checkpoint_id`.
    pub initial_checkpoint_id: Nullable<String>,
    /// Contract field `initial_checkpoint_hash`.
    pub initial_checkpoint_hash: Nullable<String>,
    /// Contract field `initial_kernel_revision`.
    pub initial_kernel_revision: Nullable<Serial>,
}
impl std::fmt::Debug for NativeRunDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeRunDescriptor { redacted }")
    }
}
