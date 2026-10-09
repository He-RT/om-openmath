//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `FrameReceiptFrameState`.
pub enum FrameReceiptFrameState {
    /// Contract value `queued`.
    #[serde(rename = "queued")]
    Queued,
    /// Contract value `gpu_completed`.
    #[serde(rename = "gpu_completed")]
    GpuCompleted,
    /// Contract value `presented`.
    #[serde(rename = "presented")]
    Presented,
    /// Contract value `failed`.
    #[serde(rename = "failed")]
    Failed,
    /// Contract value `discarded`.
    #[serde(rename = "discarded")]
    Discarded,
}
