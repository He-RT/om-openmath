//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `PreviewUpdateCellKind`.
pub enum PreviewUpdateCellKind {
    /// Contract value `Math`.
    #[serde(rename = "Math")]
    Math,
    /// Contract value `Text`.
    #[serde(rename = "Text")]
    Text,
}
