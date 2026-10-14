//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeAgentReadonlyResultOutcome`.
pub enum NativeAgentReadonlyResultOutcome {
    /// Contract value `completed`.
    #[serde(rename = "completed")]
    Completed,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
}
