//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `NativeModelCodecCommand`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NativeModelCodecCommand {
    /// Variant `NativeModelCodecPrepare`.
    NativeModelCodecPrepare(NativeModelCodecPrepare),
    /// Variant `NativeModelCodecHeaders`.
    NativeModelCodecHeaders(NativeModelCodecHeaders),
    /// Variant `NativeModelCodecFeed`.
    NativeModelCodecFeed(NativeModelCodecFeed),
    /// Variant `NativeModelCodecFinish`.
    NativeModelCodecFinish(NativeModelCodecFinish),
    /// Variant `NativeModelCodecCancel`.
    NativeModelCodecCancel(NativeModelCodecCancel),
}
