//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `Dialect`.
pub enum Dialect {
    /// Contract value `Modern`.
    #[serde(rename = "Modern")]
    Modern,
    /// Contract value `Wolfram`.
    #[serde(rename = "Wolfram")]
    Wolfram,
    /// Contract value `Auto`.
    #[serde(rename = "Auto")]
    Auto,
}
