//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InspectResultSummary`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectResultSummary {
    /// Contract field `kind`.
    pub kind: InspectResultSummaryKind,
}
impl std::fmt::Debug for InspectResultSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InspectResultSummary { redacted }")
    }
}
