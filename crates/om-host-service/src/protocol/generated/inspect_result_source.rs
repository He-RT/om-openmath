//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InspectResultSource`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectResultSource {
    /// Contract field `kind`.
    pub kind: InspectResultSourceKind,
    /// Contract field `path`.
    pub path: Vec<u32>,
    /// Contract field `format`.
    pub format: InspectResultSourceFormat,
    /// Contract field `byte_offset`.
    pub byte_offset: Serial,
    /// Contract field `byte_limit`.
    pub byte_limit: u32,
}
impl std::fmt::Debug for InspectResultSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InspectResultSource { redacted }")
    }
}
