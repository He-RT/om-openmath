//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `MarkdownBlock`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkdownBlock {
    /// Contract field `record_type`.
    pub record_type: MarkdownBlockRecordType,
    /// Contract field `block_id`.
    pub block_id: String,
    /// Contract field `source_hash`.
    pub source_hash: String,
    /// Contract field `source_span`.
    pub source_span: SourceSpan,
    /// Contract field `kind`.
    pub kind: MarkdownBlockKind,
    /// Contract field `renderer_version`.
    pub renderer_version: String,
    /// Contract field `complete_block`.
    pub complete_block: bool,
    /// Contract field `span_map`.
    pub span_map: Vec<MarkdownSpan>,
    /// Contract field `loads_remote_media`.
    pub loads_remote_media: bool,
    /// Contract field `executes_code`.
    pub executes_code: bool,
}
impl std::fmt::Debug for MarkdownBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MarkdownBlock { redacted }")
    }
}
