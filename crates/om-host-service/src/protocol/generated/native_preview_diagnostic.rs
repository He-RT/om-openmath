//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativePreviewDiagnostic`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePreviewDiagnostic {
    /// Contract field `code`.
    pub code: String,
    /// Contract field `message`.
    pub message: String,
    /// Contract field `severity`.
    pub severity: NativePreviewDiagnosticSeverity,
    /// Contract field `cell_id`.
    pub cell_id: Nullable<String>,
    /// Contract field `start_utf8`.
    pub start_utf8: Nullable<u32>,
    /// Contract field `end_utf8`.
    pub end_utf8: Nullable<u32>,
}
impl std::fmt::Debug for NativePreviewDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativePreviewDiagnostic { redacted }")
    }
}
