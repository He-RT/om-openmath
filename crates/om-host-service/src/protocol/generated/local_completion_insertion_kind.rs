//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `LocalCompletionInsertionKind`.
pub enum LocalCompletionInsertionKind {
    /// Contract value `plain`.
    #[serde(rename = "plain")]
    Plain,
    /// Contract value `verified_snippet`.
    #[serde(rename = "verified_snippet")]
    VerifiedSnippet,
}
