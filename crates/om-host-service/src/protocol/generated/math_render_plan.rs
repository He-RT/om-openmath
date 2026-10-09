//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `MathRenderPlan`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MathRenderPlan {
    /// Contract field `record_type`.
    pub record_type: MathRenderPlanRecordType,
    /// Contract field `layout_key`.
    pub layout_key: LayoutKey,
    /// Contract field `original_latex`.
    pub original_latex: String,
    /// Contract field `display_latex`.
    pub display_latex: String,
    /// Contract field `source_alternative`.
    pub source_alternative: String,
    /// Contract field `style`.
    pub style: MathRenderPlanStyle,
    /// Contract field `transform_ids`.
    pub transform_ids: Vec<String>,
    /// Contract field `parse_state`.
    pub parse_state: MathRenderPlanParseState,
    /// Contract field `fallback_source_visible`.
    pub fallback_source_visible: bool,
}
impl std::fmt::Debug for MathRenderPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MathRenderPlan { redacted }")
    }
}
