//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceSnapshotRef`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSnapshotRef {
    /// Contract field `type`.
    pub r#type: SourceSnapshotRefType,
    /// Contract field `complete_cell_ids`.
    pub complete_cell_ids: Vec<String>,
}
impl std::fmt::Debug for SourceSnapshotRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceSnapshotRef { redacted }")
    }
}
