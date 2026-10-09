//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SaveState`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveState {
    /// Contract field `state`.
    pub state: SaveStateState,
    /// Contract field `save_operation_ref`.
    pub save_operation_ref: Nullable<String>,
    /// Contract field `file_binding_revision`.
    pub file_binding_revision: Serial,
    /// Contract field `saved_document_revision`.
    pub saved_document_revision: Nullable<Serial>,
    /// Contract field `in_flight_revision`.
    pub in_flight_revision: Nullable<Serial>,
    /// Contract field `snapshot_hash`.
    pub snapshot_hash: Nullable<String>,
}
impl std::fmt::Debug for SaveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SaveState { redacted }")
    }
}
