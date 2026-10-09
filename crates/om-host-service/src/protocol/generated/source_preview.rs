//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourcePreview`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePreview {
    /// Contract field `type`.
    pub r#type: SourcePreviewType,
    /// Contract field `arguments`.
    pub arguments: serde_json::Value,
}
impl std::fmt::Debug for SourcePreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourcePreview { redacted }")
    }
}
