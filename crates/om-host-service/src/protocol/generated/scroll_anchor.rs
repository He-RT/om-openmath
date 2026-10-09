//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ScrollAnchor`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScrollAnchor {
    /// Contract field `record_type`.
    pub record_type: ScrollAnchorRecordType,
    /// Contract field `cell_id`.
    pub cell_id: String,
    /// Contract field `block_or_result_id`.
    pub block_or_result_id: Nullable<String>,
    /// Contract field `source_range_utf16`.
    pub source_range_utf16: Nullable<NativeRange>,
    /// Contract field `viewport_offset_points`.
    pub viewport_offset_points: f64,
    /// Contract field `layout_generation`.
    pub layout_generation: Serial,
    /// Contract field `requires_active_editor_pin`.
    pub requires_active_editor_pin: bool,
}
impl std::fmt::Debug for ScrollAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScrollAnchor { redacted }")
    }
}
