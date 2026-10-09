//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `DiagnosticViewSeverity`.
pub enum DiagnosticViewSeverity {
    /// Contract value `Error`.
    #[serde(rename = "Error")]
    Error,
    /// Contract value `Warning`.
    #[serde(rename = "Warning")]
    Warning,
    /// Contract value `Hint`.
    #[serde(rename = "Hint")]
    Hint,
}
