//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InspectResultPresentation`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectResultPresentation {
    /// Contract field `kind`.
    pub kind: InspectResultPresentationKind,
    /// Contract field `offset`.
    pub offset: u32,
    /// Contract field `limit`.
    pub limit: u32,
}
impl std::fmt::Debug for InspectResultPresentation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InspectResultPresentation { redacted }")
    }
}
