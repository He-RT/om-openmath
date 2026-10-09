//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativePreviewDiagnosticSeverity`.
pub enum NativePreviewDiagnosticSeverity {
    /// Contract value `error`.
    #[serde(rename = "error")]
    Error,
    /// Contract value `warning`.
    #[serde(rename = "warning")]
    Warning,
    /// Contract value `hint`.
    #[serde(rename = "hint")]
    Hint,
}
