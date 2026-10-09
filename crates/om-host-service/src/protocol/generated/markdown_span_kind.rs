//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `MarkdownSpanKind`.
pub enum MarkdownSpanKind {
    /// Contract value `text`.
    #[serde(rename = "text")]
    Text,
    /// Contract value `code`.
    #[serde(rename = "code")]
    Code,
    /// Contract value `link`.
    #[serde(rename = "link")]
    Link,
    /// Contract value `math_attachment`.
    #[serde(rename = "math_attachment")]
    MathAttachment,
    /// Contract value `image_placeholder`.
    #[serde(rename = "image_placeholder")]
    ImagePlaceholder,
}
