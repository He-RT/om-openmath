//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `DocumentCommitActor`.
pub enum DocumentCommitActor {
    /// Contract value `manual`.
    #[serde(rename = "manual")]
    Manual,
    /// Contract value `agent`.
    #[serde(rename = "agent")]
    Agent,
    /// Contract value `undo`.
    #[serde(rename = "undo")]
    Undo,
    /// Contract value `external_merge`.
    #[serde(rename = "external_merge")]
    ExternalMerge,
}
