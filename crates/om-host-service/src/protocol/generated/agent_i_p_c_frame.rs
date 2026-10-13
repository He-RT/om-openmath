//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `AgentIPCFrame`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentIPCFrame {
    /// Variant `AgentHelloFrame`.
    AgentHelloFrame(Box<AgentHelloFrame>),
    /// Variant `AgentReadyFrame`.
    AgentReadyFrame(Box<AgentReadyFrame>),
    /// Variant `AgentUserInputFrame`.
    AgentUserInputFrame(Box<AgentUserInputFrame>),
    /// Variant `AgentPrepareRequestFrame`.
    AgentPrepareRequestFrame(Box<AgentPrepareRequestFrame>),
    /// Variant `AgentModelRequestFrame`.
    AgentModelRequestFrame(Box<AgentModelRequestFrame>),
    /// Variant `AgentModelEventFrame`.
    AgentModelEventFrame(Box<AgentModelEventFrame>),
    /// Variant `AgentToolCallFrame`.
    AgentToolCallFrame(Box<AgentToolCallFrame>),
    /// Variant `AgentToolResultFrame`.
    AgentToolResultFrame(Box<AgentToolResultFrame>),
    /// Variant `AgentTranscriptEventFrame`.
    AgentTranscriptEventFrame(Box<AgentTranscriptEventFrame>),
    /// Variant `AgentAckFrame`.
    AgentAckFrame(Box<AgentAckFrame>),
    /// Variant `AgentSteerFrame`.
    AgentSteerFrame(Box<AgentSteerFrame>),
    /// Variant `AgentFollowUpFrame`.
    AgentFollowUpFrame(Box<AgentFollowUpFrame>),
    /// Variant `AgentCancelFrame`.
    AgentCancelFrame(Box<AgentCancelFrame>),
    /// Variant `AgentSettledFrame`.
    AgentSettledFrame(Box<AgentSettledFrame>),
    /// Variant `AgentErrorFrame`.
    AgentErrorFrame(Box<AgentErrorFrame>),
}
