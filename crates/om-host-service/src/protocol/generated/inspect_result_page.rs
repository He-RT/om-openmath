//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InspectResultPage`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectResultPage {
    /// Contract field `kind`.
    pub kind: InspectResultPageKind,
    /// Contract field `path`.
    pub path: Vec<u32>,
    /// Contract field `offset`.
    pub offset: u32,
    /// Contract field `limit`.
    pub limit: u32,
    /// Contract field `column_offset`.
    pub column_offset: u32,
    /// Contract field `column_limit`.
    pub column_limit: u32,
}
impl std::fmt::Debug for InspectResultPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InspectResultPage { redacted }")
    }
}
