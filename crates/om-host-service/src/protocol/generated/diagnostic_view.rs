//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `DiagnosticView`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticView {
    /// Contract field `record_type`.
    pub record_type: DiagnosticViewRecordType,
    /// Contract field `diagnostic_id`.
    pub diagnostic_id: String,
    /// Contract field `key`.
    pub key: EditorKey,
    /// Contract field `code`.
    pub code: String,
    /// Contract field `severity`.
    pub severity: DiagnosticViewSeverity,
    /// Contract field `message`.
    pub message: String,
    /// Contract field `source_span`.
    pub source_span: SourceSpan,
    /// Contract field `display_range_utf16`.
    pub display_range_utf16: Nullable<NativeRange>,
    /// Contract field `location_state`.
    pub location_state: DiagnosticViewLocationState,
    /// Contract field `fix`.
    pub fix: Nullable<DiagnosticViewFix>,
}
impl std::fmt::Debug for DiagnosticView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DiagnosticView { redacted }")
    }
}
