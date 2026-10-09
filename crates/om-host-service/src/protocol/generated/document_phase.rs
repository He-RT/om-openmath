//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `DocumentPhase`.
pub enum DocumentPhase {
    /// Contract value `opening`.
    #[serde(rename = "opening")]
    Opening,
    /// Contract value `open`.
    #[serde(rename = "open")]
    Open,
    /// Contract value `switching`.
    #[serde(rename = "switching")]
    Switching,
    /// Contract value `closing`.
    #[serde(rename = "closing")]
    Closing,
}
