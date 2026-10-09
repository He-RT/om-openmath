//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `MathRenderPlanStyle`.
pub enum MathRenderPlanStyle {
    /// Contract value `inline`.
    #[serde(rename = "inline")]
    Inline,
    /// Contract value `display`.
    #[serde(rename = "display")]
    Display,
}
