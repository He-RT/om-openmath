//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `NativeKernelHostCommand`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NativeKernelHostCommand {
    /// Variant `KernelBootstrap`.
    KernelBootstrap(KernelBootstrap),
    /// Variant `KernelRunCell`.
    KernelRunCell(KernelRunCell),
    /// Variant `KernelStatus`.
    KernelStatus(KernelStatus),
    /// Variant `KernelReadBlob`.
    KernelReadBlob(KernelReadBlob),
    /// Variant `KernelBarrier`.
    KernelBarrier(KernelBarrier),
    /// Variant `KernelCompleted`.
    KernelCompleted(Box<KernelCompleted>),
    /// Variant `KernelUnknown`.
    KernelUnknown(KernelUnknown),
    /// Variant `KernelDiscard`.
    KernelDiscard(KernelDiscard),
    /// Variant `KernelSettled`.
    KernelSettled(KernelSettled),
    /// Variant `KernelState`.
    KernelState(KernelState),
}
