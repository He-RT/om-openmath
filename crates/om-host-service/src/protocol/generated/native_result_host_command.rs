//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `NativeResultHostCommand`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NativeResultHostCommand {
    /// Variant `ResultManifest`.
    ResultManifest(ResultManifest),
    /// Variant `ResultInspect`.
    ResultInspect(ResultInspect),
    /// Variant `ResultStatus`.
    ResultStatus(ResultStatus),
    /// Variant `ResultRevoke`.
    ResultRevoke(ResultRevoke),
}
