//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `FrameReceipt`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameReceipt {
    /// Contract field `record_type`.
    pub record_type: FrameReceiptRecordType,
    /// Contract field `frame_id`.
    pub frame_id: String,
    /// Contract field `result_id`.
    pub result_id: String,
    /// Contract field `renderer_instance_id`.
    pub renderer_instance_id: String,
    /// Contract field `layout_key`.
    pub layout_key: LayoutKey,
    /// Contract field `frame_state`.
    pub frame_state: FrameReceiptFrameState,
    /// Contract field `render_state`.
    pub render_state: FrameReceiptRenderState,
    /// Contract field `screenshot_artifact_id`.
    pub screenshot_artifact_id: Nullable<String>,
    /// Contract field `error_code`.
    pub error_code: Nullable<String>,
}
impl std::fmt::Debug for FrameReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FrameReceipt { redacted }")
    }
}
