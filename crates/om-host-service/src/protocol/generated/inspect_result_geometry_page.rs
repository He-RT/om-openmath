//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InspectResultGeometryPage`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectResultGeometryPage {
    /// Contract field `kind`.
    pub kind: InspectResultGeometryPageKind,
    /// Contract field `channel`.
    pub channel: InspectResultGeometryPageChannel,
    /// Contract field `object_index`.
    pub object_index: u32,
    /// Contract field `segment_index`.
    pub segment_index: u32,
    /// Contract field `offset`.
    pub offset: u32,
    /// Contract field `limit`.
    pub limit: u32,
}
impl std::fmt::Debug for InspectResultGeometryPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InspectResultGeometryPage { redacted }")
    }
}
