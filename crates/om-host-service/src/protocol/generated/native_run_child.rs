//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeRunChild`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRunChild {
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `cell_id`.
    pub cell_id: Nullable<String>,
    /// Contract field `cell_source_hash`.
    pub cell_source_hash: Nullable<String>,
}
impl std::fmt::Debug for NativeRunChild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeRunChild { redacted }")
    }
}
