//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ResultManifest`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultManifest {
    /// Contract field `type`.
    pub r#type: ResultManifestType,
    /// Contract field `request_id`.
    pub request_id: String,
    /// Contract field `root_result_id`.
    pub root_result_id: String,
    /// Contract field `offset`.
    pub offset: u32,
    /// Contract field `limit`.
    pub limit: u32,
}
impl std::fmt::Debug for ResultManifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResultManifest { redacted }")
    }
}
