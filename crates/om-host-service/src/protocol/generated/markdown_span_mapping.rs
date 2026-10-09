//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `MarkdownSpanMapping`.
pub enum MarkdownSpanMapping {
    /// Contract value `exact`.
    #[serde(rename = "exact")]
    Exact,
    /// Contract value `atomic`.
    #[serde(rename = "atomic")]
    Atomic,
    /// Contract value `unavailable`.
    #[serde(rename = "unavailable")]
    Unavailable,
}
