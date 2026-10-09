//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `SaveStateState`.
pub enum SaveStateState {
    /// Contract value `unbound`.
    #[serde(rename = "unbound")]
    Unbound,
    /// Contract value `dirty`.
    #[serde(rename = "dirty")]
    Dirty,
    /// Contract value `saving`.
    #[serde(rename = "saving")]
    Saving,
    /// Contract value `saved`.
    #[serde(rename = "saved")]
    Saved,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `unknown`.
    #[serde(rename = "unknown")]
    Unknown,
}
