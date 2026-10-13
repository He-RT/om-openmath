//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `AgentMessagePart`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentMessagePart {
    /// Variant `AgentTextPart`.
    AgentTextPart(AgentTextPart),
    /// Variant `AgentMediaPart`.
    AgentMediaPart(AgentMediaPart),
    /// Variant `AgentToolProposalPart`.
    AgentToolProposalPart(AgentToolProposalPart),
    /// Variant `AgentToolResultPart`.
    AgentToolResultPart(AgentToolResultPart),
    /// Variant `AgentStepLinkPart`.
    AgentStepLinkPart(AgentStepLinkPart),
}
