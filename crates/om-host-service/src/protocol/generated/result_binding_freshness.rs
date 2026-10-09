//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `ResultBindingFreshness`.
pub enum ResultBindingFreshness {
    /// Contract value `current`.
    #[serde(rename = "current")]
    Current,
    /// Contract value `stale`.
    #[serde(rename = "stale")]
    Stale,
    /// Contract value `refreshing`.
    #[serde(rename = "refreshing")]
    Refreshing,
    /// Contract value `partial`.
    #[serde(rename = "partial")]
    Partial,
}
