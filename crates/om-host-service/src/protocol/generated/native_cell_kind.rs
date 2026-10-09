//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeCellKind`.
pub enum NativeCellKind {
    /// Contract value `Math`.
    #[serde(rename = "Math")]
    Math,
    /// Contract value `Text`.
    #[serde(rename = "Text")]
    Text,
    /// Contract value `Ask`.
    #[serde(rename = "Ask")]
    Ask,
}
