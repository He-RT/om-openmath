//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `AnalysisRequestKind`.
pub enum AnalysisRequestKind {
    /// Contract value `preview`.
    #[serde(rename = "preview")]
    Preview,
    /// Contract value `complete`.
    #[serde(rename = "complete")]
    Complete,
    /// Contract value `hover`.
    #[serde(rename = "hover")]
    Hover,
}
