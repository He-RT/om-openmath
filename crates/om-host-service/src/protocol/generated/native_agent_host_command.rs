//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `NativeAgentHostCommand`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NativeAgentHostCommand {
    /// Variant `NativeAgentActivate`.
    NativeAgentActivate(NativeAgentActivate),
    /// Variant `NativeAgentReceive`.
    NativeAgentReceive(NativeAgentReceive),
    /// Variant `NativeAgentConfirm`.
    NativeAgentConfirm(NativeAgentConfirm),
    /// Variant `NativeAgentSend`.
    NativeAgentSend(NativeAgentSend),
    /// Variant `NativeAgentCancel`.
    NativeAgentCancel(NativeAgentCancel),
    /// Variant `NativeAgentClose`.
    NativeAgentClose(NativeAgentClose),
    /// Variant `NativeAgentStatus`.
    NativeAgentStatus(NativeAgentStatus),
    /// Variant `NativeAgentReadTool`.
    NativeAgentReadTool(NativeAgentReadTool),
    /// Variant `NativeAgentConfirmTool`.
    NativeAgentConfirmTool(NativeAgentConfirmTool),
}
