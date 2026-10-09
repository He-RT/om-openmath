//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceSpan`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSpan {
    /// Contract field `start`.
    pub start: u32,
    /// Contract field `end`.
    pub end: u32,
}
impl std::fmt::Debug for SourceSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceSpan { redacted }")
    }
}
