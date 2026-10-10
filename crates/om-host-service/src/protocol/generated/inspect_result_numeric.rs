//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InspectResultNumeric`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectResultNumeric {
    /// Contract field `kind`.
    pub kind: InspectResultNumericKind,
    /// Contract field `path`.
    pub path: Vec<u32>,
    /// Contract field `digits`.
    pub digits: u32,
}
impl std::fmt::Debug for InspectResultNumeric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InspectResultNumeric { redacted }")
    }
}
