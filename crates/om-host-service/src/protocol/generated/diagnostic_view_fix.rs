//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `DiagnosticViewFix`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticViewFix {
    /// Contract field `source_span`.
    pub source_span: SourceSpan,
    /// Contract field `replacement`.
    pub replacement: String,
    /// Contract field `label`.
    pub label: String,
}
impl std::fmt::Debug for DiagnosticViewFix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DiagnosticViewFix { redacted }")
    }
}
