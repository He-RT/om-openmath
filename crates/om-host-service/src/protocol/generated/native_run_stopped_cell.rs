//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeRunStoppedCell`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRunStoppedCell {
    /// Contract field `cell_id`.
    pub cell_id: String,
    /// Contract field `operation_id`.
    pub operation_id: Nullable<String>,
    /// Contract field `code`.
    pub code: String,
}
impl std::fmt::Debug for NativeRunStoppedCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeRunStoppedCell { redacted }")
    }
}
