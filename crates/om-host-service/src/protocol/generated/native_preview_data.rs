//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativePreviewData`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePreviewData {
    /// Contract field `preview_kind`.
    pub preview_kind: NativePreviewDataPreviewKind,
    /// Contract field `valid`.
    pub valid: bool,
    /// Contract field `preview_ref`.
    pub preview_ref: Nullable<String>,
    /// Contract field `plan_hash`.
    pub plan_hash: String,
    /// Contract field `diagnostics`.
    pub diagnostics: Vec<NativePreviewDiagnostic>,
    /// Contract field `affected_cell_ids`.
    pub affected_cell_ids: Vec<String>,
    /// Contract field `assigned_cell_ids`.
    pub assigned_cell_ids: std::collections::BTreeMap<String, String>,
}
impl std::fmt::Debug for NativePreviewData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativePreviewData { redacted }")
    }
}
