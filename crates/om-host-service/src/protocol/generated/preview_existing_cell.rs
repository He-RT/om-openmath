//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `PreviewExistingCell`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewExistingCell {
    /// Contract field `cell_id`.
    pub cell_id: String,
}
impl std::fmt::Debug for PreviewExistingCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreviewExistingCell { redacted }")
    }
}
