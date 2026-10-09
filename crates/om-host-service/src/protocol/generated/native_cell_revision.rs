//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeCellRevision`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCellRevision {
    /// Contract field `cell_id`.
    pub cell_id: String,
    /// Contract field `revision`.
    pub revision: Serial,
}
impl std::fmt::Debug for NativeCellRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeCellRevision { redacted }")
    }
}
