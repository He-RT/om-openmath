//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `DraftState`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftState {
    /// Contract field `cell_id`.
    pub cell_id: String,
    /// Contract field `editor_generation`.
    pub editor_generation: Serial,
    /// Contract field `draft_sequence`.
    pub draft_sequence: Serial,
    /// Contract field `base_cell_revision`.
    pub base_cell_revision: Serial,
    /// Contract field `marked_text_active`.
    pub marked_text_active: bool,
    /// Contract field `has_unconfirmed_text`.
    pub has_unconfirmed_text: bool,
    /// Contract field `pending_operation_ref`.
    pub pending_operation_ref: Nullable<String>,
}
impl std::fmt::Debug for DraftState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DraftState { redacted }")
    }
}
