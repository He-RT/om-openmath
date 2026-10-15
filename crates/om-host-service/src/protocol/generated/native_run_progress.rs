//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeRunProgress`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRunProgress {
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `phase`.
    pub phase: NativeRunProgressPhase,
    /// Contract field `descriptor`.
    pub descriptor: Nullable<NativeRunDescriptor>,
    /// Contract field `next_child_index`.
    pub next_child_index: Nullable<Serial>,
    /// Contract field `next_operation_id`.
    pub next_operation_id: Nullable<String>,
    /// Contract field `child`.
    pub child: Nullable<NativeKernelHostReply>,
    /// Contract field `accepted_cell_ids`.
    pub accepted_cell_ids: Vec<String>,
    /// Contract field `stopped_cells`.
    pub stopped_cells: Vec<NativeRunStoppedCell>,
    /// Contract field `stop_code`.
    pub stop_code: Nullable<String>,
    /// Contract field `outcome_was_unknown`.
    pub outcome_was_unknown: bool,
}
impl std::fmt::Debug for NativeRunProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeRunProgress { redacted }")
    }
}
