//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InspectResultScratch`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectResultScratch {
    /// Contract field `kind`.
    pub kind: InspectResultScratchKind,
    /// Contract field `source`.
    pub source: String,
    /// Contract field `dialect`.
    pub dialect: Dialect,
}
impl std::fmt::Debug for InspectResultScratch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InspectResultScratch { redacted }")
    }
}
