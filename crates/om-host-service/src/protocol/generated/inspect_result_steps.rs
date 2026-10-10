//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InspectResultSteps`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectResultSteps {
    /// Contract field `kind`.
    pub kind: InspectResultStepsKind,
    /// Contract field `parent_path`.
    pub parent_path: Vec<u32>,
    /// Contract field `offset`.
    pub offset: u32,
    /// Contract field `limit`.
    pub limit: u32,
}
impl std::fmt::Debug for InspectResultSteps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InspectResultSteps { redacted }")
    }
}
