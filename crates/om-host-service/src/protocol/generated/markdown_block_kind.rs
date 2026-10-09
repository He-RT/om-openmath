//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `MarkdownBlockKind`.
pub enum MarkdownBlockKind {
    /// Contract value `paragraph`.
    #[serde(rename = "paragraph")]
    Paragraph,
    /// Contract value `heading`.
    #[serde(rename = "heading")]
    Heading,
    /// Contract value `list`.
    #[serde(rename = "list")]
    List,
    /// Contract value `quote`.
    #[serde(rename = "quote")]
    Quote,
    /// Contract value `code`.
    #[serde(rename = "code")]
    Code,
    /// Contract value `table`.
    #[serde(rename = "table")]
    Table,
    /// Contract value `math`.
    #[serde(rename = "math")]
    Math,
    /// Contract value `raw_html`.
    #[serde(rename = "raw_html")]
    RawHtml,
    /// Contract value `fallback`.
    #[serde(rename = "fallback")]
    Fallback,
}
