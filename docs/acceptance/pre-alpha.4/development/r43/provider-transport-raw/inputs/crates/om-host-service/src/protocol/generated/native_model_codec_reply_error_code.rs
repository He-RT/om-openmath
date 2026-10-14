//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeModelCodecReplyErrorCode`.
pub enum NativeModelCodecReplyErrorCode {
    /// Contract value `input`.
    #[serde(rename = "input")]
    Input,
    /// Contract value `unsupported`.
    #[serde(rename = "unsupported")]
    Unsupported,
    /// Contract value `replay`.
    #[serde(rename = "replay")]
    Replay,
    /// Contract value `response`.
    #[serde(rename = "response")]
    Response,
    /// Contract value `incomplete`.
    #[serde(rename = "incomplete")]
    Incomplete,
    /// Contract value `limit`.
    #[serde(rename = "limit")]
    Limit,
    /// Contract value `http`.
    #[serde(rename = "http")]
    Http,
    /// Contract value `cancelled`.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// Contract value `closed`.
    #[serde(rename = "closed")]
    Closed,
    /// Contract value `provider`.
    #[serde(rename = "provider")]
    Provider,
}
