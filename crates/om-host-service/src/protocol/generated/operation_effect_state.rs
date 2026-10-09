//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `OperationEffectState`.
pub enum OperationEffectState {
    /// Contract value `none`.
    #[serde(rename = "none")]
    None,
    /// Contract value `not_committed`.
    #[serde(rename = "not_committed")]
    NotCommitted,
    /// Contract value `committed`.
    #[serde(rename = "committed")]
    Committed,
    /// Contract value `partially_committed`.
    #[serde(rename = "partially_committed")]
    PartiallyCommitted,
    /// Contract value `unknown`.
    #[serde(rename = "unknown")]
    Unknown,
}
