//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ResultInspect`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultInspect {
    /// Contract field `type`.
    pub r#type: ResultInspectType,
    /// Contract field `request_id`.
    pub request_id: String,
    /// Contract field `result_ref`.
    pub result_ref: String,
    /// Contract field `query`.
    pub query: NativeResultQuery,
}
impl std::fmt::Debug for ResultInspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResultInspect { redacted }")
    }
}
