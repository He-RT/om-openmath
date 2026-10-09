//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeUndoGroup`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeUndoGroup {
    /// Contract field `group_id`.
    pub group_id: String,
    /// Contract field `transaction_ids`.
    pub transaction_ids: Vec<String>,
}
impl std::fmt::Debug for NativeUndoGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeUndoGroup { redacted }")
    }
}
