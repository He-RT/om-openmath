//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `AgentCanonicalMessageRole`.
pub enum AgentCanonicalMessageRole {
    /// Contract value `system`.
    #[serde(rename = "system")]
    System,
    /// Contract value `user`.
    #[serde(rename = "user")]
    User,
    /// Contract value `assistant`.
    #[serde(rename = "assistant")]
    Assistant,
    /// Contract value `tool`.
    #[serde(rename = "tool")]
    Tool,
}
