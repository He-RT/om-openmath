//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `MarkdownSpan`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkdownSpan {
    /// Contract field `span_id`.
    pub span_id: String,
    /// Contract field `source_span`.
    pub source_span: SourceSpan,
    /// Contract field `rendered_range_utf16`.
    pub rendered_range_utf16: NativeRange,
    /// Contract field `kind`.
    pub kind: MarkdownSpanKind,
    /// Contract field `mapping`.
    pub mapping: MarkdownSpanMapping,
}
impl std::fmt::Debug for MarkdownSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MarkdownSpan { redacted }")
    }
}
