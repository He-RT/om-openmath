//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceRead`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRead {
    /// Contract field `type`.
    pub r#type: SourceReadType,
}
impl std::fmt::Debug for SourceRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceRead { redacted }")
    }
}
