//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeSourceSnapshot`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSourceSnapshot {
    /// Contract field `codec_version`.
    pub codec_version: u32,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `revision`.
    pub revision: Serial,
    /// Contract field `execution_epoch`.
    pub execution_epoch: Serial,
    /// Contract field `file`.
    pub file: NativeSourceFile,
    /// Contract field `cell_revisions`.
    pub cell_revisions: Vec<NativeCellRevision>,
    /// Contract field `snapshot_hash`.
    pub snapshot_hash: String,
}
impl std::fmt::Debug for NativeSourceSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeSourceSnapshot { redacted }")
    }
}
