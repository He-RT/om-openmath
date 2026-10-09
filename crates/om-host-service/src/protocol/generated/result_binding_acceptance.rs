//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `ResultBindingAcceptance`.
pub enum ResultBindingAcceptance {
    /// Contract value `accepted`.
    #[serde(rename = "accepted")]
    Accepted,
    /// Contract value `history_only`.
    #[serde(rename = "history_only")]
    HistoryOnly,
}
