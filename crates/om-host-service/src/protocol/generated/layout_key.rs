//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `LayoutKey`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutKey {
    /// Contract field `component_id`.
    pub component_id: String,
    /// Contract field `content_hash`.
    pub content_hash: String,
    /// Contract field `result_id`.
    pub result_id: Nullable<String>,
    /// Contract field `renderer_version`.
    pub renderer_version: String,
    /// Contract field `layout_generation`.
    pub layout_generation: Serial,
    /// Contract field `viewport_generation`.
    pub viewport_generation: Serial,
    /// Contract field `camera_generation`.
    pub camera_generation: Serial,
    /// Contract field `theme_generation`.
    pub theme_generation: Serial,
    /// Contract field `font_generation`.
    pub font_generation: Serial,
    /// Contract field `width_points`.
    pub width_points: f64,
    /// Contract field `font_size_points`.
    pub font_size_points: f64,
}
impl std::fmt::Debug for LayoutKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LayoutKey { redacted }")
    }
}
