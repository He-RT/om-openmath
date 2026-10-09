//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeCalculationSettingsDialect`.
pub enum NativeCalculationSettingsDialect {
    /// Contract value `auto`.
    #[serde(rename = "auto")]
    Auto,
    /// Contract value `modern`.
    #[serde(rename = "modern")]
    Modern,
    /// Contract value `wolfram`.
    #[serde(rename = "wolfram")]
    Wolfram,
}
