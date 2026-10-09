//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `MathRenderPlanParseState`.
pub enum MathRenderPlanParseState {
    /// Contract value `pending`.
    #[serde(rename = "pending")]
    Pending,
    /// Contract value `typeset`.
    #[serde(rename = "typeset")]
    Typeset,
    /// Contract value `unsupported`.
    #[serde(rename = "unsupported")]
    Unsupported,
    /// Contract value `invalid`.
    #[serde(rename = "invalid")]
    Invalid,
    /// Contract value `budget_exceeded`.
    #[serde(rename = "budget_exceeded")]
    BudgetExceeded,
    /// Contract value `font_unavailable`.
    #[serde(rename = "font_unavailable")]
    FontUnavailable,
}
