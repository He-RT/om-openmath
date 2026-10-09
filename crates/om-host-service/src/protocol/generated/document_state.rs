//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `DocumentState`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentState {
    /// Contract field `phase`.
    pub phase: DocumentPhase,
    /// Contract field `binding`.
    pub binding: DocumentBinding,
    /// Contract field `title`.
    pub title: String,
    /// Contract field `kernel_state_revision`.
    pub kernel_state_revision: Serial,
    /// Contract field `calculation_config_revision`.
    pub calculation_config_revision: Serial,
    /// Contract field `pending_mutation_operation_ref`.
    pub pending_mutation_operation_ref: Nullable<String>,
    /// Contract field `save`.
    pub save: SaveState,
    /// Contract field `drafts`.
    pub drafts: Vec<DraftState>,
    /// Contract field `active_checkpoint_ref`.
    pub active_checkpoint_ref: Nullable<String>,
    /// Contract field `kernel_projection_revision`.
    pub kernel_projection_revision: Serial,
}
impl std::fmt::Debug for DocumentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DocumentState { redacted }")
    }
}
