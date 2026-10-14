//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeModelCodecReplyPhase`.
pub enum NativeModelCodecReplyPhase {
    /// Contract value `prepared`.
    #[serde(rename = "prepared")]
    Prepared,
    /// Contract value `headers`.
    #[serde(rename = "headers")]
    Headers,
    /// Contract value `streaming`.
    #[serde(rename = "streaming")]
    Streaming,
    /// Contract value `decoded`.
    #[serde(rename = "decoded")]
    Decoded,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `cancelled`.
    #[serde(rename = "cancelled")]
    Cancelled,
}
