//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `AgentTaskStateMode`.
pub enum AgentTaskStateMode {
    /// Contract value `discuss`.
    #[serde(rename = "discuss")]
    Discuss,
    /// Contract value `execute`.
    #[serde(rename = "execute")]
    Execute,
}
