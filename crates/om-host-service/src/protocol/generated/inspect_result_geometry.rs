//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InspectResultGeometry`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectResultGeometry {
    /// Contract field `kind`.
    pub kind: InspectResultGeometryKind,
}
impl std::fmt::Debug for InspectResultGeometry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InspectResultGeometry { redacted }")
    }
}
