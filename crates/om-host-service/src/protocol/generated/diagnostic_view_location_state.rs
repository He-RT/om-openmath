//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `DiagnosticViewLocationState`.
pub enum DiagnosticViewLocationState {
    /// Contract value `located`.
    #[serde(rename = "located")]
    Located,
    /// Contract value `invalid_span`.
    #[serde(rename = "invalid_span")]
    InvalidSpan,
}
