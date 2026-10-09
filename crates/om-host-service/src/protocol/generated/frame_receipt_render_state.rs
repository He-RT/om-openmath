//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `FrameReceiptRenderState`.
pub enum FrameReceiptRenderState {
    /// Contract value `pending`.
    #[serde(rename = "pending")]
    Pending,
    /// Contract value `rendered`.
    #[serde(rename = "rendered")]
    Rendered,
    /// Contract value `data_only`.
    #[serde(rename = "data_only")]
    DataOnly,
    /// Contract value `unavailable`.
    #[serde(rename = "unavailable")]
    Unavailable,
}
